// NSF UI rendering â€?extracted from nsf.cpp for v1.10 Cryptex Task 2.
// Contains DrawNSF (audio visualization + song info overlay).

#include "types.h"
#include "fceu.h"
#include "video.h"
#include "sound.h"
#include "nsf.h"
#include "input.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cmath>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

extern int special;
extern uint8 SongReload;
extern int32 CurrentSong;
extern NSF_HEADER NSFHeader;
uint8 FCEU_GetJoyJoy(void);

static int vismode = 1;

void DrawNSF(uint8 *XBuf) {
	char snbuf[16];
	int x;

	if (vismode == 0) return;

	memset(XBuf, 0, 256 * 240);
	memset(XDBuf, 0, 256 * 240);

	{
		int32 *Bufpl;
		int32 mul = 0;
		int l = GetSoundBuffer(&Bufpl);

		if (special == 0) {
			if (FSettings.SoundVolume)
				mul = 8192 * 240 / (16384 * FSettings.SoundVolume / 50);
			for (x = 0; x < 256; x++) {
				uint32 y;
				y = 142 + ((Bufpl[(x * l) >> 8] * mul) >> 14);
				if (y < 240) XBuf[x + y * 256] = 3;
			}
		} else if (special == 1) {
			if (FSettings.SoundVolume)
				mul = 8192 * 240 / (8192 * FSettings.SoundVolume / 50);
			for (x = 0; x < 256; x++) {
				double r;
				uint32 xp, yp;
				r = (Bufpl[(x * l) >> 8] * mul) >> 14;
				xp = 128 + r * cos(x * M_PI * 2 / 256);
				yp = 120 + r * sin(x * M_PI * 2 / 256);
				xp &= 255;
				yp %= 240;
				XBuf[xp + yp * 256] = 3;
			}
		} else if (special == 2) {
			static double theta = 0;
			if (FSettings.SoundVolume)
				mul = 8192 * 240 / (16384 * FSettings.SoundVolume / 50);
			for (x = 0; x < 128; x++) {
				double xc, yc, r, t;
				uint32 m, n;
				xc = (double)128 - x;
				yc = 0 - ((double)(((Bufpl[(x * l) >> 8]) * mul) >> 14));
				t = M_PI + atan(yc / xc);
				r = sqrt(xc * xc + yc * yc);
				t += theta;
				m = 128 + r * cos(t);
				n = 120 + r * sin(t);
				if (m < 256 && n < 240) XBuf[m + n * 256] = 3;
			}
			for (x = 128; x < 256; x++) {
				double xc, yc, r, t;
				uint32 m, n;
				xc = (double)x - 128;
				yc = (double)(((Bufpl[(x * l) >> 8]) * mul) >> 14);
				t = atan(yc / xc);
				r = sqrt(xc * xc + yc * yc);
				t += theta;
				m = 128 + r * cos(t);
				n = 120 + r * sin(t);
				if (m < 256 && n < 240) XBuf[m + n * 256] = 3;
			}
			theta += (double)M_PI / 256;
		}
	}

	static const int kFgColor = 1;
	DrawTextTrans(ClipSidesOffset + XBuf + 10 * 256 + 4 + (((31 - strlen((char*)NSFHeader.SongName)) << 2)), 256, NSFHeader.SongName, kFgColor);
	DrawTextTrans(ClipSidesOffset + XBuf + 26 * 256 + 4 + (((31 - strlen((char*)NSFHeader.Artist)) << 2)), 256, NSFHeader.Artist, kFgColor);
	DrawTextTrans(ClipSidesOffset + XBuf + 42 * 256 + 4 + (((31 - strlen((char*)NSFHeader.Copyright)) << 2)), 256, NSFHeader.Copyright, kFgColor);
	DrawTextTrans(ClipSidesOffset + XBuf + 70 * 256 + 4 + (((31 - strlen("Song:")) << 2)), 256, (uint8*)"Song:", kFgColor);
	snprintf(snbuf, sizeof(snbuf), "<%d/%d>", CurrentSong, NSFHeader.TotalSongs);
	DrawTextTrans(XBuf + 82 * 256 + 4 + (((31 - strlen(snbuf)) << 2)), 256, (uint8*)snbuf, kFgColor);

	{
		static uint8 last = 0;
		uint8 tmp = FCEU_GetJoyJoy();
		if ((tmp & JOY_RIGHT) && !(last & JOY_RIGHT)) {
			if (CurrentSong < NSFHeader.TotalSongs) { CurrentSong++; SongReload = 0xFF; }
		} else if ((tmp & JOY_LEFT) && !(last & JOY_LEFT)) {
			if (CurrentSong > 1) { CurrentSong--; SongReload = 0xFF; }
		} else if ((tmp & JOY_UP) && !(last & JOY_UP)) {
			CurrentSong += 10;
			if (CurrentSong > NSFHeader.TotalSongs) CurrentSong = NSFHeader.TotalSongs;
			SongReload = 0xFF;
		} else if ((tmp & JOY_DOWN) && !(last & JOY_DOWN)) {
			CurrentSong -= 10;
			if (CurrentSong < 1) CurrentSong = 1;
			SongReload = 0xFF;
		} else if ((tmp & JOY_START) && !(last & JOY_START))
			SongReload = 0xFF;
		else if ((tmp & JOY_A) && !(last & JOY_A))
			special = (special + 1) % 3;
		last = tmp;
	}
}

void fceu11::NSFSetVis(int mode) {
	vismode = mode;
}
