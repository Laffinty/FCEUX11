# Phase 7 Preflight Fix Plan - reach the Phase 7 entry standard (2026-08-21)

**Status:** Plan - generated from `docs/plans/phase7-preflight-review-2026-08-21.md`.
**Branch:** `wip2.0`. **Goal:** implement the review's MUST-FIX and
SHOULD-FIX items, re-verify the full matrix, and make the tree ready
for the Phase 7 deletion/flip (which remains a separate, user-approved
step).

## 0. Entry standard (what "ready for Phase 7" means)

1. Every MUST-FIX parity gap from the review is fixed and covered by a
   per-opcode test that uses a triggering vector (X != Y, page-cross).
2. `cargo test -p fceux11-core` green (expect 210 + new tests).
3. `cargo clippy -p fceux11-core --all-targets --no-deps` clean (or
   the remaining warnings are explicitly waived with justification).
4. `cargo fmt --check` clean.
5. CTest: OFF 34/34; ON 32/34 with only the two documented residuals
   (`kagami_qa_direct_smoke` blargg known-fails and the documented
   `rom_regression` 1/780 PPU render-timing artifact).
6. `golden_savestate_test` byte-equal under ON; `savestate_regression`
   0/12.
7. Docs: this plan completed with results, review doc kept,
   `cpu-rust-v2.md` status updated, `ChangeLog.md` updated.

## 1. MUST-FIX - unofficial-opcode parity (execute.rs `do_unofficial`)

All four reference lines are in `src/ops.inc` / `src/x6502.cpp`
(GetABIWR/GetIYWR/RMW_ABY macros).

### 1.1 SHX 0x9E - wrong index register

- Reference: `case 0x9E: GetABIWR(A,_Y); ...` (SHX = abs,Y; stores X).
- Current (Rust): `let addr = abs_x_write(state, bus).addr;` - X index.
- Fix: use `abs_y_write(state, bus).addr` (Y index) and apply 1.2's
  write-address quirk.

### 1.2 SHX 0x9E / SHY 0x9C - C++ write-address high-byte replacement

- Reference (0x9C SHY, 0x9E SHX):
  ```c
  unsigned int A; GetABIWR(A,_X/_Y);            // effective address
  A = ((_reg & ((A>>8)+1)) << 8) | (A & 0xff);  // replace high byte
  WrMem(A, A>>8);                                // store masked high byte
  ```
  i.e. the STORE VALUE is `reg & (eff_hi+1)` and the WRITE ADDRESS is
  `((reg & (eff_hi+1)) << 8) | (eff & 0xff)` (note: eff-high+1, NOT
  base-high+1 - distinct from AHX/TAS below).
- Current (Rust): writes `reg & ((eff>>8)+1)` at the plain effective
  address.
- Fix:
  ```rust
  let eff = abs_y_write(state, bus).addr;        // SHX; abs_x_write for SHY
  let hi = ((eff >> 8) as u8).wrapping_add(1);
  let masked = (reg & hi) as u16;
  let write_addr = (masked << 8) | (eff & 0x00FF);
  state.wr(bus, write_addr, masked as u8);
  ```
- Tests: SHX with X!=Y and SHY with X!=Y, non-cross and page-cross;
  assert the write ADDRESS and VALUE match the C++ formula.

### 1.3 AHX 0x9F/0x93, TAS 0x9B - H uses base high byte, not effective

- Reference: `ST_ABY(_A&_X&(((A-_Y)>>8)+1))` / `ST_IY(...)` and
  `ST_ABY(_S&(((A-_Y)>>8)+1))` - H = ((effective - Y)>>8)+1 = base
  high byte + 1 (for ind,Y the base is the pointer).
- Current (Rust): `h = (addr >> 8).wrapping_add(1)` with addr =
  effective.
- Fix: compute the base first:
  ```rust
  let eff = /* abs_y_write or ind_y_write */;
  let base = eff.wrapping_sub(state.regs.y as u16);
  let h = ((base >> 8) as u8).wrapping_add(1);
  // AHX: wr(eff, a & x & h); TAS: s = a & x; S = s; wr(eff, s & h)
  ```
- Tests: page-crossing vectors (base $60FF, Y=1 -> eff $6100) asserting
  the base-high+1 value.

### 1.4 LAS 0xBB - RMW write-mode + two write-backs

