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

#include "types.h"
#include "utils/safe_string.h"
#include "x6502.h"
#include "fceu.h"
#include "cart.h"
#include "ppu.h"

#include "ines.h"
#include "unif.h"
#include "state.h"
#include "file.h"
#include "utils/general.h"
#include "utils/memory.h"
#include "utils/crc32.h"
#include "utils/md5.h"
#include "utils/xstring.h"
#include "cheat.h"
#include "vsuni.h"
#include "driver.h"
#include "input.h"

#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

extern SFORMAT FCEUVSUNI_STATEINFO[];

//mbg merge 6/29/06 - these need to be global
uint8 *trainerpoo = NULL;
static FceuMallocPtr trainerpoo_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
uint8 *ROM = NULL;
uint8 *VROM = NULL;
uint8 *ExtraNTARAM = NULL;
static FceuMallocPtr ExtraNTARAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
iNES_HEADER head;

static CartInfo iNESCart;

uint8 Mirroring = 0;
uint8 MirroringAs2bits = 0;
uint32 ROM_size = 0;
uint32 VROM_size = 0;
char LoadedRomFName[4096]; //mbg merge 7/17/06 added
char LoadedRomFNamePatchToUse[4096];

static int CHRRAMSize = -1;
static int iNES_Init(int num);

static int MapperNo = 0;

int iNES2 = 0;

static DECLFR(TrainerRead) {
	return(trainerpoo[A & 0x1FF]);
}

static void iNES_ExecPower() {
	if (CHRRAMSize != -1)
		FCEU_MemoryRand(VROM, CHRRAMSize);

	if (iNESCart.Power)
		iNESCart.Power();

	if (trainerpoo) {
		int x;
		for (x = 0; x < 512; x++) {
			X6502_DMW(0x7000 + x, trainerpoo[x]);
			if (X6502_DMR(0x7000 + x) != trainerpoo[x]) {
				SetReadHandler(0x7000, 0x71FF, TrainerRead);
				break;
			}
		}
	}
}

void iNESGI(GI h) { //bbit edited: removed static keyword
	switch (h) {
	case GI_RESETSAVE:
		FCEU_ClearGameSave(&iNESCart);
		break;

	case GI_RESETM2:
		if (iNESCart.Reset)
			iNESCart.Reset();
		break;
	case GI_POWER:
		iNES_ExecPower();
		break;
	case GI_CLOSE:
	{
		FCEU_SaveGameSave(&iNESCart);
		if (iNESCart.Close)
			iNESCart.Close();
		if (ROM) {
			FCEU_free(ROM);
			ROM = NULL;
		}
		if (VROM) {
			FCEU_free(VROM);
			VROM = NULL;
		}
		if (trainerpoo) {
			free(trainerpoo);
			trainerpoo = NULL;
		}
		if (ExtraNTARAM) {
			free(ExtraNTARAM);
			ExtraNTARAM = NULL;
		}
	}
	break;
	}
}

uint32 iNESGameCRC32 = 0;

struct CRCMATCH {
	uint32 crc;
	char *name;
};

/*
* Function to set input controllers based on CRC
*/
static void SetInput(void) {
	int32_t i1 = 0, i2 = 0, ifc = 0;
	if (fceux11_rust_ines_lookup_input_crc(iNESGameCRC32, &i1, &i2, &ifc)) {
		GameInfo->input[0] = static_cast<ESI>(i1);
		GameInfo->input[1] = static_cast<ESI>(i2);
		GameInfo->inputfc = static_cast<ESIFC>(ifc);
	}
}

/*
* Function to set input controllers based on NES 2.0 header
*/
extern int eoptions;
static void SetInputNes20(uint8 expansion) {
	int32_t i1 = 0, i2 = 0, ifc = 0, eopt = 0, vsc = 0;
	if (fceux11_rust_ines_lookup_input_nes20(expansion, &i1, &i2, &ifc, &eopt, &vsc)) {
		GameInfo->input[0] = static_cast<ESI>(i1);
		GameInfo->input[1] = static_cast<ESI>(i2);
		GameInfo->inputfc = static_cast<ESIFC>(ifc);
	}
	eoptions |= eopt;
	GameInfo->vs_cswitch = vsc != 0;
}

#define INESB_INCOMPLETE  1
#define INESB_CORRUPT     2
#define INESB_HACKED      4

void CheckBad(uint64 md5partial) {
	const char* name = fceux11_rust_ines_check_bad(md5partial);
	if (name) {
		FCEU_PrintError("The copy game you have loaded, \"%s\", is bad, and will not work properly in FCEUX.", name);
	}
}


struct CHINF {
	uint32 crc32;
	int32 mapper;
	int32 mirror;
	const char* params;
};

static const TMasterRomInfo sMasterRomInfo[] = {
	{ 0x62b51b108a01d2beULL, "bonus=0" }, //4-in-1 (FK23C8021)[p1][!].nes
	{ 0x8bb48490d8d22711ULL, "bonus=0" }, //4-in-1 (FK23C8033)[p1][!].nes
	{ 0xc75888d7b48cd378ULL, "bonus=0" }, //4-in-1 (FK23C8043)[p1][!].nes
	{ 0xf81a376fa54fdd69ULL, "bonus=0" }, //4-in-1 (FK23Cxxxx, S-0210A PCB)[p1][!].nes
	{ 0xa37eb9163e001a46ULL, "bonus=0" }, //4-in-1 (FK23C8026) [p1][!].nes
	{ 0xde5ce25860233f7eULL, "bonus=0" }, //4-in-1 (FK23C8045) [p1][!].nes
	{ 0x5b3aa4cdc484a088ULL, "bonus=0" }, //4-in-1 (FK23C8056) [p1][!].nes
	{ 0x9342bf9bae1c798aULL, "bonus=0" }, //4-in-1 (FK23C8079) [p1][!].nes
	{ 0x164eea6097a1e313ULL, "busc=1" }, //Cybernoid - The Fighting Machine (U)[!].nes -- needs bus conflict emulation
};
const TMasterRomInfo* MasterRomInfo;
TMasterRomInfoParams MasterRomInfoParams;

