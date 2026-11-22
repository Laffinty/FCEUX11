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
#ifndef __FCEU_SDL_H
#define __FCEU_SDL_H

#if _SDL2
#include <SDL2/SDL.h>
#else
#include <SDL.h>
#endif

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"

// I'm using this as a #define so the compiler can optimize the
// modulo operation
#define PERIODIC_SAVE_INTERVAL 5000 // milliseconds

const int INVALID_STATE = 99;

extern int noGui;
extern int isloaded;

extern int dendy;
extern int pal_emulation;
extern bool& swapDuty;

int LoadGame(const char *path, bool silent);
int CloseGame(void);
void FCEUD_Update(uint8 *XBuf, int32 *Buffer, int Count);
uint64 FCEUD_GetTime();

#endif
