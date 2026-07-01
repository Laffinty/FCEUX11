// iNES load helper — extracted from ines.cpp for v1.10 Cryptex Phase A.3.
// Contains the bulk of iNESLoad logic, called from the thin wrapper in ines.cpp.

#include "types.h"
#include "utils/safe_string.h"
#include "x6502.h"
#include "fceu.h"
#include "cart.h"
#include "apu.h"
#include "ppu.h"
#include "ines.h"
#include "state.h"
#include "file.h"
#include "utils/general.h"
#include "utils/memory.h"
#include "vsuni.h"
#include "driver.h"
#include "input.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

// Forward declarations for globals defined in ines.cpp
extern uint8 *trainerpoo;
extern uint8 *ROM;
extern uint8 *VROM;
extern uint8 *ExtraNTARAM;
extern iNES_HEADER head;
extern uint8 Mirroring;
extern uint8 MirroringAs2bits;
extern uint32 ROM_size;
extern uint32 VROM_size;
extern int iNES2;
extern uint32 iNESGameCRC32;

// Forward declarations for functions in other files
extern void SetInput(void);
extern void SetInputNes20(uint8 expansion);
extern void CheckBad(uint64 md5partial);
extern int iNES_Init(int num);

// Helper: set up VS UniSystem fields from cart result
static void ines_setup_vsuni(const FceuInesCartResult& cart) {
	static const EGIPPU ppu_map[] = {
		GIPPU_RC2C03B, (EGIPPU)0, GIPPU_RP2C04_0001, GIPPU_RP2C04_0002,
		GIPPU_RP2C04_0003, GIPPU_RP2C04_0004, GIPPU_RC2C03B, (EGIPPU)0,
		GIPPU_RC2C05_01, GIPPU_RC2C05_02, GIPPU_RC2C05_03, GIPPU_RC2C05_04,
	};
	if (cart.vs_ppu >= 0 && cart.vs_ppu < 12)
		GameInfo->vs_ppu = ppu_map[cart.vs_ppu];

	static const EGIVS vs_type_map[] = { EGIVS_NORMAL, EGIVS_RBI, EGIVS_TKO, EGIVS_XEVIOUS };
	if (cart.vs_type >= 0 && cart.vs_type < 4)
		GameInfo->vs_type = vs_type_map[cart.vs_type];
}

// Helper: log ROM info
static void ines_log_info(const FceuInesCartResult& cart, int round, uint32 not_round_size) {
	const char* mappername = "Not Listed";
	{ const char* rn = fceux11_rust_ines_mapper_name(cart.mapper_no); if (rn) mappername = rn; }

	FCEU_printf(" PRG ROM: %d x 16KiB = %d KiB\n", round ? ROM_size : not_round_size, (round ? ROM_size : not_round_size) * 16);
	FCEU_printf(" CHR ROM: %d x  8KiB = %d KiB\n", VROM_size, VROM_size * 8);
	FCEU_printf(" ROM CRC32: 0x%08x\n", iNESGameCRC32);
	FCEU_printf(" Mapper #: %d\n", cart.mapper_no);
	FCEU_printf(" Mapper name: %s\n", mappername);
	FCEU_printf(" Mirroring: %s\n", Mirroring == 2 ? "None (Four-screen)" : Mirroring ? "Vertical" : "Horizontal");
	FCEU_printf(" Battery-backed: %s\n", cart.battery ? "Yes" : "No");
	FCEU_printf(" Trained: %s\n", cart.trainer_size > 0 ? "Yes" : "No");
}

// Helper: apply corrections and check for bad ROM
static int ines_apply_corrections(const FceuInesCartResult& cart, uint64 partialmd5) {
	FceuInesHInfoResult hinfo;
	FceuMasterRomInfoResult masterInfo;
	int32 tofix = 0;

	fceux11_rust_ines_check_hinfo(iNESGameCRC32, partialmd5, &hinfo);

	FceuInesParseResult parseResult;
	parseResult.is_nes2 = cart.is_nes2;
	parseResult.mapper_no = cart.mapper_no;
	parseResult.submapper = cart.submapper;
	parseResult.mirroring = cart.mirror;
	parseResult.mirroring_as_2bits = cart.mirror_as_2bits;
	parseResult.battery = cart.battery;
	parseResult.trainer = cart.trainer_size > 0;
	parseResult.rom_size_16kb = cart.rom_size_16kb;
	parseResult.vrom_size_8kb = cart.vrom_size_8kb;
	parseResult.rom_size_raw = cart.rom_size_raw;
	parseResult.wram_size = cart.wram_size;
	parseResult.battery_wram_size = cart.battery_wram_size;
	parseResult.vram_size = cart.vram_size;
	parseResult.battery_vram_size = cart.battery_vram_size;
	parseResult.vs_system = cart.vs_system;
	parseResult.vs_ppu = cart.vs_ppu;
	parseResult.vs_type = cart.vs_type;
	parseResult.tv_system = cart.tv_system;

	tofix = fceux11_rust_ines_apply_corrections(&parseResult, &hinfo, partialmd5, VROM_size > 0);
	if (tofix & 1) { /* MapperNo updated via parseResult */ }
	if (tofix & 2) Mirroring = parseResult.mirroring;
	if (tofix & 4) head.ROM_type |= 2;
	if (tofix & 8 && VROM_size) { VROM_size = 0; free(VROM); VROM = NULL; }

	fceux11_rust_ines_lookup_master_info(partialmd5, &masterInfo);
	FCEU_VSUniCheck(partialmd5, const_cast<int*>(&reinterpret_cast<const int&>(cart.mapper_no)), &Mirroring);
	CheckBad(partialmd5);

	return tofix;
}

