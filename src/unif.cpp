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

/* TODO:  Battery backup file saving, mirror force    */
/* **INCOMPLETE**             */
/* Override stuff: CHR RAM instead of CHR ROM,   mirroring. */

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "unif.h"
#include "ines.h"
#include "utils/endian.h"
#include "utils/memory.h"
#include "utils/md5.h"
#include "state.h"
#include "file.h"
#include "input.h"
#include "driver.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>

typedef struct {
	char ID[4];
	uint32 info;
} UNIF_HEADER;

typedef struct {
	const char *name;
	void (*init)(CartInfo *);
} BMAPPING;

typedef struct {
	const char *name;
	int (*init)(FCEUFILE *fp);
} BFMAPPING;

static CartInfo UNIFCart;

static int vramo;
static int mirrortodo;
static uint8 *boardname;
static uint8 *sboardname;

static uint32 CHRRAMSize;
uint8 *UNIFchrrama = 0;

static UNIF_HEADER unhead;
static UNIF_HEADER uchead;


static uint8 *malloced[32];
static uint32 mallocedsizes[32];

static void FreeUNIF(void) {
	int x;
	if (UNIFchrrama) {
		free(UNIFchrrama); UNIFchrrama = 0;
	}
	if (boardname) {
		free(boardname); boardname = 0;
	}
	for (x = 0; x < 32; x++) {
		if (malloced[x]) {
			free(malloced[x]); malloced[x] = 0;
		}
	}
}

static void ResetUNIF(void) {
	int x;
	for (x = 0; x < 32; x++)
		malloced[x] = 0;
	vramo = 0;
	boardname = 0;
	mirrortodo = 0;
	UNIFCart.clear();
	UNIFchrrama = 0;
}

static uint8 exntar[2048];

static void MooMirroring(void) {
	if (mirrortodo < 0x4)
		SetupCartMirroring(mirrortodo, 1, 0);
	else if (mirrortodo == 0x4) {
		FCEU_MemoryRand(exntar, sizeof(exntar), true);
		SetupCartMirroring(4, 1, exntar);
		AddExState(exntar, 2048, 0, "EXNR");
	} else
		SetupCartMirroring(0, 0, 0);
}

static int DoMirroring(FCEUFILE *fp) {
	int t;
	uint32 i;
	if (uchead.info == 1) {
		if ((t = FCEU_fgetc(fp)) == EOF)
			return(0);
		mirrortodo = t;
		{
			static const char *stuffo[6] = { "Horizontal", "Vertical", "$2000", "$2400", "\"Four-screen\"", "Controlled by Mapper Hardware" };
			if (t < 6)
				FCEU_printf(" Name/Attribute Table Mirroring: %s\n", stuffo[t]);
		}
	} else {
		FCEU_printf(" Incorrect Mirroring Chunk Size (%d). Data is:", uchead.info);
		for (i = 0; i < uchead.info; i++) {
			if ((t = FCEU_fgetc(fp)) == EOF)
				return(0);
			FCEU_printf(" %02x", t);
		}
		FCEU_printf("\n Default Name/Attribute Table Mirroring: Horizontal\n");
		mirrortodo = 0;
	}
	return(1);
}

static int NAME(FCEUFILE *fp) {
	char namebuf[100];
	int index;
	int t;

	FCEU_printf(" Name: ");
	index = 0;

	while ((t = FCEU_fgetc(fp)) > 0)
		if (index < 99)
			namebuf[index++] = t;

	namebuf[index] = 0;
	FCEU_printf("%s\n", namebuf);

	if (!GameInfo->name) {
		GameInfo->name = (uint8*)malloc(strlen(namebuf) + 1); //mbg merge 7/17/06 added cast
		FCEU_strlcpy((char*)GameInfo->name, sizeof((char*)GameInfo->name), namebuf); //mbg merge 7/17/06 added cast
	}
	return(1);
}

static int DINF(FCEUFILE *fp) {
	char name[100], method[100];
	uint8 d, m;
	uint16 y;
	int t;

	if (FCEU_fread(name, 1, 100, fp) != 100)
		return(0);
	if ((t = FCEU_fgetc(fp)) == EOF) return(0);
	d = t;
	if ((t = FCEU_fgetc(fp)) == EOF) return(0);
	m = t;
	if ((t = FCEU_fgetc(fp)) == EOF) return(0);
	y = t;
	if ((t = FCEU_fgetc(fp)) == EOF) return(0);
	y |= t << 8;
	if (FCEU_fread(method, 1, 100, fp) != 100)
		return(0);
	name[99] = method[99] = 0;
	FCEU_printf(" Dumped by: %s\n", name);
	FCEU_printf(" Dumped with: %s\n", method);
	{
		const char *months[12] = {
			"January", "February", "March", "April", "May", "June", "July",
			"August", "September", "October", "November", "December"
		};
		FCEU_printf(" Dumped on: %s %d, %d\n", months[(m - 1) % 12], d, y);
	}
	return(1);
}

