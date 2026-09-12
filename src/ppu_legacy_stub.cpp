// ppu_legacy_stub.cpp
//
// FCEUX11 v2.1.1.7 Step C (M4, 2026-09-12) - plan section C.1.
//
// The C++ PPU engine that used to own these symbols was deleted in the same
// batch (src/ppu.cpp, src/ppu_rendering.cpp, src/ppu_core.cpp). What is left
// here are pure placeholders:
//
//   * they keep the legacy headers linkable and the historical power-on
//     bytes available to anything that still takes their address,
//   * their initial values match the pre-Step-C state,
//   * they MUST NOT be read for behaviour - the Rust PPU owns the raster,
//     sprite evaluation, the $2005/$2006 write toggle and the reset arming
//     (plan section 0.1).
//
// The savestate tables in src/ppu_state.cpp no longer serialise these
// objects: since Step B.1 they point at the bridge staging block in
// src/ppu_bridge_state.cpp. The chunk-31 labels (IDLS / SR_* / SRx* / PFVx /
// PVxx / ...) survive through that staging block, not through these bytes.
//
// See docs/history/v2.1.1.7_c1_cpp_ppu_deletion.md for the batch record.

#include "types.h"
#include "ppu.h"        // SPRB / SPRBUF
#include "ppu_core.h"   // PPUREGS / SPRITE_READ

// chunk-31 "IDLS" source. The live value is Rust-owned.
uint8 idleSynch = 1;

// chunk-31 "SR_*" / "SRx*" source: the retired sprite-evaluation scratch.
SPRITE_READ spr_read;

// chunk-31 "PFVx"/"PVxx"/"PHxx"/"PVTx"/"PHTx"/"P_FV".."PST2" source: the
// retired $2005/$2006 daisy-chain register file.
PPUREGS ppur;

// FCEUPPU_Reset() arms this so the retired per-frame reset ran once at the
// start of the next frame. Nothing consumes it any more.
bool new_ppu_reset = false;

// Sprite descriptor scratch of the retired FetchSpriteData / RefreshSprites
// pair. tests/kagami/ppu_phase_c_test.cpp still pins its 4-byte layout.
alignas(64) SPRB SPRBUF[64];

// GETLASTPIXEL reference point read by the Qt debugger
// (src/drivers/Qt/ConsoleDebugger.cpp). The C++ renderer was the only writer
// and the Rust engine does not update it, so the debugger's pixel estimate
// stays at the power-on value - a display-only regression recorded in the
// Step C batch note (M5 owner QA item).
int linestartts = 0;