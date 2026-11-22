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
// mapinc_base.h - v1.8 Masonry §4 split
// Phase B: minimal base includes used by every board file.
// Provides: integer types (uint8/16/32), CartInfo struct, FCEU_gmalloc/gfree,
//           SetReadHandler/SetWriteHandler, stdio, string.
#ifndef FCEU11_MAPINC_BASE_H
#define FCEU11_MAPINC_BASE_H

#include "../types.h"
#include "../utils/memory.h"
#include "../fceu.h"
#include "../cart.h"
#include "../cheat.h"
#include <stdio.h>
#include <string.h>

#endif