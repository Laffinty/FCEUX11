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
#include "x6502.h"

#include "fceu.h"
#include "sound.h"
#include "filter.h"
#include "state.h"
#include "wave.h"
#include "debug.h"

#include <cstdlib>
#include <cstdio>
#include <cstring>

// ----------------------------------------------------------------------------
// E-3 instrument-first probe (R6 Step 1, 2026-08-01)
// ----------------------------------------------------------------------------
// Env-gated, zero-intrusion probe for APU frame counter state machine +
// $4017 write. Activated by setting FCEUX11_E3_TRACE=1 in the environment.
// When inactive, all branches fold to a single getenv result check (~1 ns).
// Records (fcnt, IRQFrameMode, fhcnt, SIRQStat) at every quarter-step, plus
// before/after state at every $4017 write. See
// docs/history/e6_survey/r6_step1_instrument_data_2026-08-01.md.
static bool e3_trace_on() {
	// Cached on first call. Setting FCEUX11_E3_TRACE after the
	// process has started will NOT turn tracing on; restart required.
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_E3_TRACE");
		return e && e[0] == '1' && e[1] == '\0';
	}();
	return on;
}

// P2 Phase 2 Step 2.2 (2026-08-01): windowed hook trace. Armed (set to a
// positive count) on every $4017 write while E3 tracing is on; each
// FCEU_SoundCPUHook call prints ts/cycles/fhcnt while > 0. Bounded (~8
// quarters) so stderr stays small. Used to measure the write->quarter->
// IRQ trajectory at instruction granularity (Step 2.2 instrument-first).
static int e3_hook_trace_window = 0;

// P2 Phase 2 Step 2.2 深化 (2026-08-02, instrument-first): absolute cycle
// counter for the E3 probe. g_cpu.timestamp_ref() wraps at frame boundaries
// in some runner paths, which makes write->quarter->read deltas ambiguous.
// This counter is accumulated in FCEU_SoundCPUHook (one add per instruction,
// before dispatch), so FSU/W4017/R4015 lines stamped during dispatch report
// the instruction-end absolute cycle. Env-gated with the rest of E3 tracing.
static uint64_t e3_abs_ts = 0;

// P2 Phase 2 Step 2.2 深化 (2026-08-02): cycle-position frame counter.
// The legacy uniform-countdown model (quarter = fhinc/48 = 7457.5 cycles)
// could not satisfy blargg's 05.len_timing window at any offset: the
// hardware's quarter positions are NON-uniform (7457/14913/22371/29828-30),
// so the first and second length clocks land at write+14916 / write+29832
// while a uniform model can only hit one of them. We port the validated
// Mesen2/RustyNES model: fhcnt now holds the CPU-cycle position since the
// last sequence start; the $4017 write schedules a 3/4-cycle maturation
// (fc_reset_in) before the new sequence begins (3 cycles when the write
// lands on an even CPU cycle = APU-aligned, 4 otherwise - the clock jitter).
static int32_t fc_reset_in = 0;
static uint8_t fc_pending_mode = 0;     // reduced 2-bit mode of pending write


static uint32 wlookup1[32];
static uint32 wlookup2[203];

// v1.6 Resonance Phase C2: TriCount/TriMode/tristep/wlcount/PSG/DMCFormat/
// RawDALatch/DMCAddressLatch/DMCSizeLatch/EnabledChannels/IRQFrameMode/
// InitialRawDALatch/DMC_7bit/EnvUnits migrated to fceu11::g_apu; accessed
// through reference aliases declared in sound.h.

static const int RectDuties[4]={1,2,4,6};

// v1.6 Resonance Phase C2: RectDutyCount/sweepon/curfreq/SweepCount/
// SweepReload/nreg/fcnt/fhcnt/fhinc/sqacc/lengthcount migrated to
// fceu11::g_apu; accessed through reference aliases declared in sound.h.

static const uint8 lengthtable[0x20]=
{
	10,254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
	12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
};


extern const uint32 NoiseFreqTableNTSC[0x10] =
{
	4, 8, 16, 32, 64, 96, 128, 160, 202,
	254, 380, 508, 762, 1016, 2034, 4068
};

extern const uint32 NoiseFreqTablePAL[0x10] =
{
	4, 7, 14, 30, 60, 88, 118, 148, 188,
	236, 354, 472, 708,  944, 1890, 3778
};


static const uint32 NTSCDMCTable[0x10]=
{
 428,380,340,320,286,254,226,214,
 190,160,142,128,106, 84 ,72,54
};

/* Previous values for PAL DMC was value - 1,
 * I am not certain if this is if FCEU handled
 * PAL differently or not, the NTSC values are right,
 * so I am assuming that the current value is handled
 * the same way NTSC is handled. */

static const uint32 PALDMCTable[0x10]=
{
	398, 354, 316, 298, 276, 236, 210, 198,
	176, 148, 132, 118,  98,  78,  66,  50
};

// v1.6 Resonance Phase C2: DMCacc/DMCPeriod/DMCBitCount/DMCAddress/
// DMCAddressLatch/DMCSize/DMCSizeLatch/DMCShift/SIRQStat/DMCHaveDMA/
// DMCDMABuf/DMCHaveSample migrated to fceu11::g_apu; accessed through
// reference aliases declared in sound.h.

static void Dummyfunc(void) {};
static void (*DoNoise)(void)=Dummyfunc;
static void (*DoTriangle)(void)=Dummyfunc;
static void (*DoPCM)(void)=Dummyfunc;
static void (*DoSQ1)(void)=Dummyfunc;
static void (*DoSQ2)(void)=Dummyfunc;

// hotfix3 D-3: lifted from local statics in RDoTriangleNoisePCMLQ to file
// scope so the split helper functions (do_tnp_*) can access them.
static uint32 tcout = 0;
static int32  triacc = 0;
static int32  noiseacc = 0;

static uint32 ChannelBC[5];

//savestate sync hack stuff
int movieSyncHackOn=0,resetDMCacc=0,movieConvertOffset1,movieConvertOffset2;

#ifdef WIN32
extern volatile int datacount, undefinedcount;
extern int debug_loggingCD;
extern unsigned char *cdloggerdata;
#endif

static void LoadDMCPeriod(uint8 V)
{
 if(PAL)
  DMCPeriod=PALDMCTable[V];
 else
  DMCPeriod=NTSCDMCTable[V];
}

static void PrepDPCM()
{
 DMCAddress=0x4000+(DMCAddressLatch<<6);
 DMCSize=(DMCSizeLatch<<4)+1;

 #ifdef WIN32
 if(debug_loggingCD)LogDPCM(0x8000+DMCAddress, DMCSize);
 #endif

}

void LogDPCM(int romaddress, int dpcmsize){
	int i = GetPRGAddress(romaddress);

	if(i == -1)return;

	for (int dpcmstart = i; dpcmstart < (i + dpcmsize); dpcmstart++) {
		if(!(cdloggerdata[dpcmstart] & 0x40)) {
			cdloggerdata[dpcmstart] |= 0x40;
			cdloggerdata[dpcmstart] |= (romaddress >> 11) & 0x0c;

			if(!(cdloggerdata[dpcmstart] & 2)){
				datacount++;
				cdloggerdata[dpcmstart] |= 2;
				if(!(cdloggerdata[dpcmstart] & 1))undefinedcount--;
			}
		}
	}
}

/* Instantaneous?  Maybe the new freq value is being calculated all of the time... */

/*static*/ int CheckFreq(uint32 cf, uint8 sr)
{
 uint32 mod;
 if(!(sr&0x8))
 {
  mod=cf>>(sr&7);
  if((mod+cf)&0x800)
   return(0);
 }
 return(1);
}

