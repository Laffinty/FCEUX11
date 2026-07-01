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

//  TODO: Add (better) file io error checking

#include "version.h"
#include "utils/safe_string.h"
#include "types.h"
#include "x6502.h"
#include "fceu.h"
#include "sound.h"
#include "utils/endian.h"
#include "utils/memory.h"
#include "utils/xstring.h"
#include "file.h"
#include "fds.h"
#include "state.h"
#include "movie.h"
#include "rust/fceux11_rust.h"
#include "ppu.h"
#include "netplay.h"
#include "video.h"
#include "input.h"
#include "driver.h"
#include "cart.h"

#ifdef _WIN32
// v0.3.15.x PHASE-3: DirectStorage probe scaffold. Only compiled in
// on Windows; the entire namespace is empty on other platforms
// (see src/platform/win11/DirectStorageProbe.h).
#include "platform/win11/DirectStorageProbe.h"
#endif
#ifdef _S9XLUA_H
#include "fceulua.h"
#endif

//TODO - we really need some kind of global platform-specific options api
#ifdef __WIN_DRIVER__
#include "drivers/win/main.h"
#include "drivers/win/cheat.h"
#include "drivers/win/ram_search.h"
#include "drivers/win/ramwatch.h"
#endif

#include <string>
#include <cstdio>
#include <cstdlib>
#include <cstring>
//#include <unistd.h> //mbg merge 7/17/06 removed

#include <vector>
#include <fstream>

// v1.2 Census §2.3: `using namespace std` removed; all std types below
// are explicitly qualified with std::.

static void (*SPreSave)(void) = NULL;
static void (*SPostSave)(void) = NULL;

static int SaveStateStatus[10];
static int StateShow;

//tells the save system innards that we're loading the old format
bool FCEU_state_loading_old_format = false;

std::string lastSavestateMade; //Stores the filename of the last savestate made (needed for UndoSavestate)
bool undoSS = false;		  //This will be true if there is lastSavestateMade, it was made since ROM was loaded, a backup state for lastSavestateMade exists
bool redoSS = false;		  //This will be true if UndoSaveState is run, will turn false when a new savestate is made

std::string lastLoadstateMade; //Stores the filename of the last state loaded (needed for Undo/Redo loadstate)
bool undoLS = false;		  //This will be true if a backupstate was made and it was made since ROM was loaded
bool redoLS = false;		  //This will be true if a backupstate was loaded, meaning redoLoadState can be run

bool internalSaveLoad = false;

bool backupSavestates = true;
bool compressSavestates = true;  //By default FCEUX compresses savestates when a movie is inactive.

// Savestate buffer management has been moved to Rust (fceux11-core::state_file).

#define SFMDATA_SIZE (128)
static SFORMAT SFMDATA[SFMDATA_SIZE];
static int SFEXINDEX;

// v1.9 Chronicle: preserve unknown chunks across load→save roundtrip.
// When loading a V2 savestate that contains chunk types this version
// doesn't recognize, we store them here so FCEUSS_SaveMS can re-include
// them. This ensures forward-compatible savestates survive a load→save
// cycle without data loss.
struct UnknownChunk {
	uint8_t type;
	std::vector<uint8_t> data;
};
static std::vector<UnknownChunk> g_unknownChunks;

#define RLSB 		FCEUSTATE_RLSB	//0x80000000


extern SFORMAT FCEUPPU_STATEINFO[];
extern SFORMAT FCEU_NEWPPU_STATEINFO[];
extern SFORMAT FCEUSND_STATEINFO[];
extern SFORMAT FCEUCTRL_STATEINFO[];
extern SFORMAT FCEUMOV_STATEINFO[];

//why two separate CPU structs?? who knows

SFORMAT SFCPU[]={
	{ &g_cpu.native_layout().PC, 2|RLSB, "PC\0"},
	{ &g_cpu.native_layout().A, 1, "A\0\0"},
	{ &g_cpu.native_layout().X, 1, "X\0\0"},
	{ &g_cpu.native_layout().Y, 1, "Y\0\0"},
	{ &g_cpu.native_layout().S, 1, "S\0\0"},
	{ &g_cpu.native_layout().P, 1, "P\0\0"},
	{ &g_cpu.native_layout().DB, 1, "DB\0"},
	{ &RAM, 0x800 | FCEUSTATE_INDIRECT, "RAM", },
	{ 0 }
};

SFORMAT SFCPUC[]={
	{ &X.jammed, 1, "JAMM"},
	{ &X.IRQlow, 4|RLSB, "IQLB"},
	{ &X.tcount, 4|RLSB, "ICoa"},
	{ &X.count,  4|RLSB, "ICou"},
	{ &timestampbase, sizeof(timestampbase) | RLSB, "TSBS"},
	{ &X.mooPI, 1, "MooP"}, // alternative to the "quick and dirty hack"
	{ 0 }
};

void foo(uint8* test) { (void)test; }

static int SubWrite(EMUFILE* os, SFORMAT *sf)
{
	uint32 acc=0;

	while(sf->v)
	{
		if(sf->s==~0u)		//Link to another struct
		{
			uint32 tmp;

			if(!(tmp=SubWrite(os,(SFORMAT *)sf->v)))
				return(0);
			acc+=tmp;
			sf++;
			continue;
		}

		acc+=8;			//Description + size
		acc+=sf->s&(~FCEUSTATE_FLAGS);

		if(os)			//Are we writing or calculating the size of this block?
		{
			os->fwrite(std::span<const std::byte>(
				reinterpret_cast<const std::byte*>(sf->desc), 4));
			write32le(sf->s&(~FCEUSTATE_FLAGS),os);

#ifdef FCEU_BIG_ENDIAN
			if(sf->s&RLSB)
				FlipByteOrder((uint8*)sf->v,sf->s&(~FCEUSTATE_FLAGS));
#endif

			if(sf->s&FCEUSTATE_INDIRECT)
				os->fwrite(std::span<const std::byte>(
					reinterpret_cast<const std::byte*>(*(char **)sf->v), sf->s&(~FCEUSTATE_FLAGS)));
			else
				os->fwrite(std::span<const std::byte>(
					reinterpret_cast<const std::byte*>((char*)sf->v), sf->s&(~FCEUSTATE_FLAGS)));

			//Now restore the original byte order.
#ifdef FCEU_BIG_ENDIAN
			if(sf->s&RLSB)
				FlipByteOrder((uint8*)sf->v,sf->s&(~FCEUSTATE_FLAGS));
#endif
		}
		sf++;
	}

	return(acc);
}

