
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
