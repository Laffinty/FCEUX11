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

/// \file
/// \brief Handles the graphical game display for the SDL implementation.

#include <array>

#include "Qt/sdl.h"
#include "Qt/nes_shm.h"
#include "common/vidblit.h"
#include "../../fceu.h"
#include "../../version.h"
#include "../../video.h"
#include "../../input.h"

#include "utils/memory.h"

#include "Qt/dface.h"

#include "common/configSys.h"
#include "Qt/sdl-video.h"
#include "Qt/AviRecord.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleWindow.h"

#ifdef CREATE_AVI
#endif

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <iostream>

// GLOBALS
extern Config *g_config;

// STATIC GLOBALS
static int s_curbpp = 0;
static int s_srendline, s_erendline;
static int s_tlines;
static int s_inited = 0;

static int s_eefx = 0;
static int s_clipSides = 0;
static int s_fullscreen = 0;
static int noframe = 0;
static int initBlitToHighDone = 0;

#define NWIDTH	(256 - (s_clipSides ? 16 : 0))
#define NOFFSET	(s_clipSides ? 8 : 0)

static int s_paletterefresh = 1;

extern bool MaxSpeed;
extern int input_display;
extern int frame_display;
extern int rerecord_display;
extern std::array<uint8_t, 0x20> PALRAM;

/**
 * Attempts to destroy the graphical video display.  Returns 0 on
 * success, -1 on failure.
 */
int
KillVideo(void)
{
	//printf("Killing Video\n");

	if ( nes_shm != NULL )
	{
		nes_shm->clear_pixbuf();
	}

	// if the rest of the system has been initialized, shut it down
	// shut down the system that converts from 8 to 16/32 bpp
	if (initBlitToHighDone)
	{
		KillBlitToHigh();

		initBlitToHighDone = 0;
	}

	// return failure if the video system was not initialized
	if (s_inited == 0)
	{
		return -1;
	}

	// SDL Video system is not used.
	// shut down the SDL video sub-system
	//SDL_QuitSubSystem(SDL_INIT_VIDEO);

	s_inited = 0;
	return 0;
}


// this variable contains information about the special scaling filters
static int s_sponge = 0;

void fceWrapper_VideoChanged(void)
{
	int buf;
	if (g_config)
		g_config->getOption("SDL.PAL", &buf);
	else
		buf = 0;
	if(buf == 1)
		PAL = 1;
	else
		PAL = 0; // NTSC and Dendy
}

void CalcVideoDimensions(void)
{
	g_config->getOption("SDL.SpecialFilter", &s_sponge);

	fceu11::GetCurrentVidSystem(&s_srendline, &s_erendline);
	s_tlines = s_erendline - s_srendline + 1;

	//printf("Calc Video: %i -> %i \n", s_srendline, s_erendline );

	nes_shm->video.preScaler.store(s_sponge, std::memory_order_release);

	switch ( s_sponge )
	{
		default:
		case 0: // None
			nes_shm->video.xscale.store(1, std::memory_order_release);
			nes_shm->video.yscale.store(1, std::memory_order_release);
		break;
		case 1: // hq2x
		case 2: // Scale2x
		case 3: // NTSC 2x
		case 6: // Prescale2x
			nes_shm->video.xscale.store(2, std::memory_order_release);
			nes_shm->video.yscale.store(2, std::memory_order_release);
		break;
		case 4: // hq3x
		case 5: // Scale3x
		case 7: // Prescale3x
			nes_shm->video.xscale.store(3, std::memory_order_release);
			nes_shm->video.yscale.store(3, std::memory_order_release);
		break;
		case 8: // Prescale4x
			nes_shm->video.xscale.store(4, std::memory_order_release);
			nes_shm->video.yscale.store(4, std::memory_order_release);
		break;
		case 9: // PAL
			nes_shm->video.xscale.store(3, std::memory_order_release);
			nes_shm->video.yscale.store(1, std::memory_order_release);
		break;
	}

	int iScale = nes_shm->video.xscale.load(std::memory_order_acquire);
	if ( s_sponge == 3 )
	{
		nes_shm->video.ncol.store(iScale*301, std::memory_order_release);
	}
	else
	{
		nes_shm->video.ncol.store(iScale*NWIDTH, std::memory_order_release);
	}
	if ( s_sponge == 9 )
	{
		nes_shm->video.nrow.store(1*s_tlines, std::memory_order_release);
		nes_shm->video.xyRatio.store(3, std::memory_order_release);
	}
	else
	{
		nes_shm->video.nrow.store(iScale*s_tlines, std::memory_order_release);
		nes_shm->video.xyRatio.store(1, std::memory_order_release);
	}
	nes_shm->video.pitch.store(nes_shm->video.ncol.load(std::memory_order_acquire) * 4, std::memory_order_release);

	// hotfix1 P1-7 (C-08): the destination buffer is `pixbuf[5][1048576]`
	// (1024×1024 uint32), so any ncol×nrow product beyond 1048576 is a
	// guaranteed scribble past the end. With the largest configured
	// scale (4×) and the largest visible line range (PAL ≈ 312 lines),
	// peak ncol × nrow should fit in 1024×1024, but a corrupt
	// configuration (scale "5", manual override, or future custom
	// filter) could blow past it. Clamp to 1024 and refuse the
	// calc — every consumer checks the bounds first.
	if ((long long)nes_shm->video.ncol.load(std::memory_order_acquire) * nes_shm->video.nrow.load(std::memory_order_acquire) > 1048576LL) {
		// hotfix1 P1-7 (C-08): report the mismatch via a transient std::cerr
		// rather than FCEUD_PrintError, which only takes a single message
		// argument; using a 3-arg printf-like form here would fail to
		// compile. The diagnostic is fire-and-forget on the first
		// overrun; after the clamp below, the rest of the program sees
		// a sane 1024×1024 surface.
		std::cerr << "video dimensions " << nes_shm->video.ncol.load(std::memory_order_acquire)
		          << "x" << nes_shm->video.nrow.load(std::memory_order_acquire)
		          << " exceed 1024x1024 framebuffer; clamping" << std::endl;
		nes_shm->video.ncol.store(1024, std::memory_order_release);
		nes_shm->video.nrow.store(1024, std::memory_order_release);
		nes_shm->video.pitch.store(4096, std::memory_order_release);
	}
}

