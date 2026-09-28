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
