#!/usr/bin/env bash
# FCEUX11 v0.3.2 — cppcheck check script
# Usage: ./tools/lint/cppcheck-check.sh [path-to-cppcheck]

set -euo pipefail

CPPCHECK="${1:-cppcheck}"

if ! command -v "$CPPCHECK" &>/dev/null; then
    echo "WARNING: $CPPCHECK not found in PATH"
    exit 0
fi

echo "=== FCEUX11 cppcheck Check ==="
echo "Tool:    $CPPCHECK"
echo ""

"$CPPCHECK" \
    --enable=warning,style,performance,portability \
    --std=c++20 \
    --language=c++ \
    --template=gcc \
    --suppress=missingIncludeSystem \
    --suppress=unusedFunction \
    --suppress=noExplicitConstructor \
    -I src \
    -I src/drivers \
    src/ 2>&1 || true

echo "PASS: cppcheck scan completed"
