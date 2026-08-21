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

// v0.2.24.1 (follow-up to v0.2.24): all cheat-list and cheat-search state
// is now owned by Rust (fceux11-debug::cheat). This C++ file retains only
// the parts that must call into x6502 (SetReadHandler / GetReadHandler) or
// the platform-native file I/O (FCEU_MakeFName + FCEUD_UTF8fopen):
//   * RebuildSubCheats     �?installs/removes read handlers per cheat
//   * SubCheatsRead        �?the read handler itself
//   * FCEU_ApplyPeriodicCheats �?pokes RAM via CheatRPtrs
//   * FCEU_CheatGetByte/SetByte �?go through ARead/BWrite
//   * FCEU_LoadGameCheats / SaveGameCheats / FlushGameCheats �?file I/O
//   * FCEU_CheatResetRAM / FCEU_CheatAddRAM �?the CheatRPtrs translation array
//
// Everything else delegates to fceux11_rust_cheat_*.

#include "types.h"
#include "utils/safe_string.h"
#include "cpu.h"
#include "cheat.h"
#include "fceu.h"
#include "file.h"
#include "cart.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "utils/memory.h"

#include "rust/fceux11_rust.h"

#include <string>
#include <cstdlib>
#include <cstring>
#include <cstdio>
#include <cctype>

// v1.2 Census §2.3: `using namespace std` removed; all std types below
// are explicitly qualified with std::.

static uint8 *CheatRPtrs[64];

std::vector<uint16> FrozenAddresses;			//List of addresses that are currently frozen
unsigned int FrozenAddressCount = 0;		//Keeps up with the Frozen address count, necessary for using in other dialogs (such as hex editor)

void FCEU_CheatResetRAM(void)
{
	int x;

	for(x=0;x<64;x++)
		CheatRPtrs[x]=0;
}

void FCEU_CheatAddRAM(int s, uint32 A, uint8 *p)
{
	uint32 AB=A>>10;
	// hotfix1 P0-7 (H-07): CheatRPtrs is a fixed 64-entry array (see reset
	// above at src/cheat.cpp:67-68). When A>>10 + (s-1) exceeds 63, the
	// unguarded loop walked past the end and corrupted unrelated globals.
	// AB is the start index; s is the byte count to register. Reject
	// out-of-range ranges rather than silently spilling into other memory.
	if (s <= 0 || AB >= 64 || AB + (uint32)s > 64) {
		return;
	}

	for (int32 x = (int32)(s - 1); x >= 0; x--)
		CheatRPtrs[AB + x] = p - A;
}


CHEATF_SUBFAST SubCheats[256];
uint32 numsubcheats = 0;
int globalCheatDisabled = 0;
int disableAutoLSCheats = 0;
bool disableShowGG = 0;
// v0.2.24: cheat-map storage moved to Rust (fceux11_rust_cheat_map_*).
// This pointer is repurposed as a presence sentinel �?non-null when the
// Rust-side buffer is allocated, NULL otherwise. The cheat code never
// dereferences it (all reads/writes go through the FFI).
static _8BYTECHEATMAP* cheatMap = NULL;
static _8BYTECHEATMAP cheatMapSentinel = 0;  // dummy storage for the sentinel

// v0.2.24.1: legacy `cheats` / `cheatsl` linked-list globals are GONE.
// The list now lives in fceux11-debug::cheat (Vec<CheatEntry>).
// Anything that needs to walk the list iterates via fceux11_rust_cheat_count
// + fceux11_rust_cheat_get(i, &view).

int savecheats = 0;

static DECLFR(SubCheatsRead)
{
	CHEATF_SUBFAST *s = SubCheats;
	int x=numsubcheats;

	do
	{
		if(s->addr==A)
		{
			if(s->compare>=0)
			{
				uint8 pv=s->PrevRead(A);

				if(pv==s->compare)
					return(s->val);
				else return(pv);
			}
			else return(s->val);
		}
		s++;
	} while(--x);
	return(0);	/* We should never get here. */
}