- Reference: `case 0xBB: RMW_ABY(_S&=x;_A=_X=_S;X_ZN(_X));` where
  `RMW_ABY = RMW_ABI(_Y) = { GetABIWR(A,_Y); x=RdMem(A); WrMem(A,x);
  op; WrMem(A,x); }`. GetABIWR = write-mode: NO page-cross penalty,
  dummy read at the non-crossed address.
- Current (Rust): `abs_y_read` (read-mode: +1 on page-cross) with no
  write-back.
- Fix:
  ```rust
  let addr = abs_y_write(state, bus).addr;   // write-mode, extra = 0
  let m = state.rd(bus, addr);
  state.wr(bus, addr, m);                    // first write-back
  let v = m & state.regs.s;
  state.regs.a = v; state.regs.x = v; state.regs.s = v;
  // set Z/N from v (current zn_table_lookup path)
  state.wr(bus, addr, m);                    // second write-back
  ```
  (both write-backs store the original `m`, exactly like C++.)
- Tests: page-crossing LAS asserting cycles = 5 (base CycTable, no
  extra) and a bus that observes exactly two write-backs to the
  effective address.

### 1.5 Regression tests for the gate gap

- Add the triggering-vector tests above to `tests/unofficial.rs`.
- Extend `tests/cycle_parity.rs` with LAS (page-cross = 5) and the
  SHX/SHY/AHX/TAS cycle counts so per-instruction cycle parity covers
  these opcodes too.
- Re-run the full unofficial + cycle-parity + nestest suites.

## 2. SHOULD-FIX (hygiene / quality)

1. `cpu/ffi.rs` `fceux11_cpu_run` doc comment: rewrite the "## Cycle
   accounting" section to the current semantics (budget added,
   count decremented per instruction, exit when count <= 0; timestamps
   advanced by the tick_cycles callback, return value informational).
2. `cpu/decode.rs` Phase-1-era comment: state that table codegen was
   explicitly deferred (see cpu-rust-v2.md Phase 1 caveat), not
   "Phase 2 will swap".
3. ARR (0x6B): remove the dead `bit7` variable and the duplicated
   `let _ = bit7;`.
4. Clippy (fceux11-core): fix the trivial warnings (unused `run`
   import in tick.rs, "operation has no effect" x4, empty line after
   doc comment, unused test imports/variables) and add a
   `# Safety` section to the FFI functions (or a module-level
   `#[allow(clippy::missing_safety_doc)]` with justification, given
   the module already documents the FFI safety contract in "## Safety").
5. `cargo fmt -p fceux11-core` (benches + lib).
6. `ChangeLog.md`: add Phase 4 (2026-08-21 closeout) and Phase 5
   entries with commit hashes.
7. `fceux11-formats` clippy errors: pre-existing, out of CPU scope -
   document in ChangeLog or defer to a separate cleanup; do not block
   Phase 7 on it unless the CI gate requires clippy.

## 3. Verification (after the fixes)

1. `cargo test -p fceux11-core` - all suites (expect 210 + ~10 new).
2. `cargo clippy -p fceux11-core --all-targets --no-deps` - clean.
3. `cargo fmt --check` - clean.
4. Rebuild `build-rust-cpu` (ON) and `build-off` (OFF).
5. CTest ON: expect 32/34 (same two documented residuals); OFF: 34/34.
6. `savestate_regression_rust_smoke` 0/12; `golden_savestate_test`
   byte-equal; `rom_regression_rust_smoke` stays at the documented
   1/780 residual.
7. Confirm the four fixed opcodes' behavior against the C++ reference
   formulas in the new per-opcode tests.

## 4. Exit criteria for the Phase 7 request

- All MUST-FIX items implemented with triggering-vector tests.
- SHOULD-FIX 1-6 done; 7 documented.
- Entry-standard checks 1-6 above all green.
- Docs updated (this plan completed, cpu-rust-v2.md, ChangeLog.md).
- A separate Phase 7 approval request is then submitted to the user
  (delete C++ CPU, flip FCEUX11_RUST_CPU default, CMake/scripts
  updates) - not started by this plan.

## 5. Execution results (completed 2026-08-21, committed as `31c5b35`)

All MUST-FIX and SHOULD-FIX items are implemented in the working tree
(committed on `wip2.0` as `31c5b35`, 2026-08-21).

### 5.1 MUST-FIX - do_unofficial parity (execute.rs)

1. **SHX 0x9E**: now indexes by **Y** (`abs_y_write`) and applies the
   C++ write-address high-byte replacement
   `((X&(eff_hi+1))<<8)|eff_lo` with store value `X&(eff_hi+1)`.
