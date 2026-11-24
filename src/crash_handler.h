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
// v2.0.9 -- two contract changes:
//
//   * ONE crash event produces ONE dump and ONE summary line. The CRT
//     terminate path reaches the reporter, calls abort(), and the SIGABRT
//     handler comes back to the reporter: without a one-shot guard that
//     produced two summary lines and two writes to the same file name
//     (the second silently overwriting the first).
//
//   * A dump file is NEVER overwritten. The name carries a second-
//     resolution stamp plus the pid, so two reports in the same second
//     used to collide; a clash now adds a `_2` / `_3` ... suffix.
//
// The crash directory can be forced with the FCEUX11_CRASH_DIR environment
// variable (the test harness uses it to keep its dumps out of the
// emulator's own crash directory).
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

/// A hook that runs once per crash event, inside the crash context, just
/// before the reporter writes the minidump.
///
/// It exists for two reasons that both came out of measurement:
///
///   * The symbolised C++ backtrace lives in the driver layer (it needs
///     dbghelp) while the reporter lives in the core layer, so the two
///     cannot call each other directly without breaking the layering.
///
///   * The SEH filter -- not std::terminate -- is what actually sees an
///     escaping C++ exception. Measured on this toolchain: the SEH
///     filter preempts terminate on the main thread (code 0xE06D7363
///     reaches the filter, terminate is never called), and an exception
///     escaping a thread goes straight to SIGABRT. So the terminate
///     handler alone would miss both of the most common paths, and a
///     hook is the only point every crash path shares.
///
/// Contract: it runs in a broken process. It must not throw, must not
/// call back into the reporter, must not allocate heavily, and must
/// return promptly. `source` is "seh" / "terminate" / "abort" / ...,
/// `code` and `address` carry the exception record when there is one and
/// are 0 otherwise.
using CrashPreHook = void (*)(const char* source, unsigned long code,
                               unsigned long address);

/// Registers the pre-report hook (one at a time; nullptr clears it).
/// Call it before `install_crash_handler()`.
void set_crash_pre_hook(CrashPreHook hook);

/// Installs the crash reporter: the unhandled-exception filter plus the
/// CRT failure hooks (invalid parameter, pure call, terminate, abort).
/// Call it as early as possible -- before any Qt object exists.
void install_crash_handler();

/// Directory the dumps are written to. Uses the emulator base directory
/// when it is known, otherwise the directory of the executable. The
/// FCEUX11_CRASH_DIR environment variable overrides both.
std::string crash_dump_dir();

/// Writes a minidump of the current process plus a one-line summary into
/// `dir`. Returns true when the dump file was created.
/// `info` is what goes into the summary; the dump itself always describes
/// the calling process.
///
/// Unlike the automatic crash paths this is a public, repeatable API: it
/// may be called any number of times, and each call writes its own dump
/// under a name that does not collide with an existing one.
bool write_crash_dump(const CrashInfo& info, const std::string& dir);

/// Raises a controlled, non-continuable exception. Used by the
/// FCEUX11_TEST_CRASH=1 probe so the whole path can be exercised by hand.
[[noreturn]] void trigger_test_crash();

} // namespace fceux11

#endif // __FCEUX11_CRASH_HANDLER_H_
