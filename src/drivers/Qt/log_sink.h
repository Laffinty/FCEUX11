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

// v2.0.8 C2/C3 O1 -- the message-log file sink.
//
// This header deliberately depends on nothing but the standard library, so
// the sink can be exercised by a headless unit test while the GUI driver
// uses the very same code.
//
// What it replaces (the old msgLogBuf_t in MsgLogViewer.cpp):
//   * the path was %TEMP%\fceux.log -- unfindable and named after the
//     upstream project; the file was opened with "w+", so every start
//     truncated the previous run's log;
//   * addLine() touched a shared FILE* and the ring offsets with no lock,
//     while the GUI thread reads/clears the same state under the wrapper
//     lock that the emulation thread never takes.
//
// Design:
//   * the file is opened LAZILY. Messages that arrive before the emulator's
//     base directory is known (the base directory only exists after
//     InitConfig(), which runs inside fceuWrapperInit()) are kept in a
//     bounded memory buffer and flushed into the file when it opens -- so
//     there is exactly ONE log file and no early message is lost;
//   * every access to the file and to the ring goes through one mutex.
//     It is deliberately NOT the wrapper lock: taking that here would
//     deadlock against the GUI thread's own use of it.

#ifndef __FCEUX11_LOG_SINK_H_
#define __FCEUX11_LOG_SINK_H_

#include <cstddef>
#include <mutex>
#include <string>

namespace fceux11 {

/// Where the log file lives, given the emulator base directory.
/// Pure function (no side effects) so it can be asserted directly.
std::string derive_log_path(const std::string& base_dir);

/// The message-log file sink: one file, one lock, one ring of offsets.
class LogSink
{
public:
	/// The single sink the GUI driver writes through.
	static LogSink& instance();

	/// Opens (or re-points) the log file for the given base directory.
	/// Any messages buffered before this call are flushed into the new
	/// file. Calling it again with the same base is a no-op.
	void open_for_base(const std::string& base_dir);

	/// Appends one message. `newline` adds a line break.
	void append(const char* text, bool newline);

	/// Pushes whatever is buffered in the C runtime out to the file, so a
	/// reader (the diagnostic dialog, the user tailing the file) sees the
	/// messages that were written so far.
	void flush();

	/// Closes the file. Further appends are dropped until the sink is
	/// pointed at a base directory again. Needed before the log directory
	/// can be removed on Windows (an open file cannot be deleted).
	void close();

	/// The text the viewer shows: the tail of the log, at most `maxBytes`.
	std::string tail_text(std::size_t maxBytes) const;

	/// Forgets the ring (the file keeps its content).
	void clear();

	/// Number of messages appended so far.
	std::size_t line_count() const;

	/// The file currently being written, empty when nothing is open yet.
	std::string current_path() const;

	/// True when a file is open.
	bool is_open() const;

private:
	LogSink() = default;

	mutable std::mutex mtx;
	std::string path;
	void* fp = nullptr;          ///< FILE*, kept opaque so <cstdio> stays out of the header
	std::size_t totalLines = 0;
	std::size_t head = 0;
	std::size_t tail = 0;
	long fpOfsList[256] = { 0 };
	std::string pending;         ///< messages written before the file was open
};

} // namespace fceux11

#endif // __FCEUX11_LOG_SINK_H_
