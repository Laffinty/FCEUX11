/* FCEUX11 - source guard: the GBA session flip clears the throttle override.
 *
 * Regression being locked
 * ----------------------
 * v2.0.5 audit "GBA throttle residue" (stable-audit report section 6): the
 * session flip in fceuWrapperUpdate used to be detected only INSIDE the
 * GBA-only branch (`if (fceu11_gba_active())`), so the false edge -- a GBA
 * session ending -- never ran.  fceuWrapper_sync_gba_audio(false) is what
 * hands the pacing rate back (SetThrottleBaseRateOverride(0.0)), so after
 * playing a .gba every NES session was pinned at 59.7275 fps -- 0.618% slow
 * (about 6.18 ms a second) until the process exited.  RefreshThrottleFPS()
 * reads the override but never clears it, so nothing else could undo it.
 *
 * The fix moves the flip detection above the machine branch, so both edges
 * run.  This file pins, on the source text:
 *
 *  G1  the flip check runs for both directions: `gbaActive` is read and
 *      compared against g_gbaSessionActive in fceuWrapperUpdate
 *  G2  the flip check sits ABOVE the machine branch -- textually, its
 *      position precedes `if ( fceu11_gba_active() )`, so the false edge is
 *      reachable (moving the block back inside the GBA branch turns this red)
 *  G3  the false edge hands the base rate back: fceuWrapper_sync_gba_audio
 *      still contains the ternary with `: 0.0`
 *  G4  the true edge guards the GBA audio configure with `if (active)`, so
 *      the Rust configure call never runs for a machine that is not active
 *  G5  SetThrottleBaseRateOverride is called from exactly one place in
 *      fceuWrapper.cpp (a second reset elsewhere would shadow this one)
 *
 * Mutation legs (each verified by hand; see the plan, section 5):
 *  M1  move the flip block back inside the GBA branch  -> G2 red
 *  M2  change the false arm to keep the GBA rate       -> G3 red
 *  M3  delete the flip block entirely                  -> G1 red
 *  M4  unguard the configure call                      -> G4 red
 *
 * Pure std, no core/Qt/SDL: one translation unit.  ctest must run with
 * WORKING_DIRECTORY=tests/.
 */

#include <cstdio>
#include <fstream>
#include <string>

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

/// The text from `marker` to the first closing brace at column zero.
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

} // namespace

int main()
{
	std::printf("== gba throttle reset guard ==\n");

	std::ifstream in("../src/drivers/Qt/fceuWrapper.cpp");
	if (!in.is_open())
	{
		std::printf("    FAIL could not open ../src/drivers/Qt/fceuWrapper.cpp\n");
		std::printf("         (ctest must run with WORKING_DIRECTORY=tests/)\n");
		return 1;
	}

	std::string src((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());

	const std::string updateBody = functionBody(src, "fceuWrapperUpdate( void )");
	const std::string syncBody = functionBody(src, "fceuWrapper_sync_gba_audio(bool active)");

	CHECK(!updateBody.empty());
	CHECK(!syncBody.empty());

	if (updateBody.empty() || syncBody.empty())
	{
		std::printf("== %d check(s) FAILED ==\n", g_failures);
		return 1;
	}

	// G1: the flip check runs for both directions.
	const size_t flipAt = updateBody.find("const bool gbaActive = fceu11_gba_active();");
	const size_t flipIfAt = updateBody.find("if ( gbaActive != g_gbaSessionActive )");

	CHECK(flipAt != std::string::npos);
	CHECK(flipIfAt != std::string::npos);
	CHECK(updateBody.find("g_gbaSessionActive = gbaActive;") != std::string::npos);

	// G2: the flip check sits ABOVE the machine branch, so the false edge (a
	// GBA session ending) is reachable.
	const size_t machineBranchAt = updateBody.find("if ( fceu11_gba_active() )");

	CHECK(machineBranchAt != std::string::npos);

	if (flipAt != std::string::npos && machineBranchAt != std::string::npos)
	{
		CHECK(flipAt < machineBranchAt);
		if (flipAt > machineBranchAt)
			std::printf("    the session flip is back inside the GBA-only branch\n");
	}

	// G3: the false edge hands the base rate back.
	const size_t overrideAt = syncBody.find("SetThrottleBaseRateOverride(active ?");
	CHECK(overrideAt != std::string::npos);
	CHECK(syncBody.find(": 0.0);", overrideAt == std::string::npos ? 0 : overrideAt)
	    != std::string::npos);

	// G4: the configure call only runs for an active GBA session.
	CHECK(syncBody.find("if (active)") != std::string::npos);
	CHECK(syncBody.find("fceu11_gba_configure_audio(") != std::string::npos);

	// G5: exactly one call site for the override in this file -- a second
	// reset elsewhere would shadow (or race) this one.
	size_t count = 0;
	for (size_t pos = src.find("SetThrottleBaseRateOverride(");
	     pos != std::string::npos;
	     pos = src.find("SetThrottleBaseRateOverride(", pos + 1))
	{
		++count;
	}
	CHECK(count == 1);
	if (count != 1)
		std::printf("    SetThrottleBaseRateOverride is called from %zu places (want exactly 1)\n", count);

	if (g_failures != 0)
	{
		std::printf("== %d check(s) FAILED ==\n", g_failures);
		return 1;
	}

	std::printf("== all checks passed ==\n");
	return 0;
}
