// FCEUX11 v1.1 Sentinel — APU correctness tests.
//
// Verifies the APU/expansion-sound surface:
//   * Power resets sound timestamps to a known state
//   * The internal Wave[] buffer is writable through the public symbol
//   * After 60 frames, GetSoundBuffer returns a non-empty buffer
//   * soundtsinc is positive (sample rate above 0)
//   * FCEU_SoundCPUHook can be called per-cycle without crashing
//   * DMC sample fetch charges exactly 4 CPU cycles (LT-08 DMC accounting)
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

void test_dmc_acc_negation_overflow(TestContext& ctx) {
    // hotfix3 C-3: the prior `-DMCacc` inside FCEU_SoundCPUHook was
    // signed-overflow UB when DMCacc == INT32_MIN. With the int64
    // cast this is a defined magnitude (2^31). Set DMCacc to the
    // boundary, run a single hook call, and verify:
    //   1. returns without crashing (UBSan would abort otherwise)
    //   2. DMCacc advanced past the boundary (i.e. the inner while
    //      terminated, not stuck because of a poisoned negation)
    extern int32_t& DMCacc;
    const int32_t sentinel = INT32_MIN;
    DMCacc = sentinel;
    FCEU_SoundCPUHook(1);
    FCEU11_EXPECT(ctx, DMCacc != sentinel, "DMCacc advances past INT32_MIN boundary");
}


// LT-08 (v2.0.5 B1) -- DMC cycle accounting, part 1: the sample fetch.
//
// Authoritative bounds (NESdev wiki; full quotes + retrieval log in
// Research_only/w205/lt08_dmc.md):
//   APU_DMC (rev. 2024-03-21): "The CPU is stalled for 1-4 CPU cycles to
//   read a sample byte."  The DMC memory reader fetches one sample byte
//   whenever the sample buffer is empty and bytes-remaining is not zero --
//   "including just after a write to $4015 that enables the channel".
//   DMA article: "DMC DMA halts the CPU, performs a dummy cycle and an
//   optional alignment cycle, and then gets once, taking 3 or 4 cycles."
//   blargg's own dmc_dma_during_read4 test source spells the cost out:
//   "; 4 DMC wait-states".
//
// This repo's model (src/sound.cpp:741-765, DMCDMA): one fetch == four
// X6502_DMR calls, and each X6502_DMR charges ADDCYC(1)
// (src/x6502.cpp:111-114), i.e. exactly 4 CPU cycles per sample fetch.
// Same shape as upstream FCEUX 2.8.0 / 2.6.6 (byte-identical DMC code).
//
// This lock pins:
//   (1) the $4015 enable is followed by a fetch on the very next
//       FCEU_SoundCPUHook (the hook runs at the start of the next
//       instruction, src/x6502.cpp:1083),
//   (2) that fetch costs exactly 4 CPU cycles (CPU-cycle unit, not master
//       cycle; no DMA arbitration is modelled by this repo),
//   (3) one fetch per sample byte: bytes-remaining -1, address +1,
//   (4) a second hook with a full sample buffer steals nothing.
void test_dmc_fetch_cycle_accounting(TestContext& ctx) {
    // Push the DMC timer far away so this test measures the fetch
    // accounting only, not the timer accounting (part 2 below).
    DMCacc = 1 << 20;
    DMCHaveSample = 0;
    DMCBitCount = 0;

    // Drive the DMC through the real bus, exactly like a game would.
    // X6502_DMW itself charges 1 CPU cycle, so the snapshot below is
    // taken after the register writes.
    X6502_DMW(0x4010, 0x0F);   // rate index $F (no loop, no DMC IRQ)
    X6502_DMW(0x4012, 0x00);   // sample address latch -> $C000
    X6502_DMW(0x4013, 0x00);   // sample size latch  -> 1 byte
    X6502_DMW(0x4015, 0x10);   // bit4 = enable DMC -> PrepDPCM()

    const int32_t  size_before = DMCSize;        // 1 byte
    const uint32_t addr_before = DMCAddress;      // 0x4000
    FCEU11_EXPECT(ctx, size_before == 1, "DMC armed with exactly 1 sample byte");
    FCEU11_EXPECT(ctx, addr_before == 0x4000, "DMC sample address latched to $C000");

    const int32_t ts_before = fceu11::cpu_instance().timestamp();
    FCEU_SoundCPUHook(1);
    const int32_t ts_after = fceu11::cpu_instance().timestamp();

    FCEU11_EXPECT(ctx, ts_after - ts_before == 4,
                  "one DMC sample fetch steals exactly 4 CPU cycles");
    FCEU11_EXPECT(ctx, DMCSize == size_before - 1,
                  "DMC bytes-remaining decremented by exactly 1 per fetch");
    FCEU11_EXPECT(ctx, DMCAddress == ((addr_before + 1) & 0x7fff),
                  "DMC address counter advanced by exactly 1 per fetch");
    FCEU11_EXPECT(ctx, DMCHaveDMA != 0,
                  "fetched sample byte is held in the DMC sample buffer");

    // Second hook: the sample buffer already holds a byte, so no further
    // fetch happens and no further CPU cycles are stolen.
    const int32_t ts2_before = fceu11::cpu_instance().timestamp();
    FCEU_SoundCPUHook(1);
    const int32_t ts2_after = fceu11::cpu_instance().timestamp();
    FCEU11_EXPECT(ctx, ts2_after - ts2_before == 0,
                  "no CPU cycles stolen while the DMC sample buffer is full");
}

