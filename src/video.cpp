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

#include "types.h"
#include "video.h"
#include "fceu.h"
#include "file.h"
#include "utils/memory.h"
#include "utils/crc32.h"
#include "state.h"
#include "movie.h"
#include "palette.h"
#include "nsf.h"
#include "input.h"
#include "vsuni.h"
#include "drawing.h"
#include "driver.h"
#include "drivers/common/vidblit.h"
#include "rust/fceux11_rust.h"
#ifdef _S9XLUA_H
#include "fceulua.h"
#endif

#ifdef __WIN_DRIVER__
#include "drivers/win/common.h" //For DirectX constants
#include "drivers/win/input.h"
#endif

#ifdef CREATE_AVI
#endif

//no stdint in win32 (but we could add it if we needed to)

#include <cstring>
#include <cstdio>
#include <cstdlib>
#include <cstdarg>
#include <vector>
#include <zlib.h>

//XBuf:
//0-63 is reserved for 7 special colours used by FCEUX (overlay, etc.)
//64-127 is the most-used emphasis setting per frame
//128-195 is the palette with no emphasis
//196-255 is the palette with all emphasis bits on
u8 *XBuf=NULL; //used for current display
u8 *XBackBuf=NULL; //ppu output is stashed here before drawing happens
u8 *XDBuf=NULL; //corresponding to XBuf but with deemph bits
u8 *XDBackBuf=NULL; //corresponding to XBackBuf but with deemph bits
int ClipSidesOffset=0;	//Used to move displayed messages when Clips left and right sides is checked
static u8 *xbsave=NULL;

GUIMESSAGE guiMessage;
GUIMESSAGE subtitleMessage;

bool vidGuiMsgEna = true;

//for input display
extern int input_display;
extern uint32 cur_input_display;

bool oldInputDisplay = false;

unsigned int lastu = 0;

// v0.2.27: AsSnapshotName is owned by Rust (fceux11_rust_video_*).
// The C++ side is a thin pass-through — see FCEUI_SetSnapshotAsName /
// FCEUI_GetSnapshotAsName below. The std::string global is gone.

void FCEUI_SetSnapshotAsName(std::string name)
{
	// v0.2.27: name is now stored in Rust. Pass the underlying bytes
	// (no NUL terminator needed by the FFI).
	fceux11_rust_video_set_snapshot_as_name(
		reinterpret_cast<const char*>(name.data()),
		name.size());
}

std::string FCEUI_GetSnapshotAsName()
{
	// v0.2.27: read the name back from Rust. The C++ std::string
	// constructor takes a (data, size) pair, matching the FFI contract
	// (no implicit NUL terminator).
	std::string out;
	const size_t kBuf = 512; // matches the historical C++ 512-byte limit
	std::vector<char> buf(kBuf, 0);
	size_t n = fceux11_rust_video_get_snapshot_as_name(buf.data(), buf.size());
	if (n > 0 && n < kBuf)
	{
		out.assign(buf.data(), n);
	}
	return out;
}

static void FCEU_DrawPauseCountDown(uint8 *XBuf);

void FCEU_KillVirtualVideo(void)
{
	if ( XBuf )
	{
		FCEU_afree(XBuf); XBuf = NULL;
	}
	if ( XBackBuf )
	{
		FCEU_afree(XBackBuf); XBackBuf = NULL;
	}
	if ( XDBuf )
	{
		FCEU_afree(XDBuf); XDBuf = NULL;
	}
	if ( XDBackBuf )
	{
		FCEU_afree(XDBackBuf); XDBackBuf = NULL;
	}
	//printf("Video Core Cleanup\n");
}

