// FCEUX11 — VS UniSystem bridge (v1.10 Cryptex Task 5, ≤120 lines).
#include "types.h"
#include "utils/safe_string.h"
#include "x6502.h"
#include "fceu.h"
#include "input.h"
#include "vsuni.h"
#include "state.h"
#include "driver.h"
#include "cart.h"
#include "ines.h"
#include "rust/fceux11_rust.h"
#include <cstring>
#include <cstdio>
static int DIPS_howlong = 0;
uint8 vsdip = 0, coinon = 0, coinon2 = 0, service = 0;
static uint8 secdata_tko[32] = { 0xff,0xbf,0xb7,0x97,0x97,0x17,0x57,0x4f,0x6f,0x6b,0xeb,0xa9,0xb1,0x90,0x94,0x14,0x56,0x4e,0x6f,0x6b,0xeb,0xa9,0xb1,0x90,0xd4,0x5c,0x3e,0x26,0x87,0x83,0x13,0x00 };
static uint8 secdata_rbi[32] = { 0x00,0x00,0x00,0x00,0xb4,0x00,0x00,0x00,0x00,0x6F,0x00,0x00,0x00,0x00,0x94,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00 };
static uint8 *secptr, VSindex;
static DECLFR(VSSecRead) {
	if (A == 0x5e00) { VSindex = 0; return g_cpu.native_layout().DB; }
	if (A == 0x5e01) return secptr[(VSindex++) & 0x1F];
	return 0x00;
}
static readfunc OldReadPPU;
static writefunc OldWritePPU[2];
static DECLFR(A2002_Gumshoe) { return (OldReadPPU(A) & ~0x3F) | 0x1C; }
static DECLFR(A2002_Topgun)  { return (OldReadPPU(A) & ~0x3F) | 0x1B; }
static DECLFR(A2002_MBJ)     { return (OldReadPPU(A) & ~0x3F) | 0x3D; }
static DECLFW(B2000_2001_2C05) { OldWritePPU[(A & 1) ^ 1](A ^ 1, V); }
static uint8 xevselect = 0;
static DECLFR(XevRead) {
	if (A == 0x54FF) return 0x5;
	if (A == 0x5678) return xevselect ? 0 : 1;
	if (A == 0x578F) return xevselect ? 0xd1 : 0x89;
	if (A == 0x5567) { xevselect ^= 1; return xevselect ? 0x37 : 0x3E; }
	return g_cpu.native_layout().DB;
}
void FCEU_VSUniSwap(uint8 *j0, uint8 *j1) {
	if (GameInfo->vs_cswitch) {
		uint16 t = *j0;
		*j0 = (*j0 & 0xC) | (*j1 & 0xF3);
		*j1 = (*j1 & 0xC) | (t & 0xF3);
	}
}
void FCEU_VSUniPower(void) {
	coinon = coinon2 = service = 0; VSindex = 0;
	if (secptr) SetReadHandler(0x5e00, 0x5e01, VSSecRead);
	switch (GameInfo->vs_ppu) {
	case GIPPU_RP2C04_0001: case GIPPU_RP2C04_0002: case GIPPU_RP2C04_0003: case GIPPU_RP2C04_0004:
		default_palette_selection = GameInfo->vs_ppu; break; default: break;
	}
	if (GameInfo->vs_ppu == GIPPU_RC2C05_04)
		{ OldReadPPU = GetReadHandler(0x2002); SetReadHandler(0x2002, 0x2002, A2002_Topgun); }
	else if (GameInfo->vs_ppu == GIPPU_RC2C05_03)
		{ OldReadPPU = GetReadHandler(0x2002); SetReadHandler(0x2002, 0x2002, A2002_Gumshoe); }
	else if (GameInfo->vs_ppu == GIPPU_RC2C05_02)
		{ OldReadPPU = GetReadHandler(0x2002); SetReadHandler(0x2002, 0x2002, A2002_MBJ); }
	if (GameInfo->vs_ppu >= GIPPU_RC2C05_01 && GameInfo->vs_ppu <= GIPPU_RC2C05_04) {
		OldWritePPU[0] = GetWriteHandler(0x2000); OldWritePPU[1] = GetWriteHandler(0x2001);
		SetWriteHandler(0x2000, 0x2001, B2000_2001_2C05);
	}
	if (GameInfo->vs_type == EGIVS_XEVIOUS) SetReadHandler(0x5400, 0x57FF, XevRead);
}
void FCEU_VSUniCheck(uint64 md5partial, int *MapperNo, uint8 *Mirroring) {
	FceuVsUniCheckResult r;
	if (!fceux11_rust_vsuni_check(md5partial, &r)) return;
	int32 f = 0;
	if (*MapperNo != r.mapper)   { f |= 1;  *MapperNo = r.mapper; }
	if (*Mirroring != r.mirroring){ f |= 2;  *Mirroring = r.mirroring; }
	if (GameInfo->type != GIT_VSUNI)  { f |= 4;  GameInfo->type = GIT_VSUNI; }
	if (GameInfo->vs_type != r.game_type) { f |= 8; GameInfo->vs_type = (EGIVS)r.game_type; }
	if (r.ppu && GameInfo->vs_ppu != r.ppu) { f |= 16; GameInfo->vs_ppu = (EGIPPU)r.ppu; }
	secptr = 0;
	if (GameInfo->vs_type == EGIVS_RBI) secptr = secdata_rbi;
	else if (GameInfo->vs_type == EGIVS_TKO) secptr = secdata_tko;
	vsdip = r.dip_value;
	if (r.use_gun && !head.expansion) { f |= 32;
		GameInfo->input[0] = static_cast<ESI>(SI_ZAPPER); GameInfo->input[1] = static_cast<ESI>(SI_NONE);
		GameInfo->inputfc = static_cast<ESIFC>(SIFC_NONE);
	} else if (!head.expansion) {
		GameInfo->input[0] = GameInfo->input[1] = static_cast<ESI>(SI_GAMEPAD);
		GameInfo->inputfc = static_cast<ESIFC>(SIFC_NONE);
	}
	if (r.swap_ab && !GameInfo->vs_cswitch) { f |= 64; GameInfo->vs_cswitch = 1; }
	if (f) {
		static const char* mt[4]={"Normal","RBI Baseball","TKO Boxing","Super Xevious"};
		static const char* mp[10]={"Default","RP2C04-0001","RP2C04-0002","RP2C04-0003","RP2C04-0004","RC2C03B","RC2C05-01","RC2C05-02","RC2C05-03","RC2C05-04"};
		static const char* mm[3]={"Horizontal","Vertical","Four-screen"};
		FCEU_printf("iNES header corrected:\n");
		if (f&1)  FCEU_printf("  Mapper→%d\n",*MapperNo);
		if (f&2)  FCEU_printf("  Mirroring→%s\n",mm[r.mirroring&3]);
		if (f&4)  FCEU_printf("  Type→Vs. System\n");
		if (f&8)  FCEU_printf("  VS type→%s\n",mt[r.game_type]);
		if (f&16) FCEU_printf("  VS PPU→%s\n",mp[r.ppu]);
		if (f&32) FCEU_printf("  Controller→Zapper\n");
		if (f&64) FCEU_printf("  Controllers swapped\n");
	}
}
void FCEU_VSUniDraw(uint8 *XBuf) { DIPS_howlong = fceux11_rust_vsuni_draw(XBuf, vsdip, DIPS_howlong); }
void FCEU_VSUniToggleDIP(int w) {
	if (GameInfo->type != GIT_VSUNI) { FCEU_DispMessage("Not Vs. System.", 0); return; }
	vsdip = fceux11_rust_vsuni_toggle_dip(GameInfo->type, vsdip, w); DIPS_howlong = 180;
	FCEU_DispMessage("DIP %d=%s", 0, w, vsdip & (1 << w) ? "on" : "off");
}
void fceu11::VSUniSetDIP(int w, int s) { if (((vsdip>>w)&1) != s) fceu11::VSUniToggleDIP(w); }
uint8 fceu11::VSUniGetDIPs() { return vsdip; }
void FCEU_VSUniCoin(uint8 s) {
	if (GameInfo->type != GIT_VSUNI) { FCEU_DispMessage("Not Vs. System.", 0); return; }
	uint8 v = fceux11_rust_vsuni_coin(GameInfo->type, s);
	if (s == 0) coinon = v; else coinon2 = v;
}
void FCEU_VSUniService() {
	if (GameInfo->type != GIT_VSUNI) FCEU_DispMessage("Not Vs. System.", 0);
	else service = fceux11_rust_vsuni_service(GameInfo->type);
}
SFORMAT FCEUVSUNI_STATEINFO[] = {
	{ &vsdip, 1, "vsdp" }, { &coinon, 1, "vscn" }, { &coinon2, 1, "vsc2" },
	{ &service, 1, "vssv" }, { &VSindex, 1, "vsin" }, { nullptr, 0, nullptr } };
