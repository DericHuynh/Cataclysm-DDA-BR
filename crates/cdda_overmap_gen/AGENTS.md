# cdda_overmap_gen DOX

## Purpose
Layer 4 crate. Owns the overmap generation pipeline: a 1:1 Rust port of CDDA master's `overmap::generate()` (overmap.cpp L932–1060) expressed as ordered Bevy ECS systems. All generation phases (init, natural terrain, cities, connections, specials, underground, finalize) live here. Terrain storage, terrain registry, pathfinding, and the seeded LCG remain in `cdda_overmap` and are read/written through the storage contracts defined there.

## Ownership
- `src/lib.rs` declares the four top-level modules: `pipeline`, `region_settings`, `special_catalog`, `steps`.
- `src/pipeline.rs` defines `OvermapGenPhase` (Bevy `States`: `Idle | Generating | Complete`, default `Idle`), `OvermapGenConfig` (Resource: `noise_seed: u32`, `om_x: i32`, `om_y: i32`; default seed `DEFAULT_NOISE_SEED = 1920237457`), `OvermapEntity` (Component marker), `OvermapGenSet` (13-variant `SystemSet`), and `OvermapGenPlugin` (`bevy_app::Plugin`).
- `src/region_settings.rs` holds the `OvermapRegionSettings` `Resource` and per-pass sub-structs (`RegionSettingsForest`, `RegionSettingsLake`, `RegionSettingsOcean`, `RegionSettingsRiver`, `RegionSettingsCity`, `RegionSettingsRavine`, `RegionSettingsForestTrail`). Master booleans (`overmap_forest`, `overmap_lake`, `overmap_ocean`, `overmap_river`, `city_spec`, `overmap_ravine`, `overmap_highway`, `forest_trail`) gate their sub-structs. Directional arrays use N-E-S-W order matching C++ `om_direction::type` (`North=0, East=1, South=2, West=3`).
- `src/special_catalog.rs` is a **stub** — types `SpecialDef`, `SpecialOmt`, `SpecialPhase`, `SpecialRule`, and the `SpecialCatalog` Resource are declared, but `SpecialCatalog::from_registry(&DefRegistry)` returns `Self::default()`. No JSON loading is wired up yet.
- `src/steps/` is a 24-file flat module tree (one file per pipeline step + `mod.rs` re-exports). `stubs::generate_shore_variants` is a registry-prep helper exported from `steps/mod.rs` but **not registered in `OvermapGenPlugin`**.

## Local Contracts
- **Bevy deps** (from `Cargo.toml`): `bevy_ecs`, `bevy_app`, `bevy_state`; workspace-deps `serde`, `tracing`, `tracing-subscriber`. Path deps: `cdda_core_types`, `cdda_components`, `cdda_data`, `cdda_overmap`, `cdda_sim`.
- **Determinism contract**: identical `(OvermapGenConfig, OvermapRegionSettings, terrain defs)` produces byte-identical overmap output. No wall clock: a crate-wide search for `Instant::now`, `Utc::now`, `SystemTime` returns no matches. All randomness flows from `XorShiftRng` (`cdda_overmap::rng`) seeded by `OvermapGenConfig::noise_seed` plus per-step constants.
- **Pipeline order** (`OvermapGenSet`, chained in `OvermapGenPlugin::build`, gated by `in_state(OvermapGenPhase::Generating)`):
  1. `InitBase` — `init_base_terrain` (spawns overmap + 756 chunk entities, z = -10..=10).
  2. `NeighborConnections` — `populate_connections_out_from_neighbors` (cross-overmap exits).
  3. `NaturalTerrain` — chained: `place_rivers` → `place_lakes` → `place_oceans` → `place_forests` → `place_swamps` → `place_ravines` → `polish_river` (#1).
  4. `Highways` — `place_highways` (before cities so cities avoid highways).
  5. `Cities` — `place_cities` (center placement).
  6. `PostCities` — chained: `place_highway_interchanges` → `build_cities` (street grids).
  7. `Connections` — two parallel chain sets gated by `OvermapRegionSettings::place_railroads_before_roads`; each chains `place_forest_trails` → (rails/roads in flag-dependent order).
  8. `Structures` — chained: `place_specials` → `place_mutable_specials`.
  9. `PreUnderground` — chained: `finalize_highways` → `place_forest_trailheads` → `polish_river` (#2).
  10. `Underground` — `generate_sub` (z < 0: sewers, subways).
  11. `Elevated` — `generate_over` (z > 0: bridges).
  12. `Population` — `place_mongroups` + `place_radios` (unordered).
  13. `Finalize` — `finalize_overmap` (inserts `Finalized` marker, logs z=0 statistics); a sibling closure transitions state to `OvermapGenPhase::Complete`.
- **Per-step gating**: each system early-returns on its corresponding `OvermapRegionSettings` toggle (`overmap_forest`, `overmap_lake`, `overmap_ocean`, `overmap_river`, `overmap_ravine`, `overmap_highway`, `forest_trail`, `city_spec`, `place_specials`, `place_roads`, `place_railroads`, `neighbor_connections`) — see `region_settings.rs` table for the full mapping.
- **State and observer wiring**: `OvermapGenPlugin` also `init_resource`s `cdda_overmap::index::ChunkIndex` and registers its `on_chunk_added` / `on_chunk_removed` observers for spatial maintenance.

## Work Guidance
- When changing step order or adding a phase, update the `OvermapGenSet` enum, the chain tuple in `OvermapGenPlugin::build`, the numbered list in `src/lib.rs` doc comment, and this AGENTS.md. The C++ `overmap.cpp` reference is the source of truth.
- New step systems must read terrain through `cdda_overmap::registry::{TerrainRegistry, CoreTerrains}` and write through `OvermapChunk::set` — never duplicate terrain lookup logic locally.
- New step systems consume `Res<OvermapGenConfig>` and `Res<OvermapRegionSettings>` for seed/coord and gating respectively. Add the corresponding `OvermapRegionSettings` toggle if the step should be skippable.
- The `stubs::generate_shore_variants` function is a registry-time helper, not a per-overmap system. Call sites belong in data loading, not the generation schedule.

## Verification
- `cargo check -p cdda_overmap_gen` for compile sanity.
- `cargo nextest run -p cdda_overmap_gen` — currently exercises only the inline `#[cfg(test)]` suites in `steps/init_base.rs` and `region_settings.rs`; the `tests/` directory is **empty** (no integration tests yet). Fall back to `cargo test -p cdda_overmap_gen` if `nextest` is unavailable.
- Determinism smoke: run the pipeline twice with identical `OvermapGenConfig` + `OvermapRegionSettings` and compare z=0 chunk `TerrainHandle` bytes.
- For full-stack runs, use `cdda-cli` (see `crates/cdda_cli/AGENTS.md`); `cdda_overmap_gen` is a library crate with no binary.

## Child DOX Index
- *(none — flat module tree under `src/steps/`; no nested `AGENTS.md` boundaries)*
