// FCEUX11 v2.0.5 (PERF-02) — the audio sink contract when there is no device.
//
// Owner's report (2026-11-18): with the USB speaker powered on everything is
// fine; power it off (the sound card disappears) and FCEUX11 loads the ROM but
// then behaves as if frozen and the keyboard does nothing.
//
// Root cause (code-level, pinned by this lock): the ring buffer's only consumer
// is the SDL audio callback, which exists only while a device is open.  With no
// device nothing drains the ring, so `s_BufferIn` reaches `s_BufferSize` and
// stays there; every `WriteSound` call then entered the
// "sound sink is not draining" spin -- SDL_Delay(1) up to waitCount > 1000,
// i.e. about one second -- and `waitCount` is a local, so the next call paid it
// again.  Roughly one frame per second = "frozen, keyboard dead".
//
// This lock is deterministic and needs no audio hardware:
//   * in a freshly started process the sink is closed (s_audioDeviceOpen ==
//     false) and s_BufferSize is still 0, so the buggy path hit
//     `while (s_BufferIn == s_BufferSize)` immediately with 0 == 0;
//   * a closed sink must return from WriteSound without waiting at all.
//
// Mutation contract: delete the `s_audioDeviceOpen` gate at the top of
// WriteSound (src/drivers/Qt/sdl-sound.cpp) and this test goes red after the
// first call, which alone costs ~1000 ms.
//
// Deliberately NOT asserted here: that a host with a real device produces
// sound.  That needs hardware and is covered by the owner's manual checks.

#include <cstdio>
#include <cstdint>
#include <vector>
#include <chrono>

#include "types.h"

#include "test_helpers.h"

// The two driver symbols this lock needs, declared here on purpose.
//
// Pulling in src/drivers/Qt/dface.h would drag input.h (FCEUGI,
// FAMILYKEYBOARD_NUM_BUTTONS, ...) into a test that has no business knowing
// about input devices; and test_helpers.h already brings in the core headers.
// The signatures are pinned by the link: if either changes, this test fails to
// link rather than silently testing something else.
void WriteSound(int32 *Buffer, int Count);
bool FCEUD_SoundDeviceIsOpen(void);

int main() {
    fceu11_test::TestContext ctx;

    // ------------------------------------------------------------------
    // Leg 1 — a fresh process has no audio device open.
    //
    // Nothing in this test opens one, so this also documents the state the
    // emulator is in when the host has no usable endpoint.
    // ------------------------------------------------------------------
    FCEU11_EXPECT(ctx, !FCEUD_SoundDeviceIsOpen(),
        "audio sink is closed in a fresh process (no device opened here)");

    // ------------------------------------------------------------------
    // Leg 2 — a closed sink must never consume the emulation thread.
    //
    // 50 calls of 256 samples is ~1 emulated frame's worth of writes on the
    // real path; on a closed sink the whole burst has to cost nothing.
    // The loop bails out as soon as the budget is blown, so the red leg costs
    // ~1 s instead of 50 s.
    // ------------------------------------------------------------------
    const int    kCalls    = 50;
    const long long kBudgetMs = 200;

    std::vector<int32> buf(256, 0);
    int       calls   = 0;
    long long elapsed = 0;

    const auto t0 = std::chrono::steady_clock::now();
    for (; calls < kCalls; ++calls) {
        WriteSound(buf.data(), static_cast<int>(buf.size()));
        elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                      std::chrono::steady_clock::now() - t0).count();
        if (elapsed > kBudgetMs) {
            ++calls;
            break;
        }
    }

    std::printf("  %d WriteSound call(s) on a closed sink: %lld ms (budget %lld ms)\n",
                calls, elapsed, kBudgetMs);
    FCEU11_EXPECT(ctx, elapsed <= kBudgetMs,
        "WriteSound on a closed sink returns immediately (no drain spin)");

    // ------------------------------------------------------------------
    // Leg 3 — the sink is still closed afterwards (WriteSound must not be
    // able to open, close or otherwise mutate the device state).
    // ------------------------------------------------------------------
    FCEU11_EXPECT(ctx, !FCEUD_SoundDeviceIsOpen(),
        "WriteSound leaves the device state untouched");

    return fceu11_test::report_and_exit(
        ctx, "FCEUX11 v2.0.5 PERF-02 - no-device audio sink contract");
}
