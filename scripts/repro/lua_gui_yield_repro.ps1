# FCEUX11 --- automated reproduction harness for "plan-out B"
# (GUI: any yielding Lua script crashes the process).
#
# Mechanism: the Qt GUI reads the config option `SDL.LuaScript` at startup and,
# when it is non-empty, calls FCEU_LoadLuaCode() automatically
# (src/drivers/Qt/fceuWrapper.cpp:1091-1108).  That reproduces the thread
# topology of plan-out B without a human GUI session: the script is CREATED on
# the GUI thread and RESUMED every frame by the emulation thread.
#
# IMPORTANT: the GUI resolves the config file by a RELATIVE path
# ("fceux.cfg", src/drivers/common/configSys.cpp:13) => the working directory
# must be the exe's own directory, otherwise the option is never read.
#
# The Lua script itself writes a self-proof counter file (see
# scripts/repro/yield_only.lua) so that "survived" can be distinguished from
# "the script never ran".
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\repro\lua_gui_yield_repro.ps1 `
#              -Seconds 90 [-Rom <rom>] [-Exe <fceux11.exe>]
#
# Exit code: 0 = survived; 1 = crashed; 2 = usage/environment error;
#            3 = survived but the script never ran (harness inconclusive).

param(
    [int]$Seconds = 90,
    [string]$Rom = "",
    [string]$Exe = "",
    [string]$Counter = ""
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Exe)   { $Exe = Join-Path $repo "build\src\fceux11.exe" }
if (-not $Rom)   { $Rom = Join-Path $repo "tests\fixtures\nestest.nes" }
if (-not $Counter) { $Counter = Join-Path $repo ".zcode\repro\yield_counter.txt" }
$Lua = Join-Path $repo "scripts\repro\yield_only.lua"

if (-not (Test-Path $Exe)) { Write-Output "MISSING EXE: $Exe"; exit 2 }
if (-not (Test-Path $Rom)) { Write-Output "MISSING ROM: $Rom"; exit 2 }
if (-not (Test-Path $Lua)) { Write-Output "MISSING LUA: $Lua"; exit 2 }

$cfgDir  = Split-Path -Parent $Exe
$cfgFile = Join-Path $cfgDir "fceux.cfg"
$cfgBak  = "$cfgFile.repro.bak"

$hadCfg = Test-Path $cfgFile
if ($hadCfg) { Copy-Item -LiteralPath $cfgFile -Destination $cfgBak -Force }

try {
    # --- write SDL.LuaScript into the GUI config (no BOM) ---
    $text = ""
    if ($hadCfg) { $text = [System.IO.File]::ReadAllText($cfgFile) }
    $lines = $text -split "`r?`n" | Where-Object { $_ -notmatch '^SDL\.LuaScript\s*=' }
    $lines += "SDL.LuaScript = $Lua"
    [System.IO.File]::WriteAllText($cfgFile, ($lines -join "`r`n"))

    # --- stale counter -> fresh run ---
    if (Test-Path $Counter) { Remove-Item -LiteralPath $Counter -Force }

    $env:FCEUX11_YIELD_COUNTER = $Counter

    $start = Get-Date
    Write-Output ("LAUNCH: $Exe `"$Rom`"  (cwd=$cfgDir, watch $Seconds s)")
    $proc = Start-Process -FilePath $Exe -ArgumentList "`"$Rom`"" -WorkingDirectory $cfgDir -PassThru

    $deadline = $start.AddSeconds($Seconds)
    $crashed = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        if ($proc.HasExited) { $crashed = $true; break }
    }

    if (-not $crashed) {
        Write-Output ("SURVIVED {0}s (pid {1})" -f $Seconds, $proc.Id)
        try { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
        $code = 0
    } else {
        $code = $proc.ExitCode
        Write-Output ("CRASHED after {0:N1}s, exit code 0x{1:X8}" -f `
            ((Get-Date) - $start).TotalSeconds, $code)
    }

    # --- self-proof: did the script actually run? ---
    if (Test-Path $Counter) {
        Write-Output ("SCRIPT-RAN: " + ((Get-Content $Counter -Raw).Trim()))
    } else {
        Write-Output "SCRIPT-NEVER-RAN: no counter file => the harness did NOT exercise the path"
        if ($code -eq 0) { $code = 3 }
    }

    # --- Windows event log ---
    $since = $start.AddSeconds(-2)
    Write-Output "--- Application event log ---"
    try {
        Get-WinEvent -FilterHashtable @{ LogName='Application'; StartTime=$since } -ErrorAction SilentlyContinue |
            Where-Object { $_.Message -match 'fceux11' } |
            Select-Object -First 5 TimeCreated, Id |
            Format-Table -AutoSize | Out-String -Width 120 | Write-Output
    } catch {
        Write-Output "(event log unavailable)"
    }

    exit $code
}
finally {
    if ($hadCfg) {
        Copy-Item -LiteralPath $cfgBak -Destination $cfgFile -Force
        Remove-Item -LiteralPath $cfgBak -Force -ErrorAction SilentlyContinue
    } else {
        Remove-Item -LiteralPath $cfgFile -Force -ErrorAction SilentlyContinue
    }
    Write-Output "config restored"
}
