// movie_settings.cpp
//
// v1.13 Phase B / Batch D-D.5-D.7 (the "small getters / toggles /
// command-dispatch / backup" cluster): a grab-bag of small public
// `FCEUMOV_*` / `FCEUI_*` / `fceu11::*` definitions relocated here from
// src/movie.cpp. The cluster is split into two semantic halves:
//
//  1. Command / counter / playback-state get/toggle surface
//     (FCEUMOV_AddCommand + IncrementRerecordCount + MovieToggle*
//      family + Get/Set Movie Toggle Read-Only + MovieGetInfo).
//
//  2. Backup management (FCEUI_CreateMovieFile + FCEUI_MakeBackupMovie).
//
// Pulling them into one TU keeps them off movie.cpp's main path; the
// API declarations stay in movie.h so callers are unaffected.

#include "movie.h"

#include "fceu.h"
#include "netplay.h"
#include "fds.h"
#include "vsuni.h"
#include "input.h"
#include "utils/safe_string.h"

#ifdef _S9XLUA_H
#include "fceulua.h"
#endif

#include <cstring>
#include <cstdio>
#include <fstream>
#include <sstream>

// ----------------------------------------------------------------------------
// Phase D-D.4 / D-D.5 cross-TU forward decls. _currCommand lives in
// movie.cpp (was TU-static, promoted). FCEUMOV_FromPoweron and the
// get/length counters are in movie.cpp. The stop + IO helpers come
// from movie_record.cpp / movie_io.cpp.
// ----------------------------------------------------------------------------
extern int _currCommand;
void StopRecording();
void StopPlayback();
void RedumpWholeMovieFile(bool justToggledRecording = false);
void OnMovieClosed();
bool FCEUMOV_FromPoweron();

// File-scope settings globals and helpers living elsewhere.
extern int currRerecordCount;
extern int frame_display;
extern int rerecord_display;
extern int input_display;
extern int pauseframe;
extern unsigned int lagCounter;          // fceu.cpp (input.h extern)
extern char lagFlag;                     // fceu.cpp (input.h extern)
extern const char* GetMovieModeStr();    // movie_record.cpp

// v1.13 Phase B / Batch D-D.4-extension: PLAY branch helpers used by
// MovieAddInputState_Playback (in this file).
void FinishPlayback();
bool FCEUD_PauseAfterPlayback();

// ----------------------------------------------------------------------------
// Misc small public-API (mode + lag + pause-frame checks + name query)
// ----------------------------------------------------------------------------

int FCEUMOV_GetFrame(void)
{
	return currFrameCounter;
}

int FCEUI_GetLagCount(void)
{
	return lagCounter;
}

bool FCEUI_GetLagged(void)
{
	if (lagFlag)
		return true;
	else
		return false;
}

void FCEUI_SetLagFlag(bool value)
{
	lagFlag = (value) ? 1 : 0;
}

bool FCEUMOV_ShouldPause(void)
{
	if(pauseframe && currFrameCounter+1 == pauseframe)
	{
		pauseframe = 0;
		return true;
	}
	else
	{
		return false;
	}
}

EMOVIEMODE FCEUMOV_Mode()
{
	return movieMode;
}

bool FCEUMOV_Mode(EMOVIEMODE modemask)
{
	return (movieMode&modemask)!=0;
}

bool FCEUMOV_Mode(int modemask)
{
	return FCEUMOV_Mode((EMOVIEMODE)modemask);
}

std::string fceu11::GetMovieName(void)
{
	return curMovieFilename;
}

// ----------------------------------------------------------------------------
// Per-command staging (cmd dispatcher for netplay bridge)
// ----------------------------------------------------------------------------

//TODO
void FCEUMOV_AddCommand(int cmd)
{
	// do nothing if not recording a movie
	if(movieMode != MOVIEMODE_RECORD && movieMode != MOVIEMODE_TASEDITOR)
		return;

	// translate "FCEU NetPlay" command to "FCEU Movie" command
	switch (cmd)
	{
		case FCEUNPCMD_RESET: cmd = MOVIECMD_RESET; break;
		case FCEUNPCMD_POWER: cmd = MOVIECMD_POWER; break;
		case FCEUNPCMD_FDSINSERT: cmd = MOVIECMD_FDS_INSERT; break;
		case FCEUNPCMD_FDSSELECT: cmd = MOVIECMD_FDS_SELECT; break;
		case FCEUNPCMD_VSUNICOIN: cmd = MOVIECMD_VS_INSERTCOIN; break;
		case FCEUNPCMD_VSUNICOIN2: cmd = MOVIECMD_VS_INSERTCOIN2; break;
		case FCEUNPCMD_VSUNISERVICE: cmd = MOVIECMD_VS_SERVICE; break;
		// all other netplay commands (e.g. FCEUNPCMD_VSUNIDIP0) are not supported by movie recorder for now
		default: return;
	}

	_currCommand |= cmd;
}

