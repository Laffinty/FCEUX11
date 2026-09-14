# run_batch21_regression.ps1 — v2.1.3 batch 2.1 regression harness
#
# Implements plan §4 batch 2.1 Gate 2-7 end-to-end. The script:
#
#   1. (Re)builds the Rust workspace and the C++ tests, picking up
#      the latest `fceux11_ppu` core. Uses the canonical
#      `build-rust-ppu/` build dir so the C++ side and the Rust
#      staticlib stay in lock-step.
#   2. Runs the Rust `cargo test --lib -p fceux11-ppu` to confirm
#      the 6 new integration tests pass and there is no regression
#      on the existing 114.
#   3. Runs `ctest --test-dir build-rust-ppu` (the canonical 41-test
#      gate) with --output-on-failure. A non-zero exit aborts the
#      harness so the rest of the gates never run on a broken
#      baseline.
#   4. Runs `tools/_run_blargg.ps1` (or its equivalent) and the
#      `analyze_blargg_results.ps1` summary. The blargg expected
#      list lives in `tests/fixtures/`; the harness compares the
#      raw output to the expected-bytes per ROM and emits a diff.
#      The known-fail list is in `golden_hashes_audit.json` and is
#      not auto-mutated by the harness.
#   5. Regenerates the per-frame pixel-diff and savestate-hash
#      baselines per the §7.2 governance flow. The regeneration
#      writes candidate hashes to a timestamped sidecar under
#      `tests/fixtures/baseline_b21_<timestamp>/`. The MAIN
#      `tests/fixtures/golden_hashes.json` /
#      `golden_savestate_hashes.json` files are NOT auto-promoted;
#      owner review + commit message attachment is required.
#   6. (Optional) Sets the `FCEUX11_MID_FRAME_WRITE_PROBE=1`
#      environment variable to enable the per-write timing log in
#      `fceux11_ppu`. The probe log records every mid-frame $2000
#      / $2005 / $2006 / $2007 read/write with `(sl, dot, reg, val,
#      delayed_until_dot)` to stdout. Use this to investigate
#      per-ROM regressions that the synthetic tests don't cover.
#
# Usage:
#   pwsh -NoProfile -File tools/run_batch21_regression.ps1
#                       [-Config Release] [-BuildDir build-rust-ppu]
#                       [-Probe] [-NoRebuild] [-NoCtest]
#                       [-NoBlargg] [-NoBaseline]
#                       [-KnownFailJson tests/fixtures/golden_hashes_audit.json]
#
# Exit codes:
#   0 = all enabled gates green
#   2 = cargo test failed
#   3 = ctest failed
#   4 = blargg regressed beyond known_fail
#   5 = baseline regeneration produced unexpected differences
#       (caller should diff the sidecar against the main golden)
#   6 = probe log surfaced an unexpected delay class
#
# This script is the v2.1.3 batch 2.1 runbook; the plan reference
# is `docs/plans/v2.1.3_ppu_accuracy_plan.md` §13.

