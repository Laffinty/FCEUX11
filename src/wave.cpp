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
/// \file
/// \brief Wave Audio Export wrapper â€?delegates to Rust.
///
/// Phase 4 (v0.2.5): Rust module provides a memory-safe WAV file writer
/// using `std::fs::File` and manual PCM header construction.

#include "types.h"
#include "fceu.h"

#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "sound.h"
#include "wave.h"

#include <cstdio>
#include <cstdlib>

#include "rust/fceux11_rust.h"

void FCEU_WriteWaveData(int32 *Buffer, int Count)
{
	int16 *temp = (int16*)alloca(Count*2);
	int16 *dest;
	int x;

	if(!fceux11_rust_wave_running()) return;

	dest=temp;
	x=Count;

	// Convert int32 samples to little-endian int16
	while(x--)
	{
		int16 tmp=*Buffer;
		*(uint8 *)dest=(((uint16)tmp)&255);
		*(((uint8 *)dest)+1)=(((uint16)tmp)>>8);
		dest++;
		Buffer++;
	}

	if(fceux11_rust_wave_running())
		fceux11_rust_wave_write(temp, Count);
}

int fceu11::EndWaveRecord()
{
	return fceux11_rust_wave_end();
}

bool fceu11::BeginWaveRecord(const char *fn)
{
	return fceux11_rust_wave_begin(fn, FSettings.SndRate);
}

bool fceu11::WaveRecordRunning()
{
	return fceux11_rust_wave_running();
}
