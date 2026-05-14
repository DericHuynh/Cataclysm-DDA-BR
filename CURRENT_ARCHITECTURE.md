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
| `cdda_core_types` | Value objects (Volume, Weight, Energy, Time), coordinate types (WorldPos, OmPos, etc.), generic `DefId<T>` string IDs, raw JSON definition structs, damage model, error types, RNG (WyRand) |

### Layer 2 — ECS components and shared schedule definitions

| Crate | Purpose | Bevy Deps |
|---|---|---|
| `cdda_components` | All Bevy ECS components: actor (creature, stats, bionics, effects, skills, mutations, morale, body parts), item (containers, pockets, inventory), def (definition template components), schedule (GameSet, SimSet), input (GameAction, BindableAction), events/messages, context (Ctx states, navigation) | `bevy_ecs`, `bevy_reflect` |
| `cdda_events` | Observer-based event types (DamageEvent, DeathEvent, EquipEvent, etc.) | `bevy_ecs` |
| `cdda_sim` | Simulation layer: state machine (AppState, TurnState), test utilities (TestBed) | `bevy_ecs` |

### Layer 3 — Game logic crates

| Crate | Purpose | Bevy Deps |
|---|---|---|
| `cdda_actor` | Creature systems: turn scheduling (ActionPoints), movement, bionics activation/deactivation, effects ticking, healing, temperature, morale decay, vision | `bevy_ecs` |
| `cdda_item` | Item logic: item relations, stacking, merging | `bevy_ecs` |
| `cdda_activity` | Player activity system (crafting, moving, waiting) | `bevy_ecs` |
| `cdda_combat` | Combat mechanics: damage calculation, hit/miss, melee, ranged | `bevy_ecs` |
| `cdda_crafting` | Crafting system: recipe lookup, component consumption, progress tracking | `bevy_ecs` |
| `cdda_equipment` | Equipment system: wielding, wearing, encumbrance | `bevy_ecs` |
| `cdda_inventory` | Inventory system: item bins, invlet assignment, item movement events, merge/stack | `bevy_ecs` |
| `cdda_ai` | AI behaviors: monster/NPC decision-making, pathfinding | `bevy_ecs` |
| `cdda_noise` | Sound propagation: noise events for AI sensory input | `bevy_ecs` |

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
| `cdda_render` | Rendering plugin: UI screens (inventory, crafting, character sheet, examine, main menu, overmap, settings), ASCII viewport, tile rendering, theming | Full `bevy` |
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

## Known Technical Debt

See the root `AGENTS.md` and the priority list below. Key issues:

- **Missing architecture docs** (this file was just created — previously absent)
- **build_def_world** is a ~900-line function mixing parsing, registry building,
  and entity spawning
- **DefRegistry** has ~100+ fields as a single global registry struct
- **Some bool fields** in Bionic and MutationEntry still exist (partially migrated
  to tag components)
- **Hardcoded magic constants** (world seed, tile sizes, stat bounds)
