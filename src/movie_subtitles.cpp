// movie_subtitles.cpp
//
// v1.13 Phase B / Batch D-D.1: subtitle trio relocated from src/movie.cpp
// (which was 1214 lines after B+C; this carve-out brings it well below the
// 300-line target).
//
// Owns:
//   - LoadSubtitles / ProcessSubtitles / FCEU_DisplaySubtitles (3 funcs)
//   - subtitleFrames / subtitleMessages (file-static, only this TU uses them)
//   - subtitlesOnAVI (file-scope; exposed via movie.h:280 `extern`)
//
// v1.12 Scissors carryover style: pure code move, zero behaviour change.

#include "movie.h"
#include "driver_callbacks.h"
#include "video.h"   // for `extern GUIMESSAGE subtitleMessage;`

#include <cstdarg>
#include <cstdio>
#include <vector>
#include <string>

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.1: Subtitles — file-scope state.
// `subtitleFrames` and `subtitleMessages` MUST stay at file scope (not
// anonymous namespace) because fceuWrapper.cpp (drivers/Qt side) reads
// them via `extern std::vector<int> subtitleFrames;` (lines 844-845) for
// the SRT-rip helper. Promoting them here is a no-op for the rest of the
// movie subsystem — only fceuWrapper.cpp ever cross-references them.
// ----------------------------------------------------------------------------
std::vector<int>    subtitleFrames;      // Frame numbers for subtitle messages
std::vector<std::string> subtitleMessages;// Messages of subtitles

// movie.h:280 exposes this via `extern bool subtitlesOnAVI;`.
bool subtitlesOnAVI = false;

//This function creates an array of frame numbers and corresponding strings for displaying subtitles
void LoadSubtitles(MovieData &moviedata)
{
	subtitleFrames.resize(0);
	subtitleMessages.resize(0);
	extern std::vector<std::string> subtitles;
	for(uint32 i=0; i < moviedata.subtitles.size() ; i++)
	{
		std::string& subtitle = moviedata.subtitles[i];
		size_t splitat = subtitle.find_first_of(' ');
		std::string key, value;

		//If we can't split them, then don't process this one
		if(splitat == std::string::npos)
		{
		}
		//Else split the subtitle into the int and string arrays
		else
		{
			key = subtitle.substr(0,splitat);
			value = subtitle.substr(splitat+1);
			subtitleFrames.push_back(atoi(key.c_str()));
			subtitleMessages.push_back(value);
		}
	}

}

//Every frame, this will be called to determine if a subtitle should be displayed, which one, and then to display it
void ProcessSubtitles(void)
{
	if (movieMode == MOVIEMODE_INACTIVE) return;

	for(uint32 i=0;i<currMovieData.subtitles.size();i++)
	{
		if (currFrameCounter == subtitleFrames[i])
			FCEU_DisplaySubtitles("%s",subtitleMessages[i].c_str());
	}
}

void FCEU_DisplaySubtitles(const char *format, ...)
{
	va_list ap;

	va_start(ap,format);
	vsnprintf(subtitleMessage.errmsg,sizeof(subtitleMessage.errmsg),format,ap);
	va_end(ap);

	subtitleMessage.howlong = 400;
	subtitleMessage.isMovieMessage = subtitlesOnAVI;
	subtitleMessage.linesFromBottom = 0;
}
