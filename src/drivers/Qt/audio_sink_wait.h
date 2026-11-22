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
/* FCEUX11 - audio sink wait policy
 *
 * The ring buffer in sdl-sound.cpp has exactly one consumer: the SDL audio
 * callback, which only exists while a device is open.  WriteSound() runs on the
 * emulation thread, inside FCEU_WRAPPER_LOCK, and used to wait for room by
 * calling SDL_Delay(1) up to 1000 times -- about a second -- whenever the ring
 * was full.  waitCount was local to the call, so a sink that stayed full made
 * every single frame pay that second: roughly one frame per second, with the
 * GUI locked out, i.e. "audio and video both crawl".
 *
 * PERF-02 (2026-11-18) closed the "no device at all" half of that by gating
 * WriteSound on s_audioDeviceOpen.  This file closes the other half: a device
 * that *is* open but stopped draining (endpoint removed after open, exclusive
 * mode takeover, driver stall, a suspended sink).  Two rules replace the spin:
 *
 *   1. never wait longer than one frame (~16 ms).  A transient full ring is
 *      normal back-pressure and resolves in a millisecond or two; a sink that
 *      is still full after a full frame is not going to drain.
 *   2. once that happens, latch "stalled" and DROP samples on the following
 *      calls instead of waiting at all.  The emulator keeps 60 fps and goes
 *      silent; when the ring drains again the latch clears by itself.
 *
 * Dropping is the correct behaviour for the same reason it was correct in
 * PERF-02: an emulator must not run slower because the host's audio output is
 * broken.  The samples' timing is derived from CPU cycles (sound.cpp), not from
 * the driver, so dropping them costs no emulation accuracy.
 *
 * Kept free of SDL/Qt so the policy can be locked on its own -- see
 * tests/f11qa/audio/sink_wait_test.cpp.
 */

#ifndef FCEUX11_AUDIO_SINK_WAIT_H
#define FCEUX11_AUDIO_SINK_WAIT_H

#include <functional>

namespace fceu11 {
namespace audio {

/// Free space in the ring buffer, clamped at zero.
///
/// GetWriteSound() used to be `s_BufferSize - s_BufferIn`, which underflows to
/// ~4 GiB when the producer has pushed s_BufferIn past s_BufferSize (the
/// underflow-duplication path in WriteSound increments it udrFlowDup times
/// without re-checking).  The GBA path feeds that value straight into
/// resize(), so the underflow was an out-of-memory abort, not a cosmetic bug.
inline unsigned int RingFree(unsigned int in, unsigned int size)
{
	return (in >= size) ? 0u : (size - in);
}

class SinkWaitGuard
{
public:
	enum class Verdict
	{
		Ready,    ///< there is room; the caller may write
		Stalled,  ///< no room and the sink is not draining; drop this frame
	};

	/// One frame of grace.  A ring that is still full after 16 ms of waiting
	/// is not jitter.
	static const int kDefaultBudgetMs = 16;

	explicit SinkWaitGuard(int budgetMs = kDefaultBudgetMs)
	    : m_budgetMs(budgetMs > 0 ? budgetMs : kDefaultBudgetMs)
	{
	}

	/// Called at the top of every write with the current ring state.  Clears
	/// the latch as soon as the ring is no longer full (the sink came back).
	void observe(unsigned int in, unsigned int size)
	{
		if (m_stalled && in < size)
			m_stalled = false;
	}

	/// Wait (bounded) until `hasRoom` or give up.  Never sleeps more than
	/// budgetMs times in one call, and never at all while latched.
	Verdict waitForRoom(const std::function<bool()> &hasRoom,
	                    const std::function<void()> &sleepMs)
	{
		if (m_stalled)
		{
			if (hasRoom())
			{
				m_stalled = false;
				return Verdict::Ready;
			}

			++m_droppedCalls;
			return Verdict::Stalled;
		}

		if (hasRoom())
			return Verdict::Ready;

		for (int waited = 0; waited < m_budgetMs; ++waited)
		{
			sleepMs();

			if (hasRoom())
				return Verdict::Ready;
		}

		m_stalled = true;
		++m_episodes;
		++m_droppedCalls;
		return Verdict::Stalled;
	}

	/// A freshly opened device has empty cursors: start from a clean slate.
	void reset()
	{
		m_stalled = false;
	}

	bool stalled() const { return m_stalled; }
	int budgetMs() const { return m_budgetMs; }
	int episodes() const { return m_episodes; }
	int droppedCalls() const { return m_droppedCalls; }

private:
	int m_budgetMs;
	bool m_stalled = false;
	int m_episodes = 0;
	int m_droppedCalls = 0;
};

} // namespace audio
} // namespace fceu11

#endif // FCEUX11_AUDIO_SINK_WAIT_H
