@echo off
REM Locate a working vcvars64.bat on this machine, preferring the same 5-path
REM list as scripts/do_build.ps1 (VS 18 BuildTools, then 2022 BuildTools/
REM Enterprise/Professional/Community). Echoes the path on stdout.
REM
REM Usage:  for /f "delims=" %%v in ('call scripts\_find_vcvars.bat') do set VCVARS=%%v
setlocal enabledelayedexpansion
set "VCVARS="
set "CANDIDATES=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat;C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat;C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat;C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat;C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
for %%p in ("%CANDIDATES:;=" "%") do (
    if exist "%%~p" (
        set "VCVARS=%%~p"
        goto :found
    )
)
:found
if defined VCVARS echo %VCVARS%
endlocal & exit /b 0
