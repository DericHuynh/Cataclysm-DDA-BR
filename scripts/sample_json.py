#!/usr/bin/env python3
"""
Extract sample JSON entries from data/core/ for a given type.
Useful for seeing the actual data format when designing Rust structs.

Usage:
  python3 scripts/sample_json.py ITEM           # show 3 random ITEM samples
  python3 scripts/sample_json.py terrain 5      # show 5 terrain samples
  python3 scripts/sample_json.py ITEM 1 --full  # full (no truncation)
"""
import json, os, sys, random

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "core")

def main():
    args = sys.argv[1:]
    dtype = args[0] if args else "ITEM"
    count = int(args[1]) if len(args) > 1 else 3
    full = "--full" in args

    samples = []
    for root, dirs, files in os.walk(DATA_DIR):
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
                    samples.append(item)

    if not samples:
        print(f"No samples found for type '{dtype}'")
        return

    selected = random.sample(samples, min(count, len(samples)))
    print(f"=== {dtype} ({len(samples)} total, showing {len(selected)}) ===\n")

    for i, s in enumerate(selected):
        # Remove type field (we know it)
        display = {k: v for k, v in s.items() if k != 'type'}
        text = json.dumps(display, indent=2, ensure_ascii=False)

        if not full and len(text) > 1500:
            text = text[:1500] + "\n  ... (truncated)"

        print(f"--- Sample {i+1} (id: {s.get('id', '<no id>')}) ---")
        print(text)
        print()

if __name__ == '__main__':
    main()