static void SQReload(int x, uint8 V)
{
	if(EnabledChannels&(1<<x))
		lengthcount[x]=lengthtable[(V>>3)&0x1f];

	/* use the low 8 bits data from pulse period
	 * instead of from the sweep period */
	/* https://forums.nesdev.com/viewtopic.php?t=219&p=1431 */
	curfreq[x]=(curfreq[x] & 0xff)|((V&7)<<8);
	RectDutyCount[x]=7;
	EnvUnits[x].reloaddec=1;
}

static DECLFW(Write_PSG)
{
	A&=0x1F;
	switch(A)
	{
	case 0x0:
		DoSQ1();
		EnvUnits[0].Mode=(V&0x30)>>4;
		EnvUnits[0].Speed=(V&0xF);
		if (swapDuty)
			V = (V&0x3F)|((V&0x80)>>1)|((V&0x40)<<1);
		break;
	case 0x1:
		DoSQ1();
		sweepon[0]=V&0x80;
		SweepReload[0]=1;
		break;
	case 0x2:
		DoSQ1();
		curfreq[0]&=0xFF00;
		curfreq[0]|=V;
		break;
	case 0x3:
		DoSQ1();
		SQReload(0,V);
		break;
	case 0x4:
		DoSQ2();
		EnvUnits[1].Mode=(V&0x30)>>4;
		EnvUnits[1].Speed=(V&0xF);
		if (swapDuty)
			V = (V&0x3F)|((V&0x80)>>1)|((V&0x40)<<1);
		break;
	case 0x5:
		DoSQ2();
		sweepon[1]=V&0x80;
		SweepReload[1]=1;
		break;
	case 0x6:
		DoSQ2();
		curfreq[1]&=0xFF00;
		curfreq[1]|=V;
		break;
	case 0x7:
		DoSQ2();
		SQReload(1,V);
		break;
	case 0xa:
		DoTriangle();
		break;
	case 0xb:
		DoTriangle();
		if(EnabledChannels&0x4)
			lengthcount[2]=lengthtable[(V>>3)&0x1f];
		TriMode=1;	// Load mode
		break;
	case 0xC:
		DoNoise();
		EnvUnits[2].Mode=(V&0x30)>>4;
		EnvUnits[2].Speed=(V&0xF);
		break;
	case 0xE:
		DoNoise();
		break;
	case 0xF:
		DoNoise();
		if(EnabledChannels&0x8)
			lengthcount[3]=lengthtable[(V>>3)&0x1f];
		EnvUnits[2].reloaddec=1;
		break;
	case 0x10:
		DoPCM();
		LoadDMCPeriod(V&0xF);
		if(SIRQStat&0x80)
		{
			if(!(V&0x80))
			{
				X6502_IRQEnd(FCEU_IQDPCM);
				SIRQStat&=~0x80;
			}
			else X6502_IRQBegin(FCEU_IQDPCM);
		}
		break;
	}
	PSG[A]=V;
}

static DECLFW(Write_DMCRegs)
{
	A&=0xF;
	
	switch(A)
	{
	case 0x00:
		DoPCM();
	    LoadDMCPeriod(V&0xF);
	
	    if(SIRQStat&0x80)
	    {
			if(!(V&0x80))
			{
				X6502_IRQEnd(FCEU_IQDPCM);
				SIRQStat&=~0x80;
			}
			else X6502_IRQBegin(FCEU_IQDPCM);
	    }
		DMCFormat=V;
		break;
	case 0x01:
		DoPCM();
		InitialRawDALatch=V&0x7F;
		RawDALatch=InitialRawDALatch;
		if (RawDALatch)
			DMC_7bit = 1;
		break;
	case 0x02:
		DMCAddressLatch=V;
		if (V)
			DMC_7bit = 0;
		break;
	case 0x03:
		DMCSizeLatch=V;
		if (V)
			DMC_7bit = 0;
		break;
	}
}

static DECLFW(StatusWrite)
{
	int x;

    DoSQ1();
    DoSQ2();
    DoTriangle();
    DoNoise();
    DoPCM();

    for(x=0;x<4;x++)
		if(!(V&(1<<x))) lengthcount[x]=0;   /* Force length counters to 0. */

    if(V&0x10)
    {
		if(!DMCSize)
			PrepDPCM();
    }
	else
	{
		DMCSize=0;
	}
	SIRQStat&=~0x80;
	X6502_IRQEnd(FCEU_IQDPCM);
	EnabledChannels=V&0x1F;
}

static DECLFR(StatusRead)
{
   int x;
   uint8 ret;

   // R6 Step 3 (2026-08-01, defect-1 deep probe): record every $4015 read
   // (blargg's wait_n timer end point) with current frame-counter state.
   // This pins down exactly when blargg reads the IRQ flag vs when we set it.
   if (e3_trace_on()) {
    fprintf(stderr, "E3 R4015 abs=%llu ts=%u fcnt=%u mode=0x%X fhcnt=%d sirq=0x%X lc=[%u,%u,%u,%u]\n",
     (unsigned long long)e3_abs_ts, (unsigned)g_cpu.timestamp_ref(), (unsigned)fcnt, (unsigned)IRQFrameMode, fhcnt, (unsigned)SIRQStat,
     (unsigned)lengthcount[0], (unsigned)lengthcount[1], (unsigned)lengthcount[2], (unsigned)lengthcount[3]);
   }
   ret=SIRQStat;

   for(x=0;x<4;x++) ret|=lengthcount[x]?(1<<x):0;
   if(DMCSize) ret|=0x10;

   #ifdef FCEUDEF_DEBUGGER
   if(!fceuindbg)
   #endif
   {
    SIRQStat&=~0x40;
    X6502_IRQEnd(FCEU_IQFCOUNT);
   }
   return ret;
}

static void FrameSoundStuff(int V)
{
 int P;

 DoSQ1();
 DoSQ2();
 DoNoise();
 DoTriangle();

 if(!(V&1)) /* Envelope decay, linear counter, length counter, freq sweep */
 {
  if(!(PSG[8]&0x80))
   if(lengthcount[2]>0)
    lengthcount[2]--;

  if(!(PSG[0xC]&0x20))  /* Make sure loop flag is not set. */
   if(lengthcount[3]>0)
    lengthcount[3]--;

  for(P=0;P<2;P++)
  {
   if(!(PSG[P<<2]&0x20))  /* Make sure loop flag is not set. */
    if(lengthcount[P]>0)
     lengthcount[P]--;

   /* Frequency Sweep Code Here */
   /* xxxx 0000 */
   /* xxxx = hz.  120/(x+1)*/
   /* http://wiki.nesdev.com/w/index.php/APU_Sweep */
   /* https://forums.nesdev.com/viewtopic.php?t=219&p=1431 */
   if (SweepCount[P] > 0) SweepCount[P]--;
   if (SweepCount[P] <= 0)
   {
    int sweepShift = (PSG[(P << 2) + 0x1] & 7);
    if (sweepon[P] && sweepShift && curfreq[P] >= 8)
    {
     int32 mod = (curfreq[P] >> sweepShift);
     if (PSG[(P << 2) + 0x1] & 0x8)
      curfreq[P] -= (mod + (P ^ 1));
     else if ((mod + curfreq[P]) < 0x800)
      curfreq[P] += mod;
    }

    SweepCount[P] = (((PSG[(P << 2) + 0x1] >> 4) & 7) + 1);
   }

   if (SweepReload[P])
   {
    SweepCount[P] = (((PSG[(P << 2) + 0x1] >> 4) & 7) + 1);
    SweepReload[P] = 0;
   }
  }
 }

 /* Now do envelope decay + linear counter. */

  if(TriMode) // In load mode?
   TriCount=PSG[0x8]&0x7F;
  else if(TriCount)
   TriCount--;

  if(!(PSG[0x8]&0x80))
   TriMode=0;

  for(P=0;P<3;P++)
  {
   if(EnvUnits[P].reloaddec)
   {
    EnvUnits[P].decvolume=0xF;
    EnvUnits[P].DecCountTo1=EnvUnits[P].Speed+1;
    EnvUnits[P].reloaddec=0;
    continue;
   }

   if(EnvUnits[P].DecCountTo1>0) EnvUnits[P].DecCountTo1--;
   if(EnvUnits[P].DecCountTo1==0)
   {
    EnvUnits[P].DecCountTo1=EnvUnits[P].Speed+1;
    if(EnvUnits[P].decvolume || (EnvUnits[P].Mode&0x2))
    {
     EnvUnits[P].decvolume--;
     EnvUnits[P].decvolume&=0xF;
    }
   }
  }
}

