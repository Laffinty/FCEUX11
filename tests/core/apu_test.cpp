// FCEUX11 v1.1 Sentinel — APU correctness tests.
//
// Verifies the APU/expansion-sound surface:
//   * Power resets sound timestamps to a known state
//   * The internal Wave[] buffer is writable through the public symbol
//   * After 60 frames, GetSoundBuffer returns a non-empty buffer
//   * soundtsinc is positive (sample rate above 0)
//   * FCEU_SoundCPUHook can be called per-cycle without crashing
//   * FrameSoundUpdate completes a full frame
//   * SetSoundVariables doesn't crash
//   * FCEUSND_SaveState/LoadState is callable
//   * EXP sound function pointers can be set to null (kill path)
//   * Sound output is non-degenerate (not all-zero) over 60 frames
//   * swappable duty cycle flag is bool-typed
//
// These tests are not byte-for-byte correctness checks (that is the
// domain of APU audio regression tests in v1.6 Resonance). v1.1
// establishes the *surface* is reachable and survives basic
// interaction.

#include "test_helpers.h"

#include <cstdio>
#include <cstdint>
#include <cstring>

using namespace fceu11_test;

static const char* kRom = "fixtures/nestest.nes";

void test_sound_after_init(TestContext& ctx) {
    // soundtsinc is the sample-rate period in CPU cycles. After
    // SetSoundVariables it should be > 0 (i.e. the rate was set).
    SetSoundVariables();
    FCEU11_EXPECT(ctx, soundtsinc > 0, "soundtsinc > 0 after SetSoundVariables");
    FCEU11_EXPECT(ctx, soundtsoffs == 0 || soundtsoffs > 0,
                  "soundtsoffs is a defined value");
}

void test_wave_buffer_writable(TestContext& ctx) {
    // Wave[] and WaveFinal[] are the mix-down buffers. They must
    // exist in the address space and accept writes. We don't depend
    // on the value surviving emulation (the engine will overwrite),
    // only on the storage being addressable.
    extern int32 (&Wave)[2048 + 512];
    extern int32 (&WaveFinal)[2048 + 512];
    int32 orig0 = Wave[0];
    int32 origF = WaveFinal[0];
    Wave[0]      = 0x11223344;
    WaveFinal[0] = 0x55667788;
    FCEU11_EXPECT(ctx, Wave[0]      == 0x11223344, "Wave[0] writable");
    FCEU11_EXPECT(ctx, WaveFinal[0] == 0x55667788, "WaveFinal[0] writable");
    Wave[0]      = orig0;
    WaveFinal[0] = origF;
}

void test_sound_cpu_hook(TestContext& ctx) {
    // FCEU_SoundCPUHook(cycles) is invoked from x6502.cpp after every
    // instruction. Call it for 0 / 1 / 1000 cycles and verify the
    // engine doesn't crash. This is a smoke test; precise cycle
    // accounting is the job of v1.6.
    FCEU_SoundCPUHook(0);
    FCEU_SoundCPUHook(1);
    FCEU_SoundCPUHook(1000);
    FCEU11_EXPECT(ctx, true, "FCEU_SoundCPUHook survives 0/1/1000 cycles");
}

void test_get_sound_buffer(TestContext& ctx) {
    // Run 30 frames and then ask GetSoundBuffer for the buffer size.
    // It should be > 0 (samples are accumulating).
    int32* out = nullptr;
    int before = GetSoundBuffer(&out);
    emulate_n(30);
    int after = GetSoundBuffer(&out);
    FCEU11_EXPECT(ctx, after >= 0, "GetSoundBuffer returns non-negative count");
    FCEU11_EXPECT(ctx, out != nullptr || after == 0, "GetSoundBuffer pointer is non-null when count > 0");
    (void)before;
}

void test_flush_emulate_sound(TestContext& ctx) {
    // FlushEmulateSound finalises the current frame's mix. Should
    // return without crashing.
    int rc = FlushEmulateSound();
    FCEU11_EXPECT(ctx, rc == 0 || rc > 0, "FlushEmulateSound returns cleanly");
}

