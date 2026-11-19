/* FCEUX11 - lock test for the scoped SDL_AUDIODRIVER fallback policy.
 *
 * Regression being locked
 * ----------------------
 * v2.0.5 audit item A3: the audio probe's fallback loop wrote
 * SDL_AUDIODRIVER with SDL_setenv(..., 1) and never restored it.  The
 * variable is process-global, inherited by child processes, and re-read by
 * every later SDL_InitSubSystem(SDL_INIT_AUDIO) -- so after one fallback the
 * WASAPI backend was never tried again inside this process, and a sound card
 * that came back (a USB amp powered on) stayed silent until restart.  Only
 * the pinned fallback driver kept working; the default path could not
 * recover.
 *
 * The fix (src/drivers/Qt/audio_driver_fallback.h) has two halves, both
 * locked here:
 *   1. the override is scoped to one probe attempt -- captured before, handed
 *      back (value or absence) after, so the environment never remembers us;
 *   2. the memory of "which fallback driver worked" is in-process sticky
 *      state, tried first on the next probe and dropped the moment it stops
 *      working.
 *
 * Legs
 * ----
 *  T1  capture-restore with an absent variable: restore() removes (unset),
 *      so the absence is reproduced, not replaced by ""
 *  T2  capture-restore with a pre-existing value: the same value comes back
 *  T3  restore() is idempotent -- only the first call after a capture acts
 *  T4  rememberSuccess records a fallback driver, ignores "", and the next
 *      rememberSuccess overwrites
 *  T5  dropSticky forgets the driver so the default chain runs again
 *  T6  a capture-restore with no writes leaves the variable untouched
 *  T7  source guard on sdl-sound.cpp: the snapshot is taken before the first
 *      open attempt and the restore comes after the last one (the override
 *      outlives nothing), and the sticky wiring (rememberSuccess/dropSticky)
 *      is still there
 *
 * Mutation legs (each verified by hand; see the plan, section 5):
 *  M1  drop the final restore()                              -> T7 red
 *  M2  move the final restore() before the fallback loop     -> T7 red
 *  M3  make restore() set "" instead of unsetting            -> T1 red
 *  M4  make capture() skip the snapshot                      -> T2 red
 *  M5  make rememberSuccess() ignore its argument            -> T4 red
 *  M6  make dropSticky() a no-op                             -> T5 red
 *
 * No emulator core, no Qt, no SDL: one translation unit, so this stays cheap
 * enough to run on every ctest invocation.
 */

#include "Qt/audio_driver_fallback.h"

#include <cstdio>
#include <fstream>
#include <map>
#include <string>
#include <utility>

using fceu11::audio::AudioDriverFallbackPolicy;

static int g_failures = 0;

