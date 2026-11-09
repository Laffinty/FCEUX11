//! The SWI fixtures — S1-a, defect domain **unimplemented BIOS SWI**.
//!
//! | fixture | what it drives | belongs to |
//! |---|---|---|
//! | `swi_unimpl_1a` | ARM code executes `SWI 0x1A` (SoundDriverInit) | S2-b |
//! | `swi_unimpl_16` | ARM code executes `SWI 0x16` (Diff8BitUnFilterWram) | S2-b + S3-a |
//!
//! # The defect (v2.0.3 plan §1.2.1)
//!
//! `dispatch` only claims 16 SWI numbers. A number it does not claim is
//! declined by the core, the CPU takes the real exception path, and the
//! program counter lands on `0x08` — the stub BIOS's SWI vector, which is
//! `BRANCH_SELF` (`bios.rs:97`) — so the machine **spins forever**.
//!
//! The fixture therefore asserts the **target** behaviour: the SWI returns,
//! the program keeps running, and the reading says which number was not
//! implemented. Today that is false, which is why the reproducer is red.
//!
//! # The mechanism, step by step
//!
//! 1. The core decodes the `SWI` (`arm7tdmi.rs:657-659`) and calls
//!    `handle_exception(SoftwareInterrupt)`.
//! 2. `handle_exception` reads the number out of the instruction word —
//!    bits 16-23 (`arm7tdmi.rs:696-699`) — and asks the dispatcher first
//!    (`arm7tdmi.rs:703` → `handle_swi_hle` → `arm7tdmi.rs:1138-1142`).
//! 3. `dispatch` declines the number (`swi/mod.rs:111-119`, the
//!    `None | Some(_)` arm), so the exception really happens: the CPU
//!    switches to Supervisor mode and jumps to `0x08`
//!    (`arm7tdmi.rs:205-211`).
//! 4. `0x08` holds `BRANCH_SELF` (`bios.rs:131-134`), so the machine spins
//!    there forever.
//!
//! # The two lock tests that pin the defect in place
//!
//! `bios.rs:219-223` (`unhandled_vectors_park_instead_of_wandering`) and
//! `bios.rs:396-401` (`the_stub_does_not_implement_swi`) both assert that
//! the SWI vector is `BRANCH_SELF`. Fixing this defect therefore means
//! changing those two tests — the plan says so in §1.2.1 and §3.2.
//!
//! # How the fixture observes "the machine did not park"
//!
//! The program is three instructions:
//!
//! ```text
//!     swi  <number>       ; the call under test
//!     mov  r5, #MARKER    ; the next instruction: it must run
//!     swi  0x02           ; Halt: the terminator, and an implemented call
//! ```
//!
//! `Halt` is `0x02` and **is** implemented, so it is only ever the
//! terminator, never the thing under test. The fixture asserts that the
//! machine reached the `Halt` and that `r5` holds the marker — i.e. that
//! the instruction *after* the unimplemented call really ran.
//!
//! # How the fixture observes "the number is readable"
//!
//! The number the core handed the dispatcher is what the fallback has to
//! make visible (plan §3.2: the SWI vector must "return and record the
//! SWI number"; plan §"S2-b 的兜底必须让未实现号码可见"). The fixture
//! therefore records it: it wraps production's own hook with a recorder
//! that notes every number and then delegates to the production
//! dispatcher unchanged (`swi/mod.rs:53`). The number is then checked
//! against the repository's SWI table (`swi/mod.rs:620-665`), which is
//! where the readable name comes from.
//!
//! The wrapper is installed *around* the production hook, not instead of
//! it: `install_swi_hook` runs first and its hook is what the recorder
//! calls, so the machine under test is exactly the production one.
//!
//! # Why this file brings its own machine
//!
//! The public base's `Fixture` installs production's hook and then hides
//! the core (`fixtures.rs:342-357`), so a test cannot see which SWI
//! numbers the dispatcher was asked about — and that number is the thing
//! the fallback has to make visible. This file therefore builds the
//! machine itself, with the same pairing `Fixture::new` uses, and adds
//! the recorder.
//!
//! **Recommendation for the common base:** give `Fixture` a way to
//! install a recording hook (or to hand out the core), so this file does
//! not have to duplicate the runner.
//!
//! # The numbers, from the GBATEK snapshot
//!
//! `.zcode/gbatek_plain.txt`:
//!
//! * `:192256` — `1Ah  -    -    -    -    SoundDriverInit`
//! * `:192251` — `16h  -    16h  -    16h  Diff8bitUnFilterWrite8bit ;"Wram"`
//! * `:184980-184993` — the GBA reads the **most significant 8 bits of
//!   the 24-bit comment field** of the `SWI` opcode, i.e. bits 16-23 —
//!   the same field the core reads (`arm7tdmi.rs:696-699`).
//!
//! Note the spelling: GBATEK calls `0x16` "Diff8bitUnFilterWrite8bit";
//! the repository's table calls it `Diff8BitUnFilterWram`
//! (`swi/mod.rs:643`). Same number, same BIOS call.
//!
//! # Contract
//!
//! * One fixture per defect. Never mix two defects in one fixture.
//! * The assertion writes the **target** behaviour, never the defect's
//!   current behaviour (writing the defect would lock it in as expected).
//! * Every reproducer carries `#[ignore = "S2-b: 修复后转常驻锁测试"]` with
//!   the owning work item named.
//! * Bit-level formats come from the GBATEK snapshot
//!   (`.zcode/gbatek_plain.txt`), never from memory.
//! * Expected values are assembled from fields, not hand-written literals.

