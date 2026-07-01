// v1.10 Cryptex: UNIF format bridge layer.
// Global variables and thin UNIFLoad wrapper.  Core logic lives in:
//   unif_load.cpp  — UNIFLoadCore (Rust FFI parsing + ROM setup)
//   unif_bmap.h    — BMAPPING bmap[] table

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "apu.h"
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

// Type definitions
typedef struct {
	const char *name;
	void (*init)(CartInfo *);
} BMAPPING;

typedef struct {
	const char *name;
	int (*init)(FCEUFILE *fp);
} BFMAPPING;

// Global variables
static CartInfo UNIFCart;
static int vramo;
static int mirrortodo;
static uint8 *boardname;
static uint8 *sboardname;
static uint32 CHRRAMSize;
uint8 *UNIFchrrama = 0;
static uint8 *malloced[32];
static uint32 mallocedsizes[32];

// Helper functions
void FreeUNIF(void) {
	if (UNIFchrrama) { free(UNIFchrrama); UNIFchrrama = 0; }
	if (boardname) { free(boardname); boardname = 0; }
	for (int x = 0; x < 32; x++)
		if (malloced[x]) { free(malloced[x]); malloced[x] = 0; }
}

void ResetUNIF(void) {
	for (int x = 0; x < 32; x++) malloced[x] = 0;
	vramo = 0; boardname = 0; mirrortodo = 0;
	UNIFCart.clear(); UNIFchrrama = 0;
}

static uint8 exntar[2048];

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

static void UNIFGI(GI h) {
	switch (h) {
	case GI_RESETSAVE: FCEU_ClearGameSave(&UNIFCart); break;
	case GI_RESETM2: if (UNIFCart.Reset) UNIFCart.Reset(); break;
	case GI_POWER:
		if (UNIFCart.Power) UNIFCart.Power();
		if (UNIFchrrama) memset(UNIFchrrama, 0, 8192);
		break;
	case GI_CLOSE:
		FCEU_SaveGameSave(&UNIFCart);
		if (UNIFCart.Close) UNIFCart.Close();
		FreeUNIF();
		break;
	}
}

// Include the bmap table
#include "unif_bmap.h"

// Forward declaration for core load function
extern int UNIFLoadCore(const char *name, FCEUFILE *fp, CartInfo& UNIFCart);

// Thin wrapper: UNIFLoad
int UNIFLoad(const char *name, FCEUFILE *fp) {
	FCEU_fseek(fp, 0, SEEK_SET);

	int result = UNIFLoadCore(name, fp, UNIFCart);
	if (result != LOADER_OK) return result;

	// Post-load setup
	FCEU_LoadGameSave(&UNIFCart);
	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name);
	GameInterface = UNIFGI;
	currCartInfo = &UNIFCart;

	if (auto cart = fceu11::create_cart_for_mapper(0, fceu11::g_bus)) {
		fceu11::assign_cart(std::move(cart));
		UNIFCart.cart_obj = fceu11::g_cart;
		UNIFCart.Power = CartInfo_PowerForward;
		UNIFCart.Reset = CartInfo_ResetForward;
		UNIFCart.Close = CartInfo_CloseForward;
	}

	if (currCartInfo && currCartInfo->cart_obj)
		currCartInfo->cart_obj->install_expansion_audio(fceu11::g_apu);

	if (fceu11::g_cart) {
		fceu11::g_cart->set_md5(UNIFCart.MD5);
		fceu11::g_cart->set_crc32(UNIFCart.CRC32);
		fceu11::g_cart->set_mirror(UNIFCart.mirror);
		fceu11::g_cart->set_mirror_as_2bits(UNIFCart.mirrorAs2Bits);
		fceu11::g_cart->set_battery(UNIFCart.battery != 0);
		fceu11::g_cart->set_mapper_number(0);
	}

	return LOADER_OK;
}
