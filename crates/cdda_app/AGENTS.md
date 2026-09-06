# cdda_app DOX

## Purpose
Leaf graphical application: constructs App, installs headless simulation and presentation/world/data adapters, and drives startup. No crate depends on cdda_app.

## Ownership
- `src/lib.rs` — CddaPlugin, run(), CddaStartupConfig, dev movement/camera and overmap adapters.
- `src/main.rs` — CLI entry and dump/schedule/render-graph tools (their own App construction).
- `src/startup.rs` — registry validation/publication, terrain/region configuration and dev-world spawning.
- `src/loading.rs` — background disk/JSON worker, bounded report consumption, frame-separated publication, cancellation/retry and terminal reporting.
- `src/preferences.rs` — display preference file adapter; `src/menu_pause.rs` preserves/restores simulation pause.
- `src/data_assets.rs` — named cdda asset source, watched file discovery and hot-reload publication.
- `examples/menu_capture.rs` — Offscreen production menu/settings/loading/error screenshot fixture, with viewport, UI-scale and theme-index arguments; requires a GPU but no OS window. Installs the production UiPresentationPlugin. `settings-tabs` saves 24 consecutive frames across four tab changes as PATH-FRAME.png to expose transient missing/fallback text.
- `assets/` — runtime fonts/core pack; gfx symlink points to repo gfx. Do not copy shared tilesets.

## Local Contracts
- Display preferences persist independently of UI interaction state; config/interface.json (CDDA_CONFIG_DIR override) uses temporary-write/rename. Unreadable configuration is preserved, with report warnings.
- ReportEvent/OperationReport is the shared terminal/UI reporting contract. Ctx::Loading owns modal controls; failed loads cannot proceed into a game without definitions. See docs/menus-and-operation-reporting.md.
- **Canonical simulation:** install `cdda_sim::runtime::SimulationPlugin` once. It owns gameplay resources/plugins, SimulationTurn phases, and the outer Input → Sim driver → Render chain. Do not duplicate gameplay systems or AP/effect/activity timers in this crate's Update.
- **World extension:** spatial sync is registered in SimulationRefresh/SimSet::SpatialUpdate after commits, including commands using spare player moves without a world tick, plus removal observers. Camera sync and overlays run in outer GameSet::Render. Input adapters run before the simulation driver in GameSet::Input.
- **Time/pause:** SimulationControl defaults to turn-based waiting for actions/activities. AppState other than InGame stops logical turns through the central gate. Optional manual/realtime pacing is a driver setting; a logical turn is always one game second.
- **Dev movement:** ordinary Move is one world tile, never 24 tiles for a single walk cost. The dev viewport still displays OMT cells and moves its camera only at OMT boundaries. Local-map rendering is separate pending work.
- **Input debt:** dev_pickup_drop_system and inventory_screen_input are authored in cdda_render and registered here before simulation. Pickup/Wield/Drop/Stow submit intents through the shared validating boundary. Item-examine input also belongs to cdda_render and submits ResumeCraft; startup.rs performs no item-examine mutation. Dev-spawn and legacy pending-craft translation remain separate adapters. Crafting input and CraftState/CategoryIndex/InventoryFocus resources are owned by CddaRenderPlugin; simulation handles pending work on its own schedule.
- **Startup states:** MainMenu → main-menu screen; DataLoading/WorldGen → modal Loading context and illustrated progress screen; InGame → Gameplay plus spawn_dev_world. Disk/JSON work runs on a worker in DataLoading; validated ECS publication yields between stages. Fatal errors retain diagnostics without advancing. Worldgen in WorldGen reports missing prerequisites as failures.
- **Data publication:** apply_registry_to_world returns bool. Build and validate candidate TerrainRegistry BEFORE destructive definition rebuilding. On reload, preserve existing numeric terrain slots through fallible rebuild_from; removals/invalid links reject the whole apply and retain old resources. Hot-reload success logging is conditional on true. This does not yet migrate all non-terrain definition Entity references.
- **Assets:** run() registers the cdda source rooted at ../../data before DefaultPlugins. request_data_files discovers StartupConfig.data_dirs JSON (skipping modinfo/mod_tileset) and keeps strong handles. reload_modified_data runs in outer Input while InGame, re-resolves watched values and calls apply_registry_to_world. Initial load uses Loader::load_reported on a worker, without duplicate ingestion. Watcher discovery and main-thread publication/worldgen stages remain synchronous.
- **Startup config:** world_seed, replay_file and record_session choose replay/recording/normal mode. Default seed is epoch seconds. Replay still needs canonical semantic-command integration; do not equate the existing log with proven determinism.
- Full Bevy rendering/window features live here. run() installs egui; the World Inspector runs only in Ctx::Gameplay, keeping menus and loading unobstructed.

## Work Guidance
- Keep the shell thin: gameplay goes in cdda_sim; input/rendering in presenter crates; world/data operations in their owners.
- New lifecycle states require matching screen transitions and context handling.
- Maintain previous validated data on rejected reload; never reinterpret existing terrain handles against fresh random-order slots.

## Verification
- `cargo check -p cdda_app`.
- `cargo nextest run -p cdda_sim --test simulation_schedule_test` tests the shared runtime headlessly.
- `cargo nextest run --workspace --exclude cdda_app`; app default dynamic-linked test binaries may require platform loader setup. Use cargo test fallback only when nextest unavailable.
- `cargo run -p cdda_app` for GUI smoke checks; compile/headless tests do not establish visual correctness.

## Child DOX Index
None (flat source module ownership above).
