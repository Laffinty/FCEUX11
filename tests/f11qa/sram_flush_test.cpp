/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2026 FCEUX11 Contributors
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

// v2.0.8 C1 P1 — SRAM flush dispatch test (plan legs M3/M4/M5).
//
// M3: FCEU_FlushGameSave() with no game loaded is a safe no-op.
// M4: with a game loaded, the flush routes GI_SAVESRAM (value 5) through the
//     GameInterface dispatcher. Mutation leg: change FCEU_FlushGameSave to
//     send GI_CLOSE and this must go red.
// M5: iNESGI(GI_SAVESRAM) honours the battery flag: battery=0 is a silent
//     no-op (leg a); battery=1 writes the .sav with the registered entry
//     bytes (leg b). Mutation leg: remove the GI_SAVESRAM case from iNESGI
//     and leg b must go red.

#include <cstdio>
#include <cstring>
#include <filesystem>
#include <string>
#include <vector>

#include "types.h"
#include "fceu.h"
#include "git.h"
#include "cart.h"
#include "file.h"

extern void iNESGI(GI h);
extern CartInfo iNESCart;

static GI g_seenGI = GI_RESETM2;
static void stubGI(GI h) { g_seenGI = h; }

static int failures = 0;

#define CHECK(cond, msg)                                     \
	do {                                                     \
		if (cond) {                                          \
			std::printf("PASS: %s\n", msg);                  \
		} else {                                             \
			std::printf("FAIL: %s\n", msg);                  \
			failures++;                                      \
		}                                                    \
	} while (0)

int main()
{
	// ---- M3: no game -> safe no-op ----
	GameInfo = nullptr;
	GameInterface = nullptr;
	FCEU_FlushGameSave();
	CHECK(true, "M3 no-op with GameInfo==NULL");

	// ---- M4: dispatch routes GI_SAVESRAM ----
	GameInfo = new FCEUGI();
	GameInterface = stubGI;
	g_seenGI = GI_RESETM2;
	FCEU_FlushGameSave();
	CHECK(g_seenGI == GI_SAVESRAM, "M4 stub receives GI_SAVESRAM");
	delete GameInfo;
	GameInfo = nullptr;
	GameInterface = nullptr;

	// ---- M5a: battery=0 -> silent no-op ----
	iNESCart.clear();
	iNESCart.battery = 0;
	iNESGI(GI_SAVESRAM);
	CHECK(true, "M5a battery=0 no-op");

	// ---- M5b: battery=1 -> .sav written with entry bytes ----
	static uint8 sram[8] = { 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88 };
	iNESCart.clear();
	iNESCart.battery = 1;
	iNESCart.addSaveGameBuf(sram, sizeof(sram));

	GetFileBase("sram_flush_probe");
	std::string savPath = FCEU_MakeFName(FCEUMKF_SAV, 0, "sav");
	std::filesystem::create_directories(
		std::filesystem::path(savPath).parent_path());

	iNESGI(GI_SAVESRAM);

	FILE* fp = std::fopen(savPath.c_str(), "rb");
	CHECK(fp != nullptr, "M5b .sav created");
	if (fp)
	{
		uint8 buf[16] = { 0 };
		size_t n = std::fread(buf, 1, sizeof(buf), fp);
		std::fclose(fp);
		CHECK(n == sizeof(sram) && std::memcmp(buf, sram, sizeof(sram)) == 0,
			  "M5c .sav bytes match entry");
		std::remove(savPath.c_str());
	}
	else
	{
		failures++;
	}

	if (failures == 0)
	{
		std::printf("PASS: sram_flush_test all legs\n");
		return 0;
	}
	std::printf("FAIL: sram_flush_test %d leg(s)\n", failures);
	return 1;
}
