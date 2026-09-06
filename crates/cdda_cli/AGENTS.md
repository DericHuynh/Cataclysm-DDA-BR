# cdda_cli DOX

## Purpose
- `cdda-cli` binary (Layer 5 app shell) — schema generation, JSON validation, definition statistics, full load check, ablation testing, a 15×15 ASCII city preview, and the data-consistency checks (Phase-A **round-trip** and Part-B **bridge** export) for CDDA mod developers. Single source file: `src/main.rs`.

## Ownership
- CLI parsing (`clap`, `Command` enum at `src/main.rs:34-80`) and per-command dispatch live here.
- All data loading goes through `cdda_data::loader::Loader`; overmap generation through `cdda_overmap_gen::pipeline::OvermapGenPlugin`; schema writing through `cdda_data::schema` and `cdda_data::schema_gen`. This crate contains no parsing or generation logic of its own.

## Local Contracts

- Loader-backed commands use load_reported and the shared ReportEvent stderr format, matching graphical startup stages and diagnostics.
- Subcommands (declared in the `Command` enum — must stay in sync):
  - `schema` — static JSON Schema files for all definition types, no dynamic enums. Calls `cdda_data::schema::write_all_schemas`.
  - `gen-schemas --core <dir> [--mod NAME=PATH]...` — dynamic per-mod schemas with autocomplete enums; writes to `schemas/<name>/` for each mod.
  - `validate <path>` — load + resolve all JSON under `<path>`, report errors.
  - `stats <path>` — print raw definition counts per type plus resolved registry totals.
  - `check <path>` — full load check (ingest + resolve + validate).
  - `city-view <data_dir> [--city-size N=12] [--seed N=42]` — run a real Bevy `App` with `OvermapGenPlugin` and render a 15×15 ASCII grid around the city nearest chunk center.
  - `ablation <baseline> <mod_dirs>...` — load baseline, then re-load with each mod removed and each mod isolated in turn, to attribute load errors.
  - `roundtrip <path> [--strict]` — run the Phase-A JSON→struct→JSON drop check via `cdda_data::roundtrip::roundtrip_all_types`, printing a per-category report. `--strict` exits 1 if any category has parse failures or unresolved defs. (Unmodeled-field findings print but don't fail unless `--strict`.)
  - `bridge <path> [--strict]` — run the Part-B export-adapter check via `cdda_data::bridge::bridge_all_types`: for every modeled def that `copy-from`s an in-category parent, compute the minimal override delta, re-apply it to the parent, and assert it reproduces the child's resolved value. `--strict` exits 1 if any category reports an export mismatch.
- I/O split: `tracing_subscriber` is pinned to `std::io::stderr`; all progress, errors, and the final `Elapsed: ...` line go to `eprintln!`. **Only `city-view` writes the rendered grid to stdout** (header, legend, rows, footer via `println!`); its tile-count summary still goes to stderr. All other subcommands are stderr-only.
- `--mod NAME=PATH`: `parse_mod_pair` splits on the first `=`, requires `path` to exist on disk, returns `ModEntry { name, path }` or an error string.
- `get_default_schema_dir()` resolves to `<workspace>/data/schemas` by `env!("CARGO_MANIFEST_DIR").pop().pop().join("data/schemas")` (i.e. `crates/cdda_cli` → workspace root → `data/schemas`).
- Bevy deps: `bevy_app`, `bevy_ecs`, `bevy_state`. Internal crate deps: `cdda_core_types`, `cdda_data`, `cdda_overmap`, `cdda_overmap_gen`. Other: `clap`, `serde_json`, `schemars`, `tracing`, `tracing-subscriber`.

## Work Guidance
- Keep command bodies thin — orchestrate `cdda_data` / `cdda_overmap_gen`, do not duplicate their logic.
- New subcommand: add a `Command` variant, a `cmd_<name>` function, and a `match` arm in `main`. Preserve the stderr/stdout split.
- Default schema output is repo-relative (`<workspace>/data/schemas`); pass a different `PathBuf` into the underlying writer if a non-default layout is needed.

## Verification
- `cargo check -p cdda_cli` for compile sanity.
- Smoke-test each subcommand against `data/core`, e.g. `cargo run -p cdda_cli -- schema`, `-- stats data/core`, `-- check data/core`, `-- city-view data/core`, `-- roundtrip data/core`.

## Child DOX Index
- (none — single-file crate)
