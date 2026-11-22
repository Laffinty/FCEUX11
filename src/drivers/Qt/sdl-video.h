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
#ifndef __FCEU_SDL_VIDEO_H
#define __FCEU_SDL_VIDEO_H
#ifdef _SDL2
#include <SDL2/SDL.h>
#else
#include <SDL.h>
#endif

uint32 PtoV(double x, double y);
bool FCEUD_ShouldDrawInputAids();
namespace fceu11 {
    bool AviDisableMovieMessages();
    bool AviEnableHUDrecording();
    void SetAviEnableHUDrecording(bool enable);
    void SetAviDisableMovieMessages(bool disable);
} // namespace fceu11
#endif

