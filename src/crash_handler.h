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

// v2.0.8 C2 -- local crash reporting.
//
// A crash in the emulator used to be a silent process death: no dump, no
// record, nothing for the user to send. This adds a local-only reporter
// (owner decision D1: no Sentry, no cloud, no telemetry -- the files never
// leave the machine unless the user copies them).
//
// What it produces, in <crash dir>:
//   fceux11_<yyyymmdd_hhmmss>_<pid>.dmp   a Windows minidump
//   crash_log.txt                          one appended line per crash
//
// The public header deliberately does not include <windows.h>: the crash
// path must not drag platform headers into every translation unit, and the
// testable entry point works on plain data.

#ifndef __FCEUX11_CRASH_HANDLER_H_
#define __FCEUX11_CRASH_HANDLER_H_

#include <string>

namespace fceux11 {

/// Plain description of a fatal error, used for the summary line.
struct CrashInfo
{
	unsigned long exceptionCode = 0;    ///< e.g. 0xC0000005 (access violation)
	unsigned long exceptionAddress = 0;  ///< faulting address
	const char* source = "unknown";      ///< "seh", "terminate", "abort", ...
};

/// Installs the crash reporter: the unhandled-exception filter plus the
/// CRT failure hooks (invalid parameter, pure call, terminate, abort).
/// Call it as early as possible -- before any Qt object exists.
void install_crash_handler();

/// Directory the dumps are written to. Uses the emulator base directory
/// when it is known, otherwise the directory of the executable.
std::string crash_dump_dir();

/// Writes a minidump of the current process plus a one-line summary into
/// `dir`. Returns true when the dump file was created.
/// `info` is what goes into the summary; the dump itself always describes
/// the calling process.
bool write_crash_dump(const CrashInfo& info, const std::string& dir);

/// Raises a controlled, non-continuable exception. Used by the
/// FCEUX11_TEST_CRASH=1 probe so the whole path can be exercised by hand.
[[noreturn]] void trigger_test_crash();

} // namespace fceux11

#endif // __FCEUX11_CRASH_HANDLER_H_
