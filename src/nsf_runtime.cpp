// NSF runtime bridge — v1.10 Cryptex Task 2.
// Thin forwarders to Rust NsfRuntimeState.  All FSM logic lives in Rust.
// Formerly 226 lines of C++; now ~140 lines of bridge code.

#include "types.h"
#include "cpu.h"
#include "fceu.h"
#include "cart.h"
#include "nsf.h"
#include "utils/memory.h"
#include "state.h"
#include "rust/fceux11_rust.h"
#include <cstring>

// ── NSFROM boot ROM (patched at load, copied to Rust at init) ────────
uint8 NSFROM[0x30+6] = {
	0x8D,0xF4,0x3F, 0xA2,0xFF,0x9A, 0xAD,0xF0,0x3F,
	0xF0,0x09,      0xAD,0xF1,0x3F, 0xAE,0xF3,0x3F,
	0x20,0x00,0x00, 0xA9,0x00, 0xAA, 0xA8,
	0x20,0x00,0x00, 0x8D,0xF5,0x3F,
	0x90,0xFE,
	0x8D,0xF3,0x3F, 0x18, 0x90,0xFE
};

// ── Globals ───────────────────────────────────────────────────────────
uint8 SongReload;
int32 CurrentSong;
int special = 0;
static NsfRuntimeState *g_nsf_state = nullptr;
static FceuMallocPtr ExWRAM_owner;
static uint8 *ExWRAM_ptr = nullptr;

void nsf_allocate_exwram(void) {
	ExWRAM_owner = FCEU_gmalloc_unique(32768 + 8192);
	ExWRAM_ptr = ExWRAM_owner.get();
}
void nsf_free_exwram(void) {
	if (ExWRAM_ptr) { ExWRAM_owner.reset(); ExWRAM_ptr = nullptr; }
}

void nsf_runtime_create(void) {
	if (!g_nsf_state) g_nsf_state = fceux11_rust_nsf_runtime_create();
}
void nsf_runtime_destroy(void) {
	if (g_nsf_state) { fceux11_rust_nsf_runtime_destroy(g_nsf_state); g_nsf_state = nullptr; }
}

// Sound chip forward declarations (C++ linkage)
void NSFVRC6_Init(void); void NSFVRC7_Init(void); void NSFMMC5_Init(void);
void NSFN106_Init(void);  void NSFAY_Init(void);   void FDSSoundReset(void);

// ── Callback adapters (bridge C++ → Rust callback table) ─────────────
extern "C" {
static void cb_set_prg4(uint32_t A, uint32_t bank)             { setprg4(A, bank); }
static void cb_set_prg8(uint32_t A, uint32_t bank)             { setprg8(A, bank); }
static void cb_set_prg8r(int32_t r, uint32_t A, uint32_t b)   { setprg8r(r, A, b); }
static void cb_set_prg32(uint32_t A, uint32_t bank)            { setprg32(A, bank); }
static void cb_setup_cart_prg_mapping(int32_t c, const uint8_t *d, uint32_t s, int32_t w)
	{ SetupCartPRGMapping(c, const_cast<uint8_t*>(d), s, w); }
static void cb_reset_cart_mapping(void)     { ResetCartMapping(); }
static void cb_bus_write(uint16_t a, uint8_t v)  { fceu11::g_bus.write(a, v); }
static void cb_trigger_nmi(void)            { TriggerNMI(); }
static void cb_sound_chip_init(uint8_t sc) {
	if (sc & 1) NSFVRC6_Init(); else if (sc & 2) NSFVRC7_Init();
	else if (sc & 4) FDSSoundReset(); else if (sc & 8) NSFMMC5_Init();
	else if (sc & 0x10) NSFN106_Init(); else if (sc & 0x20) NSFAY_Init();
}
} // extern "C"

// ── Thin DECLFR/DECLFW handlers → Rust FFI ───────────────────────────
static DECLFR(NSFROMRead)   { return fceux11_rust_nsf_from_read(g_nsf_state, A); }
static DECLFW(NSF_write)    { fceux11_rust_nsf_write(g_nsf_state, A, V); }
static DECLFR(NSF_read)     { return fceux11_rust_nsf_read(g_nsf_state, A, PAL, fceuindbg, RAM); }
static DECLFR(NSFVectorRead){
	uint8_t val;
	if (fceux11_rust_nsf_vector_read(g_nsf_state, A, &val)) return val;
	return CartBR(A);
}

// ── NSF_init ─────────────────────────────────────────────────────────
void NSF_init(void) {
	NsfRuntimeCallbacks cb = {};
	cb.set_prg4 = cb_set_prg4;   cb.set_prg8 = cb_set_prg8;
	cb.set_prg8r = cb_set_prg8r; cb.set_prg32 = cb_set_prg32;
	cb.setup_cart_prg_mapping = cb_setup_cart_prg_mapping;
	cb.reset_cart_mapping = cb_reset_cart_mapping;
	cb.bus_write = cb_bus_write; cb.trigger_nmi = cb_trigger_nmi;
	cb.sound_chip_init = cb_sound_chip_init;

	extern uint8 *NSFDATA; extern int NSFMaxBank; extern uint8 BSon;
	extern uint16 LoadAddr; extern NSF_HEADER NSFHeader;

	fceux11_rust_nsf_runtime_configure(g_nsf_state,
		NSFDATA, NSFMaxBank, BSon, NSFHeader.SoundChip,
		ExWRAM_ptr, NSFHeader.BankSwitch, LoadAddr, NSFROM, sizeof(NSFROM), &cb);
	fceux11_rust_nsf_runtime_init(g_nsf_state, NSFHeader.StartingSong);

	// Handler registration (requires C++ static function pointers)
	if (NSFHeader.SoundChip & 4) {
		SetWriteHandler(0x6000, 0xDFFF, CartBW);
		SetReadHandler(0x6000, 0xFFFF, CartBR);
	} else {
		SetReadHandler(0x6000, 0x7FFF, CartBR);
		SetWriteHandler(0x6000, 0x7FFF, CartBW);
		SetReadHandler(0x8000, 0xFFFF, CartBR);
	}
	SetReadHandler(0xFFFA, 0xFFFD, NSFVectorRead);
	SetWriteHandler(0x2000, 0x3fff, 0); SetReadHandler(0x2000, 0x37ff, 0);
	SetReadHandler(0x3836, 0x3FFF, 0);   SetReadHandler(0x3800, 0x3835, NSFROMRead);
	SetWriteHandler(0x5ff6, 0x5fff, NSF_write);
	SetWriteHandler(0x3ff0, 0x3fff, NSF_write);
	SetReadHandler(0x3ff0, 0x3fff, NSF_read);

	static SFORMAT StateRegs[] = {
		{ &SongReload, 1, "SREL" }, { &CurrentSong, 4 | FCEUSTATE_RLSB, "CURS" },
		{ &g_nsf_state->doreset, 1, "DORE" }, { &g_nsf_state->nsf_nmi_flags, 1, "NMIF" },
		{ nullptr, 0, nullptr }
	};
	AddExState(StateRegs, ~0, 0, 0);
	AddExState(ExWRAM_ptr, 32768 + 8192, 0, "ERAM");
}

// ── DoNSFFrame ───────────────────────────────────────────────────────
void DoNSFFrame(void) {
	if (fceux11_rust_nsf_frame(g_nsf_state)) TriggerNMI();
}
