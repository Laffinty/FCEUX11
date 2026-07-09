// AviVideoCodec.cpp
//

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fceu.h"
#include "common/os_utils.h"
#include "Qt/nes_shm.h"
#include "Qt/throttle.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/fceuWrapper.h"
#include "diag_api.h"
#include "utils/safe_string.h"

#include "Qt/AviVideoCodec.h"

void convertRgb_32_to_24(const unsigned char *src, unsigned char *dest, int w, int h, int nPix, bool verticalFlip)
{
	int i = 0, j = 0, x, y;

	if (verticalFlip)
	{
		y = h - 1;

		while (y >= 0)
		{
			x = 0;
			i = y * w * 4;

			while (x < w)
			{
				dest[j] = src[i]; i++; j++;
				dest[j] = src[i]; i++; j++;
				dest[j] = src[i]; i++; j++;
				i++;
				x++;
			}
			y--;
		}
	}
	else
	{
		int size;

		size = nPix * 4;
		i = 0;
		while (i < size)
		{
			dest[j] = src[i]; i++; j++;
			dest[j] = src[i]; i++; j++;
			dest[j] = src[i]; i++; j++;
			i++;
		}
	}
}

#ifdef _USE_X264

namespace X264
{
static x264_param_t param;
static x264_picture_t pic;
static x264_picture_t pic_out;
static x264_t *hdl = NULL;
static x264_nal_t *nal = NULL;
static int i_nal = 0;
static int i_frame = 0;

int init(int width, int height)
{
	if (x264_param_default_preset(&param, "medium", NULL) < 0)
	{
		goto x264_init_fail;
	}

	param.i_bitdepth = 8;
	param.i_csp = X264_CSP_I420;
	param.i_width = width;
	param.i_height = height;
	param.b_vfr_input = 0;
	param.b_repeat_headers = 1;
	param.b_annexb = 1;

	if (x264_param_apply_profile(&param, "high") < 0)
	{
		goto x264_init_fail;
	}

	if (x264_picture_alloc(&pic, param.i_csp, param.i_width, param.i_height) < 0)
	{
		goto x264_init_fail;
	}

	hdl = x264_encoder_open(&param);
	if (hdl == NULL)
	{
		goto x264_init_fail;
	}
	i_frame = 0;

	return 0;

x264_init_fail:
	return -1;
}

int encode_frame(unsigned char *inBuf, int width, int height)
{
	int luma_size = width * height;
	int chroma_size = luma_size / 4;
	int i_frame_size = 0;
	int ofs;
	unsigned int flags = 0;

	ofs = 0;
	memcpy(pic.img.plane[0], &inBuf[ofs], luma_size);   ofs += luma_size;
	memcpy(pic.img.plane[1], &inBuf[ofs], chroma_size); ofs += chroma_size;
	memcpy(pic.img.plane[2], &inBuf[ofs], chroma_size); ofs += chroma_size;

	pic.i_pts = i_frame++;

	i_frame_size = x264_encoder_encode(hdl, &nal, &i_nal, &pic, &pic_out);

	if (i_frame_size < 0)
	{
		return -1;
	}
	else if (i_frame_size)
	{
		if (pic_out.b_keyframe)
		{
			flags |= gwavi_t::IF_KEYFRAME;
		}
		gwavi->add_frame(nal->p_payload, i_frame_size, flags);
	}
	return i_frame_size;
}

int close(void)
{
	int i_frame_size;
	unsigned int flags = 0;

	while (x264_encoder_delayed_frames(hdl))
	{
		i_frame_size = x264_encoder_encode(hdl, &nal, &i_nal, NULL, &pic_out);

		if (i_frame_size < 0)
		{
			break;
		}
		else if (i_frame_size)
		{
			flags = 0;

			if (pic_out.b_keyframe)
			{
				flags |= gwavi_t::IF_KEYFRAME;
			}
			gwavi->add_frame(nal->p_payload, i_frame_size, flags);
		}
	}

	x264_encoder_close(hdl);
	x264_picture_clean(&pic);

	return 0;
}

} // End X264 namespace
#endif