// P2 Phase 2 Step 2.2 深化 (2026-08-02): frame-counter event step.
// Quarter step = FrameSoundStuff(1) (envelope + triangle linear only);
// half step = FrameSoundStuff(0) (length + sweep + envelope + linear).
// fcnt tracks the step index for the FCNT savestate chunk / E3 traces.
static void FrameSoundEvent(bool quarter, bool half)
{
 if (e3_trace_on()) {
  fprintf(stderr, "E3 FSU abs=%llu ts=%u fcnt=%u mode=0x%X fc_cycle=%d sirq=0x%X q=%d h=%d\n",
   (unsigned long long)e3_abs_ts, (unsigned)g_cpu.timestamp_ref(), (unsigned)fcnt, (unsigned)IRQFrameMode,
   fhcnt, (unsigned)SIRQStat, (int)quarter, (int)half);
 }
 // A step that clocks length+sweep also clocks the envelope/linear units
 // (single FrameSoundStuff call; matches the legacy half-step behavior).
 FrameSoundStuff(quarter && !half ? 1 : 0);
 fcnt=(fcnt+1)&3;
}

// Frame IRQ flag visibility ($4015 bit 6) vs CPU IRQ line are separate:
// the flag is set unconditionally at the IRQ step; the line is asserted
// only when not inhibited (RustyNES Session-26 iter 5, Mesen2 semantics).
static void FrameIRQSet(void)
{
 // E-3 Track-B probe (v1.17 R6 task, 2026-08-08): IRQ_BEGIN recorder at
 // the IRQ-set step. The existing E3 FSU probe fires once per quarter-
 // step call; this E3B IRQ_BEGIN fires specifically when the frame
 // counter commits to setting the IRQ flag (entry of FrameIRQSet vs.
 // FrameIRQEnd - the two functions the fhcnt==33253/33254 boundary
 // toggles between). Distinct probe name (E3B IRQ_BEGIN / E3B IRQ_END)
 // so we can tag the SET-then-clear transition per frame. Pre-state
 // captures fcnt + IRQFrameMode + fhcnt + SIRQStat for correlation with
 // blargg's wait_n / irq_flag tests (apu_single_3_irq_flag,
 // apu_single_6_irq_timing, apu_reset_4017_timing).
 if (e3_trace_on()) {
  fprintf(stderr, "E3B IRQ_BEGIN abs=%llu ts=%u fcnt=%u mode=0x%X fhcnt=%d sirq_pre=0x%X\n",
   (unsigned long long)e3_abs_ts, (unsigned)g_cpu.timestamp_ref(), (unsigned)fcnt, (unsigned)IRQFrameMode,
   fhcnt, (unsigned)SIRQStat);
 }
 SIRQStat|=0x40;
 if (!(IRQFrameMode&0x1))
  X6502_IRQBegin(FCEU_IQFCOUNT);
}

static void FrameIRQEnd(void)
{
 if (IRQFrameMode&0x1)
 {
  SIRQStat&=~0x40;
  X6502_IRQEnd(FCEU_IQFCOUNT);
 }
 else
 {
  SIRQStat|=0x40;
  X6502_IRQBegin(FCEU_IQFCOUNT);
 }
}

// Advance the sequencer by one CPU cycle and fire any event at the current
// position. fhcnt holds the position (cycles since sequence start). Wraps
// the sequence at the terminal step.
static void FrameCounterTick(void)
{
 fhcnt++;
 if (IRQFrameMode&0x2)
 {
  // 5-step (mode 1): no frame IRQ.
  if (PAL)
  {
   if (fhcnt==8313) FrameSoundEvent(true,false);
   else if (fhcnt==16627) FrameSoundEvent(true,true);
   else if (fhcnt==24939) FrameSoundEvent(true,false);
   else if (fhcnt==41565) { FrameSoundEvent(true,true); }
   else if (fhcnt==41566) { fhcnt=0; fcnt=0; }
  }
  else
  {
   if (fhcnt==7457) FrameSoundEvent(true,false);
   else if (fhcnt==14913) FrameSoundEvent(true,true);
   else if (fhcnt==22371) FrameSoundEvent(true,false);
   else if (fhcnt==37281) FrameSoundEvent(true,true);
   else if (fhcnt==37282) { fhcnt=0; fcnt=0; }
  }
 }
 else
 {
  // 4-step (mode 0)
  if (PAL)
  {
   if (fhcnt==8313) FrameSoundEvent(true,false);
   else if (fhcnt==16627) FrameSoundEvent(true,true);
   else if (fhcnt==24939) FrameSoundEvent(true,false);
   else if (fhcnt==33252) FrameIRQSet();
   else if (fhcnt==33253) { FrameIRQSet(); FrameSoundEvent(true,true); }
   else if (fhcnt==33254) { FrameIRQEnd(); fhcnt=0; fcnt=0; }
  }
  else
  {
   if (fhcnt==7457) FrameSoundEvent(true,false);
   else if (fhcnt==14913) FrameSoundEvent(true,true);
   else if (fhcnt==22371) FrameSoundEvent(true,false);
   else if (fhcnt==29828) FrameIRQSet();
   else if (fhcnt==29829) { FrameIRQSet(); FrameSoundEvent(true,true); }
   else if (fhcnt==29830) { FrameIRQEnd(); fhcnt=0; fcnt=0; }
  }
 }
}

// Compat wrapper kept for the 5-step immediate clock path (write maturation):
// fires one quarter+half step without IRQ side effects.
void FrameSoundUpdate(void)
{
 FrameSoundEvent(true,true);
}


static INLINE void tester(void)
{
 if(DMCBitCount==0)
 {
  if(!DMCHaveDMA)
   DMCHaveSample=0;
  else
  {
   DMCHaveSample=1;
   DMCShift=DMCDMABuf;
   DMCHaveDMA=0;
  }
 }
}

static INLINE void DMCDMA(void)
{
  if(DMCSize && !DMCHaveDMA)
  {
   X6502_DMR(0x8000+DMCAddress);
   X6502_DMR(0x8000+DMCAddress);
   X6502_DMR(0x8000+DMCAddress);
   DMCDMABuf=X6502_DMR(0x8000+DMCAddress);
   DMCHaveDMA=1;
   DMCAddress=(DMCAddress+1)&0x7fff;
   DMCSize--;
   if(!DMCSize)
   {
    if(DMCFormat&0x40)
     PrepDPCM();
    else
    {
     if(DMCFormat&0x80) {
      SIRQStat|=0x80;
      X6502_IRQBegin(FCEU_IQDPCM);
     }
    }
   }
 }
}

