# cdda_combat

## Purpose
Layer 3 game-logic crate for combat resolution: hit/miss formulas, damage calculation, melee and ranged attack dispatch. The current `src/systems.rs` is largely `todo!()` stubs; the contract below documents what exists now and the planned fill-in.

## Ownership
- Bevy deps: `bevy_ecs`, `bevy_reflect`, `bevy_app` (per `Cargo.toml`; `bevy_reflect` is declared but not yet used in source).
- Workspace deps: `cdda_core_types`, `cdda_components`.
- Dev-deps: `cdda_sim` (for `TestBed`), `cdda_data` (for `ItemFlagList`).
- Source files: `src/lib.rs`, `src/mod.rs`, `src/plugin.rs`, `src/systems.rs`.
- `src/lib.rs` and `src/mod.rs` both declare `pub mod plugin;` and `pub mod systems;`. The dual layout is undecided; keep them in sync until one is removed.
- Wired into the app at `crates/cdda_app/src/lib.rs` via `use cdda_combat::systems::combat_phase;` and `combat_phase.in_set(SimSet::Combat)`. `CombatPlugin` in `src/plugin.rs` is a no-op (`let _ = combat_phase;`) and is not currently registered.

## Local Contracts
- `systems::combat_phase(&mut World)` is the Bevy system entry; it calls `melee_combat_phase` then `ranged_combat_phase`. It must be added to `SimSet::Combat` from `cdda_components::schedule`.
- Damage uses `cdda_core_types::core::Damage` with `DefId<DamageTypeDef>` keys (`bash`, `cut`, `stab`, `bullet`, …). New damage types: add the marker to `cdda_core_types::damage`, register in `cdda_data::def_kinds`, then mitigate in `apply_damage_to_target` once implemented. Do not pass raw `f32` damage across crate boundaries.
- Pure formulas in `src/systems.rs`:
  - `calculate_melee_hit_chance(attacker_stats, weapon_to_hit, defender_dodge) -> f32`, clamped to `[0.05, 0.95]`.
  - `calculate_melee_damage(weapon, stats, skill_level) -> Damage` — fixed `damage_bash/cut/stab` + dice avg roll + `strength * 0.5` + `skill_level * 0.25` (all as bash).
  - `calculate_ranged_hit_chance(gun, ammo, distance, shooter_skill) -> f32`, clamped to `[0.05, 0.98]`.
- Components consumed: `cdda_components::actor::{CombatStats, DamageReduction, Health, IsAlive, Creature, Vision}`, `cdda_components::def::{WeaponData, GunData, AmmoData}`, `cdda_components::stats::Stats`, `cdda_core_types::core::coords::WorldPos`.
- Stubs (still `todo!()`): `apply_damage_to_target`, `check_and_handle_death`, `resolve_melee_attack`, `resolve_ranged_attack`, `melee_combat_phase`, `ranged_combat_phase`.
- Planned event emission: `DamageEvent` and `DeathEvent` from `cdda_events` (crate not yet a dep), plus `SoundEvent` from `cdda_components::events`. Do not call into `cdda_actor` directly from combat systems; emit events.
- Planned RNG: use `cdda_core_types::rng::SeededRng` for hit/miss rolls. No `rand::thread_rng()`.

## Work Guidance
- Add new weapons, guns, or ammo via `cdda_data` definitions; combat only reads the typed components.
- When implementing a stub, replace the `let _ = …; todo!(...)` body, then un-`#[ignore]` the matching test in `tests/combat_system_test.rs` (each is `#[ignore = "combat system not yet implemented"]`).
- Keep pure formulas free of `&mut World` so unit tests can exercise them without Bevy.
- Keep `src/lib.rs` and `src/mod.rs` module lists identical until the dual layout is resolved.

## Verification
- `cargo nextest run -p cdda_combat` (or `cargo test -p cdda_combat`) for the full suite. All non-`#[ignore]` tests should pass.
- `tests/combat_test.rs` and `tests/melee_test.rs` are live: `CombatStats`, `DamageReduction`, `Vision`, `Creature`, `Damage`, and `WeaponData` value tests + pure formula tests.
- `tests/combat_system_test.rs` (15 tests) is all `#[ignore = "combat system not yet implemented"]`. To run: `cargo nextest run -p cdda_combat -- --ignored`. Passing these is the sign a phase is done.
- `cargo check -p cdda_combat` after editing `Cargo.toml` deps.

## Child DOX Index
- `src/systems.rs` — Types (`CombatResult`, `MeleeIntent`), pure formulas, stubs, and phase orchestrators (`melee_combat_phase`, `ranged_combat_phase`, `combat_phase`).
- `src/plugin.rs` — `CombatPlugin` (no-op `build`).
- `src/lib.rs` — Public crate entry; declares `pub mod plugin; pub mod systems;`.
- `src/mod.rs` — Mirror of `src/lib.rs` module declarations (dual-file layout).
- `tests/combat_system_test.rs` — 15 `#[ignore]`d system-level tests exercising `combat_phase` with `TestBed`.
- `tests/combat_test.rs` — `CombatStats`/`DamageReduction`/`Vision`/`Creature`/`Damage` value tests with `TestBed`.
- `tests/melee_test.rs` — `WeaponData` component construction and pure formula tests (avg damage, to-hit, moves, reach, crit).
