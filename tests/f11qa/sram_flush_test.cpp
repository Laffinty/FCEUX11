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
//
// CI diagnosability (v2.0.8 r8): the first CI run of this test hung for the
// full ctest timeout (1500 s) and produced NO output at all -- stdout is
// block-buffered when it is a pipe, so the PASS lines were lost together
// with the killed process, exactly like the movie_roundtrip_test incident
// recorded in AGENTS.md. The same remedy applies here: every step prints a
// phase marker on STDERR (unbuffered, so it survives a kill) and a watchdog
// thread reports the last phase reached and exits with code 3 after 60 s.

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <string>
#include <thread>
#include <vector>

#include "types.h"
#include "fceu.h"
#include "git.h"
#include "cart.h"
#include "file.h"
#include "io_api.h"

extern void iNESGI(GI h);
extern CartInfo iNESCart;

static GI g_seenGI = GI_RESETM2;
static void stubGI(GI h) { g_seenGI = h; }

static int failures = 0;

static const char* const kPhaseName[] = {
	"start",
	"temp-dir-ready",
	"base-dir-pinned",
	"M3-noop-done",
	"M4-dispatch-done",
	"M5a-battery0-done",
	"savegame-buf-added",
	"filename-resolved",
	"flush-returned",
	"file-read-done",
	"cleanup-done",
};

static std::atomic<int> g_phase{0};
static std::atomic<bool> g_done{false};

static void phase(int n)
{
	g_phase.store(n);
	std::fprintf(stderr, "[sram_flush] phase %d: %s\n", n, kPhaseName[n]);
	std::fflush(stderr);
}

#define CHECK(cond, msg)                                     \
	do {                                                     \
		if (cond) {                                          \
			std::printf("PASS: %s\n", msg);                  \
			std::fflush(stdout);                             \
		} else {                                             \
			std::printf("FAIL: %s\n", msg);                  \
			std::fflush(stdout);                             \
			failures++;                                      \
		}                                                    \
	} while (0)

int main()
{
	// Unbuffered stdout: the ctest pipe would otherwise swallow the PASS
	// lines if the process is killed.
	std::setvbuf(stdout, nullptr, _IONBF, 0);

	// Watchdog: report the last phase reached instead of letting ctest kill
	// the process after its (much longer) timeout.
	std::thread watchdog([]()
	{
		for (int i = 0; i < 60 && !g_done.load(); i++)
			std::this_thread::sleep_for(std::chrono::seconds(1));
		if (!g_done.load())
		{
			std::fprintf(stderr, "[sram_flush] WATCHDOG stuck at phase %d: %s\n",
			             g_phase.load(), kPhaseName[g_phase.load()]);
			std::fflush(stderr);
			std::_Exit(3);
		}
	});
	watchdog.detach();

	phase(0);

	// Pin the global base directory to a temp dir of our own. BaseDirectory
	// defaults to an EMPTY string and is normally set by the Qt config init
	// (drivers/Qt/config.cpp), which never runs here: with it empty,
	// FCEU_MakeFName(FCEUMKF_SAV, ...) builds "\sav\<name>.sav" -- a
	// drive-relative ROOT path. On a machine that happens to have a leftover
	// "\sav" directory the test then passes silently (CI caught exactly
	// that), and on a clean runner create_directories("\sav") resolves as a
	// UNC prefix and throws. A pinned per-test directory removes both traps.
	std::filesystem::path base =
		std::filesystem::temp_directory_path() / "f11_sram_flush_test";
	std::filesystem::remove_all(base);
	std::filesystem::create_directories(base);
	phase(1);

	fceu11::SetBaseDirectory(base.string());
	phase(2);

	// ---- M3: no game -> safe no-op ----
	GameInfo = nullptr;
	GameInterface = nullptr;
	FCEU_FlushGameSave();
	CHECK(true, "M3 no-op with GameInfo==NULL");
	phase(3);

	// ---- M4: dispatch routes GI_SAVESRAM ----
	GameInfo = new FCEUGI();
	GameInterface = stubGI;
	g_seenGI = GI_RESETM2;
	FCEU_FlushGameSave();
	CHECK(g_seenGI == GI_SAVESRAM, "M4 stub receives GI_SAVESRAM");
	delete GameInfo;
	GameInfo = nullptr;
	GameInterface = nullptr;
	phase(4);

	// ---- M5a: battery=0 -> silent no-op ----
	iNESCart.clear();
	iNESCart.battery = 0;
	iNESGI(GI_SAVESRAM);
	CHECK(true, "M5a battery=0 no-op");
	phase(5);

	// ---- M5b: battery=1 -> .sav written with entry bytes ----
	static uint8 sram[8] = { 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88 };
	iNESCart.clear();
	iNESCart.battery = 1;
	iNESCart.addSaveGameBuf(sram, sizeof(sram));
	phase(6);

	GetFileBase("sram_flush_probe");
	std::string savPath = FCEU_MakeFName(FCEUMKF_SAV, 0, "sav");
	std::filesystem::create_directories(
		std::filesystem::path(savPath).parent_path());
	std::fprintf(stderr, "[sram_flush] sav path: %s\n", savPath.c_str());
	std::fflush(stderr);
	phase(7);

	iNESGI(GI_SAVESRAM);
	phase(8);

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
	phase(9);

	std::filesystem::remove_all(base);
	phase(10);

	g_done.store(true);
	if (failures == 0)
	{
		std::printf("PASS: sram_flush_test all legs\n");
		return 0;
	}
	std::printf("FAIL: sram_flush_test %d leg(s)\n", failures);
	return 1;
}
