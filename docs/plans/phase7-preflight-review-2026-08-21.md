# Phase 7 Preflight - Phase 1-6 quality review (2026-08-21)

**Status:** Completed. **Branch:** `wip2.0`.
**Gate:** Phase 7 (delete the C++ CPU, flip `FCEUX11_RUST_CPU` default,
CMake/scripts updates) must NOT start until the user formally approves
this review's outcome.

> **Outcome (2026-08-22):** the review found 4 MUST-FIX unofficial-opcode
> parity gaps and SHOULD-FIX hygiene items; the fix plan
> (`docs/plans/phase7-preflight-fixes-2026-08-21.md`) closed them (commit
> `31c5b35`) and the user approved Phase 7, which was executed on
> 2026-08-22 (commit `9700094`, see the fixes doc §6).

## 0. Why this review

Phase 7 deletes the C++ CPU (`x6502.{cpp,h,struct.h,abbrev.h}`,
`ops.inc`, `ops_table.inc`, `cpu.cpp`) and makes the Rust CPU the only
implementation. Before that irreversible step, every claim made by
Phases 1-6 must be verified independently - correctness gates, FFI
safety, test coverage, documentation accuracy and repository hygiene.
The review is read-only plus targeted verification runs; it makes no
Phase 7 mutations.

## 1. Review dimensions

### A. Correctness gates (cpu-rust-v2.md ?7 acceptance criteria)

1. C++ baseline verified on `wip2.0` (CTest non-perf, OFF = 34/34).
2. Rust CPU wired via FFI, `FCEUX11_RUST_CPU=ON` builds.
3. No regression under ON (documented waivers only).
4. Interrupt / DMC / mapper-IRQ parity (blargg suites; known-fails are
   baseline-equal and deferred to Phase 6/7 - verify the interpretation
   and the waiver list).
5. Unofficial opcodes + savestate parity (105/105 coverage;
   `golden_savestate_test` byte-equal; `savestate_regression` 0/12).
6. Performance (`bench_tolerance_test` - machine-dependent, verify the
   current run and the documentation).
7. C++ CPU deleted (Phase 7 - explicitly NOT part of this review).
8. Documentation (`ChangeLog.md`, `cpu-rust-v2.md`, closeout docs).

### B. Safety / ABI / FFI audit

- Every `unsafe` block and `static mut` in `fceux11-core` (raw pointer
  correctness, aliasing, single-threaded invariants).
- FFI signatures vs the cbindgen-emitted `fceux11_rust.h` (CPU
  surface: run/init/reset/power/trigger_nmi/irq_begin/end/snapshot/
  restore/set_bus/set_irq_bridge/set_nmi_fresh_bridge/set_tick/
  set_tick_cycles).
- Layout pins: `offset_of!` asserts in `state.rs` vs C++
  `static_assert`s in `cpu.cpp` and `x6502struct.h`.
- The FFI copy/static-state model (FFI_CPU_STATE, BLOB_PTR mirror,
  DB/count sync) - re-verify the mid-call reader paths.
- The NMI-fresh bridge and tick bridge (pre/post semantics) against
  the C++ reference loop.

### C. Known residuals (verify documentation + baselines, do not "fix" blindly)

- `rom_regression_rust_smoke` 1/780 (nestest transition-frame 16-pixel
  PPU render artifact; all CPU observables byte-identical).
- `kagami_qa_direct_smoke` blargg known-fails (all in
  `blargg_known_fail.json`; C++ baseline fails them too).
- `bench_tolerance_test` machine-dependence.

### D. Test quality

- Opcode coverage 105/105; interrupts (15), cycle parity (17), nestest,
  opcodes, proptest (5), unofficial (74), snapshot (4).
- Determinism / flakiness (the earlier proptest full-suite flake and
  the FFI_CPU_STATE / TICK_FN test-pollution fixes).
- C++ side: golden_savestate_test, savestate_core_test, cpu_test,
  mapper tests, frame-diff tests under both modes.

### E. Documentation consistency

- `cpu-rust-v2.md` status/progress notes vs the actual tree.
- Closeout docs (phase4, phase4.5, phase5) vs the evidence.
- `ChangeLog.md` coverage of the Phase 4/5 work.

### F. Repository hygiene

- Untracked/leftover artifacts (trace logs, temp fixtures, build-dir
  debris), stale helper scripts, `git status` cleanliness.

## 2. Method

1. Git archaeology: enumerate Phase 1-6 commits and their scope.
2. Static audit of `fceux11-core` (all modules) + FFI consumers
   (`cpu.cpp`, `x6502.cpp`, `kagami_bridge.*`, `input.cpp` DB reads).
3. Dynamic verification: `cargo test`, `cargo clippy`, `cargo fmt
   --check`, `cargo build --release`; CMake build ON + OFF; CTest
   non-perf both modes; the two regression runners; golden_savestate
   verify + compare-layout (temporary nestest scenarios if needed).
4. Cross-check every number/claim in the docs against the runs.
5. Write the findings report (severity: blocker / must-fix / nit /
   verified-ok) and a GO / NO-GO recommendation for Phase 7.

## 3. Explicit boundaries

- No deletion of C++ CPU files, no `FCEUX11_RUST_CPU` default flip,
  no CMake/scripts/CI changes, no commits advancing Phase 7.
- Any "must-fix" found is reported to the user first; fixes are not
  applied without approval (this review is diagnostic).
