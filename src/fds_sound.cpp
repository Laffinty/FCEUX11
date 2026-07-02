// FDS sound chip emulation (extracted from fds.cpp for v1.10 Cryptex Task 3).
// Contains the FDSSOUND state, waveform/modulation DSP, envelope generator,
// and the GameExpSound callback plumbing.  FDSSoundReset / FDSSoundStateAdd /
// FDSSound are the only entry points called from fds.cpp.

#include "types.h"
#include "x6502.h"
#include "fceu.h"
#include "fds.h"
#include "sound.h"
#include "state.h"
#include "cart.h"

#include <cstring>

#define FDSClock (1789772.7272727272727272 / 2)

typedef struct {
	int64 cycles;
	int64 count;
	int64 envcount;
	uint32 b19shiftreg60;
	uint32 b24adder66;
	uint32 b24latch68;
	uint32 b17latch76;
	int32 clockcount;
	uint8 b8shiftreg88;
	uint8 amplitude[2];
	uint8 speedo[2];
	uint8 mwcount;
	uint8 mwstart;
	uint8 mwave[0x20];
	uint8 cwave[0x40];
	uint8 SPSG[0xB];
} FDSSOUND;

static FDSSOUND fdso;

#define  SPSG           fdso.SPSG
#define b19shiftreg60   fdso.b19shiftreg60
#define b24adder66      fdso.b24adder66
#define b24latch68      fdso.b24latch68
#define b17latch76      fdso.b17latch76
#define b8shiftreg88    fdso.b8shiftreg88
#define clockcount      fdso.clockcount
#define amplitude       fdso.amplitude
#define speedo          fdso.speedo
static int ta;
static int32 FBC = 0;

static INLINE void ClockRise(void);
static INLINE void ClockFall(void);
static INLINE int32 FDSDoSound(void);
static void DoEnv(void);
static void RenderSound(void);
static void RenderSoundHQ(void);

void FDSSoundStateAdd(void) {
	AddExState(fdso.cwave, 64, 0, "WAVE");
	AddExState(fdso.mwave, 32, 0, "MWAV");
	AddExState(amplitude, 2, 0, "AMPL");
	AddExState(SPSG, 0xB, 0, "SPSG");
	AddExState(&b8shiftreg88, 1, 0, "B88");
	AddExState(&clockcount, 4, 1, "CLOC");
	AddExState(&b19shiftreg60, 4, 1, "B60");
	AddExState(&b24adder66, 4, 1, "B66");
	AddExState(&b24latch68, 4, 1, "B68");
	AddExState(&b17latch76, 4, 1, "B76");
}

static DECLFR(FDSSRead) {
	switch (A & 0xF) {
	case 0x0: return(amplitude[0] | (g_cpu.native_layout().DB & 0xC0));
	case 0x2: return(amplitude[1] | (g_cpu.native_layout().DB & 0xC0));
	}
	return(g_cpu.native_layout().DB);
}

static DECLFW(FDSSWrite) {
	if (FSettings.SndRate) {
		if (FSettings.soundq >= 1) RenderSoundHQ();
		else RenderSound();
	}
	A -= 0x4080;
	switch (A) {
	case 0x0: case 0x4:
		if (V & 0x80) amplitude[(A & 0xF) >> 2] = V & 0x3F;
		break;
	case 0x7: b17latch76 = 0; SPSG[0x5] = 0; break;
	case 0x8:
		b17latch76 = 0;
		fdso.mwave[SPSG[0x5] & 0x1F] = V & 0x7;
		SPSG[0x5] = (SPSG[0x5] + 1) & 0x1F;
		break;
	}
	SPSG[A] = V;
}

static void DoEnv(void) {
	for (int x = 0; x < 2; x++)
		if (!(SPSG[x << 2] & 0x80) && !(SPSG[0x3] & 0x40)) {
			static int counto[2] = { 0, 0 };
			if (counto[x] <= 0) {
				if (!(SPSG[x << 2] & 0x80)) {
					if (SPSG[x << 2] & 0x40) {
						if (amplitude[x] < 0x3F) amplitude[x]++;
					} else {
						if (amplitude[x] > 0) amplitude[x]--;
					}
				}
				counto[x] = (SPSG[x << 2] & 0x3F);
			} else counto[x]--;
		}
}

