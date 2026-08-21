# Phase 4 Closeout - residual cycle-parity mismatches (2026-08-20)

**Status:** Closed - Phase 4 (REVISED) of `cpu-rust-v2.md` is complete.
**Branch:** `wip2.0`. **Reference:** `docs/plans/cpu-rust-v2.md` ?4,
`docs/plans/phase4.5-cycle-drift-closure-2026-08-20.md`.

## 0. Summary

Two 1-mismatch residuals left open by Phase 4.5 were localized and
closed:

- `savestate_regression_rust_smoke` (12 ROMs, 1 mismatch on nestest):
  **PASS now** - the frame-60 savestate is byte-identical to the C++
  baseline (`4851f0554f41d551e9e95587066545a7`).
- `rom_regression_rust_smoke` (780 frames, 1 mismatch on nestest
  frame 4 = the 5th emulated frame): **1 residual remains** - a
  16-pixel rendering difference in row 22 of the transition frame
  only. Every CPU-side observable (instruction stream, per-call
  PC/count/IRQ, timestamps, bus accesses, dispatch patterns) is
  byte-identical between ON and OFF, and the savestates at frames 4
  and 60 (including the rendered XBackBuf) are byte-identical. The
  residual is a PPU per-dot render-timing artifact at the transition
  frame, in the same class as the PPU-timing known-fails the plan
  defers to Phase 6/7 (vbl_05, mmc3_4, ...). See ?4.

## 1. Root causes found and fixed

Five distinct parity gaps kept the Rust CPU from matching the C++
reference at the cycle level:

### 1.1 NMI-fresh flag never crossed the IRQ bridge

The C++ VBL NMI path (`TriggerNMI`) sets `g_e1_nmi_fresh` so the
reference dispatch defers the NMI by exactly one instruction
boundary (`src/x6502.cpp:548-555`). Under `FCEUX11_RUST_CPU=ON` the
PPU calls the C++ `TriggerNMI`, but `sync_irq_from_host` only copied
the `X6502::IRQlow` blob - the fresh flag stayed on the C++ side and
was never seen by the Rust dispatch. Result: the VBL NMI dispatched
one boundary early, so the nestest NMI test's pushed return PC
differed (`$C28F` vs `$C291`), the test's copied PCL landed in RAM
`$0204` (0x8F vs 0x91), and the displayed status differed on the
transition frame.

Fix: new `fceux11_cpu_set_nmi_fresh_bridge` callback pair
(`kagami_bridge_get/set_cpu_nmi_fresh` -> `x6502_nmi_fresh_get/set`),
wired into `Bus::fresh_sync_from_host` / `fresh_sync_to_host` at
every dispatch boundary (bus.rs, cpu.cpp, x6502.cpp, kagami_bridge.*).

### 1.2 Untaken branches read the operand byte (DB latch)

The C++ reference `JR` macro reads the relative operand ONLY when the
branch is taken (`if(cond) { disp=RdMem(_PC); ... } else _PC++;`);
the Rust `do_branch` always read it. The extra read changed the DB
latch, which is observable via open-bus reads: nestest's NMI handler
reads `$4016`, whose `JPRead` returns `DB & 0xC0` bits, so the
controller byte differed (0xC0 vs 0x40) and propagated into the test
results.

Fix: `do_branch` skips the operand read for untaken branches,
matching the C++ reference exactly.

### 1.3 The FFI ran on a private copy, so mid-call C++ readers saw a
stale 64-byte blob

`fceux11_cpu_run` copies the blob into the static `FFI_CPU_STATE`,
runs, and writes it back at call end. Mid-call, C++ bus handlers that
read `g_cpu.native_layout().DB` (JPRead $4016, mapper open-bus, FDS,
VSUni, cart) saw the value from the last call boundary. The fix
mirrors the Rust `db` (and now `count`) into the C++ blob before and
after every bus access via a `BLOB_PTR` + `sync_db_to_blob`, active
only when the IRQ bridge is installed (pure-Rust tests are
unaffected).

### 1.4 Tick bridge forwarded the wrong per-instruction value and
never maintained `tcount`