void RebuildSubCheats(void)
{
	uint32 x;
	for (x = 0; x < numsubcheats; x++)
	{
		SetReadHandler(SubCheats[x].addr, SubCheats[x].addr, SubCheats[x].PrevRead);
		if (cheatMap)
			fceu11::SetCheatMapByte(SubCheats[x].addr, false);
	}

	numsubcheats = 0;

	if (!globalCheatDisabled)
	{
		// v0.2.24.1: iterate the Rust-owned cheat list by index.
		uint32 count = fceux11_rust_cheat_count();
		FceuCheatEntryView view;
		for (uint32 i = 0; i < count; ++i)
		{
			if (!fceux11_rust_cheat_get(i, &view))
				continue;
			if (view.type_ == 1 && view.status && GetReadHandler((uint16)view.addr) != SubCheatsRead)
			{
				// hotfix1 P0-8 (H-08): SubCheats has a hard cap of 256 (see
				// declaration at cheat.cpp:81). Without this guard a malformed
				// or unusually large cheat list would silently clobber memory
				// past the array. Drop further entries when the table is full.
				if (numsubcheats >= 256) {
					continue;
				}
				SubCheats[numsubcheats].PrevRead = GetReadHandler((uint16)view.addr);
				SubCheats[numsubcheats].addr = (uint16)view.addr;
				SubCheats[numsubcheats].val = view.val;
				SubCheats[numsubcheats].compare = view.compare;
				SetReadHandler((uint16)view.addr, (uint16)view.addr, SubCheatsRead);
				if (cheatMap)
					fceu11::SetCheatMapByte(SubCheats[numsubcheats].addr, true);
				numsubcheats++;
			}
		}
	}
	FrozenAddressCount = numsubcheats;		//Update the frozen address list
}

void FCEU_PowerCheats()
{
	numsubcheats = 0;	/* Quick hack to prevent setting of ancient read addresses. */
	if (cheatMap)
		fceu11::RefreshCheatMap();
	RebuildSubCheats();
}

int FCEU_CalcCheatAffectedBytes(uint32 address, uint32 size) {
	// v0.2.24: implementation migrated to Rust.
	if (!cheatMap)
		return 0;
	return (int)fceux11_rust_cheat_map_count_affected(address, size);
}

static void CheatMemErr(void)
{
	FCEUD_PrintError("Error allocating memory for cheat data.");
}

/* The "override_existing" parameter is used only in cheat dialog import.
   Since the default behaviour will reset numsubcheats to 0 everytime,
   In game loading, this is absolutely right, but when importing in cheat window,
   resetting numsubcheats to 0 will override existed cheat items to make them
   invalid.
*/
void FCEU_LoadGameCheats(FILE *override, int override_existing)
{
	FILE *fp;
	unsigned int addr;
	unsigned int val;
	unsigned int status;
	unsigned int type;
	unsigned int compare;
	int x;

	char linebuf[2048] = { 0 };
	char namebuf[128] = { 0 };
	int tc = 0;
	std::string fn;

	if (override_existing)
	{
		numsubcheats = 0;
		if (cheatMap)
			fceu11::RefreshCheatMap();
	}

	if(override)
		fp = override;
	else
	{
		fn = FCEU_MakeFName(FCEUMKF_CHEAT, 0, 0);
		fp = FCEUD_UTF8fopen(fn.c_str(), "rb");
		if (!fp) {
			return;
		}
	}

	while(fgets(linebuf, 2048, fp) != nullptr)
	{
		char *tbuf = linebuf;
		int doc = 0;

		addr = val = compare = status = type = 0;

		if(tbuf[0] == 'S')
		{
			tbuf++;
			type = 1;
		}
		else
			type = 0;

		if(tbuf[0] == 'C')
		{
			tbuf++;
			doc = 1;
		}

		if(tbuf[0] == ':')
		{
			tbuf++;
			status = 0;
		}
		else status = 1;

		if(doc)
		{
			char *neo = &tbuf[4+2+2+1+1+1];
			if(sscanf(tbuf, "%04x%*[:]%02x%*[:]%02x", &addr, &val, &compare) != 3)
				continue;
			FCEU_strlcpy(namebuf, sizeof(namebuf), neo);
		}
		else
		{
			char *neo = &tbuf[4+2+1+1];
			if(sscanf(tbuf, "%04x%*[:]%02x", &addr, &val) != 2)
				continue;
			FCEU_strlcpy(namebuf, sizeof(namebuf), neo);
		}

		for(x = 0; x < (int)strlen(namebuf); x++)
		{
			if(namebuf[x] == 10 || namebuf[x] == 13)
			{
				namebuf[x] = 0;
				break;
			}
			else if(namebuf[x] > 0x00 && namebuf[x] < 0x20)
				namebuf[x] = 0x20;
		}

		// v0.2.24.1: storage moved to Rust.
		fceux11_rust_cheat_add(namebuf, addr, (uint8)val,
			doc ? (int)compare : -1, (int)status, (int)type);
		tc++;
	}

	RebuildSubCheats();

	FCEU_DispMessage("Cheats file loaded.", 0); //Tells user a cheats file was loaded.

	if(!override)
		fclose(fp);
}