static void CheckHInfo(uint64 partialmd5) {
	FceuInesHInfoResult res;
	fceux11_rust_ines_check_hinfo(iNESGameCRC32, partialmd5, &res);

	int32 tofix = 0;

	// sMasterRomInfo query kept in C++ for TMasterRomInfoParams population
	MasterRomInfo = NULL;
	MasterRomInfoParams.clear();
	for (size_t i = 0; i < ARRAY_SIZE(sMasterRomInfo); i++) {
		const TMasterRomInfo& info = sMasterRomInfo[i];
		if (info.md5lower != partialmd5)
			continue;

		MasterRomInfo = &info;
		if (!info.params) break;

		std::vector<std::string> toks = tokenize_str(info.params, ",");
		for (size_t j = 0; j < toks.size(); j++) {
			std::vector<std::string> parts = tokenize_str(toks[j], "=");
			MasterRomInfoParams[parts[0]] = parts[1];
		}
		break;
	}

	// Apply Rust-computed corrections
	if (res.clear_vrom && VROM_size) {
		VROM_size = 0;
		free(VROM);
		VROM = NULL;
		tofix |= 8;
	}
	if (res.mapper >= 0 && MapperNo != res.mapper) {
		tofix |= 1;
		MapperNo = res.mapper;
	}
	if (res.mirror >= 0) {
		if (res.mirror == 8) {
			if (Mirroring == 2) {	/* Anything but hard-wired(four screen). */
				tofix |= 2;
				Mirroring = 0;
			}
		} else if (Mirroring != res.mirror) {
			if (Mirroring != (res.mirror & ~4))
				if ((res.mirror & ~4) <= 2)	/* Don't complain if one-screen mirroring
												needs to be set(the iNES header can't
												hold this information).
												*/
						tofix |= 2;
			Mirroring = res.mirror;
		}
	}
	if (res.force_battery) {
		if (!(head.ROM_type & 2)) {
			tofix |= 4;
			head.ROM_type |= 2;
		}
	}

	/* Games that use these iNES mappers tend to have the four-screen bit set
	when it should not be.
	*/
	if ((MapperNo == 118 || MapperNo == 24 || MapperNo == 26) && (Mirroring == 2)) {
		Mirroring = 0;
		tofix |= 2;
	}

	/* Four-screen mirroring implicitly set. */
	if (MapperNo == 99)
		Mirroring = 2;

	if (tofix) {
		char gigastr[768];
		FCEU_strlcpy(gigastr, sizeof(gigastr), "The iNES header contains incorrect information.  For now, the information will be corrected in RAM.  ");
		if (tofix & 1)
			snprintf( gigastr + strlen(gigastr), sizeof(gigastr + strlen(gigastr)), "The mapper number should be set to %d.  ", MapperNo);
		if (tofix & 2) {
			const char *mstr[3] = { "Horizontal", "Vertical", "Four-screen" };
			snprintf( gigastr + strlen(gigastr), sizeof(gigastr + strlen(gigastr)), "Mirroring should be set to \"%s\".  ", mstr[Mirroring & 3]);
		}
		if (tofix & 4)
			safe_strcat(gigastr, sizeof(gigastr), "The battery-backed bit should be set.  ");
		if (tofix & 8)
			safe_strcat(gigastr, sizeof(gigastr), "This game should not have any CHR ROM.  ");
		safe_strcat(gigastr, sizeof(gigastr), "\n");

		FCEU_printf("%s", gigastr);
	}
}


typedef struct {
	int32 mapper;
	void (*init)(CartInfo *);
} NewMI;

//this is for games that is not the a power of 2
//mapper based for now...
//not really accurate but this works since games
//that are not in the power of 2 tends to come
//in obscure mappers themselves which supports such
//size
//Cah4e3 25.10.19: iNES 2.0 attempts to cover all
// boards including UNIF boards with non power 2 
// total rom size (a lot of them with a couple of 
// roms different sizes (may vary a lot)
// so we need either add here ALL ines 2.0 mappers 
// with not power2 roms or change logic here
// to something more unified for ines 2.0 specific
static int not_power2[] =
{
	53, 198, 228, 547
};

