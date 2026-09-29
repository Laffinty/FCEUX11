//! SWI dispatch (BIOS calls 0x00-0x2A).
//!
//! One entry point, [`dispatch`], which the core calls through
//! `Arm7tdmi::swi_hook` *before* its own `match`. Anything we do not claim
//! returns `false`, and the core falls through to its built-in arms and then
//! to the real BIOS image.

pub mod decompress;
pub mod ram_reset;
pub mod wait;

use gba_core::cpu::arm7tdmi::Arm7tdmi;
use gba_core::cpu::psr::Psr;

use ram_reset::{IE, IF, IME, RamResetRequest, WAITCNT};
use wait::{IntrWaitRequest, LowPower};

/// Handle one SWI.
///
/// Returns `true` if we serviced it (the core then returns from the
/// exception), `false` to let the core's own `match` try.
pub fn dispatch(cpu: &mut Arm7tdmi, swi_num: u32, old_cpsr: Psr, return_addr: u32) -> bool {
    #[cfg(test)]
    note_dispatch(swi_num);
    let swi = Swi::from_raw(swi_num);
    match swi {
        Some(Swi::RegisterRamReset) => {
            serve_register_ram_reset(cpu, old_cpsr, return_addr);
            true
        }
        Some(Swi::Halt) | Some(Swi::Stop) => {
            enter_low_power(cpu, old_cpsr, return_addr, swi);
            true
        }
        Some(Swi::IntrWait) | Some(Swi::VBlankIntrWait) => {
            enter_intr_wait(cpu, old_cpsr, return_addr, swi_num);
            true
        }
        None | Some(_) => {
            if tracing_enabled() {
                match swi {
                    Some(s) => eprintln!("[gba] swi {swi_num:#04x} {}: not implemented", s.name()),
                    None => eprintln!("[gba] swi {swi_num:#04x}: outside 0x00-0x2A"),
                }
            }
            false
        }
    }
}

/// Serve `RegisterRamReset` (0x01).
///
/// Claiming this number is all-or-nothing: the core's own arm never runs
/// behind us, so bits 0/2/3/4 are reimplemented here at the same addresses
/// rather than left to it. What it never did is the IWRAM clear (bit 1) and
/// the `0x80` I/O reset; those are the reason this arm exists. See the v2.0
/// plan r15 and known limits L5 / L6.
fn serve_register_ram_reset(cpu: &mut Arm7tdmi, old_cpsr: Psr, return_addr: u32) {
    let request = RamResetRequest::decode(cpu.registers.register_at(0));

    for region in request.regions() {
        for addr in region.start..region.end {
            cpu.bus.write_byte(addr as usize, 0);
        }
    }

    if request.resets_io() {
        // IF is write-one-to-clear, so 0xFFFF acknowledges every pending
        // interrupt; the other three are plain zero.
        cpu.bus.write_half_word(IE as usize, 0);
        cpu.bus.write_half_word(IF as usize, 0xFFFF);
        cpu.bus.write_half_word(WAITCNT as usize, 0);
        cpu.bus.write_half_word(IME as usize, 0);
    }

    // Say so rather than letting a game believe its SIO registers were reset.
    let unimplemented = request.wants_unimplemented_groups();
    if unimplemented != 0 && tracing_enabled() {
        eprintln!(
            "[gba] swi 0x01: register group {unimplemented:#04x} left alone (known limit L6)"
        );
    }

    cpu.swi_return(old_cpsr, return_addr);
}

/// Park the core until an enabled interrupt arrives.
///
/// `Halt` and `Stop` share everything except *why* the CPU wakes, and the
/// difference is not modelled this round: both wake on any enabled interrupt.
/// GBATEK has `Stop` exit on a condition selected through the control
/// register at `04000301h`; games essentially never use `Stop`, and modelling
/// that register faithfully is recorded as a known limitation rather than
/// guessed at. `wake_hook = None` is what makes the core's guard treat any
/// enabled interrupt as sufficient.
fn enter_low_power(cpu: &mut Arm7tdmi, old_cpsr: Psr, return_addr: u32, swi: Option<Swi>) {
    if tracing_enabled() {
        if let Some(mode) = swi.and_then(|s| match s {
            Swi::Halt => Some(LowPower::Halt),
            Swi::Stop => Some(LowPower::Stop),
            _ => None,
        }) {
            eprintln!("[gba] swi {}: entering {mode:?}", mode.swi_number());
        }
    }
    PENDING_INTR_WAIT.with(|pending| pending.set(None));
    cpu.wake_hook = None;
    // Return to the caller *first*, then sleep: the instruction after the SWI
    // is the one the CPU stops at, which is what makes a resume land in the
    // right place.
    cpu.swi_return(old_cpsr, return_addr);
    cpu.halted = true;
}

