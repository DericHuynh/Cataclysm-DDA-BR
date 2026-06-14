# cdda_data DOX

## Purpose
JSON loading, definition registry, `copy-from` inheritance resolution, Bevy entity spawning for all game definitions, per-category flag systems, and JSON Schema generation for modder tooling. Owns the pipeline that turns `data/**/*.json` into a queryable `DefinitionWorld` resource.

## Ownership
- Source lives under `crates/cdda_data/src/`. Modules: `loader`, `resolve`, `registry`, `def_world`, `def_registry_resource`, `def_kinds`, `patch`, `populate_flags`, `interner`, `mod_info`, `mod_layer`, `raw_values`, `assets`, `schema`, `schema_gen`, `flags`.
- Bin target: `src/bin/generate_schemas.rs` (invokes `schema::write_all_schemas`).
- Bevy deps (pinned at workspace): `bevy_ecs`, `bevy_app`, `bevy_asset`, `bevy_state`, `bevy_reflect`. Workspace crate deps: `cdda_core_types`, `cdda_components`, `cdda_events`, `cdda_sim`. Other: `serde`, `serde_json`, `tracing`, `thiserror`, `schemars`, `indexmap`, `fixedbitset`, `bidimap`. Dev-deps: `tempfile`, `bevy_ecs`, `cdda_components`, `cdda_core_types`.
- Raw data under `data/` is owned by `data/AGENTS.md`; this crate consumes it.

## Local Contracts
- **Two-pass load contract.** `Loader::ingest_all` (Pass 1) walks `data_dirs`, parses `.json` into `RawDef { id, value, source }` grouped by `"type"`, and applies `default_type_aliases` (e.g. `GUN`/`AMMO`/`COMESTIBLE`/... → `ITEM`) via `canonicalize_types`. `Loader::load` (Pass 2) calls `resolve_all`, which topologically sorts each type by `copy-from` and applies `resolve::resolve_copy_from` (extend / delete / relative / proportional semantics from `patch::apply_cdda_patch`). The final `DefRegistry` is the authoritative read-only store.
- **`DefRegistry` god object.** Single struct with ~100+ `HashMap<DefId<T>, Arc<TDef>>` fields (one per category). `for_each_raw_def_kind!` (in `def_kinds.rs`) is the single source of truth for the `(name, DefType, "json_type", field, strategy)` table; `DefRegistry::empty`, `total_count`, and `category_count` must stay in sync. Migration plan: trait-based `Registry<T>` + `RegistrySet` (`TARGET_ARCHITECTURE.md` § Phase 1.2).
- **Def-world construction.** `build_def_world(world, &DefRegistry, spawn_all)` (in `def_world.rs`) spawns definition entities into the main Bevy `World` marked with `IsDef`, then returns a `DefinitionWorld` index. Per-category builders exist: `build_item_defs`, `build_monster_defs`, `build_terrain_defs`, `build_furniture_defs`, `build_recipe_defs`, `build_body_part_defs`. `spawn_all=false` skips everything except body parts. Decomposition into more granular per-category functions is the active debt item.
- **`DefinitionWorld` resource.** `HashMap<String, Entity>` index. Public API: `empty()`, `entity_by_str(&str) -> Option<Entity>`, `len()`, `iter() -> impl Iterator<Item = (&str, Entity)>`. `register` is module-private.
- **`DefRegistryResource(Arc<DefRegistry>)`** wraps the resolved registry as a `bevy_ecs::Resource` for runtime tools (e.g. registry viewer). Separate from `DefinitionWorld`, which is the entity index.
- **Mod layering.** `mod_info.rs` defines `ModInfo`, `ModId`, `ModError`, `check_dependencies`, `check_conflicts`, `resolve_load_order` (Kahn's algorithm). `mod_layer.rs` adds `ModManager { available, core_registry, _core_loader }`, `ModLoadResult`, `ModManifest`, `topological_sort`, `check_conflicts`, `load_mods`, and the headless `load_with_mods`. Mods merge into the core registry with last-write-wins per ID; no separate `ModShard` numeric space exists yet.
- **Schema generation split.** `schema.rs` produces static JSON Schema files per type via `schemars` (`write_all_schemas`, `validate_against_schema`, `validate_all`). `schema_gen.rs` produces dynamic schemas with runtime-injected `enum` values for flags and IDs: defines `ModRegistry { all_flags, all_item_ids }`, `ItemModSchema`, `CddaSchemaPlugin`, `collect_mod_registry_v2`, `generate_dynamic_schemas`, and `generate_schemas_for_mod` (headless CLI path). Output dir resolved by `schema_output_dir` (`assets/schemas` or `$CARGO_MANIFEST_DIR/../../data/schemas`).
- **Flag system.** `flags.rs` defines `FlagMap` (bidirectional string↔`u16` map, cap 4096), per-category `*FlagRegistry` Bevy `Resource`s, and per-category `*Flags(FixedBitSet)` components. `CddaDataPlugin` initializes all registries. `populate_flags::populate_def_flags` runs after `build_def_world` and inserts the bitset component on each entity. `register_flags_from_json` mirrors the C++ `auto_flags_reader` (handles array, single-string, `extend`, `delete`).
- **Patch semantics.** `patch::apply_cdda_patch` mirrors C++ `generic_factory`: nested objects recurse, arrays replace, scalars replace. `extend` appends to an existing array field (no-op with warning if absent — never creates the field). `delete` removes matching elements (no-op with warning if absent).

## Work Guidance
- Keep the three concerns separated: parse (`Loader`) → resolve (`resolve`) → spawn (`build_def_world`). Don't re-introduce I/O in resolve or registry mutation in spawn.
- When adding a new def category: extend `def_kinds.rs` (one `for_each_raw_def_kind!` line), add the `HashMap` field to `DefRegistry`, add it to `DefRegistry::empty` / `total_count` / `category_count`, and add a per-category builder in `def_world.rs` (or wire it into an existing one). New flag categories also need a `flag_registry!` + `flag_comp!` pair in `flags.rs` and a registry init in `CddaDataPlugin`.
- `Loader` is mutable because pass-1 ingestion mutates `raw_by_type`. `ingest_all` is useful when you want raw defs without resolving (e.g. schema tooling that needs pre-resolution flag/ID discovery).
- `resolve::topological_sort` is the cycle detector; `LoaderError::CircularCopyFrom` and `MissingCopyFromTarget` come from there. `ModError::CircularDependency` and `UnknownDependency` are independent mod-graph errors.
- Mods re-parse the same JSON format as core data — no separate schema. Manifest discovery handles both `modinfo.json` and `mod.json`. `try_load_manifest` is the single ingestion point.
- Schema tools should write to the path returned by `schema_output_dir`; modder-facing schemas live at `data/schemas/*.schema.json` (referenced by `"$schema"` in mod JSON, which the loader tolerates).

## Verification
- `cargo check -p cdda_data` for compile sanity.
- `cargo nextest run -p cdda_data` (fallback `cargo test -p cdda_data`) covers the per-module test suites: loader ingest/canonicalize, `resolve` copy-from operations (extend/delete/relative/proportional/cycle), `patch` semantics, `flags` `FlagMap`, `populate_flags` no-panic, `def_world` per-builder empty-registry tests, and `schema` / `schema_gen` enum-injection and `$schema` tolerance.
- `cargo run -p cdda_data --bin generate_schemas` to regenerate static schemas into `data/schemas/`.
- `cargo run -p cdda-cli -- check data/core` for end-to-end data validation against the live CDDA JSON set.

## Child DOX Index
- None. This crate has no child `AGENTS.md` files; all contracts live in this doc and the module-level rustdoc headers.
