/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2002 Xodnizel
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

/// \file
/// \brief Handles sound emulation using the SDL.

#include "sdl.h"

#include "common/configSys.h"
#include "utils/memory.h"
#include "common/nes_shm.h"
#include "Qt/throttle.h"
#include "../../profiler.h" // B2②: WriteSound 探点（OFF 态展开为空）

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <memory>
#include <atomic>
#include <mutex>
#include <thread>

extern Config *g_config;
extern bool turbo;

static std::unique_ptr<int[]> s_Buffer;
static unsigned int s_BufferSize;
static unsigned int s_BufferSize25;
static unsigned int s_BufferSize50;
static unsigned int s_BufferSize75;
// hotfix3 B-1 (QT-CRASH-05): s_BufferRead/Write/In are touched by the SDL
// audio-callback thread (fillaudio) and the emulator thread (WriteSound).
// SDL_LockAudio already provides mutual exclusion between the two within a
// single audio session, but InitSound/KillSound mutate the cursors from the
// emulator thread without holding SDL_LockAudio, and the SDL audio callback
// can fire after SDL_CloseAudio drains. std::atomic<unsigned int> gives us
// a happens-before guarantee between init/destroy and the callback.
static std::atomic<unsigned int> s_BufferRead{0};
static std::atomic<unsigned int> s_BufferWrite{0};
static std::atomic<unsigned int> s_BufferIn{0};
static unsigned int s_SampleRate = 44100;
static double noiseGate = 0.0;
static double noiseGateRate = 0.010;
static bool   noiseGateActive = true;
static bool   windowSoundMute = false;
static bool   fillInit = 1;
static const unsigned int supportedSampleRates[] = { 11025, 22050, 44100, 48000, 96000, 0 };

static bool s_mute = false;

// ---------------------------------------------------------------------------
// v2.0.5 (PERF-01): audio device open state.
//
// Symptom (measured on the owner's machine, v2.0.4_hotfix1, 2026-11-16):
//   every ROM load runs DriverInitialize -> InitSound -> SDL_OpenAudio.  On
//   this machine the WASAPI backend cannot find an audio endpoint and
//   SDL_OpenAudio only returns (with an error) after **eight seconds**:
//       t=  0.70s  Initializing video... Video Mode: 256 x 224 x 32 bpp
//       t=  8.70s  WASAPI can't find requested audio endpoint
//   Those eight seconds run inside the GUI thread while it holds
//   FCEU_WRAPPER_LOCK, so the emulation thread is locked out and the whole
//   window stops responding -- the user's report of "loading a ROM hangs for
//   about ten seconds".
//
// The four measures below, in the order they matter:
//   1) the device is opened on a **background thread**, so no caller ever
//      waits for the backend's probe (this is the actual fix);
//   2) an already-open device with the same sample rate is **reused** rather
//      than reopened -- the old code reopened (and failed) on every load;
//   3) a failed open goes into a **backoff window**, so a broken endpoint is
//      probed once per window instead of once per ROM load;
//   4) a failed WASAPI open **falls back** to directsound / winmm, which
//      open in milliseconds here and actually produce sound.
//
// Only the SDL side runs on the background thread; the core-side settings
// (sample rate, volumes) stay on the calling thread exactly as before.
// ---------------------------------------------------------------------------
static std::atomic<bool>         s_audioDeviceOpen{false};
static std::atomic<bool>         s_audioOpenInFlight{false};
static std::atomic<unsigned int> s_audioOpenRate{0};
static std::atomic<uint32_t>     s_audioFailAtMs{0};
static std::mutex                s_audioOpenMutex;
static std::thread                s_audioOpenThread;

// How long a failed endpoint probe is suppressed before it is retried.
static const uint32_t kAudioRetryBackoffMs = 60000;

// Milliseconds since SDL start (wraps every ~49 days; the unsigned
// subtraction below stays correct across the wrap).
static uint32_t AudioNowMs(void)
{
	return static_cast<uint32_t>(SDL_GetTicks());
}

extern int EmulationPaused;
extern double frmRateAdjRatio;
extern double g_fpsScale;

/**
 * Callback from the SDL to get and play audio data.
 */
