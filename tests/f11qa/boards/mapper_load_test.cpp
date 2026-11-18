
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "cart.h"
#include "ines.h"
#include "x6502.h"
#include "ppu.h"             // NTARAM -- mapper 218's CHR-RAM -> NTARAM mapping lock
#include "boards/registry.h"
#include "boards/mmc3.h"

extern CartInfo iNESCart;     // defined in src/ines.cpp
extern BMAPPINGLocal bmap[];  // defined in src/ines_bmap.h / src/ines.cpp

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
// Lock (A-3 rewrite, 2026-11-19 -- closes the v2.0.5 r2 ④ observation): the
// original body was a bare write-then-read, and the mutation leg (M195Power's
// setprg4r + SetWriteHandler/SetReadHandler removed, leaving only
// GenMMC3Power) proved it TOOTHLESS: the readback still returned 0x5A,
// because with $5000 unmapped the open-bus echo hands back the data-bus
// latch that X6502_DMW just filled (the old comment claimed "the read comes
// back as 0x00" -- wrong, measured).  The rewrite uses the A2 WRAM_WINDOW
// technique: handler identity first, then a poison write to $4020 between
// the write and the read, so the open bus echoes the poison, not the payload
// (same immunity argument as the A2 section header above).
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
    // A-3 rewrite (2026-11-19, closes the v2.0.5 r2 observation): the
    // original body was a bare write-then-read and was measured TOOTHLESS
    // under the real mutation (M195Power's $5000 wiring removed): the
    // readback still returned 0x5A, because with $5000 unmapped the
    // open-bus echo hands back the data-bus latch that X6502_DMW just
    // filled (the old comment claimed "the read comes back as 0x00" --
    // wrong, measured).  The rewrite uses the A2 WRAM_WINDOW technique:
    // handler identity first, then a poison write to $4020 between the
    // write and the read, so the open bus echoes the poison, not the
    // payload.  Mutation-verified both ways on 2026-11-19: with the wiring
    // removed this lock FAILS (handlers 0/0, readback 0x00 = the echoed
    // poison); with the wiring restored it passes.  The old-style body was
    // also run against the same mutation and stayed green (readback 0x5A).
    const bool rd_ok = (GetReadHandler(0x5000) == &CartBR);
    const bool wr_ok = (GetWriteHandler(0x5000) == &CartBW);
    X6502_DMW(0x5000, 0x5A);   // into the board's WRAM window (when wired)
    X6502_DMW(0x4020, 0x00);   // poison the open-bus data latch
    const uint8_t readback = X6502_DMR(0x5000);
    printf("      $5000 read handler == CartBR: %d, write handler == "
           "CartBW: %d, WRAM[0x5000] readback = 0x%02X\n",
           rd_ok ? 1 : 0, wr_ok ? 1 : 0, readback);
    fceu11::CloseGame();
    if (!rd_ok || !wr_ok) {
        printf("FAIL: %s: the board's CartBR/CartBW handlers are not "
               "installed at $5000 -- mapper 198's own power function "
               "(M195Power) did not run\n", path);
        return false;
    }
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

// ==================== v2.0.5 A1: unregistered mapper board-power preservation ====================
//
// History: before v2.0.5, src/ines.cpp installed CartInfo_PowerForward (and
// the Reset/Close forwarders) UNCONDITIONALLY after the registry lookup.  For
// a mapper without a MapperEntry registry entry g_cart stays on the no-op
// placeholder, so those forwarders routed Power/Reset/Close into empty
// virtuals and the board-installed functions (MapperNN_Init -> info->Power)
// never ran: no SetWriteHandler / EXPREGS seeds / SRAM mapping / IRQ hooks
// for those numbers at all (v2.0.5 plan §1.4(A); mapper 252's "blue screen"
// symptom traced here).
//
// The fix (A1-a): install the forwarders only when the registry produced a
// concrete Cart object; on a registry miss keep the board-installed pointers.
// The UNIF loader (src/unif.cpp) already used exactly this conditional form.
//
// The sweep list below pins EVERY bmap[] number whose Init chain installs
// info->Power and which has no registry entry: 75 numbers, derived 2026-11-15
// from the runtime registry probe (Research_only/w205/probe, run against the
// shipped libraries) plus a per-board Init-chain analysis.  It supersedes the
// plan's provisional 42-number list, which missed the Latch_Init family
// (Latch_Init installs LatchPower for its callers) and delegate inits
// (Mapper255_Init -> Mapper225_Init, AA6023_Init -> CommonInit, ...).
// Registration status per number is asserted at runtime by the sweep, so a
// future registration of one of these numbers keeps the lock green.

static const uint32_t kUnregisteredBoardPowerMappers[] = {
    8, 29, 101, 107, 113, 134, 140, 144, 152, 153, 154, 155, 157, 159, 171,
    174, 178, 180, 184, 196, 197, 200, 201, 202, 203, 204, 207, 208, 209,
    211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 225, 226,
    227, 228, 229, 230, 231, 232, 233, 234, 235, 238, 240, 241, 242, 243,
    244, 246, 252, 253, 255, 256, 257, 258, 259, 260, 261, 268, 342, 354,
    361, 366, 470, 547,
};

static bool mapper_in_a1_list(uint32_t n) {
    for (uint32_t m : kUnregisteredBoardPowerMappers) {
        if (m == n) return true;
    }
    return false;
}

// Generic minimal iNES ROM: 1 x 16 KiB PRG (NOP sled), no CHR banks ->
// CHR-RAM.  Same recipe as tests/fixtures/generate_test_roms.py, whose
// fixtures already load successfully for ~200 mapper numbers.  Header
// assembled field-by-field per that generator's documented encoding; the
// first two PRG bytes carry the mapper number so each synthetic ROM has a
// distinct CRC32 and cannot collide with the ROM-correction table.
static bool write_minimal_ines_rom(const std::filesystem::path& path,
                                   uint32_t mapper) {
    uint8_t hdr[16] = {0};
    hdr[0] = 'N';
    hdr[1] = 'E';
    hdr[2] = 'S';
    hdr[3] = 0x1A;
    hdr[4] = 1;  // 1 x 16 KiB PRG
    hdr[5] = 0;  // 0 x 8 KiB CHR banks -> CHR-RAM
    // mapper low nibble lives in byte 6 bits 4-7 (both formats)
    hdr[6] = static_cast<uint8_t>((mapper & 0x0F) << 4);
    if (mapper < 256) {
        // iNES 1.0: mapper high nibble in byte 7 bits 4-7
        hdr[7] = static_cast<uint8_t>(mapper & 0xF0);
    } else {
        // iNES 2.0: bits 2-3 of byte 7 = 0b10, mid nibble in bits 4-7,
        // high nibble in byte 8 bits 0-3 (the mapper 406 fixture loads this
        // way since v2.0.3 S4).
        hdr[7] = static_cast<uint8_t>((((mapper >> 4) & 0x0F) << 4) | 0x08);
        hdr[8] = static_cast<uint8_t>((mapper >> 8) & 0x0F);
    }
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    if (!f) return false;
    f.write(reinterpret_cast<const char*>(hdr), sizeof(hdr));
    // exactly 16384 PRG bytes: 2-byte mapper tag prefix, rest NOP sled
    uint8_t prg[16384];
    prg[0] = static_cast<uint8_t>(mapper & 0xFF);
    prg[1] = static_cast<uint8_t>((mapper >> 8) & 0xFF);
    memset(prg + 2, 0xEA, sizeof(prg) - 2);
    f.write(reinterpret_cast<const char*>(prg), sizeof(prg));
    return static_cast<bool>(f);
}

// A1-S1 - classification completeness: every bmap[] entry with a non-null
// init must be either registered in the MapperEntry registry or pinned in
// the sweep list above (unregistered board-power numbers, preserved by the
// A1-a legacy path).  A future board file added to bmap without registration
// turns this red and forces classification: register it, or pin it here so
// its power preservation stays locked by the sweep.  Also asserts the pinned
// list has no stale entries that bmap no longer backs.
static bool check_bmap_classification() {
    bool ok = true;
    int bmap_count = 0, registered = 0, listed = 0;
    for (const BMAPPINGLocal* e = bmap; e->init; ++e) {
        ++bmap_count;
        const uint32_t n = static_cast<uint32_t>(e->number);
        if (fceu11::find_mapper(n)) {
            ++registered;
            continue;
        }
        if (mapper_in_a1_list(n)) {
            ++listed;
            continue;
        }
        printf("FAIL: mapper %u (\"%s\") is in bmap but neither registered "
               "nor pinned in the A1 sweep list\n",
               n, e->name ? e->name : "");
        ok = false;
    }
    for (uint32_t m : kUnregisteredBoardPowerMappers) {
        bool in_bmap = false;
        for (const BMAPPINGLocal* e = bmap; e->init; ++e) {
            if (static_cast<uint32_t>(e->number) == m) {
                in_bmap = true;
                break;
            }
        }
        if (!in_bmap) {
            printf("FAIL: sweep-listed mapper %u is not backed by bmap "
                   "(stale list entry)\n", m);
            ok = false;
        }
    }
    printf("      bmap init-bearing numbers: %d = registered %d + sweep-listed %d\n",
           bmap_count, registered, listed);
    return ok;
}

// A1-S2 - the sweep: for every unregistered board-power mapper, a real load
// must keep the board-installed Power pointer (iNESCart.Power != the
// forwarder) -- that is the fix's contract, exercised through the real
// iNESLoad wiring.  If a future change registers one of these numbers, the
// load routes through a strategy cart instead and iNESCart.Power == the
// forwarder, which is also accepted (the strategy captured the legacy power
// at construction), so the assertion is conditional on the registry state.
static bool check_unregistered_power_sweep(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    std::filesystem::create_directories(dir, ec);
    if (ec) {
        printf("FAIL: cannot create the sweep directory\n");
        return false;
    }
    bool ok = true;
    const int total = static_cast<int>(
        sizeof(kUnregisteredBoardPowerMappers) / sizeof(kUnregisteredBoardPowerMappers[0]));
    int idx = 0;
    for (uint32_t mapper : kUnregisteredBoardPowerMappers) {
        ++idx;
        const std::filesystem::path rom =
            dir / ("a1_mapper_" + std::to_string(mapper) + ".nes");
        if (!write_minimal_ines_rom(rom, mapper)) {
            printf("[%2d/%d] mapper %3u FAIL (cannot write the synthetic ROM)\n",
                   idx, total, mapper);
            ok = false;
            continue;
        }
        printf("[%2d/%d] A1 sweep mapper %3u ... ", idx, total, mapper);
        FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
        if (!gi) {
            printf("FAIL (load returned null)\n");
            ok = false;
            continue;
        }
        if (gi->mappernum != static_cast<int>(mapper)) {
            printf("FAIL (loaded as mapper %d)\n", gi->mappernum);
            ok = false;
            fceu11::CloseGame();
            continue;
        }
        const bool registered = fceu11::find_mapper(mapper) != nullptr;
        const bool forwarder = (iNESCart.Power == &CartInfo_PowerForward);
        if (registered && !forwarder) {
            printf("FAIL (registered, but Power is not the forwarder)\n");
            ok = false;
        } else if (!registered && forwarder) {
            printf("FAIL (unregistered: the forwarder clobbered the board "
                   "Power -- MxxPower never runs)\n");
            ok = false;
        } else {
            printf("OK (%s)\n", registered ? "registry path" : "board power preserved");
        }
        fceu11::CloseGame();
    }
    std::filesystem::remove_all(dir, ec);
    return ok;
}

