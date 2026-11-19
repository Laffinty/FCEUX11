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
#include "Qt/audio_probe_gate.h" // v2.0.5 (A1): device open/close state machine
#include "Qt/audio_sink_wait.h"  // v2.0.5 (stall): bounded ring-full wait policy
#include "../../profiler.h" // B2②: WriteSound 探点（OFF 态展开为空）

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <chrono>
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
//
// v2.0.5 (A1): the *close* side was the remaining hole.  KillSound() used to
// join that background thread unconditionally, and every KillSound() caller is
// on the GUI thread inside FCEU_WRAPPER_LOCK -- so a close that arrived while a
// probe was still running moved the rest of the probe (up to eight seconds on
// the machine above) back into the GUI thread.  The open/close state machine
// now lives in `audio_probe_gate.h`: a close is delegated to the probe thread
// instead of waited for, a probe that is still wanted is not thrown away, and
// SDL_CloseAudio() can still never run concurrently with SDL_OpenAudio().
// ---------------------------------------------------------------------------
static std::atomic<bool>         s_audioDeviceOpen{false};
static std::atomic<unsigned int> s_audioOpenRate{0};
static std::atomic<uint32_t>     s_audioFailAtMs{0};

// How long a failed endpoint probe is suppressed before it is retried.
static const uint32_t kAudioRetryBackoffMs = 60000;

// Milliseconds since SDL start (wraps every ~49 days; the unsigned
// subtraction below stays correct across the wrap).
static uint32_t AudioNowMs(void)
{
	return static_cast<uint32_t>(SDL_GetTicks());
}

/**
 * v2.0.5 (stall): the "ring is full" policy, shared by all three write modes.
 * See audio_sink_wait.h -- it tolerates a full ring for one frame, then drops
 * samples instead of waiting, and unlatches when the ring drains again.
 */
