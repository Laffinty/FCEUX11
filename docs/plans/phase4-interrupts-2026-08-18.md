# Phase 4 Step 1 — FFI Wiring Completed (2026-08-18, branch `wip2.0`)

> Reference: `docs/plans/cpu-rust-v2.md` §4 Phase 4 (REVISED) — interrupt / DMC / mapper-IRQ parity.
> Toolchain: MSVC 19.36+ (cl 14.51.362), Ninja, CMake 4.2.3, Rust 2024.
> This report covers the **wiring fix** that completed Phase 3 step 5's gate. The remaining cycle-accounting divergences documented in §4 below are the work for subsequent Phase 4 sub-steps.

## 0. The bug Phase 3 didn't catch

Phase 3 step 5 closed at 30/33 CTest PASS with `FCEUX11_RUST_CPU=ON` — but that number was misleading. The `X6502_RunDebug(g_cpu, cycles)` function in `src/x6502.cpp` is the **free function** called by `FCEUPPU_Loop` and the rest of the PPU loop. The `Cpu::run` method I had patched to call the FFI was a **different code path** that nothing reached. The C++ `X6502_RunDebug` body was still running the real dispatch loop untouched, so the Rust CPU was never actually exercising on tests like `savestate_core_test::test_save_load_after_reset` — that test passed despite the FFI being bypassed.

The fix: change `X6502_RunDebug` to delegate to `Cpu::run` when `FCEUX11_RUST_CPU=1`. Now the FFI dispatch is on the actual hot path.

## 1. Fixed bugs found in Phase 4 step 1

### 1.1 `X6502_RunDebug` bypassed the FFI

`src/x6502.cpp::X6502_RunDebug` was the entry point for every `X6502_Run(cycles)` call inside `FCEUPPU_Loop` (53 occurrence sites). The macro `X6502_Run(cycles)` expands to `X6502_RunDebug(g_cpu, cycles)`, which bypassed the `Cpu::run` method entirely. **Fix:** under `FCEUX11_RUST_CPU=1`, `X6502_RunDebug` is a one-line forward to `cpu.run(cycles)`. The C++ body is preserved for the `OFF` build so the reference accounting math stays in the tree for documentation.

```cpp
#if FCEUX11_RUST_CPU
void X6502_RunDebug(fceu11::Cpu& cpu, int32 cycles) {
    cpu.run(cycles);
}
#else
void X6502_RunDebug(fceu11::Cpu& cpu, int32 cycles) {
    // existing C++ dispatch loop — preserved for the OFF build
}
#endif
```

### 1.2 RESET dispatch returned 7 cycles (should be 0)

`src/rust/crates/fceux11-core/src/cpu/execute.rs::dispatch_irq` originally returned `7` for the RESET branch. The C++ RESET branch does **no** `ADDCYC` — the cycle cost comes from the instruction the CPU executes next (typically a `JMP` at the reset vector). Returning `7` mis-credits 7 cycles and let `step()`'s follow-up `unreachable!` blow up on any ROM that doesn't have a `JMP` at the reset vector. **Fix:** return `0` for RESET. The follow-up instruction's cycles are counted by the normal `count += dot(cycles)` path in `step()`.

### 1.3 `step()` assumed RESET was followed by JMP

After dispatch, `step()` had `unreachable!()` for any non-`OpKind::Jump` opcode. The C++ executes whatever the reset vector points at. **Fix:** add `Jump` and `Branch` arms; other `OpKind`s fall through with the base cycle cost. The `unreachable!()` was removed.

### 1.4 `_PI = _P` ordering

The C++ `X6502_RunDebug` runs `_PI = _P` **after** the IRQ dispatch, so the NEXT iteration's maskable-IRQ check uses the post-dispatch I flag. The Rust originally set `moo_pi = p` at the **start** of `step()`, before `dispatch_irq`. After RESET/NMI sets the I flag, the Rust's `moo_pi` still had the OLD value, so the next iteration would have allowed an interrupt that the C++ would have blocked. **Fix:** move `moo_pi = p` to **after** `dispatch_irq`, matching the C++ ordering.

### 1.5 `X6502_Reset` semantic

