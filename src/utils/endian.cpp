/* FCE Ultra - NES/Famicom Emulator
*
* Copyright notice for this file:
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

/// \file
/// \brief contains file I/O functions that write/read data LSB first.

#include <stdio.h>
#include <ostream>
#include <fstream>
#include "../types.h"
#include "endian.h"
#include "../emufile.h"

#ifndef FCEU_LITTLE_ENDIAN
#include <cstdlib> // _byteswap_ushort / _byteswap_ulong / _byteswap_uint64
#endif

namespace {
#ifdef FCEU_LITTLE_ENDIAN
constexpr uint16 bswap16(uint16 x) noexcept { return x; }
constexpr uint32 bswap32(uint32 x) noexcept { return x; }
constexpr uint64 bswap64(uint64 x) noexcept { return x; }
#else
inline uint16 bswap16(uint16 x) noexcept { return _byteswap_ushort(x); }
inline uint32 bswap32(uint32 x) noexcept { return _byteswap_ulong(x); }
inline uint64 bswap64(uint64 x) noexcept { return _byteswap_uint64(x); }
#endif
} // namespace

///endian-flips count bytes.  count should be even and nonzero.
void FlipByteOrder(uint8 *src, uint32 count) noexcept
{
	if ((count & 1u) || count == 0) return;        /* This shouldn't happen. */

	for (uint32 i = 0, j = count - 1; i < j; ++i, --j)
	{
		const uint8 tmp = src[i];
		src[i] = src[j];
		src[j] = tmp;
	}
}

///writes a little endian 16bit value to the specified file
int write16le(uint16 b, FILE *fp)
{
	uint8 s[2];
	s[0]=b;
	s[1]=b>>8;
	return((fwrite(s,1,2,fp)<2)?0:2);
}

///writes a little endian 32bit value to the specified file
int write32le(uint32 b, FILE *fp)
{
	uint8 s[4];
	s[0]=b;
	s[1]=b>>8;
	s[2]=b>>16;
	s[3]=b>>24;
	return((fwrite(s,1,4,fp)<4)?0:4);
}

int write32le(uint32 b, std::ostream* os)
{
	uint8 s[4];
	s[0]=b;
	s[1]=b>>8;
	s[2]=b>>16;
	s[3]=b>>24;
	os->write((char*)&s,4);
	return 4;
}

int write64le(uint64 b, std::ostream* os)
{
	uint8 s[8];
	s[0]=b;
	s[1]=b>>8;
	s[2]=b>>16;
	s[3]=b>>24;
	s[4]=b>>32;
	s[5]=b>>40;
	s[6]=b>>48;
	s[7]=b>>56;
	os->write((char*)&s,8);
	return 8;
}


///reads a little endian 32bit value from the specified file
int read32le(uint32 *Bufo, FILE *fp)
{
	uint32 buf;
	if(fread(&buf,1,4,fp)<4)
		return 0;
	*Bufo = bswap32(buf);
	return 1;
}

int read16le(uint16 *Bufo, std::istream *is)
{
	uint16 buf;
	if(is->read((char*)&buf,2).gcount() != 2)
		return 0;
	*Bufo = bswap16(buf);
	return 1;
}

///reads a little endian 64bit value from the specified file
int read64le(uint64 *Bufo, std::istream *is)
{
	uint64 buf;
	if(is->read((char*)&buf,8).gcount() != 8)
		return 0;
	*Bufo = bswap64(buf);
	return 1;
}


int read32le(uint32 *Bufo, std::istream *is)
{
	uint32 buf;
	if(is->read((char*)&buf,4).gcount() != 4)
		return 0;
	*Bufo = bswap32(buf);
	return 1;
}

///stores a 32bit value into the provided byte array in guaranteed little endian form
void FCEU_en32lsb(uint8 *buf, uint32 morp)
{ 
	buf[0]=morp;
	buf[1]=morp>>8;
	buf[2]=morp>>16;
	buf[3]=morp>>24;
} 

void FCEU_en16lsb(uint8* buf, uint16 morp)
{
	buf[0]=morp;
	buf[1]=morp>>8;
}

