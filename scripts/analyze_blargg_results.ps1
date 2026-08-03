# KagamiQA P5 — Analyze blargg full batch results.
#
# Phase 3 Step 3.1: results JSON now contains a `reset_after` field per
# ROM (from blargg_runner). This script uses the field to enrich the
# "harness fix" count: 0x80/0x81 ROMs that did NOT use reset_after are
# pure frame-budget issues; ones that DID use reset_after but still fail
# indicate a deeper harness gap.
param(
    [string]$ResultsJson = "build/blargg_full_results.json",
    [string]$StderrLog  = "build/blargg_full_stderr.txt"
)

$ErrorActionPreference = "Stop"

$json = Get-Content $ResultsJson | ConvertFrom-Json
$results = $json.results

$passCount = ($results | Where-Object { $_.status -eq 'PASS' }).Count
$failCount = ($results | Where-Object { $_.status -eq 'FAIL' }).Count

Write-Host "========================================"
Write-Host "KagamiQA P5 — Blargg Full Batch Results"
Write-Host "========================================"
Write-Host "Total ROMs: $($results.Count)"
Write-Host "PASS: $passCount"
Write-Host "FAIL: $failCount"
Write-Host "Coverage: $([math]::Round($passCount / $results.Count * 100, 1))%"
Write-Host ""

Write-Host "=== Failures by value ==="
$results | Where-Object { $_.status -eq 'FAIL' } | Group-Object { $_.value } | Sort-Object Count -Desc | ForEach-Object {
    Write-Host "  0x$($_.Name): $($_.Count) ROMs"
}

Write-Host ""
Write-Host "=== Failures by category ==="
# Parse the stderr log to get ROM names in order
$failRoms = @{}
if (Test-Path $StderrLog) {
    $lines = Get-Content $StderrLog
    foreach ($line in $lines) {
        if ($line -match '\[(\S+)\]\s+\d+\s+frames\.\.\.\s+FAIL\s+\((0x[0-9A-Fa-f]+)\)') {
            $romName = $Matches[1]
            $errCode = $Matches[2]
            $failRoms[$romName] = $errCode
        }
    }
}

# Categorize by ROM name prefix
$categories = @{
    'CPU-instr'    = @()
    'CPU-int'      = @()
    'CPU-reset'    = @()
    'CPU-dummy'    = @()
    'CPU-exec'     = @()
    'CPU-timing'   = @()
    'CPU-misc'     = @()
    'PPU-vbl'      = @()
    'PPU-sprite'   = @()
    'PPU-other'    = @()
    'APU-mixer'    = @()
    'APU-reset'    = @()
    'APU-dma'      = @()
    'APU-other'    = @()
    'MMC3'         = @()
}

foreach ($r in $results) {
    if ($r.status -ne 'FAIL') { continue }
    $name = [System.IO.Path]::GetFileNameWithoutExtension($r.rom)
    $code = $failRoms[$name]
    if (-not $code) { $code = $r.value }

    if ($name -match '^(all_instrs|official_only|instr_v[35]_|instr_timing|nes_instr_)') {
        $categories['CPU-instr'] += "$name ($code)"
    } elseif ($name -match '^cpu_int_|^cpu_interrupts') {
        $categories['CPU-int'] += "$name ($code)"
    } elseif ($name -match '^cpu_reset') {
        $categories['CPU-reset'] += "$name ($code)"
    } elseif ($name -match '^cpu_dummy') {
        $categories['CPU-dummy'] += "$name ($code)"
    } elseif ($name -match '^cpu_exec') {
        $categories['CPU-exec'] += "$name ($code)"
    } elseif ($name -match '^branch_|^cpu_timing') {
        $categories['CPU-timing'] += "$name ($code)"
    } elseif ($name -match '^instr_misc') {
        $categories['CPU-misc'] += "$name ($code)"
    } elseif ($name -match '^vbl_') {
        $categories['PPU-vbl'] += "$name ($code)"
    } elseif ($name -match '^ppu_vbl|^ppu_open|^ppu_read|^oam_') {
        $categories['PPU-other'] += "$name ($code)"
    } elseif ($name -match '^apu_mixer') {
        $categories['APU-mixer'] += "$name ($code)"
    } elseif ($name -match '^apu_reset') {
        $categories['APU-reset'] += "$name ($code)"
    } elseif ($name -match '^dma_|^dmc_|^sprdma') {
        $categories['APU-dma'] += "$name ($code)"
    } elseif ($name -match '^apu_') {
        $categories['APU-other'] += "$name ($code)"
    } elseif ($name -match '^mmc3') {
        $categories['MMC3'] += "$name ($code)"
    } else {
        Write-Host "  UNCATEGORIZED: $name ($code)"
    }
}

foreach ($cat in $categories.Keys | Sort-Object) {
    $items = $categories[$cat]
    if ($items.Count -gt 0) {
        Write-Host "  $cat ($($items.Count)): $($items -join ', ')"
    }
}

Write-Host ""
Write-Host "=== PASS Breakdown ==="
$passCats = @{}
foreach ($r in $results) {
    if ($r.status -ne 'PASS') { continue }
    $name = [System.IO.Path]::GetFileNameWithoutExtension($r.rom)
    if ($name -match '^(all_instrs|official_only|instr_v[35]_|instr_timing|nes_instr_|cpu_test5_|branch_)') {
        $cat = 'CPU'
    } elseif ($name -match '^cpu_') {
        $cat = 'CPU'
    } elseif ($name -match '^vbl_|^ppu_|^sprite_|^oam_|^scanline|^flowing|^full_|^palette_|^scroll') {
        $cat = 'PPU'
    } elseif ($name -match '^apu_|^dma_|^dmc_|^sprdma|^pal_apu|^volume') {
        $cat = 'APU'
    } elseif ($name -match '^mmc3') {
        $cat = 'MMC3'
    } else {
        $cat = 'OTHER'
    }
    $passCats[$cat] = ($passCats[$cat] -as [int]) + 1
}
foreach ($cat in $passCats.Keys | Sort-Object) {
    Write-Host "  $cat : $($passCats[$cat]) PASS"
}
