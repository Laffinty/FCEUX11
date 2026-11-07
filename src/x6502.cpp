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
#include "debug.h"
#include "sound.h"
#include "bus.h"   // v1.4 Gateway Phase 3 §5.1.3: hot-path g_bus.read/write
#ifdef _S9XLUA_H
#include "fceulua.h"
#endif

#include "x6502abbrev.h"

#include <array>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <type_traits>

// v1.3 Legion Phase 1: X, timestamp, soundtimestamp and MapIRQHook are now
// inline reference aliases into fceu11::cpu_instance() (see src/cpu.h and
// src/x6502.h). Their storage lives in cpu.cpp as members of fceu11::Cpu.

// hotfix3 B-5b: the v0.3.8 static_assert on the deprecated global
// `MapIRQHook` alias is removed along with the alias itself in x6502.h.
// All ~50 mapper writers now use fceu11::cpu_instance().set_map_irq_hook(X)
// directly, so there is no longer a symbol for the static_assert to
// guard against drift.

// v1.3 Legion Phase 3: cycle accounting is now a method on fceu11::Cpu.
// The macro is kept so that the opcode handlers in ops_table.inc do not
// need to be rewritten; the hot path remains inline-able.
#define ADDCYC(x) g_cpu.add_cycles(x)

//normal memory read
static INLINE uint8 RdMem(unsigned int A)
{
 _DB=fceu11::g_bus.read(static_cast<uint16_t>(A));
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, _DB, LUAMEMHOOK_READ);
 #endif
 return(_DB);
}

// S2 (A6+D5, 2026-11-07): opcode-fetch channel. Same shape as RdMem but
// routed through Bus::read_fetch — the single seam where an evidence-
// backed fetch/read semantic divergence would land (bus.h block comment;
// S1 probe evidence: kgmqa-051 IO/expo fetches behave exactly like reads,
// so the channel is a pure alias today). Only the opcode fetch point below
// uses it; operand and data reads stay on RdMem (S1 evidence: the
// exec-space criterion exercises opcode fetches only).
static INLINE uint8 RdFetch(unsigned int A)
{
 _DB=fceu11::g_bus.read_fetch(static_cast<uint16_t>(A));
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, _DB, LUAMEMHOOK_READ);
 #endif
 return(_DB);
}

//normal memory write
static INLINE void WrMem(unsigned int A, uint8 V)
{
	fceu11::g_bus.write(static_cast<uint16_t>(A), V);
	#ifdef _S9XLUA_H
	CallRegisteredLuaMemHook(A, 1, V, LUAMEMHOOK_WRITE);
	#endif
    _DB = V;
}

static INLINE uint8 RdRAM(unsigned int A)
{
  _DB=fceu11::g_bus.read(static_cast<uint16_t>(A));
  #ifdef _S9XLUA_H
  CallRegisteredLuaMemHook(A, 1, _DB, LUAMEMHOOK_READ);
  #endif
  //bbit edited: this was changed so cheat substituion would work
  // return(_DB=RAM[A]);
  return(_DB);
}

static INLINE void WrRAM(unsigned int A, uint8 V)
{
	RAM[A]=V;
	#ifdef _S9XLUA_H
	CallRegisteredLuaMemHook(A, 1, V, LUAMEMHOOK_WRITE);
	#endif
    _DB = V;
}

uint8 X6502_DMR(uint32 A)
{
 ADDCYC(1);
 _DB=fceu11::g_bus.read(static_cast<uint16_t>(A));
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, _DB, LUAMEMHOOK_READ);
 #endif
 return(_DB);
}

void X6502_DMW(uint32 A, uint8 V)
{
 ADDCYC(1);
 fceu11::g_bus.write(static_cast<uint16_t>(A), V);
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, V, LUAMEMHOOK_WRITE);
 #endif
 _DB = V;
}

#define PUSH(V) \
{       \
 uint8 VTMP=V;  \
 WrRAM(0x100+_S,VTMP);  \
 _S--;  \
}

#define POP() RdRAM(0x100+(++_S))

static uint8 ZNTable[256];
/* Some of these operations will only make sense if you know what the flag
   constants are. */

#define X_ZN(zort)      _P&=~(Z_FLAG|N_FLAG);_P|=ZNTable[zort]
#define X_ZNT(zort)  _P|=ZNTable[zort]

#define JR(cond);  \
{    \
 if(cond)  \
 {  \
  uint32 tmp;  \
  int32 disp;  \
  disp=(int8)RdMem(_PC);  \
  _PC++;  \
  ADDCYC(1);  \
  tmp=_PC;  \
  _PC+=disp;  \
  if((tmp^_PC)&0x100)  \
  ADDCYC(1);  \
 }  \
 else _PC++;  \
}


#define LDA     _A=x;X_ZN(_A)
#define LDX     _X=x;X_ZN(_X)
#define LDY  _Y=x;X_ZN(_Y)

/*  All of the freaky arithmetic operations. */
#define AND  _A&=x;X_ZN(_A)
#define BIT  _P&=~(Z_FLAG|V_FLAG|N_FLAG);_P|=ZNTable[x&_A]&Z_FLAG;_P|=x&(V_FLAG|N_FLAG)
#define EOR  _A^=x;X_ZN(_A)
#define ORA  _A|=x;X_ZN(_A)