BMAPPINGLocal bmap[] = {
	{"NROM",				  0, NROM_Init},
	{"MMC1",				  1, Mapper1_Init},
	{"UNROM",				  2, UNROM_Init},
	{"CNROM",				  3, CNROM_Init},
	{"MMC3",				  4, Mapper4_Init},
	{"MMC5",				  5, Mapper5_Init},
	{"FFE Rev. A",			  6, Mapper6_Init},
	{"ANROM",				  7, ANROM_Init},
	{"",					  8, Mapper8_Init},		// Nogaems, it's worthless
	{"MMC2",				  9, Mapper9_Init},
	{"MMC4",				 10, Mapper10_Init},
	{"Color Dreams",		 11, Mapper11_Init},
	{"REX DBZ 5",			 12, Mapper12_Init},
	{"CPROM",				 13, CPROM_Init},
	{"REX SL-1632",			 14, UNLSL1632_Init},
	{"100-in-1",			 15, Mapper15_Init},
	{"BANDAI 24C02",		 16, Mapper16_Init},
	{"FFE Rev. B",			 17, Mapper17_Init},
	{"JALECO SS880006",		 18, Mapper18_Init},	// JF-NNX (EB89018-30007) boards
	{"Namcot 106",			 19, Mapper19_Init},
//	{"",					 20, Mapper20_Init},
	{"Konami VRC2/VRC4 A",	 21, Mapper21_Init},
	{"Konami VRC2/VRC4 B",	 22, Mapper22_Init},
	{"Konami VRC2/VRC4 C",	 23, Mapper23_Init},
	{"Konami VRC6 Rev. A",	 24, Mapper24_Init},
	{"Konami VRC2/VRC4 D",	 25, Mapper25_Init},
	{"Konami VRC6 Rev. B",	 26, Mapper26_Init},
	{"CC-21 MI HUN CHE",	 27, UNLCC21_Init},		// Former dupe for VRC2/VRC4 mapper, redefined with crc to mihunche boards
	{"",					 28, Mapper28_Init},
	{"RET-CUFROM",			 29, Mapper29_Init},
	{"UNROM 512",			 30, UNROM512_Init},
	{"infiniteneslives-NSF", 31, Mapper31_Init},
	{"IREM G-101",			 32, Mapper32_Init},
	{"TC0190FMC/TC0350FMR",	 33, Mapper33_Init},
	{"IREM I-IM/BNROM",		 34, Mapper34_Init},
	{"Wario Land 2",		 35, UNLSC127_Init},
	{"TXC Policeman",		 36, Mapper36_Init},
	{"PAL-ZZ SMB/TETRIS/NWC",37, Mapper37_Init},
	{"Bit Corp.",			 38, Mapper38_Init},	// Crime Busters
//	{"",					 39, Mapper39_Init},
	{"SMB2j FDS",			 40, Mapper40_Init},
	{"CALTRON 6-in-1",		 41, Mapper41_Init},
	{"BIO MIRACLE FDS",		 42, Mapper42_Init},
	{"FDS SMB2j LF36",		 43, Mapper43_Init},
	{"MMC3 BMC PIRATE A",	 44, Mapper44_Init},
	{"MMC3 BMC PIRATE B",	 45, Mapper45_Init},
	{"RUMBLESTATION 15-in-1",46, Mapper46_Init},
	{"NES-QJ SSVB/NWC",		 47, Mapper47_Init},
	{"TAITO TCxxx",			 48, Mapper48_Init},
	{"MMC3 BMC PIRATE C",	 49, Mapper49_Init},
	{"SMB2j FDS Rev. A",	 50, Mapper50_Init},
	{"11-in-1 BALL SERIES",	 51, Mapper51_Init},	// 1993 year version
	{"MMC3 BMC PIRATE D",	 52, Mapper52_Init},
	{"SUPERVISION 16-in-1",	 53, Supervision16_Init},
//	{"",					 54, Mapper54_Init},
//	{"",					 55, Mapper55_Init},
//	{"",					 56, Mapper56_Init},
	{"SIMBPLE BMC PIRATE A", 57, Mapper57_Init},
	{"SIMBPLE BMC PIRATE B", 58, BMCGK192_Init},
	{"",					 59, Mapper59_Init},	// Check this out
	{"SIMBPLE BMC PIRATE C", 60, BMCD1038_Init},
	{"20-in-1 KAISER Rev. A",61, Mapper61_Init},
	{"700-in-1",			 62, Mapper62_Init},
//	{"",					 63, Mapper63_Init},
	{"TENGEN RAMBO1",		 64, Mapper64_Init},
	{"IREM-H3001",			 65, Mapper65_Init},
	{"MHROM",				 66, MHROM_Init},
	{"SUNSOFT-FZII",		 67, Mapper67_Init},
	{"Sunsoft Mapper #4",	 68, Mapper68_Init},
	{"SUNSOFT-5/FME-7",		 69, Mapper69_Init},
	{"BA KAMEN DISCRETE",	 70, Mapper70_Init},
	{"CAMERICA BF9093",		 71, Mapper71_Init},
	{"JALECO JF-17",		 72, Mapper72_Init},
	{"KONAMI VRC3",			 73, Mapper73_Init},
	{"TW MMC3+VRAM Rev. A",	 74, Mapper74_Init},
	{"KONAMI VRC1",			 75, Mapper75_Init},
	{"NAMCOT 108 Rev. A",	 76, Mapper76_Init},
	{"IREM LROG017",		 77, Mapper77_Init},
	{"Irem 74HC161/32",		 78, Mapper78_Init},
	{"AVE/C&E/TXC BOARD",	 79, Mapper79_Init},
	{"TAITO X1-005 Rev. A",	 80, Mapper80_Init},
//	{"",					 81, Mapper81_Init},
	{"TAITO X1-017",		 82, Mapper82_Init},
	{"YOKO VRC Rev. B",		 83, Mapper83_Init},
//	{"",					 84, Mapper84_Init},
	{"KONAMI VRC7",			 85, Mapper85_Init},
	{"JALECO JF-13",		 86, Mapper86_Init},
	{"74*139/74 DISCRETE",	 87, Mapper87_Init},
	{"NAMCO 3433",			 88, Mapper88_Init},
	{"SUNSOFT-3",			 89, Mapper89_Init},	// SUNSOFT-2 mapper
	{"HUMMER/JY BOARD",		 90, Mapper90_Init},
	{"EARLY HUMMER/JY BOARD",91, Mapper91_Init},
	{"JALECO JF-19",		 92, Mapper92_Init},
	{"SUNSOFT-3R",			 93, SUNSOFT_UNROM_Init},// SUNSOFT-2 mapper with VRAM, different wiring
	{"HVC-UN1ROM",			 94, Mapper94_Init},
	{"NAMCOT 108 Rev. B",	 95, Mapper95_Init},
	{"BANDAI OEKAKIDS",		 96, Mapper96_Init},
	{"IREM TAM-S1",			 97, Mapper97_Init},
//	{"",					 98, Mapper98_Init},
	{"VS Uni/Dual- system",	 99, Mapper99_Init},
//	{"",					100, Mapper100_Init},
	{"",					101, Mapper101_Init},
//	{"",					102, Mapper102_Init},
	{"FDS DOKIDOKI FULL",	103, Mapper103_Init},
//	{"",					104, Mapper104_Init},
	{"NES-EVENT NWC1990",	105, Mapper105_Init},
	{"SMB3 PIRATE A",		106, Mapper106_Init},
	{"MAGIC CORP A",		107, Mapper107_Init},
	{"FDS UNROM BOARD",		108, Mapper108_Init},
//	{"",					109, Mapper109_Init},
//	{"",					110, Mapper110_Init},
	{"Cheapocabra",			111, Mapper111_Init},
	{"ASDER/NTDEC BOARD",	112, Mapper112_Init},
	{"HACKER/SACHEN BOARD",	113, Mapper113_Init},
	{"MMC3 SG PROT. A",		114, Mapper114_Init},
	{"MMC3 PIRATE A",		115, Mapper115_Init},
	{"MMC1/MMC3/VRC PIRATE",116, UNLSL12_Init},
	{"FUTURE MEDIA BOARD",	117, Mapper117_Init},
	{"TSKROM",				118, TKSROM_Init},
	{"NES-TQROM",			119, Mapper119_Init},
	{"FDS TOBIDASE",		120, Mapper120_Init},
	{"MMC3 PIRATE PROT. A",	121, Mapper121_Init},
//	{"",					122, Mapper122_Init},
	{"MMC3 PIRATE H2288",	123, UNLH2288_Init},
//	{"",					124, Mapper124_Init},
	{"FDS LH32",			125, LH32_Init},
//	{"",					126, Mapper126_Init},
//	{"",					127, Mapper127_Init},
//	{"",					128, Mapper128_Init},
//	{"",					129, Mapper129_Init},
//	{"",					130, Mapper130_Init},
//	{"",					131, Mapper131_Init},
	{"TXC/MGENIUS 22111",	132, UNL22211_Init},
	{"SA72008",				133, SA72008_Init},
	{"MMC3 BMC PIRATE",		134, Mapper134_Init},
//	{"",					135, Mapper135_Init},
	{"TCU02",				136, TCU02_Init},
	{"S8259D",				137, S8259D_Init},
	{"S8259B",				138, S8259B_Init},
	{"S8259C",				139, S8259C_Init},
	{"JALECO JF-11/14",		140, Mapper140_Init},
	{"S8259A",				141, S8259A_Init},
	{"UNLKS7032",			142, UNLKS7032_Init},
	{"TCA01",				143, TCA01_Init},
	{"AGCI 50282",			144, Mapper144_Init},
	{"SA72007",				145, SA72007_Init},
	{"SA0161M",				146, SA0161M_Init},
	{"TCU01",				147, TCU01_Init},
	{"SA0037",				148, SA0037_Init},
	{"SA0036",				149, SA0036_Init},
	{"S74LS374N",			150, S74LS374N_Init},
	{"",					151, Mapper151_Init},
	{"",					152, Mapper152_Init},
	{"BANDAI SRAM",			153, Mapper153_Init},	// Bandai board 16 with SRAM instead of EEPROM
	{"",					154, Mapper154_Init},
	{"",					155, Mapper155_Init},
	{"",					156, Mapper156_Init},
	{"BANDAI BARCODE",		157, Mapper157_Init},
//	{"",					158, Mapper158_Init},
	{"BANDAI 24C01",		159, Mapper159_Init},	// Different type of EEPROM on the  bandai board
	{"SA009",				160, SA009_Init},
//	{"",					161, Mapper161_Init},
	{"",					162, UNLFS304_Init},
	{"",					163, Mapper163_Init},
	{"",					164, Mapper164_Init},
	{"",					165, Mapper165_Init},
	{"SUBOR Rev. A",		166, Mapper166_Init},
	{"SUBOR Rev. B",		167, Mapper167_Init},
	{"",					168, Mapper168_Init},
//	{"",					169, Mapper169_Init},
	{"",					170, Mapper170_Init},
	{"",					171, Mapper171_Init},
	{"",					172, Mapper172_Init},
	{"",					173, Mapper173_Init},
	{"NTDec 5-in-1",		174, Mapper174_Init},
	{"",					175, Mapper175_Init},
	{"BMCFK23C",			176, BMCFK23C_Init},	// zero 26-may-2012 - well, i have some WXN junk games that use 176 for instance ????. i dont know what game uses this BMCFK23C as mapper 176. we'll have to make a note when we find it.
	{"",					177, Mapper177_Init},
	{"",					178, Mapper178_Init},
//	{"",					179, Mapper179_Init},
	{"",					180, Mapper180_Init},
	{"",					181, Mapper181_Init},
//	{"",					182, Mapper182_Init},	// Deprecated, dupe
	{"",					183, Mapper183_Init},
	{"",					184, Mapper184_Init},
	{"",					185, Mapper185_Init},
	{"",					186, Mapper186_Init},
	{"",					187, Mapper187_Init},
	{"",					188, Mapper188_Init},
	{"",					189, Mapper189_Init},
	{"",					190, Mapper190_Init},
	{"",					191, Mapper191_Init},
	{"TW MMC3+VRAM Rev. B",	192, Mapper192_Init},
	{"NTDEC TC-112",		193, Mapper193_Init},	// War in the Gulf
	{"TW MMC3+VRAM Rev. C",	194, Mapper194_Init},
	{"TW MMC3+VRAM Rev. D",	195, Mapper195_Init},
	{"",					196, Mapper196_Init},
	{"",					197, Mapper197_Init},
	{"TW MMC3+VRAM Rev. E",	198, Mapper198_Init},
	{"",					199, Mapper199_Init},
	{"",					200, Mapper200_Init},
	{"",					201, Mapper201_Init},
	{"",					202, Mapper202_Init},
	{"",					203, Mapper203_Init},
	{"",					204, Mapper204_Init},
	{"JC-016-2",			205, Mapper205_Init},
	{"NAMCOT 108 Rev. C",	206, Mapper206_Init},	// Deprecated, Used to be "DEIROM" whatever it means, but actually simple version of MMC3
	{"TAITO X1-005 Rev. B",	207, Mapper207_Init},
	{"",					208, Mapper208_Init},
	{"",					209, Mapper209_Init},
	{"",					210, Mapper210_Init},
	{"",					211, Mapper211_Init},
	{"",					212, Mapper212_Init},
	{"",					213, Mapper213_Init},
	{"",					214, Mapper214_Init},
	{"",					215, UNL8237_Init},
	{"",					216, Mapper216_Init},
	{"",					217, Mapper217_Init},	// Redefined to a new Discrete BMC mapper
	{"",					218, Mapper218_Init},
	{"UNLA9746",			219, UNLA9746_Init},
	{"Debug Mapper",		220, QTAi_Init},
	{"UNLN625092",			221, UNLN625092_Init},
	{"",					222, Mapper222_Init},
//	{"",					223, Mapper223_Init},
//	{"",					224, Mapper224_Init},
	{"",					225, Mapper225_Init},
	{"BMC 22+20-in-1",		226, Mapper226_Init},
	{"",					227, Mapper227_Init},
	{"",					228, Mapper228_Init},
	{"",					229, Mapper229_Init},
	{"BMC Contra+22-in-1",	230, Mapper230_Init},
	{"",					231, Mapper231_Init},
	{"BMC QUATTRO",			232, Mapper232_Init},
	{"BMC 22+20-in-1 RST",	233, Mapper233_Init},
	{"BMC MAXI",			234, Mapper234_Init},
	{"",					235, Mapper235_Init},
//	{"",					236, Mapper236_Init},
//	{"",					237, Mapper237_Init},
	{"UNL6035052",			238, UNL6035052_Init},
//	{"",					239, Mapper239_Init},
	{"",					240, Mapper240_Init},
	{"",					241, Mapper241_Init},
	{"",					242, Mapper242_Init},
	{"S74LS374NA",			243, S74LS374NA_Init},
	{"DECATHLON",			244, Mapper244_Init},
	{"",					245, Mapper245_Init},
	{"FONG SHEN BANG",		246, Mapper246_Init},
//	{"",					247, Mapper247_Init},
//	{"",					248, Mapper248_Init},
	{"",					249, Mapper249_Init},
	{"",					250, Mapper250_Init},
//	{"",					251, Mapper251_Init},	// No good dumps for this mapper, use UNIF version
	{"SAN GUO ZHI PIRATE",	252, Mapper252_Init},
	{"DRAGON BALL PIRATE",	253, Mapper253_Init},
	{"",					254, Mapper254_Init},
	{"",					255, Mapper255_Init},	// dupe of 225

//-------- Mappers 256-511 is the Supplementary Multilingual Plane ----------
//-------- Mappers 512-767 is the Supplementary Ideographic Plane -----------
//-------- Mappers 3840-4095 are for rom dumps not publicly released --------

//	An attempt to make working the UNIF BOARD ROMs in INES FORMAT
//  I don't know if there a complete ines 2.0 mapper list exist, so if it does,
//  just redefine these numbers to any others which isn't used before
//  see the ines-correct.h files for the ROMs CHR list

	{"ONE-BUS Systems",		256, UNLOneBus_Init},
	{"PEC-586 Computer",	257, UNLPEC586Init},
	{"158B Prot Board",		258, UNL158B_Init},
	{"F-15 MMC3 Based",		259, BMCF15_Init},
	{"HP10xx/H20xx Boards",	260, BMCHPxx_Init},
	{"810544-CA-1",			261, BMC810544CA1_Init},
	{"AA6023/AA6023B",		268, AA6023_Init},
	{"OK-411",				361, GN45_Init},
	{"GN-45",				366, GN45_Init},
	{"COOLGIRL",			342, COOLGIRL_Init },
	{"FAM250/81-01-39-C/SCHI-24",			354, Mapper354_Init },

	{"Impact Soft MMC3 Flash Board",	406, Mapper406_Init },
	{"INX_007T_V01",		470, INX_007T_Init },

	{"KONAMI QTAi Board",	547, QTAi_Init },

	{"",					0, NULL}
};

