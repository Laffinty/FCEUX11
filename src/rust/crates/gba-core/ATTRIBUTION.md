# ATTRIBUTION — GBAEUX11

> This file is the single authoritative record of the GBA hardware core's
> provenance and of every local modification applied to it.
> Referenced by the v2.0 build plan (section 11 "状态回写" item 2, and
> section 3 "许可与合规"). Directory / crate / module names are functional
> (`f11gba`, `gba-core`) and deliberately carry no upstream identifiers;
> this file, plus the copied `LICENSE`, is where the attribution lives.

## 1. Upstream

| Field | Value |
|---|---|
| Project | **clementine** |
| Repository | `https://github.com/RIP-Comm/clementine` |
| Crate taken | `emu/` — the hardware core only |
| Baseline commit | `ee77922dd293b70e945458e104f3b2de794f0151` (`main`) |
| Upstream copyright | **Copyright (c) 2024 RIPsters** |
| Licence | **MIT** |
| Vendored as | `src/rust/crates/gba-core/` (crate name `gba-core`) |
| Licence text | `src/rust/crates/gba-core/LICENSE` (verbatim copy of the upstream root `LICENSE`) |

### What was taken, and what was discarded

| Upstream path | Disposition |
|---|---|
| `emu/src/**` (45 files) | **taken** → `crates/gba-core/src/**` |
| `emu/Cargo.toml` | taken, then rewritten (see §3) |
| `emu/tests/jsmolka.rs` (112 lines) | **not vendored** — it is the upstream test harness, not core code. FCEUX11 runs its own jsmolka integration (plan section 7.5). |
| `ui/**` (egui debugger) | **discarded** — plan section 2.0 |
| `src/main.rs` (CLI) | **discarded** — plan section 2.0 |
| `LICENSE` | **taken** → `crates/gba-core/LICENSE` |

## 2. Per-file headers

**Do not add or strip upstream file headers.** The measured situation is:

- of the 45 vendored sources, **44 carry no copyright or licence line at all**;
- the single exception is `src/cpu/thumb/alu_instructions.rs`.

The MIT attribution duty is therefore discharged by **`crates/gba-core/LICENSE` +
this file + shipping the licence text with the release artefact**, not by
per-file headers. Inventing headers on the other 44 files would misattribute
material that upstream never marked.

> This corrects a wording in the v2.0 plan section 3, which said the duty is
> discharged by "保留上游文件头与版权行". The *intent* — do not remove what
> upstream had — holds; the *mechanism* is this file plus the LICENSE copy.

## 3. Local modifications to the vendored tree

### 3.1 `Cargo.toml` — rewritten

The upstream `emu/Cargo.toml` took these from **its own** workspace root via
inheritance:

| Field | Upstream | Here | Why |
|---|---|---|---|
| `edition` | `edition.workspace` | `edition = "2024"` | Same value; FCEUX11's `[workspace.package] edition` is also 2024, so behaviour is identical, but spelled out to keep the vendored crate self-describing. |
| `repository` | `repository.workspace` | `repository = "https://github.com/RIP-Comm/clementine"` | FCEUX11's workspace does not define this field at all — inheritance would fail to resolve. |
| `license` | `license.workspace` | `license = "MIT"` | Same reason. |
| `serde` | `serde.workspace` | `serde = { version = "1", features = ["derive"] }` | FCEUX11 has no `[workspace.dependencies]` table; the `derive` feature was what upstream's workspace supplied, so it is spelled out. |
| `tracing` | `tracing.workspace` | `tracing = "0.1"` | Same reason. |
| `serde_with` | `serde_with = "3"` | unchanged | already explicit upstream |
| `rtrb` | `rtrb = "0.3"` | unchanged | already explicit upstream |
| `rand` | `rand = { workspace = true, optional = true }` | `rand = { version = "0.8", optional = true }` | Same reason; kept optional. |

`[lints.clippy]` was carried over unchanged.

### 3.2 Source files — 22 local changes across 4 files (r17 #24 reverted, see §3.2.7)

`crates/gba-core/src/**` was byte-identical to upstream `emu/src/**` at the end
of **S0** (verified file-by-file with SHA-256). Local edits begin at **S0'**, when
the core was given the hook it needs before any SWI can be supplied from
outside — `handle_swi_hle` is a private method, so there is no other way.

Entries 1-9 are all in `src/cpu/arm7tdmi.rs`:

**S0' — the SWI seam (5)**

