@echo off
REM FCEUX11 v0.3.10 — Probe that local cl accepts std::span / std::byte in C++20.
REM
REM Verifies:
REM   1. cl /std:c++20 compiles a snippet using <span> and std::byte.
REM   2. cl /std:c++20 compiles a snippet using std::span<std::byte> EMUFILE-like API.
REM   3. Rejects implicit arithmetic on std::byte (negative probe).
REM
REM Auto-discovers vcvars64.bat via _find_vcvars.bat.
setlocal enabledelayedexpansion

for /f "delims=" %%v in ('call "%~dp0_find_vcvars.bat"') do set "VCVARS=%%v"
if "%VCVARS%"=="" (
    echo VCVARS_FAIL
    exit /b 1
)
call "%VCVARS%" >nul 2>&1
if errorlevel 1 (
    echo VCVARS_FAIL
    exit /b 1
)

echo === cl version ===
cl 2>&1 | findstr /R /C:"Microsoft" /C:"Compiler Version"
echo.

set "TMPDIR=%TEMP%\fceux11_span_byte_probe"
if not exist "%TMPDIR%" mkdir "%TMPDIR%"

REM (1) Positive probe: std::byte + std::span basic usage.
(
echo #include ^<span^>
echo #include ^<cstddef^>
echo #include ^<vector^>
echo int main^(^) {
echo     std::vector^<std::byte^> v^(4^);
echo     std::span^<std::byte^> s^(v^);
echo     s[0] = std::byte{42};
echo     auto u = std::to_integer^<unsigned char^>(s[0]^);
echo     return static_cast^<int^>(u^);
echo }
) > "%TMPDIR%\probe_span_byte.cpp"

echo === probe std::span + std::byte (positive) ===
cl /nologo /std:c++20 /EHsc /Fe:"%TMPDIR%\probe_span_byte.exe" "%TMPDIR%\probe_span_byte.cpp" 2>&1
echo exit:%errorlevel%
if %errorlevel% neq 0 (
    echo [FAIL] std::span / std::byte positive probe failed.
    exit /b 1
)
echo.

REM (2) Positive probe: EMUFILE-like read/write with std::span.
(
echo #include ^<span^>
echo #include ^<cstddef^>
echo #include ^<vector^>
echo #include ^<cstring^>
echo class EmuFileLike {
echo public:
echo     virtual size_t fread^(std::span^<std::byte^> dst^) = 0;
echo     virtual void fwrite^(std::span^<const std::byte^> src^) = 0;
echo };
echo int main^(^) { return 0; }
) > "%TMPDIR%\probe_emufile_like.cpp"

echo === probe EMUFILE-like API (positive) ===
cl /nologo /std:c++20 /EHsc /c /Fo:"%TMPDIR%\probe_emufile_like.obj" "%TMPDIR%\probe_emufile_like.cpp" 2>&1
echo exit:%errorlevel%
if %errorlevel% neq 0 (
    echo [FAIL] EMUFILE-like API positive probe failed.
    exit /b 1
)
echo.

REM (3) Negative probe: arithmetic on std::byte must fail.
(
echo #include ^<cstddef^>
echo int main^(^) {
echo     std::byte b = std::byte{1};
echo     b = b + std::byte{1};
echo     return 0;
echo }
) > "%TMPDIR%\probe_byte_arith.cpp"

echo === probe std::byte arithmetic (negative, should FAIL to compile) ===
cl /nologo /std:c++20 /EHsc /c /Fo:"%TMPDIR%\probe_byte_arith.obj" "%TMPDIR%\probe_byte_arith.cpp" 2>&1
echo exit:%errorlevel%
if %errorlevel% equ 0 (
    echo [FAIL] std::byte arithmetic compiled unexpectedly; compiler may not enforce std::byte semantics.
    exit /b 1
) else (
    echo [OK] std::byte arithmetic correctly rejected.
)
echo.

echo [PASS] MSVC std::span / std::byte probes succeeded.
exit /b 0