/**
* Return: Flag that indicates whether the function was succesful or not.
*
* TODO: This function is Windows-only. It should probably be moved.
**/
int FCEU_InitVirtualVideo(void)
{
	//Some driver code may allocate XBuf externally.
	//256 bytes per scanline, * 240 scanline maximum
	if(XBuf)
		return 1;
	
	XBuf = (u8*)FCEU_amalloc(256 * 256);
	XBackBuf = (u8*)FCEU_amalloc(256 * 256);
	XDBuf = (u8*)FCEU_amalloc(256 * 256);
	XDBackBuf = (u8*)FCEU_amalloc(256 * 256);


	xbsave = XBuf;

	memset(XBuf,128,256*256);
	memset(XBackBuf,128,256*256);
	memset(XDBuf,0,256*256);
	memset(XDBackBuf,0,256*256);

	return 1;
}

#ifdef FRAMESKIP
void FCEU_PutImageDummy(void)
{
	ShowFPS();
	if(GameInfo->type!=GIT_NSF)
	{
		FCEU_DrawNTSCControlBars(XBuf);
		FCEU_DrawSaveStates(XBuf);
		FCEU_DrawMovies(XBuf);
	}
	if(guiMessage.howlong) guiMessage.howlong--; /* DrawMessage() */
}
#endif

static int dosnapsave=0;
void FCEUI_SaveSnapshot(void)
{
	dosnapsave=1;
}

void FCEUI_SaveSnapshotAs(void)
{
	dosnapsave=2;
}

static void ReallySnap(void)
{
	int x=SaveSnapshot();
	if(!x)
		FCEU_DispMessage("Error saving screen snapshot.",0);
	else
		FCEU_DispMessage("Screen snapshot %d saved.",0,x-1);
}

static uint32 GetButtonColor(uint32 held, uint32 c, uint32 ci, int bit)
{
	uint32 on = FCEUMOV_Mode(MOVIEMODE_PLAY) ? 0x90 : 0xA7;	//Standard, or Gray depending on movie mode
	uint32 oni = 0xA0;		//Color for immediate keyboard buttons
	uint32 blend = 0xB6;	//Blend of immiate and last held buttons
	uint32 ahold = 0x87;	//Auto hold
	uint32 off = 0xCF;

	uint32 color;
	uint8 mask = 1u << bit;
	if (held & mask) { //If auto-hold
		if (!(ci & mask))
			color = ahold;
		else
			color = (c & mask) ? on : off; //If the button is pressed down (immediate) that negates auto hold, however it is only off if the previous frame the button wasn't pressed!
	}
	else {
		if (c & mask)
			color = (ci & mask) ? blend : on; //If immedaite buttons are pressed and they match the previous frame, blend the colors
		else
			color = (ci & mask) ? oni : off;
	}
	return color;
}