int InitVideo(FCEUGI *gi)
{
	int doublebuf, xstretch, ystretch;
	int show_fps;
	int startNTSC, endNTSC, startPAL, endPAL;

	FCEU_printf("Initializing video...");

	// load the relevant configuration variables
	g_config->getOption("SDL.Fullscreen", &s_fullscreen);
	g_config->getOption("SDL.DoubleBuffering", &doublebuf);
	g_config->getOption("SDL.SpecialFilter", &s_sponge);
	g_config->getOption("SDL.XStretch", &xstretch);
	g_config->getOption("SDL.YStretch", &ystretch);
	g_config->getOption("SDL.ClipSides", &s_clipSides);
	g_config->getOption("SDL.NoFrame", &noframe);
	g_config->getOption("SDL.ShowFPS", &show_fps);
	g_config->getOption("SDL.ShowFrameCount", &frame_display);
	g_config->getOption("SDL.ShowLagCount", &lagCounterDisplay);
	g_config->getOption("SDL.ShowRerecordCount", &rerecord_display);
	g_config->getOption("SDL.ShowGuiMessages", &vidGuiMsgEna);
	g_config->getOption("SDL.ScanLineStartNTSC", &startNTSC);
	g_config->getOption("SDL.ScanLineEndNTSC", &endNTSC);
	g_config->getOption("SDL.ScanLineStartPAL", &startPAL);
	g_config->getOption("SDL.ScanLineEndPAL", &endPAL);
	uint32_t  rmask, gmask, bmask;

	ClipSidesOffset = s_clipSides ? 8 : 0;

	fceu11::SetRenderedLines(startNTSC, endNTSC, startPAL, endPAL);

	// check the starting, ending, and total scan lines

	fceu11::GetCurrentVidSystem(&s_srendline, &s_erendline);
	s_tlines = s_erendline - s_srendline + 1;

	nes_shm->video.preScaler.store(s_sponge, std::memory_order_release);

	switch ( s_sponge )
	{
		default:
		case 0: // None
			nes_shm->video.xscale.store(1, std::memory_order_release);
			nes_shm->video.yscale.store(1, std::memory_order_release);
		break;
		case 1: // hq2x
		case 2: // Scale2x
		case 3: // NTSC 2x
		case 6: // Prescale2x
			nes_shm->video.xscale.store(2, std::memory_order_release);
			nes_shm->video.yscale.store(2, std::memory_order_release);
		break;
		case 4: // hq3x
		case 5: // Scale3x
		case 7: // Prescale3x
			nes_shm->video.xscale.store(3, std::memory_order_release);
			nes_shm->video.yscale.store(3, std::memory_order_release);
		break;
		case 8: // Prescale4x
			nes_shm->video.xscale.store(4, std::memory_order_release);
			nes_shm->video.yscale.store(4, std::memory_order_release);
		break;
		case 9: // PAL
			nes_shm->video.xscale.store(3, std::memory_order_release);
			nes_shm->video.yscale.store(1, std::memory_order_release);
		break;
	}
	nes_shm->render_count.store(0, std::memory_order_relaxed); nes_shm->blit_count.store(0, std::memory_order_relaxed);

	s_inited = 1;

	// check to see if we are showing FPS
	FCEUI_SetShowFPS(show_fps);

	int iScale = nes_shm->video.xscale.load(std::memory_order_acquire);
	if ( s_sponge == 3 )
	{
		nes_shm->video.ncol.store(iScale*301, std::memory_order_release);
	}
	else
	{
		nes_shm->video.ncol.store(iScale*NWIDTH, std::memory_order_release);
	}
	if ( s_sponge == 9 )
	{
		nes_shm->video.nrow.store(1*s_tlines, std::memory_order_release);
		nes_shm->video.xyRatio.store(3, std::memory_order_release);
	}
	else
	{
		nes_shm->video.nrow.store(iScale*s_tlines, std::memory_order_release);
		nes_shm->video.xyRatio.store(1, std::memory_order_release);
	}
	nes_shm->video.pitch.store(nes_shm->video.ncol.load(std::memory_order_acquire) * 4, std::memory_order_release);

#ifdef FCEU_BIG_ENDIAN
	rmask = 0x00FF0000;
	gmask = 0x0000FF00;
	bmask = 0x000000FF;
#else
	rmask = 0x00FF0000;
	gmask = 0x0000FF00;
	bmask = 0x000000FF;
#endif

	s_curbpp = 32; // Bits per pixel is always 32

	FCEU_printf(" Video Mode: %d x %d x %d bpp %s\n",
				nes_shm->video.ncol.load(std::memory_order_acquire), nes_shm->video.nrow.load(std::memory_order_acquire), s_curbpp,
				s_fullscreen ? "full screen" : "");

	if (s_curbpp != 8 && s_curbpp != 16 && s_curbpp != 24 && s_curbpp != 32) 
	{
		FCEU_printf("  Sorry, %dbpp modes are not supported by FCE Ultra.  Supported bit depths are 8bpp, 16bpp, and 32bpp.\n", s_curbpp);
		KillVideo();
		return -1;
	}

	if ( !initBlitToHighDone )
	{
		InitBlitToHigh(s_curbpp >> 3,
							rmask,
							gmask,
							bmask,
							s_eefx, s_sponge, 0);

		initBlitToHighDone = 1;
	}

	s_paletterefresh = 1;

	return 0;
}

