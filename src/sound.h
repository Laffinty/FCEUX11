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

#ifndef _SOUND_H_
#define _SOUND_H_

#include "apu.h"           // v1.6 Resonance: Apu class + v1.0 reference aliases
#include "expansion_audio.h" // v1.6 Resonance: EXPSOUND + ExpansionAudio

extern EXPSOUND& GameExpSound;

// ---- Phase C1 aliases: resampling / timing ----
extern int32_t& nesincsize;

void SetSoundVariables(void);

int GetSoundBuffer(int32 **W);
int FlushEmulateSound(void);
extern int32_t (&Wave)[2048+512];
extern int32_t (&WaveFinal)[2048+512];
extern int32_t (&WaveHi)[40000];
extern uint32_t& soundtsinc;
extern uint32_t& soundtsi;

#ifdef WIN32
extern volatile int datacount, undefinedcount;
extern int debug_loggingCD;
extern unsigned char *cdloggerdata;
#endif

extern uint32_t& soundtsoffs;
extern bool& swapDuty;
#define SOUNDTS (g_cpu.sound_timestamp_ref() + soundtsoffs)

// ---- Phase C2 aliases: channel registers / envelope / length ----
extern uint8_t   (& PSG            )[0x10];
extern ENVUNIT   (& EnvUnits       )[3];
extern uint8_t&    EnabledChannels;
extern uint8_t&    IRQFrameMode;
extern uint16_t&   nreg;
extern uint8_t&    TriCount;
extern uint8_t&    TriMode;
extern int32_t&    tristep;
extern int32_t   (& wlcount        )[4];
extern int32_t   (& lengthcount    )[4];

// ---- Phase C2 aliases: square waves ----
extern int32_t   (& RectDutyCount  )[2];
extern uint8_t   (& sweepon         )[2];
extern int32_t   (& curfreq         )[2];
extern uint8_t   (& SweepCount      )[2];
extern uint8_t   (& SweepReload     )[2];
extern int32_t   (& sqacc           )[2];

// ---- Phase C2 aliases: frame counter ----
extern uint8_t&    fcnt;
extern int32_t&    fhcnt;
extern int32_t&    fhinc;

// ---- Phase C2 aliases: DMC ----
extern uint8_t&    DMCFormat;
extern uint8_t&    RawDALatch;
extern uint8_t&    InitialRawDALatch;
extern bool&       DMC_7bit;
extern int32_t&    DMCacc;
extern int32_t&    DMCPeriod;
extern uint8_t&    DMCBitCount;
extern uint32_t&   DMCAddress;
extern uint8_t&    DMCAddressLatch;
extern int32_t&    DMCSize;
extern uint8_t&    DMCSizeLatch;
extern uint8_t&    DMCShift;
extern char&       DMCHaveDMA;
extern char&       DMCHaveSample;
extern uint8_t&    DMCDMABuf;
extern uint8_t&    SIRQStat;

void SetNESSoundMap(void);
void FrameSoundUpdate(void);

void FCEUSND_Power(void);
void FCEUSND_Reset(void);
void FCEUSND_SaveState(void);
void FCEUSND_LoadState(int version);

void FCEU_SoundCPUHook(int);
void Write_IRQFM (uint32 A, uint8 V); //mbg merge 7/17/06 brought over from latest mmbuild

void LogDPCM(int romaddress, int dpcmsize);

#endif
