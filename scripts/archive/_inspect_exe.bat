@echo off
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
echo === where dumpbin ===
where dumpbin
echo.
echo === dumpbin /summary ===
dumpbin /summary "D:\Project\FCEUX11\build\src\fceux11.exe"
echo.
echo === dumpbin /headers (top 40 lines) ===
dumpbin /headers "D:\Project\FCEUX11\build\src\fceux11.exe" | findstr /N "^" | findstr /R "^\[1\] ^[1-3]"
echo.
echo === dumpbin /versioninfo ===
dumpbin /versioninfo "D:\Project\FCEUX11\build\src\fceux11.exe"
echo.
echo === dumpbin /dependents (top 40 lines) ===
dumpbin /dependents "D:\Project\FCEUX11\build\src\fceux11.exe" | findstr /N "^" | findstr /R "^\[1\] ^[1-4]"
endlocal