use gba_core::cpu::arm7tdmi::{Arm7tdmi, SwiHook};
use gba_core::cpu::psr::Psr;
use gba_core::gba::Gba;

use super::{mov_imm, swi, Cartridge, BUDGET, HALT};

/// The register the instruction after the SWI writes.
///
/// `r5` because it is not one the boot code touches (`r0`-`r2`,
/// `bios.rs:155-159`) and not one a BIOS call answers in (`r0`).
const MARKER_REGISTER: usize = 5;

/// The value that instruction writes.
///
/// One named constant, used both to build the instruction and to assert
/// the result: the same literal written twice is two literals that can
/// drift apart.
const MARKER: u32 = 0x5A;

// The SWI numbers the core handed to the dispatcher, and the production
// hook the recorder delegates to.
//
// Thread-local, like `swi::note_dispatch` (`swi/mod.rs:593-601`): the
// dispatcher is reached through a plain `fn` pointer that cannot carry
// state, and the harness runs tests in parallel. That helper is private
// to its module, so this file keeps its own.
thread_local! {
    static SEEN_SWIS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    static PRODUCTION_HOOK: std::cell::Cell<Option<SwiHook>> = const { std::cell::Cell::new(None) };
}

/// Record the number, then let production answer exactly as it would have.
fn recording_dispatch(cpu: &mut Arm7tdmi, swi_num: u32, old_cpsr: Psr, return_addr: u32) -> bool {
    SEEN_SWIS.with(|seen| seen.borrow_mut().push(swi_num));
    let production = PRODUCTION_HOOK
        .with(|slot| slot.get())
        .expect("the production SWI hook must be installed before the recorder");
    production(cpu, swi_num, old_cpsr, return_addr)
}

/// The machine under test.
///
/// The public base's `Fixture` cannot be used here for one reason: it
/// installs production's SWI hook and then hides the core, so a test
/// cannot see which SWI numbers the dispatcher was asked about. This file
/// therefore builds the machine itself — the same pairing `Fixture::new`
/// uses (`fixtures.rs:346-357`) — and wraps the production hook with a
/// recorder.
struct Machine {
    gba: Gba,
}

