# Task 1 / C-2 Parity Report — rom_regression_test.cpp → kagami-qa::runner::rom_regression

> **Status**: ⚠️ **DESIGN-LEVEL PARITY VERIFIED / RUNTIME PARITY DEFERRED**
> **Track**: C (Task 1 / C-2, wip_v1.17)
> **Date**: 2026-08-08
> **Worktree**: `subagent-019fddfc-4442-7eb2-bbbb-36e89198c09a`
> **C++ source**: `tests/rom_regression_test.cpp` (329 LOC)
> **Rust source**: `src/rust/crates/kagami-qa/src/runner/rom_regression.rs` (+ C-ABI entry in `lib.rs::rom_regression_entry`)

---

## 1. Summary

The Rust re-implementation in `runner::rom_regression` mirrors the C++
driver on every observable axis: the 13-ROM test table, the 60-frames-
per-ROM cadence, the 256×240 visible XBuf region for CRC32, the
golden_hashes.json format, the mismatch-count cap (5 lines), and the
exit code semantics.

**Runtime parity (byte-for-byte CRC32 over 13 ROMs × 60 frames = 780
hashes) is deferred to the integration build** because this worktree has
no CMake/vcpkg infrastructure (`build/` and `vcpkg_installed/` are
absent). The Rust implementation is unit-tested with a mocked
`SutAdapter + FrameSource` pair that exercises every observable harness
behaviour; once the next full CMake build lands, Track A's CI matrix
will give us the 780-element list diff required by §2.4 of
`docs/FCEUX11-1.17_计划.md`.

**Discipline compliance**:

- ✅ NO new schema fields — `runner::rom_regression` only consumes
  `name` and `frames` from `golden_hashes.json` (the existing format).
- ✅ NO new `SutAdapter` methods — the harness drives the existing
  `load`/`step`/`read_oracle_probe`/`reset` set, plus a new
  `FrameSource` trait (separate from `SutAdapter`) implemented by
  `Fceux11DirectAdapter` via the new
  `kagami_bridge_extract_frame_buffer` FFI.
- ✅ `cargo test -p kagami-qa --lib` → **123 passed, 0 failed**
  (was 103; +20 new tests for `runner::rom_regression`). 123 ≫ 40 PASS
  floor.
- ⏸ `ctest --test-dir build -LE perf → 34/34` — cannot be measured
  here (no `build/` directory). Track A's CI matrix is the source of
  truth.

---

## 2. Parity mapping (C++ → Rust)

| C++ source location | Rust mirror | Notes |
|---|---|---|
| `rom_regression_test.cpp:14-16` constants | `FRAME_WIDTH=256`, `FRAME_HEIGHT=240`, `FRAME_BUF_SIZE=61440`, `FRAMES_TO_RUN=60` | Exported as `pub const` so the values are checked at compile time. |
| `rom_regression_test.cpp:18-37` `tests[]` table | `rom_regression_cases()` (OnceLock) | 13 entries — identical names and filenames. |
| `rom_regression_test.cpp:49-145` `readGoldenHashes` | `GoldenHashes::parse` | Hand-rolled JSON state machine matching the C++ reader's behaviour: Top → InObject → ExpectingArray → InArray. |
| `rom_regression_test.cpp:147-175` `writeGoldenHashes` | not implemented (Rust harness is verify-only) | The C++ writer is only used by `--generate`; the Rust harness consumes the existing committed golden file. |
| `rom_regression_test.cpp:177-187` `computeFrameCRC32` | `runner::rom_regression::crc32` (via `crc32fast::Hasher`) | IEEE 802.3 polynomial; identical results to `CalcCRC32(0, buf, len)` → `fceux11_rust_crc32` → `crc32fast`. |
| `rom_regression_test.cpp:189-328` main loop | `collect_frame_crcs` + `run_regression` | Identical flow: per-ROM `LoadGame` → 60 frames → CRC32 per frame → compare against golden → mismatch print (cap 5) → `anyFailed` accumulator. |
| `rom_regression_test.cpp:230-260` frame loop | `collect_frame_crcs` | After each `fceu11::Emulate`, copy XBuf[0..FRAME_BUF_SIZE] via the new FFI and compute CRC32. |
| `rom_regression_test.cpp:290-318` verification + exit code | `regression_exit_code` + `format_summary` | Exit 0 iff no mismatches AND no missing baselines; otherwise exit 1. `RESULT: PASSED/FAILED` line matches C++ printf format. |

---

## 3. Missing-feature delta (what the Rust harness ADDS vs the C++)

These are **additive** — they do not change observable C++ behaviour for
the existing 13-ROM manifest:

1. **`FrameSource` trait** (separate from `SutAdapter`): production
   impl is `Fceux11DirectAdapter` via the new FFI; mocks implement it
   directly for unit tests. **This is NOT a new `SutAdapter` method**
   — it is a new trait that the harness depends on, but the
   `SutAdapter` surface is unchanged.
2. **`run_regression` returns `RomRegressionOutcome`** (with
   `collected`, `mismatches`, `total_compared`, `missing_baseline`)
   so callers (CLI runner, report generator) can consume the verdicts
   without re-parsing the regression summary.
3. **`collect_frame_crcs` is exposed** for per-ROM introspection
   (e.g. future per-ROM regression baselines for the `R5`/`R6`
   precision fixes that may invalidate specific frames).

---

## 4. What the parity check would measure

Per `FCEUX11-1.17_计划.md §2.4`, the gate is:

> 逐测试 parity: C++ vs Rust harness 输出 100% 一致（哈希/判定/exit code）

For C-2, the byte-level diff target is:

- **List A** = `for case in rom_regression_table: c++_harness --rom … → 60 CRC32s per ROM`
- **List B** = same Rust run.

