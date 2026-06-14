# cdda_events DOX

## Purpose
Observer-based `Event` / `EntityEvent` types for **immediate, reactive**
decoupled communication (UI feedback, damage reactions, death handling).
Distinct from buffered `Message` types in `crates/cdda_components/src/events.rs`
(`ItemMoveEvent`, `SoundEvent`, `SightEvent`, `SpawnEvent`, `DefChangedEvent`).

## Ownership
- Sole source file: `crates/cdda_events/src/lib.rs`.
- `Cargo.toml` deps: `bevy_ecs` (workspace), `cdda_core_types`.
- Re-exported by `crates/cdda_components/src/events.rs`:
  `DamageEvent, DeathEvent, EquipEvent, UnequipEvent, UseItemEvent,
  DeathCause, GameEvent, MoveLocation`.
- Imported directly by `crates/cdda_context/src/nav.rs`: `GameEvent`,
  `GameEventDispatch`. `cdda_sim::combat::systems` is the planned emitter
  of `DamageEvent` / `DeathEvent` (see `check_and_handle_death`).

## Local Contracts
- Observer-based only — no `Message` derives in this crate.
- Global events use `#[derive(Event)]`; targeted events use
  `#[derive(EntityEvent)]` with the target field marked `#[event_target]`
  (Bevy 0.18 also auto-detects a field named `entity` or `target`).
- Types in `src/lib.rs`:
  - Enums: `DeathCause` (`Combat(Entity)`, `Hunger`, `Thirst`,
    `Asphyxiation`, `Bleeding`, `Fall`, `Other`);
    `MoveLocation` (`Ground(WorldPos)`, `Container(Entity)`,
    `Wielded(Entity)`, `Worn(Entity)`);
    `GameEvent` (`StartNewGame`, `SaveAndQuit` — the only global `Event`).
  - `EntityEvent` structs: `DamageEvent { target, damage: Damage,
    source: Option<Entity> }`, `DeathEvent { entity, cause: DeathCause,
    position: WorldPos }`, `EquipEvent { wielder, item }`,
    `UnequipEvent { wielder, item }`, `UseItemEvent { user, item }`.
  - Resource: `GameEventDispatch(pub GameEvent)` — declared but
    currently unused. `crates/cdda_context/src/nav.rs::dispatch` sets
    `NextState<AppState>` directly, never inserts the resource.
- Payloads describe what happened, not how to mutate state.

## Work Guidance
- New events go in `src/lib.rs`. Mark `EntityEvent` target fields with
  `#[event_target]`.
- Buffered / cross-frame messages belong in
  `crates/cdda_components/src/events.rs` (`#[derive(Message)]`), not
  here.
- When adding a new public event type, update the re-export list in
  `crates/cdda_components/src/events.rs` so downstream crates see it
  through the single import path.
- Trigger global events with `commands.trigger(e)`; target an
  `EntityEvent` with `commands.entity(t).trigger(e)`. Register handlers
  via `app.add_observer(...)`.

## Verification
- `cargo check -p cdda_events` for compile sanity.
- `cargo nextest run -p cdda_events` (or `cargo test -p cdda_events`).
  No tests exist yet under `crates/cdda_events/tests/` or in `src/lib.rs`
  — add unit tests when introducing new event types.
- `cargo check --workspace` to validate downstream re-exports
  (`cdda_components`, `cdda_context`) still compile.

## Child DOX Index
