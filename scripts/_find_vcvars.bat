@echo off
REM Locate a working vcvars64.bat on this machine.
REM v1.0: prefer vswhere.exe (Microsoft-recommended discovery — works on any
REM drive letter). Fall back to 5 hard paths for edge cases where vswhere is
REM missing. Echoes the path on stdout.
REM
REM Usage:  for /f "delims=" %%v in ('call scripts\_find_vcvars.bat') do set VCVARS=%%v
setlocal enabledelayedexpansion
set "VCVARS="

REM 1) Try vswhere.exe (recommended; works on any drive letter and any VS edition)
if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" (
    for /f "usebackq delims=" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do (
        if exist "%%i\VC\Auxiliary\Build\vcvars64.bat" (
            set "VCVARS=%%i\VC\Auxiliary\Build\vcvars64.bat"
            goto :found
        )
    )
)

REM 2) Fallback: 5 hard-coded paths (VS 18 / 2022 BuildTools/Enterprise/Professional/Community)
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
