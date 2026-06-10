# Follow-up: F-1 — strncpy heap-buffer-overflow exposed by v0.3.6.5 ASan

> **Parent checkpoint**: `docs/tech/v0.3.x_Checkpoint_6.5.md` §6.1
> **CHANGELOG entry**: `[0.3.6.5]` Known Issues — F-1
> **Priority**: **P1** — must be closed before v0.3.7 starts
> **Owner**: unassigned
> **Filed**: 2026-06-10 (REDO of v0.3.6.5)

---

## 1. Problem summary

Real ASan instrumentation (MSVC 14.51 `/fsanitize=address`, validated by
`scripts/_verify_asan_instrumentation.ps1`) exposes a heap-buffer-overflow
in the ROM-loading initialisation path of the test harness.

**Pattern**:

```
malloc(5)                       ← 5-byte heap buffer
strncpy(buf, src, 7)            ← writes 7 bytes → 2-byte overflow
```

ASan reports (one of four occurrences, all identical root cause):

```
==XXXX==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x... at pc 0x... bp 0x... sp 0x...
WRITE of size 7 at 0x... thread T0
    #0 in strncpy (asan_interceptors.cpp:749)
    #1 in <test exe>+0x14008xxxx          ← caller, not yet symbolised
    ...
0x... is located 0 bytes after 5-byte region [...,...)
allocated by thread T0 here:
    #0 in malloc (asan_malloc_win.cpp:467)
    #1 in <test exe>+0x140147xxx          ← allocator caller
    ...
SUMMARY: AddressSanitizer: heap-buffer-overflow
```

Source-level `git grep -E 'strncpy\s*\([^,]+,\s*[^,]+,\s*7\s*\)'` and
`git grep -E '(malloc|FCEU_[a-z]*malloc)\s*\(\s*5\s*\)'` both return no
hits — both sizes are computed at runtime (e.g. `strlen(src)` or
`sizeof(arr)`), confirming the bug is in a code path that infers length
incorrectly.

## 2. Affected tests

| Test                  | Result under ASan | Cause              |
|-----------------------|-------------------|--------------------|
| `smoke_test`          | ✅ PASS           | does not load ROM  |
| `mapper_load_test`    | ❌ FAIL           | F-1, in `[1/8]` ROM init |
| `mapper_reset_test`   | ❌ FAIL           | F-1                |
| `rom_regression_test` | ❌ FAIL           | F-1                |
| `expected_api_test`   | ❌ FAIL           | F-1 (on `save_state` test which touches ROM load) |

`smoke_test` passing is the strongest piece of evidence that **the bug is
NOT in the main emulator hot path**. It is somewhere on the ROM
metadata / fixture / initialisation chain that only the four ROM-loading
tests trigger.

## 3. Reproduction

From the project root with vcvars64 loaded:

```powershell
scripts\_with_vcvars.bat powershell -ExecutionPolicy Bypass -File scripts\_build_asan.ps1
scripts\_with_vcvars.bat powershell -ExecutionPolicy Bypass -File scripts\_ctest_asan.ps1
```

Expected: `1/5` pass, `4/5` fail. Inspect the UTF-16 log via:

```powershell
Get-Content build-asan\_ctest_asan.log -Encoding Unicode |
    Out-File -Encoding utf8 build-asan\_ctest_asan.utf8.log
```

Then `grep -nE 'ERROR: AddressSanitizer|SUMMARY: AddressSanitizer'
build-asan\_ctest_asan.utf8.log` lists the four heap-buffer-overflow
reports.

## 4. Blocker → resolved by parent task #2

The initial v0.3.6.5 REDO could not symbolise the stack frames because
the sanitizer Release build did not emit PDBs (CMake's default Release
linker flags omit `/DEBUG`, so MSVC's bundled `llvm-symbolizer.exe`
had no symbols to resolve against — only `<exe>+0xRVA` lines came out).

This blocker is resolved by the parent checkpoint's follow-up task #2:
`CMakeLists.txt` now appends `/DEBUG /OPT:REF /OPT:ICF` to
`CMAKE_EXE_LINKER_FLAGS_RELEASE` and `CMAKE_SHARED_LINKER_FLAGS_RELEASE`
inside the `FCEUX11_ASAN` branch. Next ASan rebuild produces
`build-asan/src/fceux11.pdb` and per-test `build-asan/tests/*.pdb`.

**Next step for F-1 owner**: re-run the reproduction above with PDB
present, capture symbolised stack frames, attribute to the
`strncpy(buf, src, 7)` callsite, fix.

## 5. Fix-pass criteria (either is acceptable)

1. **Source fix**: the offending callsite is identified and patched;
   `scripts/_ctest_asan.ps1` shows `5/5 Passed` with zero ASan reports;
   an incremental regression test is added under `tests/` to prevent
   future regression of this specific pattern.

2. **Scope confinement**: symbolised stack frames prove the bug is in
   test-harness or fixture-helper code only (NOT in the shipped
   `fceux11_core` / `fceux11_drivers_*` libraries), and the test code is
   patched; main emulator unaffected stays the operating assumption.

Both outcomes must produce a commit referenced from this document and
from the parent checkpoint.

## 6. Triage notes (preserved from v0.3.6.5 REDO investigation)

- `tests/git_info_stub.cpp` is shared by **all five** test executables
  including `smoke_test`; since smoke passes, this file is **not** the
  culprit.
- The bug must lie in code reached only after `FCEUI_LoadGame`-prefix
  initialisation but before the per-frame emulation loop. Suspects (to
  be confirmed by symbolisation):
  - `tests/boards/mapper_load_test.cpp` setup helpers
  - `iNES_load` / `UNIF_load` metadata parsing (small string buffers
    such as game name, mapper name, board name)
  - `MD5` / `CRC32` formatting buffers around `printf`-style hex
    expansion
- Plan §5 v0.3.5 already swept `file.cpp` (35 `sprintf` sites) but the
  ROM-load tiny-buffer loop apparently slipped the regression; whoever
  closes F-1 should also extend the v0.3.5 audit checklist to cover
  `iNES_load` / `UNIF_load` paths.

## 7. Cross-references

- Parent checkpoint: `docs/tech/v0.3.x_Checkpoint_6.5.md` §6.1, §9
- Plan: `docs/v0.3.x_Construction_Plan_v3.md` §5 v0.3.6.5 task #3
- CHANGELOG: `[0.3.6.5]` → Known Issues → F-1
- Memory pitfall: `[[test-runs-and-pitfalls]]` (`/fsanitize=` syntax,
  PDB requirement, llvm-symbolizer location)
- Scripts: `scripts/_build_asan.ps1`, `_ctest_asan.ps1`,
  `_verify_asan_instrumentation.ps1`

## 8. Sign-off (to be filled when closed)

- [ ] Symbolised stack frames captured and attached
- [ ] Root-cause callsite identified (file:line)
- [ ] Fix committed (commit hash: ____)
- [ ] `_ctest_asan.ps1` shows 5/5 PASS
- [ ] Regression test added (path: ____)
- [ ] Parent checkpoint §6.1 updated to "F-1 closed"
- [ ] CHANGELOG `[0.3.6.5]` Known Issues updated to "F-1 closed in vX.Y.Z"
- [ ] This document moved to `docs/tech/closed/` with the closing commit hash appended to the filename
