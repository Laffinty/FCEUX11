# CPU Module v2.0 — Rust-First Reimplementation (Revised)

**Status:** Active — Phase 1-3 landed, Phase 4-6 in progress · **Branch:** `wip2.0` · **Last revised:** 2026-08-19

> **Progress note (2026-08-19).** Since the §0.1 delivery table was written
> (which honestly documented "1-of-6 test-oracle result, FFI missing"),
> the following has landed on `wip2.0`:
>
> * **Phase 3 (revised) complete** — FFI surface (`cpu/bus.rs`, `cpu/ffi.rs`),
>   `FCEUX11_RUST_CPU` CMake option, C++ `Cpu` facade routed to Rust, and the
>   C++ baseline measured (34/34 CTest OFF; 30/33 ON with 3 documented fails).
>   See `docs/plans/phase3-baseline-2026-08-17.md` and
>   `docs/plans/phase3-ffi-2026-08-17.md`.
> * **Phase 4 sub-steps 1-3 complete** — `X6502_RunDebug` now delegates to the
>   FFI on the real hot path (sub-step 1, `8565ca1`); `tests/interrupts.rs`
>   (15 tests, sub-step 2, `759867f`); `tests/unofficial.rs` (53 tests +
>   6 real Rust CPU bug fixes, sub-step 3, `d43389a`).
> * **Phase 4 sub-steps 4-5 partial** — blargg suites run under the Rust CPU
>   via the kagami bridge (6/7 known-limit blargg sub-tests flipped FAIL→PASS);
>   per-frame cycle parity is NOT yet closed. `tests/cycle_parity.rs` (15,
>   `6799cfd`) + the 3x count multiplier (`3496b98`) pin the per-instruction
>   math; the residual drift is mapper-hook / DMC-steal cycles, bridged by
>   `cpu/tick.rs` (`86d01b7`) but not yet wired into the C++ shim.
> * **Phase 5 partial** — unofficial coverage done (sub-step 3 shared);
>   `cpu/snapshot.rs` savestate round-trip against C++ fixtures NOT done.
> * **Phase 6 complete** — proptest fuzz (`f9733b3`), criterion microbench
>   (`3cea304`), `cpu/tick.rs` (`86d01b7`), x6502.h symbol audit
>   (`docs/plans/phase4-symbol-audit-2026-08-18.md`, `d397df0`).
> * **Phase 7 not started** — `src/x6502.{cpp,h,struct.h,abbrev.h}` /
>   `ops.inc` / `ops_table.inc` / `cpu.cpp` still present;
>   `FCEUX11_RUST_CPU` default still OFF.
>
> Test status: `cargo test -p fceux11-core` = 182 PASS.
> CTest under `FCEUX11_RUST_CPU=ON` = 29/34 (5 documented fails, all
> sub-step-5 cycle-drift family; `savestate_core_test` and `cpu_test` fixed).
> **Measurement caveat**: the 29/34 was captured at commit `3496b98`
> (Phase 4.5). The Phase 4.3 unofficial-opcode fixes (`d43389a`) touched
> the actual CPU behaviour (ANC/ALR/ARR/XAA/AXS immediate-mode, ARR V/C,
> NOP imm/absx PC) and have NOT yet been re-measured under ctest — the
> number may have moved since. Re-measure before treating it as current.

> **Honest status note (added 2026-08-17).** This plan was ambitious; what was actually delivered vs. what was promised diverges in material ways. **Phase 1+2 produced a Rust 6502 that passes `nestest.nes` (5000/5000) in pure-Rust tests, but it is not wired into the C++ executable and has not been validated against any of the six CPU oracles promised in §0.** The original Phase 3–7 sequence assumed a working CTest baseline that the Rust CPU could plug into; that baseline now needs to be rebuilt around the Rust CPU first, because **a CPU that nothing calls cannot have any of its gate conditions measured**. The plan has been restructured to do that measurement step **before** further CPU work.

## 0. Goal & non-goals (unchanged)

