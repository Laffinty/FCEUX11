# KagamiQA P5 — Generate blargg_manifest.json from downloaded ROMs.
#
# Walks tests/fixtures/blargg/ and produces a manifest with all .nes files,
# categorized by parent directory (cpu/ppu/apu/mmc3).

param(
    [string]$RomDir = "tests\fixtures\blargg",
    [string]$OutFile = "tests\fixtures\blargg_manifest.json"
)

$ErrorActionPreference = "Stop"

$defaultFrames = @{
    cpu  = 300
    ppu  = 300
    apu  = 600
    mmc3 = 300
}

# Phase 3 Step 3.1: per-ROM reset_after override. -1 = disabled, >=0 =
# press RESET at frame N then continue. Required for ROMs that gate
# their actual test behind a manual soft-reset (apu_reset_*, cpu_reset_*).
# Without it the ROM displays "Press RESET" and writes $6000=0x81 forever.
$resetAfterRoms = @{
    'apu_reset_4015'         = 60
    'apu_reset_4017_timing'  = 60
    'apu_reset_4017_written' = 60
    'apu_reset_irq_cleared'  = 60
    'apu_reset_len_ctrs'     = 60
    'apu_reset_works_imm'    = 60
    'cpu_reset_ram'          = 60
    'cpu_reset_regs'         = 60
}

$roms = @()
Get-ChildItem -Path $RomDir -Recurse -Filter *.nes | Sort-Object FullName | ForEach-Object {
    $cat = $_.Directory.Name
    $name = $_.BaseName
    $relPath = "fixtures/blargg/$cat/$($_.Name)"
    $frames = if ($defaultFrames.ContainsKey($cat)) { $defaultFrames[$cat] } else { 300 }
    $resetAfter = if ($resetAfterRoms.ContainsKey($name)) { $resetAfterRoms[$name] } else { -1 }

    $roms += [PSCustomObject]@{
        name        = $name
        path        = $relPath
        category    = $cat
        frames      = $frames
        reset_after = $resetAfter
        probe_addr  = 24576
        description = "blargg test ROM: $name [$cat]"
    }
}

# Build JSON manually for clean formatting
$lines = @()
$lines += '{'
$lines += '  "_comment": "KagamiQA P5 — Full blargg test ROM catalog (expanded from P2 22 ROMs). Each ROM uses `$6000-`$6003 protocol.",'
$lines += '  "_source": "https://github.com/christopherpow/nes-test-roms",'
$lines += '  "_protocol": "`$6000 = 0x00 PASS, 0x01+ FAIL with diagnostic code",'
$lines += '  "_downloader": "scripts/download_blargg_roms.ps1",'
$lines += "  `"_total`": $($roms.Count),"
$lines += '  "roms": ['

for ($i = 0; $i -lt $roms.Count; $i++) {
    $r = $roms[$i]
    $comma = if ($i -lt $roms.Count - 1) { ',' } else { '' }
    $desc = "blargg test ROM: $($r.name) [$($r.category)]"
    $lines += '    {'
    $lines += "      `"name`": `"$($r.name)`","
    $lines += "      `"path`": `"$($r.path)`","
    $lines += "      `"category`": `"$($r.category)`","
    $lines += "      `"frames`": $($r.frames),"
    if ($r.reset_after -ge 0) {
        $lines += "      `"reset_after`": $($r.reset_after),"
    }
    $lines += "      `"probe_addr`": $($r.probe_addr),"
    $lines += "      `"description`": `"$desc`""
    $lines += "    }$comma"
}

$lines += '  ]'
$lines += '}'

$lines -join "`n" | Out-File -FilePath $OutFile -Encoding utf8

Write-Host "Generated manifest with $($roms.Count) ROM entries → $OutFile"
