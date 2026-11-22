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
void DrawTextLineBG(uint8 *dest);
void DrawMessage(bool beforeMovie);
void FCEU_DrawRecordingStatus(uint8* XBuf);
void FCEU_DrawNumberRow(uint8 *XBuf, int *nstatus, int cur);
void DrawTextTrans(uint8 *dest, uint32 width, uint8 *textmsg, uint8 fgcolor);
void DrawTextTransWH(uint8 *dest, int width, uint8 *textmsg, uint8 fgcolor, int max_w, int max_h, int border);