void FCEU_SoundCPUHook(int cycles)
{
 // R6 Step 3 (2026-08-01, defect-1 deep probe): two new sample probes
 // to localize the fhcnt cumulative-drift vs quarter-boundary timing.
 //  (a) HOOK_SAMPLE: every 4096th hook call (~0.1s) records fhcnt to
 //      see long-term drift.
 //  (b) HOOK_TRIG: every quarter-frame trigger records (cycles, fhcnt
 //      before decrement, fhinc) to see exact trigger moment.
 // NOTE: skipping tcount here — it's not in Cpu's public API in v1.4
 // (it lives in private layout_). fhcnt is the primary signal.
 static uint32_t hook_call_count = 0;
 hook_call_count++;
 e3_abs_ts += (uint64_t)(uint32_t)cycles;
 // P2 Phase 2 Step 2.2 深化 (2026-08-02): cycle-position frame counter.
 // Advance the sequencer one CPU cycle at a time through this instruction,
 // firing events at the exact table positions (NTSC mode 0: 7457/14913/
 // 22371/29828/29829/29830), and mature any pending $4017 write reset
 // (3 cycles on an even CPU cycle, 4 on odd - the clock jitter).
 const int32_t fhcnt_at_entry = fhcnt;
 bool fired_quarter = false;
 int32_t rem = cycles;
 while (rem > 0) {
  rem--;
  if (fc_reset_in > 0) {
   if (--fc_reset_in == 0) {
    // Pending $4017 write takes effect: new sequence starts at cycle 0.
    fhcnt = 0;
    fcnt = 0;
    if (fc_pending_mode & 0x2) {
     // 5-step write: immediate quarter+half clock at reset maturation.
     FrameSoundUpdate();
    }
    fired_quarter = true;
   }
   continue;
  }
  FrameCounterTick();
 }
 // P2 Phase 2 Step 2.2 (2026-08-01): windowed per-call trace, armed by the
 // last $4017 write. Records ts/cycles/fhcnt before+after so the
 // write->quarter->IRQ trajectory is measurable at instruction granularity.
 if (e3_trace_on() && e3_hook_trace_window > 0) {
  e3_hook_trace_window--;
  fprintf(stderr, "E3 HOOKWIN abs=%llu ts=%u cycles=%d fhcnt_before=%d fhcnt_after=%d fcnt=%u win=%d fired=%d\n",
   (unsigned long long)e3_abs_ts, (unsigned)g_cpu.timestamp_ref(), cycles, fhcnt_at_entry, fhcnt, (unsigned)fcnt, e3_hook_trace_window, (int)fired_quarter);
 }
 if (e3_trace_on()) {
  if ((hook_call_count & 0xFFF) == 0) {
   fprintf(stderr, "E3 HOOK_SAMPLE call=%u fhcnt=%d\n",
    hook_call_count, fhcnt);
  }
  if (fired_quarter) {
   fprintf(stderr, "E3 HOOK_TRIG call=%u abs=%llu cycles=%d fc_before=%d fc_after=%d fcnt=%u\n",
    hook_call_count, (unsigned long long)e3_abs_ts, cycles, fhcnt_at_entry, fhcnt, (unsigned)fcnt);
  }
 }

 DMCDMA();
 DMCacc-=cycles;

 while(DMCacc<=0)
 {
  if(DMCHaveSample)
  {
   uint8 bah=RawDALatch;
   int t=((DMCShift&1)<<2)-2;

   /* Unbelievably ugly hack */
   if(FSettings.SndRate)
   {
		// hotfix3 C-3: lift negation through int64 — direct `-DMCacc` is signed
		// overflow UB when DMCacc == INT32_MIN. Cast back to uint32 once safe.
		const uint32 fudge = std::min<uint32>(
				static_cast<uint32>(-(int64_t)DMCacc),
				soundtsoffs + g_cpu.timestamp_ref());
		soundtsoffs -= fudge;
		DoPCM();
		soundtsoffs += fudge;
   }
   RawDALatch+=t;
   if(RawDALatch&0x80)
    RawDALatch=bah;
  }

  DMCacc+=DMCPeriod;
  DMCBitCount=(DMCBitCount+1)&7;
  DMCShift>>=1;
  tester();
 }
}

void RDoPCM(void)
{
 uint32 V; //mbg merge 7/17/06 made uint32

 for(V=ChannelBC[4];V<SOUNDTS;V++)
  WaveHi[V]+=(((RawDALatch<<16)/256) * FSettings.PCMVolume)&(~0xFFFF); // TODO get rid of floating calculations to binary. set log volume scaling.
 ChannelBC[4]=SOUNDTS;
}

/* This has the correct phase.  Don't mess with it. */
static INLINE void RDoSQ(int x)		//Int x decides if this is Square Wave 1 or 2
{
   int32 V;
   int32 amp, ampx;
   int32 rthresh;
   int32 *D;
   int32 currdc;
   int32 cf;
   int32 rc;

   if(curfreq[x]<8 || curfreq[x]>0x7ff)
    goto endit;
   if(!CheckFreq(curfreq[x],PSG[(x<<2)|0x1]))
    goto endit;
   if(!lengthcount[x])
    goto endit;

   if(EnvUnits[x].Mode&0x1)
    amp=EnvUnits[x].Speed;
   else
    amp=EnvUnits[x].decvolume;	//Set the volume of the Square Wave

   //Modify Square wave volume based on channel volume modifiers
   //adelikat: Note: the formulat x = x * y /100 does not yield exact results, but is "close enough" and avoids the need for using double vales or implicit cohersion which are slower (we need speed here)
   ampx = x ? FSettings.Square2Volume : FSettings.Square1Volume; // TODO OPTIMIZE ME!
   if (ampx != 256) amp = (amp * ampx) / 256; // CaH4e3: fixed - setting up maximum volume for square2 caused complete mute square2 channel

   amp<<=24;

   rthresh=RectDuties[(PSG[(x<<2)]&0xC0)>>6];

   D=&WaveHi[ChannelBC[x]];
   V=SOUNDTS-ChannelBC[x];

   currdc=RectDutyCount[x];
   cf=(curfreq[x]+1)*2;
   rc=wlcount[x];

   while(V>0)
   {
    if(currdc<rthresh)
     *D+=amp;
    rc--;
    if(!rc)
    {
     rc=cf;
     currdc=(currdc+1)&7;
    }
    V--;
    D++;
   }

   RectDutyCount[x]=currdc;
   wlcount[x]=rc;

   endit:
   ChannelBC[x]=SOUNDTS;
}

static void RDoSQ1(void)
{
 RDoSQ(0);
}

static void RDoSQ2(void)
{
 RDoSQ(1);
}

// hotfix3 D-3: 2-way split of RDoSQLQ. Helper functions keep the
// existing channel-state semantics exactly. A more aggressive 4-way
// split (skipping the ch0 catchup when `inie[0]==0` or vice-versa)
// would alter RectDutyCount carry-over state and potentially
// audible at the call-N -> call-(N+1) boundary where a previously-
// gated channel re-enables. That change is deferred to a future
// hotfix after per-game audio regressions can be characterised.
static FCEU_ALWAYS_INLINE void do_sq_lq_silent(int32 start, int32 end, int32 totalout)
{
   for (int32 V = start; V < end; V++)
       Wave[V>>4] += totalout;
}

static FCEU_ALWAYS_INLINE void do_sq_lq_active(int32 start, int32 end, int32* totalout_ptr,
                                              const int32 inie[2], const int32 freq[2],
                                              const int32 ttable[2][8])
{
   for (int32 V = start; V < end; V++) {
       Wave[V>>4] += *totalout_ptr;

       sqacc[0] -= inie[0];
       sqacc[1] -= inie[1];

       if (sqacc[0] <= 0) {
       rea:
           sqacc[0] += freq[0];
           RectDutyCount[0] = (RectDutyCount[0]+1) & 7;
           if (sqacc[0] <= 0) goto rea;
           *totalout_ptr = wlookup1[ ttable[0][RectDutyCount[0]] + ttable[1][RectDutyCount[1]] ];
       }

       if (sqacc[1] <= 0) {
       rea2:
           sqacc[1] += freq[1];
           RectDutyCount[1] = (RectDutyCount[1]+1) & 7;
           if (sqacc[1] <= 0) goto rea2;
           *totalout_ptr = wlookup1[ ttable[0][RectDutyCount[0]] + ttable[1][RectDutyCount[1]] ];
       }
   }
}

