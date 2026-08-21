# Phase 5 Closeout - unofficial opcode coverage + savestate round-trip (2026-08-21)

**Status:** Closed - Phase 5 (REVISED) of `cpu-rust-v2.md` is complete.
**Branch:** `wip2.0`. **Reference:** `docs/plans/cpu-rust-v2.md` ?4 Phase 5,
`docs/plans/phase4-closeout-2026-08-20.md`.

## 0. Gate

`fceux11_golden_savestate_test` byte-equal under `FCEUX11_RUST_CPU=ON`:
**GREEN** (all 8 golden fixtures byte-identical; `savestate_regression_rust_smoke`
0/12 with the nestest MD5 matching the C++ baseline).

## 1. Unofficial opcode coverage (105/105)

An audit of `tests/unofficial.rs` against the decode table found 21
unofficial opcodes with no direct register-effect test - mostly the
abs / zp,X / abs,Y / abs,X / (ind),Y addressing variants of the
RMW+ALU families (RLA, SRE, RRA, DCP, ISC), plus SAX (ind),X and the
ANC #imm duplicate opcode 2B. 21 new tests were added, one per
opcode, mirroring the existing per-opcode style (memory effect + A/X
register effect + NZC flag assertions):

- ANC: 2B
- RLA: 2F, 37, 3B
- SRE: 53, 57, 5B, 5F
- RRA: 6F, 73, 7B
- SAX: 83
- DCP: CF, D3, D7, DB, DF
- ISC: EF, F3, F7, FB

Coverage now: every opcode marked `official: false` in
`decode.rs` (105) is mentioned in at least one register-effect test.
Note: the plan's "109" is the NESdev canonical unofficial count; the
implementation's decode table marks 105 as unofficial and all 105 are
covered.

Test result: `cargo test -p fceux11-core --test unofficial` = 74 PASS
(53 + 21).

## 2. cpu/snapshot.rs + savestate round-trip

New `src/rust/crates/fceux11-core/src/cpu/snapshot.rs` provides the
plan's snapshot/restore helpers on `CpuState`:

- `snapshot_bytes() -> [u8; 64]` - serializes the 64-byte
  `X6502Layout` (the savestate contract with the C++ `X6502` blob,
  pinned by `state.rs` `offset_of!` asserts and the C++ static_asserts).
- `restore_bytes(&[u8; 64])` - overwrites the blob; clears the
  `nmi_fresh` side flag, matching `fceux11_cpu_restore`.

Four unit tests:
- `snapshot_size_is_64`
- `round_trip_restores_registers_byte_for_byte`
- `snapshot_matches_ffi_copy_semantics` (cross-checks the module helper
  against the FFI's pure `copy_nonoverlapping` path)
- `restore_resumes_execution_deterministically` (interrupt a NOP run
  with snapshot/restore; the restored CPU continues byte-identically
  to an uninterrupted run)

The C++-fixture round-trip (load each golden `.fc0`, drive frames,
save, compare byte-for-byte) is exercised by
`fceux11_golden_savestate_test` (verify + `--compare-layout`) under
`FCEUX11_RUST_CPU=ON` and passes for all 8 fixtures; the frame-4/60
nestest comparisons from the Phase 4 closeout also remain
byte-identical.

## 3. Verification

- `cargo test -p fceux11-core`: 210 PASS (185 + 21 unofficial + 4
  snapshot).
- CTest ON subset (golden_savestate_test, savestate_regression_rust_smoke,
  savestate_core_test, cpu_test): all PASS.
- `rom_regression_rust_smoke` keeps the documented 1/780 PPU
  render-timing residual from the Phase 4 closeout (not a savestate /
  CPU divergence).
- CTest OFF: 34/34 unchanged.

## 4. Files changed

- `src/rust/crates/fceux11-core/tests/unofficial.rs` - 21 new
  per-opcode tests.
- `src/rust/crates/fceux11-core/src/cpu/snapshot.rs` - new module.
- `src/rust/crates/fceux11-core/src/cpu/mod.rs` - register the module.
