// UNIF load helper �?extracted from unif.cpp for v1.10 Cryptex Phase B.1.
// Contains the core UNIFLoad logic using Rust FFI, board initialization,
// and cleanup helpers �?all moved from unif.cpp to keep the bridge < 100 lines.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "unif.h"
#include "ines.h"
#include "utils/memory.h"
#include "state.h"
#include "file.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

// Type definitions (mirrored in unif.cpp for the bridge layer)
typedef struct {
	const char *name;
	void (*init)(CartInfo *);
} BMAPPING;

// Board flags (mirrored from Rust unif.rs for C++ bridge use)
#define BMCFLAG_FORCE4    0x01

#include "unif_bmap.h"

// ── Local globals ───────────────────────────────────────────────────────────
static int mirrortodo;
static uint8 *boardname;
static uint8 *sboardname;
static uint32 CHRRAMSize;
static uint8 *malloced[32];
static uint32 mallocedsizes[32];
static uint8 exntar[2048];

// ── Externs from unif.cpp ───────────────────────────────────────────────────
extern uint8 *UNIFchrrama;
extern uint8 *ROM;
extern uint8 *VROM;
extern CartInfo UNIFCart;

// ── Cleanup & lifecycle ─────────────────────────────────────────────────────

// v1.13 Purify F2b: FCEU_malloc was used to allocate UNIFchrrama/boardname/malloced[]
// (lines 83, 138, see boards/); use matching FCEU_free() deallocator.
void FreeUNIF(void) {
	if (UNIFchrrama) { FCEU_free(UNIFchrrama); UNIFchrrama = 0; }
	if (boardname) { FCEU_free(boardname); boardname = 0; }
	for (int x = 0; x < 32; x++)
		if (malloced[x]) { FCEU_free(malloced[x]); malloced[x] = 0; }
}

void ResetUNIF(void) {
	for (int x = 0; x < 32; x++) malloced[x] = 0;
	boardname = 0; mirrortodo = 0;
	UNIFCart.clear(); UNIFchrrama = 0;
}

void MooMirroring(void) {
	if (mirrortodo < 0x4) SetupCartMirroring(mirrortodo, 1, 0);
	else if (mirrortodo == 0x4) {
		FCEU_MemoryRand(exntar, sizeof(exntar), true);
		SetupCartMirroring(4, 1, exntar);
		AddExState(exntar, 2048, 0, "EXNR");
	} else SetupCartMirroring(0, 0, 0);
}

int InitializeBoard(void) {
	if (!sboardname) return 0;
	int x = 0;
	while (bmap[x].name) {
		if (!strcmp((char*)sboardname, (char*)bmap[x].name)) {
			int flags = fceux11_rust_unif_board_flags((const char*)sboardname);
			if (flags < 0) return 1;
			if (!malloced[16]) {
				CHRRAMSize = fceux11_rust_unif_chrram_size(flags);
				if ((UNIFchrrama = (uint8*)FCEU_malloc(CHRRAMSize))) {
					SetupCartCHRMapping(0, UNIFchrrama, CHRRAMSize, 1);
					AddExState(UNIFchrrama, CHRRAMSize, 0, "CHRR");
				} else return 2;
			}
			if (flags & BMCFLAG_FORCE4) mirrortodo = 4;
			MooMirroring();
			bmap[x].init(&UNIFCart);
			return 0;
		}
		x++;
	}
	return 1;
}

// ── Core UNIF load ──────────────────────────────────────────────────────────

int UNIFLoadCore(const char *name, FCEUFILE *fp, CartInfo& UNIFCart) {
	EMUFILE_MEMORY* ms = fp->EnsureMemorystream();
	const uint8_t* file_data = reinterpret_cast<const uint8_t*>(ms->buf());
	size_t file_size = ms->size();

	FceuUnifCartResult cart;
	if (!fceux11_rust_unif_load(file_data, file_size, &cart))
		return LOADER_INVALID_FORMAT;

	ResetCartMapping();
	ResetExState(0, 0);
	ResetUNIF();

	for (int i = 0; i < 32; i++) {
		if (cart.prg[i].data && cart.prg[i].size > 0) {
			uint32 t = fceux11_rust_uppow2(cart.prg[i].size);
			if (t < 2048) t = 2048;
			uint8* buf = (uint8*)FCEU_malloc(t);
			if (!buf) return LOADER_HANDLED_ERROR;
			memset(buf + cart.prg[i].size, 0xFF, t - cart.prg[i].size);
			memcpy(buf, cart.prg[i].data, cart.prg[i].size);
			SetupCartPRGMapping(i, buf, t, 0);
		}
	}

	for (int i = 0; i < 32; i++) {
		if (cart.chr[i].data && cart.chr[i].size > 0) {
			uint32 t = fceux11_rust_uppow2(cart.chr[i].size);
			if (t < 8192) t = 8192;
			uint8* buf = (uint8*)FCEU_malloc(t);
			if (!buf) return LOADER_HANDLED_ERROR;
			memset(buf + cart.chr[i].size, 0xFF, t - cart.chr[i].size);
			memcpy(buf, cart.chr[i].data, cart.chr[i].size);
			SetupCartCHRMapping(i, buf, t, 0);
		}
	}

	if (cart.board_name && cart.board_name_len > 0) {
		boardname = (uint8*)FCEU_malloc(cart.board_name_len + 1);
		if (boardname) {
			memcpy(boardname, cart.board_name, cart.board_name_len);
			boardname[cart.board_name_len] = 0;
			sboardname = boardname;
			if (!memcmp(boardname, "NES-", 4) || !memcmp(boardname, "UNL-", 4) ||
				!memcmp(boardname, "HVC-", 4) || !memcmp(boardname, "BTL-", 4) ||
				!memcmp(boardname, "BMC-", 4))
				sboardname += 4;
		}
	}

	mirrortodo = cart.mirroring;
	MooMirroring();

	UNIFCart.battery = cart.battery ? 1 : 0;

	if (cart.tv_system == 0) {
		GameInfo->vidsys = GIV_NTSC;
		fceu11::SetVidSystem(0);
	} else if (cart.tv_system == 1) {
		GameInfo->vidsys = GIV_PAL;
		fceu11::SetVidSystem(1);
	}

	memcpy(UNIFCart.MD5, cart.md5, 16);
	memcpy(&GameInfo->MD5, cart.md5, 16);

	FCEU_printf(" ROM MD5:  0x");
	for (int x = 0; x < 16; x++)
		FCEU_printf("%02x", UNIFCart.MD5[x]);
	FCEU_printf("\n");

	int result = InitializeBoard();
	if (result != 0) {
		if (result == 1) FCEU_PrintError("UNIF mapper is not supported at all.");
		if (result == 2) FCEU_PrintError("Unable to allocate CHR-RAM.");
		FreeUNIF();
		ResetUNIF();
		fceu11::assign_cart(nullptr);
		return LOADER_HANDLED_ERROR;
	}

	return LOADER_OK;
}