// A1-S3 - execution anchor: mapper 252 (SAN GUO ZHI PIRATE).  M252Power
// installs CartBR as the read handler over $6000-$7FFF AND $8000-$FFFF, and
// Sync() maps PRG bank 0 at $8000.  Nothing else in the load path installs
// those handlers, so their presence proves the board Power EXECUTED, not
// merely that its pointer survived.
//
// ⚠️ A plain "write $6000 then read it back" lock is TOOTHLESS here: reads
// on unmapped ranges return the CPU data-bus latch (Bus::ANullImpl ->
// g_cpu.db(), src/bus.cpp) and X6502_DMW latches the written value
// (src/x6502.cpp), so the open bus echoes 0x5A back with no handler at all.
// Found by this very lock's mutation leg during implementation.  The check
// therefore compares handler identities against &CartBR and poisons the
// bus latch before reading, so an open-bus echo cannot fake the value.
static bool check_mapper252_power(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::create_directories(dir, ec);  // the sweep may have removed it
    const std::filesystem::path rom = dir / "a1_mapper_252.nes";
    if (!write_minimal_ines_rom(rom, 252)) {
        printf("FAIL: cannot write the mapper 252 ROM\n");
        return false;
    }
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 252 ROM did not load\n");
        return false;
    }
    if (gi->mappernum != 252) {
        printf("FAIL: loaded as mapper %d\n", gi->mappernum);
        fceu11::CloseGame();
        return false;
    }
    X6502_DMW(0x6000, 0x5A);  // poison the open-bus data latch (would echo 0x5A)
    const bool rd8000 = (GetReadHandler(0x8000) == &CartBR);
    const bool rd6000 = (GetReadHandler(0x6000) == &CartBR);
    const uint8_t prg0 = X6502_DMR(0x8000);  // CartBR -> PRG tag byte 0xFC
    printf("      read handler $8000=CartBR:%d $6000=CartBR:%d PRG[0x8000]=0x%02X\n",
           rd8000 ? 1 : 0, rd6000 ? 1 : 0, prg0);
    fceu11::CloseGame();
    bool ok = true;
    if (!rd8000 || !rd6000) {
        printf("FAIL: M252Power's CartBR read handlers are not installed -- "
               "the board Power did not run\n");
        ok = false;
    }
    if (prg0 != 0xFC) {
        printf("FAIL: $8000 does not read the PRG tag byte (0x%02X != 0xFC) -- "
               "either M252Power did not run or the open bus is echoing\n", prg0);
        ok = false;
    }
    return ok;
}

// A1-S4 - controls: registered mappers must keep the forwarder wiring (the
// strategy carts captured the legacy power at construction).  Guards against
// flipping the A1-a branch the wrong way.
static bool check_registered_forwarder_controls() {
    struct CtrlCase {
        const char* path;
        int mapper;
    };
    const CtrlCase cases[] = {
        { "fixtures/mapper_nrom.nes", 0 },
        { "fixtures/mapper_mmc3.nes", 4 },
        { "fixtures/mapper_mmc3_191_nonpow2.nes", 191 },
        { "fixtures/mapper_mapper199.nes", 199 },
    };
    bool ok = true;
    for (const CtrlCase& c : cases) {
        printf("[A1-S4] mapper %3d (%s) ... ", c.mapper, c.path);
        FCEUGI* gi = fceu11::LoadGame(c.path, 1, true);
        if (!gi) {
            printf("FAIL (load returned null)\n");
            ok = false;
            continue;
        }
        if (gi->mappernum != c.mapper) {
            printf("FAIL (loaded as mapper %d)\n", gi->mappernum);
            ok = false;
            fceu11::CloseGame();
            continue;
        }
        if (iNESCart.Power != &CartInfo_PowerForward) {
            printf("FAIL (registered, but Power is not the forwarder)\n");
            ok = false;
        } else {
            printf("OK (forwarder)\n");
        }
        fceu11::CloseGame();
    }
    return ok;
}

// ==================== v2.0.5 BUG-V205-01: CHR count that is not a power of two ====================
//
// A legal iNES 1.0 cart may declare a CHR bank count that is not a power of
// two (6 banks = 48 KiB is the common case).  The file is then exactly
// header-sized, but the emulator maps a ROUNDED (power-of-two) CHR buffer,
// so the tail past the file's own bytes has to be filled with 0xFF -- which
// is what upstream FCEUX 2.6.6 / 2.8.0 produce (memset the rounded buffer to
// 0xFF, then read what the file has).
//
// Before the fix this repo's Rust loader ran its file-boundary check against
// the ROUNDED count, so such a cart was rejected outright with "Unknown ROM
// file format" -- a cart that upstream loads fine.
//
// The lock pins the observable result: the cart loads, the file's own CHR
// bytes are in place, and the padding past them is 0xFF.
static bool check_non_power2_chr_rom(const std::filesystem::path& dir) {
    const uint32_t mapper = 140;      // JALECO JF-11/14 -- the reported number
    const uint32_t prg_banks = 2;
    const uint32_t chr_banks = 6;     // NOT a power of two
    const size_t prg_bytes = prg_banks * 16384;
    const size_t chr_file_bytes = chr_banks * 8192;
    const size_t chr_rounded_bytes = 8 * 8192;   // what gets allocated/mapped

    std::error_code ec;
    std::filesystem::create_directories(dir, ec);
    const std::filesystem::path rom = dir / "bugv20501_chr6.nes";
    {
        std::ofstream f(rom, std::ios::binary | std::ios::trunc);
        if (!f) {
            printf("FAIL: cannot write the CHR-6 ROM\n");
            return false;
        }
        uint8_t hdr[16] = {0};
        hdr[0] = 'N'; hdr[1] = 'E'; hdr[2] = 'S'; hdr[3] = 0x1A;
        hdr[4] = static_cast<uint8_t>(prg_banks);
        hdr[5] = static_cast<uint8_t>(chr_banks);
        hdr[6] = static_cast<uint8_t>((mapper & 0x0F) << 4);
        hdr[7] = static_cast<uint8_t>(mapper & 0xF0);
        f.write(reinterpret_cast<const char*>(hdr), sizeof(hdr));
        std::vector<uint8_t> prg(prg_bytes, 0xEA);
        std::vector<uint8_t> chr(chr_file_bytes, 0xEB);
        f.write(reinterpret_cast<const char*>(prg.data()), prg.size());
        f.write(reinterpret_cast<const char*>(chr.data()), chr.size());
    }

    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: a ROM with 6 CHR banks did not load (BUG-V205-01)\n");
        return false;
    }
    bool ok = true;
    if (gi->mappernum != static_cast<int>(mapper)) {
        printf("FAIL: loaded as mapper %d\n", gi->mappernum);
        ok = false;
    }
    if (!VROM) {
        printf("FAIL: no CHR buffer was allocated\n");
        ok = false;
    } else {
        // The file's own CHR bytes are in place ...
        const uint8_t first = VROM[0];
        const uint8_t last_file_byte = VROM[chr_file_bytes - 1];
        // ... and the tail past them is the 0xFF padding (upstream's rule).
        const uint8_t pad_first = VROM[chr_file_bytes];
        const uint8_t pad_last = VROM[chr_rounded_bytes - 1];
        printf("      CHR[0]=0x%02X CHR[%zu]=0x%02X pad[0]=0x%02X pad[last]=0x%02X\n",
               first, chr_file_bytes - 1, last_file_byte, pad_first, pad_last);
        if (first != 0xEB || last_file_byte != 0xEB) {
            printf("FAIL: the file's CHR bytes are not in place\n");
            ok = false;
        }
        if (pad_first != 0xFF || pad_last != 0xFF) {
            printf("FAIL: the CHR tail is not 0xFF padding\n");
            ok = false;
        }
    }
    fceu11::CloseGame();
    return ok;
}

// ==================== v2.0.5 A2: MMC3-family board power wiring ====================
//
// The loader overwrites info->Power with CartInfo_PowerForward whenever the
// registry produced a Cart object (src/ines.cpp:77-85), so for every
// REGISTERED mapper the board's own `info->Power = MxxPower` assignment never
// ran.  Mmc3BaseCart::on_power() hardcodes GenMMC3Power(), which installs only
// the generic MMC3 command / IRQ handlers -- the per-board write and read
// windows, the EXPREGS seeds and the extra IRQ wiring of the 18 MMC3-family
// boards below were therefore never installed, and those boards could not
// work.
//
// Every lock below asserts an observable that belongs to THAT board and that
// the generic GenMMC3Power() path cannot produce.  Three techniques are used,
// all of them immune to the open-bus echo that made a plain "write-then-read"
// lock toothless (see the A1-S3 comment):
//
//   * EXPREGS / MMC3_cmd / DRegBuf -- the boards' own register files.  The
//     test POISONS them before the load; the board's power function is the
//     only thing that resets them, so a board whose power did not run keeps
//     the poison.
//   * handler identity -- GetWriteHandler() compared against a handler that
//     only the generic MMC3 installs (MMC3_CMDWrite).
//   * a write-回读 lock only where the board's own window is the thing under
//     test (mapper 195's WRAM at $5000); there the data-bus latch is poisoned
//     between the write and the read, so the open bus cannot echo the written
//     byte back.
//
// The 18 numbers are WIRED; 17 of them have a behavioral lock below (mapper
// 245 has none -- see the note in main()).  The v2.0.5 plan §1.4(B) listed
// 17 -- mapper 165 was missing from that list (Mapper165_Init sets
// info->Power = M165Power and mapper 165 IS registered, so M165Power was
// swallowed by the forwarder as well).

