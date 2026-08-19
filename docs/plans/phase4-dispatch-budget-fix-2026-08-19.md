# Phase 4 sub-step 5 follow-up — Dispatch-budget exhaustion early-exit

Date: 2026-08-19. Branch `wip2.0`. Status: **partial — hypothesis refuted, but real fix landed**.

> Reference: `docs/plans/cpu-rust-v2.md` §4 Phase 4 sub-step 5, §7 acceptance
> criteria #3 ("no regression under FCEUX11_RUST_CPU=ON") and #5 ("savestate
> byte-equal"). **This document records the dispatch-budget exhaustion corner
> of the cycle-accounting drift, not the dominant root cause** — see §5
> "Why this fix did not close the cycle-drift family" for the honest
> post-mortem.

## 0. TL;DR

A targeted fix to align Rust's CPU dispatch loop with the C++ `X6502_RunDebug`
early-exit at `src/x6502.cpp:586-588` *did* close one specific corner case and
fixed `cpu_test` 802/802 (previously 799/802 with 3 sub-failures), but
**did not close** the cycle-drift family of failures (`apu_wav_diff_test`,
`golden_savestate_test`, `kagami_qa_direct_smoke` blargg sub-tests,
`rom_regression_rust_smoke`, `savestate_regression_rust_smoke`). The dominant
cycle drift lives elsewhere in the FFI/C++ loop integration — see §5.

## 1. What was tried

### 1.1 Bug hypothesis

When a `fceux11_cpu_run_with_tick(state, cycles_arg)` call lands with
`cycles_arg < 21` (i.e. `cycles_arg * 16 < 336 = ADDCYC(7) * 48`) AND an
interrupt is pending at the instruction boundary, the prior Rust loop ran
the dispatch's `ADDCYC(7)` AND the follow-up instruction AND fired the
mapper/APU tick — whereas the C++ `X6502_RunDebug` *returns* after the
dispatch's `_count <= 0` check, **skipping the follow-up instruction and
the hook call entirely**.

Each such corner produces ~6.6 samples of APU drift per call, observed
across ~10 calls per VBL → roughly 7 samples/frame → matches the observed
28-byte / 7-sample drift in `apu_wav_diff_test` (`golden 1512` vs
`captured 1540`).

### 1.2 Code change

Split the Rust `step()` function in `cpu/execute.rs:179-374` into:

- `pub(crate) fn dispatch_step(state, bus) -> u8` — runs `dispatch_irq`,
  sets `moo_pi = p` after the dispatch (mirroring C++ ordering), and on a
  dispatched IRQ adds `irq_cycles * 48` to `state.regs.count`.
- `pub(crate) fn execute_step(state, bus, dispatch_irq_cycles) -> u8` —
  fetches + executes the next instruction. Updates `state.regs.count`
  with `(CycTable + extras) * 48`. `state.cycles_in_run` only tracks the
  instruction portion (matching the original semantics).
- `pub fn step(...)` — kept as a thin wrapper that invokes both
  atomically. All existing tests (`unofficial.rs` × 53, `opcodes.rs`,
  `nestest.rs`, `interrupts.rs`, `cycle_parity.rs` × 15, `execute.rs::tests`
  × 9) continue to call `step()` unchanged.

`pub fn run(...)` and `tick::run_with_tick(...)` were rewritten to mirror
the C++ loop structure:

```rust
loop {
    let dc = dispatch_step(state, bus);
    let dc_plus_ic: u8;
    if dc != 0 {
        executed_cycles += dc;
        if jammed || count >= target { break; }   // ← C++ early-exit
        let ic = execute_step(state, bus, dc);
        executed_cycles += ic;
        dc_plus_ic = dc + ic;
    } else {
        let ic = execute_step(state, bus, 0);
        executed_cycles += ic;
        dc_plus_ic = ic;
    }
    tick_instruction(dc_plus_ic as i32);            // one tick per iter with sum
    if jammed { break; }
    if count >= target { break; }
}
```

Mirroring the C++ `temp = _tcount; hook(temp)` semantic at
`src/x6502.cpp:607-615`, the tick is fired *once per iteration* with the
iteration's total cycle count (dispatch cost + base + extras). The early
exit skips both the follow-up instruction AND the tick — matching the C++
path.

## 2. Tests added / updated

### 2.1 `tests/cycle_parity.rs`

