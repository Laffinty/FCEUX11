// FCEUX11 v0.3.12.5 — byte-level savestate regression test.
// Loads each ROM, runs FRAMES frames, saves state to memory, and compares
// the MD5 digest against fixtures/golden_savestate_hashes.json.
// Run with --generate to rewrite the golden file.

#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>
#include <chrono>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "state.h"
#include "emufile.h"
#include "utils/md5.h"
#include "drivers/common/nes_shm.h"
#include "x6502.h"
#include "ppu.h"

static const int FRAMES_TO_RUN = 60;
static const double WATCHDOG_SECONDS_PER_FRAME = 30.0;
static const char* GOLDEN_HASHES_PATH = "fixtures/golden_savestate_hashes.json";

struct RomTestCase {
    const char* filename;
    const char* name;
};

static const RomTestCase tests[] = {
    { "fixtures/mapper_nrom.nes",         "nrom" },
    { "fixtures/mapper_mmc1.nes",         "mmc1" },
    { "fixtures/mapper_uxrom.nes",        "uxrom" },
    { "fixtures/mapper_cnrom.nes",        "cnrom" },
    { "fixtures/mapper_mmc3.nes",         "mmc3" },
    { "fixtures/mapper_mmc5.nes",         "mmc5" },
    { "fixtures/mapper_axrom.nes",        "axrom" },
    { "fixtures/mapper_colordreams.nes",  "colordreams" },
    { "fixtures/mapper_gnrom.nes",        "gnrom" },
    { "fixtures/mapper_vrc2and4.nes",     "vrc2and4" },
    { "fixtures/mapper_vrc6.nes",         "vrc6" },
    // vrc7 is omitted: the VRC7 savestate chunk contains a heap pointer
    // (OPLL.sintbl) and is therefore non-deterministic across process runs.
    { "fixtures/nestest.nes",             "nestest" },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// Minimal JSON helpers (handles only the exact format we produce)
// ---------------------------------------------------------------------------

static bool readGoldenHashes(
    const char* path,
    std::vector<std::string>& out_names,
    std::vector<std::string>& out_hashes)
{
    FILE* f = fopen(path, "rb");
    if (!f) {
        printf("Golden hashes file not found: %s\n", path);
        return false;
    }

    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    std::string json(static_cast<size_t>(len), '\0');
    if (len > 0) {
        fread(json.data(), 1, static_cast<size_t>(len), f);
    }
    fclose(f);

    out_names.clear();
    out_hashes.clear();

    size_t i = 0;
    auto skipSpace = [&]() {
        while (i < json.size() && std::isspace(static_cast<unsigned char>(json[i]))) {
            ++i;
        }
    };
    auto readString = [&]() -> std::string {
        skipSpace();
        if (i >= json.size() || json[i] != '"') {
            return "";
        }
        ++i; // skip opening quote
        std::string s;
        while (i < json.size() && json[i] != '"') {
            if (json[i] == '\\' && i + 1 < json.size()) {
                s += json[i + 1];
                i += 2;
            } else {
                s += json[i];
                ++i;
            }
        }
        if (i < json.size() && json[i] == '"') {
            ++i; // skip closing quote
        }
        return s;
    };

    skipSpace();
    if (i < json.size() && json[i] == '{') {
        ++i;
    }

    while (true) {
        skipSpace();
        if (i < json.size() && json[i] == '}') {
            break;
        }

        std::string name = readString();
        skipSpace();
        if (i < json.size() && json[i] == ':') {
            ++i;
        }
        skipSpace();
        if (i < json.size() && json[i] == '{') {
            ++i;
        }
        std::string key = readString();
        (void)key; // expected to be "hash"
        skipSpace();
        if (i < json.size() && json[i] == ':') {
            ++i;
        }
        std::string hash = readString();
        skipSpace();
        if (i < json.size() && json[i] == '}') {
            ++i;
        }

        if (!name.empty() && !hash.empty()) {
            out_names.push_back(name);
            out_hashes.push_back(hash);
        }

        skipSpace();
        if (i < json.size() && json[i] == ',') {
            ++i;
        }
    }

    return true;
}

static bool writeGoldenHashes(
    const char* path,
    const std::vector<std::string>& names,
    const std::vector<std::string>& hashes)
{
    FILE* f = fopen(path, "w");
    if (!f) {
        printf("Failed to open %s for writing\n", path);
        return false;
    }

    fprintf(f, "{\n");
    for (size_t i = 0; i < names.size(); ++i) {
        fprintf(f, "  \"%s\": {\n", names[i].c_str());
        fprintf(f, "    \"hash\": \"%s\"\n", hashes[i].c_str());
        fprintf(f, "  }%s\n", (i + 1 < names.size()) ? "," : "");
    }
    fprintf(f, "}\n");
    fclose(f);
    return true;
}

// ---------------------------------------------------------------------------
// Savestate MD5
// ---------------------------------------------------------------------------

static std::string computeSavestateHash(const char* romPath)
{
    if (!fceu11::Initialize()) {
        fprintf(stderr, "FCEUI_Initialize failed\n");
        return "";
    }
    // Defensive re-initialization: close any stale SHM from a prior
    // Initialize/Kill cycle, then open a fresh one. This prevents headless
    // test hangs caused by reusing a partially-torn-down shared-memory block.
    close_nes_shm();
    nes_shm = open_nes_shm();

    // Make sure interactive/background features that may block in a GUI-less
    // CI runner are explicitly disabled.
    AutoResumePlay = false;
    FCEU_StateRecorderSetEnabled(false);

    FCEUI_SetInput(0, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInput(1, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE), nullptr, 0);
    FCEUI_SetInputFourscore(false);

    if (!fceu11::LoadGame(romPath, 1, true)) {
        fprintf(stderr, "Failed to load %s\n", romPath);
        fceu11::Kill();
        return "";
    }

    uint8* xbuf = nullptr;
    int32* soundBuf = nullptr;
    int32 soundBufSize = 0;
    for (int i = 0; i < FRAMES_TO_RUN; ++i) {
        auto frame_start = std::chrono::steady_clock::now();
        fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
        auto frame_elapsed = std::chrono::steady_clock::now() - frame_start;
        double seconds = std::chrono::duration<double>(frame_elapsed).count();
        if (seconds > WATCHDOG_SECONDS_PER_FRAME) {
            fprintf(stderr,
                    "WATCHDOG: frame %d took %.1f seconds (limit %.1f). "
                    "PC=%04X scanline=%d timestamp=%u\n",
                    i, seconds, WATCHDOG_SECONDS_PER_FRAME,
                    X.PC, scanline, timestamp);
            abort();
        }
    }

    std::vector<std::byte> buffer;
    EMUFILE_MEMORY file(&buffer);
    if (!FCEUSS_SaveMS(&file, 0)) {
        fprintf(stderr, "FCEUSS_SaveMS failed for %s\n", romPath);
        fceu11::CloseGame();
        fceu11::Kill();
        return "";
    }

    struct md5_context ctx;
    md5_starts(&ctx);
    md5_update(&ctx, reinterpret_cast<uint8*>(buffer.data()), static_cast<uint32>(buffer.size()));
    uint8 digest[16];
    md5_finish(&ctx, digest);

    MD5DATA md5data;
    memcpy(md5data.data, digest, 16);

    fceu11::CloseGame();
    fceu11::Kill();

    return std::string(md5_asciistr(md5data));
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

int main(int argc, char** argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);

    bool generateMode = false;
    if (argc > 1 && strcmp(argv[1], "--generate") == 0) {
        generateMode = true;
    }

    printf("=== FCEUX11 Savestate Regression Test ===\n");
    printf("Mode: %s\n\n", generateMode ? "GENERATE golden hashes" : "VERIFY against golden hashes");

    std::vector<std::string> names;
    std::vector<std::string> hashes;

    for (int i = 0; i < NUM_TESTS; ++i) {
        printf("[%2d/%2d] %s (%s)\n", i + 1, NUM_TESTS, tests[i].name, tests[i].filename);
        std::string hash = computeSavestateHash(tests[i].filename);
        if (hash.empty()) {
            printf("RESULT: FAILED (could not compute hash)\n");
            return 1;
        }
        names.push_back(tests[i].name);
        hashes.push_back(hash);
        printf("  hash: %s\n", hash.c_str());
    }

    if (generateMode) {
        if (!writeGoldenHashes(GOLDEN_HASHES_PATH, names, hashes)) {
            return 1;
        }
        printf("\nWrote %d golden hashes to %s\n", NUM_TESTS, GOLDEN_HASHES_PATH);
        printf("RESULT: GENERATED\n");
        return 0;
    }

    std::vector<std::string> goldenNames;
    std::vector<std::string> goldenHashes;
    if (!readGoldenHashes(GOLDEN_HASHES_PATH, goldenNames, goldenHashes)) {
        printf("\nNo golden hashes found. Run with --generate to create %s\n", GOLDEN_HASHES_PATH);
        return 1;
    }

    int mismatches = 0;
    for (int i = 0; i < NUM_TESTS; ++i) {
        bool found = false;
        for (size_t j = 0; j < goldenNames.size(); ++j) {
            if (goldenNames[j] == tests[i].name) {
                found = true;
                if (goldenHashes[j] != hashes[i]) {
                    printf("MISMATCH %s: expected %s, got %s\n",
                           tests[i].name, goldenHashes[j].c_str(), hashes[i].c_str());
                    ++mismatches;
                }
                break;
            }
        }
        if (!found) {
            printf("MISSING baseline for %s\n", tests[i].name);
            ++mismatches;
        }
    }

    printf("\n=== Results ===\n");
    printf("Compared: %d ROMs\n", NUM_TESTS);
    printf("Mismatches: %d\n", mismatches);
    if (mismatches == 0) {
        printf("RESULT: PASSED\n");
        return 0;
    }
    printf("RESULT: FAILED\n");
    return 1;
}