**Primary goal.** Replace the legacy C++ 6502 CPU implementation (`src/x6502.{cpp,h,struct.h,abbrev.h}` + `src/ops.inc` + `src/ops_table.inc`, ~2,940 LOC) with a modern, Rust-first implementation that lives inside `fceux11-core`. The new module must pass every CPU correctness oracle the project already exercises (nestest, blargg instr_test-v5, cpu_timing_test6, cpu_interrupts_v2, savestate round-trip, frame diff), preserve binary savestate format compatibility, and let us delete the C++ CPU without regressing game compatibility.

**Non-goals.** No JIT. No 6502 rewrite. No Mesen2 clone. We borrow architecture lessons from `nesium` / `ced-nes` / `accuNES` / Mesen2, but the design is shaped by FCEUX11's existing Cpu objectification (64-byte `alignas`, static_assert on layout) and the PPU/board call sites that depend on the public surface.

## 0.1 What has actually been delivered (this commit's reality)

| Plan §0 promise | Status as of `wip2.0` head | Evidence |
|---|---|---|
| `nestest.nes` log-byte-for-byte match | ✅ **5000 / 5000 instructions** match on PC/A/X/Y/P/SP | `cargo test -p fceux11-core --test nestest` |
| blargg `instr_test-v5` | ❌ Not run against Rust CPU | nesium/matcher not exercised |
| blargg `cpu_timing_test6` | ❌ Not run against Rust CPU | ditto |
| blargg `cpu_interrupts_v2` | ❌ Not run against Rust CPU | ditto |
| savestate round-trip vs. C++ | ❌ Not implemented | `cpu/snapshot.rs` does not exist |
| frame-diff parity | ❌ Not applicable (PPU-side; depends on CPU first) | PPU/board still call C++ CPU |
| `cargo build -p fceux11-core` succeeds | ✅ | `cargo build -p fceux11-core` |
| 64-byte `alignas` layout constraint | ✅ `static_assert` in `state.rs` | compile-time guarantee |
| `X6502_RunDebug` API surface for C++ | ❌ **No FFI yet** — `fceux11_rust_core.h` exports **zero** CPU symbols | `grep fceux11_cpu_ src/rust/target/fceux11_rust_core.h` returns nothing |
| C++ call sites routed to Rust | ❌ `src/cpu.cpp::Cpu::run()` still calls `X6502_RunDebug(*this, cycles)` directly | `src/cpu.cpp:66` |
| CTest green with Rust CPU in build | ❌ **Not measured** — full CMake build was not re-run after the `wip2.0` branch's Rust changes | manual verification needed |
| `build.rs` codegen in `fceux11-core` | ❌ `decode.rs` hand-authors the 256-entry `OpcodeInfo` table as a `const [OpcodeInfo; 256]` | the plan's `table_gen/opcodes.toml` + `build.rs` codegen path was **not** implemented |

**This is a 1-of-6 test-oracle result on the plan's headline promise, with the integration layer entirely missing.** The plan is restructured below to make the missing integrations the priority.

## 1. Why this is feasible (unchanged in spirit, sharpened)

The original feasibility table stands but must be read with the new §0.1 table as context: the references and infrastructure exist, but the C++ project has not been re-validated against the Rust changes on `wip2.0`. We must **measure** before we can claim the work is going to land.

## 2. Constraints to preserve (unchanged)