static SFORMAT *CheckS(SFORMAT *sf, uint32 tsize, char *desc)
{
	while(sf->v)
	{
		if(sf->s==~0u)		// Link to another SFORMAT structure.
		{
			SFORMAT *tmp;
			if((tmp= CheckS((SFORMAT *)sf->v, tsize, desc) ))
				return(tmp);
			sf++;
			continue;
		}
		if(!memcmp(desc,sf->desc,4))
		{
			if(tsize!=(sf->s&(~FCEUSTATE_FLAGS)))
				return(0);
			return(sf);
		}
		sf++;
	}
	return(0);
}

static bool ReadStateChunk(EMUFILE* is, SFORMAT *sf, int size)
{
	SFORMAT *tmp;
	int temp = is->ftell();

	while(is->ftell()<temp+size)
	{
		uint32 tsize;
		char toa[4];
		if(is->fread(std::span<std::byte>(
				reinterpret_cast<std::byte*>(toa), 4)) < 4)
			return false;

		read32le(&tsize,is);

		if((tmp=CheckS(sf,tsize,toa)))
		{
			if(tmp->s&FCEUSTATE_INDIRECT)
				is->fread(std::span<std::byte>(
					reinterpret_cast<std::byte*>(*(char **)tmp->v), tmp->s&(~FCEUSTATE_FLAGS)));
			else
				is->fread(std::span<std::byte>(
					reinterpret_cast<std::byte*>((char *)tmp->v), tmp->s&(~FCEUSTATE_FLAGS)));

#ifdef FCEU_BIG_ENDIAN
			if(tmp->s&RLSB)
				FlipByteOrder((uint8*)tmp->v,tmp->s&(~FCEUSTATE_FLAGS));
#endif
		}
		else
			is->fseek(tsize,SEEK_CUR);
	} // while(...)
	return true;
}

/// Buffer-based variant used by the Rust state-file loader.
static bool ReadStateChunkFromBuffer(const uint8_t* data, int size, SFORMAT *sf)
{
	SFORMAT *tmp;
	int pos = 0;
	int end = size;

	while(pos < end)
	{
		uint32 tsize;
		char toa[4];
		if(pos + 4 > end)
			return false;
		memcpy(toa, data + pos, 4);
		pos += 4;

		if(pos + 4 > end)
			return false;
		tsize = data[pos] | (data[pos+1] << 8) | (data[pos+2] << 16) | (data[pos+3] << 24);
		pos += 4;

		if((tmp=CheckS(sf,tsize,toa)))
		{
			if(pos + (int)tsize > end)
				return false;
			if(tmp->s&FCEUSTATE_INDIRECT)
				memcpy(*(char **)tmp->v, data + pos, tmp->s&(~FCEUSTATE_FLAGS));
			else
				memcpy((char *)tmp->v, data + pos, tmp->s&(~FCEUSTATE_FLAGS));

#ifdef FCEU_BIG_ENDIAN
			if(tmp->s&RLSB)
				FlipByteOrder((uint8*)tmp->v,tmp->s&(~FCEUSTATE_FLAGS));
#endif
		}
		pos += tsize;
	}
	return true;
}

static int read_sfcpuc=0, read_snd=0;

void FCEUD_BlitScreen(uint8 *XBuf); //mbg merge 7/17/06 YUCKY had to add
void UpdateFCEUWindow(void);  //mbg merge 7/17/06 YUCKY had to add
int CurrentState=0;
extern int geniestage;


