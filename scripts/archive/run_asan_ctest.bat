@echo off
rem hotfix2 Phase A: ctest under ASan.
rem Mirrors scripts\_ctest_asan.ps1 but using the existing build-asan tree.
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
    echo vcvars64.bat call failed
    exit /b 1
)
cd /d "D:\Project\FCEUX11"

rem Find MSVC bin dir (where clang_rt.asan_dynamic-x86_64.dll lives).
set "MSVC_BIN_DIR="
for /f "delims=" %%i in ('where cl.exe 2^>nul') do (
    for %%j in ("%%~dpi\.") do set "MSVC_BIN_DIR=%%~dpj"
)

set "PATH=%MSVC_BIN_DIR%;%CD%\build\vcpkg_installed\x64-windows\bin;%CD%\build\vcpkg_installed\x64-windows\debug\bin;%PATH%"
set "ASAN_OPTIONS=halt_on_error=0:abort_on_error=0"

cd /d "D:\Project\FCEUX11\build-asan"
ctest --output-on-failure -j4 -LE perf
exit /b %errorlevel%