static void RDoSQLQ(void)
{
   int32 start,end;
   int32 amp[2], ampx;
   int32 rthresh[2];
   int32 freq[2];
   int x;
   int32 inie[2];

   int32 ttable[2][8];
   int32 totalout;

   start=ChannelBC[0];
   end=(SOUNDTS<<16)/soundtsinc;
   if(end<=start) return;
   ChannelBC[0]=end;

   for(x=0;x<2;x++)
   {
    int y;

    inie[x]=nesincsize;
    if(curfreq[x]<8 || curfreq[x]>0x7ff)
     inie[x]=0;
    if(!CheckFreq(curfreq[x],PSG[(x<<2)|0x1]))
     inie[x]=0;
    if(!lengthcount[x])
     inie[x]=0;

    if(EnvUnits[x].Mode&0x1)
     amp[x]=EnvUnits[x].Speed;
    else
     amp[x]=EnvUnits[x].decvolume;

	//Modify Square wave volume based on channel volume modifiers
	//adelikat: Note: the formulat x = x * y /100 does not yield exact results, but is "close enough" and avoids the need for using double vales or implicit cohersion which are slower (we need speed here)
    // hotfix1 P2-7 (H-20): the LQ path used to read the volume in the
    // wrong order — `x ? Square1 : Square2` swapped the two channels
    // compared with the HQ path (line 572, which reads `x ? Square2 :
    // Square1`). Adjusting Square1's slider in the GUI therefore
    // affected Square2's LQ output and vice versa. Align with HQ so the
    // two paths finally agree on which knob scales which channel.
    ampx = x ? FSettings.Square2Volume : FSettings.Square1Volume;
    if (ampx != 256) amp[x] = (amp[x] * ampx) / 256; // CaH4e3: fixed - setting up maximum volume for square2 caused complete mute square2 channel

    if(!inie[x]) amp[x]=0;    /* Correct? Buzzing in MM2, others otherwise... */

    rthresh[x]=RectDuties[(PSG[x*4]&0xC0)>>6];

    for(y=0;y<8;y++)
    {
     if(y < rthresh[x])
      ttable[x][y] = amp[x];
     else
      ttable[x][y] = 0;
    }
    freq[x]=(curfreq[x]+1)<<1;
    freq[x]<<=17;
   }

   totalout = wlookup1[ ttable[0][RectDutyCount[0]] + ttable[1][RectDutyCount[1]] ];

   // hotfix3 D-3: 2-way dispatch. Silent path skips all accumulator
   // work. Active path does the existing body verbatim (no state-
   // mutation change vs pre-fix; see note on aggressive 4-way above).
   if (!inie[0] && !inie[1])
       do_sq_lq_silent(start, end, totalout);
   else
       do_sq_lq_active(start, end, &totalout, inie, freq, ttable);
}

static void RDoTriangle(void)
{
 uint32 V; //mbg merge 7/17/06 made uitn32
 int32 tcout;

 tcout=(tristep&0xF);
 if(!(tristep&0x10)) tcout^=0xF;
 tcout=(tcout*3) << 16;  //(tcout<<1);

 if(!lengthcount[2] || !TriCount)
 {           /* Counter is halted, but we still need to output. */
  /*int32 *start = &WaveHi[ChannelBC[2]];
  int32 count = SOUNDTS - ChannelBC[2];
  while(count--)
  {
   //Modify volume based on channel volume modifiers
   *start += (tcout/256*FSettings.TriangleVolume)&(~0xFFFF);  // TODO OPTIMIZE ME NOW DAMMIT!
   start++;
  }*/
  int32 cout = (tcout/256*FSettings.TriangleVolume)&(~0xFFFF);
  for(V=ChannelBC[2];V<SOUNDTS;V++)
   WaveHi[V]+=cout;
 }
 else
  for(V=ChannelBC[2];V<SOUNDTS;V++)
  {
    //Modify volume based on channel volume modifiers
	WaveHi[V]+=(tcout/256*FSettings.TriangleVolume)&(~0xFFFF);  // TODO OPTIMIZE ME!
    wlcount[2]--;
    if(!wlcount[2])
    {
     wlcount[2]=(PSG[0xa]|((PSG[0xb]&7)<<8))+1;
     tristep++;
     tcout=(tristep&0xF);
     if(!(tristep&0x10)) tcout^=0xF;
     tcout=(tcout*3) << 16;
    }
  }

 ChannelBC[2]=SOUNDTS;
}

// hotfix3 D-3: 4-way split of RDoTriangleNoisePCMLQ. Each helper is the
// exact body of the corresponding branch in the pre-fix code; the
// split makes the call site dispatch per-channel instead of walking
// the per-frame inner loop with two accumulator-update conditionals
// for whichever channels happen to be active. Both statics
// (triacc/noiseacc/tcout) stay shared and are mutated across calls.
static FCEU_ALWAYS_INLINE void do_tnp_lq_silent(int32 start, int32 end, int32 totalout)
{
   for (int32 V = start; V < end; V++)
       Wave[V>>4] += totalout;
}

static FCEU_ALWAYS_INLINE void do_tnp_lq_triangle_only(int32 start, int32 end, int32* totalout_ptr,
                                                     int32 freq0, int32 inie0, uint32 noiseout)
{
   for (int32 V = start; V < end; V++) {
       Wave[V>>4] += *totalout_ptr;
       triacc -= inie0;
       if (triacc <= 0) {
       area:
           triacc += freq0;
           tristep = (tristep + 1) & 0x1F;
           if (triacc <= 0) goto area;
           tcout = (tristep & 0xF);
           if (!(tristep & 0x10)) tcout ^= 0xF;
           tcout = tcout * 3;
           *totalout_ptr = wlookup2[tcout + noiseout + RawDALatch];
       }
   }
}

static FCEU_ALWAYS_INLINE void do_tnp_lq_noise_only(int32 start, int32 end, int32* totalout_ptr,
                                                   int32 inie1, uint32* noiseout_ptr,
                                                   int nshift, uint32 amptab[2])
{
   for (int32 V = start; V < end; V++) {
       Wave[V>>4] += *totalout_ptr;
       noiseacc -= inie1;
       if (noiseacc <= 0) {
       area2:
           if (PAL)
               noiseacc += NoiseFreqTablePAL[PSG[0xE]&0xF] << (16 + 1);
           else
               noiseacc += NoiseFreqTableNTSC[PSG[0xE]&0xF] << (16 + 1);
           nreg = (nreg << 1) + (((nreg >> nshift) ^ (nreg >> 14)) & 1);
           nreg &= 0x7fff;
           *noiseout_ptr = amptab[(nreg >> 0xe) & 1];
           if (noiseacc <= 0) goto area2;
           *totalout_ptr = wlookup2[tcout + *noiseout_ptr + RawDALatch];
       }
   }
}

static FCEU_ALWAYS_INLINE void do_tnp_lq_both(int32 start, int32 end, int32* totalout_ptr,
                                              int32 freq0, int32 inie0, int32 inie1,
                                              uint32* noiseout_ptr, int nshift,
                                              uint32 amptab[2])
{
   for (int32 V = start; V < end; V++) {
       Wave[V>>4] += *totalout_ptr;
       triacc -= inie0;
       noiseacc -= inie1;
       if (triacc <= 0) {
       rea:
           triacc += freq0;
           tristep = (tristep + 1) & 0x1F;
           if (triacc <= 0) goto rea;
           tcout = (tristep & 0xF);
           if (!(tristep & 0x10)) tcout ^= 0xF;
           tcout = tcout * 3;
           *totalout_ptr = wlookup2[tcout + *noiseout_ptr + RawDALatch];
       }
       if (noiseacc <= 0) {
       rea2:
           if (PAL)
               noiseacc += NoiseFreqTablePAL[PSG[0xE]&0xF] << (16 + 1);
           else
               noiseacc += NoiseFreqTableNTSC[PSG[0xE]&0xF] << (16 + 1);
           nreg = (nreg << 1) + (((nreg >> nshift) ^ (nreg >> 14)) & 1);
           nreg &= 0x7fff;
           *noiseout_ptr = amptab[(nreg >> 0xe) & 1];
           if (noiseacc <= 0) goto rea2;
           *totalout_ptr = wlookup2[tcout + *noiseout_ptr + RawDALatch];
       }
   }
}