| # | Change | Why |
|---|--------|-----|
| 1 | Added `pub type SwiHook = fn(&mut Arm7tdmi, u32, Psr, u32) -> bool;` | Lets the embedder service SWI 0x00-0x2A. A plain `fn` pointer, not a trait object, keeps the dependency one-directional: the embedder calls this crate, never the reverse. |
| 2 | Added `#[serde(skip)] pub swi_hook: Option<SwiHook>` on `Arm7tdmi` | The field itself. `serde(skip)` because a function pointer has no meaning to restore, and loading a savestate must not depend on it. |
| 3 | `impl Default for Arm7tdmi` initialises it to `None` | Required by the manual `Default` impl. |
| 4 | `handle_swi_hle` consults the hook before its own `match` | The seam. The hook gets first refusal, so it can both fill gaps and override the arms that exist but are shells. |
| 5 | `swi_return` made `pub` | Without it a hook could service a call but not return from it. |

**S1a-1 — the halt mechanism (4)**

The wait family needs the core to *stop executing instructions*, and that
cannot be done from outside this crate: `Bus::step` is `pub(crate)`
(`bus.rs:1214`), so an embedder has no way to advance the LCD and timers
without also stepping the CPU. This is the documented breach of the plan's
risk R1 ("keep the patch set minimal — SWI hook only"), registered as **R15**.

| # | Change | Why |
|---|--------|-----|
| 6 | Added `pub halted: bool` on `Arm7tdmi` | The state the four wait-class SWIs need. Serialized, not skipped: a halted CPU is real machine state, and a savestate that lost it would resume a game that was supposed to be asleep. |
| 7 | Added `pub type WakeHook = fn(&mut Arm7tdmi) -> bool;` and `#[serde(skip)] pub wake_hook: Option<WakeHook>` | `Halt` wakes on any enabled interrupt, but `IntrWait` must stay asleep until the *specific* BIOS flag it asked for appears, and `IntrWait(1, 1)` must not return early even when that flag is already set. Those are BIOS-level semantics the core cannot know, so it asks once per halted cycle. |
| 8 | `impl Default for Arm7tdmi` initialises `halted: false` and `wake_hook: None` | Same reason as #3. |
| 9 | `step()` gained a halt guard, and `Arm7tdmi::new` clears the CPSR I bit | The guard runs no instruction but still calls `bus.step()`, so peripherals keep their clocks — which is the only reason an interrupt can arrive. It sits **once**, before the ARM/Thumb `match`, so one insertion covers both execution paths, and the pipeline is left untouched so the CPU resumes on the instruction it would have run next. The I-bit clear is the other half of the BIOS reset state this crate already pre-initialises (see `initialize_stack_pointers`): ARM resets with CPSR bit 7 set, nothing else in the crate ever cleared it, and a permanently masked CPU can never take an interrupt. |

Upstream line references (for a future re-vendor check): struct at
`cpu/arm7tdmi.rs:144`, `Default` at `:217`, hook dispatch site
`handle_swi_hle` at `:1044`, `swi_return` at `:1327`, `step()` at `:705`,
`Bus::step` at `bus.rs:1214`, `Arm7tdmi::new` at `:830`.

> Entries 6-9 do change upstream behaviour — the `0x02` and `0x03..=0x05` arms
> and the `_ => false` fallback are untouched, but `step()` now has a branch it
> did not have. That is deliberate and is what the embedder's wait family needs;
> it is the reason R15 exists.

#### 3.2.1 S2-b3 — the real-time clock, 5 more changes in 2 more files

**This is the first time the patch set stopped being one file, and that is the
part worth recording.** Until now every local change was in
`src/cpu/arm7tdmi.rs`; the S2-b3 work put two edits in `src/cpu/hardware/rtc.rs`
and two in `src/cpu/hardware/internal_memory.rs`. Anyone re-vendoring has to
check three files, not one.

The S3511 implementation itself was left alone. It is complete, and to a game it
is already transparent: `InternalMemory` marks a cartridge as having the chip
the first time the game writes a GPIO register, feeds the pin state to
`Rtc::write`, and hands `Rtc::sio()` back on a read. **What was missing was not
the feature but the ability to verify it** — `current_unix_secs()` reads
`SystemTime::now()` from a private free function with no way in, which leaves the
core's own RTC test comparing the chip against the host clock read at the same
moment. A self-comparison cannot fail for a wrong date conversion, and that test
reads one byte of the seven, so the weekday, hour, minute and second have no
coverage at all.

