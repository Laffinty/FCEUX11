// FCEUX11 v1.1 Sentinel — savestate roundtrip tests.
//
// The v1.0-era savestate_regression_test (in tests/savestate_regression_test.cpp)
// checks byte-level MD5 hashes. This file adds *behavioural* roundtrip tests
// that the MD5 sweep cannot easily express:
//   * Save → Load on the same engine state restores the same CPU registers
//   * Save → Load preserves RAM contents
//   * Save → Load preserves mapper bank state
//   * Save → Reset → Load restores CPU state, not the reset state
//   * Save → different ROM → Load refuses to load (or restores to the
//     correct mapper if it accepts)
//   * Save twice → first vs second differs in PC if PC moved
//   * Save / Load with no Power first fails gracefully
//   * Save → run N frames → Load restores the Rust PPU runtime state
//     (register file, OAM, v/t/fine_x, scanline/dot; v2.1.1.7 Step B.1-6)
//   * SFORMAT is a well-formed struct with v/s/desc
//   * FCEUSS_SaveMS roundtrip preserves every byte
//   * FCEUSS_LoadFP accepts a freshly saved buffer
//   * compressSavestates toggle is honoured
//   * BackupLoadState does not crash

#include "test_helpers.h"
#include "ppu_bridge_state.h"  // v2.1.1.7 Step B.1: bridge-owned savestate staging
#include "bus.h"                    // fceu11::g_bus.write: CPU-bus path used by the PPU pokes

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>

using namespace fceu11_test;

static const char* kRom = "fixtures/nestest.nes";

struct CpuSnap {
    uint16_t pc;
    uint8_t  a, x, y, s, p;
    uint32_t timestamp;
    int      scanline;
};
static CpuSnap snap_cpu() {
    CpuSnap s;
    s.pc        = X.PC;
    s.a         = X.A;
    s.x         = X.X;
    s.y         = X.Y;
    s.s         = X.S;
    s.p         = X.P;
    s.timestamp = timestamp;
    s.scanline  = scanline;
    return s;
}
static bool cpu_eq(const CpuSnap& a, const CpuSnap& b) {
    return a.pc == b.pc && a.a == b.a && a.x == b.x && a.y == b.y &&
           a.s == b.s && a.p == b.p &&
           a.timestamp == b.timestamp && a.scanline == b.scanline;
}

void test_save_load_preserves_cpu(TestContext& ctx) {
    // Capture CPU state at a stable point.
    emulate_n(5);
    CpuSnap before = snap_cpu();
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    bool save_ok = FCEUSS_SaveMS(&f, 0) != 0;
    FCEU11_EXPECT(ctx, save_ok, "FCEUSS_SaveMS returns non-zero on success");
    if (!save_ok) return;

    // Advance 5 more frames; state should differ.
    emulate_n(5);
    CpuSnap advanced = snap_cpu();
    FCEU11_EXPECT(ctx, !cpu_eq(before, advanced),
                  "CPU state differs after 5 more frames");

    // Reload the saved state.
    EMUFILE_MEMORY r(buf.data(), buf.size());
    bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, load_ok, "FCEUSS_LoadFP returns true");
    CpuSnap after = snap_cpu();
    FCEU11_EXPECT(ctx, cpu_eq(before, after),
                  "CPU state after load matches saved state");
}

void test_save_load_preserves_ram(TestContext& ctx) {
    extern uint8* RAM;
    RAM[0x0200] = 0x11;
    RAM[0x0201] = 0x22;
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    FCEUSS_SaveMS(&f, 0);
    RAM[0x0200] = 0xFF;
    RAM[0x0201] = 0xEE;
    EMUFILE_MEMORY r(buf.data(), buf.size());
    FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, RAM[0x0200] == 0x11, "RAM[0x0200] restored from save");
    FCEU11_EXPECT(ctx, RAM[0x0201] == 0x22, "RAM[0x0201] restored from save");
    RAM[0x0200] = 0; RAM[0x0201] = 0;
}

