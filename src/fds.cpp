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
#include "cpu.h"
#include "fceu.h"
#include "fds.h"
#include "sound.h"
#include "file.h"
#include "utils/md5.h"
#include "utils/memory.h"
#include "utils/safe_string.h"
#include "state.h"
#include "cart.h"
#include "ines.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "movie.h"
#include "netplay.h"
#include "rust/fceux11_rust.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
extern int disableBatteryLoading;
bool isFDS = false;

static DECLFR(FDSRead4030); static DECLFR(FDSRead4031);
static DECLFR(FDSRead4032); static DECLFR(FDSRead4033);
static DECLFW(FDSWrite);
static void FDSInit(void), FDSClose(void), FDSFix(int a);

// ── v1.10 Cryptex: FDS runtime state in Rust ─────────────────────────
static FdsRuntimeState *g_fds_state = nullptr;
static uint8 *FDSRAM,*FDSBIOS,*CHRRAM;
static FceuMallocPtr FDSRAM_owner, FDSBIOS_owner, CHRRAM_owner;
static uint32 FDSRAMSize, FDSBIOSsize, CHRRAMSize;
static uint8 *diskdatao[8] = {}, *diskdata[8] = {};
static int TotalSides, DiskPtr, writeskip;

// C++ globals aliased into Rust FdsRuntimeState (macros �?g_fds_state->*)
#define mapperFDS_control    (g_fds_state->control)
#define mapperFDS_filesize   (g_fds_state->filesize)
#define mapperFDS_block      (g_fds_state->block)
#define mapperFDS_blockstart (g_fds_state->blockstart)
#define mapperFDS_blocklen   (g_fds_state->blocklen)
#define mapperFDS_diskaddr   (g_fds_state->diskaddr)
#define mapperFDS_diskaccess (g_fds_state->diskaccess)
#define IRQCount             (g_fds_state->irq_count)
#define IRQLatch             (g_fds_state->irq_latch)
#define IRQa                 (g_fds_state->irq_a)
#define DiskSeekIRQ          (g_fds_state->disk_seek_irq)
#define SelectDisk           (g_fds_state->select_disk)
#define InDisk               (g_fds_state->in_disk)
#define DiskWritten          (g_fds_state->disk_written)
#define FDSRegs              (g_fds_state->fds_regs)
#define mapperFDS_diskinsert (InDisk != 255)

enum { DSK_INIT = 0, DSK_VOLUME, DSK_FILECNT, DSK_FILEHDR, DSK_FILEDATA };

void FDSGI(GI h) {
	switch (h) {
	case GI_CLOSE: FDSClose(); break;
	case GI_POWER: FDSInit(); break;
	default: break;
	}
}
static void FDSStateRestore(int version) {
	setmirror(((FDSRegs[5] & 8) >> 3) ^ 1);
	if (version >= 9810) for (int x = 0; x < TotalSides; x++)
		fceux11_rust_fds_xor_disk_data(diskdata[x], diskdatao[x]);
}
void FDSSound(int c); void FDSSoundReset(void); void FDSSoundStateAdd(void);

// ── Initialisation ───────────────────────────────────────────────────
static void FDSInit(void) {
	memset(FDSRegs, 0, sizeof(FDSRegs));
	writeskip = DiskPtr = DiskSeekIRQ = 0;
	setmirror(1); setprg8(0xE000, 0); setprg32r(1, 0x6000, 0); setchr8(0);
	g_cpu.set_map_irq_hook(FDSFix); GameStateRestore = FDSStateRestore;
	SetReadHandler(0x4030, 0x4033, FDSRead4030);
	SetReadHandler(0x4031, 0x4031, FDSRead4031);
	SetReadHandler(0x4032, 0x4032, FDSRead4032);
	SetReadHandler(0x4033, 0x4033, FDSRead4033);
	SetWriteHandler(0x4020, 0x4025, FDSWrite);
	SetWriteHandler(0x6000, 0xDFFF, CartBW);
	SetReadHandler(0x6000, 0xFFFF, CartBR);
	FDSSoundReset();
	if (g_fds_state) fceux11_rust_fds_runtime_reset(g_fds_state);
	InDisk = 0; SelectDisk = 0;
}

void FCEU_FDSInsert(void) {
	if (TotalSides == 0) { FCEU_DispMessage("Not FDS; can't eject disk.", 0); return; }
	if (fceu11::IsEmulationPaused()) EmulationPaused |= EMULATIONPAUSED_FA;
	if (FCEUMOV_Mode(MOVIEMODE_RECORD)) FCEUMOV_AddCommand(FCEUNPCMD_FDSINSERT);
	InDisk = (InDisk == 255) ? SelectDisk : 255;
	FCEU_DispMessage("Disk %d Side %s %s", 0,
		SelectDisk >> 1, (SelectDisk & 1) ? "B" : "A",
		InDisk == 255 ? "Ejected" : "Inserted");
}

