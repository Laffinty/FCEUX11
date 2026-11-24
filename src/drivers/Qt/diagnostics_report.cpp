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

// v2.0.8 C2/C3 O2 -- the diagnostics report. See diagnostics_report.h.

#include "Qt/diagnostics_report.h"

#include <cstdio>
#include <filesystem>
#include <system_error>

#include "types.h"
#include "git.h"
// fceu.h must come before cart.h: cart.h expands DECLFR/DECLFW, which fceu.h
// defines.
#include "fceu.h"
#include "cart.h"
#include "io_api.h"
#include "diag_api.h"
#include "version.h"
#include "crash_handler.h"
#include "Qt/log_sink.h"
#include "Qt/throttle.h"
#include "common/nes_shm.h"

namespace fceux11 {

namespace {

std::string hex32(uint32_t v)
{
	char buf[16];
	snprintf(buf, sizeof(buf), "%08X", v);
	return std::string(buf);
}

std::string md5_hex(const MD5DATA& md5)
{
	std::string out;
	char buf[4];
	for (int i = 0; i < 16; i++)
	{
		snprintf(buf, sizeof(buf), "%02X", (unsigned)md5[i]);
		out += buf;
	}
	return out;
}

uint64_t file_size_of(const std::string& path)
{
	if (path.empty())
		return 0;
	std::error_code ec;
	const auto size = std::filesystem::file_size(path, ec);
	if (ec)
		return 0;
	return (uint64_t)size;
}

} // anonymous namespace

std::string DiagnosticsReport::to_text() const
{
	std::string out;

	out += "FCEUX11 diagnostics\n";
	out += "-------------------\n";
	out += "Version:    " + version + "\n";
	if (!buildInfo.empty())
		out += "Build:      " + buildInfo + "\n";
	out += "Base dir:   " + (baseDir.empty() ? "(unknown)" : baseDir) + "\n";

	if (hasRom)
	{
		out += "ROM:        " + (romName.empty() ? romPath : romName) + "\n";
		if (!romPath.empty() && romPath != romName)
			out += "Path:       " + romPath + "\n";
		if (mapper >= 0)
			out += "Mapper:     " + std::to_string(mapper) + "\n";
		out += "CRC32:      " + hex32(crc32) + "\n";
		if (!md5.empty())
			out += "MD5:        " + md5 + "\n";
	}
	else
	{
		out += "ROM:        (none loaded)\n";
	}

	out += "Frames:     rendered " + std::to_string(renderFrames) +
	       ", presented " + std::to_string(blitFrames) + "\n";
	if (nominalFps > 0.0)
		out += "Frame rate: " + std::to_string((int)nominalFps) + " fps (nominal)\n";
	out += "Audio:      " + std::to_string(audioStarveCount) + " starved samples\n";

	out += "Log file:   " + (logFilePath.empty() ? "(no file)" : logFilePath) + "\n";
	if (!logFilePath.empty())
		out += "            " + std::to_string(logFileBytes) + " bytes\n";

	out += "Crash dir:  " + (crashDir.empty() ? "(unknown)" : crashDir) + "\n";
	out += "            " + std::to_string(crashDumpCount) + " dump(s)";
	if (!newestCrashDump.empty())
		out += ", newest: " + newestCrashDump;
	out += "\n";

	return out;
}

DiagnosticsReport gather_diagnostics()
{
	DiagnosticsReport r;

	r.version = FCEU_DISPLAY_VERSION;
	r.buildInfo = FCEUD_GetCompilerString();

	const char* base = fceu11::GetBaseDirectory();
	if (base)
		r.baseDir = base;

	// The game currently loaded (null in a GBA session -- the GBA loader
	// never constructs a GameInfo -- and before any ROM is opened).
	if (GameInfo)
	{
		r.hasRom = true;
		r.romName = GameInfo->name;
		r.romPath = GameInfo->filename;
		r.md5 = md5_hex(GameInfo->MD5);
	}
	if (currCartInfo)
	{
		r.mapper = currCartInfo->mapper_number;
		r.crc32 = currCartInfo->CRC32;
	}

	// Frame and audio counters (all atomics, all read-only here).
	if (nes_shm)
	{
		r.renderFrames = nes_shm->render_count.load(std::memory_order_relaxed);
		r.blitFrames = nes_shm->blit_count.load(std::memory_order_relaxed);
		r.audioStarveCount = nes_shm->sndBuf.starveCounter.load(std::memory_order_relaxed);
	}
	r.nominalFps = getBaseFrameRate();

	// The message log.
	const std::string logPath = LogSink::instance().current_path();
	if (!logPath.empty())
	{
		r.logFilePath = logPath;
		r.logFileBytes = file_size_of(logPath);
	}

	// The crash dumps (batch 2 wrote them into <base>/crashes).
	r.crashDir = crash_dump_dir();
	if (!r.crashDir.empty())
	{
		std::error_code ec;
		uint64_t newest = 0;
		for (const auto& entry :
		     std::filesystem::directory_iterator(r.crashDir, ec))
		{
			if (ec)
				break;
			if (entry.path().extension() != ".dmp")
				continue;
			r.crashDumpCount++;
			std::error_code ec2;
			const auto t = std::filesystem::last_write_time(entry.path(), ec2);
			if (ec2)
				continue;
			const auto since = t.time_since_epoch().count();
			if (since > newest)
			{
				newest = (uint64_t)since;
				r.newestCrashDump = entry.path().filename().string();
			}
		}
	}

	return r;
}

} // namespace fceux11
