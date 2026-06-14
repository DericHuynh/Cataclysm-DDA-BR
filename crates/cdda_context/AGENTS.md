# cdda_context DOX

## Purpose
Headless context state machine for the Bevy ECS app shell. Owns the active
`Ctx` (Bevy `States` enum), the `ContextStack` parent/child navigation,
`OverlayStack` modal layer, per-screen focus tracking, the menu/examine-cursor
components, and the `CddaScreen` registration trait. Has no dependency on
`bevy_render`, `bevy_sprite`, or `bevy_window` — visual UI is the job of
`cdda_render`, which subscribes to `Ctx` transitions via `OnEnter` / `OnExit`.

## Ownership
- Bevy deps: `bevy_ecs`, `bevy_app`, `bevy_state` (plus workspace `tracing`).
  No full `bevy` — kept headless so the suite runs without a renderer.
- Crate deps: `cdda_core_types`, `cdda_components`, `cdda_events`,
  `cdda_sim`. `Ctx`, `ContextStack`, `FocusedCommandIndex`, `push_ctx`,
  `pop_ctx` are defined in `cdda_components::context` and re-exported from
  this crate's `ctx.rs` / `nav.rs`.
- Current layering note: `cdda_context` depends on `cdda_sim::runtime::state::AppState` for state transitions. This is a layering debt; future work should move the shared app-state enum into a lower-level crate or `cdda_components`.
- Modules: `actions.rs`, `config.rs`, `ctx.rs`, `cursor.rs`, `focus.rs`,
  `menu.rs`, `nav.rs`, `overlay.rs`, `screen.rs`, `systems.rs`. All are flat,
  no durable sub-folders.

## Local Contracts
- **`ContextStack` (not `NavStack`)** — `Resource<Vec<Ctx>>` from
  `cdda_components::context`. `push_ctx` / `pop_ctx` are the only mutators;
  they save/restore focus via `FocusedCommandIndex` and set `NextState<Ctx>`.
- **`Ctx` States enum** — defined in `cdda_components::context`, default
  `MainMenu`. Covers menu, character/world creation, gameplay, in-game panels
  (`Inventory`, `CraftingMenu`, `CharacterSheet`, `PauseMenu`, `ExamineLook`,
  `Overmap`, …), input prompts (`TextInput`, `QuantityInput`,
  `DirectionSelect`), debug panels (`DevSpawnPanel`, `RegistryViewer`),
  `DevWorldgen`, and `Custom(u32)` as an extensibility hatch.
- **Focus is split across two crates**:
  - `InputFocus` (this crate, `focus.rs`) — `Resource<Option<Entity>>` for
    entity-level keyboard focus across `KeyboardFocusable` components.
    Hand-rolled to avoid `bevy_input_focus` which may pull in `bevy_window`/
    `bevy_render` and would break the headless invariant.
  - `FocusedCommandIndex` (`cdda_components::context`, re-exported) — per-
    screen `usize` cursor with `HashMap<Ctx, usize>` history; `on_push` saves
    the old screen's focus, `on_pop` restores the parent's saved focus.
- **`CddaScreen` trait** (`screen.rs`) — implementors declare
  `const CTX: Ctx`, `const ACTIONS: &'static [(&str, BindableAction)]`,
  `fn spawn(&mut World)`, and optional `fn update(&mut World)`. Register via
  `app.add_plugins(Screen::<S>::default())`, which wires
  `OnEnter(CTX) → populate_actions + spawn_screen` and
  `Update → update_screen.run_if(in_state(CTX))`. Screens that spawn entities
  must use `DespawnOnExit` on the root (or call `despawn_recursive()`) — the
  trait does not auto-clean.
- `OverlayStack::input_blocked` short-circuit — `push()` sets the flag
  true, `pop()` sets it to `!stack.is_empty()`. `handle_navigation_input`
  checks this flag first and only forwards `GameAction::Cancel`; the exclusive
  system `handle_overlay_cancel` drains the cancel message and calls
  `pop_overlay`. `Overlay::ActivityProgress` is also auto-managed by
  `sync_activity_overlay` / `cleanup_activity_overlay` reacting to
  `PlayerActivity` from `cdda_sim::activity`.
- **`ContextPlugin`** (`lib.rs`) is the single Bevy `Plugin` for this crate.
  In `build` it: `init_state::<Ctx>()`; inserts resources
  (`ContextActions`, `OverlayStack`, `ContextStack`, `FocusedCommandIndex`,
  `InputFocus`, `ExamineCursor`, `GameSettings`, `CharacterCreationState`,
  `WorldCreationSettings`); and adds `Update` systems
  `(handle_navigation_input, handle_panel_openers)`, `sync_input_context`,
  and `(menu_navigation, ctx_and_cursor)`. Ordering vs. the input bridge is
  guaranteed by `GameSet` labels in the parent app's schedule — no
  `.after()` chain lives here.
- Renderers must read these resources / observe `OnEnter`/`OnExit`; they must
  not mutate `ContextStack` or `NextState<Ctx>` directly. The only way in or
  out of a screen is a `TransitionTarget` (`Push` / `Replace` / `Pop` /
  `Quit` / `Event`) dispatched by `handle_navigation_input` /
  `handle_panel_openers`.

## Work Guidance
- `TransitionTarget::Quit` calls `std::process::exit(0)` in `dispatch` — do
  not add new quit paths without an integration test in
  `tests/screen_integration_test.rs` (the workspace-level wiring test).
- `MenuItem::enabled: bool` carries a TODO to convert to `Enabled` / `Disabled`
  tag components for archetype query parity. New bool flags on UI components
  are not accepted.
- `Ctx::ItemExamine` handles its own `Cancel` — `handle_navigation_input`
  deliberately skips the auto-pop for it. Add new "self-handled cancel"
  cases by extending that match, not by duplicating pop logic.
- `Ctx::Custom(1)` / `Ctx::Custom(2)` are wired to the debug spawn panel
  and registry viewer (F2 / F3) in `handle_panel_openers`. Reserve new
  `Custom` IDs deliberately — they collide.
- `sync_input_context` maps every `Ctx` variant to one `InputContextId` so
  keybindings follow the active screen. Any new `Ctx` variant must add a
  match arm here or it will fail to compile.

## Verification
- `cargo check -p cdda_context` for compile sanity.
- `cargo nextest run -p cdda_context` for this crate's unit/integration
  tests. The four integration tests under `crates/cdda_context/tests/` are
  pure-`cdda_context` (no Bevy `App`, no `TestBed`) and cover:
  - `nav_test.rs` — `ContextStack` push/pop and focus restore.
  - `ctx_def_test.rs` — per-screen `ScreenDefinition` contents and
    `TransitionTarget` shapes.
  - `focused_index_test.rs` — `FocusedCommandIndex` save/load semantics.
  - `config_test.rs` — `GameSettings`, `CharacterCreationState`,
    `WorldCreationSettings` defaults and mutation.
  Run with `cargo nextest run -p cdda_context`.
- Cross-crate wiring still lives at the workspace level:
  `tests/screen_integration_test.rs` exercises `CddaScreen` plugin wiring,
  overlay blocking, and `ContextActions` lifecycle. Run with
  `cargo nextest run --test screen_integration_test`.
- `tests/AGENTS.md` is the workspace test conventions doc; tests must stay
  headless — no `cdda_render` or platform plugin imports here.

## Child DOX Index
No durable sub-folders; this crate's source is a single `src/` directory of
ten flat modules, and `crates/cdda_context/tests/` holds four flat
integration-test files co-located with the crate they exercise. No child
`AGENTS.md` files exist or are planned.
