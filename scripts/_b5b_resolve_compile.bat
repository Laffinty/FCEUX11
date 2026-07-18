@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
cd /d "D:\Project\FCEUX11\build"
nmake fceux11 > D:\tmp_build_merge.log 2>&1
echo ExitCode=%ERRORLEVEL%