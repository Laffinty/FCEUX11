//! SWI dispatch (BIOS calls 0x00-0x2A).
//!
//! One entry point, [`dispatch`], which the core calls through
//! `Arm7tdmi::swi_hook` *before* its own `match`. Anything we do not claim
//! returns `false`, and the core falls through to its built-in arms and then
//! to the real BIOS image.

pub mod decompress;
pub mod wait;

use gba_core::cpu::arm7tdmi::Arm7tdmi;
use gba_core::cpu::psr::Psr;

/// Handle one SWI.
///
/// Returns `true` if we serviced it (the core then returns from the
/// exception), `false` to let the core's own `match` try.
pub fn dispatch(cpu: &mut Arm7tdmi, swi_num: u32, old_cpsr: Psr, return_addr: u32) -> bool {
    let _ = (cpu, old_cpsr, return_addr);
    #[cfg(test)]
    note_dispatch(swi_num);
    let swi = Swi::from_raw(swi_num);
    if tracing_enabled() {
        match swi {
            Some(s) => eprintln!("[gba] swi {swi_num:#04x} {}: not implemented", s.name()),
            None => eprintln!("[gba] swi {swi_num:#04x}: outside 0x00-0x2A"),
        }
    }
    // S0' proves the seam only: nothing is claimed yet, so every call falls
    // through to the core's own match and then to the real BIOS. S1 claims
    // numbers one at a time (T1-b first, then T1-d, then T2).
    false
}

/// Whether to echo SWI dispatches to stderr.
///
/// Off by default: stderr in a shipping build is a firehose, and the jsmolka
/// suite drives thousands of SWIs. The S0' self-test turns it on.
static TRACE_SWI: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Turn SWI tracing on or off. Simulation thread only.
pub fn set_trace(on: bool) {
    TRACE_SWI.store(on, std::sync::atomic::Ordering::Relaxed);
}

fn tracing_enabled() -> bool {
    TRACE_SWI.load(std::sync::atomic::Ordering::Relaxed)
}

// Records the number the core last handed to `dispatch`.
//
// The core reaches us through a raw function pointer, so there is no other
// way for a test to see that a real `SWI` instruction actually arrived --
// `swi_hook.is_some()` only proves we were installed, which the core would
// honour even if the call site were deleted. Compiled out of every build
// that is not a test, and thread-local because the test harness runs tests
// in parallel and one test's `SWI` must not answer another's question.
#[cfg(test)]
thread_local! {
    static LAST_DISPATCH: std::cell::Cell<u32> = const { std::cell::Cell::new(u32::MAX) };
}

#[cfg(test)]
fn note_dispatch(swi_num: u32) {
    LAST_DISPATCH.with(|slot| slot.set(swi_num));
}

/// The number [`dispatch`] was last called with on this thread, or `None`
/// since the last read. A real SWI number is a byte, so `u32::MAX` is
/// unambiguous.
#[cfg(test)]
fn take_last_dispatch() -> Option<u32> {
    let seen = LAST_DISPATCH.with(|slot| slot.replace(u32::MAX));
    (seen != u32::MAX).then_some(seen)
}

/// BIOS software-interrupt numbers, as encoded by the ARM `SWI` immediate.
///
/// 0x00-0x2A is the range the GBA BIOS documents. Anything outside it is
/// undefined; the core returns `false` for those, which drops the CPU into
/// the exception vector -- with a stub BIOS loaded that is an unimplemented
/// vector, so SWI coverage is not optional (plan section 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Swi {
    SoftReset = 0x00,
    RegisterRamReset = 0x01,
    Halt = 0x02,
    Stop = 0x03,
    IntrWait = 0x04,
    VBlankIntrWait = 0x05,
    Div = 0x06,
    DivArm = 0x07,
    Sqrt = 0x08,
    ArcTan = 0x09,
    ArcTan2 = 0x0A,
    CpuSet = 0x0B,
    CpuFastSet = 0x0C,
    GetBiosChecksum = 0x0D,
    BitUnPack = 0x0E,
    Lz77UnCompWram = 0x10,
    Lz77UnCompVram = 0x11,
    HuffUnComp = 0x12,
    RlUnCompWram = 0x13,
    RlUnCompVram = 0x14,
    Diff8BitUnFilterWram = 0x15,
    Diff8BitUnFilterVram = 0x16,
    Diff16BitUnFilter = 0x17,
    SoundBias = 0x18,
    SoundDriverInit = 0x19,
    MidiKey2Freq = 0x1A,
    SoundDriverMode = 0x1B,
    SoundDriverMain = 0x1C,
    SoundDriverClear = 0x1D,
    MultiBoot = 0x1E,
    HardReset = 0x1F,
}

