/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
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

/// \file
/// \brief This file contains all code for coordinating the mapping in of the address space external to the NES.

#include "types.h"
#include "fceu.h"
#include "ppu.h"
#include "driver.h"

#include "cart.h"
#include "x6502.h"

#include "file.h"
#include "utils/memory.h"


#include <cstring>
#include <cstdlib>
#include <cstdio>
#include <climits>

// Phase 2 (v1.4 Gateway): all the page / ROM-pointer / mask /
// RAM-flag storage moved to fceu11::Bus. The legacy global names
// (Page, VPage, VPageG, MMC5SPRVPage, MMC5BGVPage, PRGptr, CHRptr,
// PRGram, CHRram, PRGsize, CHRsize, PRGmask2-32, CHRmask1-8, PRGIsRAM,
// VPageR) are now inline reference-to-array aliases declared in
// bus.h. cart.cpp only retains the non-Bus surface (CartInfo, the
// r-variant setprg*/setchr* free functions, Genie state).

int geniestage = 0;

int modcon;

uint8 genieval[3];
uint8 geniech[3];

uint32 genieaddr[3];

CartInfo *currCartInfo;

static INLINE void setpageptr(int s, uint32 A, uint8 *p, int ram) {
	uint32 AB = A >> 11;
	int x;

	if (p)
		for (x = (s >> 1) - 1; x >= 0; x--) {
			PRGIsRAM[AB + x] = ram;
			Page[AB + x] = p - A;
		}
	else
		for (x = (s >> 1) - 1; x >= 0; x--) {
			PRGIsRAM[AB + x] = 0;
			Page[AB + x] = 0;
		}
}

DECLFR(CartBR) {
	uint8 *p = Page[A >> 11];
	if ((A & 0x7FF) < (0x800 - 64))
		FCEUX11_PREFETCH(&p[A + 64]);
	return p[A];
}

DECLFW(CartBW) {
	//printf("Ok: %04x:%02x, %d\n",A,V,PRGIsRAM[A>>11]);
	if (PRGIsRAM[A >> 11] && Page[A >> 11])
		Page[A >> 11][A] = V;
}

DECLFR(CartBROB) {
	uint8 *p = Page[A >> 11];
	if (!p)
		return(g_cpu.native_layout().DB);
	if ((A & 0x7FF) < (0x800 - 64))
		FCEUX11_PREFETCH(&p[A + 64]);
	return p[A];
}

void setprg2r(int r, uint32 A, uint32 V) {
	V &= PRGmask2[r];
	setpageptr(2, A, PRGptr[r] ? (&PRGptr[r][V << 11]) : 0, PRGram[r]);
}

void setprg2(uint32 A, uint32 V) {
	setprg2r(0, A, V);
}

void setprg4r(int r, uint32 A, uint32 V) {
	V &= PRGmask4[r];
	setpageptr(4, A, PRGptr[r] ? (&PRGptr[r][V << 12]) : 0, PRGram[r]);
}

void setprg4(uint32 A, uint32 V) {
	setprg4r(0, A, V);
}

void setprg8r(int r, uint32 A, uint32 V) {
	if (PRGsize[r] >= 8192) {
		V &= PRGmask8[r];
		setpageptr(8, A, PRGptr[r] ? (&PRGptr[r][V << 13]) : 0, PRGram[r]);
	} else {
		uint32 VA = V << 2;
		int x;
		for (x = 0; x < 4; x++)
			setpageptr(2, A + (x << 11), PRGptr[r] ? (&PRGptr[r][((VA + x) & PRGmask2[r]) << 11]) : 0, PRGram[r]);
	}
}

void setprg16r(int r, uint32 A, uint32 V) {
	if (PRGsize[r] >= 16384) {
		V &= PRGmask16[r];
		setpageptr(16, A, PRGptr[r] ? (&PRGptr[r][V << 14]) : 0, PRGram[r]);
	} else {
		uint32 VA = V << 3;
		int x;

		for (x = 0; x < 8; x++)
			setpageptr(2, A + (x << 11), PRGptr[r] ? (&PRGptr[r][((VA + x) & PRGmask2[r]) << 11]) : 0, PRGram[r]);
	}
}

void setprg32r(int r, uint32 A, uint32 V) {
	if (PRGsize[r] >= 32768) {
		V &= PRGmask32[r];
		setpageptr(32, A, PRGptr[r] ? (&PRGptr[r][V << 15]) : 0, PRGram[r]);
	} else {
		uint32 VA = V << 4;
		int x;

		for (x = 0; x < 16; x++)
			setpageptr(2, A + (x << 11), PRGptr[r] ? (&PRGptr[r][((VA + x) & PRGmask2[r]) << 11]) : 0, PRGram[r]);
	}
}

void setchr1r(int r, uint32 A, uint32 V) {
	if (!CHRptr[r]) return;
	FCEUPPU_LineUpdate();
	V &= CHRmask1[r];
	if (CHRram[r])
		PPUCHRRAM |= (1 << (A >> 10));
	else
		PPUCHRRAM &= ~(1 << (A >> 10));
	VPageR[(A) >> 10] = &CHRptr[r][(V) << 10] - (A);
}

