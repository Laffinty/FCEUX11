// mapinc_bus.h - v1.8 Masonry §4 split
// Phase B: pulls in mapinc_state.h plus x6502.h (CPU access macros).
// Provides: setprg*/setchr*/setmirror*, getRealAddress, addressing macros.
// Default include for any bank-switching board (97 + 1 = 98 of 171 files).
#ifndef FCEU11_MAPINC_BUS_H
#define FCEU11_MAPINC_BUS_H

#include "mapinc_state.h"
#include "../cpu.h"   // X6502_IRQBegin/End, FCEU_IQ* (Phase 7: was ../x6502.h)

#endif