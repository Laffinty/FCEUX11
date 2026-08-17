# CPU Module v2.0 — Rust-First Reimplementation

**Status:** Draft · **Branch:** `wip2.0` · **Author:** WIP · **Date:** 2026-08-17

## 0. Goal & non-goals

**Primary goal.** Replace the legacy C++ 6502 CPU implementation (`src/x6502.{cpp,h,struct.h,abbrev.h}` + `src/ops.inc` + `src/ops_table.inc`, ~2,940 LOC) with a modern, Rust-first implementation that lives inside `fceux11-core`. The new module must pass every CPU correctness oracle the project already exercises (nestest, blargg instr_test-v5, cpu_timing_test6, cpu_interrupts_v2, savestate round-trip, frame diff), preserve binary savestate format compatibility, and let us delete the C++ CPU without regressing game compatibility.

**Non-goals.**
- Not a JIT / dynamic recompiler. The CPU stays an interpreter (cycle-accurate, not opcode-translated).
- Not a from-scratch 6502. We intentionally reuse the proven opcode taxonomy already documented on [NESdev Wiki — 6502 instructions](https://www.nesdev.org/wiki/6502_instructions) and [NESdev Wiki — CPU unofficial opcodes](https://www.nesdev.org/wiki/CPU_unofficial_opcodes).
- Not a Mesen2 clone. We borrow architecture lessons from `nesium` / `ced-nes` / `accuNES` / Mesen2, but the design is shaped by FCEUX11's existing Cpu objectification (64-byte `alignas`, static_assert on layout) and the PPU/board call sites that depend on the public surface.

## 1. Why this is feasible

| Constraint | Reality |
|---|---|
| Rust workspace ready | `src/rust/` already builds 6 crates via CMake; `fceux11-core` already defines the `Cpu` trait + `NesSystem` scaffold + a `SimpleBus` performance prototype. |
| Test oracles exist | `tests/kagami/core/cpu_test.cpp` (12 cases over nestest), `tests/fixtures/nes-test-roms` (blargg suite, instr_test-v5, cpu_timing_test6, cpu_interrupts_v2, cpu_dummy_reads, etc.), `tests/fixtures/golden_savestate` (byte-level savestate regression), `fceux11_ppu_frame_diff_test` (visual regression), KagamiQA's `kagami_qa_direct_runner` (blargg 177-ROM harness). |
| Reference implementations exist | [`mikai233/nesium`](https://github.com/mikai233/nesium) (cycle-accurate Rust NES, Mesen2-inspired, 40+ ROM suites passing), [`cbeust/ced-nes`](https://github.com/cbeust/ced-nes) (memory-cycled `cpu2.rs`), [`dustinbowers/nes-emulator`](https://dustinbowers.com/projects/nes-emulator) (memory-cycle-accurate WASM target), [`accuNES`](https://forums.nesdev.org/viewtopic.php?t=26749). |
| Cycle-accurate design options | Two mature patterns documented in the literature: **instruction-stepped** (run full instruction, return cycles) and **memory-cycled** (`tick()` per cycle). FCEUX11's existing `X6502_RunDebug(cycles)` API is instruction-stepped, which is sufficient for every existing test oracle — so we adopt (a) and expose (b) as an opt-in for the cycle-gated mapper IRQ / DMC DMA cases. |
| Incremental migration | Project already operates an FFI-bridge pattern: `src/rust/` produces `fceux11_rust.lib` linked into the C++ executable; `kagami_qa_direct_runner` shows the bidirectional FFI calling pattern. Per [JetBrains/Mainmatter guidance](https://blog.jetbrains.com/rust/2026/07/27/cpp-to-rust-migration/), we go **vertical-slice through the CPU hot path** (not leaf-first) because every existing test oracle targets the CPU directly. |

**Verdict: feasible, with hard parity gates.** The CPU is small, well-understood, and has first-class test coverage. The risk is not "can we write it" but "can we keep savestate binary format + IRQ constants + per-instruction cycle counts + open-bus behaviour byte-identical".

## 2. Constraints to preserve (lock these in before coding)

| # | Constraint | Source | How we honour it |
|---|---|---|---|
| C1 | `alignas(64)` on Cpu, `sizeof(Cpu::layout_) == 64` | `src/cpu.cpp:11–22` `static_assert` | Rust struct mirrors the 64-byte X6502 blob (A, X, Y, S, P, DB, PC, mooPI, jammed, _IRQlow, _IRQlow+1, + 50 bytes scratch). The Rust `#[repr(C, align(64))]` struct is bit-compatible with the C++ layout. Savestate binary format stays valid. |
| C2 | `X6502_RunDebug` API surface (called from `ppu_rendering.cpp` ~15 sites, `cpu.cpp:run`, debugger) | `src/x6502.h:55` | The Rust CPU exposes `run(cpu: &mut Cpu, cycles: i32) -> i32` via `extern "C"`; the C++ `Cpu::run()` is rewired to call the Rust symbol. No call site changes outside `src/cpu.cpp`. |
| C3 | IRQ source bitmask constants (`FCEU_IQEXT=0x001`, `FCEU_IQNMI=0x080`, `FCEU_IQFCOUNT=0x200`, …) | `src/x6502.h:96–103` | Rust enum derives `#[repr(u32)]` matching the exact values; C++ callers continue to use the existing constants unchanged. |
| C4 | Flag bitmasks (`N_FLAG=0x80`, `V_FLAG=0x40`, `U_FLAG=0x20`, `B_FLAG=0x10`, `D_FLAG=0x08`, `I_FLAG=0x04`, `Z_FLAG=0x02`, `C_FLAG=0x01`) | `src/x6502.h:75–82` | `bitflags!` crate with the same numeric values. |
| C5 | nestest.nes reference log + blargg 177-ROM suite parity | `tests/kagami/core/cpu_test.cpp:5–30`, `scripts/download_blargg_roms.ps1` | A new `cpu_rust_test` CTest target runs the exact same nestest baseline + KagamiQA blargg harness against the Rust CPU. Failure means the PR is blocked. |
| C6 | DB (data-bus cache) + mooPI semantics | `src/cpu.cpp:51–55` (`db()` / `set_db()` / `pi()` / `set_pi()`) | Fields preserved in the 64-byte layout. |
| C7 | `TriggerNMI` / `TriggerNMI2` / `X6502_IRQBegin` / `X6502_IRQEnd` | `src/x6502.h:108–112` | Equivalent exported symbols; the C++ thin wrappers in `src/cpu.cpp:71–73` are kept (they just call into Rust). |
| C8 | `cpu.cpp` is `namespace fceu11`; singleton via `cpu_instance()` | `src/cpu.cpp:107–111` | Rust returns a `*mut CpuState` handle to C++; `cpu_instance()` continues to own the storage. |

## 3. Architecture

### 3.1 Crate shape (target end-state)

```
src/rust/crates/fceux11-core/
├── Cargo.toml                  # adds bitflags, proptest (dev), serde (dev)
├── src/
│   ├── lib.rs                  # existing — re-exports + NesSystem
│   ├── traits.rs               # existing — Cpu trait becomes the public surface
│   ├── bus.rs                  # existing
│   ├── cpu/                    # NEW module
│   │   ├── mod.rs              # public re-exports + top-level CPU struct
│   │   ├── state.rs            # registers, flags, IRQ state (mirrors 64-byte Cpu layout)
│   │   ├── addressing.rs       # 13 addressing modes (impl, zp, zpx, zpy, abs, absx, …, ind)
│   │   ├── decode.rs           # build.rs-generated opcode tables; sealed Opcode enum
│   │   ├── execute.rs          # `step()` — instruction-stepped model returning cycles
│   │   ├── interrupt.rs        # NMI, IRQ (maskable), BRK, RESET vectors
│   │   ├── unofficial.rs       # all 109 unofficial opcodes per NESdev wiki matrix
│   │   ├── ffi.rs              # #[no_mangle] extern "C" entry points for C++
│   │   ├── snapshot.rs         # savestate load/store (must produce byte-identical blob to C++)
│   │   └── tick.rs             # OPT-IN memory-cycled execution for DMA / DMC / mapper IRQ
│   ├── table_gen/              # NEW — build-time codegen of opcode tables
│   └── tests/                  # NEW — Rust-side unit + property tests
│       ├── opcodes.rs          # table-walk: every opcode runs a tiny synthetic bus
│       ├── nestest.rs          # nestest.log byte-for-byte match
│       ├── interrupts.rs       # IRQ/NMI/BRK timing
│       ├── unofficial.rs       # full unofficial opcode matrix
│       ├── proptest.rs         # random reset/state vectors vs golden snapshot
│       └── savestate.rs        # round-trip via the C++ savestate path
└── build.rs                    # NEW — runs table_gen at compile time
```

### 3.2 Execution model (instruction-stepped primary; memory-cycled opt-in)

**Primary path — `Cpu::step(&mut self, bus: &mut dyn Bus) -> u8`.**
Mirrors `X6502_RunDebug` semantics: fetch opcode, decode addressing mode (consumes the right number of bus cycles), execute the operation, return total cycle count. PPU/APU advance by the returned cycles. This is what every existing PPU call site needs and what all ROM-based tests exercise.

**Opt-in path — `Cpu::tick(&mut self, bus: &mut dyn Bus) -> TickOutcome`.**
Adopted from `cbeust/ced-nes`'s `cpu2.rs`: one cycle = one memory access, with internal state for `current_opcode` + `current_cycle`. Used **only** in the mapper IRQ + DMC DMA + sprite-0-hit timing edge cases where instruction-stepped breaks. The `tick` path is not on the hot path for normal gameplay — it is exercised by `cpu_timing_test6`, `cpu_dummy_reads_suite`, `dmc_dma_during_read4_suite`, `vbl_nmi_timing_suite`. Default to `step`; opt in via `cpu.use_cycle_accurate(true)`.

### 3.3 Opcode table generation

Compile-time codegen via `build.rs` in `fceux11-core`:
- Source data: `src/rust/crates/fceux11-core/table_gen/opcodes.toml` — 256 entries keyed by byte, holding `mnemonic`, `addr_mode`, `base_cycles`, `is_unoffical`, `category` (ALU / RMW / branch / control / data-move).
- Output: `OUT_DIR/opcode_table.rs` — `pub const fn info(byte: u8) -> OpcodeInfo` returning a `const`-evaluable record for each of the 256 entries.
- Rust match expression inside `execute.rs` switches on `info(opcode).category`, then delegates to the addressing-mode + ALU helpers. The compiler optimises this to a jump table.
- This replaces the C++ `ops.inc` / `ops_table.inc` (the latter is 1,707 lines of hand-written tables). Net source-code reduction: **~1,800 LOC deleted**.

### 3.4 Unofficial opcode coverage

All 109 unofficial opcodes are implemented per the [NESdev Wiki matrix](https://www.nesdev.org/wiki/CPU_unofficial_opcodes). The decode table sets `category = RMW | ALU` for the `0x__3` / `0x__7` / `0x__B` / `0x__F` columns and `category = NOP` for the read-NOPs (`0x__2` / `0x__4` / etc.). The well-known unstable opcodes (`ANC`, `ARR`, `XAA`, `LAS`, `TAS`, `SAX`, `AHX`, `SHX`, `SHY`, `LAX`, `DCP`, `AXS`, `ISC`, `SLO`, `RLA`, `SRE`, `RRA`) are exercised by the `blargg_nes_cpu_test5_suite` and `nes_instr_test_suite` from the nesium suite list — both already in `tests/fixtures/`.

### 3.5 Savestate compatibility

The 64-byte X6502 layout is preserved by mirroring the field offsets exactly. `CpuState::snapshot(&self) -> [u8; 64]` and `CpuState::restore(&mut self, &[u8; 64])` produce / consume the same bytes as the C++ `X6502` struct. Round-trip tests run the same `.fc0` files through both implementations and assert byte equality — the same approach as the existing `fceux11_golden_savestate_test`.

## 4. Phased build plan

### Phase 1 — Scaffold & opcode table (`week 1`)

- [ ] Add `fceux11-core/src/cpu/{mod,state,decode,addressing,execute}.rs`.
- [ ] Implement `CpuState` struct with `#[repr(C, align(64))]` matching the C++ X6502 blob field-for-field.
- [ ] Port `table_gen/opcodes.toml` from the C++ `ops_table.inc` (machine-translated by hand; do **not** auto-translate — re-author from the NESdev wiki to drop dead entries).
- [ ] `build.rs` writes `OUT_DIR/opcode_table.rs`; `mod.rs` `include!`s it.
- [ ] Implement all 13 addressing modes as functions `fn mode_xxx(state: &mut CpuState, bus: &mut dyn Bus) -> (u16, u8)` returning (effective_addr, extra_cycles).
- [ ] **Gate:** `cargo build -p fceux11-core` succeeds; `fceux11_cpu_table_test` walks all 256 opcodes without panicking.

### Phase 2 — Instruction execution (`week 2`)

- [ ] Implement ALU helpers: `adc`, `sbc`, `and/ora/eor`, `cmp/cpx/cpy`, `asl`, `rol`, `lsr`, `ror`, `inc/dec`, `bit`.
- [ ] Implement control flow: branches (with page-cross cycle penalty), `jmp`, `jsr`, `rts`, `rti`, `brk`.
- [ ] Implement data movement: `lda/ldx/ldy`, `sta/stx/sty`, `tax/tay/tsx/txa/tya/txs`.
- [ ] Implement all 109 unofficial opcodes.
- [ ] Implement `step()` returning cycle count.
- [ ] **Gate:** `nestest.nes` PC/cycle trace matches the published nestest.log line-by-line for the first 5,000 instructions (`fceux11_cpu_nestest_test`).

### Phase 3 — Interrupts, RESET, power (`week 3`)

- [ ] RESET vector load + 7-cycle reset sequence (matches the `test_reset_state` expectations in `tests/kagami/core/cpu_test.cpp:46–63`).
- [ ] NMI edge detection, IRQ mask, IRQ source OR accumulation (`_IRQlow` bitmask).
- [ ] BRK sequence (push PCH, PCL, P|B_FLAG, load vector from $FFFE/$FFFF).
- [ ] DMC DMA steal cycle (consults `FCEU_IQDPCM` bit; cycles borrowed during DMA).
- [ ] Idle CPU model for DMC DMA silence (matches `cpu_dummy_reads_suite`).
- [ ] **Gate:** `blargg_nes_cpu_test5_suite` PASS, `cpu_timing_test6_suite` PASS, `cpu_interrupts_v2_suite` PASS, `dmc_dma_during_read4_suite` PASS (`kagami_qa_blargg_runner` against these four suites; ROMs already in `tests/fixtures/nes-test-roms/`).

### Phase 4 — Memory-cycled opt-in + savestate round-trip (`week 4`)

- [ ] Implement `tick()` for the memory-cycled model. Validate against `ced-nes cpu2.rs` Harte single-step tests.
- [ ] Implement `CpuState::snapshot` / `restore`; byte-identical to the C++ X6502 blob.
- [ ] **Gate:** `fceux11_golden_savestate_test` passes (existing 5 fixture files, 0-byte tolerance). `cpu_dummy_reads_suite`, `cpu_dummy_writes_suite`, `cpu_exec_space_suite` PASS.

### Phase 5 — FFI bridge + feature flag (`week 5`)

- [ ] `cpu/ffi.rs` exposes `extern "C"` symbols: `fceux11_cpu_init`, `fceux11_cpu_reset`, `fceux11_cpu_power`, `fceux11_cpu_run(cpu_state: *mut u8, cycles: i32) -> i32`, `fceux11_cpu_trigger_nmi`, `fceux11_cpu_irq_begin/end`, `fceux11_cpu_dmr/dmw`, `fceux11_cpu_jammed`, `fceux11_cpu_get_opcode_cycles`, `fceux11_cpu_snapshot/restore`.
- [ ] `src/cpu.cpp` delegates all bodies to the Rust FFI symbols. Compile-time `FCEUX11_RUST_CPU` CMake flag selects Rust impl. Default: ON.
- [ ] `cbindgen.toml` in `src/rust/` regenerates `fceux11_rust_core.h` with the new CPU FFI declarations.
- [ ] **Gate:** `cmake --build build -j` produces the executable; running `fceux11_smoke_test` + `fceux11_cpu_test` + `kagami_qa_direct_smoke` all pass; PPU/board binary symbols unchanged.

### Phase 6 — Delete the C++ CPU (`week 6`)

- [ ] Delete `src/x6502.{cpp,h,struct.h,abbrev.h}`, `src/ops.inc`, `src/ops_table.inc`, `src/cpu.cpp` shell.
- [ ] Delete `scripts/generate_x6502_dispatch.py` (no longer needed; build.rs replaces it).
- [ ] Remove `src/x6502*` references from `src/CMakeLists.txt`, `tests/CMakeLists.txt`, `scripts/`.
- [ ] **Gate:** full CTest run is green: `ctest -L '^((?!perf).)*$' --output-on-failure` exits 0. Frame diff (`fceux11_ppu_frame_diff_test`) + WAV diff (`fceux11_apu_wav_diff_test`) byte-equal.

### Phase 7 — Hardening & ship (`week 7`)

- [ ] Property-based fuzz: `proptest` over (random ROM bytes × random initial state) vs golden savestate round-trip.
- [ ] Benchmark: `cargo bench` for `step()` + `tick()`; assert within 5% of the C++ baseline in `tests/benchmarks/baseline_v1.0.json`.
- [ ] Audit: every public symbol from `x6502.h` is either re-exported by `fceux11_rust_core.h` or removed (and the removal documented in `ChangeLog.md`).
- [ ] **Gate:** PR description lists the test suite results; reviewer signs off on the ChangeLog entry; merge to `main` removes ~2,940 LOC of C++.

## 5. Risk register & mitigations

| Risk | Probability | Mitigation |
|---|---|---|
| Savestate blob drift (0x00 vs 0xFF padding) | High | Write a Python script that opens the existing `.fc0` files and prints the field offsets; align the Rust struct field-by-field with `#[repr(C)]` and explicit padding arrays. |
| Open-bus behaviour subtle differences | Medium | Run `ppu_open_bus_suite` and `dmc_tests_suite` continuously through Phases 3–5. FCEUX11 already passes these in C++; if Rust regresses, the diff is small and easy to fix. |
| DMC DMA steal cycle accounting | Medium | The `step()` model already cycles correctly for DMC IRQ assertion; the steal itself happens on the *next* read. Test with `dmc_dma_during_read4_suite`. |
| Per-instruction cycle count regression | High | The blargg `cpu_timing_test6_suite` is the authority. Add a Rust-side diff-vs-expected for every opcode's `base_cycles` in `tests/opcodes.rs`. |
| Performance regression on the per-dot PPU path | Medium | `fceux11_bench_x6502_exec` benchmark; assert ≤105% of the v1.0 baseline. |
| 64-byte alignment breaks `#[derive(Default)]` | Low | Use explicit `Default` impl zeroing the struct via `core::ptr::write_bytes` (Rust 2024 stdlib stabilises `core::mem::MaybeUninit::zeroed()` already). |
| `alignas` mismatch on C++ side after struct reorder | High | Never reorder fields. The `static_assert` in `src/cpu.cpp:11–22` is the canary — keep it; add an identical assertion in the Rust crate's `build.rs` that compares `memoffset::offset_of!(CpuState, PC)` against the C++ header. |

## 6. What we delete when this ships

| File | Lines | Status |
|---|---:|---|
| `src/x6502.cpp` | 655 | deleted in Phase 6 |
| `src/x6502.h` | 99 | deleted in Phase 6 |
| `src/x6502struct.h` | 36 | deleted in Phase 6 |
| `src/x6502abbrev.h` | 19 | deleted in Phase 6 |
| `src/ops.inc` | 424 | deleted in Phase 6 |
| `src/ops_table.inc` | 1,707 | deleted in Phase 6 |
| `src/cpu.cpp` (facade) | 112 | replaced by ~40 LOC of FFI shim calls |
| `scripts/generate_x6502_dispatch.py` | — | deleted; replaced by Rust `build.rs` |
| **Total C++ deleted** | **~3,052 LOC** | — |
| **New Rust** | ~2,400 LOC (estimate; `Cpu::step` core ~600, table + addressing ~400, unofficial ~250, interrupts ~200, FFI ~150, tests ~800) | +15 net file count, but each file is single-responsibility and self-documenting |

## 7. Acceptance criteria for merging

The PR for v2.0 CPU module is mergeable when **all** of these are true:

1. `ctest -L '^((?!perf).)*$' --output-on-failure` exits 0.
2. `kagami_qa_direct_runner --manifest tests.json` reports PASS on all ROMs that currently PASS in `main` (no regression).
3. `fceux11_golden_savestate_test` byte-equal on all 5 fixtures (0-byte tolerance).
4. `fceux11_ppu_frame_diff_test` 0-pixel tolerance on all fixtures.
5. `fceux11_bench_x6502_exec` ≤105% of the baseline in `tests/benchmarks/baseline_v1.0.json`.
6. `static_assert(sizeof(Cpu::layout_) == 64)` (C++ side) holds; the Rust `build.rs` offset cross-check passes.
7. `ChangeLog.md` records the migration; `docs/plans/cpu-rust-v2.md` (this file) is referenced.
8. The 8 deleted files appear in `git log --diff-filter=D --name-only` for the merge commit.

## 8. References

- NESdev Wiki — [6502 instructions](https://www.nesdev.org/wiki/6502_instructions), [CPU unofficial opcodes](https://www.nesdev.org/wiki/CPU_unofficial_opcodes), [Emulator tests](https://www.nesdev.org/wiki/Emulator_tests), [Visual6502 test programs](https://www.nesdev.org/wiki/Visual6502wiki/6502TestPrograms).
- Existing FCEUX11 test infrastructure: `tests/kagami/core/cpu_test.cpp`, `tests/fixtures/nes-test-roms/`, `tests/fixtures/golden_savestate/`, `fceux11_golden_savestate_test`, `kagami_qa_blargg_runner`, `kagami_qa_direct_runner`.
- External Rust NES implementations: [`mikai233/nesium`](https://github.com/mikai233/nesium) (cycle-accurate), [`cbeust/ced-nes`](https://github.com/cbeust/ced-nes) (memory-cycled), [`patsoffice/rust-emu`](https://github.com/patsoffice/rust-emu) (trait-based framework), [`accuNES`](https://forums.nesdev.org/viewtopic.php?t=26749).
- Migration patterns: JetBrains / Mainmatter — [C++ to Rust Migration](https://blog.jetbrains.com/rust/2026/07/27/cpp-to-rust-migration/).
- Property-based testing: [proptest-rs/proptest](https://github.com/proptest-rs/proptest), [Rust Fuzz Book](https://rust-fuzz.github.io/book/).