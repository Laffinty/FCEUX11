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

#include "types.h"
#include "x6502.h"
#include "fceu.h"
#include "fds.h"
#include "sound.h"
#include "file.h"
#include "utils/md5.h"
#include "utils/memory.h"
#include "utils/safe_string.h"
#include "state.h"
#include "file.h"
#include "cart.h"
#include "ines.h"
#include "netplay.h"
#include "driver.h"
#include "movie.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

//	TODO:  Add code to put a delay in between the time a disk is inserted
//	and the when it can be successfully read/written to.  This should
//	prevent writes to wrong places OR add code to prevent disk ejects
//	when the virtual motor is on (mmm...virtual motor).
extern int disableBatteryLoading;

bool isFDS = false; //flag for determining if a FDS game is loaded, movie.cpp needs this

static DECLFR(FDSRead4030);
static DECLFR(FDSRead4031);
static DECLFR(FDSRead4032);
static DECLFR(FDSRead4033);

static DECLFW(FDSWrite);

static void FDSInit(void);
static void FDSClose(void);
static void FDSFix(int a);

// ── v1.10 Cryptex Task 3: FDS runtime state in Rust ────────────────
static FdsRuntimeState *g_fds_state = nullptr;

static uint8 FDSRegs[6];
static int32 IRQLatch, IRQCount;
static uint8 IRQa;

static uint8 *FDSRAM = NULL;
static FceuMallocPtr FDSRAM_owner;
static uint32 FDSRAMSize;
static uint8 *FDSBIOS = NULL;
static FceuMallocPtr FDSBIOS_owner;
static uint32 FDSBIOSsize;
static uint8 *CHRRAM = NULL;
static FceuMallocPtr CHRRAM_owner;
static uint32 CHRRAMSize;

static uint8 *diskdatao[8] = { 0, 0, 0, 0, 0, 0, 0, 0 };
static uint8 *diskdata[8] = { 0, 0, 0, 0, 0, 0, 0, 0 };

static int TotalSides;
static uint8 DiskWritten = 0;
static uint8 writeskip;
static int32 DiskPtr;
static int32 DiskSeekIRQ;
static uint8 SelectDisk, InDisk;

enum FDS_DiskBlockIDs { DSK_INIT = 0, DSK_VOLUME, DSK_FILECNT, DSK_FILEHDR, DSK_FILEDATA };
static uint8  mapperFDS_control;
static uint16 mapperFDS_filesize;
static uint8  mapperFDS_block;
static uint16 mapperFDS_blockstart;
static uint16 mapperFDS_blocklen;
static uint16 mapperFDS_diskaddr;
static uint8  mapperFDS_diskaccess;
#define fds_disk() (diskdata[InDisk][mapperFDS_blockstart + mapperFDS_diskaddr])
#define mapperFDS_diskinsert (InDisk != 255)


#define DC_INC    1

void FDSGI(GI h) {
	switch (h)
	{
		case GI_CLOSE: FDSClose(); break;
		case GI_POWER: FDSInit(); break;

		// Unhandled Cases
		case GI_RESETM2:
		case GI_RESETSAVE:
			break;
	}
}

static void FDSStateRestore(int version) {
	int x;

	setmirror(((FDSRegs[5] & 8) >> 3) ^ 1);

	if (version >= 9810)
		for (x = 0; x < TotalSides; x++) {
			fceux11_rust_fds_xor_disk_data(diskdata[x], diskdatao[x]);
		}
}

// FDS sound chip (in fds_sound.cpp)
void FDSSound(int c);
void FDSSoundReset(void);
void FDSSoundStateAdd(void);