void setchr2r(int r, uint32 A, uint32 V) {
	if (!CHRptr[r]) return;
	FCEUPPU_LineUpdate();
	V &= CHRmask2[r];
	VPageR[(A) >> 10] = VPageR[((A) >> 10) + 1] = &CHRptr[r][(V) << 11] - (A);
	if (CHRram[r])
		PPUCHRRAM |= (3 << (A >> 10));
	else
		PPUCHRRAM &= ~(3 << (A >> 10));
}

void setchr4r(int r, unsigned int A, unsigned int V) {
	if (!CHRptr[r]) return;
	FCEUPPU_LineUpdate();
	V &= CHRmask4[r];
	VPageR[(A) >> 10] = VPageR[((A) >> 10) + 1] =
							VPageR[((A) >> 10) + 2] = VPageR[((A) >> 10) + 3] = &CHRptr[r][(V) << 12] - (A);
	if (CHRram[r])
		PPUCHRRAM |= (15 << (A >> 10));
	else
		PPUCHRRAM &= ~(15 << (A >> 10));
}

void setchr8r(int r, uint32 V) {
	int x;

	if (!CHRptr[r]) return;
	FCEUPPU_LineUpdate();
	V &= CHRmask8[r];
	for (x = 7; x >= 0; x--)
		VPageR[x] = &CHRptr[r][V << 13];
	if (CHRram[r])
		PPUCHRRAM |= (255);
	else
		PPUCHRRAM = 0;
}

void setchr2(uint32 A, uint32 V) {
	setchr2r(0, A, V);
}

/* setchr1/4/8, setntamem, setmirror, setmirrorw, SetupCartMirroring
 * are now Bus member functions (see bus.h / bus.cpp). The inline
 * forwarders in bus.h preserve the legacy call-site syntax. */

/* The `nothing[8192]` open-bus buffer used to live here; it moved
 * to bus.cpp's anonymous namespace where Bus::reset_mapping() and
 * the hot-path ANull handler reference it. */

static uint8 *GENIEROM = 0;

void FixGenieMap(void);

// Called when a game(file) is opened successfully. Returns TRUE on error.
bool FCEU_OpenGenie(void)
{
	FILE *fp;
	int x;

	if (!GENIEROM)
	{
		char *fn;

		if (!(GENIEROM = (uint8*)FCEU_malloc(4096 + 1024)))
			return true;

		fn = strdup(FCEU_MakeFName(FCEUMKF_GGROM, 0, 0).c_str());
		fp = FCEUD_UTF8fopen(fn, "rb");
		if (!fp)
		{
			FCEU_PrintError("Error opening Game Genie ROM image!\nIt should be named \"gg.rom\"!");
			free(GENIEROM);
			GENIEROM = 0;
			return true;
		}
		if (fread(GENIEROM, 1, 16, fp) != 16)
		{
 grerr:
			FCEU_PrintError("Error reading from Game Genie ROM image!");
			free(GENIEROM);
			GENIEROM = 0;
			fclose(fp);
			return true;
		}
		if (GENIEROM[0] == 0x4E)
		{
			/* iNES ROM image */
			if (fread(GENIEROM, 1, 4096, fp) != 4096)
				goto grerr;
			if (fseek(fp, 16384 - 4096, SEEK_CUR))
				goto grerr;
			if (fread(GENIEROM + 4096, 1, 256, fp) != 256)
				goto grerr;
		} else
		{
			if (fread(GENIEROM + 16, 1, 4352 - 16, fp) != (4352 - 16))
				goto grerr;
		}
		fclose(fp);

		/* Workaround for the FCE Ultra CHR page size only being 1KB */
		for (x = 0; x < 4; x++)
		{
			memcpy(GENIEROM + 4096 + (x << 8), GENIEROM + 4096, 256);
		}
	}

	geniestage = 1;
	return false;
}

/* Called when a game is closed. */
void FCEU_CloseGenie(void) {
	/* No good reason to free() the Game Genie ROM image data. */
	geniestage = 0;
	FlushGenieRW();
	VPageR = VPage;
}

void FCEU_KillGenie(void) {
	if (GENIEROM) {
		free(GENIEROM);
		GENIEROM = 0;
	}
}

static DECLFR(GenieRead) {
	return GENIEROM[A & 4095];
}

static DECLFW(GenieWrite) {
	switch (A) {
	case 0x800c:
	case 0x8008:
	case 0x8004: genieval[((A - 4) & 0xF) >> 2] = V; break;

	case 0x800b:
	case 0x8007:
	case 0x8003: geniech[((A - 3) & 0xF) >> 2] = V; break;

	case 0x800a:
	case 0x8006:
	case 0x8002: genieaddr[((A - 2) & 0xF) >> 2] &= 0xFF00; genieaddr[((A - 2) & 0xF) >> 2] |= V; break;

	case 0x8009:
	case 0x8005:
	case 0x8001: genieaddr[((A - 1) & 0xF) >> 2] &= 0xFF; genieaddr[((A - 1) & 0xF) >> 2] |= (V | 0x80) << 8; break;

	case 0x8000:
		if (!V)
			FixGenieMap();
		else {
			modcon = V ^ 0xFF;
			if (V == 0x71)
				modcon = 0;
		}
		break;
	}
}