bool FCEUSS_SaveMS(EMUFILE* outstream, int compressionLevel)
{
	FCEUPPU_SaveState();
	FCEUSND_SaveState();

	struct Chunk {
		uint8_t type;
		std::vector<uint8_t> data;
	};
	std::vector<Chunk> chunks;

	// v1.9 Chronicle: C++ handles SFORMAT field-level serialization via
	// SubWrite (safe memory access to C++ globals). Rust handles V2 file
	// format wrapping (CRC32, compression). This split avoids unsafe
	// cross-language pointer operations on SFORMAT tables.
	auto addSformatChunk = [&](uint8_t type, SFORMAT* sf) {
		if (!sf || !sf->v) return;
		EMUFILE_MEMORY mem;
		int size = SubWrite(&mem, sf);
		if (size > 0) {
			std::vector<uint8_t> buf(size);
			mem.fseek(0, SEEK_SET);
			mem.fread(std::span<std::byte>(reinterpret_cast<std::byte*>(buf.data()), size));
			// v1.9: validate SFORMAT stream and compute CRC32 via Rust
			int32_t entries = fceux11_rust_sformat_validate(buf.data(), buf.size());
			if (entries >= 0) {
				chunks.push_back({type, std::move(buf)});
			}
			// If validation fails, skip this chunk (corrupt SFORMAT data)
		}
	};

	addSformatChunk(1, SFCPU);
	addSformatChunk(2, SFCPUC);
	addSformatChunk(3, FCEUPPU_STATEINFO);
	addSformatChunk(31, FCEU_NEWPPU_STATEINFO);
	addSformatChunk(4, FCEUCTRL_STATEINFO);
	addSformatChunk(5, FCEUSND_STATEINFO);

	if (FCEUMOV_Mode(MOVIEMODE_PLAY | MOVIEMODE_RECORD | MOVIEMODE_FINISHED))
	{
		addSformatChunk(6, FCEUMOV_STATEINFO);

		if (!FCEUMOV_Mode(MOVIEMODE_TASEDITOR))
		{
			EMUFILE_MEMORY movMem;
			int movSize = FCEUMOV_WriteState(&movMem);
			if (movSize > 0) {
				std::vector<uint8_t> buf(movSize);
				movMem.fseek(0, SEEK_SET);
				movMem.fread(std::span<std::byte>(reinterpret_cast<std::byte*>(buf.data()), movSize));
				chunks.push_back({7, std::move(buf)});
			}
		}
	}

	// save back buffer
	{
		extern uint8 *XBackBuf;
		std::vector<uint8_t> buf(256 * 256);
		memcpy(buf.data(), XBackBuf, 256 * 256);
		chunks.push_back({8, std::move(buf)});
	}

	// v1.7 Phase D: give the current cart a chance to prepare state before
	// the SFMDATA chunk is serialized. Default implementation is a no-op.
	if (currCartInfo && currCartInfo->cart_obj)
		currCartInfo->cart_obj->on_save_pre();

	if (SPreSave) SPreSave();
	addSformatChunk(0x10, SFMDATA);
	if (SPostSave) SPostSave();

	// v1.9 Chronicle: re-include unknown chunks preserved from a prior load.
	// This ensures forward-compatible savestates survive load→save roundtrip.
	for (auto& uc : g_unknownChunks) {
		chunks.push_back({uc.type, uc.data});
	}

	// Build FFI inputs and call Rust state-file serializer
	std::vector<FceuStateChunkInput> inputs;
	inputs.reserve(chunks.size());
	for (auto& c : chunks) {
		inputs.push_back({c.type, c.data.data(), c.data.size()});
	}

	FceuStateBuffer outbuf = {nullptr, 0, 0};
	bool ok;

	// v1.9 Chronicle: use V2 format (FCEU11ST) by default for per-chunk
	// CRC32 integrity. Movie recording mode forces V1 (FCSX) for
	// cross-emulator compatibility.
	if (FCEUMOV_Mode(MOVIEMODE_PLAY | MOVIEMODE_RECORD | MOVIEMODE_FINISHED)) {
		ok = fceux11_rust_state_file_save(
			inputs.data(), inputs.size(),
			FCEU_VERSION_NUMERIC,
			compressionLevel,
			&outbuf
		);
	} else {
		ok = fceux11_rust_state_file_save_v2(
			inputs.data(), inputs.size(),
			compressionLevel,
			&outbuf
		);
	}

	if (ok && outbuf.ptr && outbuf.len > 0) {
		outstream->fwrite(std::span<const std::byte>(
			reinterpret_cast<const std::byte*>(outbuf.ptr), outbuf.len));
		fceux11_rust_state_file_buf_free(outbuf);
	}

	return ok;
}


void FCEUSS_Save(const char *fname, bool display_message)
{
	EMUFILE* st = 0;
	std::string fn;

	// v0.3.15.x PHASE-3: DirectStorage probe scaffold added.
	// probeDirectStorage() in src/platform/win11/DirectStorageProbe.cpp
	// is invoked once during fceuWrapperInit() and the result is
	// cached in fceu11::platform::win11::g_directStorageCaps. When
	// caps.isSupported is true, the v0.4.x takeover will route the
	// .fc0/.fcs write through IDStorageFactory -> IDStorageQueue for
	// zero-copy async I/O. The current path uses std::fstream, which
	// is bound by the OS page cache. Implementation deferred to v0.4.x
	// (requires vcpkg `directstorage` dep and a 1-2 person-day
	// refactor of EMUFILE_FILE).
	extern fceu11::platform::win11::DirectStorageCaps g_directStorageCaps;
	(void)g_directStorageCaps; // referenced for future v0.4.x takeover

	if (geniestage==1)
	{
		if (display_message)
			FCEU_DispMessage("Cannot save FCS in GG screen.",0);
		return;
	}

	if(fname)	//If filename is given use it.
	{
		st = FCEUD_UTF8_fstream(fname, "wb");
		fn.assign(fname);
	}
	else		//Else, generate one
	{
		//FCEU_PrintError("daCurrentState=%d",CurrentState);
		fn = FCEU_MakeFName(FCEUMKF_STATE,CurrentState,0);

		//backup existing savestate first
		if (CheckFileExists(fn.c_str()) && backupSavestates)	//adelikat:  If the files exists and we are allowed to make backup savestates
		{
			CreateBackupSaveState(fn.c_str());		//Make a backup of previous savestate before overwriting it
			lastSavestateMade.assign(fn);	//Remember what the last savestate filename was (for undoing later)
			undoSS = true;					//Backup was created so undo is possible
		}
		else
			undoSS = false;					//so backup made so lastSavestateMade does have a backup file, so no undo

		st = FCEUD_UTF8_fstream(fn.c_str(),"wb");
	}

	if (st == NULL || st->get_fp() == NULL)
	{
		if (display_message)
			FCEU_DispMessage("State %d save error.", 0, CurrentState);
		return;
	}

	#ifdef _S9XLUA_H
	if (!internalSaveLoad)
	{
		LuaSaveData saveData;
		CallRegisteredLuaSaveFunctions(CurrentState, saveData);

		std::string luaSaveFilename;
		luaSaveFilename.assign(fn.c_str());
		luaSaveFilename.append(".luasav");
		if(saveData.recordList)
		{
			FILE* luaSaveFile = fopen(luaSaveFilename.c_str(), "wb");
			if(luaSaveFile)
			{
				saveData.ExportRecords(luaSaveFile);
				fclose(luaSaveFile);
			}
		}
		else
		{
			unlink(luaSaveFilename.c_str());
		}
	}
	#endif

	if(FCEUMOV_Mode(MOVIEMODE_INACTIVE))
		FCEUSS_SaveMS(st,-1);
	else
		FCEUSS_SaveMS(st,0);

	delete st;

	if(!fname)
	{
		SaveStateStatus[CurrentState] = 1;
		if (display_message)
			FCEU_DispMessage("State %d saved.", 0, CurrentState);
	}
	redoSS = false;					//we have a new savestate so redo is not possible
}