static void
fillaudio(void *udata,
		uint8 *stream,
		int len)
{
	char bufStarveDetected = 0;
	static int16_t sample = 0;
	char mute;
	//unsigned int starve_lp = nes_shm->sndBuf.starveCounter;
	int16 *tmps = (int16*)stream;
	len >>= 1;

	if ( s_BufferIn > s_BufferSize25 )
	{
		fillInit = 0;
	}
	// If emulation is paused:
	// 1. preserve sound buffer as is
	// 2. fade from last known sample to zero
	// 3. activate noise gate to avoid popping when coming out of pause.
	if ( EmulationPaused || fillInit )
	{
		while ( len )
		{
			if ( sample > 0 )
			{
				sample--;
			}
			else if ( sample < 0 )
			{
				sample++;
			}
			*tmps = sample;
			tmps++;
			len--;
		}
		noiseGate = 0.0;
		noiseGateActive = 1;
		return;
	}
	mute = EmulationPaused || windowSoundMute || s_mute;

	if ( mute || noiseGateActive )
	{
		// This noise gate helps avoid abrupt snaps in audio
		// when pausing emulation.
		while (len) 
		{
			if (mute)
			{
				noiseGate -= noiseGateRate;

				if ( noiseGate < 0.0 )
				{
					noiseGate = 0.0;
				}
				noiseGateActive = 1;
			}
			else
			{
				if ( s_BufferIn )
				{	
					noiseGate += noiseGateRate;

					if ( noiseGate > 1.0 )
					{
						noiseGate = 1.0;
						noiseGateActive = 0;
					}
				}
			}
			if (s_BufferIn) 
			{
				sample = s_Buffer[s_BufferRead] * noiseGate;
				s_BufferRead = (s_BufferRead + 1) % s_BufferSize;
				s_BufferIn--;

				*tmps = sample * noiseGate;
			}
			else
			{
         			// Retain last known sample value, helps avoid clicking
         			// noise when sound system is starved of audio data.
				*tmps = sample * noiseGate;
			}

			tmps++;
			len--;
		}
	}
	else
	{
		while (len) 
		{
			if (s_BufferIn) 
			{
				sample = s_Buffer[s_BufferRead];
				s_BufferRead = (s_BufferRead + 1) % s_BufferSize;
				s_BufferIn--;
			} else {
        	 		// Retain last known sample value, helps avoid clicking
        	 		// noise when sound system is starved of audio data.
				//sample = 0; 
				bufStarveDetected = 1;
				nes_shm->sndBuf.starveCounter.fetch_add(1, std::memory_order_relaxed);
			}

			nes_shm->push_sound_sample( sample );

			*tmps = sample;
			tmps++;
			len--;
		}
	}
	if ( bufStarveDetected )
	{
		//s_StarveCounter = nes_shm->sndBuf.starveCounter - starve_lp;
		//printf("Starve:%u\n", s_StarveCounter );
	}
}

/**
 * Open the audio device on a background thread.
 *
 * Why a thread: SDL_OpenAudio() is not just a handle hand-out -- the
 * backend probes the machine for a usable endpoint, and on a host whose
 * default endpoint is missing the WASAPI backend spends eight seconds
 * discovering that (see the PERF-01 note above).  Paying that cost on the
 * GUI thread, inside the wrapper lock, is what made "load a ROM" look like
 * a ten-second freeze.
 *
 * The caller never waits for the result; the device simply becomes
 * available a moment later.
 */
