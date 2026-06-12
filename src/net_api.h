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

// FCEUX11 v0.3.9 — net_api.h (physical split of driver.h per plan
// v3 §5 v0.3.9). This header owns the netplay surface: start/stop a
// network play session, send/receive opaque blobs of input or chat
// text, and the network-shutdown callback. Independent of the rest of
// the API surface — peer to core_api.h / io_api.h / diag_api.h.

#ifndef __FCEU_NET_API_H_
#define __FCEU_NET_API_H_

#include "types.h"

// Call only when a game is loaded. `nlocal` is the local player index,
// `divisor` controls input-frame merging (1 = sync every frame, 2 =
// sync every other frame, …).
int  FCEUI_NetplayStart(int nlocal, int divisor);

// Call when network play needs to stop.
void FCEUI_NetplayStop(void);

// Note: YOU MUST NOT CALL ANY FCEUI_* FUNCTIONS WHILE IN
// FCEUD_SendData() or FCEUD_RecvData() — these run in the network
// thread and the core is not re-entrant.
//
// Return 0 on failure, 1 on success.
int FCEUD_SendData(void *data, uint32 len);
int FCEUD_RecvData(void *data, uint32 len);

// Display text received over the network.
void FCEUD_NetplayText(uint8 *text);
// Encode and send text over the network.
void FCEUI_NetplayText(uint8 *text);

// Called when a fatal error occurred and network play can't continue.
// This function should call FCEUI_NetplayStop() after it has
// deinitialized the network on the driver side.
void FCEUD_NetworkClose(void);

#endif //__FCEU_NET_API_H_
