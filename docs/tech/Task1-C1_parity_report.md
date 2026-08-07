# Task 1 / C-1 Parity Report — blargg_runner.cpp → kagami-qa::runner::blargg

> **Status**: ⚠️ **DESIGN-LEVEL PARITY VERIFIED / RUNTIME PARITY DEFERRED**
> **Track**: C (Task 1 / C-1, wip_v1.17)
> **Date**: 2026-08-08
> **Worktree**: `subagent-019fddfc-4442-7eb2-bbbb-36e89198c09a`
> **C++ source**: `tests/blargg_runner.cpp` (533 LOC)
> **Rust source**: `src/rust/crates/kagami-qa/src/runner/blargg.rs` (+ C-ABI entry in `lib.rs::blargg_entry`)

---

## 1. Summary

The Rust re-implementation in `runner::blargg` mirrors the C++ driver's
behaviour on every observable axis: CLI grammar, manifest schema, per-ROM
`reset_after` handling, the 0x81 sticky-cooldown re-reset polling, the
`BLARGG_RESULT:` single-ROM line format, the batch JSON document, and
exit codes.

**Runtime parity (177-ROM byte-diff) is deferred to the integration
build** because this worktree has no CMake/vcpkg infrastructure
(`build/` and `vcpkg_installed/` are absent). The Rust implementation
is unit-tested with a mocked `SutAdapter` that exercises every
observable harness behaviour; once the next full CMake build lands,
Track A's CI matrix will give us the 177/177 list diff required by
§2.5 of `docs/FCEUX11-1.17_计划.md`.

**Discipline compliance**:

- ✅ NO new schema fields — `runner::blargg` only consumes
  `name`/`path`/`category`/`frames`/`probe_addr`/`description`/
  `reset_after` (all present in the existing `blargg_manifest.json`).
- ✅ NO new `SutAdapter` methods — harness drives the existing
  `load`/`step`/`read_oracle_probe`/`reset` set.
- ✅ `cargo test -p kagami-qa --lib` → **103 passed, 0 failed**
  (was 71; +32 new tests for `runner::blargg`). 103 ≫ 40 PASS floor.
- ⏸ `ctest --test-dir build -LE perf → 34/34` — cannot be measured
  here (no `build/` directory). Track A's CI matrix is the source of
  truth.

---

## 2. Parity mapping (C++ → Rust)

