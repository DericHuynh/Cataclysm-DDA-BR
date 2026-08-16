# Current Architecture — CDDA-BR

> **Last updated:** This document describes the architecture as currently
> implemented. See [TARGET_ARCHITECTURE.md](TARGET_ARCHITECTURE.md) for the
> planned migration path.

## Overview

CDDA-BR is a reimplementation of Cataclysm: Dark Days Ahead in Rust using
Bevy ECS 0.18. The project is organized into 22 workspace crates under
`crates/`, plus the root binary crate.

## Crate Layout

### Layer 1 — Pure domain types (no Bevy ECS dependency)

| Crate | Purpose |
|---|---|
| `cdda_core_types` | Value objects (Volume, Weight, Energy, Time), coordinate types (WorldPos, OmPos, etc.), generic `DefId<T>` string IDs, raw JSON definition structs, damage model, error types, RNG (SeededRng, SimId) |

### Layer 2 — ECS components and shared schedule definitions

| Crate | Purpose | Bevy Deps |
|---|---|---|
| `cdda_components` | All Bevy ECS components: actor (creature, stats, bionics, effects, skills, mutations, morale, body parts), item (containers, pockets, inventory), activity (progress, crafting, reading, weariness), def (definition template components), schedule (GameSet, SimSet), input (GameAction, BindableAction), events/messages, context (Ctx states, navigation). **Single home for all shared domain components and event/message types.** | `bevy_ecs`, `bevy_reflect` |
| `cdda_sim` | Simulation layer: the consolidated game-logic submodules (actor, ai, activity, combat, crafting, equipment, inventory, item, noise) plus the state machine (AppState, TurnState) and test utilities (TestBed). Owns systems, not component data. | `bevy_ecs` |

### Layer 3 — Game logic crates

The game-logic subsystems listed below were **consolidated into `cdda_sim` submodules** (the older separate crates `cdda_actor`, `cdda_item`, `cdda_activity`, `cdda_combat`, `cdda_crafting`, `cdda_equipment`, `cdda_inventory`, `cdda_ai`, `cdda_noise` no longer exist as crates). Each submodule owns one gameplay concern's systems and shares data via `cdda_components`:

| Subsystem (`cdda_sim::`) | Purpose |
|---|---|
| `actor` | Creature turn scheduling (ActionPoints), movement, bionics, effects, healing, temperature, morale, vision |
| `item` | Item type registration |
| `activity` | Player activity ticking (crafting, aiming, reading, waiting, reloading) — drives `cdda_components::activity` |
| `combat` | Damage, hit/miss, melee, ranged |
| `crafting` | Recipe lookup, component consumption, progress |
| `equipment` | Wielding, wearing, encumbrance |
| `inventory` | Stacks, invlets, binned lookups, item movement |
| `ai` | Monster/NPC decision-making, pathfinding |
| `noise` | Sound propagation for AI sensory input |

### Layer 4 — World and data crates

| Crate | Purpose | Bevy Deps |
|---|---|---|
| `cdda_data` | JSON loading (two-pass: ingest → resolve), `copy-from` inheritance (extend/delete/relative/proportional), DefRegistry (single authoritative store of all game definitions), flag population, schema generation | `bevy_ecs`, `bevy_state` |
| `cdda_overmap` | Overmap storage, overmap terrain queries, pathfinding | `bevy_ecs` |
| `cdda_overmap_gen` | Overmap generation: pipeline (matching C++ order), city/special/connection/mongroup placement, deterministic RNG | `bevy_ecs` |

### Layer 5 — Input, Render, and App

| Crate | Purpose | Bevy Deps |
|---|---|---|
| `cdda_context` | Headless context state machine (Ctx states, pop/push navigation, overlay stack, focus management, menu state) | `bevy_ecs`, `bevy_state` |
| `cdda_input` | Input plugin: ActionState → InputAction bridging, keybinding maps, input context stacks, bindable actions | Full `bevy` |
| `cdda_render` | Rendering plugin: UI screens (inventory, crafting, character sheet, examine, main menu, overmap, settings), ASCII viewport, tile rendering, theming. **Also hosts the screen input adapters (`render/input.rs`) — the presenter layer that translates `InputAction` (UI vocabulary) into `cdda_sim` use-case calls, so `cdda_sim` never matches `GameAction`.** | Full `bevy` |
| `cdda_replay` | Replay system: session logging, deterministic replay, state hashing | `bevy_ecs` |
| `cdda_app` | Binary entry point: wires all subsystems, configures Bevy DefaultPlugins, registers system ordering (Input → Sim → Render) | Full `bevy` |
| `cdda_cli` | CLI subcommands: `run` (default), `schedule-graph`, `render-graph`, `dump` | Full `bevy` |

### Hub crate: `cdda_core`

`cdda_core` re-exports everything from all other crates and provides startup
systems (`load_data_system`, `spawn_dev_world`, `worldgen_system`). It acts
as a facade but also creates circular dependency resilience.

## Data Loading Pipeline

The data loading follows a two-pass approach:

1. **Pass 1 (Ingest):** `Loader::ingest_all()` walks `data/` directories, reads all
   `.json` files, groups raw `serde_json::Value`s by their `"type"` field.
2. **Pass 2 (Resolve):** `Loader::load()` → `resolve_copy_from()` deserializes each
   raw def into typed structs, resolves `copy-from` inheritance chains
   (extend/delete/relative/proportional), produces `DefRegistry`.
