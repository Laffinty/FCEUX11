@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
cd /d "D:\Project\FCEUX11\build"
cmake . -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release > D:\tmp_cmake.log 2>&1
echo ExitCode=%ERRORLEVEL%