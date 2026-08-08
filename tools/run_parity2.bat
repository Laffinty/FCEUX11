@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
set PATH=D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\bin;D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\debug\bin;%PATH%
cd /d D:\Project\FCEUX11\tests

echo === ROM CPP ===
D:\Project\FCEUX11\build\tests\fceux11_rom_regression_test.exe > D:\Project\FCEUX11\build\rom_cpp.out 2>&1
echo ROM_CPP_EXIT=%ERRORLEVEL%
echo === ROM RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_rom_regression_runner.exe > D:\Project\FCEUX11\build\rom_rust.out 2>&1
echo ROM_RUST_EXIT=%ERRORLEVEL%
echo === SAVE CPP ===
D:\Project\FCEUX11\build\tests\fceux11_savestate_regression_test.exe > D:\Project\FCEUX11\build\save_cpp.out 2>&1
echo SAVE_CPP_EXIT=%ERRORLEVEL%
echo === SAVE RUST ===
D:\Project\FCEUX11\build\tests\kagami_qa_savestate_regression_runner.exe > D:\Project\FCEUX11\build\save_rust.out 2>&1
echo SAVE_RUST_EXIT=%ERRORLEVEL%
