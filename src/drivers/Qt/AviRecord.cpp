/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
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
//
// AviRecord.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <stdint.h>
#include <stdarg.h>
#include <string.h>
#include <string>
#include <memory>

#include <QFile>
#include <QDate>
#include <QLocale>
#include <QSysInfo>
#include <QObject>
#include <QHeaderView>
#include <QMessageBox>
#include <QInputDialog>

#include "fceu.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "version.h"
#include "common/os_utils.h"

#include "Qt/AviRecord.h"
#include "Qt/avi/gwavi.h"
#include "Qt/nes_shm.h"
#include "Qt/throttle.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/fceuWrapper.h"

#include "Qt/AviVideoCodec.h"
#include "Qt/AviAudioCodec.h"

gwavi_t  *gwavi = NULL;
bool      recordEnable = false;
bool      recordAudio  = true;
int       vbufHead = 0;
int       vbufTail = 0;
int       vbufSize = 0;
int       abufHead = 0;
int       abufTail = 0;
int       abufSize = 0;
// v1.13 Purify F3a: std::unique_ptr<T[]> RAII (was raw malloc/free)
std::unique_ptr<uint32_t[]> rawVideoBuf;
std::unique_ptr<int16_t[]>   rawAudioBuf;
int       aviDriver = 0;
int       videoFormat = AVI_RGB24;
int       audioSampleRate = 48000;
FILE     *avLogFp = NULL;

//**************************************************************************************
#ifdef _USE_LIBAV