void FCEU_SaveGameCheats(FILE* fp, int release)
{
	// v0.2.24.1: iterate the Rust-owned list, write each entry to the file
	// in the same legacy format as the original C++ implementation.
	uint32 count = fceux11_rust_cheat_count();
	FceuCheatEntryView view;
	for (uint32 i = 0; i < count; ++i)
	{
		if (!fceux11_rust_cheat_get(i, &view))
			continue;
		if (view.type_)
			fputc('S', fp);
		if (view.compare >= 0)
			fputc('C', fp);
		if (!view.status)
			fputc(':', fp);

		const char* name = view.name_ptr ? view.name_ptr : "";
		if (view.compare >= 0)
			fprintf(fp, "%04x:%02x:%02x:%s\n", view.addr, view.val, view.compare, name);
		else
			fprintf(fp, "%04x:%02x:%s\n", view.addr, view.val, name);
	}
	if (release)
		fceux11_rust_cheat_delete_all();
}

void FCEU_FlushGameCheats(FILE *override, int nosave)
{
	// v0.2.24.1: CheatComp lives in Rust now.
	fceux11_rust_cheat_comp_release();

	if((!savecheats || nosave) && !override)	/* Always save cheats if we're being overridden. */
	{
		fceux11_rust_cheat_delete_all();
	}
	else
	{
		std::string fn;

		if(!override)
			fn = FCEU_MakeFName(FCEUMKF_CHEAT,0,0);

		if(fceux11_rust_cheat_count() > 0)
		{
			FILE *fp;

			if(override)
				fp = override;
			else
				fp=FCEUD_UTF8fopen(fn.c_str(),"wb");

			if(fp)
			{
				FCEU_SaveGameCheats(fp, 1);
				if(!override)
					fclose(fp);
			}
			else
				FCEUD_PrintError("Error saving cheats.");
			fceux11_rust_cheat_delete_all();
		}
		else if(!override)
			remove(fn.c_str());
	}

	RebuildSubCheats();  /* Remove memory handlers. */
}


int fceu11::AddCheat(const char *name, uint32 addr, uint8 val, int compare, int type)
{
	// v0.2.24.1: storage moved to Rust.
	fceux11_rust_cheat_add(name, addr, val, compare, 1, type);
	savecheats = 1;
	RebuildSubCheats();

	return 1;
}

int fceu11::DelCheat(uint32 which)
{
	// v0.2.24.1: storage moved to Rust.
	if (!fceux11_rust_cheat_delete(which))
		return 0;
	savecheats = 1;
	RebuildSubCheats();
	return 1;
}

void FCEU_ApplyPeriodicCheats(void)
{
	// v0.2.24.1: iterate the Rust-owned cheat list.
	uint32 count = fceux11_rust_cheat_count();
	if (count == 0)
		return;
	FceuCheatEntryView view;
	for (uint32 i = 0; i < count; ++i)
	{
		if (!fceux11_rust_cheat_get(i, &view))
			continue;
		if (view.status && !view.type_)
		{
			if (CheatRPtrs[view.addr >> 10])
				CheatRPtrs[view.addr >> 10][view.addr] = view.val;
		}
	}
}


void fceu11::ListCheats(int (*callb)(const char *name, uint32 a, uint8 v, int compare, int s, int type, void *data), void *data)
{
	// v0.2.24.1: iterate the Rust-owned cheat list.
	uint32 count = fceux11_rust_cheat_count();
	FceuCheatEntryView view;
	for (uint32 i = 0; i < count; ++i)
	{
		if (!fceux11_rust_cheat_get(i, &view))
			continue;
		const char* name = view.name_ptr ? view.name_ptr : "";
		if (!callb(name, view.addr, view.val, view.compare, view.status, view.type_, data))
			break;
	}
}

