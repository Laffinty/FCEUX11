#ifndef _OPS_TABLE_VARIANT_H_
#define _OPS_TABLE_VARIANT_H_

// v2.0.4 V8 (A7 alias experiment) — dispatch-table selector.
//
// The canonical table src/ops_table.inc routes the `_PC` / `_A` / ... macros
// through the global `g_cpu` accessor (src/x6502abbrev.h); the handler
// parameter `xp` is therefore unused (`(void)xp;`).
//
// The experiment table src/ops_table_a7.inc re-routes the same macros through
// the handler parameter -- the state the dispatcher was handed -- so the
// compiler no longer has to re-load the global reference after every store
// through the layout.  It is selected only when the build is configured with
// -DFCEUX11_X6502_ALIAS_VARIANT=ON and is never part of a default build.
//
// The selector lives in this header (instead of an #ifdef block in
// src/x6502.cpp) on purpose: it keeps src/x6502.cpp line-for-line identical to
// the pre-V8 revision, so every `x6502.cpp:<line>` coordinate quoted in the
// docs and in tests/tests.json keeps pointing at the same line.
//
// Regenerate the experiment table with:
//   python scripts/generate_x6502_dispatch.py --variant a7

#ifdef FCEUX11_X6502_ALIAS_VARIANT
#include "ops_table_a7.inc"
#else
#include "ops_table.inc"
#endif

#endif // _OPS_TABLE_VARIANT_H_
