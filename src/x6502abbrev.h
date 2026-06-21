#ifndef _X6502ABBREV_H_
#define _X6502ABBREV_H_

//include this file LAST, or else the #defines will overwrite CRT and STL symbols

// v1.3 Legion Phase 2: these abbreviations now route through g_cpu so the
// X6502 state object is fully owned by fceu11::Cpu. The global `X` inline
// alias in x6502.h is kept for files that have not yet been migrated.
#define _PC        g_cpu.native_layout().PC
#define _A         g_cpu.native_layout().A
#define _X         g_cpu.native_layout().X
#define _Y         g_cpu.native_layout().Y
#define _S         g_cpu.native_layout().S
#define _P         g_cpu.native_layout().P
#define _PI        g_cpu.native_layout().mooPI
#define _DB        g_cpu.native_layout().DB
#define _count     g_cpu.native_layout().count
#define _tcount    g_cpu.native_layout().tcount
#define _IRQlow    g_cpu.native_layout().IRQlow
#define _jammed    g_cpu.native_layout().jammed


#endif