`X6502_Reset()` does `_IRQlow = FCEU_IQRESET` (overwrite, not OR). The Rust originally did `layout.irq_low |= RESET.bit()`, which left any pending NMI bit set after a warm reset. The unit test `cpu::ffi::tests::reset_only_sets_reset_bit` was updated to lock in the overwrite behaviour.

### 1.6 Side-state `nmi_fresh` in the FFI

`CpuState::nmi_fresh` (the Rust counterpart of C++ `g_e1_nmi_fresh`) is held in the FFI's `static FFI_CPU_STATE`. The `fceux11_cpu_power`, `fceux11_cpu_reset`, and `fceux11_cpu_restore` paths all reset it to `false`. `fceux11_cpu_trigger_nmi` sets it to `true`. Confirmed identical to C++ semantics in the unit test `cpu::ffi::tests::trigger_nmi_sets_nmi_bit_and_fresh`.

### 1.7 `Cpu::timestamp_` advancement

The C++ `Cpu::add_cycles(c)` (called per instruction) does `timestamp_ += c`. The FFI returns total CPU cycles consumed (sum of every opcode's base cycle cost + page-cross / branch-taken extras); the C++ shim in `src/cpu.cpp::Cpu::run` adds that to `timestamp_` and `sound_timestamp_`. This is documented as a deliberate split between the 64-byte X6502 layout (which the FFI operates on) and the C++ `Cpu` object (which owns the timestamps).

## 2. C++ baseline (default `FCEUX11_RUST_CPU=OFF`) — 34/34 PASS

After the `X6502_RunDebug` refactor was added under the `#if FCEUX11_RUST_CPU` guard, the existing C++ CPU still passes every test:

```
100% tests passed, 0 tests failed out of 34
Total Test time (real) =  58.82 sec
```

The new `x6502.cpp` include for `rust/fceux11_rust.h` is also `#if`-guarded, so the OFF build is unaffected.

## 3. Rust CPU (FCEUX11_RUST_CPU=ON) — 28/33 PASS

Now that the FFI is actually wired into the host path, the Rust CPU is exercised end-to-end. The new measurement is:

```
85% tests passed, 5 tests failed out of 33
Total Test time (real) =  40.47 sec

The following tests FAILED:
    8 - cpu_test (Failed)                      - 3 sub-cases
   15 - apu_wav_diff_test (Failed)
   23 - golden_savestate_test (Failed)
   32 - rom_regression_rust_smoke (Failed)     - 2/780 frame mismatches
   33 - savestate_regression_rust_smoke (Failed) - 12/12 ROM mismatches
```

### 3.1 Per-test analysis

| Test | Baseline (C++) | Rust CPU | Diagnosis |
|---|---|---|---|
| `savestate_core_test` | 26/26 PASS | **26/26 PASS** ✓ | The original Phase 3 step 5 failure ("ResetNES changes CPU state") is now fixed. The wiring was the missing piece. |
| `cpu_test` | 802/802 PASS | 799/802 PASS — "PC has not wrapped below reset vector" / "PC has advanced past the first instruction" / "CPU not jammed after nestest run" | The `X6502_Run(1)` cold path with my `(cycles/3)*16` multiplier becomes `(1/3)*16 = 0` which the Rust `run` short-circuits. With the C++ CPU, `X6502_Run(1)` does at least one loop iteration even when 1 cycle isn't enough for a full instruction. The Rust CPU needs to handle `cycles < 3` by still allowing at least one step of the dispatch loop. |
| `apu_wav_diff_test` | 0-byte diff | mismatched | The Rust CPU's per-frame cycle count drifts relative to the C++. The `Cpu::timestamp_` accounting is over- or under-counting by a few cycles per frame, which desynchs the APU's sample-window boundaries. |
| `golden_savestate_test` | byte-equal | mismatched | The Rust CPU's divergent state is captured by the savestate and re-loaded differently on the C++ side. Will close once the cycle-accounting drift is fixed. |
| `rom_regression_rust_smoke` | 0/780 mismatch | 2/780 mismatch (~0.3%) | Two specific frames diverge. These are the same family of cycle-accounting divergence. |
| `savestate_regression_rust_smoke` | 0/12 mismatch | 12/12 ROM mismatch | The hash-comparison is per-ROM, so a drift in any ROM registers as a mismatch. The breadth (12/12) is more pessimistic than the per-frame rate (2/780) because the hash is taken after a fixed number of frames and the drift accumulates. |
| All other 28 tests | PASS | PASS | Identical to the C++ baseline. |

### 3.2 The remaining work — Phase 4 sub-steps 2-5

The 5 failures cluster into two symptom classes:

1. **`X6502_Run(1)` short-circuit bug** (`cpu_test`): the Rust `run()` early-exits when `cycles <= 0` (line 244). The `(cycles/3)*16` pre-scaling produces `0` for any `cycles < 3`, which is then passed to `run()` as `cycles <= 0`, returning 0 immediately. The C++ loop body requires at least one full iteration per call (it always reads the next opcode, even if no instruction completes). The fix is to keep the dispatch loop iteration guarantee even when `scaled_cycles == 0` — minimum 1 step per `fceux11_cpu_run` call.

2. **Cycle-accounting drift** (`apu_wav_diff`, `golden_savestate`, `rom_regression`, `savestate_regression`): the Rust's `count += CycTable * 16` per instruction does not account for the same penalty/extras that the C++ accumulates via `ADDCYC` inside the opcode handlers. The C++ decrement is `CycTable * 48` per instruction, which already includes the C++'s per-handler extras (page-cross penalty, branch-taken, etc.). The Rust's `mode_result.extra_cycles` adds the same extras to `cycles` (which is then multiplied by 16), so the totals should match — but somewhere they don't. A per-frame cycle-count assertion is needed to localise the drift before the next phase item.

## 4. Code handoff for Phase 4 sub-steps 2-5

| Sub-step | What it covers | Gating test |
|---|---|---|
| 1 (this report) | FFI wiring, RESET/`_PI`/NMI dispatch | `savestate_core_test` (DONE), `cpu_test` (partially) |
| 2 | `tests/interrupts.rs` — IRQ/NMI/BRK priority, edge-detect, mask | Per-opcode unit tests catch single-opcode bugs locally |
| 3 | `tests/unofficial.rs` — 109 unofficial opcodes, register-effect coverage | `nestest_first_5000_instructions_match_log` (DONE) + per-opcode |
| 4 | Wire blargg `cpu_timing_test6`, `cpu_interrupts_v2`, `cpu_dummy_reads_suite` through the FFI | Each suite PASS under `FCEUX11_RUST_CPU=ON` |
| 5 | Per-frame cycle-count parity assertion | `cpu_test` 3 sub-cases PASS, `rom_regression` 0/780, `savestate_regression` 0/12 |

## 5. Files modified in this session

| File | Change |
|---|---|
| `src/x6502.cpp` | `#if FCEUX11_RUST_CPU` short-circuit `X6502_RunDebug` to `cpu.run(cycles)`; include `rust/fceux11_rust.h` under the same guard |
| `src/rust/crates/fceux11-core/src/cpu/execute.rs` | `dispatch_irq` returns 0 for RESET (was 7); `step()` removes the `unreachable!` JMP assumption; `moo_pi = p` moved to after `dispatch_irq` |
| `src/rust/crates/fceux11-core/src/cpu/ffi.rs` | `fceux11_cpu_reset` overwrites `irq_low` (was OR); test `run_executes_nestest_reset_to_c000` passes 3 (was 10) and asserts 3-cycle consumption |

## 6. What we did NOT do

- We did not write `tests/interrupts.rs` or `tests/unofficial.rs` (Phase 4 sub-steps 2-3).
- We did not wire blargg `cpu_timing_test6` / `cpu_interrupts_v2` (Phase 4 sub-step 4). These suites are present in `tests/fixtures/blargg/` (177 ROMs total, see `scripts/download_blargg_roms.ps1`) and the `kagami_qa_direct_runner` already drives them under the C++ CPU — no FFI-specific runner is needed.
- We did not fix the `cpu_test` cold-path short-circuit (sub-step 5 ticket).
- We did not commit or push — the working tree is the source of truth for the next session.