// movie_fm2.cpp
//
// v1.12 Scissors Phase F-A: FM2 format parsing/serialization split.
//
// Pure code move from src/movie.cpp — lines 200 (mnemonics initializer),
// 202-373 (MovieRecord FM2 I/O), 392-395 (MovieData::truncateAt),
// 397-462 (installValue), 464-544 (dump), 596-686 (LoadFM2).
//
// See movie_fm2.h.

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
 *  Copyright (C) 2003 Xodnizel
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

#include "types.h"
#include "movie.h"
#include "movie_fm2.h"

#include "emufile.h"
#include "utils/endian.h"
#include "utils/memory.h"
#include "utils/xstring.h"

#include "fceu.h"

#include <cstdint>
#include <cstring>
#include <span>
#include <string>

extern "C" {
#include "rust.h"
}

// ----------------------------------------------------------------------------
// MovieRecord::mnemonics — bit-order mnemonic table for the FM2 joypad
// dump. Lives here because the only consumer (MovieRecord::dumpJoy)
// also lives in this TU.
// ----------------------------------------------------------------------------

const char MovieRecord::mnemonics[8] = {'A','B','S','T','U','D','L','R'};

void MovieRecord::dumpJoy(EMUFILE* os, uint8 joystate)
{
	//these are mnemonics for each joystick bit.
	//since we usually use the regular joypad, these will be more helpful.
	//but any character other than ' ' or '.' should count as a set bit
	//maybe other input types will need to be encoded another way..
	for(int bit=7;bit>=0;bit--)
	{
		int bitmask = (1<<bit);
		char mnemonic = mnemonics[bit];
		//if the bit is set write the mnemonic
		if(joystate & bitmask)
			os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&mnemonic), 1));
		else //otherwise write an unset bit
			write8le('.',os);
	}
}

void MovieRecord::parseJoy(EMUFILE* is, uint8& joystate)
{
	char buf[8];
	is->fread(std::span<std::byte>(reinterpret_cast<std::byte*>(buf), 8));
	joystate = 0;
	for(int i=0;i<8;i++)
	{
		joystate <<= 1;
		joystate |= ((buf[i]=='.'||buf[i]==' ')?0:1);
	}
}

void MovieRecord::parse(MovieData* md, EMUFILE* is)
{
	//by the time we get in here, the initial pipe has already been extracted

	//extract the commands
	commands = uint32DecFromIstream(is);
	//*is >> commands;
	is->fgetc(); //eat the pipe

	//a special case: if fourscore is enabled, parse four gamepads
	if(md->fourscore)
	{
		parseJoy(is,joysticks[0]); is->fgetc(); //eat the pipe
		parseJoy(is,joysticks[1]); is->fgetc(); //eat the pipe
		parseJoy(is,joysticks[2]); is->fgetc(); //eat the pipe
		parseJoy(is,joysticks[3]); is->fgetc(); //eat the pipe
	}
	else
	{
		for(int port=0;port<2;port++)
		{
			if(md->ports[port] == SI_GAMEPAD)
				parseJoy(is, joysticks[port]);
			else if(md->ports[port] == SI_ZAPPER)
			{
				zappers[port].x = uint32DecFromIstream(is);
				zappers[port].y = uint32DecFromIstream(is);
				zappers[port].b = uint32DecFromIstream(is);
				zappers[port].bogo = uint32DecFromIstream(is);
				zappers[port].zaphit = uint64DecFromIstream(is);
			}

			is->fgetc(); //eat the pipe
		}
	}

	//(no fcexp data is logged right now)
	is->fgetc(); //eat the pipe

	//should be left at a newline
}


bool MovieRecord::parseBinary(MovieData* md, EMUFILE* is)
{
	commands = (uint8)is->fgetc();

	//check for eof
	if(is->eof()) return false;

	if(md->fourscore)
	{
		is->fread(std::span<std::byte>(reinterpret_cast<std::byte*>(&joysticks), 4));
	}
	else
	{
		for(int port=0;port<2;port++)
		{
			if(md->ports[port] == SI_GAMEPAD)
				joysticks[port] = (uint8)is->fgetc();
			else if(md->ports[port] == SI_ZAPPER)
			{
				zappers[port].x = (uint8)is->fgetc();
				zappers[port].y = (uint8)is->fgetc();
				zappers[port].b = (uint8)is->fgetc();
				zappers[port].bogo = (uint8)is->fgetc();
				read64le(&zappers[port].zaphit,is);
			}
		}
	}

	return true;
}


