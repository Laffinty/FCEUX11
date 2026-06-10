# FCEUX11 v0.3.6.5 — Run ctest against the ASan build
#
# tests/CMakeLists.txt already injects ENVIRONMENT_MODIFICATION on each
# test to put MSVC bin + vcpkg debug bin on PATH (so the test process
# finds clang_rt.asan_dynamic-x86_64.dll + SDL2d.dll). This script
# additionally exports PATH at the parent ctest level, which helps when
# ctest is invoked manually outside CMake (e.g. raw `ctest --test-dir`).
param(
    [string]$BuildDir = "build-asan",
    [string]$TestArgs = "--output-on-failure",
    [switch]$Rerun
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}
if (-not (Test-Path $BuildDir)) {
    throw "$BuildDir does not exist. Run scripts\_build_asan.ps1 first."
}

# Locate the MSVC bin directory from the cache (where clang_rt.asan_dynamic-x86_64.dll lives).
$cache = Join-Path $BuildDir "CMakeCache.txt"
$cxxLine = Select-String -Path $cache -Pattern "CMAKE_CXX_COMPILER:STRING=" -SimpleMatch
if (-not $cxxLine) { throw "CMAKE_CXX_COMPILER not found in $cache." }
$cl = ($cxxLine -split "=", 2)[1]
$msvcBin = Split-Path -Parent $cl

$vcpkgRel = Join-Path $ProjectRoot "vcpkg_installed\x64-windows\bin"
$vcpkgDbg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows\debug\bin"
$env:PATH = "$msvcBin;$vcpkgRel;$vcpkgDbg;$env:PATH"
Write-Host "[PATH] prepended: $msvcBin ; $vcpkgRel ; $vcpkgDbg" -ForegroundColor Gray

# ASan options - keep minimal. MSVC ASan on Windows does NOT implement
# LeakSanitizer (Linux-only); asking for detect_leaks=1 makes ASan exit
# immediately with "detect_leaks is not supported on this platform".
# Only UAF / bounds / stack-buffer-overflow / etc. checks engage here.
$env:ASAN_OPTIONS = "halt_on_error=0:abort_on_error=0"
Write-Host "[ASAN_OPTIONS] $env:ASAN_OPTIONS" -ForegroundColor Gray

$testLog = Join-Path $BuildDir "_ctest_asan.log"
$ctestArgsList = @("--test-dir", $BuildDir) + ($TestArgs -split " ")
if ($Rerun) { $ctestArgsList += "--rerun-failed" }

Write-Host "[CTEST] ctest $ctestArgsList (tee → $testLog)" -ForegroundColor Cyan
& ctest @ctestArgsList 2>&1 | Tee-Object -FilePath $testLog
$exit = $LASTEXITCODE

# MSVC ASan on Windows does not implement LeakSanitizer (see ASAN_OPTIONS
# comment above) - leaks are NOT detected, only buffer overflows / UAF /
# stack-buffer-overflow etc. v0.3.6.5 documents this explicitly.
$leakSummary = Select-String -Path $testLog -Pattern "LeakSanitizer" -SimpleMatch -Quiet
if ($leakSummary) {
    Write-Host "[LSAN] Leak summary found in $testLog - review:" -ForegroundColor Yellow
    Select-String -Path $testLog -Pattern "LeakSanitizer" -Context 0,5 | ForEach-Object { Write-Host $_ }
} else {
    Write-Host "[LSAN] No LeakSanitizer summary (MSVC Windows ASan has no LSan; UAF + bounds checks were active)." -ForegroundColor Green
}

if ($exit -ne 0) { throw "ctest failed (exit $exit). See $testLog." }
Write-Host "[SUCCESS] ASan ctest pass." -ForegroundColor Green