#define ADC  {  \
        uint32 l=_A+x+(_P&1);  \
        _P&=~(Z_FLAG|C_FLAG|N_FLAG|V_FLAG);  \
        _P|=((((_A^x)&0x80)^0x80) & ((_A^l)&0x80))>>1;  \
        _P|=(l>>8)&C_FLAG;  \
        _A=l;  \
        X_ZNT(_A);  \
       }

#define SBC  {  \
        uint32 l=_A-x-((_P&1)^1);  \
        _P&=~(Z_FLAG|C_FLAG|N_FLAG|V_FLAG);  \
        _P|=((_A^l)&(_A^x)&0x80)>>1;  \
        _P|=((l>>8)&C_FLAG)^C_FLAG;  \
        _A=l;  \
        X_ZNT(_A);  \
       }

#define CMPL(a1,a2) {  \
         uint32 t=a1-a2;  \
         X_ZN(t&0xFF);  \
         _P&=~C_FLAG;  \
         _P|=((t>>8)&C_FLAG)^C_FLAG;  \
		    }

/* Special undocumented operation.  Very similar to CMP. */
#define AXS      {  \
                     uint32 t=(_A&_X)-x;    \
                     X_ZN(t&0xFF);      \
                     _P&=~C_FLAG;       \
         _P|=((t>>8)&C_FLAG)^C_FLAG;  \
         _X=t;  \
        }

#define CMP    CMPL(_A,x)
#define CPX    CMPL(_X,x)
#define CPY          CMPL(_Y,x)

/* The following operations modify the byte being worked on. */
#define DEC         x--;X_ZN(x)
#define INC    x++;X_ZN(x)

#define ASL  _P&=~C_FLAG;_P|=x>>7;x<<=1;X_ZN(x)
#define LSR  _P&=~(C_FLAG|N_FLAG|Z_FLAG);_P|=x&1;x>>=1;X_ZNT(x)

/* For undocumented instructions, maybe for other things later... */
#define LSRA  _P&=~(C_FLAG|N_FLAG|Z_FLAG);_P|=_A&1;_A>>=1;X_ZNT(_A)

#define ROL  {  \
     uint8 l=x>>7;  \
     x<<=1;  \
     x|=_P&C_FLAG;  \
     _P&=~(Z_FLAG|N_FLAG|C_FLAG);  \
     _P|=l;  \
     X_ZNT(x);  \
    }
#define ROR  {  \
     uint8 l=x&1;  \
     x>>=1;  \
     x|=(_P&C_FLAG)<<7;  \
     _P&=~(Z_FLAG|N_FLAG|C_FLAG);  \
     _P|=l;  \
     X_ZNT(x);  \
		}

/* Icky icky thing for some undocumented instructions.  Can easily be
   broken if names of local variables are changed.
*/

/* Absolute */
#define GetAB(target)   \
{  \
 target=RdMem(_PC);  \
 _PC++;  \
 target|=RdMem(_PC)<<8;  \
 _PC++;  \
}

/* Absolute Indexed(for reads) */
#define GetABIRD(target, i)  \
{  \
 unsigned int tmp;  \
 GetAB(tmp);  \
 target=tmp;  \
 target+=i;  \
 if((target^tmp)&0x100)  \
 {  \
  target&=0xFFFF;  \
  RdMem(target^0x100);  \
  ADDCYC(1);  \
 }  \
}

/* Absolute Indexed(for writes and rmws) */
#define GetABIWR(target, i)  \
{  \
 unsigned int rt;  \
 GetAB(rt);  \
 target=rt;  \
 target+=i;  \
 target&=0xFFFF;  \
 RdMem((target&0x00FF)|(rt&0xFF00));  \
}

/* Zero Page */
#define GetZP(target)  \
{  \
 target=RdMem(_PC);   \
 _PC++;  \
}

/* Zero Page Indexed */
#define GetZPI(target,i)  \
{  \
 target=i+RdMem(_PC);  \
 _PC++;  \
}

/* Indexed Indirect */
#define GetIX(target)  \
{  \
 uint8 tmp;  \
 tmp=RdMem(_PC);  \
 _PC++;  \
 tmp+=_X;  \
 target=RdRAM(tmp);  \
 tmp++;    \
 target|=RdRAM(tmp)<<8;  \
}

/* Indirect Indexed(for reads) */
#define GetIYRD(target)  \
{  \
 unsigned int rt;  \
 uint8 tmp;  \
 tmp=RdMem(_PC);  \
 _PC++;  \
 rt=RdRAM(tmp);  \
 tmp++;  \
 rt|=RdRAM(tmp)<<8;  \
 target=rt;  \
 target+=_Y;  \
 if((target^rt)&0x100)  \
 {  \
  target&=0xFFFF;  \
  RdMem(target^0x100);  \
  ADDCYC(1);  \
 }  \
}

/* Indirect Indexed(for writes and rmws) */
#define GetIYWR(target)  \
{  \
 unsigned int rt;  \
 uint8 tmp;  \
 tmp=RdMem(_PC);  \
 _PC++;  \
 rt=RdRAM(tmp);  \
 tmp++;  \
 rt|=RdRAM(tmp)<<8;  \
 target=rt;  \
 target+=_Y;  \
 target&=0xFFFF; \
 RdMem((target&0x00FF)|(rt&0xFF00));  \
}

/* Now come the macros to wrap up all of the above stuff addressing mode functions
   and operation macros.  Note that operation macros will always operate(redundant
   redundant) on the variable "x".
*/