static void
AudioOpenDeviceWorker(SDL_AudioSpec spec)
{
	bool ok = false;
	char firstErr[256] = { 0 };

	if (SDL_InitSubSystem(SDL_INIT_AUDIO) < 0)
	{
		snprintf(firstErr, sizeof(firstErr), "%s", SDL_GetError());
	}
	else if (SDL_OpenAudio(&spec, nullptr) >= 0)
	{
		ok = true;
	}
	else
	{
		snprintf(firstErr, sizeof(firstErr), "%s", SDL_GetError());

		// Fallback: the default backend (WASAPI on Windows) is the one that
		// stalls and then fails when the host has no usable default endpoint.
		// directsound and winmm open in milliseconds on the same machine and
		// do produce sound, so a broken WASAPI install should not cost the
		// user their audio.
		static const char* const fallbacks[] = { "directsound", "winmm" };
		for (const char* fb : fallbacks)
		{
			SDL_CloseAudio();
			SDL_QuitSubSystem(SDL_INIT_AUDIO);
			SDL_setenv("SDL_AUDIODRIVER", fb, 1);

			if (SDL_InitSubSystem(SDL_INIT_AUDIO) < 0)
				continue;
			if (SDL_OpenAudio(&spec, nullptr) >= 0)
			{
				ok = true;
				break;
			}
		}
	}

	if (ok)
	{
		SDL_PauseAudio(0);
		s_audioOpenRate.store(static_cast<unsigned int>(spec.freq), std::memory_order_release);
		s_audioDeviceOpen.store(true, std::memory_order_release);
		s_audioFailAtMs.store(0, std::memory_order_release);

		const char* driverName = SDL_GetCurrentAudioDriver();
		fprintf(stderr, "Loading SDL sound with %s driver...\n", driverName ? driverName : "unknown");
	}
	else
	{
		// Remember the failure so the next ROM load does not pay the same
		// probe again straight away.
		s_audioFailAtMs.store(AudioNowMs(), std::memory_order_release);
		fprintf(stderr,
		        "Sound: no usable audio endpoint (%s); retrying in %u s\n",
		        firstErr[0] ? firstErr : "unknown error",
		        (unsigned)(kAudioRetryBackoffMs / 1000));
	}

	s_audioOpenInFlight.store(false, std::memory_order_release);
}

/**
 * Make sure an audio device is (being) opened for the given sample rate.
 *
 * Returns true when a device with that rate is already open (the caller
 * must then NOT touch the ring buffer -- the audio callback thread is
 * reading it), false when the device is not ready yet (it is being opened
 * on the background thread, or the last attempt failed and is still inside
 * the backoff window).
 */
static bool
AudioEnsureDeviceOpen(const SDL_AudioSpec& spec)
{
	if (s_audioDeviceOpen.load(std::memory_order_acquire) &&
	    s_audioOpenRate.load(std::memory_order_acquire) == static_cast<unsigned int>(spec.freq))
	{
		// Already open with the rate we want -- reuse it.  The previous
		// code called SDL_OpenAudio() again here, which both failed and
		// cost the backend's full probe time on every single ROM load.
		return true;
	}

	if (s_audioOpenInFlight.load(std::memory_order_acquire))
	{
		// An open is already running; do not start a second one and do not
		// wait for it.
		return false;
	}

	const uint32_t now = AudioNowMs();
	const uint32_t failAt = s_audioFailAtMs.load(std::memory_order_acquire);
	if (failAt != 0 && (now - failAt) < kAudioRetryBackoffMs)
	{
		// The endpoint probe failed recently.  Stay quiet until the window
		// expires instead of stalling every ROM load for the same seconds.
		return false;
	}

	s_audioOpenInFlight.store(true, std::memory_order_release);
	{
		std::lock_guard<std::mutex> lk(s_audioOpenMutex);
		if (s_audioOpenThread.joinable())
			s_audioOpenThread.join();   // previous attempt has finished
	}
	s_audioOpenThread = std::thread(AudioOpenDeviceWorker, spec);
	return false;
}

/**
 * Initialize the audio subsystem.
 */