void MovieRecord::dumpBinary(MovieData* md, EMUFILE* os, int index)
{
	write8le(commands,os);
	if(md->fourscore)
	{
		for(int i=0;i<4;i++)
			os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&joysticks[i]), sizeof(joysticks[i])));
	}
	else
	{
		for(int port=0;port<2;port++)
		{
			if(md->ports[port] == SI_GAMEPAD)
				os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&joysticks[port]), sizeof(joysticks[port])));
			else if(md->ports[port] == SI_ZAPPER)
			{
				write8le(zappers[port].x,os);
				write8le(zappers[port].y,os);
				write8le(zappers[port].b,os);
				write8le(zappers[port].bogo,os);
				write64le(zappers[port].zaphit, os);
			}
		}
	}
}

void MovieRecord::dump(MovieData* md, EMUFILE* os, int index)
{
	// dump the commands
	//*os << '|' << setw(1) << (int)commands;
	os->fputc('|');
	putdec<uint8,3,false>(os, commands);	// "variable length decimal integer"

	//a special case: if fourscore is enabled, dump four gamepads
	if(md->fourscore)
	{
		os->fputc('|');
		dumpJoy(os,joysticks[0]); os->fputc('|');
		dumpJoy(os,joysticks[1]); os->fputc('|');
		dumpJoy(os,joysticks[2]); os->fputc('|');
		dumpJoy(os,joysticks[3]); os->fputc('|');
	}
	else
	{
		for(int port=0;port<2;port++)
		{
			os->fputc('|');
			if(md->ports[port] == SI_GAMEPAD)
				dumpJoy(os, joysticks[port]);
			else if(md->ports[port] == SI_ZAPPER)
			{
				putdec<uint8,3,true>(os,zappers[port].x); os->fputc(' ');
				putdec<uint8,3,true>(os,zappers[port].y); os->fputc(' ');
				putdec<uint8,1,true>(os,zappers[port].b); os->fputc(' ');
				putdec<uint8,1,true>(os,zappers[port].bogo); os->fputc(' ');
				putdec<uint64,20,false>(os,zappers[port].zaphit);
			}
		}
		os->fputc('|');
	}

	//(no fcexp data is logged right now)
	os->fputc('|');

	//each frame is on a new line
	os->fputc('\n');
}

// ----------------------------------------------------------------------------
// MovieData::truncateAt — resize the records[] vector to `frame`.
//
// Allocated to movie_fm2.cpp because the FM2 records[] shape is the
// closest semantic neighbor (dump/LoadFM2 also manipulate records[]
// shape). Called from FCEUMOV_AddInputState RECORD branch and from
// playback/record manipulators; resolved cross-TU through the
// MovieData declaration in movie.h.
// ----------------------------------------------------------------------------

void MovieData::truncateAt(int frame)
{
	records.resize(frame);
}

void MovieData::installValue(std::string& key, std::string& val)
{
	//todo - use another config system, or drive this from a little data structure. because this is gross
	if(key == "FDS")
		installInt(val,fds);
	else if(key == "NewPPU")
		installBool(val,PPUflag);
	else if(key == "RAMInitOption")
		installInt(val,RAMInitOption);
	else if(key == "RAMInitSeed")
		installInt(val,RAMInitSeed);
	else if(key == "version")
		installInt(val,version);
	else if(key == "emuVersion")
		installInt(val,emuVersion);
	else if(key == "rerecordCount")
		installInt(val,rerecordCount);
	else if(key == "palFlag")
		installBool(val,palFlag);
	else if(key == "romFilename")
		romFilename = val;
	else if(key == "romChecksum")
		StringToBytes(val,&romChecksum,MD5DATA::size);
	else if(key == "guid")
		guid = FCEU_Guid::fromString(val);
	else if(key == "fourscore")
		installBool(val,fourscore);
	else if(key == "microphone")
		installBool(val,microphone);
	else if(key == "port0")
		installInt(val,ports[0]);
	else if(key == "port1")
		installInt(val,ports[1]);
	else if(key == "port2")
		installInt(val,ports[2]);
	else if(key == "binary")
		installBool(val,binaryFlag);
	else if(key == "comment")
		comments.push_back(mbstowcs(val));
	else if (key == "subtitle")
		subtitles.push_back(val); //mbstowcs(val));
	else if(key == "savestate")
	{
		int len = Base64StringToBytesLength(val);
		if(len == -1) len = HexStringToBytesLength(val); // wasn't base64, try hex
		if(len >= 1)
		{
			savestate.resize(len);
			StringToBytes(val,&savestate[0],len); // decodes either base64 or hex
		}
	}
	else if(key == "saveram")
	{
		int len = Base64StringToBytesLength(val);
		if(len == -1) len = HexStringToBytesLength(val); // wasn't base64, try hex
		if(len >= 1)
		{
			saveram.resize(len);
			StringToBytes(val,&saveram[0],len); // decodes either base64 or hex
		}
	}
	else if (key == "length")
	{
		installInt(val, loadFrameCount);
	}
}

