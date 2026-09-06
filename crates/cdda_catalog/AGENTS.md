# cdda_catalog

## Purpose
Runtime definition contracts without JSON loaders, filesystem or input dependencies.

## Ownership
`definition.rs` owns typed entity indexes; `interner.rs` owns stable session tokens;
`htn.rs` owns the native planner-program interface; `inventory.rs` owns immutable item/recipe records, typed stable keys and snapshot references. Import adapters live in cdda_data.

## Local Contracts
- Never depend on cdda_data, cdda_defs_raw, cdda_input, assets, or rendering.
- Entity values and interned tokens are session-local; persistent references use stable keys.
- InventoryCatalog validates keys, counts/work and item references. Key serialization is independent of Entity bits/token order; it is not full gameplay persistence.
- Native program arguments are validated by registered kernels during compilation.

## Work Guidance
Keep normalization and source-format handling in import adapters.

## Verification
`cargo check -p cdda_catalog`; `cargo nextest run -p cdda_catalog`.

## Child DOX Index
None.
