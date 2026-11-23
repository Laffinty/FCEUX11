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

// v2.0.8 C2/C3 O1 -- message-log sink lock (plan legs O1a / O1b).
//
// O1a: the path derivation. With a base directory the log lives under
//      <base>/logs/; without one it falls back to the temp directory.
//      The old sink wrote "%TEMP%\fceux.log" -- unfindable, and named
//      after the upstream project.
// O1b: concurrent writes must not tear. The emulation thread appends
//      while the GUI thread reads and clears the same ring; the old
//      addLine() touched the shared FILE* and the ring offsets with no
//      lock at all.

#include <atomic>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <set>
#include <string>
#include <thread>
#include <vector>

#include "Qt/log_sink.h"

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
	// Unbuffered stdout: if this test ever dies mid-way, the lines printed
	// so far must survive (a piped stdout is block-buffered and would be
	// lost together with the process).
	std::setvbuf(stdout, nullptr, _IONBF, 0);

	// ---- O1a: path derivation ----
	{
		const std::string base = "C:\\fceux_base";
		const std::string path = fceux11::derive_log_path(base);
		const std::filesystem::path p(path);
		CHECK(p.has_parent_path(), "O1a1 the path has a parent directory");
		CHECK(p.parent_path().filename() == "logs",
			  "O1a2 the log lives in a logs/ directory");
		CHECK(p.filename() == "fceux11_log.txt",
			  "O1a3 the file is named fceux11_log.txt");
		CHECK(path.find("fceux.log") == std::string::npos,
			  "O1a4 the upstream file name is gone");

		// Without a base directory it must still land somewhere writable --
		// an ABSOLUTE path under the temp directory, in its own "fceux11"
		// subdirectory (a relative path would depend on the working
		// directory and could not be found by the user).
		const std::string fallback = fceux11::derive_log_path("");
		CHECK(!fallback.empty(), "O1a5 empty base falls back to a path");
		const std::filesystem::path fb(fallback);
		CHECK(fb.is_absolute(), "O1a6 the fallback path is absolute");
		CHECK(fb.parent_path().filename() == "fceux11",
			  "O1a7 the fallback lives in its own fceux11 directory");
	}

	// ---- O1b: concurrent writes do not tear ----
	{
		const std::filesystem::path dir =
			std::filesystem::temp_directory_path() / "f11_log_sink_test";
		std::filesystem::remove_all(dir);
		std::filesystem::create_directories(dir);
		const std::string base = dir.string();

		fceux11::LogSink& sink = fceux11::LogSink::instance();
		sink.open_for_base(base);
		CHECK(sink.is_open(), "O1b1 the sink opened its file");
		CHECK(sink.current_path().find("logs") != std::string::npos,
			  "O1b2 the file sits under logs/");

		const int kThreads = 8;
		const int kLines = 400;
		std::atomic<int> ready{ 0 };
		std::atomic<bool> stop{ false };
		std::atomic<int> badReads{ 0 };
		std::vector<std::thread> workers;

		// A reader runs concurrently with the writers: this is the GUI
		// thread refreshing the message-log view. Without the sink's own
		// lock the ring can be read half-updated (a lost offset update
		// leaves a stale tail offset), and the view then starts in the
		// middle of a line. Every complete line must therefore be a whole
		// "T<thread>-<seq>" record.
		std::thread reader([&]()
		{
			while (!stop.load())
			{
				const std::string text = sink.tail_text(65536);
				std::size_t start = 0;
				for (;;)
				{
					const std::size_t nl = text.find('\n', start);
					if (nl == std::string::npos)
						break;
					const std::string line = text.substr(start, nl - start);
					// "T7-0399" -> 7 characters: 'T', digit, '-', four digits
					if (line.size() != 7 || line[0] != 'T' || line[2] != '-')
					{
						badReads.fetch_add(1);
						break;
					}
					start = nl + 1;
				}
			}
		});

		for (int t = 0; t < kThreads; t++)
		{
			workers.emplace_back([t, &ready, &sink]()
			{
				ready.fetch_add(1);
				while (ready.load() < 8)
					std::this_thread::yield();
				for (int i = 0; i < kLines; i++)
				{
					char line[64];
					snprintf(line, sizeof(line), "T%d-%04d", t, i);
					sink.append(line, true);
				}
			});
		}
		for (auto& w : workers)
			w.join();
		stop.store(true);
		reader.join();

		const int total = kThreads * kLines;
		CHECK(sink.line_count() == static_cast<std::size_t>(total),
			  "O1b3 every message was counted exactly once");
		CHECK(badReads.load() == 0,
			  "O1b7 a concurrent reader never saw a torn ring");

		// The file must contain each line exactly once, uncut. The sink
		// buffers in the C runtime, so the test asks for a flush before
		// reading -- otherwise it would read a lagging file (which is
		// exactly the defect this leg exists to catch).
		sink.flush();
		const std::string path = sink.current_path();
		std::ifstream in(path, std::ios::binary);
		std::string content((std::istreambuf_iterator<char>(in)),
		                    std::istreambuf_iterator<char>());
		in.close();

		std::set<std::string> seen;
		std::size_t lines = 0;
		std::size_t start = 0;
		bool torn = false;
		for (;;)
		{
			const std::size_t nl = content.find('\n', start);
			if (nl == std::string::npos)
				break;
			const std::string line = content.substr(start, nl - start);
			if (line.find("T") != 0 || line.size() < 6)
				torn = true;
			seen.insert(line);
			lines++;
			start = nl + 1;
		}
		CHECK(!torn, "O1b4 no line was torn");
		CHECK(lines == static_cast<std::size_t>(total),
			  "O1b5 the file holds every line");
		CHECK(seen.size() == static_cast<std::size_t>(total),
			  "O1b6 no line was lost or duplicated");

		// Close before removing the directory: Windows refuses to delete a
		// file that is still open (the removal would throw and abort the
		// test process).
		sink.close();
		std::filesystem::remove_all(dir);
	}

	if (failures == 0)
	{
		std::printf("PASS: log_sink_test all legs\n");
		return 0;
	}
	std::printf("FAIL: log_sink_test %d leg(s)\n", failures);
	return 1;
}