static void RDoTriangleNoisePCMLQ(void)
{
   int32 start,end;
   int32 freq[2];
   int32 inie[2];
   uint32 amptab[2];
   uint32 noiseout;
   int nshift;

   int32 totalout;

   start=ChannelBC[2];
   end=(SOUNDTS<<16)/soundtsinc;
   if(end<=start) return;
   ChannelBC[2]=end;

   inie[0]=inie[1]=nesincsize;

   freq[0]=(((PSG[0xa]|((PSG[0xb]&7)<<8))+1));

   if(!lengthcount[2] || !TriCount || freq[0]<=4)
    inie[0]=0;

   freq[0]<<=17;
   if(EnvUnits[2].Mode&0x1)
    amptab[0]=EnvUnits[2].Speed;
   else
    amptab[0]=EnvUnits[2].decvolume;

   //Modify Square wave volume based on channel volume modifiers
   //adelikat: Note: the formulat x = x * y /100 does not yield exact results, but is "close enough" and avoids the need for using double vales or implicit cohersion which are slower (we need speed here)
   if (FSettings.TriangleVolume != 256) amptab[0] = (amptab[0] * FSettings.TriangleVolume) / 256;  // TODO OPTIMIZE ME!

   amptab[1]=0;
   amptab[0]<<=1;

   if(!lengthcount[3])
    amptab[0]=inie[1]=0;  /* Quick hack speedup, set inie[1] to 0 */

   noiseout=amptab[(nreg>>0xe)&1];

   if(PSG[0xE]&0x80)
    nshift=8;
   else
    nshift=13;


   totalout = wlookup2[tcout+noiseout+RawDALatch];

   // hotfix3 D-3: 4-way dispatch. Each helper is the exact body of
   // its pre-fix branch; semantics preserved. The shared statics
   // (tcout, triacc, noiseacc) cross call boundaries regardless.
   if (inie[0] && inie[1])
       do_tnp_lq_both(start, end, &totalout, freq[0], inie[0], inie[1], &noiseout, nshift, amptab);
   else if (inie[0])
       do_tnp_lq_triangle_only(start, end, &totalout, freq[0], inie[0], noiseout);
   else if (inie[1])
       do_tnp_lq_noise_only(start, end, &totalout, inie[1], &noiseout, nshift, amptab);
   else
       do_tnp_lq_silent(start, end, totalout);
}


static void RDoNoise(void)
{
 uint32 V; //mbg merge 7/17/06 made uint32
 int32 outo;
 uint32 amptab[2];

 if(EnvUnits[2].Mode&0x1)
  amptab[0]=EnvUnits[2].Speed;
 else
  amptab[0]=EnvUnits[2].decvolume;

 //Modfiy Noise channel volume based on channel volume setting
 //adelikat: Note: the formulat x = x * y /100 does not yield exact results, but is "close enough" and avoids the need for using double vales or implicit cohersion which are slower (we need speed here)
 if (FSettings.NoiseVolume != 256) amptab[0] = (amptab[0] * FSettings.NoiseVolume) / 256;  // TODO OPTIMIZE ME!
 amptab[0]<<=16;
 amptab[1]=0;

 amptab[0]<<=1;

 outo=amptab[(nreg>>0xe)&1];

 if(!lengthcount[3])
 {
  outo=amptab[0]=0;
 }

 // hotfix3 D-3: collapse the two for-bodies by hoisting the only
 // difference - which feedback bit picks up the LFSR tap. Pre-fix
 // the entire body was duplicated for `PSG[0xE] & 0x80` short vs long
 // noise modes, with the only difference being feedback_shift = 8 or
 // 13. Single loop now reads the shift count once.
 const uint8_t feedback_shift = (PSG[0xE] & 0x80) ? 8 : 13;
 for (V=ChannelBC[3]; V<SOUNDTS; V++)
 {
   WaveHi[V] += outo;
   wlcount[3]--;
   if (!wlcount[3])
   {
     uint8 feedback;
     if (PAL)
       wlcount[3]=NoiseFreqTablePAL[PSG[0xE]&0xF];
     else
       wlcount[3]=NoiseFreqTableNTSC[PSG[0xE]&0xF];
     feedback=((nreg>>feedback_shift)&1)^((nreg>>14)&1);
     nreg=(nreg<<1)+feedback;
     nreg&=0x7fff;
     outo=amptab[(nreg>>0xe)&1];
   }
 }
 ChannelBC[3]=SOUNDTS;
}

DECLFW(Write_IRQFM)
{
 // E-3 probe (R6 Step 1, 2026-08-01): capture before/after $4017 write
 // state. Critical for verifying defect 2 hypothesis: $4017 write
 // unconditionally clears IRQ flag (should only clear when 5-step or
 // inhibit bit is set).
 if (e3_trace_on()) {
  uint8 pre_mode = IRQFrameMode, pre_fcnt = fcnt, pre_sirq = SIRQStat;
  uint32 ts = g_cpu.timestamp_ref();
  fprintf(stderr, "E3 W4017_IN abs=%llu ts=%u ts_mod4=%u V=0x%X pre_mode=0x%X pre_fcnt=%u pre_sirq=0x%X\n",
   (unsigned long long)e3_abs_ts, (unsigned)ts, (unsigned)(ts & 3), (unsigned)V, (unsigned)pre_mode, (unsigned)pre_fcnt, (unsigned)pre_sirq);
  // P2 Phase 2 Step 2.2 (2026-08-01): arm the windowed hook trace so the
  // post-write quarter trajectory is captured at instruction granularity.
  e3_hook_trace_window = 20000;
 }
 // R6-2a (2026-08-01): save the raw $4017 value BEFORE the (V&0xC0)>>6
 // reduction — bit 6 of the raw byte is the IRQ inhibit bit. Per Nesdev,
 // writing $4017 without the inhibit bit set must NOT clear the frame IRQ
 // flag; only a write that sets inhibit (raw bit6=1) clears it. blargg
 // apu_single_3_irq_flag #6 asserts "Writing $00 or $80 to $4017 shouldn't
 // affect flag". Previously this was unconditional (defect 2).
 const uint8 raw = V;
 V=(V&0xC0)>>6;
 // P2 Phase 2 Step 2.2 深化 (2026-08-02): cycle-position model. The write
 // schedules a reset that matures 3 CPU cycles later when the write lands
 // on an even CPU cycle (APU-aligned) and 4 cycles later when odd (the
 // clock jitter; RustyNES/Mesen2 validated). The new sequence starts at
 // cycle 0; a 5-step write additionally fires an immediate quarter+half
 // clock when the reset matures (handled in FCEU_SoundCPUHook).
 // P2 Phase 2 Step 2.2 深化 (2026-08-02): use the MONOTONIC absolute cycle
 // count for the APU write parity. g_cpu.timestamp_ref() is reset to 0 at
 // every frame boundary (fceu.cpp FCEU_Emulate), and NTSC frames are
 // 29780.67 cycles, so the within-frame parity flips at frame boundaries.
 // The APU-alignment jitter must track the true absolute parity or the
 // blargg sync_apu phase alignment (bne +/- 1 cycle) picks the wrong branch
 // after a frame boundary, landing the next $4017 write on the wrong
 // parity (apu_single_5 M4 "second length too late").
 const uint64 abs_ts = g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
 fc_reset_in = ((abs_ts & 1) == 0) ? 3 : 4;
 fc_pending_mode = V;
 if (raw & 0x40) {
  X6502_IRQEnd(FCEU_IQFCOUNT);
  SIRQStat&=~0x40;
 }
 IRQFrameMode=V;
 if (e3_trace_on()) {
  fprintf(stderr, "E3 W4017_OUT abs=%llu post_mode=0x%X post_fcnt=%u post_sirq=0x%X\n",
   (unsigned long long)e3_abs_ts, (unsigned)IRQFrameMode, (unsigned)fcnt, (unsigned)SIRQStat);
 }
}

