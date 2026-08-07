@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
set PATH=D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\bin;D:\Project\FCEUX11\build\vcpkg_installed\x64-windows\debug\bin;%PATH%
cd /d D:\Project\FCEUX11\tests
echo === CPP RUN ===
D:\Project\FCEUX11\build\tests\fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json > D:\Project\FCEUX11\build\parity_cpp.json 2> D:\Project\FCEUX11\build\parity_cpp.stderr
echo CPP_EXIT=%ERRORLEVEL%
echo === RUST RUN ===
D:\Project\FCEUX11\build\tests\kagami_qa_blargg_runner.exe --manifest fixtures/blargg_manifest.json > D:\Project\FCEUX11\build\parity_rust.json 2> D:\Project\FCEUX11\build\parity_rust.stderr
echo RUST_EXIT=%ERRORLEVEL%
