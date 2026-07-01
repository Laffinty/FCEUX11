/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2003 Xodnizel
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 */

/* [v0.2.17] Rust-backed wrapper.
 * VSUniGames database, DIP helpers, and draw routine forwarded to Rust.
 * Callback registrations, global state, and FCEUVSUNI_STATEINFO remain in C++.
 */

#include "types.h"
#include "utils/safe_string.h"
#include "x6502.h"
#include "fceu.h"
#include "input.h"
#include "netplay.h"
#include "vsuni.h"
#include "state.h"
#include "driver.h"
#include "cart.h"
#include "ines.h"
#include "rust/fceux11_rust.h"

#include <cstring>
#include <cstdio>

static int DIPS_howlong = 0;
uint8 vsdip = 0;

static uint8 secdata_tko[32] =
{
    0xff, 0xbf, 0xb7, 0x97, 0x97, 0x17, 0x57, 0x4f,
    0x6f, 0x6b, 0xeb, 0xa9, 0xb1, 0x90, 0x94, 0x14,
    0x56, 0x4e, 0x6f, 0x6b, 0xeb, 0xa9, 0xb1, 0x90,
    0xd4, 0x5c, 0x3e, 0x26, 0x87, 0x83, 0x13, 0x00
};
static uint8 secdata_rbi[32] =
{
    0x00, 0x00, 0x00, 0x00, 0xb4, 0x00, 0x00, 0x00,
    0x00, 0x6F, 0x00, 0x00, 0x00, 0x00, 0x94, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
};

static uint8 *secptr;
static uint8 VSindex;
uint8 coinon = 0;
uint8 coinon2 = 0;
uint8 service = 0;

static DECLFR(VSSecRead) {
    switch (A) {
    case 0x5e00: VSindex = 0; return g_cpu.native_layout().DB;
    case 0x5e01: return(secptr[(VSindex++) & 0x1F]);
    }
    return(0x00);
}

static readfunc OldReadPPU;
static writefunc OldWritePPU[2];

static DECLFR(A2002_Gumshoe) {
    return((OldReadPPU(A) & ~0x3F) | 0x1C);
}

static DECLFR(A2002_Topgun) {
    return((OldReadPPU(A) & ~0x3F) | 0x1B);
}

static DECLFR(A2002_MBJ) { // Mighty Bomb Jack
    return((OldReadPPU(A) & ~0x3F) | 0x3D);
}
static DECLFW(B2000_2001_2C05) {
    OldWritePPU[(A & 1) ^ 1](A ^ 1, V);
}

static uint8 xevselect = 0;
static DECLFR(XevRead) {
    if (A == 0x54FF) {
        return(0x5);
    } else if (A == 0x5678) {
        return(xevselect ? 0 : 1);
    } else if (A == 0x578F) {
        return(xevselect ? 0xd1 : 0x89);
    } else if (A == 0x5567) {
        xevselect ^= 1;
        return(xevselect ? 0x37 : 0x3E);
    }
    return(g_cpu.native_layout().DB);
}

void FCEU_VSUniSwap(uint8 *j0, uint8 *j1) {
    if (GameInfo->vs_cswitch) {
        uint16 t = *j0;
        *j0 = (*j0 & 0xC) | (*j1 & 0xF3);
        *j1 = (*j1 & 0xC) | (t & 0xF3);
    }
}

void FCEU_VSUniPower(void) {
    coinon = coinon2 = service = 0;
    VSindex = 0;

    if (secptr)
        SetReadHandler(0x5e00, 0x5e01, VSSecRead);

    switch (GameInfo->vs_ppu) {
    case GIPPU_RP2C04_0001:
    case GIPPU_RP2C04_0002:
    case GIPPU_RP2C04_0003:
    case GIPPU_RP2C04_0004:
        default_palette_selection = GameInfo->vs_ppu;
        break;
    default:
        break;
    }
    if (GameInfo->vs_ppu == GIPPU_RC2C05_04) {
        OldReadPPU = GetReadHandler(0x2002);
        SetReadHandler(0x2002, 0x2002, A2002_Topgun);
    } else if (GameInfo->vs_ppu == GIPPU_RC2C05_03) {
        OldReadPPU = GetReadHandler(0x2002);
        SetReadHandler(0x2002, 0x2002, A2002_Gumshoe);
    } else if (GameInfo->vs_ppu == GIPPU_RC2C05_02) {
        OldReadPPU = GetReadHandler(0x2002);
        SetReadHandler(0x2002, 0x2002, A2002_MBJ);
    }
    if (GameInfo->vs_ppu == GIPPU_RC2C05_01 || GameInfo->vs_ppu == GIPPU_RC2C05_02 || GameInfo->vs_ppu == GIPPU_RC2C05_03 || GameInfo->vs_ppu == GIPPU_RC2C05_04) {
        OldWritePPU[0] = GetWriteHandler(0x2000);
        OldWritePPU[1] = GetWriteHandler(0x2001);
        SetWriteHandler(0x2000, 0x2001, B2000_2001_2C05);
    }
    if (GameInfo->vs_type == EGIVS_XEVIOUS) {
        SetReadHandler(0x5400, 0x57FF, XevRead);
    }
}

