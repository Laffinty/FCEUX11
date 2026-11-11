//! The SWI fixtures — S1-a, defect domain **unimplemented BIOS SWI**.
//!
//! | fixture | what it drives | belongs to | status |
//! |---|---|---|---|
//! | `swi_unimpl_1a` | ARM code executes `SWI 0x1A` (SoundDriverInit) | S2-b | 常驻绿锁（S2-b 转正） |
//! | `thumb_swi_unimpl_1a` | Thumb 代码执行 `SWI 0x1A` | S2-b | 常驻绿锁（S2-b 新增） |
//! | `swi_diff8bit_unfilter_wram` | ARM code executes `SWI 0x16` (Diff8BitUnFilterWram) | S3-a | 常驻绿锁（S3-a **改写**：由「兜底返回」改成「真实现」） |
//! | `swi_diff8bit_unfilter_vram` | ARM code executes `SWI 0x17` (Diff8BitUnFilterVram) | S3-a | 常驻绿锁（S3-a 新增） |
//! | `swi_diff16bit_unfilter` | ARM code executes `SWI 0x18` (Diff16BitUnFilter) | S3-a | 常驻绿锁（S3-a 新增） |
//!
//! # What S3-a did about it (2026-11-11, v2.0.3 r15)
//!
//! S2-b made the *fallback* work; S3-a gave three of the numbers **real
//! semantics**. `SWI 0x16`, `0x17` and `0x18` are the BIOS's diff-unfilter
//! calls: the source holds a difference stream and the destination must hold
//! the running sum. GBATEK's own example (`.zcode/gbatek_plain.txt:192826-192827`)
//! is the golden vector these fixtures assert against:
//!
//! ```text
//! unfiltered:   10  11  12  13  14  15  16  17  18  19
//! filtered:     10  +1  +1  +1  +1  +1  +1  +1  +1  +1
//! ```
//!
//! The expected values in the fixtures are **GBATEK's published row**, not a
//! recomputation of the same sum the implementation does — a test that
//! recomputes the answer is a mirror of the code, and the two can be wrong
//! together (v2.0.3 plan §九 r13's lesson, twice applied).
//!
//! `swi_unimpl_16` was the fallback lock for `0x16`; now that the call has a
//! real implementation the fallback contract for that number is gone, and the
//! fixture was **rewritten** to assert the implementation instead. The
//! fallback itself is still locked — by `swi_unimpl_1a`, whose number is still
//! unimplemented on purpose.
//!
//! # The defect, and what S2-b did about it (v2.0.3 plan §1.2.1 / §3.2)
//!
//! `dispatch` only claims 16 SWI numbers. A number it does not claim is
//! declined by the core, the CPU takes the real exception path, and the
//! program counter lands on `0x08` — the stub BIOS's SWI vector. The vector
//! used to be `BRANCH_SELF` (`b .`), so the machine **spun forever** on the
//! first unimplemented call.
//!
//! S2-b turned that vector into a `MOVS PC, LR` return trampoline
//! (`bios.rs`): the machine comes back from the call, registers untouched,
//! and `swi::note_unimplemented_swi` records the number. This is a
//! **degradation, not a fix** — the call still does nothing; only S3-a's
//! implementations give it semantics. The fixtures here assert the
//! **fallback contract**: the call returns, the program keeps running, and
//! the number is readable. The S3-a work must keep these green.
//!
//! # The mechanism, step by step
//!
//! 1. The core decodes the `SWI` (`arm7tdmi.rs:657-659`) and calls
//!    `handle_exception(SoftwareInterrupt)`.
//! 2. `handle_exception` reads the number out of the instruction word —
//!    bits 16-23 (`arm7tdmi.rs:696-699`) — and asks the dispatcher first
//!    (`arm7tdmi.rs:703` → `handle_swi_hle` → `arm7tdmi.rs:1138-1142`).
//! 3. `dispatch` declines the number (`swi/mod.rs`, the `None | Some(_)`
//!    arm; `note_unimplemented_swi` records it), so the exception really
//!    happens: the CPU switches to Supervisor mode and jumps to `0x08`
//!    (`arm7tdmi.rs:205-211`), with `LR` already pointing at the
//!    instruction after the `SWI`.
//! 4. `0x08` holds `MOVS PC, LR` — GBATEK: "For Returning from SWI use
//!    MOVS PC,R14" (`.zcode/gbatek_plain.txt:185672-185674`) — which
//!    restores the caller's CPSR (and THUMB state) from `SPSR_svc` and
//!    lands on that instruction. Execution continues.
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
//! dispatcher unchanged (`swi/mod.rs`). The number is then checked
//! against the repository's SWI table (`swi/mod.rs`), which is where the
//! readable name comes from. Production's own recorder
//! (`note_unimplemented_swi`) is exercised by the locks in `bios.rs` and
//! `swi/mod.rs`; the fixtures keep the dispatcher-level record.
//!
//! The wrapper is installed *around* the production hook, not instead of
//! it: `install_swi_hook` runs first and its hook is what the recorder
//! calls, so the machine under test is exactly the production one.
//!
//! # Why this file brings its own machine
//!
//! The public base's `Fixture` installs production's hook and then hides
//! the core (`fixtures.rs`), so a test cannot see which SWI numbers the
//! dispatcher was asked about — and that number is the thing the fallback
//! has to make visible. This file therefore builds the machine itself,
//! with the same pairing `Fixture::new` uses, and adds the recorder.
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
//! * `:189203-189210` — "as called from THUMB mode, it'll also restore
//!   THUMB mode", and `:189222` — Thumb entry saves `R14_svc = PC+2`.
//!
//! Note the spelling: GBATEK calls `0x16` "Diff8bitUnFilterWrite8bit";
//! the repository's table calls it `Diff8BitUnFilterWram`
//! (`swi/mod.rs`). Same number, same BIOS call.
//!
//! # Contract
//!
//! * One fixture per defect. Never mix two defects in one fixture.
//! * The assertion writes the **target** behaviour, never the defect's
//!   current behaviour (writing the defect would lock it in as expected).
//! * Reproducers that are still red carry `#[ignore]` with the owning work
//!   item named; once their work item lands they are promoted to resident
//!   green locks (`oam.rs` on S2-a, this file on S2-b) and the guards'
//!   `REPRODUCERS` set shrinks with them.
//! * Bit-level formats come from the GBATEK snapshot
//!   (`.zcode/gbatek_plain.txt`), never from memory.
//! * Expected values are assembled from fields, not hand-written literals.

