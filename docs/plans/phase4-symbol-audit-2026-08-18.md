# Phase 4 sub-step 6 — Public symbol audit (x6502.h vs fceux11_rust.h)

Date: 2026-08-18. Sub-step 6 part 4 of `docs/plans/cpu-rust-v2.md`.

Under `FCEUX11_RUST_CPU=ON` the Rust CPU replaces the C++ X6502 dispatch
loop entirely. The `x6502.h` API is therefore not consumed at runtime
in that build mode. This document maps every public symbol in
`src/x6502.h` to its replacement so sub-step 7 (delete the C++ CPU)
can do so without losing functionality.

Status legend:
- **OK FFI** — exported via `fceux11_rust.h` (cbindgen-emitted).
- **OK Rust** — available via the Rust `fceux11_core::cpu::*` API
  (consumed by tests and by the FFI shim).
- **OK inline** — same constant / inline accessor as the C++ side; no
  FFI surface needed because the value is compile-time.
- **DEFER** — intentionally not exported; defer until the C++ CPU is
  fully removed (sub-step 7) and any last consumer is migrated.
- **REMOVE** — only meaningful with the C++ CPU; the C++ side is
  being deleted, so the symbol is removed with it (sub-step 7).

## x6502.h public symbols

### Table data

| Symbol                          | Status     | Replacement |
|---------------------------------|------------|-------------|
| `extern const uint8 opsize[256]` | OK Rust    | `fceux11_core::cpu::decode::OP_SIZE: [u8; 256]` |
| `extern const uint8 optype[256]` | DEFER      | Equivalent info available per-opcode via `OpcodeInfo.kind`; no C-array exposure. |
| `extern const uint8 opwrite[256]`| DEFER      | Write-back semantics are baked into the per-`OpKind` handlers in `execute.rs`; no table needed. |

### Lifecycle hooks

| Symbol                          | Status     | Replacement |
|---------------------------------|------------|-------------|
| `X6502_Debug(hook)`             | REMOVE     | C++-only debug hook; not used in FCEUX11_RUST_CPU=ON builds. The hot path doesn't call it. |
| `X6502_DMW(A, V)`               | REMOVE     | C++ debug write (bypasses bus side-effects). Only used by debug tooling that is itself C++. |

### Dispatch loop

| Symbol                          | Status     | Replacement |
|---------------------------------|------------|-------------|
| `X6502_RunDebug(cpu, cycles)`   | OK FFI     | `fceux11_cpu_run(state, cycles)` (calls `fceux11_core::cpu::execute::run`). |

### Cpu object members

| Symbol                              | Status     | Replacement |
|-------------------------------------|------------|-------------|
| `timestamp`                         | OK inline  | Stays on the C++ `fceu11::Cpu`; the FFI `fceux11_cpu_run` returns consumed cycles and the C++ shim does `timestamp_ += consumed`. Rust never reads it directly. |
| `soundtimestamp`                    | OK inline  | Same pattern. |
| `scanline`                          | OK inline  | Stays on the C++ `fceu11::Cpu`; not consumed by Rust. |

### Flag constants

| Symbol                | Status      | Replacement |
|-----------------------|-------------|-------------|
| `N_FLAG` (`0x80`)     | OK Rust     | `Flags::NEGATIVE` (`bitflags!`) |
| `V_FLAG` (`0x40`)     | OK Rust     | `Flags::OVERFLOW` |
| `U_FLAG` (`0x20`)     | OK Rust     | `Flags::UNUSED` |
| `B_FLAG` (`0x10`)     | OK Rust     | `Flags::BREAK` |
| `D_FLAG` (`0x08`)     | OK Rust     | `Flags::DECIMAL` |
| `I_FLAG` (`0x04`)     | OK Rust     | `Flags::IRQ_DIS` |
| `Z_FLAG` (`0x02`)     | OK Rust     | `Flags::ZERO` |
| `C_FLAG` (`0x01`)     | OK Rust     | `Flags::CARRY` |

### NTSC / PAL

| Symbol            | Status     | Replacement |
|-------------------|------------|-------------|
| `dendy`           | REMOVE     | C++-only timing config; unused by Rust CPU. |
| `NTSC_CPU_freq()` | REMOVE     | Same. |
| `PAL_CPU`         | REMOVE     | Same. |