int
InitSound()
{
	int i, sound, soundrate, soundbufsize, soundvolume, soundtrianglevolume, soundsquare1volume, soundsquare2volume, soundnoisevolume, soundpcmvolume, soundq;
	SDL_AudioSpec spec;
	int frmRateSampleAdj = 0;
	int samplesPerFrame;
	bool sampleRateIsSupported = false;

	g_config->getOption("SDL.Sound", &sound);
	if (!sound) 
	{
		return 0;
	}

	memset(&spec, 0, sizeof(spec));
	// The audio subsystem itself is initialised on the background thread
	// together with the device open -- see AudioEnsureDeviceOpen.

	// load configuration variables
	g_config->getOption("SDL.Sound.Mute", &s_mute);
	g_config->getOption("SDL.Sound.Rate", &soundrate);
	g_config->getOption("SDL.Sound.BufSize", &soundbufsize);
	g_config->getOption("SDL.Sound.Volume", &soundvolume);
	g_config->getOption("SDL.Sound.Quality", &soundq);
	g_config->getOption("SDL.Sound.TriangleVolume", &soundtrianglevolume);
	g_config->getOption("SDL.Sound.Square1Volume", &soundsquare1volume);
	g_config->getOption("SDL.Sound.Square2Volume", &soundsquare2volume);
	g_config->getOption("SDL.Sound.NoiseVolume", &soundnoisevolume);
	g_config->getOption("SDL.Sound.PCMVolume", &soundpcmvolume);

	i = 0;
	while (supportedSampleRates[i] != 0)
	{
		if ( static_cast<unsigned int>(soundrate) == supportedSampleRates[i])
		{
			sampleRateIsSupported = true;
			break;
		}
		i++;
	}

	if (!sampleRateIsSupported)
	{
		printf("Error: Audio Sample Rate %i is either invalid or not supported, reverting to default of 44100\n", soundrate);
		soundrate = 44100;
		g_config->setOption("SDL.Sound.Rate", soundrate);
	}

	if ( (soundbufsize < 15) || (soundbufsize > 200) )
	{
		printf("Error: Audio Buffer Size of %i ms is invalid, reverting to default of 128\n", soundbufsize);
		soundbufsize = 128;
		g_config->setOption("SDL.Sound.BufSize", soundbufsize);
	}

	spec.freq = s_SampleRate = soundrate;
	spec.format = AUDIO_S16SYS;
	spec.channels = 1;
	spec.samples = 512; // This must stay a power of two per SDL documentation
	spec.callback = fillaudio;
	spec.userdata = 0;

	samplesPerFrame = (int)( ( (double)s_SampleRate / getBaseFrameRate() ) );

	if ( samplesPerFrame >= 1024 )
	{
		spec.samples = 1024;
	}

	s_BufferSize = soundbufsize * soundrate / 1000;

	// For safety, set a bare minimum:
	if (s_BufferSize < static_cast<unsigned int>(spec.samples * 4))
	{
		s_BufferSize = spec.samples * 4;
	}
	s_BufferSize25 =    s_BufferSize/4;
	s_BufferSize50 =    s_BufferSize/2;
	s_BufferSize75 = (3*s_BufferSize)/4;

	//printf("Audio Buffer: %i  %i \n", spec.samples, s_BufferSize );

	noiseGate = 0.0;
	noiseGateRate = 1.0 / (double)spec.samples;
	noiseGateActive = true;
	fillInit = 1;

	// v2.0.5 (PERF-01): the device is opened on a background thread.
	// Waiting for SDL_OpenAudio() right here is what put the backend's
	// endpoint probe (eight seconds on a host without a usable default
	// endpoint) inside the GUI thread's wrapper lock, freezing the window
	// for the duration of every ROM load.  AudioEnsureDeviceOpen() returns
	// without waiting; the device becomes usable a moment later.
	const bool deviceReady = AudioEnsureDeviceOpen(spec);

	if (!deviceReady)
	{
		// The ring buffer is only (re)allocated when a device actually has
		// to be opened.  When the device is already open the audio callback
		// thread is reading this buffer, and replacing it under its feet is
		// a data race the old code had as well (it reopened the device on
		// every load, which tore the callback down and up again).
		s_Buffer = std::make_unique<int[]>(s_BufferSize);

		if (!s_Buffer)
		{
			return 0;
		}
		// hotfix3 B-1: store-release so the audio-callback thread, which loads
		// these with acquire before touching the ring, sees the s_BufferStorage
		// allocation above as well as the freshly-zeroed cursors.
		s_BufferRead.store(0, std::memory_order_release);
		s_BufferWrite.store(0, std::memory_order_release);
		s_BufferIn.store(0, std::memory_order_release);
	}
 
	frmRateSampleAdj = (int)( ( ((double)soundrate) * getFrameRateAdjustmentRatio()) - ((double)soundrate) );
	//frmRateSampleAdj = 0;
	//printf("Sample Rate Adjustment: %+i\n", frmRateSampleAdj );

	fceu11::SetSoundVolume(soundvolume);
	fceu11::SetSoundQuality(soundq);
	fceu11::Sound(soundrate + frmRateSampleAdj);
	fceu11::SetTriangleVolume(soundtrianglevolume);
	fceu11::SetSquare1Volume(soundsquare1volume);
	fceu11::SetSquare2Volume(soundsquare2volume);
	fceu11::SetNoiseVolume(soundnoisevolume);
	fceu11::SetPCMVolume(soundpcmvolume);
	return 1;
}


