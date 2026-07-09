// AviVideoCodec.h
//

#pragma once

#include <stdint.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#ifdef WIN32
#include <windows.h>
#include <vfw.h>
#endif

#include <string>

#ifdef _USE_X264
#include "x264.h"
#endif
#ifdef _USE_X265
#include "x265.h"
#endif
#ifdef _USE_LIBAV
#ifdef __cplusplus
extern "C"
{
#include "libavutil/opt.h"
#include "libavutil/pixdesc.h"
#include "libavformat/avformat.h"
#include "libavcodec/avcodec.h"
#include "libswscale/swscale.h"
#include "libswresample/swresample.h"
}
#endif
#endif

#include "Qt/avi/gwavi.h"

#define  AV_LOG_FILE_NAME  "fceuxAV.log"

extern gwavi_t *gwavi;
extern FILE    *avLogFp;
extern int      audioSampleRate;

extern bool      recordEnable;
extern bool      recordAudio;
extern int       vbufHead;
extern int       vbufTail;
extern int       vbufSize;
extern int       abufHead;
extern int       abufTail;
extern int       abufSize;
extern uint32_t *rawVideoBuf;
extern int16_t  *rawAudioBuf;
extern int       aviDriver;
extern int       videoFormat;

void convertRgb_32_to_24(const unsigned char *src, unsigned char *dest, int w, int h, int nPix, bool verticalFlip);

#define  RGB2YUV_SHIFT  15

#define BY ((int)(0.114 * 219 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define BV (-(int)(0.081 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define BU ((int)(0.500 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define GY ((int)(0.587 * 219 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define GV (-(int)(0.419 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define GU (-(int)(0.331 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define RY ((int)(0.299 * 219 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define RV ((int)(0.500 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))
#define RU (-(int)(0.169 * 224 / 255 * (1 << RGB2YUV_SHIFT) + 0.5))

template<int PixStride>
static void Convert_4byte_To_I420Frame(const void* data, unsigned char* dest, unsigned npixels, unsigned width)
{
	const unsigned char* src = (const unsigned char*)data;
	unsigned height = npixels / width;

	unsigned pos = 0;
	unsigned ypos = 0;
	unsigned vpos = npixels;
	unsigned upos = vpos + npixels / 4;
	unsigned stride = width * PixStride;
	int Y, U, V;

	static const int Y_ADD = 16;
	static const int U_ADD = 128;
	static const int V_ADD = 128;

	for (unsigned y = 0; y < height; y += 2)
	{
		for (unsigned x = 0; x < width; x += 2)
		{
			{
				int c[3], rgb[3][4];

				for (int n = 0; n < 3; ++n) c[n] = rgb[n][0] = src[pos + n];
				for (int n = 0; n < 3; ++n) c[n] += rgb[n][1] = src[pos + n + stride];
				pos += PixStride;

				for (int n = 0; n < 3; ++n) c[n] += rgb[n][2] = src[pos + n];
				for (int n = 0; n < 3; ++n) c[n] += rgb[n][3] = src[pos + n + stride];
				pos += PixStride;

				unsigned destpos[4] = { ypos, ypos + width, ypos + 1, ypos + width + 1 };
				for (int n = 0; n < 4; ++n)
				{
					Y = Y_ADD + ((RY * rgb[0][n]
						+ GY * rgb[1][n]
						+ BY * rgb[2][n]
						) >> RGB2YUV_SHIFT);

					dest[destpos[n]] = Y;
				}

				U = (U_ADD + ((RU * c[0] + GU * c[1] + BU * c[2]) >> (RGB2YUV_SHIFT + 2)));
				V = (V_ADD + ((RV * c[0] + GV * c[1] + BV * c[2]) >> (RGB2YUV_SHIFT + 2)));

				dest[upos++] = U;
				dest[vpos++] = V;
			}

			ypos += 2;
		}
		pos += stride;
		ypos += width;
	}
}

#ifdef _USE_X264
namespace X264
{
int init(int width, int height);
int encode_frame(unsigned char *inBuf, int width, int height);
int close(void);
}
#endif

#ifdef _USE_X265
namespace X265
{
int init(int width, int height);
int encode_frame(unsigned char *inBuf, int width, int height);
int close(void);
}
#endif

#ifdef WIN32
namespace VFW
{
extern COMPVARS cmpvars;
int chooseConfig(int width, int height);
int init(int width, int height);
int close(void);
int encode_frame(unsigned char *inBuf, int width, int height);
}
#endif

#ifdef _USE_LIBAV

struct OutputStream
{
	AVStream  *st;
	AVCodecContext *enc;
	AVFrame *frame;
	AVFrame *tmp_frame;
	AVPacket *pkt;
	struct SwsContext *sws_ctx;
	struct SwrContext *swr_ctx;
	int64_t next_pts;
	int      bytesPerSample;
	int      frameSize;
	int      pixelFormat;
	int      sampleFormat;
	int      sampleRate;
	int      chanLayout;
	bool     isAudio;
	bool     writeError;
	std::string selEnc;

	OutputStream(void);
	void close(void);
};

namespace LIBAV
{
extern AVFormatContext *oc;
extern OutputStream  video_st;
extern OutputStream  audio_st;

void log_callback(void *avcl, int level, const char *fmt, va_list vl);
int loadCodecConfig(int type, const char *codec_name, AVCodecContext *ctx);
int saveCodecConfig(int type, const char *codec_name, AVCodecContext *ctx);

AVFrame *alloc_picture(enum AVPixelFormat pix_fmt, int width, int height);
int initVideoStream(const char *codec_name, OutputStream *ost);

int encode_video_frame(unsigned char *inBuf);
int init(int width, int height);
int close(void);

int setCodecFromConfig(void);
int initMedia(const char *filename);
}

#endif
