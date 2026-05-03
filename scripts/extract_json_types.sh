#!/usr/bin/env bash
# Extract all unique JSON "type" values from data/core/
# Useful for discovering what definition types exist.
set -e

DATA_DIR="${1:-data/core}"

if [ ! -d "$DATA_DIR" ]; then
    echo "Usage: $0 [path-to-data-dir]"
    echo "Default: data/core"
    exit 1
fi

echo "=== Unique JSON types in $DATA_DIR ==="
find "$DATA_DIR" -name '*.json' -exec grep -rho '"type"[[:space:]]*:[[:space:]]*"[^"]*"' {} \; \
    | sed 's/"type"\s*:\s*"//;s/"//' \
    | sort -u
echo ""
echo "Total: $(find "$DATA_DIR" -name '*.json' -exec grep -rho '"type"[[:space:]]*:[[:space:]]*"[^"]*"' {} \; \
    | sed 's/"type"\s*:\s*"//;s/"//' \
    | sort -u | wc -l) unique types"
