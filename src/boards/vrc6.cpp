/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2009 CaH4e3
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
 *
 * VRC-6
 *
 */

#include "mapinc.h"

static uint8 is26;
FCEUX11_MAPPER_HOT static uint8 prg[2], chr[8], mirr;
static uint8 IRQLatch, IRQa, IRQd, IRQMode;
static int32 IRQCount, CycleCount;
static uint8 *WRAM = NULL;
static FceuMallocPtr WRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint32 WRAMSIZE=0;

static SFORMAT StateRegs[] =
{
	{ prg, 2, "PRG" },
	{ chr, 8, "CHR" },
	{ &mirr, 1, "MIRR" },
	{ &IRQa, 1, "IRQA" },
	{ &IRQd, 1, "IRQD" },
	{ &IRQLatch, 1, "IRQL" },
	{ &IRQCount, 4, "IRQC" },
	{ &CycleCount, 4, "CYCC" },
	{ &IRQMode, 1, "IRQM" },
	{ 0 }
};

static void Sync(void) {
	uint8 i;
	if (is26)
		setprg8r(0x10, 0x6000, 0);
	setprg16(0x8000, prg[0]);
	setprg8(0xc000, prg[1]);
	setprg8(0xe000, ~0);
	for (i = 0; i < 8; i++)
		setchr1(i << 10, chr[i]);
	switch (mirr & 3) {
	case 0: setmirror(MI_V); break;
	case 1: setmirror(MI_H); break;
	case 2: setmirror(MI_0); break;
	case 3: setmirror(MI_1); break;
	}
}

static DECLFW(VRC6Write);

static void VRC6Power(void) {
	Sync();
	SetReadHandler(0x6000, 0xFFFF, CartBR);
	SetWriteHandler(0x6000, 0x7FFF, CartBW);
	SetWriteHandler(0x8000, 0xFFFF, VRC6Write);
	FCEU_CheatAddRAM(WRAMSIZE >> 10, 0x6000, WRAM);
}

static void VRC6IRQHook(int a) {
	if (IRQa) {
		if (IRQMode) {
			CycleCount += a;
			while (CycleCount > 0) {
				CycleCount--;
				IRQCount++;
				if (IRQCount & 0x100) {
					X6502_IRQBegin(FCEU_IQEXT);
					IRQCount = IRQLatch;
				}
			}
		} else {
			CycleCount += a * 3;
			while(CycleCount >= 341) {
				CycleCount -= 341;
				IRQCount++;
				if (IRQCount == 0x100) {
					IRQCount = IRQLatch;
					X6502_IRQBegin(FCEU_IQEXT);
				}
			}
		}
	}
}

static void VRC6Close(void)
{
	FCEU11_ExpKill(&GameExpSound);
	WRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	WRAM = nullptr;
}

static void StateRestore(int version) {
	Sync();
}

// ---------------------------------------------------------------------------
// VRC6 Sound — v1.6 Resonance Phase E: migrated to fceu11::ExpansionAudio.
// The class owns all VRC6 audio state. A single static instance lives for the
// process lifetime so AddExState pointers remain valid across game loads.
// ---------------------------------------------------------------------------

class Vrc6Audio : public fceu11::ExpansionAudio {
public:
	Vrc6Audio();

	void fill(int32_t count) override;
	void hi_fill() override;
	void hi_sync(int32_t ts) override;
	void region_changed() override;
	void kill() override;

	void run_sfun(int index) {
		if (sfun_[index]) sfun_[index]();
	}

	// Register arrays are reached directly by the static write handler.
	uint8 vpsg1_[8];
	uint8 vpsg2_[4];

public:
	// Called by the thin free-function wrappers bound to sfun_.
	void do_sqv(int x);
	void do_sqv_hq(int x);
	void do_sawv();
	void do_sawv_hq();

private:

	void full_reset();

	int32 cvbc_[3];
	int32 vcount_[3];
	int32 dcount_[2];
	void (*sfun_[3])(void);

	// LQ saw-channel persistent state (was function-local static in DoSawV).
	int32 lq_saw1phaseacc_;
	uint8 lq_saw_b3_;
	int32 lq_saw_phaseacc_;
	uint32 lq_saw_duff_;

	// HQ saw-channel persistent state (was function-local static in DoSawVHQ).
	uint8 hq_saw_b3_;
	int32 hq_saw_phaseacc_;

	SFORMAT state_regs_[3];
	bool state_registered_;
};

static Vrc6Audio g_vrc6_audio;

