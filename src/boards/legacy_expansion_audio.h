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
// FCEUX11 — v1.8 Masonry Phase G: Legacy ExpansionAudio wrapper.
//
// Thin wrapper that delegates ExpansionAudio virtual calls to the existing
// EXPSOUND function pointers (GameExpSound.Fill, etc.).  This allows
// mappers that already set up GameExpSound in their Init functions to
// satisfy the install_expansion_audio contract without refactoring their
// audio code into classes.

#ifndef FCEU11_LEGACY_EXPANSION_AUDIO_H
#define FCEU11_LEGACY_EXPANSION_AUDIO_H

#include "expansion_audio.h"
#include "sound.h"  // GameExpSound, EXPSOUND

namespace fceu11 {

class LegacyExpansionAudio : public ExpansionAudio {
public:
    void fill(int32_t count) override {
        if (GameExpSound.Fill) GameExpSound.Fill(count);
    }
    void hi_fill() override {
        if (GameExpSound.HiFill) GameExpSound.HiFill();
    }
    void hi_sync(int32_t ts) override {
        if (GameExpSound.HiSync) GameExpSound.HiSync(ts);
    }
    void region_changed() override {
        if (GameExpSound.RChange) GameExpSound.RChange();
    }
    void kill() override {
        if (GameExpSound.Kill) GameExpSound.Kill();
    }
    void neo_fill(int32_t* wave, int32_t count) override {
        if (GameExpSound.NeoFill) GameExpSound.NeoFill(wave, count);
    }
};

// Static instances for each mapper.
static LegacyExpansionAudio g_vrc7_expansion_audio;
static LegacyExpansionAudio g_mmc5_expansion_audio;
static LegacyExpansionAudio g_n106_expansion_audio;
static LegacyExpansionAudio g_s5b_expansion_audio;
static LegacyExpansionAudio g_fds_expansion_audio;

} // namespace fceu11

#endif // FCEU11_LEGACY_EXPANSION_AUDIO_H
