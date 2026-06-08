#!/usr/bin/env bash
# FCEUX11 v0.3.2 — clang-tidy check script
# Usage: ./tools/lint/clang-tidy-check.sh [path-to-clang-tidy]

set -euo pipefail

CLANG_TIDY="${1:-clang-tidy}"
BUILD_DIR="${2:-build}"

if ! command -v "$CLANG_TIDY" &>/dev/null; then
    echo "WARNING: $CLANG_TIDY not found in PATH"
    exit 0
fi

echo "=== FCEUX11 clang-tidy Check ==="
echo "Tool:    $CLANG_TIDY"
echo "Build:   $BUILD_DIR"
echo ""

# Require compile_commands.json
if [[ ! -f "$BUILD_DIR/compile_commands.json" ]]; then
    echo "WARNING: compile_commands.json not found in $BUILD_DIR"
    echo "  Re-run cmake with -DCMAKE_EXPORT_COMPILE_COMMANDS=ON"
    exit 0
fi

find src -type f \( -name "*.cpp" -o -name "*.c" \) -print0 | \
    xargs -0 "$CLANG_TIDY" -p "$BUILD_DIR" --quiet

echo "PASS: clang-tidy check completed"
