// AviAudioCodec.cpp
//

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fceu.h"
#include "Qt/nes_shm.h"
#include "Qt/fceuWrapper.h"

#include "Qt/AviAudioCodec.h"

#ifdef _USE_LIBAV

namespace LIBAV
{

AVFrame *alloc_audio_frame(const AVCodecContext *c, int nb_samples)
{
	AVFrame *frame = av_frame_alloc();
	int ret;
	if (!frame)
	{
		fprintf(stderr, "Error allocating an audio frame\n");
		return NULL;
	}
	frame->format = c->sample_fmt;
#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
	frame->channel_layout = c->channel_layout;
#else
	av_channel_layout_copy(&frame->ch_layout, &c->ch_layout);
#endif
	frame->sample_rate = c->sample_rate;
	frame->nb_samples = nb_samples;

	if (nb_samples)
	{
		ret = av_frame_get_buffer(frame, 0);
		if (ret < 0)
		{
			fprintf(stderr, "Error allocating an audio buffer\n");
			return NULL;
		}
	}
	return frame;
}

#if LIBAVUTIL_VERSION_INT >= AV_VERSION_INT(57, 28, 100)
static int select_audio_channel_layout(const OutputStream *ost, const AVCodec *codec, AVChannelLayout *dst)
{
	int best_nb_channels = 0;
	const AVChannelLayout *p, *best_ch_layout;
#if __cplusplus >= 202002L
	const AVChannelLayout defaultLayout = AV_CHANNEL_LAYOUT_MONO;
#else
	AVChannelLayout defaultLayout;
	av_channel_layout_from_mask(&defaultLayout, AV_CH_LAYOUT_MONO);
#endif

	if (!codec->ch_layouts)
	{
		return av_channel_layout_copy(dst, &defaultLayout);
	}

	best_ch_layout = p = codec->ch_layouts;
	while (p && p->nb_channels)
	{
		int nb_channels = p->nb_channels;

		if (ost->chanLayout > 0)
		{
			if (ost->chanLayout == p->u.mask)
			{
				best_ch_layout = p;
				best_nb_channels = nb_channels;
				break;
			}
		}
		if (nb_channels > best_nb_channels)
		{
			best_ch_layout = p;
			best_nb_channels = nb_channels;
		}
		p++;
	}
	return av_channel_layout_copy(dst, best_ch_layout);
}
#endif

int initAudioStream(const char *codec_name, OutputStream *ost)
{
	int ret, nb_samples;
	const AVCodec *codec;
	AVCodecContext *c;

	ost->isAudio = true;

	codec = avcodec_find_encoder_by_name(codec_name);

	if (codec == NULL)
	{
		fprintf(avLogFp, "Audio codec not found: '%s'\n", codec_name);
		return -1;
	}

	ost->st = avformat_new_stream(oc, NULL);

	if (ost->st == NULL)
	{
		fprintf(avLogFp, "Could not alloc audio stream\n");
		return -1;
	}

	c = avcodec_alloc_context3(codec);

	if (c == NULL)
	{
		fprintf(avLogFp, "Could not alloc an audio encoding context\n");
		return -1;
	}
	loadCodecConfig(1, codec_name, c);

	ost->enc = c;

	if (ost->sampleFormat > 0)
	{
		c->sample_fmt = (AVSampleFormat)ost->sampleFormat;

		if (codec->sample_fmts)
		{
			int i = 0, formatOk = 0;
			while (codec->sample_fmts[i] != -1)
			{
				if (c->sample_fmt == codec->sample_fmts[i])
				{
					formatOk = true; break;
				}
				i++;
			}
			if (!formatOk)
			{
				c->sample_fmt = codec->sample_fmts ? codec->sample_fmts[0] : AV_SAMPLE_FMT_S16;
			}
		}
	}
	else
	{
		c->sample_fmt = codec->sample_fmts ? codec->sample_fmts[0] : AV_SAMPLE_FMT_S16;
	}

	if (ost->sampleRate > 0)
	{
		c->sample_rate = ost->sampleRate;

		if (codec->supported_samplerates)
		{
			int i = 0, formatOk = 0;
			while (codec->supported_samplerates[i] != 0)
			{
				if (c->sample_rate == codec->supported_samplerates[i])
				{
					formatOk = true; break;
				}
				i++;
			}
			if (!formatOk)
			{
				c->sample_rate = codec->supported_samplerates ? codec->supported_samplerates[0] : audioSampleRate;
			}
		}
	}
	else
	{
		c->sample_rate = codec->supported_samplerates ? codec->supported_samplerates[0] : audioSampleRate;
	}

#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
	if (ost->chanLayout > 0)
	{
		c->channel_layout = ost->chanLayout;

		if (codec->channel_layouts)
		{
			int i = 0, formatOk = 0;
			while (codec->channel_layouts[i] != 0)
			{
				if (c->channel_layout == codec->channel_layouts[i])
				{
					formatOk = true; break;
				}
				i++;
			}
			if (!formatOk)
			{
				c->channel_layout = codec->channel_layouts ? codec->channel_layouts[0] : AV_CH_LAYOUT_STEREO;
			}
		}
	}
	else
	{
		c->channel_layout = codec->channel_layouts ? codec->channel_layouts[0] : AV_CH_LAYOUT_STEREO;
	}
	c->channels = av_get_channel_layout_nb_channels(c->channel_layout);
#else
	if (select_audio_channel_layout(ost, codec, &c->ch_layout))
	{
		fprintf(avLogFp, "Error selecting the audio channel layout\n");
		return -1;
	}
#endif
	c->bit_rate = 64000;
	ost->st->time_base.num = 1;
	ost->st->time_base.den = c->sample_rate;
	if (oc->oformat->flags & AVFMT_GLOBALHEADER)
	{
		c->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
	}
	ost->swr_ctx = swr_alloc();
	if (!ost->swr_ctx)
	{
		fprintf(avLogFp, "Error allocating the audio resampling context\n");
		return -1;
	}
	av_opt_set_sample_fmt(ost->swr_ctx, "in_sample_fmt", AV_SAMPLE_FMT_S16, 0);
	av_opt_set_int(ost->swr_ctx, "in_sample_rate", audioSampleRate, 0);
#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
	av_opt_set_int(ost->swr_ctx, "in_channel_layout", AV_CH_LAYOUT_MONO, 0);
#else
#if __cplusplus >= 202002L
	AVChannelLayout src_ch_layout = AV_CHANNEL_LAYOUT_MONO;
#else
	AVChannelLayout src_ch_layout;
	av_channel_layout_from_mask(&src_ch_layout, AV_CH_LAYOUT_MONO);
#endif
	av_opt_set_chlayout(ost->swr_ctx, "in_chlayout", &src_ch_layout, 0);
#endif
	av_opt_set_sample_fmt(ost->swr_ctx, "out_sample_fmt", c->sample_fmt, 0);
	av_opt_set_int(ost->swr_ctx, "out_sample_rate", c->sample_rate, 0);
#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
	av_opt_set_int(ost->swr_ctx, "out_channel_layout", c->channel_layout, 0);
#else
	av_opt_set_chlayout(ost->swr_ctx, "out_chlayout", &c->ch_layout, 0);
#endif

	ret = swr_init(ost->swr_ctx);
	if (ret < 0)
	{
		fprintf(avLogFp, "Error: Could not init the audio resampling context\n");
		return -1;
	}

	if (avcodec_open2(c, NULL, NULL) < 0)
	{
		fprintf(avLogFp, "Error: Could not open codec: %s\n", codec_name);
		return -1;
	}

	ost->pkt = av_packet_alloc();
	if (ost->pkt == NULL)
	{
		fprintf(avLogFp, "Could not allocate the audio packet\n");
		return -1;
	}

	if (c->codec->capabilities & AV_CODEC_CAP_VARIABLE_FRAME_SIZE)
	{
		nb_samples = audioSampleRate / 4;

		if (nb_samples < 10000)
		{
			nb_samples = 10000;
		}
	}
	else
	{
		nb_samples = c->frame_size;
	}

	ost->frameSize = nb_samples;
	ost->bytesPerSample = av_get_bytes_per_sample(c->sample_fmt);

	ost->frame = alloc_audio_frame(c, nb_samples);
	ost->tmp_frame = alloc_audio_frame(c, nb_samples);

	ret = avcodec_parameters_from_context(ost->st->codecpar, c);
	if (ret < 0)
	{
		fprintf(avLogFp, "Error: Audio avcodec_parameters_from_context Failed. Could not copy the stream parameters\n");
		return -1;
	}
	ost->frame->nb_samples = 0;
	ost->writeError = false;

	return 0;
}

int write_audio_frame(AVFrame *frame)
{
	int ret;
	OutputStream *ost = &audio_st;

	if (ost->writeError)
	{
		return -1;
	}
	if (frame)
	{
		frame->pts = ost->next_pts;
		ost->next_pts += frame->nb_samples;
	}

	ret = avcodec_send_frame(ost->enc, frame);

	if (ret < 0)
	{
		fprintf(avLogFp, "Error submitting audio frame for encoding\n");
		ost->writeError = true;
		return -1;
	}
	while (ret >= 0)
	{
		ret = avcodec_receive_packet(ost->enc, ost->pkt);
		if (ret < 0 && ret != AVERROR(EAGAIN) && ret != AVERROR_EOF)
		{
			fprintf(avLogFp, "Error encoding audio frame\n");
			ost->writeError = true;
			return -1;
		}
		else if (ret >= 0)
		{
			av_packet_rescale_ts(ost->pkt, ost->enc->time_base, ost->st->time_base);
			ost->pkt->stream_index = ost->st->index;
			ret = av_interleaved_write_frame(oc, ost->pkt);
			if (ret < 0)
			{
				fprintf(avLogFp, "Error while writing audio frame\n");
				ost->writeError = true;
				return -1;
			}
			av_packet_unref(ost->pkt);
		}
	}
	return 0;
}

int encode_audio_frame(int16_t *audioOut, int numSamples)
{
	int i, ret;
	OutputStream *ost = &audio_st;
	const uint8_t *inData[AV_NUM_DATA_POINTERS];

	if (ost->st == NULL)
	{
		return -1;
	}
	if (ost->writeError)
	{
		return -1;
	}

	for (i = 0; i < AV_NUM_DATA_POINTERS; i++)
	{
		inData[i] = 0;
	}

	if (audioOut)
	{
		inData[0] = (const uint8_t*)audioOut;
	}

	ret = av_frame_make_writable(ost->tmp_frame);

	if (ret < 0)
	{
		return -1;
	}

	if (audioOut)
	{
		ret = swr_convert(ost->swr_ctx, ost->tmp_frame->data, ost->tmp_frame->linesize[0], inData, numSamples);
	}
	else
	{
		ret = swr_convert(ost->swr_ctx, ost->tmp_frame->data, ost->tmp_frame->linesize[0], NULL, 0);
	}

	if (ret < 0)
	{
		fprintf(stderr, "Error feeding audio data to the resampler\n");
		return -1;
	}
	if (ret > 0)
	{
		int spaceAvail, samplesLeft, copySize, srcOffset = 0, frameSize;

		frameSize = ost->frameSize;

		samplesLeft = ost->tmp_frame->nb_samples = ret;

		spaceAvail = frameSize - ost->frame->nb_samples;

		while (samplesLeft > 0)
		{
			if (spaceAvail >= samplesLeft)
			{
				copySize = samplesLeft;
			}
			else
			{
				copySize = spaceAvail;
			}

			ret = av_frame_make_writable(ost->frame);

			if (ret < 0)
			{
				fprintf(stderr, "Error audio av_frame_make_writable\n");
				return -1;
			}

			ret = av_samples_copy(ost->frame->data, ost->tmp_frame->data,
				ost->frame->nb_samples, srcOffset, copySize,
#if LIBAVUTIL_VERSION_INT < AV_VERSION_INT(57, 28, 100)
				ost->frame->channels,
#else
				ost->frame->ch_layout.nb_channels,
#endif
				ost->enc->sample_fmt);

			if (ret < 0)
			{
				return -1;
			}
			ost->frame->nb_samples += copySize;
			srcOffset += copySize;
			samplesLeft -= copySize;

			if (ost->frame->nb_samples >= frameSize)
			{
				if (write_audio_frame(ost->frame))
				{
					return -1;
				}
				ost->frame->nb_samples = 0;
			}
			spaceAvail = frameSize - ost->frame->nb_samples;
		}
	}

	if (audioOut == NULL)
	{
		if (ost->frame->nb_samples > 0)
		{
			if (write_audio_frame(ost->frame))
			{
				return -1;
			}
			ost->frame->nb_samples = 0;
		}
		if (write_audio_frame(NULL))
		{
			return -1;
		}
	}
	return 0;
}

} // End namespace LIBAV
#endif
