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

/// How many messages the in-memory ring keeps for the viewer.
constexpr std::size_t kMaxRingLines = 256;
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

/// "<path>.1", "<path>.2", ... -- the rotated generations.
std::string generation_path(const std::string& path, std::size_t generation)
{
	return path + "." + std::to_string(generation);
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
	writtenBytes = 0;

	// Flush whatever was buffered before the base directory was known.
	if (!pending.empty())
	{
		const std::size_t n = std::fwrite(pending.data(), 1, pending.size(), f);
		writtenBytes += n;
		pending.clear();
		std::fflush(f);
	}
}

void LogSink::append(const char* text, bool newline, bool urgent)
{
	if (!text)
		return;

	std::lock_guard<std::mutex> lock(mtx);

	std::string line(text);
	if (newline)
		line += '\n';

	// The in-memory ring always keeps the message: the message-log view
	// must keep working even when the file sink is switched off.
	ring.push_back(line);
	while (ring.size() > kMaxRingLines)
		ring.pop_front();
	totalLines++;

	if (!enabled_)
		return;

	if (!fp)
	{
		// Not open yet: buffer it (bounded) instead of losing it.
		pending += line;
		if (pending.size() > kMaxPendingBytes)
			pending.erase(0, pending.size() - kMaxPendingBytes);
		return;
	}

	FILE* f = static_cast<FILE*>(fp);
	const std::size_t n = std::fwrite(line.data(), 1, line.size(), f);
	writtenBytes += n;

	if (urgent)
		std::fflush(f); // errors must reach the disk right away

	if (writtenBytes >= maxFileBytes)
		rotate_locked();
}

void LogSink::rotate_locked()
{
	if (!fp)
		return;

	// Windows cannot rename an open file, so the handle has to go first.
	std::fclose(static_cast<FILE*>(fp));
	fp = nullptr;

	std::error_code ec;
	// Drop the oldest generation, then shift the others down -- highest
	// first, so a generation is never overwritten before it was moved.
	std::filesystem::remove(generation_path(path, kRotatedGenerations), ec);
	for (std::size_t g = kRotatedGenerations; g >= 2; g--)
	{
		std::filesystem::rename(generation_path(path, g - 1),
		                        generation_path(path, g), ec);
		if (ec)
			ec.clear();
	}
	std::filesystem::rename(path, generation_path(path, 1), ec);
	if (ec)
		ec.clear();

	// Re-open for appending: the fresh file starts empty.
	FILE* f = std::fopen(path.c_str(), "a");
	if (f)
	{
		fp = f;
		writtenBytes = 0;
	}
	else
	{
		path.clear();
	}
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
	writtenBytes = 0;
}

std::string LogSink::tail_text(std::size_t maxBytes) const
{
	std::lock_guard<std::mutex> lock(mtx);

	std::string out;
	// Walk backwards so that the cap keeps the newest messages.
	for (auto it = ring.rbegin(); it != ring.rend(); ++it)
	{
		if (out.size() + it->size() > maxBytes)
			break;
		out.insert(0, *it);
	}
	return out;
}

void LogSink::clear()
{
	std::lock_guard<std::mutex> lock(mtx);
	ring.clear();
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

void LogSink::set_enabled(bool on)
{
	std::lock_guard<std::mutex> lock(mtx);
	enabled_ = on;
}

bool LogSink::enabled() const
{
	std::lock_guard<std::mutex> lock(mtx);
	return enabled_;
}

void LogSink::set_max_file_bytes(std::size_t bytes)
{
	std::lock_guard<std::mutex> lock(mtx);
	maxFileBytes = bytes;
}

std::size_t LogSink::max_file_bytes() const
{
	std::lock_guard<std::mutex> lock(mtx);
	return maxFileBytes;
}

} // namespace fceux11