bool FCEUSS_LoadFP(EMUFILE* is, ENUM_SSLOADPARAMS params)
{
	if(!is) return false;

	// v1.9: clear previously preserved unknown chunks before loading new state
	g_unknownChunks.clear();

	//maybe make a backup savestate
	bool backup = (params == SSLOADPARAM_BACKUP);
	EMUFILE_MEMORY msBackupSavestate;
	if(backup)
	{
		FCEUSS_SaveMS(&msBackupSavestate,0);
	}

	// Read entire file into memory
	is->fseek(0, SEEK_END);
	size_t fileSize = is->ftell();
	is->fseek(0, SEEK_SET);
	std::vector<uint8_t> fileData(fileSize);
	is->fread(std::span<std::byte>(reinterpret_cast<std::byte*>(fileData.data()), fileSize));

	// Detect old format for compatibility flags
	bool isOldFormat = false;
	if (fileData.size() >= 4 && fileData[0] == 'F' && fileData[1] == 'C' && fileData[2] == 'S') {
		if (fileData[3] != 'X') isOldFormat = true;
	}

	FceuStateChunkOutput* rustChunks = nullptr;
	size_t chunkCount = 0;
	uint32_t version = 0;
	uint32_t totalsize = 0;

	if (!fceux11_rust_state_file_load(
			fileData.data(), fileData.size(),
			&rustChunks, &chunkCount,
			&version, &totalsize)) {
		if (backup) {
			msBackupSavestate.fseek(0, SEEK_SET);
			FCEUSS_LoadFP(&msBackupSavestate, SSLOADPARAM_NOBACKUP);
		}
		return false;
	}

	FCEU_state_loading_old_format = isOldFormat;
	FCEUMOV_PreLoad();

	bool ret = true;
	bool warned = false;
	read_sfcpuc = 0;
	read_snd = 0;

	// v1.9 Chronicle: count SFORMAT entries for Rust FFI.
	auto countSfEntries = [](SFORMAT* sf) -> size_t {
		size_t n = 0;
		while (sf && sf->v) { ++n; ++sf; }
		return n;
	};

	// v1.9 Chronicle: C++ handles SFORMAT field-level deserialization via
	// ReadStateChunkFromBuffer (safe memory access to C++ globals).
	// Rust validates SFORMAT stream structure before deserialization.
	auto deserializeSformatChunk = [&](uint8_t* data, int size, SFORMAT* sf) -> bool {
		if (!sf || !sf->v) return true;
		// Validate SFORMAT stream structure via Rust FFI
		int32_t entries = fceux11_rust_sformat_validate(data, size);
		if (entries < 0) {
			return false; // corrupt stream
		}
		return ReadStateChunkFromBuffer(data, size, sf);
	};

	for (size_t i = 0; i < chunkCount; i++) {
		uint8_t t = rustChunks[i].chunk_type;
		uint8_t* data = rustChunks[i].data;
		int size = (int)rustChunks[i].len;

		switch (t) {
		case 1:
			if (!deserializeSformatChunk(data, size, SFCPU)) ret = false;
			break;
		case 2:
			if (!deserializeSformatChunk(data, size, SFCPUC)) ret = false;
			else read_sfcpuc = 1;
			break;
		case 3:
			if (!deserializeSformatChunk(data, size, FCEUPPU_STATEINFO)) ret = false;
			break;
		case 31:
			if (!deserializeSformatChunk(data, size, FCEU_NEWPPU_STATEINFO)) ret = false;
			break;
		case 4:
			if (!deserializeSformatChunk(data, size, FCEUCTRL_STATEINFO)) ret = false;
			break;
		case 5:
			if (!deserializeSformatChunk(data, size, FCEUSND_STATEINFO)) ret = false;
			else read_snd = 1;
			break;
		case 6:
			if (FCEUMOV_Mode(MOVIEMODE_PLAY | MOVIEMODE_RECORD | MOVIEMODE_FINISHED)) {
				if (!ReadStateChunkFromBuffer(data, size, FCEUMOV_STATEINFO)) ret = false;
			}
			break;
		case 7:
			{
				EMUFILE_MEMORY mem(data, size);
				if (!FCEUMOV_ReadState(&mem, size)) {
					if (!FCEU_state_loading_old_format)
						ret = false;
				}
			}
			break;
		case 8:
			{
				extern uint8 *XBackBuf;
				if (size == 256 * 256 + 8) {
					memcpy(XBackBuf, data, 256 * 256);
				} else {
					memcpy(XBackBuf, data, size);
				}
			}
			break;
		case 0x10:
			if (!deserializeSformatChunk(data, size, SFMDATA)) ret = false;
			break;
		default:
			// v1.9 Chronicle: preserve unknown chunks for forward-compatible
			// load→save roundtrip. Store raw data so FCEUSS_SaveMS can re-include them.
			g_unknownChunks.push_back({t, std::vector<uint8_t>(data, data + size)});
			if (!warned) {
				char str[256];
				snprintf( str, sizeof(str), "Warning: Found unknown save chunk of type %d (preserved).\nThis could indicate the save state is made with a newer version.", t);
				FCEUD_PrintError(str);
				warned = true;
			}
			break;
		}
	}

	fceux11_rust_state_file_chunks_free(rustChunks, chunkCount);
	FCEU_state_loading_old_format = false;

	if (read_sfcpuc && version < 9500)
	{
		X.IRQlow = 0;
	}

	if (GameStateRestore)
	{
		GameStateRestore(version);
	}
	if (ret)
	{
		FCEUPPU_LoadState(version);
		FCEUSND_LoadState(version);
		ret = FCEUMOV_PostLoad();
		// v1.7 Phase D: notify the current cart after a successful load.
		// Default implementation is a no-op, so savestate compatibility is
		// unchanged.
		if (ret && currCartInfo && currCartInfo->cart_obj)
			currCartInfo->cart_obj->on_load_post();
	} else if (backup)
	{
		msBackupSavestate.fseek(0, SEEK_SET);
		FCEUSS_LoadFP(&msBackupSavestate, SSLOADPARAM_NOBACKUP);
	}

	return ret;
}


