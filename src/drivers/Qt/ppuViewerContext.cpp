// ppuViewerContext.cpp
//
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include <QFileDialog>
#include <QSettings>

#include "../../fceu.h"
#include "../../cart.h"
#include "../../ppu.h"
#include "../../debug.h"
#include "../../palette.h"
#include "../../types.h"

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ppuViewerContext.h"

#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"

ppuPatternTable_t pattern0;
ppuPatternTable_t pattern1;
oamPatternTable_t oamPattern;

uint8_t pallast[35]    = { 0 };
uint8_t palcache[36]   = { 0 };
uint8_t chrcache0[0x1000] = { 0 };
uint8_t chrcache1[0x1000] = { 0 };
uint8_t logcache0[0x1000] = { 0 };
uint8_t logcache1[0x1000] = { 0 };
uint8_t oam[256]        = { 0 };

bool redrawWindow            = true;
bool PPUView_maskUnusedGraphics = true;
bool PPUView_invertTheMask   = false;
int  PPUViewScanline         = 0;
int  PPUViewSkip             = 0;
int  PPUViewRefresh          = 1;
int  PPUView_sprite16Mode[2] = { 0, 0 };
int  pindex[2]               = { 0, 0 };

int conv2hex(int i)
{
	int h = 0;
	if (i >= 10)
	{
		h = 'A' + i - 10;
	}
	else
	{
		h = '0' + i;
	}
	return h;
}

int getPPU(unsigned int i)
{
	i &= 0x3FFF;
	if (i < 0x2000) return VPage[(i) >> 10][(i)];
	if (GameInfo->type == GIT_NSF)
		return 0;
	else
	{
		if (i < 0x3F00)
			return vnapage[(i >> 10) & 0x3][i & 0x3FF];
		return READPAL_MOTHEROFALL(i & 0x1F);
	}
	return 0;
}

void PalettePoke(uint32_t addr, uint8_t data)
{
	data = data & 0x3F;
	addr = addr & 0x1F;
	if ((addr & 3) == 0)
	{
		addr = (addr & 0xC) >> 2;
		if (addr == 0)
		{
			PALRAM[0x00] = PALRAM[0x04] = PALRAM[0x08] = PALRAM[0x0C] = data;
		}
		else
		{
			UPALRAM[addr - 1] = data;
		}
	}
	else
	{
		PALRAM[addr] = data;
	}
}

int writeMemPPU(unsigned int addr, int value)
{
	addr &= 0x3FFF;
	if (addr < 0x2000)
	{
		VPage[addr >> 10][addr] = value;
	}
	if ((addr >= 0x2000) && (addr < 0x3F00))
	{
		vnapage[(addr >> 10) & 0x3][addr & 0x3FF] = value;
	}
	if ((addr >= 0x3F00) && (addr < 0x3FFF))
	{
		PalettePoke(addr, value);
	}
	return 0;
}

void initPPUViewer(void)
{
	memset(pallast, 0, sizeof(pallast));
	memset(palcache, 0, sizeof(palcache));
	memset(chrcache0, 0, sizeof(chrcache0));
	memset(chrcache1, 0, sizeof(chrcache1));
	memset(logcache0, 0, sizeof(logcache0));
	memset(logcache1, 0, sizeof(logcache1));
	memset(oam, 0, sizeof(oam));

	palcache[(8 * 4) + 0] = 0x0F;
	palcache[(8 * 4) + 1] = 0x00;
	palcache[(8 * 4) + 2] = 0x10;
	palcache[(8 * 4) + 3] = 0x20;

	pindex[0] = 0;
	pindex[1] = 0;
}

void DrawPatternTable(ppuPatternTable_t *pattern, uint8_t *table, uint8_t *log, uint8_t pal)
{
	int i, j, x, y, index = 0;
	int p = 0, tmp;
	uint8_t chr0, chr1, logs, shift;

	if (palo == NULL)
	{
		return;
	}

	pal <<= 2;
	for (i = 0; i < 16; i++)
	{
		for (j = 0; j < 16; j++)
		{
			for (y = 0; y < 8; y++)
			{
				chr0 = table[index];
				chr1 = table[index + 8];
				logs = log[index] & log[index + 8];
				tmp = 7;
				shift = (PPUView_maskUnusedGraphics && debug_loggingCD && (((logs & 3) != 0) == PPUView_invertTheMask)) ? 3 : 0;
				for (x = 0; x < 8; x++)
				{
					p = (chr0 >> tmp) & 1;
					p |= ((chr1 >> tmp) & 1) << 1;

					pattern->tile[i][j].pixel[y][x].val = p;

					p = palcache[p | pal];
					tmp--;
					pattern->tile[i][j].pixel[y][x].color.setBlue(palo[p].b >> shift);
					pattern->tile[i][j].pixel[y][x].color.setGreen(palo[p].g >> shift);
					pattern->tile[i][j].pixel[y][x].color.setRed(palo[p].r >> shift);
				}
				index++;
			}
			index += 8;
		}
	}
}