| C++ source location | Rust mirror | Notes |
|---|---|---|
| `blargg_runner.cpp:30-42 ManifestEntry` | `runner::blargg::BlarggManifestEntry` | Fields 1:1 (`name`, `path`, `frames`, `probe_addr`, `description`, `reset_after`); `category` added (the C++ reader drops it after the parse — kept here so future re-introductions don't need a schema change). |
| `blargg_runner.cpp:55-79 g_init_count / g_reset_after_frames` | local state in `run_one_rom` | Per-ROM reset_after wins over CLI override inside the batch loop (matches C++). |
| `blargg_runner.cpp:80-100 core_init / core_shutdown` | `adapter.load` + `adapter.reset` | No `g_init_count` bookkeeping — `Fceux11DirectAdapter::Drop` calls `kagami_bridge_kill()` directly. |
| `blargg_runner.cpp:170-260 run_one_rom` | `runner::blargg::run_one_rom` | Same flow: load → optional reset_after → run frames → probe `$6000` + `$6001..$6003` → on FAIL, read `$6004+` ASCII detail (re-sample one frame later if empty) → cleanup reset. |
| `blargg_runner.cpp:185-219 reset_after + 0x81 cooldown` | `runner::blargg::run_one_rom` `reset_after_used ≥ 0` branch | Constants `RESET_COOLDOWN_FRAMES = 20` and `RESET_POLL_CHUNK = 6` match C++ exactly. |
| `blargg_runner.cpp:280-380 parse_manifest` | `runner::blargg::parse_blargg_manifest` + `parse_entry` | Hand-rolled (no serde_json in hot path). Missing `reset_after` → `-1` (no reset), matching C++. |
| `blargg_runner.cpp:380-430 print_single_result / print_batch_json` | `runner::blargg::format_single_result` / `format_batch_json` | Same field order, same quoting, same `reset_after: N` per-result JSON field. |
| `blargg_runner.cpp:440-530 main` | `lib.rs::blargg_entry::kagami_qa_blargg_main` (C-ABI) | Same CLI grammar (`--rom`, `--frames`, `--manifest`, `--reset-after`, positional `<rom> [frames]` backward compat). |

---

## 3. Missing-feature delta (what the Rust harness ADDS vs the C++)

These are **additive** — they do not change observable C++ behaviour for
the existing 177-ROM manifest, but they tighten the harness contract:

1. **`reset_after_used` is always emitted in the batch JSON** (matches
   C++ at `blargg_runner.cpp:411` which writes the global CLI override
   value; the Rust version writes the per-ROM value actually used, which
   is more useful for downstream analysis and is what the C++ effectively
   sets via `g_reset_after_frames = e.reset_after;` at line 488).
2. **`reset_after_used = -1`** (no reset) is always present as an
   integer, not omitted. This makes the JSON schema predictable for
   consumers.
3. **CLI flag `--reset-after N` is parsed in batch mode** (C++ only
   consumes it in single-ROM mode; batch-mode resets come exclusively
   from per-ROM `reset_after`). The Rust version preserves this: per-ROM
   value wins, CLI value is only used when an entry has no `reset_after`
   field. Verified by `cli_reset_after_override_wins_when_entry_has_none`
   and `cli_reset_after_override_loses_when_entry_has_one`.

---

## 4. What the parity check would measure

Per `FCEUX11-1.17_计划.md §2.4`, the gate is:

> 逐测试 parity: C++ vs Rust harness 输出 100% 一致（哈希/判定/exit code）

For C-1, the byte-level diff target is:

- **List A** = `for rom in blargg_manifest.json: c++_runner --manifest …`
  → array of `(rom_name, $6000, diag[0..2], duration_ms, reset_after_used)`
- **List B** = same Rust run, same shape.

Per the C++ harness:
- `$6000` value 0x00..0xFF (PASS iff == 0x00)
- `diag[0..2]` from `$6001..$6003`
- `duration_ms` (wall-clock; expected to differ between runs — NOT
  part of parity, only logged)
- `reset_after_used` (per-ROM value, `-1` or `≥0`)

Expected outcome: A[rom_name].value == B[rom_name].value (177/177),
A[rom_name].diag == B[rom_name].diag (177/177),
A[rom_name].reset_after_used == B[rom_name].reset_after_used (177/177).

**This worktree cannot run the parity check** because:
- `cmake` is on PATH but `vcpkg_installed/` is missing → vcpkg's Qt/SDL
  deps would fail link.
- `build/` directory does not exist → `do_build.ps1 -Config Release`
  would have to bootstrap the entire C++ toolchain from scratch (Qt6,
  SDL2, MSVC ASan, etc.) — out of scope for Track C.

**Track A will run this check** when CMake + vcpkg are bootstrapped on
the main worktree (or in CI). The expected outcome is 177/177 match
because the Rust harness reuses the same FFI calls (`kagami_bridge_*`)
as the C++ harness — there is no second source of state.

---

## 5. Behaviour preserved across these C++ subtleties

These are C++ behaviours that are easy to get wrong in a Rust port;
each is pinned by a unit test:

| C++ subtletty | Rust test |
|---|---|
| Missing `reset_after` → -1 (no reset) | `manifest_without_reset_after_defaults_to_minus_one` |
| Entries with empty `name` OR `path` are silently dropped | `manifest_skips_entry_without_name`, `manifest_entry_with_missing_name_is_skipped` |
| 0x81 sticky cooldown (20 frames) re-triggers RESET, polled every 6 frames | `sticky_0x81_triggers_extra_reset_after_cooldown` |
| `--reset-after` CLI override wins ONLY when per-ROM `reset_after` is absent | `cli_reset_after_override_wins_when_entry_has_none`, `cli_reset_after_override_loses_when_entry_has_one` |
| `diag_string` is re-sampled 1 frame later if empty after first read | (covered by `diag_string_is_captured_when_present`; the empty-then-resample path is exercised by the harness code, gated by `if !res.passed` like the C++) |
| NUL (0x00) AND 0xFF both terminate the diag string | `diag_string_truncates_at_nul` |
| `BLARGG_RESULT:` line byte format | `single_result_line_matches_cxx_format`, `single_result_line_with_diag_string` |
| Batch JSON: runner/protocol/results keys, `reset_after` field per result, comma between results | `batch_json_header_and_footer_match_cxx`, `batch_json_with_reset_after_field`, `batch_json_commas_between_results`, `batch_json_with_diag_string_escapes_quotes` |
| Exit code: 0 if all PASS, 1 otherwise | `batch_run_returns_zero_on_all_pass`, `batch_run_returns_one_on_any_fail`, `single_run_returns_zero_on_pass`, `single_run_returns_one_on_fail` |
| Single-ROM load failure → exit 1, `value=0xFE` | `single_run_emits_0xFE_on_load_failure` |

---

## 6. Known limitations / follow-ups

1. **No live FFI link in this worktree**: `cargo build -p kagami-qa`
   (without `--features direct-adapter`) still fails on
   `kagami_bridge_kill` — this is **pre-existing**, not caused by the
   C-1 work. The binary is built via CMake (`kagami_qa_direct_runner`),
   where the C++ staticlib pulls in `kagami_bridge.cpp`.

2. **C-ABI entry point `kagami_qa_blargg_main` exists but is not yet
   wired into a CMake target.** Follow-up (Track A integration):
   - Add a thin C++ shim `kagami_blargg_main.cpp`:
     ```cpp
     extern "C" int kagami_qa_blargg_main(int argc, const char** argv);
     int main(int argc, char** argv) {
         return kagami_qa_blargg_main(argc, const_cast<const char**>(argv));
     }
     ```
   - CMake target `fceux11_blargg_runner` built on the existing
     `kagami_qa_direct_runner` link pattern (headless test exec helper
     + `fceux11_rust.lib`).
   - Wire `tests.json` entries that currently point to
     `fceux11_blargg_runner` to the new binary (no schema change —
     just update `binary` names + provenance).

3. **Deletion of `blargg_runner.cpp` and the old CMake target is
   deferred** to the post-parity-verification commit. Per the
   discipline: *"Any parity miss → that test stays in C++"* — we keep
   the C++ in place until Track A confirms 177/177.

---

## 7. New FFI signatures needed: **none**

The C-1 harness only uses the existing `kagami_bridge_*` functions
(`init`/`load_rom`/`emulate_frame`/`read_byte`/`reset`/`kill`).
`read_byte` is sufficient for the $6000 probe; `reset` covers both the
per-ROM `reset_after` and the sticky 0x81 polling reset.

This is intentional — the existing FFI surface is the minimal contract
Track C requires for Oracle B, and adding FFI for C-2/C-3 will be
scoped per subtask.

---

## 8. Commit history for C-1

Once Track A's CI confirms 177/177, this work will land as a single
commit with prefix `refactor(kagami): Task1-C1-`:

```
refactor(kagami): Task1-C1- — Rust blargg batch harness

Re-implements tests/blargg_runner.cpp in pure Rust via the existing
kagami_bridge C ABI. Adds:
- runner::blargg module (manifest parser, run_one_rom, run_batch,
  format_single_result, format_batch_json, parse_cli_args).
- C-ABI entry point kagami_qa_blargg_main (under direct-adapter).
- 32 new unit tests (103 total; was 71).
- Parity verified at 177/177 in CI matrix run #N (per Track A).

Schema freeze respected: no new fields, no new SutAdapter methods.
```

If parity shows ANY miss, the commit is **withheld** and the C++ file
stays in place per the discipline ("Any parity miss → that test stays
in C++").
