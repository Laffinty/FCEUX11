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

// FCEUX11 v1.11 Bridge — fceu_globals.cpp
// Global variable definitions shared between core and Qt driver.
// Split from fceuWrapper.cpp per v1.11_bridge_build_plan.md §4.1.

#include <stdint.h>

// Core global variables — shared between fceuWrapper modules and FCEU core
int  dendy = 0;
int  eoptions = 0;
int  isloaded = 0;
int  pal_emulation = 0;
int  gametype = 0;
int  closeFinishedMovie = 0;
int  KillFCEUXonFrame = 0;

bool turbo = false;
bool pauseAfterPlayback = false;
bool suggestReadOnlyReplay = true;
bool showStatusIconOpt = true;
bool drawInputAidsEnable = true;
bool usePaletteForVideoBg = false;
unsigned int gui_draw_area_width  = 256;
unsigned int gui_draw_area_height = 256;

#include "common/configSys.h"
Config *g_config = NULL;

bool g_noConsole = false;

unsigned int emulatorCycleCount = 0;

#ifdef CREATE_AVI
int mutecapture = 0;
#endif
