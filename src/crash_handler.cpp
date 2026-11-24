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

// v2.0.8 C2 -- local crash reporting. See crash_handler.h for the contract.
//
// v2.0.9 -- one-shot reporting, non-clobbering dump names, and a
// pre-report hook for the driver layer's diagnostic backtrace.
//
// Rules for everything below the SEH filter:
//   * no UI, no dialogs, no exception escaping into the filter
//   * minimal allocation (the CRT heap may already be broken)
//   * one-shot: an atomic guard makes sure that ONE crash event produces
//     ONE dump and ONE summary line, however many handlers it passes
//     through on the way out

#include "crash_handler.h"

#include <cstdio>
#include <cstring>
#include <ctime>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <dbghelp.h>
#include <csignal>
#include <cstdint>
#include <cstdlib>
#include <exception>
#include <process.h>
#include <atomic>
#include <mutex>
#endif

#include "io_api.h"
#include "version.h"

namespace fceux11 {

namespace {

#ifdef _WIN32

// v2.0.9 -- the one-shot guard for the whole crash-report path.
//
// It used to be a plain `bool` that only `seh_filter` and
// `write_crash_dump` touched, which left `report()` itself unprotected.
// The CRT terminate path reached `report("terminate")`, then `abort()`
// raised SIGABRT, whose handler called `report("abort")` again: one crash
// event produced two summary lines and two writes to the same dump file
// name (a second-resolution stamp plus the pid), the second silently
// overwriting the first.
//
// The guard now lives inside `report()` and is an atomic exchange, so
// exactly one caller wins across every path (SEH / invalid parameter /
// pure call / terminate / abort) and across threads. It is set BEFORE any
// work happens, so a crash inside the reporter cannot recurse.
std::atomic<bool> g_reported{false};

// Serialises the dump write and the summary append, so a report is never
// interleaved with another one. `write_crash_dump` -- a public and
// repeatable API -- shares this lock.
std::mutex g_reportMutex;

// The pre-report hook (see crash_handler.h). Atomic because the driver
// layer registers it before the crash handler is installed, while a
// crash may fire on any thread.
std::atomic<CrashPreHook> g_preHook{nullptr};

/// Timestamp for the dump file name: yyyymmdd_hhmmss.
void make_stamp(char* out, size_t n)
{
	const time_t now = time(nullptr);
	struct tm tmv;
	localtime_s(&tmv, &now);
	snprintf(out, n, "%04d%02d%02d_%02d%02d%02d",
	         tmv.tm_year + 1900, tmv.tm_mon + 1, tmv.tm_mday,
	         tmv.tm_hour, tmv.tm_min, tmv.tm_sec);
}

/// Directory of the running executable (empty when it cannot be resolved).
std::string exe_dir()
{
	char buf[MAX_PATH];
	DWORD n = GetModuleFileNameA(nullptr, buf, sizeof(buf));
	if (n == 0 || n >= sizeof(buf))
		return std::string();
	std::string p(buf, n);
	const size_t slash = p.find_last_of("\\/");
	if (slash == std::string::npos)
		return std::string();
	return p.substr(0, slash);
}

/// Create `dir` if it is missing. Returns false when the directory could
/// not be created.
bool ensure_dir(const std::string& dir)
{
	if (dir.empty())
		return false;
	if (GetFileAttributesA(dir.c_str()) != INVALID_FILE_ATTRIBUTES)
		return true;
	return CreateDirectoryA(dir.c_str(), nullptr) != 0;
}

/// Append one summary line to <dir>/crash_log.txt.
void append_summary(const std::string& dir, const CrashInfo& info,
                    const std::string& dumpName)
{
	const std::string path = dir + "\\crash_log.txt";
	FILE* fp = fopen(path.c_str(), "a");
	if (!fp)
		return;

	time_t now = time(nullptr);
	struct tm tmv;
	localtime_s(&tmv, &now);
	char stamp[32];
	strftime(stamp, sizeof(stamp), "%Y-%m-%d %H:%M:%S", &tmv);

	fprintf(fp, "%s | version=%s | source=%s | code=0x%08lX | address=0x%08lX | pid=%lu | dump=%s\n",
	        stamp, FCEU_DISPLAY_VERSION,
	        info.source ? info.source : "unknown",
	        info.exceptionCode, info.exceptionAddress,
	        (unsigned long)GetCurrentProcessId(),
	        dumpName.empty() ? "(none)" : dumpName.c_str());
	fclose(fp);
}

/// Open a dump file under `dir`, never overwriting an existing one.
///
/// The documented name carries a second-resolution stamp plus the pid, so
/// two reports inside the same second used to land on the same path and
/// CREATE_ALWAYS truncated the first dump. We create with CREATE_NEW and,
/// on a name clash, append `_2`, `_3`, ... until a free name is found.
/// On success `outName` receives the file name that was actually used.
HANDLE open_unique_dump(const std::string& dir, const std::string& name,
                        std::string& outName)
{
	for (int attempt = 0; attempt < 100; ++attempt)
	{
		std::string candidate = name;
		if (attempt > 0)
		{
			const size_t dot = name.find_last_of('.');
			const std::string stem =
				(dot == std::string::npos) ? name : name.substr(0, dot);
			const std::string ext =
				(dot == std::string::npos) ? std::string() : name.substr(dot);
			candidate = stem + "_" + std::to_string(attempt + 1) + ext;
		}

		HANDLE hFile = CreateFileA((dir + "\\" + candidate).c_str(), GENERIC_WRITE,
		                           0, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL,
		                           nullptr);
		if (hFile != INVALID_HANDLE_VALUE)
		{
			outName = candidate;
			return hFile;
		}

		const DWORD err = GetLastError();
		if (err != ERROR_FILE_EXISTS && err != ERROR_ALREADY_EXISTS)
			return INVALID_HANDLE_VALUE;
	}
	return INVALID_HANDLE_VALUE;
}

/// Ask dbghelp for a minidump of the current process into `hFile`.
bool write_minidump(HANDLE hFile, EXCEPTION_POINTERS* ep)
{
	MINIDUMP_EXCEPTION_INFORMATION mei;
	mei.ExceptionPointers = ep;
	mei.ThreadId = GetCurrentThreadId();
	mei.ClientPointers = FALSE;

	const BOOL ok = MiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(),
	                                  hFile, MiniDumpNormal,
	                                  ep ? &mei : nullptr, nullptr, nullptr);
	return ok == TRUE;
}

/// The one place that writes a dump plus the summary line. `ep` may be null
/// (the CRT failure paths have no exception record).
///
/// One-shot: the first caller wins and every later call returns without
/// writing anything. That matters because a single crash event can reach
/// this function more than once -- `std::terminate` calls `abort()`, whose
/// SIGABRT handler comes back here -- and a second report would add a
/// second summary line and overwrite the first dump.
void report(const char* source, unsigned long code, unsigned long address,
            EXCEPTION_POINTERS* ep)
{
	if (g_reported.exchange(true))
		return;

	std::lock_guard<std::mutex> guard(g_reportMutex);

	// The driver layer's diagnostic backtrace runs here, before the dump:
	// this is the one point every crash path shares (see the routing table
	// in crash_handler.h), so it is where the backtrace belongs.
	if (CrashPreHook hook = g_preHook.load())
		hook(source, code, address);

	const std::string dir = crash_dump_dir();
	if (!ensure_dir(dir))
		return;

	char stamp[32];
	make_stamp(stamp, sizeof(stamp));
	char name[128];
	snprintf(name, sizeof(name), "fceux11_%s_%lu.dmp", stamp,
	         (unsigned long)GetCurrentProcessId());

	std::string used;
	HANDLE hFile = open_unique_dump(dir, name, used);
	bool ok = false;
	if (hFile != INVALID_HANDLE_VALUE)
	{
		ok = write_minidump(hFile, ep);
		CloseHandle(hFile);
	}

	CrashInfo info;
	info.source = source;
	info.exceptionCode = code;
	info.exceptionAddress = address;
	append_summary(dir, info, ok ? used : std::string("(failed)"));
}

LONG WINAPI seh_filter(EXCEPTION_POINTERS* ep)
{
	unsigned long code = 0;
	unsigned long address = 0;
	if (ep && ep->ExceptionRecord)
	{
		code = ep->ExceptionRecord->ExceptionCode;
		address = (unsigned long)(uintptr_t)ep->ExceptionRecord->ExceptionAddress;
	}
	// The one-shot guard lives in report(): this filter only has to make
	// sure it does not swallow the exception.
	report("seh", code, address, ep);

	// Hand the exception back to Windows: the process dies the way it would
	// have without the reporter, and Windows Error Reporting still gets it.
	return EXCEPTION_CONTINUE_SEARCH;
}

void invalid_parameter_handler(const wchar_t* expression, const wchar_t* function,
                               const wchar_t* file, unsigned int line, uintptr_t reserved)
{
	(void)expression; (void)function; (void)file; (void)line; (void)reserved;
	report("invalid-parameter", 0, 0, nullptr);
}

void purecall_handler()
{
	report("purecall", 0, 0, nullptr);
}

void terminate_handler()
{
	report("terminate", 0, 0, nullptr);
	abort();
}

void abort_handler(int sig)
{
	report("abort", (unsigned long)sig, 0, nullptr);
	_exit(3);
}

#endif // _WIN32

} // anonymous namespace

