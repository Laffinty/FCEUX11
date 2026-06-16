@echo off
rem %~dp0 = this script's directory (ends with a backslash).
rem Project root is one level up; build dir sits at <repo>\build.
cd /d "%~dp0..\build"
"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"
cmake --build . --config Release