int iNESLoad(const char *name, FCEUFILE *fp, int OverwriteVidMode) {
	int result;
	struct md5_context md5;
	uint64 partialmd5 = 0;
	const char* mappername = "Not Listed";

	if (FCEU_fread(&head, 1, 16, fp) != 16 || memcmp(&head, "NES\x1A", 4))
		return LOADER_INVALID_FORMAT;
	
	fceux11_rust_ines_header_cleanup((uint8*)&head);

	iNESCart.clear();

	iNES2 = ((head.ROM_type2 & 0x0C) == 0x08);
	if(iNES2)
	{
		iNESCart.ines2 = true;
		iNESCart.wram_size = (head.RAM_size & 0x0F)?(64 << (head.RAM_size & 0x0F)):0;
		iNESCart.battery_wram_size = (head.RAM_size & 0xF0)?(64 << ((head.RAM_size & 0xF0)>>4)):0;
		iNESCart.vram_size = (head.VRAM_size & 0x0F)?(64 << (head.VRAM_size & 0x0F)):0;
		iNESCart.battery_vram_size = (head.VRAM_size & 0xF0)?(64 << ((head.VRAM_size & 0xF0)>>4)):0;
		iNESCart.submapper = head.ROM_type3 >> 4;
	}

	MapperNo = (head.ROM_type >> 4);
	MapperNo |= (head.ROM_type2 & 0xF0);
	if(iNES2) MapperNo |= ((head.ROM_type3 & 0x0F) << 8);
	
	if (head.ROM_type & 8) {
		Mirroring = 2;
	} else
		Mirroring = (head.ROM_type & 1);

	MirroringAs2bits = head.ROM_type & 1;
	if (head.ROM_type & 8) MirroringAs2bits |= 2;

	FceuRomSizes sizes;
	fceux11_rust_cart_compute_rom_sizes((const FceuInesHeader*)&head, iNES2, &sizes);
	ROM_size = sizes.rom_size_16kb;
	VROM_size = sizes.vrom_size_8kb;
	uint32 not_round_size = sizes.rom_size_raw;

	int round = !fceux11_rust_ines_not_power2(MapperNo);

	ROM = (uint8*)FCEU_malloc(ROM_size << 14);
	memset(ROM, 0xFF, ROM_size << 14);

	if (VROM_size) {
		VROM = (uint8*)FCEU_malloc(VROM_size << 13);
		memset(VROM, 0xFF, VROM_size << 13);
	}

	// Set Vs. System flag if need
	if (!iNES2) {
		GameInfo->type = !(head.ROM_type2 & 1) ? GIT_CART : GIT_VSUNI;
	}
	else {
		switch (!(head.ROM_type2 & 2) ? (head.ROM_type2 & 3) : (head.VS_hardware & 0xF)) {
		case 0: 
			GameInfo->type = GIT_CART;
			break;
		case 1:
			GameInfo->type = GIT_VSUNI;
			break;
		default:
			FCEU_PrintError("Game type is not supported at all.");
			goto init_error;
		}
	}

	// Set Vs. System PPU type if need
	if (GameInfo->type == GIT_VSUNI && !(head.ROM_type2 & 2)) {
		switch (head.VS_hardware & 0xF) { 
		case 0x0: GameInfo->vs_ppu = GIPPU_RC2C03B; break;
		//case 0x1: GameInfo->vs_ppu = GIPPU_RPC2C03C; break;
		case 0x2: GameInfo->vs_ppu = GIPPU_RP2C04_0001; break;
		case 0x3: GameInfo->vs_ppu = GIPPU_RP2C04_0002; break;
		case 0x4: GameInfo->vs_ppu = GIPPU_RP2C04_0003; break;
		case 0x5: GameInfo->vs_ppu = GIPPU_RP2C04_0004; break;
		case 0x6: GameInfo->vs_ppu = GIPPU_RC2C03B; break;
		//case 0x7: GameInfo->ppu = GIPPU_RPC2C03C; break;
		case 0x8: GameInfo->vs_ppu = GIPPU_RC2C05_01; break;
		case 0x9: GameInfo->vs_ppu = GIPPU_RC2C05_02; break;
		case 0xA: GameInfo->vs_ppu = GIPPU_RC2C05_03; break;
		case 0xB: GameInfo->vs_ppu = GIPPU_RC2C05_04; break;
		//case 0xC: GameInfo->ppu = GIPPU_RPC2C05_05; break;
		default:
			FCEU_PrintError("Vs. System PPU type is not supported at all.");
			goto init_error;
		}

		switch (head.VS_hardware >> 4) {
		case 0x0: GameInfo->vs_type = EGIVS_NORMAL; break;
		case 0x1: GameInfo->vs_type = EGIVS_RBI; break;
		case 0x2: GameInfo->vs_type = EGIVS_TKO; break;
		case 0x3: GameInfo->vs_type = EGIVS_XEVIOUS; break;
		default:
			FCEU_PrintError("Vs. System type is not supported at all.");
			goto init_error;
		}
	}

	if (head.ROM_type & 4) {	/* Trainer */
		trainerpoo_owner = FCEU_gmalloc_unique(512);  // v0.3.6: RAII-wrapped
		trainerpoo = trainerpoo_owner.get();
		FCEU_fread(trainerpoo, 512, 1, fp);
	}

	ResetCartMapping();
	ResetExState(0, 0);

	SetupCartPRGMapping(0, ROM, ROM_size << 14, 0);

	FCEU_fread(ROM, 0x4000, (round) ? ROM_size : not_round_size, fp);

	if (VROM_size)
		FCEU_fread(VROM, 0x2000, VROM_size, fp);

	md5_starts(&md5); 
	md5_update(&md5, ROM, ROM_size << 14);

	iNESGameCRC32 = CalcCRC32(0, ROM, ROM_size << 14);

	if (VROM_size) {
		iNESGameCRC32 = CalcCRC32(iNESGameCRC32, VROM, VROM_size << 13);
		md5_update(&md5, VROM, VROM_size << 13);
	}
	md5_finish(&md5, iNESCart.MD5);
	memcpy(&GameInfo->MD5, &iNESCart.MD5, sizeof(iNESCart.MD5));
	for (int x = 0; x < 8; x++)
		partialmd5 |= (uint64)iNESCart.MD5[7 - x] << (x * 8);

	iNESCart.CRC32 = iNESGameCRC32;

	FCEU_printf(" PRG ROM: %d x 16KiB = %d KiB\n", round ? ROM_size : not_round_size, (round ? ROM_size : not_round_size) * 16);
	FCEU_printf(" CHR ROM: %d x  8KiB = %d KiB\n", VROM_size, VROM_size * 8);
	FCEU_printf(" ROM CRC32: 0x%08x\n", iNESGameCRC32);
	{
		int x;
		FCEU_printf(" ROM MD5:  0x");
		for(x=0;x<16;x++)
			FCEU_printf("%02x",iNESCart.MD5[x]);
		FCEU_printf("\n");
	}

	{
		const char* rust_name = fceux11_rust_ines_mapper_name(MapperNo);
		if (rust_name) mappername = rust_name;
	}

	FCEU_printf(" Mapper #: %d\n", MapperNo);
	FCEU_printf(" Mapper name: %s\n", mappername);
	FCEU_printf(" Mirroring: %s\n", Mirroring == 2 ? "None (Four-screen)" : Mirroring ? "Vertical" : "Horizontal");
	FCEU_printf(" Battery-backed: %s\n", (head.ROM_type & 2) ? "Yes" : "No");
	FCEU_printf(" Trained: %s\n", (head.ROM_type & 4) ? "Yes" : "No");
	if(iNES2) 
	{
		FCEU_printf(" NES2.0 Extensions\n");
		FCEU_printf(" Sub Mapper #: %d\n", iNESCart.submapper);
		FCEU_printf(" Total WRAM size: %d KiB\n", (iNESCart.wram_size + iNESCart.battery_wram_size) / 1024);
		FCEU_printf(" Total VRAM size: %d KiB\n", (iNESCart.vram_size + iNESCart.battery_vram_size) / 1024);
		if(head.ROM_type & 2)
		{
			FCEU_printf(" WRAM backed by battery: %d KiB\n", iNESCart.battery_wram_size / 1024);
			FCEU_printf(" VRAM backed by battery: %d KiB\n", iNESCart.battery_vram_size / 1024);
		}
	}

	SetInput();
	// Input can be overriden by NES 2.0 header
	if (iNES2) SetInputNes20(head.expansion);
	CheckHInfo(partialmd5);
	FCEU_VSUniCheck(partialmd5, &MapperNo, &Mirroring);
	CheckBad(partialmd5);

	/* Must remain here because above functions might change value of
	VROM_size and free(VROM).
	*/
	if (VROM_size)
		SetupCartCHRMapping(0, VROM, VROM_size * 0x2000, 0);

	if (Mirroring == 2) {
		ExtraNTARAM_owner = FCEU_gmalloc_unique(2048);  // v0.3.6: RAII-wrapped
		ExtraNTARAM = ExtraNTARAM_owner.get();
		SetupCartMirroring(4, 1, ExtraNTARAM);
	} else if (Mirroring >= 0x10)
		SetupCartMirroring(2 + (Mirroring & 1), 1, 0);
	else
		SetupCartMirroring(Mirroring & 1, (Mirroring & 4) >> 2, 0);

	iNESCart.battery = (head.ROM_type & 2) ? 1 : 0;
	iNESCart.mirror = Mirroring;
	iNESCart.mirrorAs2Bits = MirroringAs2bits;

	result = iNES_Init(MapperNo);
	switch(result)
	{
	case 0:
		goto init_ok;
	case 1:
		FCEU_PrintError("iNES mapper #%d is not supported at all.", MapperNo);
		break;
	case 2:
		FCEU_PrintError("Unable to allocate CHR-RAM.");
		break;
	}

init_error:
	fceu11::assign_cart(nullptr);
	if (ROM) free(ROM);
	if (VROM) free(VROM);
	if (trainerpoo) free(trainerpoo);
	if (ExtraNTARAM) free(ExtraNTARAM);
	ROM = NULL;
	VROM = NULL;
	trainerpoo = NULL;
	ExtraNTARAM = NULL;
	return LOADER_HANDLED_ERROR;

init_ok:

	GameInfo->mappernum = MapperNo;
	FCEU_LoadGameSave(&iNESCart);

	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name); //bbit edited: line added

	// Extract Filename only. Should account for Windows/Unix this way.
	if (strrchr(name, '/')) {
		name = strrchr(name, '/') + 1;
	} else if (strrchr(name, '\\')) {
		name = strrchr(name, '\\') + 1;
	}

	GameInterface = iNESGI;
	currCartInfo = &iNESCart;

	// v1.7 Phase D: install a concrete Cart subclass when the factory returns
	// one. Phase D factory returns nullptr, so board-set function pointers
	// remain in effect. Phase E/F will override the factory for NROM/MMC1/MMC3.
	if (auto cart = fceu11::create_cart_for_mapper(MapperNo, fceu11::g_bus)) {
		fceu11::assign_cart(std::move(cart));
		iNESCart.cart_obj = fceu11::g_cart;
		iNESCart.Power = CartInfo_PowerForward;
		iNESCart.Reset = CartInfo_ResetForward;
		iNESCart.Close = CartInfo_CloseForward;
	}

	// v1.7 Phase C2: sync parsed iNES metadata into the objectized Cart.
	// Cart setters dual-write back to currCartInfo, so CartInfo fields stay
	// populated for the 168 un-migrated board files.
	if (fceu11::g_cart) {
		fceu11::g_cart->set_md5(iNESCart.MD5);
		fceu11::g_cart->set_crc32(iNESCart.CRC32);
		fceu11::g_cart->set_mirror(iNESCart.mirror);
		fceu11::g_cart->set_mirror_as_2bits(iNESCart.mirrorAs2Bits);
		fceu11::g_cart->set_battery(iNESCart.battery != 0);
		fceu11::g_cart->set_ines2(iNESCart.ines2 != 0);
		fceu11::g_cart->set_submapper(iNESCart.submapper);
		fceu11::g_cart->set_wram_size(iNESCart.wram_size);
		fceu11::g_cart->set_battery_wram_size(iNESCart.battery_wram_size);
		fceu11::g_cart->set_vram_size(iNESCart.vram_size);
		fceu11::g_cart->set_battery_vram_size(iNESCart.battery_vram_size);
		fceu11::g_cart->set_mapper_number(MapperNo);
	}

	FCEU_printf("\n");

	// since apparently the iNES format doesn't store this information,
	// guess if the settings should be PAL or NTSC from the ROM name
	// TODO: MD5 check against a list of all known PAL games instead?
	if (iNES2) {
		fceu11::SetVidSystem(((head.TV_system & 3) == 1) ? 1 : 0);
	} else if (OverwriteVidMode) {
		if (strstr(name, "(E)") || strstr(name, "(e)")
			|| strstr(name, "(Europe)") || strstr(name, "(PAL)")
			|| strstr(name, "(F)") || strstr(name, "(f)")
			|| strstr(name, "(G)") || strstr(name, "(g)")
			|| strstr(name, "(I)") || strstr(name, "(i)"))
			fceu11::SetVidSystem(1);
		else
			fceu11::SetVidSystem(0);
	}
	return LOADER_OK;
}

