@echo off
REM KagamiQA P2 — build C++ blargg runner and run Oracle B suite.
REM
REM Run this from a Visual Studio Developer Command Prompt:
REM   Start → Visual Studio 2026 → Developer Command Prompt for VS 2026
REM   cd /d D:\Project\FCEUX11
REM   scripts\build_blargg_runner.bat
REM
REM Or run from PowerShell after calling VsDevCmd.ps1:
REM   & 'D:\Program Files\Microsoft Visual Studio\18\Community\Common7\Tools\Launch-VsDevShell.ps1'
REM   .\scripts\build_blargg_runner.bat

setlocal enabledelayedexpansion
set ROOT=%~dp0..
cd /d "%ROOT%"

echo === KagamiQA P2: Build blargg runner ===

:: Step 1: CMake reconfigure (pick up new target)
echo.
echo [1/3] Reconfiguring CMake...
cd build
if exist CMakeCache.txt del CMakeCache.txt
if exist CMakeFiles rmdir /s /q CMakeFiles
cmake .. -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release
if %ERRORLEVEL% neq 0 (
    echo ERROR: CMake configure failed — make sure you're in a VS Developer Command Prompt.
    exit /b 1
)
echo   CMake configure OK

:: Step 2: Build blargg runner
echo.
echo [2/3] Building fceux11_blargg_runner...
nmake fceux11_blargg_runner
if %ERRORLEVEL% neq 0 (
    echo ERROR: Build failed.
    exit /b 1
)
echo   Build OK

:: Step 3: Quick smoke test
echo.
echo [3/3] Smoke test with nestest.nes...
cd ..\tests
if exist "..\build\tests\fceux11_blargg_runner.exe" (
    set RUNNER=..\build\tests\fceux11_blargg_runner.exe
) else (
    echo WARNING: Binary not found. Check build output.
    dir /s ..\build\tests\fceux11_blargg_runner*
    exit /b 1
)

echo.
echo --- Smoke test ---
!RUNNER! --rom fixtures/nestest.nes --frames 60
echo Smoke exit code: %ERRORLEVEL%

cd ..
echo.
echo === Build complete ===
echo.
echo Next steps:
echo   1. Download ROMs:  powershell -File scripts\download_blargg_roms.ps1
echo   2. Run full suite: build\tests\fceux11_blargg_runner --manifest tests\fixtures\blargg_manifest.json
echo   3. Build Rust:     cd src\rust ^&^& cargo build --package kagami-qa --bin kagami-qa-runner --release
echo   4. Generate table: .\src\rust\target\release\kagami-qa-runner --manifest tests\tests.json --bin-dir build\tests --accuracy-table docs\FCEUX11-1.16_KagamiQA-P2-accuracy-table.md
endlocal
