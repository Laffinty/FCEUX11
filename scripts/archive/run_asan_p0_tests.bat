@echo off
rem hotfix2 Phase A: manually run LUT + frame-diff tests under ASan.
rem The CMake CTest ENVIRONMENT_MODIFICATION (tests/CMakeLists.txt:383-395)
rem OMITS ppu_rendering_lut_test — known gap; this script supplies the PATH
rem + ASAN_OPTIONS by hand for both binaries. Run from project root.
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
    echo vcvars64.bat call failed
    exit /b 1
)
cd /d "D:\Project\FCEUX11"

rem Locate the directory containing clang_rt.asan_dynamic-x86_64.dll.
set MSVC_BIN_DIR=
for /f "delims=" %%i in ('where cl.exe 2^>nul') do (
    for %%j in ("%%~dpi\.") do set "MSVC_BIN_DIR=%%~dpj"
)
if "%MSVC_BIN_DIR%"=="" (
    for /f "delims=" %%i in ('where clang_rt.asan_dynamic-x86_64.dll 2^>nul') do (
        for %%j in ("%%~dpi\.") do set "MSVC_BIN_DIR=%%~dpj"
    )
)

set PATH=%MSVC_BIN_DIR%;%CD%\build\vcpkg_installed\x64-windows\bin;%CD%\build\vcpkg_installed\x64-windows\debug\bin;%PATH%
set ASAN_OPTIONS=halt_on_error=0:abort_on_error=0

cd /d "D:\Project\FCEUX11\tests"

echo === fceux11_ppu_rendering_lut_test (ASan) ===
..\build-asan\tests\fceux11_ppu_rendering_lut_test.exe
set LUT_EXIT=%ERRORLEVEL%
echo.   exit=%LUT_EXIT%

echo.
echo === fceux11_ppu_frame_diff_test (ASan) ===
..\build-asan\tests\fceux11_ppu_frame_diff_test.exe
set DIFF_EXIT=%ERRORLEVEL%
echo.   exit=%DIFF_EXIT%

exit /b %DIFF_EXIT%