int fceu11::GetCheat(uint32 which, std::string *name, uint32 *a, uint8 *v, int *compare, int *s, int *type)
{
	// v0.2.24.1: read from Rust-owned cheat list.
	FceuCheatEntryView view;
	if (!fceux11_rust_cheat_get(which, &view))
		return 0;
	if (name)
		*name = view.name_ptr ? view.name_ptr : "";
	if (a)
		*a = view.addr;
	if (v)
		*v = view.val;
	if (s)
		*s = view.status;
	if (compare)
		*compare = view.compare;
	if (type)
		*type = view.type_;
	return 1;
}

/* Returns 1 on success, 0 on failure. Sets *a,*v,*c. */
int fceu11::DecodeGG(const char *str, int *a, int *v, int *c)
{
	// v0.2.24: pure-computation algorithm migrated to Rust.
	// (The legacy `GGtobin` lookup table now lives inside the Rust decoder.)
	return fceux11_rust_cheat_decode_gg(str, a, v, c);
}

int fceu11::DecodePAR(const char *str, int *a, int *v, int *c, int *type)
{
	// v0.2.24: pure-computation algorithm migrated to Rust.
	return fceux11_rust_cheat_decode_par(str, a, v, c, type);
}

/* name can be NULL if the name isn't going to be changed. */
/* same goes for a, v, and s(except the values of each one must be <0) */

int fceu11::SetCheat(uint32 which, const std::string *name, int32 a, int32 v, int c, int s, int type)
{
	// v0.2.24.1: storage moved to Rust.
	const char* namePtr = name ? name->c_str() : nullptr;
	if (!fceux11_rust_cheat_set(which, namePtr, a, v, c, s, type))
		return 0;
	savecheats = 1;
	RebuildSubCheats();
	return 1;
}

/* Convenience function. */
int fceu11::ToggleCheat(uint32 which)
{
	// v0.2.24.1: storage moved to Rust.
	int new_status = fceux11_rust_cheat_toggle(which);
	if (new_status < 0)
		return -1;
	savecheats = 1;
	RebuildSubCheats();
	return new_status;
}

int fceu11::GlobalToggleCheat(int global_enabled)
{
	unsigned int _numsubcheats = numsubcheats;
	globalCheatDisabled = !global_enabled;
	RebuildSubCheats();
	return _numsubcheats != numsubcheats;
}

// ---------------------------------------------------------------------------
// Cheat search �?all storage in Rust; C++ builds the memory snapshots that
// Rust needs to evaluate the comparators.
// ---------------------------------------------------------------------------

namespace {
// Snapshot helpers �?populate `mem` from CheatRPtrs (current memory) and
// `pres` from the presence of a CheatRPtrs entry (1 = real RAM, 0 = absent).
void buildMemorySnapshot(uint8* mem, uint8* pres)
{
	for (uint32 x = 0; x < 0x10000; ++x)
	{
		if (CheatRPtrs[x >> 10])
		{
			mem[x] = CheatRPtrs[x >> 10][x];
			pres[x] = 1;
		}
		else
		{
			mem[x] = 0;
			pres[x] = 0;
		}
	}
}
} // namespace

void fceu11::CheatSearchSetCurrentAsOriginal(void)
{
	// v0.2.24.1: search snapshot lives in Rust.
	static uint8 mem[0x10000];
	static uint8 pres[0x10000];
	buildMemorySnapshot(mem, pres);
	if (!fceux11_rust_cheat_comp_exists())
	{
		if (!fceux11_rust_cheat_comp_init())
		{
			CheatMemErr();
			return;
		}
	}
	fceux11_rust_cheat_comp_set_current_as_original(mem, pres);
}

void fceu11::CheatSearchShowExcluded(void)
{
	fceux11_rust_cheat_comp_show_excluded();
}


int32 fceu11::CheatSearchGetCount(void)
{
	static uint8 mem[0x10000];
	static uint8 pres[0x10000];
	buildMemorySnapshot(mem, pres);
	return fceux11_rust_cheat_comp_count(pres);
}

/* This function will give the initial value of the search and the current value at a location. */
void fceu11::CheatSearchGet(int (*callb)(uint32 a, uint8 last, uint8 current, void *data),void *data)
{
	if (!fceux11_rust_cheat_comp_exists())
	{
		if (!fceux11_rust_cheat_comp_init())
			CheatMemErr();
		return;
	}
	for (uint32 x = 0; x < 0x10000; ++x)
	{
		uint32 slot = fceux11_rust_cheat_comp_get(x);
		// `slot` is uint16 logically; high bits encode CHEATC_NONE / EXCLUDED.
		if ((slot & 0xC000) == 0 && CheatRPtrs[x >> 10])
		{
			if (!callb(x, (uint8)slot, CheatRPtrs[x >> 10][x], data))
				break;
		}
	}
}