| # | Change | Why |
|---|---|---|
| 10 | `rtc.rs`: added `time_override: Option<i64>` to `Rtc`, **serialized** | A fixed instant to report instead of the host clock. `None` is upstream's behaviour untouched. Serialized on purpose: a state taken with the clock pinned has to come back pinned, or the override silently reverts itself on the next load. |
| 11 | `rtc.rs`: `set_time_override` / `time_override` accessors, and a private `now_unix_secs()` | The injection point and the read-back. Three small members rather than a clock trait, so the embedder's surface stays a setter and a getter. |
| 12 | `rtc.rs`: the two `datetime_bytes(current_unix_secs())` call sites now read `self.now_unix_secs()` | The whole point of #10 and #11. The date/time read (command 2) and the time-only read (command 3) are the only two places the chip asks what time it is. |
| 13 | `internal_memory.rs`: `pub const fn rtc(&self) -> &Rtc` | The `rtc` field is private and the root crate has to reach it. An accessor rather than a public field because nothing outside the module drives GPIO — the *protocol* goes through `write`/`sio`, and this is only for the *time*. |
| 14 | `internal_memory.rs`: `pub fn rtc_mut(&mut self) -> &mut Rtc` | Same reason, for pinning. `Bus::internal_memory` and `Arm7tdmi.bus` were already `pub`, so no change to `bus.rs` was needed. |

Upstream line references (for a future re-vendor check): `Rtc` struct at
`cpu/hardware/rtc.rs:39`, `current_unix_secs` at `:214`, the two read commands in
`decode_command` at `:146-155`, `InternalMemory::rtc` field at
`cpu/hardware/internal_memory.rs:182`.

> **These 5 do not change upstream behaviour** — with no override set, the chip
> reports the host clock exactly as before. The difference is that the behaviour
> is now *reachable*, which is what lets a test assert a date instead of
> asserting that two reads of the same clock agree.

#### 3.2.2 v2.0.1 (plan r56) — the wake-IRQ return address, 1 more change in `arm7tdmi.rs`

| # | Change | Why |
|---|--------|-----|
| 17 | The halt-wake path in `step()` presents the PC in pipeline convention (`PC += 4` Thumb / `+= 8` Arm) before taking a pending IRQ | `enter_intr_wait` parks the CPU with PC at the SWI's **return address** — the next instruction to execute — which is not the convention `lr_offset` subtracts the pipeline width from. The wake-IRQ therefore computed `LR = PC`, and the BIOS return `SUBS PC, LR, #4` re-entered the SWI itself, swallowing the continuation after it. Hardware computes `LR = return address + 4`; the presentation reproduces that in either state. A wait-then-work program — Mario Kart Super Circuit registers its 16 boot tasks from the continuation of the very first `VBlankIntrWait` — lost the work entirely (plan §9.1 L16, the white screen of r53 ⑧). |

Upstream line references: the insertion sits inside the local halt guard in
`step()` (upstream `step()` at `cpu/arm7tdmi.rs:705`, local wake site at
`:760-782` after the S1a-1 guard); `lr_offset` itself is untouched.

> **This entry does change upstream behaviour** — a second, deliberate breach
> of the same kind as S1a-1 (R15): without it, every wake from a
> wait-class SWI re-executed the SWI, which no software can observe as
> correct. Locked by
> `an_irq_return_from_a_swi_halt_lands_after_the_swi` (mutation-checked:
> reverting the presentation turns it red).

#### 3.2.3 v2.0.1 (plan P0) — the ARM SWI number, 1 more change in `arm7tdmi.rs`

Upstream reads the SWI number of an ARM-mode `SWI` out of the **low byte** of
the instruction (`swi_instr & 0xFF`, in the `HLE` block of `handle_exception`).
The ARM7TDMI puts that number in **bits 16-23**; the low 8 bits are a comment
field. Three independent sources agree on bits 16-23: the GBA BIOS itself, the
ARM-mode convention every assembler emits (`swi 0x60000`), and mGBA 0.10.5,
which decodes `0xEF060000` as SWI `06` where this crate decoded `0x00`.

| # | Change | Why |
|---|--------|-----|
| 18 | The ARM branch of the SWI-number extraction is now `(swi_instr >> 16) & 0xFF`; the Thumb branch is untouched | Number `0x06` (`Div`) read as `0x00` (`SoftReset`), so **every** ARM-mode BIOS call became a soft reset: registers zeroed, PC back to `08000000h`, cartridge re-entered. A ROM whose diagnostic path calls a BIOS function from ARM mode restarts itself forever and hangs on a blank screen. The THUMB branch was already correct, which is exactly why the THUMB-mode suites passed while the ARM-mode ones hung — the split was the tell. |

