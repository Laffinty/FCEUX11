// ppu_bridge_state.cpp
//
// FCEUX11 v2.1.1.7 Step B.1 (D1-A). See ppu_bridge_state.h for the
// contract and docs/history/v2.1.1.7_cpp_ppu_removal_archived_2026-09-12.md section B.1 for
// the design rationale.

#include "types.h"
#include "ppu.h"
#include "ppu_state.h"        // kook / ppudead / PPUSPL (tombstone mirrors)
#include "ppu_rust_bridge.h"  // ppu_rust_bridge_get_state + window helpers
#include "ppu_bridge_state.h"  // staging storage + PST0/PST1 slot indices

#ifdef FCEUX11_RUST_PPU
#include "rust/fceux11_rust.h"  // fceux11_ppu_state_block_export / _apply
#endif

#include <cstdint>
#include <cstdio>
#include <cstring>

// Must equal STATE_BLOCK_SIZE in
// src/rust/crates/fceux11-ppu/src/ffi.rs; the byte layout is documented
// in docs/history/v2.1.1.7_b1_batch1.md section 1.2.
static constexpr uint32_t kPpuStateBlockSize = 272;

// ---------------------------------------------------------------------------
// Staging storage - chunk-3 / chunk-31 shapes (see ppu_bridge_state.h).
// ---------------------------------------------------------------------------
uint8_t  bridge_ppu_regs[4];
uint8_t  bridge_oam[0x100];
int      bridge_kook;
int      bridge_ppudead;
uint8_t  bridge_ppuspl;
uint8_t  bridge_xoffset;
uint8_t  bridge_vtoggle;
uint16_t bridge_refresh_addr;
uint16_t bridge_temp_addr;
uint8_t  bridge_vram_buffer;
uint8_t  bridge_ppu_gen_latch;
uint8_t  bridge_newppu_idle_synch;
int32_t  bridge_newppu_spr_slots[15];
int32_t  bridge_newppu_ppur_slots[15];

#ifdef FCEUX11_RUST_PPU

namespace {

// Byte-block offsets - keep in sync with ffi.rs (state_block_export).
enum : uint32_t {
	kOffRegs       = 0,
	kOffOam        = 4,
	kOffWriteTog   = 260,
	kOffVramBuffer = 261,
	kOffFineX      = 262,
	kOffDataBus    = 263,
	kOffV          = 264,
	kOffT          = 266,
	kOffScanline   = 268,
	kOffDot        = 270,
};

uint16_t ReadLe16(const uint8_t* p) {
	return static_cast<uint16_t>(p[0] | (p[1] << 8));
}

void WriteLe16(uint8_t* p, uint16_t v) {
	p[0] = static_cast<uint8_t>(v & 0xFF);
	p[1] = static_cast<uint8_t>((v >> 8) & 0xFF);
}

}  // namespace

void bridge_state_refresh_from_rust() {
	PpuState* state = ppu_rust_bridge_get_state();
	if (state == nullptr) {
		return;  // Rust PPU inactive: keep the previous staging contents.
	}

	uint8_t block[kPpuStateBlockSize];
	if (fceux11_ppu_state_block_export(state, block, kPpuStateBlockSize) != 0) {
		std::fprintf(stderr,
			"ppu_bridge_state: state_block_export failed; staging keeps its previous contents\n");
		return;
	}

	std::memcpy(bridge_ppu_regs, block + kOffRegs, sizeof(bridge_ppu_regs));
	std::memcpy(bridge_oam, block + kOffOam, sizeof(bridge_oam));
	bridge_vtoggle       = block[kOffWriteTog];
	bridge_vram_buffer   = block[kOffVramBuffer];
	bridge_xoffset       = block[kOffFineX];
	bridge_ppu_gen_latch = block[kOffDataBus];

	// NES "v" is the live VRAM address ($2007 auto-increment); NES "t" is
	// the scroll / $2006 double-write latch. FCEUX calls them RefreshAddr
	// and TempAddr, and its SFORMAT labels are RADD / TADD in that same
	// order. Do not swap the two.
	bridge_refresh_addr = ReadLe16(block + kOffV);
	bridge_temp_addr    = ReadLe16(block + kOffT);

	bridge_newppu_ppur_slots[bridge_newppu_pst0] =
		static_cast<int32_t>(static_cast<int16_t>(ReadLe16(block + kOffScanline)));
	bridge_newppu_ppur_slots[bridge_newppu_pst1] =
		static_cast<int32_t>(ReadLe16(block + kOffDot));

	// KOOK / DEAD / PSPL have no Rust counterpart. Mirror the C++
	// tombstones so the saved bytes stay identical to the pre-B.1 engine;
	// they are not read back on load (see the apply path below).
	bridge_kook    = kook;
	bridge_ppudead = ppudead;
	bridge_ppuspl  = PPUSPL;
}

void bridge_state_apply_to_rust() {
	PpuState* state = ppu_rust_bridge_get_state();
	if (state == nullptr) {
		return;
	}

	uint8_t block[kPpuStateBlockSize] = {};
	std::memcpy(block + kOffRegs, bridge_ppu_regs, sizeof(bridge_ppu_regs));
	std::memcpy(block + kOffOam, bridge_oam, sizeof(bridge_oam));
	block[kOffWriteTog]   = bridge_vtoggle;
	block[kOffVramBuffer] = bridge_vram_buffer;
	block[kOffFineX]      = bridge_xoffset;
	block[kOffDataBus]    = bridge_ppu_gen_latch;
	WriteLe16(block + kOffV, bridge_refresh_addr);
	WriteLe16(block + kOffT, bridge_temp_addr);
	WriteLe16(block + kOffScanline,
		static_cast<uint16_t>(bridge_newppu_ppur_slots[bridge_newppu_pst0] & 0xFFFF));
	WriteLe16(block + kOffDot,
		static_cast<uint16_t>(bridge_newppu_ppur_slots[bridge_newppu_pst1] & 0xFFFF));

	if (fceux11_ppu_state_block_apply(state, block, kPpuStateBlockSize) != 0) {
		std::fprintf(stderr,
			"ppu_bridge_state: state_block_apply failed; Rust PPU keeps its previous state\n");
		return;
	}

	// The Rust renderer reads window COPIES and the bridge caches the
	// mirror mode, so both must be re-installed after a load - otherwise
	// the first frame after the load renders stale CHR/NT/palette data.
	ppu_rust_bridge_refresh_windows();
	ppu_rust_bridge_push_mirror_mode_if_dirty();
}

#else  // !FCEUX11_RUST_PPU

void bridge_state_refresh_from_rust() {
	// Legacy build: the C++ PPU engine owns the savestate fields directly.
}

void bridge_state_apply_to_rust() {
	// Legacy build: there is no Rust PPU state to push.
}

#endif  // FCEUX11_RUST_PPU