void SetNESSoundMap(void)
{
  SetWriteHandler(0x4000,0x400F,Write_PSG);
  SetWriteHandler(0x4010,0x4013,Write_DMCRegs);
  SetWriteHandler(0x4017,0x4017,Write_IRQFM);

  SetWriteHandler(0x4015,0x4015,StatusWrite);
  SetReadHandler(0x4015,0x4015,StatusRead);
}

static int32 inbuf=0;
int FlushEmulateSound(void)
{
  int x;
  int32 end,left;

  if(!g_cpu.sound_timestamp_ref()) return(0);

  if(!FSettings.SndRate)
  {
   left=0;
   end=0;
   goto nosoundo;
  }

  DoSQ1();
  DoSQ2();
  DoTriangle();
  DoNoise();
  DoPCM();

  if(FSettings.soundq>=1)
  {
   int32 *tmpo=&WaveHi[soundtsoffs];

   FCEU11_ExpHiFill(&GameExpSound);

   for(x=g_cpu.sound_timestamp_ref();x;x--)
   {
    uint32 b=*tmpo;
    *tmpo=(b&65535)+wlookup2[(b>>16)&255]+wlookup1[b>>24];
    tmpo++;
   }
   end=NeoFilterSound(WaveHi,WaveFinal,SOUNDTS,&left);

   // hotfix3 D-4 REVERTED: the trailing memset must stay. The Do*()
   // channel writers (RDoSQ/RDoTriangle/RDoNoise/RDoPCM at
   // sound.cpp:598/760/766/965/553 and the mapper writers in
   // boards/vrc6, n106, 69) ACCUMULATE into WaveHi with `+=`, they do
   // NOT overwrite. So the [left, SOUNDTS) region every frame writes
   // into must be zeroed first; otherwise the previous frame's samples
   // remain and the next frame sums on top of them, producing runaway
   // amplitude / periodic buzz instead of music. The memmove keeps the
   // FIR carryover in [0, left); the memset re-zeros the accumulation
   // region for the next call.
   memmove(WaveHi, WaveHi+SOUNDTS-left, left*sizeof(uint32));
   memset(WaveHi+left, 0, sizeof(WaveHi)-left*sizeof(uint32));

   FCEU11_ExpHiSync(&GameExpSound, left);
   for(x=0;x<5;x++)
    ChannelBC[x]=left;
  }
  else
  {
   end=(SOUNDTS<<16)/soundtsinc;
   FCEU11_ExpFill(&GameExpSound, end&0xF);

   SexyFilter(Wave,WaveFinal,end>>4);

   if(end&0xF)
    Wave[0]=Wave[(end>>4)];
   Wave[end>>4]=0;
  }
  nosoundo:

  if(FSettings.soundq>=1)
  {
   soundtsoffs=left;
  }
  else
  {
   for(x=0;x<5;x++)
    ChannelBC[x]=end&0xF;
   soundtsoffs = (soundtsinc*(end&0xF))>>16;
   end>>=4;
  }
  inbuf=end;

  FCEU_WriteWaveData(WaveFinal, end); /* This function will just return
				    if sound recording is off. */
  return(end);
}

int GetSoundBuffer(int32 **W)
{
 *W=WaveFinal;
 return(inbuf);
}

/* FIXME:  Find out what sound registers get reset on reset.  I know $4001/$4005 don't,
due to that whole MegaMan 2 Game Genie thing.
*/

void FCEUSND_Reset(bool is_power)
{
	int x;

	// P2 Phase 2 Step 2.1 (2026-08-01): power-on / reset 相位分离。
	// 硬件语义（blargg_apu_2005.07.30 readme "Misc" + apu_reset/readme.txt，P2 方案 §1.2/§1.4）：
	//  - power-on：APU 等效"$4017=$00 写 + 9~12 时钟延迟"后开始执行 → IRQFrameMode=0x0（4-step，IRQ 使能）。
	//  - soft reset：重写最后一次写入 $4017 的值（非 $00）→ IRQFrameMode 保留最后写入值；
	//    bit6 inhibit 按"有时清除"语义处理，先按保留实测（用 apu_reset_4017_written 判据校准）。
	//  - 两者均为"写后相位"：fcnt=1（首个 IRQ 于第 4 个 quarter=29830，而非 fcnt=0 的首个 quarter≈7459）。
	// R6-2b (2026-08-01) 曾试 fcnt=1，被 golden 碎裂误伤回滚；P2 方案 §1.4 复核判定方向正确
	// （当时只修了一半：apu_single_4 走 $4017 写路径，与 power-on fcnt 无关，故未见效）。
	// 注：运行期起始值变化 → golden_savestate_test 哈希碎裂属预期流程
	// （tests/CMakeLists.txt:348 `fceux11_golden_savestate_test --generate` 重生，非事故）。
	if (is_power)
		IRQFrameMode=0x0;   // power: $4017=$00；reset: 保留最后写入值
	// P2 Phase 2 Step 2.2 深化 (2026-08-02): at power/reset the APU acts as
	// if $4017 were written with $00 (power) / the last value (reset) from
	// 9-12 clocks before the first instruction (blargg apu_reset readme).
	// Start the sequence already D cycles in so the frame IRQ phase lands in
	// apu_reset_4017_timing's measured window. Empirically the count tracks
	// ~count = 5 + D (D=0 -> 5, D=9 -> 14); D=4 -> count ~9, mid-window.
	// fhcnt holds the CPU-cycle position since sequence start.
	fhcnt=4;
	fcnt=0;
	fc_reset_in=0;
	nreg=1;

	for(x=0;x<2;x++)
	{
		wlcount[x]=2048;
		if(nesincsize) // lq mode
			sqacc[x]=((uint32)2048<<17)/nesincsize;
		else
			sqacc[x]=1;
		sweepon[x]=0;
		curfreq[x]=0;
	}

	wlcount[2]=1;  //2048;
	wlcount[3]=2048;

	DMCHaveDMA=DMCHaveSample=0;
	SIRQStat=0x00;

	RawDALatch=0x00;
	TriCount=0;
	TriMode=0;
	tristep=0;
	EnabledChannels=0;
	for(x=0;x<4;x++)
	 lengthcount[x]=0;

	DMCAddressLatch=0;
	DMCSizeLatch=0;
	DMCFormat=0;
	DMCAddress=0;
	DMCSize=0;
	DMCShift=0;

	// MAJOR BUG WAS HERE: DMCacc and DMCBitCount never got reset...
	// so, do some ridiculous hackery if a movie's about to play to keep it in sync...


	if(movieSyncHackOn)
	{
		if(resetDMCacc)
		{
			// no value in movie save state
		#ifdef WIN32
			// use editbox fields
			DMCacc=movieConvertOffset1;
			DMCBitCount=movieConvertOffset2;
		#else
			// no editbox fields, so leave the values alone
			// and print out a warning that says what they are
			FCEU_PrintError("Warning: These variables were not found in the save state and will keep their current value: DMCacc=%d, DMCBitCount=%d\n", DMCacc, DMCBitCount);
		#endif
		}
		else
		{
			// keep values loaded from movie save state or reset earlier
		}
	}
	else
	{
		// reset these variables like should have done in the first place
		DMCacc=1;
		DMCBitCount=0;
	}

//	FCEU_PrintError("DMCacc=%d, DMCBitCount=%d",DMCacc,DMCBitCount);
}

