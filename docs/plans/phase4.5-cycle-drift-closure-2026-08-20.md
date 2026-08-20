# Phase 4.5 follow-up — Cycle-drift closure (IRQ bridge, timestamps, P-register parity, page-cross cycles)

Date: 2026-08-20. Branch `wip2.0`. Status: **resolved for the traced surface; two 1-mismatch residuals remain unlocated**.

> Reference: `docs/plans/cpu-rust-v2.md` §4 Phase 4, §7 acceptance criteria
> #3 ("no regression under FCEUX11_RUST_CPU=ON") and #5 ("savestate
> byte-equal"). This document records the four root causes closed in this
> pass and the honest remainder.

## 0. TL;DR

Four distinct defects kept the Rust CPU's per-frame cycle accounting from
matching the C++ reference:

1. **IRQ-line visibility** — mapper / APU-frame IRQ assertions and `$4017`
   clears land on the C++ `X6502::IRQlow` blob mid-call; the Rust state
   snapshot at call start never saw them.
2. **Frozen per-instruction timestamp** — `timestamp_` advanced once per
   `Cpu::run` call instead of per instruction, so the MMC1 `lreset`
   write-throttle dropped legitimate writes.
3. **P-register B/U semantics** — Rust PLP/RTI masked B/U (datasheet),
   C++ `_P=POP()` restores them; RESET differed too (Rust `I|U` vs C++
   exact `I`).
4. **Page-cross extra-cycle loss (the dominant residual)** — ALU /
   compare / LAX / LAS read-path indexed modes discarded the
   addressing-mode `extra_cycles`, charging 1 cycle less than C++
   `GetABIRD` / `GetIYRD` per page cross.

After all four fixes, the 300-frame `instr_v5_all` cycle trace and the
244,254-line register trace are **byte-identical** between the two builds
(`first PC div=None`, `first irq div=None`), and `golden_savestate_test`
flips FAIL→PASS under `FCEUX11_RUST_CPU=ON`.

## 1. Root causes, evidence, and fixes

### 1.1 IRQ bridge (mmc3_4)

The mmc3_4 trace diverged at frame 17 / row 14494: Rust kept `irq_low =
0x200` while C++ cleared it. A `$4017` write with bit 6 set triggers
`X6502_IRQEnd(FCEU_IQFCOUNT)` on the C++ side; the Rust snapshot was taken
at call start and the OR-merge preserved the already-cleared bit.

Fix: `Bus::sync_irq_from_host` / `sync_irq_to_host` with an
`IRQ_BRIDGE_INSTALLED` flag — overwrite the Rust `irq_low` from the host
blob at every dispatch boundary when the bridge is installed; no-op in
pure-Rust tests. Result: mmc3_4 42-frame trace fully identical.

### 1.2 Per-instruction timestamp (cpu_instrs / MMC1)

The `cpu_instrs` 100-frame trace diverged at `$FFF8` reads ($EAB3 vs
$EB9B): the MMC1 `lreset` throttle (`mmc1.cpp:136-138`, ignores writes
when `ts < lreset+2`) dropped the 5-bit register writes following a
`$8000=94` reset because `timestamp_` was call-granular.

Fix: `cpu_rust_tick_thunk` now advances `timestamp_` /
`sound_timestamp_` per instruction (matching C++ `add_cycles` per
`ADDCYC`); `Cpu::run` no longer adds `consumed` to them. Result:
instr_v5 100-frame trace fully identical.

### 1.3 P-register semantics (diagnostic noise, now byte-parity)

Register dumps showed `p=6F` (Rust) vs `p=7F` (C++) — a persistent B-flag
difference, plus later U-flag differences. C++ `_P=POP()` on PLP/RTI
restores B/U from the stack and RESET wipes P to exactly `I_FLAG`; Rust
masked B/U and set `I|U` on RESET. These bits are behaviourally inert
(push paths explicitly set/clear B; nothing reads the stored value), but
they broke byte-parity of the observable register file (debugger/Lua) and
polluted every diagnostic.

Fix: PLP/RTI assign the full popped byte (`_P=POP()` parity, RTI also
refreshes `moo_pi`); RESET sets `p = IRQ_DIS` exactly. `nestest.rs` now
compares P via the classic display convention `(P & ~B) | U`, and
`interrupts.rs` asserts the C++ RESET value. Result: all 244,254 register
trace lines identical.

### 1.4 Page-cross extra-cycle loss (the dominant residual)

The 300-frame instr_v5 trace diverged at row 129238 (frame 135): the
per-call `cum_count` differed by exactly 48 units (1 cycle). A
per-instruction count trace (`[ins] pc op cnt`) pinned it to
`$E305 op=D9` (CMP abs,Y): Rust charged 4 cycles, C++ 5. Root cause:
`do_alu_a`, `do_compare`, and the unofficial LAX/LAS used
`abs_x_read(state, bus).addr` etc., discarding `extra_cycles`; only
`OpKind::Load` and `OpKind::NopRead` propagated them. The `$E303`-`$E30C`
loop page-crosses on every `D9`, so the deficit accumulated one cycle per
iteration until the call boundary shifted.

Fix: `do_alu_a` / `do_compare` return the addressing-mode `extra_cycles`
(0 for non-indexed), and LAX / LAS accumulate them. Result: full
300-frame trace byte-identical.

## 2. Diagnostics used (all env-gated, stripped before commit)

- `FCEUX11_CYCLE_LOG` (kept — test infra): per-call CSV
  (`frame,call_idx,cycles_arg,pc_after,cum_count,irq_low`), diffed via
  `tools/find_irq_full.py`.
- `FCEUX11_LOG_REG` (stripped): per-instruction register dump, PC-gated.
- `FCEUX11_LOG_INS` (stripped): per-instruction count residual, ROM-gated.
- `FCEUX11_LOG_FRAME` (stripped): frame-scoped bus-read log.

Tooling notes for the next session: the Rust `std::env::var` gates were
cached (raw per-instruction env lookups made the ON build ~6x slower),
and stderr redirection on Windows must use cmd-native `2>file` — a
PowerShell `2>` pipe deadlocks once the pipe buffer fills.

## 3. Honest remainder

- **`rom_regression_rust_smoke`**: 780 frames, 1 mismatch.
- **`savestate_regression_rust_smoke`**: 12 ROMs, 1 mismatch.
  Both are one small step from green but unlocated; the direct-smoke /
  cycle-trace tooling above should find them quickly (next task).
- **`kagami_qa_direct_smoke` blargg FAILs (11)**: every failing ROM is in
  `tests/fixtures/blargg_known_fail.json` (the OFF-build baseline fails
  them too) — not Rust regressions. Closing them is Phase 6/7 accuracy
  work (PPU VBL/NMI timing, MMC3, APU edge cases), not cycle parity.
- **`bench_tolerance_test`**: performance-baseline check that fails on
  this machine (+~27% vs a fast-machine baseline); not a correctness gate.
- **`mapper_byte_diff_rust_smoke`**: last CTest, result not captured this
  pass (run was cut off); re-run next session.
- The full 177-ROM blargg batch was re-run at commit time (correct CWD is
  `tests/` — the manifest paths are relative to it); diff against the
  known-fail baseline is in the commit message if captured.