2. **SHY 0x9C**: keeps the X index and applies the same replacement.
3. **AHX 0x93/0x9F** and **TAS 0x9B**: H is now `base_hi + 1` where
   `base = eff - Y` (recovering the absolute/pointer operand), exactly
   like the C++ `(((A-_Y)>>8)+1)`.
4. **LAS 0xBB**: RMW write-mode (`abs_y_write`, no page-cross extra)
   with two write-backs of the ORIGINAL read value, matching
   `RMW_ABI`.

Tests: 8 new triggering-vector tests in `tests/unofficial.rs` (SHX x2,
SHY x2, AHX-indY page-cross, AHX-absY page-cross, TAS page-cross, LAS
write-back/count) plus 6 per-instruction cycle tests in
`tests/cycle_parity.rs`.

**Correction to the review's measured numbers**: the review stated
"C++ LAS cycles=5 always". The authoritative source (shared `CycTable`
`0xBB` = 4 in both `src/x6502.cpp:365-377` and `decode.rs`, plus the
`RMW_ABY` handler which contains no `ADDCYC`) gives LAS = 4 cycles on
every execution, page-cross or not. The fix and tests assert 4; the
review's "5" was an off-by-one in the measured write-up.

### 5.2 SHOULD-FIX

1. `ffi.rs` `fceux11_cpu_run` doc rewritten (C++-polarity count: budget
   added, decremented per dispatch/instruction, exit on `count <= 0`;
   timestamps advanced by the tick-cycles callback; return value
   informational). `execute.rs::run` header corrected likewise.
2. `decode.rs` Phase-1-era comment now documents the explicit codegen
   deferral (with `cpu-rust-v2.md` reference); the C++-mirroring table
   rows are preserved via `#[rustfmt::skip]`.
3. ARR 0x6B dead `bit7` removed.
4. Clippy clean: `cargo clippy -p fceux11-core --all-targets --no-deps`
   exits 0 (missing-safety-doc handled via module-level allows with
   justification for bus/ffi/tick, per-function `# Safety` for sformat;
   identity-op masks, unused imports/variables, duplicated `#[test]`,
   unnecessary unsafe blocks, test lint noise all fixed).
5. `cargo fmt --check -p fceux11-core` clean.
6. `ChangeLog.md` updated (Phase 4/4.5/5 entries with commit hashes,
   preflight-fix entry, current test counts, known-limits rewrite).
7. `fceux11-formats` clippy errors documented in `ChangeLog.md`;
   deferred to a separate cleanup, not a Phase 7 blocker.

### 5.3 Extra stability fixes found during execution

- **Parallel-test pollution (pre-existing flake)**: the global `TICK_FN`
  slot fired into a tick test's counting callback from unrelated lib
  tests' `step()` calls running on other threads, making
  `cargo test -p fceux11-core` fail ~deterministically in parallel mode.
  Fix: the tick callbacks now fire only on the installing thread
  (`TICK_THREAD_ACTIVE` thread-local in `tick.rs`); production is
  single-threaded install-then-run, so behaviour is unchanged.
- **proptest `nmi_fresh_coalesces_and_fires_once`**: the random PC can
  point at a JAM opcode; step 1 jams the CPU and step 2's NMI dispatch
  is correctly suppressed (C++ `else if(!_jammed)`,
  `src/x6502.cpp:582`), leaving the NMI bit set. The assertion is now
  conditional on `jammed == 0`.

### 5.4 Verification matrix (all green)

| Check | Result |
|---|---|
| `cargo test -p fceux11-core` | **221 PASS** (lib 89, unofficial 80, cycle_parity 23, interrupts 15, opcodes 7, proptest 5, nestest 2) |
| `cargo clippy -p fceux11-core --all-targets --no-deps` | clean (exit 0) |
| `cargo fmt --check -p fceux11-core` | clean |
| CTest `FCEUX11_RUST_CPU=ON` (non-perf) | **32/34** - only `kagami_qa_direct_smoke` (blargg known-fails, C++ baseline fails them too) and `rom_regression_rust_smoke` (documented 1/780 PPU render-timing artifact) |
| CTest `FCEUX11_RUST_CPU=OFF` (non-perf) | **34/34** |
| `golden_savestate_test` | PASS under ON and OFF (byte-equal) |
| `savestate_regression_rust_smoke` | PASS (0/12) under ON |
| `bench_tolerance_test` | PASS under ON and OFF |

