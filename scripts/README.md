# Developer Scripts

Tools for working with CDDA data definitions and Rust structs.

## Quick Reference

```bash
# List ALL JSON types in data/core/ (unique type strings)
./scripts/extract_json_types.sh

# Count definitions per type
./scripts/list_all_types.sh

# Check Rust struct coverage against actual JSON fields
./scripts/verify_def_coverage.py            # all types
./scripts/verify_def_coverage.py ITEM       # single type
./scripts/verify_def_coverage.py ITEM 50    # show top 50 missing fields

# Sample JSON entries (see the real format)
./scripts/sample_json.py ITEM
./scripts/sample_json.py terrain 5
./scripts/sample_json.py monster 1 --full
```

## Workflow

1. **Discover types**: `extract_json_types.sh` shows what `"type"` values exist.
2. **Check coverage**: `verify_def_coverage.py` compares JSON fields to Rust structs.
3. **View samples**: `sample_json.py` shows real entries to verify struct design.
4. **Fix gaps**: Add missing fields to the corresponding `crates/cdda_core_types/src/core/raw_defs/*.rs` file.

## Architecture checks

- `python3 scripts/check_runtime_dependencies.py` checks actual transitive normal Cargo graphs for cdda_sim, cdda_catalog and cdda_ui; exits nonzero on forbidden edges. Dev-only fixtures are excluded.
- `python3 scripts/content_baseline.py <file-or-directory> ...` prints file counts, sizes and hashes. Directories include sorted JSON paths. Digest input: each relative UTF-8 path, NUL, SHA-256 of its bytes. No files are written.