/**
 * Toggles the full-screen display.
 */
void ToggleFS(void)
{
    // pause while we we are making the switch
	bool paused = fceu11::IsEmulationPaused();
	if(!paused)
		fceu11::ToggleEmulationPause();

	// flip the fullscreen flag
	g_config->setOption("SDL.Fullscreen", !s_fullscreen);

	// TODO Call method to make full Screen

	// if we paused to make the switch; unpause
	if(!paused)
		fceu11::ToggleEmulationPause();
}

static SDL_Color s_psdl[256];

/**
 * Sets the color for a particular index in the palette.
 */
void
fceWrapper_SetPalette(uint8 index,
                 uint8 r,
                 uint8 g,
                 uint8 b)
{
	s_psdl[index].r = r;
	s_psdl[index].g = g;
	s_psdl[index].b = b;

	s_paletterefresh = 1;
}

/**
 * Gets the color for a particular index in the palette.
 */
void
fceWrapper_GetPalette(uint8 index,
				uint8 *r,
				uint8 *g,
				uint8 *b)
{
	*r = s_psdl[index].r;
	*g = s_psdl[index].g;
	*b = s_psdl[index].b;
}

/** 
 * Pushes the palette structure into the underlying video subsystem.
 */
static void RedoPalette()
{
	if (s_curbpp > 8) 
	{
		//printf("Refresh Palette\n");
		SetPaletteBlitToHigh((uint8*)s_psdl);
	} 
}
// XXX soules - console lock/unlock unimplemented?

///Currently unimplemented.
void LockConsole(){}

///Currently unimplemented.
void UnlockConsole(){}

static void vsync_test(void)
{
	int i, j, k, l;
	int cycleLen, halfCycleLen;
	static int ofs = 0;
	uint32_t *pixbuf;

	pixbuf = nes_shm->pixBufPool.slot( nes_shm->pixBufIdx.load(std::memory_order_acquire) );

	cycleLen = nes_shm->video.ncol.load(std::memory_order_acquire) / 4;

	halfCycleLen = cycleLen / 2;

	k=0;
	for (j=0; j<nes_shm->video.nrow.load(std::memory_order_acquire); j++)
	{
		for (i=0; i<nes_shm->video.ncol.load(std::memory_order_acquire); i++)
		{
			l = ((i+ofs) % cycleLen);

			if ( l < halfCycleLen )
			{
				pixbuf[k] = 0xFFFFFFFF; k++;
			}
			else
			{
				pixbuf[k] = 0x00000000; k++;
			}
		}
	}
	ofs = (ofs + 1) % nes_shm->video.ncol.load(std::memory_order_acquire);
}

