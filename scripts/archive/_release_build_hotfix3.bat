@echo off
setlocal
rem Release build for v1.15(hotfix3). Run from repo root or anywhere.
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (echo [build] vcvars64 failed & exit /b 1)
cd /d "D:\Project\FCEUX11\build"
cmake --build . --config Release 2>&1
echo [build] exit=%errorlevel%
endlocal