// bbit edited: the whole function below was added
int iNesSave(void) {
	char name[2048];

	FCEU_strlcpy(name, sizeof(name), LoadedRomFName);
	if (strcmp(name + strlen(name) - 4, ".nes") != 0) { //para edit
		safe_strcat(name, sizeof(name), ".nes");
	}

	return iNesSaveAs(name);
}

int iNesSaveAs(const char* name)
{
	//adelikat: TODO: iNesSave() and this have pretty much the same code, outsource the common code to a single function
	//caitsith2: done. iNesSave() now gets filename and calls iNesSaveAs with that filename.
	FILE *fp;

	if ((GameInfo->type != GIT_CART) && (GameInfo->type != GIT_VSUNI)) return 0;
	if (GameInterface != iNESGI) return 0;

	fp = fopen(name, "wb");
	if (!fp)
		return 0;

	if (fwrite(&head, 1, 16, fp) != 16)
	{
		fclose(fp);
		return 0;
	}

	if (head.ROM_type & 4)
	{
		/* Trainer */
		fwrite(trainerpoo, 512, 1, fp);
	}

	fwrite(ROM, 0x4000, ROM_size, fp);

	if (head.VROM_size)
		fwrite(VROM, 0x2000, head.VROM_size, fp);

	fclose(fp);
	return 1;
}

