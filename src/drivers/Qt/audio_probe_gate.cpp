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
/* FCEUX11 - audio device probe gate.  See audio_probe_gate.h for why. */

#include "audio_probe_gate.h"

#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <chrono>
#include <utility>

namespace fceu11 {
namespace audio {

static bool traceEnabled()
{
	static const bool on = []
	{
		const char *v = std::getenv("FCEUX11_AUDIO_TRACE");
		return v != nullptr && v[0] != '\0' && v[0] != '0';
	}();
	return on;
}

// Milliseconds since the first trace call.  Without a clock the trace says what
// happened but not when, which is useless for "did the close land inside the
// probe or after it".
static unsigned long long traceNowMs()
{
	static const auto t0 = std::chrono::steady_clock::now();
	return static_cast<unsigned long long>(
	    std::chrono::duration_cast<std::chrono::milliseconds>(
	        std::chrono::steady_clock::now() - t0).count());
}

void AudioProbeGate::trace(const char *fmt, ...) const
{
	if (!traceEnabled())
		return;

	char buf[256];

	va_list ap;
	va_start(ap, fmt);
	std::vsnprintf(buf, sizeof(buf), fmt, ap);
	va_end(ap);

	std::fprintf(stderr, "[audio-gate] +%llu ms %s\n", traceNowMs(), buf);
	std::fflush(stderr);
}

void AudioProbeGate::setCloseHandler(CloseFn fn)
{
	std::lock_guard<std::mutex> lk(m_mutex);
	m_close = std::move(fn);
}

bool AudioProbeGate::isOpen() const
{
	return m_openRate.load(std::memory_order_acquire) != 0;
}

bool AudioProbeGate::isOpenAt(unsigned int rate) const
{
	return rate != 0 && m_openRate.load(std::memory_order_acquire) == rate;
}

unsigned int AudioProbeGate::openRate() const
{
	return m_openRate.load(std::memory_order_acquire);
}

bool AudioProbeGate::isInFlight() const
{
	return m_inFlight.load(std::memory_order_acquire);
}

void AudioProbeGate::queueLocked(unsigned int rate, OpenFn open)
{
	m_pendingRate = rate;
	m_pendingOpen = std::move(open);
}

bool AudioProbeGate::takePendingLocked(unsigned int &rate, OpenFn &open)
{
	if (m_pendingRate == 0)
		return false;

	rate = m_pendingRate;
	open = std::move(m_pendingOpen);
	m_pendingRate = 0;
	m_pendingOpen = nullptr;
	return true;
}

void AudioProbeGate::closeDevice()
{
	CloseFn close;

	{
		std::lock_guard<std::mutex> lk(m_mutex);
		close = m_close;
	}

	if (close)
	{
		std::lock_guard<std::mutex> sdlLock(m_sdlMutex);
		close();
	}

	m_openRate.store(0, std::memory_order_release);
}

void AudioProbeGate::workerLoop(unsigned int rate, OpenFn open)
{
	bool haveAttempt = true;

	for (;;)
	{
		// ---- A: honour a teardown that arrived before or during the probe ----
		//
		// Deferring it to here is the whole point: the caller never waits for
		// the backend, and the close still cannot run concurrently with an
		// open, because this thread is the one that opened it.
		bool closeNow = false;
		{
			std::lock_guard<std::mutex> lk(m_mutex);

			if (m_teardown)
			{
				m_teardown = false;
				m_openRate.store(0, std::memory_order_release);
				closeNow = true;
			}
		}

		if (closeNow)
		{
			closeDevice();
			trace("delegated teardown performed");
		}

		// ---- B: pick up a request queued while we were busy ----
		if (!haveAttempt)
		{
			std::lock_guard<std::mutex> lk(m_mutex);

			haveAttempt = takePendingLocked(rate, open);

			if (haveAttempt)
				m_inFlightRate = rate;
		}

		// ---- C: nothing left -- go idle, unless a request landed in between --
		if (!haveAttempt)
		{
			bool idle = false;
			{
				std::lock_guard<std::mutex> lk(m_mutex);

				if (!m_teardown && m_pendingRate == 0)
				{
					m_inFlightRate = 0;
					m_inFlight.store(false, std::memory_order_release);
					idle = true;
				}
			}

			if (idle)
			{
				trace("probe thread idle");
				return;
			}

			continue;
		}

		// ---- D: the probe itself.  Nothing is held across it ----
		bool ok = false;
		{
			std::lock_guard<std::mutex> sdlLock(m_sdlMutex);
			ok = open ? open() : false;
		}
		haveAttempt = false;

		{
			std::lock_guard<std::mutex> lk(m_mutex);

			if (m_teardown)
			{
				// Closed by phase A on the next pass.  Leave m_openRate alone:
				// it must never claim "open" for a device this thread is about
				// to close.
			}
			else
			{
				m_openRate.store(ok ? rate : 0u, std::memory_order_release);

				if (ok && m_pendingRate == rate)
				{
					// This attempt is exactly what the queued request wanted;
					// do not probe a second time.
					m_pendingRate = 0;
					m_pendingOpen = nullptr;
				}
			}
		}
	}
}

AudioProbeGate::Request AudioProbeGate::requestOpen(unsigned int rate, OpenFn open)
{
	std::lock_guard<std::mutex> lk(m_mutex);

	if (rate == 0)
	{
		// Not a rate a device can be opened at; nothing to do.
		return Request::Opened;
	}

	if (!m_inFlight.load(std::memory_order_acquire) &&
	    m_openRate.load(std::memory_order_acquire) == rate)
	{
		// Already open at this rate: reuse it.  This is the branch the v2.0.5
		// note called "reuse an already-open device" -- it is reachable
		// whenever a device was opened outside DriverInitialize (the sound
		// dialog's "enable" path does exactly that), so it must stay.
		trace("reuse: device already open at %u Hz", rate);
		return Request::Opened;
	}

	if (m_inFlight.load(std::memory_order_acquire))
	{
		// A probe is running.  Record what we want.
		//
		// Same rate: the attempt in flight is exactly what was asked for, so
		// cancel a teardown requested in the meantime -- otherwise the device
		// would be closed and then probed a second time for nothing (this is
		// the `KillSound(); InitSound();` dialog pattern).
		//
		// Different rate: the teardown must STAND.  SDL_OpenAudio() cannot
		// open a device that is already open, so the close has to happen
		// between the two attempts; the probe loop runs this request after it
		// (queue depth 1, last request wins).
		const bool sameRate = (m_inFlightRate == rate);

		if (sameRate)
			m_teardown = false;

		if (m_pendingRate != 0 && m_pendingRate != rate)
			trace("queued open at %u Hz supersedes %u Hz", rate, m_pendingRate);

		queueLocked(rate, std::move(open));

		return sameRate ? Request::OpeningSameRate : Request::Queued;
	}

	if (m_thread.joinable())
		m_thread.join(); // previous attempt has finished; this is microseconds

	m_inFlightRate = rate;
	m_inFlight.store(true, std::memory_order_release);
	m_teardown = false;

	try
	{
		m_thread = std::thread(&AudioProbeGate::workerLoop, this, rate, std::move(open));
	}
	catch (...)
	{
		m_inFlightRate = 0;
		m_inFlight.store(false, std::memory_order_release);
		throw;
	}

	trace("probe started for %u Hz", rate);
	return Request::Started;
}

bool AudioProbeGate::requestClose()
{
	{
		std::lock_guard<std::mutex> lk(m_mutex);

		if (m_inFlight.load(std::memory_order_acquire))
		{
			// Hand the close to the probe thread.  Drop a queued open: the
			// caller asked for the device to go away, and a follow-up
			// requestOpen() will queue its own.
			m_teardown = true;
			m_pendingRate = 0;
			m_pendingOpen = nullptr;
			trace("close delegated to the probe thread");
			return false;
		}
	}

	closeDevice();
	trace("close performed inline");
	return true;
}

void AudioProbeGate::waitIdle()
{
	{
		std::lock_guard<std::mutex> lk(m_mutex);

		m_pendingRate = 0;
		m_pendingOpen = nullptr;

		if (m_inFlight.load(std::memory_order_acquire))
		{
			m_teardown = true;
			trace("waitIdle: waiting for the probe thread");
		}
	}

	// The join cannot be done under m_mutex: the worker needs that lock to
	// reach its idle state.  This is the documented exit-only entry point, so
	// there is no concurrent creator of m_thread to race with.
	if (m_thread.joinable())
		m_thread.join();
}

AudioProbeGate::~AudioProbeGate()
{
	// Safety net for the case that made this class necessary: a device opened
	// outside DriverInitialize (the sound dialog's "enable" path) and therefore
	// never closed by DriverKill.  Without this join the joinable static
	// std::thread would call std::terminate during static destruction.
	waitIdle();
}

} // namespace audio
} // namespace fceu11