// The poison value for the boards' register files.  It has to differ from
// every value a lock expects, otherwise the lock would pass even when the
// board's handler never ran (mapper 254 expects EXPREGS[0] == 0xFF, so the
// poison must not be 0xFF).
static const uint8_t kA2Poison = 0xA5;

enum class A2Probe {
    EXPR_ZERO,        // the board's power must reset EXPREGS[expr_index] to 0
    EXPR_VALUE,       // the board's power must set EXPREGS[expr_index] to expr_expect
    WRITE_EXPR,       // a write must reach the board's handler and land in EXPREGS
    WRITE_REGS,       // a write must reach the board's handler and land in MMC3_cmd/DRegBuf
    HANDLER_NOT_MMC3, // the generic MMC3 command handler must be gone
    WRAM_WINDOW,      // the board's WRAM window at $5000 must be there
};

struct A2Case {
    uint32_t mapper;
    A2Probe  probe;
    uint32_t addr;
    uint8_t  value;
    uint8_t  expr_index;
    uint8_t  expr_expect;
    uint8_t  dreg_index;   // 0xFF = not checked
    uint8_t  dreg_expect;
    const char* note;
};

static const A2Case kA2Cases[] = {
    {  12, A2Probe::EXPR_VALUE,       0,      0,    2, 0x01, 0xFF, 0x00,
      "M12Power: EXPREGS[2]=1 (chinese default) + $4100-$5FFF handlers" },
    {  37, A2Probe::WRITE_EXPR,       0x6000, 0x04, 0, 0x02, 0xFF, 0x00,
      "M37Write at $6000-$7FFF: EXPREGS[0]=(V&6)>>1" },
    {  44, A2Probe::WRITE_EXPR,       0xA001, 0x5A, 0, 0x02, 0xFF, 0x00,
      "M44Write at $A000-$BFFF: EXPREGS[0]=V&7" },
    {  45, A2Probe::EXPR_ZERO,        0,      0,    0, 0x00, 0xFF, 0x00,
      "M45Power: EXPREGS[0..5]=0 (the A4 window lock follows below)" },
    {  47, A2Probe::WRITE_EXPR,       0x6000, 0x01, 0, 0x01, 0xFF, 0x00,
      "M47Write at $6000-$7FFF: EXPREGS[0]=V&1" },
    {  49, A2Probe::EXPR_ZERO,        0,      0,    0, 0x00, 0xFF, 0x00,
      "M49Power -> M49Reset: EXPREGS[0]=0" },
    {  52, A2Probe::WRITE_EXPR,       0x6000, 0x80, 0, 0x80, 0xFF, 0x00,
      "M52Write at $6000-$7FFF: EXPREGS[0]=V" },
    { 114, A2Probe::WRITE_EXPR,       0x5000, 0x5A, 0, 0x5A, 0xFF, 0x00,
      "M114ExWrite at $5000-$7FFF: EXPREGS[0]=V" },
    { 115, A2Probe::WRITE_EXPR,       0x6000, 0x5A, 0, 0x5A, 0xFF, 0x00,
      "M115Write at $4100-$7FFF: EXPREGS[0]=V" },
    { 116, A2Probe::HANDLER_NOT_MMC3, 0x8000, 0,    0, 0x00, 0xFF, 0x00,
      "UNLSL12Power replaces the generic MMC3 command handler at $8000-$FFFF" },
    { 165, A2Probe::EXPR_VALUE,       0,      0,    0, 0xFD, 0xFF, 0x00,
      "M165Power: EXPREGS[0]=0xFD (the CHR-RAM mode seed)" },
    { 195, A2Probe::WRAM_WINDOW,      0x5000, 0x5A, 0, 0x00, 0xFF, 0x00,
      "M195Power: 16 KiB WRAM window at $5000 + CartBR/CartBW handlers" },
    { 205, A2Probe::WRITE_EXPR,       0x6000, 0x03, 0, 0x03, 0xFF, 0x00,
      "M205Write at $6000-$7FFF: EXPREGS[0]=V&3" },
    { 249, A2Probe::WRITE_EXPR,       0x5000, 0x5A, 0, 0x5A, 0xFF, 0x00,
      "M249Write at $5000: EXPREGS[0]=V" },
    { 250, A2Probe::WRITE_REGS,       0x80FF, 0xFF, 0, 0xFF, 0,    0x00,
      "M250Write mangles the written value to A&0xFF" },
    { 254, A2Probe::WRITE_EXPR,       0x8000, 0x00, 0, 0xFF, 0xFF, 0x00,
      "M254Write at $8000-$BFFF: EXPREGS[0]=0xFF" },
    { 406, A2Probe::WRITE_REGS,       0x8002, 0x5A, 0, 0x00, 0,    0x5A,
      "M406CMDWrite permutes the address ($8002 -> $8003)" },
};

static const int NUM_A2_CASES =
    static_cast<int>(sizeof(kA2Cases) / sizeof(kA2Cases[0]));

// The control for the "the generic MMC3 handler must be gone" probe: a plain
// MMC3 (mapper 4) MUST still have MMC3_CMDWrite at $8000, otherwise the
// mapper-116 assertion would pass vacuously.
static bool check_a2_generic_control() {
    FCEUGI* gi = fceu11::LoadGame("fixtures/mapper_mmc3.nes", 1, true);
    if (!gi) {
        printf("FAIL: the plain MMC3 fixture did not load\n");
        return false;
    }
    const bool is_cmd = (GetWriteHandler(0x8000) == &MMC3_CMDWrite);
    printf("      plain MMC3: write handler $8000 == MMC3_CMDWrite: %d\n",
           is_cmd ? 1 : 0);
    fceu11::CloseGame();
    if (!is_cmd) {
        printf("FAIL: the plain MMC3 does not use MMC3_CMDWrite at $8000 -- "
               "the control for the mapper-116 lock is broken\n");
        return false;
    }
    return true;
}

static bool check_a2_case(const A2Case& c, const std::filesystem::path& dir) {
    const std::filesystem::path rom =
        dir / ("a2_mapper_" + std::to_string(c.mapper) + ".nes");
    if (!write_minimal_ines_rom(rom, c.mapper)) {
        printf("FAIL: cannot write the mapper %u ROM\n", c.mapper);
        return false;
    }

    // Poison the boards' register files BEFORE the load.  The board's own
    // power function is the only thing that resets them, so a board whose
    // power did not run keeps the poison.
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA2Poison;

    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper %u ROM did not load\n", c.mapper);
        return false;
    }
    if (gi->mappernum != static_cast<int>(c.mapper)) {
        printf("FAIL: loaded as mapper %d\n", gi->mappernum);
        fceu11::CloseGame();
        return false;
    }

    bool ok = true;
    switch (c.probe) {
    case A2Probe::EXPR_ZERO:
    case A2Probe::EXPR_VALUE: {
        const uint8_t got = EXPREGS[c.expr_index];
        printf("      EXPREGS[%u] = 0x%02X (expected 0x%02X)\n",
               c.expr_index, got, c.expr_expect);
        if (got != c.expr_expect) {
            printf("FAIL: mapper %u: EXPREGS[%u] is 0x%02X, expected 0x%02X -- "
                   "the board's own power function did not run\n",
                   c.mapper, c.expr_index, got, c.expr_expect);
            ok = false;
        }
        break;
    }
    case A2Probe::WRITE_EXPR: {
        X6502_DMW(c.addr, c.value);
        const uint8_t got = EXPREGS[c.expr_index];
        printf("      write $%04X <- 0x%02X => EXPREGS[%u] = 0x%02X "
               "(expected 0x%02X)\n",
               c.addr, c.value, c.expr_index, got, c.expr_expect);
        if (got != c.expr_expect) {
            printf("FAIL: mapper %u: the write to $%04X did not reach the "
                   "board's handler (EXPREGS[%u] is 0x%02X, expected 0x%02X)\n",
                   c.mapper, c.addr, c.expr_index, got, c.expr_expect);
            ok = false;
        }
        break;
    }
    case A2Probe::WRITE_REGS: {
        X6502_DMW(c.addr, c.value);
        // The board's handler must have mangled the address / the value:
        // MMC3_cmd carries the mangled result, DRegBuf the bank register.
        const uint8_t cmd = MMC3_cmd;
        const uint8_t dreg = (c.dreg_index == 0xFF) ? 0x00 : DRegBuf[c.dreg_index];
        printf("      write $%04X <- 0x%02X => MMC3_cmd=0x%02X DRegBuf[%u]=0x%02X\n",
               c.addr, c.value, cmd, c.dreg_index, dreg);
        if (cmd != c.expr_expect) {
            printf("FAIL: mapper %u: MMC3_cmd is 0x%02X, expected 0x%02X -- "
                   "the write did not reach the board's handler\n",
                   c.mapper, cmd, c.expr_expect);
            ok = false;
        }
        if (c.dreg_index != 0xFF && dreg != c.dreg_expect) {
            printf("FAIL: mapper %u: DRegBuf[%u] is 0x%02X, expected 0x%02X\n",
                   c.mapper, c.dreg_index, dreg, c.dreg_expect);
            ok = false;
        }
        break;
    }
    case A2Probe::HANDLER_NOT_MMC3: {
        const bool is_cmd = (GetWriteHandler(c.addr) == &MMC3_CMDWrite);
        printf("      write handler $%04X == MMC3_CMDWrite: %d\n",
               c.addr, is_cmd ? 1 : 0);
        if (is_cmd) {
            printf("FAIL: mapper %u: the generic MMC3 command handler is still "
                   "installed at $%04X -- the board's power function did not "
                   "run\n", c.mapper, c.addr);
            ok = false;
        }
        break;
    }
    case A2Probe::WRAM_WINDOW: {
        const bool rd_ok = (GetReadHandler(0x5000) == &CartBR);
        const bool wr_ok = (GetWriteHandler(0x5000) == &CartBW);
        X6502_DMW(0x5000, 0x5A);   // into the board's WRAM window
        X6502_DMW(0x4020, 0x00);   // poison the open-bus data latch
        const uint8_t rb = X6502_DMR(0x5000);
        printf("      $5000 read handler == CartBR: %d, write handler == "
               "CartBW: %d, WRAM readback = 0x%02X\n",
               rd_ok ? 1 : 0, wr_ok ? 1 : 0, rb);
        if (!rd_ok || !wr_ok) {
            printf("FAIL: mapper %u: the board's CartBR/CartBW handlers are "
                   "not installed at $5000 -- the board's power function did "
                   "not run\n", c.mapper);
            ok = false;
        }
        if (rb != 0x5A) {
            printf("FAIL: mapper %u: $5000 is not the board's WRAM window "
                   "(readback 0x%02X, expected 0x5A)\n", c.mapper, rb);
            ok = false;
        }
        break;
    }
    }
    fceu11::CloseGame();
    return ok;
}

