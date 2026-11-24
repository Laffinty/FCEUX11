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

// v2.0.8 C2/C3 O2 -- diagnostics report lock (plan leg O2a).
//
// The report must be gatherable in every state the emulator can be in --
// most importantly with NO game loaded (and in a GBA session, where
// GameInfo is null by design). It must carry the version and the base
// directory, and its text must not be empty (a report that says nothing
// would be a toothless gate).

#include <cstdio>
#include <string>

#include "io_api.h"
#include "Qt/diagnostics_report.h"

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
	std::setvbuf(stdout, nullptr, _IONBF, 0);

	// ---- O2a: gathering with no game loaded ----
	// The base directory is set by the GUI's InitConfig(), which never runs
	// in a headless test -- so the test sets one itself and then checks
	// that the report actually carries THAT directory (which proves the
	// report reads the base directory instead of, say, the working
	// directory).
	const std::string testBase = "D:\\fceux11_diag_test_base";
	fceu11::SetBaseDirectory(testBase);

	const fceux11::DiagnosticsReport report = fceux11::gather_diagnostics();

	CHECK(!report.version.empty(), "O2a1 the report carries a version");
	CHECK(report.baseDir == testBase, "O2a2 the report carries the base directory");
	CHECK(!report.hasRom, "O2a3 no ROM is reported when none is loaded");
	CHECK(report.mapper < 0, "O2a4 no mapper is reported when none is loaded");

	const std::string text = report.to_text();
	CHECK(!text.empty(), "O2a5 the report text is not empty");
	CHECK(text.find(report.version) != std::string::npos,
		  "O2a6 the report text names the version");
	CHECK(text.find("(none loaded)") != std::string::npos,
		  "O2a7 the report says that no ROM is loaded");
	CHECK(text.find("Audio:") != std::string::npos,
		  "O2a8 the report carries the audio starve counter");
	CHECK(text.find("Log file:") != std::string::npos,
		  "O2a9 the report carries the log file line");
	CHECK(text.find("Crash dir:") != std::string::npos,
		  "O2a10 the report carries the crash directory line");

	// Gathering twice must not crash and must stay consistent (the dialog
	// refreshes on demand).
	const fceux11::DiagnosticsReport again = fceux11::gather_diagnostics();
	CHECK(again.version == report.version, "O2a11 a second gathering is stable");

	if (failures == 0)
	{
		std::printf("PASS: diagnostics_report_test all legs\n");
		return 0;
	}
	std::printf("FAIL: diagnostics_report_test %d leg(s)\n", failures);
	return 1;
}
