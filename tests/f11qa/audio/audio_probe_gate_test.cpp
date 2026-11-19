/* FCEUX11 - lock test for the audio device probe gate.
 *
 * Regression being locked
 * ----------------------
 * v2.0.5 (PERF-01, 36af048) moved SDL_OpenAudio() to a probe thread.  Its
 * companion KillSound() joined that thread unconditionally, on the GUI thread,
 * inside FCEU_WRAPPER_LOCK -- so a close that arrived while a probe was still
 * running moved the rest of the probe back into the GUI thread and froze the
 * window (up to eight seconds on a host whose audio endpoint is slow to fail).
 *
 * The gate (src/drivers/Qt/audio_probe_gate.h) is the fix: requestClose()
 * never waits while a probe is in flight, it delegates the close to the probe
 * thread.  This test injects a slow "open" and pins four properties of that
 * contract, plus one source guard on the caller.
 *
 * Legs
 * ----
 *  T1  requestClose() during a probe returns in < 100 ms (probe takes 500 ms)
 *  T2  the delegated teardown closes the device exactly once and leaves the
 *      gate closed and idle
 *  T3  close() followed by open(same rate) does NOT probe twice -- the attempt
 *      in flight is reused (this is the settings-dialog pattern
 *      `KillSound(); InitSound();`)
 *  T4  close() followed by open(different rate) closes once, then probes the
 *      new rate and ends up open at it
 *  T5  source guard: sdl-sound.cpp contains no thread join (the defect was
 *      exactly that call) and still calls requestClose()
 *
 * Mutation legs (each was verified by hand; see the plan, section 5):
 *  M1  make requestClose() join the probe thread        -> T1 red
 *  M2  drop the teardown branch in the probe loop       -> T2 red
 *  M3  drop the "same rate is satisfied by this attempt" rule, or stop
 *      cancelling a pending teardown on requestOpen()   -> T3 red
 *  M4  drop the post-probe pending hand-off             -> T4 red
 *  M5  restore `s_audioOpenThread.join()` in KillSound  -> T5 red
 *
 * No emulator core, no Qt, no SDL: two translation units, so this stays cheap
 * enough to run on every ctest invocation.
 */

#include "Qt/audio_probe_gate.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <string>
#include <thread>

using fceu11::audio::AudioProbeGate;
using Clock = std::chrono::steady_clock;

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

struct Probe
{
	std::atomic<int> openCalls{0};
	std::atomic<int> closeCalls{0};
	std::atomic<unsigned int> lastRate{0};
	int delayMs = 500;

	AudioProbeGate::OpenFn opener(unsigned int rate)
	{
		return [this, rate]()
		{
			lastRate.store(rate, std::memory_order_release);
			openCalls.fetch_add(1);
			std::this_thread::sleep_for(std::chrono::milliseconds(delayMs));
			return true;
		};
	}

	AudioProbeGate::CloseFn closer()
	{
		return [this]() { closeCalls.fetch_add(1); };
	}
};

static void sleepMs(int ms)
{
	std::this_thread::sleep_for(std::chrono::milliseconds(ms));
}

// Wait until the probe thread has entered its open() call.
static bool waitForOpenStart(Probe &p, int timeoutMs = 2000)
{
	for (int i = 0; i < timeoutMs; ++i)
	{
		if (p.openCalls.load() != 0)
			return true;
		sleepMs(1);
	}
	return false;
}

// Wait until the gate reports itself idle *without* asking it to stop, so the
// attempt in flight is allowed to finish and publish normally.
static bool waitUntilIdle(AudioProbeGate &g, int timeoutMs = 4000)
{
	for (int i = 0; i < timeoutMs; ++i)
	{
		if (!g.isInFlight())
			return true;
		sleepMs(1);
	}
	return false;
}

// ---------------------------------------------------------------------------

static void T1_close_during_a_probe_does_not_wait()
{
	std::printf("T1  requestClose() during a probe returns immediately\n");

	AudioProbeGate gate;
	Probe p;
	gate.setCloseHandler(p.closer());

	gate.requestOpen(44100, p.opener(44100));
	CHECK(gate.isInFlight());
	CHECK(waitForOpenStart(p));

	const auto t0 = Clock::now();
	const bool closedInline = gate.requestClose();
	const auto t1 = Clock::now();

	const long long elapsedMs =
	    std::chrono::duration_cast<std::chrono::milliseconds>(t1 - t0).count();

	std::printf("    close returned in %lld ms (probe needs %d ms)\n", elapsedMs, p.delayMs);

	CHECK(elapsedMs < 100);
	CHECK(closedInline == false); // it was delegated, not performed inline

	gate.waitIdle();
}