static void DoSQV1(void);
static void DoSQV2(void);
static void DoSawV(void);
static void DoSQV1HQ(void);
static void DoSQV2HQ(void);
static void DoSawVHQ(void);

Vrc6Audio::Vrc6Audio() : state_registered_(false) {
	state_regs_[0] = { vpsg1_, 8, "PSG1" };
	state_regs_[1] = { vpsg2_, 4, "PSG2" };
	state_regs_[2] = { nullptr, 0, nullptr };
	full_reset();
}

void Vrc6Audio::full_reset() {
	memset(vpsg1_, 0, sizeof(vpsg1_));
	memset(vpsg2_, 0, sizeof(vpsg2_));
	memset(cvbc_, 0, sizeof(cvbc_));
	memset(vcount_, 0, sizeof(vcount_));
	memset(dcount_, 0, sizeof(dcount_));
	memset(sfun_, 0, sizeof(sfun_));
	lq_saw1phaseacc_ = 0;
	lq_saw_b3_ = 0;
	lq_saw_phaseacc_ = 0;
	lq_saw_duff_ = 0;
	hq_saw_b3_ = 0;
	hq_saw_phaseacc_ = 0;
}

void Vrc6Audio::do_sqv(int x) {
	int32 V;
	int32 amp = (((vpsg1_[x << 2] & 15) << 8) * 6 / 8) >> 4;
	int32 start, end;

	start = cvbc_[x];
	end = (SOUNDTS << 16) / soundtsinc;
	if (end <= start) return;
	cvbc_[x] = end;

	if (vpsg1_[(x << 2) | 0x2] & 0x80) {
		if (vpsg1_[x << 2] & 0x80) {
			for (V = start; V < end; V++)
				Wave[V >> 4] += amp;
		} else {
			int32 thresh = (vpsg1_[x << 2] >> 4) & 7;
			int32 freq = ((vpsg1_[(x << 2) | 0x1] | ((vpsg1_[(x << 2) | 0x2] & 15) << 8)) + 1) << 17;
			for (V = start; V < end; V++) {
				if (dcount_[x] > thresh)
					Wave[V >> 4] += amp;
				vcount_[x] -= nesincsize;
				while (vcount_[x] <= 0) {
					vcount_[x] += freq;
					dcount_[x] = (dcount_[x] + 1) & 15;
				}
			}
		}
	}
}

void Vrc6Audio::do_sqv_hq(int x) {
	int32 V;
	int32 amp = ((vpsg1_[x << 2] & 15) << 8) * 6 / 8;

	if (vpsg1_[(x << 2) | 0x2] & 0x80) {
		if (vpsg1_[x << 2] & 0x80) {
			for (V = cvbc_[x]; V < (int)SOUNDTS; V++)
				WaveHi[V] += amp;
		} else {
			int32 thresh = (vpsg1_[x << 2] >> 4) & 7;
			for (V = cvbc_[x]; V < (int)SOUNDTS; V++) {
				if (dcount_[x] > thresh)
					WaveHi[V] += amp;
				vcount_[x]--;
				if (vcount_[x] <= 0) {
					vcount_[x] = (vpsg1_[(x << 2) | 0x1] | ((vpsg1_[(x << 2) | 0x2] & 15) << 8)) + 1;
					dcount_[x] = (dcount_[x] + 1) & 15;
				}
			}
		}
	}
	cvbc_[x] = SOUNDTS;
}

void Vrc6Audio::do_sawv() {
	int V;
	int32 start, end;

	start = cvbc_[2];
	end = (SOUNDTS << 16) / soundtsinc;
	if (end <= start) return;
	cvbc_[2] = end;

	if (vpsg2_[2] & 0x80) {
		uint32 freq3 = (vpsg2_[1] + ((vpsg2_[2] & 15) << 8) + 1);

		for (V = start; V < end; V++) {
			lq_saw1phaseacc_ -= nesincsize;
			if (lq_saw1phaseacc_ <= 0) {
				int32 t;
			 rea:
				t = freq3;
				t <<= 18;
				lq_saw1phaseacc_ += t;
				lq_saw_phaseacc_ += vpsg2_[0] & 0x3f;
				lq_saw_b3_++;
				if (lq_saw_b3_ == 7) {
					lq_saw_b3_ = 0;
					lq_saw_phaseacc_ = 0;
				}
				if (lq_saw1phaseacc_ <= 0)
					goto rea;
				lq_saw_duff_ = (((lq_saw_phaseacc_ >> 3) & 0x1f) << 4) * 6 / 8;
			}
			Wave[V >> 4] += lq_saw_duff_;
		}
	}
}

