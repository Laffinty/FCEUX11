@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set VCPKG_ROOT=D:\vcpkg
cmake --build build --config Release
