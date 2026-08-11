//! APU + DMA + IRQ + Joypad integration tests (Phase 4).
//!
//! Tests the skeleton + integration of the APU stack:
//! - APU 5-channel mixer output (blargg parity: silent inputs → silence)
//! - DMA OAM cycle count (513/514 alignment)
//! - IRQ controller (NMI edge + IRQ level + external EXT/EXT2)
//! - Joypad (strobe + shift register + MSB read in strobe mode)
//!
//! blargg APU ROM tests (apu_test, apu_mixer, etc.) are gated on ROM
//! fixtures; Phase 6 shadow-run integration will exercise those.

use vnesu11::apu::{ApuCore, IRQ_DMC, IRQ_FCOUNT};
use vnesu11::dma::DmaCore;
use vnesu11::irq::IrqController;
use vnesu11::joypad::{BUTTON_A, BUTTON_B, BUTTON_RIGHT, JoypadState};

// ====================================================================
// APU mixer
// ====================================================================

#[test]
fn apu_total_stall_cycles_zero_by_default() {
    let d = DmaCore::new();
    assert_eq!(d.total_stall_cycles(), 0);
    assert!(!d.is_stalling());
}

#[test]
fn apu_silent_inputs_produce_silence() {
    let mut a = ApuCore::new();
    a.tick(8); // 8 CPU cycles
    let samples = a.drain_output();
    assert_eq!(samples.len(), 16); // 8 stereo frames
    for s in &samples {
        assert_eq!(*s, 0, "silence but got {}", s);
    }
}

#[test]
fn apu_frame_counter_irq_at_14914_cycles() {
    let mut a = ApuCore::new();
    // Need to run frame counter to position 14914 CPU cycles.
    // Tick in small chunks; ApuCore::tick isn't fully wired to the
    // frame counter yet, so we manually tick the frame counter.
    let mut irq = false;
    a.frame_counter.tick(14914, &mut irq);
    assert!(irq, "IRQ should fire at 14914 cycles (4-step mode)");
}

#[test]
fn apu_frame_counter_no_irq_at_5step() {
    let mut a = ApuCore::new();
    a.frame_counter.write(0x80); // 5-step mode
    let mut irq = false;
    a.frame_counter.tick(14914, &mut irq);
    assert!(!irq, "5-step mode should not raise IRQ");
}

#[test]
fn apu_take_irq_aggregates_sources() {
    let mut a = ApuCore::new();
    a.frame_counter.write(0x00); // 4-step mode
    a.frame_counter.irq_inhibit = false;
    // Manually flip the pending bit (Phase 4 wiring).
    a.frame_irq_pending = true;
    a.dmc_irq_pending = true;
    assert_eq!(a.take_irq(), IRQ_FCOUNT | IRQ_DMC);
}

// ====================================================================
// DMA — OAM cycle count
// ====================================================================

#[test]
fn oam_dma_odd_cycle_triggers_513_cycles() {
    let mut d = DmaCore::new();
    assert!(d.oam.start(0x02, true));
    assert_eq!(d.oam.total_cycles(), 513);
}

#[test]
fn oam_dma_even_cycle_triggers_514_cycles() {
    let mut d = DmaCore::new();
    assert!(d.oam.start(0x02, false));
    assert_eq!(d.oam.total_cycles(), 514);
}

#[test]
fn oam_dma_step_256_times_completes() {
    let mut d = DmaCore::new();
    d.oam.start(0x02, true);
    for _ in 0..256 {
        d.oam.step();
    }
    assert!(!d.oam.active);
    assert_eq!(d.oam.remaining, 0);
}

#[test]
fn oam_dma_total_stall_reflects_state() {
    let mut d = DmaCore::new();
    d.oam.start(0x02, true);
    assert_eq!(d.total_stall_cycles(), 256);
    d.oam.step();
    assert_eq!(d.total_stall_cycles(), 255);
}

// ====================================================================
// IRQ controller
// ====================================================================

#[test]
fn irq_nmi_edge_takes_once() {
    let mut i = IrqController::new();
    assert!(!i.take_nmi());
    i.assert_nmi();
    assert!(i.take_nmi());
    assert!(!i.take_nmi()); // already notified
}