/**
 * Returns the size of the audio buffer.
 */
uint32
GetMaxSound(void)
{
	return(s_BufferSize);
}

/**
 * Returns the rate the device was actually opened at, in Hz.
 *
 * v2.0 S2-b4 stage 3'. Added for a second machine whose resampler is built
 * around the rate: it has to be told the rate the *device* negotiated, not the
 * rate the configuration asked for, and the two differ whenever `InitSound`
 * fell back to a supported value.
 *
 * Reads the same static `InitSound` wrote, so a caller cannot see a different
 * rate than the one the buffer was sized against. Says nothing about which
 * machine is running -- that is for the caller's own bookkeeping.
 */
uint32
FCEUD_GetSoundRate(void)
{
	return(s_SampleRate);
}

/**
 * Returns the amount of free space in the audio buffer.
 */
uint32
GetWriteSound(void)
{
	return(s_BufferSize - s_BufferIn);
}

/**
 * Send a sound clip to the audio subsystem.
 */
void
WriteSound(int32 *buf,
           int Count)
{
	FCEU_PROFILE_FUNC(prof, "WriteSound"); // B2② 探点

	int uflowMode = 0;
	int ovrFlowSkip = 1;
	int udrFlowDup  = 1;
	static int skipCounter = 0;

	if ( (NoWaiting & 0x01) || turbo )
	{	// During Turbo mode, don't bother with sound as
		// overflowing the audio buffer can cause delays.
		return;
	}

	if ( g_fpsScale >= 0.99995 )
	{
		uflowMode = 0;
		ovrFlowSkip = (int)(g_fpsScale * 1000);

		if ( s_BufferIn >= s_BufferSize50 )
		{
			ovrFlowSkip += 1;
		}
	}
	else
	{
		udrFlowDup = (int)(1.0 / g_fpsScale);

		if ( udrFlowDup < 1 )
		{
			udrFlowDup = 1;
		}
		if ( s_BufferIn < s_BufferSize50 )
		{
			udrFlowDup++;
		}
		else if ( s_BufferIn > s_BufferSize75 )
		{
			udrFlowDup--;
		}
		uflowMode = (udrFlowDup > 1);
	}
	extern int EmulationPaused;
	if (EmulationPaused == 0)
	{
		int waitCount = 0;

		if ( uflowMode )
		{	// Underflow mode
			SDL_LockAudio();
			while (Count)
			{
				if ( s_BufferIn == s_BufferSize )
				{
					SDL_UnlockAudio();
					while (s_BufferIn == s_BufferSize) 
					{
						SDL_Delay(1); waitCount++;

						if ( waitCount > 1000 )
						{
							printf("Error: Sound sink is not draining... Breaking out of audio loop to prevent lockup.\n");
							return;
						}
					}
					SDL_LockAudio();
				}

				for (int i=0; i<udrFlowDup; i++)
				{
					s_Buffer[s_BufferWrite] = *buf;
					s_BufferWrite = (s_BufferWrite + 1) % s_BufferSize;
            
					s_BufferIn++;
				}
            
				Count--;
				buf++;
			}
			SDL_UnlockAudio();
		}
		else
		{
			if ( ovrFlowSkip <= 1000 )
			{	// Perfect one to one realtime
				skipCounter = 0;

				SDL_LockAudio();
				while (Count)
				{
					if (s_BufferIn == s_BufferSize) 
					{
						SDL_UnlockAudio();
						while (s_BufferIn == s_BufferSize) 
						{
							SDL_Delay(1); waitCount++;

							if ( waitCount > 1000 )
							{
								printf("Error: Sound sink is not draining... Breaking out of audio loop to prevent lockup.\n");
								return;
							}
						}
						SDL_LockAudio();
					}

					s_Buffer[s_BufferWrite] = *buf;
					Count--;
					s_BufferWrite = (s_BufferWrite + 1) % s_BufferSize;
            
					s_BufferIn++;
					buf++;
				}
				SDL_UnlockAudio();
			}
			else
			{	// Overflow mode
				SDL_LockAudio();
				while (Count)
				{
					if (s_BufferIn == s_BufferSize) 
					{
						SDL_UnlockAudio();
						while (s_BufferIn == s_BufferSize) 
						{
							SDL_Delay(1); waitCount++;

							if ( waitCount > 1000 )
							{
								printf("Error: Sound sink is not draining... Breaking out of audio loop to prevent lockup.\n");
								return;
							}
						}
						SDL_LockAudio();
					}

					//printf("%i >= %i \n", skipCounter, ovrFlowSkip );

					if ( skipCounter >= ovrFlowSkip )
					{
						s_Buffer[s_BufferWrite] = *buf;
						s_BufferWrite = (s_BufferWrite + 1) % s_BufferSize;
            
						s_BufferIn++;

						skipCounter -= ovrFlowSkip;
					}
					skipCounter = (skipCounter+1000);
            
					Count--;
					buf++;
				}
				SDL_UnlockAudio();
			}

		}
	}
}