static void FDSInit(void) {
	memset(FDSRegs, 0, sizeof(FDSRegs));
	writeskip = DiskPtr = DiskSeekIRQ = 0;

	setmirror(1);
	setprg8(0xE000, 0);			// BIOS
	setprg32r(1, 0x6000, 0);	// 32KB RAM
	setchr8(0);					// 8KB CHR RAM

	g_cpu.map_irq_hook_ref() = FDSFix;
	GameStateRestore = FDSStateRestore;

	SetReadHandler(0x4030, 0x4030, FDSRead4030);
	SetReadHandler(0x4031, 0x4031, FDSRead4031);
	SetReadHandler(0x4032, 0x4032, FDSRead4032);
	SetReadHandler(0x4033, 0x4033, FDSRead4033);

	SetWriteHandler(0x4020, 0x4025, FDSWrite);

	SetWriteHandler(0x6000, 0xDFFF, CartBW);
	SetReadHandler(0x6000, 0xFFFF, CartBR);

	IRQCount = IRQLatch = IRQa = 0;

	FDSSoundReset();
	InDisk = 0;
	SelectDisk = 0;

	// v1.10 Task 3: reset Rust runtime state
	if (g_fds_state) fceux11_rust_fds_runtime_reset(g_fds_state);
	mapperFDS_control = 0;
	mapperFDS_filesize = 0;
	mapperFDS_block = 0;
	mapperFDS_blockstart = 0;
	mapperFDS_blocklen = 0;
	mapperFDS_diskaddr = 0;
	mapperFDS_diskaccess = 0;
}

void FCEU_FDSInsert(void)
{
	if (TotalSides == 0)
	{
		FCEU_DispMessage("Not FDS; can't eject disk.", 0);
		return;
	}

	if (fceu11::IsEmulationPaused())
		EmulationPaused |= EMULATIONPAUSED_FA;

	if (FCEUMOV_Mode(MOVIEMODE_RECORD))
		FCEUMOV_AddCommand(FCEUNPCMD_FDSINSERT);

	if (InDisk == 255)
	{
		FCEU_DispMessage("Disk %d Side %s Inserted", 0, SelectDisk >> 1, (SelectDisk & 1) ? "B" : "A");
		InDisk = SelectDisk;
	} else
	{
		FCEU_DispMessage("Disk %d Side %s Ejected", 0, SelectDisk >> 1, (SelectDisk & 1) ? "B" : "A");
		InDisk = 255;
	}
}
/*
void FCEU_FDSEject(void)
{
InDisk=255;
}
*/
void FCEU_FDSSelect(void)
{
	if (TotalSides == 0)
	{
		FCEU_DispMessage("Not FDS; can't select disk.", 0);
		return;
	}
	if (InDisk != 255)
	{
		FCEU_DispMessage("Eject disk before selecting.", 0);
		return;
	}

	if (fceu11::IsEmulationPaused())
		EmulationPaused |= EMULATIONPAUSED_FA;

	if (FCEUMOV_Mode(MOVIEMODE_RECORD))
		FCEUMOV_AddCommand(FCEUNPCMD_FDSSELECT);

	SelectDisk = fceux11_rust_fds_compute_select_disk_next(SelectDisk, (uint8)TotalSides);
	FCEU_DispMessage("Disk %d Side %c Selected", 0, SelectDisk >> 1, (SelectDisk & 1) ? 'B' : 'A');
}

#define IRQ_Repeat  0x01
#define IRQ_Enabled 0x02

static void FDSFix(int a) {
	FceuFdsIrqState st;
	st.irq_count = IRQCount;
	st.irq_latch = IRQLatch;
	st.irq_a = IRQa;
	st.disk_seek_irq = DiskSeekIRQ;
	st.fds_regs_5 = FDSRegs[5];

	FceuFdsIrqTickResult r = fceux11_rust_fds_irq_tick(&st, a);

	IRQCount = st.irq_count;
	IRQLatch = st.irq_latch;
	IRQa = st.irq_a;
	DiskSeekIRQ = st.disk_seek_irq;

	/* Puff Puff Golf notes:
	Game freezes while music playing ingame after inserting Disk Side B.
	IRQ is usually fired at scanline 169 and 183 for music to work.

	At some point after inserting disk B, an IRQ is fired at scanline 174 which
	will just freeze game while music plays.

	If you ignore triggering IRQ altogether, game plays but no music
	*/
	if (r.timer_fire) X6502_IRQBegin(FCEU_IQEXT);
	if (r.seek_fire)  X6502_IRQBegin(FCEU_IQEXT2);
}

static DECLFR(FDSRead4030) {
	uint8 ret = fceux11_rust_fds_read_4030_value(
		(X.IRQlow & FCEU_IQEXT) != 0,
		(X.IRQlow & FCEU_IQEXT2) != 0);

	if (!fceuindbg) {
		X6502_IRQEnd(FCEU_IQEXT);
		X6502_IRQEnd(FCEU_IQEXT2);
	}
	return ret;
}

