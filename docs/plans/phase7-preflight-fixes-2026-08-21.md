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