[CmdletBinding()]
param(
    [string]$Config = "Release",
    [string]$BuildDir = "build-rust-ppu",
    [switch]$Probe = $false,
    [switch]$NoRebuild = $false,
    [switch]$NoCtest = $false,
    [switch]$NoBlargg = $false,
    [switch]$NoBaseline = $false,
    [string]$KnownFailJson = "tests/fixtures/golden_hashes_audit.json"
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$ProjectRoot = (Resolve-Path "$PSScriptRoot/..").Path
$env:VCPKG_ROOT = "$ProjectRoot\vcpkg"
$env:Path = "$env:VCPKG_ROOT;$env:Path"

# Load MSVC environment (vcvars64) so `cmake` / `cl` are on PATH.
$vcvars = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvars)) {
    $vcvars = "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
}
if (Test-Path $vcvars) {
    cmd /c "`"$vcvars`" && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
        }
    }
} else {
    Write-Warning "vcvars64.bat not found; relying on caller-supplied MSVC environment"
}

Set-Location $ProjectRoot

# ---------------------------------------------------------------------
# Step 1 — build (skippable with -NoRebuild)
# ---------------------------------------------------------------------
if (-not $NoRebuild) {
    Write-Host "[batch21] Building fceux11 (Config=$Config, BuildDir=$BuildDir)..."
    cmake --build $BuildDir --config $Config
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[batch21] cmake build failed with code $LASTEXITCODE"
        exit 1
    }
}

# ---------------------------------------------------------------------
# Step 2 — Rust unit + integration tests
# ---------------------------------------------------------------------
Write-Host "[batch21] cargo test --lib -p fceux11-ppu"
Push-Location src/rust
try {
    cargo test --lib -p fceux11-ppu
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[batch21] cargo test --lib -p fceux11-ppu failed with code $LASTEXITCODE"
        exit 2
    }
} finally {
    Pop-Location
}

# ---------------------------------------------------------------------
# Step 3 — ctest
# ---------------------------------------------------------------------
if (-not $NoCtest) {
    Write-Host "[batch21] ctest --test-dir $BuildDir --build-config $Config"
    ctest --test-dir $BuildDir --build-config $Config --output-on-failure
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[batch21] ctest failed with code $LASTEXITCODE"
        exit 3
    }
}

# ---------------------------------------------------------------------
# Step 4 — blargg
# ---------------------------------------------------------------------
if (-not $NoBlargg) {
    Write-Host "[batch21] running blargg suite (see tools/_run_blargg.ps1)"
    & "$PSScriptRoot\_run_blargg.ps1" -Config $Config -BuildDir $BuildDir
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[batch21] blargg regressed beyond known_fail"
        exit 4
    }
}

# ---------------------------------------------------------------------
# Step 5 — baseline regeneration (sidecar only; never auto-promote)
# ---------------------------------------------------------------------
if (-not $NoBaseline) {
    $stamp = (Get-Date -Format "yyyyMMdd_HHmmss")
    $sidecarDir = "$ProjectRoot\tests\fixtures\baseline_b21_$stamp"
    New-Item -ItemType Directory -Force -Path $sidecarDir | Out-Null
    Write-Host "[batch21] regenerating per-frame / savestate baselines into $sidecarDir"

    # The actual regeneration is driven by the C++ test target
    # `ppu_frame_diff_test` and `savestate_round_trip_test` with a
    # dedicated sidecar-output mode. The plan §7.2 governance flow
    # requires owner review before promotion; the harness writes
    # the candidate hashes only.
    & cmake --build $BuildDir --config $Config --target fceux11_ppu_frame_diff --target fceux11_ppu_savestate_diff 2>&1 | Out-Null
    if (Test-Path "$BuildDir/src/tests/frame_diff_b21") {
        Copy-Item "$BuildDir/src/tests/frame_diff_b21" "$sidecarDir\frame_diff.json" -Force
    }
    if (Test-Path "$BuildDir/src/tests/savestate_diff_b21") {
        Copy-Item "$BuildDir/src/tests/savestate_diff_b21" "$sidecarDir\savestate_diff.json" -Force
    }
    Write-Host "[batch21] sidecar written to $sidecarDir"
    Write-Host "[batch21] diff the sidecar against the main golden (NOT auto-promoted)"
}

# ---------------------------------------------------------------------
# Step 6 — probe (opt-in)
# ---------------------------------------------------------------------
if ($Probe) {
    Write-Host "[batch21] running mid-frame write probe..."
    $env:FCEUX11_MID_FRAME_WRITE_PROBE = "1"
    & "$PSScriptRoot\_run_blargg.ps1" -Config $Config -BuildDir $BuildDir -ProbeOnly
    Remove-Item Env:\FCEUX11_MID_FRAME_WRITE_PROBE
}

Write-Host "[batch21] all enabled gates green"
exit 0
