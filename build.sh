#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

BUILD_DIR="build"
MAX_RETRIES=2

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

echo "=== CMake Configure ==="
cmake .. -G "MSYS Makefiles" -DCMAKE_BUILD_TYPE=Release
if [ $? -ne 0 ]; then
    echo "ERROR: CMake configuration failed" >&2
    exit 1
fi

echo "=== Build (attempt 1/${MAX_RETRIES}) ==="
make -j$(nproc) 2>&1
if [ $? -eq 0 ]; then
    echo "=== Build succeeded ==="
    exit 0
fi

for attempt in $(seq 2 $MAX_RETRIES); do
    echo "=== Build failed, retrying (attempt ${attempt}/${MAX_RETRIES}) ==="
    make -j$(nproc) 2>&1
    if [ $? -eq 0 ]; then
        echo "=== Build succeeded on attempt ${attempt} ==="
        exit 0
    fi
done

echo "ERROR: Build failed after ${MAX_RETRIES} attempts" >&2
exit 1
