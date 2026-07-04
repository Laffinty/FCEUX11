// NSF load helper â€?extracted from nsf.cpp for v1.10 Cryptex Phase B.2.
// Contains the core NSFLoad logic using Rust FFI, called from the thin wrapper in nsf.cpp.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "nsf.h"
#include "utils/memory.h"
#include "file.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

// Forward declarations for globals defined in nsf.cpp
extern uint8 *NSFDATA;
extern int32 NSFSize;
extern int NSFMaxBank;
extern uint8 BSon;
extern uint16 LoadAddr;
extern uint16 InitAddr;
extern uint16 PlayAddr;
extern NSF_HEADER NSFHeader;

// Forward declaration for NSFROM patch
extern uint8 NSFROM[0x30+6];

// Main NSF load logic â€?called from thin wrapper in nsf.cpp
int NSFLoadCore(const char *name, FCEUFILE *fp) {
	// Read entire file into memory
	EMUFILE_MEMORY* ms = fp->EnsureMemorystream();
	const uint8_t* file_data = reinterpret_cast<const uint8_t*>(ms->buf());
	size_t file_size = ms->size();

	// Parse via Rust FFI
	FceuNsfCartResult cart;
	if (!fceux11_rust_nsf_load(file_data, file_size, &cart))
		return LOADER_INVALID_FORMAT;

	// Store header for later use
	memcpy(&NSFHeader, file_data, 0x80);

	// Set global variables
	LoadAddr = cart.load_addr;
	InitAddr = cart.init_addr;
	PlayAddr = cart.play_addr;
	NSFMaxBank = cart.max_bank;
	BSon = cart.bank_switch ? 1 : 0;
	NSFSize = cart.nsf_size;

	// Allocate NSF data memory
	if (!(NSFDATA = (uint8*)FCEU_malloc(NSFMaxBank * 4096))) {
		FCEU_PrintError("Unable to allocate memory.");
		return LOADER_HANDLED_ERROR;
	}

	// Copy NSF data
	memset(NSFDATA, 0x00, NSFMaxBank * 4096);
	memcpy(NSFDATA + (LoadAddr & 0xFFF), cart.nsf_data, cart.nsf_size);
	NSFMaxBank--;

	// Set game info
	GameInfo->type = GIT_NSF;
	GameInfo->input[0] = GameInfo->input[1] = static_cast<ESI>(SI_GAMEPAD);
	GameInfo->cspecial = SIS_NSF;

	// Patch NSFROM
	if (!fceux11_rust_nsf_patch_nsfrom(NSFROM, sizeof(NSFROM), InitAddr, PlayAddr))
		return LOADER_HANDLED_ERROR;

	// Set video system
	if (cart.video_system == 0) GameInfo->vidsys = GIV_NTSC;
	else if (cart.video_system == 1) GameInfo->vidsys = GIV_PAL;

	// Log info
	FCEU_printf("\nNSF Loaded.\nFile information:\n");
	FCEU_printf(" Name:       %s\n Artist:     %s\n Copyright:  %s\n\n",
		cart.song_name, cart.artist, cart.copyright);

	if (cart.sound_chip) {
		uint8 chip_mask = 0;
		const char* chip_name = fceux11_rust_nsf_chip_name(cart.sound_chip, &chip_mask);
		if (chip_name) {
			FCEU_printf(" Expansion hardware:  %s\n", chip_name);
			// Update sound_chip in header
			NSFHeader.SoundChip = chip_mask;
		}
	}

	if (BSon) FCEU_printf(" Bank-switched.\n");
	FCEU_printf(" Load address:  $%04x\n Init address:  $%04x\n Play address:  $%04x\n",
		LoadAddr, InitAddr, PlayAddr);
	FCEU_printf(" %s\n", (cart.video_system & 1) ? "PAL" : "NTSC");
	FCEU_printf(" Starting song:  %d / %d\n\n", cart.starting_song, cart.total_songs);

	// Set video system
	fceu11::SetVidSystem(cart.video_system);

	return LOADER_OK;
}
