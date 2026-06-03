// drawing.cpp — Thin C++ wrappers around Rust FFI implementations (v0.2.23).
//
// All pixel-buffer algorithms have been migrated to
// `src/rust/crates/fceux11-media/src/drawing.rs`. This file retains the
// original C++ function signatures and global-state lookups, then forwards
// the resulting coordinates/buffers to the Rust side. ABI is unchanged:
// every call site (movie.cpp, nsf.cpp, palette.cpp, state.cpp, video.cpp)
// continues to link against the same symbols.

#include "types.h"
#include "fceu.h"
#include "drawing.h"
#include "video.h"
#include "movie.h"
#include "driver.h"

#include "rust/fceux11_rust.h"

#include <string.h>

// ---------------------------------------------------------------------------
// DrawTextLineBG — dim the 14-row message-line background
// ---------------------------------------------------------------------------
void DrawTextLineBG(uint8 *dest)
{
	// 14 rows × 256 columns = 3584 bytes; XBuf is always 256×256.
	fceux11_rust_drawing_draw_text_line_bg(dest, 14 * 256);
}

// ---------------------------------------------------------------------------
// DrawMessage — top/bottom message banner, called from video.cpp
// ---------------------------------------------------------------------------
void DrawMessage(bool beforeMovie)
{
	if (guiMessage.howlong)
	{
		//don't display movie messages if we're not before the movie
		if (beforeMovie && !guiMessage.isMovieMessage)
			return;

		uint8 *t;
		guiMessage.howlong--;

		if (guiMessage.linesFromBottom > 0)
			t = XBuf + FCEU_TextScanlineOffsetFromBottom(guiMessage.linesFromBottom) + 1;
		else
			t = XBuf + FCEU_TextScanlineOffsetFromBottom(20) + 1;

		/*
		FCEU palette:
		$00: [8] unvpalette found in palettes/palettes.h
		black, white, black, greyish, redish, bright green, bluish
		$80:
		nes palette
		$C0:
		dim version of nes palette
		*/

		if (t >= XBuf)
		{
			int color = 0x20;
			if (guiMessage.howlong <= 40) color = 0x3C;
			if (guiMessage.howlong <= 32) color = 0x31;
			if (guiMessage.howlong <= 24) color = 0x21;
			if (guiMessage.howlong <= 16) color = 0x51;
			if (guiMessage.howlong <=  8) color = 0x41;
			DrawTextTrans(ClipSidesOffset + t, 256, (uint8 *)guiMessage.errmsg, color + 0x80);
		}
	}

	if (subtitleMessage.howlong)
	{
		//don't display movie messages if we're not before the movie
		if (beforeMovie && !subtitleMessage.isMovieMessage)
			return;

		uint8 *tt;
		subtitleMessage.howlong--;
		tt = XBuf + FCEU_TextScanlineOffsetFromBottom(216);

		if (tt >= XBuf)
		{
			int color = 0x20;
			if (subtitleMessage.howlong == 39) color = 0x38;
			if (subtitleMessage.howlong <= 30) color = 0x2C;
			if (subtitleMessage.howlong <= 20) color = 0x1C;
			if (subtitleMessage.howlong <= 10) color = 0x11;
			if (subtitleMessage.howlong <=  5) color = 0x1;
			DrawTextTrans(ClipSidesOffset + tt, 256, (uint8 *)subtitleMessage.errmsg, color + 0x80);
		}
	}
}

// ---------------------------------------------------------------------------
// FCEU_DrawRecordingStatus — play/record/pause indicator
// ---------------------------------------------------------------------------
static void drawstatus(uint8* XBuf, int n, int y, int xofs)
{
	// Mirrors the original C++ arithmetic:
	//   XBuf += FCEU_TextScanlineOffsetFromBottom(y) + 240 + 255 + xofs;
	// where FCEU_TextScanlineOffsetFromBottom(y) == (LastSLine - y) * 256.
	// The Rust FFI receives the resulting base pointer and the remaining
	// byte count to the end of XBuf (256*256).
	if (n < 0 || n > 3) return;
	uint8 *base = XBuf + FCEU_TextScanlineOffsetFromBottom(y) + 240 + 255 + xofs;
	if (base < XBuf) return;
	size_t base_len = (XBuf + 256 * 256) - base;
	fceux11_rust_drawing_draw_status_icon(base, base_len, (uint8_t)n);
}

void FCEU_DrawRecordingStatus(uint8* XBuf)
{
	if (FCEUD_ShowStatusIcon())
	{
		bool hasPlayRecIcon = false;
		if (FCEUMOV_Mode(MOVIEMODE_RECORD))
		{
			drawstatus(XBuf - ClipSidesOffset, 2, 28, 0);
			hasPlayRecIcon = true;
		}
		else if (FCEUMOV_Mode(MOVIEMODE_PLAY | MOVIEMODE_FINISHED))
		{
			drawstatus(XBuf - ClipSidesOffset, 1, 28, 0);
			hasPlayRecIcon = true;
		}

		if (EmulationPaused & (EMULATIONPAUSED_PAUSED | EMULATIONPAUSED_TIMER))
			drawstatus(XBuf - ClipSidesOffset, 3, 28, hasPlayRecIcon ? -16 : 0);
	}
}

// ---------------------------------------------------------------------------
// FCEU_DrawNumberRow — save-state status digit row
// ---------------------------------------------------------------------------
void FCEU_DrawNumberRow(uint8 *XBuf, int *nstatus, int cur)
{
	// XBaf = XBuf - 4 + (LastSLine - 34) * 256
	uint8 *XBaf = XBuf - 4 + (FSettings.LastSLine - 34) * 256;
	if (XBaf < XBuf) return; // matches original C++ guard
	// The Rust side requires a contiguous region starting at XBaf; pass
	// the full XBuf tail length so it can bounds-check the per-tile writes.
	size_t xbaf_len = (XBuf + 256 * 256) - XBaf;
	fceux11_rust_drawing_draw_number_row(XBaf, xbaf_len, nstatus, cur);
}

// ---------------------------------------------------------------------------
// DrawTextTrans / DrawTextTransWH — text rasterization
// ---------------------------------------------------------------------------
void DrawTextTransWH(uint8 *dest, int width, uint8 *textmsg, uint8 fgcolor,
                     int max_w, int max_h, int border)
{
	// XBuf-style buffers are always 256-wide, so we have exactly width*max_h
	// bytes available starting at `dest`. Compute the precise slice length.
	if (width <= 0 || dest == nullptr || textmsg == nullptr) return;
	if (width > 256) width = 256;
	if (max_h > 64) max_h = 64;
	if (max_w > 256) max_w = 256;
	size_t slice_len = (size_t)width * (size_t)max_h;
	fceux11_rust_drawing_draw_text_trans(
		dest,
		slice_len,
		(const char *)textmsg,
		(uint32_t)width,
		fgcolor,
		(int32_t)max_w,
		(int32_t)max_h,
		(int32_t)border);
}

void DrawTextTrans(uint8 *dest, uint32 width, uint8 *textmsg, uint8 fgcolor)
{
	DrawTextTransWH(dest, (int)width, textmsg, fgcolor, 256, 16, 2);
}
