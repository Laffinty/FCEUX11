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

#ifndef _X6502H

#include "x6502struct.h"
#include "fceu11_core_types.h"
#include "cpu.h"

// v1.3 Legion Phase 1: legacy CPU-state globals are now inline reference
// aliases into the single fceu11::Cpu instance. Existing source files that
// read/write X, timestamp, soundtimestamp, scanline or MapIRQHook continue
// to compile and link without changes.
inline auto& X = fceu11::cpu_instance().native_layout();


//the opsize table is used to quickly grab the instruction sizes (in bytes)
extern const uint8 opsize[256];

//the optype table is a quick way to grab the addressing mode for any 6502 opcode
extern const uint8 optype[256];

// the opwrite table aids in predicting the value written for any 6502 opcode
extern const uint8 opwrite[256];

//-----------
//mbg 6/30/06 - some of this was removed to mimic XD
//#ifdef FCEUDEF_DEBUGGER
void X6502_Debug(void (*CPUHook)(X6502 *),
    uint8 (*ReadHook)(X6502 *, unsigned int),
    void (*WriteHook)(X6502 *, unsigned int, uint8));

//extern void (*X6502_Run)(int32 cycles);
//#else
//void X6502_Run(int32 cycles);
//#endif
// v1.3 Legion Phase 3: X6502_RunDebug now operates on the Cpu object
// directly. The macro keeps the old call sites working.
void X6502_RunDebug(fceu11::Cpu& cpu, int32 cycles);
#define X6502_Run(cycles) X6502_RunDebug(g_cpu, cycles)
//------------

inline auto& timestamp = fceu11::cpu_instance().timestamp_ref();
inline auto& soundtimestamp = fceu11::cpu_instance().sound_timestamp_ref();
inline auto& scanline = fceu11::cpu_instance().scanline_ref();

// v1.13 Purify H: #define → constexpr (6502 P-register flag masks)
inline constexpr uint8_t N_FLAG = 0x80;
inline constexpr uint8_t V_FLAG = 0x40;
inline constexpr uint8_t U_FLAG = 0x20;
inline constexpr uint8_t B_FLAG = 0x10;
inline constexpr uint8_t D_FLAG = 0x08;
inline constexpr uint8_t I_FLAG = 0x04;
inline constexpr uint8_t Z_FLAG = 0x02;
inline constexpr uint8_t C_FLAG = 0x01;

// v0.3.8/v1.3: declared via fceu11::MapIRQHook typedef for compile-time type
// identity. The symbol stays at global namespace as an inline alias so the
// 35 mapper .cpp files in src/boards/ that assign to it keep linking.
inline auto& MapIRQHook = fceu11::cpu_instance().map_irq_hook_ref();

#define NTSC_CPU (dendy ? 1773447.467 : 1789772.7272727272727272)  // v1.13: depends on runtime ::dendy; defer to v1.14
inline constexpr double PAL_CPU = 1662607.125;  // v1.13 Purify H: #define → constexpr

// v1.13 Purify H: #define → constexpr (IRQ source bitmasks)
inline constexpr uint32_t FCEU_IQEXT    = 0x001;
inline constexpr uint32_t FCEU_IQEXT2   = 0x002;
inline constexpr uint32_t FCEU_IQRESET  = 0x020;
inline constexpr uint32_t FCEU_IQNMI2   = 0x040;  // Delayed NMI, gets converted to FCEU_IQNMI
inline constexpr uint32_t FCEU_IQNMI    = 0x080;
inline constexpr uint32_t FCEU_IQDPCM   = 0x100;
inline constexpr uint32_t FCEU_IQFCOUNT = 0x200;
inline constexpr uint32_t FCEU_IQTEMP   = 0x800;

void X6502_Init(void);
void X6502_Reset(void);
void X6502_Power(void);

void TriggerNMI(void);
void TriggerNMI2(void);

uint8 X6502_DMR(uint32 A);
void X6502_DMW(uint32 A, uint8 V);

void X6502_IRQBegin(int w);
void X6502_IRQEnd(int w);

int X6502_GetOpcodeCycles( int op );

#define _X6502H
#endif