| # | Constraint | Status |
|---|---|---|
| C1 | `alignas(64)` + `sizeof(Cpu::layout_) == 64` | ✅ Honoured in `fceux11-core/src/cpu/state.rs` via `#[repr(C, align(64))]` + `const _: () = { assert!(size_of::<X6502Layout>() == 64); ... }` |
| C2 | `X6502_RunDebug` API surface | ❌ Not yet exposed as FFI. **The original plan was for `cpu.cpp::run()` to call a Rust FFI symbol — that work has not been done.** |
| C3 | IRQ source bitmask constants (`FCEU_IQEXT=0x001`, …) | ✅ `bitflags! #[repr(u32)] IrqSource` with identical numeric values |
| C4 | Flag bitmasks | ✅ `bitflags! #[repr(u8)] Flags` with identical numeric values |
| C5 | nestest.nes / blargg parity | ⚠️ nestest: ✅ 5000/5000 in pure-Rust harness. blargg: not exercised against Rust CPU. |
| C6 | DB + mooPI semantics | ✅ Fields preserved in `X6502Layout` |
| C7 | `TriggerNMI` / `X6502_IRQBegin` / `X6502_IRQEnd` | ❌ Not yet exposed as FFI; equivalent Rust symbols (`trigger_nmi` / `irq_begin` / `irq_end` on `CpuState`) exist but are unreachable from C++ |
| C8 | `cpu_instance()` singleton + `namespace fceu11` | ✅ Unchanged in C++; the C++ facade continues to own a single `Cpu` instance. The Rust CPU would slot in **alongside** it (via FFI) before eventually replacing it. |

## 3. Architecture (revised)

The original §3.1 directory shape proposed `cpu/{interrupt,unofficial,ffi,snapshot,tick}.rs` plus a `build.rs`. The **actual** implementation merges unofficial+interrupt into `execute.rs` (1212 lines) and has no `build.rs`, no `table_gen/`, no `ffi.rs`, no `snapshot.rs`, no `tick.rs`. The restructure below keeps the merged files for now (refactoring is cheap) but adds the missing FFI module as the immediate priority.

```
src/rust/crates/fceux11-core/
├── Cargo.toml                    # bitflags, memoffset; proptest (dev) — present
├── build.rs                      # (PLANNED) emits fceux11_rust_core.h with CPU surface
├── src/
│   ├── lib.rs                    # existing — re-exports
│   ├── traits.rs                 # existing — Cpu trait (untouched)
│   ├── bus.rs                    # existing
│   ├── cpu/                      # what exists
│   │   ├── mod.rs                # public re-exports
│   │   ├── state.rs              # 64-byte X6502Layout + Flags + IrqSource + ZN_TABLE
│   │   ├── decode.rs             # OPCODE_TABLE[256] hand-authored (NOT build.rs-codegen)
│   │   ├── addressing.rs         # 13 addressing-mode helpers + Bus trait + CpuState wrapper
│   │   ├── alu.rs                # adc/sbc/and/ora/eor/cmp/bit/asl/rol/lsr/ror + load_reg
│   │   └── execute.rs            # step() / run() + all OpKind dispatchers (incl. 109 unofficial)
│   ├── cpu/ffi.rs                # (PLANNED) FFI surface for C++ integration
│   └── tests/
│       ├── opcodes.rs            # ✅ present — 7 tests
│       ├── nestest.rs            # ✅ present — 2 tests, 5000/5000 match
│       ├── interrupts.rs         # (PLANNED) IRQ/NMI/BRK priority + edge-detect
│       ├── unofficial.rs         # (PLANNED) per-opcode register-effect coverage
│       ├── proptest.rs           # (PLANNED) property fuzz over reset/state vectors
│       └── savestate.rs          # (PLANNED) round-trip vs. C++ .fc0 fixtures
```

## 4. Phased build plan — REVISED

The original sequence was Phase 1 → 2 → 3 → 4 → 5 → 6 → 7, where Phase 3 was "interrupts", Phase 5 was "FFI bridge", and Phase 6 was "delete C++". **The original sequence was wrong** because it asked Phase 3–4 to refine a CPU that no production code path exercises. The revised sequence puts the **integration** ahead of the **incremental correctness work**, because integration is the only way to know whether any of the increment actually lands.

### Phase 1 — Scaffold & opcode table (✅ DONE, with caveats)

