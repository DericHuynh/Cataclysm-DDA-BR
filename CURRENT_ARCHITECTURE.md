# Current Architecture — CDDA-BR

CDDA-BR is a Rust/Bevy 0.18 reimplementation. This document distinguishes implemented contracts from pending architecture work. Local AGENTS files own APIs and verification commands; TARGET_ARCHITECTURE.md owns the remaining roadmap.

## Ownership

The virtual Cargo workspace has 15 members:

| Boundary | Crates / responsibility |
|---|---|
| Domain values and raw data | cdda_core_types (units/coordinates/IDs/RNG), cdda_defs_raw (JSON AST) |
| Shared ECS contracts | cdda_components (domain components, relationships, intent/results, schedule labels, current input/context vocabulary) |
| Planner core | cdda_htn (bevy_ecs-only, no cdda dependencies) |
| Simulation | cdda_sim (runtime, actor, AI, intents, activities, combat, crafting, equipment, inventory, items, noise) |
| Data and world | cdda_data (resolve/project/catalog), cdda_overmap (OMT storage/terrain registry/spatial index), cdda_overmap_gen (generation) |
| Adapters | cdda_context, cdda_input, cdda_render, cdda_replay |
| Entry points | cdda_app, cdda_cli |
| Cross-crate tests | cdda_integration_tests |

The planner core was developed standalone as bevy_bhtn, moved into this workspace, and renamed cdda_htn, replacing the old reflection-based planner. Game-specific integration lives in cdda_sim::ai::htn.

Crate layers are organizational guidance, not proof of isolation. Current sim depends on the data crate's catalog plus loader/asset surface; components also contains input-framework types. No crate may depend on application entry points.

## Canonical simulation schedule

`cdda_sim::runtime::SimulationPlugin` installs the persistent `SimulationTurn` schedule and gameplay subsystem plugins/resources. The graphical app uses this same plugin; headless tests need no renderer or window.

Outer `Update` is ordered:

1. GameSet::Input — adapters publish work.
2. GameSet::Sim — drive_simulation invokes logical turns.
3. GameSet::Render — presentation reads committed state.

Inside each SimulationTurn:

TurnTick → IntentDeclare → IntentResolve → Activity → Effects → Healing → Bionics → Morale → Temperature → Vision → Spawning → Inventory → SpatialUpdate.

Craft start precedes its tick and completion follows it. Inventory movement, invlet assignment, and bin rebuild are chained. The app extends the logical schedule with spatial synchronization; camera and overlay extraction stay in outer Update.

### Time and pause

- GameTime and parsed definition Time agree: one logical turn is one game second (3600/hour, 86400/day).
- SimulationControl defaults to TurnBased: idle render updates do not grant AP, decay effects or move AI. Declared living-actor intents, ongoing activities, pending crafts and legacy item moves request work.
- Manual mode accepts explicit request_steps; step_simulation runs one production turn with persistent system-local state.
- Optional RealTime mode uses SimClock only as wall pacing (default 100 ms). Multiple turns can execute per frame up to a cap; elapsed backlog is retained. Pausing clears wall debt, not explicit queued requests.
- The central gate freezes logical turns if SimulationControl.paused or an installed AppState is not InGame. Headless worlds may omit AppState. Calling raw run_schedule bypasses the driver and is not the supported stepping API.

**Remaining:** player budgets bank rather than force re-prompting; activities tick a fixed per-turn slice instead of consuming the action budget; combat verbs (MeleeAttack/UseItem/Reload) are still unsupported on the intent path.

## Action execution

ActionIntent is a request, not a completed action. The collector orders requests by descending AP then ascending SimId (Entity bits only as a fallback) and, under the budget scheduler, collects only the currently selected `ActingEntity`.

The exclusive resolver validates each request against the live world, synchronously commits its mutation and AP cost, then publishes ActionOutcome. Later requests see earlier commits; two actors cannot both successfully pick up one ground item.

- Move: existing position, nonzero one-tile offset, checked coordinate arithmetic, no ECS Solid occupant.
- Pickup/Wield/Drop/Stow: the shared transactional boundary `inventory::transfer::apply_inventory_action` validates live exclusive location, ownership-chain/cycle safety, same-z reach for ground items, hand counts and the exact-tile floor cap, then charges 100 AP once. Inventory screen and dev input adapters declare intents only — no AP or relationship bypass.
- Wait completes and consumes AP. Rejected and unsupported Failed actions charge no AP.

Collision currently has no local terrain model beyond ECS Solid entities. Inventory messages, UI equipment mutations, stack merging and pending-craft dispatch still need to use shared validating operations.

Relationships maintain reverse links, not gameplay capacity/ownership/resource invariants. Explicit operations and phase ordering remain necessary. Events are notifications/bounded reactions, not a replacement scheduler.

## Data and definition projection

Loader ingests core/mod JSON then resolves copy-from/patches into typed DefRegistry. ModManager layers mods into the retained loader; recipe composite keys survive resolution. The raw resolved JSON is retained through bridge import/export adapters for lossless tooling.

build_def_world has per-category builders and spawns IsDef entities into the **main ECS World**. DefinitionWorld is an index resource, not a separate World. Runtime queries must filter definitions where appropriate.

**Remaining:** the shared string-only index loses category identity; recipe projection discards its stable key; destructive definition rebuild invalidates retained Entity references. General validated definition-generation publication and migration are not yet complete.

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

Next foundations: definition generations; local submap/active-region lifecycle and catch-up; replay integration; combat verbs and legacy message-path consolidation. Missing combat/physiology/world features are not presented as completed architecture.