static fceu11::audio::SinkWaitGuard &
AudioSinkWait(void)
{
	static fceu11::audio::SinkWaitGuard guard;
	return guard;
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
 * Diagnostics hook for the PERF-01/A1 defect.
 *
 * The freeze showed up on hosts whose audio backend spends seconds discovering
 * that it has no usable endpoint, and it is invisible on a host whose probe
 * takes 300 ms -- so the fix cannot be demonstrated (or regressed against) by
 * ordinary use on most machines.  Setting
 *
 *     FCEUX11_TEST_AUDIO_OPEN_DELAY_MS=3000
 *
 * makes every device probe take at least that long.  Closing or reloading a ROM
 * (or touching an audio/video setting) while such a probe is running must NOT
 * freeze the window any more.  Unset -- the normal case -- this costs one
 * cached getenv() at startup.
 *
 * Same family as FCEUX11_AUDIO_TRACE (see audio_probe_gate.cpp).
 */
static int
AudioProbeTestDelayMs(void)
{
	static const int ms = []
	{
		const char *v = std::getenv("FCEUX11_TEST_AUDIO_OPEN_DELAY_MS");
		return v ? std::atoi(v) : 0;
	}();

	return ms;
}

/**
 * Diagnostics hook for the stalled-sink defect (v2.0.5 phase 2).
 *
 * "Device open but no longer draining" is easy to describe and hard to reach on
 * demand: it needs an endpoint to disappear after the device was opened.  With
 *
 *     FCEUX11_TEST_AUDIO_STALL=1
 *
 * the device is opened and then paused immediately, so the callback never
 * consumes the ring while s_audioDeviceOpen stays true -- exactly the state the
 * old code spun ~1 second per frame in.  Read once, unset by default.
 */
static bool
AudioProbeTestStall(void)
{
	static const bool on = []
	{
		const char *v = std::getenv("FCEUX11_TEST_AUDIO_STALL");
		return v != nullptr && v[0] != '\0' && v[0] != '0';
	}();
	return on;
}

/**
 * Probe the machine for an audio endpoint and open the device.
 *
 * Why a thread: SDL_OpenAudio() is not just a handle hand-out -- the
 * backend probes the machine for a usable endpoint, and on a host whose
 * default endpoint is missing the WASAPI backend spends eight seconds
 * discovering that (see the PERF-01 note above).  Paying that cost on the
 * GUI thread, inside the wrapper lock, is what made "load a ROM" look like
 * a ten-second freeze.
 *
 * This function runs on the probe thread (or, for a close, nothing here runs
 * at all).  It returns true when the device is open at `spec.freq`.
 */
static bool
AudioOpenDeviceNow(SDL_AudioSpec spec)
{
	bool ok = false;
	char firstErr[256] = { 0 };

	if (AudioProbeTestDelayMs() > 0)
	{
		std::this_thread::sleep_for(std::chrono::milliseconds(AudioProbeTestDelayMs()));
	}

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

		if (AudioProbeTestStall())
		{
			// Diagnostic: keep the device open but stop the consumer, so the
			// ring can never drain (see AudioProbeTestStall).
			SDL_PauseAudio(1);
			fprintf(stderr, "Sound: FCEUX11_TEST_AUDIO_STALL - device opened and paused\n");
		}
		// v2.0.5 (PERF-02): while no device was open WriteSound dropped its
		// samples, so the cursors may hold values from before the device was
		// closed (or the ring may look full).  Reset them *before* publishing
		// s_audioDeviceOpen: the emulation thread gates on that flag with
		// acquire, so it can never observe a stale full ring that this device
		// will not drain.
		s_BufferRead.store(0, std::memory_order_release);
		s_BufferWrite.store(0, std::memory_order_release);
		s_BufferIn.store(0, std::memory_order_release);
		s_audioOpenRate.store(static_cast<unsigned int>(spec.freq), std::memory_order_release);
		s_audioDeviceOpen.store(true, std::memory_order_release);
		s_audioFailAtMs.store(0, std::memory_order_release);

		// v2.0.5 (stall): a fresh device starts with empty cursors, so a latch
		// left over from the previous one must not make us deaf to it.
		AudioSinkWait().reset();

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

	return ok;
}

/**
 * Close the device.  Runs on whichever thread performed the close -- the
 * caller when nothing was in flight, otherwise the probe thread, which is the
 * only way to keep SDL_CloseAudio() from running concurrently with the
 * SDL_OpenAudio() that precedes it.
 */
static void
AudioCloseDeviceNow(void)
{
	SDL_CloseAudio();
	SDL_QuitSubSystem(SDL_INIT_AUDIO);
	s_audioDeviceOpen.store(false, std::memory_order_release);
	s_audioOpenRate.store(0, std::memory_order_release);
}

/**
 * The open/close state machine.  A function-local static so that its
 * destructor joins the probe thread before static teardown finishes: a
 * device opened outside DriverInitialize (the sound dialog's "enable" path)
 * is never closed by DriverKill, and a joinable std::thread would otherwise
 * call std::terminate() on the way out.
 */
struct AudioProbeHolder
{
	fceu11::audio::AudioProbeGate gate;

	AudioProbeHolder()
	{
		gate.setCloseHandler(AudioCloseDeviceNow);
	}
};

static fceu11::audio::AudioProbeGate &
AudioProbe(void)
{
	static AudioProbeHolder holder;
	return holder.gate;
}

/**
 * Make sure an audio device is (being) opened for the given sample rate.
 *
 * Returns true when a device with that rate is already open (the caller
 * must then NOT touch the ring buffer -- the audio callback thread is
 * reading it), false when the device is not ready yet (it is being opened
 * on the probe thread, or the last attempt failed and is still inside
 * the backoff window).
 *
 * Never waits for the backend.
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

	const uint32_t now = AudioNowMs();
	const uint32_t failAt = s_audioFailAtMs.load(std::memory_order_acquire);
	if (failAt != 0 && (now - failAt) < kAudioRetryBackoffMs)
	{
		// The endpoint probe failed recently.  Stay quiet until the window
		// expires instead of stalling every ROM load for the same seconds.
		return false;
	}

	// The gate decides what an in-flight probe means: the same rate is
	// reused (a close requested in between is cancelled), a different rate is
	// queued behind it.  Either way this call returns immediately.
	const SDL_AudioSpec captured = spec;

	AudioProbe().requestOpen(static_cast<unsigned int>(spec.freq),
	                         [captured]() { return AudioOpenDeviceNow(captured); });

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
		// v2.0.5 (A1): never replace the ring under a live audio callback.
		// Before the gate, KillSound() had already joined the probe and closed
		// the device by the time this ran, so there was no consumer.  A close
		// can now be *delegated*, so in principle a device can still be open
		// here -- that needs a rate change that was not preceded by a
		// KillSound(), which no call site does, but keeping the rule explicit
		// is cheaper than relying on that.  The probe that is in flight closes
		// the device and resets the cursors when it reopens.
		if (!AudioProbe().isOpen())
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
 * True while an audio device is open (the ring buffer has a consumer).
 *
 * v2.0.5 (PERF-02): see the gate at the top of WriteSound.
 */
bool
FCEUD_SoundDeviceIsOpen(void)
{
	return s_audioDeviceOpen.load(std::memory_order_acquire);
}

/**
 * Returns the amount of free space in the audio buffer.
 *
 * v2.0.5 (A2-stall): clamped.  This used to be `s_BufferSize - s_BufferIn`,
 * which underflows to ~4 GiB once the producer has pushed s_BufferIn past
 * s_BufferSize -- and the GBA path feeds this value straight into resize().
 */
uint32
GetWriteSound(void)
{
	return fceu11::audio::RingFree(s_BufferIn.load(std::memory_order_acquire), s_BufferSize);
}

/**
 * The ring buffer is full.  Give the sink one frame's worth of grace, then stop
 * waiting and report failure so the caller drops this frame's samples.
 *
 * v2.0.5 (stall): this replaces three copies of
 *
 *     while (s_BufferIn == s_BufferSize) { SDL_Delay(1); if (++waitCount > 1000) return; }
 *
 * which cost about a second per call -- per *frame*, because waitCount was
 * local -- whenever the device was open but nothing drained it.  Callers must
 * hold no SDL audio lock here: the wait happens unlocked.
 */
/**
 * True when FCEUX11_AUDIO_TRACE is set: every dropped frame gets its own line,
 * which is also a frame counter for a stalled-sink run ("N lines in T seconds
 * = N frames in T seconds").  Without it the warning is once per episode.
 */
static bool
AudioTraceStallDrops(void)
{
	static const bool on = []
	{
		const char *v = std::getenv("FCEUX11_AUDIO_TRACE");
		return v != nullptr && v[0] != '\0' && v[0] != '0';
	}();
	return on;
}

static bool
AudioWaitForRingRoom(void)
{
	const bool ready =
	    AudioSinkWait().waitForRoom(
	        []() { return s_BufferIn.load(std::memory_order_acquire) < s_BufferSize; },
	        []() { SDL_Delay(1); }) == fceu11::audio::SinkWaitGuard::Verdict::Ready;

	if (!ready)
	{
		// Once per episode, not once per dropped frame: a sink that is gone
		// stays gone, and a message per frame would bury the log while the
		// emulator happily keeps running at full speed.
		static int loggedEpisode = 0;
		const int episode = AudioSinkWait().episodes();

		if (AudioTraceStallDrops())
		{
			fprintf(stderr, "[audio-sink] dropped one frame (episode %d, total %d)\n",
			        episode, AudioSinkWait().droppedCalls());
		}
		else if (episode != loggedEpisode)
		{
			loggedEpisode = episode;
			fprintf(stderr,
			        "Warning: Sound sink is not draining; dropping samples to keep the "
			        "emulator running (episode %d).\n",
			        episode);
		}
	}

	return ready;
}

/**
 * Send a sound clip to the audio subsystem.
 */
void
WriteSound(int32 *buf,
           int Count)
{
	FCEU_PROFILE_FUNC(prof, "WriteSound"); // B2② 探点

	// v2.0.5 (PERF-02): a closed sink must never cost the emulation thread a
	// single microsecond.
	//
	// The ring buffer's only consumer is the SDL audio callback, which exists
	// only while a device is open.  With no device (host whose USB sound card
	// is unplugged/powered off -- the owner's report of 2026-11-18, or a CI
	// runner, which has no audio endpoint at all) nothing ever drains the ring,
	// so s_BufferIn climbs to s_BufferSize and stays there.  Every subsequent
	// call then entered the "sound sink is not draining" spin below: one
	// SDL_Delay(1) millisecond at a time until waitCount passed 1000, then a
	// printf and a return -- and waitCount is local, so the next call paid the
	// full second again.  Net effect: roughly one frame per second, which is
	// exactly "the ROM loaded but everything is frozen and the keyboard does
	// nothing".  (It also dereferenced the released ring after KillSound,
	// because s_BufferSize keeps its old value while s_Buffer is null.)
	//
	// Dropping the samples is the correct behaviour: no device means no sink,
	// and an emulator must not run slower because the host has no sound card.
	// The ring is re-zeroed when a device opens later (see the worker below).
	if (!s_audioDeviceOpen.load(std::memory_order_acquire))
	{
		return;
	}

	// v2.0.5 (stall): clear the latch as soon as the ring is no longer full --
	// the sink came back and this frame can wait again if it has to.
	AudioSinkWait().observe(s_BufferIn.load(std::memory_order_acquire), s_BufferSize);

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
		if ( uflowMode )
		{	// Underflow mode
			SDL_LockAudio();
			while (Count)
			{
				if ( s_BufferIn == s_BufferSize )
				{
					SDL_UnlockAudio();
					if ( !AudioWaitForRingRoom() )
						return;
					SDL_LockAudio();
				}

				for (int i=0; i<udrFlowDup; i++)
				{
					s_Buffer[s_BufferWrite] = *buf;
					s_BufferWrite = (s_BufferWrite + 1) % s_BufferSize;
            
					s_BufferIn++;
				}

				// v2.0.5 (stall): the duplication loop can push the count past
				// the end within one call; clamp it so the fill level -- and
				// GetWriteSound's arithmetic -- stay inside the ring.
				if ( s_BufferIn > s_BufferSize )
				{
					s_BufferIn = s_BufferSize;
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
						if ( !AudioWaitForRingRoom() )
							return;
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
						if ( !AudioWaitForRingRoom() )
							return;
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
 *
 * v2.0.5 (A1): this used to join the probe thread unconditionally.  Every
 * caller is on the GUI thread inside FCEU_WRAPPER_LOCK (CloseGame ->
 * DriverKill, and the three settings dialogs), so a close that arrived while
 * a probe was still running blocked the GUI for the rest of the probe -- up
 * to eight seconds on a host whose audio endpoint is slow to fail.  The close
 * is now delegated to the probe thread instead (`requestClose()` returns
 * false), which keeps SDL_CloseAudio() from running concurrently with
 * SDL_OpenAudio() without making anyone wait.
 *
 * The published flags are cleared first either way: WriteSound must stop
 * feeding a ring that is about to lose its consumer.  The ring buffer itself
 * is only released when this call performed the close itself -- a delegated
 * close may still be followed by an InitSound() (the dialog pattern is
 * `KillSound(); InitSound();`), which reallocates it.
 */
int
KillSound(void)
{
	fceu11::Sound(0);

	s_audioDeviceOpen.store(false, std::memory_order_release);
	s_audioOpenRate.store(0, std::memory_order_release);

	if (AudioProbe().requestClose())
	{
		s_Buffer.reset();
		s_BufferIn.store(0, std::memory_order_release);
	}

	return 0;
}

/**
 * Exit path only: wait for the audio probe thread to finish, so the caller can
 * tear SDL down without racing an SDL_OpenAudio() that is still running.
 *
 * Blocking by design -- the process is on its way out.  It also joins a probe
 * that was started outside DriverInitialize and therefore never closed by
 * DriverKill (see AudioProbeHolder).
 */
void
WaitSoundProbeIdle(void)
{
	AudioProbe().waitIdle();
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
