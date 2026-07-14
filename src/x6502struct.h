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
#ifdef _MSC_VER
#pragma warning(pop)
#endif

#endif
