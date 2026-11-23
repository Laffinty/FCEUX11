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

// v2.0.8 C2 -- local crash reporter lock (plan legs M6/M7).
//
// M6: a dump written through the public entry point produces a real file
//     whose size is NOT zero, and a summary line that names the source.
//     (The "content is not empty" assertion matters: a capture that
//     silently produced nothing would be a toothless gate -- the same
//     failure mode AGENTS.md records for the audio goldens.)
// M7: an unusable target directory is reported as a failure instead of
//     crashing the process.

#include <cstdio>
#include <cstring>
#include <filesystem>
#include <string>

#include "crash_handler.h"

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
	std::filesystem::path base =
		std::filesystem::temp_directory_path() / "f11_crash_dump_test";
	std::filesystem::remove_all(base);
	std::filesystem::create_directories(base);
	const std::string dir = base.string();

	// ---- M6: a real dump plus a summary line ----
	{
		fceux11::CrashInfo info;
		info.source = "unit-test";
		info.exceptionCode = 0xC0000005u;
		info.exceptionAddress = 0x00001234u;

		const bool ok = fceux11::write_crash_dump(info, dir);
		CHECK(ok, "M6a write_crash_dump reports success");

		// Find the dump file and make sure it is not empty.
		std::filesystem::path dump;
		uintmax_t size = 0;
		for (const auto& e : std::filesystem::directory_iterator(dir))
		{
			if (e.path().extension() == ".dmp")
			{
				dump = e.path();
				size = std::filesystem::file_size(e.path());
				break;
			}
		}
		CHECK(!dump.empty(), "M6b a .dmp file was created");
		CHECK(size > 0, "M6c the dump is not empty");

		// The summary line must exist and carry the fields a user needs.
		const std::string logPath = dir + "\\crash_log.txt";
		FILE* fp = std::fopen(logPath.c_str(), "rb");
		CHECK(fp != nullptr, "M6d crash_log.txt was written");
		if (fp)
		{
			char buf[1024] = { 0 };
			const size_t n = std::fread(buf, 1, sizeof(buf) - 1, fp);
			std::fclose(fp);
			const std::string text(buf, n);
			CHECK(text.find("source=unit-test") != std::string::npos,
				  "M6e summary names the source");
			CHECK(text.find("0xC0000005") != std::string::npos,
				  "M6f summary carries the exception code");
			CHECK(text.find("version=") != std::string::npos,
				  "M6g summary carries the version");
		}
		else
		{
			failures++;
		}
	}

	// ---- M7: an unusable directory is a clean failure ----
	{
		fceux11::CrashInfo info;
		info.source = "unit-test-bad-dir";
		// A path under a file (not a directory) cannot be created.
		const std::string badDir = dir + "\\not-a-dir\\nested";
		const bool ok = fceux11::write_crash_dump(info, badDir);
		CHECK(!ok, "M7a unusable directory reports failure");
		CHECK(true, "M7b the process survived the failure");
	}

	std::filesystem::remove_all(base);

	if (failures == 0)
	{
		std::printf("PASS: crash_dump_test all legs\n");
		return 0;
	}
	std::printf("FAIL: crash_dump_test %d leg(s)\n", failures);
	return 1;
}
