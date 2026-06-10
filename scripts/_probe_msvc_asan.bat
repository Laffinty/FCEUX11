@echo off
REM Probe whether local cl accepts /fsanitize=address (= form, MSVC official)
REM and /fsanitize=undefined (NOT expected — clang-only)
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
if errorlevel 1 (
    echo VCVARS_FAIL
    exit /b 1
)
echo === cl version ===
cl 2>&1 | findstr /R /C:"Microsoft" /C:"Compiler Version"
echo.
echo === probe /fsanitize=address ===
echo int main(){return 0;} > %TEMP%\probe_asan.c
cl /nologo /c /fsanitize=address %TEMP%\probe_asan.c /Fo%TEMP%\probe_asan.obj 2>&1
echo exit:%errorlevel%
echo.
echo === probe /fsanitize=undefined ===
cl /nologo /c /fsanitize=undefined %TEMP%\probe_asan.c /Fo%TEMP%\probe_ubsan.obj 2>&1
echo exit:%errorlevel%
echo.
echo === probe /fsanitize:address (legacy buggy form) ===
cl /nologo /c /fsanitize:address %TEMP%\probe_asan.c /Fo%TEMP%\probe_legacy.obj 2>&1
echo exit:%errorlevel%
echo.
echo === ASan runtime DLL presence ===
where clang_rt.asan_dynamic-x86_64.dll 2>&1
echo.
echo === ASan lib presence ===
for /f "delims=" %%i in ('dir /b /s "%VCToolsInstallDir%lib\x64\clang_rt.asan*.lib" 2^>nul') do echo %%i
del /q %TEMP%\probe_asan.c %TEMP%\probe_asan.obj %TEMP%\probe_ubsan.obj %TEMP%\probe_legacy.obj 2>nul
