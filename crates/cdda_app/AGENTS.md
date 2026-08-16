# cdda_app DOX

## Purpose
Binary entry point for `cdda` — owns `App::new()` construction, full-Bevy plugin
wiring, `AppState` machine, and runtime startup. Layer 5 leaf crate (see
`crates/AGENTS.md`); nothing may depend on it.

## Ownership
- `src/main.rs` — `clap` CLI: subcommands `run` (default), `schedule-graph`, `render-graph`, `dump`. Dump subcommands build their own `App::new()` and call `CddaPlugin`.
- `src/lib.rs` — `CddaStartupConfig` resource, `CddaPlugin`, `dev_player_move`, `toggle_overmap`, and `run()` (the live-game `App::new()`).
- `src/startup.rs` — data loading + dev-world spawn. No `App::new()` here. `load_data_system` (legacy disk path) and the shared `apply_registry_to_world` (builds the def-world + all dependent resources from a resolved `DefRegistry`).
- `src/data_assets.rs` — asset-driven data loading. Registers the `"cdda"` asset source (rooted at repo `data/`), discovers `data_dirs` `.json` files with `std::fs`, loads each as a `CddaJsonFile` asset, and hot-reloads definitions on `AssetEvent`.
- `assets/` — runtime asset root. Contains `core.pack`, `fonts/Inter-VariableFont.ttf`, `fonts/ShareTechMono-Regular.ttf`, and `gfx` (symlink → `../../../gfx`, the repo's shared tileset tree).

## Local Contracts
- **Bevy deps** (`Cargo.toml`): full `bevy` (with `3d`), `bevy_ecs`, `bevy_state` 0.18, `bevy_egui` 0.39, `bevy-inspector-egui`, `bevy_mod_debugdump` 0.15, plus `cdda_components`, `cdda_context`, `cdda_core_types`, `cdda_data`, `cdda_defs_raw`, `cdda_events`, `cdda_input`, `cdda_overmap`, `cdda_overmap_gen`, `cdda_render`, `cdda_replay`, and `cdda_sim`. `default = ["dynamic"]` and `dynamic = ["bevy/dynamic_linking"]` — the binary always dynamically links Bevy in dev.
- **Where `App::new()` lives.** `lib.rs::run()` for the live game; `main.rs` for the three dump subcommands. `CddaPlugin::build` is the subsystem registration site — *not* `startup.rs`. `startup.rs` is data-loading + dev-spawn only.
- **`CddaStartupConfig`** (`lib.rs`): `world_seed: u64`, `replay_file: Option<String>`, `record_session: bool`. Three modes wired in `CddaPlugin::build`:
  - `replay_file = Some(path)` → `SessionLog::load_compressed` + `CddaReplayModePlugin`.
  - `record_session = true` → `CddaReplayPlugin { world_seed }`.
  - else → default seed = UNIX-epoch seconds, no replay.
- **Plugin registration order** (`CddaPlugin::build`): `ActivityPlugin`, `ActorPlugin`, `ItemPlugin`, `CddaAssetsPlugin`, `CraftingPlugin`, `CddaDataPlugin`, then `OvermapGenPlugin`. The `Update` schedule uses `(GameSet::Input, GameSet::Sim, GameSet::Render).chain()`; `SimSet` is chained inside `GameSet::Sim` (`TurnTick → Activity → Ai → Movement → Combat → Effects → Healing → Bionics → Morale → Temperature → Vision → Spawning → Inventory → SpatialUpdate`).
- **Screen input adapters register in `cdda_render`, dispatched here.** `dev_pickup_drop_system` and `inventory_screen_input` (from `cdda_render::render::input`) are added to `SimSet::Inventory` in `CddaPlugin::build` so they run inside the sim's inventory phase, but they are *authored* in the presenter layer. `crafting_menu_input` is registered by `CddaRenderPlugin` against `Screen::CraftingMenu`.
- **`AppState` → `Screen` wiring** (all `OnEnter`): `MainMenu → Screen::MainMenu`, `DataLoading → Screen::DevWorldgen`, `WorldGen → Screen::DevWorldgen`, `InGame → Screen::Gameplay`. `OnEnter(InGame)` also spawns the dev world. `load_data_system` runs on `Update` while in `DataLoading`; `worldgen_system` runs on `Update` while in `WorldGen`.
- **Asset-driven data loading + hot reload.** `run()` registers a named `"cdda"` asset source (`AssetSourceBuilder::platform_default("../../data", None)`) BEFORE `DefaultPlugins` so the source is built at `AssetPlugin` startup. `OnEnter(DataLoading)` runs `request_data_files`, which `std::fs`-discovers every `.json` under `StartupConfig.data_dirs` (recursively, skipping `modinfo.json`/`mod_tileset.json`), loads each through `asset_server.load::<CddaJsonFile>(cdda://rel)` and keeps strong `Handle`s in the `CddaDataFiles` resource. `reload_modified_data` (exclusive system, `Update` while `InGame`) reacts to `AssetEvent::<CddaJsonFile>` `Modified`/`LoadedWithDependencies`, re-ingests the in-use files with `Loader::ingest_values` + `resolve`, and rebuilds everything via `apply_registry_to_world`. The initial definition build stays in `load_data_system`; `request_data_files` only seeds the watched-handle set.
- **Dev tooling is unconditional.** `run()` always adds `bevy_egui::EguiPlugin` and `WorldInspectorPlugin` before `CddaPlugin`. There is no flag to strip them.
- **Turn tick.** `tick_move_points` is gated on `AppState::InGame` and `on_timer(Duration::from_millis(100))`; placed in `SimSet::TurnTick`.

## Work Guidance
- Keep app code thin. New behavior belongs in a focused subsystem crate; this crate only wires and configures.
- `startup.rs` stays a data-loading + dev-spawn module. Do not move `App::new()` here.
- When adding a state, update both the `OnEnter(AppState::X) → Screen::Y` block in `CddaPlugin::build` and the screen's own entry in `cdda_context` / `cdda_render`.
- Replay-related work goes through `CddaStartupConfig`; do not read CLI args from subsystem crates.
- The `assets/gfx` symlink resolves to `../../../gfx`. Do not copy tilesets into this crate — the symlink is the contract.

## Verification
- `cargo check -p cdda_app` for compile sanity.
- `cargo nextest run -p cdda_app` (fall back to `cargo test -p cdda_app` if `nextest` is unavailable).
- `cargo run -p cdda_app -- dump` (schedule graph) and `cargo run -p cdda_app -- render-graph` (render graph) to validate wiring.
- `cargo run -p cdda_app` for live startup validation when wiring changes.

## Child DOX Index
None. Binary crate; no durable sub-folders.