void FCEU_PutImage(void)
{
	if(dosnapsave==2)	//Save screenshot as, currently only flagged & run by the Win32 build. //TODO SDL: implement this?
	{
		char nameo[512];
		strcpy(nameo,FCEUI_GetSnapshotAsName().c_str());
		if (nameo[0])
		{
			SaveSnapshot(nameo);
			FCEU_DispMessage("Snapshot Saved.",0);
		}
		dosnapsave=0;
	}
	if(GameInfo->type==GIT_NSF)
	{
		DrawNSF(XBuf);

#ifdef _S9XLUA_H
		FCEU_LuaGui(XBuf);
#endif

		//Save snapshot after NSF screen is drawn.  Why would we want to do it before?
		if(dosnapsave==1)
		{
			ReallySnap();
			dosnapsave=0;
		}
	}
	else
	{
		//Save backbuffer before overlay stuff is written.
		if(!FCEUI_EmulationPaused())
			memcpy(XBackBuf, XBuf, 256*256);

		//Some messages need to be displayed before the avi is dumped
		DrawMessage(true);

#ifdef _S9XLUA_H
		// Lua gui should draw before the avi is dumped.
		FCEU_LuaGui(XBuf);
#endif

		//Save snapshot
		if(dosnapsave==1)
		{
			ReallySnap();
			dosnapsave=0;
		}

		if (!FCEUI_AviEnableHUDrecording()) snapAVI();

		if(GameInfo->type==GIT_VSUNI)
			FCEU_VSUniDraw(XBuf);

		FCEU_DrawSaveStates(XBuf);
		FCEU_DrawMovies(XBuf);
		FCEU_DrawLagCounter(XBuf);
		FCEU_DrawNTSCControlBars(XBuf);
		FCEU_DrawRecordingStatus(XBuf);
		FCEU_DrawPauseCountDown(XBuf);
		ShowFPS();
	}

	if(FCEUD_ShouldDrawInputAids())
		FCEU_DrawInput(XBuf);

	//Fancy input display code
	if(input_display)
	{
		int i, j;
		uint8 *t = XBuf+(FSettings.LastSLine-9)*256 + 20;		//mbg merge 7/17/06 changed t to uint8*
		if(input_display > 4) input_display = 4;
		for(int controller = 0; controller < input_display; controller++, t += 56)
		{
			for(i = 0; i < 34;i++)
				for(j = 0; j <9 ; j++)
					t[i+j*256] = (t[i+j*256] & 0x30) | 0xC1;
			for(i = 3; i < 6; i++)
				for(j = 3; j< 6; j++)
					t[i+j*256] = 0xCF;

			uint32 held = 0;
			uint32 c = cur_input_display >> (controller * 8);
			uint32 ci = 0;
			uint32 color;

#ifdef __WIN_DRIVER__
			extern uint32 JSAutoHeld;
			// This doesn't work in anything except windows for now.
			// It doesn't get set anywhere in other ports.
			if (!oldInputDisplay)
				ci = FCEUMOV_Mode(MOVIEMODE_PLAY) ? 0 : GetGamepadPressedImmediate() >> (controller * 8);

			if (!oldInputDisplay && !FCEUMOV_Mode(MOVIEMODE_PLAY))
				held = (JSAutoHeld >> (controller * 8));
#else
			// Put other port info here
#endif

			//adelikat: I apologize to anyone who ever sifts through this color assignment
			//A
			color = GetButtonColor(held, c, ci, 0);
			for(i=0; i < 4; i++)
			{
				for(j = 0; j < 4; j++)
				{
					if(i%3==0 && j %3 == 0)
						continue;
					t[30+4*256+i+j*256] = color;
				}
			}
			//B
			color = GetButtonColor(held, c, ci, 1);
			for(i=0; i < 4; i++)
			{
				for(j = 0; j < 4; j++)
				{
					if(i%3==0 && j %3 == 0)
						continue;
					t[24+4*256+i+j*256] = color;
				}
			}
			//Select
			color = GetButtonColor(held, c, ci, 2);
			for(i = 0; i < 4; i++)
			{
				t[11+5*256+i] = color;
				t[11+6*256+i] = color;
			}
			//Start
			color = GetButtonColor(held, c, ci, 3);
			for(i = 0; i < 4; i++)
			{
				t[17+5*256+i] = color;
				t[17+6*256+i] = color;
			}
			//Up
			color = GetButtonColor(held, c, ci, 4);
			for(i = 0; i < 3; i++)
			{
				for(j = 0; j < 3; j++)
				{
					t[3+i+256*j] = color;
				}
			}
			//Down
			color = GetButtonColor(held, c, ci, 5);
			for(i = 0; i < 3; i++)
			{
				for(j = 0; j < 3; j++)
				{
					t[3+i+256*j+6*256] = color;
				}
			}
			//Left
			color = GetButtonColor(held, c, ci, 6);
			for(i = 0; i < 3; i++)
			{
				for(j = 0; j < 3; j++)
				{
					t[3*256+i+256*j] = color;
				}
			}
			//Right
			color = GetButtonColor(held, c, ci, 7);
			for(i = 0; i < 3; i++)
			{
				for(j = 0; j < 3; j++)
				{
					t[6+3*256+i+256*j] = color;
				}
			}
		}
	}

	if (FCEUI_AviEnableHUDrecording())
	{
		if (FCEUI_AviDisableMovieMessages())
		{
			snapAVI();
			DrawMessage(false);
		} else
		{
			DrawMessage(false);
			snapAVI();
		}
	} else DrawMessage(false);

}
void snapAVI()
{
	//Update AVI
	if(!FCEUI_EmulationPaused())
		FCEUI_AviVideoUpdate(XBuf);
}

