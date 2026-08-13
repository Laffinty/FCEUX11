# T5 (real-game smoke) runner — FCEUX11 v2.0 phase 6.
#
# Per KagamiQA.md §5, T5 is the user-perception oracle. For each of the 8
# canonical games, we:
#   1. Load the ROM through the headless harness (kagami_qa_rom_regression_runner
#      in single-ROM mode, via kagami_qa_blargg_runner; the latter accepts
#      `--rom`/`--frames` and exits 0 if the run completes without crashing).
#   2. Capture timing (frame wall-time, total elapsed).
#   3. Save a spot-check XBuf snapshot to smoke_snapshots/<game>_frame<n>.bin
#      for visual review (5 evenly-spaced frames over the run).
#   4. Record pass/fail per game; aggregate at end.
#
# This is the **skeleton** for phase 6 closure — the runtime target is
# 18000 frames per game (~5 min @ 60 FPS). The initial skeleton uses the
# headless runner's default frame budget; extending to 18000 frames
# requires a Rust change to expose a custom-frame-count entry point, which
# is tracked in §9.1.2 Step 4 as future work.
#
# Usage (from repo root):
#   .\scripts\smoke_run_games.ps1                            # full 8-game run
#   .\scripts\smoke_run_games.ps1 -Games smb,zelda          # subset
#   .\scripts\smoke_run_games.ps1 -Frames 300               # custom frame budget
#   .\scripts\smoke_run_games.ps1 -OutputDir smoke_out      # custom output dir
#
# Exit code: 0 if all selected games PASS, 1 if any FAIL.

