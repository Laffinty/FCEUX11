/* FCE Ultra - NES/Famicom Emulator
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
// mapinc_bus.h - v1.8 Masonry §4 split
// Phase B: pulls in mapinc_state.h plus x6502.h (CPU access macros).
// Provides: setprg*/setchr*/setmirror*, getRealAddress, addressing macros.
// Default include for any bank-switching board (97 + 1 = 98 of 171 files).
#ifndef FCEU11_MAPINC_BUS_H
#define FCEU11_MAPINC_BUS_H

#include "mapinc_state.h"
#include "../x6502.h"

#endif