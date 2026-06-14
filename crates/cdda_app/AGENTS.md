# cdda_app DOX

## Purpose
Binary entry point for `cdda` — owns `App::new()` construction, full-Bevy plugin
wiring, `AppState` machine, and runtime startup. Layer 5 leaf crate (see
`crates/AGENTS.md`); nothing may depend on it.

## Ownership
- `src/main.rs` — `clap` CLI: subcommands `run` (default), `schedule-graph`, `render-graph`, `dump`. Dump subcommands build their own `App::new()` and call `CddaPlugin`.
- `src/lib.rs` — `CddaStartupConfig` resource, `CddaPlugin`, `dev_player_move`, `toggle_overmap`, and `run()` (the live-game `App::new()`).
- `src/startup.rs` — data loading + dev-world spawn. No `App::new()` here.
- `assets/` — runtime asset root. Contains `core.pack`, `fonts/Inter-VariableFont.ttf`, `fonts/ShareTechMono-Regular.ttf`, and `gfx` (symlink → `../../../gfx`, the repo's shared tileset tree).

## Local Contracts
- **Bevy deps** (`Cargo.toml`): full `bevy` (with `3d`), `bevy_ecs`, `bevy_state` 0.18, `bevy_egui` 0.39, `bevy-inspector-egui`, `bevy_mod_debugdump` 0.15, plus every workspace crate (`cdda_components`, `cdda_activity`, `cdda_actor`, `cdda_ai`, `cdda_combat`, `cdda_context`, `cdda_crafting`, `cdda_data`, `cdda_input`, `cdda_inventory` (path dep), `cdda_item`, `cdda_events`, `cdda_overmap`, `cdda_overmap_gen`, `cdda_render`, `cdda_replay`, `cdda_sim`, `cdda_core_types`). `default = ["dynamic"]` and `dynamic = ["bevy/dynamic_linking"]` — the binary always dynamically links Bevy in dev.
- **Where `App::new()` lives.** `lib.rs::run()` for the live game; `main.rs` for the three dump subcommands. `CddaPlugin::build` is the subsystem registration site — *not* `startup.rs`. `startup.rs` is data-loading + dev-spawn only.
- **`CddaStartupConfig`** (`lib.rs`): `world_seed: u64`, `replay_file: Option<String>`, `record_session: bool`. Three modes wired in `CddaPlugin::build`:
  - `replay_file = Some(path)` → `SessionLog::load_compressed` + `CddaReplayModePlugin`.
  - `record_session = true` → `CddaReplayPlugin { world_seed }`.
  - else → default seed = UNIX-epoch seconds, no replay.
- **Plugin registration order** (`CddaPlugin::build`): `ActivityPlugin`, `ActorPlugin`, `ItemPlugin`, `CddaAssetsPlugin`, `CraftingPlugin`, `CddaDataPlugin`, then `OvermapGenPlugin`. The `Update` schedule uses `(GameSet::Input, GameSet::Sim, GameSet::Render).chain()`; `SimSet` is chained inside `GameSet::Sim` (`TurnTick → Activity → Ai → Movement → Combat → Effects → Healing → Bionics → Morale → Temperature → Vision → Spawning → Inventory → SpatialUpdate`).
- **`AppState` → `Screen` wiring** (all `OnEnter`): `MainMenu → Screen::MainMenu`, `DataLoading → Screen::DevWorldgen`, `WorldGen → Screen::DevWorldgen`, `InGame → Screen::Gameplay`. `OnEnter(InGame)` also spawns the dev world. `load_data_system` runs on `Update` while in `DataLoading`; `worldgen_system` runs on `Update` while in `WorldGen`.
- **Dev tooling is unconditional.** `run()` always adds `bevy_egui::EguiPlugin` and `WorldInspectorPlugin` before `CddaPlugin`. There is no flag to strip them.
- **Turn tick.** `tick_move_points` is gated on `AppState::InGame` and `on_timer(Duration::from_millis(100))`; placed in `SimSet::TurnTick`.

## Work Guidance
- Keep app code thin. New behavior belongs in a focused subsystem crate; this crate only wires and configures.
- `startup.rs` stays a data-loading + dev-spawn module. Do not move `App::new()` here.
- When adding a state, update both the `OnEnter(AppState::X) → Screen::Y` block in `CddaPlugin::build` and the screen's own entry in `cdda_context` / `cdda_render`.
- Replay-related work goes through `CddaStartupConfig`; do not read CLI args from subsystem crates.
- The `assets/gfx` symlink resolves to `../../../gfx`. Do not copy tilesets into this crate — the symlink is the contract.
- Current `startup.rs` state reflects the most recent edit: `build_terrain_registry` now takes `_def_world: &DefinitionWorld` (unused, prefixed to silence warnings) and the unused `use std::sync::Arc;` was removed.

## Verification
- `cargo check -p cdda_app` for compile sanity.
- `cargo nextest run -p cdda_app` (fall back to `cargo test -p cdda_app` if `nextest` is unavailable).
- `cargo run -p cdda_app -- dump` (schedule graph) and `cargo run -p cdda_app -- render-graph` (render graph) to validate wiring.
- `cargo run -p cdda_app` for live startup validation when wiring changes.

## Child DOX Index
None. Binary crate; no durable sub-folders.
