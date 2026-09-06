# cdda_overmap DOX

## Purpose
Layer 4 crate. Owns overmap chunk storage, the terrain registry, the entity spatial index, deterministic A* pathfinding, the CDDA-mirroring direction enums, the seeded LCG PRNG, and the binary chunk save format. Consumed by `cdda_overmap_gen` (generation pipeline) and `cdda_render`/`cdda_context` (overmap UI). No mapgen passes, city placement, or submap tile content live here.

## Ownership
- `src/lib.rs` re-exports the public surface; modules are `camera`, `chunk`, `connections`, `direction`, `index`, `pathfinding`, `query`, `registry`, `rng`, `serial`, `spatial`.
- Storage entities: `OvermapChunk` (terrain), `ChunkPosition` (packed key), `ChunkState` (lifecycle), `ChunkOfOvermap` / `OvermapChunks` (relationship pair).
- Registry resources: `TerrainRegistry` (generic SoA), `CoreTerrains` (game-specific handles), `TerrainQuery` (SystemParam), `EntitySpatialIndex` (dynamic entities), `ChunkIndex` (entity lookup), `OvermapCamera` (viewport).
- `OmDirection` / `CubeDirection` mirror CDDA `om_direction::type` / `cube_direction`; `XorShiftRng` mirrors CDDA `rng()`. The `Rng` trait (minimal `random_usize`) lives in `direction.rs`.

