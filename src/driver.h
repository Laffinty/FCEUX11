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

// FCEUX11 v0.3.9 — Backward-compatibility shim for the historic driver.h.
// This file was physically split into four peer API headers per plan v3 §5
// (core_api.h / io_api.h / net_api.h / diag_api.h).  Re-include all four so
// existing #include "driver.h" call sites compile without source edits.
//
// v1.11 Bridge tracks the final removal of this shim once every call site
// has been migrated to include only the APIs it actually uses.

#ifndef __FCEU_DRIVER_H_
#define __FCEU_DRIVER_H_

#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"

#endif // __FCEU_DRIVER_H_
