// FCEUX11 v2.0.2 A5 — audio WAV diff regression test, HIGH-QUALITY path.
//
// Companion of core/apu_wav_diff_test.cpp (kgmqa-024), which pins the
// LOW-quality (soundq == 0) audio path. That harness never reaches the
// FSettings.soundq >= 1 branch of FlushEmulateSound (sound.cpp:1409), so
// the HQ channel renderers (RDoSQ / RDoTriangle / RDoNoise / RDoPCM), the
// WaveHi dequant loop and the 484-tap NeoFilterSound FIR were uncovered by
// any golden. A5 (NeoFilterSound lossless SIMD) touches exactly that FIR,
// so this test pins the HQ path to byte-exact goldens BEFORE any filter.rs
// change lands (G0 of plan v1 2026-11-06).
//
// Per ROM:
//   1. fceu11::Initialize / LoadGame
//   2. FCEUI_Sound(48000); fceu11::SetSoundQuality(1);
//      (same order as bench_tolerance_test.cpp run_bench — after LoadGame,
//      because fceu11::Initialize() memsets FSettings and SetSoundQuality
//      re-runs SetSoundVariables → MakeFilters with the new soundq)
//   3. Run N frames
//   4. After the last frame, capture the sound buffer returned by Emulate()
//   5. Capture TWO representations:
//        a. int16 WAV (44-byte RIFF header, 48000 Hz mono) — same format as
//           the kgmqa-024 golden, i.e. what the driver would play;
//        b. RAW int32 dump of the mix buffer (ssize * 4 bytes) — the true
//           bit-exact lock: the int16 cast can hide low-bit drift that the
//           FIR/SIMD work is sensitive to.
//   6. In GENERATE mode: write to fixtures/golden_wav_hq/<name>.{wav,pcm32}
//      In VERIFY mode:   read both goldens and memcmp against the capture
//
// Golden files: fixtures/golden_wav_hq/<name>.wav + <name>.pcm32.
// Byte-level memcmp is the diff mechanism; 0-sample tolerance.
//
// Run with --generate to write fresh goldens (run once against the
// UNMODIFIED filter.rs build, then VERIFY twice to confirm determinism).

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>
#include <chrono>
#include <cstdint>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "sound.h"
#include "state.h"
#include "drivers/common/nes_shm.h"

static const int SAMPLE_RATE = 48000;
static const int CHANNELS = 1;
static const int BITS_PER_SAMPLE = 16;
static const int BYTES_PER_SAMPLE = BITS_PER_SAMPLE / 8;
static const int WAV_HEADER_SIZE = 44;
static const double WATCHDOG_SECONDS_PER_FRAME = 30.0;
static const char* GOLDEN_DIR = "fixtures/golden_wav_hq";

struct RomTestCase {
    const char* filename;
    const char* name;
    int         frames;  // number of frames to run before capturing audio
};

