---
name: Project overview
description: Cataclysm-DDA-BR Bevy ECS game project — crate layout, key types, current state
type: project
---

Bevy 0.18 ECS game reimplementing Cataclysm: Dark Days Ahead. Workspace at `/home/deric/git/Cataclysm-DDA-BR`.

## Crate layout (current)
- `cdda_core` — IDs, time, units, primitive types; zero Bevy deps
- `cdda_data` — two-pass JSON loader, `DefRegistry`, raw defs; zero Bevy deps
- `cdda_assets` — NEW: Bevy `Asset` wrapper around `DefRegistry`; `CddaDataPackLoader` for `.pack` manifests; `CddaAssetsPlugin`
- `cdda_actor` — creature/player/NPC/stats/bionics ECS components; `ActorPlugin` owns `register_type`
- `cdda_item` — item/inventory/pocket ECS components; `ItemPlugin` owns `register_type`
- `cdda_sim` — simulation systems, `def_world.rs` loader, `world_setup.rs`, test utilities
- `cdda_map` — map data structures
- `cdda_render` — Bevy rendering plugin
- `cdda_input` — input bindings
- `cdda_ui` — screen states (`Screen` enum, `GameEvent`)
- `cdda_app` — binary entry point, `CddaPlugin` wires everything
- `cdda_replay` — session recording/playback

## Key architectural decisions completed
- All 12 original architectural fixes done (state sync, turn gating, Vec→ECS relationships, etc.)
- `cdda_ui` dep removed from `cdda_sim` (it was unused; the Screen transition is handled by OnEnter in cdda_app)
- Actor/item components use `#[relationship]` / `#[relationship_target]` pattern (like BionicOf/InstalledBionics)
- Per-crate plugins: `ActorPlugin`, `ItemPlugin`, `CddaAssetsPlugin`

**Why:** Each crate owns its reflect type registrations so cdda_app doesn't need to import every type.

**How to apply:** When adding new components to cdda_actor or cdda_item, add `app.register_type::<T>()` to the respective plugin.

## Data loading
- Current path: `cdda_sim::def_world::load_data_system` reads `StartupConfig.data_dirs` → runs `cdda_data::Loader`
- Future path: `cdda_assets::CddaAssetsPlugin` + `AssetServer::load("core.pack")` → `CddaDataPack` asset with hot-reload
- `assets/core.pack` manifest exists at `crates/cdda_app/assets/core.pack`
