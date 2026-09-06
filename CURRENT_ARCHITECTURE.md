# Current Architecture — CDDA-BR

Player-visible equivalence with master is **not established**. The [semantics audit](docs/master-semantics-audit.md) records resolved neutral timing scenarios and remaining differences in crafting checks/modifiers, reach and item handling. Architectural delivery and BR regression tests do not imply master parity.

CDDA-BR is a Rust/Bevy 0.18 reimplementation. This document distinguishes implemented contracts from pending architecture work. Local AGENTS files own APIs and verification commands; TARGET_ARCHITECTURE.md owns the remaining roadmap.

## Ownership

The virtual Cargo workspace has 17 members:

| Boundary | Crates / responsibility |
|---|---|
| Domain values and raw data | cdda_core_types (units/coordinates/IDs/RNG), cdda_defs_raw (JSON AST) |
| Shared ECS contracts | cdda_components (domain components, relationships, intent/results, schedule labels) |
| Native catalog | cdda_catalog (typed index, interners, normalized inventory definitions, HTN input) |
| Generic UI | cdda_ui (virtual-list geometry, retained keyed rows, scroll and selection reveal) |
| Planner core | cdda_htn (bevy_ecs-only, no cdda dependencies) |
| Simulation | cdda_sim (runtime, actor, AI, intents, activities, combat, crafting, equipment, inventory, items, noise) |
| Data and world | cdda_data (parse/resolve/translate/project), cdda_overmap (OMT storage/terrain registry/spatial index), cdda_overmap_gen (generation) |
| Adapters | cdda_context, cdda_input, cdda_render, cdda_replay |
| Entry points | cdda_app, cdda_cli |
| Cross-crate tests | cdda_integration_tests |

The planner core was developed standalone as bevy_bhtn, moved into this workspace, and renamed cdda_htn, replacing the old reflection-based planner. Game-specific integration lives in cdda_sim::ai::htn.

Crate layers are organizational guidance, not proof of isolation. Simulation depends on cdda_catalog and native contracts. Data/raw schemas are test-only dependencies; input lives in cdda_input and navigation in cdda_context. scripts/check_runtime_dependencies.py verifies the transitive boundary. No crate may depend on application entry points.

## Canonical simulation schedule

`cdda_sim::runtime::SimulationPlugin` installs the persistent `SimulationTurn` schedule and gameplay subsystem plugins/resources. The graphical app uses this same plugin; headless tests need no renderer or window.

Outer `Update` is ordered:

1. GameSet::Input — adapters publish work.
2. GameSet::Sim — drive_simulation invokes logical turns.
3. GameSet::Render — presentation reads committed state.

Inside each SimulationTurn:

TurnTick → Effects → Healing → Bionics → Morale → Temperature → Vision → Spawning. SimulationIngress then handles legacy requests; the shared budget loop selects actors for SimulationAction (IntentDeclare → IntentResolve) or SimulationActivity (typed work → completion). SimulationRefresh runs Inventory → SpatialUpdate after commits.

The legacy PendingCraft mailbox becomes a StartCraft intent before arbitration; ingredient consumption occurs during resolution. Craft work and completion share the selected actor’s activity schedule. Legacy inventory requests run in ingress; invlet assignment and bin rebuild run after commits. The app extends SimulationRefresh with spatial synchronization; camera and overlay extraction stay in outer Update.

### Time and pause

- GameTime and parsed definition Time agree: one logical turn is one game second (3600/hour, 86400/day).
- SimulationControl defaults to TurnBased: idle render updates do not grant AP, decay effects or move AI. Declared living-actor intents, ongoing activities, pending crafts and legacy item moves request work.
- Manual mode accepts explicit request_steps; step_simulation runs one production turn with persistent system-local state.
- Optional RealTime mode uses SimClock only as wall pacing (default 100 ms). Multiple turns can execute per frame up to a cap; elapsed backlog is retained. Pausing clears wall debt, not explicit queued requests.
- The central gate freezes logical turns if SimulationControl.paused or an installed AppState is not InGame. Headless worlds may omit AppState. Calling raw run_schedule bypasses the driver and is not the supported stepping API.