static DECLFR(FDSRead4031) {
	// v1.10 Cryptex: Use Rust FFI for disk read
	FceuFdsDiskIoState state;
	state.block = mapperFDS_block;
	state.block_start = mapperFDS_blockstart;
	state.block_len = mapperFDS_blocklen;
	state.disk_addr = mapperFDS_diskaddr;
	state.file_size = mapperFDS_filesize;
	state.control = mapperFDS_control;
	state.disk_inserted = mapperFDS_diskinsert ? 1 : 0;
	state.disk_access = mapperFDS_diskaccess;

	FceuFdsDiskReadResult result;
	if (fceux11_rust_fds_disk_read(&state, diskdata[InDisk], &result)) {
		mapperFDS_diskaddr = result.new_disk_addr;
		mapperFDS_filesize = result.new_file_size;
		mapperFDS_diskaccess = 1;
		if (result.trigger_seek_irq) {
			DiskSeekIRQ = 150;
			X6502_IRQEnd(FCEU_IQEXT2);
		}
		return result.value;
	}
	return 0xff;
}

static DECLFR(FDSRead4032) {
	return fceux11_rust_fds_read_4032_value(InDisk, FDSRegs[5], g_cpu.native_layout().DB);
}

static DECLFR(FDSRead4033) {
	return 0x80; // battery
}

static DECLFW(FDSWrite) {
	switch (A) {
	case 0x4020: IRQLatch &= 0xFF00; IRQLatch |= V; break;
	case 0x4021:
		IRQLatch &= 0xFF; IRQLatch |= V << 8;
		break;
	case 0x4022:
		if (FDSRegs[3] & 1) {
			IRQa = V & 0x03;
			if (IRQa & IRQ_Enabled) IRQCount = IRQLatch;
			else X6502_IRQEnd(FCEU_IQEXT);
		}
		break;
	case 0x4023:
		if (!(V & 0x01)) {
			IRQa &= ~IRQ_Enabled;
			X6502_IRQEnd(FCEU_IQEXT);
			X6502_IRQEnd(FCEU_IQEXT2);
		}
		break;
	case 0x4024:
		// v1.10 Cryptex: Use Rust FFI for disk write
		if (mapperFDS_diskinsert && ~mapperFDS_control & 0x04) {
			if (mapperFDS_diskaccess == 0) {
				mapperFDS_diskaccess = 1;
				break;
			}
			FceuFdsDiskIoState state;
			state.block = mapperFDS_block;
			state.block_start = mapperFDS_blockstart;
			state.block_len = mapperFDS_blocklen;
			state.disk_addr = mapperFDS_diskaddr;
			state.file_size = mapperFDS_filesize;
			state.control = mapperFDS_control;
			state.disk_inserted = mapperFDS_diskinsert ? 1 : 0;
			state.disk_access = mapperFDS_diskaccess;
			FceuFdsDiskWriteResult result;
			if (fceux11_rust_fds_disk_write(&state, diskdata[InDisk], V, &result)) {
				mapperFDS_diskaddr = result.new_disk_addr;
				mapperFDS_filesize = result.new_file_size;
				if (result.disk_written) DiskWritten = 1;
			}
		}
		break;
	case 0x4025:
		// v1.10 Task 3: Rust handles the disk-block FSM
		X6502_IRQEnd(FCEU_IQEXT2);
		{
			FceuFdsWrite4025Result wr = fceux11_rust_fds_compute_write_4025(
				mapperFDS_block, mapperFDS_filesize, mapperFDS_control,
				V, mapperFDS_diskinsert ? 1 : 0);
			if (mapperFDS_diskinsert) {
				if (wr.motor_on_edge) {
					mapperFDS_diskaccess = 0; DiskSeekIRQ = 150;
					mapperFDS_blockstart += mapperFDS_diskaddr;
					mapperFDS_diskaddr = 0;
					mapperFDS_block = wr.new_block;
					mapperFDS_blocklen = wr.new_blocklen;
				}
				if (wr.transfer_reset) {
					mapperFDS_block = DSK_INIT; mapperFDS_blockstart = 0;
					mapperFDS_blocklen = 0; mapperFDS_diskaddr = 0;
					DiskSeekIRQ = 150;
				}
				if (wr.motor_on) DiskSeekIRQ = 150;
			}
		}
		mapperFDS_control = V;
		setmirror(((V >> 3) & 1) ^ 1);
		break;
	}
	FDSRegs[A & 7] = V;
}

