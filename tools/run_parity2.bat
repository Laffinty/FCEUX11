@echo off
rem Parity verification tool for the Task-1 Rust harnesses.
rem
rem v1.17 Task1: the C++ harnesses (fceux11_rom_regression_test.exe /
rem fceux11_savestate_regression_test.exe / fceux11_mapper_byte_diff_test.exe /
rem fceux11_lua_runner.exe) were deleted after Rust parity was verified, so the
rem C++ legs of this script were removed. It now verifies the Rust runners
rem still execute and exit 0 (PASS) against the committed goldens.
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
set PATH=D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\bin;D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\debug\bin;%PATH%
cd /d D:\Project\FCEUX11\tests

echo === ROM RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_rom_regression_runner.exe > D:\Project\FCEUX11\build\rom_rust.out 2>&1
echo ROM_RUST_EXIT=%ERRORLEVEL%
echo === SAVE RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_savestate_regression_runner.exe > D:\Project\FCEUX11\build\save_rust.out 2>&1
echo SAVE_RUST_EXIT=%ERRORLEVEL%
echo === MAPPER RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_mapper_byte_diff_runner.exe > D:\Project\FCEUX11\build\mapper_rust.out 2>&1
echo MAPPER_RUST_EXIT=%ERRORLEVEL%
echo === LUA RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_lua_runner.exe lua_scripts/test_bit.lua > D:\Project\FCEUX11\build\lua_rust.out 2>&1
echo LUA_RUST_EXIT=%ERRORLEVEL%
