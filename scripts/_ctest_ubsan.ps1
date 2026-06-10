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

# Suppress interactive RTC MessageBoxes so ctest stays headless. RTC
# still writes the failure to stderr and exits with a non-zero code.
$env:_NO_DEBUG_HEAP = "1"

$testLog = Join-Path $BuildDir "_ctest_ubsan.log"
$ctestArgsList = @("--test-dir", $BuildDir) + ($TestArgs -split " ")

Write-Host "[CTEST] ctest $ctestArgsList (tee → $testLog)" -ForegroundColor Cyan
& ctest @ctestArgsList 2>&1 | Tee-Object -FilePath $testLog
$exit = $LASTEXITCODE

if ($exit -ne 0) { throw "ctest failed (exit $exit). See $testLog." }
Write-Host "[SUCCESS] UB-substitute ctest pass." -ForegroundColor Green
