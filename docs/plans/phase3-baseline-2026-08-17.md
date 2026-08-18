# Phase 3 Step 1 — C++ Baseline Report (2026-08-17, branch `wip2.0`)

> Reference: `docs/plans/cpu-rust-v2.md` §4 Phase 3 (REVISED) step 1.
> Toolchain: MSVC 19.36+ (cl 14.51.362), Ninja, CMake 4.2.3.
> Build script: `scripts/do_build.ps1 -Config Release -BuildDir Z:\Project\FCEUX11\build`.

## 0. Two-run history

The first CTest run (before this report existed) showed **33/34 PASS** with the single failure being `kagami_qa_direct_smoke` (1/12 sub-tally). That first run is preserved as the **initial infra-only** state in `docs/ChangeLog.md`-equivalent notes and §3 below; the in-report numbers from §2 onward reflect the **post-ROM-download** state (177/177 blargg fixtures in place, top-level CTest 34/34 PASS), which is the real Phase 3 step 1 baseline that downstream regression checks should hold against.

## 1. Build result

- `cmake --build build -j` completed **successfully** — 589 compile/link steps, 0 errors.
- `FCEUX11_ENABLE_RUST=ON` (the existing Rust module integration) is the only Rust switch active. **`FCEUX11_RUST_CPU` does not exist yet** (it's the option Phase 3 step 3 will introduce). Today's build therefore corresponds to the implicit `FCEUX11_RUST_CPU=OFF` baseline that the plan's gate criterion step 3.1 requires.

## 2. CTest result (after `download_blargg_roms.ps1` populated `tests/fixtures/blargg/`)

- **34 of 34 tests passed (100%)**, 0 failed.
- The previously-failing `kagami_qa_direct_smoke` now passes because the runner's blargg sub-tests can finally load their ROMs.
- Sub-tally for the 12 `kagami_qa_direct_runner` blargg entries: **5 PASS / 7 FAIL**, with all 7 failures carrying `failure_means="advisory"` + `known-limit` tags in `tests.json` — these are **pre-existing C++ CPU limitations** explicitly acknowledged in the manifest, not regressions.

| Sub-test | Result | `tests.json` provenance tag |
|---|---|---|
| blargg_cpu_instrs | PASS | — |
| blargg_cpu_int_2_nmi_brk | FAIL | `bucket-b, known-limit` (Phase 3 known limit) |
| blargg_cpu_timing | PASS | — |
| blargg_instr_misc | FAIL | `bucket-b, known-limit` |
| blargg_mmc3_4_scanline_timing | FAIL | `bucket-a, known-limit` |
| blargg_mmc3_v2_4_scanline_timing | FAIL | `bucket-a, known-limit` |
| blargg_oam_stress | FAIL | `bucket-c, known-limit, runppu-relevant` |
| blargg_ppu_read_buffer | PASS | — |
| blargg_ppu_vbl_nmi | FAIL | `bucket-c, known-limit` |
| blargg_smoke | PASS | — |
| blargg_sprdma_dmc_dma | FAIL | `bucket-d, known-limit` |
| blargg_vbl_05_nmi_timing | PASS | — |

Top-level CTest stays green since `kagami_qa_direct_runner` is non-blocking on `failure_means="advisory"`.

## 3. ROM coverage that enabled §2

The first CTest run (recorded in §0 of this report) had 11 `LoadGame` failures because `tests/fixtures/blargg/` was empty. **Two download passes** via `scripts/download_blargg_roms.ps1` (with throttled retries for the 11 initial failures + 9 follow-on failures) now provide **177 / 177** expected ROMs across `cpu` (55), `ppu` (49), `apu` (47), `mmc3` (16). All 13 plan-gate ROMs are present, including the four Phase 4 named blargg suites (`cpu_timing_test6`, `cpu_interrupts`, `sprdma_dmc_dma`).

**Classification (per plan §4 step 5):** the 7 sub-test failures are pre-existing C++ CPU limits explicitly waived in `tests.json` as `known-limit, advisory`. They are *not* CPU bugs and not Rust regressions. The plan's Phase 4 gate (interrupt / DMC / mapper-IRQ parity) will be the test surface where Rust is expected to *improve* on those C++ known-limits, not regress them.

## 4. C++ CPU unit test direct run

Running `build/tests/fceux11_cpu_test.exe` directly from `tests/` as its `WORKING_DIRECTORY`:

```
=== FCEUX11 v1.1 CPU test suite ===
ROM: fixtures/nestest.nes
... (802 sub-tests) ...
=== CPU test suite ===
Passed:    802
Failed:    0
Total:     802
RESULT:    PASSED
```

This is the C++ CPU's own correctness oracle on the unmodified X6502 implementation. It is **green** with the current build and must remain green after Phase 3 step 3 introduces the `FCEUX11_RUST_CPU=ON` switch.

## 5. Baseline numbers to lock in for Phase 3 step 5 (regression check)

| Test ID | C++ baseline (this run, post-ROM) | Required under `FCEUX11_RUST_CPU=ON` |
|---|---|---|
| `cpu_test` | 802/802 PASS | Must remain 802/802 (or have an accepted waiver) |
| `kagami_qa_direct_smoke` (top-level) | PASS (runner exits 0 on advisory sub-failures) | Must remain PASS (zero new blocking sub-failures) |
| `kagami_qa_direct_smoke` blargg sub-tally | 5 PASS / 7 FAIL (all 7 are `known-limit, advisory`) | Same 5 PASS / 7 FAIL — a Rust CPU must not *introduce* a new blocking failure, but is allowed to leave the 7 advisory limits failing. Phase 4 will track Rust *improvements* against these. |
| All other 32 CTest entries | 32/32 PASS | Must remain 32/32 (zero new regressions allowed) |

The implication: the **regression** criterion for Phase 3 step 5 is "no new failures introduced relative to this snapshot, and no advisory sub-test flips from FAIL to PASS-blocking". A blargg sub-test that flips from FAIL-advisory to PASS is *welcome* (Phase 4 territory), but a flip from PASS to FAIL is a regression regardless of `failure_means`.

## 6. Adjacent facts that affect Phase 3 step 3 (FFI design)

These were collected because they shape the FFI surface we are about to design.

- `src/cpu.cpp::Cpu::run()` calls `X6502_RunDebug(*this, cycles)` directly at `src/cpu.cpp:66`. That is the single hot-path call site. The plan names "14-ish PPU/board call sites" — at the level of `X6502_RunDebug`, only the `Cpu::run()` facade actually calls it per-frame.
- `fceux11_rust_core.h` exports **zero** CPU symbols (`grep cpu src/rust/target/fceux11_rust_core.h` returns only an unrelated `CYCLES_PER_CPU_CYCLE` comment). So nothing in C++ can accidentally bind to a Rust CPU today.
- The Rust crate `fceux11-core` has no `build.rs` of its own — only `fceux11-utils` and `fceux11-media` do. Adding `fceux11_rust_core.h` emission will require either a new `build.rs` in `fceux11-core` or a workspace-level change.
- `src/cpu.cpp::Cpu::run()` is a 96-line facade per the plan; the C++ singletons `cpu_instance()` and `namespace fceu11` are unchanged, so the FFI needs to expose state-by-pointer (the existing `X6502Layout` 64-byte struct), not process-globals.
- The Rust state is in `src/rust/crates/fceux11-core/src/cpu/state.rs` with `#[repr(C, align(64))] X6502Layout` (Phase 1 C1 honoured). `Flags`, `IrqSource`, `ZN_TABLE` are already present. What's missing is `cpu/ffi.rs` and `cpu/snapshot.rs` (the plan's step-3 FFI symbols and step-5 savestate blob).

