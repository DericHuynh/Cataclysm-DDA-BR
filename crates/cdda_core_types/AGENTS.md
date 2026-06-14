# cdda_core_types DOX

## Purpose
Owns pure domain value types, coordinates, IDs, damage, stats, flags, and RNG.

## Ownership
- Core value objects and marker types live in this crate.
- ECS components and shared contracts belong in `cdda_components`.

## Local Contracts
- Core types should avoid Bevy-specific behavior except where required by existing value-object derives.
- IDs, units, damage, stats, and RNG behavior are canonical for the workspace.

## Work Guidance
- Prefer pure functions and deterministic RNG for mechanics shared across crates.
- Keep domain types independent from rendering, input, and app crates.

## Verification
- Run `cargo check -p cdda_core_types`.
- Run `cargo test -p cdda_core_types` when core behavior changes.

## Child DOX Index