int MovieData::dump(EMUFILE *os, bool binary, bool seekToCurrFramePos)
{
	FceuMovieDataInput input;
	memset(&input, 0, sizeof(input));
	input.version = version;
	input.emu_version = emuVersion;
	input.fds = fds;
	input.pal_flag = palFlag;
	input.ppu_flag = PPUflag;
	memcpy(input.rom_checksum, romChecksum.data, 16);
	input.rom_filename = romFilename.c_str();
	input.rerecord_count = rerecordCount;
	input.guid = guid.toString().c_str();
	input.binary_flag = binary;
	input.load_frame_count = loadFrameCount;
	input.ports[0] = ports[0];
	input.ports[1] = ports[1];
	input.ports[2] = ports[2];
	input.fourscore = fourscore;
	input.microphone = microphone;
	input.ram_init_option = RAMInitOption;
	input.ram_init_seed = RAMInitSeed;

	input.savestate = savestate.empty() ? nullptr : savestate.data();
	input.savestate_len = savestate.size();
	input.saveram = saveram.empty() ? nullptr : saveram.data();
	input.saveram_len = saveram.size();

	std::vector<FceuMovieRecord> recs(records.size());
	for (size_t i = 0; i < records.size(); i++) {
		recs[i].joysticks[0] = records[i].joysticks[0];
		recs[i].joysticks[1] = records[i].joysticks[1];
		recs[i].joysticks[2] = records[i].joysticks[2];
		recs[i].joysticks[3] = records[i].joysticks[3];
		recs[i].zapper_x[0] = records[i].zappers[0].x;
		recs[i].zapper_x[1] = records[i].zappers[1].x;
		recs[i].zapper_y[0] = records[i].zappers[0].y;
		recs[i].zapper_y[1] = records[i].zappers[1].y;
		recs[i].zapper_b[0] = records[i].zappers[0].b;
		recs[i].zapper_b[1] = records[i].zappers[1].b;
		recs[i].zapper_bogo[0] = records[i].zappers[0].bogo;
		recs[i].zapper_bogo[1] = records[i].zappers[1].bogo;
		recs[i].zapper_zaphit[0] = records[i].zappers[0].zaphit;
		recs[i].zapper_zaphit[1] = records[i].zappers[1].zaphit;
		recs[i].commands = records[i].commands;
	}
	input.records = recs.data();
	input.records_count = recs.size();

	std::vector<const char*> commentPtrs;
	std::vector<std::string> commentStrs;
	for (auto& wc : comments) {
		commentStrs.push_back(wcstombs(wc));
		commentPtrs.push_back(commentStrs.back().c_str());
	}
	input.comments = commentPtrs.data();
	input.comments_count = commentPtrs.size();

	std::vector<const char*> subtitlePtrs;
	for (auto& s : subtitles) {
		subtitlePtrs.push_back(s.c_str());
	}
	input.subtitles = subtitlePtrs.data();
	input.subtitles_count = subtitlePtrs.size();

	EmuFileMem* mem = fceux11_rust_emufile_mem_create();
	int currFramePos = -1;
	int bytes = fceux11_rust_movie_data_dump(&input, mem, binary, seekToCurrFramePos, currFrameCounter, &currFramePos);

	size_t memSize = fceux11_rust_emufile_mem_size(mem);
	if (memSize > 0) {
		std::vector<uint8> tmp(memSize);
		fceux11_rust_emufile_mem_fread(mem, tmp.data(), memSize);
		os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(tmp.data()), memSize));
	}
	fceux11_rust_emufile_mem_destroy(mem);

	if (currFramePos >= 0)
		os->fseek(currFramePos, SEEK_SET);
	return bytes;
}

// ----------------------------------------------------------------------------
// LoadFM2 — top-level FM2 parser. Delegates the heavy lifting (FM2 v1/v2
// discrimination, header parsing, record decoding, base64 savestate/
// saveram decoding) to the Rust bridge `fceux11_rust_movie_load_fm2`,
// then projects the Rust-owned FceuMovieData into our C++ MovieData.
//
// `stopAfterHeader` returns only the header (used by fceu11::MovieGetInfo
// from the Qt MoviePlay dialog to populate the info pane).
// ----------------------------------------------------------------------------

