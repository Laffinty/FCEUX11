/*
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
/* FCEUX11 - audio device probe gate
 *
 * Why this file exists
 * --------------------
 * v2.0.5 (PERF-01, 36af048) moved SDL_OpenAudio() to a background thread so a
 * host whose audio backend spends seconds discovering that it has no usable
 * endpoint would not freeze the GUI while loading a ROM.  The device open is
 * still worth doing off the GUI thread; what went wrong is how the *close* was
 * handled: KillSound() joined that thread unconditionally, so whenever a probe
 * was still running the caller (always the GUI thread, always inside
 * FCEU_WRAPPER_LOCK) paid the remaining probe time as a window freeze.
 *
 * Skipping the join is not enough on its own -- it introduces two new defects:
 *   1) SDL_CloseAudio() would then run concurrently with SDL_OpenAudio();
 *   2) the callers that do `KillSound(); InitSound();` (the three settings
 *      dialogs) would leave the device closed and never reopened, or would
 *      queue a second full-length probe.
 *
 * So teardown is *delegated*: requestClose() never waits.  When a probe is in
 * flight it only records the request, and the probe thread performs the close
 * as soon as its SDL call returns.  A requestClose() with nothing in flight
 * closes inline and says so, which lets the caller (which owns the ring buffer)
 * do its own cleanup only when that cleanup is safe.
 *
 * The class is deliberately free of Qt/SDL/config: the only things it knows how
 * to do are call `open` on a worker thread and `close` when asked.  That keeps
 * the regression lock cheap (two translation units, no emulator core) and makes
 * the state machine testable on its own -- see
 * tests/f11qa/audio/audio_probe_gate_test.cpp.
 */

#ifndef FCEUX11_AUDIO_PROBE_GATE_H
#define FCEUX11_AUDIO_PROBE_GATE_H

#include <atomic>
#include <functional>
#include <mutex>
#include <thread>

namespace fceu11 {
namespace audio {

class AudioProbeGate
{
public:
	/// Runs on the worker thread.  Returns true when the device is open.
	/// Must be the only place that talks to the audio backend while opening.
	using OpenFn = std::function<bool()>;

	/// Runs on whichever thread performed the close (worker or caller).
	/// Must be idempotent: it can be called when nothing is open.
	using CloseFn = std::function<void()>;

	enum class Request
	{
		Opened,           ///< already open at the requested rate; reuse it
		OpeningSameRate,  ///< that exact attempt is in flight (teardown cancelled)
		Queued,           ///< another rate is in flight; this runs afterwards
		Started,          ///< nothing was in flight, a new attempt was started
	};

	AudioProbeGate() = default;
	~AudioProbeGate();

	AudioProbeGate(const AudioProbeGate &) = delete;
	AudioProbeGate &operator=(const AudioProbeGate &) = delete;

	void setCloseHandler(CloseFn fn);

	/// Ask for a device at `rate`.  Never waits.
	Request requestOpen(unsigned int rate, OpenFn open);

	/// Ask for the device to go away.  Never waits while a probe is in flight.
	/// Returns true when the close was performed synchronously (nothing was in
	/// flight) -- only then may the caller release the ring buffer.
	bool requestClose();

	/// Exit path only: cancel any queued open and wait for the probe thread to
	/// finish, so the caller can tear SDL down without racing it.  Blocking.
	void waitIdle();

	bool isOpen() const;
	bool isOpenAt(unsigned int rate) const;
	unsigned int openRate() const;
	bool isInFlight() const;

private:
	void workerLoop(unsigned int rate, OpenFn open);
	void closeDevice();
	void trace(const char *fmt, ...) const;

	void queueLocked(unsigned int rate, OpenFn open);
	bool takePendingLocked(unsigned int &rate, OpenFn &open);

	std::mutex m_mutex;          // protects the state transitions below
	std::mutex m_sdlMutex;       // serialises open/close against each other

	std::atomic<bool> m_inFlight{false};
	std::atomic<unsigned int> m_openRate{0};

	unsigned int m_inFlightRate = 0;
	bool m_teardown = false;               // guarded by m_mutex
	unsigned int m_pendingRate = 0;        // guarded by m_mutex, depth 1
	OpenFn m_pendingOpen;                  // guarded by m_mutex

	CloseFn m_close;
	std::thread m_thread;                  // guarded by m_mutex
};

} // namespace audio
} // namespace fceu11

#endif // FCEUX11_AUDIO_PROBE_GATE_H