void FCEU_DispMessageOnMovie( __FCEU_PRINTF_FORMAT const char *format, ...)
{
	va_list ap;

	va_start(ap,format);
	vsnprintf(guiMessage.errmsg,sizeof(guiMessage.errmsg),format,ap);
	va_end(ap);

	if ( vidGuiMsgEna )
	{
		guiMessage.howlong = 180;
	}
	guiMessage.isMovieMessage = true;
	guiMessage.linesFromBottom = 0;

	if (FCEUI_AviIsRecording() && FCEUI_AviDisableMovieMessages())
		guiMessage.howlong = 0;
}

void FCEU_DispMessage( __FCEU_PRINTF_FORMAT const char *format, int disppos=0, ...)
{
	va_list ap;

	va_start(ap,disppos);
	vsnprintf(guiMessage.errmsg,sizeof(guiMessage.errmsg),format,ap);
	va_end(ap);
	// also log messages
	char temp[2048];
	va_start(ap,disppos);
	vsnprintf(temp,sizeof(temp),format,ap);
	va_end(ap);
	strcat(temp, "\n");
	FCEU_printf("%s",temp);

	if ( vidGuiMsgEna )
	{
		guiMessage.howlong = 180;
	}
	guiMessage.isMovieMessage = false;

	guiMessage.linesFromBottom = disppos;

	//adelikat: Pretty sure this code fails, Movie playback stopped is done with FCEU_DispMessageOnMovie()
	#ifdef CREATE_AVI
	if(LoggingEnabled == 2)
	{
		/* While in AVI recording mode, only display bare minimum
		 * of messages
		 */
		if(strcmp(guiMessage.errmsg, "Movie playback stopped.") != 0)
			guiMessage.howlong = 0;
	}
	#endif
}

void FCEU_ResetMessages()
{
	guiMessage.howlong = 0;
	guiMessage.isMovieMessage = false;
	guiMessage.linesFromBottom = 0;
}


// v0.2.27: WritePNGChunk has moved to Rust
// (fceux11_rust_video_encode_png_rgb / fceux11_rust_video_encode_png_indexed).
// The C++ side just owns the file I/O, file-name resolution, and the
// indexed→RGB deemph conversion. The PNG chunk layout, CRC32, and zlib
// compression are now done in Rust.
//
// Reference: src/rust/crates/fceux11-media/src/video.rs.
//
// (Function body removed in v0.2.27; keep this comment block as a
// breadcrumb in case someone greps for the symbol.)

// v0.2.27: 24-bit RGB access for a single pixel.
//
// The Rust FFI expects a 64-entry NES palette (3 bytes per index,
// R then G then B). We build it on the stack from FCEUD_GetPalette, which
// is the same source the original code used.
//
// Returns 0x00RRGGBB on success, ~0u on out-of-bounds.
uint32 GetScreenPixel(int x, int y, bool usebackup) {
	if (((x < 0) || (x > 255)) || ((y < 0) || (y > 255)))
		return -1;

	uint8 palette_64[64 * 3];
	for (int i = 0; i < 64; ++i)
		FCEUD_GetPalette((uint8)i, &palette_64[i*3], &palette_64[i*3+1], &palette_64[i*3+2]);

	FceuSlice xbuf_slice     = { XBuf,     XBuf     ? 256*256 : 0 };
	FceuSlice xbackbuf_slice = { XBackBuf, XBackBuf ? 256*256 : 0 };
	FceuSlice pal_slice      = { palette_64, sizeof(palette_64) };
	uint8 rgb[3] = { 0, 0, 0 };
	FceuSliceMut rgb_mut     = { rgb, sizeof(rgb) };

	int32_t rc = fceux11_rust_video_get_screen_pixel(
		(int32_t)x, (int32_t)y, usebackup,
		xbuf_slice, xbackbuf_slice, pal_slice, rgb_mut);
	if (rc != 0)
		return (uint32_t)-1;
	return ((uint32_t)rgb[0] << 16) | ((uint32_t)rgb[1] << 8) | (uint32_t)rgb[2];
}

