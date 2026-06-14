# cdda_activity DOX

## Purpose
Multi-turn player activity system. Drives `PlayerActivity` components through
a `Pending` → `Active` → `Done` lifecycle each turn and tracks per-character
weariness / calorie balance via `ActivityTracker`.

## Ownership
- Owns: `PlayerActivity`, `ActivityTracker`, the `ActivityActor` enum, its
  seven concrete actor structs, the `ActorImpl` trait, `ActivityPlugin`, and
  the `CRAFT_COMPLETE_HOOK` global.
- Does not own: turn / action-point logic (Layer 3 `cdda_actor`),
  crafting rules (Layer 3 `cdda_crafting`), inventory or item components
  (Layer 2 `cdda_components`).
- Craft completion is reached through `CRAFT_COMPLETE_HOOK: OnceLock<…>` to
  avoid a `cdda_activity` ↔ `cdda_crafting` circular dependency. The hook is
  installed by `cdda_core` at startup; this crate never depends on
  `cdda_crafting` directly.

## Local Contracts
- Bevy deps: `bevy_ecs`, `bevy_reflect`, `bevy_app`, `bevy_state`, `serde`;
  path crates `cdda_core_types`, `cdda_components`, `cdda_actor`. No
  `cdda_crafting` dep.
- All systems register in `Update` inside `cdda_components::schedule::SimSet::Activity`,
  ordered: `start_pending_activities` → `tick_activities` →
  `cleanup_done_activities`. Runs between `SimSet::TurnTick` and
  `SimSet::Ai`.
- `ActivityPhase`: `Pending` (default; `start()` not yet called), `Active`
  (`do_turn()` runs each tick), `Suspended` (resumable), `Done` (terminal;
  component is removed by `finish_activity` or `cleanup_done_activities`).
- `ActorImpl` is the trait implemented by every concrete actor. `ActivityActor`
  is the enum-of-structs dispatching to `Idle`, `Aim`, `Read`, `Reload`,
  `Craft`, `Wait`, `Interact` via `start` / `do_turn` / `finish` / `canceled`.
- Actor methods receive `&mut World` plus extracted `moves_total` /
  `moves_left` fields (not `&mut PlayerActivity`) so the system can hold
  `&mut World` without aliasing the component.
- `PlayerActivity` is removed on completion in `finish_activity`; the
  `cleanup_done_activities` system is a safety net for activities left in
  `Done` by direct `phase` assignment.

## Work Guidance
- Mirrors the C++ `activity_actor` / `player_activity` / `activity_tracker`
  classes; keep field names and shapes aligned where practical.
- New activity kinds: add a struct implementing `ActorImpl`, a variant in
  `ActivityActor`, a string in `activity_type_id`, and four match arms (one
  per lifecycle method). For state that needs crafting or inventory
  interaction, route through a new `OnceLock` hook — not a direct dep.
- One-shot setup belongs in `start_pending_activities`, not inside the per-turn
  tick. The schedule ordering assumes `start` runs before `tick`.
- Do not reimplement weariness, calorie balance, or crafting completion
  locally — extend `ActivityTracker` and `CRAFT_COMPLETE_HOOK` instead.

## Verification
- `cargo check -p cdda_activity` for compile sanity.
- `cargo nextest run -p cdda_activity` (or `cargo test -p cdda_activity` if
  `nextest` is unavailable). This crate has no `tests/` directory yet; once
  crate-local tests are added, they live at
  `crates/cdda_activity/tests/`.

## Child DOX Index
- `src/lib.rs` — Crate root, `pub mod` re-exports, `CRAFT_COMPLETE_HOOK` once-cell.
- `src/actor.rs` — `ActivityActor` enum, `ActorImpl` trait, and the seven
  concrete actor structs (`IdleActor`, `AimActor`, `ReadActor`, `ReloadActor`,
  `CraftActor`, `WaitActor`, `InteractActor`).
- `src/components.rs` — `ActivityTypeId`, `ActivityPhase` enum, `PlayerActivity`
  component.
- `src/plugin.rs` — `ActivityPlugin` registering systems in `SimSet::Activity`.
- `src/systems.rs` — `start_pending_activities`, `tick_activities`, `tick_one`,
  `finish_activity`, `cleanup_done_activities`, `cancel_activity`.
- `src/tracker.rs` — `ActivityTracker` component and exertion-level constants
  (`NO_EXERCISE` … `EXTRA_EXERCISE`).