///unpacks a 64bit little endian value from the provided byte array into host byte order
uint64 FCEU_de64lsb(uint8 *morp)
{
	return morp[0]|(morp[1]<<8)|(morp[2]<<16)|(morp[3]<<24)|((uint64)morp[4]<<32)|((uint64)morp[5]<<40)|((uint64)morp[6]<<48)|((uint64)morp[7]<<56);
}

///unpacks a 32bit little endian value from the provided byte array into host byte order
uint32 FCEU_de32lsb(uint8 *morp)
{
	return morp[0]|(morp[1]<<8)|(morp[2]<<16)|(morp[3]<<24);
}

///unpacks a 16bit little endian value from the provided byte array into host byte order
uint16 FCEU_de16lsb(uint8 *morp)
{
	return morp[0]|(morp[1]<<8);
}


//well. just for the sake of consistency
int write8le(u8 b, EMUFILE*os)
{
	// v0.3.10: std::span virtual directly (avoids [[deprecated]] shim).
	os->fwrite(std::span<const std::byte>(
		reinterpret_cast<const std::byte*>(&b), sizeof(b)));
	return 1;
}

//well. just for the sake of consistency
int read8le(u8 *Bufo, EMUFILE*is)
{
	// v0.3.10: std::span virtual directly (avoids [[deprecated]] shim).
	if(is->fread(std::span<std::byte>(
		reinterpret_cast<std::byte*>(Bufo), sizeof(*Bufo))) != sizeof(*Bufo))
		return 0;
	return 1;
}

///writes a little endian 16bit value to the specified file
int write16le(u16 b, EMUFILE *fp)
{
	u8 s[2];
	s[0]=(u8)b;
	s[1]=(u8)(b>>8);
	// v0.3.10: std::span virtual directly.
	fp->fwrite(std::span<const std::byte>(
		reinterpret_cast<const std::byte*>(s), sizeof(s)));
	return 2;
}


///writes a little endian 32bit value to the specified file
int write32le(u32 b, EMUFILE *fp)
{
	uint8 s[4];
	s[0]=(u8)b;
	s[1]=(u8)(b>>8);
	s[2]=(u8)(b>>16);
	s[3]=(u8)(b>>24);
	// v0.3.10: std::span virtual directly.
	fp->fwrite(std::span<const std::byte>(
		reinterpret_cast<const std::byte*>(s), sizeof(s)));
	return 4;
}

void writebool(bool b, EMUFILE* os) { write32le(b?1:0,os); }

int write64le(uint64 b, EMUFILE* os)
{
	uint8 s[8];
	s[0]=(u8)b;
	s[1]=(u8)(b>>8);
	s[2]=(u8)(b>>16);
	s[3]=(u8)(b>>24);
	s[4]=(u8)(b>>32);
	s[5]=(u8)(b>>40);
	s[6]=(u8)(b>>48);
	s[7]=(u8)(b>>56);
	// v0.3.10: std::span virtual directly.
	os->fwrite(std::span<const std::byte>(
		reinterpret_cast<const std::byte*>(s), sizeof(s)));
	return 8;
}


int read32le(uint32 *Bufo, EMUFILE *fp)
{
	uint32 buf=0;
	// v0.3.10: std::span virtual directly.
	if(fp->fread(std::span<std::byte>(
		reinterpret_cast<std::byte*>(&buf), sizeof(buf))) < sizeof(buf))
		return 0;
	*Bufo = bswap32(buf);
	return 1;
}

int read16le(u16 *Bufo, EMUFILE *is)
{
	u16 buf=0;
	// v0.3.10: std::span virtual directly.
	if(is->fread(std::span<std::byte>(
		reinterpret_cast<std::byte*>(&buf), sizeof(buf))) != sizeof(buf))
		return 0;
	*Bufo = bswap16(buf);
	return 1;
}

int read64le(uint64 *Bufo, EMUFILE *is)
{
	u64 buf=0;
	// v0.3.10: std::span virtual directly.
	if(is->fread(std::span<std::byte>(
		reinterpret_cast<std::byte*>(&buf), sizeof(buf))) != sizeof(buf))
		return 0;
	*Bufo = bswap64(buf);
	return 1;
}
