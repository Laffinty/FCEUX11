// FCEUX11 v2.0.5 A6 -- mapper 210 (Namco 175 / Namco 340) mirroring lock.
//
// History: mapper 210's loader path shared mapper 19's (Namco 163) mirror
// sync through a single `is210` int flag and one SyncMirror() switch with
// the table 0->MI_0, 1->MI_V, 2->MI_H, 3->MI_0 -- case 3 duplicated case 0
// and entry 2 was wrong for the Namco 340, whose $E000 bits 6-7 select
// 0=One-screen A, 1=Vertical, 2=One-screen B, 3=Horizontal per the NESdev
// wiki "INES Mapper 210" (newest Wayback capture 20260304042007,
// 2026-03-04; identical to upstream FCEUX 2.8.0 n106.cpp:66-73).
// OBSERVATION (registered, not adopted): Mesen v2.2.0 claims the 2/3
// entries are swapped (2=Horizontal, 3=One-screen B); the current wiki
// text does not corroborate that swap, so the wiki/upstream table stands.
//
// The fix (v2.0.5 A6, src/boards/n106.cpp):
//   * is210 -> `enum NamcoType { N163, N175, N340 } type`.
//   * N340SyncMirror() carries the wiki table.
//   * $E000 stores gorko = V>>6 on every variant but only N340 applies it
//     (N175 has hardwired mirroring; N163 mirrors through FixNTAR).
//   * Mapper210_StateRestore re-applies N340 mirroring after a savestate
//     load -- vnapage[] is NOT part of the savestate, so the mapper hook
//     is the only restorer (verified: no vnapage SFORMAT in src/ppu.cpp /
//     src/state.cpp).
//   * Mapper210_DetectType(): NES 2.0 submapper 1/2 wins, otherwise five
//     known N175 CRC32s (upstream 2.8.0 constants), default N340.
//
// The legs below assert the PPU-visible nametable layout -- the `vnapage`
// pointer table bound to g_ppu.vnapage_ (src/ppu_class.h:372-374) -- against
// the four layouts Ppu::set_mirror_mode builds (src/ppu_class.cpp:121-141).
// They never restate the mapper's switch, so a wrong table cannot agree
// with the test.
//
// Mutation map (each one-line revert must turn the named leg red):
//   M1  N340SyncMirror: case 2 -> MI_H and case 3 -> MI_0 (the old broken
//       table shape)                    -> leg1 steps 0x80/0xC0 red,
//                                          leg4 steps 0x80/0xC0 red.
//   M2  Mapper210_StateRestore: delete the N340SyncMirror call
//                                       -> leg6 (savestate roundtrip)
//                                          final assert red.
//   M3a $E000 gate `type == N340` -> `type != N163`
//                                       -> leg2 red (N175 starts switching).
//   M3b $E000 gate `type == N340` -> unconditional
//                                       -> leg3 red (mapper 19 starts
//                                          switching through gorko).
//   M3c Power gate `type == N163` -> unconditional
//                                       -> leg1/leg2 baselines red (N163's
//                                          FixNTAR collapses the pages onto
//                                          screen B).
//   M3d DetectType submapper 1<->2 swapped
//                                       -> leg1/leg2 red.
//   M3e DetectType CRC constant mistyped
//                                       -> leg5 red (CRC branch falls
//                                          through to default N340).
//   M3f remove the "GORK" entry from N106_StateRegs
//                                       -> leg6 red (gorko does not
//                                          roundtrip through the state).
//
// CRC engineering: the loader hashes the PADDED PRG+CHR (for these 16 KiB
// fixtures that is exactly the PRG bytes).  Legs 4/5 force the file's CRC32
// to a chosen value by solving a 4-byte patch (CRC32 is affine over GF(2);
// the solver self-checks the result, so a solver bug fails loudly instead
// of producing a silently wrong fixture).  Leg 5's target is the real
// Chibi Maruko Chan CRC 0x0c47946d, which also hits the loader's
// ROM-correction entry {mapper 210, mirror vertical} -- a no-op for this
// synthetic ROM (the header is already mapper 210 with the vertical bit),
// so the leg exercises the exact real-cart path.  The natural CRCs of the
// other legs were verified clear of both CRC-keyed loader tables
// (ROM_CORRECTIONS and INPSEL_CRC) at implementation time.

#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cstring>
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
#include "ppu.h"         // NTARAM + vnapage aliases (bound to g_ppu members)
#include "state.h"       // FCEUSS_SaveMS / FCEUSS_LoadFP / SSLOADPARAM_NOBACKUP
#include "emufile.h"     // EMUFILE_MEMORY