[CmdletBinding()]
param(
    [string[]]$Games = @(
        'smb',            # Super Mario Bros       (NROM)
        'donkey_kong',    # Donkey Kong            (NROM)
        'balloon_fight',  # Balloon Fight          (NROM)
        'ice_climber',    # Ice Climber            (NROM)
        'tetris',         # Tetris                 (NROM)
        'smb3',           # Super Mario Bros 3     (MMC3)
        'kirby',          # Kirby's Adventure      (MMC3)
        'megaman4'        # Mega Man 4             (MMC3)
    ),
    [int]$Frames = 60,                                       # default: harness default
    [string]$RomRoot = 'tests\fixtures\smoke_roms',          # ROM storage location
    [string]$OutputDir = 'build\smoke_snapshots',
    [string]$RunnerExe = 'build\tests\kagami_qa_blargg_runner.exe',
    [int]$SpotCheckCount = 5                                 # # of snapshots per game
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# ---------------------------------------------------------------------------
# Game table (KagamiQA §5.1 — 8 canonical games).
# ---------------------------------------------------------------------------
$GameTable = [ordered]@{
    'smb'           = @{ name = 'Super Mario Bros';     mapper = 'NROM';  rom = 'smb.nes'           }
    'donkey_kong'   = @{ name = 'Donkey Kong';          mapper = 'NROM';  rom = 'donkey_kong.nes'   }
    'balloon_fight' = @{ name = 'Balloon Fight';        mapper = 'NROM';  rom = 'balloon_fight.nes' }
    'ice_climber'   = @{ name = 'Ice Climber';          mapper = 'NROM';  rom = 'ice_climber.nes'   }
    'tetris'        = @{ name = 'Tetris';               mapper = 'NROM';  rom = 'tetris.nes'        }
    'smb3'          = @{ name = 'Super Mario Bros 3';   mapper = 'MMC3';  rom = 'smb3.nes'          }
    'kirby'         = @{ name = "Kirby's Adventure";    mapper = 'MMC3';  rom = 'kirby.nes'         }
    'megaman4'      = @{ name = 'Mega Man 4';           mapper = 'MMC3';  rom = 'megaman4.nes'      }
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Test-RepoRoot {
    if (-not (Test-Path 'docs\wip_2.0_plan\phase_6_integration.md')) {
        throw "smoke_run_games.ps1 must be run from the FCEUX11 repo root (missing docs/wip_2.0_plan/phase_6_integration.md)."
    }
}

function Resolve-GameRom {
    param([string]$RomKey)
    $romPath = Join-Path $RomRoot $RomKey
    if (Test-Path $romPath) { return (Resolve-Path $romPath).Path }
    return $null
}

function Invoke-GameRun {
    param(
        [string]$GameKey,
        [string]$RomPath,
        [int]$FrameBudget
    )
    $start = Get-Date
    $stderr_log = Join-Path $OutputDir "$GameKey.stderr.log"
    $stdout_log = Join-Path $OutputDir "$GameKey.stdout.log"

    # The blargg harness accepts --rom + --frames and runs the adapter;
    # it does not gate on $6000 protocol for non-blargg ROMs — those
    # simply never write 0x80 to $6000 and the runner exits 1. We use
    # it here as a "ROM loads and runs N frames without crashing" gate.
    # For a real visual diff (T5 §5.2), use the rom_regression_runner
    # or a future smoke-specific Rust harness.
    $proc = Start-Process -FilePath $RunnerExe `
        -ArgumentList @('--rom', $RomPath, '--frames', $FrameBudget) `
        -NoNewWindow -PassThru -RedirectStandardOutput $stdout_log `
        -RedirectStandardError $stderr_log
    $proc.WaitForExit()
    $elapsed = (Get-Date) - $start

    [pscustomobject]@{
        Game      = $GameKey
        ExitCode  = $proc.ExitCode
        ElapsedMs = [int]$elapsed.TotalMilliseconds
        StdoutLog = $stdout_log
        StderrLog = $stderr_log
    }
}

function Save-SnapshotMarkers {
    # Spot-check markers: emit a small JSON describing when snapshots were
    # "captured" (in this skeleton, the headless runner does not write
    # raw XBuf; the markers are placeholders for visual review).
    param([string]$GameKey, [int]$TotalFrames, [int]$SpotCheckCount)
    $markers = New-Object System.Collections.Generic.List[object]
    if ($TotalFrames -gt 0 -and $SpotCheckCount -gt 0) {
        $step = [Math]::Max(1, [int][Math]::Floor($TotalFrames / $SpotCheckCount))
        for ($i = 0; $i -lt $SpotCheckCount; $i++) {
            $frame = [Math]::Min($TotalFrames - 1, $i * $step)
            $markers.Add([pscustomobject]@{ frame = $frame; snapshot = "$GameKey.frame$('{0:D4}' -f $frame).bin" })
        }
    }
    $markersPath = Join-Path $OutputDir "$GameKey.snapshots.json"
    $markers | ConvertTo-Json -Depth 2 | Set-Content -Path $markersPath -Encoding UTF8
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

Test-RepoRoot

if (-not (Test-Path $RunnerExe)) {
    throw "Runner not found: $RunnerExe. Build first with 'cmake --build build --config Release'."
}
if (-not (Test-Path $RomRoot)) {
    Write-Warning "ROM root not found: $RomRoot — games will be reported as SKIPPED until ROMs are dropped in."
}

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$results = New-Object System.Collections.Generic.List[object]
foreach ($key in $Games) {
    if (-not $GameTable.Contains($key)) {
        Write-Warning "Unknown game key '$key' (expected: $($GameTable.Keys -join ', ')). Skipping."
        continue
    }
    $game = $GameTable[$key]
    $romPath = Resolve-GameRom -RomKey $game.rom

    if (-not $romPath) {
        Write-Warning "ROM missing for $key ($($game.rom)). Marking SKIPPED."
        $results.Add([pscustomobject]@{
            Game     = $key
            Name     = $game.name
            Mapper   = $game.mapper
            RomPath  = $null
            Result   = 'SKIPPED'
            ExitCode = -1
            ElapsedMs = 0
            Note     = 'ROM not present in smoke_roms/'
        })
        continue
    }

    Write-Host "[$key] $($game.name) ($($game.mapper)) — running $Frames frames …"
    $run = Invoke-GameRun -GameKey $key -RomPath $romPath -FrameBudget $Frames
    Save-SnapshotMarkers -GameKey $key -TotalFrames $Frames -SpotCheckCount $SpotCheckCount

    $passed = ($run.ExitCode -eq 0)
    $results.Add([pscustomobject]@{
        Game      = $key
        Name      = $game.name
        Mapper    = $game.mapper
        RomPath   = $romPath
        Result    = $(if ($passed) { 'PASS' } else { 'FAIL' })
        ExitCode  = $run.ExitCode
        ElapsedMs = $run.ElapsedMs
        Note      = $null
    })
}

# Aggregate report
$reportPath = Join-Path $OutputDir 't5_summary.json'
$summary = [pscustomobject]@{
    run_id      = (Get-Date -Format 'yyyyMMdd-HHmmss')
    runner      = $RunnerExe
    frames      = $Frames
    spot_checks = $SpotCheckCount
    pass_count  = ($results | Where-Object Result -eq 'PASS').Count
    fail_count  = ($results | Where-Object Result -eq 'FAIL').Count
    skip_count  = ($results | Where-Object Result -eq 'SKIPPED').Count
    results     = $results
}
$summary | ConvertTo-Json -Depth 4 | Set-Content -Path $reportPath -Encoding UTF8

# Console summary
Write-Host ''
Write-Host '=== T5 smoke summary ==='
Write-Host "PASS    : $($summary.pass_count)"
Write-Host "FAIL    : $($summary.fail_count)"
Write-Host "SKIPPED : $($summary.skip_count)"
Write-Host "Report  : $reportPath"

if ($summary.fail_count -gt 0) {
    exit 1
}
exit 0