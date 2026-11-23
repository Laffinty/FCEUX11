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
// Rules for everything below the SEH filter:
//   * no UI, no dialogs, no exception escaping into the filter
//   * minimal allocation (the CRT heap may already be broken)
//   * one-shot: a re-entrancy guard keeps a crash inside the handler from
//     recursing

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
#endif

#include "io_api.h"
#include "version.h"

namespace fceux11 {

namespace {

bool g_inCrash = false;

#ifdef _WIN32

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
                    const char* dumpName)
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
	        dumpName ? dumpName : "(none)");
	fclose(fp);
}

/// Ask dbghelp for a minidump of the current process.
bool write_minidump(const std::string& path, EXCEPTION_POINTERS* ep)
{
	HANDLE hFile = CreateFileA(path.c_str(), GENERIC_WRITE, 0, nullptr,
	                           CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
	if (hFile == INVALID_HANDLE_VALUE)
		return false;

	MINIDUMP_EXCEPTION_INFORMATION mei;
	mei.ExceptionPointers = ep;
	mei.ThreadId = GetCurrentThreadId();
	mei.ClientPointers = FALSE;

	const BOOL ok = MiniDumpWriteDump(GetCurrentProcess(), GetCurrentProcessId(),
	                                  hFile, MiniDumpNormal,
	                                  ep ? &mei : nullptr, nullptr, nullptr);
	CloseHandle(hFile);
	return ok == TRUE;
}

/// The one place that writes a dump plus the summary line. `ep` may be null
/// (the CRT failure paths have no exception record).
void report(const char* source, unsigned long code, unsigned long address,
            EXCEPTION_POINTERS* ep)
{
	const std::string dir = crash_dump_dir();
	if (!ensure_dir(dir))
		return;

	char stamp[32];
	make_stamp(stamp, sizeof(stamp));
	char name[128];
	snprintf(name, sizeof(name), "fceux11_%s_%lu.dmp", stamp,
	         (unsigned long)GetCurrentProcessId());

	const bool ok = write_minidump(dir + "\\" + name, ep);

	CrashInfo info;
	info.source = source;
	info.exceptionCode = code;
	info.exceptionAddress = address;
	append_summary(dir, info, ok ? name : "(failed)");
}

LONG WINAPI seh_filter(EXCEPTION_POINTERS* ep)
{
	if (g_inCrash)
		return EXCEPTION_CONTINUE_SEARCH;
	g_inCrash = true;

	unsigned long code = 0;
	unsigned long address = 0;
	if (ep && ep->ExceptionRecord)
	{
		code = ep->ExceptionRecord->ExceptionCode;
		address = (unsigned long)(uintptr_t)ep->ExceptionRecord->ExceptionAddress;
	}
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
	if (g_inCrash)
		return false;
	g_inCrash = true;

	if (!ensure_dir(dir))
	{
		g_inCrash = false;
		return false;
	}

	char stamp[32];
	make_stamp(stamp, sizeof(stamp));
	char name[128];
	snprintf(name, sizeof(name), "fceux11_%s_%lu.dmp", stamp,
	         (unsigned long)GetCurrentProcessId());

	const bool ok = write_minidump(dir + "\\" + name, nullptr);
	append_summary(dir, info, ok ? name : "(failed)");
	g_inCrash = false;
	return ok;
#else
	(void)info; (void)dir;
	return false;
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
