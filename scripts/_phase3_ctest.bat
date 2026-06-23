@echo off
REM Phase 3 verification: run ctest on the lib changes (bus_instance -> g_bus).
REM Mirrors plan §5.1.2 verification commands.

call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1

set "PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%"

cd /d D:\Project\FCEUX11

echo === Plan §5.1.2 — core tests (bus_test / cpu_test / savestate_core_test / golden_savestate_test) ===
ctest --test-dir build -C Release -R "bus_test|cpu_test|savestate_core_test|golden_savestate_test" --output-on-failure
if errorlevel 1 (
    echo CORE TESTS FAILED
    exit /b 1
)

echo === Full ctest suite ===
ctest --test-dir build -C Release --output-on-failure
if errorlevel 1 (
    echo FULL CTEST FAILED
    exit /b 1
)

echo === CTEST OK ===