void FCEUSND_Power(void)
{
        int x;

        SetNESSoundMap();
        memset(PSG,0x00,sizeof(PSG));
	FCEUSND_Reset(true);   // power-on: $4017=$00 相位（P2 Step 2.1）

	memset(Wave,0,sizeof(Wave));
        memset(WaveHi,0,sizeof(WaveHi));
	memset(&EnvUnits,0,sizeof(EnvUnits));

        for(x=0;x<5;x++)
         ChannelBC[x]=0;
        soundtsoffs=0;
        LoadDMCPeriod(DMCFormat&0xF);
}


void SetSoundVariables(void)
{
  int x;

  fhinc=PAL?16626:14915;  // *2 CPU clock rate
  fhinc*=24;

  if(FSettings.SndRate)
  {
   wlookup1[0]=0;
   for(x=1;x<32;x++)
   {
    wlookup1[x]=(double)16*16*16*4*95.52/((double)8128/(double)x+100);
    if(!FSettings.soundq) wlookup1[x]>>=4;
   }
   wlookup2[0]=0;
   for(x=1;x<203;x++)
   {
    wlookup2[x]=(double)16*16*16*4*163.67/((double)24329/(double)x+100);
    if(!FSettings.soundq) wlookup2[x]>>=4;
   }
   if(FSettings.soundq>=1)
   {
    DoNoise=RDoNoise;
    DoTriangle=RDoTriangle;
    DoPCM=RDoPCM;
    DoSQ1=RDoSQ1;
    DoSQ2=RDoSQ2;
   }
   else
   {
    DoNoise=DoTriangle=DoPCM=DoSQ1=DoSQ2=Dummyfunc;
    DoSQ1=RDoSQLQ;
    DoSQ2=RDoSQLQ;
    DoTriangle=RDoTriangleNoisePCMLQ;
    DoNoise=RDoTriangleNoisePCMLQ;
    DoPCM=RDoTriangleNoisePCMLQ;
   }
  }
  else
  {
   DoNoise=DoTriangle=DoPCM=DoSQ1=DoSQ2=Dummyfunc;
   return;
  }

  MakeFilters(FSettings.SndRate);

  FCEU11_ExpRegionChanged(&GameExpSound);

  nesincsize=(int64)(((int64)1<<17)*(double)(PAL?PAL_CPU:NTSC_CPU)/(FSettings.SndRate * 16));
  memset(sqacc,0,sizeof(sqacc));
  memset(ChannelBC,0,sizeof(ChannelBC));

  LoadDMCPeriod(DMCFormat&0xF);  // For changing from PAL to NTSC

  soundtsinc=(uint32)((uint64)(PAL?(long double)PAL_CPU*65536:(long double)NTSC_CPU*65536)/(FSettings.SndRate * 16));
}

void fceu11::Sound(int Rate)
{
	FSettings.SndRate=Rate;
	SetSoundVariables();
}

void fceu11::SetLowPass(int q)
{
	FSettings.lowpass=q;
}

void fceu11::SetSoundQuality(int quality)
{
	FSettings.soundq=quality;
	SetSoundVariables();
}

void fceu11::SetSoundVolume(uint32 volume)
{
	FSettings.SoundVolume=volume;
}

void fceu11::SetTriangleVolume(uint32 volume)
{
	FSettings.TriangleVolume=volume;
}

void fceu11::SetSquare1Volume(uint32 volume)
{
	FSettings.Square1Volume=volume;
}

void fceu11::SetSquare2Volume(uint32 volume)
{
	FSettings.Square2Volume=volume;
}

void fceu11::SetNoiseVolume(uint32 volume)
{
	FSettings.NoiseVolume=volume;
}

void fceu11::SetPCMVolume(uint32 volume)
{
	FSettings.PCMVolume=volume;
}

SFORMAT FCEUSND_STATEINFO[]={

 // v1.6 Resonance Phase C2: every descriptor now points directly at the
 // canonical storage inside fceu11::g_apu. Chunk names, sizes, and order
 // are unchanged to keep savestates byte-compatible with v1.5.
 { &fceu11::g_apu.fhcnt(), 4|FCEUSTATE_RLSB,"FHCN"},
 { &fceu11::g_apu.fcnt(), 1, "FCNT"},
 { fceu11::g_apu.psg(), 0x10, "PSG"},
 { &fceu11::g_apu.enabled_channels(), 1, "ENCH"},
 { &fceu11::g_apu.irq_frame_mode(), 1, "IQFM"},
 { &fceu11::g_apu.nreg(), 2|FCEUSTATE_RLSB, "NREG"},
 { &fceu11::g_apu.tri_mode(), 1, "TRIM"},
 { &fceu11::g_apu.tri_count(), 1, "TRIC"},

 { &fceu11::g_apu.env_units()[0].Speed, 1, "E0SP"},
 { &fceu11::g_apu.env_units()[1].Speed, 1, "E1SP"},
 { &fceu11::g_apu.env_units()[2].Speed, 1, "E2SP"},

 { &fceu11::g_apu.env_units()[0].Mode, 1, "E0MO"},
 { &fceu11::g_apu.env_units()[1].Mode, 1, "E1MO"},
 { &fceu11::g_apu.env_units()[2].Mode, 1, "E2MO"},

 { &fceu11::g_apu.env_units()[0].DecCountTo1, 1, "E0D1"},
 { &fceu11::g_apu.env_units()[1].DecCountTo1, 1, "E1D1"},
 { &fceu11::g_apu.env_units()[2].DecCountTo1, 1, "E2D1"},

 { &fceu11::g_apu.env_units()[0].decvolume, 1, "E0DV"},
 { &fceu11::g_apu.env_units()[1].decvolume, 1, "E1DV"},
 { &fceu11::g_apu.env_units()[2].decvolume, 1, "E2DV"},

 { &fceu11::g_apu.lengthcount()[0], 4|FCEUSTATE_RLSB, "LEN0"},
 { &fceu11::g_apu.lengthcount()[1], 4|FCEUSTATE_RLSB, "LEN1"},
 { &fceu11::g_apu.lengthcount()[2], 4|FCEUSTATE_RLSB, "LEN2"},
 { &fceu11::g_apu.lengthcount()[3], 4|FCEUSTATE_RLSB, "LEN3"},
 { fceu11::g_apu.sweepon(), 2, "SWEE"},
 { &fceu11::g_apu.curfreq()[0], 4|FCEUSTATE_RLSB,"CRF1"},
 { &fceu11::g_apu.curfreq()[1], 4|FCEUSTATE_RLSB,"CRF2"},
 { fceu11::g_apu.sweep_count(), 2,"SWCT"},

 { &fceu11::g_apu.sirq_stat(), 1, "SIRQ"},

 { &fceu11::g_apu.dmc_acc(), 4|FCEUSTATE_RLSB, "5ACC"},
 { &fceu11::g_apu.dmc_bit_count(), 1, "5BIT"},
 { &fceu11::g_apu.dmc_address(), 4|FCEUSTATE_RLSB, "5ADD"},
 { &fceu11::g_apu.dmc_size(), 4|FCEUSTATE_RLSB, "5SIZ"},
 { &fceu11::g_apu.dmc_shift(), 1, "5SHF"},

 { &fceu11::g_apu.dmc_have_dma(), 1, "5HVDM"},
 { &fceu11::g_apu.dmc_have_sample(), 1, "5HVSP"},

 { &fceu11::g_apu.dmc_size_latch(), 1, "5SZL"},
 { &fceu11::g_apu.dmc_address_latch(), 1, "5ADL"},
 { &fceu11::g_apu.dmc_format(), 1, "5FMT"},
 { &fceu11::g_apu.raw_da_latch(), 1, "RWDA"},
 { 0 }
};

void FCEUSND_SaveState(void)
{

}

void FCEUSND_LoadState(int version)
{
 LoadDMCPeriod(DMCFormat&0xF);
 RawDALatch&=0x7F;
 DMCAddress&=0x7FFF;
}
