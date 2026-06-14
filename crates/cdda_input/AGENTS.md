# cdda_input DOX

## Purpose
Decoupled input plugin: bridges leafwing `ActionState<BindableAction>` and
raw `KeyboardInput` into semantic `InputAction` Bevy messages, manages
context-keyed `InputMap`s, and supports runtime rebinding.

## Ownership
- `CddaInputPlugin` (`src/lib.rs`), `RebindCapture` resource, `GlobalInputEntity` marker.
- Per-context + global `InputMap<BindableAction>` tables (`ContextInputMaps`).
- Default bindings, key-label formatting, and the `ActiveKeybindings` resource that
  UI crates read for dynamic key hints.
- Canonical action enums (`Direction`, `GameAction`, `ActionSource`, `InputAction`,
  `BindableAction`, `InputContextId`, `InputContextStack`) live in
  `cdda_components::input`; this crate re-exports them from `actions.rs` and
  `context.rs` so downstream crates do not need a direct `cdda_components` import.

## Local Contracts
- Bevy deps in `Cargo.toml` are `bevy_ecs`, `bevy_app`, `bevy_input` (feature `keyboard`),
  `bevy_reflect`, plus `cdda_core_types`, `cdda_components`, `leafwing-input-manager`,
  `serde`, `tracing`. No full-`bevy` dependency.
- **Two-layer action model.** `BindableAction` (flat, unit-only, `Actionlike`) is the
  leafwing `InputMap`/`ActionState` key. `GameAction` is the rich, data-carrying
  message type. Conversion happens in `BindableAction::to_game_action()` in
  `cdda_components::input` and is invoked by `bridge_actionstate`.
- **Single decoupling point: `InputAction` messages.** Downstream systems must read
  `MessageReader<InputAction>` and never `ButtonInput<KeyCode>` directly. Two
  producers feed this stream: `bridge_actionstate` (Update, leafwing) and
  `handle_raw_input` (PreUpdate, rebind-cancel + `TextInput` context only).
- **Pipeline order in `CddaInputPlugin::build`.**
  1. `add_plugins(InputManagerPlugin::<BindableAction>)` — leafwing runs in PreUpdate.
  2. Startup: `spawn_global_input_entity` seeds the `GlobalInputEntity` with the
     `merged_for(MainMenu)` map.
  3. PreUpdate: `handle_raw_input` runs before leafwing sees events so it can intercept
     rebind-capture and `TextInput` keys.
  4. Update: `(bridge_actionstate, clear_rebind_flag).chain()` — `clear_rebind_flag`
     must run after the bridge to consume `just_captured`.
  5. Update: `sync_leafwing_input_map` then `refresh_active_keybindings`.
- **Rebind capture flow.** Set `RebindCapture::pending = Some(RebindCaptureInner { context, action })`.
  `handle_raw_input` captures the next non-modifier key, calls
  `ContextInputMaps::rebind`, refreshes the live `InputMap` on `GlobalInputEntity`,
  and sets `just_captured = true` for one Update frame to prevent the bridge from
  dispatching the captured key as a normal action. `Escape` cancels and emits
  `GameAction::Cancel`.
- **Context sync.** `sync_leafwing_input_map` reacts to changes on
  `InputContextStack` or `ContextInputMaps` and rewrites the live
  `InputMap<BindableAction>` on `GlobalInputEntity` from `merged_for(&top)`. Global
  bindings are included in every merged map. `bridge_actionstate` skips dispatch
  while the active context is `TextInput` (handled by `handle_raw_input`).
- **UI key hints.** UI crates must read `ActiveKeybindings::key_for(action)` rather
  than hardcoding `[w]`/`[d]`. The resource is rebuilt by
  `refresh_active_keybindings` whenever context or maps change.

## Work Guidance
- Add or change bindings in `default_bindings()` in `src/bindings.rs`. The same map
  must be re-registered for every new `InputContextId` (the 15 contexts currently
  used are listed in `default_bindings`).
- To add a new bindable action: extend `BindableAction` in `cdda_components::input`
  with a variant + label + `to_game_action` arm, then bind keys in the relevant
  context(s). Data-carrying `GameAction` variants (e.g. `TextChar`, `Move(Direction)`)
  must NOT appear in `BindableAction` or in an `InputMap`.
- Rebinding for new contexts: register the context in `default_bindings`, then
  trigger via `RebindCapture`. The rebind path writes to `ContextInputMaps` and
  refreshes the live `InputMap` itself; do not duplicate that logic.
- Display labels for non-US keys: extend the `match` in `format_wrapper` in
  `src/bindings.rs`.

## Verification
- `cargo check -p cdda_input` for compile sanity.
- `cargo nextest run -p cdda_input` — the four unit tests in `src/bindings.rs::tests` cover gameplay bindings, global hotkeys, context presence, and `merged_for` including globals (fall back to `cargo test -p cdda_input` if `nextest` is unavailable).

## Child DOX Index

- `src/lib.rs` — `CddaInputPlugin` and the system-ordering contract.
- `src/actions.rs` — Re-exports of `Direction`, `GameAction`, `ActionSource`,
  `InputAction`, `BindableAction` from `cdda_components::input`; two-layer model.
- `src/bindings.rs` — `ContextInputMaps` (per-context + global `InputMap`),
  `ActiveKeybindings`, `default_bindings()`, `format_wrapper`.
- `src/context.rs` — Re-exports of `InputContextId` and `InputContextStack`.
- `src/systems.rs` — `RebindCapture`/`RebindCaptureInner`, `GlobalInputEntity`,
  `handle_raw_input`, `bridge_actionstate`, `clear_rebind_flag`,
  `sync_leafwing_input_map`, `refresh_active_keybindings`.