void FCEUMOV_IncrementRerecordCount()
{
#ifdef _S9XLUA_H
	if(!FCEU_LuaRerecordCountSkip())
		if (movieMode != MOVIEMODE_TASEDITOR)
			currRerecordCount++;
		else
			currMovieData.rerecordCount++;
#else
	if (movieMode != MOVIEMODE_TASEDITOR)
		currRerecordCount++;
	else
		currMovieData.rerecordCount++;
#endif
	if (movieMode != MOVIEMODE_TASEDITOR)
		currMovieData.rerecordCount = currRerecordCount;
}

// ----------------------------------------------------------------------------
// HUD toggle / getter surface (per-frame + GUI binding)
// ----------------------------------------------------------------------------

void fceu11::MovieToggleFrameDisplay(void)
{
	frame_display=!frame_display;
}

void FCEUI_MovieToggleRerecordDisplay()
{
	rerecord_display ^= 1;
}

void FCEUI_ToggleInputDisplay(void)
{
	switch(input_display)
	{
	case 0:
		input_display = 1;
		break;
	case 1:
		input_display = 2;
		break;
	case 2:
		input_display = 4;
		break;
	default:
		input_display = 0;
		break;
	}
}

int FCEUI_GetMovieLength()
{
	return currMovieData.records.size();
}

int FCEUI_GetMovieRerecordCount()
{
	return currMovieData.rerecordCount;
}

bool fceu11::GetMovieToggleReadOnly()
{
	return movie_readonly;
}

void fceu11::SetMovieToggleReadOnly(bool which)
{
	if (which)	//If set to readonly
	{
		if (!movie_readonly)	//If not already set
		{
			movie_readonly = true;
			FCEU_DispMessage("Movie is now Read-Only.",0);
		}
		else					//Else restate message
			FCEU_DispMessage("Movie is Read-Only.",0);
	}
	else		//If set to read+write
	{
		if (movie_readonly)		//If not already set
		{
			movie_readonly = false;
			FCEU_DispMessage("Movie is now Read+Write.",0);
		}
		else					//Else restate message
			FCEU_DispMessage("Movie is Read+Write.",0);
	}
}

//auqnull: What's the point to toggle Read-Only without a movie loaded?
void fceu11::MovieToggleReadOnly()
{
	char message[260];

	movie_readonly = !movie_readonly;
	if (movie_readonly)
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read-Only");
	else
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read+Write");

	safe_strcat(message, sizeof(message), GetMovieModeStr());
	FCEU_DispMessage("%s",0,message);
}

bool fceu11::MovieGetInfo(FCEUFILE* fp, MOVIE_INFO& info, bool skipFrameCount)
{
	MovieData md;
	if(!LoadFM2(md, fp->stream, fp->size, skipFrameCount))
		return false;

	info.movie_version = md.version;
	info.poweron = md.savestate.size()==0;
	info.reset = false; //Soft-reset isn't used from starting movies anymore, so this will be false, better for FCEUFILE to have that info (as |1| on the first frame indicates it
	info.pal = md.palFlag;
	info.ppuflag = md.PPUflag;
	info.RAMInitOption = md.RAMInitOption;
	info.RAMInitSeed = md.RAMInitSeed;
	info.nosynchack = true;
	info.num_frames = md.records.size();
	info.md5_of_rom_used = md.romChecksum;
	info.emu_version_used = md.emuVersion;
	info.name_of_rom_used = md.romFilename;
	info.rerecord_count = md.rerecordCount;
	info.comments = md.comments;
	info.subtitles = md.subtitles;

	return true;
}

// ----------------------------------------------------------------------------
// Backup management
// ----------------------------------------------------------------------------

