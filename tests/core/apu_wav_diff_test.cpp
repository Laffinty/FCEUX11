// FCEUX11 v1.6 Resonance — audio WAV diff regression test.
//
// Goal: pin down APU audio output to a byte-exact golden WAV, so any v1.6
// refactor of the APU (Apu class, state migration, ExpansionAudio adapter)
// that drifts a single sample will fail the test suite.
//
// Per ROM:
//   1. fceu11::Initialize / LoadGame
//   2. Run N frames
//   3. After the last frame, capture the sound buffer returned by Emulate()
//   4. Convert the int32 mix buffer to 16-bit mono PCM
//   5. Wrap it in a standard 44-byte WAV header (44100 Hz, mono, 16-bit)
//   6. In GENERATE mode: write to fixtures/golden_wav/<rom>.wav
//      In VERIFY mode:   read the golden file and memcmp against the capture
//
// Golden file format:
//   Standard RIFF/WAVE PCM, 44-byte header + 16-bit mono samples.
//   Sample rate is fixed at 44100 Hz to match the default FCEUX11 audio
//   configuration. Byte-level memcmp is the diff mechanism.
//
// Run with --generate to write fresh goldens (run once after the v1.5
// baseline is locked; rerun ONLY when an intentional audio change lands).

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
#include "drivers/Qt/nes_shm.h"

static const int SAMPLE_RATE = 44100;
static const int CHANNELS = 1;
static const int BITS_PER_SAMPLE = 16;
static const int BYTES_PER_SAMPLE = BITS_PER_SAMPLE / 8;
static const int WAV_HEADER_SIZE = 44;
static const double WATCHDOG_SECONDS_PER_FRAME = 30.0;
static const char* GOLDEN_DIR = "fixtures/golden_wav";

struct RomTestCase {
    const char* filename;
    const char* name;
    int         frames;  // number of frames to run before capturing audio
};

static const RomTestCase tests[] = {
    // v1.6 Phase A baseline ROMs.
    // Frame counts chosen to produce stable, non-silent audio output.
    { "fixtures/mapper_nrom.nes",  "nrom",  60 },
    { "fixtures/mapper_mmc1.nes",  "mmc1",  90 },
    { "fixtures/mapper_vrc6.nes",  "vrc6",  60 },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// WAV header builder
// ---------------------------------------------------------------------------

static std::vector<uint8_t> buildWavHeader(uint32_t dataSize)
{
    std::vector<uint8_t> header(WAV_HEADER_SIZE, 0);
    uint32_t fileSize = dataSize + WAV_HEADER_SIZE - 8;
    uint32_t byteRate = SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE;
    uint16_t blockAlign = CHANNELS * BYTES_PER_SAMPLE;

    // RIFF chunk descriptor
    std::memcpy(header.data() + 0,  "RIFF", 4);
    std::memcpy(header.data() + 4,  &fileSize, 4);
    std::memcpy(header.data() + 8,  "WAVE", 4);

    // fmt sub-chunk
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

    // data sub-chunk
    std::memcpy(header.data() + 36, "data", 4);
    std::memcpy(header.data() + 40, &dataSize, 4);

    return header;
}

// ---------------------------------------------------------------------------
// Per-ROM: load, run, capture audio buffer
// ---------------------------------------------------------------------------

static bool captureAudio(const char* romPath, int framesToRun,
                         std::vector<uint8_t>& outWav)
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

    // Fix the audio rate for deterministic golden output.
    FCEUI_Sound(SAMPLE_RATE);

    if (!fceu11::LoadGame(romPath, 1, true)) {
        std::fprintf(stderr, "LoadGame failed for %s\n", romPath);
        fceu11::Kill();
        return false;
    }

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

    // Convert int32 mix buffer to 16-bit mono PCM.
    // The driver casts these values directly to int16 in the SDL callback,
    // so we replicate that behavior for a byte-exact representation of
    // what the user would hear.
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

    outWav = std::move(wav);

    fceu11::CloseGame();
    fceu11::Kill();
    return true;
}

// ---------------------------------------------------------------------------
// Golden file I/O
// ---------------------------------------------------------------------------

static std::string goldenPath(const char* name) {
    std::string p = GOLDEN_DIR;
    p += "/";
    p += name;
    p += ".wav";
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
    if (len < WAV_HEADER_SIZE) {
        std::fprintf(stderr,
            "Golden %s is too small: %ld bytes (min %d)\n",
            path, len, WAV_HEADER_SIZE);
        std::fclose(f);
        return false;
    }
    out.assign(static_cast<size_t>(len), 0);
    size_t got = std::fread(out.data(), 1, static_cast<size_t>(len), f);
    std::fclose(f);
    return got == static_cast<size_t>(len);
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

    std::printf("=== FCEUX11 v1.6 APU WAV Diff Test ===\n");
    std::printf("Mode: %s\n\n",
                generateMode ? "GENERATE goldens" : "VERIFY against goldens");

    int failures = 0;
    for (int i = 0; i < NUM_TESTS; ++i) {
        const RomTestCase& tc = tests[i];
        std::printf("[%d/%d] %-5s (%s, %d frames)\n",
                    i + 1, NUM_TESTS, tc.name, tc.filename, tc.frames);

        std::vector<uint8_t> wav;
        if (!captureAudio(tc.filename, tc.frames, wav)) {
            std::printf("  RESULT: capture failed\n");
            ++failures;
            continue;
        }

        std::string gpath = goldenPath(tc.name);
        if (generateMode) {
            if (!writeGolden(gpath.c_str(), wav)) {
                std::printf("  RESULT: golden write failed\n");
                ++failures;
                continue;
            }
            std::printf("  ok    wrote golden (%zu bytes): %s\n",
                        wav.size(), gpath.c_str());
        } else {
            std::vector<uint8_t> golden;
            if (!readGolden(gpath.c_str(), golden)) {
                std::printf("  RESULT: missing golden %s\n", gpath.c_str());
                ++failures;
                continue;
            }
            if (golden.size() != wav.size()) {
                std::printf("  RESULT: size mismatch (golden %zu, captured %zu)\n",
                            golden.size(), wav.size());
                ++failures;
                continue;
            }
            if (std::memcmp(golden.data(), wav.data(), wav.size()) != 0) {
                // Find first differing sample for diagnostics.
                size_t diffOffset = 0;
                for (size_t j = 0; j < wav.size(); ++j) {
                    if (golden[j] != wav[j]) {
                        diffOffset = j;
                        break;
                    }
                }
                std::printf("  RESULT: byte mismatch at offset %zu\n", diffOffset);
                ++failures;
                continue;
            }
            std::printf("  ok    %zu bytes match\n", wav.size());
        }
    }

    std::printf("\n=== Summary ===\n");
    std::printf("Tests:  %d\n", NUM_TESTS);
    std::printf("Passed: %d\n", NUM_TESTS - failures);
    std::printf("Failed: %d\n", failures);
    std::printf("RESULT: %s\n", failures == 0 ? "PASSED" : "FAILED");

    return failures == 0 ? 0 : 1;
}