static DECLFR(FDSWaveRead) {
	return(fdso.cwave[A & 0x3f] | (g_cpu.native_layout().DB & 0xC0));
}

static DECLFW(FDSWaveWrite) {
	if (SPSG[0x9] & 0x80) fdso.cwave[A & 0x3f] = V & 0x3F;
}

static INLINE void ClockRise(void) {
	if (!clockcount) {
		ta++;
		b19shiftreg60 = (SPSG[0x2] | ((SPSG[0x3] & 0xF) << 8));
		b17latch76 = (SPSG[0x6] | ((SPSG[0x07] & 0xF) << 8)) + b17latch76;
		if (!(SPSG[0x7] & 0x80)) {
			int t = fdso.mwave[(b17latch76 >> 13) & 0x1F] & 7;
			int t2 = amplitude[1];
			int adj = 0;
			if ((t & 3)) {
				if ((t & 4)) adj -= (t2 * ((4 - (t & 3))));
				else adj += (t2 * ((t & 3)));
			}
			adj *= 2;
			if (adj > 0x7F) adj = 0x7F;
			if (adj < -0x80) adj = -0x80;
			b8shiftreg88 = 0x80 + adj;
		} else {
			b8shiftreg88 = 0x80;
		}
	} else {
		b19shiftreg60 <<= 1;
		b8shiftreg88 >>= 1;
	}
	b24adder66 = (b24latch68 + b19shiftreg60) & 0x1FFFFFF;
}

static INLINE void ClockFall(void) {
	if ((b8shiftreg88 & 1)) b24latch68 = b24adder66;
	clockcount = (clockcount + 1) & 7;
}

static INLINE int32 FDSDoSound(void) {
	fdso.count += fdso.cycles;
	if (fdso.count >= ((int64)1 << 40)) {
 dogk:
		fdso.count -= (int64)1 << 40;
		ClockRise(); ClockFall();
		fdso.envcount--;
		if (fdso.envcount <= 0) {
			fdso.envcount += SPSG[0xA] * 3;
			DoEnv();
		}
	}
	if (fdso.count >= 32768) goto dogk;
	{
		int k = amplitude[0];
		if (k > 0x20) k = 0x20;
		return (fdso.cwave[b24latch68 >> 19] * k) * 4 / ((SPSG[0x9] & 0x3) + 2);
	}
}

static void RenderSound(void) {
	int32 start = FBC;
	int32 end = (SOUNDTS << 16) / soundtsinc;
	if (end <= start) return;
	FBC = end;
	if (!(SPSG[0x9] & 0x80))
		for (int32 x = start; x < end; x++) {
			uint32 t = FDSDoSound();
			t += t >> 1; t >>= 4;
			Wave[x >> 4] += t;
		}
}

static void RenderSoundHQ(void) {
	if (!(SPSG[0x9] & 0x80))
		for (uint32 x = FBC; x < SOUNDTS; x++) {
			uint32 t = FDSDoSound();
			t += t >> 1;
			WaveHi[x] += t;
		}
	FBC = SOUNDTS;
}

static void HQSync(int32 ts) { FBC = ts; }

void FDSSound(int c) { RenderSound(); FBC = c; }

static void FDS_ESI(void) {
	if (FSettings.SndRate) {
		if (FSettings.soundq >= 1) {
			fdso.cycles = (int64)1 << 39;
		} else {
			fdso.cycles = ((int64)1 << 40) * FDSClock;
			fdso.cycles /= FSettings.SndRate * 16;
		}
	}
	SetReadHandler(0x4040, 0x407f, FDSWaveRead);
	SetWriteHandler(0x4040, 0x407f, FDSWaveWrite);
	SetWriteHandler(0x4080, 0x408A, FDSSWrite);
	SetReadHandler(0x4090, 0x4092, FDSSRead);
}

void FDSSoundReset(void) {
	memset(&fdso, 0, sizeof(fdso));
	FDS_ESI();
	GameExpSound.HiSync = HQSync;
	GameExpSound.HiFill = RenderSoundHQ;
	GameExpSound.Fill = FDSSound;
	GameExpSound.RChange = FDS_ESI;
}