// v2.0.5 A4: mapper 45's write window must start at $6000, not $5000.
//
// M45Write executes `WRAM[A - 0x6000]`, so a write into $5000-$5FFF is a
// NEGATIVE index into WRAM -- an out-of-bounds write.  It was unreachable
// while the board's power function never ran; wiring the power (A2) without
// narrowing the window would have woken the defect.  Upstream 2.8.0 uses the
// same $6000-$7FFF window.
//
// The board's own register file is the observable: a write that reaches
// M45Write lands in EXPREGS (the register path) or in WRAM (the locked
// path).  A write that does NOT reach it leaves EXPREGS alone -- so
// "EXPREGS[0] is unchanged after a write to $5000" is the lock.
static bool check_mapper45_window(const std::filesystem::path& dir) {
    const std::filesystem::path rom = dir / "a2_mapper_45.nes";
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 45 ROM did not load\n");
        return false;
    }
    bool ok = true;
    // The board's power function resets EXPREGS[0..5].
    for (int i = 0; i < 6; ++i) {
        if (EXPREGS[i] != 0) {
            printf("FAIL: mapper 45: EXPREGS[%d] is 0x%02X, expected 0\n",
                   i, EXPREGS[i]);
            ok = false;
        }
    }
    // A write to $5000 must NOT reach the board's handler (that would be the
    // negative-index WRAM write).
    X6502_DMW(0x5000, 0x5A);
    if (EXPREGS[0] != 0) {
        printf("FAIL: mapper 45: a write to $5000 reached the board's handler "
               "(EXPREGS[0] = 0x%02X) -- the write window must start at $6000\n",
               EXPREGS[0]);
        ok = false;
    }
    // ... and a write to $6000 must reach it.
    X6502_DMW(0x6000, 0x5A);
    if (EXPREGS[0] != 0x5A) {
        printf("FAIL: mapper 45: a write to $6000 did not reach the board's "
               "handler (EXPREGS[0] = 0x%02X, expected 0x5A)\n", EXPREGS[0]);
        ok = false;
    }
    // The window identity: $5000 and $6000 have different write handlers.
    const bool split = (GetWriteHandler(0x5000) != GetWriteHandler(0x6000));
    printf("      write handlers $5000 != $6000: %d\n", split ? 1 : 0);
    if (!split) {
        printf("FAIL: mapper 45: $5000 and $6000 share the same write handler "
               "-- the window was not narrowed to $6000-$7FFF\n");
        ok = false;
    }
    fceu11::CloseGame();
    return ok;
}

// ==================== A-1 (2026-11-18 plan §二 A-1): MMC3-family board reset wiring ====================
//
// The loader overwrites info->Reset with CartInfo_ResetForward whenever the
// registry produced a Cart object (src/ines.cpp:82), so for every REGISTERED
// mapper the board's own `info->Reset = MxxReset` assignment never ran.
// Mmc3BaseCart::on_reset() calls MMC3RegReset() only, so the seven boards
// whose reset does more than MMC3RegReset lost their reset semantics:
//
//     12   M12Reset   EXPREGS[0..1]=0; EXPREGS[2]^=1 -- reset toggles the
//                      chinese/english language bit (a real user feature of
//                      these multi-cart boards)
//     37   M37Reset   EXPREGS[0]=0
//     45   M45Reset   EXPREGS[0..4]=0; EXPREGS[5]=++&7 -- the multi-cart
//                      lock counter advances once per reset
//     49   M49Reset   EXPREGS[0]=0
//     52   M52Reset   EXPREGS[0..1]=0
//     114  M114Reset  EXPREGS[0]=0
//     205  M205Reset  EXPREGS[0]=0; EXPREGS[1]^=2 -- the solder-pad bit
//
// While unwired, EXPREGS values written by the game at runtime SURVIVED a
// user reset on 37/49/52/114 (state leak -- the board power clears them only
// at power-on), and the toggle/counter semantics of 12/45/205 never fired.
//
// Every lock below poisons EXPREGS AFTER the load (from the game's point of
// view -- the runtime state a user reset has to clean up) and drives
// fceu11::ResetNES(): the same path a user reset takes (FCEUNPCMD_RESET ->
// GameInterface(GI_RESETM2) -> iNESCart.Reset -> CartInfo_ResetForward ->
// cart_obj->on_reset()).  The base-class path keeps the poison, so every
// lock is red against the unwired behavior.  The observable is the boards'
// own EXPREGS register file -- never a bus readback (the A1-S3 open-bus
// echo trap).
//
// Mutation legs this section guards (discipline 3, verify two-way):
//   M1: any ONE of the seven on_reset overrides reverted to the base-class
//       body (MMC3RegReset() only) -> exactly that board's lock goes red;
//       the other six and the controls stay green (per-board independence).
//   M2: the EXPREGS line inside a board reset function dropped (e.g.
//       `EXPREGS[2] ^= 1;` removed from M12Reset) -> that board's toggle /
//       counter leg goes red on the poisoned index.
//
// Controls pin the seven-number list: plain MMC3 (mapper 4, the Mmc3Cart
// override) and the six boards WITHOUT a board reset function (74/118/119/
// 191/192/194 -- the header-only classes in mmc3_variants_carts.h) must
// KEEP the poison: MMC3RegReset never touches EXPREGS.  An on_reset
// override added by analogy to a board whose Init never sets info->Reset
// turns the control red the moment it clears EXPREGS; an override that only
// calls MMC3RegReset is observationally identical to the base class and
// cannot be caught here (the mapper 245 precedent) -- the header comment in
// mmc3_variants_carts.h forbids it instead.

// The poison value for the reset locks (same role as kA2Poison above).  It
// differs from every post-reset value the locks below assert (0 and the
// toggle results 1 / 2), so a board whose reset did not run keeps the
// poison and the lock reads red.
static const uint8_t kA1Poison = 0xA5;

// The four boards whose reset zeroes EXPREGS[0..zeroed) on top of
// MMC3RegReset.  With the base-class path the poison survives -> red.
struct A1ClearCase {
    uint32_t    mapper;
    int         zeroed;    // EXPREGS[0..zeroed) must read 0 after one reset
    const char* reset_fn;  // names the expected actor in the run log
};
static const A1ClearCase kA1ClearCases[] = {
    {  37, 1, "M37Reset"  },
    {  49, 1, "M49Reset"  },
    {  52, 2, "M52Reset"  },  // zeroes EXPREGS[0] AND EXPREGS[1]
    { 114, 1, "M114Reset" },
};
static const int NUM_A1_CLEAR_CASES =
    static_cast<int>(sizeof(kA1ClearCases) / sizeof(kA1ClearCases[0]));

// Loads the synthetic A-1 ROM for `mapper` and guards the mappernum.  The
// caller poisons EXPREGS, drives fceu11::ResetNES() and asserts; the caller
// owns the CloseGame() on the success path.
static bool load_a1_rom(uint32_t mapper, const std::filesystem::path& dir) {
    const std::filesystem::path rom =
        dir / ("a1_mapper_" + std::to_string(mapper) + ".nes");
    if (!write_minimal_ines_rom(rom, mapper)) {
        printf("FAIL: cannot write the mapper %u ROM\n", mapper);
        return false;
    }
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper %u ROM did not load\n", mapper);
        return false;
    }
    if (gi->mappernum != static_cast<int>(mapper)) {
        printf("FAIL: loaded as mapper %d (expected %u)\n",
               gi->mappernum, mapper);
        fceu11::CloseGame();
        return false;
    }
    return true;
}

// Asserts the full 8-byte EXPREGS footprint against `expect` and prints the
// actual values under `tag`.
static bool a1_expect_expregs(uint32_t mapper, const char* tag,
                              const uint8_t expect[8]) {
    printf("      %s EXPREGS =", tag);
    for (int i = 0; i < 8; ++i) printf(" %02X", EXPREGS[i]);
    printf("\n");
    bool ok = true;
    for (int i = 0; i < 8; ++i) {
        if (EXPREGS[i] != expect[i]) {
            printf("FAIL: mapper %u: %s EXPREGS[%d] is 0x%02X, expected "
                   "0x%02X\n", mapper, tag, i, EXPREGS[i], expect[i]);
            ok = false;
        }
    }
    return ok;
}

// 37/49/52/114: one reset must zero EXPREGS[0..zeroed) and leave the rest
// of the register file alone (the exact upstream footprint).
static bool check_a1_reset_clears(const A1ClearCase& c,
                                  const std::filesystem::path& dir) {
    if (!load_a1_rom(c.mapper, dir)) return false;
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
    fceu11::ResetNES();
    uint8_t expect[8];
    for (int i = 0; i < 8; ++i)
        expect[i] = kA1Poison;
    for (int i = 0; i < c.zeroed; ++i)
        expect[i] = 0x00;
    char tag[64];
    snprintf(tag, sizeof(tag), "after 1 reset (%s):", c.reset_fn);
    const bool ok = a1_expect_expregs(c.mapper, tag, expect);
    fceu11::CloseGame();
    return ok;
}

// Mapper 12: the reset toggles the language bit EXPREGS[2] (^= 1) and
// clears EXPREGS[0..1].  Two resets are needed to observe the toggle in
// both directions (one reset cannot tell ^=1 from =1).  EXPREGS[2] is
// poisoned with 0 BEFORE the resets -- poisoning it with kA1Poison and
// asserting the two-reset result would be vacuous (0xA5 ^ 1 ^ 1 == 0xA5:
// the poison would come back on its own).
static bool check_a1_mapper12_reset(const std::filesystem::path& dir) {
    if (!load_a1_rom(12, dir)) return false;
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
    EXPREGS[2] = 0x00;

    bool ok = true;
    fceu11::ResetNES();
    const uint8_t after1[8] = { 0x00, 0x00, 0x01,
                                kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison };
    ok = a1_expect_expregs(12, "after reset 1 (M12Reset):", after1) && ok;
    fceu11::ResetNES();
    const uint8_t after2[8] = { 0x00, 0x00, 0x00,
                                kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison };
    ok = a1_expect_expregs(12, "after reset 2 (toggle back):", after2) && ok;
    fceu11::CloseGame();
    return ok;
}

