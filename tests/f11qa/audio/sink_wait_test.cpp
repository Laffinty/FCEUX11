/* FCEUX11 - lock test for the audio sink wait policy.
 *
 * Regression being locked
 * ----------------------
 * A device that is open but stops draining (endpoint removed after open,
 * exclusive-mode takeover, driver stall) used to cost WriteSound ~1 second per
 * frame -- waitCount was local to the call, so the spin restarted every frame
 * -- inside FCEU_WRAPPER_LOCK.  Result: ~1 fps with the window frozen, i.e.
 * "audio and video crawl".  PERF-02 only covered "no device at all"; this lock
 * covers the "open but stalled" half, plus the GetWriteSound underflow.
 *
 * Legs
 * ----
 *  T1  a sink that never drains: one bounded wait (<= budget), then latched --
 *      the next call must return WITHOUT sleeping
 *  T2  when the ring drains again the latch clears, and a second episode is
 *      counted separately
 *  T3  a healthy sink never sleeps (no regression on the normal path)
 *  T4  a transient full ring (jitter) resolves inside the budget, no latch
 *  T5  RingFree() never underflows (the GBA path feeds it into resize())
 *
 * Mutation legs (each verified by hand; see the plan):
 *  M1  drop the latch in waitForRoom()            -> T1 red (second call sleeps)
 *  M2  make the budget unbounded                  -> T1 red (throws: too many sleeps)
 *  M3  RingFree() back to `size - in`             -> T5 red
 *  M4  drop the latch clearing in observe()       -> T2 red
 *
 * Pure std: no SDL, no Qt, no emulator core.
 */

#include "Qt/audio_sink_wait.h"

#include <cstdio>
#include <vector>

using fceu11::audio::RingFree;
using fceu11::audio::SinkWaitGuard;

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

// A mutant that removes the bound would spin forever; make that a failure
// instead of a hung ctest run.
struct TooManySleeps
{
};

struct FakeClock
{
	int sleeps = 0;
	int limit = 400;

	void sleep()
	{
		if (++sleeps > limit)
			throw TooManySleeps();
	}
};

} // namespace

static void T1_stalled_sink_waits_once_then_drops()
{
	std::printf("T1  a sink that never drains: bounded wait once, then no waiting\n");

	SinkWaitGuard guard(8);
	FakeClock clk;

	const auto noRoom = []() { return false; };

	const auto v1 = guard.waitForRoom(noRoom, [&]() { clk.sleep(); });

	std::printf("    first call: verdict=%s sleeps=%d\n",
	            v1 == SinkWaitGuard::Verdict::Stalled ? "Stalled" : "Ready", clk.sleeps);

	CHECK(v1 == SinkWaitGuard::Verdict::Stalled);
	CHECK(clk.sleeps <= 8);      // bounded by the budget
	CHECK(clk.sleeps > 0);       // it did give the sink a chance
	CHECK(guard.stalled());
	CHECK(guard.episodes() == 1);

	const int sleepsAfterFirst = clk.sleeps;
	const auto v2 = guard.waitForRoom(noRoom, [&]() { clk.sleep(); });
	const int sleptOnSecond = clk.sleeps - sleepsAfterFirst;

	std::printf("    second call: verdict=%s additional sleeps=%d\n",
	            v2 == SinkWaitGuard::Verdict::Stalled ? "Stalled" : "Ready", sleptOnSecond);

	CHECK(v2 == SinkWaitGuard::Verdict::Stalled);
	CHECK(sleptOnSecond == 0);   // latched: dropping must cost nothing
	CHECK(guard.episodes() == 1);
	CHECK(guard.droppedCalls() == 2);
}

static void T2_latch_clears_when_the_sink_drains()
{
	std::printf("T2  the latch clears when the ring drains, and episodes are counted\n");

	SinkWaitGuard guard(4);
	FakeClock clk;

	const auto noRoom = []() { return false; };
	const auto roomNow = []() { return true; };

	CHECK(guard.waitForRoom(noRoom, [&]() { clk.sleep(); }) == SinkWaitGuard::Verdict::Stalled);
	CHECK(guard.stalled());

	// The callback drained the ring: the next write reports free space.
	guard.observe(10, 100);
	CHECK(guard.stalled() == false);

	clk.sleeps = 0;
	CHECK(guard.waitForRoom(roomNow, [&]() { clk.sleep(); }) == SinkWaitGuard::Verdict::Ready);
	CHECK(clk.sleeps == 0);

	// A second stall is a new episode, not a continuation of the first.
	CHECK(guard.waitForRoom(noRoom, [&]() { clk.sleep(); }) == SinkWaitGuard::Verdict::Stalled);
	CHECK(guard.episodes() == 2);
}

static void T3_healthy_sink_never_sleeps()
{
	std::printf("T3  a healthy sink never sleeps\n");

	SinkWaitGuard guard;
	FakeClock clk;

	CHECK(guard.waitForRoom([]() { return true; }, [&]() { clk.sleep(); }) ==
	      SinkWaitGuard::Verdict::Ready);
	CHECK(clk.sleeps == 0);
	CHECK(guard.episodes() == 0);
	CHECK(guard.stalled() == false);
}

static void T4_transient_full_ring_resolves_inside_the_budget()
{
	std::printf("T4  a transient full ring resolves inside the budget, no latch\n");

	SinkWaitGuard guard(16);
	FakeClock clk;

	// Room appears after three milliseconds -- normal back-pressure.
	const auto roomAfter3 = [&clk]() { return clk.sleeps >= 3; };

	CHECK(guard.waitForRoom(roomAfter3, [&]() { clk.sleep(); }) ==
	      SinkWaitGuard::Verdict::Ready);
	CHECK(clk.sleeps == 3);
	CHECK(guard.stalled() == false);
	CHECK(guard.episodes() == 0);
}

static void T5_ring_free_never_underflows()
{
	std::printf("T5  RingFree() clamps at zero\n");

	CHECK(RingFree(0, 0) == 0u);
	CHECK(RingFree(3, 0) == 0u);          // the old `size - in` gave 0xFFFFFFFD
	CHECK(RingFree(100, 100) == 0u);
	CHECK(RingFree(103, 100) == 0u);      // producer pushed past the end
	CHECK(RingFree(100, 1000) == 900u);

	// The contract the GBA path relies on when it sizes a buffer:
	const std::vector<unsigned> ins{0, 1, 99, 100, 101, 0xFFFFFFFFu};
	for (unsigned in : ins)
		CHECK(RingFree(in, 100) <= 100u);
}

int main()
{
	std::printf("== audio sink wait policy lock ==\n");

	try
	{
		T1_stalled_sink_waits_once_then_drops();
		T2_latch_clears_when_the_sink_drains();
		T3_healthy_sink_never_sleeps();
		T4_transient_full_ring_resolves_inside_the_budget();
		T5_ring_free_never_underflows();
	}
	catch (const TooManySleeps &)
	{
		std::printf("    FAIL: the wait is not bounded (too many sleeps)\n");
		++g_failures;
	}

	if (g_failures != 0)
	{
		std::printf("== %d check(s) FAILED ==\n", g_failures);
		return 1;
	}

	std::printf("== all checks passed ==\n");
	return 0;
}
