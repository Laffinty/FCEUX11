/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2002 Xodnizel
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

#include <string.h>
#include <stdlib.h>
#include "share.h"

// D9 A5 was withdrawn (2026-11-06 review): registering the report state in
// FCEUCTRL_STATEINFO drifted the frozen savestate baselines
// (golden_savestate_test + golden_savestate_hashes.json) by +22 bytes per
// CTRL chunk, so the struct stays file-local and is NOT savestate-serialized.
typedef struct {
	bool strobe;
	uint32 latch; // latched data (read when strobe goes high to low)
	uint32 sensitivity; // reading while strobe is high cycles sensitivity 0,1,2
	int32 dx, dy; // relative position to communicate
	int32 mb; // current buttons
	uint8 sig; // D9 A1: report signature phase (alternates 0xFF/0x00 per report)
} SNES_MOUSE;

static SNES_MOUSE SNESMouse;

static uint8 ReadSNESMouse(int w)
{
	if (SNESMouse.strobe)
	{
		SNESMouse.sensitivity += 1;
		if (SNESMouse.sensitivity > 2) SNESMouse.sensitivity = 0;
	}

	uint8 result = (SNESMouse.latch & 0x80000000) >> 31;
	SNESMouse.latch = (SNESMouse.latch << 1);

	return result;
}

static void WriteSNESMouse(uint8 v)
{
	bool strobing = v & 1;

	if (SNESMouse.strobe && !strobing)
	{
		int dx = SNESMouse.dx;
		int dy = SNESMouse.dy;
		SNESMouse.dx = 0;
		SNESMouse.dy = 0;

		// convert to sign and magnitude
		bool sx = (dx < 0);
		bool sy = (dy < 0);
		if (dx < 0) dx = -dx;
		if (dy < 0) dy = -dy;

		// D9 A2/A3: apply the sensitivity ratio. The hardware ratios are
		// x1/x2/x4 (a plain shift by the sensitivity step), and dy must be
		// scaled symmetrically with dx — the previous code doubled the dx
		// application and never touched dy.
		uint32 sens = SNESMouse.sensitivity;
		if (sens > 2) sens = 2; // defensive: read path cycles 0..2
		if (sens > 0) {
			dx <<= sens;
			dy <<= sens;
		}

		// D9 A4: the report carries a 6-bit magnitude; anything beyond 63
		// saturates at 63 and sets the overflow bit (bit6) instead of
		// wrapping into a 7-bit field.
		bool ox = (dx > 63);
		bool oy = (dy > 63);
		if (ox) dx = 63;
		if (oy) dy = 63;

		// D9 A1: the signature byte alternates 0xFF/0x00 on every report
		// (strobe falling edge) so the game can recognise the start of a
		// new report. First report carries 0xFF (polarity pending mset
		// verification — see D9 report).
		uint8 byte0 = SNESMouse.sig ? 0x00 : 0xFF;
		SNESMouse.sig ^= 1;

		uint8 byte1 =
			0x1 | // signature
			((SNESMouse.sensitivity & 3) << 4) | // sensitivity
			((SNESMouse.mb & 3) << 6); // buttons
		uint8 byte2 = uint8(dy) | (oy ? 0x40 : 0x00) | (sy ? 0x80 : 0x00);
		uint8 byte3 = uint8(dx) | (ox ? 0x40 : 0x00) | (sx ? 0x80 : 0x00);

		SNESMouse.latch = (byte0 << 24) | (byte1 << 16) | (byte2 << 8) | (byte3 << 0);
	}

	SNESMouse.strobe = strobing;
}

static void UpdateSNESMouse(int w, void *data, int arg)
{
	int32 *ptr=(int32*)data;

	SNESMouse.dx += ptr[0]; ptr[0] = 0;
	SNESMouse.dy += ptr[1]; ptr[1] = 0;
	SNESMouse.mb = ptr[2] & 3; // bit 0 = left button, bit 1 = right button
}

static INPUTC SNES_MOUSEC =
{
	ReadSNESMouse, // Read
	WriteSNESMouse, // Write
	0, // Strobe (handled by Write)
	UpdateSNESMouse, // Update
	0, // SLHook
	0, // Draw
	0, // Log
	0, // Load
};

INPUTC *FCEU_InitSNESMouse(int w)
{
	memset(&SNESMouse,0,sizeof(SNESMouse));
	return(&SNES_MOUSEC);
}