int GetScreenPixelPalette(int x, int y, bool usebackup) {
	if (((x < 0) || (x > 255)) || ((y < 0) || (y > 255)))
		return -1;

	FceuSlice xbuf_slice     = { XBuf,     XBuf     ? 256*256 : 0 };
	FceuSlice xbackbuf_slice = { XBackBuf, XBackBuf ? 256*256 : 0 };

	return fceux11_rust_video_get_screen_pixel_palette(
		(int32_t)x, (int32_t)y, usebackup,
		xbuf_slice, xbackbuf_slice);
}

// v0.2.27: Build the deemph-aware RGB scanline buffer that the
// Rust encoder consumes. One scanline = 256 pixels × 3 channels,
// totallines scanlines total. The conversion uses
// ModernDeemphColorMap (from drivers/common/vidblit.cpp) which depends
// on the XBuf + scanline pointer walk that the C++ side is best
// positioned to perform.
//
// `out_rgb` must be at least 256 * 3 * totallines bytes. Returns
// `out_rgb` on success, nullptr on bad input.
static uint8 *video_build_rgb_scanlines(
	int first_sline,
	int totallines,
	uint8 *out_rgb)
{
	if (!out_rgb || first_sline < 0 || totallines <= 0)
		return nullptr;
	uint8 *tmp = XBuf + first_sline * 256;
	uint8 *dest = out_rgb;
	for (int y = 0; y < totallines; ++y)
	{
		for (int x = 0; x < 256; ++x)
		{
			u32 color = ModernDeemphColorMap(tmp, XBuf, 1);
			*dest++ = (color >> 0x10) & 0xFF;
			*dest++ = (color >> 0x08) & 0xFF;
			*dest++ = (color >> 0x00) & 0xFF;
			tmp++;
		}
	}
	return out_rgb;
}

int SaveSnapshot(void)
{
	int totallines = FSettings.LastSLine - FSettings.FirstSLine + 1;
	unsigned int u;
	FILE *pp = NULL;

	// Resolve the next free filename (matches the historical C++ loop).
	for (u = lastu; u < 99999; ++u)
	{
		pp = FCEUD_UTF8fopen(FCEU_MakeFName(FCEUMKF_SNAP, u, "png").c_str(), "rb");
		if (pp == NULL) break;
		fclose(pp);
	}
	lastu = u;

	std::string outpath = FCEU_MakeFName(FCEUMKF_SNAP, u, "png");
	pp = FCEUD_UTF8fopen(outpath.c_str(), "wb");
	if (pp == NULL) return 0;

	// Build the RGB scanlines (256 * 3 * totallines bytes).
	const size_t rgb_bytes = (size_t)256 * 3 * (size_t)totallines;
	std::vector<uint8> rgb(rgb_bytes, 0);
	if (!video_build_rgb_scanlines(FSettings.FirstSLine, totallines, rgb.data()))
	{
		fclose(pp);
		return 0;
	}

	// Encode the PNG into a stack buffer. 256 KiB is well over the
	// worst case (~190 KiB for 240 lines).
	uint8 png_buf[256 * 1024];
	FceuSlice rgb_slice = { rgb.data(), rgb.size() };
	FceuSliceMut out_mut = { png_buf, sizeof(png_buf) };
	FceuVideoEncodeArgs args = {};
	args.xbuf       = XBuf;
	args.xbuf_len   = XBuf ? 256 * 256 : 0;
	args.first_sline = FSettings.FirstSLine;
	args.last_sline  = FSettings.LastSLine;
	args.out         = out_mut;
	size_t n = fceux11_rust_video_encode_png_rgb(&args, rgb_slice);
	if (n == 0 || n > sizeof(png_buf))
	{
		fclose(pp);
		return 0;
	}

	if (fwrite(png_buf, 1, n, pp) != n)
	{
		fclose(pp);
		return 0;
	}
	fclose(pp);
	return (int)(u + 1);
}

