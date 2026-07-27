# KagamiQA P2 — Download blargg test ROM suite from GitHub mirror.
#
# Blargg's test ROMs are public-domain NES diagnostic ROMs that use the
# $6000 memory-mapped result protocol. This script fetches them from the
# christopherpow/nes-test-roms GitHub mirror.
#
# Usage:
#   .\scripts\download_blargg_roms.ps1
#   .\scripts\download_blargg_roms.ps1 -OutDir tests\fixtures\blargg

param(
    [string]$OutDir = "tests\fixtures\blargg"
)

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------------------
# GitHub raw base URL (christopherpow/nes-test-roms mirror)
# ---------------------------------------------------------------------------
$BaseRaw = "https://raw.githubusercontent.com/christopherpow/nes-test-roms/master"

# ---------------------------------------------------------------------------
# ROM inventory — name → [source_rel_path, dest_rel_path, frames, category]
# ---------------------------------------------------------------------------
$Roms = @(
    # === CPU instruction tests (instr_test-v5) ===
    @{Name="all_instrs";       Src="instr_test-v5/all_instrs.nes";         Cat="cpu"; Frames=300},
    @{Name="official_only";    Src="instr_test-v5/official_only.nes";      Cat="cpu"; Frames=300},

    # === CPU timing ===
    @{Name="cpu_timing_test";  Src="cpu_timing_test6/cpu_timing_test.nes";  Cat="cpu"; Frames=500},

    # === CPU interrupts ===
    @{Name="cpu_interrupts";   Src="cpu_interrupts_v2/cpu_interrupts.nes";  Cat="cpu"; Frames=500},

    # === PPU VBL NMI ===
    @{Name="ppu_vbl_nmi";      Src="ppu_vbl_nmi/ppu_vbl_nmi.nes";           Cat="ppu"; Frames=300},

    # === PPU sprite overflow (5 ROMs) ===
    @{Name="sprite_overflow_1_basics";   Src="sprite_overflow_tests/1.Basics.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_2_details";  Src="sprite_overflow_tests/2.Details.nes";   Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_3_timing";   Src="sprite_overflow_tests/3.Timing.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_4_obscure";  Src="sprite_overflow_tests/4.Obscure.nes";   Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_5_emu";      Src="sprite_overflow_tests/5.Emulator.nes";  Cat="ppu"; Frames=300},

    # === PPU sprite 0 hit (11 ROMs) ===
    @{Name="sprite_hit_01_basics";       Src="sprite_hit_tests_2005.10.05/01.basics.nes";       Cat="ppu"; Frames=300},
    @{Name="sprite_hit_02_alignment";    Src="sprite_hit_tests_2005.10.05/02.alignment.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_hit_03_corners";      Src="sprite_hit_tests_2005.10.05/03.corners.nes";      Cat="ppu"; Frames=300},
    @{Name="sprite_hit_04_flip";         Src="sprite_hit_tests_2005.10.05/04.flip.nes";         Cat="ppu"; Frames=300},
    @{Name="sprite_hit_05_left_clip";    Src="sprite_hit_tests_2005.10.05/05.left_clip.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_hit_06_right_edge";   Src="sprite_hit_tests_2005.10.05/06.right_edge.nes";   Cat="ppu"; Frames=300},
    @{Name="sprite_hit_07_screen_bottom";Src="sprite_hit_tests_2005.10.05/07.screen_bottom.nes";Cat="ppu"; Frames=300},
    @{Name="sprite_hit_08_double_height";Src="sprite_hit_tests_2005.10.05/08.double_height.nes";Cat="ppu"; Frames=300},
    @{Name="sprite_hit_09_timing";       Src="sprite_hit_tests_2005.10.05/09.timing_basics.nes";Cat="ppu"; Frames=300},
    @{Name="sprite_hit_10_timing_order"; Src="sprite_hit_tests_2005.10.05/10.timing_order.nes"; Cat="ppu"; Frames=300},
    @{Name="sprite_hit_11_edge_timing";  Src="sprite_hit_tests_2005.10.05/11.edge_timing.nes";  Cat="ppu"; Frames=300},

    # === APU ===
    @{Name="apu_test";           Src="apu_test/apu_test.nes";                   Cat="apu"; Frames=600}

    # === MMC3 ===
    # mmc3_test_2 ROMs are in rom_singles/ subdirectory; need to check exact filenames
)

# ---------------------------------------------------------------------------
# Create output directory
# ---------------------------------------------------------------------------
$null = New-Item -ItemType Directory -Force -Path $OutDir

# ---------------------------------------------------------------------------
# Download each ROM
# ---------------------------------------------------------------------------
Write-Host "=== KagamiQA P2: Downloading blargg test ROM suite ==="
Write-Host "Source: $BaseRaw"
Write-Host "Output: $(Resolve-Path $OutDir -ErrorAction SilentlyContinue)"
Write-Host ""

$total = $Roms.Count
$ok    = 0
$skip  = 0
$fail  = 0

foreach ($rom in $Roms) {
    $destDir  = Join-Path $OutDir $rom.Cat
    $destPath = Join-Path $destDir "$($rom.Name).nes"
    $srcUrl   = "$BaseRaw/$($rom.Src)"

    $null = New-Item -ItemType Directory -Force -Path $destDir

    if (Test-Path $destPath) {
        $size = (Get-Item $destPath).Length
        if ($size -gt 256) {
            Write-Host "  [$($rom.Cat)] $($rom.Name).nes — cached ($size bytes)"
            $ok++
            continue
        } else {
            Write-Host "  [$($rom.Cat)] $($rom.Name).nes — cached file too small ($size bytes), re-fetching"
            Remove-Item $destPath -Force
        }
    }

    try {
        Write-Host "  [$($rom.Cat)] $($rom.Name).nes — downloading..."
        Invoke-WebRequest -Uri $srcUrl -OutFile $destPath -ErrorAction Stop
        $size = (Get-Item $destPath).Length
        if ($size -gt 256) {
            Write-Host "    OK ($size bytes)"
            $ok++
        } else {
            Write-Host "    WARN: file too small ($size bytes), may be an HTML redirect"
            Remove-Item $destPath -Force
            $fail++
        }
    } catch {
        Write-Host "    FAIL: $_"
        $fail++
    }
}

Write-Host ""
Write-Host "=== Summary ==="
Write-Host "Total:  $total"
Write-Host "OK:     $ok"
Write-Host "Failed: $fail"
Write-Host ""

if ($fail -gt 0) {
    Write-Host "Some ROMs could not be downloaded. These may be in rom_singles/ subdirectories"
    Write-Host "or have different filenames. Check: $BaseRaw"
    Write-Host "Manual download: https://github.com/christopherpow/nes-test-roms"
}