This one is worth registering in more detail than its size suggests, because
**upstream is wrong here rather than merely incomplete**, so a re-vendor that
takes a newer revision may well "restore" the low-byte read and silently undo
it. Check it first.

The same defect was invisible to this project's own suite for a reason worth
recording next to it: `src/gba/gate.rs` and `src/gba/swi/mod.rs` built their
ARM `SWI` words with the number in the **low byte** as well, so the tests were
written to match the bug and the whole set stayed green through it. Both
encoders were corrected in the same change (`0xEF00_0000 | (n & 0xFF) << 16`),
which is first-party code and so is not part of this count.

Locked by two gate tests, `the_number_lives_at_bits_16_23` and
`the_low_byte_is_not_also_a_number`. Both are mutation-checked, and the second
one exists for a reason beyond coverage: it pins the decision **not** to accept
the low-byte encoding as a fallback. A fallback is the tempting repair for a
library that got this wrong, and adopting it would both invent a hardware
behaviour and re-hide the defect behind the very programs that were wrong.

> **This entry changes upstream behaviour, in the direction of the real
> machine.** Note that the first draft of the negative lock test above was
> **invalid** and was caught by mutation: it used an ARM `SWI` terminator to
> decide whether the program stopped, so a core reading the wrong byte read the
> *terminator* as `SoftReset`, and the test passed under the mutation it was
> meant to catch. The terminator was replaced by watching `r0` — a `Div` writes
> the quotient there, a `SoftReset` zeroes it — which needs no exit condition and
> is unaffected by the convention under test.

#### 3.2.4 v2.0.1 (plan P2 groundwork) — the `AGS` wait-state expectations, 1 more change in `bus.rs`

Tests only. No production line in this crate is touched by this entry, which is
also why it is the first local change outside `cpu/` — and the first one that
could be made at all, see §3.5.

| # | Change | Why |
|---|--------|-----|
| 19 | `bus.rs`: two new tests in the existing `mod tests`, plus three test-only constants and two test-only helpers (`ags_waitcnt`, `four_loads`). No production line is touched. | Pins the `GamePak` wait-state arithmetic to the `AGS` Aging Cartridge v7.0's own expected values, decoded from the Normmatt/`ags_aging` disassembly (`src/sub_8002CAC.c` `sub_80030E8`, and `src/sub_80031B8.c` `sub_80031B8` against the measurement routines in `src/sub_800326C.arm.s`). The 28 authoritative values — 24 ROM wait-state cases plus 4 cartridge-RAM cases — are reproduced exactly by the crate's **existing** `access_cycles` / `gamepak_cycles` / `gamepak_waits`, with a single fixed overhead of 8 cycles for the cartridge's measurement window. That the same 8 falls out of all 28 cases is what makes the table a check on the formula rather than a fit to it. |

The disassembly targets `aging.gba` with sha1 `c67e0a5e…`; our AGS v7.0 (Rev 1)
is `5c73fb40…`. The two binaries are **not** the same build, so the table was not
assumed to apply: both measurement functions' literal pools — `0x04000204`,
`0x04000100`, and the `0xf8ff` / `0xfcff` write masks, padding byte included —
are byte-identical in our ROM, and those literals are what decide what gets
measured.

Mutation-checked both ways: corrupting any one of the 28 expected values turns
`the_ags_expectations_are_one_overhead_and_a_wait_state_formula` red, and so does
moving the overhead constant off 8.

#### 3.2.6 v2.0.1 (plan r12) — the S-cycle was billed twice, 1 more change in `bus.rs`

**This one changes what every instruction costs, in every game.** It is the
fourth entry that alters upstream behaviour (after §3.2.2, §3.2.3 and the halt
mechanism), and the first whose blast radius is the whole instruction stream
rather than a corner of it.

| # | Change | Why |
|---|--------|-----|
| 23 | `bus.rs`: `access_cycles` now subtracts one cycle when `in_opcode_fetch` is set, delegating to a new `access_cycles_full` for the unmodified per-region cost. | On this part an instruction fetch **is** the instruction's S-cycle — the two overlap and there is only ever one cycle for them. `bus::step` already charged that S-cycle, and the fetch charged it again, so **every instruction was billed twice for the same cycle**: a straight-line register operation cost 2 instead of 1, and a taken branch cost 6 instead of the hardware's `2S + 1N = 3`. The `AGS` Aging Cartridge `TIMER PRESCALER` test times 1024 iterations of `SUBS` + `BNE` and expects 4096; this core measured **8192**. |