| Test | Asserts |
|---|---|
| `dispatch_just_exhausts_budget_does_not_execute_followup_instruction` | budget = 8 cycles; iteration 1 defers + runs NOP at $4000 (2 cycles); iteration 2 dispatches NMI (7 cycles, count = 432 ≥ 128) → early-exit. PC = $5000 (NMI vector), NOT past the follow-up NOP at $5000. consumed = 9 (= 2 + 7) |
| `dispatch_does_not_suppress_followup_when_budget_is_ample` | budget = 100 cycles. Both defer+NOP and dispatch+NOP iterations execute; consumed > 9; PC > $5000 |

After the fix this file has 17 tests (was 15), all passing.

### 2.2 `tick.rs::tests::dispatch_cycles_are_included_in_tick` — UPDATED

The pre-fix assertion was:

```rust
assert_eq!(TICK_COUNT, 2);          // enforced the buggy tick-on-budget-exhaustion behavior
assert_eq!(TICK_SUM, 2 + 9);        // tick with dispatch(7) + NOP(2) on iteration 2
```

The post-fix assertion locks the correct C++ behaviour:

```rust
assert_eq!(TICK_COUNT, 1);          // only the defer+NOP iteration ticks
assert_eq!(TICK_SUM, 2);            // dispatch-budget exhaust → no tick on iteration 2
assert_eq!(cpu.regs.pc, 0x5000);    // NMI vector, follow-up NOP suppressed
```

The test was *enforcing the wrong behavior* before this change; updating
it is part of the fix, not a regression.

### 2.3 in-module tests

All 9 `execute.rs::tests::*` (driven by `pub fn step()`) and all 3 tick.rs
internal tests pass under the fix. The `no_hook_installed_is_noop` test
still asserts `run_with_tick == run` and holds: both paths delegate to
`dispatch_step` + `execute_step` symmetrically.

## 3. Rust-side test result (`cargo test -p fceux11-core`)

```
test result: ok. 86 passed; 0 failed   (lib tests)
test result: ok. 17 passed; 0 failed   (cycle_parity: +2)
test result: ok. 15 passed; 0 failed   (interrupts)
test result: ok.  2 passed; 0 failed
test result: ok.  7 passed; 0 failed   (nestest)
test result: ok.  5 passed; 0 failed   (opcodes)
test result: ok.  5 passed; 0 failed   (proptest_fuzz)
test result: ok. 53 passed; 0 failed   (unofficial)
─────────────────────────────────────────
              190 PASS / 0 FAIL
```

Total Rust-side tests: 190 (was 182 baseline; +2 new cycle_parity tests,
plus the exact test count skewed by the renaming of the proptest count).
All pass.

## 4. CTest result (`scripts/_phase5_rebuild_rust_cpu.ps1`)

### 4.1 `FCEUX11_RUST_CPU=OFF` baseline — 34/34 PASS

Unchanged from the pre-fix run.

### 4.2 `FCEUX11_RUST_CPU=ON` — 29/34 PASS, 5 FAIL

Compared to the baseline before this commit (`f522451` head, captured in
`__rust_cpu_rebuild_fresh.log`):

| Test | Pre-fix | Post-fix | Δ |
|---|---|---|---|
| `apu_wav_diff_test` | 3/3 FAIL (size 1540 vs 1512) | **3/3 FAIL (size 1540 vs 1512)** | **0** |
| `golden_savestate_test` | 7/8 MD5 FAIL | **7/8 MD5 FAIL** (same MD5s) | **0** |
| `kagami_qa_direct_smoke` | 6 FAIL / 12 | **6 FAIL / 12** (same 6 sub-tests) | **0** |
| `rom_regression_rust_smoke` | 1/780 | **1/780** | **0** |
| `savestate_regression_rust_smoke` | 12/12 FAIL | **12/12 FAIL** (same 11 hashes; nestest hash changed) | **0** |
| **`cpu_test` sub-cases (3 had been failing)** | 799/802 | **802/802** ✅ | **+3 PASS** |

Per-test sub-tally for `kagami_qa_direct_smoke` is identical to pre-fix
(same 6 failures: `cpu_instrs`, `cpu_int_2_nmi_brk`,
`mmc3_v2_4_scanline_timing`, `ppu_vbl_nmi`, `sprdma_dmc_dma`,
`vbl_05_nmi_timing`) and 6 PASS / 6 FAIL / 2 blocking. The 12-ROM hash
tally from `savestate_regression_rust_smoke` reproduces 11 of 12 hashes
verbatim; the lone `nestest` hash changed
(`347cb1272f19acbfb8b232d849ffb408` → `da3713eab5381cb6738eb3ca916be14f`).