// ---------------------------------------------------------------------------
// CRC32 (zlib flavor: reflected, poly 0xEDB88320, init/xorout all-ones).
// Must match the loader's crc32fast computation over the mapped PRG bytes.
// ---------------------------------------------------------------------------
static uint32_t crc32_range(const uint8_t* p, size_t n) {
    static uint32_t table[256];
    static bool table_ready = false;
    if (!table_ready) {
        for (uint32_t i = 0; i < 256; ++i) {
            uint32_t r = i;
            for (int k = 0; k < 8; ++k)
                r = (r & 1u) ? (0xEDB88320u ^ (r >> 1)) : (r >> 1);
            table[i] = r;
        }
        table_ready = true;
    }
    uint32_t c = 0xFFFFFFFFu;
    for (size_t i = 0; i < n; ++i)
        c = table[(c ^ p[i]) & 0xFFu] ^ (c >> 8);
    return c ^ 0xFFFFFFFFu;
}

// Solve a 4-byte patch so that crc32(message with the patch) == target.
// With the message fixed, F(x) = crc(msg(x)) = base ^ L(x) with L linear
// over GF(2); build the 32 basis images of L, XOR-reduce the wanted delta,
// then SELF-CHECK (crc must equal target) so a solver bug cannot fake a
// pass.  Returns false (leaving the last trial patch in place) on failure.
static bool force_prg_crc32(uint8_t* prg, size_t prg_len, size_t patch_off,
                            uint32_t target) {
    auto crc_with = [&](uint32_t x) -> uint32_t {
        prg[patch_off + 0] = static_cast<uint8_t>(x >> 24);
        prg[patch_off + 1] = static_cast<uint8_t>(x >> 16);
        prg[patch_off + 2] = static_cast<uint8_t>(x >> 8);
        prg[patch_off + 3] = static_cast<uint8_t>(x);
        return crc32_range(prg, prg_len);
    };
    const uint32_t base = crc_with(0);
    uint32_t basis[32];
    uint32_t bmask[32];
    bool have[32];
    memset(have, 0, sizeof(have));
    for (int bit = 0; bit < 32; ++bit) {
        uint32_t v = crc_with(1u << bit) ^ base;
        uint32_t m = 1u << bit;
        while (v) {
            int hi = 31;
            while (!(v & (1u << hi))) --hi;
            if (!have[hi]) {
                have[hi] = true;
                basis[hi] = v;
                bmask[hi] = m;
                break;
            }
            v ^= basis[hi];
            m ^= bmask[hi];
        }
    }
    uint32_t x = 0;
    uint32_t w = target ^ base;
    bool solvable = true;
    for (int hi = 31; hi >= 0; --hi) {
        if (w & (1u << hi)) {
            if (!have[hi]) { solvable = false; break; }
            w ^= basis[hi];
            x ^= bmask[hi];
        }
    }
    const bool ok = solvable && (w == 0) && (crc_with(x) == target);
    return ok;
}

// ---------------------------------------------------------------------------
// Synthetic ROM writer.  Self-contained adaptation of the pattern in
// tests/f11qa/boards/mapper_load_test.cpp (write_minimal_ines_rom):
// 1 x 16 KiB PRG, no CHR banks -> CHR-RAM, header assembled field by field.
// ---------------------------------------------------------------------------
struct RomSpec {
    const char* filename;
    uint32_t    mapper;
    bool        nes2;        // NES 2.0 header (needed for the submapper)
    int         submapper;   // NES 2.0 byte 8 high nibble (0 for iNES 1.0)
    uint8_t     tag;         // PRG[0]; keeps natural CRCs distinct per leg
    int         mirror_bit;  // header byte 6 bit 0: 0 = horizontal, 1 = vertical
    bool        force_crc;   // patch 4 PRG bytes so the loader CRC == crc_target
    uint32_t    crc_target;
};

static const size_t kPrgBytes = 16384;
static const size_t kPrgPatchOff = 0x200;  // inside the NOP sled, never executed