**Why the subtraction is `1` and not "the sequential wait states."** mGBA bills an
opcode fetch at the *sequential* wait (`ARM_PREFETCH_CYCLES = 1 + activeSeqCycles32`,
`isa-arm.h:13`), which assumes the prefetcher kept up. That is the optimistic
approximation the v2.0.1 plan criticises in §四, and modelling it is the P3
phase's job. Subtracting exactly the one duplicated S-cycle removes the
double-billing **without importing the prefetch assumption**, and the S/N
decision below is untouched.

**A correction this entry forced.** `prefetch_makes_sequential_opcode_fetches_cheap`
asserted `access_cycles(...) == (1 + 2) + (1 + 2)` for a sequential opcode fetch —
it had **locked the double-billing in as the specification**. That is the v2.0.1
P0 failure mode in a new place: a gate that cannot fail because it was written
from the implementation's own belief. It now asserts the same expressions minus
one, with the reason inline.

**It also collapsed a two-part diagnosis into one.** The plan recorded the 2x as
"fetch double-billing **plus** refill at full price", with separate acceptance for
each. In fact the refill's overcharge *was* the fetch double-billing: the two
post-branch steps that execute nothing were each paying `1` for the step and `1`
for a fetch of the branch target the branch had already paid for. With the single
change above a taken branch costs `1` (its own S) + `2` (the two refill steps) =
3, with no separate refill pricing to fix. The mutation check therefore has one
site rather than two, and it turns all four tests red at once.

#### 3.2.5 v2.0.1 (plan r5) — `LDRH` / `LDRSH` destination width, 2 more changes in `arm7tdmi.rs`

Tests only, in the existing `mod tests`.

| # | Change | Why |
|---|--------|-----|
| 21 | `arm7tdmi.rs`: `arm_ldrh_zero_extends_into_the_whole_destination_register` | Written to **try to refute** a candidate, not to confirm it. The differential probe in `src/rust/src/gba/waitprobe.rs` read back a value too large to have come from a 16-bit load, and the obvious suspect was `LDRH` leaving the upper half of its destination register stale. The test preloads the destination with `0xFFFF_FFFF` and stores `0x1234`, so a load that only wrote the low half would leave `0xFFFF_1234` and fail. It passes: **`LDRH` is correct and the candidate is dead.** |
| 22 | `arm7tdmi.rs`: `arm_ldrsh_sign_extends_from_bit_15` | The pairing test. Zero-extension and sign-extension are the two halves of "what does a halfword load leave in the register", and pinning only one of them would let the other rot. Stores `0x9234` with bit 15 set and requires `0xFFFF_9234`. |

> A numbering slip is worth recording rather than quietly fixing: entry 20 was
> briefly used twice — once for the `LDRH` test here and once for the
> `Cargo.toml` dev-dependency in §3.5. The dev-dependency is chronologically
> earlier, so it keeps 20 and these became 21 and 22. The set is only worth
> counting if the count is right, and a duplicate is exactly the kind of thing
> that makes a count stop being checkable.

Both are worth having beyond this round: they are the cheapest available check
on a whole family of halfword transfers, and the first of them is a worked
example of the discipline in AGENTS.md — **a test that can kill your own
hypothesis is worth more than one that can only confirm it.**

> The second test failed on its first run, and the fault was in the test: the
> `LDRSH` opcode was hand-built with the transfer-kind field written into the
> `Rd` slot (`0xE1D190B0`, so it targeted r9). The correct encoding is
> `0xE1D100F0` — the same word as `LDRH` with bits 6-5 set to `10b`. Worth
> recording because the failure looked exactly like a core bug in sign extension,
> and a red test is not evidence until you know *whose* mistake it is.

#### 3.2.7 v2.0.1 (plan r17) — the timer prescaler read the wrong register, 1 more change in `timers.rs`

The first local change outside `cpu/arm7tdmi.rs` to alter what hardware does.

