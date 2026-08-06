# FCEUX11 v0.3.6.5 — Run ctest against the UB-substitute build
#
# Like _ctest_asan.ps1 but no LSan/ASan env probing (MSVC native UB
# checks don't have a runtime that prints summaries). RTC failures
# surface as MessageBox + non-zero exit; ctest catches the latter.
param(
    [string]$BuildDir = "build-ubsan",
    [string]$TestArgs = "--output-on-failure"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}
if (-not (Test-Path $BuildDir)) {
    throw "$BuildDir does not exist. Run scripts\_build_ubsan.ps1 first."
}

$cache = Join-Path $BuildDir "CMakeCache.txt"
$cxxLine = Select-String -Path $cache -Pattern "CMAKE_CXX_COMPILER:STRING=" -SimpleMatch
if (-not $cxxLine) { throw "CMAKE_CXX_COMPILER not found in $cache." }
$cl = ($cxxLine -split "=", 2)[1]
$msvcBin = Split-Path -Parent $cl

$vcpkgRel = Join-Path $ProjectRoot "vcpkg_installed\x64-windows\bin"
$vcpkgDbg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows\debug\bin"
$env:PATH = "$msvcBin;$vcpkgRel;$vcpkgDbg;$env:PATH"
Write-Host "[PATH] prepended: $msvcBin ; $vcpkgRel ; $vcpkgDbg" -ForegroundColor Gray

# /RTC failure handling: ctest will see the non-zero exit code from a
# violated /RTC check and fail the test. The previous draft set
# `_NO_DEBUG_HEAP=1` here under the misbelief that it suppressed RTC
# MessageBoxes — actually that var DISABLES the debug heap, which also
# disables /RTC1's heap-canary checks. The v0.3.6.5 code review
# (docs/tech/v0.3.x_CodeReview_6.5.md Q5) flags this as the wrong var.
# If interactive MessageBox suppression is later needed on a developer
# workstation, do it via `_CrtSetReportMode(_CRT_ERROR, _CRTDBG_MODE_DEBUG)`
# in the test exe init or `SetErrorMode(SEM_NOGPFAULTERRORBOX)` — do NOT
# reintroduce `_NO_DEBUG_HEAP=1` here.

$testLog = Join-Path $BuildDir "_ctest_ubsan.log"
$ctestArgsList = @("--test-dir", $BuildDir) + ($TestArgs -split " ")

Write-Host "[CTEST] ctest $ctestArgsList (tee → $testLog)" -ForegroundColor Cyan
& ctest @ctestArgsList 2>&1 | Tee-Object -FilePath $testLog
$exit = $LASTEXITCODE

if ($exit -ne 0) { throw "ctest failed (exit $exit). See $testLog." }
Write-Host "[SUCCESS] UB-substitute ctest pass." -ForegroundColor Green