bool FCEUSS_Load(const char *fname, bool display_message)
{
	// v0.3.6: std::unique_ptr<EMUFILE> replaces the deprecated fceuScopedPtr.
	// The unique_ptr's default deleter calls `delete` on the owned EMUFILE,
	// matching the previous FCEU_ALLOC_TYPE_NEW behavior.
	std::unique_ptr<EMUFILE> st;
	std::string fn;

	//mbg movie - this needs to be overhauled
	////this fixes read-only toggle problems
	//if(FCEUMOV_IsRecording()) {
	//	FCEUMOV_AddCommand(0);
	//	MovieFlushHeader();
	//}

	if (geniestage == 1)
	{
		if (display_message)
			FCEU_DispMessage("Cannot load FCS in GG screen.",0);
		return false;
	}
	if (fname)
	{
		st.reset(FCEUD_UTF8_fstream(fname, "rb"));
		fn.assign(fname);
	}
	else
	{
		fn = FCEU_MakeFName(FCEUMKF_STATE,CurrentState,fname);
		st.reset(FCEUD_UTF8_fstream(fn.c_str(),"rb"));
        	lastLoadstateMade.assign(fn);
	}

	if (st.get() == NULL || (st.get()->get_fp() == NULL))
	{
		if (display_message)
		{
			FCEU_DispMessage("State %d load error.", 0, CurrentState);
			//FCEU_DispMessage("State %d load error. Filename: %s", 0, CurrentState, fn.c_str());
		}
		SaveStateStatus[CurrentState] = 0;
		return false;
	}

	//If in bot mode, don't do a backup when loading.
	//Otherwise you eat at the hard disk, since so many
	//states are being loaded.
	if (FCEUSS_LoadFP(st.get(), backupSavestates ? SSLOADPARAM_BACKUP : SSLOADPARAM_NOBACKUP))
	{
		if (fname)
		{
			char szFilename[260]={0};
			splitpath(fname, 0, 0, szFilename, 0);
			if (display_message)
			{
				FCEU_DispMessage("State %s loaded.", 0, szFilename);
				//FCEU_DispMessage("State %s loaded. Filename: %s", 0, szFilename, fn.c_str());
			}
		}
		else
		{
			if (display_message)
			{
				FCEU_DispMessage("State %d loaded.", 0, CurrentState);
				//FCEU_DispMessage("State %d loaded. Filename: %s", 0, CurrentState, fn.c_str());
			}
			SaveStateStatus[CurrentState] = 1;
		}

		#ifdef _S9XLUA_H
		if (!internalSaveLoad)
		{
			LuaSaveData saveData;

			std::string luaSaveFilename;
			luaSaveFilename.assign(fn.c_str());
			luaSaveFilename.append(".luasav");
			FILE* luaSaveFile = fopen(luaSaveFilename.c_str(), "rb");
			if(luaSaveFile)
			{
				saveData.ImportRecords(luaSaveFile);
				fclose(luaSaveFile);
			}

			CallRegisteredLuaLoadFunctions(CurrentState, saveData);
		}
		#endif

#ifdef __WIN_DRIVER__
		Update_RAM_Search(); // Update_RAM_Watch() is also called.
#endif

		//Update input display if movie is loaded
		extern uint32 cur_input_display;
		extern uint8 FCEU_GetJoyJoy(void);

		cur_input_display = FCEU_GetJoyJoy(); //Input display should show the last buttons pressed (stored in the savestate)

		return true;
	}
	else
	{
		if(!fname)
			SaveStateStatus[CurrentState] = 1;

		if (display_message)
		{
			FCEU_DispMessage("Error(s) reading state %d!", 0, CurrentState);
			//FCEU_DispMessage("Error(s) reading state %d! Filename: %s", 0, CurrentState, fn);
		}
		return 0;
	}
}

void FCEUSS_CheckStates(void)
{
	FILE *st=NULL;
	int ssel;

	for(ssel=0;ssel<10;ssel++)
	{
		st=FCEUD_UTF8fopen(FCEU_MakeFName(FCEUMKF_STATE,ssel,0),"rb");
		if(st)
		{
			SaveStateStatus[ssel]=1;
			fclose(st);
		}
		else
			SaveStateStatus[ssel]=0;
	}

	CurrentState=1;
	StateShow=0;
}

void ResetExState(void (*PreSave)(void), void (*PostSave)(void))
{
	int x;
	for(x=0;x<SFEXINDEX;x++)
	{
		if(SFMDATA[x].desc)
			FCEU_free( (void*)SFMDATA[x].desc);
	}
	// adelikat, 3/14/09:  had to add this to clear out the size parameter.  NROM(mapper 0) games were having savestate crashes if loaded after a non NROM game	because the size variable was carrying over and causing savestates to save too much data
	SFMDATA[0].s = 0;

	SPreSave = PreSave;
	SPostSave = PostSave;
	SFEXINDEX=0;
}

// v1.4 Gateway Phase 6: per-board PreSave setter. Replaces SPreSave
// directly without disturbing the SFORMAT registration table, so
// board mappers like vrc7.cpp can install a PreSave trampoline that
// copies pointer-laden state into a snapshot buffer before the SFORMAT
// walker serialises.
void FCEU_SetStatePreSave(void (*PreSave)(void))
{
	SPreSave = PreSave;
}

void AddExState(void *v, uint32 s, int type, const char *desc)
{
	//do not accept extra state information if a null pointer was provided for v, so list won't terminate early
	if (v == 0) return;
	
	if(s==~0u)
	{
		SFORMAT* sf = (SFORMAT*)v;
		std::map<std::string,bool> names;
		while(sf->v)
		{
			char tmp[5] = {0};
			memcpy(tmp,sf->desc,4);
			std::string desc = tmp;
			if(names.find(desc) != names.end())
			{
#ifdef __WIN_DRIVER__
				MessageBox(NULL,"OH NO!!! YOU HAVE AN INVALID SFORMAT! POST A BUG TICKET ALONG WITH INFO ON THE ROM YOURE USING\n","OOPS",MB_OK);
#else
				printf("OH NO!!! YOU HAVE AN INVALID SFORMAT! POST A BUG TICKET ALONG WITH INFO ON THE ROM YOURE USING\n");
#endif
				exit(0);
			}
			names[desc] = true;
			sf++;
		}
	}

	if(desc)
	{
		// v0.3.6.5-followup: capture the actual buffer size; do NOT use
		// sizeof((char*)SFMDATA[SFEXINDEX].desc) — that is the size of a
		// pointer (8 on x64), not the malloc'd buffer, and triggers an
		// ASan heap-buffer-overflow for any desc shorter than 7 chars
		// (e.g. the 4-char "CHRR"/"EXNR" tags registered during iNES_Init).
		const size_t desc_len = strlen(desc) + 1;
		SFMDATA[SFEXINDEX].desc = (const char *)FCEU_malloc(desc_len);
		FCEU_strlcpy((char*)SFMDATA[SFEXINDEX].desc, desc_len, desc);
	}
	else
		SFMDATA[SFEXINDEX].desc=0;
	SFMDATA[SFEXINDEX].v=v;
	SFMDATA[SFEXINDEX].s=s;
	if(type) SFMDATA[SFEXINDEX].s|=RLSB;
	if(SFEXINDEX<SFMDATA_SIZE-1)
		SFEXINDEX++;
	else
	{
		static int once=1;
		if(once)
		{
			once=0;
			FCEU_PrintError("Error in AddExState: SFEXINDEX overflow.\nSomebody made SFMDATA_SIZE too small.");
		}
	}
	SFMDATA[SFEXINDEX].v=0;		// End marker.
}

