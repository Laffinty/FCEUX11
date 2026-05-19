#!/bin/bash
export PATH="/c/Users/ikrx2/.cargo/bin:$PATH"
cd /c/Users/ikrx2/Desktop/project/FCEUX11
rm -rf build
mkdir -p build
cd build
echo "=== CMake Configuration ==="
cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release 2>&1
CMAKE_RC=$?
if [ $CMAKE_RC -ne 0 ]; then
    echo "CMake configuration failed with exit code $CMAKE_RC"
    exit $CMAKE_RC
fi

echo ""
echo "=== Building ==="
make -j4 2>&1
MAKE_RC=$?
if [ $MAKE_RC -ne 0 ]; then
    echo "Build failed with exit code $MAKE_RC"
    exit $MAKE_RC
fi

echo ""
echo "=== Build Complete ==="
ls -la src/fceux11.exe 2>/dev/null && echo "SUCCESS: fceux11.exe built" || echo "ERROR: fceux11.exe not found"
exit $MAKE_RC