The entry standard (section 0, checks 1-6) is met.

## 6. Phase 7 execution results (completed 2026-08-22)

Phase 7 was executed after user approval ("执行phase7", 2026-08-22) on
top of commit `31c5b35` (the preflight fixes above).

### 6.1 Deletions (7 files)

- `src/x6502.cpp` (708 LOC C++ dispatch loop) — deleted.
- `src/x6502.h` — deleted; kept surface migrated to `src/cpu.h`.
- `src/x6502struct.h` — deleted; X6502 struct moved into `src/cpu.h`.
- `src/x6502abbrev.h` — deleted; register macros moved into `src/cpu.h`.
- `src/ops.inc`, `src/ops_table.inc` — deleted.
- `scripts/generate_x6502_dispatch.py` — deleted.
- `scripts/_rebuild_off_target.ps1` — deleted (OFF build no longer exists).

`src/cpu.cpp` was **retained** as the Rust FFI integration facade: its
`#if FCEUX11_RUST_CPU` guards and `#else` C++-CPU branches were
removed, making the Rust path unconditional. (The plan's Phase 7 file
list included cpu.cpp as "the 96-line facade"; that line predates the
Phase 3-6 FFI landing that turned cpu.cpp into the Rust bridge.)

### 6.2 Migrations

- `src/cpu.h` now owns: `X6502` struct, `_A`/`_X`/... register macros,
  flag + IRQ constants, `dendy`/`NTSC_CPU`/`PAL_CPU`, `opsize`/`optype`/
  `opwrite` externs, `X6502_Run` macro (→ `cpu_instance().run`),
  `TriggerNMI/2`, `X6502_IRQBegin/End`, `X6502_DMR/DMW`,
  `X6502_GetOpcodeCycles`, `x6502_nmi_fresh_get/set`, `fceu11_e1_last_pc`.
- `src/cpu.cpp` now defines the migrated helpers (DMR/DMW with Lua hooks,
  IRQ pin control, CycTable + 3 opcode tables, TriggerNMI/2 + e1 probes,
  `fceu11::NMI/IRQ`, `FCEUI_GetIVectors`).
- ~22 consumer files switched `#include "x6502.h"` → `#include "cpu.h"`
  (incl. board common header `src/boards/mapinc_bus.h` and
  `src/input/share.h`).

### 6.3 CMake / build

- `FCEUX11_RUST_CPU` default flipped **ON**; `=OFF` is a configure-time
  fatal error (option retained for build-flag compatibility).
- `src/CMakeLists.txt`: `x6502.cpp` removed from SRC_CORE; Rust staticlib
  link + cbindgen-header dependency unconditional; the
  `-DFCEUX11_RUST_CPU=1` define removed (no source guards remain).
- Root `CMakeLists.txt`: `/wd4244` comment updated (ops_table.inc/x6502.cpp
  gone). `tests/CMakeLists.txt`: cycle-trace harness comment updated
  (ON-only).
- `COPYRIGHT_AUDIT.{csv,md}`: the 4 deleted x6502 file rows removed.

### 6.4 Verification (post-deletion, Rust CPU only)

| Check | Result |
|---|---|
| `cargo test -p fceux11-core` | **221 PASS** (unchanged: lib 89, unofficial 80, cycle_parity 23, interrupts 15, opcodes 7, proptest 5, nestest 2) |
| `cargo clippy -p fceux11-core --all-targets --no-deps` | clean (exit 0) |
| `cargo fmt --check -p fceux11-core` | clean |
| CMake build `build-rust-cpu` (default ON) | clean build |
| CTest (non-perf) | **32/34** - same two documented residuals (`kagami_qa_direct_smoke`, `rom_regression_rust_smoke` 1/780) |
| `golden_savestate_test` | byte-equal |
| `savestate_regression_rust_smoke` | PASS (0/12) |
| `fceux11_pp_frame_diff` / `fceux11_ppu_frame_diff_test` | PASS (0-pixel) |

### 6.5 Gate deviations (documented)

1. The plan gate "8 deleted files" counts cpu.cpp; only 7 files were
   deleted because cpu.cpp is the Rust facade (see §6.1). The 7
   deletions appear in `git log --diff-filter=D`.
2. `ctest` "exits 0" is interpreted per the preflight entry standard
   (§0 item 5): 32/34 with the two documented residuals, which the
   C++ baseline also fails or which are PPU-side (not CPU).