void fceu11::CheatSearchGetRange(uint32 first, uint32 last, int (*callb)(uint32 a, uint8 last, uint8 current))
{
	if (!fceux11_rust_cheat_comp_exists())
	{
		if (!fceux11_rust_cheat_comp_init())
			CheatMemErr();
		return;
	}
	uint32 in = 0;
	for (uint32 x = 0; x < 0x10000; ++x)
	{
		uint32 slot = fceux11_rust_cheat_comp_get(x);
		if ((slot & 0xC000) == 0 && CheatRPtrs[x >> 10])
		{
			if (in >= first)
				if (!callb(x, (uint8)slot, CheatRPtrs[x >> 10][x]))
					break;
			in++;
			if (in > last)
				return;
		}
	}
}

void fceu11::CheatSearchBegin(void)
{
	static uint8 mem[0x10000];
	static uint8 pres[0x10000];
	buildMemorySnapshot(mem, pres);
	if (!fceux11_rust_cheat_comp_search_begin(mem, pres))
	{
		CheatMemErr();
		return;
	}
}


void fceu11::CheatSearchEnd(int type, uint8 v1, uint8 v2)
{
	static uint8 mem[0x10000];
	static uint8 pres[0x10000];
	buildMemorySnapshot(mem, pres);
	if (!fceux11_rust_cheat_comp_exists())
	{
		if (!fceux11_rust_cheat_comp_init())
		{
			CheatMemErr();
			return;
		}
	}
	fceux11_rust_cheat_comp_search_end(type, v1, v2, mem, pres);
}

int FCEU_CheatGetByte(uint32 A)
{
	if(A < 0x10000) {
		uint32 ret;
		fceuindbg=1;
		ret = fceu11::g_bus.read(static_cast<uint16_t>(A));
		fceuindbg=0;
		return ret;
	} else
		return 0;
}

void FCEU_CheatSetByte(uint32 A, uint8 V)
{
   if(CheatRPtrs[A>>10])
    CheatRPtrs[A>>10][A]=V;
   else if(A < 0x10000)
    fceu11::g_bus.write(static_cast<uint16_t>(A), V);
}

// disable all cheats
int FCEU_DisableAllCheats(void)
{
	// v0.2.24.1: storage moved to Rust.
	int count = fceux11_rust_cheat_disable_all();
	savecheats = 1;
	RebuildSubCheats();
	return count;
}

// delete all cheats
int FCEU_DeleteAllCheats(void)
{
	// v0.2.24.1: storage moved to Rust.
	fceux11_rust_cheat_delete_all();
	savecheats = 1;
	RebuildSubCheats();
	return 0;
}

int fceu11::FindCheatMapByte(uint16 address)
{
	// v0.2.24: bit storage migrated to Rust.
	return fceux11_rust_cheat_map_find(address);
}

void fceu11::SetCheatMapByte(uint16 address, bool cheat)
{
	// v0.2.24: bit storage migrated to Rust. The original C++ ternary is
	// `cheat ? OR : XOR` �?preserved exactly inside the Rust impl.
	fceux11_rust_cheat_map_set(address, cheat ? 1 : 0);
}

void fceu11::CreateCheatMap(void)
{
	// v0.2.24: bit storage migrated to Rust. We keep `cheatMap` as a non-null
	// presence sentinel so existing `if (cheatMap)` guards still work.
	if (!cheatMap)
	{
		fceux11_rust_cheat_map_create();
		cheatMap = &cheatMapSentinel;
	}
	fceu11::RefreshCheatMap();
}

void fceu11::RefreshCheatMap(void)
{
	// v0.2.24: zero the Rust-side buffer, then re-mark currently-active
	// substitute cheats (same as the original C++ implementation).
	fceux11_rust_cheat_map_refresh_clear();
	for (uint32 i = 0; i < numsubcheats; ++i)
		fceu11::SetCheatMapByte(SubCheats[i].addr, true);
}

void fceu11::ReleaseCheatMap()
{
	if (cheatMap)
	{
		fceux11_rust_cheat_map_release();
		cheatMap = NULL;
	}
}
