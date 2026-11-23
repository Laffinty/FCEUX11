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

// v2.0.8 C1 (P3) -- periodic battery-SRAM flush decision.
//
// The decision lives in a header so it can be unit-tested without a running
// emulator: the driver thread calls it once per frame and flushes only when
// it says so. Rules:
//   * interval_seconds <= 0  -> disabled: never due (and the armed flag is
//     cleared, so re-enabling the interval re-arms instead of flushing on
//     the very next frame).
//   * first call after arming -> records the baseline and reports NOT due.
//     This is the "armed" rule: a ROM that was just loaded must not be
//     written out on frame 0.
//   * otherwise -> due once the wall clock has advanced by the interval.
//
// The clock is a steady (wall) clock on purpose: with a frame clock, turbo
// would turn the interval into "N frames" and hammer the disk.

#ifndef __FCEUX11_SRAM_FLUSH_H_
#define __FCEUX11_SRAM_FLUSH_H_

#include <chrono>

namespace fceux11 {

/// Decides whether the periodic SRAM flush is due.
/// @param now             current wall-clock reading
/// @param last            baseline timestamp (updated when a flush fires)
/// @param interval_seconds flush period in seconds; <= 0 disables the flush
/// @param armed           in/out arming flag
/// @return true when a flush should happen now
inline bool sram_flush_due(std::chrono::steady_clock::time_point now,
                            std::chrono::steady_clock::time_point& last,
                            int interval_seconds,
                            bool& armed)
{
	if (interval_seconds <= 0)
	{
		armed = false;
		return false;
	}

	if (!armed)
	{
		armed = true;
		last = now;
		return false;
	}

	if (now - last < std::chrono::seconds(interval_seconds))
		return false;

	last = now;
	return true;
}

} // namespace fceux11

#endif // __FCEUX11_SRAM_FLUSH_H_
