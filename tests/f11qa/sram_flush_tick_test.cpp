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

// v2.0.8 C1 P3 — periodic SRAM flush decision (driver-side gating logic).
//
// The decision helper is header-only so it can be exercised without a
// running emulator; the emulation thread calls it once per frame.
//
// Legs:
//   T1: interval <= 0 disables the flush entirely (never due).
//   T2: the first call only arms the baseline -- it must NOT flush on the
//       frame right after a ROM was loaded.
//   T3: a flush is due once the wall clock advanced by the interval.
//   T4: repeated calls before the interval elapses stay not-due.
//   T5: after a flush the baseline moves -- the next flush is a full
//       interval later, not immediately.
//   T6: re-enabling the interval re-arms instead of flushing at once.

#include <chrono>
#include <cstdio>

#include "Qt/sram_flush.h"

using clk = std::chrono::steady_clock;

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
	// ---- T1: disabled ----
	{
		clk::time_point last{};
		bool armed = false;
		bool due = fceux11::sram_flush_due(clk::now(), last, 0, armed);
		CHECK(!due, "T1a interval=0 never due");
		CHECK(!armed, "T1b interval=0 disarms");
		// Even far in the future it stays off.
		clk::time_point far = clk::now() + std::chrono::hours(24);
		due = fceux11::sram_flush_due(far, last, -5, armed);
		CHECK(!due, "T1c negative interval never due");
	}

	// ---- T2: first call arms, does not flush ----
	{
		clk::time_point last{};
		bool armed = false;
		const clk::time_point t0 = clk::now();
		bool due = fceux11::sram_flush_due(t0, last, 30, armed);
		CHECK(!due, "T2a first call does not flush");
		CHECK(armed, "T2b first call arms");
	}

	// ---- T3/T4: due only after the interval ----
	{
		clk::time_point last{};
		bool armed = false;
		const clk::time_point t0 = clk::now();
		fceux11::sram_flush_due(t0, last, 30, armed);

		bool due = fceux11::sram_flush_due(t0 + std::chrono::seconds(29), last, 30, armed);
		CHECK(!due, "T3a not due one second before the interval");

		due = fceux11::sram_flush_due(t0 + std::chrono::seconds(31), last, 30, armed);
		CHECK(due, "T3b due one second after the interval");
	}

	// ---- T5: baseline moves after a flush ----
	{
		clk::time_point last{};
		bool armed = false;
		const clk::time_point t0 = clk::now();
		fceux11::sram_flush_due(t0, last, 30, armed);

		const clk::time_point t1 = t0 + std::chrono::seconds(31);
		bool due = fceux11::sram_flush_due(t1, last, 30, armed);
		CHECK(due, "T5a flush fires at the interval");
		// Immediately after, it must not fire again.
		due = fceux11::sram_flush_due(t1 + std::chrono::milliseconds(1), last, 30, armed);
		CHECK(!due, "T5b no second flush right after");
	}

	// ---- T6: re-enabling re-arms ----
	{
		clk::time_point last{};
		bool armed = false;
		const clk::time_point t0 = clk::now();
		fceux11::sram_flush_due(t0, last, 0, armed);   // disabled
		CHECK(!armed, "T6a disabled disarms");
		bool due = fceux11::sram_flush_due(t0 + std::chrono::seconds(600), last, 30, armed);
		CHECK(!due, "T6b re-enabling re-arms instead of flushing at once");
		CHECK(armed, "T6c re-enabling arms again");
	}

	if (failures == 0)
	{
		std::printf("PASS: sram_flush_tick_test all legs\n");
		return 0;
	}
	std::printf("FAIL: sram_flush_tick_test %d leg(s)\n", failures);
	return 1;
}