TurnBased dispatch reuses positive PlayerData/DevPlayer moves across input frames without advancing GameTime, granting AP or ticking effects. Other actors wait until player moves are exhausted. Ingress and post-commit refresh also run for commands within a turn; idle input does not repeat refresh work. Manual and RealTime stepping remain explicit world-turn operations. Craft ticks consume the entire available budget, including finishing overshoot; neutral partial TIME costs use integer truncation.

**Remaining:** crafting modifiers/checks/reach and detailed world-phase ordering differ from master; combat verbs (MeleeAttack/UseItem/Reload) are still unsupported on the intent path.

## Combat data ownership

MeleeCapability, DodgeDefense and IntrinsicArmor are independent runtime components. CombatStats is a plain legacy construction record with explicit into_bundle conversion, never a second runtime authority. The character sheet handles changes/removals independently. Monster definition JSON still translates through cdda_data; its dodge projection now reads dodge rather than melee dice. Equipment-derived protection and native melee/ranged execution remain pending.

## Action execution

ActionIntent is a request, not a completed action. The collector orders requests by descending AP then ascending SimId (Entity bits only as a fallback) and, under the budget scheduler, collects only the currently selected `ActingEntity`.

The exclusive resolver validates each request against the live world, synchronously commits its mutation and AP cost, then publishes ActionOutcome. Later requests see earlier commits; two actors cannot both successfully pick up one ground item.

- Move: existing position, nonzero one-tile offset, checked coordinate arithmetic, no ECS Solid occupant.
- Pickup/Wield/Drop/Stow/Transfer: the shared transactional boundary `inventory::transfer::apply_inventory_action` validates live exclusive location, ownership-chain/cycle safety, same-z reach for ground items, hand counts, projected pocket/ancestor volume and weight, sealed access and the whole-stack exact-tile floor cap, then charges 100 AP once. Stable storage selection tries fitting pockets; explicit transfers cannot bypass capacity through loose inventory. Restricted/specialized pockets and non-counted-solid dimensions are rejected until supported. Inventory screen and dev input adapters declare intents only — no AP or relationship bypass.
- InterruptActivity and ResumeCraft resolve before selected activity work. They retain saved craft progress, validate ownership/access and emit correlated outcomes without setup cost. BT/GOAP preserve submitted intents; an external command invalidates the previous HTN plan. Item-examine input now submits native commands from cdda_render.
- StartCraft validates/consumes ingredients and starts an activity; Completed reports that start. Craft ticks spend all available AP, including finishing overshoot; aim/reload use bounded quanta. Reading/waiting/interaction consume the budget for elapsed time, truncating proportional cost on partial final seconds. Non-craft completion leaves unspent AP for queued actions, including zero-cost TIME completion. Interruption retains craft items; resume validates ownership/access, and completion requires finished work.
- Wait completes and consumes AP. Rejected and unsupported Failed actions charge no AP.

Collision currently has no local terrain model beyond ECS Solid entities. Legacy ItemMoveEvent requests now use the same live transfer boundary and emit ItemMoveResult. Stale source/count claims and partial stacks reject without mutation; persistent cursors prevent replay. Letter assignment no longer implicitly merges or relocates stacks. Explicit colocated merges preserve dimensions/native snapshots and reject containers/overflow. Remaining work includes remaining activity-start verbs, specialized pocket semantics, and direct spawn/craft-output placement policy.

Relationships maintain reverse links, not gameplay capacity/ownership/resource invariants. Explicit operations and phase ordering remain necessary. Events are notifications/bounded reactions, not a replacement scheduler.

## Data and definition projection

Loader ingests core/mod JSON then resolves copy-from/patches into typed DefRegistry. ModManager layers mods into the retained loader; recipe composite keys survive resolution. The raw resolved JSON is retained through bridge import/export adapters for lossless tooling.

build_def_world has per-category builders and spawns IsDef entities into the **main ECS World**. DefinitionWorld is an index resource, not a separate World. Runtime queries must filter definitions where appropriate.

**Identity:** the index is category-qualified, recipe projections carry composite keys, and rebuilds bump a generation. Legacy rebuilding still invalidates cached Entities. The separate native inventory publisher validates before mutation; items and prepared craft outputs retain snapshots across replacement. General actor/AI migration remains pending.