void Vrc6Audio::do_sawv_hq() {
	int32 V;

	if (vpsg2_[2] & 0x80) {
		for (V = cvbc_[2]; V < (int)SOUNDTS; V++) {
			WaveHi[V] += (((hq_saw_phaseacc_ >> 3) & 0x1f) << 8) * 6 / 8;
			vcount_[2]--;
			if (vcount_[2] <= 0) {
				vcount_[2] = (vpsg2_[1] + ((vpsg2_[2] & 15) << 8) + 1) << 1;
				hq_saw_phaseacc_ += vpsg2_[0] & 0x3f;
				hq_saw_b3_++;
				if (hq_saw_b3_ == 7) {
					hq_saw_b3_ = 0;
					hq_saw_phaseacc_ = 0;
				}
			}
		}
	}
	cvbc_[2] = SOUNDTS;
}

void Vrc6Audio::fill(int32_t count) {
	do_sqv(0);
	do_sqv(1);
	do_sawv();
	for (int x = 0; x < 3; x++)
		cvbc_[x] = count;
}

void Vrc6Audio::hi_fill() {
	do_sqv_hq(0);
	do_sqv_hq(1);
	do_sawv_hq();
}

void Vrc6Audio::hi_sync(int32_t ts) {
	for (int x = 0; x < 3; x++)
		cvbc_[x] = ts;
}

void Vrc6Audio::region_changed() {
	// Preserve register state across region/rate changes, just like the
	// original VRC6_ESI did; only reset the counters and rebind sfun.
	memset(cvbc_, 0, sizeof(cvbc_));
	memset(vcount_, 0, sizeof(vcount_));
	memset(dcount_, 0, sizeof(dcount_));

	if (FSettings.SndRate) {
		if (FSettings.soundq >= 1) {
			sfun_[0] = DoSQV1HQ;
			sfun_[1] = DoSQV2HQ;
			sfun_[2] = DoSawVHQ;
		} else {
			sfun_[0] = DoSQV1;
			sfun_[1] = DoSQV2;
			sfun_[2] = DoSawV;
		}
	} else {
		memset(sfun_, 0, sizeof(sfun_));
	}

	if (!state_registered_) {
		AddExState(state_regs_, ~0, 0, 0);
		state_registered_ = true;
	}
}

void Vrc6Audio::kill() {
	full_reset();
	GameExpSound.expansion = nullptr;
}

static void DoSQV1(void) { g_vrc6_audio.do_sqv(0); }
static void DoSQV2(void) { g_vrc6_audio.do_sqv(1); }
static void DoSawV(void) { g_vrc6_audio.do_sawv(); }
static void DoSQV1HQ(void) { g_vrc6_audio.do_sqv_hq(0); }
static void DoSQV2HQ(void) { g_vrc6_audio.do_sqv_hq(1); }
static void DoSawVHQ(void) { g_vrc6_audio.do_sawv_hq(); }

static DECLFW(VRC6SW) {
	A &= 0xF003;
	if (A >= 0x9000 && A <= 0x9002) {
		g_vrc6_audio.vpsg1_[A & 3] = V;
		g_vrc6_audio.run_sfun(0);
	} else if (A >= 0xA000 && A <= 0xA002) {
		g_vrc6_audio.vpsg1_[4 | (A & 3)] = V;
		g_vrc6_audio.run_sfun(1);
	} else if (A >= 0xB000 && A <= 0xB002) {
		g_vrc6_audio.vpsg2_[A & 3] = V;
		g_vrc6_audio.run_sfun(2);
	}
}

static DECLFW(VRC6Write) {
	if (is26)
		A = (A & 0xFFFC) | ((A >> 1) & 1) | ((A << 1) & 2);
	if (A >= 0x9000 && A <= 0xB002) {
		VRC6SW(A, V);
		return;
	}
	switch (A & 0xF003) {
	case 0x8000: prg[0] = V; Sync(); break;
	case 0xB003: mirr = (V >> 2) & 3; Sync(); break;
	case 0xC000: prg[1] = V; Sync(); break;
	case 0xD000: chr[0] = V; Sync(); break;
	case 0xD001: chr[1] = V; Sync(); break;
	case 0xD002: chr[2] = V; Sync(); break;
	case 0xD003: chr[3] = V; Sync(); break;
	case 0xE000: chr[4] = V; Sync(); break;
	case 0xE001: chr[5] = V; Sync(); break;
	case 0xE002: chr[6] = V; Sync(); break;
	case 0xE003: chr[7] = V; Sync(); break;
	case 0xF000: IRQLatch = V; X6502_IRQEnd(FCEU_IQEXT); break;
	case 0xF001:
		IRQMode = V & 4;
		IRQa = V & 2;
		IRQd = V & 1;
		if (V & 2)
			IRQCount = IRQLatch;
		CycleCount = 0;
		X6502_IRQEnd(FCEU_IQEXT);
		break;
	case 0xF002:
		IRQa = IRQd;
		X6502_IRQEnd(FCEU_IQEXT);
	}
}