static readfunc GenieBackup[3];

static DECLFR(GenieFix1) {
	uint8 r = GenieBackup[0](A);

	if ((modcon >> 1) & 1) // No check
		return genieval[0];
	else if (r == geniech[0])
		return genieval[0];

	return r;
}

static DECLFR(GenieFix2) {
	uint8 r = GenieBackup[1](A);

	if ((modcon >> 2) & 1) // No check
		return genieval[1];
	else if (r == geniech[1])
		return genieval[1];

	return r;
}

static DECLFR(GenieFix3) {
	uint8 r = GenieBackup[2](A);

	if ((modcon >> 3) & 1) // No check
		return genieval[2];
	else if (r == geniech[2])
		return genieval[2];

	return r;
}


void FixGenieMap(void) {
	int x;

	geniestage = 2;

	for (x = 0; x < 8; x++)
		VPage[x] = VPageG[x];

	VPageR = VPage;
	FlushGenieRW();
	//printf("Rightyo\n");
	for (x = 0; x < 3; x++)
		if ((modcon >> (4 + x)) & 1) {
			readfunc tmp[3] = { GenieFix1, GenieFix2, GenieFix3 };
			GenieBackup[x] = GetReadHandler(genieaddr[x]);
			SetReadHandler(genieaddr[x], genieaddr[x], tmp[x]);
		}
}

void FCEU_GeniePower(void) {
	uint32 x;

	if (!geniestage)
		return;

	geniestage = 1;
	for (x = 0; x < 3; x++) {
		genieval[x] = 0xFF;
		geniech[x] = 0xFF;
		genieaddr[x] = 0xFFFF;
	}
	modcon = 0;

	SetWriteHandler(0x8000, 0xFFFF, GenieWrite);
	SetReadHandler(0x8000, 0xFFFF, GenieRead);

	for (x = 0; x < 8; x++)
		VPage[x] = GENIEROM + 4096 - 0x400 * x;

	if (AllocGenieRW())
		VPageR = VPageG;
	else
		geniestage = 2;
}


void FCEU_SaveGameSave(CartInfo *LocalHWInfo)
{
	if (LocalHWInfo->battery && !LocalHWInfo->SaveGame.empty())
	{
		std::string soot = FCEU_MakeFName(FCEUMKF_SAV, 0, "sav");
		std::vector<FceuSaveGameEntry> entries;
		entries.reserve(LocalHWInfo->SaveGame.size());
		for (size_t x = 0; x < LocalHWInfo->SaveGame.size(); x++)
		{
			if (LocalHWInfo->SaveGame[x].bufptr)
			{
				entries.push_back({LocalHWInfo->SaveGame[x].bufptr,
					                   static_cast<uint32_t>(LocalHWInfo->SaveGame[x].buflen)});
			}
		}
		fceux11_rust_cart_battery_save(soot.c_str(), entries.data(), entries.size());
	}
}

// hack, movie.cpp has to communicate with this function somehow
int disableBatteryLoading = 0;

void FCEU_LoadGameSave(CartInfo *LocalHWInfo)
{
	if (LocalHWInfo->battery && !LocalHWInfo->SaveGame.empty() && !disableBatteryLoading)
	{
		std::string soot = FCEU_MakeFName(FCEUMKF_SAV, 0, "sav");
		std::vector<FceuSaveGameEntry> entries;
		entries.reserve(LocalHWInfo->SaveGame.size());
		for (size_t x = 0; x < LocalHWInfo->SaveGame.size(); x++)
		{
			if (LocalHWInfo->SaveGame[x].bufptr)
			{
				entries.push_back({LocalHWInfo->SaveGame[x].bufptr,
					                   static_cast<uint32_t>(LocalHWInfo->SaveGame[x].buflen)});
			}
		}
		fceux11_rust_cart_battery_load(soot.c_str(), entries.data(), entries.size());
	}
}

//clears all save memory. call this if you want to pretend the saveram has been reset (it doesnt touch what is on disk though)
void FCEU_ClearGameSave(CartInfo *LocalHWInfo)
{
	if (LocalHWInfo->battery && !LocalHWInfo->SaveGame.empty())
	{
		std::vector<FceuSaveGameEntry> entries;
		entries.reserve(LocalHWInfo->SaveGame.size());
		for (size_t x = 0; x < LocalHWInfo->SaveGame.size(); x++)
		{
			if (LocalHWInfo->SaveGame[x].bufptr)
			{
				entries.push_back({LocalHWInfo->SaveGame[x].bufptr,
					                   static_cast<uint32_t>(LocalHWInfo->SaveGame[x].buflen)});
			}
		}
		fceux11_rust_cart_battery_clear(entries.data(), entries.size());
		for (size_t x = 0; x < LocalHWInfo->SaveGame.size(); x++)
		{
			if (LocalHWInfo->SaveGame[x].resetFunc)
			{
				LocalHWInfo->SaveGame[x].resetFunc();
			}
		}
	}
}
