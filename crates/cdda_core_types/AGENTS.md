# cdda_core_types

## Purpose
Pure value types for CDDA: coords, units, IDs, damage, flags, RNG, and the simulation id. Zero `bevy_ecs` dependency — this crate is a leaf that everything else depends on.

## Ownership
- All shared numeric/string value types live here so the rest of the workspace has a single source of truth.
- The 138 raw JSON def structs were moved to a separate `cdda_defs_raw` crate in Phase 3a; this crate is now exclusively value types.
- Bevy deps used: `bevy_ecs` (for `Resource`/`Component` derives only — no system code), `bevy_reflect`, plus `serde`, `schemars`, `rand`, `thiserror`.

## Local Contracts
- New value types that get used in more than one crate belong here. Crate-local types stay in their crate.
- Coordinates live under `core/coords/`. Each coordinate has a docstring explaining what space (world, submap, overmap) and scale (tile, OMT) it represents.
- `DefId<T>` wraps a `String` plus a phantom marker type. The marker types (`ItemDef`, `MonsterDef`, etc.) are declared in `cdda_components::def_markers` to avoid a circular dep.
- Units are typed (e.g. `Volume(u64)`, `Weight(u64)`, `Energy(u64)`, `Length(u32)`, `Time(i64)`); arithmetic operators live in `core/units/`. Do not pass raw numeric values across crate boundaries — use the typed wrappers.
- RNG: `wyrand` is the default for deterministic gameplay. `rng.rs` re-exports the chosen generator plus a Bevy-friendly `Resource` wrapper.
- The 138 `raw_defs/*.rs` files were moved to `cdda_defs_raw` in Phase 3a to give consumers a smaller compile surface. New def types go there.

## Work Guidance
- When adding a new coordinate type, add a `Direction`-style helper in the same file and a unit test in `tests/coordinate_test.rs`.
- When adding a new unit, add a `Quantity` impl block in `core/units/<unit>.rs` with constructor + arithmetic + `Display`.
- **Do not add raw def structs here.** They belong in `cdda_defs_raw`.
- Do not introduce a Bevy runtime dep (no `App`, `Schedule`, etc.) in this crate. The only Bevy items allowed are derive macros on plain types.

## Verification
- `cargo test -p cdda_core_types` for the unit, coordinate, RNG, and stats suites.
- Changes to coordinate math must keep `tests/coordinate_test.rs` green and the existing `cdda_overmap` chunk tests must still pass (spatial consistency is the cross-crate contract).
- Changes to `DefId<T>` are reflected in `cdda_components` and `cdda_data` — run `cargo check --workspace` after edits.

## Child DOX Index
- `src/core/coords/` — Coordinate types and direction helpers. No further durable sub-boundaries; each file owns one coordinate family.
- `src/core/units/` — Typed units (energy, length, time, volume, weight). No further durable sub-boundaries.
- `src/rng.rs`, `src/sim_id.rs`, `src/wyrand.rs` — RNG and ID helpers. No further durable sub-boundaries.
- `src/core/damage.rs`, `src/core/error.rs`, `src/core/flags.rs`, `src/core/id.rs` — Damage model, error types, flag set, `DefId`/`DefCategory`. No further durable sub-boundaries.
- `tests/` — Coordinate, RNG, stats, and units unit tests. No further durable sub-boundaries.
