/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2001 Aaron Oneal
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

// FCEUX11 v0.3.9 — diag_api.h (physical split of driver.h per plan
// v3 §5 v0.3.9). This header owns the diagnostic surface: the build
// identifier string reported by the "About" dialog, the runtime version
// string, and the FCEU_NAME_AND_VERSION macro re-exported as an
// inline accessor. Independent of the rest of the API surface — peer
// to core_api.h / io_api.h / net_api.h.

#ifndef __FCEU_DIAG_API_H_
#define __FCEU_DIAG_API_H_

#include "types.h"
#include "version.h"

// mbg 7/23/06 — emitted into the "About" / build-info dialog.
const char *FCEUD_GetCompilerString();

// Returns the current project's human-readable version string,
// e.g. "v0.3.9". This is a thin inline accessor over the
// FCEU_DISPLAY_VERSION macro defined in version.h, exposed as a
// function so the Rust FFI layer (fceux11-formats) can call it
// through a stable C ABI without pulling in the preprocessor.
inline const char *FCEU_GetVersion() {
    return FCEU_DISPLAY_VERSION;
}

// Returns the project name + version, e.g. "FCEUX11 v0.3.9". Same
// inline-accessor pattern as FCEU_GetVersion() above.
inline const char *FCEU_GetNameAndVersion() {
    return FCEU_NAME_AND_VERSION;
}

#endif //__FCEU_DIAG_API_H_
