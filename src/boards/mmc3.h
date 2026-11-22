/* FCE Ultra - NES/Famicom Emulator
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
// FCEUX11 — mmc3.h shared declarations.
// v1.8 Masonry Phase D.3: CartInfo forward decl so callers that include
// mmc3.h standalone (mmc3_base_cart.cpp) get a parseable declaration.
// mmc3.cpp itself transitively pulls cart.h via mapinc_mmc3.h.
struct CartInfo;

extern uint8 MMC3_cmd;
extern uint8 mmc3opts;
extern uint8 A000B;
extern uint8 A001B;
extern uint8 EXPREGS[8];
extern uint8 DRegBuf[8];

#undef IRQCount
#undef IRQLatch
#undef IRQa
extern uint8 IRQCount,IRQLatch,IRQa;
extern uint8 IRQReload;

extern void (*pwrap)(uint32 A, uint8 V);
extern void (*cwrap)(uint32 A, uint8 V);
extern void (*mwrap)(uint8 V);

void GenMMC3Power(void);
void GenMMC3Restore(int version);
void MMC3RegReset(void);
void FixMMC3PRG(int V);
void FixMMC3CHR(int V);
DECLFW(MMC3_CMDWrite);
DECLFW(MMC3_IRQWrite);

void GenMMC3_Init(CartInfo *info, int prg, int chr, int wram, int battery);
