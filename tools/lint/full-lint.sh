#!/usr/bin/env bash
# FCEUX11 v0.3.2 — Full lint suite entry point
# Runs: format-check, clang-tidy, cppcheck

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="${1:-$PROJECT_ROOT/build}"

echo "========================================"
echo "FCEUX11 v0.3.2 — Full Lint Suite"
echo "Project: $PROJECT_ROOT"
echo "Build:   $BUILD_DIR"
echo "========================================"
echo ""

echo "[1/3] Format check..."
bash "$SCRIPT_DIR/format-check.sh"
echo ""

echo "[2/3] clang-tidy check..."
bash "$SCRIPT_DIR/clang-tidy-check.sh" clang-tidy "$BUILD_DIR"
echo ""

echo "[3/3] cppcheck scan..."
bash "$SCRIPT_DIR/cppcheck-check.sh"
echo ""

echo "========================================"
echo "Lint suite complete."
echo "========================================"
