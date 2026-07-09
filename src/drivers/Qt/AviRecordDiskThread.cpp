// AviRecordDiskThread.cpp
//

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#include <QThread>

#include "fceu.h"
#include "Qt/AviRecord.h"
#include "Qt/AviVideoCodec.h"
#include "Qt/AviAudioCodec.h"
#include "Qt/nes_shm.h"
#include "Qt/throttle.h"

AviRecordDiskThread_t::AviRecordDiskThread_t( QObject *parent )
	: QThread(parent)
{
	setObjectName( QString("AviRecordDiskThread") );
}

AviRecordDiskThread_t::~AviRecordDiskThread_t(void)
{

}

void AviRecordDiskThread_t::run(void)
{
	int numPixels, width, height, numPixelsReady = 0;
	int numSamples = 0;
	double fps = 60.0;
	unsigned char *rgb24;
	int16_t *audioOut;
	uint32_t *videoOut;
	char writeAudio = 1;
	char localRecordAudio = 0;
	int  avgAudioPerFrame, audioChunkSize, audioSamplesAvail=0;
	int  localVideoFormat;

	fprintf( avLogFp, "AVI Record Disk Thread Start\n");

	setPriority( QThread::HighestPriority );

	fps = getBaseFrameRate();

	avgAudioPerFrame = ( audioSampleRate / fps) + 1;
	audioChunkSize   = ( audioSampleRate / 4 );

	fprintf( avLogFp, "Avg Audio Sample Rate per Frame: %i \n", avgAudioPerFrame );

	width     = nes_shm->video.ncol;
	height    = nes_shm->video.nrow;
	numPixels = width * height;

	rgb24 = (unsigned char *)malloc( numPixels * sizeof(uint32_t) );

	if ( rgb24 )
	{
		memset( rgb24, 0, numPixels * sizeof(uint32_t) );
	}
	else
	{
		return;
	}
#ifdef _USE_LIBAV
	if ( aviDriver == AVI_DRIVER_LIBAV )
	{
		localVideoFormat = AVI_LIBAV;
	}
	else
#endif
	{
		localVideoFormat = videoFormat;
	}
	localRecordAudio = recordAudio;

#ifdef _USE_X264
	if ( localVideoFormat == AVI_X264)
	{
		X264::init( width, height );
	}
#endif
#ifdef _USE_X265
	if ( localVideoFormat == AVI_X265)
	{
		X265::init( width, height );
	}
#endif
#ifdef _USE_LIBAV
	if ( localVideoFormat == AVI_LIBAV)
	{
		LIBAV::init( width, height );

		audioChunkSize = avgAudioPerFrame;
	}
#endif
#ifdef WIN32
	if ( localVideoFormat == AVI_VFW)
	{
		VFW::init( width, height );
	}
#endif

	audioOut = (int16_t *)malloc(96000);
	videoOut = (uint32_t*)malloc(1048576);

	while ( !isInterruptionRequested() )
	{

		while ( (numPixelsReady < numPixels) && (vbufTail != vbufHead) )
		{
			videoOut[ numPixelsReady ] = rawVideoBuf[ vbufTail ]; numPixelsReady++;

			vbufTail = (vbufTail + 1) % vbufSize;
		}

		if ( numPixelsReady >= numPixels )
		{

			writeAudio = 1;

			if ( localVideoFormat == AVI_I420)
			{
				Convert_4byte_To_I420Frame<4>(videoOut,rgb24,numPixels,width);
				gwavi->add_frame( rgb24, (numPixels*3)/2 );
			}
			#ifdef _USE_X264
			else if ( localVideoFormat == AVI_X264)
			{
				Convert_4byte_To_I420Frame<4>(videoOut,rgb24,numPixels,width);
				X264::encode_frame( rgb24, width, height );
			}
			#endif
			#ifdef _USE_X265
			else if ( localVideoFormat == AVI_X265)
			{
				Convert_4byte_To_I420Frame<4>(videoOut,rgb24,numPixels,width);
				X265::encode_frame( rgb24, width, height );
			}
			#endif
			#ifdef WIN32
			else if ( localVideoFormat == AVI_VFW)
			{
				convertRgb_32_to_24( (const unsigned char*)videoOut, rgb24,
						width, height, numPixels, true );
				VFW::encode_frame( rgb24, width, height );
			}
			#endif
			#ifdef _USE_LIBAV
			else if ( localVideoFormat == AVI_LIBAV)
			{
				LIBAV::encode_video_frame( (unsigned char*)videoOut );
			}
			#endif
			else
			{
				convertRgb_32_to_24( (const unsigned char*)videoOut, rgb24,
						width, height, numPixels, true );
				gwavi->add_frame( rgb24, numPixels*3 );
			}

			numPixelsReady = 0;

			audioSamplesAvail = abufHead - abufTail;

			if ( audioSamplesAvail < 0 )
			{
				audioSamplesAvail += abufSize;
			}
			writeAudio = (audioSamplesAvail >= audioChunkSize);

			if ( writeAudio && localRecordAudio )
			{
				numSamples = 0;

				while ( abufHead != abufTail )
				{
					audioOut[ numSamples ] = rawAudioBuf[ abufTail ]; numSamples++;

					abufTail = (abufTail + 1) % abufSize;

					if ( numSamples >= audioChunkSize )
					{
						break;
					}
				}

				if ( numSamples > 0 )
				{
					#ifdef _USE_LIBAV
					if ( localVideoFormat == AVI_LIBAV)
					{
						LIBAV::encode_audio_frame( audioOut, numSamples );
					}
					else
					#endif
					{
						gwavi->add_audio( (unsigned char *)audioOut, numSamples*2);
					}

					numSamples = 0;
				}
			}
		}
		else
		{
			msleep(1);
		}
	}

	audioSamplesAvail = abufHead - abufTail;

	if ( audioSamplesAvail < 0 )
	{
		audioSamplesAvail += abufSize;
	}
	writeAudio = (audioSamplesAvail > 0);

	if ( writeAudio && localRecordAudio )
	{
		numSamples = 0;

		while ( abufHead != abufTail )
		{
			audioOut[ numSamples ] = rawAudioBuf[ abufTail ]; numSamples++;

			abufTail = (abufTail + 1) % abufSize;
		}

		if ( numSamples > 0 )
		{
			#ifdef _USE_LIBAV
			if ( localVideoFormat == AVI_LIBAV)
			{
				LIBAV::encode_audio_frame( audioOut, numSamples );
			}
			else
			#endif
			{
				gwavi->add_audio( (unsigned char *)audioOut, numSamples*2);
			}

			numSamples = 0;
		}
	}

	free(rgb24);

#ifdef _USE_X264
	if ( localVideoFormat == AVI_X264)
	{
		X264::close();
	}
#endif
#ifdef _USE_X265
	if ( localVideoFormat == AVI_X265)
	{
		X265::close();
	}
#endif
#ifdef _USE_LIBAV
	if ( localVideoFormat == AVI_LIBAV)
	{
		LIBAV::close();
	}
#endif
#ifdef WIN32
	if ( localVideoFormat == AVI_VFW)
	{
		VFW::close();
	}
#endif
	aviRecordClose();

	free(audioOut);
	free(videoOut);

	fprintf( avLogFp, "AVI Record Disk Thread Exit\n");
	emit finished();

	if ( avLogFp != NULL )
	{
		if ( avLogFp != stdout )
		{
			fclose(avLogFp);
		}
		avLogFp = NULL;
	}
}
