# FCEUX11 v0.3.6.5 - Verify that the ASan build is actually instrumented.
#
# Three independent witnesses are required to claim "real ASan":
#   1. dumpbin /symbols  finds at least one __asan_* symbol
#   2. dumpbin /imports  shows clang_rt.asan_dynamic-x86_64.dll
#   3. _build_asan.log contains zero `warning D9002`
#
# This catches the v0.3.6 PARTIAL-PASS failure mode where cl silently
# dropped /fsanitize:address and the binary built+linked+ran fine but
# contained no instrumentation.
param(
    [string]$BuildDir = "build-asan",
    [string]$TargetExe = "src\fceux11.exe"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}
$exe = Join-Path $BuildDir $TargetExe
if (-not (Test-Path $exe)) { throw "Target exe not found: $exe" }

# Locate dumpbin via the same MSVC install used to build.
$cache = Join-Path $BuildDir "CMakeCache.txt"
$cxxLine = Select-String -Path $cache -Pattern "CMAKE_CXX_COMPILER:STRING=" -SimpleMatch
if (-not $cxxLine) { throw "CMAKE_CXX_COMPILER not found in $cache." }
$cl = ($cxxLine -split "=", 2)[1]
$dumpbin = Join-Path (Split-Path -Parent $cl) "dumpbin.exe"
if (-not (Test-Path $dumpbin)) { throw "dumpbin.exe not found beside cl ($dumpbin)." }

Write-Host "=== FCEUX11 v0.3.6.5 ASan Instrumentation Verifier ===" -ForegroundColor Cyan
Write-Host "Target:  $exe"
Write-Host "dumpbin: $dumpbin"
Write-Host ""

# (1) ASan/sanitizer function imports witness.
# Note: `dumpbin /symbols` does NOT list __asan_* on a linked exe (those
# live in obj files and are pulled by the linker). The correct witness
# is the import table — instrumented code imports many __asan_* /
# __sanitizer_* thunks from clang_rt.asan_dynamic-x86_64.dll.
$importsRaw = & $dumpbin /imports $exe 2>$null
$asanImports = $importsRaw | Select-String -Pattern "__asan_|__sanitizer_"
$asanCount = ($asanImports | Measure-Object).Count
$color1 = if ($asanCount -gt 0) { "Green" } else { "Red" }
Write-Host "[1/3] __asan_/__sanitizer_ imports in $exe : $asanCount" -ForegroundColor $color1
if ($asanCount -eq 0) {
    Write-Host "      -> binary is NOT instrumented; build used a buggy flag (likely /fsanitize: colon form)." -ForegroundColor Red
}

# (2) clang_rt.asan_dynamic DLL import witness
$dllImport = $importsRaw | Select-String -Pattern "clang_rt.asan" -SimpleMatch
$hasDll = $null -ne $dllImport
$color2 = if ($hasDll) { "Green" } else { "Red" }
$state2 = if ($hasDll) { "present" } else { "MISSING" }
Write-Host "[2/3] clang_rt.asan_dynamic DLL import : $state2" -ForegroundColor $color2
if ($hasDll) { Write-Host ("      " + $dllImport.Line.Trim()) -ForegroundColor Gray }

# (3) Build log clean of D9002
$buildLog = Join-Path $BuildDir "_build_asan.log"
$d9002Count = 0
if (Test-Path $buildLog) {
    $d9002Count = (Select-String -Path $buildLog -Pattern "warning D9002" -SimpleMatch | Measure-Object).Count
}
$color3 = if ($d9002Count -eq 0) { "Green" } else { "Red" }
Write-Host "[3/3] 'warning D9002' in $buildLog : $d9002Count" -ForegroundColor $color3

Write-Host ""
if ($asanCount -gt 0 -and $hasDll -and $d9002Count -eq 0) {
    Write-Host "[VERDICT] REAL ASan instrumentation confirmed. OK" -ForegroundColor Green
    exit 0
} else {
    Write-Host "[VERDICT] ASan instrumentation FAILED - see witnesses above." -ForegroundColor Red
    exit 1
}