#define CHECK(cond)                                                            \
	do                                                                         \
	{                                                                          \
		if (!(cond))                                                           \
		{                                                                      \
			std::printf("    FAIL %s:%d  %s\n", __FILE__, __LINE__, #cond);     \
			++g_failures;                                                      \
		}                                                                      \
	} while (0)

namespace {

/// A fake environment: the variable's value ("" = absent) plus counters so a
/// leg can pin *how* the policy touched it, not only what it reads back.
struct FakeEnv
{
	std::string value; // "" means absent
	int setCalls = 0;
	int unsetCalls = 0;
	std::string lastSet;

	AudioDriverFallbackPolicy policy()
	{
		return AudioDriverFallbackPolicy(
		    [this]() -> std::string { return value; },
		    [this](const std::string &v)
		    {
			    value = v;
			    lastSet = v;
			    ++setCalls;
		    },
		    [this]()
		    {
			    value.clear();
			    ++unsetCalls;
		    });
	}
};

} // namespace

static void T1_absent_variable_is_restored_to_absence()
{
	std::printf("T1  an absent SDL_AUDIODRIVER is restored to absence, not \"\"\n");

	FakeEnv env;
	AudioDriverFallbackPolicy p = env.policy(); // value == "" (absent)

	p.capture();
	p.rememberSuccess("directsound");
	p.restore();

	CHECK(env.value.empty());
	CHECK(env.unsetCalls == 1);
	CHECK(env.setCalls == 0);
}

static void T2_preexisting_value_is_restored_exactly()
{
	std::printf("T2  a pre-existing SDL_AUDIODRIVER is handed back exactly\n");

	FakeEnv env;
	env.value = "winmm";
	AudioDriverFallbackPolicy p = env.policy();

	p.capture();
	p.rememberSuccess("directsound");
	p.restore();

	CHECK(env.value == "winmm");
	CHECK(env.setCalls == 1);
	CHECK(env.lastSet == "winmm");
	CHECK(env.unsetCalls == 0);
}

static void T3_restore_is_idempotent()
{
	std::printf("T3  restore() acts only on the first call after a capture\n");

	FakeEnv env;
	AudioDriverFallbackPolicy p = env.policy();

	p.capture();
	p.restore();
	p.restore();
	p.restore();

	CHECK(env.unsetCalls == 1);
}

static void T4_sticky_records_and_overwrites()
{
	std::printf("T4  rememberSuccess records the working fallback driver\n");

	FakeEnv env;
	AudioDriverFallbackPolicy p = env.policy();

	CHECK(p.sticky().empty());

	p.rememberSuccess("directsound");
	CHECK(p.sticky() == "directsound");

	p.rememberSuccess(""); // not a driver: ignored
	CHECK(p.sticky() == "directsound");

	p.rememberSuccess("winmm"); // a later fallback replaces it
	CHECK(p.sticky() == "winmm");
}

static void T5_drop_sticky_falls_back_to_the_default_chain()
{
	std::printf("T5  dropSticky forgets the driver so the default chain runs again\n");

	FakeEnv env;
	AudioDriverFallbackPolicy p = env.policy();

	p.rememberSuccess("directsound");
	CHECK(!p.sticky().empty());

	p.dropSticky();
	CHECK(p.sticky().empty());

	// And the policy can learn again afterwards (a new fallback still sticks).
	p.rememberSuccess("winmm");
	CHECK(p.sticky() == "winmm");
}

static void T6_untouched_attempt_leaves_the_variable_alone()
{
	std::printf("T6  capture-restore with no writes leaves the value as found\n");

	FakeEnv env;
	env.value = "directsound";
	AudioDriverFallbackPolicy p = env.policy();

	p.capture();
	p.restore();

	CHECK(env.value == "directsound");
	CHECK(env.setCalls == 1); // rewritten with the same value, not dropped
	CHECK(env.lastSet == "directsound");
	CHECK(env.unsetCalls == 0);
}

// The policy can only enforce the contract it owns; whether the probe really
// scopes its override lives in sdl-sound.cpp.  Three things must hold in
// AudioOpenDeviceNow:
//   * the snapshot (.capture()) precedes the first open attempt,
//   * a restore follows the LAST open attempt (the mid-flow restore on the
//     sticky-failure path does not satisfy this -- that is the point), and
//   * the sticky wiring is still there (rememberSuccess / dropSticky).
// Any regression turns this leg red.
static std::string functionBody(const std::string &src, const std::string &marker)
{
	const size_t at = src.find(marker);

	if (at == std::string::npos)
		return std::string();

	const size_t end = src.find("\n}", at);

	if (end == std::string::npos)
		return std::string();

	return src.substr(at, end - at);
}

static void T7_source_guard_probe_scopes_the_override()
{
	std::printf("T7  sdl-sound.cpp scopes the SDL_AUDIODRIVER override to the attempt\n");

	std::ifstream in("../src/drivers/Qt/sdl-sound.cpp");
	if (!in.is_open())
	{
		std::printf("    FAIL could not open ../src/drivers/Qt/sdl-sound.cpp\n");
		std::printf("         (ctest must run with WORKING_DIRECTORY=tests/)\n");
		++g_failures;
		return;
	}

	std::string src((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());

	const std::string body = functionBody(src, "AudioOpenDeviceNow(SDL_AudioSpec spec)");
	CHECK(!body.empty());

	if (body.empty())
		return;

	const bool hasRemember = body.find("rememberSuccess(") != std::string::npos;
	const bool hasDrop = body.find("dropSticky()") != std::string::npos;

	CHECK(hasRemember);
	CHECK(hasDrop);

	// Scoping: the snapshot precedes the first attempt, and a restore comes
	// after the LAST attempt.  The mid-flow restore on the sticky-failure
	// path sits before the fallback loop, so it cannot satisfy the second
	// assertion on its own -- only the final hand-back can.
	const size_t captureAt = body.find(".capture()");
	const size_t firstAttemptAt = body.find("attempt(");
	const size_t lastAttemptAt = body.rfind("attempt(");
	const size_t lastRestoreAt = body.rfind(".restore()");

	CHECK(captureAt != std::string::npos);
	CHECK(firstAttemptAt != std::string::npos);
	CHECK(lastAttemptAt != std::string::npos);
	CHECK(lastRestoreAt != std::string::npos);

	if (captureAt != std::string::npos && firstAttemptAt != std::string::npos)
	{
		CHECK(captureAt < firstAttemptAt);
	}

	if (lastRestoreAt != std::string::npos && lastAttemptAt != std::string::npos)
	{
		CHECK(lastRestoreAt > lastAttemptAt);
		if (lastRestoreAt < lastAttemptAt)
			std::printf("    the SDL_AUDIODRIVER override is handed back before the last attempt\n");
	}
}

int main()
{
	std::printf("== audio driver fallback lock ==\n");

	T1_absent_variable_is_restored_to_absence();
	T2_preexisting_value_is_restored_exactly();
	T3_restore_is_idempotent();
	T4_sticky_records_and_overwrites();
	T5_drop_sticky_falls_back_to_the_default_chain();
	T6_untouched_attempt_leaves_the_variable_alone();
	T7_source_guard_probe_scopes_the_override();

	if (g_failures != 0)
	{
		std::printf("== %d check(s) FAILED ==\n", g_failures);
		return 1;
	}

	std::printf("== all checks passed ==\n");
	return 0;
}