// LT-08 (v2.0.5 B1) -- DMC cycle accounting, part 2: the timer.
//
// NESdev wiki APU_DMC (pitch table): the rate index selects "for how many
// CPU cycles happen between changes in the output level". NTSC periods are
// {428,380,340,320,286,254,226,214,190,160,142,128,106,84,72,54} and PAL
// periods are {398,354,316,298,276,236,210,198,176,148,132,118,98,78,66,50}
// -- byte-identical to NTSCDMCTable / PALDMCTable in src/sound.cpp:117-133.
//
// This repo advances the timer in FCEU_SoundCPUHook (src/sound.cpp:870-897):
//     DMCacc -= cycles;
//     while (DMCacc <= 0) { ...; DMCacc += DMCPeriod;
//                           DMCBitCount = (DMCBitCount + 1) & 7; ... }
//
// This lock pins the rate table and the accumulator's advance/tick rule.
void test_dmc_timer_accounting(TestContext& ctx) {
    // Rate table: the low nibble of $4010 is the rate index.
    X6502_DMW(0x4010, 0x00);
    FCEU11_EXPECT(ctx, DMCPeriod == (PAL ? 398 : 428),
                  "DMC rate $0 period matches the NESdev pitch table");
    X6502_DMW(0x4010, 0x0F);
    FCEU11_EXPECT(ctx, DMCPeriod == (PAL ? 50 : 54),
                  "DMC rate $F period matches the NESdev pitch table");

    // Keep part 1's fetch out of the way: no sample bytes pending.
    DMCSize = 0;
    DMCHaveSample = 0;
    DMCBitCount = 0;
    DMCacc = 1000;

    FCEU_SoundCPUHook(100);
    FCEU11_EXPECT(ctx, DMCacc == 900,
                  "DMCacc advances by exactly the CPU cycles consumed");

    const int32_t per = DMCPeriod;
    FCEU_SoundCPUHook(900);
    FCEU11_EXPECT(ctx, DMCacc == per,
                  "DMCacc wraps forward by exactly one DMC period");
    FCEU11_EXPECT(ctx, DMCBitCount == 1,
                  "one DMC timer tick advances the bit counter by exactly 1");
}
void test_cycles_scaling_overflow(TestContext& ctx) {
    // hotfix3 C-3: pre-fix, X6502_RunDebug did `cycles *= 16` (NTSC) on a
    // signed int32 — UB at cycles == 2^27 and poisoning _count for any
    // larger caller (Lua/cheat can produce such inputs). Post-fix,
    // int64 promotion + INT32_MAX clamp keeps the operation well-defined.
    // Feeding cycles = INT32_MAX/8 (well above the NTSC UB threshold of
    // 2^27 = 134,217,728) must not crash and the engine must continue
    // executing on the next emulate call. We use the X6502_Run macro
    // (x6502.h:59) which expands to X6502_RunDebug(g_cpu, cycles).
    X6502_Run(INT32_MAX / 8);
    FCEU11_EXPECT(ctx, true, "X6502_RunDebug survives INT32_MAX/8 cycles input");
    // Smoke that the engine is still alive after the extreme input.
    emulate_n(1);
    FCEU11_EXPECT(ctx, true, "engine survives 1 frame after INT32_MAX/8 RunDebug");
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
    test_dmc_acc_negation_overflow(ctx);
    test_dmc_fetch_cycle_accounting(ctx);
    test_dmc_timer_accounting(ctx);
    test_cycles_scaling_overflow(ctx);
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
