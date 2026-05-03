#!/usr/bin/env bash
# Summary of all JSON definition types in data/core with counts
set -e

DATA_DIR="${1:-data/core}"

if [ ! -d "$DATA_DIR" ]; then
    echo "Usage: $0 [path-to-data-dir]"
    exit 1
fi

echo "=== Definition type counts in $DATA_DIR ==="
find "$DATA_DIR" -name '*.json' -exec python3 -c "
import json, sys, os
from collections import Counter

c = Counter()
for path in sys.argv[1:]:
    try:
        with open(path) as f:
            data = json.load(f)
    except:
        continue
    items = data if isinstance(data, list) else [data]
    for item in items:
        if isinstance(item, dict):
            t = item.get('type', '<no type>')
            c[t] += 1

for t, n in c.most_common():
    print(f'{n:>6}  {t}')
" {} +
