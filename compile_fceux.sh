#!/bin/bash
export PATH="/d/msys64/mingw64/bin:/d/msys64/usr/bin:$PATH"
cd /c/Users/ikrx2/Desktop/project/FCEUX11
rm -rf build
mkdir -p build
cd build
cmake .. -G "MSYS Makefiles" -DCMAKE_BUILD_TYPE=Release
if [ $? -ne 0 ]; then
    echo "CMake failed"
    exit 1
fi
make -j8