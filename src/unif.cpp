// v1.10 Cryptex: UNIF format bridge layer (< 100 lines).
// Core logic lives in unif_load.cpp (Rust FFI parsing + ROM setup + board init)
// and unif_bmap.h (BMAPPING bmap[] table).

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "apu.h"
#include "unif.h"
#include "ines.h"
#include "file.h"

#include <cstring>

// Global variables
CartInfo UNIFCart;
uint8 *UNIFchrrama = 0;

// Forward declarations for functions in unif_load.cpp
extern void FreeUNIF(void);
extern int UNIFLoadCore(const char *name, FCEUFILE *fp, CartInfo& UNIFCart);

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

int UNIFLoad(const char *name, FCEUFILE *fp) {
	FCEU_fseek(fp, 0, SEEK_SET);

	int result = UNIFLoadCore(name, fp, UNIFCart);
	if (result != LOADER_OK) return result;

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
