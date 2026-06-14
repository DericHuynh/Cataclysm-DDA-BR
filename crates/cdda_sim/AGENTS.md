# cdda_sim DOX

## Purpose
Owns the simulation-layer state machine, in-game clock, and the canonical headless test harness used by every other crate's tests.

## Ownership
- `src/state.rs` — `AppState` (`States`), `TurnState` (`Resource`), `GameTime` (re-export), `LoadingStatus`, `StartupConfig`.
- `src/test_utils.rs` — `TestBed` plus the `register_all_def_components` / `register_gameplay_components` batch registrars.
- Shared `GameSet` / `SimSet` schedule labels and all ECS components live in `cdda_components`, not here. The `sim` submodule of `cdda_components` owns `WorldPosition`, `Solid`, `Velocity`, `InFlight`, and the source-of-truth definition of `GameTime`; `cdda_sim` re-exports `GameTime` only.

## Local Contracts
- **Headless dependency boundary.** This crate uses `bevy_ecs` + `bevy_state` only — no full `bevy`, no renderer, no platform plugins. The same is true for `TestBed`; tests under `crates/*/tests/` and `tests/` must not need anything `TestBed` does not provide.
- **`AppState` (`src/state.rs`)** is the top-level lifecycle `States` enum. Variants: `MainMenu` (default), `DataLoading`, `WorldGen`, `InGame`, `Paused`, `GameOver`. Drivers: `cdda_app::CddaPlugin` calls `init_state::<AppState>()`; `cdda_context::nav` sets transitions (`StartNewGame → DataLoading`, `SaveAndQuit → MainMenu`).
- **`TurnState` (`src/state.rs`)** is a `Resource` (not a `States` enum) with `WaitingForInput | PlayerActed | Simulating | Animating`. The main tick system checks it to pick sub-systems.
- **`GameTime` is re-exported, not defined here.** Canonical import path is `cdda_sim::state::GameTime` (used by `cdda_actor` tests, `cdda_app`, and `tests/calendar_test.rs`). Do not import it from `cdda_components::sim` directly in consumer code.
- **`LoadingStatus` / `StartupConfig`** are `Resource`s. `StartupConfig` is built by the pre-game UI flow and consumed by `cdda_app::startup::load_data_system`; defaults: `data/core`, scenario `evacuee`, profession `unemployed`, world name `New World`, seed `0`.
- **`TestBed` (`src/test_utils.rs`)** is the workspace-wide test harness. Contract:
  - `TestBed::new()` / `TestBed::default()` build a fresh `bevy_ecs::world::World` (no plugins, no schedules).
  - Surface: `world()`, `world_mut()`, `spawn(bundle)`, `get::<C>(e)`, `resource::<R>()`, `resource_mut::<R>()`, `insert_resource(r)`, `register::<C>()`, `add_message::<M>()`, `run_system(sys)`.
  - `run_system` calls `initialize` → `run` → `apply_deferred`, so `Commands` queues flush inside one call. For multi-system tests, call `run_system` per system in order.
  - Two static batch registrars take `&mut World`: `TestBed::register_all_def_components` (every component in `cdda_components::def`) and `TestBed::register_gameplay_components` (world/actor/item/relationship components). Call them once per test before spawning components from those groups.
  - Consumer crates: `cdda_actor/tests/`, `cdda_combat/tests/`, `tests/`. Adding a new component to `cdda_components` that tests need usually means adding it to one of the two batch registrars.

## Work Guidance
- Keep the `cdda_sim` dep set minimal: `bevy_ecs`, `bevy_state`, `cdda_core_types`, `cdda_components`, `cdda_actor`. If you stop using one (currently `cdda_core_types` and `cdda_actor` are declared but have no `use` in `src/`), either remove it from `Cargo.toml` or document the forward-looking reason here.
- New top-level states belong on `AppState`. New per-tick phases belong in `cdda_components::schedule::SimSet`, not here. Do not add a new state enum for a sub-flow that fits `TurnState`.
- `TestBed` API is a stable contract. Treat method names, signatures, and the `register_all_def_components` / `register_gameplay_components` component lists as public — changing them breaks every crate's integration tests.
- Use `cdda_sim::state::GameTime` (the re-export) in test code and app code. Only `state.rs` should ever need `use cdda_components::sim::GameTime`.
- Write tests that exercise state transitions with `NextState::<AppState>` in `TestBed`-style apps; do not pull in `bevy::app::App` from a unit test in this crate.

## Verification
- `cargo check -p cdda_sim` — compile sanity for the state and harness changes.
- `cargo test -p cdda_sim` — runs the three in-crate `TestBed` smoke tests (`test_bed_spawns_entity`, `test_bed_runs_system`, `test_world_can_query`).
- `cargo nextest run --workspace` (or `cargo test --workspace` if `nextest` is unavailable) to confirm every downstream crate's `TestBed`-based tests still pass after any harness change. `tests/AGENTS.md` states the same preference.

## Child DOX Index
- `crates/cdda_sim/src/state.rs` — `AppState`, `TurnState`, `GameTime` (re-export), `LoadingStatus`, `StartupConfig`.
- `crates/cdda_sim/src/test_utils.rs` — `TestBed` and the two `register_*_components` batch registrars.