| # | Change | Why |
|---|--------|-----|
| 24 | `timers.rs`: the divider is taken from the **low** register's latched reload value (`get_prescaler(self.tm0_reload)`, and a `reload` parameter threaded into `step_timer` for timers 1-3) instead of from the high control register. `get_prescaler`'s own doc comment now says which register it wants. | The prescaler occupies **bits 0-1 of `TMxCNT_L`** — the same bits as the counter and reload value. The high register `TMxCNT_H` holds cascade (bit 2), IRQ (bit 6) and enable (bit 7) and no divider at all. Reading it from there yields `0x0080 & 3 == 0`, i.e. **divide by one, forever**: measured `{4147, 4148, 4149, 4150}` for divide-by-{1, 64, 256, 1024}, where the only thing that varied was the counter's starting value. mGBA agrees (`timer.c:124`, `prescaleTable[4] = { 0, 6, 8, 10 }`, indexed by the low register's `& 0x0003`). |

**REVERTED (plan r19, 2026-10-06).** The hardware model above is wrong. The prescaler selection lives in **`TMxCNT_H` bits 0-1** (GBATEK; mGBA `timer.c:124` — its `control` operand *is* the value written to `TMxCNT_H`, not "the low register"). The AGS aging cartridge's own PRESCALER routine writes `(j<<16)|0x800000`, i.e. `TMxCNT_H = 0x0080|j` — enable (bit 7) + prescaler (bits 0-1), standard layout — and its expectation table `{4096, 64, 16, 4}` only fits that layout. Change #24 is reverted (`get_prescaler(self.tm0cnt_h)`, the upstream form; the two unit-test fixtures restored to control-word encoding). Timer patch count returns to 22 across 4 files. This entry is retained as a record of the misdiagnosis chain (r15 probe never set the prescaler field → r16 misread mGBA → r17 wrong-direction fix → r18 AGS decode caught it → r19 adjudication). See plan §十一 r19.

#### 3.2.8 v2.0.1 (plan r22) — two more changes in `timers.rs`: the cascade bit on timer 0, and a 2-cycle start delay

| # | Change | Why |
|---|--------|-----|
| 25 | `timers.rs`: timer 0 is now gated on `is_enabled(tm0cnt_h)` alone. Upstream also required `!is_cascade(tm0cnt_h)`, which **stopped timer 0 dead** whenever the cascade bit was set. | The cascade bit has no effect on timer 0 — timer 0 always counts on its own prescaler. Upstream's own comment said so ("Timer 0 never cascades; the cascade bit is ignored on hardware") while the code did the opposite. This is a defect, not a modelling choice: the AGS `TIMER CONNECT` routine (`sub_8009294`) writes `0x0084` — cascade **and** enable — to all four control registers, so on this core the cascade chain never started (a local model read TM3 as 0 instead of 512). Pinned by `timer_zero_keeps_counting_when_the_cascade_bit_is_set` and `the_ags_timer_connect_chain_reaches_512`. |
| 26 | `timers.rs`: `tm0..tm3_start_delay`, armed on the enable edge and consumed before the prescaler advances (`advance` takes `&mut u8`). Cascaded timers are unaffected — they go through `apply_ticks`. | The AGS `TIMER PRESCALER` window is 4098 master cycles (1024 × `SUBS`+`BNE` plus two `mov r0, r0`) and the cartridge expects the counter to read 4096. The loop itself is exact (4092 cycles measured over 1023 iterations = 4.00/iteration), so two cycles sit between the enable edge and the start of counting. Pinned by `the_ags_window_counts_4096_of_its_4098_cycles`, which uses **literal** window and expectation values — the tests that spend `4096 + START_DELAY_CYCLES` follow the constant and would stay green when it changes. |

**⚠️ Change #26 is a calibrated constant, not a derived one.** Inside this model a start delay and no start delay produce the same constant offset and there is no internal reference for it; only the AGS anchor pins it. `TIMER CONNECT` does **not** corroborate it — mutation shows that case returns 512 with the delay set to 0 as well. It is recorded here as a fitted value on purpose, so that a future re-vendor or a second measurement treats it as an assumption to be re-tested, not a settled fact. See plan §十一 r21 ⑤ and r22 ⑧.

Timer patch count: 22 → **24 across 4 files**. |

The value is read from the **latched reload**, not from the running counter,
because the counter's low two bits advance as it runs; the divider is latched on
each write to `TMxCNT_L`, which is exactly when `set_reload` runs.

**Two of this crate's own tests had the defect locked green**, and both had to
change in the same commit — the same shape as §3.2.6 and the v2.0.1 P0 finding:

