# cdda_overmap DOX

## Purpose
Owns overmap storage, terrain registry, spatial index, and serialization.

## Ownership
- Overmap terrain data structures, terrain registry, spatial index, pathfinding, and serialization live in this crate.
- Overmap generation systems live in `cdda_overmap_gen`.

## Local Contracts
- The terrain registry is the canonical source for overmap terrain lookup.
- Spatial index and pathfinding helpers should avoid render/input dependencies.

## Work Guidance
- Keep storage and query APIs stable for generation, rendering, and gameplay consumers.
- Prefer deterministic data structures for serialized overmap state.

## Verification
- Run `cargo check -p cdda_overmap`.
- Run `cargo test -p cdda_overmap` when storage or pathfinding behavior changes.

## Child DOX Index