std::string crash_dump_dir()
{
#ifdef _WIN32
	// An explicit override wins over everything else: the test harness
	// uses it to keep its dumps out of the emulator's own crash directory,
	// and a user can point the dumps anywhere.
	if (const char* forced = getenv("FCEUX11_CRASH_DIR"))
	{
		if (forced[0] != '\0')
			return std::string(forced);
	}

	// Prefer the emulator base directory (where fcs/, sav/ and snaps/ live)
	// so the dumps sit next to the user's other files; fall back to the
	// executable's directory when the base directory is not known yet -- a
	// crash during start-up happens exactly when it is not.
	const char* base = fceu11::GetBaseDirectory();
	if (base && base[0] != '\0')
		return std::string(base) + "\\crashes";

	const std::string ed = exe_dir();
	if (!ed.empty())
		return ed + "\\crashes";

	char tmp[MAX_PATH];
	if (GetTempPathA(sizeof(tmp), tmp) > 0)
		return std::string(tmp) + "fceux11\\crashes";
	return std::string("crashes");
#else
	return std::string("crashes");
#endif
}

bool write_crash_dump(const CrashInfo& info, const std::string& dir)
{
#ifdef _WIN32
	// This is a public, repeatable API (the test suite calls it more than
	// once), so it deliberately does NOT consume the one-shot guard that
	// report() uses -- it only takes the same lock, so a dump and its
	// summary line are never interleaved with another report.
	std::lock_guard<std::mutex> guard(g_reportMutex);

	if (!ensure_dir(dir))
		return false;

	char stamp[32];
	make_stamp(stamp, sizeof(stamp));
	char name[128];
	snprintf(name, sizeof(name), "fceux11_%s_%lu.dmp", stamp,
	         (unsigned long)GetCurrentProcessId());

	std::string used;
	HANDLE hFile = open_unique_dump(dir, name, used);
	if (hFile == INVALID_HANDLE_VALUE)
		return false;

	const bool ok = write_minidump(hFile, nullptr);
	CloseHandle(hFile);

	append_summary(dir, info, ok ? used : std::string("(failed)"));
	return ok;
#else
	(void)info; (void)dir;
	return false;
#endif
}

void set_crash_pre_hook(CrashPreHook hook)
{
#ifdef _WIN32
	g_preHook.store(hook);
#else
	(void)hook;
#endif
}

void install_crash_handler()
{
#ifdef _WIN32
	SetUnhandledExceptionFilter(seh_filter);
	_set_invalid_parameter_handler(invalid_parameter_handler);
	_set_purecall_handler(purecall_handler);
	std::set_terminate(terminate_handler);
	signal(SIGABRT, abort_handler);
#endif
}

[[noreturn]] void trigger_test_crash()
{
#ifdef _WIN32
	// A non-continuable exception: the SEH filter runs, writes the dump,
	// and returns EXCEPTION_CONTINUE_SEARCH, so the process dies the way a
	// real crash would.
	RaiseException(0xE0000001u, EXCEPTION_NONCONTINUABLE, 0, nullptr);
	_exit(4); // not reached
#else
	abort();
#endif
}

} // namespace fceux11