namespace LIBAV
{

int setCodecFromConfig(void)
{
	std::string s;
	const AVCodec *c;
#if  LIBAVFORMAT_VERSION_INT >= AV_VERSION_INT( 59, 0, 0 )
	const AVOutputFormat *fmt;
#else
	AVOutputFormat *fmt;
#endif

	fmt = av_guess_format("avi", NULL, NULL);

	g_config->getOption("SDL.AviFFmpegVideoCodec", &s);

	if ( s.size() > 0 )
	{
		c = avcodec_find_encoder_by_name(s.c_str());

		if ( c )
		{
			video_st.selEnc = c->name;
		}
	}

	g_config->getOption("SDL.AviFFmpegAudioCodec", &s);

	if ( s.size() > 0 )
	{
		c = avcodec_find_encoder_by_name(s.c_str());

		if ( c )
		{
			audio_st.selEnc = c->name;
		}
	}

	if ( fmt )
	{
		if ( video_st.selEnc.size() == 0 )
		{
			c = avcodec_find_encoder( fmt->video_codec );

			if ( c )
			{
				video_st.selEnc = c->name;
			}
		}
		if ( audio_st.selEnc.size() == 0 )
		{
			c = avcodec_find_encoder( fmt->audio_codec );

			if ( c )
			{
				audio_st.selEnc = c->name;
			}
		}
	}

	g_config->getOption("SDL.AviFFmpegVideoPixFmt"    , &video_st.pixelFormat);
	g_config->getOption("SDL.AviFFmpegAudioSmpFmt"    , &audio_st.sampleFormat);
	g_config->getOption("SDL.AviFFmpegAudioSmpRate"   , &audio_st.sampleRate);
	g_config->getOption("SDL.AviFFmpegAudioChanLayout", &audio_st.chanLayout);

	return 0;
}

int initMedia( const char *filename )
{
#if  LIBAVFORMAT_VERSION_INT >= AV_VERSION_INT( 59, 0, 0 )
	const AVOutputFormat *fmt;
#else
	AVOutputFormat *fmt;
#endif

	av_log_set_callback( log_callback );

	fmt = av_guess_format(NULL, filename, NULL);

	if (fmt == NULL)
	{
		fprintf( avLogFp, "Could not deduce output format from file extension: using AVI.\n");
		fmt = av_guess_format("avi", NULL, NULL);
	}
	if (fmt == NULL)
	{
		fprintf( avLogFp, "Error: Could not find suitable output format\n");
		goto LIBAV_INIT_MEDIA_ERROR_EXIT;
	}

	oc = avformat_alloc_context();
	if (oc == NULL)
	{
		fprintf( avLogFp, "Memory error: avformat_alloc_context failed\n");
		goto LIBAV_INIT_MEDIA_ERROR_EXIT;
	}
	oc->oformat = fmt;

	setCodecFromConfig();

	if ( initVideoStream( video_st.selEnc.c_str(), &video_st ) )
	{
		fprintf( avLogFp, "Video Stream Init Failed\n");
		goto LIBAV_INIT_MEDIA_ERROR_EXIT;
	}
	if ( recordAudio )
	{
		if ( initAudioStream( audio_st.selEnc.c_str(), &audio_st ) )
		{
			fprintf( avLogFp, "Audio Stream Init Failed\n");
			goto LIBAV_INIT_MEDIA_ERROR_EXIT;
		}
	}

	av_dump_format(oc, 0, filename, 1);

	if ( !(fmt->flags & AVFMT_NOFILE))
	{
		if (avio_open( &oc->pb, filename, AVIO_FLAG_WRITE) < 0)
		{
			fprintf( avLogFp, "Error: Could not open file: '%s'\n", filename);
			goto LIBAV_INIT_MEDIA_ERROR_EXIT;
		}
		else
		{
			fprintf( avLogFp, "Opened file for writing: %s\n", filename);
		}
	}

	for ( auto it = avi_info.kvmap.begin(); it != avi_info.kvmap.end(); it++)
	{
		av_dict_set( &oc->metadata, it->first.c_str(), it->second.c_str(), 0 );
	}

	if ( avformat_write_header(oc, NULL) )
	{
		fprintf( avLogFp, "Error: avformat_write_header Failed: Could not write avformat header\n");
		goto LIBAV_INIT_MEDIA_ERROR_EXIT;
	}

	return 0;

LIBAV_INIT_MEDIA_ERROR_EXIT:

	video_st.close();
	audio_st.close();

	if ( oc )
	{
		avio_close(oc->pb);

		avformat_free_context(oc); oc = NULL;
	}
	return -1;
}

int close(void)
{
	encode_audio_frame( NULL, 0 );

	av_write_trailer(oc);

	video_st.close();
	audio_st.close();

	avio_close(oc->pb);

	avformat_free_context(oc);

	oc = NULL;

	return 0;
}

} // End namespace LIBAV
#endif
//**************************************************************************************
int aviRecordInit(void)
{
	g_config->getOption("SDL.AviDriver", &aviDriver);
	g_config->getOption("SDL.AviVideoFormat", &videoFormat);
	g_config->getOption("SDL.AviRecordAudio", &recordAudio);
	g_config->getOption("SDL.Sound.Rate", &audioSampleRate);

#ifdef _USE_LIBAV
	if ( videoFormat == AVI_LIBAV )
	{
		aviSetSelVideoFormat( AVI_RGB24 );
	}
	LIBAV::setCodecFromConfig();
#endif
	return 0;
}
//**************************************************************************************
int aviRecordLogClose(void)
{
	if ( avLogFp != NULL )
	{
		if ( avLogFp != stdout )
		{
			fclose(avLogFp);
		}
		avLogFp = NULL;
	}
	return 0;
}
//**************************************************************************************
int aviRecordLogOpen(void)
{
	if ( avLogFp != NULL )
	{
		aviRecordLogClose();
	}
	if ( avLogFp == NULL )
	{
		avLogFp = fopen( AV_LOG_FILE_NAME, "w");

		if ( avLogFp == NULL )
		{
			char msg[512];
			snprintf( msg, sizeof(msg), "Error: Failed to open AV Recording log file for writing: %s\n", AV_LOG_FILE_NAME);
			FCEUD_PrintError(msg);
			avLogFp = stdout;
		}
	}
	return avLogFp == NULL;
}
//**************************************************************************************
int aviRecordOpenFile( const char *filepath )
{
	QDate date;
	QLocale locale;
	char fourcc[8];
	gwavi_audio_t  audioConfig;
	double fps;
	std::string fileName;
	char txt[512];
	const char *romFile;

	if ( aviRecordLogOpen() )
	{
		return -1;
	}
	g_config->getOption("SDL.AviRecordAudio", &recordAudio);

	if ( filepath != NULL )
	{
		fileName.assign( filepath );
	}
	else
	{

		romFile = getRomFile();

		if ( romFile )
		{
			char base[512];
			const char *baseDir = fceu11::GetBaseDirectory();
			std::string lastPath;

			getFileBaseName( romFile, base );

			g_config->getOption ("SDL.AviFilePath", &lastPath);

			if ( lastPath.size() > 0 )
			{
				fileName.assign( lastPath.c_str() );
				fileName.append( "/" );
			}
			else if ( baseDir )
			{
				fileName.assign( baseDir );
				fileName.append( "/avi/" );
			}
			else
			{
				fileName.clear();
			}
			fileName.append( base );
			fileName.append(".avi");
		}
		else
		{
			return -1;
		}
	}

	if ( fileName.size() > 0 )
	{
		QFile file(fileName.c_str());

		if ( file.exists() )
		{
			int ret;
			std::string msg;

			msg = "Pre-existing AVI file will be overwritten:\n\n" +
				fileName +	"\n\nReplace file?";

			ret = QMessageBox::warning( consoleWindow, QObject::tr("Overwrite Warning"),
					QString::fromStdString(msg), QMessageBox::Yes | QMessageBox::No );

			if ( ret == QMessageBox::No )
			{
				return -1;
			}
		}
	}

	date = QDate::currentDate();

	avi_info.add_pair( "ICRD", date.toString(Qt::ISODate).toStdString().c_str() );

	avi_info.add_pair( "ILNG", QLocale::languageToString( locale.language() ).toStdString().c_str() );

	avi_info.add_pair( "IARL", QLocale::countryToString( locale.country() ).toStdString().c_str() );

	avi_info.add_pair( "IMED", QSysInfo::prettyProductName().toStdString().c_str() );

	snprintf( txt, sizeof(txt), "FCEUX %s", FCEU_VERSION_STRING );
	avi_info.add_pair( "ITCH", txt );

	romFile = getRomFile();

	if ( romFile )
	{
		getFileBaseName( romFile, txt );

		if ( txt[0] != 0 )
		{
			avi_info.add_pair( "ISRC", txt );
		}
		if ( GameInfo )
		{
			avi_info.add_pair( "ISRF", md5_asciistr(GameInfo->MD5) );
		}
	}

	g_config->getOption("SDL.AviVideoFormat", &videoFormat);

#ifdef WIN32
	if ( (aviDriver == AVI_DRIVER_LIBGWAVI) && (videoFormat == AVI_VFW) )
	{
		if ( VFW::chooseConfig( nes_shm->video.ncol, nes_shm->video.nrow ) )
		{
			return -1;
		}
	}
#endif

	if ( gwavi != NULL )
	{
		delete gwavi; gwavi = NULL;
	}
	fps = getBaseFrameRate();

	g_config->getOption("SDL.Sound.Rate", &audioSampleRate);

	audioConfig.channels = 1;
	audioConfig.bits     = 16;
	audioConfig.samples_per_second = audioSampleRate;

	memset( fourcc, 0, sizeof(fourcc) );

	if ( videoFormat == AVI_I420 )
	{
		FCEU_strlcpy( fourcc, sizeof(fourcc), "I420");
	}
	#ifdef _USE_X264
	else if ( videoFormat == AVI_X264 )
	{
		FCEU_strlcpy( fourcc, sizeof(fourcc), "X264");
	}
	#endif
	#ifdef _USE_X265
	else if ( videoFormat == AVI_X265 )
	{
		FCEU_strlcpy( fourcc, sizeof(fourcc), "H265");
	}
	#endif
	#ifdef WIN32
	else if ( videoFormat == AVI_VFW )
	{
		memcpy( fourcc, &VFW::cmpvars.fccHandler, 4);
		for (int i=0; i<4; i++)
		{
			fourcc[i] = toupper(fourcc[i]);
		}
	}
	#endif

#ifdef _USE_LIBAV
	if ( aviDriver == AVI_DRIVER_LIBAV )
	{
		if ( LIBAV::initMedia( fileName.c_str() ) )
		{
			char msg[512];
			fprintf( avLogFp, "Error: Failed to open AVI file.\n");
			recordEnable = false;
			snprintf( msg, sizeof(msg), "Error: AV Recording Initialization Failed.\nSee %s for details...\n", AV_LOG_FILE_NAME);
			FCEUD_PrintError(msg);
			return -1;
		}
	}
	else
#endif
	{
		gwavi = new gwavi_t();

		if ( gwavi->open( fileName.c_str(), nes_shm->video.ncol, nes_shm->video.nrow, fourcc, fps, recordAudio ? &audioConfig : NULL ) )
		{
			char msg[512];
			fprintf( avLogFp, "Error: Failed to open AVI file.\n");
			recordEnable = false;
			snprintf( msg, sizeof(msg), "Error: AV Recording Initialization Failed.\nSee %s for details...\n", AV_LOG_FILE_NAME);
			FCEUD_PrintError(msg);
			return -1;
		}
	}

	vbufSize    = 1024 * 1024 * 60;
	rawVideoBuf = std::make_unique<uint32_t[]>(vbufSize);

	abufSize    = 96000;
	rawAudioBuf = std::make_unique<int16_t[]>(abufSize);

	vbufHead = 0;
	vbufTail = 0;
	abufHead = 0;
	abufTail = 0;

	recordEnable = true;
	return 0;
}
//**************************************************************************************
int aviRecordAddFrame( void )
{
	if ( !recordEnable )
	{
		return -1;
	}

	if ( fceu11::IsEmulationPaused() )
	{
		return 0;
	}

	int i, head, numPixels, availSize;

	numPixels  = nes_shm->video.ncol * nes_shm->video.nrow;

	availSize = (vbufTail - vbufHead);
	if ( availSize <= 0 )
	{
		availSize += vbufSize;
	}

	while ( numPixels > availSize )
	{
		msleep(1);

		availSize = (vbufTail - vbufHead);
		if ( availSize <= 0 )
		{
			availSize += vbufSize;
		}
	}

	i = 0; head = vbufHead;

	while ( i < numPixels )
	{
		rawVideoBuf[ head ] = nes_shm->avibuf[i]; i++;

		head = (head + 1) % vbufSize;
	}
	vbufHead = head;

	return 0;
}
//**************************************************************************************
int aviRecordAddAudioFrame( int32_t *buf, int numSamples )
{
	if ( !recordEnable )
	{
		return -1;
	}

	if ( !recordAudio )
	{
		return -1;
	}

	for (int i=0; i<numSamples; i++)
	{
		rawAudioBuf[ abufHead ] = buf[i];

		abufHead = (abufHead + 1) % abufSize;
	}

	return 0;
}
//**************************************************************************************
int aviRecordClose(void)
{
	recordEnable = false;

	if ( gwavi != NULL )
	{
		gwavi->close();

		delete gwavi; gwavi = NULL;
	}

	// v1.13 Purify F3a: std::unique_ptr<T[]> RAII; .reset() replaces free+NULL
	rawVideoBuf.reset();
	rawAudioBuf.reset();
	vbufTail = abufTail = 0;
	vbufSize = abufSize = 0;

	return 0;
}
//**************************************************************************************
bool aviGetAudioEnable(void)
{
	return recordAudio;
}
//**************************************************************************************
void aviSetAudioEnable(bool val)
{
	recordAudio = val;

	g_config->setOption("SDL.AviRecordAudio", val);
}
//**************************************************************************************
bool aviRecordRunning(void)
{
	return recordEnable;
}
bool fceu11::AviIsRecording()
{
	return recordEnable;
}
//**************************************************************************************
void fceWrapper_AviRecordTo(void)
{
	return;
}
//**************************************************************************************
void fceWrapper_AviStop(void)
{
	return;
}
//**************************************************************************************
int aviGetSelDriver(void)
{
	return aviDriver;
}
//**************************************************************************************
void aviSetSelDriver(int idx)
{
	aviDriver = idx;

	g_config->setOption("SDL.AviDriver", aviDriver);
}
//**************************************************************************************
int aviGetSelVideoFormat(void)
{
	return videoFormat;
}
//**************************************************************************************
void aviSetSelVideoFormat(int idx)
{
	videoFormat = idx;

	g_config->setOption("SDL.AviVideoFormat", videoFormat);
}
//**************************************************************************************
int FCEUD_AviGetDriverList( std::vector <std::string> &formatList )
{
	std::string s;

	for (int i=0; i<AVI_NUM_DRIVERS; i++)
	{
		switch ( i )
		{
			default:
				s.assign("Unknown");
			break;
			case AVI_DRIVER_LIBGWAVI:
				s.assign("libgwavi");
			break;
			#ifdef _USE_LIBAV
			case AVI_DRIVER_LIBAV:
				s.assign("libav (ffmpeg)");
			break;
			#endif
		}
		formatList.push_back(s);
	}
	return AVI_NUM_DRIVERS;
}
//**************************************************************************************
int FCEUD_AviGetFormatOpts( std::vector <std::string> &formatList )
{
	std::string s;

	for (int i=0; i<AVI_NUM_ENC; i++)
	{
		switch ( i )
		{
			default:
				s.assign("Unknown");
			break;
			case AVI_RGB24:
				s.assign("RGB24 (Uncompressed)");
			break;
			case AVI_I420:
				s.assign("I420 (YUV 4:2:0)");
			break;
			#ifdef _USE_X264
			case AVI_X264:
				s.assign("X264 (H.264)");
			break;
			#endif
			#ifdef _USE_X265
			case AVI_X265:
				s.assign("X265 (H.265)");
			break;
			#endif
			#ifdef _USE_LIBAV
			case AVI_LIBAV:
				s.assign("libav (ffmpeg)");
			break;
			#endif
			#ifdef WIN32
			case AVI_VFW:
				s.assign("VfW (Video for Windows)");
			break;
			#endif
		}
		formatList.push_back(s);
	}
	return AVI_NUM_ENC;
}
//**************************************************************************************
//**************************************************************************************
//*****************************  Options Pages *****************************************
//**************************************************************************************
//**************************************************************************************