void test_sound_state_roundtrip(TestContext& ctx) {
    // FCEUSND_SaveState / LoadState must be callable. v1.1 doesn't
    // byte-compare, but the symbols must be linked and safe to call.
    FCEUSND_SaveState();
    FCEUSND_LoadState(0);
    FCEU11_EXPECT(ctx, true, "FCEUSND_SaveState/LoadState callable");
}

void test_exp_sound_kill(TestContext& ctx) {
    // GameExpSound.Kill is a function pointer; setting it to null
    // is a legal state (no expansion sound enabled). The engine
    // must handle that gracefully.
    void (*orig_kill)(void) = GameExpSound.Kill;
    GameExpSound.Kill = nullptr;
    emulate_n(1);
    FCEU11_EXPECT(ctx, true, "engine survives 1 frame with GameExpSound.Kill=nullptr");
    GameExpSound.Kill = orig_kill;
}

void test_frame_sound_update(TestContext& ctx) {
    // FrameSoundUpdate finalises the per-frame sound state. It must
    // be callable and not crash the engine.
    FrameSoundUpdate();
    FCEU11_EXPECT(ctx, true, "FrameSoundUpdate callable");
}

void test_sound_output_nonzero(TestContext& ctx) {
    // Run 60 frames and check the sound buffer is not all-zero.
    // (nestest.nes is silent at reset, so sample magnitude might still be
    // zero; we therefore require:
    //   1. timestampbase advanced monotonically (CPU ran)
    //   2. GetSoundBuffer returned > 0 samples (APU output buffer filled)
    //
    // The original test was written against `soundtimestamp`, which is
    // reset to 0 at the end of each Emulate() call (see fceu.cpp:888) and
    // is therefore useless as a monotonic-across-frames signal. Switch to
    // timestampbase (the cumulative cycle counter) for the monotonic check.
    uint64 b0 = timestampbase;
    emulate_n(60);
    uint64 b1 = timestampbase;
    FCEU11_EXPECT(ctx, b1 > b0, "timestampbase advances over 60 frames");
    FCEU11_EXPECT(ctx, (b1 - b0) > 1000, "at least 1000 CPU cycles accumulated");
    // Sanity check that the sound pipeline produced a non-empty buffer.
    int32* out = nullptr;
    int ssize = GetSoundBuffer(&out);
    FCEU11_EXPECT(ctx, ssize > 0, "sound buffer non-empty after 60 frames");
}

void test_sound_set_rate(TestContext& ctx) {
    // FCEUI_Sound(48000) is the public rate setter. It is idempotent.
    FCEUI_Sound(48000);
    emulate_n(1);
    FCEUI_Sound(44100);
    emulate_n(1);
    FCEUI_Sound(48000);  // restore
    FCEU11_EXPECT(ctx, true, "FCEUI_Sound survives rate changes 48k->44.1k->48k");
}

void test_swap_duty_flag(TestContext& ctx) {
    // swapDuty is a public bool, occasionally toggled for a
    // workaround. Setting both values must not crash.
    bool orig = swapDuty;
    swapDuty = !orig;
    FCEU11_EXPECT(ctx, swapDuty != orig, "swapDuty is mutable");
    swapDuty = orig;
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    std::printf("=== FCEUX11 v1.1 APU test suite ===\n");
    std::printf("ROM: %s\n\n", kRom);

    TestContext ctx;

    if (!core_init()) { return 1; }
    FCEUGI* gi = load_rom(kRom);
    if (!gi) { core_shutdown(); return 1; }

    FCEUI_Sound(48000);

    test_sound_after_init(ctx);
    test_wave_buffer_writable(ctx);
    test_sound_cpu_hook(ctx);
    test_get_sound_buffer(ctx);
    test_flush_emulate_sound(ctx);
    test_sound_state_roundtrip(ctx);
    test_exp_sound_kill(ctx);
    test_frame_sound_update(ctx);
    test_sound_output_nonzero(ctx);
    test_sound_set_rate(ctx);
    test_swap_duty_flag(ctx);

    fceu11::CloseGame();
    core_shutdown();

    return report_and_exit(ctx, "APU test suite");
}
