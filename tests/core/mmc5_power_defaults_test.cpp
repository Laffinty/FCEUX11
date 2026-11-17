// FCEUX11 v2.0.5 A7-1 - MMC5 power-on defaults lock.
//
// What is locked here:
//   GenMMC5Power (src/boards/mmc5.cpp) must start the MMC5 CHR bank
//   registers and the fill/mirroring bytes at 0, matching upstream FCEUX
//   2.8.0 mmc5.cpp (GenMMC5Power fills CHRBanksA / CHRBanksB with 0 and
//   assigns NTAMirroring = NTFill = ATFill = 0 at the same spots). This fork
//   previously powered them up at 0xFF. PRGBanks and WRAMMaskEnable keep
//   their 0xFF power-on fill (upstream agrees) and are deliberately NOT
//   locked here.
//
// Observation channel (why the savestate scan):
//   The five symbols are file-static in src/boards/mmc5.cpp - no extern
//   linkage, so unlike WRAMSIZE / iNESCart in other mapper tests they cannot
//   be referenced directly. They ARE registered to the savestate through
//   MMC5_StateRegs (AddExState, linked into SFMDATA), with tags
//     "CHRA" = CHRBanksA,   16 bytes (8 x uint16)
//     "CHRB" = CHRBanksB,    8 bytes (4 x uint16)
//     "NTAM" = NTAMirroring, 1 byte
//     "NTFL" = NTFill,       1 byte
//     "ATFL" = ATFill,       1 byte
//   FCEUSS_SaveMS at compression level 0 writes the state payload
//   uncompressed (Rust V2 state-file serializer: level 0 = raw), and each
//   SFORMAT entry is serialized as its 4-byte tag, a 4-byte little-endian
//   size, then the raw bytes (src/state.cpp SubWrite). So the register
//   values are recoverable from the in-memory savestate buffer.
//
// Timing (why this asserts GenMMC5Power's real output):
//   load_rom runs the full LoadGame path, which ends in PowerNES ->
//   cart->Power() -> GenMMC5Power. The save below happens with ZERO emulated
//   frames, so the CPU has not executed a single instruction and nothing can
//   have written $5110-$5FF7 after power-on: the saved register values are
//   exactly what GenMMC5Power produced.
//
// Scanner robustness:
//   Earlier SFMDATA entries (WRAM / ERAM random-fill content) could in
//   principle contain the tag bytes, so a match only counts when the 4-byte
//   size field that follows the tag equals the expected entry size.
//
// Mutation contract (plan A7-1): in src/boards/mmc5.cpp GenMMC5Power,
// reverting any of the three A7-1 lines back to its pre-A7-1 form -
//   CHRBanksA.fill(0xFF);  (A7-1 form: CHRBanksA.fill(0);)
//   CHRBanksB.fill(0xFF);  (A7-1 form: CHRBanksB.fill(0);)
//   NTAMirroring = NTFill = ATFill = 0xFF;  (A7-1 form: = 0;)
// must turn the corresponding zero-value check red (CHRA data becomes
// FF 00 repeated for each uint16 0x00FF; NTAM / NTFL / ATFL become 0xFF).
// Restoring the A7-1 form turns the suite green again. Removing the
// CHRA / CHRB / NTAM / NTFL / ATFL registrations from MMC5_StateRegs turns
// the tag-found checks red.

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

#include "types.h"
#include "fceu.h"
#include "cart.h"
#include "state.h"
#include "emufile.h"

#include "test_helpers.h"

// Guard symbol for the observation channel: the fixture must load as
// mapper 5 so the MMC5 state registrations are the ones scanned. Same
// extern precedent as f11qa/boards/mapper_load_test.cpp.
extern CartInfo iNESCart;  // defined in src/ines.cpp