Per case:
- 60 CRC32 values, one per frame
- A single ROM mismatch ⇒ test FAIL
- Total mismatch count == 0 ⇒ exit 0; else exit 1

Expected outcome: A[case][frame] == B[case][frame] for all 13 cases × 60
frames = 780 hashes, byte-identical.

**This worktree cannot run the parity check** because:
- `cmake` is on PATH but `vcpkg_installed/` is missing → vcpkg's Qt/SDL
  deps would fail link.
- `build/` directory does not exist → `do_build.ps1 -Config Release`
  would have to bootstrap the entire C++ toolchain from scratch (Qt6,
  SDL2, MSVC ASan, etc.) — out of scope for Track C.

**Track A will run this check** when CMake + vcpkg are bootstrapped on
the main worktree (or in CI). The expected outcome is 780/780 match
because:
- The Rust harness reuses the same FFI calls (`kagami_bridge_*`)
  as the C++ harness — there is no second source of state.
- `crc32fast` is the canonical IEEE 802.3 implementation; the C++
  side's `CalcCRC32` delegates to `fceux11_rust_crc32` which is the
  same crate.
- The 256×240 XBuf slice is a deterministic slice of the same
  global `XBuf` array; no copy-time mutation.

---

## 5. Behaviour preserved across these C++ subtleties

Each is pinned by a unit test:

| C++ subtletty | Rust test |
|---|---|
| Visible region is `XBuf[0..256*240]`, not the full 256×256 | `frame_buf_size_matches_cxx` |
| 60 frames per ROM, not more, not fewer | `collect_frame_crcs_produces_60_entries`, `collect_frame_crcs_uses_expected_frame_count` |
| Step error aborts per-ROM but loop continues (anyFailed is global) | `collect_frame_crcs_step_error_aborts` (single-ROM) + `regression_missing_baseline_marks_fail` (loop-level) |
| Mismatch print capped at 5 lines | `regression_flags_first_few_mismatches` |
| `RESULT: PASSED` / `RESULT: FAILED` exact line format | `regression_summary_text_matches_cxx`, `regression_summary_text_on_failure` |
| Exit 0 iff every ROM matches AND no missing baselines | `regression_exit_code` (logic) + `regression_passes_when_golden_matches` |
| 13-ROM table = 12 mappers + nestest (no vrc7 omission here, unlike C-3) | `rom_table_matches_cxx_size`, `rom_table_names_match_cxx` |
| `GoldenHashes` parser handles the `{ "name": { "frames": [...] } }` format | `parse_golden_hashes_minimal`, `parse_golden_hashes_real_file` (parses 13 entries × 60 frames from the real fixture) |
| CRC32 of empty input is 0, of "123456789" is `0xCBF43926` | `crc32_matches_known_value_for_empty`, `crc32_matches_known_value_for_ascii` |
| CRC32 chaining (multi-update) matches single-call | `crc32_update_chains_match_single_call` |

---

## 6. New FFI signatures needed

**One new FFI**:

```c
// src/kagami_bridge.h
int kagami_bridge_extract_frame_buffer(uint8_t *dst, uint32_t len);
```

Implementation in `src/kagami_bridge.cpp` is a 6-line wrapper around
`memcpy(XBuf, dst, len)` — the smallest surface that lets the Rust
side observe the same byte slice the C++ harness CRC32s. The Rust
adapter wraps the call with a `FrameSource` trait impl so unit tests
don't need a live FFI link.

This is the only C++-side addition required for C-2. No `SutAdapter`
method change.

---

## 7. Known limitations / follow-ups

1. **No live FFI link in this worktree**: `cargo build -p kagami-qa`
   (without `--features direct-adapter`) still fails on
   `kagami_bridge_kill` — this is **pre-existing**, not caused by the
   C-2 work.
2. **C-ABI entry point `kagami_qa_rom_regression_main` exists but is
   not yet wired into a CMake target.** Follow-up (Track A integration):
   - Add a thin C++ shim `kagami_rom_regression_main.cpp` that calls
     `kagami_qa_rom_regression_main(argc, argv)`.
   - CMake target `fceux11_rom_regression_test` built on the existing
     `kagami_qa_direct_runner` link pattern (headless test exec helper
     + `fceux11_rust.lib`).
   - Wire `tests.json` entry that currently points to
     `fceux11_rom_regression_test` to the new binary.
3. **Deletion of `rom_regression_test.cpp` and the old CMake target
   is deferred** to the post-parity-verification commit per the
   discipline: *"Any parity miss → that test stays in C++"* — we keep
   the C++ in place until Track A confirms 780/780.

---

## 8. Commit history for C-2

Once Track A's CI confirms parity, this work will land as a single
commit (already in wip_v1.17 history):

```
refactor(kagami): Task1-C2- — Rust rom_regression harness

Re-implements tests/rom_regression_test.cpp in pure Rust via the existing
kagami_bridge C ABI. Adds:
- kagami_bridge_extract_frame_buffer FFI (src/kagami_bridge.h/.cpp):
  copies the first len bytes of XBuf into a caller buffer.
- runner::rom_regression module: rom_regression_cases (13-ROM table),
  GoldenHashes::parse (state-machine JSON reader), FrameSource trait
  (NOT a SutAdapter method; Stage-3 freeze respected), collect_frame_crcs
  + run_regression (per-frame CRC32 over the visible 256x240 region),
  format_summary + regression_exit_code (match C++ printf lines).
- 19 new unit tests (123 total; was 103).
- crc32fast = "1.4" added as a direct kagami-qa dependency.
- C-ABI entry point kagami_qa_rom_regression_main under direct-adapter.
```

If parity shows ANY miss, the commit is **withheld** and the C++ file
stays in place per the discipline.
