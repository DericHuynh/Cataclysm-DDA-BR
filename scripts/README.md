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