static void
doBlitScreen(uint8_t *XBuf, uint8_t *dest)
{
	int w, h, pitch, bw, ixScale, iyScale;

	// refresh the palette if required
	if (s_paletterefresh) 
	{
		RedoPalette();
		s_paletterefresh = 0;
	}

	// XXX soules - not entirely sure why this is being done yet
	XBuf += s_srendline * 256;

	//dest    = (uint8*)nes_shm->pixbuf;
	ixScale = nes_shm->video.xscale.load(std::memory_order_acquire);
	iyScale = nes_shm->video.yscale.load(std::memory_order_acquire);

	if ( s_sponge == 3 )
	{
		w = ixScale*301;
		bw = 256;
	}
	else
	{
		w = ixScale*NWIDTH;
		bw = NWIDTH;
	}
	if ( s_sponge == 9 )
	{
		h  = 1*s_tlines;
	}
	else
	{
		h  = ixScale*s_tlines;
	}
	pitch  = w*4;

	nes_shm->video.ncol.store(w, std::memory_order_release);
	nes_shm->video.nrow.store(h, std::memory_order_release);
	nes_shm->video.pitch.store(pitch, std::memory_order_release);
	nes_shm->video.preScaler.store(s_sponge, std::memory_order_release);

	if ( dest == NULL ) return;

	if ( nes_shm->video.test.load(std::memory_order_acquire) )
	{
		switch ( nes_shm->video.test.load(std::memory_order_acquire) )
		{
			case 1:
				vsync_test();
			break;
			default:
				// Unknown Test Pattern
			break;
		}
	}
	else
	{
		Blit8ToHigh(XBuf + NOFFSET, dest, bw, s_tlines, pitch, ixScale, iyScale);
	}
}
/**
 * Pushes the given buffer of bits to the screen.
 */
void
BlitScreen(uint8 *XBuf)
{
	int i = nes_shm->pixBufIdx.load(std::memory_order_acquire);

	if (usePaletteForVideoBg)
	{
		unsigned char r, g, b;
		FCEUD_GetPalette(0x80 | PALRAM[0], &r, &g, &b);

		if (consoleWindow)
		{
			QColor *bgColor = consoleWindow->getVideoBgColorPtr();

			*bgColor = QColor::fromRgb(r,g,b);
		}
	}

	doBlitScreen(XBuf, (uint8_t*)nes_shm->pixBufPool.slot(i));

	nes_shm->pixBufIdx.store( (i+1) % NES_VIDEO_BUFLEN, std::memory_order_release );
	nes_shm->blit_count.fetch_add(1, std::memory_order_relaxed);
	nes_shm->blitUpdated.store(1, std::memory_order_release);
}

void fceu11::AviVideoUpdate(const unsigned char* buffer)
{	// This is not used by Qt Emulator, avi recording pulls from the post processed video buffer
	// instead of emulation core video buffer. This allows for the video scaler effects
	// and higher resolution to be seen in recording.
	doBlitScreen( (uint8_t*)buffer, (uint8_t*)nes_shm->aviBuf.slot(0));

	aviRecordAddFrame();

	return;
}

/**
 *  Converts an x-y coordinate in the window manager into an x-y
 *  coordinate on FCEU's screen.
 */
uint32 PtoV(double nx, double ny)
{
	int x, y;

	y = (int)( ny * (double)nes_shm->video.nrow.load(std::memory_order_acquire) );

	if ( nes_shm->video.preScaler.load(std::memory_order_acquire) == 3 )
	{
		x = (int)( nx * (double)nes_shm->video.ncol.load(std::memory_order_acquire) * (256.0/301.0) );
	}
	else
	{
		x = (int)( nx * (double)nes_shm->video.ncol.load(std::memory_order_acquire) );
	}

	//printf("Scaled (%i,%i) \n", x, y);

	x = x / nes_shm->video.xscale.load(std::memory_order_acquire);
	y = y / nes_shm->video.yscale.load(std::memory_order_acquire);

	//if ( nes_shm->video.xyRatio.load(std::memory_order_acquire) == 1 )
	//{
	//	y = y / nes_shm->video.scale;
	//}
	//printf("UnScaled (%i,%i) \n", x, y);

	if (s_clipSides) 
	{
		x += 8;
	}
	y += s_srendline;

	return (x | (y << 16));
}

bool enableHUDrecording = false;
bool fceu11::AviEnableHUDrecording()
{
	if (enableHUDrecording)
		return true;

	return false;
}
void fceu11::SetAviEnableHUDrecording(bool enable)
{
	enableHUDrecording = enable;
}

bool disableMovieMessages = false;
bool fceu11::AviDisableMovieMessages()
{
	if (disableMovieMessages)
		return true;

	return false;
}
void fceu11::SetAviDisableMovieMessages(bool disable)
{
	disableMovieMessages = disable;
}