// Main iNES load logic — called from thin wrapper in ines.cpp
int iNESLoadCore(const char *name, FCEUFILE *fp, CartInfo& iNESCart, FceuMallocPtr& trainerpoo_owner, FceuMallocPtr& ExtraNTARAM_owner) {
	EMUFILE_MEMORY* ms = fp->EnsureMemorystream();
	const uint8_t* file_data = ms->buf();
	size_t file_size = ms->size();

	FceuInesCartResult cart;
	if (!fceux11_rust_ines_load(file_data, file_size, &cart))
		return LOADER_INVALID_FORMAT;

	iNESCart.clear();

	// Apply parsed results to C++ globals
	iNES2 = cart.is_nes2;
	Mirroring = cart.mirror;
	MirroringAs2bits = cart.mirror_as_2bits;
	ROM_size = cart.rom_size_16kb;
	VROM_size = cart.vrom_size_8kb;
	uint32 not_round_size = cart.rom_size_raw;
	int MapperNo = cart.mapper_no;

	// NES 2.0 fields
	if (iNES2) {
		iNESCart.ines2 = true;
		iNESCart.wram_size = cart.wram_size;
		iNESCart.battery_wram_size = cart.battery_wram_size;
		iNESCart.vram_size = cart.vram_size;
		iNESCart.battery_vram_size = cart.battery_vram_size;
		iNESCart.submapper = cart.submapper;
	}

	int round = !fceux11_rust_ines_not_power2(MapperNo);

	// Copy PRG-ROM to emulator-owned memory
	ROM = (uint8*)FCEU_malloc(cart.prg_size);
	memset(ROM, 0xFF, cart.prg_size);
	memcpy(ROM, cart.prg_data, cart.prg_size);

	// Copy CHR-ROM to emulator-owned memory
	if (cart.chr_size > 0) {
		VROM = (uint8*)FCEU_malloc(cart.chr_size);
		memset(VROM, 0xFF, cart.chr_size);
		memcpy(VROM, cart.chr_data, cart.chr_size);
	}

	// VS UniSystem detection
	if (cart.vs_system < 0) {
		FCEU_PrintError("Game type is not supported at all.");
		return LOADER_HANDLED_ERROR;
	}
	GameInfo->type = (cart.vs_system == 1) ? GIT_VSUNI : GIT_CART;

	if (GameInfo->type == GIT_VSUNI) {
		if (cart.vs_ppu < 0) {
			FCEU_PrintError("Vs. System PPU type is not supported at all.");
			return LOADER_HANDLED_ERROR;
		}
		ines_setup_vsuni(cart);
	}

	// Copy trainer data
	if (cart.trainer_size > 0) {
		trainerpoo_owner = FCEU_gmalloc_unique(512);
		trainerpoo = trainerpoo_owner.get();
		memcpy(trainerpoo, cart.trainer_data, 512);
	}

	ResetCartMapping();
	ResetExState(0, 0);
	SetupCartPRGMapping(0, ROM, cart.prg_size, 0);

	// Copy hash results
	memcpy(iNESCart.MD5, cart.md5, 16);
	memcpy(&GameInfo->MD5, cart.md5, 16);
	iNESGameCRC32 = cart.crc32;
	iNESCart.CRC32 = cart.crc32;
	uint64 partialmd5 = cart.partial_md5;

	head.ROM_type = cart.battery ? 2 : 0;
	head.expansion = 0;

	ines_log_info(cart, round, not_round_size);

	// Apply corrections
	SetInput();
	if (iNES2) SetInputNes20(0);
	ines_apply_corrections(cart, partialmd5);

	if (VROM_size)
		SetupCartCHRMapping(0, VROM, VROM_size * 0x2000, 0);

	if (Mirroring == 2) {
		ExtraNTARAM_owner = FCEU_gmalloc_unique(2048);
		ExtraNTARAM = ExtraNTARAM_owner.get();
		SetupCartMirroring(4, 1, ExtraNTARAM);
	} else if (Mirroring >= 0x10)
		SetupCartMirroring(2 + (Mirroring & 1), 1, 0);
	else
		SetupCartMirroring(Mirroring & 1, (Mirroring & 4) >> 2, 0);

	iNESCart.battery = cart.battery ? 1 : 0;
	iNESCart.mirror = Mirroring;
	iNESCart.mirrorAs2Bits = MirroringAs2bits;

	int result = iNES_Init(MapperNo);
	if (result != 0) {
		if (result == 1) FCEU_PrintError("iNES mapper #%d is not supported at all.", MapperNo);
		if (result == 2) FCEU_PrintError("Unable to allocate CHR-RAM.");
		return LOADER_HANDLED_ERROR;
	}

	return LOADER_OK;
}