#ifdef _USE_X265

namespace X265
{
static x265_param *param = NULL;
static x265_picture *pic = NULL;
static x265_picture pic_out;
static x265_encoder *hdl = NULL;
static x265_nal *nal = NULL;
static unsigned int i_nal = 0;

int init(int width, int height)
{
	double fps;
	unsigned int usec;

	fps = getBaseFrameRate();

	usec = (unsigned int)((1000000.0 / fps) + 0.50);

	param = x265_param_alloc();

	x265_param_default(param);

	param->internalCsp = X265_CSP_I420;
	param->sourceWidth = width;
	param->sourceHeight = height;
	param->bRepeatHeaders = 1;
	param->fpsNum = 1000000;
	param->fpsDenom = usec;

	if ((pic = x265_picture_alloc()) == NULL)
	{
		goto x265_init_fail;
	}
	x265_picture_init(param, pic);

	hdl = x265_encoder_open(param);
	if (hdl == NULL)
	{
		goto x265_init_fail;
	}

	return 0;

x265_init_fail:
	return -1;
}

int encode_frame(unsigned char *inBuf, int width, int height)
{
	int luma_size = width * height;
	int chroma_size = luma_size / 4;
	int ret = 0;
	int ofs;
	unsigned int flags = 0, totalPayload = 0;

	ofs = 0;
	pic->planes[0] = &inBuf[ofs]; ofs += luma_size;
	pic->planes[1] = &inBuf[ofs]; ofs += chroma_size;
	pic->planes[2] = &inBuf[ofs]; ofs += chroma_size;
	pic->stride[0] = width;
	pic->stride[1] = width / 2;
	pic->stride[2] = width / 2;

	ret = x265_encoder_encode(hdl, &nal, &i_nal, pic, &pic_out);

	if (ret <= 0)
	{
		return -1;
	}
	else if (i_nal > 0)
	{
		flags = 0;
		totalPayload = 0;

		if (IS_X265_TYPE_I(pic_out.sliceType))
		{
			flags |= gwavi_t::IF_KEYFRAME;
		}

		for (unsigned int i = 0; i < i_nal; i++)
		{
			totalPayload += nal[i].sizeBytes;
		}
		gwavi->add_frame(nal[0].payload, totalPayload, flags);
	}
	return ret;
}

int close(void)
{
	int ret;
	unsigned int flags = 0, totalPayload = 0;

	while (hdl != NULL)
	{
		ret = x265_encoder_encode(hdl, &nal, &i_nal, NULL, &pic_out);

		if (ret <= 0)
		{
			break;
		}
		else if (i_nal > 0)
		{
			totalPayload = 0;
			flags = 0;

			if (IS_X265_TYPE_I(pic_out.sliceType))
			{
				flags |= gwavi_t::IF_KEYFRAME;
			}
			for (unsigned int i = 0; i < i_nal; i++)
			{
				totalPayload += nal[i].sizeBytes;
			}
			gwavi->add_frame(nal[0].payload, totalPayload, flags);
		}
	}

	x265_encoder_close(hdl);
	x265_picture_free(pic);
	x265_param_free(param);

	return 0;
}

} // End X265 namespace
#endif