void FCEU_VSUniCheck(uint64 md5partial, int *MapperNo, uint8 *Mirroring) {
    // v1.10 Cryptex: Use Rust FFI for VS UniSystem check
    FceuVsUniCheckResult result;
    if (!fceux11_rust_vsuni_check(md5partial, &result))
        return;

    int32 tofix = 0;
    if (*MapperNo != result.mapper) {
        tofix |= 1;
        *MapperNo = result.mapper;
    }
    if (*Mirroring != result.mirroring) {
        tofix |= 2;
        *Mirroring = result.mirroring;
    }
    if (GameInfo->type != GIT_VSUNI) {
        tofix |= 4;
        GameInfo->type = GIT_VSUNI;
    }
    if (GameInfo->vs_type != result.game_type) {
        tofix |= 8;
        GameInfo->vs_type = (EGIVS)result.game_type;
    }
    if (result.ppu && (GameInfo->vs_ppu != result.ppu)) {
        tofix |= 16;
        GameInfo->vs_ppu = (EGIPPU)result.ppu;
    }

    secptr = 0;
    switch (GameInfo->vs_type)
    {
    case EGIVS_RBI: secptr = secdata_rbi; break;
    case EGIVS_TKO: secptr = secdata_tko; break;
    default: secptr = 0; break;
    }

    vsdip = result.dip_value;

    if (result.use_gun && !head.expansion) {
        tofix |= 32;
        GameInfo->input[0] = static_cast<ESI>(SI_ZAPPER);
        GameInfo->input[1] = static_cast<ESI>(SI_NONE);
        GameInfo->inputfc = static_cast<ESIFC>(SIFC_NONE);
    }
    else if (!head.expansion) {
        GameInfo->input[0] = GameInfo->input[1] = static_cast<ESI>(SI_GAMEPAD);
        GameInfo->inputfc = static_cast<ESIFC>(SIFC_NONE);
    }
    if (result.swap_ab && !GameInfo->vs_cswitch) {
        tofix |= 64;
        GameInfo->vs_cswitch = 1;
    }

    if (tofix) {
        char gigastr[768];
        FCEU_strlcpy(gigastr, sizeof(gigastr), "The iNES header contains incorrect information.  For now, the information will be corrected in RAM.  ");
        if (tofix & 4) FCEU_strlcat(gigastr, sizeof(gigastr), "Game type should be set to Vs. System.  ");
        if (tofix & 1) { char tmp[80]; snprintf(tmp, sizeof(tmp), "The mapper number should be set to %d.  ", *MapperNo); FCEU_strlcat(gigastr, sizeof(gigastr), tmp); }
        if (tofix & 2) { const char* mstr[3] = { "Horizontal", "Vertical", "Four-screen" }; char tmp[80]; snprintf(tmp, sizeof(tmp), "Mirroring should be set to \"%s\".  ", mstr[result.mirroring & 3]); FCEU_strlcat(gigastr, sizeof(gigastr), tmp); }
        if (tofix & 8) { const char* mstr[4] = { "Normal", "RBI Baseball protection", "TKO Boxing protection", "Super Xevious protection"}; char tmp[80]; snprintf(tmp, sizeof(tmp), "Vs. System type should be set to \"%s\".  ", mstr[result.game_type]); FCEU_strlcat(gigastr, sizeof(gigastr), tmp); }
        if (tofix & 16) { const char* mstr[10] = { "Default", "RP2C04-0001", "RP2C04-0002", "RP2C04-0003", "RP2C04-0004", "RC2C03B", "RC2C05-01", "RC2C05-02" , "RC2C05-03" , "RC2C05-04" }; char tmp[80]; snprintf(tmp, sizeof(tmp), "Vs. System PPU should be set to \"%s\".  ", mstr[result.ppu]); FCEU_strlcat(gigastr, sizeof(gigastr), tmp); }
        if (tofix & 32) FCEU_strlcat(gigastr, sizeof(gigastr), "The controller type should be set to zapper.  ");
        if (tofix & 64) FCEU_strlcat(gigastr, sizeof(gigastr), "The controllers should be swapped.  ");
        FCEU_strlcat(gigastr, sizeof(gigastr), "\n");
        FCEU_printf("%s", gigastr);
    }
}

void FCEU_VSUniDraw(uint8 *XBuf) {
    DIPS_howlong = fceux11_rust_vsuni_draw(XBuf, vsdip, DIPS_howlong);
}

void FCEU_VSUniToggleDIP(int w) {
    if (GameInfo->type != GIT_VSUNI) {
        FCEU_DispMessage("Not Vs. System; toggle DIP switch.", 0);
        return;
    }
    vsdip = fceux11_rust_vsuni_toggle_dip(GameInfo->type, vsdip, w);
    DIPS_howlong = 180;
    FCEU_DispMessage("DIP switch %d is %s.", 0, w, vsdip & (1 << w) ? "on" : "off");
}

void fceu11::VSUniSetDIP(int w, int state) {
    if (((vsdip >> w) & 1) != state)
        fceu11::VSUniToggleDIP(w);
}

uint8 fceu11::VSUniGetDIPs() {
    return(vsdip);
}

void FCEU_VSUniCoin(uint8 slot) {
    if (GameInfo->type != GIT_VSUNI)
        FCEU_DispMessage("Not Vs. System; can't insert coin.", 0);
    else {
        uint8 val = fceux11_rust_vsuni_coin(GameInfo->type, slot);
        switch (slot) {
        case 0:
            coinon = val; break;
        case 1:
            coinon2 = val; break;
        }
    }
}

void FCEU_VSUniService() {
    if (GameInfo->type != GIT_VSUNI)
        FCEU_DispMessage("Not Vs. System; can't press service button.", 0);
    else
        service = fceux11_rust_vsuni_service(GameInfo->type);
}

SFORMAT FCEUVSUNI_STATEINFO[] = {
    { &vsdip, 1, "vsdp" },
    { &coinon, 1, "vscn" },
    { &coinon2, 1, "vsc2" },
    { &service, 1, "vssv" },
    { &VSindex, 1, "vsin" },
    { 0 }
};