void drawSpriteTable(void)
{
	int j = 0, y, x, yy, xx, p, tmp, idx, chr0, chr1, pal, t0, t1;
	uint8_t *chrcache;
	struct oamSpriteData_t *spr;

	if (palo == NULL)
	{
		return;
	}
	oamPattern.mode8x16 = (PPU[0] & 0x20) ? 1 : 0;

	for (int i = 0; i < 64; i++)
	{
		spr = &oamPattern.sprite[i];

		spr->y = (oam[j]);
		spr->x = (oam[j + 3]);
		spr->pal = (oam[j + 2] & 0x03) | 0x04;
		spr->pri = (oam[j + 2] & 0x20) ? 1 : 0;
		spr->hFlip = (oam[j + 2] & 0x40) ? 1 : 0;
		spr->vFlip = (oam[j + 2] & 0x80) ? 1 : 0;

		if (oamPattern.mode8x16)
		{
			spr->bank = (oam[j + 1] & 0x01);
			spr->tNum = (oam[j + 1] & 0xFE);
		}
		else
		{
			spr->bank = (PPU[0] & 0x08) ? 1 : 0;
			spr->tNum = (oam[j + 1]);
		}

		idx = spr->tNum << 4;

		if (spr->bank)
		{
			chrcache = chrcache1;
			spr->chrAddr = 0x1000 + idx;
		}
		else
		{
			chrcache = chrcache0;
			spr->chrAddr = idx;
		}

		if (oamPattern.mode8x16 && spr->vFlip)
		{
			t0 = 1; t1 = 0;
		}
		else
		{
			t0 = 0; t1 = 1;
		}

		pal = spr->pal * 4;

		for (yy = 0; yy < 8; yy++)
		{
			if (spr->vFlip)
			{
				y = 7 - yy;
			}
			else
			{
				y = yy;
			}

			chr0 = chrcache[idx];
			chr1 = chrcache[idx + 8];
			tmp = 7;

			for (xx = 0; xx < 8; xx++)
			{
				if (spr->hFlip)
				{
					x = 7 - xx;
				}
				else
				{
					x = xx;
				}

				p = (chr0 >> tmp) & 1;
				p |= ((chr1 >> tmp) & 1) << 1;

				spr->tile[t0].pixel[y][x].val = p;

				p = palcache[p | pal];
				tmp--;
				spr->tile[t0].pixel[y][x].color.setBlue(palo[p].b);
				spr->tile[t0].pixel[y][x].color.setGreen(palo[p].g);
				spr->tile[t0].pixel[y][x].color.setRed(palo[p].r);
			}
			idx++;
		}
		idx += 8;

		for (yy = 0; yy < 8; yy++)
		{
			if (spr->vFlip)
			{
				y = 7 - yy;
			}
			else
			{
				y = yy;
			}
			chr0 = chrcache[idx];
			chr1 = chrcache[idx + 8];
			tmp = 7;

			for (xx = 0; xx < 8; xx++)
			{
				if (spr->hFlip)
				{
					x = 7 - xx;
				}
				else
				{
					x = xx;
				}

				p = (chr0 >> tmp) & 1;
				p |= ((chr1 >> tmp) & 1) << 1;

				spr->tile[t1].pixel[y][x].val = p;

				p = palcache[p | pal];
				tmp--;
				spr->tile[t1].pixel[y][x].color.setBlue(palo[p].b);
				spr->tile[t1].pixel[y][x].color.setGreen(palo[p].g);
				spr->tile[t1].pixel[y][x].color.setRed(palo[p].r);
			}
			idx++;
		}
		idx += 8;

		j += 4;
	}
}

int exportActivePaletteACT(const char *filename)
{
	FILE *fp;
	int i = 0, c, ret = 0, numBytes;
	unsigned char buf[768];

	fp = fopen(filename, "wb");

	if (fp == NULL)
	{
		return -1;
	}
	memset(buf, 0, sizeof(buf));

	i = 0;
	for (int p = 0; p < 32; p++)
	{
		c = palcache[p];

		if (palo)
		{
			buf[i] = palo[c].r; i++;
			buf[i] = palo[c].g; i++;
			buf[i] = palo[c].b; i++;
		}
	}

	numBytes = ::fwrite(buf, 1, 768, fp);

	if (numBytes != 768)
	{
		printf("Error Failed to Export Palette\n");
		ret = -1;
	}
	::fclose(fp);

	return ret;
}