//overloaded SaveSnapshot for "Savesnapshot As" function
int SaveSnapshot(char fileName[512])
{
	int totallines = FSettings.LastSLine - FSettings.FirstSLine + 1;
	FILE *pp = FCEUD_UTF8fopen(fileName, "wb");
	if (pp == NULL) return 0;

	// Build the 256-entry NES palette (768 bytes).
	uint8 pdata[256 * 3];
	for (int x = 0; x < 256; ++x)
		FCEUD_GetPalette((uint8)x, pdata + x*3, pdata + x*3 + 1, pdata + x*3 + 2);

	// Encode the indexed PNG into a stack buffer.
	uint8 png_buf[256 * 1024];
	FceuSlice plte_slice = { pdata, sizeof(pdata) };
	FceuSliceMut out_mut = { png_buf, sizeof(png_buf) };
	FceuVideoEncodeArgs args = {};
	args.xbuf       = XBuf;
	args.xbuf_len   = XBuf ? 256 * 256 : 0;
	args.first_sline = FSettings.FirstSLine;
	args.last_sline  = FSettings.LastSLine;
	args.out         = out_mut;
	size_t n = fceux11_rust_video_encode_png_indexed(&args, plte_slice);
	if (n == 0 || n > sizeof(png_buf))
	{
		fclose(pp);
		return 0;
	}

	if (fwrite(png_buf, 1, n, pp) != n)
	{
		fclose(pp);
		return 0;
	}
	fclose(pp);
	return 0;
}
// called when another ROM is opened
void ResetScreenshotsCounter()
{
	lastu = 0;
}

uint64 FCEUD_GetTime(void);
uint64 FCEUD_GetTimeFreq(void);
bool Show_FPS = false;
// Control whether the frames per second of the emulation is rendered.
bool FCEUI_ShowFPS()
{
	return Show_FPS;
}
void FCEUI_SetShowFPS(bool showFPS)
{
	if ( Show_FPS != showFPS )
	{
		ResetFPS();
	}
	Show_FPS = showFPS;
}
void FCEUI_ToggleShowFPS()
{
	Show_FPS ^= 1;

	ResetFPS();
}

static uint64 boop_ts = 0;
static unsigned int boopcount = 0;

void ResetFPS(void)
{
	boop_ts = 0;
	boopcount = 0;
}

void ShowFPS(void)
{
	if (Show_FPS == false)
	{
		return;
	}
	static char fpsmsg[16] = { 0 };
	uint64 ts = FCEUD_GetTime();
	uint64 da;

	if ( boop_ts == 0 )
	{
		boop_ts = ts;
	}
	da = ts - boop_ts;

	if ( da > FCEUD_GetTimeFreq() )
	{
		sprintf(fpsmsg, "%.1f", (double)boopcount / ((double)da / FCEUD_GetTimeFreq()));

		boopcount = 0;
		boop_ts = ts;
	}
	boopcount++;

	DrawTextTrans(XBuf + ((256 - ClipSidesOffset) - 40) + (FSettings.FirstSLine + 4) * 256, 256, (uint8*)fpsmsg, 0xA0);
}

bool showPauseCountDown = true;

static void FCEU_DrawPauseCountDown(uint8 *XBuf)
{
	if (EmulationPaused & EMULATIONPAUSED_TIMER)
	{
		int pauseFramesLeft = FCEUI_PauseFramesRemaining();

		if (showPauseCountDown && (pauseFramesLeft > 0) )
		{
			char text[32];
			int framesPerSec;

			if (PAL || dendy)
			{
				framesPerSec = 50;
			}
			else
			{
				framesPerSec = 60;
			}

			sprintf(text, "Unpausing in %d...", (pauseFramesLeft / framesPerSec) + 1);

			if (text[0])
			{
				DrawTextTrans(XBuf + ClipSidesOffset + (FSettings.FirstSLine) * 256, 256, (uint8*)text, 0xA0);
			}
		}
	}
}
