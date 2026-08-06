# KagamiQA P2 — Build C++ blargg runner and run full accuracy suite.
#
# Prerequisites:
#   - Run from a Visual Studio Developer PowerShell (or cmd with VsDevCmd.bat called first)
#   - Qt6 + vcpkg installed (existing build/ directory already configured)
#
# Usage:
#   .\scripts\build_and_run_blargg.ps1
#   .\scripts\build_and_run_blargg.ps1 -SkipDownload   (skip ROM download)
#   .\scripts\build_and_run_blargg.ps1 -BuildOnly       (only build, don't run)

param(
    [switch]$SkipDownload,
    [switch]$BuildOnly
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path $PSScriptRoot -Parent
$BuildDir = Join-Path $RootDir "build"
$TestsDir = Join-Path $RootDir "tests"
$RustDir = Join-Path $RootDir "src/rust"

Write-Host "=== KagamiQA P2: Build & Run Blargg Suite ==="
Write-Host "Root:     $RootDir"
Write-Host "Build:    $BuildDir"
Write-Host ""

# ---------------------------------------------------------------------------
# Step 1: Download blargg ROMs
# ---------------------------------------------------------------------------
if (-not $SkipDownload) {
    Write-Host "[1/5] Downloading blargg test ROMs..."
    $downloadScript = Join-Path $RootDir "scripts/download_blargg_roms.ps1"
    & $downloadScript
    Write-Host ""
}

# ---------------------------------------------------------------------------
# Step 2: Build Rust kagami-qa runner
# ---------------------------------------------------------------------------
Write-Host "[2/5] Building Rust kagami-qa-runner..."
Push-Location $RustDir
try {
    cargo build --package kagami-qa --bin kagami-qa-runner --release
    if ($LASTEXITCODE -ne 0) { throw "Rust build failed" }
    Write-Host "  Rust build OK"
} finally {
    Pop-Location
}
Write-Host ""

# ---------------------------------------------------------------------------
# Step 3: CMake reconfigure (pick up new blargg_runner target)
# ---------------------------------------------------------------------------
Write-Host "[3/5] Reconfiguring CMake..."
Push-Location $BuildDir
try {
    # Remove cache to force clean reconfigure with current generator.
    Remove-Item CMakeCache.txt -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force CMakeFiles -ErrorAction SilentlyContinue

    cmake .. -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }
    Write-Host "  CMake configure OK"
} finally {
    Pop-Location
}
Write-Host ""

# ---------------------------------------------------------------------------
# Step 4: Build C++ blargg runner
# ---------------------------------------------------------------------------
Write-Host "[4/5] Building C++ fceux11_blargg_runner..."
Push-Location $BuildDir
try {
    cmake --build . --target fceux11_blargg_runner --config Release
    if ($LASTEXITCODE -ne 0) { throw "C++ build failed" }

    $RunnerPath = Join-Path $BuildDir "tests/Release/fceux11_blargg_runner.exe"
    if (-not (Test-Path $RunnerPath)) {
        # NMake puts binaries in tests/, MSBuild uses tests/Release/
        $RunnerPath = Join-Path $BuildDir "tests/fceux11_blargg_runner.exe"
    }
    if (Test-Path $RunnerPath) {
        Write-Host "  C++ build OK: $RunnerPath"
    } else {
        Write-Host "  WARNING: binary not found at expected paths. Check build output."
        Get-ChildItem -Recurse "$BuildDir/tests/fceux11_blargg_runner*" -ErrorAction SilentlyContinue
    }
} finally {
    Pop-Location
}
Write-Host ""

if ($BuildOnly) {
    Write-Host "Build complete (--BuildOnly). Skipping run."
    exit 0
}

# ---------------------------------------------------------------------------
# Step 5: Run full Oracle B suite
# ---------------------------------------------------------------------------
Write-Host "[5/5] Running Oracle B accuracy suite..."

# Find the blargg runner binary.
$RunnerBin = $null
$SearchPaths = @(
    (Join-Path $BuildDir "tests/Release/fceux11_blargg_runner.exe"),
    (Join-Path $BuildDir "tests/fceux11_blargg_runner.exe"),
    (Join-Path $BuildDir "tests/Debug/fceux11_blargg_runner.exe")
)
foreach ($p in $SearchPaths) {
    if (Test-Path $p) {
        $RunnerBin = $p
        break
    }
}

if (-not $RunnerBin) {
    Write-Host "ERROR: Cannot find fceux11_blargg_runner.exe after build."
    Write-Host "Searched: $SearchPaths"
    exit 1
}

# Quick smoke test with nestest.nes.
Write-Host ""
Write-Host "--- Smoke test (nestest.nes) ---"
Push-Location $TestsDir
try {
    & $RunnerBin --rom fixtures/nestest.nes --frames 60
    Write-Host "Smoke test exit code: $LASTEXITCODE"
} finally {
    Pop-Location
}

# Full batch run (all available ROMs in manifest).
$ManifestPath = Join-Path $TestsDir "fixtures/blargg_manifest.json"
if (Test-Path $ManifestPath) {
    Write-Host ""
    Write-Host "--- Full batch run ---"
    $startTime = Get-Date
    Push-Location $RootDir
    try {
        $BatchOutput = & $RunnerBin --manifest $ManifestPath 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    $elapsed = (Get-Date) - $startTime

    Write-Host ""
    Write-Host "=== Batch run complete ==="
    Write-Host "Duration: $($elapsed.TotalSeconds.ToString('F1'))s"
    Write-Host "Exit code: $exitCode"
    Write-Host ""

    # Save batch output.
    $BatchOutPath = Join-Path $RootDir "kagamiqa_blargg_batch_output.json"
    $BatchOutput | Out-File -FilePath $BatchOutPath -Encoding utf8
    Write-Host "Batch output saved to: $BatchOutPath"

    # Generate accuracy table using the Rust runner.
    Write-Host ""
    Write-Host "--- Generating accuracy table ---"
    $AccuracyTablePath = Join-Path $RootDir "docs/FCEUX11-1.16_KagamiQA-P2-accuracy-table.md"
    $RustRunnerPath = Join-Path $RustDir "target/release/kagami-qa-runner.exe"
    if (Test-Path $RustRunnerPath) {
        # Note: the --accuracy-table flag generates from parsed BLARGG_RESULT lines.
        # For now, we use the Python helper to parse batch JSON → markdown.
        Write-Host "See kagamiqa_blargg_batch_output.json for raw results."
        Write-Host "Run: kagami-qa-runner --manifest tests/tests.json --bin-dir $BuildDir/tests --accuracy-table $AccuracyTablePath"
    }
} else {
    Write-Host "NOTE: blargg_manifest.json not found. Run scripts/download_blargg_roms.ps1 first."
}

Write-Host ""
Write-Host "=== P2 build & run pipeline complete ==="