void fceu11::SelectStateNext(int n)
{
	if(n>0)
		CurrentState=(CurrentState+1)%10;
	else
		CurrentState=(CurrentState+9)%10;
	fceu11::SelectStateSlot(CurrentState, 1);
}

int fceu11::SelectStateSlot(int w, int show)
{
	int oldstate=CurrentState;
	FCEUSS_CheckStates();
	if(w == -1) { StateShow = 0; return 0; } //mbg merge 7/17/06 had to make return a value

	CurrentState=w;
	if(show)
	{
		StateShow=180;
		FCEU_DispMessage("-select state-",0);
	}
	return oldstate;
}

void fceu11::SaveStateFile(const char *fname, bool display_message)
{
	if(!FCEU_IsValidUI(FCEUI_SAVESTATE)) return;

	StateShow = 0;

	FCEUSS_Save(fname, display_message);
}

int loadStateFailed = 0; // hack, this function should return a value instead

bool file_exists(const char * filename)
{
    if (FILE * file = fopen(filename, "r")) //I'm sure, you meant for READING =)
    {
        fclose(file);
        return true;
    }
    return false;
}
void fceu11::LoadStateFile(const char *fname, bool display_message)
{
	if(!FCEU_IsValidUI(FCEUI_LOADSTATE)) return;

	StateShow = 0;
	loadStateFailed = 0;

	/* For network play, be load the state locally, and then save the state to a temporary file,
	and send that.  This insures that if an older state is loaded that is missing some
	information expected in newer save states, desynchronization won't occur(at least not
	from this ;)).
	*/
	if (backupSavestates)
		BackupLoadState();	// If allowed, backup the current state before loading a new one

	if (!movie_readonly && autoMovieBackup && freshMovie) //If auto-backup is on, movie has not been altered this session and the movie is in read+write mode
	{
		FCEUI_MakeBackupMovie(false);	//Backup the movie before the contents get altered, but do not display messages
	}
	if (fname != NULL && !file_exists(fname))
	{
		loadStateFailed = 1;
		return; // state doesn't exist; exit cleanly
	}

	if (FCEUSS_Load(fname, display_message))
	{
		//in case we're loading a savestate made with old ppu, we need to make sure ppur's regs used for dividing are ready to go
		newppu_hacky_emergency_reset();

		//mbg todo netplay
		freshMovie = false;		//The movie has been altered so it is no longer fresh
	} else
	{
		loadStateFailed = 1;
	}
}

void FCEU_DrawSaveStates(uint8 *XBuf)
{
	if(!StateShow) return;

	FCEU_DrawNumberRow(XBuf,SaveStateStatus,CurrentState);
	StateShow--;
}

//*************************************************************************
//Savestate backup functions
//(Used when making savestates)
//*************************************************************************

std::string GenerateBackupSaveStateFn(const char *fname)
{
	//This backup is for the backup "slot" for any savestate made.  Example: smb.fc0 becomes smb-bak.fc0
	std::string filename;
	filename = fname;	//Convert fname to a string object
	int x = filename.find_last_of("."); //Find file extension
	filename.insert(x,"-bak");		//add "-bak" before the dot.

	return filename;
}


void CreateBackupSaveState(const char *fname)
{
	std::string newFilename = GenerateBackupSaveStateFn(fname);	//Get backup savestate filename
	if (CheckFileExists(newFilename.c_str()))				//See if backup already exists
		remove(newFilename.c_str())	;						//If so, delete it
	rename(fname,newFilename.c_str());						//Rename savestate to backup filename
	undoSS = true;		//There is a backup savestate file to mast last loaded, so undo is possible
}

void SwapSaveState()
{
	//--------------------------------------------------------------------------------------------
	//Both files must exist
	//--------------------------------------------------------------------------------------------

	if (lastSavestateMade.empty())
	{
		FCEU_DispMessage("Can't Undo",0);
		FCEU_printf("Undo savestate was attempted but unsuccessful because there was not a recently used savestate.\n");
		return;		//If there is no last savestate, can't undo
	}
	std::string backup = GenerateBackupSaveStateFn(lastSavestateMade.c_str());	//Get filename of backup state
	if (!CheckFileExists(backup.c_str()))
	{
		FCEU_DispMessage("Can't Undo",0);
		FCEU_printf("Undo savestate was attempted but unsuccessful because there was not a backup of the last used savestate.\n");
		return;		//If no backup, can't undo
	}

	//--------------------------------------------------------------------------------------------
	//So both exists, now swap the last savestate and its backup
	//--------------------------------------------------------------------------------------------
	std::string temp = backup;					//Put backup filename in temp
	temp.append("x");						//Add x

	rename(backup.c_str(),temp.c_str());			//rename backup file to temp file
	rename(lastSavestateMade.c_str(),backup.c_str());	//rename current as backup
	rename(temp.c_str(),lastSavestateMade.c_str());		//rename backup as current

	undoSS = true;	//Just in case, if this was run, then there is definately a last savestate and backup
	if (redoSS)				//This was a redo function, so if run again it will be an undo again
		redoSS = false;
	else					//This was an undo function so next will be redo, so flag it
		redoSS = true;

	FCEU_DispMessage("%s restored",0,backup.c_str());
	FCEU_printf("%s restored\n",backup.c_str());
}