static void FreeFDSMemory(void) {
	for (int x = 0; x < TotalSides; x++) { free(diskdata[x]); diskdata[x] = 0; }
}

static int SubLoad(FCEUFILE *fp) {
	struct md5_context md5;
	uint8 header[16];

	FCEU_fseek(fp, 0, SEEK_SET);
	FCEU_fread(header, 16, 1, fp);

	FceuFdsHeaderInfo hi = fceux11_rust_fds_validate_header(header, 16);
	if (hi.kind == 0) return 1;
	if (hi.kind == 2) FCEU_fseek(fp, 0, SEEK_SET); // raw image: rewind

	long file_size = FCEU_fgetsize(fp);
	TotalSides = (int)fceux11_rust_fds_compute_total_sides(
		(uintptr_t)file_size, hi.advertised_sides, hi.kind == 1 ? 1 : 0);

	md5_starts(&md5);
	for (int x = 0; x < TotalSides; x++) {
		if (!(diskdata[x] = (uint8*)FCEU_malloc(65500))) return 2;
		FCEU_fread(diskdata[x], 1, 65500, fp);
		md5_update(&md5, diskdata[x], 65500);
	}
	md5_finish(&md5, GameInfo->MD5.data);
	return 0;
}

static void PreSave(void) {
	int x;
	for (x = 0; x < TotalSides; x++) {
		fceux11_rust_fds_xor_disk_data(diskdata[x], diskdatao[x]);
	}
}

static void PostSave(void) {
	int x;
	for (x = 0; x < TotalSides; x++) {
		fceux11_rust_fds_xor_disk_data(diskdata[x], diskdatao[x]);
	}
}

