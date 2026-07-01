// UNIF load helper — extracted from unif.cpp for v1.10 Cryptex Phase B.1.
// Contains the core UNIFLoad logic using Rust FFI, called from the thin wrapper in unif.cpp.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "unif.h"
#include "ines.h"
#include "utils/memory.h"
#include "state.h"
#include "file.h"
#include "driver.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

// Forward declarations for globals defined in unif.cpp
extern uint8 *UNIFchrrama;
extern uint8 *ROM;
extern uint8 *VROM;

// Forward declarations for functions in unif.cpp
extern void FreeUNIF(void);
extern void ResetUNIF(void);
extern void MooMirroring(void);
extern int InitializeBoard(void);

// Main UNIF load logic — called from thin wrapper in unif.cpp
int UNIFLoadCore(const char *name, FCEUFILE *fp, CartInfo& UNIFCart) {
	// Read entire file into memory
	EMUFILE_MEMORY* ms = fp->EnsureMemorystream();
	const uint8_t* file_data = ms->buf();
	size_t file_size = ms->size();

	// Parse via Rust FFI
	FceuUnifCartResult cart;
	if (!fceux11_rust_unif_load(file_data, file_size, &cart))
		return LOADER_INVALID_FORMAT;

	ResetCartMapping();
	ResetExState(0, 0);
	ResetUNIF();

	// Copy PRG banks to emulator-owned memory
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

	// Copy CHR banks to emulator-owned memory
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

	// Set board name
	if (cart.board_name && cart.board_name_len > 0) {
		// Store board name for InitializeBoard
		extern uint8 *boardname;
		extern uint8 *sboardname;
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

	// Set mirroring
	extern int mirrortodo;
	mirrortodo = cart.mirroring;
	MooMirroring();

	// Set battery
	UNIFCart.battery = cart.battery ? 1 : 0;

	// Set TV system
	if (cart.tv_system == 0) {
		GameInfo->vidsys = GIV_NTSC;
		fceu11::SetVidSystem(0);
	} else if (cart.tv_system == 1) {
		GameInfo->vidsys = GIV_PAL;
		fceu11::SetVidSystem(1);
	}

	// Compute MD5
	memcpy(UNIFCart.MD5, cart.md5, 16);
	memcpy(&GameInfo->MD5, cart.md5, 16);

	FCEU_printf(" ROM MD5:  0x");
	for (int x = 0; x < 16; x++)
		FCEU_printf("%02x", UNIFCart.MD5[x]);
	FCEU_printf("\n");

	// Initialize board (mapper)
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
