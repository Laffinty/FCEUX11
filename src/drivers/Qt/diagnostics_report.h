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

// v2.0.8 C2/C3 O2 -- the diagnostics report.
//
// The data gathering is kept apart from the dialog so it can be exercised
// by a headless test: everything here is plain data plus a plain string.
//
// Every counter it reads is an existing atomic (nes_shm render/blit counts,
// the audio sink's starve counter) or an immutable string; nothing here
// changes any state.

#ifndef __FCEUX11_DIAGNOSTICS_REPORT_H_
#define __FCEUX11_DIAGNOSTICS_REPORT_H_

#include <cstdint>
#include <string>

namespace fceux11 {

/// Everything the diagnostics dialog shows, gathered at one moment.
struct DiagnosticsReport
{
	std::string version;       ///< e.g. "v2.0.7"
	std::string buildInfo;     ///< compiler / build identification
	std::string baseDir;       ///< the emulator base directory

	bool hasRom = false;       ///< false when no game is loaded
	std::string romName;
	std::string romPath;
	int mapper = -1;
	uint32_t crc32 = 0;
	std::string md5;

	uint64_t renderFrames = 0; ///< frames rendered
	uint64_t blitFrames = 0;   ///< frames presented
	double nominalFps = 0.0;   ///< the region's nominal frame rate
	uint64_t audioStarveCount = 0; ///< samples the audio sink starved of

	std::string logFilePath;   ///< the message-log file, if any
	uint64_t logFileBytes = 0;

	std::string crashDir;      ///< where crash dumps are written
	uint64_t crashDumpCount = 0;
	std::string newestCrashDump;

	/// The human-readable report: what the dialog shows and what the
	/// "copy to clipboard" button puts on the clipboard.
	std::string to_text() const;
};

/// Gathers the current state. Safe to call with no game loaded.
DiagnosticsReport gather_diagnostics();

} // namespace fceux11

#endif // __FCEUX11_DIAGNOSTICS_REPORT_H_
