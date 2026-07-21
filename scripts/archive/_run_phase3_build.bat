@echo off
REM Phase 3 build helper: load vcvars64 + configure + build in one cmd session
REM so PATH/INCLUDE/LIB inheritance is preserved (PowerShell Set-Item drops
REM PATH values containing `=`).

call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1

set "PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%"

cd /d D:\Project\FCEUX11

echo === Reconfigure ===
cmake -S . -B build -G Ninja ^
  -DCMAKE_BUILD_TYPE=Release ^
  -DCMAKE_C_COMPILER=cl ^
  -DCMAKE_CXX_COMPILER=cl ^
  -DCMAKE_PREFIX_PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows ^
  -DVCPKG_MANIFEST_MODE=OFF ^
  -DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE ^
  -DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE
if errorlevel 1 exit /b 1

echo === Build ===
cmake --build build --config Release
if errorlevel 1 exit /b 1

echo === DONE ===
