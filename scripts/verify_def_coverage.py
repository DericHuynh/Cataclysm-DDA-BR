#!/usr/bin/env python3
"""
Verify Rust struct definitions against actual JSON data.

For each JSON "type" and its corresponding Rust struct, extracts all
JSON field names, compares them to Rust struct fields, and reports:
- Fields in JSON but missing from the Rust struct
- Fields in the Rust struct but not in JSON (possible dead code)

Usage:
  python3 scripts/verify_def_coverage.py [type_name]

Examples:
  python3 scripts/verify_def_coverage.py          # check all types
  python3 scripts/verify_def_coverage.py ITEM     # check only ITEM
  python3 scripts/verify_def_coverage.py ITEM 50  # show top 50 missing
"""
import json, os, sys, re
from collections import Counter

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "core")
SRC_DIR  = os.path.join(os.path.dirname(__file__), "..", "crates", "cdda_data", "src", "defs")

TYPE_MAP = {
    "ITEM": "ItemDef",
    "MONSTER": "MonsterDef",
    "terrain": "TerrainDef",
    "furniture": "FurnitureDef",
    "recipe": "RecipeDef",
    "item_group": "ItemGroupDef",
    "palette": "MapgenPaletteDef",
    "effect_type": "EffectDef",
    "bionic": "BionicDef",
    "mutation": "MutationDef",
    "field_type": "FieldDef",
    "vehicle_part": "VehiclePartDef",
    "overmap_terrain": "OvermapTerrainDef",
    "overmap_special": "OvermapSpecialDef",
    "scenario": "ScenarioDef",
    "faction": "FactionDef",
    "material": "MaterialDef",
    "skill": "SkillDef",
    "trap": "TrapDef",
    "start_location": "StartLocationDef",
    "overmap_connection": "OvermapConnectionDef",
    "overmap_location": "OvermapLocationDef",
    "overmap_land_use_code": "OvermapLandUseCodeDef",
    "vehicle_part_location": "VehiclePartLocationDef",
    "vehicle_part_category": "VehiclePartCategoryDef",
    "mutation_category": "MutationCategoryDef",
    "trait_group": "TraitGroupDef",
}

def get_json_keys_freq(dtype, data_dir):
    """Extract field frequencies from JSON files."""
    c = Counter()
    cnt = 0
    for root, dirs, files in os.walk(data_dir):
        for f in files:
            if not f.endswith('.json'):
                continue
            path = os.path.join(root, f)
            try:
                with open(path) as fh:
                    data = json.load(fh)
            except:
                continue
            items = data if isinstance(data, list) else [data]
            for item in items:
                if isinstance(item, dict) and item.get('type') == dtype:
                    for k in item:
                        if not k.startswith('//'):
                            c[k] += 1
                    cnt += 1
    return c, cnt

def get_rust_fields(struct_name):
    """Extract Rust struct field names with serde rename info."""
    for fname in os.listdir(SRC_DIR):
        if not fname.endswith('.rs'):
            continue
        path = os.path.join(SRC_DIR, fname)
        with open(path) as fh:
            content = fh.read()

            if f'pub struct {struct_name}' not in content:
                continue

            lines = content.split('\n')
            fields = set()
            in_struct = False
            brace_depth = 0
            serde_rename = None

            for i, line in enumerate(lines):
                if f'pub struct {struct_name}' in line:
                    in_struct = True
                    brace_depth = line.count('{') - line.count('}')
                    continue

                if not in_struct:
                    continue

                # Track serde rename attributes
                if '#[serde(rename' in line:
                    m = re.search(r'rename\s*=\s*"([^"]+)"', line)
                    if m:
                        serde_rename = m.group(1)

                brace_depth += line.count('{') - line.count('}')

                m = re.match(r'^\s*pub\s+(\w+)', line)
                if m and brace_depth > 0:
                    rust_name = m.group(1)
                    if rust_name == 'r#type':
                        fields.add('type')
                    else:
                        fields.add(serde_rename or rust_name)
                    serde_rename = None

                if brace_depth <= 0:
                    break

            return fields
    return set()

def main():
    filter_type = sys.argv[1] if len(sys.argv) > 1 else None
    top_n = int(sys.argv[2]) if len(sys.argv) > 2 else 20

    for dtype, struct_name in sorted(TYPE_MAP.items()):
        if filter_type and dtype != filter_type:
            continue

        freq, cnt = get_json_keys_freq(dtype, DATA_DIR)
        rust_fields = get_rust_fields(struct_name)

        if cnt == 0:
            print(f"{dtype:35} -> {struct_name:25}  NO JSON DATA")
            continue

        # Build a set of JSON keys we handle
        handled = set()
        for k in rust_fields:
            handled.add(k)

        # Find top missing
        missing = []
        for k, n in freq.most_common(999):
            if k not in handled:
                missing.append((k, n))

        coverage = (len(freq) - len(missing)) * 100 // len(freq) if freq else 0
        print(f"{dtype:35} -> {struct_name:25}  {cnt:6} items  {len(rust_fields):3} fields  {coverage:2}% coverage")
        if missing:
            print(f"{'':35}   MISSING (top {min(top_n, len(missing))}):")
            for k, n in missing[:top_n]:
                print(f"{'':35}     {k:30} {n:6} ({n*100//cnt:2}%)")
        print()

if __name__ == '__main__':
    main()
