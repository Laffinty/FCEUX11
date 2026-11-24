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

// v2.0.9 -- one-shot crash reporting lock.
//
// The defect this locks: the reporter's re-entrancy guard used to be a
// plain `bool` that only the SEH filter and `write_crash_dump` touched,
// so `report()` itself was unprotected. A single crash event could reach
// it twice -- `std::terminate` calls `abort()`, whose SIGABRT handler
// calls back into the reporter -- and the second report added a second
// summary line and overwrote the first dump (the file name carries a
// second-resolution stamp plus the pid).
//
// The test drives the real crash handler in a child process and counts
// what lands on disk:
//
//   C1  terminate   -> exactly ONE dump and ONE summary line
//   C2  abort       -> exactly ONE dump and ONE summary line
//   C3  seh         -> exactly ONE dump and ONE summary line
//   C4  two write_crash_dump() calls in the same second -> TWO dumps
//       under TWO different names (the second must not overwrite the
//       first) and two summary lines
//   C5  every dump is non-empty (a capture that produced nothing would be
//       a toothless gate -- the same failure mode AGENTS.md records for
//       the audio goldens)
//
// The child modes are deliberately chosen so that the process always ends
// through an explicit exit, never through the operating system's
// unhandled-exception path: that path can raise a WER dialog, which on a
// CI runner without an interactive desktop blocks forever (AGENTS.md
// records that failure mode twice). For the SEH leg the child wraps the
// reporter's filter so it can exit deterministically once the reporter
// has done its work.

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <filesystem>
#include <string>

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

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

/// What the crash directory holds after one crash event.
struct DirStats
{
	int dumps = 0;
	bool anyDumpEmpty = false;
	long long smallestDump = 0;
	int summaryLines = 0;
	int hookCalls = 0;
};

static int count_lines(const std::filesystem::path& path)
{
	if (path.empty())
		return 0;
	FILE* fp = std::fopen(path.string().c_str(), "rb");
	if (!fp)
		return 0;
	char buf[1024];
	int lines = 0;
	while (fgets(buf, sizeof(buf), fp))
		++lines;
	std::fclose(fp);
	return lines;
}

static DirStats inspect(const std::string& dir)
{
	DirStats st;
	std::error_code ec;
	std::filesystem::path logPath;
	std::filesystem::path hookPath;
	for (const auto& e : std::filesystem::directory_iterator(dir, ec))
	{
		if (e.path().extension() == ".dmp")
		{
			++st.dumps;
			const long long size = (long long)std::filesystem::file_size(e.path(), ec);
			if (size == 0)
				st.anyDumpEmpty = true;
			if (st.smallestDump == 0 || size < st.smallestDump)
				st.smallestDump = size;
		}
		else if (e.path().filename() == "crash_log.txt")
		{
			logPath = e.path();
		}
		else if (e.path().filename() == "hook_calls.txt")
		{
			hookPath = e.path();
		}
	}

	st.summaryLines = count_lines(logPath);
	st.hookCalls = count_lines(hookPath);
	return st;
}

// ---------------------------------------------------------------------------
// Child side
// ---------------------------------------------------------------------------

/// The reporter's own unhandled-exception filter, captured so the child
/// can call it and then leave deterministically.
static LPTOP_LEVEL_EXCEPTION_FILTER g_prev_filter = nullptr;

/// Pre-report hook. It appends one line per call, so the parent can count
/// how many times the reporter invoked it. This is the mechanism the
/// driver layer uses to write the symbolised backtrace, and the point of
/// the one-shot guard is that it runs exactly once per crash event.
static void mark_hook(const char* source, unsigned long code, unsigned long)
{
	const std::string dir = fceux11::crash_dump_dir();
	FILE* fp = nullptr;
	if (fopen_s(&fp, (dir + "\\hook_calls.txt").c_str(), "ab") == 0 && fp)
	{
		std::fprintf(fp, "hook source=%s code=%lu\n", source ? source : "?", code);
		std::fclose(fp);
	}
}

static LONG WINAPI child_filter(EXCEPTION_POINTERS* ep)
{
	// The reporter's filter does the real work (writes the dump and the
	// summary line) and returns EXCEPTION_CONTINUE_SEARCH. We ignore that
	// and exit ourselves: letting the exception continue would hand it to
	// the operating system, which may raise a dialog.
	if (g_prev_filter)
		g_prev_filter(ep);
	_exit(70);
}

static int run_child(const char* mode)
{
	fceux11::set_crash_pre_hook(mark_hook);
	fceux11::install_crash_handler();

	if (std::strcmp(mode, "--terminate") == 0)
	{
		// terminate -> report("terminate") -> abort() -> SIGABRT ->
		// report("abort"). This is the path that used to report twice.
		std::terminate();
	}
	else if (std::strcmp(mode, "--abort") == 0)
	{
		abort();
	}
	else if (std::strcmp(mode, "--seh") == 0)
	{
		g_prev_filter = SetUnhandledExceptionFilter(child_filter);
		RaiseException(0xE0000001u, EXCEPTION_NONCONTINUABLE, 0, nullptr);
		_exit(71); // not reached
	}
	return 0;
}

