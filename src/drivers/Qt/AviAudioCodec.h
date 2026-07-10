// AviAudioCodec.h
//

#pragma once

#include "Qt/AviVideoCodec.h"

#ifdef _USE_LIBAV

namespace LIBAV
{

AVFrame *alloc_audio_frame(const AVCodecContext *c, int nb_samples);
int write_audio_frame(AVFrame *frame);
int encode_audio_frame(int16_t *audioOut, int numSamples);

int initAudioStream(const char *codec_name, struct OutputStream *ost);

}

#endif
