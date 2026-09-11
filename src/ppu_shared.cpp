// ppu_shared.cpp
//
// FCEUX11 v2.1.1.7 Step B.2 (2026-09-11) - symbols that the C++ PPU used to
// own but that no longer carry engine behaviour.
//
// FCEUPPU_LineUpdate() flushed the in-progress scanline
// (RefreshLine(GETLASTPIXEL)) so a mapper CHR/NT bank switch or an input
// hook landing mid-line would not render half a line from the old bank.
// It only ever ran when newppu == 0 AND Pline != 0, and Pline is written
// exclusively by the retired C++ render pipeline (ResetRL / DoLine / EndRL
// in ppu_rendering.cpp), which the Rust engine never enters.
//
// B.2 removed the seven non-engine call sites listed in
// docs/plans/v2.1.1.7_cpp_ppu_removal.md section B.2 (cart.cpp x4,
// input/zapper.cpp, input/shadow.cpp, ppu_class.cpp), the eight inside
// ppu.cpp retired register handlers, and the one inside ResetRL, then moved
// the definition here as an empty function.
//
// Probe evidence (docs/history/v2.1.1.7_b2_line_update.md): with the old body
// instrumented, ppu_frame_diff_test (nrom/mmc1/mmc3/mmc5/vrc6),
// rom_regression_rust_smoke (780 frames) and mapper_mmc3_byte_diff all
// printed
//     B2PROBE first call: newppu=0 Pline=0000000000000000
// and never the FIRED line, i.e. the flush never executed. Deleting the
// call sites is therefore behaviour-neutral.
//
// The symbol stays until Step C (M4) retires the C++ PPU; nothing calls it
// any more, it is only declared in ppu.h.

#include "types.h"
#include "ppu.h"

void FCEUPPU_LineUpdate(void) {
	// Rust PPU engine: no-op. The renderer consumes the CHR / NT window
	// copies that ppu_rust_bridge.cpp re-installs at scanline granularity,
	// so a mid-line bank switch needs no flush.
}