void FCEU_FDSSelect(void) {
	if (TotalSides == 0) { FCEU_DispMessage("Not FDS; can't select disk.", 0); return; }
	if (InDisk != 255) { FCEU_DispMessage("Eject disk before selecting.", 0); return; }
	if (fceu11::IsEmulationPaused()) EmulationPaused |= EMULATIONPAUSED_FA;
	if (FCEUMOV_Mode(MOVIEMODE_RECORD)) FCEUMOV_AddCommand(FCEUNPCMD_FDSSELECT);
	SelectDisk = fceux11_rust_fds_compute_select_disk_next(SelectDisk, (uint8)TotalSides);
	FCEU_DispMessage("Disk %d Side %c Selected", 0, SelectDisk >> 1, (SelectDisk & 1) ? 'B' : 'A');
}

// ── IRQ / Register reads / Write dispatch ────────────────────────────
// v1.13 Purify H: #define → constexpr
inline constexpr uint8_t IRQ_Repeat  = 0x01;
inline constexpr uint8_t IRQ_Enabled = 0x02;

static void FDSFix(int a) {
	FceuFdsIrqState st = { IRQCount, IRQLatch, IRQa, DiskSeekIRQ, FDSRegs[5] };
	FceuFdsIrqTickResult r = fceux11_rust_fds_irq_tick(&st, a);
	IRQCount = st.irq_count; IRQLatch = st.irq_latch; IRQa = st.irq_a;
	DiskSeekIRQ = st.disk_seek_irq;
	if (r.timer_fire) X6502_IRQBegin(FCEU_IQEXT);
	if (r.seek_fire)  X6502_IRQBegin(FCEU_IQEXT2);
}

static DECLFR(FDSRead4030) {
	uint8 ret = fceux11_rust_fds_read_4030_value(
		(X.IRQlow & FCEU_IQEXT) != 0, (X.IRQlow & FCEU_IQEXT2) != 0);
	if (!fceuindbg) { X6502_IRQEnd(FCEU_IQEXT); X6502_IRQEnd(FCEU_IQEXT2); }
	return ret;
}

static DECLFR(FDSRead4031) {
	FceuFdsDiskIoState s = { mapperFDS_block, mapperFDS_blockstart, mapperFDS_blocklen,
		mapperFDS_diskaddr, mapperFDS_filesize, mapperFDS_control,
		mapperFDS_diskinsert ? (uint8)1 : (uint8)0, mapperFDS_diskaccess };
	FceuFdsDiskReadResult r;
	if (fceux11_rust_fds_disk_read(&s, diskdata[InDisk], &r)) {
		mapperFDS_diskaddr = r.new_disk_addr; mapperFDS_filesize = r.new_file_size;
		mapperFDS_diskaccess = 1;
		if (r.trigger_seek_irq) { DiskSeekIRQ = 150; X6502_IRQEnd(FCEU_IQEXT2); }
		return r.value;
	}
	return 0xff;
}

static DECLFR(FDSRead4032) {
	return fceux11_rust_fds_read_4032_value(InDisk, FDSRegs[5], g_cpu.native_layout().DB);
}
static DECLFR(FDSRead4033) { return 0x80; }

static DECLFW(FDSWrite) {
	if (A == 0x4024 && mapperFDS_diskinsert && ~mapperFDS_control & 0x04) {
		if (mapperFDS_diskaccess == 0) { mapperFDS_diskaccess = 1; goto done; }
		FceuFdsDiskIoState s = { mapperFDS_block, mapperFDS_blockstart, mapperFDS_blocklen,
			mapperFDS_diskaddr, mapperFDS_filesize, mapperFDS_control, 1, mapperFDS_diskaccess };
		FceuFdsDiskWriteResult r;
		if (fceux11_rust_fds_disk_write(&s, diskdata[InDisk], V, &r)) {
			mapperFDS_diskaddr = r.new_disk_addr; mapperFDS_filesize = r.new_file_size;
			if (r.disk_written) DiskWritten = 1;
		}
		goto done;
	}
	if (A == 0x4025) {
		FceuFdsWrite4025Action act;
		fceux11_rust_fds_handle_write_4025(g_fds_state, V, &act);
		if (act.irq_end_ext2) X6502_IRQEnd(FCEU_IQEXT2);
		if (act.mirror_changed) setmirror(act.mirror_mode);
		goto done;
	}
	{
		FceuFdsWrite4025Action act;
		fceux11_rust_fds_handle_write_4020_4024(g_fds_state, A, V, &act);
		if (act.irq_end_ext)  X6502_IRQEnd(FCEU_IQEXT);
		if (act.irq_end_ext2) X6502_IRQEnd(FCEU_IQEXT2);
	}
done:
	FDSRegs[A & 7] = V;
}