// The `IntrWait` / `VBlankIntrWait` request the machine is currently asleep on.
//
// Thread-local because the wake predicate the core stores is a bare `fn`
// pointer with nowhere to carry state. One simulation thread per machine is
// the §4.1 contract, so this is one outstanding wait at a time.
thread_local! {
    static PENDING_INTR_WAIT: std::cell::Cell<Option<IntrWaitRequest>> =
        const { std::cell::Cell::new(None) };
}

fn read_bios_flags(cpu: &mut Arm7tdmi) -> u8 {
    cpu.bus.read_byte(wait::BIOS_FLAGS_ADDR as usize)
}

fn write_bios_flags(cpu: &mut Arm7tdmi, flags: u8) {
    cpu.bus.write_byte(wait::BIOS_FLAGS_ADDR as usize, flags);
}

/// Serve an `IntrWait`, or sleep until its flag arrives.
fn enter_intr_wait(cpu: &mut Arm7tdmi, old_cpsr: Psr, return_addr: u32, swi_num: u32) {
    let request = if swi_num == Swi::VBlankIntrWait as u32 {
        wait::vblank_intr_wait()
    } else {
        // r0 is a mask of the flags to wait for, r1 selects the mode.
        IntrWaitRequest::decode(cpu.registers.register_at(0), cpu.registers.register_at(1))
    };

    if request.is_unsatisfiable() {
        // The BIOS would block forever on a request that names no defined
        // flag. Returning instead of hanging turns a wedged machine into a
        // visible no-op, and the trace says which number it was.
        if tracing_enabled() {
            eprintln!("[gba] swi {swi_num:#04x}: IntrWait names no defined flag, returning");
        }
        cpu.swi_return(old_cpsr, return_addr);
        return;
    }

    let flags = read_bios_flags(cpu);
    if request.satisfied_by(flags) {
        // Already satisfied: take the flag we were asked for and go. The BIOS
        // clears the flag it consumed, which is what lets a later
        // `VBlankIntrWait` wait for the *next* frame.
        write_bios_flags(cpu, request.flags_after(flags));
        cpu.swi_return(old_cpsr, return_addr);
        return;
    }

    PENDING_INTR_WAIT.with(|pending| pending.set(Some(request)));
    cpu.wake_hook = Some(wake_for_intr_wait);
    cpu.swi_return(old_cpsr, return_addr);
    cpu.halted = true;
}

/// Wake predicate: keep sleeping until the awaited flag is set, then consume it.
///
/// Returning `true` means "stay asleep", which is what lets one interrupt wake
/// a `Halt` but not an `IntrWait` that is waiting for a different group.
fn wake_for_intr_wait(cpu: &mut Arm7tdmi) -> bool {
    let Some(request) = PENDING_INTR_WAIT.with(|pending| pending.take()) else {
        return false;
    };
    let flags = read_bios_flags(cpu);
    if !request.is_set(flags) {
        PENDING_INTR_WAIT.with(|pending| pending.set(Some(request)));
        if tracing_enabled() {
            eprintln!(
                "[gba] IntrWake: flag {:#04x} not set yet, staying asleep",
                flags
            );
        }
        return true;
    }
    write_bios_flags(cpu, request.flags_after(flags));
    false
}

