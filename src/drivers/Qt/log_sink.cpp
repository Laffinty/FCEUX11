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

// v2.0.8 C2/C3 O1 -- the message-log file sink. See log_sink.h.

#include "Qt/log_sink.h"

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <system_error>

namespace fceux11 {

namespace {

constexpr std::size_t kMaxLines = 256;
/// Upper bound for the memory buffer that holds messages written before
/// the file could be opened. Beyond this the oldest pending messages are
/// dropped -- the file is the product, not the buffer.
constexpr std::size_t kMaxPendingBytes = 64 * 1024;

/// Directory name used under the emulator base directory.
const char* kLogDirName = "logs";
/// Log file name (no longer the upstream "fceux.log").
const char* kLogFileName = "fceux11_log.txt";

std::string temp_dir_path()
{
#ifdef _WIN32
	const char* t = std::getenv("TEMP");
	if (!t || !t[0])
		t = std::getenv("TMP");
	if (t && t[0])
		return std::string(t);
	return std::string("C:\\Windows\\Temp");
#else
	const char* t = std::getenv("TMPDIR");
	if (t && t[0])
		return std::string(t);
	return std::string("/tmp");
#endif
}

} // anonymous namespace

std::string derive_log_path(const std::string& base_dir)
{
	try
	{
		if (!base_dir.empty())
		{
			std::filesystem::path p(base_dir);
			p /= kLogDirName;
			p /= kLogFileName;
			return p.string();
		}
		std::filesystem::path p(temp_dir_path());
		p /= "fceux11";
		p /= kLogFileName;
		return p.string();
	}
	catch (const std::exception&)
	{
		// A path we cannot even build is not worth crashing over.
		return std::string(kLogFileName);
	}
}

LogSink& LogSink::instance()
{
	static LogSink sink;
	return sink;
}

void LogSink::open_for_base(const std::string& base_dir)
{
	const std::string target = derive_log_path(base_dir);

	std::lock_guard<std::mutex> lock(mtx);

	if (fp && path == target)
		return; // already there

	if (fp)
	{
		std::fclose(static_cast<FILE*>(fp));
		fp = nullptr;
	}

	std::error_code ec;
	std::filesystem::path p(target);
	if (p.has_parent_path())
		std::filesystem::create_directories(p.parent_path(), ec);
	if (ec)
	{
		// Cannot create the directory: keep buffering, do not fail the app.
		path.clear();
		return;
	}

	// "a" (append), never "w+": a restart must not truncate the previous
	// run's log.
	FILE* f = std::fopen(target.c_str(), "a");
	if (!f)
	{
		path.clear();
		return;
	}

	fp = f;
	path = target;

	// Flush whatever was buffered before the base directory was known.
	if (!pending.empty())
	{
		// The ring needs an entry for the flushed block as well, otherwise
		// the viewer would not show the start-up messages (they would be
		// in the file but outside the ring's window).
		std::fseek(f, 0L, SEEK_END);
		const long ofs = std::ftell(f);
		std::fwrite(pending.data(), 1, pending.size(), f);
		fpOfsList[head] = ofs;
		head = (head + 1) % kMaxLines;
		if (head == tail)
			tail = (tail + 1) % kMaxLines;
		pending.clear();
		std::fflush(f);
	}
}

void LogSink::append(const char* text, bool newline)
{
	if (!text)
		return;

	std::lock_guard<std::mutex> lock(mtx);

	std::string line(text);
	if (newline)
		line += '\n';

	if (!fp)
	{
		// Not open yet: buffer it (bounded) instead of losing it.
		pending += line;
		if (pending.size() > kMaxPendingBytes)
			pending.erase(0, pending.size() - kMaxPendingBytes);
		totalLines++;
		return;
	}

	FILE* f = static_cast<FILE*>(fp);
	std::fseek(f, 0L, SEEK_END);
	const long ofs = std::ftell(f);
	std::fwrite(line.data(), 1, line.size(), f);

	fpOfsList[head] = ofs;
	head = (head + 1) % kMaxLines;
	if (head == tail)
		tail = (tail + 1) % kMaxLines;
	totalLines++;
}

void LogSink::flush()
{
	std::lock_guard<std::mutex> lock(mtx);
	if (fp)
		std::fflush(static_cast<FILE*>(fp));
}

void LogSink::close()
{
	std::lock_guard<std::mutex> lock(mtx);
	if (!fp)
		return;
	std::fclose(static_cast<FILE*>(fp));
	fp = nullptr;
	path.clear();
	head = tail = 0;
}

std::string LogSink::tail_text(std::size_t maxBytes) const{
	std::lock_guard<std::mutex> lock(mtx);

	if (!fp || head == tail)
		return std::string();

	FILE* f = static_cast<FILE*>(fp);
	std::fseek(f, fpOfsList[tail], SEEK_SET);

	std::string out;
	out.resize(maxBytes);
	const std::size_t n = std::fread(&out[0], 1, maxBytes, f);
	out.resize(n);
	return out;
}

void LogSink::clear()
{
	std::lock_guard<std::mutex> lock(mtx);
	head = tail = 0;
}

std::size_t LogSink::line_count() const
{
	std::lock_guard<std::mutex> lock(mtx);
	return totalLines;
}

std::string LogSink::current_path() const
{
	std::lock_guard<std::mutex> lock(mtx);
	return path;
}

bool LogSink::is_open() const
{
	std::lock_guard<std::mutex> lock(mtx);
	return fp != nullptr;
}

} // namespace fceux11
