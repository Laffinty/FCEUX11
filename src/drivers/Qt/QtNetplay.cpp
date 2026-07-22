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

// hotfix4 D-8: NetPlay formally removed (see docs/FCEUX11-1.15_LTS-hotfix4-PLAN.md).
// This file now provides only the symbol stubs required by
// fceu_callbacks.cpp (which still wires qNetplay_* into the driver
// callback table). Network state is no longer reachable: ROM loader
// never calls FCEUD_NetworkConnect, and no NetPlay menu exists.
// Core-side src/netplay.cpp is intentionally untouched to preserve
// ABI / savestate compatibility.

#include <cstdint>

// Match the originals' signatures exactly so the callback registration
// in fceu_callbacks.cpp links unchanged.
int qNetplay_SendData(void * /*data*/, uint32_t /*len*/)
{
	return 0;
}

int qNetplay_RecvData(void * /*data*/, uint32_t /*len*/)
{
	return 0;
}

void qNetplay_NetplayText(uint8_t * /*text*/)
{
}

void qNetplay_NetworkClose(void)
{
}