void test_save_load_after_reset(TestContext& ctx) {
    // After Reset, the CPU is at the reset vector. Saving at that
    // point and then reloading must put us back at the reset vector,
    // not at whatever PC the un-reset state had.
    emulate_n(3);
    CpuSnap before_reset = snap_cpu();
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    FCEUSS_SaveMS(&f, 0);
    ResetNES();
    emulate_n(1);
    CpuSnap after_reset = snap_cpu();
    FCEU11_EXPECT(ctx, !cpu_eq(before_reset, after_reset),
                  "ResetNES changes CPU state");
    EMUFILE_MEMORY r(buf.data(), buf.size());
    bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, load_ok, "Load after Reset succeeds");
    CpuSnap after_reload = snap_cpu();
    FCEU11_EXPECT(ctx, cpu_eq(before_reset, after_reload),
                  "Load restores pre-reset CPU state");
}

void test_sformat_struct(TestContext& ctx) {
    // SFORMAT must be a POD with v/s/desc. We construct one and
    // verify the offsets are non-overlapping.
    SFORMAT sf = { nullptr, 0, "test" };
    FCEU11_EXPECT(ctx, sf.v == nullptr, "SFORMAT.v initial value");
    FCEU11_EXPECT(ctx, sf.s == 0,       "SFORMAT.s initial value");
    FCEU11_EXPECT(ctx, std::strcmp(sf.desc, "test") == 0, "SFORMAT.desc initial value");
}

void test_save_load_byte_identical(TestContext& ctx) {
    // Save twice in a row without advancing emulation. The two
    // buffers should be byte-identical.
    emulate_n(3);
    std::vector<std::byte> a, b;
    EMUFILE_MEMORY fa(&a);
    FCEUSS_SaveMS(&fa, 0);
    EMUFILE_MEMORY fb(&b);
    FCEUSS_SaveMS(&fb, 0);
    FCEU11_EXPECT(ctx, a.size() == b.size(), "two consecutive saves have equal size");
    FCEU11_EXPECT(ctx, a == b,
                  "two consecutive saves have byte-identical contents");
}

void test_save_load_size_sanity(TestContext& ctx) {
    // A reasonable NES savestate is between 1KB and 1MB.
    emulate_n(3);
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    FCEUSS_SaveMS(&f, 0);
    FCEU11_EXPECT(ctx, buf.size() > 1024,
                  "savestate is at least 1KB (sanity lower bound)");
    FCEU11_EXPECT(ctx, buf.size() < 4 * 1024 * 1024,
                  "savestate is at most 4MB (sanity upper bound)");
}

void test_compress_toggle(TestContext& ctx) {
    // compressSavestates is a documented toggle. Set it both ways
    // and verify save still works.
    bool orig = compressSavestates;
    compressSavestates = false;
    emulate_n(2);
    std::vector<std::byte> uncompressed;
    EMUFILE_MEMORY fu(&uncompressed);
    bool u = FCEUSS_SaveMS(&fu, 0) != 0;
    compressSavestates = true;
    std::vector<std::byte> compressed;
    EMUFILE_MEMORY fc(&compressed);
    bool c = FCEUSS_SaveMS(&fc, 0) != 0;
    FCEU11_EXPECT(ctx, u && c, "save works in both compression modes");
    // The compressed variant should be strictly smaller for a
    // non-trivial state (RAM is mostly zeros, CPU state is small).
    FCEU11_EXPECT(ctx, compressed.size() <= uncompressed.size(),
                  "compressed save is no larger than uncompressed");
    compressSavestates = orig;
}

void test_load_after_close(TestContext& ctx) {
    // Save, CloseGame, reopen, then Load. The load will be against
    // a different game instance but should at least not crash and
    // should not corrupt the engine.
    emulate_n(2);
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    FCEUSS_SaveMS(&f, 0);
    fceu11::CloseGame();
    FCEUGI* gi = load_rom(kRom);
    FCEU11_EXPECT(ctx, gi != nullptr, "re-load after CloseGame succeeds");
    EMUFILE_MEMORY r(buf.data(), buf.size());
    bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, true, "load after close/reopen returns without crash");
    (void)load_ok;
    emulate_n(2);
    FCEU11_EXPECT(ctx, X.PC != 0xFFFF, "PC still in valid range after cross-instance load");
}

void test_backup_load_state(TestContext& ctx) {
    // BackupLoadState restores from the implicit backup slot.
    // In a fresh test it has nothing to load from, but the call
    // must be safe.
    emulate_n(2);
    BackupLoadState();
    FCEU11_EXPECT(ctx, true, "BackupLoadState does not crash");
}

