# FCEUX11 --- A/B benchmark driver for the v2.0.4 V8 "A7 alias" experiment.
#
# Two binaries that differ by exactly one compile macro
# (FCEUX11_X6502_ALIAS_VARIANT):
#   base    : register macros route through g_cpu.native_layout()   (canonical)
#   variant : register macros route through the handler parameter xp (ops_table_a7.inc)
#
# Measurement discipline (AGENTS.md r55: bench readings are advisory):
#   * same machine, same scenario, alternating A/B to cancel clock drift;
#   * each round reports the bench's own median_total_ms;
#   * compare the MEDIAN of the rounds; only large sustained regressions matter
#     (historical true-regression magnitude is about +100%).
#
# Scenarios:
#   S1 = tests/fixtures/nestest.nes (the bench's default ROM)
#   S2 = <workdir>/fixtures/nestest.nes holding a copy of
#        tests/fixtures/instr_test_v5_all.nes (blargg all-instruction test,
#        ALU heavy -> more CPU-bound).  The bench hardcodes the relative ROM
#        path "fixtures/nestest.nes", so the swap is done by copying.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\bench\alias_ab_bench.ps1 -Rounds 7

param(
    [int]$Rounds = 7,
    [string]$BaseExe    = "",
    [string]$VariantExe = ""
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $BaseExe)    { $BaseExe    = Join-Path $repo "build\tests\fceux11_bench_x6502_exec.exe" }
if (-not $VariantExe) { $VariantExe = Join-Path $repo "build-a7\tests\fceux11_bench_x6502_exec.exe" }

$scenario1 = Join-Path $repo "tests"
$scenario2 = Join-Path $repo ".zcode\bench-s2"

# scenario 2: stage the ALU-heavy ROM under the bench's fixed relative path
New-Item -ItemType Directory -Force -Path (Join-Path $scenario2 "fixtures") | Out-Null
Copy-Item (Join-Path $repo "tests\fixtures\instr_test_v5_all.nes") `
          (Join-Path $scenario2 "fixtures\nestest.nes") -Force

$env:PATH = (Join-Path $repo "vcpkg_installed\x64-windows\bin") + ";$env:PATH"

# NOTE: do not name this function "Measure" -- that collides with the
# Measure-Object alias and PowerShell resolves the alias instead.
function Measure-Bench([string]$exe, [string]$cwd) {
    Push-Location $cwd
    try {
        $out = & $exe --json 2>&1 | Out-String
    } finally {
        Pop-Location
    }
    if ($out -notmatch '"median_total_ms":([0-9.]+)') { return $null }
    return [double]$Matches[1]
}

function Get-Median([double[]]$xs) {
    $s = $xs | Sort-Object
    $n = $s.Count
    if ($n % 2 -eq 1) { return $s[[int]($n / 2)] }
    return ($s[$n / 2 - 1] + $s[$n / 2]) / 2
}

foreach ($sc in @(@{ name = 'S1 nestest'; dir = $scenario1 }, @{ name = 'S2 instr_v5_all'; dir = $scenario2 })) {
    Write-Output ("=== {0} (cwd={1}) ===" -f $sc.name, $sc.dir)
    $b = @(); $v = @()
    for ($i = 1; $i -le $Rounds; $i++) {
        if ($i % 2 -eq 1) {
            $mb = Measure-Bench $BaseExe $sc.dir
            $mv = Measure-Bench $VariantExe $sc.dir
        } else {
            $mv = Measure-Bench $VariantExe $sc.dir
            $mb = Measure-Bench $BaseExe $sc.dir
        }
        if ($null -ne $mb) { $b += $mb }
        if ($null -ne $mv) { $v += $mv }
        $d = 0
        if ($null -ne $mb -and $null -ne $mv -and $mb -ne 0) { $d = (($mv / $mb - 1) * 100) }
        Write-Output ("  round {0}: base={1} ms  variant={2} ms  delta={3:N2}%" -f $i, $mb, $mv, $d)
    }
    if ($b.Count -gt 0 -and $v.Count -gt 0) {
        $mbm = Get-Median $b
        $mvm = Get-Median $v
        $dm = 0
        if ($mbm -ne 0) { $dm = (($mvm / $mbm - 1) * 100) }
        Write-Output ("  MEDIAN base={0:N3} ms  variant={1:N3} ms  delta={2:N2}%" -f $mbm, $mvm, $dm)
    }
    Write-Output ""
}
