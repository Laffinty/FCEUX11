@echo off
rem hotfix2 Phase A verification: ASan build (Release + /fsanitize=address)
rem Mirrors scripts\build_full.bat but with -DFCEUX11_ASAN=ON. Output to build-asan/.
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
    echo vcvars64.bat call failed
    exit /b 1
)
cd /d "D:\Project\FCEUX11"

if not exist "build\vcpkg_installed\x64-windows\tools" (
    if not exist "vcpkg_installed\x64-windows\tools" (
        echo vcpkg_installed not found at build\ or project root
        exit /b 1
    )
)

if exist build-asan (
    echo [CLEAN] Removing stale build-asan
    rmdir /s /q build-asan
)

set VCPKG_DIR=%CD%\build\vcpkg_installed\x64-windows
set VCPKG_BIN=%VCPKG_DIR%\bin
set VCPKG_DBG=%VCPKG_DIR%\debug\bin

cmake -S . -B build-asan -G "NMake Makefiles" ^
    -DCMAKE_BUILD_TYPE=Release ^
    -DCMAKE_C_COMPILER=cl ^
    -DCMAKE_CXX_COMPILER=cl ^
    -DCMAKE_PREFIX_PATH="%VCPKG_DIR%" ^
    -DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE ^
    -DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE ^
    -DFCEUX11_BUILD_TESTS=ON ^
    -DENABLE_LINT_CPPCHECK=OFF ^
    -DFCEUX11_ASAN=ON
if errorlevel 1 exit /b 1

cd build-asan
nmake /f Makefile 2>&1
exit /b %errorlevel%
