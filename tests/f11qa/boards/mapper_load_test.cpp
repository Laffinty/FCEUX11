
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "ines.h"
#include "x6502.h"
#include "boards/registry.h"
#include "boards/mmc3.h"

struct MapperTestCase {
    const char* filename;
    int expectedMapper;
};

static const MapperTestCase tests[] = {
    { "fixtures/mapper_nrom.nes",   0 },
    { "fixtures/mapper_mmc1.nes",   1 },
    { "fixtures/mapper_uxrom.nes",  2 },
    { "fixtures/mapper_mmc3.nes",   4 },
    { "fixtures/mapper_mmc5.nes",   5 },
    { "fixtures/mapper_axrom.nes",  7 },
    { "fixtures/mapper_vrc6.nes",  24 },
    { "fixtures/mapper_vrc7.nes",  85 },
    // GitHub issue #12: PRG 12 x 16 KiB (not a power of two) + mapper 191.
    // The old loader rounded the PRG size up to 16 banks, so this file was
    // rejected as "Unknown ROM file format".
    { "fixtures/mapper_mmc3_191_nonpow2.nes", 191 },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// Mapper 199 has a power function of its own (boards/199.cpp: M199Power),
// which seeds the DBZ expanded registers and installs the 0x8001 write
// handler.  The loader routes the power cycle through
// CartInfo_PowerForward -> cart_obj->on_power(), so the legacy
// `info->Power = M199Power` never runs -- the v1.8 Cart has to call it.
// If it did not, EXPREGS would still be zero and the 0xC000/0xE000 windows
// would point at PRG bank 0, i.e. the game would run the wrong code.
static bool check_mapper199_power(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        printf("FAIL: %s did not load\n", path);
        return false;
    }
    if (gi->mappernum != 199) {
        printf("FAIL: %s loaded as mapper %d\n", path, gi->mappernum);
        fceu11::CloseGame();
        return false;
    }
    const uint8_t expected[4] = { 0xFE, 0xFF, 0x01, 0x03 };
    printf("      EXPREGS = %02X %02X %02X %02X\n",
           EXPREGS[0], EXPREGS[1], EXPREGS[2], EXPREGS[3]);
    bool ok = true;
    for (int i = 0; i < 4; ++i) {
        if (EXPREGS[i] != expected[i]) {
            printf("FAIL: %s: EXPREGS[%d] is 0x%02X, expected 0x%02X -- "
                   "mapper 199's own power function did not run\n",
                   path, i, EXPREGS[i], expected[i]);
            ok = false;
        }
    }
    fceu11::CloseGame();
    return ok;
}

// GitHub issue #12, end to end: a ROM whose PRG size is not a power of two
// has to load, and the PRG the emulator ends up with must be the file's 12
// banks followed by 0xFF padding -- not the file's CHR swallowed into the
// PRG, which is what slicing the file with the rounded size did.
//
// Mapper 191 maps bank 0 at 0x8000 and the two last banks at 0xC000/0xE000
// (FixMMC3PRG), so 0x8000 is the file's PRG and 0xC000 is the padding.
static bool check_non_power2_prg(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        printf("FAIL: %s did not load\n", path);
        return false;
    }
    const uint8_t prg_start = X6502_DMR(0x8000);
    const uint8_t prg_tail = X6502_DMR(0xC000);
    const uint8_t prg_last = X6502_DMR(0xE000);
    printf("      PRG[0x8000]=0x%02X PRG[0xC000]=0x%02X PRG[0xE000]=0x%02X\n",
           prg_start, prg_tail, prg_last);
    bool ok = true;
    if (prg_start != 0xEA) {
        printf("FAIL: %s: 0x8000 is not the file's PRG (0x%02X)\n", path, prg_start);
        ok = false;
    }
    if (prg_tail != 0xFF || prg_last != 0xFF) {
        printf("FAIL: %s: the PRG tail is not 0xFF padding -- the CHR was "
               "swallowed into the PRG\n", path);
        ok = false;
    }
    fceu11::CloseGame();
    return ok;
}

int main() {
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("=== Mapper Load Test ===\n\n");
    fflush(stdout);

    // Initialize emulator core (required before loading games)
    printf("Calling fceu11::Initialize()...\n");
    fflush(stdout);
    if (!fceu11::Initialize()) {
        printf("FAIL: fceu11::Initialize() returned false\n");
        return 1;
    }
    printf("fceu11::Initialize() OK\n");
    fflush(stdout);

    bool failed = false;
    int passed = 0;

    // GitHub issue #12: mappers 191 and 199 had implementations -- bmap[]
    // ran them through the legacy path -- but no entry in the v1.8
    // MapperEntry registry.  An unregistered mapper keeps g_cart on the
    // no-op placeholder, so the power cycle never runs and the ROM comes
    // up grey.  Assert the entries exist before the load checks below.
    for (uint32_t mapper : {191u, 199u}) {
        const fceu11::MapperEntry* entry = fceu11::find_mapper(mapper);
        if (!entry) {
            printf("FAIL: mapper %u has no registry entry\n", mapper);
            failed = true;
        } else if (entry->mapper_number != mapper) {
            printf("FAIL: mapper %u registry entry reports mapper %u\n",
                   mapper, entry->mapper_number);
            failed = true;
        } else {
            printf("OK: mapper %u registered as \"%s\"\n", mapper, entry->name);
        }
    }

    for (int i = 0; i < NUM_TESTS; ++i) {
        const char* path = tests[i].filename;
        int expected = tests[i].expectedMapper;

        printf("[%d/%d] Loading %-30s (expected mapper %3d) ... ",
               i + 1, NUM_TESTS, path, expected);

        FCEUGI* gi = fceu11::LoadGame(path, 1, true);
        if (!gi) {
            printf("FAIL (load returned null)\n");
            failed = true;
            continue;
        }

        int actual = gi->mappernum;
        if (actual != expected) {
            printf("FAIL (mapper %d)\n", actual);
            failed = true;
        } else {
            printf("OK (mapper %d)\n", actual);
            ++passed;
        }

        fceu11::CloseGame();
    }

    // The byte-level half of the issue #12 lock: the non-power-of-two PRG
    // ROM not only has to load, its PRG has to be the file's PRG plus
    // padding.
    printf("[extra] Checking the non-power-of-two PRG ROM ... ");
    if (check_non_power2_prg("fixtures/mapper_mmc3_191_nonpow2.nes")) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // Mapper 199 has to load, and its own power function has to run.
    printf("[extra] Checking mapper 199's power function ... ");
    if (check_mapper199_power("fixtures/mapper_mapper199.nes")) {
        printf("OK\n");
    } else {
        failed = true;
    }

    fceu11::Kill();

    printf("\n=== Results ===\n");
    printf("Passed: %d / %d\n", passed, NUM_TESTS);

    if (failed) {
        printf("RESULT: FAILED\n");
        return 1;
    } else {
        printf("RESULT: PASSED\n");
        return 0;
    }
}
