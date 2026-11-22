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
#include "common/args.h"
#include "common/config.h"

#include "Qt/input.h"

extern CFGSTRUCT DriverConfig[];
extern ARGPSTRUCT DriverArgs[];

void DoDriverArgs(void);

int InitSound();
void WriteSound(int32 *Buffer, int Count);
int KillSound(void);
// v2.0.5 (A1): block until the audio device probe thread is idle.  Exit path
// only, immediately before SDL_Quit(): the probe calls SDL_OpenAudio() on its
// own thread, so quitting SDL underneath it is a race.  Never call this from an
// interactive path -- waiting for that probe is the freeze this replaced.
void WaitSoundProbeIdle(void);
uint32 GetMaxSound(void);
uint32 GetWriteSound(void);
// v2.0.5 (PERF-02): true while an SDL audio device is open, i.e. while the ring
// buffer actually has a consumer. When this is false WriteSound drops its
// samples instead of waiting for a callback that does not exist. Exposed so a
// test can pin that contract, and so the sound configuration dialog can say
// "no device" instead of showing a buffer that will never drain.
bool FCEUD_SoundDeviceIsOpen(void);
// v2.0 S2-b4 stage 3': the rate the device was opened at, which is not always
// the rate that was asked for. For a machine whose resampler is built around
// it. See the definition in sdl-sound.cpp.
uint32 FCEUD_GetSoundRate(void);
bool FCEUD_SoundIsMuted(void);
void FCEUD_MuteSoundOutput(bool value);
void FCEUD_MuteSoundWindow(bool value);

void SilenceSound(int s); /* DOS and SDL */

int InitJoysticks(void);
int KillJoysticks(void);
int AddJoystick( int which );
int RemoveJoystick( int which );
int FindJoystickByInstanceID( int which );
uint32 *GetJSOr(void);

int InitVideo(FCEUGI *gi);
int KillVideo(void);
void CalcVideoDimensions(void);
void BlitScreen(uint8 *XBuf);
void LockConsole(void);
void UnlockConsole(void);
void ToggleFS();		/* SDL */

int LoadGame(const char *path, bool silent);
//int CloseGame(void);

void Giggles(int);
void DoFun(void);

int FCEUD_NetworkConnect(void);