/**
 * Pause (1) or unpause (0) the audio output.
 */
void
SilenceSound(int n)
{ 
	SDL_PauseAudio(n);   
}

/**
 * Shut down the audio subsystem.
 */
int
KillSound(void)
{
	fceu11::Sound(0);

	// v2.0.5 (PERF-01): a device open may still be running on the background
	// thread.  Join it before touching the SDL audio subsystem, otherwise
	// SDL_CloseAudio() could run concurrently with SDL_OpenAudio().
	{
		std::lock_guard<std::mutex> lk(s_audioOpenMutex);
		if (s_audioOpenThread.joinable())
			s_audioOpenThread.join();
	}

	SDL_CloseAudio();
	SDL_QuitSubSystem(SDL_INIT_AUDIO);
	s_Buffer.reset();
	s_BufferIn.store(0, std::memory_order_release);
	s_audioDeviceOpen.store(false, std::memory_order_release);
	s_audioOpenRate.store(0, std::memory_order_release);
	return 0;
}


/**
 * Adjust the volume either down (-1), up (1), or to the default (0).
 * Unmutes if mute was active before.
 */
void
fceWrapper_SoundVolumeAdjust(int n)
{
	int soundvolume;
	g_config->getOption("SDL.Sound.Volume", &soundvolume);

	switch(n) {
	case -1:
		soundvolume -= 10;
		if(soundvolume < 0) {
			soundvolume = 0;
		}
		break;
	case 0:
		soundvolume = 100;
		break;
	case 1:
		soundvolume += 10;
		if(soundvolume > 150) {
			soundvolume = 150;
		}
		break;
	}

	s_mute = false;
	g_config->setOption("SDL.Sound.Mute", s_mute);

	fceu11::SetSoundVolume(soundvolume);
	g_config->setOption("SDL.Sound.Volume", soundvolume);

	FCEU_DispMessage("Sound volume %d.",0, soundvolume);
}

/**
 * Toggles the sound on or off.
 */
void fceWrapper_SoundToggle(void)
{
	FCEUD_MuteSoundOutput( !s_mute );
}

bool FCEUD_SoundIsMuted(void)
{
	return s_mute;
}

void FCEUD_MuteSoundOutput( bool value )
{
	if (value != s_mute)
	{
		g_config->setOption("SDL.Sound.Mute", value);

		if (value)
		{
			FCEU_DispMessage("Sound mute on.",0);
		}
		else
		{
			FCEU_DispMessage("Sound mute off.",0);
		}
	}
	s_mute = value;
}

// This function is used by the GUI to mute sound when main window is not in focus.
void FCEUD_MuteSoundWindow( bool value )
{
	windowSoundMute = value;
}
