
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "utils/crc32.h"
#include "drivers/Qt/nes_shm.h"

// Frame hash dimensions: XBuf is 256x256 bytes, but visible area is 256x240.
static const int FRAME_BUF_SIZE = 256 * 240;
static const int FRAMES_TO_RUN = 60;
static const char* GOLDEN_HASHES_PATH = "fixtures/golden_hashes.json";

struct RomTestCase {
    const char* filename;
    const char* name;
};

static const RomTestCase tests[] = {
    { "fixtures/mapper_nrom.nes",   "nrom" },
    { "fixtures/mapper_mmc1.nes",   "mmc1" },
    { "fixtures/mapper_mmc3.nes",   "mmc3" },
    { "fixtures/mapper_mmc5.nes",   "mmc5" },
    { "fixtures/mapper_axrom.nes",  "axrom" },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// Minimal JSON helpers (only handles the exact format we produce)
// ---------------------------------------------------------------------------

static bool readGoldenHashes(
    const char* path,
    std::vector<std::vector<uint32>>& out_hashes,
    std::vector<std::string>& out_names)
{
    FILE* f = fopen(path, "r");
    if (!f) {
        printf("Golden hashes file not found: %s\n", path);
        return false;
    }

    out_hashes.clear();
    out_names.clear();

    // Very simple parser: look for "name": { "frames": [ num, num, ... ] }
    // We scan character by character.
    int c;
    std::string currentName;
    std::vector<uint32> currentFrames;
    enum State {
        S_TOP,       // looking for "name"
        S_IN_OBJECT, // inside { ... }
        S_IN_ARRAY,  // inside [ ... ]
        S_IN_NUMBER, // reading a number
    };
    State state = S_TOP;
    bool inQuotes = false;
    std::string token;
    bool expectFramesKey = false;

    while ((c = fgetc(f)) != EOF) {
        if (c == '"') {
            if (!inQuotes) {
                token.clear();
                inQuotes = true;
            } else {
                inQuotes = false;
                if (state == S_TOP) {
                    currentName = token;
                    expectFramesKey = false;
                } else if (state == S_IN_OBJECT) {
                    if (token == "frames") {
                        expectFramesKey = true;
                    }
                }
            }
            continue;
        }

        if (inQuotes) {
            token.push_back(static_cast<char>(c));
            continue;
        }

        switch (state) {
        case S_TOP:
            if (c == '{') {
                state = S_IN_OBJECT;
            }
            break;
        case S_IN_OBJECT:
            if (c == '[' && expectFramesKey) {
                currentFrames.clear();
                state = S_IN_ARRAY;
                expectFramesKey = false;
            } else if (c == '}') {
                state = S_TOP;
                if (!currentName.empty()) {
                    out_names.push_back(currentName);
                    out_hashes.push_back(currentFrames);
                    currentName.clear();
                    currentFrames.clear();
                }
            }
            break;
        case S_IN_ARRAY:
            if (c == '-' || (c >= '0' && c <= '9')) {
                token.clear();
                token.push_back(static_cast<char>(c));
                state = S_IN_NUMBER;
            } else if (c == ']') {
                state = S_IN_OBJECT;
            }
            break;
        case S_IN_NUMBER:
            if (c >= '0' && c <= '9') {
                token.push_back(static_cast<char>(c));
            } else {
                currentFrames.push_back(static_cast<uint32>(strtoul(token.c_str(), nullptr, 10)));
                if (c == ']') {
                    state = S_IN_OBJECT;
                } else {
                    state = S_IN_ARRAY;
                }
            }
            break;
        }
    }

    fclose(f);
    return true;
}

static bool writeGoldenHashes(
    const char* path,
    const std::vector<std::vector<uint32>>& hashes,
    const std::vector<std::string>& names)
{
    FILE* f = fopen(path, "w");
    if (!f) {
        printf("Failed to open %s for writing\n", path);
        return false;
    }

    fprintf(f, "{\n");
    for (size_t i = 0; i < names.size(); ++i) {
        fprintf(f, "  \"%s\": {\n", names[i].c_str());
        fprintf(f, "    \"frames\": [");
        for (size_t j = 0; j < hashes[i].size(); ++j) {
            fprintf(f, "%lu", static_cast<unsigned long>(hashes[i][j]));
            if (j + 1 < hashes[i].size()) fprintf(f, ", ");
        }
        fprintf(f, "]\n");
        fprintf(f, "  }%s\n", (i + 1 < names.size()) ? "," : "");
    }
    fprintf(f, "}\n");
    fclose(f);
    return true;
}

// ---------------------------------------------------------------------------
// CRC32 of a frame buffer
// ---------------------------------------------------------------------------

static uint32 computeFrameCRC32(const uint8* buf, int len)
{
    return CalcCRC32(0, const_cast<uint8*>(buf), static_cast<uint32>(len));
}

// ---------------------------------------------------------------------------
// Main test logic
// ---------------------------------------------------------------------------

int main(int argc, char** argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);

    bool generateMode = false;
    if (argc > 1 && strcmp(argv[1], "--generate") == 0) {
        generateMode = true;
    }

    printf("=== FCEUX11 ROM Regression Test ===\n");
    printf("Mode: %s\n\n", generateMode ? "GENERATE golden hashes" : "VERIFY against golden hashes");

    if (!FCEUI_Initialize()) {
        printf("FAIL: FCEUI_Initialize() returned false\n");
        return 1;
    }

    // Initialize the shared memory structure required by Qt video/avi drivers
    if (!nes_shm) {
        nes_shm = open_nes_shm();
    }

    // Set up dummy input to prevent null driver dereference in FCEU_UpdateInput
    // v0.3.8: SI_*/SIFC_* are `inline constexpr int` aliases (back-compat);
    // FCEUI_SetInput[FC] takes typed ESI/ESIFC (= fceu11::InputDevice[FC]).
    FCEUI_SetInput(0, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInput(1, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE), nullptr, 0);
    FCEUI_SetInputFourscore(false);

    std::vector<std::vector<uint32>> collected_hashes;
    std::vector<std::string> collected_names;

    bool anyFailed = false;

    for (int i = 0; i < NUM_TESTS; ++i) {
        const char* path = tests[i].filename;
        const char* name = tests[i].name;

        printf("[%d/%d] Testing %s (%s)\n", i + 1, NUM_TESTS, name, path);

        FCEUGI* gi = FCEUI_LoadGame(path, 1, true);
        if (!gi) {
            printf("  FAIL: could not load ROM\n");
            anyFailed = true;
            continue;
        }

        std::vector<uint32> frameHashes;
        frameHashes.reserve(FRAMES_TO_RUN);

        for (int frame = 0; frame < FRAMES_TO_RUN; ++frame) {
            uint8* xbuf = nullptr;
            int32* soundBuf = nullptr;
            int32 soundBufSize = 0;
            FCEUI_Emulate(&xbuf, &soundBuf, &soundBufSize, 0);

            if (xbuf) {
                uint32 crc = computeFrameCRC32(xbuf, FRAME_BUF_SIZE);
                frameHashes.push_back(crc);
            } else {
                frameHashes.push_back(0);
            }
        }

        FCEUI_CloseGame();

        collected_names.push_back(name);
        collected_hashes.push_back(frameHashes);
        printf("  Collected %zu frame hashes\n", frameHashes.size());
    }

    FCEUI_Kill();

    if (generateMode) {
        if (!writeGoldenHashes(GOLDEN_HASHES_PATH, collected_hashes, collected_names)) {
            printf("\nFAIL: Could not write golden hashes\n");
            return 1;
        }
        printf("\nGolden hashes written to %s\n", GOLDEN_HASHES_PATH);
        printf("RESULT: PASSED (baseline generated)\n");
        return 0;
    }

    // Verify mode
    std::vector<std::vector<uint32>> expected_hashes;
    std::vector<std::string> expected_names;
    if (!readGoldenHashes(GOLDEN_HASHES_PATH, expected_hashes, expected_names)) {
        printf("\nFAIL: Could not read golden hashes. Run with --generate to create baseline.\n");
        return 1;
    }

    int totalCompared = 0;
    int mismatches = 0;

    for (size_t i = 0; i < collected_names.size(); ++i) {
        const std::string& name = collected_names[i];
        const std::vector<uint32>& actual = collected_hashes[i];

        // Find matching expected entry
        size_t expIdx = static_cast<size_t>(-1);
        for (size_t j = 0; j < expected_names.size(); ++j) {
            if (expected_names[j] == name) {
                expIdx = j;
                break;
            }
        }

        if (expIdx == static_cast<size_t>(-1)) {
            printf("  WARNING: no baseline for %s\n", name.c_str());
            continue;
        }

        const std::vector<uint32>& expected = expected_hashes[expIdx];
        size_t compareCount = actual.size() < expected.size() ? actual.size() : expected.size();

        for (size_t f = 0; f < compareCount; ++f) {
            totalCompared++;
            if (actual[f] != expected[f]) {
                if (mismatches < 5) {
                    printf("  MISMATCH %s frame %zu: expected 0x%08X, got 0x%08X\n",
                           name.c_str(), f, static_cast<unsigned int>(expected[f]),
                           static_cast<unsigned int>(actual[f]));
                }
                mismatches++;
            }
        }
    }

    printf("\n=== Results ===\n");
    printf("Compared: %d frames\n", totalCompared);
    printf("Mismatches: %d\n", mismatches);

    if (mismatches > 0 || anyFailed) {
        printf("RESULT: FAILED\n");
        return 1;
    } else {
        printf("RESULT: PASSED\n");
        return 0;
    }
}
