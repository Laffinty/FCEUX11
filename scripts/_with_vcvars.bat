@echo off
REM Helper: load VS 18 BuildTools vcvars64 + run any command in that env.
REM Usage:  scripts\_with_vcvars.bat <command...>
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
if errorlevel 1 (
    echo [_with_vcvars] vcvars64.bat failed >&2
    exit /b 1
)
%*