static void T2_delegated_teardown_closes_the_device()
{
	std::printf("T2  delegated teardown closes exactly once and leaves the gate closed\n");

	AudioProbeGate gate;
	Probe p;
	gate.setCloseHandler(p.closer());

	gate.requestOpen(44100, p.opener(44100));
	CHECK(waitForOpenStart(p));

	CHECK(gate.requestClose() == false);
	gate.waitIdle();

	CHECK(p.closeCalls.load() == 1);
	CHECK(gate.isOpen() == false);
	CHECK(gate.isInFlight() == false);
	CHECK(gate.openRate() == 0);

	std::printf("    openCalls=%d closeCalls=%d openRate=%u\n",
	            p.openCalls.load(), p.closeCalls.load(), gate.openRate());
}

static void T3_reopen_same_rate_reuses_the_attempt_in_flight()
{
	std::printf("T3  KillSound(); InitSound(); at the same rate probes only once\n");

	AudioProbeGate gate;
	Probe p;
	gate.setCloseHandler(p.closer());

	gate.requestOpen(44100, p.opener(44100));
	CHECK(waitForOpenStart(p));

	// The dialog pattern: close, then immediately ask for the same device.
	CHECK(gate.requestClose() == false);
	const AudioProbeGate::Request again = gate.requestOpen(44100, p.opener(44100));
	CHECK(again == AudioProbeGate::Request::OpeningSameRate);

	CHECK(waitUntilIdle(gate));

	std::printf("    openCalls=%d closeCalls=%d openRate=%u\n",
	            p.openCalls.load(), p.closeCalls.load(), gate.openRate());

	CHECK(p.openCalls.load() == 1);   // not two probes
	CHECK(p.closeCalls.load() == 0);  // the teardown was cancelled
	CHECK(gate.isOpen());
	CHECK(gate.openRate() == 44100);
}

static void T4_reopen_different_rate_queues_one_more_attempt()
{
	std::printf("T4  close(); open(other rate) closes once, then probes the new rate\n");

	AudioProbeGate gate;
	Probe p;
	p.delayMs = 150; // two probes in this leg; keep the suite fast
	gate.setCloseHandler(p.closer());

	gate.requestOpen(44100, p.opener(44100));
	CHECK(waitForOpenStart(p));

	CHECK(gate.requestClose() == false);
	const AudioProbeGate::Request q = gate.requestOpen(48000, p.opener(48000));
	CHECK(q == AudioProbeGate::Request::Queued);

	CHECK(waitUntilIdle(gate));

	std::printf("    openCalls=%d lastRate=%u closeCalls=%d openRate=%u\n",
	            p.openCalls.load(), p.lastRate.load(), p.closeCalls.load(), gate.openRate());

	CHECK(p.openCalls.load() == 2);
	CHECK(p.closeCalls.load() == 1);
	CHECK(gate.isOpen());
	CHECK(gate.openRate() == 48000);
}

// The gate can only enforce the contract it owns.  The defect lived in the
// caller (KillSound joining the probe thread), so pin that too.  Two things
// must hold in sdl-sound.cpp:
//   * no thread join anywhere in the file (that call is the defect), and
//   * KillSound's body asks for the close with requestClose() and does not
//     then wait for the probe (WaitSoundProbeIdle is the exit-path entry point;
//     calling it from KillSound is the same freeze under another name).
// Either regression turns this leg red.
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

static void T5_sdl_sound_does_not_join_the_probe_thread()
{
	std::printf("T5  sdl-sound.cpp KillSound() does not wait for the probe thread\n");

	std::ifstream in("../src/drivers/Qt/sdl-sound.cpp");
	if (!in.is_open())
	{
		std::printf("    FAIL could not open ../src/drivers/Qt/sdl-sound.cpp\n");
		std::printf("         (ctest must run with WORKING_DIRECTORY=tests/)\n");
		++g_failures;
		return;
	}

	std::string src((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());

	const bool hasJoin = src.find(".join()") != std::string::npos;
	const std::string killBody = functionBody(src, "KillSound(void)");
	const bool hasKillBody = !killBody.empty();
	const bool killUsesRequestClose =
	    killBody.find("requestClose()") != std::string::npos;
	const bool killWaitsForProbe =
	    killBody.find("WaitSoundProbeIdle") != std::string::npos;

	CHECK(hasJoin == false);
	CHECK(hasKillBody);
	CHECK(killUsesRequestClose);
	CHECK(killWaitsForProbe == false);

	if (hasJoin)
		std::printf("    a thread join is back in the GUI-side close path\n");
	if (!hasKillBody)
		std::printf("    could not locate KillSound() in sdl-sound.cpp\n");
	if (killWaitsForProbe)
		std::printf("    KillSound() waits for the probe thread again\n");
}

} // namespace

int main()
{
	std::printf("== audio probe gate lock ==\n");

	T1_close_during_a_probe_does_not_wait();
	T2_delegated_teardown_closes_the_device();
	T3_reopen_same_rate_reuses_the_attempt_in_flight();
	T4_reopen_different_rate_queues_one_more_attempt();
	T5_sdl_sound_does_not_join_the_probe_thread();

	if (g_failures != 0)
	{
		std::printf("== %d check(s) FAILED ==\n", g_failures);
		return 1;
	}

	std::printf("== all checks passed ==\n");
	return 0;
}