#define RMW_A(op) {uint8 x=_A; op; _A=x;  } /* Meh... */
#define RMW_AB(op) {unsigned int A; uint8 x; GetAB(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ABI(reg,op) {unsigned int A; uint8 x; GetABIWR(A,reg); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ABX(op)  RMW_ABI(_X,op)
#define RMW_ABY(op)  RMW_ABI(_Y,op)
#define RMW_IX(op)  {unsigned int A; uint8 x; GetIX(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_IY(op)  {unsigned int A; uint8 x; GetIYWR(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ZP(op)  {uint8 A; uint8 x; GetZP(A); x=RdRAM(A); op; WrRAM(A,x);  }
#define RMW_ZPX(op) {uint8 A; uint8 x; GetZPI(A,_X); x=RdRAM(A); op; WrRAM(A,x); }

#define LD_IM(op)  {uint8 x; x=RdMem(_PC); _PC++; op; }
#define LD_ZP(op)  {uint8 A; uint8 x; GetZP(A); x=RdRAM(A); op; }
#define LD_ZPX(op)  {uint8 A; uint8 x; GetZPI(A,_X); x=RdRAM(A); op; }
#define LD_ZPY(op)  {uint8 A; uint8 x; GetZPI(A,_Y); x=RdRAM(A); op; }
#define LD_AB(op)  {unsigned int A; FCEU_MAYBE_UNUSED uint8 x; GetAB(A); x=RdMem(A); op;  }
#define LD_ABI(reg,op)  {unsigned int A; FCEU_MAYBE_UNUSED uint8 x; GetABIRD(A,reg); x=RdMem(A); op; }
#define LD_ABX(op)  LD_ABI(_X,op)
#define LD_ABY(op)  LD_ABI(_Y,op)
#define LD_IX(op)  {unsigned int A; uint8 x; GetIX(A); x=RdMem(A); op; }
#define LD_IY(op)  {unsigned int A; uint8 x; GetIYRD(A); x=RdMem(A); op; }

#define ST_ZP(r)  {uint8 A; GetZP(A); WrRAM(A,r); }
#define ST_ZPX(r)  {uint8 A; GetZPI(A,_X); WrRAM(A,r); }
#define ST_ZPY(r)  {uint8 A; GetZPI(A,_Y); WrRAM(A,r); }
#define ST_AB(r)  {unsigned int A; GetAB(A); WrMem(A,r); }
#define ST_ABI(reg,r)  {unsigned int A; GetABIWR(A,reg); WrMem(A,r);  }
#define ST_ABX(r)  ST_ABI(_X,r)
#define ST_ABY(r)  ST_ABI(_Y,r)
#define ST_IX(r)  {unsigned int A; GetIX(A); WrMem(A,r);  }
#define ST_IY(r)  {unsigned int A; GetIYWR(A); WrMem(A,r);  }

#include "ops_table.inc"