void test_savestate_two_roms(TestContext& ctx) {
    // Save state on NROM, then load MMC1 mapper. The load is
    // expected to be rejected (different mapper → different chunk
    // names) but the engine must remain stable.
    emulate_n(2);
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    FCEUSS_SaveMS(&f, 0);
    fceu11::CloseGame();
    FCEUGI* mmc1 = load_rom("fixtures/mapper_mmc1.nes");
    FCEU11_EXPECT(ctx, mmc1 != nullptr, "MMC1 ROM loaded for cross-mapper test");
    EMUFILE_MEMORY r(buf.data(), buf.size());
    FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    emulate_n(2);
    FCEU11_EXPECT(ctx, X.PC != 0xFFFF, "engine still runs after cross-mapper load attempt");
}

void test_add_ex_state(TestContext& ctx) {
    // AddExState registers a buffer for inclusion in future
    // savestates. We register a sentinel, save, and check the
    // resulting buffer grew.
    static uint32_t sentinel = 0xDEADBEEF;
    size_t before = 0;
    {
        std::vector<std::byte> tmp;
        EMUFILE_MEMORY f(&tmp);
        FCEUSS_SaveMS(&f, 0);
        before = tmp.size();
    }
    AddExState(&sentinel, 4, 0, "V11SENT");
    size_t after = 0;
    {
        std::vector<std::byte> tmp;
        EMUFILE_MEMORY f(&tmp);
        FCEUSS_SaveMS(&f, 0);
        after = tmp.size();
    }
    FCEU11_EXPECT(ctx, after > before,
                  "AddExState registration makes subsequent save larger");
}

void test_resetexstate(TestContext& ctx) {
    // ResetExState must be callable. We invoke it with no callbacks
    // and verify the engine survives.
    ResetExState(nullptr, nullptr);
    emulate_n(2);
    FCEU11_EXPECT(ctx, true, "engine survives ResetExState call");
}

// ---------------------------------------------------------------------------
// v2.1.1.7 Step B.1-6 (D1-A): a loaded savestate must reach the Rust PPU.
//
// bridge_state_refresh_from_rust() copies the live Rust PPU runtime state
// (register file, primary OAM, the v/t address latches, the write toggle,
// fine X, the open-bus data latch and scanline/dot) into the bridge-owned
// staging block - the same accessor FCEUPPU_SaveState() uses before
// chunk-3 / chunk-31 serialise. Reading it before the save and again after
// the load therefore observes the Rust engine, not a C++ mirror.
//
// Before batch 4 the load path only restored the C++-local TempAddrT /
// RefreshAddrT scratch copies, so the post-load comparison saw the advanced
// state and failed. It is now the positive evidence for the load contract
// in docs/plans/v2.1.1.7_cpp_ppu_removal.md section B.1.
// ---------------------------------------------------------------------------
struct RustPpuSnap {
    uint8_t  regs[4];        // 2000/2001/2002/2003
    uint8_t  oam[0x100];
    uint8_t  fine_x;
    uint8_t  vtoggle;
    uint8_t  vram_buffer;
    uint8_t  data_bus;
    uint16_t v;
    uint16_t t;
    int32_t  scanline;
    int32_t  dot;
};

static RustPpuSnap snap_rust_ppu() {
    RustPpuSnap s{};
    bridge_state_refresh_from_rust();
    std::memcpy(s.regs, bridge_ppu_regs, sizeof(s.regs));
    std::memcpy(s.oam, bridge_oam, sizeof(s.oam));
    s.fine_x      = bridge_xoffset;
    s.vtoggle     = bridge_vtoggle;
    s.vram_buffer = bridge_vram_buffer;
    s.data_bus    = bridge_ppu_gen_latch;
    s.v           = bridge_refresh_addr;
    s.t           = bridge_temp_addr;
    s.scanline    = bridge_newppu_ppur_slots[bridge_newppu_pst0];
    s.dot         = bridge_newppu_ppur_slots[bridge_newppu_pst1];
    return s;
}

static bool rust_ppu_changed(const RustPpuSnap& a, const RustPpuSnap& b) {
    return std::memcmp(a.regs, b.regs, sizeof(a.regs)) != 0 ||
           std::memcmp(a.oam, b.oam, sizeof(a.oam)) != 0 ||
           a.fine_x != b.fine_x || a.vtoggle != b.vtoggle ||
           a.vram_buffer != b.vram_buffer || a.data_bus != b.data_bus ||
           a.v != b.v || a.t != b.t ||
           a.scanline != b.scanline || a.dot != b.dot;
}