impl Swi {
    /// Map a raw SWI immediate onto a known BIOS call, or `None` if the number
    /// is outside the documented range.
    pub fn from_raw(raw: u32) -> Option<Swi> {
        Some(match raw {
            0x00 => Swi::SoftReset,
            0x01 => Swi::RegisterRamReset,
            0x02 => Swi::Halt,
            0x03 => Swi::Stop,
            0x04 => Swi::IntrWait,
            0x05 => Swi::VBlankIntrWait,
            0x06 => Swi::Div,
            0x07 => Swi::DivArm,
            0x08 => Swi::Sqrt,
            0x09 => Swi::ArcTan,
            0x0A => Swi::ArcTan2,
            0x0B => Swi::CpuSet,
            0x0C => Swi::CpuFastSet,
            0x0D => Swi::GetBiosChecksum,
            0x0E => Swi::BitUnPack,
            0x10 => Swi::Lz77UnCompWram,
            0x11 => Swi::Lz77UnCompVram,
            0x12 => Swi::HuffUnComp,
            0x13 => Swi::RlUnCompWram,
            0x14 => Swi::RlUnCompVram,
            0x15 => Swi::Diff8BitUnFilterWram,
            0x16 => Swi::Diff8BitUnFilterVram,
            0x17 => Swi::Diff16BitUnFilter,
            0x18 => Swi::SoundBias,
            0x19 => Swi::SoundDriverInit,
            0x1A => Swi::MidiKey2Freq,
            0x1B => Swi::SoundDriverMode,
            0x1C => Swi::SoundDriverMain,
            0x1D => Swi::SoundDriverClear,
            0x1E => Swi::MultiBoot,
            0x1F => Swi::HardReset,
            _ => return None,
        })
    }

    /// Human-readable name, for trace output and the SWI coverage table.
    pub const fn name(self) -> &'static str {
        match self {
            Swi::SoftReset => "SoftReset",
            Swi::RegisterRamReset => "RegisterRamReset",
            Swi::Halt => "Halt",
            Swi::Stop => "Stop",
            Swi::IntrWait => "IntrWait",
            Swi::VBlankIntrWait => "VBlankIntrWait",
            Swi::Div => "Div",
            Swi::DivArm => "DivArm",
            Swi::Sqrt => "Sqrt",
            Swi::ArcTan => "ArcTan",
            Swi::ArcTan2 => "ArcTan2",
            Swi::CpuSet => "CpuSet",
            Swi::CpuFastSet => "CpuFastSet",
            Swi::GetBiosChecksum => "GetBiosChecksum",
            Swi::BitUnPack => "BitUnPack",
            Swi::Lz77UnCompWram => "Lz77UnCompWram",
            Swi::Lz77UnCompVram => "Lz77UnCompVram",
            Swi::HuffUnComp => "HuffUnComp",
            Swi::RlUnCompWram => "RlUnCompWram",
            Swi::RlUnCompVram => "RlUnCompVram",
            Swi::Diff8BitUnFilterWram => "Diff8BitUnFilterWram",
            Swi::Diff8BitUnFilterVram => "Diff8BitUnFilterVram",
            Swi::Diff16BitUnFilter => "Diff16BitUnFilter",
            Swi::SoundBias => "SoundBias",
            Swi::SoundDriverInit => "SoundDriverInit",
            Swi::MidiKey2Freq => "MidiKey2Freq",
            Swi::SoundDriverMode => "SoundDriverMode",
            Swi::SoundDriverMain => "SoundDriverMain",
            Swi::SoundDriverClear => "SoundDriverClear",
            Swi::MultiBoot => "MultiBoot",
            Swi::HardReset => "HardReset",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Swi, dispatch, take_last_dispatch};
    use crate::gba::install_swi_hook;
    use gba_core::gba::Gba;

