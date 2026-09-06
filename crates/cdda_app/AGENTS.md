# cdda_app DOX

## Purpose
Leaf graphical application: constructs App, installs headless simulation and presentation/world/data adapters, and drives startup. No crate depends on cdda_app.

## Ownership
- `src/lib.rs` — CddaPlugin, run(), CddaStartupConfig, dev movement/camera and overmap adapters.
- `src/main.rs` — CLI entry and dump/schedule/render-graph tools (their own App construction).
- `src/startup.rs` — load_data_system, apply_registry_to_world, terrain/region configuration and dev-world spawning. No App construction.
- `src/data_assets.rs` — named cdda asset source, watched file discovery and hot-reload publication.
- `assets/` — runtime fonts/core pack; gfx symlink points to repo gfx. Do not copy shared tilesets.

## Local Contracts
- **Canonical simulation:** install `cdda_sim::runtime::SimulationPlugin` once. It owns gameplay resources/plugins, SimulationTurn phases, and the outer Input → Sim driver → Render chain. Do not duplicate gameplay systems or AP/effect/activity timers in this crate's Update.
- **World extension:** spatial sync is registered in SimulationTurn/SimSet::SpatialUpdate, plus removal observers. Camera sync and overlays run in outer GameSet::Render. Input adapters run before the simulation driver in GameSet::Input.
- **Time/pause:** SimulationControl defaults to turn-based waiting for actions/activities. AppState other than InGame stops logical turns through the central gate. Optional manual/realtime pacing is a driver setting; a logical turn is always one game second.
- **Dev movement:** ordinary Move is one world tile, never 24 tiles for a single walk cost. The dev viewport still displays OMT cells and moves its camera only at OMT boundaries. Local-map rendering is separate pending work.
- **Input debt:** dev_pickup_drop_system and inventory_screen_input are authored in cdda_render and registered here before simulation. They still have legacy mutation/AP bypasses to consolidate. Crafting input is registered by CddaRenderPlugin; simulation handles pending work on its own schedule.
- **Startup states:** MainMenu → main-menu screen; DataLoading/WorldGen → DevWorldgen screen; InGame → Gameplay plus spawn_dev_world. Disk loading runs in DataLoading, worldgen in WorldGen.
- **Data publication:** apply_registry_to_world returns bool. Build and validate candidate TerrainRegistry BEFORE destructive definition rebuilding. On reload, preserve existing numeric terrain slots through fallible rebuild_from; removals/invalid links reject the whole apply and retain old resources. Hot-reload success logging is conditional on true. This does not yet migrate all non-terrain definition Entity references.
- **Assets:** run() registers the cdda source rooted at ../../data before DefaultPlugins. request_data_files discovers StartupConfig.data_dirs JSON (skipping modinfo/mod_tileset) and keeps strong handles. reload_modified_data runs in outer Input while InGame, re-resolves watched values and calls apply_registry_to_world. Initial load remains disk-backed.
- **Startup config:** world_seed, replay_file and record_session choose replay/recording/normal mode. Default seed is epoch seconds. Replay still needs canonical semantic-command integration; do not equate the existing log with proven determinism.
- Full Bevy rendering/window features live here; default dynamic feature enables Bevy dynamic linking. run() currently always installs egui and inspector dev tooling.

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
