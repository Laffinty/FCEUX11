// FCEUX11 v1.7 Cartograph — mapper state byte-diff regression test (Phase A).
//
// Goal: pin down mapper internal state to a byte-exact golden, so any v1.7
// refactor of the Cart class (CartInfo migration, mapper subclass PoCs) that
// drifts a mapper register will fail the test suite.
//
// Per mapper:
//   1. fceu11::Initialize / LoadGame
//   2. Run N frames (per docs/internal/v1.7_mapper_byte_diff_test_design.md)
//   3. After the last frame, capture the Cart::save_mapper_state() output
//   4. Validate header magic + version + total_size
//   5. In GENERATE mode: write to fixtures/golden_mapper/<name>.bin
//      In VERIFY mode:   read golden, memcmp body bytes
//
// Golden file format (16-byte header + 64-512 byte body):
//   magic:    "FMAP\0\0\0" (8 bytes)
//   version:  uint32 LE (= 1)
//   total:    uint32 LE (= HEADER + body size)
//   body:     mapper-specific binary snapshot
//
// Run with --generate to write fresh goldens (deferred to Phase E/F when
// Cart subclasses are implemented).
//
// Phase A acceptance: compiles + links + runs; all tests [SKIP] until
// Phase E (NROM PoC) generates the first golden.

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
#include "state.h"
#include "drivers/Qt/nes_shm.h"

static const size_t HEADER_SIZE = 16;
static const uint8_t MAGIC[8] = {'F', 'M', 'A', 'P', 0, 0, 0, 0};
static const uint32_t VERSION = 1;
static const double WATCHDOG_SECONDS_PER_FRAME = 30.0;
static const char*  GOLDEN_DIR = "fixtures/golden_mapper";

struct RomTestCase {
    const char* filename;
    const char* name;
    int         frames;
    uint32_t    total_size;  // body size (excluding 16-byte header)
};

static const RomTestCase tests[] = {
    // docs/internal/v1.7_mapper_byte_diff_test_design.md §2.4
    { "fixtures/mapper_nrom.nes", "nrom", 60,  64  },
    { "fixtures/mapper_mmc1.nes", "mmc1", 90,  256 },
    { "fixtures/mapper_mmc3.nes", "mmc3", 120, 512 },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// Phase A stubs — all [SKIP] until Cart::save_mapper_state is implemented
// ---------------------------------------------------------------------------

// Build the golden file content (header + zero-filled body) for --generate mode.
static std::vector<uint8_t> buildGoldenZeroFilled(uint32_t body_size) {
    std::vector<uint8_t> out(HEADER_SIZE + body_size, 0);
    std::memcpy(out.data(),         MAGIC, 8);
    uint32_t version = VERSION;
    uint32_t total = HEADER_SIZE + body_size;
    std::memcpy(out.data() + 8,  &version, 4);
    std::memcpy(out.data() + 12, &total, 4);
    return out;
}

// Try to open the golden file. Returns empty vector if not found.
static std::vector<uint8_t> readGoldenFile(const char* name) {
    char path[512];
    std::snprintf(path, sizeof(path), "%s/%s.bin", GOLDEN_DIR, name);
    FILE* fp = std::fopen(path, "rb");
    if (!fp) return {};
    std::fseek(fp, 0, SEEK_END);
    long size = std::ftell(fp);
    std::fseek(fp, 0, SEEK_SET);
    std::vector<uint8_t> data(static_cast<size_t>(size));
    size_t n = std::fread(data.data(), 1, data.size(), fp);
    std::fclose(fp);
    if (n != data.size()) return {};
    return data;
}

// Validate golden header. Returns true if magic+version+size match.
static bool validateGoldenHeader(const std::vector<uint8_t>& data,
                                 uint32_t expected_body_size) {
    if (data.size() < HEADER_SIZE) return false;
    if (std::memcmp(data.data(), MAGIC, 8) != 0) return false;
    uint32_t version;
    std::memcpy(&version, data.data() + 8, 4);
    if (version != VERSION) return false;
    uint32_t total;
    std::memcpy(&total, data.data() + 12, 4);
    if (total != HEADER_SIZE + expected_body_size) return false;
    return true;
}

// Write the golden file (--generate mode).
static bool writeGoldenFile(const char* name, const std::vector<uint8_t>& data) {
    char path[512];
    std::snprintf(path, sizeof(path), "%s/%s.bin", GOLDEN_DIR, name);
    // v0.3.x mkdir-p fallback: rely on test runner to pre-create GOLDEN_DIR.
    // (applies to --generate mode only; verify mode reads without writing.)
    FILE* fp = std::fopen(path, "wb");
    if (!fp) {
        std::fprintf(stderr, "Cannot write golden %s\n", path);
        return false;
    }
    size_t n = std::fwrite(data.data(), 1, data.size(), fp);
    std::fclose(fp);
    return n == data.size();
}

// ---------------------------------------------------------------------------
// main — GENERATE or VERIFY
// ---------------------------------------------------------------------------

int main(int argc, char** argv) {
    bool generate = (argc > 1 && std::strcmp(argv[1], "--generate") == 0);

    int total = NUM_TESTS;
    int failed = 0;
    int skipped = 0;
    int generated = 0;

    std::printf("=== mapper_byte_diff_test (Phase A skeleton) ===\n");
    std::printf("Mode: %s\n", generate ? "GENERATE" : "VERIFY");
    std::printf("Total cases: %d\n\n", total);

    for (int i = 0; i < NUM_TESTS; ++i) {
        const RomTestCase& t = tests[i];
        if (generate) {
            // Phase A: --generate writes a zero-filled golden with valid header.
            // Phase E/F: replace with real Cart::save_mapper_state output.
            std::vector<uint8_t> golden = buildGoldenZeroFilled(t.total_size);
            if (!writeGoldenFile(t.name, golden)) {
                std::printf("[GENERATE FAIL] %s\n", t.name);
                failed++;
                continue;
            }
            std::printf("[GENERATE] %s: %zu bytes (header + %u body, zero-filled)\n",
                        t.name, golden.size(), t.total_size);
            generated++;
        } else {
            // VERIFY mode
            std::vector<uint8_t> golden = readGoldenFile(t.name);
            if (golden.empty()) {
                std::printf("[SKIP] %s: golden not generated yet (run --generate first)\n",
                            t.name);
                skipped++;
                continue;
            }
            if (!validateGoldenHeader(golden, t.total_size)) {
                std::printf("[FAIL] %s: golden header validation failed\n", t.name);
                failed++;
                continue;
            }
            // Phase A: body byte-diff is trivially 0 (golden body is zero-filled;
            // expected body is also 0 since Cart::save_mapper_state returns 0
            // until Phase E/F subclasses are implemented).
            std::printf("[SKIP] %s: body byte-diff deferred to Phase E/F (Cart subclass impl)\n",
                        t.name);
            skipped++;
        }
    }

    std::printf("\n=== Summary ===\n");
    std::printf("Total:     %d\n", total);
    if (generate) {
        std::printf("Generated: %d\n", generated);
    } else {
        std::printf("Skipped:   %d\n", skipped);
    }
    std::printf("Failed:    %d\n", failed);
    std::printf("RESULT:    %s\n", failed == 0 ? "PASS" : "FAIL");

    // Phase A: exit 0 (skipped tests don't fail)
    return failed == 0 ? 0 : 1;
}