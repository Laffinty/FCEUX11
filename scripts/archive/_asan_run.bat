@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
set PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%
set ASAN_OPTIONS=halt_on_error=0:abort_on_error=0:detect_leaks=0
cd /d D:\Project\FCEUX11\tests
..\build-asan\tests\fceux11_ppu_frame_diff_test.exe 2>&1