static int CTRL(FCEUFILE *fp) {
	int t;
	uint32 i;
	if (uchead.info == 1) {
		if ((t = FCEU_fgetc(fp)) == EOF)
			return(0);
		/* The information stored in this byte isn't very helpful, but it's
		better than nothing...maybe.
		*/

		if (t & 1)
			GameInfo->input[0] = GameInfo->input[1] = SI_GAMEPAD;
		else
			GameInfo->input[0] = GameInfo->input[1] = SI_NONE;
		if (t & 2)
			GameInfo->input[1] = SI_ZAPPER;
	} else {
		FCEU_printf(" Incorrect Control Chunk Size (%d). Data is:", uchead.info);
		for (i = 0; i < uchead.info; i++) {
			t = FCEU_fgetc(fp);
			FCEU_printf(" %02x", t);
		}
		FCEU_printf("\n");
		GameInfo->input[0] = GameInfo->input[1] = SI_GAMEPAD;
	}
	return(1);
}

static int TVCI(FCEUFILE *fp) {
	int t;
	if ((t = FCEU_fgetc(fp)) == EOF)
		return(0);
	if (t <= 2) {
		const char *stuffo[3] = { "NTSC", "PAL", "NTSC and PAL" };
		if (t == 0) {
			GameInfo->vidsys = GIV_NTSC;
			FCEUI_SetVidSystem(0);
		} else if (t == 1) {
			GameInfo->vidsys = GIV_PAL;
			FCEUI_SetVidSystem(1);
		}
		FCEU_printf(" TV Standard Compatibility: %s\n", stuffo[t]);
	}
	return(1);
}

static int EnableBattery(FCEUFILE *fp) {
	FCEU_printf(" Battery-backed.\n");
	if (FCEU_fgetc(fp) == EOF)
		return(0);
	UNIFCart.battery = 1;
	return(1);
}

static int LoadPRG(FCEUFILE *fp) {
	int z, t;
	z = uchead.ID[3] - '0';

	if (z < 0 || z > 15)
		return(0);
	FCEU_printf(" PRG ROM %d size: %d", z, (int)uchead.info);
	if (malloced[z])
		free(malloced[z]);
	t = fceux11_rust_uppow2(uchead.info);
	if (t < 2048) t = 2048;
	if (!(malloced[z] = (uint8*)FCEU_malloc(t)))
		return(0);
	mallocedsizes[z] = t;
	memset(malloced[z] + uchead.info, 0xFF, t - uchead.info);
	if (FCEU_fread(malloced[z], 1, uchead.info, fp) != uchead.info) {
		FCEU_printf("Read Error!\n");
		return(0);
	} else
		FCEU_printf("\n");

	SetupCartPRGMapping(z, malloced[z], t, 0);
	return(1);
}

static int SetBoardName(FCEUFILE *fp) {
	if (!(boardname = (uint8*)FCEU_malloc(uchead.info + 1)))
		return(0);
	FCEU_fread(boardname, 1, uchead.info, fp);
	boardname[uchead.info] = 0;
	FCEU_printf(" Board name: %s\n", boardname);
	sboardname = boardname;
	if (!memcmp(boardname, "NES-", 4) || !memcmp(boardname, "UNL-", 4) || !memcmp(boardname, "HVC-", 4) || !memcmp(boardname, "BTL-", 4) || !memcmp(boardname, "BMC-", 4))
		sboardname += 4;
	return(1);
}

static int LoadCHR(FCEUFILE *fp) {
	int z, t;
	z = uchead.ID[3] - '0';
	if (z < 0 || z > 15)
		return(0);
	FCEU_printf(" CHR ROM %d size: %d", z, (int)uchead.info);
	if (malloced[16 + z])
		free(malloced[16 + z]);
	t = fceux11_rust_uppow2(uchead.info);
	if (t < 8192) t = 8192;
	if (!(malloced[16 + z] = (uint8*)FCEU_malloc(t)))
		return(0);
	mallocedsizes[16 + z] = t;
	memset(malloced[16 + z] + uchead.info, 0xFF, t - uchead.info);
	if (FCEU_fread(malloced[16 + z], 1, uchead.info, fp) != uchead.info) {
		FCEU_printf("Read Error!\n");
		return(0);
	} else
		FCEU_printf("\n");

	SetupCartCHRMapping(z, malloced[16 + z], t, 0);
	return(1);
}

