@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
set PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%
cd /d D:\Project\FCEUX11
cmake --build build --target fceux11_mapper_byte_diff_test --config Release 2>&1