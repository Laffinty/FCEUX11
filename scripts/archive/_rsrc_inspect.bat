@echo off
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
echo === Resource section full contents ===
dumpbin /SECTION:.rsrc /RAWDATA "D:\Project\FCEUX11\build\src\fceux11.exe" | findstr /N "^" | findstr /R "^\[1\] ^[1-9][0-9]:"
echo.
echo === Resource directory tree ===
dumpbin /RC "D:\Project\FCEUX11\build\src\fceux11.exe" 2>&1 | head -30
endlocal