// ── Savestate / disk helpers ─────────────────────────────────────────
static void PreSave(void) {
	for (int x = 0; x < TotalSides; x++)
		fceux11_rust_fds_xor_disk_data(diskdata[x], diskdatao[x]);
}
static void PostSave(void) { PreSave(); }
static void FreeFDSMemory(void) {
	// v1.13 Purify F2a: diskdata[] was allocated with FCEU_malloc(); use matching FCEU_free()
	for (int x = 0; x < TotalSides; x++) { FCEU_free(diskdata[x]); diskdata[x] = 0; }
}

static int SubLoad(FCEUFILE *fp) {
	struct md5_context md5; uint8 header[16];
	FCEU_fseek(fp, 0, SEEK_SET);
	FCEU_fread(header, 16, 1, fp);
	FceuFdsHeaderInfo hi = fceux11_rust_fds_validate_header(header, 16);
	if (hi.kind == 0) return 1;
	if (hi.kind == 2) FCEU_fseek(fp, 0, SEEK_SET);
	TotalSides = (int)fceux11_rust_fds_compute_total_sides(
		(uintptr_t)FCEU_fgetsize(fp), hi.advertised_sides, hi.kind == 1 ? 1 : 0);
	md5_starts(&md5);
	for (int x = 0; x < TotalSides; x++) {
		if (!(diskdata[x] = (uint8*)FCEU_malloc(65500))) return 2;
		FCEU_fread(diskdata[x], 1, 65500, fp);
		md5_update(&md5, diskdata[x], 65500);
	}
	md5_finish(&md5, GameInfo->MD5.data);
	return 0;
}