static bool write_test_rom(const RomSpec& s, const std::filesystem::path& path) {
    std::vector<uint8_t> file;
    file.reserve(16 + kPrgBytes);
    uint8_t hdr[16] = { 0 };
    hdr[0] = 'N';
    hdr[1] = 'E';
    hdr[2] = 'S';
    hdr[3] = 0x1A;
    hdr[4] = 1;   // 1 x 16 KiB PRG
    hdr[5] = 0;   // 0 x 8 KiB CHR banks -> CHR-RAM
    if (!s.nes2) {
        // iNES 1.0: mapper low nibble in byte 6 bits 4-7, high nibble in
        // byte 7 bits 4-7 (mapper 210 = 0xD2 and mapper 19 = 0x13 both fit).
        hdr[6] = static_cast<uint8_t>(((s.mapper & 0x0F) << 4) | (s.mirror_bit & 1));
        hdr[7] = static_cast<uint8_t>(s.mapper & 0xF0);
    } else {
        // NES 2.0: byte 7 bits 2-3 = 0b10 signature; byte 8 low nibble =
        // mapper bits 8-11, high nibble = submapper.
        hdr[6] = static_cast<uint8_t>(((s.mapper & 0x0F) << 4) | (s.mirror_bit & 1));
        hdr[7] = static_cast<uint8_t>((((s.mapper >> 4) & 0x0F) << 4) | 0x08);
        hdr[8] = static_cast<uint8_t>(((s.mapper >> 8) & 0x0F) | ((s.submapper & 0x0F) << 4));
    }
    file.insert(file.end(), hdr, hdr + sizeof(hdr));
    file.resize(16 + kPrgBytes, 0xEA);   // NOP sled
    file[16 + 0] = s.tag;
    file[16 + 1] = static_cast<uint8_t>(s.mapper & 0xFF);
    if (s.force_crc) {
        if (!force_prg_crc32(file.data() + 16, kPrgBytes, kPrgPatchOff,
                             s.crc_target)) {
            printf("      FAIL: CRC32 forcing to 0x%08X failed for %s\n",
                   s.crc_target, s.filename);
            return false;
        }
    }
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    if (!f) return false;
    f.write(reinterpret_cast<const char*>(file.data()),
            static_cast<std::streamsize>(file.size()));
    return static_cast<bool>(f);
}

// ---------------------------------------------------------------------------
// PPU-visible layout probe.  Classifies the current vnapage[0..3] pointer
// table into one of the four layouts Ppu::set_mirror_mode produces
// (src/ppu_class.cpp:121-141).  Returns "UNRECOGNIZED" for anything else
// (e.g. a mapper page pointer), which every expectation below treats as red.
// ---------------------------------------------------------------------------
static const char* layout_name() {
    uint8_t* nt  = &NTARAM[0];
    uint8_t* nth = &NTARAM[0x400];
    uint8_t* p0 = vnapage[0];
    uint8_t* p1 = vnapage[1];
    uint8_t* p2 = vnapage[2];
    uint8_t* p3 = vnapage[3];
    if (p0 == nt && p1 == nt && p2 == nt && p3 == nt) return "MI_0";
    if (p0 == nth && p1 == nth && p2 == nth && p3 == nth) return "MI_1";
    if (p0 == nt && p1 == nt && p2 == nth && p3 == nth) return "MI_H";
    if (p0 == nt && p1 == nth && p2 == nt && p3 == nth) return "MI_V";
    return "UNRECOGNIZED";
}

static bool expect_layout(const char* what, const char* expected) {
    const char* got = layout_name();
    if (strcmp(got, expected) == 0) {
        printf("      ok: %-34s -> %s\n", what, got);
        return true;
    }
    printf("      FAIL: %-30s -> vnapage layout %s, expected %s\n",
           what, got, expected);
    return false;
}

// Load a spec'd ROM and check the reported mapper number.  The mapper check
// doubles as the no-correction assertion for the forced-CRC legs: a CRC
// colliding with the loader's correction table would change the mapper.
static FCEUGI* load_spec(const std::filesystem::path& dir, const RomSpec& s,
                         int want_mapper) {
    const std::filesystem::path path = dir / s.filename;
    if (!write_test_rom(s, path)) {
        printf("      FAIL: cannot write %s\n", s.filename);
        return nullptr;
    }
    FCEUGI* gi = fceu11::LoadGame(path.string().c_str(), 1, true);
    if (!gi) {
        printf("      FAIL: %s did not load\n", s.filename);
        return nullptr;
    }
    if (gi->mappernum != want_mapper) {
        printf("      FAIL: %s loaded as mapper %d, expected %d\n",
               s.filename, gi->mappernum, want_mapper);
        fceu11::CloseGame();
        return nullptr;
    }
    return gi;
}