//------------------------------------------------------------------------------------------------------------------------------------------------------
//*************************************************************************
//Loadstate backup functions
//(Used when Loading savestates)
//*************************************************************************

std::string GetBackupFileName()
{
	//This backup savestate is a special one specifically made whenever a loadstate occurs so that the user's place in a movie/game is never lost
	//particularly from unintentional loadstating
	std::string filename;
	int x;

	filename = FCEU_MakeFName(FCEUMKF_STATE,CurrentState,0);	//Generate normal savestate filename
	x = filename.find_last_of(".");		//Find last dot
	filename = filename.substr(0,x);	//Chop off file extension
	filename.append(".bak.fc0");		//add .bak

	return filename;
}

bool CheckBackupSaveStateExist()
{
	//This function simply checks to see if the backup loadstate exists, the backup loadstate is a special savestate
	//That is made before loading any state, so that the user never loses his data
	std::string filename = GetBackupFileName(); //Get backup savestate filename

	//Check if this filename exists
	std::fstream test;
	test.open(filename.c_str(),std::fstream::in);

	if (test.fail())
	{
		test.close();
		return false;
	}
	else
	{
		test.close();
		return true;
	}
}

void BackupLoadState()
{
	std::string filename = GetBackupFileName();
	internalSaveLoad = true;
	FCEUSS_Save(filename.c_str());
	internalSaveLoad = false;
	undoLS = true;
}

void LoadBackup()
{
	if (!undoLS) return;
	std::string filename = GetBackupFileName();	//Get backup filename
	if (CheckBackupSaveStateExist())
	{
		//internalSaveLoad = true;
		FCEUSS_Load(filename.c_str());		//Load it
		//internalSaveLoad = false;
		redoLS = true;						//Flag redoLoadState
		undoLS = false;						//Flag that LoadBackup cannot be run again
	}
	else
		FCEU_DispMessage("Error: Could not load %s",0,filename.c_str());
}

void RedoLoadState()
{
	if (!redoLS) return;
	if (!lastLoadstateMade.empty() && redoLS)
	{
		FCEUSS_Load(lastLoadstateMade.c_str());
		FCEU_printf("Redoing %s\n",lastLoadstateMade.c_str());
	}
	redoLS = false;		//Flag that RedoLoadState can not be run again
	undoLS = true;		//Flag that LoadBackup can be run again
}

//-----------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------
//----------- Save State History ----------------
//-----------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------
static StateRecorderConfigData stateRecorderConfig;

class StateRecorder
{
	public:
		StateRecorder(void)
		{
			loadConfig( stateRecorderConfig );

			for (int i=0; i<ringBufSize; i++)
			{
				EMUFILE_MEMORY *em = new EMUFILE_MEMORY( 0x1000 );

				ringBuf.push_back(em);
			}
			ringStart = ringHead = ringTail = 0;
			frameCounter = 0;
			lastState = ringHead;
			loadIndexReset = false;
			lastLoadFrame = 0;
		}

		~StateRecorder(void)
		{
			for (size_t i=0; i<ringBuf.size(); i++)
			{
				delete ringBuf[i];
			}
			ringBuf.clear();
		}

		void loadConfig( StateRecorderConfigData &config )
		{
			if (config.framesBetweenSnaps < 1)
			{
				config.framesBetweenSnaps = 1;
			}
			if (config.timeBetweenSnapsMinutes < 0.0)
			{
				config.timeBetweenSnapsMinutes = 3.0f / 60.0f;
			}
			if (config.timeBetweenSnapsMinutes > config.historyDurationMinutes)
			{
				config.historyDurationMinutes = config.timeBetweenSnapsMinutes;
			}

			if (config.timingMode)
			{
				const double fhistMin  = config.historyDurationMinutes;
				const double fsnapMin  = config.timeBetweenSnapsMinutes;
				const double fnumSnaps = fhistMin / fsnapMin;

				ringBufSize = static_cast<int>( fnumSnaps + 0.5f );

				int32_t fps = fceu11::GetDesiredFPS(); // Do >> 24 to get in Hz

				double hz = ( ((double)fps) / 16777216.0 );

				double framesPerSnapf = hz * fsnapMin * 60.0;

				framesPerSnap = static_cast<unsigned int>( framesPerSnapf + 0.50 );
			}
			else
			{
				const double fhistMin  = config.historyDurationMinutes;
				int32_t fps = fceu11::GetDesiredFPS(); // Do >> 24 to get in Hz
				double hz = ( ((double)fps) / 16777216.0 );

				const double fsnapMin  = static_cast<double>(config.framesBetweenSnaps) / (hz * 60.0);
				const double fnumSnaps = fhistMin / fsnapMin;

				ringBufSize = static_cast<int>( fnumSnaps + 0.5f );
				framesPerSnap = config.framesBetweenSnaps;
			}

			printf("ringBufSize:%i  framesPerSnap:%i\n", ringBufSize, framesPerSnap );

			compressionLevel = config.compressionLevel;
			loadPauseTime    = config.loadPauseTimeSeconds;
			pauseOnLoad      = config.pauseOnLoad;
		}

		void update(void)
		{
			bool isPaused = EmulationPaused ? true : false;

			unsigned int curFrame = static_cast<unsigned int>(currFrameCounter);

			if (!isPaused && loadIndexReset)
			{
				ringHead = (lastState + 1) % ringBufSize;

				frameCounter = curFrame;

				loadIndexReset = false;
			}

			if (!isPaused && (curFrame > frameCounter) )
			{
				frameCounter = curFrame;

				if ( (frameCounter % framesPerSnap) == 0 )
				{
					EMUFILE_MEMORY *em = ringBuf[ ringHead ];

					em->set_len(0);

					FCEUSS_SaveMS( em, compressionLevel );

					//printf("Frame:%u  Save:%i  Size:%zu  Total:%zukB \n", frameCounter, ringHead, em->size(), dataSize() / 1024 );

					lastState = ringHead;

					ringHead = (ringHead + 1) % ringBufSize;

					if (ringStart == ringHead)
					{
						ringStart = (ringHead + 1) % ringBufSize;
					}
				}
			}
		}

