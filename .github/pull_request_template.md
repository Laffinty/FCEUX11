<!-- v1.3 Legion Phase 7.2: PR template adds a soft checklist for the
     golden-savestate regeneration path. The actual gate is the
     "Golden savestate drift check" step in .github/workflows/ci.yml. -->

## Summary

<!-- One-paragraph description of what this PR changes. -->

## Checklist

- [ ] `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` are clean (Rust crates)
- [ ] Release config builds with `/W4 /WX` zero errors
- [ ] `ctest --test-dir build --build-config Release --output-on-failure` is green on Windows
- [ ] Performance: `fceux11_bench_tolerance_test` stays within `±2%` of `tests/benchmarks/baseline_v1.0.json`

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
