#!/usr/bin/env bash
# FCEUX11 v0.3.0 — Format check script (CI entry point)
# Usage: ./tools/lint/format-check.sh [path-to-clang-format]

set -euo pipefail

CLANG_FORMAT="${1:-clang-format}"
SRC_DIR="src"

if ! command -v "$CLANG_FORMAT" &>/dev/null; then
    echo "ERROR: $CLANG_FORMAT not found in PATH"
    exit 1
fi

echo "=== FCEUX11 Format Check ==="
echo "Formatter: $CLANG_FORMAT"
echo "Source:    $SRC_DIR"
echo ""

# Dry-run check with error on mismatch
find "$SRC_DIR" \
    -type f \
    \( -name "*.cpp" -o -name "*.h" -o -name "*.c" -o -name "*.hpp" \) \
    -exec "$CLANG_FORMAT" --dry-run --Werror {} + \
    && echo "PASS: All files match .clang-format" \
    || { echo "FAIL: Format violations detected"; exit 1; }
