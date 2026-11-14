
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdint>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "ines.h"
#include "x6502.h"
#include "ppu.h"             // NTARAM -- mapper 218's CHR-RAM -> NTARAM mapping lock
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
    // v2.0.4 V9: mapper 218 wires CHR-RAM's 1K pages into NTARAM.
    { "fixtures/mapper_mapper218.nes", 218 },
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

// v2.0.3 S5-A3: mapper 198 (TW MMC3+VRAM Rev. E) runs mapper 195's power
// function upstream -- `info->Power = M195Power` -- which maps the board's
// 16 KiB WRAM at $5000 and installs its read/write handlers.  The generic
// GenMMC3Power() maps the WRAM at $6000 instead, so $5000 stays unmapped and
// the cartridge has no work RAM there.
//
// Lock: write a byte to $5000 and read it back.  With M195Power running it
// lands in the WRAM; without it the write is dropped (open bus) and the read
// comes back as 0x00.
static bool check_mapper198_power(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        printf("FAIL: %s did not load\n", path);
        return false;
    }
    if (gi->mappernum != 198) {
        printf("FAIL: %s loaded as mapper %d\n", path, gi->mappernum);
        fceu11::CloseGame();
        return false;
    }
    X6502_DMW(0x5000, 0x5A);
    const uint8_t readback = X6502_DMR(0x5000);
    printf("      WRAM[0x5000] readback = 0x%02X\n", readback);
    fceu11::CloseGame();
    if (readback != 0x5A) {
        printf("FAIL: %s: $5000 is not writable -- mapper 198's own power "
               "function (M195Power) did not run\n", path);
        return false;
    }
    return true;
}

// v2.0.4 V9: mapper 218 (the CHR-RAM -> NTARAM board) maps CHR-RAM's 1K
// pages into NTARAM through VPageR[] (src/boards/datalatch.cpp).  The PPU
// reaches CHR with `VPage[v >> 10][v]` (src/ppu.cpp:285): the page comes
// from the top bits of the 12-bit CHR address, but the byte offset is the
// FULL address, unmasked.  A correct mapping therefore needs every VPageR[]
// entry to be an OFFSET base pointer (upstream commit 6738471).
//
// The lock asserts the OBSERVABLE result -- every CHR access lands inside
// NTARAM[0x800] -- and deliberately does NOT restate the pointer formula, so
// the test cannot agree with a wrong implementation (the "test and
// implementation are wrong together" trap this repo has hit before).
//
// With the old formula (the 1K page number used as a byte offset) the
// mapping leaves NTARAM: the fixture's mirrorAs2Bits is 0, so the page map is
// 0,1,0,1,0,1,0,1 and 6145 of the 8192 CHR addresses land outside NTARAM,
// the last one 0x1800 bytes past its end.
static bool check_mapper218_chr_mapping(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        printf("FAIL: %s did not load\n", path);
        return false;
    }
    if (gi->mappernum != 218) {
        printf("FAIL: %s loaded as mapper %d\n", path, gi->mappernum);
        fceu11::CloseGame();
        return false;
    }

    const uintptr_t nt_lo = reinterpret_cast<uintptr_t>(&NTARAM[0]);
    const uintptr_t nt_hi = nt_lo + sizeof(NTARAM);
    uint32_t outside = 0;
    uint32_t first_bad = 0;
    uintptr_t max_over = 0;
    for (uint32_t v = 0; v < 0x2000u; ++v) {
        const uintptr_t addr = reinterpret_cast<uintptr_t>(VPageR[v >> 10]) + v;
        if (addr < nt_lo || addr >= nt_hi) {
            if (outside == 0u) first_bad = v;
            ++outside;
            const uintptr_t over = (addr >= nt_hi) ? (addr - nt_hi) : (nt_lo - addr);
            if (over > max_over) max_over = over;
        }
    }

    bool ok = true;
    if (outside) {
        printf("FAIL: %s: %u of 8192 CHR addresses fall outside NTARAM "
               "(first at 0x%04X, worst is %llu bytes out)\n",
               path, outside, first_bad, (unsigned long long)max_over);
        ok = false;
    } else {
        printf("      all 8192 CHR addresses land inside NTARAM\n");
    }
    fceu11::CloseGame();
    return ok;
}

// v2.0.3 S5-A2: the rule that carries the CRC-based ROM correction into the
// mapper number the loader uses.  The corrected mapper has to reach
// `iNES_Init()` and the v1.8 cart factory; before this rule existed the
// correction was computed and then dropped (the whole 251-entry table was
// inert for mapper corrections).
static bool check_effective_mapper_rule() {
    bool ok = true;
    // 吞食天地2: header says mapper 4, upstream's table says 198.
    if (FCEU_EffectiveMapperNo(4, 198) != 198) {
        printf("FAIL: a mapper correction must win over the header's mapper\n");
        ok = false;
    }
    // No correction for this ROM -> the header's mapper stands.
    if (FCEU_EffectiveMapperNo(4, -1) != 4) {
        printf("FAIL: without a correction the header's mapper must stand\n");
        ok = false;
    }
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

    // v2.0.3 S5-A3: mapper 198 runs mapper 195's power function upstream, and
    // that is what puts the board's WRAM at $5000.
    printf("[extra] Checking mapper 198's power function ... ");
    if (check_mapper198_power("fixtures/mapper_mapper198.nes")) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // v2.0.4 V9: mapper 218's CHR-RAM -> NTARAM mapping has to stay inside
    // NTARAM for every CHR address (upstream commit 6738471).
    printf("[extra] Checking mapper 218's CHR-RAM -> NTARAM mapping ... ");
    if (check_mapper218_chr_mapping("fixtures/mapper_mapper218.nes")) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // v2.0.3 S5-A2: the CRC-based ROM-correction table has to be able to
    // replace the header's mapper.
    printf("[extra] Checking the CRC-correction mapper rule ... ");
    if (check_effective_mapper_rule()) {
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