		int loadStateRelativeToEnd( int numSnapsFromLatest )
		{
			if (numSnapsFromLatest < 0)
			{
				numSnapsFromLatest = 0;
			}
			numSnapsFromLatest = numSnapsFromLatest % ringBufSize;

			int snapIdx = ringHead - numSnapsFromLatest - 1;

			loadStateByIndex(snapIdx);

			return 0;
		}

		int loadStateByIndex( int snapIdx )
		{
			if (snapIdx < 0)
			{
				snapIdx = snapIdx + ringBufSize;
			}
			snapIdx = snapIdx % ringBufSize;

			EMUFILE_MEMORY *em = ringBuf[ snapIdx ];

			em->fseek(SEEK_SET, 0);

			FCEUSS_LoadFP( em, SSLOADPARAM_NOBACKUP );

			frameCounter = lastLoadFrame = static_cast<unsigned int>(currFrameCounter);

			lastState = snapIdx;
			loadIndexReset = true;

			if (pauseOnLoad == StateRecorderConfigData::TEMPORARY_PAUSE)
			{
				if (loadPauseTime > 0)
				{	// Temporary pause after loading new state for user to have time to process
					fceu11::PauseForDuration(loadPauseTime);
				}
			}
			else if (pauseOnLoad == StateRecorderConfigData::FULL_PAUSE)
			{
				fceu11::SetEmulationPaused( EMULATIONPAUSED_PAUSED );
			}
			return 0;
		}

		int loadPrevState(void)
		{
			int snapIdx = lastState;

			if ( lastState == ringHead )
			{	// No States to Load
				return -1;
			}
			if ( lastState != ringStart )
			{
				if ( (lastLoadFrame+30) > frameCounter)
				{
					snapIdx--;

					if (snapIdx < 0)
					{
						snapIdx += ringBufSize;
					}
				}
			}
			return loadStateByIndex( snapIdx );
		}

		int loadNextState(void)
		{
			int snapIdx =  lastState;
			int nextIdx = (lastState + 1) % ringBufSize;

			if ( nextIdx != ringHead )
			{
				snapIdx = nextIdx;
			}
			return loadStateByIndex( snapIdx );
		}

		int getHeadIndex(void)
		{
			return ringHead;
		}

		int getStartIndex(void)
		{
			return ringStart;
		}

		int numSnapsSaved(void)
		{
			int numSnaps = ringHead - ringStart;

			if (numSnaps < 0)
			{
				numSnaps = numSnaps + static_cast<int>( ringBuf.size() );
			}
			return numSnaps;
		}

		size_t  dataSize(void)
		{
			return ringBuf.size() * ringBuf[0]->size();
		}

		size_t  ringBufferSize(void)
		{
			return ringBuf.size();
		}
		static bool enabled;
		static int  lastState;
	private:

		void doSnap(void)
		{

		}

		std::vector <EMUFILE_MEMORY*> ringBuf;
		int  ringHead;
		int  ringTail;
		int  ringStart;
		int  ringBufSize;
		int  compressionLevel;
		int  loadPauseTime;
		StateRecorderConfigData::PauseType pauseOnLoad;
		unsigned int frameCounter;
		unsigned int framesPerSnap;
		unsigned int lastLoadFrame;
		bool loadIndexReset;

};

static StateRecorder *stateRecorder = nullptr;
bool StateRecorder::enabled = false;
int StateRecorder::lastState = 0;

int FCEU_StateRecorderStart(void)
{
	if (stateRecorder == nullptr)
	{
		stateRecorder = new StateRecorder();
	}
	return stateRecorder == nullptr;
}

int FCEU_StateRecorderStop(void)
{
	if (stateRecorder != nullptr)
	{
		delete stateRecorder; stateRecorder = nullptr;
	}
	return stateRecorder != nullptr;
}

int FCEU_StateRecorderUpdate(void)
{
	if (stateRecorder != nullptr)
	{
		stateRecorder->update();
	}
	return 0;
}

bool FCEU_StateRecorderIsEnabled(void)
{
	return StateRecorder::enabled;
}

void FCEU_StateRecorderSetEnabled(bool enabled)
{
	StateRecorder::enabled = enabled;
}

bool FCEU_StateRecorderRunning(void)
{
	return stateRecorder != nullptr;
}

int FCEU_StateRecorderGetMaxSnaps(void)
{
	int size = 0;

	if (stateRecorder != nullptr)
	{
		size = stateRecorder->ringBufferSize();
	}
	return size;
}

int FCEU_StateRecorderGetNumSnapsSaved(void)
{
	int n = 0;

	if (stateRecorder != nullptr)
	{
		n = stateRecorder->numSnapsSaved();
	}
	return n;
}

int FCEU_StateRecorderLoadState(int snapIndex)
{
	int ret = -1;

	if (stateRecorder != nullptr)
	{
		ret = stateRecorder->loadStateByIndex(snapIndex);
	}
	return ret;
}

int FCEU_StateRecorderGetStateIndex(void)
{
	return StateRecorder::lastState;
}

int FCEU_StateRecorderLoadPrevState(void)
{
	int ret = -1;

	if (stateRecorder != nullptr)
	{
		ret = stateRecorder->loadPrevState();
	}
	return ret;
}

int FCEU_StateRecorderLoadNextState(void)
{
	int ret = -1;

	if (stateRecorder != nullptr)
	{
		ret = stateRecorder->loadNextState();
	}
	return ret;
}

const StateRecorderConfigData& FCEU_StateRecorderGetConfigData(void)
{
	return stateRecorderConfig;
}
int FCEU_StateRecorderSetConfigData(const StateRecorderConfigData &newConfig)
{
	stateRecorderConfig = newConfig;

	if (stateRecorder != nullptr)
	{
		stateRecorder->loadConfig( stateRecorderConfig );
	}
	return 0;
}
