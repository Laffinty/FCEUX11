# KagamiQA P5 — Download complete blargg test ROM suite from GitHub mirror.
#
# Expanded from P2 (22 ROMs) to full coverage: CPU ~73, PPU ~45, APU ~52, Mapper ~20
# Target: >=140 ROMs covering all blargg sub-categories for authoritative QA defense line.
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
#
# Categories map repository directories to our fixture layout:
#   cpu/   — CPU instruction, timing, interrupt, reset tests
#   ppu/   — PPU VBL/NMI, sprite, open bus, OAM, scanline tests
#   apu/   — APU length counter, envelope, sweep, DMC, mixer tests
#   mmc3/  — MMC3 IRQ, scanline, and CHR banking tests
# ---------------------------------------------------------------------------

$Roms = @(

    # =========================================================================
    # CPU — Instruction tests (blargg_nes_cpu_test5)
    # =========================================================================
    @{Name="cpu_test5_cpu";       Src="blargg_nes_cpu_test5/cpu.nes";            Cat="cpu"; Frames=500},
    @{Name="cpu_test5_official";  Src="blargg_nes_cpu_test5/official.nes";       Cat="cpu"; Frames=500},

    # =========================================================================
    # CPU — Instruction tests v3 (instr_test-v3)
    # =========================================================================
    @{Name="instr_v3_all";        Src="instr_test-v3/all_instrs.nes";            Cat="cpu"; Frames=300},
    @{Name="instr_v3_official";   Src="instr_test-v3/official_only.nes";         Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Instruction tests v5 (instr_test-v5) — already have all_instrs/official_only
    # =========================================================================
    @{Name="instr_v5_all";        Src="instr_test-v5/all_instrs.nes";            Cat="cpu"; Frames=300},
    @{Name="instr_v5_official";   Src="instr_test-v5/official_only.nes";         Cat="cpu"; Frames=300},
    # v5 rom_singles — individual instruction group tests
    @{Name="instr_v5_01_basics";       Src="instr_test-v5/rom_singles/01-basics.nes";       Cat="cpu"; Frames=300},
    @{Name="instr_v5_02_implied";      Src="instr_test-v5/rom_singles/02-implied.nes";      Cat="cpu"; Frames=300},
    @{Name="instr_v5_03_immediate";    Src="instr_test-v5/rom_singles/03-immediate.nes";    Cat="cpu"; Frames=300},
    @{Name="instr_v5_04_zero_page";    Src="instr_test-v5/rom_singles/04-zero_page.nes";    Cat="cpu"; Frames=300},
    @{Name="instr_v5_05_zp_xy";        Src="instr_test-v5/rom_singles/05-zp_xy.nes";        Cat="cpu"; Frames=300},
    @{Name="instr_v5_06_absolute";     Src="instr_test-v5/rom_singles/06-absolute.nes";     Cat="cpu"; Frames=300},
    @{Name="instr_v5_07_abs_xy";       Src="instr_test-v5/rom_singles/07-abs_xy.nes";       Cat="cpu"; Frames=300},
    @{Name="instr_v5_08_ind_x";        Src="instr_test-v5/rom_singles/08-ind_x.nes";        Cat="cpu"; Frames=300},
    @{Name="instr_v5_09_ind_y";        Src="instr_test-v5/rom_singles/09-ind_y.nes";        Cat="cpu"; Frames=300},
    @{Name="instr_v5_10_branches";     Src="instr_test-v5/rom_singles/10-branches.nes";     Cat="cpu"; Frames=300},
    @{Name="instr_v5_11_stack";        Src="instr_test-v5/rom_singles/11-stack.nes";        Cat="cpu"; Frames=300},
    @{Name="instr_v5_12_jmp_jsr";      Src="instr_test-v5/rom_singles/12-jmp_jsr.nes";      Cat="cpu"; Frames=300},
    @{Name="instr_v5_13_rts";          Src="instr_test-v5/rom_singles/13-rts.nes";          Cat="cpu"; Frames=300},
    @{Name="instr_v5_14_rti";          Src="instr_test-v5/rom_singles/14-rti.nes";          Cat="cpu"; Frames=300},
    @{Name="instr_v5_15_brk";          Src="instr_test-v5/rom_singles/15-brk.nes";          Cat="cpu"; Frames=300},
    @{Name="instr_v5_16_special";      Src="instr_test-v5/rom_singles/16-special.nes";      Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Instruction timing (instr_timing)
    # =========================================================================
    @{Name="instr_timing";         Src="instr_timing/instr_timing.nes";               Cat="cpu"; Frames=500},
    @{Name="instr_timing_v2_1";    Src="instr_timing/rom_singles/1-instr_timing.nes"; Cat="cpu"; Frames=500},
    @{Name="instr_timing_v2_2";    Src="instr_timing/rom_singles/2-branch_timing.nes";Cat="cpu"; Frames=500},

    # =========================================================================
    # CPU — Branch timing
    # =========================================================================
    @{Name="branch_basics";        Src="branch_timing_tests/1.Branch_Basics.nes";    Cat="cpu"; Frames=300},
    @{Name="branch_backward";      Src="branch_timing_tests/2.Backward_Branch.nes";  Cat="cpu"; Frames=300},
    @{Name="branch_forward";       Src="branch_timing_tests/3.Forward_Branch.nes";   Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Dummy reads / writes
    # =========================================================================
    @{Name="cpu_dummy_reads";      Src="cpu_dummy_reads/cpu_dummy_reads.nes";           Cat="cpu"; Frames=300},
    @{Name="cpu_dummy_writes_oam"; Src="cpu_dummy_writes/cpu_dummy_writes_oam.nes";     Cat="cpu"; Frames=300},
    @{Name="cpu_dummy_writes_ppu"; Src="cpu_dummy_writes/cpu_dummy_writes_ppumem.nes";  Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Exec space
    # =========================================================================
    @{Name="cpu_exec_space_apu";   Src="cpu_exec_space/test_cpu_exec_space_apu.nes";   Cat="cpu"; Frames=300},
    @{Name="cpu_exec_space_ppuio"; Src="cpu_exec_space/test_cpu_exec_space_ppuio.nes"; Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Interrupts v2 (already have cpu_interrupts.nes)
    # =========================================================================
    @{Name="cpu_interrupts";       Src="cpu_interrupts_v2/cpu_interrupts.nes";              Cat="cpu"; Frames=500},
    @{Name="cpu_int_1_cli_latency";   Src="cpu_interrupts_v2/rom_singles/1-cli_latency.nes";    Cat="cpu"; Frames=300},
    @{Name="cpu_int_2_nmi_brk";       Src="cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes";    Cat="cpu"; Frames=300},
    @{Name="cpu_int_3_nmi_irq";       Src="cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes";    Cat="cpu"; Frames=300},
    @{Name="cpu_int_4_irq_dma";       Src="cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes";    Cat="cpu"; Frames=300},
    @{Name="cpu_int_5_branch_irq";    Src="cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes"; Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Reset
    # =========================================================================
    @{Name="cpu_reset_ram";        Src="cpu_reset/ram_after_reset.nes";  Cat="cpu"; Frames=300},
    @{Name="cpu_reset_regs";       Src="cpu_reset/registers.nes";        Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — Timing test 6
    # =========================================================================
    @{Name="cpu_timing_test6";     Src="cpu_timing_test6/cpu_timing_test.nes";  Cat="cpu"; Frames=500},

    # =========================================================================
    # CPU — Instruction misc
    # =========================================================================
    @{Name="instr_misc";           Src="instr_misc/instr_misc.nes";              Cat="cpu"; Frames=300},
    @{Name="instr_misc_01_abs_x";     Src="instr_misc/rom_singles/01-abs_x_wrap.nes";       Cat="cpu"; Frames=300},
    @{Name="instr_misc_02_branch";     Src="instr_misc/rom_singles/02-branch_wrap.nes";     Cat="cpu"; Frames=300},
    @{Name="instr_misc_03_dummy";      Src="instr_misc/rom_singles/03-dummy_reads.nes";     Cat="cpu"; Frames=300},
    @{Name="instr_misc_04_dummy_apu";  Src="instr_misc/rom_singles/04-dummy_reads_apu.nes"; Cat="cpu"; Frames=300},

    # =========================================================================
    # CPU — nes_instr_test (11 rom_singles)
    # =========================================================================
    @{Name="nes_instr_01_implied";  Src="nes_instr_test/rom_singles/01-implied.nes";    Cat="cpu"; Frames=300},
    @{Name="nes_instr_02_imm";      Src="nes_instr_test/rom_singles/02-immediate.nes";  Cat="cpu"; Frames=300},
    @{Name="nes_instr_03_zp";       Src="nes_instr_test/rom_singles/03-zero_page.nes";  Cat="cpu"; Frames=300},
    @{Name="nes_instr_04_zp_xy";    Src="nes_instr_test/rom_singles/04-zp_xy.nes";      Cat="cpu"; Frames=300},
    @{Name="nes_instr_05_abs";      Src="nes_instr_test/rom_singles/05-absolute.nes";   Cat="cpu"; Frames=300},
    @{Name="nes_instr_06_abs_xy";   Src="nes_instr_test/rom_singles/06-abs_xy.nes";     Cat="cpu"; Frames=300},
    @{Name="nes_instr_07_ind_x";    Src="nes_instr_test/rom_singles/07-ind_x.nes";      Cat="cpu"; Frames=300},
    @{Name="nes_instr_08_ind_y";    Src="nes_instr_test/rom_singles/08-ind_y.nes";      Cat="cpu"; Frames=300},
    @{Name="nes_instr_09_branches"; Src="nes_instr_test/rom_singles/09-branches.nes";   Cat="cpu"; Frames=300},
    @{Name="nes_instr_10_stack";    Src="nes_instr_test/rom_singles/10-stack.nes";      Cat="cpu"; Frames=300},
    @{Name="nes_instr_11_special";  Src="nes_instr_test/rom_singles/11-special.nes";    Cat="cpu"; Frames=300},

    # =========================================================================
    # PPU — VBL/NMI (already have main ROM; add rom_singles)
    # =========================================================================
    @{Name="ppu_vbl_nmi";              Src="ppu_vbl_nmi/ppu_vbl_nmi.nes";                    Cat="ppu"; Frames=300},
    @{Name="vbl_01_basics";            Src="ppu_vbl_nmi/rom_singles/01-vbl_basics.nes";      Cat="ppu"; Frames=300},
    @{Name="vbl_02_set_time";          Src="ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes";    Cat="ppu"; Frames=300},
    @{Name="vbl_03_clear_time";        Src="ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes";  Cat="ppu"; Frames=300},
    @{Name="vbl_04_nmi_control";       Src="ppu_vbl_nmi/rom_singles/04-nmi_control.nes";     Cat="ppu"; Frames=300},
    @{Name="vbl_05_nmi_timing";        Src="ppu_vbl_nmi/rom_singles/05-nmi_timing.nes";      Cat="ppu"; Frames=300},
    @{Name="vbl_06_suppression";       Src="ppu_vbl_nmi/rom_singles/06-suppression.nes";     Cat="ppu"; Frames=300},
    @{Name="vbl_07_nmi_on_timing";     Src="ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes";   Cat="ppu"; Frames=300},
    @{Name="vbl_08_nmi_off_timing";    Src="ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes";  Cat="ppu"; Frames=300},
    @{Name="vbl_09_even_odd_frames";   Src="ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes"; Cat="ppu"; Frames=300},
    @{Name="vbl_10_even_odd_timing";   Src="ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes"; Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — VBL/NMI timing (blargg_ppu_tests_2005.09.15b)
    # =========================================================================
    @{Name="ppu_palette_ram";       Src="blargg_ppu_tests_2005.09.15b/palette_ram.nes";      Cat="ppu"; Frames=300},
    @{Name="ppu_power_up_palette";  Src="blargg_ppu_tests_2005.09.15b/power_up_palette.nes"; Cat="ppu"; Frames=300},
    @{Name="ppu_sprite_ram";        Src="blargg_ppu_tests_2005.09.15b/sprite_ram.nes";       Cat="ppu"; Frames=300},
    @{Name="ppu_vbl_clear_time";    Src="blargg_ppu_tests_2005.09.15b/vbl_clear_time.nes";   Cat="ppu"; Frames=300},
    @{Name="ppu_vram_access";       Src="blargg_ppu_tests_2005.09.15b/vram_access.nes";      Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — VBL/NMI timing (vbl_nmi_timing directory)
    # =========================================================================
    @{Name="vbl_timing_1_frame";        Src="vbl_nmi_timing/1.frame_basics.nes";        Cat="ppu"; Frames=300},
    @{Name="vbl_timing_2_vbl";          Src="vbl_nmi_timing/2.vbl_timing.nes";          Cat="ppu"; Frames=300},
    @{Name="vbl_timing_3_even_odd";     Src="vbl_nmi_timing/3.even_odd_frames.nes";     Cat="ppu"; Frames=300},
    @{Name="vbl_timing_4_clear_timing"; Src="vbl_nmi_timing/4.vbl_clear_timing.nes";    Cat="ppu"; Frames=300},
    @{Name="vbl_timing_5_suppression";  Src="vbl_nmi_timing/5.nmi_suppression.nes";     Cat="ppu"; Frames=300},
    @{Name="vbl_timing_6_disable";      Src="vbl_nmi_timing/6.nmi_disable.nes";         Cat="ppu"; Frames=300},
    @{Name="vbl_timing_7_nmi";          Src="vbl_nmi_timing/7.nmi_timing.nes";          Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — Sprite overflow (already have all 5)
    # =========================================================================
    @{Name="sprite_overflow_1_basics";   Src="sprite_overflow_tests/1.Basics.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_2_details";  Src="sprite_overflow_tests/2.Details.nes";   Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_3_timing";   Src="sprite_overflow_tests/3.Timing.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_4_obscure";  Src="sprite_overflow_tests/4.Obscure.nes";   Cat="ppu"; Frames=300},
    @{Name="sprite_overflow_5_emu";      Src="sprite_overflow_tests/5.Emulator.nes";  Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — Sprite 0 hit (already have all 11)
    # =========================================================================
    @{Name="sprite_hit_01_basics";       Src="sprite_hit_tests_2005.10.05/01.basics.nes";        Cat="ppu"; Frames=300},
    @{Name="sprite_hit_02_alignment";    Src="sprite_hit_tests_2005.10.05/02.alignment.nes";     Cat="ppu"; Frames=300},
    @{Name="sprite_hit_03_corners";      Src="sprite_hit_tests_2005.10.05/03.corners.nes";       Cat="ppu"; Frames=300},
    @{Name="sprite_hit_04_flip";         Src="sprite_hit_tests_2005.10.05/04.flip.nes";          Cat="ppu"; Frames=300},
    @{Name="sprite_hit_05_left_clip";    Src="sprite_hit_tests_2005.10.05/05.left_clip.nes";     Cat="ppu"; Frames=300},
    @{Name="sprite_hit_06_right_edge";   Src="sprite_hit_tests_2005.10.05/06.right_edge.nes";    Cat="ppu"; Frames=300},
    @{Name="sprite_hit_07_screen_bottom";Src="sprite_hit_tests_2005.10.05/07.screen_bottom.nes"; Cat="ppu"; Frames=300},
    @{Name="sprite_hit_08_double_height";Src="sprite_hit_tests_2005.10.05/08.double_height.nes"; Cat="ppu"; Frames=300},
    @{Name="sprite_hit_09_timing";       Src="sprite_hit_tests_2005.10.05/09.timing_basics.nes"; Cat="ppu"; Frames=300},
    @{Name="sprite_hit_10_timing_order"; Src="sprite_hit_tests_2005.10.05/10.timing_order.nes";  Cat="ppu"; Frames=300},
    @{Name="sprite_hit_11_edge_timing";  Src="sprite_hit_tests_2005.10.05/11.edge_timing.nes";   Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — Open bus, OAM, scanline
    # =========================================================================
    @{Name="ppu_open_bus";      Src="ppu_open_bus/ppu_open_bus.nes";            Cat="ppu"; Frames=300},
    @{Name="ppu_read_buffer";   Src="ppu_read_buffer/test_ppu_read_buffer.nes"; Cat="ppu"; Frames=300},
    @{Name="oam_read";          Src="oam_read/oam_read.nes";                    Cat="ppu"; Frames=300},
    @{Name="oam_stress";        Src="oam_stress/oam_stress.nes";                Cat="ppu"; Frames=300},
    @{Name="scanline";          Src="scanline/scanline.nes";                    Cat="ppu"; Frames=300},
    @{Name="scanline_a1";       Src="scanline-a1/scanline.nes";                 Cat="ppu"; Frames=300},

    # =========================================================================
    # PPU — Other visual/behavioral tests
    # =========================================================================
    @{Name="full_palette";         Src="full_palette/full_palette.nes";          Cat="ppu"; Frames=300},
    @{Name="flowing_palette";      Src="full_palette/flowing_palette.nes";       Cat="ppu"; Frames=300},
    @{Name="palette_smooth";       Src="full_palette/full_palette_smooth.nes";   Cat="ppu"; Frames=300},
    @{Name="scrolltest";           Src="scrolltest/scroll.nes";                  Cat="ppu"; Frames=300},

    # =========================================================================
    # APU — Length counter, envelope, sweep, DMC (blargg_apu_2005.07.30)
    # =========================================================================
    @{Name="apu_01_len_ctr";        Src="blargg_apu_2005.07.30/01.len_ctr.nes";           Cat="apu"; Frames=600},
    @{Name="apu_02_len_table";      Src="blargg_apu_2005.07.30/02.len_table.nes";         Cat="apu"; Frames=600},
    @{Name="apu_03_irq_flag";       Src="blargg_apu_2005.07.30/03.irq_flag.nes";          Cat="apu"; Frames=600},
    @{Name="apu_04_clock_jitter";   Src="blargg_apu_2005.07.30/04.clock_jitter.nes";      Cat="apu"; Frames=600},
    @{Name="apu_05_len_mode0";      Src="blargg_apu_2005.07.30/05.len_timing_mode0.nes";  Cat="apu"; Frames=600},
    @{Name="apu_06_len_mode1";      Src="blargg_apu_2005.07.30/06.len_timing_mode1.nes";  Cat="apu"; Frames=600},
    @{Name="apu_07_irq_flag_timing";Src="blargg_apu_2005.07.30/07.irq_flag_timing.nes";   Cat="apu"; Frames=600},
    @{Name="apu_08_irq_timing";     Src="blargg_apu_2005.07.30/08.irq_timing.nes";        Cat="apu"; Frames=600},
    @{Name="apu_09_reset_timing";   Src="blargg_apu_2005.07.30/09.reset_timing.nes";      Cat="apu"; Frames=600},
    @{Name="apu_10_len_halt";       Src="blargg_apu_2005.07.30/10.len_halt_timing.nes";   Cat="apu"; Frames=600},
    @{Name="apu_11_len_reload";     Src="blargg_apu_2005.07.30/11.len_reload_timing.nes"; Cat="apu"; Frames=600},

    # =========================================================================
    # APU — apu_test (already have main ROM; add rom_singles)
    # =========================================================================
    @{Name="apu_test";             Src="apu_test/apu_test.nes";                     Cat="apu"; Frames=600},
    @{Name="apu_single_1_len_ctr";      Src="apu_test/rom_singles/1-len_ctr.nes";         Cat="apu"; Frames=600},
    @{Name="apu_single_2_len_table";    Src="apu_test/rom_singles/2-len_table.nes";       Cat="apu"; Frames=600},
    @{Name="apu_single_3_irq_flag";    Src="apu_test/rom_singles/3-irq_flag.nes";        Cat="apu"; Frames=600},
    @{Name="apu_single_4_jitter";       Src="apu_test/rom_singles/4-jitter.nes";          Cat="apu"; Frames=600},
    @{Name="apu_single_5_len_timing";   Src="apu_test/rom_singles/5-len_timing.nes";      Cat="apu"; Frames=600},
    @{Name="apu_single_6_irq_timing";   Src="apu_test/rom_singles/6-irq_flag_timing.nes"; Cat="apu"; Frames=600},
    @{Name="apu_single_7_dmc_basics";   Src="apu_test/rom_singles/7-dmc_basics.nes";      Cat="apu"; Frames=600},
    @{Name="apu_single_8_dmc_rates";    Src="apu_test/rom_singles/8-dmc_rates.nes";       Cat="apu"; Frames=600},

    # =========================================================================
    # APU — Reset
    # =========================================================================
    @{Name="apu_reset_4015";         Src="apu_reset/4015_cleared.nes";        Cat="apu"; Frames=300},
    @{Name="apu_reset_4017_timing";  Src="apu_reset/4017_timing.nes";         Cat="apu"; Frames=300},
    @{Name="apu_reset_4017_written"; Src="apu_reset/4017_written.nes";        Cat="apu"; Frames=300},
    @{Name="apu_reset_irq_cleared";  Src="apu_reset/irq_flag_cleared.nes";    Cat="apu"; Frames=300},
    @{Name="apu_reset_len_ctrs";     Src="apu_reset/len_ctrs_enabled.nes";    Cat="apu"; Frames=300},
    @{Name="apu_reset_works_imm";    Src="apu_reset/works_immediately.nes";   Cat="apu"; Frames=300},

    # =========================================================================
    # APU — Mixer (square, triangle, noise, DMC)
    # =========================================================================
    @{Name="apu_mixer_square";    Src="apu_mixer/square.nes";     Cat="apu"; Frames=600},
    @{Name="apu_mixer_triangle";  Src="apu_mixer/triangle.nes";   Cat="apu"; Frames=600},
    @{Name="apu_mixer_noise";     Src="apu_mixer/noise.nes";      Cat="apu"; Frames=600},
    @{Name="apu_mixer_dmc";       Src="apu_mixer/dmc.nes";        Cat="apu"; Frames=600},

    # =========================================================================
    # APU — DMC tests
    # =========================================================================
    @{Name="dmc_buffer_retained";  Src="dmc_tests/buffer_retained.nes";  Cat="apu"; Frames=300},
    @{Name="dmc_latency";          Src="dmc_tests/latency.nes";          Cat="apu"; Frames=300},
    @{Name="dmc_status";           Src="dmc_tests/status.nes";           Cat="apu"; Frames=300},
    @{Name="dmc_status_irq";       Src="dmc_tests/status_irq.nes";       Cat="apu"; Frames=300},

    # =========================================================================
    # APU — DMC DMA during read
    # =========================================================================
    @{Name="dma_2007_read";        Src="dmc_dma_during_read4/dma_2007_read.nes";      Cat="apu"; Frames=300},
    @{Name="dma_2007_write";       Src="dmc_dma_during_read4/dma_2007_write.nes";     Cat="apu"; Frames=300},
    @{Name="dma_4016_read";        Src="dmc_dma_during_read4/dma_4016_read.nes";      Cat="apu"; Frames=300},
    @{Name="dma_double_2007_read"; Src="dmc_dma_during_read4/double_2007_read.nes";   Cat="apu"; Frames=300},
    @{Name="dma_read_write_2007";  Src="dmc_dma_during_read4/read_write_2007.nes";    Cat="apu"; Frames=300},

    # =========================================================================
    # APU — Sprite DMA + DMC DMA
    # =========================================================================
    @{Name="sprdma_dmc_dma";     Src="sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes";     Cat="apu"; Frames=300},
    @{Name="sprdma_dmc_dma_512"; Src="sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes"; Cat="apu"; Frames=300},

    # =========================================================================
    # APU — Volume tests
    # =========================================================================
    @{Name="volume_test"; Src="volume_tests/volumes.nes"; Cat="apu"; Frames=300},

    # =========================================================================
    # APU — PAL APU tests
    # =========================================================================
    @{Name="pal_apu_01_len_ctr";        Src="pal_apu_tests/01.len_ctr.nes";           Cat="apu"; Frames=600},
    @{Name="pal_apu_02_len_table";      Src="pal_apu_tests/02.len_table.nes";         Cat="apu"; Frames=600},
    @{Name="pal_apu_03_irq_flag";       Src="pal_apu_tests/03.irq_flag.nes";          Cat="apu"; Frames=600},
    @{Name="pal_apu_04_clock_jitter";   Src="pal_apu_tests/04.clock_jitter.nes";      Cat="apu"; Frames=600},
    @{Name="pal_apu_05_len_mode0";      Src="pal_apu_tests/05.len_timing_mode0.nes";  Cat="apu"; Frames=600},
    @{Name="pal_apu_06_len_mode1";      Src="pal_apu_tests/06.len_timing_mode1.nes";  Cat="apu"; Frames=600},
    @{Name="pal_apu_07_irq_flag_timing";Src="pal_apu_tests/07.irq_flag_timing.nes";   Cat="apu"; Frames=600},
    @{Name="pal_apu_08_irq_timing";     Src="pal_apu_tests/08.irq_timing.nes";        Cat="apu"; Frames=600},
    @{Name="pal_apu_10_len_halt";       Src="pal_apu_tests/10.len_halt_timing.nes";   Cat="apu"; Frames=600},
    @{Name="pal_apu_11_len_reload";     Src="pal_apu_tests/11.len_reload_timing.nes"; Cat="apu"; Frames=600},

    # =========================================================================
    # Mapper — MMC3 tests
    # =========================================================================
    @{Name="mmc3_1_clocking";        Src="mmc3_test/1-clocking.nes";           Cat="mmc3"; Frames=300},
    @{Name="mmc3_2_details";         Src="mmc3_test/2-details.nes";            Cat="mmc3"; Frames=300},
    @{Name="mmc3_3_A12_clocking";    Src="mmc3_test/3-A12_clocking.nes";      Cat="mmc3"; Frames=300},
    @{Name="mmc3_4_scanline_timing"; Src="mmc3_test/4-scanline_timing.nes";   Cat="mmc3"; Frames=300},
    @{Name="mmc3_5_MMC3";            Src="mmc3_test/5-MMC3.nes";              Cat="mmc3"; Frames=300},
    @{Name="mmc3_6_MMC6";            Src="mmc3_test/6-MMC6.nes";              Cat="mmc3"; Frames=300},

    # =========================================================================
    # Mapper — MMC3 IRQ tests
    # =========================================================================
    @{Name="mmc3_irq_1_Clocking";        Src="mmc3_irq_tests/1.Clocking.nes";        Cat="mmc3"; Frames=300},
    @{Name="mmc3_irq_2_Details";         Src="mmc3_irq_tests/2.Details.nes";         Cat="mmc3"; Frames=300},
    @{Name="mmc3_irq_3_A12_clocking";    Src="mmc3_irq_tests/3.A12_clocking.nes";    Cat="mmc3"; Frames=300},
    @{Name="mmc3_irq_4_Scanline_timing"; Src="mmc3_irq_tests/4.Scanline_timing.nes"; Cat="mmc3"; Frames=300},
    @{Name="mmc3_irq_5_MMC3_rev_A";      Src="mmc3_irq_tests/5.MMC3_rev_A.nes";      Cat="mmc3"; Frames=300},
    @{Name="mmc3_irq_6_MMC3_rev_B";      Src="mmc3_irq_tests/6.MMC3_rev_B.nes";      Cat="mmc3"; Frames=300},

    # =========================================================================
    # Mapper — MMC3 test 2 (alternate versions)
    # =========================================================================
    @{Name="mmc3_v2_1_clocking";         Src="mmc3_test_2/rom_singles/1-clocking.nes";         Cat="mmc3"; Frames=300},
    @{Name="mmc3_v2_2_details";          Src="mmc3_test_2/rom_singles/2-details.nes";          Cat="mmc3"; Frames=300},
    @{Name="mmc3_v2_3_A12_clocking";     Src="mmc3_test_2/rom_singles/3-A12_clocking.nes";     Cat="mmc3"; Frames=300},
    @{Name="mmc3_v2_4_scanline_timing";  Src="mmc3_test_2/rom_singles/4-scanline_timing.nes";  Cat="mmc3"; Frames=300},
    @{Name="mmc3_v2_5_MMC3";             Src="mmc3_test_2/rom_singles/5-MMC3.nes";             Cat="mmc3"; Frames=300},
    @{Name="mmc3_v2_6_MMC3_alt";         Src="mmc3_test_2/rom_singles/6-MMC3_alt.nes";         Cat="mmc3"; Frames=300}
)

# ---------------------------------------------------------------------------
# Create output directory
# ---------------------------------------------------------------------------
$null = New-Item -ItemType Directory -Force -Path $OutDir

# ---------------------------------------------------------------------------
# Download each ROM
# ---------------------------------------------------------------------------
Write-Host "=== KagamiQA P5: Downloading complete blargg test ROM suite ==="
Write-Host "Source: $BaseRaw"
Write-Host "Output: $(Resolve-Path $OutDir -ErrorAction SilentlyContinue)"
Write-Host "Total ROMs: $($Roms.Count)"
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
Write-Host "Skipped: $skip"
Write-Host "Failed: $fail"
Write-Host ""

if ($fail -gt 0) {
    Write-Host "Some ROMs could not be downloaded."
    Write-Host "Manual download: https://github.com/christopherpow/nes-test-roms"
}
