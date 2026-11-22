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
// ConsoleViewerInterface.cpp
//
#include "Qt/ConsoleViewerInterface.h"

//----------------------------------------------------------
void ConsoleViewerBase::memset32( void *buf, uint32_t val, size_t size)
{
	uint32_t *p = static_cast<uint32_t*>(buf);
	size_t n = size / sizeof(uint32_t);

	for (size_t i=0; i<n; i++)
	{
		*p = val; p++;
	}
}
//----------------------------------------------------------
void ConsoleViewerBase::copyPixels32( void *dest, void *src, size_t size, uint32_t alphaMask)
{
	uint32_t *d = static_cast<uint32_t*>(dest);
	uint32_t *s = static_cast<uint32_t*>(src);
	size_t n = size / sizeof(uint32_t);

	for (size_t i=0; i<n; i++)
	{
		*d = *s | alphaMask; d++; s++;
	}
}
//----------------------------------------------------------