#define BMCFLAG_FORCE4    0x01
#define BMCFLAG_16KCHRR   0x02
#define BMCFLAG_32KCHRR   0x04
#define BMCFLAG_128KCHRR  0x08
#define BMCFLAG_256KCHRR  0x10

static BMAPPING bmap[] = {
	{ "11160", BMC11160_Init },
	{ "12-IN-1", BMC12IN1_Init },
	{ "13in1JY110", BMC13in1JY110_Init },
	{ "190in1", BMC190in1_Init },
	{ "22211", UNL22211_Init },
	{ "3D-BLOCK", UNL3DBlock_Init },
	{ "411120-C", BMC411120C_Init },
	{ "42in1ResetSwitch", Mapper226_Init },
	{ "43272", UNL43272_Init },
	{ "603-5052", UNL6035052_Init },
	{ "64in1NoRepeat", BMC64in1nr_Init },
	{ "70in1", BMC70in1_Init },
	{ "70in1B", BMC70in1B_Init },
	{ "810544-C-A1", BMC810544CA1_Init },
	{ "8157", UNL8157_Init },
	{ "8237", UNL8237_Init },
	{ "8237A", UNL8237A_Init },
	{ "830118C", BMC830118C_Init },
	{ "A65AS", BMCA65AS_Init },
	{ "AC08", AC08_Init },
	{ "ANROM", ANROM_Init },
	{ "AX5705", UNLAX5705_Init },
	{ "BB", UNLBB_Init },
	{ "BS-5", BMCBS5_Init },
	{ "CC-21", UNLCC21_Init },
	{ "CITYFIGHT", UNLCITYFIGHT_Init },
	{ "10-24-C-A1", BMC1024CA1_Init },
	{ "CNROM", CNROM_Init },
	{ "CPROM", CPROM_Init },
	{ "D1038", BMCD1038_Init },
	{ "DANCE", UNLOneBus_Init },	// redundant
	{ "DANCE2000", UNLD2000_Init },
	{ "DREAMTECH01", DreamTech01_Init },
	{ "EDU2000", UNLEDU2000_Init },
	{ "EKROM", EKROM_Init },
	{ "ELROM", ELROM_Init },
	{ "ETROM", ETROM_Init },
	{ "EWROM", EWROM_Init },
	{ "FK23C", BMCFK23C_Init },
	{ "FK23CA", BMCFK23CA_Init },
	{ "FS304", UNLFS304_Init },
	{ "G-146", BMCG146_Init },
	{ "GK-192", BMCGK192_Init },
	{ "GS-2004", BMCGS2004_Init },
	{ "GS-2013", BMCGS2013_Init },
	{ "Ghostbusters63in1", BMCGhostbusters63in1_Init },
	{ "H2288", UNLH2288_Init },
	{ "HKROM", HKROM_Init },
	{ "KOF97", UNLKOF97_Init },
	{ "KONAMI-QTAI", QTAi_Init },
	{ "KS7010", UNLKS7010_Init },
	{ "KS7012", UNLKS7012_Init },
	{ "KS7013B", UNLKS7013B_Init },
	{ "KS7016", UNLKS7016_Init },
	{ "KS7017", UNLKS7017_Init },
	{ "KS7030", UNLKS7030_Init },
	{ "KS7031", UNLKS7031_Init },
	{ "KS7032", UNLKS7032_Init },
	{ "KS7037", UNLKS7037_Init },
	{ "KS7057", UNLKS7057_Init },
	{ "LE05", LE05_Init },
	{ "LH10", LH10_Init },
	{ "LH32", LH32_Init },
	{ "LH53", LH53_Init },
	{ "MALISB", UNLMaliSB_Init },
	{ "MARIO1-MALEE2", MALEE_Init },
	{ "MHROM", MHROM_Init },
	{ "N625092", UNLN625092_Init },
	{ "NROM", NROM_Init },
	{ "NROM-128", NROM_Init },
	{ "NROM-256", NROM_Init },
	{ "NTBROM", Mapper68_Init },
	{ "NTD-03", BMCNTD03_Init },
	{ "NovelDiamond9999999in1", Novel_Init },
	{ "OneBus", UNLOneBus_Init },
	{ "PEC-586", UNLPEC586Init },
	{ "RET-CUFROM", Mapper29_Init },
	{ "RROM", NROM_Init },
	{ "RROM-128", NROM_Init },
	{ "SA-002", TCU02_Init },
	{ "SA-0036", SA0036_Init },
	{ "SA-0037", SA0037_Init },
	{ "SA-009", SA009_Init },
	{ "SA-016-1M", SA0161M_Init },
	{ "SA-72007", SA72007_Init },
	{ "SA-72008", SA72008_Init },
	{ "SA-9602B", SA9602B_Init },
	{ "SA-NROM", TCA01_Init },
	{ "SAROM", SAROM_Init },
	{ "SBROM", SBROM_Init },
	{ "SC-127", UNLSC127_Init },
	{ "SCROM", SCROM_Init },
	{ "SEROM", SEROM_Init },
	{ "SGROM", SGROM_Init },
	{ "SHERO", UNLSHeroes_Init },
	{ "SKROM", SKROM_Init },
	{ "SL12", UNLSL12_Init },
	{ "SL1632", UNLSL1632_Init },
	{ "SL1ROM", SL1ROM_Init },
	{ "SLROM", SLROM_Init },
	{ "SMB2J", UNLSMB2J_Init },
	{ "SNROM", SNROM_Init },
	{ "SOROM", SOROM_Init },
	{ "SSS-NROM-256", SSSNROM_Init },
	{ "SUNSOFT_UNROM", SUNSOFT_UNROM_Init },	// fix me, real pcb name, real pcb type
	{ "Sachen-74LS374N", S74LS374N_Init },
	{ "Sachen-74LS374NA", S74LS374NA_Init },	//seems to be custom mapper
	{ "Sachen-8259A", S8259A_Init },
	{ "Sachen-8259B", S8259B_Init },
	{ "Sachen-8259C", S8259C_Init },
	{ "Sachen-8259D", S8259D_Init },
	{ "Super24in1SC03", Super24_Init },
	{ "SuperHIK8in1", Mapper45_Init },
	{ "Supervision16in1", Supervision16_Init },
	{ "T-227-1", BMCT2271_Init },
	{ "T-230", UNLT230_Init },
	{ "T-262", BMCT262_Init },
	{ "TBROM", TBROM_Init },
	{ "TC-U01-1.5M", TCU01_Init },
	{ "TEK90", Mapper90_Init },
	{ "TEROM", TEROM_Init },
	{ "TF1201", UNLTF1201_Init },
	{ "TFROM", TFROM_Init },
	{ "TGROM", TGROM_Init },
	{ "TKROM", TKROM_Init },
	{ "TKSROM", TKSROM_Init },
	{ "TLROM", TLROM_Init },
	{ "TLSROM", TLSROM_Init },
	{ "TQROM", TQROM_Init },
	{ "TR1ROM", TFROM_Init },
	{ "TSROM", TSROM_Init },
	{ "TVROM", TLROM_Init },
	{ "Transformer", Transformer_Init },
	{ "UNROM", UNROM_Init },
	{ "UNROM-512-8", UNROM512_Init },
	{ "UNROM-512-16", UNROM512_Init },
	{ "UNROM-512-32", UNROM512_Init },
	{ "UOROM", UNROM_Init },
	{ "VRC7", UNLVRC7_Init },
	{ "YOKO", UNLYOKO_Init },
	{ "SB-2000", UNLSB2000_Init },
	{ "COOLBOY", COOLBOY_Init },
	{ "158B", UNL158B_Init },
	{ "DRAGONFIGHTER", UNLBMW8544_Init },
	{ "EH8813A", UNLEH8813A_Init },
	{ "HP898F", BMCHP898F_Init },
	{ "F-15", BMCF15_Init },
	{ "RT-01", UNLRT01_Init },
	{ "81-01-31-C", BMC810131C_Init },
	{ "8-IN-1", BMC8IN1_Init },
	{ "80013-B", BMC80013B_Init },
	{ "HPxx", BMCHPxx_Init },
	{ "MINDKIDS", MINDKIDS_Init },
	{ "FNS", FNS_Init },
	{ "BS-400R", BS400R_Init },
	{ "BS-4040R", BS4040R_Init },
	{ "COOLGIRL", COOLGIRL_Init },
	{ "JC-016-2", Mapper205_Init },

	{ 0, 0 }
};

