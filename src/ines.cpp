/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
 *  Copyright (C) 2002 Xodnizel
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
// v1.10 Cryptex: iNES format bridge layer.
// Global variables and thin iNESLoad wrapper.  Core logic lives in:
//   ines_load.cpp  â€?iNESLoadCore (Rust FFI parsing + ROM setup)
//   ines_init.cpp  â€?iNES_Init (mapper initialization)
//   ines_gi.cpp    â€?iNESGI (game interface hooks)
//   ines_save.cpp  â€?iNesSave/iNesSaveAs
//   ines_bmap.h    â€?BMAPPINGLocal bmap[] table

#include "types.h"
#include "utils/safe_string.h"
#include "utils/memory.h"
#include "fceu.h"
#include "cart.h"
#include "apu.h"
#include "ines.h"
#include "unif.h"
#include "file.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "vsuni.h"

#include <cstring>

#include "ines_bmap.h"

TMasterRomInfoParams MasterRomInfoParams;

// Global variables (extern-declared in ines.h)
uint8 *trainerpoo = NULL;
FceuMallocPtr trainerpoo_owner;
uint8 *ROM = NULL;
uint8 *VROM = NULL;
uint8 *ExtraNTARAM = NULL;
FceuMallocPtr ExtraNTARAM_owner;
iNES_HEADER head;
CartInfo iNESCart;
uint8 Mirroring = 0;
uint8 MirroringAs2bits = 0;
uint32 ROM_size = 0;
uint32 VROM_size = 0;
char LoadedRomFName[4096];
char LoadedRomFNamePatchToUse[4096];
int CHRRAMSize = -1;
int MapperNo = 0;
int iNES2 = 0;
uint32 iNESGameCRC32 = 0;

// v1.10 Cryptex: iNESLoad is now a thin wrapper around iNESLoadCore (ines_load.cpp)
extern int iNESLoadCore(const char *name, FCEUFILE *fp, CartInfo& iNESCart, FceuMallocPtr& trainerpoo_owner, FceuMallocPtr& ExtraNTARAM_owner);
extern void iNESGI(GI h);

int iNESLoad(const char *name, FCEUFILE *fp, int OverwriteVidMode) {
	int result = iNESLoadCore(name, fp, iNESCart, trainerpoo_owner, ExtraNTARAM_owner);
	if (result != LOADER_OK) return result;

	int MapperNo = iNESCart.mapper_number;
	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name);
	const char* basename = strrchr(name, '/') ? strrchr(name, '/') + 1 : strrchr(name, '\\') ? strrchr(name, '\\') + 1 : name;

	GameInterface = iNESGI;
	currCartInfo = &iNESCart;

	// v2.0.5 A1-a: install the CartInfo_*Forward lifecycle only when the
	// MapperEntry registry produced a concrete Cart object.  On a registry
	// miss the Power/Reset/Close pointers stay at whatever the board's
	// MapperNN_Init installed (ines_load.cpp iNES_Init -> bmap): the
	// placeholder's on_power/on_reset/on_close are no-ops, so the legacy
	// pointers are the only effective path.  Overwriting them unconditionally
	// silently disabled the board Power of every unregistered mapper (75
	// numbers as of the 2026-11-15 derivation in
	// tests/f11qa/boards/mapper_load_test.cpp; SetWriteHandler / EXPREGS
	// seeds / SRAM mapping / IRQ hooks never installed; mapper 252's "blue
	// screen" symptom traces here).  The UNIF loader (src/unif.cpp) already
	// uses this conditional form.
	auto cart_obj = fceu11::create_cart_for_mapper(MapperNo, fceu11::g_bus);
	if (cart_obj) {
		fceu11::assign_cart(std::move(cart_obj));
		iNESCart.cart_obj = fceu11::g_cart;
		iNESCart.Power = CartInfo_PowerForward;
		iNESCart.Reset = CartInfo_ResetForward;
		iNESCart.Close = CartInfo_CloseForward;
	} else {
		fceu11::assign_cart(nullptr);
		iNESCart.cart_obj = fceu11::g_cart;
	}

	if (currCartInfo && currCartInfo->cart_obj)
		currCartInfo->cart_obj->install_expansion_audio(fceu11::g_apu);

	if (fceu11::g_cart) {
		fceu11::g_cart->set_md5(iNESCart.MD5); fceu11::g_cart->set_crc32(iNESCart.CRC32);
		fceu11::g_cart->set_mirror(iNESCart.mirror); fceu11::g_cart->set_mirror_as_2bits(iNESCart.mirrorAs2Bits);
		fceu11::g_cart->set_battery(iNESCart.battery != 0); fceu11::g_cart->set_ines2(iNESCart.ines2 != 0);
		fceu11::g_cart->set_submapper(iNESCart.submapper); fceu11::g_cart->set_mapper_number(MapperNo);
		fceu11::g_cart->set_wram_size(iNESCart.wram_size); fceu11::g_cart->set_battery_wram_size(iNESCart.battery_wram_size);
		fceu11::g_cart->set_vram_size(iNESCart.vram_size); fceu11::g_cart->set_battery_vram_size(iNESCart.battery_vram_size);
	}

	if (iNES2) fceu11::SetVidSystem(iNESCart.tv_system == 1 ? 1 : 0);
	else if (OverwriteVidMode) {
		fceu11::SetVidSystem((strstr(basename, "(E)") || strstr(basename, "(e)") || strstr(basename, "(Europe)")
			|| strstr(basename, "(PAL)") || strstr(basename, "(F)") || strstr(basename, "(f)")
			|| strstr(basename, "(G)") || strstr(basename, "(g)") || strstr(basename, "(I)") || strstr(basename, "(i)")) ? 1 : 0);
	}
	return LOADER_OK;
}