FCEUX11_CACHE_ALIGN static uint8 CycTable[256] =
{
/*0x00*/ 7,6,2,8,3,3,5,5,3,2,2,2,4,4,6,6,
/*0x10*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x20*/ 6,6,2,8,3,3,5,5,4,2,2,2,4,4,6,6,
/*0x30*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x40*/ 6,6,2,8,3,3,5,5,3,2,2,2,3,4,6,6,
/*0x50*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x60*/ 6,6,2,8,3,3,5,5,4,2,2,2,5,4,6,6,
/*0x70*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x80*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
/*0x90*/ 2,6,2,6,4,4,4,4,2,5,2,5,5,5,5,5,
/*0xA0*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
/*0xB0*/ 2,5,2,5,4,4,4,4,2,4,2,4,4,4,4,4,
/*0xC0*/ 2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
/*0xD0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0xE0*/ 2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
/*0xF0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
};

int X6502_GetOpcodeCycles( int op )
{
	return CycTable[op];
}

// ----------------------------------------------------------------------------
// D1 取证探针 — 中断线时刻账本（INTLEDGER，kgmqa-037 轮询下沉/hijack 专线
// S1，2026-11-07）。只观测、不修复：D1 的设计裁定是「线时刻账本」（v2.0.2
// 计划 r12——本仓无每访问周期步进，Mesen2 两拍不可照抄），本探针把账本本体
// 落到磁盘，供 S2 起的模型改造前后对账与锁测试标定：
//
//   1) intledger.bin：每条中断线（NMI/NMI2/EXT/EXT2/DPCM/FCOUNT/TEMP/
//      RESET）的 assert/release 记录 + 循环顶边界判定快照。abs 用
//      g_cpu.timestamp_base()+timestamp_ref()（= e1_cpu_abs 同口径，CPU
//      周期）；count 为边界处 _count 残量（1/48 CPU 周期单位，架构精确态，
//      纪律 11：判定/退出条件不绑 PC）。
//   2) 快照只在 `_IRQlow != 0` 的边界记录（线全零的边界是空轮询，每帧
//      数万条、零信息）；decision 字段记录该边界实际走到的判定分支。
//   3) FCEUX11_INTLEDGER=<dir> 指定输出目录；未设或空 = 探针完全不激活
//      （零运行成本，e1_cpu_trace_on 惯例的一次性 static bool 读 env）。
//      六个注入点全部在 x6502.cpp 内（TriggerNMI / TriggerNMI2 /
//      X6502_IRQBegin / X6502_IRQEnd / fceu11::NMI / fceu11::IRQ）+ 循环
//      顶判定处，不新增编译单元。文件打开失败时探针自禁用并报一句。
//
// 记录格式（24 字节定长，intledger.bin 以 8 字节头 "INTL1"+24 起始）：
struct IntLedgerRec {
	uint64_t abs;      // e1_cpu_abs() 同口径绝对 CPU 周期
	int32_t  count;    // 事件/快照时刻 _count 残量
	uint32_t irqlow;   // assert/release: 事件后的线值；快照: 判定前线值
	uint16_t pc;       // 辅助字段（不作为任何判据）
	uint8_t  kind;     // 0..7 = 行号（见下表），0xF = 边界快照
	uint8_t  edge;     // 0 = release, 1 = assert, 2 = boundary snapshot
	uint8_t  decision; // 快照专用：0=无, 1=reset, 2=nmi2->nmi, 3=nmi-defer,
	                   // 4=nmi-dispatch, 5=irq-dispatch, 6=held(I旗标/jammed)
	uint8_t  pad;
};
static_assert(sizeof(IntLedgerRec) == 24, "keep the on-disk ledger record layout fixed");

enum IntLedgerKind : uint8_t {
	INTL_NMI    = 0,  // FCEU_IQNMI   (TriggerNMI / fceu11::NMI / X6502_IRQBegin)
	INTL_NMI2   = 1,  // FCEU_IQNMI2  (TriggerNMI2)
	INTL_EXT    = 2,  // FCEU_IQEXT   (mapper / FDS)
	INTL_EXT2   = 3,  // FCEU_IQEXT2
	INTL_DPCM   = 4,  // FCEU_IQDPCM  (APU DMC)
	INTL_FCOUNT = 5,  // FCEU_IQFCOUNT (APU frame counter)
	INTL_TEMP   = 6,  // FCEU_IQTEMP  (fceu11::IRQ one-shot)
	INTL_RESET  = 7,  // FCEU_IQRESET
	INTL_BOUND  = 15, // loop-top boundary snapshot
};

static bool intledger_on() {
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_INTLEDGER");
		return e && e[0] != '\0';
	}();
	return on;
}

static const char* intledger_dir() {
	static const char* dir = []() {
		const char* e = std::getenv("FCEUX11_INTLEDGER");
		return (e && e[0] != '\0') ? e : (const char*)nullptr;
	}();
	return dir;
}

// 边界快照可能在「线被 I 旗标/jammed 长期挂起」时每指令重复一条，给
// 64 MiB 上限防失控（与 OAMBUS 同思路、更紧——账本事件本身是低频的）。
static constexpr uint64_t INTLEDGER_CAP_BYTES = 64ull * 1024 * 1024;

static FILE*   s_intl_fp = nullptr;
static bool    s_intl_files_ready = false;
static bool    s_intl_active = false;
static bool    s_intl_capped = false;
static uint64_t s_intl_bytes = 0;
static uint32_t s_intl_asserts[8] = {0};
static uint32_t s_intl_releases[8] = {0};
static uint32_t s_intl_snapshots[7] = {0};   // decision 0..6
static uint64_t s_intl_held_suppressed = 0;  // 折叠掉的连续 held 快照数
static uint64_t s_intl_first_abs = 0;
static uint64_t s_intl_last_abs = 0;

static void intledger_atexit();

static void intledger_init_files() {
	if (s_intl_files_ready) return;
	s_intl_files_ready = true;
	const char* dir = intledger_dir();
	if (!dir) return;
	std::error_code ec;
	std::filesystem::create_directories(dir, ec);
	char path[1024];
	std::snprintf(path, sizeof(path), "%s/intledger.bin", dir);
	s_intl_fp = std::fopen(path, "wb");
	if (!s_intl_fp) {
		std::fprintf(stderr,
			"INTLEDGER ERROR: cannot open intledger.bin under '%s' — probe disabled\n",
			dir);
		return;
	}
	const uint8_t hdr[8] = { 'I', 'N', 'T', 'L', '1', 24, 0, 0 };
	s_intl_bytes += std::fwrite(hdr, 1, sizeof(hdr), s_intl_fp);
	s_intl_active = true;
	std::atexit(intledger_atexit);
}

static void intledger_atexit() {
	if (!s_intl_fp) return;
	static const char* kind_names[8] = {
		"NMI", "NMI2", "EXT", "EXT2", "DPCM", "FCOUNT", "TEMP", "RESET" };
	static const char* dec_names[7] = {
		"none", "reset", "nmi2_conv", "nmi_defer", "nmi_dispatch",
		"irq_dispatch", "held" };
	char path[1024];
	std::snprintf(path, sizeof(path), "%s/summary.log", intledger_dir());
	FILE* s = std::fopen(path, "w");
	if (s) {
		std::fprintf(s, "INTLEDGER summary cap=%llu bytes bytes=%llu capped=%d\n",
			(unsigned long long)INTLEDGER_CAP_BYTES,
			(unsigned long long)s_intl_bytes, s_intl_capped ? 1 : 0);
		std::fprintf(s, "abs_range %llu .. %llu\n",
			(unsigned long long)s_intl_first_abs,
			(unsigned long long)s_intl_last_abs);
		for (int k = 0; k < 8; ++k)
			std::fprintf(s, "line %-7s assert=%u release=%u\n",
				kind_names[k], s_intl_asserts[k], s_intl_releases[k]);
		for (int d = 0; d < 7; ++d)
			std::fprintf(s, "boundary %-12s %u\n", dec_names[d], s_intl_snapshots[d]);
		std::fprintf(s, "held_suppressed %llu\n",
			(unsigned long long)s_intl_held_suppressed);
		std::fclose(s);
	}
	std::fflush(s_intl_fp);
	std::fclose(s_intl_fp);
	s_intl_fp = nullptr;
}

static inline uint64_t intledger_now_abs() {
	return g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
}

static void intledger_write(const IntLedgerRec& r) {
	if (!s_intl_active) return;
	if (!s_intl_fp || s_intl_capped) return;
	if (s_intl_bytes >= INTLEDGER_CAP_BYTES) {
		s_intl_capped = true;
		std::fprintf(stderr,
			"INTLEDGER: intledger.bin hit %llu-byte cap — further records dropped (summary.log continues)\n",
			(unsigned long long)INTLEDGER_CAP_BYTES);
		return;
	}
	s_intl_bytes += std::fwrite(&r, 1, sizeof(r), s_intl_fp);
}

static inline void intledger_emit_line(uint8_t kind, uint8_t edge) {
	if (!intledger_on()) return;
	intledger_init_files();
	if (!s_intl_active) return;
	IntLedgerRec r;
	r.abs = intledger_now_abs();
	r.count = _count;
	r.irqlow = _IRQlow;
	r.pc = _PC;
	r.kind = kind;
	r.edge = edge;
	r.decision = 0;
	r.pad = 0;
	if (edge == 1) ++s_intl_asserts[kind & 7];
	else           ++s_intl_releases[kind & 7];
	if (!s_intl_first_abs || r.abs < s_intl_first_abs) s_intl_first_abs = r.abs;
	s_intl_last_abs = r.abs;
	intledger_write(r);
}

// 把一个 IRQ 掩码拆成逐线的 assert/release 记录（X6502_IRQBegin/End 的
// 调用方传单线常量，但接口语义允许组合掩码，逐位展开不丢事件）。
static inline void intledger_decode_mask(uint32_t w, uint8_t edge) {
	if (!intledger_on()) return;
	static const struct { uint32_t bit; uint8_t kind; } map[8] = {
		{FCEU_IQEXT,    INTL_EXT},    {FCEU_IQEXT2,   INTL_EXT2},
		{FCEU_IQRESET,  INTL_RESET},  {FCEU_IQNMI2,   INTL_NMI2},
		{FCEU_IQNMI,    INTL_NMI},    {FCEU_IQDPCM,   INTL_DPCM},
		{FCEU_IQFCOUNT, INTL_FCOUNT}, {FCEU_IQTEMP,   INTL_TEMP},
	};
	for (const auto& m : map)
		if (w & m.bit) intledger_emit_line(m.kind, edge);
}

// 循环顶边界快照：在 `if(_IRQlow)` 块尾调用（所有判定分支之后、TEMP
// 清除之前）。decision 由各判定分支写 pre_decision 局部变量传入。
// 「held」快照（decision=6，线被 I 旗标/jammed 长期挂起）会逐指令重复
// 同一线值——连续相同（同 decision 同 irqlow）的 held 快照折叠为一条，
// 被折叠的重复数记入 summary（held_suppressed）；其余 decision 逐边界
// 全记。折叠只影响探针产物，不影响任何仿真状态。
static inline void intledger_emit_boundary(uint32_t pre_irqlow, uint8_t decision) {
	if (!intledger_on()) return;
	intledger_init_files();
	if (!s_intl_active) return;
	static uint32_t s_intl_prev_held_irqlow = 0;
	static bool     s_intl_prev_was_held = false;
	if (decision == 6 && s_intl_prev_was_held && s_intl_prev_held_irqlow == pre_irqlow) {
		++s_intl_held_suppressed;
		return;
	}
	s_intl_prev_was_held = (decision == 6);
	s_intl_prev_held_irqlow = pre_irqlow;
	IntLedgerRec r;
	r.abs = intledger_now_abs();
	r.count = _count;
	r.irqlow = pre_irqlow;
	r.pc = _PC;
	r.kind = INTL_BOUND;
	r.edge = 2;
	r.decision = decision;
	r.pad = 0;
	++s_intl_snapshots[decision & 7];
	if (!s_intl_first_abs || r.abs < s_intl_first_abs) s_intl_first_abs = r.abs;
	s_intl_last_abs = r.abs;
	intledger_write(r);
}

void X6502_IRQBegin(int w)
{
 _IRQlow|=w;
 intledger_decode_mask((uint32_t)w, 1);
}

void X6502_IRQEnd(int w)
{
 _IRQlow&=~w;
 intledger_decode_mask((uint32_t)w, 0);
}

// E-1 probe (Phase 1 Step 1.3, 2026-08-02): env-gated absolute-cycle trace
// for NMI latch set / CPU sampling. Uses FCEUX11_E1_TRACE like the PPU-side
// probe so one env var drives the whole frame-boundary investigation.
static bool e1_cpu_trace_on() {
 static const bool on = []() {
  const char* e = std::getenv("FCEUX11_E1_TRACE");
  return e && e[0] == '1' && e[1] == '\0';
 }();
 return on;
}

// E-1 probe (Phase 1 Step 1.3): last instruction PC observed by the CPU at
// the current boundary, so the PPU VBL block can log exactly where the CPU
// instruction stream is when the frame boundary fires.
static uint16 e1_last_pc = 0;
uint16 fceu11_e1_last_pc() { return e1_last_pc; }

// D5 取证探针 (FCEUX11_FETCH_PROBE, 2026-11-07): one-shot env parse, the
// e1_cpu_trace_on convention. When on, the CPU sets
// fceu11::g_fetchprobe_fetch_pending right before the opcode-fetch read
// (b1=RdMem(_PC) below); the inlined Bus::read() consumes the marker on
// that read and routes (addr, data) to bus.cpp's fetch_probe_record.
// Off by default: pending stays false for the whole process, so the only
// resident cost is one static-bool load + predicted-not-taken branch per
// instruction. Recorder / trace format / domain summary: bus.cpp.
static bool d5_fetch_probe_on() {
 static const bool on = []() {
  const char* e = std::getenv("FCEUX11_FETCH_PROBE");
  return e && e[0] != '\0';
 }();
 return on;
}

static uint64 e1_cpu_abs() {
 return g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
}

// E-1 probe (Phase 1 Step 1.3, 2026-08-02): NMI-latch freshness flag.
// The VBL path (TriggerNMI) is called from the PPU loop between CPU runs;
// the CPU's next loop-top check happens at the boundary where the previous
// instruction ENDED (before the latch was asserted), so dispatching there
// would fire NMI one instruction too early. Per 6502 semantics the line is
// sampled at the END of each instruction; a latch asserted at/after boundary
// B must be observed at the NEXT boundary. TriggerNMI2 already defers via
// the NMI2->NMI conversion on the same check; give TriggerNMI the same
// one-boundary deferral (04-nmi_control #11 "after NEXT instruction").
static bool g_e1_nmi_fresh = false;

void TriggerNMI(void)
{
 // E-1 Track-B probe (v1.17 R5 task, 2026-08-08): enhanced NMI_LATCH
 // recorder at the CPU-side callee of the NMI dispatch path. The
 // existing E1 NMI_SET prints only the absolute timestamp; this E1B
 // NMI_LATCH_CALLEE adds the CPU budget residual (count, 1/16-dot
 // units), the freshly-latched IRQ-low byte, and the last PC the CPU
 // observed, all before _IRQlow|=FCEU_IQNMI mutates state. Distinct
 // probe name (E1B NMI_LATCH_CALLEE) so the caller-side E1B NMI_LATCH
 // in ppu_rendering.cpp and this CPU-side line can be cross-correlated
 // for the vbl_05/vbl_07 NMI dispatch-latency analysis.
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1B NMI_LATCH_CALLEE abs=%llu (path=VBL) count=%d IRQlow_pre=0x%X lastpc=%04X\n",
   (unsigned long long)e1_cpu_abs(),
   (int)g_cpu.native_layout().count,
   (unsigned)_IRQlow,
   (unsigned)e1_last_pc);
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1 NMI_SET abs=%llu (path=VBL)\n", (unsigned long long)e1_cpu_abs());
 _IRQlow|=FCEU_IQNMI;
 g_e1_nmi_fresh = true;
 intledger_emit_line(INTL_NMI, 1);
}

void TriggerNMI2(void)
{
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1 NMI_SET2 abs=%llu (path=W2000-edge)\n", (unsigned long long)e1_cpu_abs());
 _IRQlow|=FCEU_IQNMI2;
 intledger_emit_line(INTL_NMI2, 1);
}

void X6502_Reset(void)
{
 _IRQlow=FCEU_IQRESET;
}
/**
* Initializes the 6502 CPU
**/
void X6502_Init(void)
{
	unsigned int i;

	// Initialize the CPU structure
	memset((void *)&g_cpu.native_layout(),0,sizeof(g_cpu.native_layout()));

	for(i = 0; i < sizeof(ZNTable); i++)
	{
		if(!i)
		{
			ZNTable[i] = Z_FLAG;
		}
		else if ( i & 0x80 )
		{
			ZNTable[i] = N_FLAG;
		}
		else
		{
			ZNTable[i] = 0;
		}
	}
}

extern int StackAddrBackup;
// Set true by X6502_Power so the next RESET vector fetch (power-on)
// does not apply the soft-reset S-=3 rule. blargg cpu_reset_regs.
static bool g_power_on_reset_pending = true;
void X6502_Power(void)
{
 _count=_tcount=_IRQlow=_PC=_A=_X=_Y=_P=_PI=_DB=_jammed=0;
 // blargg cpu_reset_regs: at power A,X,Y=0, P=$34 (U|B|I), S=$FD.
 // The subsequent RESET vector fetch must NOT apply the soft-reset
 // S-=3 rule — that applies only to a later reset-button press.
 _P=_PI=(U_FLAG|B_FLAG|I_FLAG);
 _S=0xFD;
 g_power_on_reset_pending = true;
 g_cpu.timestamp_ref()=g_cpu.sound_timestamp_ref()=0;
 X6502_Reset();
 StackAddrBackup = -1;
}

void X6502_RunDebug(fceu11::Cpu& cpu, int32 cycles)
{
  // hotfix3 C-3: lift multiply to int64 to avoid signed-overflow UB
  // when `cycles` is near INT32_MAX (Lua/cheat can call X6502_Run with such
  // arguments). The pre-fix `cycles *= 16` was UB at `cycles == 2^27`, and
  // the subsequent `_count += cycles` could then propagate a poisoned value.
  {
   int64 scaled = static_cast<int64>(cycles) * (PAL ? 15 : 16);
   if (scaled > INT32_MAX) scaled = INT32_MAX;
   _count += static_cast<int32>(scaled);
  }

  while(_count>0)
  {
   int32 temp;
   uint8 b1;

   if(_IRQlow) [[unlikely]]
   {
    // D1 INTLEDGER: capture the line value this boundary determination
    // sees (pre-decision), then record the actual decision branch taken.
    // Default = 6 ("held": nothing dispatchable this boundary — I flag
    // set / jammed / conversion only); dispatch branches overwrite 1-5.
    // Emitted at the block tail, after the branch chain and before the
    // per-boundary TEMP clear.
    const uint32 intledger_pre_irqlow = _IRQlow;
    uint8 intledger_decision = 6;
    if(_IRQlow&FCEU_IQRESET)
    {
	 DEBUG( if(debug_loggingCD) LogCDVectors(0xFFFC); )
     _PC=RdMem(0xFFFC);
     _PC|=RdMem(0xFFFD)<<8;
     _jammed=0;
     // blargg cpu_reset_regs:
     //   power-on RESET: A=X=Y=0, P=$34, S=$FD (already set by X6502_Power;
     //   do not apply the soft-reset rule below).
     //   soft reset:     A/X/Y unchanged; P |= I ($04); S -= 3;
     //                   nothing written to stack.
     if (g_power_on_reset_pending) {
      g_power_on_reset_pending = false;
      // Power-on: force the blargg cpu_reset_regs contract even if
      // PowerNES was not the last lifecycle call (LoadGame may only reset).
      _A=_X=_Y=0;
      _P=_PI=(U_FLAG|B_FLAG|I_FLAG);
      _S=0xFD;
     } else {
      _P |= I_FLAG;
      _S = (uint8)(_S - 3);
     }
     _PI = _P;
     _IRQlow&=~FCEU_IQRESET;
     intledger_decision = 1;
    }
    else if(_IRQlow&FCEU_IQNMI2)
     {
     _IRQlow&=~FCEU_IQNMI2;
     _IRQlow|=FCEU_IQNMI;
     intledger_decision = 2;
    }
    else if(_IRQlow&FCEU_IQNMI)
    {
     if (g_e1_nmi_fresh)
     {
      // Latch was asserted at/after this boundary; observe it at the NEXT
      // instruction boundary (6502 samples the line at instruction end).
      if (e1_cpu_trace_on())
       fprintf(stderr, "E1 NMI_DEFER abs=%llu pc=%04X\n",
        (unsigned long long)e1_cpu_abs(), (unsigned)_PC);
      g_e1_nmi_fresh = false;
      intledger_decision = 3;
     }
     else if(!_jammed)
     {
      if (e1_cpu_trace_on())
       fprintf(stderr, "E1 NMI_DISPATCH abs=%llu pc=%04X\n",
        (unsigned long long)e1_cpu_abs(), (unsigned)_PC);
      ADDCYC(7);
      PUSH(_PC>>8);
      PUSH(_PC);
      PUSH((_P&~B_FLAG)|(U_FLAG));
      _P|=I_FLAG;
	  DEBUG( if(debug_loggingCD) LogCDVectors(0xFFFA) );
      _PC=RdMem(0xFFFA);
      _PC|=RdMem(0xFFFB)<<8;
      _IRQlow&=~FCEU_IQNMI;
      intledger_decision = 4;
     }
    }
    else
    {
     if(!(_PI&I_FLAG) && !_jammed)
     {
      ADDCYC(7);
      PUSH(_PC>>8);
      PUSH(_PC);
      PUSH((_P&~B_FLAG)|(U_FLAG));
      _P|=I_FLAG;
	  DEBUG( if(debug_loggingCD) LogCDVectors(0xFFFE) );
      _PC=RdMem(0xFFFE);
      _PC|=RdMem(0xFFFF)<<8;
      intledger_decision = 5;
     }
    }
    intledger_emit_boundary(intledger_pre_irqlow, intledger_decision);
    _IRQlow&=~(FCEU_IQTEMP);
    if(_count<=0)
    {
     _PI=_P;
     return;
     } //Should increase accuracy without a
              //major speed hit.
   }

	//will probably cause a major speed decrease on low-end systems
   DEBUG( DebugCycle() );

   IncrementInstructionsCounters();

   _PI=_P;
   // A1 (v2.0.2 P1): the E-1 probe is the only consumer of this variable, and
   // every reader sits behind the same env gate, so the write pays nothing
   // when the probe is off.
   if (e1_cpu_trace_on()) e1_last_pc = (uint16)_PC;
   // D5 取证探针 (FCEUX11_FETCH_PROBE): mark this read as an opcode fetch.
   // Consumed (and cleared) inside Bus::read — no return semantics touched.
   // S2: the fetch itself goes through the read_fetch channel (RdFetch).
   if (d5_fetch_probe_on()) fceu11::g_fetchprobe_fetch_pending = true;
   b1=RdFetch(_PC);

   ADDCYC(CycTable[b1]);

   temp=_tcount;
   _tcount=0;
   // hotfix3 B-5a + B-5b: single atomic acquire load (cached in a local)
   // instead of the two ref-grabs per instruction that the legacy
   // map_irq_hook_ref() returning-by-reference pattern required.
   if (const auto hook = g_cpu.map_irq_hook()) [[unlikely]] hook(temp);

   if (!g_cpu.overclocking()) [[likely]]
    FCEU_SoundCPUHook(temp);
   #ifdef _S9XLUA_H
   CallRegisteredLuaMemHook(_PC, 1, 0, LUAMEMHOOK_EXEC);
   #endif
   _PC++;
   x6502_dispatch[b1](&cpu.native_layout());
  }
}

//--------------------------
//---Called from debuggers
// v0.3.10 P4.1: definitions live in fceu11:: per plan v3 §5 v0.3.10;
// the global FCEUI_NMI / FCEUI_IRQ symbols are preserved via the inline
// reference aliases declared in core_api.h.
void fceu11::NMI()
{
 _IRQlow|=FCEU_IQNMI;
 intledger_emit_line(INTL_NMI, 1);
}

void fceu11::IRQ()
{
 _IRQlow|=FCEU_IQTEMP;
 intledger_emit_line(INTL_TEMP, 1);
}

void FCEUI_GetIVectors(uint16 *reset, uint16 *irq, uint16 *nmi)
{
 fceuindbg=1;

 *reset=RdMem(0xFFFC);
 *reset|=RdMem(0xFFFD)<<8;
 *nmi=RdMem(0xFFFA);
 *nmi|=RdMem(0xFFFB)<<8;
 *irq=RdMem(0xFFFE);
 *irq|=RdMem(0xFFFF)<<8;
 fceuindbg=0;
}

//the opsize table is used to quickly grab the instruction sizes (in bytes)
FCEUX11_CACHE_ALIGN const uint8 opsize[256] = {
#ifdef BRK_3BYTE_HACK
/*0x00*/	3, //BRK
#else
/*0x00*/	1, //BRK
#endif
/*0x01*/      2,0,0,0,2,2,0,1,2,1,0,0,3,3,0,
/*0x10*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x20*/	3,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0x30*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x40*/	1,2,0,0,0,2,2,0,1,2,1,0,3,3,3,0,
/*0x50*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x60*/	1,2,0,0,0,2,2,0,1,2,1,0,3,3,3,0,
/*0x70*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x80*/	0,2,0,0,2,2,2,0,1,0,1,0,3,3,3,0,
/*0x90*/	2,2,0,0,2,2,2,0,1,3,1,0,0,3,0,0,
/*0xA0*/	2,2,2,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xB0*/	2,2,0,0,2,2,2,0,1,3,1,0,3,3,3,0,
/*0xC0*/	2,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xD0*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0xE0*/	2,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xF0*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0
};


//the optype table is a quick way to grab the addressing mode for any 6502 opcode
//
//  0 = Implied\Accumulator\Immediate\Branch\NULL
//  1 = (Indirect,X)
//  2 = Zero Page
//  3 = Absolute
//  4 = (Indirect),Y
//  5 = Zero Page,X
//  6 = Absolute,Y
//  7 = Absolute,X
//  8 = Zero Page,Y
//
FCEUX11_CACHE_ALIGN const uint8 optype[256] = {
/*0x00*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x10*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x20*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x30*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x40*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x50*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x60*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x70*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x80*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x90*/	0,4,0,3,5,5,8,8,0,6,0,6,7,7,6,6,
/*0xA0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xB0*/	0,4,0,3,5,5,8,8,0,6,0,6,7,7,6,6,
/*0xC0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xD0*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0xE0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xF0*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
};

// the opwrite table aids in predicting the value written for any 6502 opcode
//
//  0 = No value written
//  1 = Write from A
//  2 = Write from X
//  3 = Write from Y
//  4 = Write from P
//  5 = ASL (SLO)
//  6 = LSR (SRE)
//  7 = ROL (RLA)
//  8 = ROR (RRA)
//  9 = INC (ISC)
// 10 = DEC (DCP)
// 11 = (SAX)
// 12 = (AHX)
// 13 = (SHY)
// 14 = (SHX)
// 15 = (TAS)

FCEUX11_CACHE_ALIGN const uint8 opwrite[256] = {
/*0x00*/	 0, 0, 0, 5, 0, 0, 5, 5, 4, 0, 0, 0, 0, 0, 5, 5,
/*0x10*/	 0, 0, 0, 5, 0, 0, 5, 5, 0, 0, 0, 5, 0, 0, 5, 5,
/*0x20*/	 0, 0, 0, 7, 0, 0, 7, 7, 0, 0, 7, 0, 0, 0, 7, 7,
/*0x30*/	 0, 0, 0, 7, 0, 0, 7, 7, 0, 0, 0, 7, 0, 0, 7, 7,
/*0x40*/	 0, 0, 0, 6, 0, 0, 6, 6, 1, 0, 6, 0, 0, 0, 6, 6,
/*0x50*/	 0, 0, 0, 6, 0, 0, 6, 6, 0, 0, 0, 6, 0, 0, 6, 6,
/*0x60*/	 0, 0, 0, 8, 0, 0, 8, 8, 0, 0, 8, 0, 0, 0, 8, 8,
/*0x70*/	 0, 0, 0, 8, 0, 0, 8, 8, 0, 0, 0, 8, 0, 0, 8, 8,
/*0x80*/	 0, 1, 0,11, 3, 1, 2,11, 0, 0, 0, 0, 3, 1, 2,11,
/*0x90*/	 0, 1, 0,12, 3, 1, 2,11, 0, 1, 0,15,13, 1,14,12,
/*0xA0*/	 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
/*0xB0*/	 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
/*0xC0*/	 0, 0, 0,10, 0, 0,10,10, 0, 0, 0, 0, 0, 0,10,10,
/*0xD0*/	 0, 0, 0,10, 0, 0,10,10, 0, 0, 0,10, 0, 0,10,10,
/*0xE0*/	 0, 0, 0, 9, 0, 0, 9, 9, 0, 0, 0, 0, 0, 0, 9, 9,
/*0xF0*/	 0, 0, 0, 9, 0, 0, 9, 9, 0, 0, 0, 9, 0, 0, 9, 9,
};
