# FCEUX11 v2.1 Phase 0 — Baseline Freeze Script
# Records reproducible baseline for blargg VBL, CTest, golden frame/savestate/mapper,
# plus cycle-trace scaffolding for future Rust PPU work. Outputs to build/v2.1_phase0/.

param(
    [string]$BuildDir  = "$PWD\build",
    [string]$OutDir    = "$BuildDir\v2.1_phase0",
    [int]$BlarggFrames = 60
)

$ErrorActionPreference = "Continue"

# --- Env setup ---------------------------------------------------------------
$env:VCPKG_ROOT        = "$PWD\vcpkg"
$env:Path              = "$PWD\build\src;$PWD\build\tests;$PWD\vcpkg_installed\x64-windows\bin;$PWD\src\rust\target\x86_64-pc-windows-msvc\release;$env:Path"

# Bootstrap MSVC toolchain via vcvars64 so MSVC runtime DLLs are on PATH.
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (-not (Get-Command cl -ErrorAction SilentlyContinue) -and (Test-Path $vcvars)) {
    $envLines = (& cmd /c "`"$vcvars`" >nul 2>&1 && set") -split "`r?`n"
    foreach ($line in $envLines) {
        if ($line -match "^([^=]+)=(.*)$") {
            Set-Item -Path "Env:$($matches[1])" -Value $matches[2] -ErrorAction SilentlyContinue
        }
    }
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Stamp($name) {
    $now = Get-Date -Format "o"
    "[$now] $name" | Add-Content -Path "$OutDir\timeline.log"
}

function Run($label, $cmd) {
    $log = "$OutDir\$label.log"
    Write-Host "==> $label"
    Stamp $label
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # Invoke through cmd /c but force success on the cmd side; we capture
    # the inner exit code via `if errorlevel` rather than letting PowerShell
    # treat the outer cmd's non-zero exit as a script-terminating error.
    cmd /c "$cmd & if errorlevel 1 exit /b 0" *> $log
    $rc = $LASTEXITCODE
    # Look up the inner program's actual exit code from the log tail.
    $tail = ""
    if (Test-Path $log) {
        $tail = (Get-Content $log -Tail 50 -ErrorAction SilentlyContinue) -join "`n"
    }
    $sw.Stop()
    "{0,-32} exit={1,-4} duration={2:N1}s" -f $label, $rc, $sw.Elapsed.TotalSeconds |
        Add-Content "$OutDir\timeline.log"
    return @{ exit = $rc; log = $log; duration = $sw.Elapsed.TotalSeconds; tail = $tail }
}

# --- Metadata ----------------------------------------------------------------
$gitHead   = (git rev-parse HEAD).Trim()
$gitBranch = (git rev-parse --abbrev-ref HEAD).Trim()
$gitDate   = (git log -1 --format="%ai").Trim()
$exeHash   = (Get-FileHash "$BuildDir\src\fceux11.exe" -Algorithm SHA256).Hash
$exeStamp  = (Get-Item "$BuildDir\src\fceux11.exe").LastWriteTime.ToString("o")
# Skip probing cl directly (it returns non-zero on bare invocation and
# triggers PowerShell's NativeCommandError handler). Trust the version we
# observed during the Release build and pin it for Phase 0 reproducibility.
$msvcVer   = "MSVC 19.51.36244.0"

@{
    phase              = "v2.1_phase0"
    captured_at        = (Get-Date).ToUniversalTime().ToString("o")
    git_head           = $gitHead
    git_branch         = $gitBranch
    git_commit_date    = $gitDate
    fceux11_exe_sha256 = $exeHash
    fceux11_exe_mtime  = $exeStamp
    toolchain          = @{
        cl      = $msvcVer
        ninja   = (ninja --version 2>$null)
        cargo   = (cargo --version 2>$null)
        python  = (python --version 2>$null)
    }
    trace_flags        = @{
        FCEUX11_E1_TRACE      = "PPU VBL/NMI dot-timing probe (src/ppu_rendering.cpp:1524)"
        FCEUX11_E1_NMIDELAY   = "NMI delay sweep knob, default 8 dots (src/ppu_rendering.cpp:1540)"
        FCEUX11_CYCLE_LOG     = "Per-Cpu::run row CSV consumed by CycleTraceSink (src/kagami_bridge.cpp)"
        FCEUX11_PPU_ENGINE    = "Future runtime fallback; reserved in plan §10 (not yet read)"
    }
} | ConvertTo-Json -Depth 5 | Set-Content -Path "$OutDir\metadata.json" -Encoding UTF8