// ---------------------------------------------------------------------------
// Parent side
// ---------------------------------------------------------------------------

/// Run this executable again as a child and wait for it. The child's exit
/// code is a crash code by design, so it is reported, not judged.
static int spawn_child(const char* mode)
{
	char self[MAX_PATH] = { 0 };
	if (GetModuleFileNameA(nullptr, self, MAX_PATH) == 0)
		return -1;

	std::string cmd = "\"";
	cmd += self;
	cmd += "\" --child ";
	cmd += mode;
	cmd += " >nul 2>&1";

	FILE* p = _popen(cmd.c_str(), "r");
	if (!p)
		return -1;
	char buf[512];
	while (fgets(buf, sizeof(buf), p))
		; // drain
	return _pclose(p);
}

int main(int argc, char** argv)
{
	if (argc > 1 && std::strcmp(argv[1], "--child") == 0)
	{
		return run_child(argc > 2 ? argv[2] : "--terminate");
	}

	const std::filesystem::path base =
		std::filesystem::temp_directory_path() / "f11_crash_once_test";
	std::error_code ec;
	std::filesystem::remove_all(base, ec);
	std::filesystem::create_directories(base, ec);
	const std::string dir = base.string();

	// The children write into this directory (the reporter honours the
	// override), which keeps the test out of the emulator's own crash
	// directory and out of the build tree.
	_putenv_s("FCEUX11_CRASH_DIR", dir.c_str());

	struct Case
	{
		const char* mode;
		const char* label;
	};
	const Case cases[] = {
		{ "--terminate", "C1 terminate" },
		{ "--abort", "C2 abort" },
		{ "--seh", "C3 seh" },
	};

	for (const Case& c : cases)
	{
		std::filesystem::remove_all(base, ec);
		std::filesystem::create_directories(base, ec);

		const int rc = spawn_child(c.mode);
		const DirStats st = inspect(dir);

		std::printf("  [%s] child exit=%d dumps=%d summaryLines=%d hookCalls=%d smallestDump=%lld\n",
		            c.mode, rc, st.dumps, st.summaryLines, st.hookCalls, st.smallestDump);
		std::fflush(stdout);

		char tag[64];
		snprintf(tag, sizeof(tag), "%s", c.label);
		char msg[128];
		snprintf(msg, sizeof(msg), "%s: exactly one dump file", tag);
		CHECK(st.dumps == 1, msg);
		snprintf(msg, sizeof(msg), "%s: exactly one summary line", tag);
		CHECK(st.summaryLines == 1, msg);
		snprintf(msg, sizeof(msg), "%s: the dump is not empty", tag);
		CHECK(!st.anyDumpEmpty, msg);
		// C6: the pre-report hook -- the single point every crash path
		// shares, and the mechanism the driver layer hangs the symbolised
		// backtrace on -- must run exactly once per crash event.
		snprintf(msg, sizeof(msg), "%s: the pre-report hook ran exactly once", tag);
		CHECK(st.hookCalls == 1, msg);
	}

	// ---- C4: two dumps in the same second must not overwrite each other --
	{
		std::filesystem::remove_all(base, ec);
		std::filesystem::create_directories(base, ec);

		fceux11::CrashInfo first;
		first.source = "unit-test-1";
		first.exceptionCode = 0xC0000005u;
		first.exceptionAddress = 0x1234u;

		fceux11::CrashInfo second;
		second.source = "unit-test-2";
		second.exceptionCode = 0xC0000006u;
		second.exceptionAddress = 0x5678u;

		const bool ok1 = fceux11::write_crash_dump(first, dir);
		const bool ok2 = fceux11::write_crash_dump(second, dir);

		const DirStats st = inspect(dir);
		std::printf("  [C4] ok1=%d ok2=%d dumps=%d summaryLines=%d\n", ok1, ok2,
		            st.dumps, st.summaryLines);
		std::fflush(stdout);

		CHECK(ok1 && ok2, "C4a both write_crash_dump calls succeed");
		CHECK(st.dumps == 2, "C4b two calls produce two dump files");
		CHECK(st.summaryLines == 2, "C4c two calls produce two summary lines");
		CHECK(!st.anyDumpEmpty, "C4d neither dump is empty");
	}

	std::filesystem::remove_all(base, ec);

	if (failures == 0)
	{
		std::printf("PASS: crash_once_test all legs\n");
		return 0;
	}
	std::printf("FAIL: crash_once_test %d leg(s)\n", failures);
	return 1;
}