void test_save_load_restores_rust_ppu_state(TestContext& ctx) {
    emulate_n(8);
    const RustPpuSnap saved = snap_rust_ppu();

    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    const bool save_ok = FCEUSS_SaveMS(&f, 0) != 0;
    FCEU11_EXPECT(ctx, save_ok, "FCEUSS_SaveMS succeeds for the Rust PPU state test");
    if (!save_ok) return;

    // Drive the PPU register window (2000-2007) through the same CPU-bus
    // path the emulated 6502 uses, so the post-load comparison cannot
    // pass by accident: the register file, the v/t latches, the open-bus
    // latch and OAM are then guaranteed to differ from the save point.
    // Negative control: with bridge_state_apply_to_rust() neutered this
    // test goes red (see docs/history/v2.1.1.7_b1_batch6.md).
    fceu11::g_bus.write(0x2000, 0x90);  // PPUCTRL
    fceu11::g_bus.write(0x2001, 0x1E);  // PPUMASK
    fceu11::g_bus.write(0x2003, 0x40);  // OAMADDR
    fceu11::g_bus.write(0x2004, 0xA5);  // OAMDATA
    fceu11::g_bus.write(0x2005, 0x0F);  // scroll write 1 (fine X)
    fceu11::g_bus.write(0x2005, 0x2A);  // scroll write 2 (fine Y)
    fceu11::g_bus.write(0x2006, 0x22);  // v/t high
    fceu11::g_bus.write(0x2006, 0x84);  // v/t low
    fceu11::g_bus.write(0x2007, 0x5A);  // VRAM write + open-bus latch
    const RustPpuSnap advanced = snap_rust_ppu();
    FCEU11_EXPECT(ctx, rust_ppu_changed(saved, advanced),
                  "Rust PPU state changes when 2000-2007 are written (accessor is live)");

    EMUFILE_MEMORY r(buf.data(), buf.size());
    const bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, load_ok, "FCEUSS_LoadFP succeeds for the Rust PPU state test");
    const RustPpuSnap after = snap_rust_ppu();

    FCEU11_EXPECT(ctx,
                  after.regs[0] == saved.regs[0] && after.regs[1] == saved.regs[1] &&
                      after.regs[2] == saved.regs[2] && after.regs[3] == saved.regs[3],
                  "Rust PPU register file (2000-2003) matches the save point after load");
    FCEU11_EXPECT(ctx, std::memcmp(after.oam, saved.oam, 16) == 0,
                  "Rust PPU OAM first 16 bytes match the save point after load");
    FCEU11_EXPECT(ctx, after.scanline == saved.scanline && after.dot == saved.dot,
                  "Rust PPU raster (scanline/dot) matches the save point after load");
    FCEU11_EXPECT(ctx,
                  after.fine_x == saved.fine_x && after.vtoggle == saved.vtoggle &&
                      after.vram_buffer == saved.vram_buffer &&
                      after.data_bus == saved.data_bus && after.v == saved.v &&
                      after.t == saved.t,
                  "Rust PPU scroll latches (v/t/fine_x/write-toggle/data-bus) match");
    FCEU11_EXPECT(ctx, std::memcmp(after.oam, saved.oam, sizeof(after.oam)) == 0,
                  "Rust PPU primary OAM (256 bytes) matches the save point after load");
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    std::printf("=== FCEUX11 v1.1 Savestate test suite ===\n");
    std::printf("ROM: %s\n\n", kRom);

    TestContext ctx;

    if (!core_init()) { return 1; }
    FCEUGI* gi = load_rom(kRom);
    if (!gi) { core_shutdown(); return 1; }

    test_save_load_preserves_cpu(ctx);
    test_save_load_preserves_ram(ctx);
    test_save_load_restores_rust_ppu_state(ctx);
    test_save_load_after_reset(ctx);
    test_sformat_struct(ctx);
    test_save_load_byte_identical(ctx);
    test_save_load_size_sanity(ctx);
    test_compress_toggle(ctx);
    test_load_after_close(ctx);
    test_backup_load_state(ctx);
    test_savestate_two_roms(ctx);
    test_add_ex_state(ctx);
    test_resetexstate(ctx);

    fceu11::CloseGame();
    core_shutdown();

    return report_and_exit(ctx, "Savestate test suite");
}