// ---------------------------------------------------------------------------
// Leg 1 -- Namco 340 (NES 2.0 submapper 2): $E000 bits 6-7 switch mirroring
// through the wiki table.  The baseline also pins that the board's Power
// took the non-N163 branch (the N163-only FixNTAR would have collapsed all
// four pages onto screen B, i.e. "MI_1", instead of the header's vertical).
// ---------------------------------------------------------------------------
static bool leg_n340_submapper2(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_n340_submapper2.nes", 210, true, 2,
                           0xA1, 1, false, 0 };
    printf("[leg1] N340 (submapper 2): $E000 mirror switch\n");
    FCEUGI* gi = load_spec(dir, spec, 210);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after load (header V)", "MI_V");
    struct Step { uint8_t v; const char* expect; };
    const Step steps[] = {
        { 0x00, "MI_0" },   // one-screen A
        { 0x40, "MI_V" },   // vertical
        { 0x80, "MI_1" },   // one-screen B  (old broken table: MI_H)
        { 0xC0, "MI_H" },   // horizontal    (old broken table: MI_0)
    };
    for (const Step& st : steps) {
        X6502_DMW(0xE000, st.v);
        char label[64];
        snprintf(label, sizeof(label), "$E000 <- 0x%02X (gorko=%d)",
                 st.v, st.v >> 6);
        ok &= expect_layout(label, st.expect);
    }
    fceu11::CloseGame();
    return ok;
}

// ---------------------------------------------------------------------------
// Leg 2 -- Namco 175 (NES 2.0 submapper 1): mirroring is hardwired; $E000
// bits 6-7 must NOT switch it.  Old behavior (is210 as a plain flag with the
// shared SyncMirror) switched mirroring here with the wrong table.
// ---------------------------------------------------------------------------
static bool leg_n175_submapper1(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_n175_submapper1.nes", 210, true, 1,
                           0xB2, 1, false, 0 };
    printf("[leg2] N175 (submapper 1): $E000 must not touch mirroring\n");
    FCEUGI* gi = load_spec(dir, spec, 210);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after load (header V)", "MI_V");
    X6502_DMW(0xE000, 0xC0);
    ok &= expect_layout("$E000 <- 0xC0 (gorko=3)", "MI_V");
    X6502_DMW(0xE000, 0x00);
    ok &= expect_layout("$E000 <- 0x00 (gorko=0)", "MI_V");
    fceu11::CloseGame();
    return ok;
}

// ---------------------------------------------------------------------------
// Leg 3 -- mapper 19 (Namco 163) control: behavior must be IDENTICAL to
// pre-A6.  Its mirroring goes through FixNTAR/DoNTARAMROM, never gorko:
//  * baseline: N163 Power seeded NTAPage[0..3] = 0xFF and ran FixNTAR, so
//    all four pages point at screen B ("MI_1") regardless of the header;
//  * $E000 writes only touch PRG + the savestate's gorko byte, the layout
//    must not move.
// ---------------------------------------------------------------------------
static bool leg_mapper19_control(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_mapper19_control.nes", 19, false, 0,
                           0xC3, 1, false, 0 };
    printf("[leg3] mapper 19 (N163) control: $E000 must not switch mirrors\n");
    FCEUGI* gi = load_spec(dir, spec, 19);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after N163 Power (FixNTAR)", "MI_1");
    X6502_DMW(0xE000, 0xC0);
    ok &= expect_layout("$E000 <- 0xC0", "MI_1");
    X6502_DMW(0xE000, 0x00);
    ok &= expect_layout("$E000 <- 0x00", "MI_1");
    fceu11::CloseGame();
    return ok;
}

// ---------------------------------------------------------------------------
// Leg 4 -- iNES 1.0 mapper 210 with an out-of-table CRC: DetectType must
// default to N340 (CRC branch default), so $E000 switches mirroring.  The
// forced CRC 0x0BADC0DE was verified clear of ROM_CORRECTIONS / INPSEL_CRC.
// ---------------------------------------------------------------------------
static bool leg_crc_default_n340(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_crc_default_n340.nes", 210, false, 0,
                           0xD4, 0, true, 0x0BADC0DEu };
    printf("[leg4] iNES 1.0, CRC not in table: default type = N340\n");
    FCEUGI* gi = load_spec(dir, spec, 210);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after load (header H)", "MI_H");
    X6502_DMW(0xE000, 0x80);
    ok &= expect_layout("$E000 <- 0x80 (gorko=2 -> one-screen B)", "MI_1");
    X6502_DMW(0xE000, 0xC0);
    ok &= expect_layout("$E000 <- 0xC0 (gorko=3 -> horizontal)", "MI_H");
    fceu11::CloseGame();
    return ok;
}