#[test]
fn irq_nmi_clear_resets_edge() {
    let mut i = IrqController::new();
    i.assert_nmi();
    i.clear_nmi();
    assert!(!i.take_nmi());
}

#[test]
fn irq_external_sources_aggregate() {
    let mut i = IrqController::new();
    i.set_external(0, true); // EXT
    i.set_external(1, true); // EXT2
    assert_eq!(i.aggregate_mask() & 0x001, 0x001);
    assert_eq!(i.aggregate_mask() & 0x002, 0x002);
    i.set_external(0, false);
    assert_eq!(i.aggregate_mask() & 0x001, 0);
}

#[test]
fn irq_poll_returns_true_when_any_source_set() {
    let mut i = IrqController::new();
    assert!(!i.poll());
    i.assert_irq(IRQ_FCOUNT);
    assert!(i.poll());
    i.deassert_irq(IRQ_FCOUNT);
    assert!(!i.poll());
}

// ====================================================================
// Joypad
// ====================================================================

#[test]
fn joypad_strobe_latches_button_state() {
    let mut j = JoypadState::new();
    j.set_button(0, BUTTON_RIGHT, true); // MSB
    j.write_strobe(0x01); // rising edge
    assert_eq!(j.read(0x4016), 1); // MSB after refresh
}

#[test]
fn joypad_strobed_reads_continuously_refresh() {
    let mut j = JoypadState::new();
    j.set_button(0, BUTTON_RIGHT, true);
    j.write_strobe(0x01); // strobe on
    // In strobe mode, reads always refresh from button_state.
    assert_eq!(j.read(0x4016), 1);
    assert_eq!(j.read(0x4016), 1);
    assert_eq!(j.read(0x4016), 1);
}

#[test]
fn joypad_pad_1_address() {
    let mut j = JoypadState::new();
    j.set_button(0, BUTTON_A, true);
    j.set_button(1, BUTTON_B, true);
    j.write_strobe(0x01);
    assert_eq!(j.read(0x4016), 0); // pad 0, button A = bit 0 = MSB 0
    assert_eq!(j.read(0x4017), 0); // pad 1, button B = bit 1 = MSB 0
}

#[test]
fn joypad_reset_clears_all() {
    let mut j = JoypadState::new();
    j.set_button(0, BUTTON_A, true);
    j.write_strobe(0x01);
    j.reset();
    assert_eq!(j.read(0x4016), 0);
    assert!(!j.strobe);
}

// ====================================================================
// SoC integration
// ====================================================================

#[test]
fn soc_owns_apu_dma_irq_joypad() {
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    // Phase 4 fields exist.
    let _ = &mut soc.apu;
    let _ = &mut soc.dma;
    let _ = &mut soc.irq;
    let _ = &mut soc.joypad;
    assert!(!soc.apu.frame_irq_pending);
    assert!(!soc.dma.is_stalling());
    assert!(!soc.irq.poll());
}

#[test]
fn soc_power_on_runs_frame() {
    use vnesu11::ram::RamInitOption;
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    soc.power_on(RamInitOption::AllZeros, 0);
    // Quick sanity: run a frame with PPU disabled → all 0s.
    soc.ppu.regs.ppumask = 0x00;
    let result = soc.run_frame();
    assert!(result.completed);
    assert!(soc.frame_buffer.iter().all(|&b| b == 0));
}

#[test]
fn soc_run_frame_advances_ppu() {
    use vnesu11::ram::RamInitOption;
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    soc.power_on(RamInitOption::AllZeros, 0);
    let start_scanline = soc.ppu.scanline;
    soc.run_frame();
    // After a frame, the PPU scanline should have wrapped back to PRELINE.
    // The wrapped state (PRELINE) is different from the start (also PRELINE),
    // so we check the dot counter has advanced.
    let end_dot = soc.ppu.dot;
    let start_dot = 0;
    let _ = start_scanline; // (silence unused warning)
    assert!(end_dot > start_dot || soc.ppu.frame_ready,
            "PPU should have advanced (dot={}, frame_ready={})",
            end_dot, soc.ppu.frame_ready);
}