/// Whether to echo SWI dispatches to stderr.
///
/// Off by default: stderr in a shipping build is a firehose, and the jsmolka
/// suite drives thousands of SWIs. The S0' self-test turns it on.
static TRACE_SWI: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
    use super::{Swi, dispatch, ram_reset, read_bios_flags, take_last_dispatch, write_bios_flags};
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

    /// S0' claimed nothing and this test was written to fail the day a number
    /// landed. S1a-1 replaced it with the wait family, and S1b-c has added
    /// `RegisterRamReset`. Everything else still has to fall through, so
    /// T1-d (decompressors) and T2 (math) can land one at a time.
    #[test]
    fn dispatch_claims_exactly_what_is_implemented() {
        let mut gba = Gba::new([0u8; 0x4000], &[0u8; 0x200]);
        let cpsr = gba.cpu.cpsr;
        let claimed: Vec<u32> = (0..=0xFFu32)
            .filter(|n| dispatch(&mut gba.cpu, *n, cpsr, ROM_BASE))
            .collect();
        for expected in [
            Swi::RegisterRamReset as u32,
            Swi::Halt as u32,
            Swi::Stop as u32,
            Swi::IntrWait as u32,
            Swi::VBlankIntrWait as u32,
        ] {
            assert!(
                claimed.contains(&expected),
                "SWI {expected:#04x} must be claimed"
            );
        }
        // Everything outside the claim set still has to fall through to the
        // core, so T1-d (decompressors) and T2 (math) can land one at a time.
        assert_eq!(
            claimed,
            vec![
                Swi::RegisterRamReset as u32,
                Swi::Halt as u32,
                Swi::Stop as u32,
                Swi::IntrWait as u32,
                Swi::VBlankIntrWait as u32
            ],
            "only the wait family and RegisterRamReset are claimed so far"
        );
    }

    /// A number we decline must leave the machine exactly as it was -- no
    /// sleep, no wake predicate, no BIOS flag touched. This is what lets the
    /// core's own `match` and the real BIOS keep their say for every number we
    /// have not implemented yet.
    #[test]
    fn declining_a_number_has_no_side_effects() {
        let mut gba = Gba::new([0u8; 0x4000], &[0u8; 0x200]);
        let cpsr = gba.cpu.cpsr;
        for n in 0..=0xFFu32 {
            let is_claimed = matches!(
                Swi::from_raw(n),
                Some(
                    Swi::RegisterRamReset
                        | Swi::Halt
                        | Swi::Stop
                        | Swi::IntrWait
                        | Swi::VBlankIntrWait
                )
            );
            if is_claimed {
                continue;
            }
            assert!(!dispatch(&mut gba.cpu, n, cpsr, ROM_BASE), "{n:#04x}");
            assert!(!gba.cpu.halted, "SWI {n:#04x} parked the machine");
            assert!(
                gba.cpu.wake_hook.is_none(),
                "SWI {n:#04x} installed a wake predicate"
            );
        }
        assert_eq!(
            read_bios_flags(&mut gba.cpu),
            0,
            "declining must not touch the BIOS flag word"
        );
    }

    /// A cartridge whose entry point is the given ARM program.
    ///
    /// The words start at ROM offset 8: the pipeline executes the instruction
    /// at `PC` after a warm-up, and setting the program counter to
    /// `ROM_BASE + 8` makes offset 8 the one that runs.
    fn cart_with_program(words: &[u32]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x200];
        for (i, word) in words.iter().enumerate() {
            let at = 8 + i * 4;
            rom[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        rom[0xC0..0xC4].copy_from_slice(&ROM_BASE.to_le_bytes());
        rom
    }

    /// A machine already positioned to run `cart_with_program`.
    fn machine(program: &[u32]) -> gba_core::gba::Gba {
        let mut gba = Gba::new([0u8; 0x4000], &cart_with_program(program));
        install_swi_hook(&mut gba);
        gba.cpu.registers.set_program_counter(ROM_BASE + 8);
        gba
    }

    /// Run until the CPU parks, and report whether it did.
    fn run_until_halted(gba: &mut gba_core::gba::Gba, max_steps: usize) -> bool {
        (0..max_steps).any(|_| {
            gba.step();
            gba.cpu.halted
        })
    }

    /// Run a fixed number of cycles. Used where the point is that the CPU
    /// keeps going, so "did it park" is not the question being asked.
    fn run(gba: &mut gba_core::gba::Gba, steps: usize) {
        for _ in 0..steps {
            gba.step();
        }
    }

    const IRQ_HALT: u32 = 0xE3A0_5034; // mov r5, #0x34
    const PARK: u32 = 0xEAFF_FFFE; // b .

    /// `Halt` has to actually stop the CPU: the instruction after the SWI must
    /// not run, and the machine must keep advancing anyway, because that is
    /// what lets an interrupt ever arrive.
    #[test]
    fn halt_stops_the_cpu_and_the_machine_keeps_running() {
        let program = [arm_swi(0x02), IRQ_HALT, PARK];
        let mut gba = machine(&program);
        assert!(run_until_halted(&mut gba, 32), "SWI 0x02 did not halt");

        let cycles_at_halt = gba.cpu.current_cycle;
        for _ in 0..64 {
            gba.step();
        }
        assert!(
            gba.cpu.halted,
            "a Halt with interrupts disabled must stay asleep"
        );
        assert_eq!(
            gba.cpu.registers.register_at(5),
            0,
            "the instruction after SWI 0x02 ran while halted"
        );
        assert!(
            gba.cpu.current_cycle > cycles_at_halt,
            "the machine clock stopped while the CPU slept"
        );
    }

    /// Arm a keypad interrupt, the one source a test can raise without
    /// waiting a frame for VBlank. IF is *not* written directly: on hardware
    /// (and in this core) writing 1 to a request bit clears it, so software
    /// cannot conjure a pending interrupt -- it has to come from a peripheral.
    fn press_key_to_raise_an_irq(gba: &mut gba_core::gba::Gba) {
        // KEYCNT: bit 14 enables the interrupt, bits 0-9 select the keys,
        // bit 15 would mean AND; 0 means "any selected key".
        gba.cpu.bus.keypad.key_interrupt_control = (1 << 14) | 0x0001;
        // KEYINPUT is active low: clear bit 0 to press A.
        gba.cpu.bus.keypad.key_input &= !0x0001;
        gba.cpu.bus.write_half_word(0x0400_0200, 1 << 12); // IE  = keypad
        gba.cpu.bus.write_half_word(0x0400_0208, 0x0001); // IME = enable
    }

    /// An enabled interrupt wakes `Halt`, and it is taken in the same cycle:
    /// the CPU must land on the BIOS IRQ vector, not execute one more
    /// instruction first.
    ///
    /// The interrupt is raised *after* the CPU parks. Raising it first would
    /// have the IRQ taken long before the cartridge reached its `SWI`.
    #[test]
    fn halt_wakes_on_an_enabled_interrupt() {
        let program = [arm_swi(0x02), IRQ_HALT, PARK];
        let mut gba = machine(&program);
        assert!(run_until_halted(&mut gba, 32), "SWI 0x02 did not halt");
        assert!(gba.cpu.halted, "it should still be asleep before the IRQ");

        press_key_to_raise_an_irq(&mut gba);

        // The keypad raises IF from the bus step, so the wake lands on a later
        // cycle than the one where the key state was written -- the same lag
        // real hardware has between a key press and the interrupt.
        let woke = (0..8).any(|_| {
            gba.step();
            !gba.cpu.halted
        });
        assert!(woke, "an enabled interrupt must wake a Halt");
        assert_eq!(
            gba.cpu.registers.program_counter(),
            0x18,
            "the IRQ must be taken on the same cycle we wake, not a cycle later"
        );
    }

    /// With the master enable clear, nothing wakes the CPU -- a machine left
    /// asleep with interrupts off is how a hang reproduces.
    #[test]
    fn halt_does_not_wake_while_interrupts_are_masked() {
        let program = [arm_swi(0x02), IRQ_HALT, PARK];
        let mut gba = machine(&program);
        gba.cpu.bus.write_half_word(0x0400_0200, 0x0001); // IE = VBlank
        gba.cpu.bus.write_half_word(0x0400_0202, 0x0001); // IF = VBlank
        gba.cpu.bus.write_half_word(0x0400_0208, 0x0000); // IME = disabled

        assert!(run_until_halted(&mut gba, 32), "SWI 0x02 did not halt");
        for _ in 0..64 {
            gba.step();
        }
        assert!(gba.cpu.halted, "a pending but masked IRQ must not wake us");
    }

    /// `IntrWait` is the reason the wake predicate exists: an interrupt is
    /// pending, but the wait is for one specific BIOS flag, and until the game
    /// reports it the CPU stays asleep. `Halt` would have woken here.
    #[test]
    fn intr_wait_ignores_interrupts_it_did_not_ask_for() {
        // r0 = 1 (VBlank flag), r1 = 0 (return if already set).
        let program = [
            0xE3A0_0001,   // mov r0, #1
            arm_swi(0x04), // swi IntrWait
            IRQ_HALT,
            PARK,
        ];
        let mut gba = machine(&program);
        assert!(
            run_until_halted(&mut gba, 32),
            "SWI 0x04 should have slept waiting for the VBlank flag"
        );

        // Now a keypad interrupt actually arrives, and the game's handler has
        // not reported the flag yet. A Halt would wake here; IntrWait must not.
        press_key_to_raise_an_irq(&mut gba);
        run(&mut gba, 32);
        assert!(
            gba.cpu.halted,
            "a pending IRQ must not satisfy an IntrWait that wants a flag"
        );
        assert_eq!(read_bios_flags(&mut gba.cpu), 0, "no flag was reported");

        // Now the game's handler reports VBlank, and the wait is over.
        write_bios_flags(&mut gba.cpu, 0x01);
        gba.step();
        assert!(!gba.cpu.halted, "the reported flag must wake the wait");
    }

    /// A satisfied wait consumes the flag it was waiting for. That is what
    /// lets the next `VBlankIntrWait` wait for the *next* frame instead of
    /// returning immediately forever.
    #[test]
    fn intr_wait_consumes_the_flag_it_waited_for() {
        let program = [
            0xE3A0_0001,   // mov r0, #1
            arm_swi(0x04), // swi IntrWait -- already satisfied, must not sleep
            IRQ_HALT,
            PARK,
        ];
        let mut gba = machine(&program);
        write_bios_flags(&mut gba.cpu, 0x01 | 0x02);

        run(&mut gba, 16);
        assert!(
            !gba.cpu.halted,
            "an already-set flag must not make us sleep"
        );
        assert_eq!(
            gba.cpu.registers.register_at(5),
            0x34,
            "the SWI returned and execution carried on"
        );
        assert_eq!(
            read_bios_flags(&mut gba.cpu),
            0x02,
            "the waited-for bit must be cleared, the others untouched"
        );
    }

    /// `VBlankIntrWait` is `IntrWait(1, 1)`, and the trailing 1 is the whole
    /// point: it must sleep even when the flag is already set.
    #[test]
    fn vblank_intr_wait_always_sleeps() {
        let program = [arm_swi(0x05), IRQ_HALT, PARK];
        let mut gba = machine(&program);
        write_bios_flags(&mut gba.cpu, 0x01);

        assert!(
            run_until_halted(&mut gba, 32),
            "execution never reached the SWI"
        );
        assert!(
            gba.cpu.halted,
            "VBlankIntrWait must always sleep, flag or no flag"
        );
    }

    /// A request that names none of the four defined flags can never be
    /// satisfied. The BIOS would block forever; we return instead, so a
    /// mistaken argument shows up as a no-op rather than a wedged machine.
    #[test]
    fn intr_wait_on_an_undefined_flag_returns_instead_of_hanging() {
        let program = [
            0xE3A0_0010,   // mov r0, #0x10 -- bit 4, not a defined flag
            arm_swi(0x04), // swi IntrWait
            IRQ_HALT,
            PARK,
        ];
        let mut gba = machine(&program);
        run(&mut gba, 16);
        assert!(
            !gba.cpu.halted,
            "an unsatisfiable wait must not park the CPU"
        );
        assert_eq!(
            gba.cpu.registers.register_at(5),
            0x34,
            "the SWI returned and execution carried on"
        );
    }

    // ---- T1-a regression -------------------------------------------------
    //
    // S1a-1 claims four SWI numbers, which puts the core's own implementations
    // of the rest one layer further away than they were. These measure them
    // through the real path -- a cartridge executing the instruction, not a
    // call into the core -- so what is verified is the whole seam, not just
    // the arithmetic.

    /// `MOV r1, #imm` for an 8-bit immediate.
    const fn mov_r1(v: u32) -> u32 {
        0xE3A0_1000 | (v & 0xFF)
    }
    /// `MVN r0, r0` -- all ones.
    const MVN_R0_R0: u32 = 0xE3E0_0000;
    /// `BIC r0, r0, #6`
    const BIC_R0_R0_6: u32 = 0xE3C0_0006;
    /// `LDR r0, [pc, #12]`
    const LDR_R0_PC_12: u32 = 0xE59F_000C;
    /// `LDR r1, [pc, #12]`
    const LDR_R1_PC_12: u32 = 0xE59F_100C;
    /// `LDR r2, [pc, #12]`
    const LDR_R2_PC_12: u32 = 0xE59F_200C;
    /// Halt, so a test can stop the machine and read the results.
    const SWI_HALT_WORD: u32 = 0xEF00_0002;

    /// `Div` (0x06) is signed, truncates toward zero, and leaves the absolute
    /// quotient in r3 -- the three properties games depend on.
    #[test]
    fn t1a_div_is_signed_and_truncates_toward_zero() {
        // r0 = -7 has to be assembled: an ARM immediate is an 8-bit value, so
        // neither `mov r0, #0xF9` (+249) nor a single rotated byte reaches it.
        //   mov r0, #0 ; mvn r0, r0 ; bic r0, r0, #6 ; mov r1, #3
        let program = [
            0xE3A0_0000,
            MVN_R0_R0,
            BIC_R0_R0_6,
            mov_r1(3),
            arm_swi(0x06),
            SWI_HALT_WORD,
        ];
        let mut gba = machine(&program);
        assert!(run_until_halted(&mut gba, 32), "did not reach the Halt");

        assert_eq!(gba.cpu.registers.register_at(0) as i32, -2, "quotient");
        assert_eq!(gba.cpu.registers.register_at(1) as i32, -1, "remainder");
        assert_eq!(
            gba.cpu.registers.register_at(3) as i32,
            2,
            "absolute quotient"
        );
    }

    /// `CpuFastSet` (0x0C) moves 32-bit blocks and is what the C runtime and
    /// the decompressors actually call. The three arguments are 16-bit
    /// constants, so they are loaded from a literal pool placed after the
    /// `Halt` -- the return address of the SWI is the instruction right after
    /// it, so anything between the SWI and the pool would be executed.
    #[test]
    fn t1a_cpu_fast_set_moves_32_bit_blocks() {
        const SRC: u32 = 0x0300_0000;
        const DST: u32 = 0x0300_0100;
        // r2: block count in bits 8-23, bit 26 selects 32-bit access.
        const COUNT_32BIT_4: u32 = 0x0400_0000 | 4 << 8;

        let program = [
            LDR_R0_PC_12,
            LDR_R1_PC_12,
            LDR_R2_PC_12,
            arm_swi(0x0C),
            SWI_HALT_WORD,
            SRC,
            DST,
            COUNT_32BIT_4,
        ];
        let mut gba = machine(&program);
        for i in 0..4u32 {
            gba.cpu
                .bus
                .write_word((SRC + i * 4) as usize, 0x1111_0000 | i);
        }
        assert!(run_until_halted(&mut gba, 32), "did not reach the Halt");

        for i in 0..4u32 {
            assert_eq!(
                gba.cpu.bus.read_word((DST + i * 4) as usize),
                0x1111_0000 | i,
                "word {i} was not copied"
            );
        }
    }

    /// Our claim set must not swallow the numbers the core implements. This is
    /// the invariant that keeps T1-d and T2 free to land one at a time.
    ///
    /// `RegisterRamReset` was on this list until S1b-c claimed it -- that is
    /// the one entry whose removal is a change in behaviour rather than an
    /// omission, and it is why claiming a number means reimplementing the
    /// whole of it (plan r15).
    #[test]
    fn t1a_dispatch_leaves_the_other_core_arms_reachable() {
        let mut gba = Gba::new([0u8; 0x4000], &[0u8; 0x200]);
        let cpsr = gba.cpu.cpsr;
        for n in [
            Swi::SoftReset,
            Swi::Div,
            Swi::DivArm,
            Swi::CpuSet,
            Swi::CpuFastSet,
        ] {
            assert!(
                !dispatch(&mut gba.cpu, n as u32, cpsr, ROM_BASE),
                "SWI {n:?} must fall through to the core"
            );
        }
    }

    // ---- S1b-c: RegisterRamReset ---------------------------------------
    //
    // Claiming a number is all-or-nothing, so these are measured through the
    // real `dispatch` and assert what the machine looks like afterwards --
    // not that a function returned.

    /// A machine with the hook installed and a flag byte staged in `r0`.
    fn gba_for_ram_reset(flags: u8) -> Gba {
        let mut gba = Gba::new([0u8; 0x4000], &[0u8; 0x200]);
        install_swi_hook(&mut gba);
        gba.cpu.registers.set_register_at(0, flags as u32);
        gba
    }

    /// Run `RegisterRamReset` with the staged `r0`.
    fn run_ram_reset(gba: &mut Gba) {
        let cpsr = gba.cpu.cpsr;
        assert!(
            dispatch(&mut gba.cpu, Swi::RegisterRamReset as u32, cpsr, ROM_BASE),
            "0x01 must be claimed"
        );
    }

    /// Bit 0 clears EWRAM but keeps the last `0x200` bytes. The core already
    /// did this, so it is a regression guard on our reimplementation rather
    /// than a new feature -- claiming the number took the duty over.
    #[test]
    fn register_ram_reset_clears_ewram_and_keeps_its_tail() {
        let mut gba = gba_for_ram_reset(ram_reset::CLEAR_EWRAM);
        gba.cpu.bus.write_byte(0x0200_0000, 0xAB);
        gba.cpu.bus.write_byte(0x0203_FDFF, 0xCD);
        gba.cpu.bus.write_byte(0x0203_FE00, 0x11);
        gba.cpu.bus.write_byte(0x0203_FFFF, 0x22);

        run_ram_reset(&mut gba);

        assert_eq!(gba.cpu.bus.read_byte(0x0200_0000), 0, "EWRAM base cleared");
        assert_eq!(
            gba.cpu.bus.read_byte(0x0203_FDFF),
            0,
            "last cleared byte"
        );
        assert_eq!(
            gba.cpu.bus.read_byte(0x0203_FE00),
            0x11,
            "the 0x200 tail must survive"
        );
        assert_eq!(
            gba.cpu.bus.read_byte(0x0203_FFFF),
            0x22,
            "the 0x200 tail must survive"
        );
    }

    /// Bit 1 clears IWRAM, keeping the last `0x200` -- where the BIOS flags,
    /// the IRQ vector and the three stack pointers live. This is the gap the
    /// core left open with a TODO (known limit L5).
    #[test]
    fn register_ram_reset_clears_iwram_and_keeps_the_bios_tail() {
        let mut gba = gba_for_ram_reset(ram_reset::CLEAR_IWRAM);
        gba.cpu.bus.write_byte(0x0300_0000, 0xAB);
        gba.cpu.bus.write_byte(0x0300_7DFF, 0xCD);
        gba.cpu.bus.write_byte(0x0300_7FF8, 0x11);
        gba.cpu.bus.write_byte(0x0300_7FFC, 0x22);

        run_ram_reset(&mut gba);

        assert_eq!(gba.cpu.bus.read_byte(0x0300_0000), 0, "IWRAM base cleared");
        assert_eq!(
            gba.cpu.bus.read_byte(0x0300_7DFF),
            0,
            "last cleared byte"
        );
        assert_eq!(
            gba.cpu.bus.read_byte(0x0300_7FF8),
            0x11,
            "BIOS interrupt flags must survive"
        );
        assert_eq!(
            gba.cpu.bus.read_byte(0x0300_7FFC),
            0x22,
            "the IRQ vector must survive"
        );
    }

    /// The three regions the core already handled still work now that we own
    /// the number. This is the "claiming is all-or-nothing" tax, paid.
    #[test]
    fn register_ram_reset_still_clears_palette_vram_and_oam() {
        let mut gba = gba_for_ram_reset(
            ram_reset::CLEAR_PALETTE | ram_reset::CLEAR_VRAM | ram_reset::CLEAR_OAM,
        );
        gba.cpu.bus.write_byte(0x0500_0000, 0xAB);
        gba.cpu.bus.write_byte(0x0600_0000, 0xAB);
        gba.cpu.bus.write_byte(0x0601_7FFF, 0xAB);
        gba.cpu.bus.write_byte(0x0700_03FF, 0xAB);

        run_ram_reset(&mut gba);

        assert_eq!(gba.cpu.bus.read_byte(0x0500_0000), 0, "palette");
        assert_eq!(gba.cpu.bus.read_byte(0x0600_0000), 0, "VRAM base");
        assert_eq!(gba.cpu.bus.read_byte(0x0601_7FFF), 0, "VRAM top");
        assert_eq!(gba.cpu.bus.read_byte(0x0700_03FF), 0, "OAM top");
    }

    /// A flag byte with no bits set must leave the machine alone. Without
    /// this, "claims the number" and "clears everything" would be the same
    /// test.
    #[test]
    fn register_ram_reset_with_no_flags_clears_nothing() {
        let mut gba = gba_for_ram_reset(0);
        gba.cpu.bus.write_byte(0x0200_0000, 0xAB);
        gba.cpu.bus.write_byte(0x0600_0000, 0xAB);

        run_ram_reset(&mut gba);

        assert_eq!(gba.cpu.bus.read_byte(0x0200_0000), 0xAB, "EWRAM untouched");
        assert_eq!(gba.cpu.bus.read_byte(0x0600_0000), 0xAB, "VRAM untouched");
    }

    /// Bit `0x80` zeroes IE/WAITCNT/IME and acknowledges every pending IF
    /// flag. The core had three comments and no code here.
    #[test]
    fn register_ram_reset_zeroes_the_io_registers() {
        let mut gba = gba_for_ram_reset(ram_reset::RESET_IO);
        gba.cpu.bus.write_half_word(ram_reset::IE as usize, 0xFFFF);
        gba.cpu.bus.write_half_word(ram_reset::WAITCNT as usize, 0x4317);
        gba.cpu.bus.write_half_word(ram_reset::IME as usize, 1);

        run_ram_reset(&mut gba);

        assert_eq!(gba.cpu.bus.read_half_word(ram_reset::IE as usize), 0);
        assert_eq!(gba.cpu.bus.read_half_word(ram_reset::WAITCNT as usize), 0);
        assert_eq!(gba.cpu.bus.read_half_word(ram_reset::IME as usize), 0);
    }

    /// The IF half of bit `0x80` has to be measured against a flag that is
    /// genuinely raised, because IF is write-one-to-clear: there is no
    /// software way to stage one, and `interrupt_control` is private to the
    /// core. So the keypad raises it the way hardware does. IE and IME stay
    /// off so the raised flag is not immediately turned into an IRQ.
    #[test]
    fn register_ram_reset_acknowledges_pending_interrupts() {
        // Keypad is IF bit 12 on GBA; bit 7 (SIO) and bits 13-15 are the
        // other sources. The core's `IrqType::get_idx_in_if` agrees.
        const KEYPAD_IF: u16 = 1 << 12;

        let mut gba = gba_for_ram_reset(ram_reset::RESET_IO);
        gba.cpu.bus.keypad.key_interrupt_control = (1 << 14) | 0x0001;
        gba.cpu.bus.keypad.key_input &= !0x0001;
        for _ in 0..16 {
            gba.step();
        }
        assert_eq!(
            gba.cpu.bus.read_half_word(ram_reset::IF as usize) & KEYPAD_IF,
            KEYPAD_IF,
            "the keypad flag must be raised, or this test proves nothing"
        );

        run_ram_reset(&mut gba);

        assert_eq!(
            gba.cpu.bus.read_half_word(ram_reset::IF as usize),
            0,
            "every pending flag must be acknowledged"
        );
    }

    /// The call must return to the caller, not wedge the machine. The
    /// core's own arm ends in `swi_return` and so does ours, and that is what
    /// puts the PC on the return address.
    #[test]
    fn register_ram_reset_returns_to_the_caller() {
        const RETURN_ADDR: u32 = 0x18;
        let mut gba = gba_for_ram_reset(ram_reset::CLEAR_VRAM);
        gba.cpu.registers.set_program_counter(0x30);
        let cpsr = gba.cpu.cpsr;
        assert!(dispatch(
            &mut gba.cpu,
            Swi::RegisterRamReset as u32,
            cpsr,
            RETURN_ADDR
        ));
        assert_eq!(
            gba.cpu.registers.program_counter(),
            RETURN_ADDR as usize,
            "the SWI must return to its caller rather than park"
        );
        assert!(!gba.cpu.halted, "the SWI must not park the machine");
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
