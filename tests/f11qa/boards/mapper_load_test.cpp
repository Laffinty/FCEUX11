
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
