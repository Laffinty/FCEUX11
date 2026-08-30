# Phase 6 kickoff regression matrix (Oracle A via the kagami-qa runner).
#
# History: the original version of this script was written against the
# Phase-0-era layout (D:\Project\FCEUX11, build-c1, src/rust/target).
# Rewritten for the current layout in the Phase 5 closeout (plan §Phase 5
# closeout gate: "Phase 6 开工前必须自测一遍 scripts/run_matrix.ps1").
#
# Usage:
#   .\scripts\run_matrix.ps1              # run the manifest matrix, write the report
#   .\scripts\run_matrix.ps1 -Baseline    # additionally compare against the
#                                         #   frozen Phase-0 Oracle-A baseline
#                                         #   (expected to differ: golden sets
#                                         #   were regenerated under plan v2.3)
param(
    [switch]$Baseline
)
$ErrorActionPreference = "Continue"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot
$env:PATH = "$ProjectRoot\build\tests;$ProjectRoot\vcpkg_installed\x64-windows\bin;$ProjectRoot\vcpkg_installed\x64-windows\debug\bin;" + $env:PATH

$runner = "$ProjectRoot\build\src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe"
if (-not (Test-Path $runner)) {
    # Cargo output path is pinned by src/rust/.cargo/config.toml to
    # target/x86_64-pc-windows-msvc/<profile>/ (NOT target/release).
    Write-Host "[ERROR] kagami-qa-runner.exe not found at $runner"
    Write-Host "        Build it with (from a Developer PowerShell):"
    Write-Host '          $env:CARGO_TARGET_DIR = "<root>\build\src\rust\target"'
    Write-Host '          cargo build --release -p kagami-qa --bin kagami-qa-runner'
    exit 2
}

# NOTE (S-4): the git revision is stamped at COMPILE time by
# src/rust/crates/kagami-qa/build.rs (report/matrix.rs uses option_env!).
# Setting FCEUX11_GIT_REV here would be too late to affect the report, so it
# is deliberately not set. Rebuild the runner to re-stamp.

$baselineArgs = @()
if ($Baseline) {
    $baselineArgs = @("--baseline", "$ProjectRoot\tests\fixtures\kagamiqa_baseline_frozen.json")
}

& $runner `
    --manifest "$ProjectRoot\tests\tests.json" `
    --bin-dir  "$ProjectRoot\build\tests" `
    --output   "$ProjectRoot\build\kagamiqa_migration_matrix.json" `
    @baselineArgs
Write-Host ("EXIT=" + $LASTEXITCODE)