use gba_core::cpu::arm7tdmi::{Arm7tdmi, SwiHook};
use gba_core::cpu::psr::{CpuState, Psr};
use gba_core::gba::Gba;

use super::{mov_imm, orr_imm, swi, Cartridge, BUDGET, HALT, SCRATCH};

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

// ---- Thumb encoders (S2-b's Thumb fixture) ---------------------------------
//
// Same discipline as `fixtures.rs`'s ARM encoders: fields, not literals, and
// each layout pinned to the vendored decoder that executes it plus the GBATEK
// snapshot that documents it.

/// Thumb `SWI n` — the number in bits 7-0.
///
/// The vendor decodes `get_bits(8..=15) == 0b1101_1111` as `Instruction::Swi`
/// (`thumb/instruction.rs:174-175`); the core then recovers the number from
/// the **byte at the halfword's address** (`arm7tdmi.rs:691-694`), which on
/// this little-endian bus is exactly the low byte of the encoding.
const fn thumb_swi(n: u32) -> u16 {
    assert!(n < 0x100, "the Thumb SWI immediate is 8 bits");
    0xDF00 | n as u16
}

/// Thumb `MOV rd, #imm8` — format 3, the "move/compare/add/subtract
/// immediate" group: bits 15-13 = `001`, bits 12-11 = `00` (the vendor maps
/// operation `0` to `Mov`, `flags.rs:108-118`), bits 10-8 = rd, bits 7-0 =
/// imm8 (`thumb/instruction.rs:298-303`).
///
/// `rd` is the 3-bit low register file (`r0`-`r7`); `imm` must be an 8-bit
/// value, and anything wider is an error rather than a truncation.
const fn thumb_mov_imm(rd: u32, imm: u16) -> u16 {
    assert!(rd < 8, "the Thumb low register file is r0-r7");
    assert!(imm < 0x100, "format 3 carries an 8-bit immediate");
    (0b001 << 13) | (0b00 << 11) | ((rd as u16) << 8) | imm
}