#ifdef WIN32
namespace VFW
{
static bool cmpSet = false;
COMPVARS  cmpvars;
static BITMAPINFOHEADER   bmapIn;
static LPBITMAPINFOHEADER bmapOut = NULL;
static DWORD frameNum = 0;
static DWORD dwQuality = 0;
static DWORD icErrCount = 0;
static DWORD flagsOut = 0;
static LPVOID outBuf = NULL;

int chooseConfig(int width, int height)
{
	bool ret;
	char fccHandler[8];
	std::string fccHandlerString;

	if (cmpSet)
	{
		ICCompressorFree(&cmpvars);
		cmpSet = false;
	}
	memset(fccHandler, 0, sizeof(fccHandler));
	memset(&cmpvars, 0, sizeof(COMPVARS));
	cmpvars.cbSize = sizeof(COMPVARS);

	g_config->getOption("SDL.AviVfwFccHandler", &fccHandlerString);

	if (fccHandlerString.size() > 0)
	{
		FCEU_strlcpy(fccHandler, sizeof(fccHandler), fccHandlerString.c_str());
		memcpy(&cmpvars.fccHandler, fccHandler, 4);
		cmpvars.dwFlags = ICMF_COMPVARS_VALID;
	}

	ret = ICCompressorChoose(HWND(consoleWindow->winId()), ICMF_CHOOSE_ALLCOMPRESSORS,
		0, NULL, &cmpvars, 0);

	memcpy(fccHandler, &cmpvars.fccHandler, 4);
	fccHandler[4] = 0;

	printf("FCC:%08X  %c%c%c%c \n", cmpvars.fccHandler,
		(cmpvars.fccHandler & 0x000000FF),
		(cmpvars.fccHandler & 0x0000FF00) >> 8,
		(cmpvars.fccHandler & 0x00FF0000) >> 16,
		(cmpvars.fccHandler & 0xFF000000) >> 24);

	if (ret)
	{
		g_config->setOption("SDL.AviVfwFccHandler", fccHandler);
		cmpSet = true;
	}
	return (cmpSet == false) ? -1 : 0;
}

int init(int width, int height)
{
	void *h;
	ICINFO icInfo;
	DWORD dwFormatSize, dwCompressBufferSize;
	bool qualitySupported = false;

	memset(&bmapIn, 0, sizeof(bmapIn));
	bmapIn.biSize = sizeof(BITMAPINFOHEADER);
	bmapIn.biWidth = width;
	bmapIn.biHeight = height;
	bmapIn.biPlanes = 1;
	bmapIn.biBitCount = 24;
	bmapIn.biCompression = BI_RGB;
	bmapIn.biSizeImage = width * height * 3;

	dwFormatSize = ICCompressGetFormatSize(cmpvars.hic, &bmapIn);

	if (ICGetInfo(cmpvars.hic, &icInfo, sizeof(icInfo)))
	{
		printf("Name : %ls\n", icInfo.szName);
		printf("Flags: 0x%08X", icInfo.dwFlags);

		if (icInfo.dwFlags & VIDCF_CRUNCH)
		{
			printf("  VIDCF_CRUNCH  ");
		}

		if (icInfo.dwFlags & VIDCF_TEMPORAL)
		{
			printf("  VIDCF_TEMPORAL  ");
		}

		if (icInfo.dwFlags & VIDCF_TEMPORAL)
		{
			printf("  VIDCF_TEMPORAL  ");
		}

		if (icInfo.dwFlags & VIDCF_QUALITY)
		{
			printf("  VIDCF_QUALITY  ");
			qualitySupported = true;
		}

		if (icInfo.dwFlags & VIDCF_FASTTEMPORALC)
		{
			printf("  VIDCF_FASTTEMPORALC  ");
		}

		if (icInfo.dwFlags & VIDCF_FASTTEMPORALD)
		{
			printf("  VIDCF_FASTTEMPORALD  ");
		}
		printf("\n");

	}

	h = GlobalAlloc(GHND, dwFormatSize);
	bmapOut = (LPBITMAPINFOHEADER)GlobalLock(h);
	memset(bmapOut, 0, sizeof(bmapOut));
	bmapOut->biSize = sizeof(BITMAPINFOHEADER);
	ICCompressGetFormat(cmpvars.hic, &bmapIn, bmapOut);

	dwCompressBufferSize = ICCompressGetSize(cmpvars.hic, &bmapIn, bmapOut);

	h = GlobalAlloc(GHND, dwCompressBufferSize);
	outBuf = (LPVOID)GlobalLock(h);
	memset(outBuf, 0, dwCompressBufferSize);

	if (qualitySupported)
	{
		dwQuality = cmpvars.lQ;
	}
	else
	{
		dwQuality = 0;
	}

	if (ICCompressBegin(cmpvars.hic, &bmapIn, bmapOut) != ICERR_OK)
	{
		printf("Error: ICCompressBegin\n");
		icErrCount = 9999;
		return -1;
	}

	frameNum = 0;
	flagsOut = 0;
	icErrCount = 0;

	return 0;
}

int close(void)
{
	ICCompressEnd(cmpvars.hic);

	GlobalFree(bmapOut);
	GlobalFree(outBuf);

	if (cmpSet)
	{
		ICCompressorFree(&cmpvars);
		cmpSet = false;
	}
	return 0;
}

int encode_frame(unsigned char *inBuf, int width, int height)
{
	DWORD ret;
	DWORD reserved = 0;
	int bytesWritten = 0;

	if (icErrCount > 10)
	{
		return -1;
	}

	ret = ICCompress(
		cmpvars.hic,
		0,
		bmapOut,
		outBuf,
		&bmapIn,
		inBuf,
		&reserved,
		&flagsOut,
		frameNum++,
		0,
		dwQuality,
		NULL, NULL);

	if (ret == ICERR_OK)
	{
		bytesWritten = bmapOut->biSizeImage;
		gwavi->add_frame((unsigned char*)outBuf, bytesWritten, flagsOut);
	}
	else
	{
		printf("Compression Error Frame:%i\n", frameNum);
		icErrCount++;
	}

	return bytesWritten;
}

} // End namespace VFW
#endif

