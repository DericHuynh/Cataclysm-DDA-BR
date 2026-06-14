# Data DOX

## Purpose
Owns the game-data files loaded by `cdda_data` (JSON definitions, mods, generated JSON Schemas) plus the legacy `LOADING_ORDER.md` note.

## Ownership
- The loader pipeline lives in `crates/cdda_data/`; this subtree is the data it consumes.
- Mods are merged at load time; this folder is the source of truth for what is shipped.
- Generated JSON Schemas live under `data/schemas/`; they are produced by `cdda_data` and consumed by mod tooling.

## Local Contracts
- `data/core/` — Vanilla CDDA definitions. Add new entries under the appropriate subdirectory by `"type"`.
- `data/mods/` — Mod overlays applied on top of `core` by `cdda_data::mod_layer`. Each mod owns its subfolder; mods must declare an `modinfo.json` and follow the layering rules in `TARGET_ARCHITECTURE.md` § ACL.
- `data/schemas/` — Generated JSON Schemas for the public def type surface. Do not hand-edit; regenerate via `cdda_cli` (schema generation subcommand).
- All shipped definitions must round-trip through `cdda_data` (ingest → resolve → `DefRegistry` → `build_def_world`). Anything that fails to parse is a data bug, not a loader bug.

## Work Guidance
- Discover new JSON types with `scripts/extract_json_types.sh`; check Rust struct coverage with `scripts/verify_def_coverage.py`.
- When adding a new definition category, add a `raw_defs/*.rs` struct in `crates/cdda_core_types/src/core/raw_defs/`, register it in `cdda_data::def_kinds`, and regenerate the schema in `data/schemas/`.
- Keep `data/core/LOADING_ORDER.md` reviewed if the loader's type-subdirectory layout changes; it currently documents the legacy upstream `data/json/` layout.

## Verification
- `cargo nextest run --workspace` exercises the full data-loading pipeline against the bundled JSON (fall back to `cargo test --workspace` if `nextest` is unavailable).
- `crates/cdda_data/tests/def_world_load.rs` and `crates/cdda_data/tests/hot_reload_test.rs` are the canonical regression suites for this subtree.
- `scripts/verify_def_coverage.py ITEM` (and other type names) reports JSON fields with no Rust struct counterpart.

## Child DOX Index
- `data/core/` — Vanilla CDDA JSON definitions, organized by `"type"`. No further durable sub-boundaries; subdirectories are content categories, not contracts.
- `data/mods/Magiclysm/` — Bundled mod. No further durable sub-boundaries.
- `data/schemas/` — Generated JSON Schemas. No further durable sub-boundaries.