/// ARM `BX rm` — the same word shape as the boot code's `BX r2` (`bios.rs`
/// `BX_R2 = 0xE12FFF12`): `cond 0001 0010 1111 1111 1111 0001 Rm`, the
/// register in bits 3-0. Note bits 7-4 are `0001`, not zero: with them clear
/// the word no longer matches the BX pattern and the vendor decodes it as an
/// `MSR` flags transfer instead — the first draft of this encoder did
/// exactly that, and the Thumb fixture parked at BIOS 0x08 with the
/// dispatcher never consulted.
///
/// Bit 0 of the register's *value* selects Thumb on the branch; the
/// halfword-aligned target address and the state-selecting bit are set by
/// the caller separately (see [`Cartridge::thumb_base`]).
const fn bx_reg(rm: u32) -> u32 {
    0xE12F_FF10 | (rm & 0x0F)
}

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
    /// Build the machine for an already-assembled cartridge.
    ///
    /// The recorder wraps the production hook, so the machine under test is
    /// exactly the production one; the ARM-entry fixtures go through
    /// [`Machine::for_body`], the Thumb fixture brings its own cartridge.
    fn for_cartridge(cartridge: &Cartridge) -> Self {
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

    /// Build the machine for `body`.
    fn for_body(body: &[u32]) -> Self {
        let mut cartridge = Cartridge::new();
        cartridge.code(body);
        Self::for_cartridge(&cartridge)
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

    /// Whether the CPU is (still) in Thumb state.
    fn in_thumb(&self) -> bool {
        self.gba.cpu.cpsr.cpu_state() == CpuState::Thumb
    }

    /// Put `data` at `address` through the bus, before the program runs.
    ///
    /// The stub BIOS's boot code only programs `WAITCNT` and branches to the
    /// cartridge (`bios.rs:176-184`), so nothing it does disturbs a planted
    /// block in IWRAM.
    fn plant(&mut self, address: u32, data: &[u8]) {
        for (index, byte) in data.iter().enumerate() {
            self.gba
                .cpu
                .bus
                .write_byte((address + index as u32) as usize, *byte);
        }
    }

    /// Read `len` bytes at `address` through the bus.
    fn read_bytes(&mut self, address: u32, len: usize) -> Vec<u8> {
        (0..len)
            .map(|index| self.gba.cpu.bus.read_byte((address + index as u32) as usize))
            .collect()
    }

    /// Read the halfword at `address` through the bus.
    fn read_half_word(&mut self, address: u32) -> u16 {
        self.gba.cpu.bus.read_half_word(address as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bx_reg, mov_imm, orr_imm, swi, thumb_mov_imm, thumb_swi, Cartridge, Machine, BUDGET,
        HALT, MARKER, MARKER_REGISTER, SCRATCH,
    };
    use crate::gba::swi::Swi;
    use crate::gba::swi::unfilter::{DIFF_DATA_SIZE_16BIT, DIFF_DATA_SIZE_8BIT, DIFF_TYPE};

    /// `SWI 0x1A` (SoundDriverInit) is not implemented: the machine must come
    /// back from the call and keep running.
    ///
    /// GBATEK `:192256`: `1Ah  -    -    -    -    SoundDriverInit`.
    ///
    /// Resident since S2-b (v2.0.3, promoted from a red reproducer): the
    /// vector returns, the program continues. S3-a's implementation must
    /// keep this green while adding the call's actual semantics.
    #[test]
    fn swi_unimpl_1a() {
        unimplemented_swi_keeps_running(Swi::SoundDriverInit);
    }

    /// `SWI 0x16` (Diff8BitUnFilterWram) now has a real implementation: the
    /// destination must hold GBATEK's unfiltered row.
    ///
    /// GBATEK `:192251`: `16h  -    16h  -    16h  Diff8bitUnFilterWrite8bit ;"Wram"`.
    ///
    /// This fixture **was** `swi_unimpl_16` -- the fallback lock for the same
    /// number, promoted by S2-b. S3-a gave the number real semantics, so the
    /// fallback contract for it is gone and the fixture was rewritten to
    /// assert the implementation instead. The fallback itself is still
    /// locked, by `swi_unimpl_1a`, whose number is still unimplemented on
    /// purpose.
    #[test]
    fn swi_diff8bit_unfilter_wram() {
        let program = call_program(Swi::Diff8BitUnFilterWram as u32, SOURCE, DEST_8BIT);
        let mut machine = Machine::for_body(&program);
        machine.plant(SOURCE, &diff_block(DIFF_DATA_SIZE_8BIT, &FILTERED));

        assert_the_call_was_served(
            &mut machine,
            Swi::Diff8BitUnFilterWram as u32,
            "Diff8BitUnFilterWram",
        );

        assert_eq!(
            machine.read_bytes(DEST_8BIT, UNFILTERED.len()),
            UNFILTERED.to_vec(),
            "the destination must hold GBATEK's unfiltered row, byte for byte"
        );
    }

    /// `SWI 0x17` (Diff8BitUnFilterVram) decodes the same 8-bit stream, but
    /// the answer goes out as **halfwords**.
    ///
    /// GBATEK `:192252`: `17h  -    -    -    -  Diff8bitUnFilterWrite16bit ;"Vram"`.
    ///
    /// The bytes at the destination are the same row as `0x16`'s -- that is
    /// the point of the "Vram" variant: the *data* is identical and only the
    /// store width differs. The halfword assertion pins the pairing order
    /// (the first decoded byte in the low half), which the byte row alone
    /// cannot: a swapped pair would still put the same ten bytes in memory.
    ///
    /// **The VRAM destination is what gives this fixture teeth about the
    /// store width**: the vendored bus duplicates a byte written to BG VRAM
    /// across its halfword and drops one written to the OBJ region
    /// (`bus.rs:1235-1262`), so a byte-store implementation would leave a
    /// *different* picture there. In work RAM the two are
    /// indistinguishable, and the width would go unpinned.
    #[test]
    fn swi_diff8bit_unfilter_vram() {
        let program = call_program(Swi::Diff8BitUnFilterVram as u32, SOURCE, DEST_VRAM);
        let mut machine = Machine::for_body(&program);
        machine.plant(SOURCE, &diff_block(DIFF_DATA_SIZE_8BIT, &FILTERED));

        assert_the_call_was_served(
            &mut machine,
            Swi::Diff8BitUnFilterVram as u32,
            "Diff8BitUnFilterVram",
        );

        assert_eq!(
            machine.read_bytes(DEST_VRAM, UNFILTERED.len()),
            UNFILTERED.to_vec(),
            "the destination must hold GBATEK's unfiltered row, byte for byte"
        );
        assert_eq!(
            machine.read_half_word(DEST_VRAM),
            u16::from(UNFILTERED[0]) | (u16::from(UNFILTERED[1]) << 8),
            "the first two decoded bytes go out as one halfword, the first one \
             in the low half"
        );
    }

    /// `SWI 0x18` (Diff16BitUnFilter) reads **halfword** units.
    ///
    /// GBATEK `:192253`: `18h  -    18h  -    18h  Diff16bitUnFilter`, and
    /// `:192840-192841`: "Data units may be either 8bit or 16bit used with
    /// Diff8bit or Diff16bit functions respectively."
    ///
    /// The numbers are GBATEK's own example again, in halfwords this time:
    /// the same ten values, so the running sum is the same sequence.
    #[test]
    fn swi_diff16bit_unfilter() {
        const FILTERED_16: [u16; 10] = [10, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        const UNFILTERED_16: [u16; 10] = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19];

        let program = call_program(Swi::Diff16BitUnFilter as u32, SOURCE, DEST_16BIT);
        let mut machine = Machine::for_body(&program);
        machine.plant(SOURCE, &diff_block_16(DIFF_DATA_SIZE_16BIT, &FILTERED_16));

        assert_the_call_was_served(
            &mut machine,
            Swi::Diff16BitUnFilter as u32,
            "Diff16BitUnFilter",
        );

        for (index, expected) in UNFILTERED_16.iter().enumerate() {
            assert_eq!(
                machine.read_half_word(DEST_16BIT + index as u32 * 2),
                *expected,
                "unit {index} must be GBATEK's unfiltered value"
            );
        }
    }

    /// The same call, issued from **Thumb** state.
    ///
    /// The exception path has two halves that the ARM fixtures cannot see:
    /// entry saves `R14_svc = PC+2` in Thumb (GBATEK `:189222`, vs `PC+4`
    /// in ARM, `:185686`), and the `MOVS PC, LR` trampoline must restore
    /// THUMB mode on the way out (GBATEK `:189203-189210`: "as called from
    /// THUMB mode, it'll also restore THUMB mode"). If the return landed
    /// anywhere else, or came back in ARM state, the halfword after the
    /// call would decode as the wrong instruction — so a *different*
    /// post-call marker (not the pre-call one) is what proves the round
    /// trip: it can only be set by really executing Thumb code after the
    /// unimplemented call returned.
    ///
    /// The program:
    ///
    /// ```text
    ///     mov r5, #0x5A       ; Thumb, pre-call marker
    ///     swi 0x1A            ; the call under test, from Thumb
    ///     mov r5, #0xA5       ; Thumb, post-call marker: proves Thumb resumed
    ///     swi 0x02            ; Halt terminator (an implemented call)
    /// ```
    ///
    /// entered through an ARM `BX` with bit 0 set (`fixtures.rs`
    /// `Cartridge::thumb_code`).
    #[test]
    fn thumb_swi_unimpl_1a_returns_and_continues_in_thumb() {
        const PRE: u16 = 0x5A;
        const POST: u16 = 0xA5;

        let mut cartridge = Cartridge::new();
        // ARM entry: build the Thumb entry address with bit 0 set, then
        // branch-exchange into it. `0x08000000` is `0x08` rotated right 8
        // (`fixtures.rs::mov_imm`), `|1` turns it into the state-selecting
        // value, and the ORR carries the low byte.
        let thumb_base = cartridge.thumb_base() + 12; // after the three ARM words below
        cartridge.code(&[
            mov_imm(0, 0x0800_0000),
            super::orr_imm(0, 0, ((thumb_base | 1) & 0xFF) as u32),
            bx_reg(0),
        ]);
        assert_eq!(
            cartridge.thumb_base(),
            thumb_base,
            "the Thumb body must sit exactly where the BX points"
        );
        cartridge.thumb_code(&[
            thumb_mov_imm(5, PRE),
            thumb_swi(Swi::SoundDriverInit as u32),
            thumb_mov_imm(5, POST),
            thumb_swi(HALT),
        ]);

        let mut machine = Machine::for_cartridge(&cartridge);
        let reached_halt = machine.run_to_halt(BUDGET);

        assert!(
            machine.seen_swis().contains(&(Swi::SoundDriverInit as u32)),
            "SWI 0x1A issued from Thumb never reached the SWI dispatcher"
        );
        assert!(
            reached_halt,
            "the Thumb program never reached its Halt: the SWI either parked \
             the machine at BIOS 0x08 or the return went somewhere else"
        );
        assert_eq!(
            machine.reg(5),
            u32::from(POST),
            "the Thumb instruction after the unimplemented SWI never ran: the \
             exception return did not land back in Thumb state (or did not \
             land at all)"
        );
        assert!(
            machine.in_thumb(),
            "the machine must still be in Thumb state after the round trip \
             through the ARM exception vector"
        );
    }

    // ---- the diff-unfilter fixtures' toolkit (S3-a) ------------------------
    //
    // One golden vector, three numbers. The vector is GBATEK's own
    // (`.zcode/gbatek_plain.txt:192826-192827`), and the expected values are
    // **published** rather than recomputed -- a test that recomputes the
    // answer with the implementation's own algorithm is a mirror of the code,
    // and the two can be wrong together.

    /// GBATEK's filtered stream: the first value, then the differences.
    const FILTERED: [u8; 10] = [10, 1, 1, 1, 1, 1, 1, 1, 1, 1];

    /// GBATEK's unfiltered row: what the destination must hold.
    const UNFILTERED: [u8; 10] = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19];

    /// Where the block is planted.
    const SOURCE: u32 = SCRATCH as u32;

    /// Where the 8-bit call's answer goes.
    ///
    /// IWRAM: the "Wram" variant is meant for work RAM, where a byte store
    /// and a halfword store put the same bytes in memory.
    const DEST_8BIT: u32 = SOURCE + 0x80;

    /// Where the "Vram" call's answer goes.
    ///
    /// **VRAM, on purpose.** The vendored bus treats a byte write to VRAM
    /// differently from a halfword write (`bus.rs:1235-1262`: a byte written
    /// to BG VRAM is duplicated across the halfword, and one written to the
    /// OBJ region is dropped), so a VRAM destination is what makes the
    /// fixture **able to see the store width** -- in work RAM the two are
    /// indistinguishable. It is also where the "Vram" variant belongs.
    const DEST_VRAM: u32 = 0x0600_0000;

    /// Where the 16-bit call's answer goes.
    ///
    /// VRAM as well, for the same reason: the call stores halfwords, and a
    /// byte-store implementation would leave a different picture there.
    const DEST_16BIT: u32 = 0x0600_0040;

    /// The diff block's header word, assembled from its fields
    /// (GBATEK `:192848-192853`: bits 0-3 data size, bits 4-7 type, bits 8-31
    /// size in bytes).
    fn diff_header(data_size: u8, size: u32) -> u32 {
        assert!(size <= 0x00FF_FFFF, "the size field is 24 bits wide");
        (size << 8) | (u32::from(DIFF_TYPE) << 4) | u32::from(data_size)
    }

    /// A whole diff block: the header followed by the data units.
    fn diff_block(data_size: u8, data: &[u8]) -> Vec<u8> {
        let mut block = Vec::with_capacity(4 + data.len());
        block.extend_from_slice(&diff_header(data_size, data.len() as u32).to_le_bytes());
        block.extend_from_slice(data);
        block
    }

    /// A whole diff block whose units are halfwords.
    fn diff_block_16(data_size: u8, values: &[u16]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(values.len() * 2);
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        diff_block(data_size, &bytes)
    }

    /// The program that calls `number` with `src` and `dest`.
    ///
    /// The addresses are built with `MOV` + `ORR`, because the encoders only
    /// cover **rotated** immediates (`fixtures.rs`: "旋转立即数旋不进就报错")
    /// and the destination is not one of them.
    fn call_program(number: u32, src: u32, dest: u32) -> Vec<u32> {
        vec![
            mov_imm(0, src & !0xFF),
            orr_imm(0, 0, src & 0xFF),
            mov_imm(1, dest & !0xFF),
            orr_imm(1, 1, dest & 0xFF),
            swi(number),
            mov_imm(MARKER_REGISTER as u32, MARKER),
            swi(HALT),
        ]
    }

    /// The assertions every diff fixture shares.
    ///
    /// * the call returned -- the machine reached its `Halt`;
    /// * the instruction after the call ran;
    /// * the call was **served**: the number is not on the "still
    ///   unimplemented" list, and the dispatcher's tally says so.
    ///
    /// The third one is what tells the S2-b fallback and the S3-a
    /// implementation apart: both of them let the machine carry on, so only
    /// the tally and the recorder can say which happened.
    fn assert_the_call_was_served(machine: &mut Machine, number: u32, name: &str) {
        assert!(
            machine.run_to_halt(BUDGET),
            "the program never reached its Halt: SWI {number:#04x} ({name}) did \
             not return"
        );
        assert_eq!(
            machine.reg(MARKER_REGISTER),
            MARKER,
            "the instruction after SWI {number:#04x} ({name}) never ran"
        );

        let tallies = crate::gba::swi::take_swi_tallies();
        assert!(
            tallies
                .iter()
                .any(|(num, calls, served)| *num == number && *calls == 1 && *served == 1),
            "SWI {number:#04x} ({name}) must be served by this crate's dispatcher, \
             not declined: {tallies:?}"
        );

        let declined = crate::gba::swi::unimplemented_swi_counts();
        assert!(
            !declined.iter().any(|(num, _)| *num == number),
            "SWI {number:#04x} ({name}) was recorded as unimplemented: {declined:?}"
        );
    }

    /// The body the fallback reproducers share.
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
