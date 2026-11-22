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
// ppuViewerContext.h
//
#pragma once

#include <stdint.h>
#include <QColor>
#include <QWidget>

struct ppuPatternTable_t
{
	struct
	{
		struct
		{
			QColor color;
			char   val;
		} pixel[8][8];

		int  x;
		int  y;

	} tile[16][16];

	int  w;
	int  h;
};

struct oamSpriteData_t
{
	struct
	{
		struct
		{
			QColor color;
			char   val;
		} pixel[8][8];

	} tile[2];

	uint8_t tNum;
	uint8_t bank;
	uint8_t pal;
	uint8_t pri;
	uint8_t hFlip;
	uint8_t vFlip;
	int     chrAddr;
	int     x;
	int     y;
};

struct oamPatternTable_t
{
	struct oamSpriteData_t sprite[64];

	bool  mode8x16;
	int  w;
	int  h;
};

extern ppuPatternTable_t pattern0;
extern ppuPatternTable_t pattern1;
extern oamPatternTable_t oamPattern;

extern uint8_t pallast[35];
extern uint8_t palcache[36];
extern uint8_t chrcache0[0x1000];
extern uint8_t chrcache1[0x1000];
extern uint8_t logcache0[0x1000];
extern uint8_t logcache1[0x1000];
extern uint8_t oam[256];

extern bool redrawWindow;
extern bool PPUView_maskUnusedGraphics;
extern bool PPUView_invertTheMask;
extern int  PPUViewScanline;
extern int  PPUViewSkip;
extern int  PPUViewRefresh;
extern int  PPUView_sprite16Mode[2];
extern int  pindex[2];

int conv2hex(int i);
int getPPU(unsigned int i);
void PalettePoke(uint32_t addr, uint8_t data);
int writeMemPPU(unsigned int addr, int value);
void initPPUViewer(void);
void DrawPatternTable(ppuPatternTable_t *pattern, uint8_t *table, uint8_t *log, uint8_t pal);
void drawSpriteTable(void);
int exportActivePaletteACT(const char *filename);
