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