bool LoadFM2(MovieData& movieData, EMUFILE* fp, int size, bool stopAfterHeader)
{
	std::ios::pos_type curr = fp->ftell();
	fp->fseek(0, SEEK_END);
	std::ios::pos_type end = fp->ftell();
	fp->fseek(curr, SEEK_SET);

	int to_read = size;
	if (size <= 0 || (end - curr) < (std::streamoff)size)
		to_read = (int)(end - curr);

	std::vector<uint8> buf(to_read);
	if (to_read > 0)
		fp->fread(std::span<std::byte>(reinterpret_cast<std::byte*>(buf.data()), to_read));

	FceuMovieData* rustMd = fceux11_rust_movie_load_fm2(buf.data(), buf.size(), stopAfterHeader);
	if (!rustMd)
		return false;

	movieData = MovieData();
	movieData.version = fceux11_rust_movie_data_version(rustMd);
	movieData.emuVersion = fceux11_rust_movie_data_emu_version(rustMd);
	movieData.fds = fceux11_rust_movie_data_fds(rustMd);
	movieData.palFlag = fceux11_rust_movie_data_pal_flag(rustMd);
	movieData.PPUflag = fceux11_rust_movie_data_ppu_flag(rustMd);
	movieData.rerecordCount = fceux11_rust_movie_data_rerecord_count(rustMd);
	movieData.binaryFlag = fceux11_rust_movie_data_binary_flag(rustMd);
	movieData.loadFrameCount = fceux11_rust_movie_data_load_frame_count(rustMd);
	movieData.fourscore = fceux11_rust_movie_data_fourscore(rustMd);
	movieData.microphone = fceux11_rust_movie_data_microphone(rustMd);
	movieData.RAMInitOption = fceux11_rust_movie_data_ram_init_option(rustMd);
	movieData.RAMInitSeed = fceux11_rust_movie_data_ram_init_seed(rustMd);
	fceux11_rust_movie_data_ports(rustMd, movieData.ports);
	fceux11_rust_movie_data_rom_checksum(rustMd, movieData.romChecksum.data);

	const char* romFilename = fceux11_rust_movie_data_rom_filename(rustMd);
	if (romFilename) movieData.romFilename = romFilename;

	const char* guid = fceux11_rust_movie_data_guid(rustMd);
	if (guid) movieData.guid = FCEU_Guid::fromString(guid);

	size_t recCount = fceux11_rust_movie_data_records_count(rustMd);
	movieData.records.resize(recCount);
	for (size_t i = 0; i < recCount; i++) {
		FceuMovieRecord rec;
		if (fceux11_rust_movie_data_record_get(rustMd, i, &rec)) {
			movieData.records[i].joysticks[0] = rec.joysticks[0];
			movieData.records[i].joysticks[1] = rec.joysticks[1];
			movieData.records[i].joysticks[2] = rec.joysticks[2];
			movieData.records[i].joysticks[3] = rec.joysticks[3];
			movieData.records[i].zappers[0].x = rec.zapper_x[0];
			movieData.records[i].zappers[0].y = rec.zapper_y[0];
			movieData.records[i].zappers[0].b = rec.zapper_b[0];
			movieData.records[i].zappers[0].bogo = rec.zapper_bogo[0];
			movieData.records[i].zappers[0].zaphit = rec.zapper_zaphit[0];
			movieData.records[i].zappers[1].x = rec.zapper_x[1];
			movieData.records[i].zappers[1].y = rec.zapper_y[1];
			movieData.records[i].zappers[1].b = rec.zapper_b[1];
			movieData.records[i].zappers[1].bogo = rec.zapper_bogo[1];
			movieData.records[i].zappers[1].zaphit = rec.zapper_zaphit[1];
			movieData.records[i].commands = rec.commands;
		}
	}

	size_t ssLen = fceux11_rust_movie_data_savestate_len(rustMd);
	if (ssLen > 0) {
		movieData.savestate.resize(ssLen);
		fceux11_rust_movie_data_savestate_copy(rustMd, movieData.savestate.data(), ssLen);
	}

	size_t srLen = fceux11_rust_movie_data_saveram_len(rustMd);
	if (srLen > 0) {
		movieData.saveram.resize(srLen);
		fceux11_rust_movie_data_saveram_copy(rustMd, movieData.saveram.data(), srLen);
	}

	size_t commentCount = fceux11_rust_movie_data_comments_count(rustMd);
	for (size_t i = 0; i < commentCount; i++) {
		const char* c = fceux11_rust_movie_data_comment_get(rustMd, i);
		if (c) movieData.comments.push_back(mbstowcs(c));
	}

	size_t subtitleCount = fceux11_rust_movie_data_subtitles_count(rustMd);
	for (size_t i = 0; i < subtitleCount; i++) {
		const char* s = fceux11_rust_movie_data_subtitle_get(rustMd, i);
		if (s) movieData.subtitles.push_back(s);
	}

	fceux11_rust_movie_data_free(rustMd);
	return true;
}