3. **Def World Construction:** `build_def_world()` spawns Bevy entities for each
   definition (items, monsters, terrain, furniture, recipes, body parts) with
   typed components.

After loading, the `DefinitionWorld` resource maps string IDs to entity IDs for
runtime lookup.

### Part-B bridge (import/export adapters)

The **fully-resolved raw JSON** (the output of `Loader::resolve_type_raw`) is the
lossless source of truth for every def. Typed structs and Bevy components are
*projections*, not independent stores. A decoupled import/export `bridge`
(`crates/cdda_data/src/bridge.rs`) is the seam a GUI JSON editor / format
migration builds on:

- **Import:** resolved JSON → `DefRecord<T>` (a Bevy `Component` carrying both
  the raw `Value` and the typed parse), so unmodeled keys are never dropped.
- **Export:** `compute_overrides` + `apply_delta` + `export_override_def`
  rebuild the *minimal* `copy-from` override delta against a def's parent —
  inherited (unchanged) fields are omitted, new fields added, removed inherited
  fields become `delete`. Re-applying the delta to the parent reproduces the
  child (verified across `data/core` by the `bridge` CLI / `bridge_all_types`).

Import and export are fully independent (no shared state), so a new wire format
(v2, a mod-pack delta, a different storage layout) needs only a new adapter.

## Simulation Tick

The simulation runs in `GameSet::Sim` with phases ordered via `SimSet`:
1. `TurnTick` — grant action points
2. `Activity` — process ongoing activities
3. `Ai` — AI decision-making (monsters, NPCs)
4. `Movement` — resolve movement
5. `Combat` — resolve combat
6. `Effects` — tick status effects
7. `Healing` — natural healing
8. `Bionics` — bionic power drain
9. `Morale` — morale decay
10. `Temperature` — body temperature
11. `Vision` — sight range updates
12. `Spawning` — creature/item spawning
13. `Inventory` — item movement events, bin building
14. `SpatialUpdate` — spatial index maintenance

## Key Design Decisions

### Relationships over components
Bevy ECS relationships (`#[relationship]` / `#[relationship_target]`) model
all entity-to-entity connections: inside-container, wielded-by, worn-on,
skill-of, mutation-of, bionic-of, effect-on. Mutate by reinserting via
`commands.insert()`, never via `&mut` access.

### Tag components over bool fields
Boolean properties use tag components for archetype-level filtering:
`With<Visible>`, `With<Active>`, `With<Sealed>`, `With<Rigid>`, etc.

### Messages vs Events
- **Messages** (buffered, processed next frame): `ItemMoveEvent`, `SoundEvent`,
  `SightEvent`, `SpawnEvent`. Used for bulk/batched operations.
- **Events / EntityEvent** (immediate, observer-based): `DamageEvent`,
  `DeathEvent`, `EquipEvent`, `UnequipEvent`, `UseItemEvent`. Used for
  entity-to-entity reactions.

### DefId<T> pattern
String-based type-safe identifiers via `DefId<T>`. Each definition category
has a marker type (ItemDef, MonsterDef, TerrainDef, etc.) for type-level
distinction.

### Fair turn ordering (no player priority)
Player input and AI decisions both declare an `ActionIntent` into a single
buffered `IntentQueue` that is sorted by **action points descending** and
resolved first-highest-AP-wins. This deliberately differs from CDDA-Master's
blocking player loop (where the player acts for their whole budget before `monmove()`):
in the rewrite a fast monster can act before a slow player. Validated by the
`higher_ap_monster_goes_before_lower_ap_player` test.

### Pluggable AI planners
Each AI mob carries one planner marker component
(`PlannerBehaviourTree` / `PlannerGoap` / `PlannerHtn` / inert `PlannerNone`)
that selects its decision algorithm. A per-marker system (`drive_<planner>`,
`.run_if` on the marker) produces an `AiGoal` which is translated into an
`ActionIntent` feeding the shared queue. So a dumb zombie can use a behaviour
tree, a feral zombie GOAP, and a survivor / high-level zombie an HTN — all
planners share one dispatch seam and one AP-sort.

The HTN planner itself lives in the **headless `cdda_htn` crate** (a leaf with
no ECS / `Component` / `cdda_sim` dependency), so it can be adopted by any AI
layer. It imports `.htn` files (the `htn.pest` DSL), drives strongly-typed
operators via `bevy_reflect`, and supports both **forward** planning (MTR
backtracking over method decomposition) and **backward / goal-state** planning
given a `goal_task`. The marker/`cdda_sim` integration (wiring `PlannerHtn` to
produce `ActionIntent`s) is the seam the sim exposes; the full HTN hookup is a
follow-up.

## Known Technical Debt

See the root `AGENTS.md` and the priority list below. Key issues:

- **Missing architecture docs** (this file was just created — previously absent)
- **build_def_world** is a ~900-line function mixing parsing, registry building,
  and entity spawning
- **DefRegistry** has ~100+ fields as a single global registry struct
  (the per-field `total_count` / `category_count` / `resolve_all` / skipped-type
  list are now macro-generated from the `for_each_raw_def_kind!` table, but the
  struct fields themselves and `DefRegistry::empty` are still hand-maintained)
- **Some bool fields** in Bionic and MutationEntry still exist (partially migrated
  to tag components)
- **Hardcoded magic constants** (world seed, tile sizes, stat bounds)