static void VRC6_ESI(void) {
	memset(&GameExpSound, 0, sizeof(GameExpSound));
	GameExpSound.expansion = &g_vrc6_audio;
	g_vrc6_audio.region_changed();
}

void Mapper24_Init(CartInfo *info) {
	is26 = 0;
	info->Power = VRC6Power;
	g_cpu.map_irq_hook_ref() = VRC6IRQHook;
	VRC6_ESI();
	GameStateRestore = StateRestore;
	AddExState(&StateRegs, ~0, 0, 0);
}

void Mapper26_Init(CartInfo *info) {
	is26 = 1;
	info->Power = VRC6Power;
	info->Close = VRC6Close;
	g_cpu.map_irq_hook_ref() = VRC6IRQHook;
	VRC6_ESI();
	GameStateRestore = StateRestore;

	WRAMSIZE = 8192;
	WRAM_owner = FCEU_gmalloc_unique(WRAMSIZE);  // v0.3.6: RAII-wrapped
	WRAM = WRAM_owner.get();
	SetupCartPRGMapping(0x10, WRAM, WRAMSIZE, 1);
	AddExState(WRAM, WRAMSIZE, 0, "WRAM");
	if (info->battery) {
		info->addSaveGameBuf( WRAM, WRAMSIZE );
	}

	AddExState(&StateRegs, ~0, 0, 0);
}

void NSFVRC6_Init(void) {
	VRC6_ESI();
	SetWriteHandler(0x8000, 0xbfff, VRC6SW);
}

// ---------------------------------------------------------------------------
// v1.7 Phase E: Vrc6Cart subclass (Strategy A).
//
// Definitions live in vrc6.cpp because they need access to the static
// `g_vrc6_audio` instance, `is26` flag, and the Mapper24_Init / Mapper26_Init
// bodies. The cart subclass only owns the *call sequence* (Power ->
// cart_obj->on_power -> Mapper24_Init / Mapper26_Init -> info->Power =
// VRC6Power + IRQ hook + VRC6_ESI), not the cart wiring logic itself.
// ---------------------------------------------------------------------------

#include "boards/vrc6_cart.h"

namespace fceu11 {

void Vrc6Cart::on_power() noexcept {
	// Mapper24_Init / Mapper26_Init was already called once during
	// iNES_Init (step 1). Calling it again would duplicate the SFORMAT
	// entries (StateRegs, and for mapper 26 also WRAM). Instead, fire
	// the VRC6Power function pointer that the Init set up. At this point
	// info->Power has been redirected by the v1.7 factory block to
	// CartInfo_PowerForward, so we temporarily swap in the legacy
	// VRC6Power pointer, invoke it, then restore the forwarding function
	// pointer so subsequent PowerNES calls also route through
	// cart_obj->on_power.
	if (!currCartInfo) return;
	void (*saved)(void) = currCartInfo->Power;
	currCartInfo->Power = VRC6Power;
	currCartInfo->Power();
	currCartInfo->Power = saved;
}

void Vrc6Cart::on_close() noexcept {
	// Mapper24 / Mapper26_Init set g_cpu.map_irq_hook_ref() = VRC6IRQHook.
	// On game close, drop the hook so a subsequent load of a non-VRC6 ROM
	// does not receive stale IRQ callbacks.
	g_cpu.map_irq_hook_ref() = nullptr;
}

void Vrc6Cart::install_expansion_audio(fceu11::Apu& apu) noexcept {
	// v1.6 §11.1 contract: cart subclass installs the EXPSOUND adapter into
	// the global APU. We reuse the same logic as VRC6_ESI() but route through
	// the APU's set_exp_sound() entry point instead of touching the global
	// GameExpSound directly. The Fill / NeoFill / HiFill / HiSync / RChange
	// function pointers are still populated by VRC6_ESI; only the
	// ExpansionAudio backend pointer is duplicated here so the APU's view is
	// in sync with GameExpSound's view.
	EXPSOUND es = apu.exp_sound();
	es.expansion = &g_vrc6_audio;
	apu.set_exp_sound(es);
	g_vrc6_audio.region_changed();
}

} // namespace fceu11
