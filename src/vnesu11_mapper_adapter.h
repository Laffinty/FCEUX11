// vNESU11 mapper adapter — Phase 5 stage 1.
//
// Bridges the C++ mapper registration API (fceu.cpp::SetReadHandler /
// SetWriteHandler, used by all ~250 board files) into the vNESU11 Rust
// per-range handler table (ADR-010). When VNESU11_CORE=ON the Rust
// range table becomes the read/write authority for the mapper region;
// the C++ ARead[]/BWrite[] tables are still written so the newppu=0
// fallback path keeps working (double-write strategy, phase_5 §2.4).
//
// The Rust side only sees wrapper thunks + a stable `ctx` per range —
// the original C++ `readfunc`/`writefunc` are recovered from the ctx,
// and Game Genie / cheat wrapping stays in fceu.cpp (the Rust side
// never sees the unwrapped handler).

#pragma once

#include <stdint.h>

#include "types.h"  // readfunc / writefunc

namespace fceu11 {

// Forward a read range registration into the vNESU11 Rust range table.
// Called from fceu.cpp::SetReadHandler when VNESU11_CORE=ON. No-op
// when the Rust core is not enabled (or the SoC is not created).
void vnesu11_forward_set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept;

// Forward a write range registration (see above).
void vnesu11_forward_set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept;

// Prepare the vNESU11 SoC for a newly loaded game: clear the previous
// game's handler ranges + thunk pool, set the system type (C++ `EGIT`
// encoding: GIT_CART/GIT_VSUNI/GIT_FDS/GIT_NSF), and attach the mapper
// meta vtable (mirroring / audio / IRQ / savestate). Called from
// fceu.cpp::LoadGameVirtual *before* PowerNES() so the mapper's
// Power() registrations flow straight into the Rust table.
void vnesu11_on_game_load(int system_type) noexcept;

}  // namespace fceu11