void FCEUI_CreateMovieFile(std::string fn)
{
	MovieData md = currMovieData;							//Get current movie data
	EMUFILE* outf = FCEUD_UTF8_fstream(fn, "wb");		//open/create file
	md.dump(outf,false);									//dump movie data
	delete outf;											//clean up, delete file object
}

void FCEUI_MakeBackupMovie(bool dispMessage)
{
	//This function generates backup movie files
	std::string currentFn;					//Current movie fillename
	std::string backupFn;					//Target backup filename
	std::string tempFn;						//temp used in back filename creation
	std::stringstream stream;
	int x;								//Temp variable for string manip
	bool exist = false;					//Used to test if filename exists
	bool overflow = false;				//Used for special situation when backup numbering exceeds limit

	currentFn = curMovieFilename;		//Get current moviefilename
	backupFn = curMovieFilename;		//Make backup filename the same as current moviefilename
	x = backupFn.find_last_of(".");		 //Find file extension
	backupFn = backupFn.substr(0,x);	//Remove extension
	tempFn = backupFn;					//Store the filename at this point
	for (unsigned int backNum=0;backNum<999;backNum++) //999 = arbituary limit to backup files
	{
		stream.str("");					 //Clear stream
		if (backNum > 99)
			stream << "-" << backNum;	 //assign backNum to stream
		else if (backNum <=99 && backNum >= 10)
			stream << "-0";				//Make it 010, etc if two digits
		else
			stream << "-00" << backNum;	 //Make it 001, etc if single digit
		backupFn.append(stream.str());	 //add number to bak filename
		backupFn.append(".bak");		 //add extension

		exist = CheckFileExists(backupFn.c_str());	//Check if file exists

		if (!exist)
			break;						//Yeah yeah, I should use a do loop or something
		else
		{
			backupFn = tempFn;			//Before we loop again, reset the filename

			if (backNum == 999)			//If 999 exists, we have overflowed, let's handle that
			{
				backupFn.append("-001.bak"); //We are going to simply overwrite 001.bak
				overflow = true;		//Flag that we have exceeded limit
				break;					//Just in case
			}
		}
	}
	FCEUI_CreateMovieFile(backupFn);

	//TODO, decide if fstream successfully opened the file and print error message if it doesn't

	if (dispMessage)	//If we should inform the user
	{
		if (overflow)
			FCEU_DispMessage("Backup overflow, overwriting %s",0,backupFn.c_str()); //Inform user of overflow
		else
			FCEU_DispMessage("%s created",0,backupFn.c_str()); //Inform user of backup filename
	}
}

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.4-extension: per-frame PLAY branch
// dispatcher. The main FCEUMOV_AddInputState() in movie.cpp forwards
// to this when movieMode == MOVIEMODE_PLAY.
// ----------------------------------------------------------------------------
void MovieAddInputState_Playback()
{
	//stop when we run out of frames
	if (currFrameCounter >= (int)currMovieData.records.size())
	{
		FinishPlayback();
		//tell all drivers to poll input and set up their logical states
		for(int port=0;port<2;port++)
			joyports[port].driver->Update(port,joyports[port].ptr,joyports[port].attrib);
		portFC.driver->Update(portFC.ptr,portFC.attrib);
	} else
	{
		MovieRecord* mr = &currMovieData.records[currFrameCounter];

		//reset and power cycle if necessary
		if(mr->command_power())
			PowerNES();
		if(mr->command_reset())
			ResetNES();
		if(mr->command_fds_insert())
			FCEU_FDSInsert();
		if(mr->command_fds_select())
			FCEU_FDSSelect();
		if (mr->command_vs_insertcoin())
			FCEU_VSUniCoin(0);
		if (mr->command_vs_insertcoin2())
			FCEU_VSUniCoin(1);
		if (mr->command_vs_service())
			FCEU_VSUniService();

		joyports[0].load(mr);
		joyports[1].load(mr);
	}

	//if we are on the last frame, then pause the emulator if the player requested it
	if ( static_cast<size_t>(currFrameCounter) == currMovieData.records.size()-1)
	{
		if(FCEUD_PauseAfterPlayback())
		{
			fceu11::ToggleEmulationPause();
		}
	}

	//pause the movie at a specified frame
	if (FCEUMOV_ShouldPause() && fceu11::IsEmulationPaused()==0)
	{
		fceu11::ToggleEmulationPause();
		FCEU_DispMessage("Paused at specified movie frame",0);
	}
}