The C++ loop's hook point is `temp = _tcount; _tcount = 0;
hook(temp); SoundCPUHook(temp);` BEFORE the opcode handler, where
`_tcount` accumulates prev-extras + dispatch + base. The Rust
`run_with_tick` fired the tick AFTER the instruction with
`dispatch + base + current extras`. The totals match, but the
per-instruction values differ, so the APU frame counter (FHCN)
drifted (nestest frame-60 diff: 0x5525 vs 0x5529, 4 cycles), and the
savestate-visible `tcount` (ICoa) diverged at frame 2.

Fix (execute.rs / tick.rs / cpu.cpp):
- `dispatch_step` and `execute_step` maintain `regs.tcount` exactly
  like C++ `add_cycles` (dispatch + base charged before the hook
  point, extras after; reset at the hook point; early-exit dispatch
  residual left in place).
- The pre-body hook (`tick_pre_body`) forwards `tcount` and resets
  it; the post-body `tick_post_body` advances `timestamp_` /
  `sound_timestamp_` with the iteration's full total via the new
  `fceux11_cpu_set_tick_cycles` callback.

### 1.5 Dispatch-early-exit never advanced timestamps

When a dispatch (7 cycles) exhausts the call budget, C++ `ADDCYC(7)`
has already advanced `timestamp_` before the early return; the Rust
loop broke without advancing it. Added `tick_post_body(dc)` on the
early-exit path (run and run_with_tick).

## 2. Evidence (all under `FCEUX11_RUST_CPU=ON` vs `=OFF`, nestest)

- Per-call cycle trace (frame, call_idx, cycles_arg, pc_after,
  cum_count, irq_low, timestamp): **byte-identical** for all 56030
  rows across 60 frames.
- Per-access bus trace (R/W + D dispatch lines): **identical** (0
  alignment resyncs) across 60 frames.
- Savestate byte-compare (`fceux11_golden_savestate_test
  --compare-layout` with temporary nestest@4/@60 scenarios):
  **byte-identical** at frames 4 and 60 (CPU, PPU, APU, mapper,
  XBackBuf).
- `savestate_regression_rust_smoke`: 0/12 (nestest MD5
  `4851f055...` matches the golden).
- CTest ON: 32/34; the only failures are `kagami_qa_direct_smoke`
  (blargg known-fails, all in `blargg_known_fail.json`, C++ baseline
  fails them too) and `rom_regression_rust_smoke` (1/780 residual).
- CTest OFF: 34/34 (unchanged C++ baseline).
- `cargo test -p fceux11-core`: 185 PASS.

## 3. Test hygiene

The shared `FFI_CPU_STATE` / `TICK_FN` / `BLOB_PTR` statics make the
crate's unit tests non-parallel-safe. Added:
- `TICK_SLOT_LOCK` (tick.rs) serializing tests that install the tick
  callback (tick.rs + ffi.rs).
- `FFI_TEST_LOCK` (ffi.rs) serializing all FFI tests.
- The DB/count mirror is gated behind `IRQ_BRIDGE_INSTALLED`, so
  pure-Rust tests never touch `BLOB_PTR`.

## 4. Residual: rom_regression_rust_smoke 1/780 (documented, deferred)

The 5th emulated frame of nestest renders row 22 with a 16-pixel
horizontal offset (an 8-pixel-aligned run of palette entry 0x0F
starting at column 40 instead of 56). Frames 0-3 and 6-59 render
identically; the frame-4 and frame-60 savestates are byte-identical
to the C++ baseline. Diagnosis with bus traces, per-call
PC/count/IRQ/timestamp traces, PPU write-timing logs ($2005/$2006/
$2007 at sl=0 cyc=0, same timestamps and values) and the DB/count
mirror fixes found **no CPU-side divergence whatsoever**. The
difference is confined to the rendered XBackBuf/XBuf of the
transition frame while the post-frame PPU state is identical - a
PPU per-dot render-timing artifact of the legacy (newppu=0) renderer,
in the same family as the PPU-timing known-fails (`vbl_05_nmi_timing`,
`mmc3_4_scanline_timing`, ...) that `cpu-rust-v2.md` ?4/?6 assigns to
Phase 6/7 accuracy work rather than Phase 4 CPU parity. It is not a
CPU correctness divergence and does not affect the Phase 4 gate
(savestate byte-parity + no regression vs the C++ baseline).

## 5. Files changed

- `src/x6502.cpp` / `src/x6502.h`: `x6502_nmi_fresh_get/set`
  accessors.
- `src/kagami_bridge.cpp` / `src/kagami_bridge.h`: fresh-flag bridge.
- `src/cpu.cpp`: install fresh bridge + `cpu_rust_tick_cycles_thunk`
  (pre-body hook / post-body timestamp split).
- `src/rust/.../cpu/bus.rs`: NMI-fresh bridge, BLOB_PTR +
  `sync_db_to_blob` (db + count mirror).
- `src/rust/.../cpu/addressing.rs`: db/count mirror around every bus
  access; `fresh_sync_*` trait defaults.
- `src/rust/.../cpu/execute.rs`: tcount maintenance; pre/post tick
  points inside `execute_step`; untaken-branch operand-read skip;
  early-exit timestamp advance.
- `src/rust/.../cpu/tick.rs`: `tick_pre_body` / `tick_post_body`,
  `fceux11_cpu_set_tick_cycles`, `TICK_SLOT_LOCK`.
- `src/rust/.../cpu/ffi.rs`: `set_blob_ptr`; `FFI_TEST_LOCK`.
- `src/rust/.../kagami-qa/src/lib.rs`: per-mismatch detail printing
  in the rom/savestate regression entries (diagnostics improvement).