// Mapper 45: the reset zeroes EXPREGS[0..4] and advances the multi-cart
// lock counter EXPREGS[5] once per reset (++ then &7).  The counter is
// poisoned with 0xFF so the wrap leg (0xFF + 1 -> 0) is on the first reset
// and the increment leg (0 -> 1) on the second.
static bool check_a1_mapper45_reset(const std::filesystem::path& dir) {
    if (!load_a1_rom(45, dir)) return false;
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
    EXPREGS[5] = 0xFF;

    bool ok = true;
    fceu11::ResetNES();
    const uint8_t after1[8] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                kA1Poison, kA1Poison };
    ok = a1_expect_expregs(45, "after reset 1 (M45Reset, 0xFF wraps to 0):", after1) && ok;
    fceu11::ResetNES();
    const uint8_t after2[8] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
                                kA1Poison, kA1Poison };
    ok = a1_expect_expregs(45, "after reset 2 (counter advances to 1):", after2) && ok;
    fceu11::CloseGame();
    return ok;
}

// Mapper 205: the reset zeroes EXPREGS[0] and flips the solder-pad bit
// EXPREGS[1] bit 1 (^= 2).  EXPREGS[1] is poisoned with 0 before the resets
// (same vacuity argument as mapper 12's language bit) so the two-reset
// sequence is 0 -> 2 -> 0.
static bool check_a1_mapper205_reset(const std::filesystem::path& dir) {
    if (!load_a1_rom(205, dir)) return false;
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
    EXPREGS[1] = 0x00;

    bool ok = true;
    fceu11::ResetNES();
    const uint8_t after1[8] = { 0x00, 0x02,
                                kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison };
    ok = a1_expect_expregs(205, "after reset 1 (M205Reset):", after1) && ok;
    fceu11::ResetNES();
    const uint8_t after2[8] = { 0x00, 0x00,
                                kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison, kA1Poison };
    ok = a1_expect_expregs(205, "after reset 2 (toggle back):", after2) && ok;
    fceu11::CloseGame();
    return ok;
}

// The control legs pin the seven-number list.  Plain MMC3 (mapper 4 -- the
// Mmc3Cart override calls MMC3RegReset only) and the six boards WITHOUT a
// board reset function (74/118/119/191/192/194) must KEEP the poison:
// MMC3RegReset never touches EXPREGS, and neither does the base-class
// on_reset those six use.
static bool check_a1_no_reset_controls(const std::filesystem::path& dir) {
    bool ok = true;
    FCEUGI* gi = fceu11::LoadGame("fixtures/mapper_mmc3.nes", 1, true);
    if (!gi) {
        printf("FAIL: the plain MMC3 fixture did not load\n");
        return false;
    }
    for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
    fceu11::ResetNES();
    const uint8_t keep[8] = { kA1Poison, kA1Poison, kA1Poison, kA1Poison,
                              kA1Poison, kA1Poison, kA1Poison, kA1Poison };
    ok = a1_expect_expregs(4, "after 1 reset (poison must survive):", keep) && ok;
    fceu11::CloseGame();

    // The six boards whose Init never sets info->Reset: they run the base
    // class's on_reset, and the poison must survive it too.
    static const uint32_t kNoResetMappers[] = { 74, 118, 119, 191, 192, 194 };
    for (uint32_t mapper : kNoResetMappers) {
        if (!load_a1_rom(mapper, dir)) return false;
        for (int i = 0; i < 8; ++i) EXPREGS[i] = kA1Poison;
        fceu11::ResetNES();
        ok = a1_expect_expregs(mapper,
                               "after 1 reset (no board reset, poison must survive):",
                               keep) && ok;
        fceu11::CloseGame();
    }
    return ok;
}

// ==================== v2.0.5 A8: mapper 471 (Impact Soft IM1) port ====================
//
// Upstream 2.8.0 src/boards/471.cpp, ported verbatim (v2.0.5 plan §三 A8):
// NES 2.0 mapper 471 is iNES mapper 201 plus a scanline IRQ (Haratyler /
// Haraforce).  The board's whole state is one uint32 latch, and the write
// handler latches the WRITE ADDRESS, not the data bus value (`latch = A`).
//
// Three legs:
//   * A8-S1 dual registration: 471 must be in bmap[] (loader dispatch) AND
//     in the MapperEntry registry.  The A1-S1 bmap classification above
//     enforces exactly this pairing for every bmap number, so a bmap-only
//     add would turn the S1 sweep red; this check pins 471's side of it.
//   * A8-S2 handler identity: the board's Power installs CartBR at $8000
//     and its own write handler (not the generic CartBW), and the load goes
//     through the strategy cart (the A1-a forwarder wiring).
//   * A8-S3 the "address is data" anchor: the latch is read out of a
//     savestate dump through its "LATC" SFORMAT tag, so the lock exercises
//     the real AddExState("LATC") path.  SubWrite (src/state.cpp:147)
//     serialises each field as [4-byte tag][u32 LE size][raw data], and
//     FCEUSS_SaveMS level 0 stores chunk payloads uncompressed
//     (fceux11-core/src/state_file.rs: compress_payload level 0 -> raw), so
//     the tag's bytes appear verbatim in the buffer.  A write-then-read-back
//     at $8000-$FFFF would be TOOTHLESS here (the A1-S3 comment above): the
//     latch has no CPU-visible read port, PRG reads come from mapped ROM /
//     open bus and can never echo the latch.
//
// Mutation legs this section guards (each is verified two-way red):
//   M1a: drop the bmap[] line      -> A8-S1 red + the 471 load fails
//   M1b: drop kMapper471Register   -> A8-S1 red + the A1-S1 classification
//       sweep goes red (471 in bmap, unregistered, not in the 75-list)
//   M2 : `latch = A` -> `latch = V` -> every A8-S3 value leg red
//   M3 : drop `latch = 0` in Reset -> the A8-S3 reset leg red
//
// The scanline IRQ hook (GameHBIRQHook = HBHook) is wired but NOT locked
// here: firing it needs ScreenON/SpriteON rendering with PPU[0]&0x38 !=
// 0x18, for which there is no cheap headless precondition.  Register an IRQ
// lock when a real 471 ROM (Haratyler / Haraforce) is available.

#include "emufile.h"     // EMUFILE_MEMORY (A8-S3 savestate dump)
#include "state.h"       // FCEUSS_SaveMS
#include <cstddef>       // std::byte

// Reads the 471 latch out of a fresh savestate dump via its "LATC" tag.
// Returns false when the save failed or the tag is missing / ambiguous --
// never silently "skips", so a broken AddExState cannot pass vacuously.
static bool read_mapper471_latch_via_savestate(uint32_t* out) {
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    if (!FCEUSS_SaveMS(&f, 0)) return false;
    const uint8_t* p = reinterpret_cast<const uint8_t*>(buf.data());
    const size_t n = buf.size();
    static const uint8_t kTag[4] = { 'L', 'A', 'T', 'C' };
    int hits = 0;
    bool ok = false;
    for (size_t i = 0; i + 12 <= n; ++i) {
        if (memcmp(p + i, kTag, 4) != 0) continue;
        const uint32_t size = static_cast<uint32_t>(p[i + 4]) |
                              (static_cast<uint32_t>(p[i + 5]) << 8) |
                              (static_cast<uint32_t>(p[i + 6]) << 16) |
                              (static_cast<uint32_t>(p[i + 7]) << 24);
        if (size != 4) continue;      // only the 4-byte latch entry matches
        if (++hits > 1) return false; // the tag must identify exactly one field
        *out = static_cast<uint32_t>(p[i + 8]) |
               (static_cast<uint32_t>(p[i + 9]) << 8) |
               (static_cast<uint32_t>(p[i + 10]) << 16) |
               (static_cast<uint32_t>(p[i + 11]) << 24);
        ok = true;
    }
    return ok;
}

// A8-S1: 471 is in bmap[] with a non-null init AND registered in the
// MapperEntry registry (as "Impact Soft IM1", matching upstream's bmap name).
static bool check_mapper471_registered() {
    bool in_bmap = false;
    for (const BMAPPINGLocal* e = bmap; e->init; ++e) {
        if (e->number == 471) { in_bmap = true; break; }
    }
    if (!in_bmap) {
        printf("FAIL: mapper 471 is not in bmap[] -- the loader cannot "
               "dispatch it\n");
        return false;
    }
    const fceu11::MapperEntry* entry = fceu11::find_mapper(471);
    if (!entry) {
        printf("FAIL: mapper 471 has no registry entry -- the A1-S1 bmap "
               "classification requires the pairing\n");
        return false;
    }
    if (entry->mapper_number != 471 || !entry->name ||
        strcmp(entry->name, "Impact Soft IM1") != 0 || !entry->factory) {
        printf("FAIL: mapper 471 registry entry is { %u, \"%s\", factory:%d }\n",
               entry->mapper_number, entry->name ? entry->name : "",
               entry->factory ? 1 : 0);
        return false;
    }
    printf("      bmap + registry both carry 471 as \"%s\"\n", entry->name);
    return true;
}