static BFMAPPING bfunc[] = {
	{ "CTRL", CTRL },
	{ "TVCI", TVCI },
	{ "BATR", EnableBattery },
	{ "MIRR", DoMirroring },
	{ "PRG", LoadPRG },
	{ "CHR", LoadCHR },
	{ "NAME", NAME },
	{ "MAPR", SetBoardName },
	{ "DINF", DINF },
	{ NULL, NULL }
};

int LoadUNIFChunks(FCEUFILE *fp) {
	int x;
	int t;
	for (;; ) {
		t = FCEU_fread(&uchead, 1, 4, fp);
		if (t < 4) {
			if (t > 0)
				return 0;
			return 1;
		}
		if (!(FCEU_read32le(&uchead.info, fp)))
			return 0;
		t = 0;
		x = 0;
		while (bfunc[x].name) {
			if (!memcmp(&uchead, bfunc[x].name, strlen(bfunc[x].name))) {
				if (!bfunc[x].init(fp))
					return 0;
				t = 1;
				break;
			}
			x++;
		}
		if (!t)
			if (FCEU_fseek(fp, uchead.info, SEEK_CUR) < 0)
				return(0);
	}
}

static int InitializeBoard(void) {
	int x = 0;

	if (!sboardname) return(0);

	while (bmap[x].name) {
		if (!strcmp((char*)sboardname, (char*)bmap[x].name)) {
			int flags = fceux11_rust_unif_board_flags((const char*)sboardname);
			if (flags < 0) return 1;

			if (!malloced[16]) {
				CHRRAMSize = fceux11_rust_unif_chrram_size(flags);
				if ((UNIFchrrama = (uint8*)FCEU_malloc(CHRRAMSize))) {
					SetupCartCHRMapping(0, UNIFchrrama, CHRRAMSize, 1);
					AddExState(UNIFchrrama, CHRRAMSize, 0, "CHRR");
				} else
					return 2;
			}
			if (flags & BMCFLAG_FORCE4)
				mirrortodo = 4;
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
	case GI_RESETSAVE:
		FCEU_ClearGameSave(&UNIFCart);
		break;

	case GI_RESETM2:
		if (UNIFCart.Reset)
			UNIFCart.Reset();
		break;
	case GI_POWER:
		if (UNIFCart.Power)
			UNIFCart.Power();
		if (UNIFchrrama) memset(UNIFchrrama, 0, 8192);
		break;
	case GI_CLOSE:
		FCEU_SaveGameSave(&UNIFCart);
		if (UNIFCart.Close)
			UNIFCart.Close();
		FreeUNIF();
		break;
	}
}

int UNIFLoad(const char *name, FCEUFILE *fp) {
	FCEU_fseek(fp, 0, SEEK_SET);
	FCEU_fread(&unhead, 1, 4, fp);
	if (memcmp(&unhead, "UNIF", 4))
		return LOADER_INVALID_FORMAT;

	ResetCartMapping();

	ResetExState(0, 0);
	ResetUNIF();
	if (!FCEU_read32le(&unhead.info, fp)
		|| (FCEU_fseek(fp, 0x20, SEEK_SET) < 0)
		|| !LoadUNIFChunks(fp))
	{
		FreeUNIF();
		ResetUNIF();
		FCEU_PrintError("Error reading UNIF ROM image.");
		return LOADER_HANDLED_ERROR;
	}

	struct md5_context md5;
	md5_starts(&md5);
	for (int x = 0; x < 32; x++)
		if (malloced[x]) {
			md5_update(&md5, malloced[x], mallocedsizes[x]);
		}
	md5_finish(&md5, UNIFCart.MD5);
	FCEU_printf(" ROM MD5:  0x");
	for (int x = 0; x < 16; x++)
		FCEU_printf("%02x", UNIFCart.MD5[x]);
	FCEU_printf("\n");
	memcpy(&GameInfo->MD5, &UNIFCart.MD5, sizeof(UNIFCart.MD5));

	int result = InitializeBoard();
	switch (result)
	{
	case 0:
		goto init_ok;
	case 1:
		FCEU_PrintError("UNIF mapper \"%s\" is not supported at all.", sboardname);
		break;
	case 2:
		FCEU_PrintError("Unable to allocate CHR-RAM.");
		break;
	}
	FreeUNIF();
	ResetUNIF();
	return LOADER_HANDLED_ERROR;

init_ok:

	FCEU_LoadGameSave(&UNIFCart);
	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name); //For the debugger list
	GameInterface = UNIFGI;
	currCartInfo = &UNIFCart;
	return LOADER_OK;
}
