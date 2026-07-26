# KagamiQA — FCEUX11 Independent Test Framework

KagamiQA is the test orchestration framework for FCEUX11 v1.16. It wraps the
existing 30 CTest tests into a manifest-driven, dual-oracle, machine-judgeable
system that serves as both FCEUX11's quality defence line and an AI agent's
"answer key reader."

## Architecture

```
kagami-qa/
  src/
    core/         — QaConfig, QaError (framework-agnostic)
    manifest/     — tests.json schema, parser
    adapter/      — SutAdapter trait + SubprocessAdapter (P1)
    runner/       — TestScheduler
    oracle/       — Oracle A (regression-equivalence, exit-code based)
    report/       — TransitionMatrix JSON (SWE-bench isomorphic)
    main.rs       — CLI runner binary
```

## P1 Scope (current)

- **Subprocess adapter**: wraps existing CTest binaries as subprocesses
- **tests.json manifest**: 30 entries covering all CTest + script gates
- **Migration matrix JSON**: FAIL_TO_PASS / PASS_TO_PASS report
- **Null Driver**: headless test execution without Qt

## P2+ Roadmap

- In-process SutAdapter (C ABI link core via FFI)
- Oracle B (hardware-consistency via blargg $6000 protocol)
- Lua script channel (dynamic test cases)
- Frame hash / WAV diff / mapper state comparison oracles

## Usage

```bash
# Build the runner
cargo build --bin kagami-qa-runner

# Run all tests
kagami-qa-runner \
  --manifest tests/tests.json \
  --bin-dir build/tests \
  --output kagamiqa_migration_matrix.json
```

## AI-Friendly Design

- Tests are data (JSON), not code — AI agents can read, generate, and validate
- `failure_means` field prevents vacuous tests
- Migration matrix gives machine-readable pass/fail signals
- `SutAdapter` trait enables cross-project reuse