* `prescaler_divides_and_carries_remainder` set its divider with
  `set_control(0, ENABLE | IRQ | 0b01)`, i.e. in the **control** word, and passed
  because the implementation read the divider from there too. Worse, its setup
  was *impossible on real hardware*: it wanted a one-tick wrap period, which
  forces `reload = 0xFFFF`, whose low two bits select divide-by-1024. Only a
  split between the two registers let it hold both at once.
* `cascade_ticks_once_per_lower_overflow` used the same impossible pairing
  (timer 0, `reload = 0xFFFF`, "overflows every cycle") and now uses
  `0xFFF0` — prescaler 1, period of 16 ticks.

`the_ags_prescaler_cases_hold_for_every_divider` is new and asserts all four of
the `AGS` `TIMER PRESCALER` expectations `{4096, 64, 16, 4}` separately, so
passing on one divider cannot hide a failure on another. The small cases are the
ones that catch a divider that is simply never applied.

### 3.3 `src/gba/` — new code, no upstream content

`src/rust/src/gba/` is entirely first-party: the SWI implementations and the
C ABI. It is a plain **module of the root crate**, not a `crates/f11gba`
crate — see the v2.0 plan's R14 and the r8 entry for why a separate crate does
not build on rustc 1.96 fat LTO. Its C ABI surface is specified in the v2.0
plan section 4.1; S0' exposes the probe surface only, with the real surface
landing in S2.

### 3.5 `Cargo.toml` — the dropped `[dev-dependencies]`, and 227 tests that never ran

The vendoring kept `[dependencies]` and did not carry `[dev-dependencies]` over.
Upstream's tests `use pretty_assertions` in six modules, so `cargo test -p
gba-core` did not compile — and because the project's gate only ever ran
`cargo test -p fceux11-rust`, **nothing had noticed**.

| # | Change | Why |
|---|--------|-----|
| 20 | Added `[dev-dependencies] pretty_assertions = "1"` to `Cargo.toml` | Restores the section upstream had. First run after adding it: **227 tests, 0 failed.** The crate's entire unit-test suite — including every `GamePak` timing test in `bus.rs`, which is where the v2.0.1 precision work is aimed — had been unreachable rather than failing. |

This is worth more than its one line. A vendored crate whose tests cannot compile
is a crate whose tests do not exist, and "the suite is green" is a sentence that
can be true of a suite nobody ever invoked. It is also the same shape as the P0
finding in §3.2.3 one commit earlier — a gate that cannot fail is not a gate —
arrived at from the opposite direction: there the gate ran and was blind, here it
never ran at all. `AGENTS.md` now runs both crates in its gate command.

## 4. Patch-set size

| Item | Status |
|---|---|
| `gba-core` source changes | **22**, across **4** source files: 13 in `src/cpu/arm7tdmi.rs` (5 S0' hook wiring, no upstream logic altered + 4 S1a-1 halt mechanism, **does change `step()`** — see R15 + 1 v2.0.1 r56 wake-IRQ return address, see §3.2.2 + 1 v2.0.1 P0 ARM SWI number at bits 16-23, **changes what an ARM-mode BIOS call does**, see §3.2.3 + 2 v2.0.1 r5 halfword-load width tests, §3.2.5, **tests only**) + 3 in `src/cpu/hardware/rtc.rs` + 4 in `src/cpu/hardware/internal_memory.rs` (3 S2-b3 real-time clock + 2 S3-1 battery save; **neither changes behaviour when nothing is overridden**, see §3.2.1) + 2 in `src/bus.rs` (1 test-only block, see §3.2.4 + 1 **the S-cycle double-billing fix, which changes what every instruction costs**, see §3.2.6). **The file list is measured, not counted by hand**: `git log --name-only --diff-filter=M -- src/rust/crates/gba-core/src/` names exactly those files. Earlier drafts of this table said "4 files" while enumerating three and then "3" while the fourth was `Cargo.toml`; `bus.rs` is the first source file outside `cpu/` to be modified, and entry 20 was briefly used twice (§3.2.5) |
| `gba-core/Cargo.toml` | rewritten (metadata only) + one restored `[dev-dependencies]` section, see §3.5 |
| `src/gba/` | new, 100% first-party, 9 files |

The plan's risk **R1** says "keep the patch set minimal (SWI hook only)". That
constraint held for the first five edits and then broke — see below.

**S1a-1 breaks that constraint and says so.** Four more edits (total **9**)
add the halt mechanism, and unlike the first five they *do* change behaviour:
`step()` gains a branch. This is not a preference — `Bus::step` being
`pub(crate)` leaves no way to park the CPU from outside this crate — and it is
registered as **R15** in the v2.0 plan rather than slipped in here. The four
edits are kept as small as the feature allows: one guard, one state flag, one
callback, one `Default` line each.

The first five edits are `#[serde(skip)]`-safe and therefore invisible to
savestates. Of the new four, `halted` is **serialized** (it is machine state);
`wake_hook` is skipped like the SWI hook, and the embedder re-installs it after
a load.