// A8-S2 + A8-S3: one real load, then the board behaviour.
static bool check_mapper471_latch(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::create_directories(dir, ec);  // the sweep may have removed it
    const std::filesystem::path rom = dir / "a8_mapper_471.nes";
    if (!write_minimal_ines_rom(rom, 471)) {
        printf("FAIL: cannot write the mapper 471 ROM\n");
        return false;
    }
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 471 ROM did not load\n");
        return false;
    }
    bool ok = true;
    if (gi->mappernum != 471) {
        printf("FAIL: loaded as mapper %d (expected 471)\n", gi->mappernum);
        ok = false;
    }
    // Registered mappers load through the strategy cart: the A1-a wiring
    // installs the forwarders (the contract the A1-S4 controls pin).
    const bool forwarder = (iNESCart.Power == &CartInfo_PowerForward);
    printf("      iNESCart.Power is the forwarder: %d\n", forwarder ? 1 : 0);
    if (!forwarder) {
        printf("FAIL: mapper 471 is registered but the Power forwarder was "
               "not installed\n");
        ok = false;
    }
    // Handler identity: Power maps $8000-$FFFF reads onto CartBR and installs
    // the board's own write handler (the generic CartBW is the WRAM handler,
    // which no 471 path would install over PRG).
    const bool rd = (GetReadHandler(0x8000) == &CartBR);
    const bool wr = (GetWriteHandler(0x8000) != &CartBW);
    printf("      read handler $8000 == CartBR: %d, write handler != CartBW: %d\n",
           rd ? 1 : 0, wr ? 1 : 0);
    if (!rd) {
        printf("FAIL: $8000 is not CartBR -- the board's Power did not run\n");
        ok = false;
    }
    if (!wr) {
        printf("FAIL: $8000 still has the generic CartBW write handler\n");
        ok = false;
    }

    uint32_t latch_value = 0;
    // Power -> Reset: the latch starts at 0.
    latch_value = 0xFFFFFFFFu;
    if (!read_mapper471_latch_via_savestate(&latch_value) || latch_value != 0) {
        printf("FAIL: after power the LATC latch reads 0x%08X (expected 0)\n",
               latch_value);
        ok = false;
    }
    // The "address is data" anchor: each write lands the ADDRESS in the
    // latch.  Non-trivial addresses only -- every expected value differs from
    // both 0 and the written data byte, so the M2 mutation (`latch = V`)
    // turns every leg red.
    struct LatcStep { uint32_t addr; uint8_t v; };
    const LatcStep steps[] = {
        { 0x8003, 0x00 }, { 0x8041, 0x5A }, { 0xFFFF, 0x12 },
    };
    for (const LatcStep& s : steps) {
        X6502_DMW(s.addr, s.v);
        const uint32_t expect = s.addr;
        latch_value = 0xFFFFFFFFu;
        if (!read_mapper471_latch_via_savestate(&latch_value) ||
            latch_value != expect) {
            printf("FAIL: write $%04X <- 0x%02X: LATC latch reads 0x%08X "
                   "(expected the ADDRESS 0x%08X -- `latch = A` did not run)\n",
                   s.addr, s.v, latch_value, expect);
            ok = false;
        } else {
            printf("      write $%04X <- 0x%02X: LATC = 0x%08X\n",
                   s.addr, s.v, latch_value);
        }
    }
    // Reset semantics: the board's Reset zeroes the latch and re-Syncs.
    fceu11::ResetNES();
    latch_value = 0xFFFFFFFFu;
    if (!read_mapper471_latch_via_savestate(&latch_value) || latch_value != 0) {
        printf("FAIL: after Reset the LATC latch reads 0x%08X (expected 0)\n",
               latch_value);
        ok = false;
    } else {
        printf("      reset: LATC = 0x%08X\n", latch_value);
    }
    fceu11::CloseGame();
    return ok;
}

// ==================== v2.0.5 A5: mapper 4 NES 2.0 PRG-RAM ====================
//
// (r14 rewrite -- the A5 branch was REVERTED; see the plan doc, A5 / 9.24.)
//
// Mapper4_Init sizes the MMC3 WRAM.  A5 (r13) adopted upstream 2.8.0's branch
// (src/boards/mmc3.cpp:363-375):
//
//     if (info->ines2) ws = (info->wram_size + info->battery_wram_size) / 1024;
//
// which forwards the NES 2.0 header's declared PRG-RAM size straight into the
// WRAM size.  That is wrong for MMC3: the chip carries exactly ONE 8 KiB PRG
// RAM at $6000-$7FFF (NESdev "MMC3": "PRG RAM: Up to 8 KB"), so a declared
// size can only mean "present" (8 KiB) or "absent" (0) -- never 1 KiB,
// 32 KiB, ...  Forwarding it raw therefore dropped the WRAM of every mapper 4
// cartridge whose header declares 0 (the chip is optional, so dump tools
// leave it at 0): all six mapper 4 + NES 2.0 fixtures in this repo, and
// tepples' mmc3bigchrram -- which uses $6000 as its test-result channel.
// F11QA kgmqa-075 went PASS -> FAIL and the R4 gate flagged a pass_to_fail.
//
// The mapper 4 WRAM is therefore back to the hardware truth: 8 KiB,
// regardless of what the header declares.  The locks below pin that.
//
// The observable is WRAMSIZE (src/boards/mmc3.cpp:73, `uint32 WRAMSIZE`, set
// by GenMMC3_Init as `wram << 10`), read straight after each load:
//   * leg 1  NES 2.0 mapper 4, byte-10 LOW nibble shift = 0 (the real-world
//            case: the optional chip is simply not declared) -> WRAMSIZE ==
//            8192.  This is the leg the old lock was missing, and it is the
//            one that catches the regression: with the upstream formula the
//            declared size is 0 -> ws = 0 -> no WRAM at all -> RED.
//   * leg 2  control: the iNES 1.0 fixture mapper_mmc3.nes must keep the
//            hardcoded default, WRAMSIZE == 8192 -- pins the reverse guard
//            (the non-NES 2.0 path is untouched).
//   * leg 3  NES 2.0 mapper 4, byte-10 LOW nibble shift = 4 (a size MMC3
//            cannot have) -> WRAMSIZE == 8192 as well.  This leg exists purely
//            to prove the declared size is NOT forwarded: the upstream
//            formula would yield 64<<4 = 1024 bytes -> 1024 -> RED.
//   * leg 4  NES 2.0 mapper 4, byte-10 HIGH nibble (battery) shift = 4 ->
//            WRAMSIZE == 8192, for the same reason against the battery half
//            of the upstream formula.
//
// Mutation legs this section guards:
//   M1: restore the reverted A5 branch
//       (`ws = (info->wram_size + info->battery_wram_size) / 1024`)
//                                                          -> legs 1 + 3 + 4 red
//   M2: `int ws = 8;` -> `int ws = 1;`                   -> all legs red

extern uint32 WRAMSIZE;  // src/boards/mmc3.cpp:73 (GenMMC3_Init: WRAMSIZE = wram << 10)

// NES 2.0 variant of write_minimal_ines_rom: the same 1 x 16 KiB PRG recipe,
// with the byte-10 PRG-RAM shift nibbles set.  Hardcoded to mapper 4; the
// NES 2.0 marker (byte 7 bits 2-3 = 0b10) is always written, unlike
// write_minimal_ines_rom which only writes NES 2.0 headers for mapper >= 256.
static bool write_nes2_mapper4_rom(const std::filesystem::path& path,
                                   uint8_t wram_shift,
                                   uint8_t battery_wram_shift) {
    uint8_t hdr[16] = {0};
    hdr[0] = 'N';
    hdr[1] = 'E';
    hdr[2] = 'S';
    hdr[3] = 0x1A;
    hdr[4] = 1;     // 1 x 16 KiB PRG
    hdr[5] = 0;     // 0 x 8 KiB CHR banks -> CHR-RAM
    hdr[6] = 0x40;  // mapper 4 low nibble (bits 4-7)
    hdr[7] = 0x08;  // NES 2.0 marker: bits 2-3 = 0b10 ((rom[7] & 0x0C) == 0x08);
                    // mapper mid nibble (bits 4-7) = 0 for mapper 4
    hdr[8] = 0x00;  // mapper MSB nibble (bits 0-3) = 0, submapper (bits 4-7) = 0
    // byte 10 PRG-RAM shift nibbles: size = 64 << shift bytes, 0 = none.
    // LOW nibble -> wram_size, HIGH nibble -> battery_wram_size (ines.rs:564-565).
    hdr[10] = static_cast<uint8_t>(((battery_wram_shift & 0x0F) << 4) |
                                   (wram_shift & 0x0F));
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    if (!f) return false;
    f.write(reinterpret_cast<const char*>(hdr), sizeof(hdr));
    // exactly 16384 PRG bytes: 2-byte mapper tag prefix, rest NOP sled
    uint8_t prg[16384];
    prg[0] = 4;
    prg[1] = 0;
    memset(prg + 2, 0xEA, sizeof(prg) - 2);
    f.write(reinterpret_cast<const char*>(prg), sizeof(prg));
    return static_cast<bool>(f);
}

static bool check_a5_mapper4_nes2_wram(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::create_directories(dir, ec);  // the sweep may have removed it
    bool ok = true;

    // Leg 1: NES 2.0 with the PRG-RAM shift nibble = 0 -- the real-world
    // case (the optional 8 KiB chip is simply not declared).  The WRAM must
    // still be there: 8192.
    const std::filesystem::path rom = dir / "a5_mapper4_nes2_zero.nes";
    if (!write_nes2_mapper4_rom(rom, 0, 0)) {
        printf("FAIL: cannot write the NES 2.0 mapper 4 ROM\n");
        return false;
    }
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the NES 2.0 mapper 4 ROM did not load\n");
        return false;
    }
    if (gi->mappernum != 4) {
        printf("FAIL: the NES 2.0 ROM loaded as mapper %d (expected 4)\n",
               gi->mappernum);
        fceu11::CloseGame();
        return false;
    }
    printf("      NES 2.0 shift=0 (chip not declared): WRAMSIZE = %u "
           "(expected 8192)\n", static_cast<unsigned>(WRAMSIZE));
    if (WRAMSIZE != 8192) {
        printf("FAIL: NES 2.0 mapper 4 with PRG-RAM shift 0 gave WRAMSIZE %u, "
               "expected 8192 -- mapper 4 always has its 8 KiB PRG RAM at "
               "$6000-$7FFF; the declared size must not be forwarded\n",
               static_cast<unsigned>(WRAMSIZE));
        ok = false;
    }
    fceu11::CloseGame();

    // Leg 2 (control): iNES 1.0 must keep the hardcoded default ws = 8.
    gi = fceu11::LoadGame("fixtures/mapper_mmc3.nes", 1, true);
    if (!gi) {
        printf("FAIL: the iNES 1.0 mapper 4 fixture did not load\n");
        return false;
    }
    printf("      iNES 1.0 control: WRAMSIZE = %u (expected 8192)\n",
           static_cast<unsigned>(WRAMSIZE));
    if (WRAMSIZE != 8192) {
        printf("FAIL: iNES 1.0 mapper 4 gave WRAMSIZE %u, expected 8192 -- "
               "the hardcoded default must stay for non-NES 2.0 ROMs\n",
               static_cast<unsigned>(WRAMSIZE));
        ok = false;
    }
    fceu11::CloseGame();

    // Leg 3: NES 2.0 with the size nibble = 4 -- a size MMC3 cannot have.
    // The WRAM must still be 8192: this leg proves the declared size is NOT
    // forwarded (the upstream formula would give 64<<4 = 1024 bytes).
    const std::filesystem::path rom2 = dir / "a5_mapper4_nes2_size.nes";
    if (!write_nes2_mapper4_rom(rom2, 4, 0)) {
        printf("FAIL: cannot write the NES 2.0 mapper 4 size ROM\n");
        return false;
    }
    gi = fceu11::LoadGame(rom2.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the NES 2.0 mapper 4 size ROM did not load\n");
        return false;
    }
    printf("      NES 2.0 shift=4 (64<<4 = 1024 bytes): WRAMSIZE = %u "
           "(expected 8192)\n", static_cast<unsigned>(WRAMSIZE));
    if (WRAMSIZE != 8192) {
        printf("FAIL: NES 2.0 mapper 4 with PRG-RAM shift 4 gave WRAMSIZE "
               "%u, expected 8192 -- the declared size must not be forwarded "
               "to the MMC3 WRAM\n", static_cast<unsigned>(WRAMSIZE));
        ok = false;
    }
    fceu11::CloseGame();

    // Leg 4: the battery half of the same thing (byte-10 HIGH nibble).
    const std::filesystem::path rom3 = dir / "a5_mapper4_nes2_battery.nes";
    if (!write_nes2_mapper4_rom(rom3, 0, 4)) {
        printf("FAIL: cannot write the NES 2.0 mapper 4 battery ROM\n");
        return false;
    }
    gi = fceu11::LoadGame(rom3.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the NES 2.0 mapper 4 battery ROM did not load\n");
        return false;
    }
    printf("      NES 2.0 battery shift=4 (64<<4 = 1024 bytes): WRAMSIZE = %u "
           "(expected 8192)\n", static_cast<unsigned>(WRAMSIZE));
    if (WRAMSIZE != 8192) {
        printf("FAIL: NES 2.0 mapper 4 with battery PRG-RAM shift 4 gave "
               "WRAMSIZE %u, expected 8192 -- the declared size must not be "
               "forwarded to the MMC3 WRAM\n", static_cast<unsigned>(WRAMSIZE));
        ok = false;
    }
    fceu11::CloseGame();

    return ok;
}