impl Machine {
    /// Build the machine for `body`.
    fn for_body(body: &[u32]) -> Self {
        let mut cartridge = Cartridge::new();
        cartridge.code(body);
        let rom = cartridge.build();

        let mut gba = Gba::new(crate::gba::bios::stub(), &rom);
        // Production's wiring first: the recorder must delegate to the
        // hook production installs, not to a copy of it.
        crate::gba::install_swi_hook(&mut gba);
        let production = gba
            .cpu
            .swi_hook
            .expect("install_swi_hook must install the production SWI hook");
        PRODUCTION_HOOK.with(|slot| slot.set(Some(production)));
        // ... and only then wrap it.
        gba.cpu.swi_hook = Some(recording_dispatch);
        SEEN_SWIS.with(|seen| seen.borrow_mut().clear());

        Self { gba }
    }

    /// Step until the machine halts, or the budget runs out.
    ///
    /// The budget is a **step** count, never a program-counter test: the
    /// ARM7TDMI fetch pointer runs speculatively past a not-yet-taken
    /// branch, so "the PC reached address X" fires on the first pass
    /// through a one-instruction loop (`fixtures.rs:374-379`).
    fn run_to_halt(&mut self, budget: usize) -> bool {
        for _ in 0..budget {
            if self.gba.cpu.halted {
                return true;
            }
            self.gba.step();
        }
        self.gba.cpu.halted
    }

    /// Register `n`.
    fn reg(&self, n: usize) -> u32 {
        self.gba.cpu.registers.register_at(n)
    }

    /// The SWI numbers the core handed to the dispatcher, in order.
    fn seen_swis(&self) -> Vec<u32> {
        SEEN_SWIS.with(|seen| seen.borrow().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{mov_imm, swi, Machine, BUDGET, HALT, MARKER, MARKER_REGISTER};
    use crate::gba::swi::Swi;

    /// `SWI 0x1A` (SoundDriverInit) is not implemented: the machine must come
    /// back from the call and keep running.
    ///
    /// GBATEK `:192256`: `1Ah  -    -    -    -    SoundDriverInit`.
    #[test]
    #[ignore = "S2-b: 修复后转常驻锁测试"]
    fn swi_unimpl_1a() {
        unimplemented_swi_keeps_running(Swi::SoundDriverInit);
    }

    /// `SWI 0x16` (Diff8BitUnFilterWram) is not implemented: the same target
    /// behaviour, and the number the fallback has to report is this one.
    ///
    /// GBATEK `:192251`: `16h  -    16h  -    16h  Diff8bitUnFilterWrite8bit ;"Wram"`.
    #[test]
    #[ignore = "S2-b / S3-a: 修复后转常驻锁测试"]
    fn swi_unimpl_16() {
        unimplemented_swi_keeps_running(Swi::Diff8BitUnFilterWram);
    }

    /// The body both reproducers share.
    ///
    /// The program is
    ///
    /// ```text
    ///     swi  <number>       ; the call under test
    ///     mov  r5, #MARKER    ; the next instruction: it must run
    ///     swi  0x02           ; Halt: the terminator (an implemented call)
    /// ```
    fn unimplemented_swi_keeps_running(called: Swi) {
        let number = called as u32;
        let name = called.name();

        let body = [
            swi(number),
            mov_imm(MARKER_REGISTER as u32, MARKER),
            swi(HALT),
        ];
        let mut machine = Machine::for_body(&body);

        let reached_halt = machine.run_to_halt(BUDGET);

        // The number must be readable: the dispatcher is told which call it
        // is being asked about, and the repository's SWI table knows that
        // number by name.
        assert!(
            machine.seen_swis().contains(&number),
            "SWI {number:#04x} ({name}) never reached the SWI dispatcher"
        );
        assert_eq!(
            Swi::from_raw(number),
            Some(called),
            "SWI {number:#04x} must be a call the SWI table knows as {name}"
        );

        // The machine must come back from the call ...
        assert!(
            reached_halt,
            "SWI {number:#04x} ({name}) is not implemented: the machine must \
             return from the call and keep running, not park on the stub \
             BIOS's SWI vector at 0x08"
        );
        // ... and the instruction after the call must really have run.
        assert_eq!(
            machine.reg(MARKER_REGISTER),
            MARKER,
            "the instruction after SWI {number:#04x} ({name}) never ran"
        );
    }
}
