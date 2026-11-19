# FCEUX11 -- PGO training-run script (batch 3 deliverable, 2026-11-19).
#
# Companion to the PGO release-policy proposal in
# docs/history/plans/FCEUX11-PGO发布策略立项材料-2026-11-19.md (decision item D2 /
# C-2).  The policy decision is: PGO stays OFF by default; this script makes
# the training leg reproducible so the proposal can be evaluated without
# re-deriving the procedure.
#
# Prerequisites (see CMakeLists.txt FCEUX11_PGO / FCEUX11_PGO_USE, mutually
# exclusive, MSVC-only):
#   1. Configure a Phase-1 tree:  cmake -B build-pgo -DFCEUX11_PGO=ON ...
#   2. Build it serially (AGENTS.md pitfall 12: -j 1).
#   3. Run THIS script against that tree's exe.
#   4. Configure/build the Phase-2 tree with -DFCEUX11_PGO_USE=ON; the
#      .pgd written by step 3 must sit next to the exe.
#
# The three training scenarios and the 12,000-frame budget come from the
# v2.0.5 measurement (CHANGELOG [2.0.5], PGO entry): throughput +31.7%
# (Makaimura) / +36.7% (SMB3), 7-round A/B, same /GL + /LTCG baseline.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\train_pgo.ps1 `
#       -Exe <path-to-instrumented-fceux11.exe> `
#       -Rom1 <Makaimura.nes> -Rom2 <SMB3.nes> -Rom3 <Zelda.nes>
#
# The GUI needs no interaction for training: it loads the ROM from the
# command line (v2.0.5 C2 fixed UTF-8 argv reconstruction in
# src/drivers/Qt/main.cpp) and runs at unlimited speed without audio.
# Each leg is killed after the frame budget; the .pgc files land next to
# the exe and are consumed by the Phase-2 link.

param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Rom1,
    [Parameter(Mandatory = $true)][string]$Rom2,
    [Parameter(Mandatory = $true)][string]$Rom3,
    [int]$Frames = 12000
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe))  { Write-Output "MISSING EXE: $Exe";  exit 2 }

# Frame-kill is how the instrumented run ends deterministically:
# SDL.KillFCEUXonFrame is honored by the driver loop (fceuWrapper.cpp).
$cfgDir  = Split-Path -Parent $Exe
$cfgFile = Join-Path $cfgDir "fceux.cfg"
$cfgBak  = "$cfgFile.train.bak"
$hadCfg  = Test-Path $cfgFile
if ($hadCfg) { Copy-Item -LiteralPath $cfgFile -Destination $cfgBak -Force }

function Train-OneLeg {
    param([string]$Rom, [string]$Name)
    if (-not (Test-Path $Rom)) { Write-Output "MISSING ROM: $Rom"; return $false }
    $text = if ($hadCfg) { [IO.File]::ReadAllText($cfgFile) } else { '' }
    $lines = $text -split "`r?`n" | Where-Object { $_ -notmatch '^SDL\.KillFCEUXonFrame\s*=' }
    $lines += "SDL.KillFCEUXonFrame = $Frames"
    [IO.File]::WriteAllText($cfgFile, ($lines -join "`r`n"))

    $start = Get-Date
    Write-Output ("TRAIN [{0}]: {1} for {2} frames" -f $Name, $Rom, $Frames)
    $p = Start-Process -FilePath $Exe -ArgumentList "`"$Rom`"" `
            -WorkingDirectory $cfgDir -PassThru
    $p.WaitForExit()
    $sec = ((Get-Date) - $start).TotalSeconds
    Write-Output ("TRAIN [{0}]: exited after {1:N0} s (frame kill)" -f $Name, $sec)
    return $true
}

$ok = $true
$ok = (Train-OneLeg -Rom $Rom1 -Name 'Makaimura') -and $ok
$ok = (Train-OneLeg -Rom $Rom2 -Name 'SMB3')       -and $ok
$ok = (Train-OneLeg -Rom $Rom3 -Name 'Zelda')      -and $ok

if (Test-Path "$cfgDir\*.pgc") {
    Write-Output "--- .pgc files produced ---"
    Get-ChildItem "$cfgDir\*.pgc" | ForEach-Object {
        Write-Output ("  {0}  ({1:N0} KB)" -f $_.Name, ($_.Length / 1KB))
    }
} else {
    Write-Output "WARNING: no .pgc files found next to the exe -- check that this is the Phase-1 (instrumented) build."
}

if ($hadCfg) {
    Copy-Item -LiteralPath $cfgBak -Destination $cfgFile -Force
    Remove-Item -LiteralPath $cfgBak -Force
} else {
    Remove-Item -LiteralPath $cfgFile -Force
}
Write-Output "config restored"
if ($ok) { exit 0 } else { exit 1 }