- Temporary verification artifacts (compare-layout scenarios, trace
  files) are restored/removed before the review concludes.

## 4. Deliverables

- This document, completed with findings + severity.
- A concise review report to the user with a GO / NO-GO
  recommendation, awaiting formal approval before Phase 7.

## 5. Findings (completed 2026-08-21)

### 5.1 MUST-FIX - unofficial-opcode parity gaps in `do_unofficial` (Phase 2-era, gate-missed)

Four pre-existing divergences from the C++ reference were found in
`src/rust/crates/fceux11-core/src/cpu/execute.rs::do_unofficial`.
They are NOT covered by any current gate: nestest's 60-frame traces
never fetch the LAS test sites, the SHX/SHY/AHX/TAS vectors nestest
executes do not trigger the divergences (register-only log; X==Y; no
page cross), and the Phase 5 tests asserted the Rust's own formula
with non-triggering values. Empirically confirmed on the Rust side
(measured) against the unambiguous C++ macros (`ops.inc` /
`x6502.cpp`).

1. **SHX 0x9E wrong index register.** Rust uses `abs_x_write`
   (indexed by X); the C++ reference uses `GetABIWR(A,_Y)` and NESdev
   says SHX = abs,Y. Write address differs whenever X != Y. Measured:
   X=1, Y=2, base $6000 -> Rust writes $6001; C++ would write
   `((X&(eff_hi+1))<<8)|eff_lo` = $0102.
2. **SHX 0x9E / SHY 0x9C missing the C++ write-address high-byte
   replacement.** C++ computes `A = ((reg&((A>>8)+1))<<8) | (A&0xff)`
   and writes `A>>8` to that modified address; Rust writes
   `reg&((eff>>8)+1)` at the plain effective address. Measured: SHY
   X=2, Y=1, base $6000 -> Rust writes $6002=01; C++ writes $0102=01.
3. **AHX 0x9F/0x93 and TAS 0x9B use effective-high+1 for H; C++ uses
   base-high+1 (`(A-_Y)>>8)+1`).** Stored value differs on page-cross.
   Measured: base $60FF, Y=1 -> Rust stores 0x02 (A&X=0x0F);
   C++ would store 0x01.
4. **LAS 0xBB uses read-mode addressing; C++ uses RMW write-mode.**
   C++ `RMW_ABY` (GetABIWR: no page-cross penalty) does two WrMem
   write-backs; Rust `abs_y_read` charges +1 on page-cross and does no
   write-back. Measured: Rust page-cross cycles=5, writes=0; C++
   cycles=5 always, writes=2. Diverges in cycles (page-cross) and bus
   access pattern (always).

Impact: games/ROMs that execute these opcodes with triggering values
diverge from the C++ baseline (frame/savestate parity). The
`rom_regression`/`nestest` gates do not exercise them. These must be
fixed and covered by per-opcode tests that use page-crossing and
X!=Y vectors before Phase 7.

### 5.2 SHOULD-FIX (hygiene / quality)

5. `cpu/ffi.rs` `fceux11_cpu_run` doc comment is stale: it describes
   the pre-Phase-4.5 count semantics ("cumulative counter... exits
   when count >= target") and says the return value advances
   timestamps. The code now uses C++-polarity count (exit on
   count <= 0) and timestamps advance via `fceux11_cpu_set_tick_cycles`
   (the run return is discarded).
6. `cpu/decode.rs` Phase-1-era comment ("Phase 2 will swap for a
   build.rs-generated file") is stale; the codegen was explicitly
   deferred and should be documented as such in the comment.
7. ARR (0x6B): dead `bit7` variable + duplicated `let _ = bit7;`
   (cosmetic).
8. `cargo clippy` is not clean: fceux11-core has ~25 warnings
   (FFI functions missing `# Safety` doc sections, "operation has no
   effect" x4, unused import `run` in tick.rs, etc.); the workspace
   build fails clippy in `fceux11-formats` (pre-existing, out of CPU
   scope). No CI gate currently depends on it.
9. `cargo fmt --check` fails (benches/step_bench.rs, addressing.rs,
   etc.).
10. `ChangeLog.md` has not been updated for the Phase 4/5 closeouts
    (acceptance criterion #8).

### 5.3 VERIFIED-OK

- 64-byte layout pins (state.rs `offset_of!` vs C++ static_asserts),
  flag/IRQ constants.
- FFI surface complete in the cbindgen header (incl.
  `set_nmi_fresh_bridge`, `set_tick_cycles`).
- DB/count blob mirror, NMI-fresh bridge, C++-exact tick semantics
  (pre-body temp / post-body total / tcount), early-exit timestamp
  advance - all consistent with the C++ reference loop.
- `cpu/snapshot.rs` + 4 tests; unofficial coverage 105/105.
- `cargo test -p fceux11-core` = 210 PASS; CTest ON 32/34
  (direct_smoke known-fails + documented rom_regression 1/780),
  CTest OFF 34/34.
- Commit history clean (19 commits Phase 1-5); worktree clean after
  review cleanup (diagnostic debris removed).

### 5.4 GO / NO-GO

**NO-GO for Phase 7** until the MUST-FIX set (5.1 items 1-4) is
implemented, covered by per-opcode tests, and re-verified (cargo +
CTest ON/OFF + regression runners). SHOULD-FIX items 5-10 are
recommended before the Phase 7 merge; items 8-10 are hygiene and can
be bundled with Phase 7's CMake/scripts pass if approved.
