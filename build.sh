#!/bin/bash
cd /c/Users/ikrx2/Desktop/project/FCEUX11
rm -rf build
mkdir -p build
cd build
cmake .. -G "MSYS Makefiles" -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)