#ifndef _X6502STRUCTH
#define _X6502STRUCTH

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4324)
#endif
// hotfix1 P3-1: identifiers beginning with two underscores (or with an
// underscore followed by an upper-case letter) are reserved for the
// implementation per the C and C++ standards. `__X6502` was therefore
// technically a strict-aliasing / name-collision hazard on conforming
// toolchains. Rename the struct tag to `X6502` (the same name as the
// public typedef at the bottom of this file) and update the few
// internal references — the public typedef is unchanged so all existing
// `X6502 foo;` and `X6502 *bar;` callers keep working.

// v2.0 wip (Phase 0, audit S1): layout assertions for vNESU11 compat.
// The Rust `CpuRegsLayout` mirrors this struct byte-for-byte; any drift
// here must be reflected in `src/rust/crates/vnesu11/src/cpu/regs.rs`.
// NOTE: the static_asserts must appear AFTER the struct definition
// (they reference `X6502`), which they do at the bottom of this file.
typedef struct alignas(64) X6502 {
  int32 tcount;     /* Temporary cycle counter */
  uint16 PC;        /* I'll change this to uint32 later... */
                                /* I'll need to AND PC after increments to 0xFFFF */
                                /* when I do, though.  Perhaps an IPC() macro? */
        uint8 A,X,Y,S,P,mooPI;
        uint8 jammed;

	int32 count;
  uint32 IRQlow;    /* Simulated IRQ pin held low(or is it high?).
                                   And other junk hooked on for speed reasons.*/
  uint8 DB;         /* Data bus "cache" for reads from certain areas */

  int preexec;      /* Pre-exec'ing for debug breakpoints. */

	#ifdef FCEUDEF_DEBUGGER
        void (*CPUHook)(struct X6502 *);
        uint8 (*ReadHook)(struct X6502 *, unsigned int);
        void (*WriteHook)(struct X6502 *, unsigned int, uint8);
	#endif

} X6502;

// v2.0 wip (Phase 0, audit S1): per-field offset + size + align locks.
// The Rust `CpuRegsLayout` mirrors this byte-for-byte; any drift here
// must be reflected in `src/rust/crates/vnesu11/src/cpu/regs.rs`.
static_assert(offsetof(X6502, tcount) == 0,   "tcount must be at offset 0");
static_assert(offsetof(X6502, PC)     == 4,   "PC must be at offset 4");
static_assert(offsetof(X6502, A)      == 6,   "A must be at offset 6");
static_assert(offsetof(X6502, jammed) == 12,  "jammed must be at offset 12");
static_assert(offsetof(X6502, count)  == 16,  "count must be at offset 16");
static_assert(offsetof(X6502, IRQlow) == 20,  "IRQlow must be at offset 20");
static_assert(offsetof(X6502, DB)     == 24,  "DB must be at offset 24");
static_assert(offsetof(X6502, preexec) == 28, "preexec must be at offset 28");
#ifdef FCEUDEF_DEBUGGER
static_assert(offsetof(X6502, CPUHook) == 32,  "CPUHook must be at offset 32");
static_assert(offsetof(X6502, ReadHook) == 40, "ReadHook must be at offset 40");
static_assert(offsetof(X6502, WriteHook) == 48,"WriteHook must be at offset 48");
#endif
static_assert(sizeof(X6502) == 64, "X6502 must be 64 bytes (alignas(64))");
static_assert(alignof(X6502) == 64, "X6502 must be 64-byte aligned");

#ifdef _MSC_VER
#pragma warning(pop)
#endif

#endif
