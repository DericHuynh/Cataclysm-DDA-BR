# cdda_overmap_gen DOX

## Purpose
Owns overmap generation pipeline and deterministic RNG.

## Ownership
- Overmap generation systems, region settings, city/special/connection/mongroup placement, and deterministic generation flow live in this crate.
- Terrain storage and terrain registry remain in `cdda_overmap`.

## Local Contracts
- Generation must be deterministic for a given seed and configuration.
- The generation pipeline should write into `cdda_overmap` storage and registry contracts.

## Work Guidance
- Keep generation phases explicit and ordered.
- Avoid duplicating terrain lookup logic outside `cdda_overmap`.

## Verification
- Run `cargo check -p cdda_overmap_gen`.
- Run `cargo test -p cdda_overmap_gen` when generation behavior changes.
- Use `cargo run -p cdda-cli -- city-view data/core` for generation smoke tests.

## Child DOX Index