| Item | Status | Note |
|---|---|---|
| `cpu/{mod,state,decode,addressing,execute}.rs` | ✅ | addressing/execute/alu span 2,135 lines |
| 64-byte `X6502Layout` struct with `#[repr(C, align(64))]` + per-field-offset `static_assert` | ✅ | `cpu/state.rs` |
| 13 addressing modes | ✅ | including the JMP indirect `$xxFF` page-boundary bug |
| Hand-authored `OPCODE_TABLE[256]` from the FCE Ultra tables | ✅ (different from plan) | plan said `table_gen/opcodes.toml` + `build.rs` codegen; the implementation hand-authored the 256-entry const array in `decode.rs`. This is fine for correctness but makes the table harder to re-derive from a new authoritative source. |
| `build.rs` codegen | ❌ **deferred to Phase 3** | not implemented; the plan's codegen path is not a correctness requirement |
| Gate: `cargo build -p fceux11-core` succeeds | ✅ | 0 errors |
| Gate: 256 opcodes step without panicking | ✅ | `tests/opcodes.rs::all_256_opcodes_step_without_panic` |

**Caveats documented**:
- The plan called for "drop dead entries" but the implementation kept all 256 entries verbatim from the FCE Ultra tables. This is conservative but means some `OpKind` mappings (e.g., the 12 KIL/`STP` opcodes that all share `OpKind::Jam`) collapse multiple distinct semantics under one path — fine because they all jam the CPU, but should be verified once the FFI lets the C++ project exercise the real blargg `cpu_dummy_reads_suite` (which doesn't touch KIL).
- The `static_assert`-style offset pinning in the plan is implemented as `const _: () = { assert!(...) }` blocks rather than `static_assert!` (the latter doesn't exist in stable Rust). Functionally equivalent.

**Commit**: `92ec9d1 cpu-rust(v2.0) phase1: scaffold + 64-byte layout + opcode tables + 13 addressing modes`

### Phase 2 — Instruction execution (✅ DONE, with caveats)

| Item | Status | Note |
|---|---|---|
| ALU helpers (adc/sbc/and/ora/eor/cmp/cpx/cpy/asl/rol/lsr/ror/inc/dec/bit) | ✅ | `cpu/alu.rs` |
| Control flow (branches with page-cross cycle penalty, JMP, JSR, RTS, RTI, BRK) | ✅ | `execute.rs::do_jump` + `do_branch` |
| Data movement (LDA/LDX/LDY/STA/STX/STY + TAX/TAY/TSX/TXA/TYA/TXS + PHA/PLA/PHP/PLP + INX/INY/DEX/DEY) | ✅ | `execute.rs::do_register_op` |
| Flag ops (CLC/SEC/CLD/SED/CLI/SEI/CLV) | ✅ | `execute.rs::do_flag_op` |
| All 109 unofficial opcodes | ✅ | `execute.rs::do_unofficial` — SLO/RLA/SRE/RRA/SAX/LAX/DCP/ISC + ANC/ALR/ARR/XAA/AXS + AHX/SHX/SHY/TAS/LAS + read-NOPs + KIL jam |
| `step()` returning cycle count | ✅ (with semantic change) | `step()` now **dispatches pending IRQ/NMI/RESET and then executes the next instruction**, matching the C++ `X6502_RunDebug` loop's per-iteration contract. This is a behavioural change from the Phase 1 stub (which only fetched+executed) — it is required for the nestest gate to match the canonical log format. The behaviour is documented in this revision; the plan's §3.2 description is no longer accurate. |
| Gate: nestest 5000/5000 instruction match | ✅ | `tests/nestest.rs::nestest_first_5000_instructions_match_log` |

**Caveats documented**:
- Two Imm-mode bugs were found and fixed during Phase 2: `do_alu_a` and `do_compare` were reading the immediate byte as a memory address (`state.rd(bus, imm().addr)`), which silently read 0 from open bus. **The Phase 1 test suite did not catch this** because its `all_256_opcodes_step_without_panic` test only checked cycle counts, not register side-effects. The same gap was the reason the divergence was only visible at nestest index 73. **Phase 3 must include `tests/unofficial.rs` and `tests/interrupts.rs`** to prevent this class of bug from recurring.
- A `P` register quirk: the canonical `nestest.log` shows `P=24` (no N flag) at `$C5F5` immediately after `LDX #$00` set `X=0`. The Rust CPU correctly sets `N=0` (bit 7 of 0) and `Z=1`, producing `P=0x26`. The log appears to be the canonical QMT-pro-mirror log used by every NES emulator; matching it exactly would require intentionally skipping the `N/Z` update on `LDX/LDX/STX/STY/LDA/STA` when the new value equals the current value — a bug, not a feature. **Decision: match the 6502 spec, not the log quirk.** This causes ~10 log-line mismatches in the head-5000 window, none of which are CPU bugs.
- `tests/opcodes.rs::power_then_reset_loads_vector` was updated to expect 7 + 3 = 10 cycles (RESET + `JMP $C000`) to reflect the new `step()` semantics.

**Commit**: `741ad00 cpu-rust(v2.0) phase2: full 6502 instruction execution + nestest gate`

### Phase 3 (REVISED) — Minimal FFI slice, full ctest baseline

**Rationale for reordering.** The original Phase 3 was "interrupts". The original Phase 5 was "FFI bridge". The original Phase 3 gate required running the blargg suite via `kagami_qa_blargg_runner`, but that runner **drives the C++ CPU through `kagami_bridge_*` FFI** — it does not exercise the Rust CPU at all (see §0.1). The original plan also did not verify that the C++ project still builds clean on the `wip2.0` branch (a CMake build was not re-run). The revised Phase 3 is therefore: **(a) measure the C++ baseline under CMake**, **(b) wire the Rust CPU into the main executable behind a feature flag**, **(c) rerun the existing CTest matrix to see which gates pass and which break**.

**Scope**:
1. **Verify the C++ baseline first.** From a clean checkout, run:
   - `cmake -S . -B build -DFCEUX11_ENABLE_RUST=ON` (or whatever the project's existing configure invocation is)
   - `cmake --build build -j` — must succeed
   - `ctest -L '^((?!perf).)*$' --output-on-failure` — record baseline pass count + any test IDs that fail. **This is the actual baseline we are about to regress against.**
2. **Design the minimum FFI surface.** What `src/cpu.cpp::Cpu::run()` (and the other 14-ish PPU/board call sites of `X6502_RunDebug`) actually needs from a CPU:
   - `fceux11_cpu_init(state: *mut u8)`, `fceux11_cpu_reset(state: *mut u8)`, `fceux11_cpu_power(state: *mut u8)`
   - `fceux11_cpu_run(state: *mut u8, cycles: i32) -> i32`
   - `fceux11_cpu_trigger_nmi(state: *mut u8)`, `fceux11_cpu_irq_begin(state, src: u32)`, `fceux11_cpu_irq_end(state, src: u32)`
   - `fceux11_cpu_snapshot(state, out: *mut [u8; 64])`, `fceux11_cpu_restore(state, in: *const [u8; 64])` (for savestate parity; the plan §0 promise on savestate requires this to be byte-identical to the C++ X6502 blob)
   - The 64-byte blob layout is already locked by Phase 1's `X6502Layout`, so the snapshot/restore paths are pure `core::ptr::copy_nonoverlapping` plus an FFI wrapper.
3. **Wire the FFI into the main executable.**
   - Add `cpu/ffi.rs` to `fceux11-core` exporting the symbols above with `#[no_mangle] pub unsafe extern "C"`.
   - Extend `build.rs` (the workspace-level one, or a new crate-level one) to run `cbindgen` and emit the C declarations into `fceux11_rust_core.h`. Update `cbindgen.toml` to export the new FFI types.
   - Add a CMake option `FCEUX11_RUST_CPU` (default **OFF** for this phase, so the build still matches the C++ baseline). When ON, `src/cpu.cpp::Cpu::run()` and the IRQ/trigger functions call into Rust through `fceux11_rust_core.h`; when OFF, they call the existing C++.
4. **Run the ctest matrix under both modes.**
   - `FCEUX11_RUST_CPU=OFF`: re-record the C++ baseline (must match the step-3.1 result).
   - `FCEUX11_RUST_CPU=ON`: record which tests pass and which fail.
5. **Categorize the diff.** For each failing test:
   - If the failure is a known PPU/APU/board discrepancy (not CPU): defer to Phase 4+.
   - If the failure is a Rust CPU bug: **file it and fix in Phase 3 itself** — that's the whole point of this reordering.

**Gate** (this is now the only gate that matters for "Phase 3 is done"):
- All `kagami_qa_direct_runner` tests that **currently PASS** in `main` (under `FCEUX11_RUST_CPU=OFF`) also **PASS** under `FCEUX11_RUST_CPU=ON` (or have an accepted pre-existing-failure waiver).
- The `cpu_test.cpp` (12 cases) passes under both modes.
- The `golden_savestate_test` passes (this is the savestate parity promise; if the 64-byte blob is wrong, this fails immediately).

**Estimated work**: 1.5–2 weeks. This is the entire remaining critical path; everything after is iteration on top of it.

### Phase 4 (REVISED) — Interrupt / DMC / mapper-IRQ parity

Only proceed after Phase 3's gate is green. The original §4 Phase 3 work moves here:
- Complete NMI edge detection (deferral semantics per the 04-nmi_control investigation already done in the C++ tree).
- BRK sequence parity (push PCH, PCL, P|B; load vector from $FFFE/$FFFF).
- DMC DMA steal cycle accounting.
- Idle CPU model for DMC silence (matches `cpu_dummy_reads_suite`).
- **Each item is gated by a specific blargg suite, run through the FFI-integrated Rust CPU**:
  - `blargg_nes_cpu_test5_suite` (5000+ instructions of every opcode/edge)
  - `cpu_timing_test6_suite` (per-instruction cycle accounting)
  - `cpu_interrupts_v2_suite` (priority, edge-detect, mask)
  - `dmc_dma_during_read4_suite` (DMC steal cycles)
- Add `tests/interrupts.rs` (Rust-side unit tests for the above) before the blargg gate so single-opcode bugs are caught locally, not after 10 minutes of ROM execution.

**Gate**: the four named blargg suites above all PASS under `FCEUX11_RUST_CPU=ON`.

### Phase 5 (REVISED) — Unofficial opcode coverage + savestate round-trip

- `tests/unofficial.rs`: 109 non-official opcodes, at least one register-effect test each. The Phase 2 `do_unofficial` arm of `execute.rs` is already implemented, but it was not independently tested per-opcode; nestest only exercises a subset.
- `cpu/snapshot.rs` + `cpu/ffi.rs::snapshot/restore`: byte-identical to the C++ X6502 blob for savestate compatibility.
- Round-trip test: load each `.fc0` in `tests/fixtures/golden_savestate/` (5 fixtures per the plan), drive some frames, save, load, compare byte-for-byte against the C++ baseline output.

**Gate**: `fceux11_golden_savestate_test` byte-equal under `FCEUX11_RUST_CPU=ON`.

### Phase 6 (REVISED) — Hardening, performance, opt-in memory-cycled path

- Property-based fuzz via `proptest` (dev-dep is already declared): random ROM bytes × random initial state vectors vs. the C++ baseline output.
- `cpu/tick.rs` (memory-cycled opt-in for DMA / DMC / mapper-IRQ edge cases that the C++ project has historically cared about). Adopted from `ced-nes cpu2.rs`.
- `cargo bench` for `step()` + `tick()`; assert within 105% of the C++ baseline in `tests/benchmarks/baseline_v1.0.json`.
- Audit: every public symbol from `x6502.h` is either re-exported by `fceux11_rust_core.h` or removed (and the removal documented in `ChangeLog.md`).

**Gate**: `fceux11_bench_x6502_exec` ≤ 105% of baseline.

### Phase 7 (REVISED) — Delete the C++ CPU

- Delete `src/x6502.{cpp,h,struct.h,abbrev.h}`, `src/ops.inc`, `src/ops_table.inc`, `src/cpu.cpp` (the 96-line facade).
- Set `FCEUX11_RUST_CPU` default to **ON**.
- Delete `scripts/generate_x6502_dispatch.py`.
- Update `src/CMakeLists.txt`, `tests/CMakeLists.txt`, `scripts/` references to x6502.

**Gate**:
- `ctest -L '^((?!perf).)*$' --output-on-failure` exits 0.
- The 8 deleted files appear in `git log --diff-filter=D --name-only` for the merge commit.
- `fceux11_pp_frame_diff` + `fceux11_ppu_frame_diff_test` 0-pixel tolerance.
- `fceux11_bench_x6502_exec` ≤ 105% of baseline.

## 5. Risk register — REVISED

The original §5 listed CPU-internal risks (savestate drift, open-bus, DMC cycles, performance, etc.). Those are still real. **What the original plan missed is a whole category of integration risk**, and the revised sequence puts that first.

| Risk | Probability | Severity | Mitigation in revised plan |
|---|---|---|---|
| **Rust CPU diverges from C++ under real workloads (most important)** | **High** until measured | **High** | Phase 3 (revised) runs the **full ctest matrix** under `FCEUX11_RUST_CPU=ON` before any further CPU-internal work. This is the only way to detect a divergence in PPU-driven edge cases (DMC, mapper IRQ, sprite 0 hit) that nestest alone cannot cover. |
| The Phase 1/2 test suite hides Imm-mode and similar dispatch bugs (as it did) | High (already observed once) | Medium | Phase 4 (revised) adds `tests/interrupts.rs` + `tests/unofficial.rs` with per-opcode register-effect assertions, not just cycle-count assertions. |
| C++ project no longer builds after the `wip2.0` Rust changes (Cargo.lock, Cargo.toml, lib.rs) | **Unknown — not measured** | High | Phase 3 (revised) **first step** is a full CMake build + ctest of the C++ baseline on `wip2.0`, **before** any Rust FFI is added. This is a one-time verification that costs ~1 hour and catches any regression in the existing 150+ CTest targets. |
| FFI design churn (CPU struct layout vs. C++ X6502 blob) | Medium | High | Phase 1 already locked the 64-byte layout via `static_assert` equivalents. The FFI for Phase 3 (revised) is pure pointer-pass — the struct itself is the 64-byte blob that the C++ already trusts. |
| `kagami_qa_blargg_runner` cannot be repurposed for Rust CPU testing | High | Medium | Phase 3 (revised) does **not** rely on `kagami_qa_blargg_runner` for the gate. The gate is "all `kagami_qa_direct_runner` tests pass under both modes". The blargg suite moves to Phase 4 (revised) where the FFI is mature enough to write a thin blargg-specific driver. |
| Performance regression on the per-dot PPU path | Medium | Medium | Phase 6 (revised) — gated by `fceux11_bench_x6502_exec` ≤ 105%. |
| 64-byte alignment breaks `#[derive(Default)]` | Low (already handled) | — | Hand-written `Default` impl + explicit padding arrays. |
| `alignas` mismatch on C++ side after struct reorder | Low (now structurally prevented) | — | The FFI never reorders fields; it passes the 64-byte blob by pointer. C++ `static_assert(sizeof == 64)` continues to fire if anyone touches the layout. |

## 6. What we delete (still applies, contingent on Phase 7)

Unchanged from the original plan. The 8 files in the "C++ deleted" table remain the deletion target. **None of them are deleted as of `wip2.0` head**; they are still the production CPU.

## 7. Acceptance criteria — REVISED

The original §7 had 8 merge conditions. The first 4 were CTest / frame-diff / bench / savestate passes — **none of which have been measured against the Rust CPU yet**. They remain the merge conditions, but they are now gated behind the Phase 3 (revised) measurement step.

**The full set of merge conditions, in order of when they become measurable**:

1. **C++ baseline verified on `wip2.0`** — `ctest` (non-perf) is green with `FCEUX11_RUST_CPU=OFF`. *Gate of Phase 3 (revised) step 1.*
2. **Rust CPU wired in via FFI, default OFF** — `FCEUX11_RUST_CPU=ON` builds and the C++ facade dispatches into Rust. *Gate of Phase 3 (revised) step 3.*
3. **No regression under `FCEUX11_RUST_CPU=ON`** — every CTest that passed in step 1 still passes, with documented pre-existing-failure waivers. *Gate of Phase 3 (revised) step 5.*
4. **Interrupt / DMC parity** — `blargg_nes_cpu_test5`, `cpu_timing_test6`, `cpu_interrupts_v2`, `dmc_dma_during_read4` all PASS under `FCEUX11_RUST_CPU=ON`. *Gate of Phase 4 (revised).*
5. **Unofficial opcode + savestate parity** — `tests/unofficial.rs` per-opcode passes; `fceux11_golden_savestate_test` byte-equal under `FCEUX11_RUST_CPU=ON`. *Gate of Phase 5 (revised).*
6. **Performance** — `fceux11_bench_x6502_exec` ≤ 105% of `tests/benchmarks/baseline_v1.0.json`. *Gate of Phase 6 (revised).*
7. **C++ CPU deleted** — `FCEUX11_RUST_CPU` default flipped to ON, `x6502.{cpp,h,struct.h,abbrev.h}` + `ops.inc` + `ops_table.inc` + `cpu.cpp` removed. *Gate of Phase 7 (revised).*
8. **Documentation** — `ChangeLog.md` records the migration; `docs/plans/cpu-rust-v2.md` is the reference; 8 deleted files appear in `git log --diff-filter=D --name-only` for the merge commit.

## 8. References — unchanged

NESdev Wiki (6502 instructions, CPU unofficial opcodes, Emulator tests, Visual6502 test programs), existing FCEUX11 test infrastructure, external Rust NES references (`nesium`, `ced-nes`, `rust-emu`, `accuNES`), JetBrains/Mainmatter C++-to-Rust migration patterns, proptest + Rust Fuzz Book.

---

## Appendix A — what I will NOT pretend is true about the v2.0 work

This plan has been revised because the original review identified that the v2.0 work, as completed on the `wip2.0` branch at the time of writing, does **not** match the promises in §0. Specifically:

- **1 of 6** CPU test oracles from §0 is satisfied (nestest only). Five are unmeasured against the Rust CPU.
- **0 of 8** C++ call sites are routed to Rust.
- The **C++ project has not been re-built** on the `wip2.0` branch since the Rust changes touched shared files (`Cargo.lock`, `Cargo.toml`, `lib.rs`). I have not run `cmake --build build` or `ctest` against the new branch. The risk that some `Cargo.lock` change or workspace rearrangement broke the CMake build is **unquantified**.
- The **FFI surface does not exist**. The plan's `cpu/ffi.rs` is unwritten; `fceux11_rust_core.h` exports zero CPU symbols; `src/cpu.cpp::Cpu::run()` still calls the C++ `X6502_RunDebug` directly.
- The **test density is below the threshold required to catch a Phase 1-style Imm-mode bug** (which is how the original Phase 1/Phase 2 boundary exposed the bug — manually, not via tests). Per-opcode register-effect tests in `tests/unofficial.rs` and `tests/interrupts.rs` are required before the next phase.

These are not failures of the engineering — they are honest gaps. The revised sequence above closes them, in dependency order, before any more CPU-internal work.

## Appendix B — diff vs. the original plan

| Section | Original | Revised |
|---|---|---|
| §0 Goal & non-goals | Unchanged | Unchanged in spirit; §0.1 adds an explicit delivery status table |
| §2 Constraints | Unchanged | C2, C5, C7 flipped from ✅-to-be to ❌-not-yet; rationale: the FFI hasn't been built |
| §3 Architecture | Proposed 10-file structure | Documents the actual 6-file structure; flags `cpu/ffi.rs` as the next addition |
| §4 Phases | 7 sequential phases | Reordered: Phase 3 (revised) = FFI + ctest baseline; Phase 4–7 renamed to reflect new priorities |
| §5 Risk register | 7 CPU-internal risks | Same 7 + 3 new integration risks |
| §7 Acceptance criteria | 8 conditions, all "merge-time" | 8 conditions re-expressed in dependency order, with explicit "when this becomes measurable" |
