# cdda_defs_raw

## Purpose
Raw JSON def structs — the typed AST layer of the data pipeline. Each file under `src/raw_defs/` mirrors one `"type"` value that appears in `data/core/*.json`. No Bevy, no logic, no Bevy ECS — just `serde`-deserialized Rust structs.

## Ownership
- A new foundational crate, added at Layer 1.5 between `cdda_core_types` (value types) and `cdda_data` (the resolver + registry).
- No Bevy deps. `serde`, `serde_json`, `schemars`, `thiserror`, `indexmap`, plus the `cdda_core_types` workspace crate.
- Two top-level submodules: `src/raw_defs/` (138 files, one per def category) and `src/raw_types/` (3 files: `copy_from`, `localized`, `mod`).

## Local Contracts
- **One file per def category.** `src/raw_defs/<type>.rs` mirrors `data/core/<type>.json`. The 138 categories are exhaustive as of this writing; new categories go in their own `raw_defs/<type>.rs` file.
- **Typed AST output, not a registry.** These structs represent the unvalidated, unresolved JSON shape. The `cdda_data` crate's loader produces them from `serde_json::Value`; the resolver walks `copy_from` chains and produces the final typed `Foo` structs.
- **No `cdda_defs_raw` symbols are Bevy entities.** They are plain Rust. `cdda_data::def_world` is what spawns Bevy entities from these types.
- **Raw types re-export.** `src/raw_types/` provides `copy_from.rs`, `localized.rs`, and `mod.rs`. The `LocalizedString`, `CopyFromOp`, and `DefId` re-exports are convenient for consumers.

## Work Guidance
- Adding a new def type:
  1. Add `src/raw_defs/<type>.rs` mirroring the JSON shape.
  2. Add the module to `src/raw_defs/mod.rs`.
  3. Register the type in `cdda_data::def_kinds.rs`.
  4. Add a `DefRegistry` field in `cdda_data::registry.rs`.
  5. Add a `def_world.rs` builder.
  6. Regenerate the JSON schema via `cdda-cli schema`.
- The `def_kind!` declarative table macro (planned for a future commit) will collapse steps 1-5 into a single `def_kind! { Foo { id: "id", … } }` line. Until then, each def type is a 6-step change.
- **Do not** import Bevy types into this crate. It is intentionally a leaf that the rest of the workspace depends on.
- **Do not** add business logic here. Any function that walks the typed AST, resolves `copy-from`, or builds a Bevy entity belongs in `cdda_data`.

## Verification
- `cargo check -p cdda_defs_raw` for compile sanity.
- `cargo nextest run --workspace` exercises the full data pipeline (this crate is consumed by `cdda_data` and the 138 raw defs are used in `tests/def_world_load.rs` and `tests/hot_reload_*.rs`).
- Cross-crate impact: any change to a raw def's shape must be accompanied by a corresponding change in the registry, def world builder, and the JSON schema in `data/schemas/`.

## Child DOX Index
- `src/lib.rs` — Module declarations and re-exports.
- `src/raw_defs/` — 138 files, one per CDDA def category. Listed in `src/raw_defs/mod.rs`; new categories are added to that module list and get their own file.
- `src/raw_types/` — Shared types: `copy_from.rs` (the `CopyFromOp` enum and `copy_from` field type), `localized.rs` (`LocalizedString`), and `mod.rs` (re-exports).