// ==================== v2.0.5 A7-3: mapper 72 write window ====================
//
// M72Power installed the board's write handler over $6000-$FFFF; upstream
// 2.8.0 (src/boards/72.cpp:59) uses $8000-$FFFF.  M72Write latches
// preg = V & 0xF / creg = V & 0xF from EVERY byte written into its window,
// and mapper 72 has no WRAM at $6000-$7FFF, so the $6000 window re-banked
// PRG/CHR on ordinary writes into that range.
//
// M72Write is static in src/boards/72.cpp, so the identity is asserted by
// handler-pointer structure (the A2 HANDLER_NOT_MMC3 pattern), NOT by a
// write-then-read-back (the A1-S3 comment: that lock is toothless -- the
// open-bus echo answers instead of the board):
//   * leg 1  $6000's write handler equals $5000's (both fall back to the
//            same unmapped default) -- with the $6000 window restored,
//            $6000 carries M72Write while $5000 stays default -> red;
//   * leg 2  $6000's write handler differs from $8000's (the board's own
//            handler over PRG) -- with the $6000 window restored, one
//            handler spans both halves -> red.
static bool check_mapper72_window(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::create_directories(dir, ec);
    const std::filesystem::path rom = dir / "a7_mapper_72.nes";
    if (!write_minimal_ines_rom(rom, 72)) {
        printf("FAIL: cannot write the mapper 72 ROM\n");
        return false;
    }
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 72 ROM did not load\n");
        return false;
    }
    bool ok = true;
    if (gi->mappernum != 72) {
        printf("FAIL: loaded as mapper %d (expected 72)\n", gi->mappernum);
        ok = false;
    }
    // The board's power ran: PRG reads go through CartBR (M72Power).
    const bool rd = (GetReadHandler(0x8000) == &CartBR);
    const bool unmapped_lo = (GetWriteHandler(0x6000) == GetWriteHandler(0x5000));
    const bool split = (GetWriteHandler(0x6000) != GetWriteHandler(0x8000));
    const bool not_cartbw = (GetWriteHandler(0x8000) != &CartBW);
    printf("      read handler $8000 == CartBR: %d, write handler $6000 == "
           "$5000 (unmapped): %d, write handlers $6000 != $8000: %d, write "
           "handler $8000 != CartBW: %d\n",
           rd ? 1 : 0, unmapped_lo ? 1 : 0, split ? 1 : 0,
           not_cartbw ? 1 : 0);
    if (!rd) {
        printf("FAIL: mapper 72: $8000 is not CartBR -- the board's power "
               "function did not run\n");
        ok = false;
    }
    if (!unmapped_lo) {
        printf("FAIL: mapper 72: $6000 carries a mapped write handler that "
               "$5000 does not have -- the write window still starts at "
               "$6000 (must be $8000-$FFFF, upstream 2.8.0 72.cpp:59)\n");
        ok = false;
    }
    if (!split) {
        printf("FAIL: mapper 72: $6000 and $8000 share the same write "
               "handler -- the write window was not narrowed to $8000\n");
        ok = false;
    }
    if (!not_cartbw) {
        printf("FAIL: mapper 72: the generic CartBW is installed at $8000 -- "
               "the board's own write handler was lost\n");
        ok = false;
    }
    fceu11::CloseGame();
    return ok;
}

// ==================== v2.0.5 A7-2: bandai x24c02 device-address check ====================
//
// x24c02_write() (src/boards/bandai.cpp) accepted EVERY 7-bit address shifted
// in after START and latched the READ/WRITE state from the 8th bit; upstream
// 2.8.0 (bandai.cpp:181-187) rejects an address whose device bits are not
// 1010xxx, i.e. (addr & 0x78) != 0x50: SDA is released (out = 1, no ACK) and
// the state machine drops back to STANDBY.
//
// The lock drives the REAL protocol through the board's write handler --
// mapper 16 ("BANDAI 24C02", the only bandai board whose BandaiWrite routes
// case 0x0D to x24c02_write via x24c02 = 1 AND registers ONLY the x24c02
// state block: mapper 157 would add the x24c01 set with the SAME 4-char
// tags, making a tag scan ambiguous) -- then reads the device's private
// state out of a savestate dump through its SFORMAT tags "ADDR"/"STAT"/
// "OUT" (x24c02StateRegs, bandai.cpp:82-93; 1 byte each).  The symbols are
// static, so the savestate is the only non-invasive window; the A8-S3
// comment documents the serialisation ([4-byte tag][u32 LE size][raw data],
// level 0 = uncompressed) and ResetExState (state.cpp:882, called per load
// from ines_load.cpp:265) guarantees no earlier board's tags are still
// registered.  No core SFORMAT uses those tags (354.cpp's "ADDR" is 2 bytes
// and only registered while mapper 354 is loaded).
//
// Protocol encoding (x24c02_write): scl = (V >> 5) & 1, sda = (V >> 6) & 1.
// START = SDA falling while SCL stays high; address bits shift in on SCL
// RISING edges, MSB first.  Two loads, one transaction each:
//   * leg 1  wrong device address 0x70 (0b1110000, R/W = 1): with the check
//            -> STANDBY (0), out = 1, addr = 0x70; with the check removed
//            -> READ state, out stays 0.
//   * leg 2  polarity control: correct device address 0x50 (0b1010000,
//            R/W = 1) must still proceed to READ (3); one extra edge
//            consumes the READ ACK, which ASSIGNS out = 0 (robust against
//            a stale out from earlier process state).
//
// Mutation leg: delete the `if ((x24c02_addr & 0x78) != 0x50)` branch ->
// leg 1 goes red (state READ != STANDBY, out 0 != 1); leg 2 stays green.

// Reads ONE 1-byte SFORMAT field out of a fresh savestate dump via its
// 4-byte tag (the A8-S3 paradigm with a 1-byte payload).  Returns false when
// the save failed or the tag is missing / ambiguous -- never silently skips.
static bool read_x24c02_field_via_savestate(const char* tag4, uint8_t* out) {
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    if (!FCEUSS_SaveMS(&f, 0)) return false;
    const uint8_t* p = reinterpret_cast<const uint8_t*>(buf.data());
    const size_t n = buf.size();
    int hits = 0;
    bool ok = false;
    for (size_t i = 0; i + 9 <= n; ++i) {
        if (memcmp(p + i, tag4, 4) != 0) continue;
        const uint32_t size = static_cast<uint32_t>(p[i + 4]) |
                              (static_cast<uint32_t>(p[i + 5]) << 8) |
                              (static_cast<uint32_t>(p[i + 6]) << 16) |
                              (static_cast<uint32_t>(p[i + 7]) << 24);
        if (size != 1) continue;      // only the 1-byte field entries match
        if (++hits > 1) return false; // the tag must identify exactly one field
        *out = p[i + 8];
        ok = true;
    }
    return ok;
}

// Drives one x24c02 I2C transaction through the board's write handler:
// START, then eight SCL rising edges carrying the 7-bit device address
// MSB-first plus the R/W bit, each followed by an SCL falling edge.  Write
// address $600D: BandaiWrite masks A & 0x0F -> case 0x0D -> x24c02_write.
// With with_ack, one more rising edge after the 8th bit consumes the READ
// ACK (which assigns out = 0).
static void x24c02_transaction(uint8_t addr7, uint8_t rw, bool with_ack) {
    // preamble: SDA high while SCL low, then SCL high -- SDA can now fall
    // while SCL is high, which is the START condition
    X6502_DMW(0x600D, 0x40);  // sda=1, scl=0
    X6502_DMW(0x600D, 0x60);  // sda=1, scl=1 (rising; STANDBY ignores it)
    X6502_DMW(0x600D, 0x20);  // sda 1->0 while scl high -> START
    X6502_DMW(0x600D, 0x00);  // scl falling, arm the first bit edge
    for (int i = 7; i >= 0; --i) {
        const uint8_t bit =
            static_cast<uint8_t>((i > 0) ? ((addr7 >> (i - 1)) & 1) : (rw & 1));
        X6502_DMW(0x600D, bit ? 0x60 : 0x20);  // SCL rising with SDA = bit
        X6502_DMW(0x600D, 0x00);               // SCL falling
    }
    if (with_ack) {
        // the loop's last falling write already has scl low; the next rise
        // is the ACK position of the READ state (bitcount == 8 -> out = 0)
        X6502_DMW(0x600D, 0x00);  // no-op bus state
        X6502_DMW(0x600D, 0x60);  // scl rising -> READ ACK: out = 0
        X6502_DMW(0x600D, 0x00);  // scl falling
    }
}

