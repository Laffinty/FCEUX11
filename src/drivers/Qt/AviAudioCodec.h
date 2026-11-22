/* FCE Ultra - NES/Famicom Emulator
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
