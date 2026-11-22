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
/* FCEUX11 - scoped SDL_AUDIODRIVER fallback policy (v2.0.5 A3)
 *
 * Why this file exists
 * --------------------
 * The audio probe's fallback loop writes SDL_AUDIODRIVER through SDL_setenv()
 * so a broken WASAPI install can still produce sound through directsound or
 * winmm.  SDL_setenv() changes the *process* environment: the value survives
 * the probe, is inherited by child processes, and is re-read by every later
 * SDL_InitSubSystem(SDL_INIT_AUDIO).  Net effect after one fallback: the
 * WASAPI backend is never tried again inside this process, so a sound card
 * that comes back (a USB amp powered on) stays silent until the emulator is
 * restarted.  The default/WASAPI path does not recover; only the pinned
 * fallback driver keeps working.
 *
 * The fix has two halves, both owned here so the regression lock can run
 * without SDL:
 *
 *   1. The override is *scoped to one probe attempt*.  The caller captures
 *      the variable before it touches it (`capture()`) and hands the exact
 *      previous state back when the attempt is over (`restore()`), whether
 *      that is a value or an absence.  The environment never remembers us.
 *
 *   2. The memory of "which fallback driver worked" becomes *in-process
 *      state* (`rememberSuccess()` / `sticky()` / `dropSticky()`) instead of
 *      environment state.  The next probe tries the sticky driver first, so
 *      a broken-WASAPI machine does not re-pay that backend's seconds-long
 *      probe on every reopen -- and when the sticky driver stops working it
 *      is dropped, so a machine whose directsound broke while its WASAPI
 *      recovered finds its way back to the default chain.
 *
 * Pure std, no Qt/SDL/config: the environment itself is injected, the same
 * shape as audio_probe_gate.h and audio_sink_wait.h.
 */

#ifndef FCEUX11_AUDIO_DRIVER_FALLBACK_H
#define FCEUX11_AUDIO_DRIVER_FALLBACK_H

#include <functional>
#include <string>
#include <utility>

namespace fceu11 {
namespace audio {

class AudioDriverFallbackPolicy
{
public:
	/// Reads the driver variable ("" when it is absent).
	using GetFn = std::function<std::string()>;
	/// Writes the driver variable.
	using SetFn = std::function<void(const std::string &)>;
	/// Removes the driver variable.
	using UnsetFn = std::function<void()>;

	AudioDriverFallbackPolicy(GetFn get, SetFn set, UnsetFn unset)
	    : m_get(std::move(get)), m_set(std::move(set)), m_unset(std::move(unset))
	{
	}

	/// Snapshot the variable exactly as the attempt found it.  Call once per
	/// probe attempt, before anything may write the variable.
	void capture()
	{
		m_saved = m_get();
		m_savedValid = true;
	}

	/// Hand the snapshot back: the same value if there was one, the same
	/// absence if there was not.  Idempotent -- only the first call after a
	/// capture does anything.
	void restore()
	{
		if (!m_savedValid)
			return;
		if (m_saved.empty())
			m_unset();
		else
			m_set(m_saved);
		m_savedValid = false;
		m_saved.clear();
	}

	/// The fallback driver remembered from a previous probe, or "" when the
	/// default chain should be used.
	const std::string &sticky() const
	{
		return m_sticky;
	}

	/// Record `driver` as the fallback that opened the device.  Only for an
	/// explicit fallback driver -- never for the default (empty) attempt.
	void rememberSuccess(const std::string &driver)
	{
		if (!driver.empty())
			m_sticky = driver;
	}

	/// Forget the sticky driver: it stopped working, and the default chain
	/// (including WASAPI, which may have recovered) must be tried again.
	void dropSticky()
	{
		m_sticky.clear();
	}

private:
	GetFn m_get;
	SetFn m_set;
	UnsetFn m_unset;

	std::string m_saved;
	bool m_savedValid = false;

	std::string m_sticky;
};

} // namespace audio
} // namespace fceu11

#endif // FCEUX11_AUDIO_DRIVER_FALLBACK_H
