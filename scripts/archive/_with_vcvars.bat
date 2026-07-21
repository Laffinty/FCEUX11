@echo off
REM Helper: load vcvars64 + run any command in that env.
REM Auto-discovers vcvars64.bat via scripts\_find_vcvars.bat (5-path fallback
REM matching do_build.ps1, no hard-coded VS 18 BuildTools path).
REM Usage:  scripts\_with_vcvars.bat <command...>
for /f "delims=" %%v in ('call "%~dp0_find_vcvars.bat"') do set "VCVARS=%%v"
if "%VCVARS%"=="" (
    echo [_with_vcvars] no vcvars64.bat found in standard locations >&2
    exit /b 1
)
call "%VCVARS%" >nul 2>&1
if errorlevel 1 (
    echo [_with_vcvars] vcvars64.bat failed at %VCVARS% >&2
    exit /b 1
)
%*