int FDSLoad(const char *name, FCEUFILE *fp) {
	FreeFDSMemory();
	int r = SubLoad(fp);
	if (r == 1) { FreeFDSMemory(); return LOADER_INVALID_FORMAT; }
	if (r == 2) { FreeFDSMemory(); FCEU_PrintError("Unable to allocate memory."); return LOADER_HANDLED_ERROR; }

	// BIOS
	std::string fn = FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0);
	FILE *zp = FCEUD_UTF8fopen(fn.c_str(), "rb");
	if (!zp) { FCEU_PrintError("FDS BIOS ROM image missing: %s", FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0).c_str());
		FreeFDSMemory(); return LOADER_HANDLED_ERROR; }
	fseek(zp, 0L, SEEK_END);
	if (ftell(zp) != 8192) { fclose(zp); FreeFDSMemory();
		FCEU_PrintError("FDS BIOS ROM image incompatible: %s", FCEU_MakeFName(FCEUMKF_FDSROM, 0, 0).c_str());
		return LOADER_HANDLED_ERROR; }
	fseek(zp, 0L, SEEK_SET); ResetCartMapping();
	// v1.13 Purify F2a: use FceuMallocPtr owners; FDSBIOS/RAM/CHRRAM RAII via .reset()
	FDSBIOS_owner.reset(); FDSRAM_owner.reset(); CHRRAM_owner.reset();
	FDSBIOS = FDSRAM = CHRRAM = NULL;
	FDSBIOSsize = 8192;
	FDSBIOS_owner = FCEU_gmalloc_unique(FDSBIOSsize); FDSBIOS = FDSBIOS_owner.get();
	SetupCartPRGMapping(0, FDSBIOS, FDSBIOSsize, 0);
	if (fread(FDSBIOS, 1, FDSBIOSsize, zp) != FDSBIOSsize) {
		FDSBIOS_owner.reset(); FDSBIOS = NULL; fclose(zp); FreeFDSMemory();
		FCEU_PrintError("Error reading FDS BIOS ROM image."); return LOADER_HANDLED_ERROR; }
	fclose(zp);

	// Battery-backed save
	if (!disableBatteryLoading) {
		for (int x = 0; x < TotalSides; x++) {
			diskdatao[x] = (uint8*)FCEU_malloc(65500);
			memcpy(diskdatao[x], diskdata[x], 65500);
		}
		std::string fn2 = FCEU_MakeFName(FCEUMKF_FDS, 0, 0);
		FCEUFILE *tp = FCEU_fopen(fn2.c_str(), 0, "rb", 0);
		if (tp) {
			FCEU_printf("Disk was written. Auxiliary FDS file open \"%s\".\n", fn2.c_str());
			FreeFDSMemory();
			if (SubLoad(tp)) { FDSBIOS_owner.reset(); FDSBIOS = NULL; FreeFDSMemory();
				FCEU_PrintError("Error reading auxiliary FDS file."); return LOADER_HANDLED_ERROR; }
			FCEU_fclose(tp); DiskWritten = 1;
		}
	}

	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name);
	if (!g_fds_state) g_fds_state = fceux11_rust_fds_runtime_create();
	GameInfo->type = GIT_FDS; GameInterface = FDSGI; isFDS = true;
	SelectDisk = 0; InDisk = 255;
	ResetExState(PreSave, PostSave); FDSSoundStateAdd();

	for (int x = 0; x < TotalSides; x++) {
		char temp[5]; snprintf(temp, sizeof(temp), "DDT%d", x);
		AddExState(diskdata[x], 65500, 0, temp);
	}
	AddExState(FDSRegs, sizeof(FDSRegs), 0, "FREG");
	AddExState(&IRQCount, 4, 1, "IRQC");  AddExState(&IRQLatch, 4, 1, "IQL1");
	AddExState(&IRQa, 1, 0, "IRQA");       AddExState(&writeskip, 1, 0, "WSKI");
	AddExState(&DiskPtr, 4, 1, "DPTR");    AddExState(&DiskSeekIRQ, 4, 1, "DSIR");
	AddExState(&SelectDisk, 1, 0, "SELD"); AddExState(&InDisk, 1, 0, "INDI");
	AddExState(&DiskWritten, 1, 0, "DSKW");
	AddExState(&mapperFDS_control, 1, 0, "CTRG"); AddExState(&mapperFDS_filesize, 2, 1, "FLSZ");
	AddExState(&mapperFDS_block, 1, 0, "BLCK");   AddExState(&mapperFDS_blockstart, 2, 1, "BLKS");
	AddExState(&mapperFDS_blocklen, 2, 1, "BLKL"); AddExState(&mapperFDS_diskaddr, 2, 1, "DADR");
	AddExState(&mapperFDS_diskaccess, 1, 0, "DACC");

	CHRRAMSize = 8192;
	CHRRAM_owner = FCEU_gmalloc_unique(CHRRAMSize); CHRRAM = CHRRAM_owner.get();
	SetupCartCHRMapping(0, CHRRAM, CHRRAMSize, 1); AddExState(CHRRAM, CHRRAMSize, 0, "CHRR");

	FDSRAMSize = 32768;
	FDSRAM_owner = FCEU_gmalloc_unique(FDSRAMSize); FDSRAM = FDSRAM_owner.get();
	SetupCartPRGMapping(1, FDSRAM, FDSRAMSize, 1); AddExState(FDSRAM, FDSRAMSize, 0, "FDSR");

	SetupCartMirroring(0, 0, 0);
	FCEU_printf(" Sides: %d\n\n", TotalSides);
	fceu11::SetVidSystem(0);
	return LOADER_OK;
}

void FDSClose(void) {
	isFDS = false;
	bool was_written = g_fds_state ? (g_fds_state->disk_written != 0) : false;
	if (g_fds_state) { fceux11_rust_fds_runtime_destroy(g_fds_state); g_fds_state = nullptr; }
	if (!was_written) return;
	const std::string &fn = FCEU_MakeFName(FCEUMKF_FDS, 0, 0);
	FILE *fp = FCEUD_UTF8fopen(fn.c_str(), "wb");
	if (!fp) return;
	for (int x = 0; x < TotalSides; x++)
		if (fwrite(diskdata[x], 1, 65500, fp) != 65500)
			{ FCEU_PrintError("Error saving FDS image!"); fclose(fp); return; }
	// v1.13 Purify F2a: diskdatao[] was FCEU_malloc(); use matching FCEU_free()
	for (int x = 0; x < TotalSides; x++) { FCEU_free(diskdatao[x]); diskdatao[x] = 0; }
	FreeFDSMemory();
	// FDSBIOS/FDSRAM/CHRRAM use the FceuMallocPtr owners (FDSBIOS_owner etc.); RAII frees them.
	FDSBIOS_owner.reset(); FDSRAM_owner.reset(); CHRRAM_owner.reset();
	FDSBIOS = FDSRAM = CHRRAM = NULL;
	fclose(fp);
}