static bool check_bandai_x24c02_address(const std::filesystem::path& dir) {
    std::error_code ec;
    std::filesystem::create_directories(dir, ec);
    const std::filesystem::path rom = dir / "a7_mapper_16.nes";
    if (!write_minimal_ines_rom(rom, 16)) {
        printf("FAIL: cannot write the mapper 16 ROM\n");
        return false;
    }
    bool ok = true;

    // Leg 1: a transaction addressed to 0x70 (device bits 1110, not 1010)
    // must be rejected -- STANDBY + SDA released (out = 1) -- while the
    // shifted-in bits still prove the ADDRESS state machine ran.
    FCEUGI* gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 16 ROM did not load\n");
        return false;
    }
    if (gi->mappernum != 16) {
        printf("FAIL: loaded as mapper %d (expected 16)\n", gi->mappernum);
        fceu11::CloseGame();
        return false;
    }
    x24c02_transaction(0x70, 1, false);
    uint8_t state = 0xFF, out = 0xFF, addr = 0xFF;
    if (!read_x24c02_field_via_savestate("STAT", &state) ||
        !read_x24c02_field_via_savestate("OUT", &out) ||
        !read_x24c02_field_via_savestate("ADDR", &addr)) {
        printf("FAIL: the x24c02 STAT/OUT/ADDR savestate fields are missing "
               "or ambiguous -- AddExState(x24c02StateRegs) did not run\n");
        fceu11::CloseGame();
        return false;
    }
    printf("      wrong address 0x70: STAT=%u (X24C0X_STANDBY=0) OUT=%u "
           "(NAK=1) ADDR=0x%02X\n", state, out, addr);
    if (addr != 0x70) {
        printf("FAIL: x24c02 ADDR is 0x%02X, expected 0x70 -- the 7 address "
               "bits did not reach the device (write handler or protocol "
               "sequence broken)\n", addr);
        ok = false;
    }
    if (state != 0) {  // 0 = X24C0X_STANDBY (bandai.cpp:53)
        printf("FAIL: x24c02 state is %u after a wrong device address, "
               "expected STANDBY (0) -- the device-address check did not "
               "reject the transaction\n", state);
        ok = false;
    }
    if (out != 1) {
        printf("FAIL: x24c02 out is %u after a wrong device address, "
               "expected 1 (SDA released, no ACK)\n", out);
        ok = false;
    }
    fceu11::CloseGame();

    // Leg 2 (polarity control): the correct device address 0x50 must still
    // proceed to READ -- a check that rejects everything would fail here.
    // A fresh load starts the device in STANDBY with clear registers.
    gi = fceu11::LoadGame(rom.string().c_str(), 1, true);
    if (!gi) {
        printf("FAIL: the mapper 16 ROM did not load (second pass)\n");
        return false;
    }
    x24c02_transaction(0x50, 1, true);
    state = 0xFF; out = 0xFF; addr = 0xFF;
    if (!read_x24c02_field_via_savestate("STAT", &state) ||
        !read_x24c02_field_via_savestate("OUT", &out) ||
        !read_x24c02_field_via_savestate("ADDR", &addr)) {
        printf("FAIL: the x24c02 STAT/OUT/ADDR savestate fields are missing "
               "or ambiguous (second pass)\n");
        fceu11::CloseGame();
        return false;
    }
    printf("      correct address 0x50: STAT=%u (X24C0X_READ=3) OUT=%u "
           "(ACK=0) ADDR=0x%02X\n", state, out, addr);
    if (addr != 0x50) {
        printf("FAIL: x24c02 ADDR is 0x%02X, expected 0x50\n", addr);
        ok = false;
    }
    if (state != 3) {  // 3 = X24C0X_READ (bandai.cpp:56)
        printf("FAIL: x24c02 state is %u after the correct device address "
               "0x50 with R/W = 1, expected READ (3) -- the check rejected "
               "an address it must accept\n", state);
        ok = false;
    }
    if (out != 0) {
        printf("FAIL: x24c02 out is %u after the READ ACK, expected 0\n",
               out);
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

    // ==================== v2.0.5 A1 ====================
    printf("\n--- v2.0.5 A1: unregistered mapper board-power preservation ---\n");
    const std::filesystem::path sweep_dir =
        std::filesystem::temp_directory_path() / "f11qa_a1_sweep";

    printf("[A1-S1] bmap classification completeness ... \n");
    if (!check_bmap_classification()) failed = true;

    printf("[A1-S2] unregistered board-power sweep (%u numbers) ...\n",
           static_cast<unsigned>(sizeof(kUnregisteredBoardPowerMappers) /
                                 sizeof(kUnregisteredBoardPowerMappers[0])));
    if (!check_unregistered_power_sweep(sweep_dir)) failed = true;

    printf("[A1-S3] mapper 252 WRAM execution anchor ... \n");
    if (!check_mapper252_power(sweep_dir)) failed = true;

    if (!check_registered_forwarder_controls()) failed = true;

    // ==================== v2.0.5 A2 ====================
    // The 18 MMC3-family boards whose own power function was swallowed by
    // CartInfo_PowerForward (and mapper 45's write window, A4).
    printf("\n--- v2.0.5 A2: MMC3-family board power wiring ---\n");
    {
        std::error_code ec;
        std::filesystem::create_directories(sweep_dir, ec);
    }
    printf("[A2] generic MMC3 control ... ");
    if (check_a2_generic_control()) {
        printf("OK\n");
    } else {
        failed = true;
    }
    for (int i = 0; i < NUM_A2_CASES; ++i) {
        const A2Case& c = kA2Cases[i];
        printf("[A2] mapper %3u ... ", c.mapper);
        if (check_a2_case(c, sweep_dir)) {
            printf("OK\n");
        } else {
            printf("FAILED\n");
            failed = true;
        }
    }
    // Mapper 245 is wired too, but it gets NO behavioral lock: M245Power
    // seeds EXPREGS[0]=0 and then calls GenMMC3Power(), whose MMC3RegReset
    // -> FixMMC3CHR -> cwrap -> M245CW immediately overwrites EXPREGS[0]
    // with a CHR bank value (0x07 after the reset).  The board path and the
    // base-class path therefore end in the SAME observable state, so no
    // lock can tell them apart.  The override is kept because it is the
    // semantically correct thing to do -- but it is honestly NOT covered.
    printf("[A2] mapper 245 ... NO LOCK (M245Power is observationally "
           "identical to GenMMC3Power: the EXPREGS[0] it seeds is "
           "immediately overwritten by its own CHR wrap M245CW)\n");
    printf("[A2] mapper  45 write window (A4) ... ");
    if (check_mapper45_window(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // ==================== A-1 (2026-11-18 plan §二 A-1) ====================
    // The 7 MMC3-family boards whose own reset function was swallowed by
    // CartInfo_ResetForward -- the Reset-side sibling of the A2 power
    // wiring above.  See the A-1 section comment for the leg structure and
    // the mutation legs.
    printf("\n--- A-1: MMC3-family board reset wiring ---\n");
    {
        std::error_code ec;
        std::filesystem::create_directories(sweep_dir, ec);
    }
    for (int i = 0; i < NUM_A1_CLEAR_CASES; ++i) {
        const A1ClearCase& c = kA1ClearCases[i];
        printf("[A-1] mapper %3u clear ... ", c.mapper);
        if (check_a1_reset_clears(c, sweep_dir)) {
            printf("OK\n");
        } else {
            printf("FAILED\n");
            failed = true;
        }
    }
    printf("[A-1] mapper  12 language-bit toggle ... ");
    if (check_a1_mapper12_reset(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }
    printf("[A-1] mapper  45 lock counter ... ");
    if (check_a1_mapper45_reset(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }
    printf("[A-1] mapper 205 solder-pad toggle ... ");
    if (check_a1_mapper205_reset(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }
    printf("[A-1] no-reset controls (mapper 4 + 74/118/119/191/192/194) ... ");
    if (check_a1_no_reset_controls(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // ==================== v2.0.5 A8 ====================
    // Mapper 471 (Impact Soft IM1): upstream 2.8.0 port, iNES 201 + scanline
    // IRQ.  See the A8 section above for the leg structure and mutations.
    printf("\n--- v2.0.5 A8: mapper 471 (Impact Soft IM1) port ---\n");
    printf("[A8-S1] bmap + registry dual registration ... ");
    if (check_mapper471_registered()) {
        printf("OK\n");
    } else {
        failed = true;
    }
    printf("[A8-S2] power/handler identity + address-as-data latch + reset ...\n");
    if (!check_mapper471_latch(sweep_dir)) failed = true;

    // ==================== v2.0.5 A5 ====================
    // Mapper 4 NES 2.0 PRG-RAM sizing (upstream 2.8.0 mmc3.cpp:363-375).
    // See the A5 section above for the leg structure and mutations.
    printf("\n--- v2.0.5 A5: mapper 4 NES 2.0 PRG-RAM ---\n");
    printf("[A5] mapper 4 WRAM is 8 KiB regardless of the NES 2.0 declaration ...\n");
    if (!check_a5_mapper4_nes2_wram(sweep_dir)) failed = true;

    // ==================== v2.0.5 A7-3 ====================
    // Mapper 72 write window must start at $8000 (upstream 2.8.0 72.cpp:59).
    printf("\n--- v2.0.5 A7-3: mapper 72 write window ---\n");
    printf("[A7-72] $6000 must not carry the board write handler ... ");
    if (check_mapper72_window(sweep_dir)) {
        printf("OK\n");
    } else {
        failed = true;
    }

    // ==================== v2.0.5 A7-2 ====================
    // x24c02 rejects a wrong device address (upstream 2.8.0 bandai.cpp:181-187).
    printf("\n--- v2.0.5 A7-2: bandai x24c02 device-address check ---\n");
    printf("[A7-bandai] wrong device address -> STANDBY+NAK; 0x50 -> READ ...\n");
    if (!check_bandai_x24c02_address(sweep_dir)) failed = true;

    // v2.0.5 BUG-V205-01: a ROM whose CHR bank count is not a power of two
    // must load, with the file's CHR bytes in place and 0xFF padding after
    // them (upstream's rule).
    printf("[BUG-V205-01] CHR bank count that is not a power of two ... \n");
    if (!check_non_power2_chr_rom(sweep_dir)) failed = true;

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