## 5. Why this fix did not close the cycle-drift family

### 5.1 Hypothesis test

The hypothesis predicted:

- 7 samples/frame APU drift from ~10 dispatch-budget-exhaustion events per frame.
- Spread across multiple calls per frame (PPU loop calls `X6502_Run(cycles)`
  many times per scanline; the budget is small enough that this case fires
  frequently on the VBL boundary).

If the hypothesis were dominant, removing the extra instruction + tick on
the dispatch-budget-exhaustion case would reduce `apu_wav_diff_test`'s
sample count from `1540` to `~1512 ± 1`. It did not — `1540` is
unchanged byte-for-byte.

The 11 ROM hashes from `savestate_regression_rust_smoke` reproducing
verbatim across the fix is the strongest signal that the dominant cycle
drift lives in a code path that this fix did not touch. The dispatch-
budget-exhaustion case is *rare enough* under real workloads that removing
its over-shoot does not materially move the per-frame state.

### 5.2 Open candidates for the dominant cycle drift

In order of priority for the next investigation:

1. **DMC DMA steal cycles.** The C++ FCEUX stalls the CPU for 1–4 cycles
   per DMC DMA byte transfer (`fceu.cpp::FCEUI_DDPCM`). The Rust CPU has
   no concept of DMA steal — it executes the full `CycTable[b1]` regardless
   of DMC state. The mapper/APU `tick` hook already forwards correct cycle
   counts, but the `Cpu::timestamp_` / `sound_timestamp_` accumulators do
   NOT reflect the steals, which still desynchronises audio window
   closure and frame-end MMIO state.

2. **OAM DMA steal cycles** (`$4014` write). Same root cause as DMC.

3. **`state.regs.count` saturation boundary.** Rust uses
   `saturating_add(dot(cycles) * 3)` (clamp to `i32::MAX`); C++ uses
   raw `i32` decrement that wraps on `_count -= ...`. For typical
   workloads `count` stays well within range, but proptest seeds and
   pathological instruction streams might brush the saturation boundary.

4. **PPU loop ↔ CPU `fceux11_cpu_run_with_tick` per-call duration.** The
   FFI shim in `src/cpu.cpp::Cpu::run(cycles)` passes `cycles` directly,
   but the C++ `X6502_RunDebug` is called *many* times per frame from
   `FCEUPPU_Loop` with small cycles values. If a tiny difference per call
   compounds over ~10,000 calls/frame, the per-frame trajectory is
   deterministic but divergent.

5. **`CycTable` value drift between Rust and C++.** Per-instruction
   `tests/cycle_parity.rs` (15 tests) locks this for ~30 opcodes
   individually; the full 256-entry table is `cycle_table_has_256_entries`
   + `known_opcodes_match_nesdev_matrix`. Any divergence in unmonitored
   opcodes would accumulate silently.

The cheapest next diagnostic is **(4) + (5)**: add a cross-language
harness that runs the same ROM for 1 NTSC frame under both C++ and Rust,
dumps `(instruction_pc, dispatch_fired, dc, ic, count_after, target)` to a
CSV per 1000 instructions, then `diff`s the two CSVs. The first divergence
localises the dominant root cause within a few minutes.

## 6. Files modified

- `src/rust/crates/fceux11-core/src/cpu/execute.rs` — split `step` into
  `dispatch_step` + `execute_step`; rewrote `pub fn run`.
- `src/rust/crates/fceux11-core/src/cpu/tick.rs` — rewrote
  `run_with_tick` for early-exit; updated
  `tests::dispatch_cycles_are_included_in_tick` to lock the corrected
  behaviour.
- `src/rust/crates/fceux11-core/tests/cycle_parity.rs` — added two tests.

## 7. Repro / verify

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/_phase5_rebuild_rust_cpu.ps1
```

Expected (matching the post-fix observation): `29/34 PASS, 5 FAIL` (same
5 as before; cycle-drift family still open).

To verify the corner-case fix without running the full rebuild:

```
cd src/rust
cargo test -p fceux11-core --test cycle_parity
```

Expected: 17 PASS, including the new
`dispatch_just_exhausts_budget_does_not_execute_followup_instruction`.
