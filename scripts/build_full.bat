@echo off
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
    echo vcvars64.bat call failed
    exit /b 1
)
cd /d "D:\Project\FCEUX11\build"
nmake /f Makefile fceux11 2>&1 | findstr /v "warning" | findstr /v "Note:" | findstr /v "^$" | findstr /v "D9025"
exit /b %errorlevel%