### IRQ source bitmask constants

| Symbol                 | Status     | Replacement |
|------------------------|------------|-------------|
| `FCEU_IQEXT   (0x001)` | OK Rust    | `IrqSource::EXTERNAL` |
| `FCEU_IQEXT2  (0x002)` | OK Rust    | `IrqSource::EXTERNAL2` |
| `FCEU_IQRESET (0x020)` | OK Rust    | `IrqSource::RESET` |
| `FCEU_IQNMI2  (0x040)` | OK Rust    | `IrqSource::NMI2` |
| `FCEU_IQNMI   (0x080)` | OK Rust    | `IrqSource::NMI` |
| `FCEU_IQDPCM  (0x100)` | OK Rust    | `IrqSource::DPCM` |
| `FCEU_IQFCOUNT(0x200)` | OK Rust    | `IrqSource::FRAME` |
| `FCEU_IQTEMP  (0x800)` | OK Rust    | `IrqSource::TEMP` |

### Lifecycle

| Symbol               | Status     | Replacement |
|----------------------|------------|-------------|
| `X6502_Init()`       | OK FFI     | `fceux11_cpu_init` |
| `X6502_Reset()`      | OK FFI     | `fceux11_cpu_reset` |
| `X6502_Power()`      | OK FFI     | `fceux11_cpu_power` |
| `TriggerNMI()`       | OK FFI     | `fceux11_cpu_trigger_nmi` |
| `TriggerNMI2()`      | OK FFI     | `fceux11_cpu_trigger_nmi2` |
| `X6502_IRQBegin(w)`  | OK FFI     | `fceux11_cpu_irq_begin(state, w)` |
| `X6502_IRQEnd(w)`    | OK FFI     | `fceux11_cpu_irq_end(state, w)` |

### Helpers

| Symbol                          | Status     | Replacement |
|---------------------------------|------------|-------------|
| `X6502_GetOpcodeCycles(op)`     | OK Rust    | `fceux11_core::cpu::cycle_count(op: u8) -> u8` (returns CycTable[op]) |
| `TriggerNMI / TriggerNMI2`      | OK FFI     | (see Lifecycle) |

## Summary

- **OK FFI**: 7 lifecycle + 1 dispatch + 4 NMI/IRQ control (already
  present in `fceux11_rust.h`).
- **OK Rust**: 8 flag + 8 IRQ source + 1 opcode-cycle helper (already
  available via the `fceux11_core::cpu` module).
- **OK inline**: `timestamp`/`soundtimestamp`/`scanline` (stays on
  C++ side; Rust consumes only via the FFI return value).
- **DEFER**: 3 table-data symbols (`optype`, `opwrite`, plus `opsize`
  duplicated via Rust API). None are FFI-required today; revisit
  only if a future C++ consumer needs them.
- **REMOVE**: 5 symbols (`X6502_Debug`, `X6502_DMW`, `dendy`,
  `NTSC_CPU_freq`, `PAL_CPU`). All are C++-only debug or timing
  config that the Rust CPU does not consume. Removal happens with
  sub-step 7 (delete `src/x6502.cpp` / `src/ops.inc` / etc.).

## Notes on the 64-byte `X6502Layout`

The C++ `X6502` struct (the savestate layout) is NOT in `x6502.h`
itself but is the contract the FFI operates on. The Rust `X6502Layout`
in `src/rust/.../cpu/state.rs` is pinned to the same byte offsets by
`offset_of!` asserts and matches the C++ `static_assert`s in
`src/cpu.cpp:11-22`. Any future layout change must update both.

## Action items before sub-step 7

1. Decide whether `optype` / `opwrite` need to remain in any
   C++ consumer. If yes, expose a Rust-side accessor via the FFI;
   if no, document the deprecation in `ChangeLog.md` and remove
   the C++ table when sub-step 7 deletes `ops.inc`.
2. Move `x6502.h` to `src/x6502.h.disabled` (or equivalent) once
   sub-step 7 removes the last C++ consumer, since `X6502_RunDebug`
   in OFF mode is the only remaining caller.