// ---------------------------------------------------------------------------
// Leg 5 -- iNES 1.0 mapper 210 whose CRC is the REAL Chibi Maruko Chan CRC
// (0x0c47946d, one of the five N175 constants): DetectType's CRC branch must
// return N175, so $E000 must NOT switch mirroring.  The same CRC hits the
// loader's ROM-correction entry {mapper 210, mirror vertical} -- a no-op
// here (header already mapper 210 vertical), so mappernum == 210 below also
// proves the correction did not distort the load.
// ---------------------------------------------------------------------------
static bool leg_crc_n175(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_crc_chibi_n175.nes", 210, false, 0,
                           0xE5, 1, true, 0x0c47946du };
    printf("[leg5] iNES 1.0, Chibi Maruko Chan CRC: type = N175\n");
    FCEUGI* gi = load_spec(dir, spec, 210);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after load (header V)", "MI_V");
    X6502_DMW(0xE000, 0xC0);
    ok &= expect_layout("$E000 <- 0xC0 must not switch", "MI_V");
    X6502_DMW(0xE000, 0x00);
    ok &= expect_layout("$E000 <- 0x00 must not switch", "MI_V");
    fceu11::CloseGame();
    return ok;
}

// ---------------------------------------------------------------------------
// Leg 6 -- savestate roundtrip on N340: write $E000 (gorko=2, layout MI_1),
// save, move the machine (write $E000 0xC0 -> layout MI_H), load the state
// back.  vnapage[] is NOT in the savestate, so ONLY the chain
// "GORK" chunk -> Mapper210_StateRestore -> N340SyncMirror can bring the
// layout back to MI_1.  Deleting that call (mutation M2) or dropping the
// "GORK" savestate entry (M3f) turns this leg red.
// ---------------------------------------------------------------------------
static bool leg_savestate_restore(const std::filesystem::path& dir) {
    const RomSpec spec = { "a6_n340_savestate.nes", 210, true, 2,
                           0xF6, 1, false, 0 };
    printf("[leg6] N340 savestate roundtrip restores gorko mirroring\n");
    FCEUGI* gi = load_spec(dir, spec, 210);
    if (!gi) return false;
    bool ok = true;
    ok &= expect_layout("baseline after load (header V)", "MI_V");
    X6502_DMW(0xE000, 0x80);
    ok &= expect_layout("$E000 <- 0x80 before save", "MI_1");

    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    const bool save_ok = FCEUSS_SaveMS(&f, 0);
    if (!save_ok) {
        printf("      FAIL: FCEUSS_SaveMS returned false\n");
        fceu11::CloseGame();
        return false;
    }
    X6502_DMW(0xE000, 0xC0);
    ok &= expect_layout("$E000 <- 0xC0 after save (state moved)", "MI_H");

    EMUFILE_MEMORY r(buf.data(), buf.size());
    const bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    if (!load_ok) {
        printf("      FAIL: FCEUSS_LoadFP returned false\n");
        fceu11::CloseGame();
        return false;
    }
    ok &= expect_layout("after state load (gorko=2 restored)", "MI_1");
    fceu11::CloseGame();
    return ok;
}

int main() {
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("=== Mapper 210 Namco 175/340 Mirror Test (v2.0.5 A6) ===\n\n");
    fflush(stdout);

    if (!fceu11::Initialize()) {
        printf("FAIL: fceu11::Initialize() returned false\n");
        return 1;
    }
    printf("fceu11::Initialize() OK\n\n");
    fflush(stdout);

    const std::filesystem::path dir =
        std::filesystem::temp_directory_path() / "f11qa_a6_mapper210";
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    std::filesystem::create_directories(dir, ec);
    if (ec) {
        printf("FAIL: cannot create the fixture directory\n");
        fceu11::Kill();
        return 1;
    }

    int passed = 0;
    const int total = 6;
    bool failed = false;

    if (leg_n340_submapper2(dir))    ++passed; else failed = true;
    if (leg_n175_submapper1(dir))    ++passed; else failed = true;
    if (leg_mapper19_control(dir))   ++passed; else failed = true;
    if (leg_crc_default_n340(dir))   ++passed; else failed = true;
    if (leg_crc_n175(dir))           ++passed; else failed = true;
    if (leg_savestate_restore(dir))  ++passed; else failed = true;

    std::filesystem::remove_all(dir, ec);

    fceu11::Kill();

    printf("\n=== Results ===\n");
    printf("Legs passed: %d / %d\n", passed, total);
    if (failed) {
        printf("RESULT: FAILED\n");
        return 1;
    }
    printf("RESULT: PASSED\n");
    return 0;
}