namespace {

// Scan a raw savestate buffer for one SFORMAT entry. A match requires the
// 4-byte tag AND the following 4-byte little-endian size to equal
// expected_size; the entry payload is copied to out. Returns the first
// well-formed match, false if none.
bool find_sformat_entry(const std::vector<std::byte>& buf,
                        const char* tag, uint32_t expected_size,
                        std::vector<uint8_t>& out) {
    if (buf.size() < 8) return false;
    for (size_t i = 0; i + 8 <= buf.size(); ++i) {
        if (std::memcmp(buf.data() + i, tag, 4) != 0) continue;
        const uint8_t* sp =
            reinterpret_cast<const uint8_t*>(buf.data() + i + 4);
        const uint32_t size = static_cast<uint32_t>(sp[0]) |
                              (static_cast<uint32_t>(sp[1]) << 8) |
                              (static_cast<uint32_t>(sp[2]) << 16) |
                              (static_cast<uint32_t>(sp[3]) << 24);
        if (size != expected_size) continue;
        if (i + 8 + static_cast<size_t>(size) > buf.size()) continue;
        const uint8_t* dp =
            reinterpret_cast<const uint8_t*>(buf.data() + i + 8);
        out.assign(dp, dp + size);
        return true;
    }
    return false;
}

bool all_zero(const std::vector<uint8_t>& v) {
    for (const uint8_t b : v) {
        if (b != 0) return false;
    }
    return true;
}

}  // namespace

int main() {
    fceu11_test::TestContext ctx;

    // ------------------------------------------------------------------
    // Bring the engine up and load the MMC5 fixture through the real
    // LoadGame path (ends in PowerNES -> GenMMC5Power).
    // ------------------------------------------------------------------
    FCEU11_EXPECT(ctx, fceu11_test::core_init(), "core_init");

    // Headless hygiene (same as movie_roundtrip_test): disable interactive
    // / GUI-only features that can block without a desktop session.
    AutoResumePlay = false;
    FCEU_StateRecorderSetEnabled(false);

    FCEUGI* gi = fceu11_test::load_rom("fixtures/mapper_mmc5.nes");
    FCEU11_EXPECT(ctx, gi != nullptr,
        "load fixture ROM (fixtures/mapper_mmc5.nes, mapper 5)");
    if (!gi) {
        std::printf("  (test aborted: no ROM loaded - run from tests/ so "
                    "fixtures/mapper_mmc5.nes resolves)\n");
        return fceu11_test::report_and_exit(
            ctx, "FCEUX11 v2.0.5 A7-1 - MMC5 power-on defaults test");
    }

    FCEU11_EXPECT(ctx, iNESCart.mapper_number == 5,
        "fixture loads as mapper 5 (iNESCart.mapper_number)");

    // ------------------------------------------------------------------
    // Zero frames: save the fresh power-on state uncompressed, then read
    // the MMC5 register values out of the SFMDATA stream.
    // ------------------------------------------------------------------
    std::vector<std::byte> buf;
    {
        EMUFILE_MEMORY f(&buf);
        const bool save_ok = FCEUSS_SaveMS(&f, 0) != 0;
        FCEU11_EXPECT(ctx, save_ok, "FCEUSS_SaveMS level 0 succeeds");
    }
    FCEU11_EXPECT(ctx, buf.size() > 0, "savestate buffer is non-empty");

    auto expect_power_on_zero = [&](const char* tag, uint32_t size,
                                    const char* what) {
        std::vector<uint8_t> data;
        const bool found = find_sformat_entry(buf, tag, size, data);

        const std::string label_found =
            std::string(what) + ": SFORMAT entry found in savestate";
        FCEU11_EXPECT(ctx, found, label_found.c_str());

        const std::string label_zero =
            std::string(what) + ": power-on value == 0 (A7-1)";
        if (found) {
            FCEU11_EXPECT(ctx, all_zero(data), label_zero.c_str());
        } else {
            FCEU11_EXPECT(ctx, false, label_zero.c_str());
        }
    };

    expect_power_on_zero("CHRA", 16, "CHRBanksA (8 x uint16)");
    expect_power_on_zero("CHRB", 8, "CHRBanksB (4 x uint16)");
    expect_power_on_zero("NTAM", 1, "NTAMirroring");
    expect_power_on_zero("NTFL", 1, "NTFill");
    expect_power_on_zero("ATFL", 1, "ATFill");

    fceu11_test::core_shutdown();

    return fceu11_test::report_and_exit(
        ctx, "FCEUX11 v2.0.5 A7-1 - MMC5 power-on defaults test");
}
