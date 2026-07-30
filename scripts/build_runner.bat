@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d D:\Project\FCEUX11\build-c1
ninja fceux11_blargg_runner
echo EXIT=%ERRORLEVEL%