//para edit: added function below
char *iNesShortFName(void) {
	char *ret;

	if (!(ret = strrchr(LoadedRomFName, '\\')))
	{
		if (!(ret = strrchr(LoadedRomFName, '/')))
			return 0;
	}
	return ret + 1;
}

static int iNES_Init(int num) {
	BMAPPINGLocal *tmp = bmap;

	CHRRAMSize = -1;

	if (GameInfo->type == GIT_VSUNI)
		AddExState(FCEUVSUNI_STATEINFO, ~0, 0, 0);

	while (tmp->init) {
		if (num == tmp->number) {
			UNIFchrrama = NULL;	// need here for compatibility with UNIF mapper code
			if (!VROM_size) {
				CHRRAMSize = fceux11_rust_cart_compute_chrram_size(
					num, iNESCart.ines2, iNESCart.vram_size,
					iNESCart.battery_vram_size, false);
				if (!iNESCart.ines2)
				{
					iNESCart.vram_size = CHRRAMSize;
				}
				if (CHRRAMSize > 0)
				{
					int mCHRRAMSize = (CHRRAMSize < 1024) ? 1024 : CHRRAMSize; // VPage has a resolution of 1k banks, ensure minimum allocation to prevent malicious access from NES software
					if ((UNIFchrrama = VROM = (uint8*)FCEU_dmalloc(mCHRRAMSize)) == NULL) return 2;
					FCEU_MemoryRand(VROM, CHRRAMSize);
					SetupCartCHRMapping(0, VROM, CHRRAMSize, 1);
					AddExState(VROM, CHRRAMSize, 0, "CHRR");
				}
				else {
					// mapper 256 (OneBus) has not CHR-RAM _and_ has not CHR-ROM region in iNES file
					// so zero-sized CHR should be supported at least for this mapper
					VROM = NULL;
				}
			}
			if (head.ROM_type & 8)
			{
				if (ExtraNTARAM != NULL)
				{
					AddExState(ExtraNTARAM, 2048, 0, "EXNR");
				}
			}
			tmp->init(&iNESCart);
			return 0;
		}
		tmp++;
	}
	return 1;
}