// Same ROM set / frame counts as the kgmqa-024 low-quality harness.
// vrc6 must stay: with soundq >= 1 the VRC6 hi_fill (FCEU11_ExpHiFill →
// VRC6EXP->hiFill) is confirmed active, so this ROM exercises the
// neo_fill callback path of NeoFilterSound on top of the base channels.
static const RomTestCase tests[] = {
    { "fixtures/mapper_nrom.nes",  "nrom",  60 },
    { "fixtures/mapper_mmc1.nes",  "mmc1",  90 },
    { "fixtures/mapper_vrc6.nes",  "vrc6",  60 },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// WAV header builder (same 44-byte layout as apu_wav_diff_test.cpp)
// ---------------------------------------------------------------------------

static std::vector<uint8_t> buildWavHeader(uint32_t dataSize)
{
    std::vector<uint8_t> header(WAV_HEADER_SIZE, 0);
    uint32_t fileSize = dataSize + WAV_HEADER_SIZE - 8;
    uint32_t byteRate = SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE;
    uint16_t blockAlign = CHANNELS * BYTES_PER_SAMPLE;

    std::memcpy(header.data() + 0,  "RIFF", 4);
    std::memcpy(header.data() + 4,  &fileSize, 4);
    std::memcpy(header.data() + 8,  "WAVE", 4);

    std::memcpy(header.data() + 12, "fmt ", 4);
    uint32_t subChunk1Size = 16;
    std::memcpy(header.data() + 16, &subChunk1Size, 4);
    uint16_t audioFormat = 1; // PCM
    uint16_t numChannels = CHANNELS;
    std::memcpy(header.data() + 20, &audioFormat, 2);
    std::memcpy(header.data() + 22, &numChannels, 2);
    std::memcpy(header.data() + 24, &SAMPLE_RATE, 4);
    std::memcpy(header.data() + 28, &byteRate, 4);
    std::memcpy(header.data() + 32, &blockAlign, 2);
    uint16_t bitsPerSample = BITS_PER_SAMPLE;
    std::memcpy(header.data() + 34, &bitsPerSample, 2);

    std::memcpy(header.data() + 36, "data", 4);
    std::memcpy(header.data() + 40, &dataSize, 4);

    return header;
}

// ---------------------------------------------------------------------------
// Per-ROM: load, run, capture audio buffer (int16 WAV + raw int32)
// ---------------------------------------------------------------------------

static bool captureAudio(const char* romPath, int framesToRun,
                         std::vector<uint8_t>& outWav,
                         std::vector<uint8_t>& outPcm32)
{
    if (!fceu11::Initialize()) {
        std::fprintf(stderr, "FCEUI_Initialize failed for %s\n", romPath);
        return false;
    }
    close_nes_shm();
    nes_shm = open_nes_shm();

    // Disable interactive / GUI-only features.
    AutoResumePlay = false;
    FCEU_StateRecorderSetEnabled(false);
    FCEUI_SetInput(0,   static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInput(1,   static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),   nullptr, 0);
    FCEUI_SetInputFourscore(false);

    if (!fceu11::LoadGame(romPath, 1, true)) {
        std::fprintf(stderr, "LoadGame failed for %s\n", romPath);
        fceu11::Kill();
        return false;
    }

    // High-quality sound path. Order mirrors bench_tolerance_test.cpp
    // run_bench(): after LoadGame, FCEUI_Sound first, then SetSoundQuality
    // (which re-runs SetSoundVariables → MakeFilters with soundq = 1).
    FCEUI_Sound(SAMPLE_RATE);
    fceu11::SetSoundQuality(1);

    uint8*  xbuf  = nullptr;
    int32*  sbuf  = nullptr;
    int32   ssize = 0;
    for (int i = 0; i < framesToRun; ++i) {
        auto t0 = std::chrono::steady_clock::now();
        fceu11::Emulate(&xbuf, &sbuf, &ssize, 0);
        auto dt = std::chrono::steady_clock::now() - t0;
        double secs = std::chrono::duration<double>(dt).count();
        if (secs > WATCHDOG_SECONDS_PER_FRAME) {
            std::fprintf(stderr,
                "WATCHDOG: %s frame %d took %.1fs (limit %.1fs)\n",
                romPath, i, secs, WATCHDOG_SECONDS_PER_FRAME);
            fceu11::CloseGame();
            fceu11::Kill();
            return false;
        }
    }

    if (!sbuf || ssize <= 0) {
        std::fprintf(stderr, "No sound buffer after %d frames for %s\n",
                     framesToRun, romPath);
        fceu11::CloseGame();
        fceu11::Kill();
        return false;
    }

    // (a) int16 mono PCM WAV — what the SDL callback would cast out.
    const uint32_t dataSize = static_cast<uint32_t>(ssize) * BYTES_PER_SAMPLE;
    std::vector<uint8_t> wav;
    wav.reserve(WAV_HEADER_SIZE + dataSize);

    std::vector<uint8_t> header = buildWavHeader(dataSize);
    wav.insert(wav.end(), header.begin(), header.end());

    for (int i = 0; i < ssize; ++i) {
        int16_t sample = static_cast<int16_t>(sbuf[i]);
        wav.push_back(static_cast<uint8_t>(sample & 0xFF));
        wav.push_back(static_cast<uint8_t>((sample >> 8) & 0xFF));
    }

    // (b) raw int32 mix buffer dump — the bit-exact lock.
    const uint8_t* raw = reinterpret_cast<const uint8_t*>(sbuf);
    outPcm32.assign(raw, raw + static_cast<size_t>(ssize) * 4);

    outWav = std::move(wav);

    fceu11::CloseGame();
    fceu11::Kill();
    return true;
}

// ---------------------------------------------------------------------------
// Golden file I/O
// ---------------------------------------------------------------------------

static std::string goldenPath(const char* name, const char* ext) {
    std::string p = GOLDEN_DIR;
    p += "/";
    p += name;
    p += ext;
    return p;
}

static bool writeGolden(const char* path, const std::vector<uint8_t>& bytes) {
    FILE* f = std::fopen(path, "wb");
    if (!f) {
        std::fprintf(stderr, "Cannot open golden for write: %s\n", path);
        return false;
    }
    size_t wrote = std::fwrite(bytes.data(), 1, bytes.size(), f);
    std::fclose(f);
    if (wrote != bytes.size()) {
        std::fprintf(stderr, "Short write on %s (%zu/%zu)\n",
                     path, wrote, bytes.size());
        return false;
    }
    return true;
}

static bool readGolden(const char* path, std::vector<uint8_t>& out) {
    FILE* f = std::fopen(path, "rb");
    if (!f) return false;
    std::fseek(f, 0, SEEK_END);
    long len = std::ftell(f);
    std::fseek(f, 0, SEEK_SET);
    if (len <= 0) {
        std::fprintf(stderr, "Golden %s is empty\n", path);
        std::fclose(f);
        return false;
    }
    out.assign(static_cast<size_t>(len), 0);
    size_t got = std::fread(out.data(), 1, static_cast<size_t>(len), f);
    std::fclose(f);
    return got == static_cast<size_t>(len);
}

// Verify one captured artifact against its golden. Returns false on any
// mismatch (missing golden counts as a failure — VERIFY is the default and
// only mode of the ctest entry).
static bool verifyArtifact(const std::string& gpath,
                           const std::vector<uint8_t>& captured)
{
    std::vector<uint8_t> golden;
    if (!readGolden(gpath.c_str(), golden)) {
        std::printf("  RESULT: missing golden %s\n", gpath.c_str());
        return false;
    }
    if (golden.size() != captured.size()) {
        std::printf("  RESULT: size mismatch (%s: golden %zu, captured %zu)\n",
                    gpath.c_str(), golden.size(), captured.size());
        return false;
    }
    if (std::memcmp(golden.data(), captured.data(), captured.size()) != 0) {
        size_t diffOffset = 0;
        for (size_t j = 0; j < captured.size(); ++j) {
            if (golden[j] != captured[j]) {
                diffOffset = j;
                break;
            }
        }
        std::printf("  RESULT: byte mismatch in %s at offset %zu\n",
                    gpath.c_str(), diffOffset);
        return false;
    }
    return true;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

int main(int argc, char** argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);

    bool generateMode = false;
    if (argc > 1 && std::strcmp(argv[1], "--generate") == 0) {
        generateMode = true;
    }

    std::printf("=== FCEUX11 A5 HQ APU WAV Diff Test (soundq=1, 48000 Hz) ===\n");
    std::printf("Mode: %s\n\n",
                generateMode ? "GENERATE goldens" : "VERIFY against goldens");

    int failures = 0;
    for (int i = 0; i < NUM_TESTS; ++i) {
        const RomTestCase& tc = tests[i];
        std::printf("[%d/%d] %-5s (%s, %d frames)\n",
                    i + 1, NUM_TESTS, tc.name, tc.filename, tc.frames);

        std::vector<uint8_t> wav;
        std::vector<uint8_t> pcm32;
        if (!captureAudio(tc.filename, tc.frames, wav, pcm32)) {
            std::printf("  RESULT: capture failed\n");
            ++failures;
            continue;
        }

        if (generateMode) {
            std::string wpath = goldenPath(tc.name, ".wav");
            std::string ppath = goldenPath(tc.name, ".pcm32");
            if (!writeGolden(wpath.c_str(), wav) ||
                !writeGolden(ppath.c_str(), pcm32)) {
                std::printf("  RESULT: golden write failed\n");
                ++failures;
                continue;
            }
            std::printf("  ok    wrote goldens: %s (%zu bytes), %s (%zu bytes)\n",
                        wpath.c_str(), wav.size(), ppath.c_str(), pcm32.size());
        } else {
            std::string wpath = goldenPath(tc.name, ".wav");
            std::string ppath = goldenPath(tc.name, ".pcm32");
            bool okW = verifyArtifact(wpath, wav);
            bool okP = okW && verifyArtifact(ppath, pcm32);
            if (okW && okP) {
                std::printf("  ok    wav %zu bytes + pcm32 %zu bytes match\n",
                            wav.size(), pcm32.size());
            } else {
                ++failures;
                continue;
            }
        }
    }

    std::printf("\n=== Summary ===\n");
    std::printf("Tests:  %d\n", NUM_TESTS);
    std::printf("Passed: %d\n", NUM_TESTS - failures);
    std::printf("Failed: %d\n", failures);
    std::printf("RESULT: %s\n", failures == 0 ? "PASSED" : "FAILED");

    return failures == 0 ? 0 : 1;
}
