<!-- v1.3 Legion Phase 7.2: PR template adds a soft checklist for the
     golden-savestate regeneration path. The actual gate is the
     "Golden savestate drift check" step in .github/workflows/ci.yml.
     v1.4 Gateway Phase 7.4: adds v1.4-specific bus dispatch and
     hot-path reference-alias drift checklist items. -->

## Summary

<!-- One-paragraph description of what this PR changes. -->

## Checklist

- [ ] `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` are clean (Rust crates)
- [ ] Release config builds with `/W4 /WX` zero errors
- [ ] `ctest --test-dir build --build-config Release --output-on-failure` is green on Windows
- [ ] Performance: `fceux11_bench_tolerance_test` stays within the v1.4 asymmetric gate (`≤ +2.5%` max-regression; speedups always pass) of `tests/fixtures/bench_baseline.json`

### If you touched any of these files

- `src/state.cpp`, `src/x6502.cpp`, `src/ppu.cpp`, `src/sound.cpp`, `src/cpu.cpp`, `src/cpu.h`, `tests/fixtures/golden/golden_savestate_test.cpp`

then you **must** also commit a regenerated golden savestate. The CI
"Golden savestate drift check" step will warn if you forgot:

```powershell
cd tests
cmake --build ../build --config Release --target fceux11_golden_savestate_test
..\build\tests\fceux11_golden_savestate_test.exe --generate
git add fixtures/golden/
git commit -m "Regenerate golden savestates for <PR description>"
```

If your change should *not* alter any savestate, run `--compare-layout`
to confirm and paste the output in the PR description:

```powershell
..\build\tests\fceux11_golden_savestate_test.exe --compare-layout
```

The output should end with `RESULT:    PASSED (all N savestates layout-identical)`.

### v1.4 Gateway bus-surface items (Phase 7.4)

If you modified any of:

- `src/bus.h`, `src/bus.cpp`, `src/cart.h`, `src/cart.cpp`
- `src/boards/*.cpp` (any of the 175 board files)

then verify:

- [ ] `ctest --test-dir build -C Release -R "mapper_core_test|rom_regression_test"` is green
- [ ] `fceux11_bench_tolerance_test` is 3/3 PASS (CI gate; if regressing, see plan §5.1.7 / §6.2)
- [ ] No new direct `ARead[]` / `BWrite[]` / `Page[]` / `VPage[]` /
      `VPageG[]` / `PRGptr[]` / `CHRptr[]` / `MMC5SPRVPage[]` /
      `MMC5BGVPage[]` / `VPageR[]` indexing in `src/boards/` (the
      "Board-file direct-array drift check" CI step enforces this)
- [ ] No new direct `ARead[]` / `BWrite[]` / `VPage[]` / `MMC5SPRVPage[]`
      / `MMC5BGVPage[]` indexing in the hot-path files
      (`x6502.cpp` / `ppu.cpp` / `cart.cpp` / `cheat.cpp` / `debug.cpp`
      / `nsf.cpp` / `lua-engine.cpp`), unless adding to the documented
      exception list (the "Hot-path reference-alias drift check" CI
      step enforces this; documented exceptions are `fceu.cpp`
      handler-registration cold path and `ppu.cpp` VPage/MMC5SPRVPage
      hot-path retention per plan §5.1.5)
- [ ] If you added or modified an `AddExState` registration, recorded
      it in `docs/internal/savestate_layout_audit.md` §"v1.4 Gateway diff"
- [ ] If you added new direct access to a hot path
      (`Bus::read` / `Bus::write` / mapper register handler /
      `x6502.cpp` / `ppu.cpp` / `cart.cpp` `g_bus.` access), ran
      `fceux11_bench_tolerance_test` locally and confirmed 3/3 PASS