**S2-b3 breaks the "one file" property that held until then** (see §3.2.1), and
that is the honest cost of making the clock verifiable: the `rtc` field is
private and `current_unix_secs()` has no external seam, so there is no version of
this that touches zero vendor files. Five more edits, two more files, total
**14** — and, unlike S1a-1, **none of them changes what the hardware does when
the embedder does not use them.** The R1 constraint ("keep the patch set minimal")
was already registered as broken by S1a-1; this is a second, smaller breach of
the same kind, and the patch set is now 14 entries over 3 files rather than 9
over 1.

**S3-1 adds two more to that same file, for the same reason** — the battery save
needs a way to ask what kind of save hardware is present (`backup_type`) and a
way to correct it (`set_backup_type`, which resizes the backing buffer and keeps
the old contents up to the shorter length). `battery_data`, `load_battery` and
`take_save_dirty` were already `pub`, so those two are the *only* reason the file
is touched at all. Neither changes behaviour when the host overrides nothing,
which is the normal case. The set was **16 entries over 3 files** at that point
(the "4 files" written here before r56 was a slip — the fourth file was
`Cargo.toml`, which is not a source change; see §3.3's measured note).

**v2.0.1 (r56) adds one more to `arm7tdmi.rs`** — the wake-IRQ return address
must land *after* the SWI that halted, not on it (see §3.2.2 for the mechanism
and the cartridge that exposed it). Unlike S2-b3/S3-1 this one **does change
what the hardware does on every wake from a wait-class SWI**, in the direction
of the real machine; locked by a mutation-checked lock test. The set is now
**17 entries over 3 files**.

**v2.0.1 (P0) adds one more to the same file** — the ARM SWI number comes from
bits 16-23, not the low byte (see §3.2.3). This one has a property none of the
others share: **upstream is wrong, not merely incomplete**, so a re-vendor can
lose it by taking a newer upstream revision that still reads the low byte. It is
also the only entry whose *absence* this project's own test suite could not
have caught, because the suite encoded ARM `SWI` words the same wrong way — two
first-party encoders had to be corrected in the same change for the fix to be
observable at all. The set is now **18 entries over 3 files**.

**v2.0.1 (P2 groundwork) adds the first one outside `cpu/`, and the first one
that is tests only** — two `bus.rs` tests holding the `AGS` Aging Cartridge's own
expected wait-state values (see §3.2.4). Production code is untouched, so unlike
every earlier entry this one cannot change what a game observes. It is
**19 entries over 4 files**, and the fourth file is a file the previous sentence
in this document had to be corrected about twice.

**And the number that matters more than any of them: 227 tests in this crate
were never being run** (see §3.5). Every claim this document makes about the
core's behaviour was made against a suite that covered only the embedder's side
of the seam. The count went from "246 green in the root crate" to "246 + 227",
and the first run of the 227 was green too — which is the outcome worth having
before anyone starts changing the timing model in §3.2.4.

**v2.0.1 (r5) adds two more to `arm7tdmi.rs`, both tests only** — the
halfword-load width pair, written to try to kill a candidate defect rather than
to confirm one (see §3.2.5). They killed it. The set is now **21 entries over 4
files**, and the entry count has stopped being the interesting number: **three
of the last four entries change no production line at all.**

## 5. Update procedure

When re-vendoring from a newer upstream commit, update in this order:

1. `Baseline commit` in §1 and in plan section 2.0.
2. `crates/gba-core/src/**` and `crates/gba-core/LICENSE`.
3. Re-run §3.1: if upstream's workspace still supplies different dependency
   versions, reconcile against `Cargo.lock` and record the delta here.
4. Re-check §2 (per-file header count) — if a future upstream revision adds
   headers to more files, note it so the rule in §2 stays accurate.
5. Re-run the plan's S0 exit criteria: `cargo check`, the staticlib symbol
   check, and the NES zero-regression gate.
