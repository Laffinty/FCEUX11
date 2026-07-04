/// \file
/// \brief Wave Audio Export wrapper — delegates to Rust.
///
/// Phase 4 (v0.2.5): Rust module provides a memory-safe WAV file writer
/// using `std::fs::File` and manual PCM header construction.

#include "types.h"
#include "fceu.h"

#include "driver.h"
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
