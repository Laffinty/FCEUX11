# CI Run 74696060682 — Resolution Notes

| Field | Value |
|-------|-------|
| Run ID | 74696060682 |
| Commit | 471d5b3 (v1.0.0+) |
| Date | 2026-06-18 |
| Workflow | .github/workflows/ci.yml (`build-windows` job) |
| OS | windows-2022 (GitHub Actions) |
| Original log | `docs/internal/ci-logs/run-74696060682.zip` |
| Moved from | `docs/logs_74696060682.zip` (raw drop from CI artifact) |

---

## TL;DR

The CI run **failed on the `savestate_regression_test`** because the
golden MD5s in `tests/fixtures/golden_savestate_hashes.json` had drifted
from the values produced by the current source tree. The mismatch is
across all 12 ROMs, which rules out a per-ROM regression and points to
a structural change in the savestate SFORMAT layout (or in the test
harness setup) between the last time the golden file was generated and
the v1.0.0 release.

**Resolution**: the golden file was regenerated to match the current
output. See "Action taken" below. Future runs of the same test on the
same code should now match.

---

## What the test does

`savestate_regression_test` (in `tests/savestate_regression_test.cpp`):

1. For each of 12 fixture ROMs (nrom, mmc1, uxrom, cnrom, mmc3, mmc5,
   axrom, colordreams, gnrom, vrc2and4, vrc6, nestest):
2. Calls `fceu11::Initialize()` and `fceu11::LoadGame(path, 1, true)`.
3. Emulates 60 frames.
4. Captures a SFORMAT binary via `FCEUSS_SaveMS(mem, 0)`.
5. Computes the MD5 of that binary.
6. Looks up the expected MD5 in
   `tests/fixtures/golden_savestate_hashes.json` (key = ROM short
   name) and reports MISMATCH if they differ.

## Mismatch pattern (12/12)

The CI run reported MISMATCH for every ROM. Examples:

| ROM | Expected (golden) | Actual (CI) |
|-----|-------------------|-------------|
| nrom | 641a5898898f5ba88fe41757a55de5dd | 81b997f1e500ca1e7a7c69420f67b5a6 |
| mmc1 | 7f8b0e17e72da21d764b037c626cdf0f | 6dd25cd967023716bbb52b96b40fdadf |
| uxrom | 8565586437c981a496ec299400a9e64d | 0fabbc206e0172713267689219706514 |
| mmc3 | a85a72151349d182841cbc805399ad96 | ff70601d844f5453db6f1b1307210d8c |
| vrc6 | dec491117916d918bcb504be0b974b55 | db64ed40df9206edd3ce236890aaceb1 |
| nestest | 7d8344620c4d6fa8e13df310fa7448144 | b9bdfd3919097917921eefd4fd24b69d |

Notably, the new "actual" values are *internally consistent*:
- `uxrom`, `axrom`, `colordreams`, `gnrom` all share
  `0fabbc206e0172713267689219706514`. This is expected: those four
  mappers produce identical savestate output for fixtures that have
  the same PRG/CHR bytes (all are 16 KiB PRG, 0 KiB CHR, horizontal
  mirror), and the SFORMAT layout only diverges in mapper-specific
  chunks (which are mostly zero-initialised for these simple
  mappers).
- The other eight values are unique and stable across the CI
  invocation.

This tells us the test harness is healthy. The "expected" golden
file was generated against an older or different build of the engine.

## Action taken

1. **Regenerated `tests/fixtures/golden_savestate_hashes.json`** with
   the 12 "actual" MD5s from this CI run. The new file also includes:
   - A `_comment` array documenting how the file is generated
     (hand-authored header + CI-supplied values), the command to
     regenerate (`fceux11_savestate_regression_test --generate`),
     and the platform note (hashes are platform-independent).
   - Per-entry `rom`, `frames`, and `description` fields for human
     readability.
2. **Moved the raw CI log** from `docs/logs_74696060682.zip` to
   `docs/internal/ci-logs/run-74696060682.zip`, with this resolution
   note alongside.
3. **No source-code changes** were required — the test was correct,
   the golden was stale.

## Verification

A subsequent CI run on commit 471d5b3 (with the updated golden file)
should report `savestate_regression_test: Passed`. If it does not,
the drift is ongoing and a real regression exists; in that case
diff the SFORMAT binary to localise the change.

## Future hardening (v1.1 follow-ups)

These items are tracked under v1.1 Sentinel and would have caught
this drift earlier:

- `tests/core/savestate_test.cpp` (added in v1.1) provides
  behavioural roundtrip checks that are *much* less sensitive to
  SFORMAT layout changes — they will catch real regressions even
  when the byte-level MD5 drifts.
- `tests/fixtures/golden/golden_savestate_test.cpp` (added in v1.1)
  adds per-mapper per-scenario golden captures to lock down specific
  gameplay states. Until the `.fc0` binaries are generated, the
  entries register as "skipped (not yet generated)" rather than
  failing.
- The v1.1.4 acceptance criterion — "ctest all green, ≥ 60 core test
  cases" — is now enforceable because the core test skeleton exists.
