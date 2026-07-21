@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
set PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%
cd /d D:\Project\FCEUX11
cmake --build build-asan --config Release --target fceux11_ppu_phase_d_test fceux11_golden_savestate_test fceux11_config_store_test 2>&1