The strict native family in cdda_data::inventory_import retains original/resolved values with capability diagnostics. It covers counted generic items, unrestricted pockets, qualities and counted recipes; broad legacy app loading remains separate. See docs/ecs-compatibility-baseline.md. Crafting reads nested inventory through the shared traversal, excluding sealed contents. It validates outputs and reserves all slots before consumption; PreparedItem creates populated outputs. CraftModel/CraftState/CategoryIndex/InventoryFocus are renderer-owned. Crafting read-model extraction and membership filtering are independent of focus changes. Generic scrolling and retained keyed text rows live in cdda_ui; crafting, character, Settings, registry and debug spawn reuse row/cell entities and patch changed values. Counters and headings update in place outside list scrolling; registry raw/parsed headings are fixed above their detail panes. RegistryCatalog/DevSpawnCatalog source projections are independent of selection state. Registry input and presentation use typed queries; heterogeneous source extraction is the sole exclusive World operation. Catalog refresh preserves stable selections, source changes invalidate details, and empty registry categories clear stale content. Spawn filter membership is cached independently of navigation.

## Terrain identities and persistence

TerrainHandle indices are runtime-only. Registration upserts a stable ID in place; new IDs append slots. TerrainRegistry::rebuild_from preserves existing terrain/family slots and remaps rotation links. Removed old IDs or invalid links reject the rebuild atomically rather than reinterpreting existing chunks.

App reload validates the terrain candidate before destructive definition rebuilding. A rejected candidate preserves the previous runtime and is not logged as successful. This protection is specific to terrain; it is not a general definition migration solution.

Binary chunk format v1 uses magic/version, coordinates, a palette of stable UTF-8 terrain IDs, and cells containing palette index plus rotation. Read/write APIs require a TerrainRegistry. Loading against a differently ordered registry preserves terrain meaning. Unknown IDs, malformed data, unsupported versions and the old raw-index format are rejected. See cdda_overmap/AGENTS.md for framing and limits.

Overmap chunks store 30×30 **OMT** handles, not local submap tile content. The dev viewport is an OMT preview; normal walking now moves one world tile rather than teleporting 24 tiles for one action cost.

## HTN integration

Mods author htn_compound definitions; Rust kernels define predicate/operator meaning; simulation determines whether operations happen. The compiler uses explicit graph handles, specializes parameterized calls, supports recursive graph edges and rejects unknown kernels/references with located errors.

Actor observations cover needs, recursive inventory and nearby/navigation facts. Planning effects stay in scratch models. HtnRuntime holds domain/catalog together; the execution adapter consumes correlated outcomes instead of assuming submitted actions succeeded.

The graph/compiler/adapter are tested headlessly. Complete startup/reload publication, generation invalidation and shared action-budget integration remain pending; BT/GOAP drivers are still placeholders.

## Verification and remaining work

Implemented regression suites:
- simulation_schedule_test: production schedule, idle frames, central pause/lifecycle gates, persistent local state, frame partition and bounded catch-up, calendar units.
- intent_transaction_test: competing pickup, position/range/ownership/cycle validation, committed spatial/AP visibility, stable tie order.
- terrain_persistence_tests: reordered registries, rotation preservation, malformed/unknown data rejection, append-only rebuild and rollback.
- Existing HTN, data-loader and inventory traversal tests remain relevant.

The isolated TestBed recreates systems per call and is not proof of production schedule behavior. The migrated actor/combat/inventory suites are restored to Cargo discovery via `migrated_{actor,combat,inventory}.rs` aggregators (302 tests; 83 remain `#[ignore]`d stubs). Replay hashing now digests committed gameplay state — per-entity position, AP, health, stack count and containment/wield/wear ownership resolved through stable `SimId`s, sorted so spawn order cannot change the digest — and runs after the simulation driver in `GameSet::Render`. Replay mode never mutates the expected log it is compared against; divergence fires `SimulationDiverged` per turn. Remaining replay debt: `InputAction`-level recording with turn-at-drain stamps instead of semantic commands, no RNG/definition-version in the digest, and replay-speed handling of missed turns.

Next: crafting reach/access, supported recipe checks and outputs, then the remaining master parity gates. Later foundations: definition generations; local submap/active-region lifecycle and catch-up; replay integration; combat verbs and legacy message-path consolidation. Missing combat/physiology/world features are not presented as completed architecture.