## Local Contracts
- **Bevy deps** (from `Cargo.toml`): `bevy_ecs`, `bevy_app`, `serde`, `bytemuck`, `cdda_core_types`, `cdda_components` (`WorldPosition` for spatial sync), `cdda_sim`. **No `postcard`, `bincode`, or any external codec** — `serial.rs` is hand-rolled little-endian binary over `std::io::{Read, Write}` using `to_le_bytes`/`from_le_bytes`. `serde` and `bytemuck` are declared but not yet exercised in source.
- **Current layering debt**: `cdda_overmap` depends on `cdda_sim`. This violates the intended bottom-up Layer 4 rule and should be cleaned up by extracting shared types or moving the dependency.
- **Coordinate model** (`chunk.rs` constants): `SUBMAP_DIM=12`, `SUBMAP_SIZE=144`, `SUBMAPS_PER_OMT_AXIS=2`, `SUBMAPS_PER_OMT=4`, `OMT_DIM_TILES=24`, `CHUNK_DIM=30`, `CHUNK_SIZE=900`, `CHUNKS_PER_LAYER=36`, `CHUNKS_PER_OVERMAP=756`, `OMAP_DIM=180`, `OMAP_DIM_SUBMAPS=360`. 180/6/30 divides exactly — no edge waste.
- **`TerrainHandle`**: `u32` packed `[type_index:24 | rotation:8]`; index 0 is `NULL`. Indices are process-local, never persistent IDs. `TerrainRegistry` is a `Resource` with parallel SoA `Vec`s keyed by `type_index()` for O(1) property and stable-ID reverse lookup. Registration upserts existing IDs in place; new IDs append slots.
- **Terrain reload**: `TerrainRegistry::rebuild_from` atomically imports a fresh registry by stable ID, preserving existing terrain/family slots and remapping rotation links; new terrain IDs append. Any removed old ID (even unused), unknown family, or invalid rotation target rejects the rebuild with `TerrainRegistryReloadError` and leaves the old registry unchanged. Callers must reject the dependent definition reload before world mutation, not replace the registry with fresh numeric slots. Removal requires a separate explicit world migration; there is no silent fallback/reuse.
- **`CoreTerrains`**: separate `Resource` with pre-resolved handles — `field`, `forest`, `forest_thick`, `forest_water`, `road_ns`, `road_ew`, `road_nesw`, `lake_surface`, `lake_shore`, `ocean`, `river_center`. Populated post-load via `from_registry`; missing IDs log to `eprintln!` and fall back to `NULL`.
- **`TerrainFlags`** (u16: `ROAD`, `FOREST`, `RIVER`, `OCEAN`, `IMPASSABLE`, …) and `family_id` (u32) are assigned at registration. Runtime checks must use `has_flag` / `has_family` (`query.rs`, O(1)) — `is_ot_match` and `id.contains` are cold-path only.
- **`ChunkState`**: `Generating → Finalizing → Ready`. `Changed<OvermapChunk>` is meaningful only in the first two; downstream systems gate reads on `Ready`.
- **`ChunkIndex`**: `HashMap<u64, Entity>` keyed by `ChunkPosition::to_key()`. Maintained by `OnAdd`/`OnRemove` observers (`index.rs`) — not by systems — so removals are O(1).
- **`EntitySpatialIndex`** (`spatial.rs`): 3D grid with `CELL_SIZE=16` world tiles, `Z_CELL_SIZE=1`; `query_radius` (3D Chebyshev) and `query_radius_2d`; `cell_of(entity)` for diagnostics/tests. Synced by `sync_spatial_index` on `Changed<WorldPosition>` **and** `Changed<WorldPos>` (gameplay entities carry the `WorldPosition` wrapper — movement writes it — while raw `WorldPos` entities stay supported; the raw query is `Without<WorldPosition>` so nothing syncs twice). Torn down by the `remove_from_spatial_index` / `remove_raw_pos_from_spatial_index` observers on `Remove<WorldPosition>` / `Remove<WorldPos>`. (`cdda_overmap` now depends on `cdda_components` for `WorldPosition`.) Pinned by the `sync_tracks_world_position_and_raw_pos_alike` test.
- **Pathfinding** (`pathfinding.rs`): **hand-rolled A*/best-first**, not the `pathfinding` crate. `greedy_path(start, end, max, &scoring_fn) -> Vec<DirectedNode>`, 4-cardinal moves, min-heap priority queue. `NodeScore::REJECTED` (`node_cost < 0`) blocks a node. Returned path is **destination→start** (CDDA convention) — `reverse()` for start→end.
- **Wire format v1** (`serial.rs`): chunk magic `OMTC` + `version:u16`, then 11-byte coordinates (`chunk_x:u8, chunk_y:u8, z_index:u8, om_x:i32, om_y:i32`), `palette_count:u16`, length-prefixed UTF-8 stable terrain IDs (`length:u16`), and 900 cells of `(palette_index:u16, rotation:u8)`. Palette index 0 is implicit NULL with rotation 0; other rotations preserve all 8 bits. Palette count ≤900; each nonempty ID ≤4096 bytes. Multi-chunk streams prepend `OMTS`, `version:u16`, `count:u32` to chunk records. All integers are little-endian. Both read/write APIs require the owning/destination `TerrainRegistry`; unknown IDs/handles, legacy raw-index files, unsupported versions, duplicate palette IDs, malformed coordinates/indices/NULL rotations and truncation are errors. Readers consume exactly the framed records, allowing concatenated streams. Submap tile content is **not** included.
- **`XorShiftRng`** (`rng.rs`): LCG `state = state*1_103_515_245 + 12_345`; methods `range_i32` (inclusive), `range_f32` ([lo, hi)), `one_in`, `x_in_y`, `roll_remainder` mirror CDDA. Seed 0 is internally bumped to 1 to escape the absorbing state. Implements the `Rng` trait from `direction.rs`.
- **`OmDirection`**: `#[repr(i32)]` with discriminants `Invalid=-1, North=0, East=1, South=2, West=3` — matches C++ ABI. Rotation math matches `overmap.cpp` L2796–2920.

## Work Guidance
- `OvermapChunk::set` no-ops on equal writes; preserve this to keep `Changed<OvermapChunk>` from firing spuriously.
- Mutate `OvermapChunk` terrain only while `ChunkState::Generating` or `Finalizing`. Treat as immutable after `Ready`.
- Reordering `OmDirection` / `CubeDirection` discriminants breaks C++ ABI compatibility — do not reorder.
- `debug/` and `tmp/` at the crate root are local Cargo build cache, not durable artifacts.

## Verification
- `cargo check -p cdda_overmap` for compile sanity.
- `cargo nextest run -p cdda_overmap` for the per-module `#[cfg(test)]` suites in `chunk`, `serial`, `direction`, `rng`, `pathfinding`, `connections`, and the integration tests in `tests/overmap_tests.rs` and `tests/terrain_persistence_tests.rs` (stable-ID/rotation roundtrips across reordered registries, corrupt/unknown input rejection, append-only reload identity and removal rollback; fall back to `cargo test -p cdda_overmap` if `nextest` is unavailable).

## Child DOX Index
- *(none — flat single-crate module tree; no nested `AGENTS.md` boundaries)*