int FDSLoad(const char *name, FCEUFILE *fp) {
	int x;

	FreeFDSMemory();
	int load_result = SubLoad(fp);
	if (load_result == 1) { FreeFDSMemory(); return LOADER_INVALID_FORMAT; }
	if (load_result == 2) { FreeFDSMemory(); FCEU_PrintError("Unable to allocate memory."); return LOADER_HANDLED_ERROR; }

	// Load FDS BIOS
	char *fn = strdup(FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0).c_str());
	FILE *zp = FCEUD_UTF8fopen(fn, "rb");
	if (!zp) {
		FCEU_PrintError("FDS BIOS ROM image missing: %s", FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0).c_str());
		free(fn); FreeFDSMemory(); return LOADER_HANDLED_ERROR;
	}
	free(fn);
	fseek(zp, 0L, SEEK_END);
	if (ftell(zp) != 8192) {
		fclose(zp); FreeFDSMemory();
		FCEU_PrintError("FDS BIOS ROM image incompatible: %s", FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0).c_str());
		return LOADER_HANDLED_ERROR;
	}
	fseek(zp, 0L, SEEK_SET);
	ResetCartMapping();

	free(FDSBIOS); FDSBIOS = NULL;
	free(FDSRAM); FDSRAM = NULL;
	free(CHRRAM); CHRRAM = NULL;

	FDSBIOSsize = 8192;
	FDSBIOS_owner = FCEU_gmalloc_unique(FDSBIOSsize);
	FDSBIOS = FDSBIOS_owner.get();
	SetupCartPRGMapping(0, FDSBIOS, FDSBIOSsize, 0);
	if (fread(FDSBIOS, 1, FDSBIOSsize, zp) != FDSBIOSsize) {
		free(FDSBIOS); FDSBIOS = NULL; fclose(zp); FreeFDSMemory();
		FCEU_PrintError("Error reading FDS BIOS ROM image."); return LOADER_HANDLED_ERROR;
	}
	fclose(zp);

	// Battery-backed disk save
	if (!disableBatteryLoading) {
		for (x = 0; x < TotalSides; x++) {
			diskdatao[x] = (uint8*)FCEU_malloc(65500);
			memcpy(diskdatao[x], diskdata[x], 65500);
		}
		char *fn2 = strdup(FCEU_MakeFName(FCEUMKF_FDS, 0, 0).c_str());
		FCEUFILE *tp = FCEU_fopen(fn2, 0, "rb", 0);
		if (tp) {
			FCEU_printf("Disk was written. Auxiliary FDS file open \"%s\".\n", fn2);
			FreeFDSMemory();
			if (SubLoad(tp)) {
				free(FDSBIOS); FDSBIOS = NULL; free(fn2); FreeFDSMemory();
				FCEU_PrintError("Error reading auxiliary FDS file."); return LOADER_HANDLED_ERROR;
			}
			FCEU_fclose(tp); DiskWritten = 1;
		}
		free(fn2);
	}

	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name);

	if (!g_fds_state) g_fds_state = fceux11_rust_fds_runtime_create();

	GameInfo->type = GIT_FDS;
	GameInterface = FDSGI;
	isFDS = true;
	SelectDisk = 0; InDisk = 255;

	ResetExState(PreSave, PostSave);
	FDSSoundStateAdd();

	for (x = 0; x < TotalSides; x++) {
		char temp[5];
		snprintf(temp, sizeof(temp), "DDT%d", x);
		AddExState(diskdata[x], 65500, 0, temp);
	}

	AddExState(FDSRegs, sizeof(FDSRegs), 0, "FREG");
	AddExState(&IRQCount, 4, 1, "IRQC"); AddExState(&IRQLatch, 4, 1, "IQL1");
	AddExState(&IRQa, 1, 0, "IRQA"); AddExState(&writeskip, 1, 0, "WSKI");
	AddExState(&DiskPtr, 4, 1, "DPTR"); AddExState(&DiskSeekIRQ, 4, 1, "DSIR");
	AddExState(&SelectDisk, 1, 0, "SELD"); AddExState(&InDisk, 1, 0, "INDI");
	AddExState(&DiskWritten, 1, 0, "DSKW");
	AddExState(&mapperFDS_control, 1, 0, "CTRG"); AddExState(&mapperFDS_filesize, 2, 1, "FLSZ");
	AddExState(&mapperFDS_block, 1, 0, "BLCK"); AddExState(&mapperFDS_blockstart, 2, 1, "BLKS");
	AddExState(&mapperFDS_blocklen, 2, 1, "BLKL"); AddExState(&mapperFDS_diskaddr, 2, 1, "DADR");
	AddExState(&mapperFDS_diskaccess, 1, 0, "DACC");

	CHRRAMSize = 8192;
	CHRRAM_owner = FCEU_gmalloc_unique(CHRRAMSize);
	CHRRAM = CHRRAM_owner.get();
	SetupCartCHRMapping(0, CHRRAM, CHRRAMSize, 1);
	AddExState(CHRRAM, CHRRAMSize, 0, "CHRR");

	FDSRAMSize = 32768;
	FDSRAM_owner = FCEU_gmalloc_unique(FDSRAMSize);
	FDSRAM = FDSRAM_owner.get();
	SetupCartPRGMapping(1, FDSRAM, FDSRAMSize, 1);
	AddExState(FDSRAM, FDSRAMSize, 0, "FDSR");

	SetupCartMirroring(0, 0, 0);
	FCEU_printf(" Sides: %d\n\n", TotalSides);
	fceu11::SetVidSystem(0);
	return LOADER_OK;
}

void FDSClose(void) {
	isFDS = false;
	if (g_fds_state) { fceux11_rust_fds_runtime_destroy(g_fds_state); g_fds_state = nullptr; }
	if (!DiskWritten) return;

	const std::string &fn = FCEU_MakeFName(FCEUMKF_FDS, 0, 0);
	FILE *fp = FCEUD_UTF8fopen(fn.c_str(), "wb");
	if (!fp) return;

	for (int x = 0; x < TotalSides; x++)
		if (fwrite(diskdata[x], 1, 65500, fp) != 65500) {
			FCEU_PrintError("Error saving FDS image!"); fclose(fp); return;
		}

	for (int x = 0; x < TotalSides; x++) { free(diskdatao[x]); diskdatao[x] = 0; }

	FreeFDSMemory();
	free(FDSBIOS); FDSBIOS = NULL;
	free(FDSRAM); FDSRAM = NULL;
	free(CHRRAM); CHRRAM = NULL;
	fclose(fp);
}