    /// Where a cartridge is mapped; ROM offset 0 lives at this address.
    const ROM_BASE: u32 = 0x0800_0000;

    /// ARM encoding of `SWI n` with the `always` condition: `cond 1111 imm24`.
    fn arm_swi(n: u32) -> u32 {
        0xEF00_0000 | (n & 0xFF)
    }

    /// A cartridge whose opening instructions are `SWI n`.
    ///
    /// The word is repeated across the first pipeline window on purpose. The
    /// core recovers the immediate from `PC-8`, and how many fetch steps it
    /// takes to get there is the core's own bookkeeping; padding keeps this
    /// test about *our* seam instead of re-testing the pipeline.
    fn cartridge_with_swi(n: u32) -> Vec<u8> {
        let mut rom = vec![0u8; 0x200];
        for word in rom[..0xC0].chunks_exact_mut(4) {
            word.copy_from_slice(&arm_swi(n).to_le_bytes());
        }
        rom[0xC0..0xC4].copy_from_slice(&ROM_BASE.to_le_bytes());
        rom
    }

    /// Boot a machine whose ROM starts with `SWI n` and report the number
    /// `dispatch` was handed, or `None` if the core never asked.
    fn swi_seen_by_dispatch(n: u32) -> Option<u32> {
        let _ = take_last_dispatch();
        let mut gba = Gba::new([0u8; 0x4000], &cartridge_with_swi(n));
        install_swi_hook(&mut gba);
        // The ARM7TDMI pipeline executes the instruction at PC-8, and the
        // core recovers the immediate from that same address, so PC starts
        // one instruction past the SWI we want it to run.
        gba.cpu.registers.set_program_counter(ROM_BASE + 8);
        (0..32).find_map(|_| {
            gba.step();
            take_last_dispatch()
        })
    }

    /// The S0' exit criterion: a real `SWI` instruction in a real cartridge
    /// has to reach *our* function, carrying the number it encoded. Checking
    /// only that the hook pointer was installed would pass even if the core's
    /// call site were gone.
    #[test]
    fn core_calls_our_dispatch_with_the_encoded_swi_number() {
        // Spread over the documented range: the first number, the first one
        // past the core's own match, and one from the middle.
        for n in [0x00u32, 0x0B, 0x12] {
            assert_eq!(
                swi_seen_by_dispatch(n),
                Some(n),
                "SWI {n:#04x} never reached swi::dispatch"
            );
        }
    }

    /// The seam is consulted before any range check, so an undefined number
    /// still reaches us -- which is what lets S1 claim it.
    #[test]
    fn dispatch_is_consulted_for_numbers_outside_the_bios_range() {
        assert_eq!(swi_seen_by_dispatch(0x2B), Some(0x2B));
    }

    /// S0' claims nothing: every number falls through to the core's own
    /// match. Claiming is S1's job, and this test is what fails the day one
    /// lands without its own coverage.
    #[test]
    fn s0_dispatch_claims_nothing() {
        let mut gba = Gba::new([0u8; 0x4000], &[0u8; 0x200]);
        let cpsr = gba.cpu.cpsr;
        for n in 0..=0xFFu32 {
            assert!(
                !dispatch(&mut gba.cpu, n, cpsr, ROM_BASE),
                "S0' must not claim SWI {n:#04x}"
            );
        }
    }

    /// `from_raw` is the table S1 will claim numbers out of; a number it
    /// maps has to survive the round trip to its BIOS name.
    #[test]
    fn every_mapped_number_round_trips_through_its_name() {
        for n in 0..=0x2Au32 {
            let Some(swi) = Swi::from_raw(n) else {
                continue;
            };
            assert_eq!(swi as u32, n);
            assert!(!swi.name().is_empty());
        }
        assert_eq!(Swi::from_raw(0x2B), None);
    }
}
