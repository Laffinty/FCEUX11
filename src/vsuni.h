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
enum IOPTION {
	VS_OPTION_GUN = 0x1,
	VS_OPTION_SWAPDIRAB = 0x2,
	VS_OPTION_PREDIP = 0x10,
};

typedef struct {
	const char* name;
	uint64 md5partial;
	int mapper;
	int mirroring;
	EGIPPU ppu;
	int ioption;
	int predip;
	EGIVS type;
} VSUNIENTRY;

void FCEU_VSUniPower(void);
void FCEU_VSUniCheck(uint64 md5partial, int *, uint8 *);
void FCEU_VSUniDraw(uint8 *XBuf);

void FCEU_VSUniToggleDIP(int);  /* For movies and netplay */
void FCEU_VSUniCoin(uint8 slot);
void FCEU_VSUniService();
void FCEU_VSUniSwap(uint8 *j0, uint8 *j1);