#ifdef _USE_LIBAV

namespace LIBAV
{

AVFormatContext *oc = NULL;
OutputStream  video_st;
OutputStream  audio_st;

OutputStream::OutputStream(void)
{
	st = NULL;
	enc = NULL;
	frame = tmp_frame = NULL;
	pkt = NULL;
	sws_ctx = NULL;
	swr_ctx = NULL;
	bytesPerSample = 0;
	frameSize = 0;
	next_pts = 0;
	writeError = false;
	isAudio = false;
	pixelFormat = -1;
	sampleFormat = -1;
	sampleRate = -1;
	chanLayout = -1;
}

void OutputStream::close(void)
{
	if (writeError)
	{
		char msg[512];
		snprintf(msg, sizeof(msg), "%s Stream Write Errors Detected.\nOutput may be incomplete or corrupt.\nSee log file '%s' for details\n",
			       isAudio ? "Audio" : "Video", AV_LOG_FILE_NAME);
		FCEUD_PrintError(msg);
	}
	if (enc != NULL)
	{
		avcodec_free_context(&enc); enc = NULL;
	}
	if (pkt != NULL)
	{
		av_packet_free(&pkt); pkt = NULL;
	}
	if (frame != NULL)
	{
		av_frame_free(&frame); frame = NULL;
	}
	if (tmp_frame != NULL)
	{
		av_frame_free(&tmp_frame); tmp_frame = NULL;
	}
	if (sws_ctx != NULL)
	{
		sws_freeContext(sws_ctx); sws_ctx = NULL;
	}
	if (swr_ctx != NULL)
	{
		swr_free(&swr_ctx); swr_ctx = NULL;
	}
	st = NULL;
	writeError = false;
	bytesPerSample = 0;
	next_pts = 0;
}

void log_callback(void *avcl, int level, const char *fmt, va_list vl)
{
	if (avLogFp != NULL)
	{
		va_list vl2;

		va_copy(vl2, vl);

		vfprintf(avLogFp, fmt, vl2);

		va_end(vl2);
	}

	av_log_default_callback(avcl, level, fmt, vl);
}

int loadCodecConfig(int type, const char *codec_name, AVCodecContext *ctx)
{
	int i, j;
	char filename[4096];
	char line[512];
	char section[256], id[256], val[256];
	void *obj, *child;
	FILE *fp;
	const char *baseDir = fceu11::GetBaseDirectory();

	snprintf(filename, sizeof(filename), "%s/avi/%s.conf", baseDir, codec_name);

	filename[sizeof(filename) - 1] = 0;

	fp = fopen(filename, "r");

	if (fp == NULL)
	{
		printf("Error: Failed to open file '%s' for reading\n", filename);
		return -1;
	}
	section[0] = 0;

	obj = ctx;

	while (fgets(line, sizeof(line), fp) != 0)
	{
		i = 0;

		while (line[i] != 0)
		{
			if (line[i] == '#')
			{
				line[i] = 0; break;
			}
			i++;
		}
		i = 0;

		while (isspace(line[i])) i++;

		if (line[i] == '[')
		{
			i++;
			while (isspace(line[i])) i++;

			j = 0;
			while ((line[i] != 0) && (line[i] != ']'))
			{
				section[j] = line[i]; i++; j++;
			}
			section[j] = 0; j--;

			while ((j >= 0) && isspace(section[j]))
			{
				section[j] = 0; j--;
			}

			continue;
		}

		j = 0;
		while ((line[i] != 0) && (line[i] != '='))
		{
			id[j] = line[i]; i++; j++;
		}
		id[j] = 0; j--;

		while ((j >= 0) && isspace(id[j]))
		{
			id[j] = 0; j--;
		}

		if (id[0] == 0)
		{
			continue;
		}

		if (line[i] != '=')
		{
			continue;
		}
		i++;

		while (isspace(line[i])) i++;

		j = 0;
		while ((line[i] != 0))
		{
			val[j] = line[i]; i++; j++;
		}
		val[j] = 0; j--;

		while ((j >= 0) && isspace(val[j]))
		{
			val[j] = 0; j--;
		}

		if (section[0] == 0)
		{
			continue;
		}

		obj = ctx;
		child = NULL;

		while (obj != NULL)
		{
			const char *groupName = (*static_cast<AVClass**>(obj))->class_name;

			if (strcmp(groupName, section) == 0)
			{
				break;
			}
			obj = child = av_opt_child_next(ctx, child);
		}

		if (obj != NULL)
		{
			if (av_opt_set(obj, id, val, 0) < 0)
			{
				printf("Error: Failed to set option %s.%s = %s\n", section, id, val);
			}
		}
	}
	fclose(fp);

	return 0;
}

int saveCodecConfig(int type, const char *codec_name, AVCodecContext *ctx)
{
	void *obj, *child = NULL;
	FILE *fp;
	uint8_t *str;
	char filename[4096];
	const AVOption *opt;
	bool useOpt;
	const char *baseDir = fceu11::GetBaseDirectory();

	snprintf(filename, sizeof(filename), "%s/avi/%s.conf", baseDir, codec_name);

	filename[sizeof(filename) - 1] = 0;

	fp = fopen(filename, "w");

	if (fp == NULL)
	{
		printf("Error: Failed to open file '%s' for writing\n", filename);
		return -1;
	}

	obj = ctx;

	while (obj != NULL)
	{
		const char *groupName = (*static_cast<AVClass**>(obj))->class_name;

		fprintf(fp, "\n[ %s ]\n", groupName);

		opt = av_opt_next(obj, NULL);

		while (opt != NULL)
		{
			useOpt = (opt->name != NULL) &&
				(opt->type != AV_OPT_TYPE_BINARY) &&
				(opt->type != AV_OPT_TYPE_DICT);

			if (type)
			{
				useOpt = useOpt &&
					(opt->flags & AV_OPT_FLAG_ENCODING_PARAM) &&
					(opt->flags & AV_OPT_FLAG_AUDIO_PARAM);
			}
			else
			{
				useOpt = useOpt &&
					(opt->flags & AV_OPT_FLAG_ENCODING_PARAM) &&
					(opt->flags & AV_OPT_FLAG_VIDEO_PARAM);
			}

			if (useOpt)
			{
				str = NULL;

				av_opt_get(obj, opt->name, 0, &str);

				if (str)
				{
					if (av_opt_is_set_to_default(obj, opt) == 0)
					{
						fprintf(fp, "%s=%s   # %s\n", opt->name, str,
							opt->help ? opt->help : "");
					}
					av_free(str); str = NULL;
				}
			}
			opt = av_opt_next(obj, opt);
		}
		obj = child = av_opt_child_next(ctx, child);
	}
	fclose(fp);

	return 0;
}

AVFrame *alloc_picture(enum AVPixelFormat pix_fmt, int width, int height)
{
	AVFrame *picture;
	int ret;
	picture = av_frame_alloc();
	if (!picture)
	{
		return NULL;
	}
	picture->format = pix_fmt;
	picture->width = width;
	picture->height = height;
	picture->pts = 0;

	ret = av_frame_get_buffer(picture, 0);
	if (ret < 0)
	{
		fprintf(stderr, "Could not allocate frame data.\n");
		return NULL;
	}

	return picture;
}

int initVideoStream(const char *codec_name, OutputStream *ost)
{
	int ret;
	const AVCodec *codec;
	AVCodecContext *c;
	double fps;
	int fps1000;
	unsigned int usec;

	fps = getBaseFrameRate();

	usec = (unsigned int)((1.0e6 / fps) + 0.50);

	fps1000 = (int)(fps * 1000.0);

	codec = avcodec_find_encoder_by_name(codec_name);

	if (codec == NULL)
	{
		fprintf(avLogFp, "Video codec not found: %s\n", codec_name);
		return -1;
	}

	ost->st = avformat_new_stream(oc, NULL);

	if (ost->st == NULL)
	{
		fprintf(avLogFp, "Error: Could not alloc video stream\n");
		return -1;
	}

	c = avcodec_alloc_context3(codec);

	if (c == NULL)
	{
		fprintf(avLogFp, "Error: Could not alloc an video encoding context\n");
		return -1;
	}
	c->bit_rate = 400000;
	c->gop_size = 12;

	loadCodecConfig(0, codec_name, c);

	ost->enc = c;

	c->width = nes_shm->video.ncol;
	c->height = nes_shm->video.nrow;

	if (codec->id == AV_CODEC_ID_MPEG4)
	{
		ost->st->time_base.num = 1000;
		ost->st->time_base.den = fps1000;
	}
	else
	{
		ost->st->time_base.num = usec;
		ost->st->time_base.den = 1000000u;
	}
	c->time_base = ost->st->time_base;
	c->pix_fmt = (AVPixelFormat)ost->pixelFormat;

	printf("AVI Encoded Video FPS: %.12lf\n", (double)ost->st->time_base.den / (double)ost->st->time_base.num);

	if (codec->pix_fmts)
	{
		if (ost->pixelFormat == -1)
		{
			c->pix_fmt = avcodec_find_best_pix_fmt_of_list(codec->pix_fmts, AV_PIX_FMT_BGRA, 0, NULL);
		}

		int i = 0, formatOk = 0;
		while (codec->pix_fmts[i] != -1)
		{
			if (codec->pix_fmts[i] == c->pix_fmt)
			{
				printf("CODEC Supports PIX_FMT:%i\n", c->pix_fmt);
				formatOk = 1;
			}
			i++;
		}
		if (!formatOk)
		{
			printf("CODEC Does Not Support PIX_FMT:%i\n", c->pix_fmt);

			c->pix_fmt = avcodec_find_best_pix_fmt_of_list(codec->pix_fmts, AV_PIX_FMT_BGRA, 0, NULL);

			printf("Changing to:%i\n", c->pix_fmt);
		}
	}
	else
	{
		if (ost->pixelFormat == -1)
		{
			c->pix_fmt = AV_PIX_FMT_YUV420P;
		}
	}

	if (c->codec_id == AV_CODEC_ID_MPEG2VIDEO)
	{
		c->max_b_frames = 2;
	}
	if (c->codec_id == AV_CODEC_ID_MPEG1VIDEO)
	{
		c->mb_decision = 2;
	}
	if (oc->oformat->flags & AVFMT_GLOBALHEADER)
	{
		c->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
	}

	if (avcodec_open2(c, NULL, NULL) < 0)
	{
		fprintf(avLogFp, "Error: Could not open codec: %s\n", codec_name);
		return -1;
	}

	ost->pkt = av_packet_alloc();
	if (ost->pkt == NULL)
	{
		fprintf(avLogFp, "Could not allocate the video packet\n");
		return -1;
	}

	ost->frame = alloc_picture(c->pix_fmt, c->width, c->height);

	if (!ost->frame)
	{
		fprintf(avLogFp, "Error: Could not allocate picture\n");
		return -1;
	}

	ost->tmp_frame = alloc_picture(AV_PIX_FMT_BGRA, c->width, c->height);

	if (ost->tmp_frame == NULL)
	{
		fprintf(avLogFp, "Error: Could not allocate temporary picture\n");
		return -1;
	}

	ost->sws_ctx = sws_getContext(c->width, c->height,
		AV_PIX_FMT_BGRA,
		c->width, c->height,
		c->pix_fmt,
		SWS_BICUBIC, NULL, NULL, NULL);

	if (ost->sws_ctx == NULL)
	{
		fprintf(avLogFp, "Error: Video sws_getContext Failed. Video conversion not possible\n");
		return -1;
	}

	ret = avcodec_parameters_from_context(ost->st->codecpar, c);

	if (ret < 0)
	{
		fprintf(avLogFp, "Error: Video avcodec_parameters_from_context Failed. Could not copy the stream parameters\n");
		return -1;
	}
	ost->writeError = false;

	return 0;
}

int encode_video_frame(unsigned char *inBuf)
{
	int ret, y, ofs, inLineSize;
	OutputStream *ost = &video_st;
	AVCodecContext *c = video_st.enc;
	unsigned char *outBuf;

	if (ost->writeError)
	{
		return -1;
	}
	ret = av_frame_make_writable(video_st.frame);

	if (ret < 0)
	{
		return -1;
	}

	ofs = 0;

	inLineSize = c->width * 4;

	outBuf = ost->tmp_frame->data[0];

	for (y = 0; y < c->height; y++)
	{
		memcpy(outBuf, &inBuf[ofs], inLineSize); ofs += inLineSize;

		outBuf += ost->tmp_frame->linesize[0];
	}

	sws_scale(ost->sws_ctx, (const uint8_t * const *)ost->tmp_frame->data,
		ost->tmp_frame->linesize, 0, c->height, ost->frame->data,
		ost->frame->linesize);

	video_st.frame->pts = video_st.next_pts++;

	ret = avcodec_send_frame(c, video_st.frame);
	if (ret < 0)
	{
		fprintf(avLogFp, "Error submitting a video frame for encoding\n");
		ost->writeError = true;
		return -1;
	}

	while (ret >= 0)
	{
		ret = avcodec_receive_packet(c, ost->pkt);

		if (ret < 0 && ret != AVERROR(EAGAIN) && ret != AVERROR_EOF)
		{
			fprintf(avLogFp, "Error encoding a video frame\n");
			ost->writeError = true;
			return -1;
		}
		else if (ret >= 0)
		{
			av_packet_rescale_ts(ost->pkt, c->time_base, video_st.st->time_base);
			ost->pkt->stream_index = video_st.st->index;
			ret = av_interleaved_write_frame(oc, ost->pkt);
			if (ret < 0)
			{
				fprintf(avLogFp, "Error while writing video frame\n");
				ost->writeError = true;
				return -1;
			}
			av_packet_unref(ost->pkt);
		}
	}

	return ret == AVERROR_EOF;
}

int init(int width, int height)
{
	return 0;
}

} // End namespace LIBAV
#endif