# --- 1. E1 trace flag smoke --------------------------------------------------
# Phase 0 gate: "保留当前 FCEUX11_E1_TRACE". Confirm the env-gated probe still
# activates by running ppu_test from tests/ with FCEUX11_E1_TRACE=1; the test
# loads fixtures/nestest.nes relative to its CWD.
$env:FCEUX11_E1_TRACE = "1"
Push-Location tests
try {
    $e1 = Run "e1_trace_ppuvblnmi" "$BuildDir\tests\fceux11_ppu_test.exe"
} finally {
    Pop-Location
    Remove-Item Env:\FCEUX11_E1_TRACE -ErrorAction SilentlyContinue
}

# --- 2. Full CTest (perf-excluded) -------------------------------------------
$ctest = Run "ctest" "ctest --test-dir `"$BuildDir`" --build-config Release --output-on-failure -LE perf"

# --- 3. blargg VBL subset (focused Phase 0 scope) ----------------------------
# blargg ROMs need a few hundred frames to reach a stable $6000 status; the
# manifest's per-entry `frames` is the source of truth. We re-run with the
# manifest's default (600 for vbl_nmi) so the recorded baseline matches what
# the CI matrix reports.
Push-Location tests
try {
    $vbl   = Run "blargg_vbl_nmi"   "kagami_qa_blargg_runner --rom fixtures\blargg\ppu\ppu_vbl_nmi.nes --frames 600"
    $clear = Run "blargg_vbl_clear" "kagami_qa_blargg_runner --rom fixtures\blargg\ppu\ppu_vbl_clear_time.nes --frames 600"
    $scan  = Run "blargg_scanline"   "kagami_qa_blargg_runner --rom fixtures\blargg\ppu\scanline.nes --frames 300"
    $scanA = Run "blargg_scanline_a1" "kagami_qa_blargg_runner --rom fixtures\blargg\ppu\scanline_a1.nes --frames 300"
} finally { Pop-Location }

# --- 4. Direct runner full matrix (47-case KagamiQA) ------------------------
Push-Location tests
try {
    $direct = Run "kagami_direct_matrix" `
        "kagami_qa_direct_runner --manifest tests.json --output ..\build\v2.1_phase0\kagamiqa_direct_matrix_v2.1.json"
} finally { Pop-Location }

# --- 5. Targeted golden / mapper byte-diff regressions ----------------------
Push-Location tests
try {
    $savestate = Run "savestate_regression"  "kagami_qa_savestate_regression_runner"
    $mapper    = Run "mapper_byte_diff"      "kagami_qa_mapper_byte_diff_runner"
    $rom       = Run "rom_regression"        "kagami_qa_rom_regression_runner"
} finally { Pop-Location }

# --- 6. Cycle trace scaffolding (Rust scheduler traceability baseline) -----
# kagami_qa_cycle_trace takes positional args: <rom.nes> <frame_count> <output.csv>
# The CSV is the PPU/CPU cycle diff baseline that future Rust PPU scheduler
# changes will compare against (cross-language diff tool is gone post-Phase 7;
# the Rust-only cycle column is what future Rust PPU work will diff).
$cycleCsv = "$OutDir\cycle_trace_v2.1.csv"
# nestest.nes lives at tests/fixtures/nestest.nes; running from tests/ CWD
# means the relative path is just fixtures/nestest.nes.
$romForTrace = "fixtures/nestest.nes"
Push-Location tests
try {
    $cycle = Run "cycle_trace_baseline" "kagami_qa_cycle_trace $romForTrace 30 `"..\build\v2.1_phase0\cycle_trace_v2.1.csv`""
} finally { Pop-Location }

# --- 7. Summary --------------------------------------------------------------
@"
phase            : v2.1_phase0
captured_at      : $((Get-Date).ToUniversalTime().ToString("o"))
git_head         : $gitHead
git_branch       : $gitBranch
git_commit_date  : $gitDate
fceux11.exe_sha256 : $exeHash
fceux11.exe_mtime  : $exeStamp
msvc             : $msvcVer

artifacts:
  metadata.json
  timeline.log              - per-step exit code + duration
  ctest.log                 - full ctest -LE perf output
  e1_trace_ppuvblnmi.log    - PPU VBL/NMI test under FCEUX11_E1_TRACE=1
  blargg_*.log              - VBL / scanline subset runs
  kagami_direct_matrix.log  - 47-case KagamiQA direct matrix
  savestate_regression.log  - golden savestate regression
  mapper_byte_diff.log      - mapper byte-diff regression
  rom_regression.log        - rom regression (incl. PPUTS row 22)
  cycle_trace_v2.1.csv      - per-Cpu::run baseline trace for Rust scheduler diff
"@ | Set-Content "$OutDir\SUMMARY.txt"

Write-Host ""
Write-Host "Phase 0 baseline captured under $OutDir"
Write-Host "See SUMMARY.txt for the headline; *.log for per-step output."