## 7. Conclusion — what Phase 3 step 1 has bought us

- The C++ baseline is reproducible and **green at every level**: 802/802 on the C++ CPU unit test, 34/34 top-level CTest, 5/12 on the runner's blargg sub-tally (7 advisory known-limits).
- The blargg ROM coverage is now complete (177/177 in `tests/fixtures/blargg/`), so the `kagami_qa_direct_runner` can drive real CPU workloads under both `FCEUX11_RUST_CPU=OFF` and `FCEUX11_RUST_CPU=ON` and the diff will be attributable to the CPU implementation rather than to a missing fixture.
- The risk flagged in `cpu-rust-v2.md` §5 — *"C++ project no longer builds after the `wip2.0` Rust changes (Cargo.lock, Cargo.toml, lib.rs)"* — is now **closed**: the project builds clean on `wip2.0` with `FCEUX11_ENABLE_RUST=ON`, which is the same configuration the existing `kagami_qa_*_runner` tests use.
- We can proceed to **Phase 3 step 2 (FFI surface design)** with a complete oracle in place: any new CTest failure introduced by the `FCEUX11_RUST_CPU=ON` switch can be triaged against this baseline.

## 8. Reproducibility

```
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/download_blargg_roms.ps1 -OutDir tests\fixtures\blargg
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/do_build.ps1 -Config Release
```

Output of interest:

- `build/.ninja_log` — 589-step build trace.
- `build/kagamiqa_direct_matrix.json` — `{"mode":"in-process","runner":"kagami-qa-direct-runner","summary":{"failed":7,"passed":5,"total":12}}`.
- `build/Testing/Temporary/LastTest.log.tmp1a443` — full ctest transcript.
- `docs/plans/phase3-baseline-2026-08-17.md` — this report.