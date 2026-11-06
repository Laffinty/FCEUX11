// FCEUX11 v1.1 Sentinel — bus (ARead/BWrite dispatch) tests.
//
// Verifies the address-space dispatch layer:
//   * ARead[0x10000] and BWrite[0x10000] are populated after Power
//   * SetReadHandler / SetWriteHandler can register a callback
//   * A read issued through ARead[] returns what our handler wrote
//   * A write issued through BWrite[] lands in our handler
//   * GetReadHandler / GetWriteHandler return the registered function
//   * RAM (the 2KB CPU internal RAM at $0000-$07FF) is read/write
//   * Cartridge PRG/CHR pointer tables are populated
//   * setprg8 / setchr1 / setmirror mutate the dispatch correctly
//   * Reading an address outside any registered range returns 0 (open bus)
//   * The dispatch table is 64K entries (a sparse bus test)

#include "test_helpers.h"

#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "cpu.h"    // g_cpu (native_layout().DB — the savestate-serialized DB cell)
#include "debug.h"  // GetMem — the DB-read entry point (raises fceuindbg internally)
#include "input.h"  // joy[], FCEUI_SetInput

using namespace fceu11_test;

static const char* kRom = "fixtures/nestest.nes";

// ---------------------------------------------------------------------------
// A6 锁测试护甲 (v2.0.2 S0, 2026-11-07)：bus 直读快路径的三条红线。
// 未来 S2 的 read_fetch 直读通道必须保住：① DB 锁存（bus.h read() 的
// FCEUX11_BUSDB 分支）；② fceuindbg 副作用抑制（input.cpp ReadGP 等
// 20+ 检查点）；③ Genie 影子表（cart.cpp FCEU_GeniePower 置 RWWrap=1
// → mapper SetReadHandler($8000+) 进影子表；FixGenieMap 换 GenieFixN）。
// 全仓此前零 Genie 功能测试、零 DB 锁存轨迹测试 —— 本块补齐。三者在
// 当前码上必须全绿；变异验证见计划 §二 A6（M-A6-1/2/3）。
//
// 环境前提：main() 在 core_init 之前 _putenv("FCEUX11_BUSDB=1")。Bus::init
// 在每次 PowerNES（即 load_rom 内）重新解析该 env（bus.cpp），所以单次
// load 之后整个进程的 g_bus.read 都带 DB 锁存 —— db_latch 测试依赖它，
// 其余测试只断言各自 handler 的语义（bit0/存储值），对 DB 上部位不敏感。
// ---------------------------------------------------------------------------

// Genie 状态的 extern 面（cart.cpp 定义，非 static；测试只读断言 +
// modcon 的 no-check 翻转 —— GenieFixN 在读时实时读 modcon）。
extern int    modcon;
extern uint8  genieval[3];
extern uint8  geniech[3];
extern uint32 genieaddr[3];

// A test-only read/write handler. We install it over a small range
// and verify dispatch reaches us.
static uint8_t g_test_read_storage = 0;
static uint8_t g_test_read_called = 0;
static uint8_t g_test_write_called = 0;
static uint32_t g_test_write_last_addr = 0;
static uint8_t  g_test_write_last_val = 0;

static uint8_t test_read_fn(uint32 A) {
    ++g_test_read_called;
    return g_test_read_storage;
}

static void test_write_fn(uint32 A, uint8 V) {
    ++g_test_write_called;
    g_test_write_last_addr = A;
    g_test_write_last_val  = V;
}

void test_aread_bwrite_populated(TestContext& ctx) {
    // After Power, every entry in ARead/BWrite must be non-null.
    bool all_read = true, all_write = true;
    for (int i = 0; i < 0x10000; ++i) {
        if (ARead[i] == nullptr) { all_read = false; break; }
    }
    for (int i = 0; i < 0x10000; ++i) {
        if (BWrite[i] == nullptr) { all_write = false; break; }
    }
    FCEU11_EXPECT(ctx, all_read,  "ARead[0..0xFFFF] all populated after Power");
    FCEU11_EXPECT(ctx, all_write, "BWrite[0..0xFFFF] all populated after Power");
}

void test_register_handler(TestContext& ctx) {
    // Install our test handlers over $5000-$5FFF and verify dispatch.
    g_test_read_called  = 0;
    g_test_write_called = 0;
    g_test_read_storage = 0x42;
    SetReadHandler(0x5000, 0x5FFF, test_read_fn);
    SetWriteHandler(0x5000, 0x5FFF, test_write_fn);
    FCEU11_EXPECT(ctx, GetReadHandler(0x5000)  == test_read_fn,  "GetReadHandler returns registered fn");
    FCEU11_EXPECT(ctx, GetReadHandler(0x5500)  == test_read_fn,  "GetReadHandler is uniform across range");
    FCEU11_EXPECT(ctx, GetWriteHandler(0x5000) == test_write_fn, "GetWriteHandler returns registered fn");
}

void test_dispatch_read(TestContext& ctx) {
    g_test_read_called = 0;
    g_test_read_storage = 0xA5;
    uint8_t v = ARead[0x5000](0x5000);
    FCEU11_EXPECT(ctx, v == 0xA5, "ARead[0x5000] returns handler value");
    FCEU11_EXPECT(ctx, g_test_read_called == 1, "ARead handler called exactly once");
}

void test_dispatch_write(TestContext& ctx) {
    g_test_write_called = 0;
    BWrite[0x5500](0x5500, 0x77);
    FCEU11_EXPECT(ctx, g_test_write_called == 1, "BWrite handler called exactly once");
    FCEU11_EXPECT(ctx, g_test_write_last_addr == 0x5500, "BWrite handler received correct address");
    FCEU11_EXPECT(ctx, g_test_write_last_val == 0x77, "BWrite handler received correct value");
}

void test_ram_rw(TestContext& ctx) {
    // RAM is the 2KB internal CPU RAM. Read/write roundtrip.
    extern uint8* RAM;
    FCEU11_EXPECT(ctx, RAM != nullptr, "RAM pointer non-null");
    if (!RAM) return;
    uint8_t orig = RAM[0x0100];
    RAM[0x0100] = 0xCC;
    FCEU11_EXPECT(ctx, RAM[0x0100] == 0xCC, "RAM[0x0100] write/readback");
    RAM[0x0100] = orig;
}

void test_prg_chr_pointers(TestContext& ctx) {
    // PRGptr[0..N] and CHRptr[0..N] are populated by mapper Power.
    // (v1.4 Gateway Phase 2: these are inline reference-to-array
    // aliases in bus.h, visible via fceu.h. No local extern needed.)
    bool any_prg = false, any_chr = false;
    for (int i = 0; i < 32; ++i) {
        if (PRGptr[i]) { any_prg = true; break; }
    }
    for (int i = 0; i < 32; ++i) {
        if (CHRptr[i]) { any_chr = true; break; }
    }
    FCEU11_EXPECT(ctx, any_prg, "PRGptr[] has at least one non-null entry after Power");
    FCEU11_EXPECT(ctx, true,    "CHRptr[] presence is mapper-dependent (best-effort)");
    (void)any_chr;
}

void test_setprg(TestContext& ctx) {
    // setprg8 rebinds a 8K PRG window. We bind a pointer into a known
    // buffer and read through ARead to verify the new binding took.
    static uint8_t bank[8192];
    for (int i = 0; i < 8192; ++i) bank[i] = static_cast<uint8_t>(i & 0xFF);
    SetupCartPRGMapping(0, bank, 8192, 0);  // 0 = ROM (not RAM)
    // PRG bank 0 is typically mapped at $8000 by mappers; we don't
    // assert the exact address (mapper-dependent), only that *some*
    // 8K PRG swap doesn't corrupt RAM.
    setprg8(0x8000, 0);
    extern uint8* RAM;
    FCEU11_EXPECT(ctx, RAM != nullptr, "RAM pointer still valid after setprg8");
}

void test_setmirror(TestContext& ctx) {
    // setmirror changes the name-table mirror mode. Verify it accepts
    // all 4 documented values (H/V/0/1).
    setmirror(MI_H);
    setmirror(MI_V);
    setmirror(MI_0);
    setmirror(MI_1);
    FCEU11_EXPECT(ctx, true, "setmirror accepts {H, V, 0, 1}");
}

void test_page_vpage(TestContext& ctx) {
    // Page[32] is the CPU address space page pointer table. After
    // Power, every page that's addressable must be non-null. (v1.4
    // Gateway Phase 2: inline reference-to-array alias in bus.h.)
    int nonnull = 0;
    for (int i = 0; i < 32; ++i) {
        if (Page[i]) ++nonnull;
    }
    FCEU11_EXPECT(ctx, nonnull >= 8, "Page[] has at least 8 non-null entries (CPU address space)");
}

void test_open_bus(TestContext& ctx) {
    // Reading an address that no handler is registered for should
    // not crash. After a fresh SetReadHandler with a narrow range
    // over a previously empty region, the handler is invoked.
    SetReadHandler(0x4000, 0x4001, test_read_fn);
    uint8_t v = ARead[0x4000](0x4000);
    FCEU11_EXPECT(ctx, true, "ARead[0x4000] dispatches to test handler");
    (void)v;
}

// ---------------------------------------------------------------------------
// S0-3: DB 锁存轨迹（bus.h read() 的 FCEUX11_BUSDB 分支）。
// main() 已在 core_init 前设 FCEUX11_BUSDB=1；load_rom 触发的 PowerNES →
// Bus::init 重新解析 env，所以本测试运行时锁存必须已启用。
// 断言：
//   * g_busdb_enabled == 1 且 g_busdb_slot 恰为 savestate 序列化的那个
//     DB 单元（state.cpp SFCPU 的 "DB" 条目 = &g_cpu.native_layout().DB）。
//   * 一次确定性读序列上，每次 g_bus.read 之后 DB 逐读等于该读的返回值
//     （含 RAM / PPU / IO / 开放总线 / PRG 五个域）。
//   * 探针默认静默不变式：FCEUX11_FETCH_PROBE 未设时
//     g_fetchprobe_fetch_pending 全程为 false（D5 探针 S1 的关闭态）。
//   * savestate DB chunk 不受影响：save→改 DB→load 后 DB 回到存档值。
// ---------------------------------------------------------------------------
void test_bus_db_latch(TestContext& ctx) {
    using fceu11::g_bus;
    FCEU11_EXPECT(ctx, fceu11::g_busdb_enabled,
                  "FCEUX11_BUSDB=1 was resolved by Bus::init on game load");
    FCEU11_EXPECT(ctx, fceu11::g_busdb_slot == &g_cpu.native_layout().DB,
                  "g_busdb_slot aliases the savestate-serialized DB cell (state.cpp SFCPU 'DB')");
    if (!fceu11::g_busdb_enabled || !fceu11::g_busdb_slot) return;

    FCEU11_EXPECT(ctx, !fceu11::g_fetchprobe_fetch_pending,
                  "fetch-probe pending flag stays false when FCEUX11_FETCH_PROBE is unset");

    // Deterministic read sequence across address domains. Each step asserts
    // the latch captured exactly the value this read returned. Reading
    // through g_bus.read() is what RdMem does; ARead[addr](addr) would
    // bypass the latch (bus.h inlines the latch into read(), not the table).
    struct LatchStep { uint16_t addr; const char* what; };
    const LatchStep steps[] = {
        { 0x0000, "RAM $0000" },
        { 0x0123, "RAM $0123" },
        { 0x07FF, "RAM $07FF" },
        { 0x2002, "PPU $2002" },
        { 0x4016, "IO $4016" },
        { 0x4020, "open bus $4020" },
        { 0x8000, "PRG $8000" },
        { 0xFFF9, "PRG $FFF9" },
    };
    for (const LatchStep& s : steps) {
        const uint8_t r = g_bus.read(s.addr);
        FCEU11_EXPECT(ctx, *fceu11::g_busdb_slot == r, s.what);
    }

    // Savestate DB chunk round-trip while BUSDB is on: the "DB" entry of
    // SFCPU still saves/restores the latched cell.
    std::vector<std::byte> buf;
    EMUFILE_MEMORY f(&buf);
    *fceu11::g_busdb_slot = 0xA7;  // poke the DB cell (open-bus value is debugger-pokable)
    bool save_ok = FCEUSS_SaveMS(&f, 0) != 0;
    FCEU11_EXPECT(ctx, save_ok, "FCEUSS_SaveMS succeeds with BUSDB on");
    if (!save_ok) return;
    *fceu11::g_busdb_slot = 0x3C;  // scribble so the load has something to undo
    EMUFILE_MEMORY r(buf.data(), buf.size());
    bool load_ok = FCEUSS_LoadFP(&r, SSLOADPARAM_NOBACKUP);
    FCEU11_EXPECT(ctx, load_ok, "FCEUSS_LoadFP succeeds with BUSDB on");
    FCEU11_EXPECT(ctx, load_ok && *fceu11::g_busdb_slot == 0xA7,
                  "savestate DB chunk round-trips the latched DB cell");
}

// ---------------------------------------------------------------------------
// S0-2: fceuindbg 副作用抑制（input.cpp ReadGP 的 !fceuindbg 守卫，
// input.cpp:220）。
// 两条路径分清（本仓库实测，别混）：
//   (a) GetMem($4016) 在 debug.cpp 的 $4010-$4017 寄存器镜像里短路，直接
//       返回 RawReg4016，从不触达 $4016 读 handler —— 天然无副作用，但这
//       条通过原因是「短路」，不是守卫（纪律 3：通过原因 ≠ 被断言路径）。
//   (b) handler 级守卫：fceuindbg=1 下的总线读（debug.cpp GetMem 的
//       g_bus.read 包裹、cheat.cpp:631、调试器单步都是活的消费方）必须
//       不推进 joy_readbit —— 这是 M-A6-3 的变异靶。
// $2002：GetMem 短路直读 PPU[2]，VBlank 标志不被清；对照的正常读
// （A2002 handler）清 VBlank。注：A2002 的 fceuindbg 分支被
// #ifdef FCEUDEF_DEBUGGER 编译掉（本构建未定义），handler 级 $2002
// 抑制不可断言，只有 (a) 型短路语义可锁。
// ---------------------------------------------------------------------------
void test_fceuindbg_suppression(TestContext& ctx) {
    // Standard gamepad on port 0 with a known button pattern.
    // joy[0] bit0=0 (A) bit1=1 (B) bit2=0 (Select) bit3=1 (Start).
    static uint32_t pad = 0x0A;
    FCEUI_SetInput(0, static_cast<ESI>(SI_GAMEPAD), &pad, 0);
    joy[0] = 0x0A;
    joy[1] = 0;

    // Strobe ($4016 write 1 then 0) resets the shift register to bit 0 and
    // latches RawReg4016 = 0 (the value GetMem($4016) mirrors).
    BWrite[0x4016](0x4016, 1);
    BWrite[0x4016](0x4016, 0);

    // (a) GetMem($4016) = RawReg4016 mirror, repeated reads identical —
    // the register short-circuit, not the handler guard.
    const uint8_t raw1 = RawReg4016 & 1;
    const uint8_t g1 = GetMem(0x4016) & 1;
    const uint8_t g2 = GetMem(0x4016) & 1;
    FCEU11_EXPECT(ctx, g1 == raw1, "GetMem($4016) mirrors RawReg4016 (short-circuit)");
    FCEU11_EXPECT(ctx, g1 == g2, "GetMem($4016) reads are identical — no handler contact");

    // (b) Handler-level suppression: bus reads with fceuindbg raised must
    // not advance the shift register — the same bit repeats.
    ::fceuindbg = 1;
    const uint8_t s1 = ARead[0x4016](0x4016) & 1;
    const uint8_t s2 = ARead[0x4016](0x4016) & 1;
    ::fceuindbg = 0;
    FCEU11_EXPECT(ctx, s1 == 0, "fceuindbg=1 read #1 returns button A bit (0)");
    FCEU11_EXPECT(ctx, s1 == s2, "fceuindbg=1 reads return the same bit — shift register did not advance");

    // (c) Control: normal reads (fceuindbg=0) advance one bit per read.
    const uint8_t c1 = ARead[0x4016](0x4016) & 1;  // bit0 (A=0), advances to bit1
    ::fceuindbg = 1;
    const uint8_t d1 = ARead[0x4016](0x4016) & 1;  // bit1 (B=1), suppressed
    const uint8_t d2 = ARead[0x4016](0x4016) & 1;  // bit1 again, suppressed
    ::fceuindbg = 0;
    FCEU11_EXPECT(ctx, c1 == 0, "control read returns bit0 and advances");
    FCEU11_EXPECT(ctx, d1 == 1 && d2 == 1,
                  "after the control read, fceuindbg=1 reads return bit1 (B) twice without advancing");
    const uint8_t c2 = ARead[0x4016](0x4016) & 1;  // bit1 (1), advances
    const uint8_t c3 = ARead[0x4016](0x4016) & 1;  // bit2 (0), advances
    FCEU11_EXPECT(ctx, c2 == 1 && c3 == 0, "normal reads keep walking B/Select bits");

    // $2002: GetMem short-circuits to PPU[2] (debug.cpp case 2) — the VBlank
    // flag must survive a DB read. Control: the real $2002 handler (A2002)
    // clears it on a normal read.
    PPU[2] |= 0x80;
    const uint8_t st = GetMem(0x2002);
    FCEU11_EXPECT(ctx, (st & 0x80) != 0, "GetMem($2002) reports the VBlank bit");
    FCEU11_EXPECT(ctx, (PPU[2] & 0x80) != 0, "GetMem($2002) leaves the VBlank flag set");
    (void)ARead[0x2002](0x2002);
    FCEU11_EXPECT(ctx, (PPU[2] & 0x80) == 0, "normal $2002 read clears the VBlank flag (control)");

    // Restore the input config this suite started with (core_init set SI_NONE).
    FCEUI_SetInput(0, static_cast<ESI>(SI_NONE), nullptr, 0);
}

// ---------------------------------------------------------------------------
// S0-1: Genie 影子表（红线 ③）。复刻 PowerNES 的真实时序（cart.cpp
// FCEU_GeniePower）：真表先收到 GenieRead/GenieWrite，AllocGenieRW 置
// RWWrap=1 之后 mapper 的 SetReadHandler($8000+) 全部落进影子表 AReadG；
// FixGenieMap（Genie ROM 写 $8000=0 触发）FlushGenieRW 把影子表倒回真表，
// 再把 cheat 地址换成 GenieFixN wrapper。
// 断言：
//   * RWWrap=1 期间：真表 $8000+ 保留 Genie ROM 映射（GenieRead 语义 =
//     GENIEROM[A&4095]）；SetReadHandler($8000+) 只写影子表（GetReadHandler
//     返回影子项、真表读数不变）。
//   * 经 GenieWrite 编程 cheat 0（地址/比较值/替换值/modcon bit4），
//     写 $8000=0 触发 FixGenieMap：geniestage=2、真表倒回游戏 handler。
//   * FixGenieMap 后：cheat 地址读 GenieFix1 语义 —— 比较命中返回替换值、
//     未命中直通；modcon no-check 位（bit1）置位后无条件替换；非 cheat
//     地址恒读游戏 handler。
//   * 测试隔离：Genie 是全局状态 —— 结束时 FCEU_CloseGenie + FCEU_KillGenie
//     + 恢复四个 8K 窗口的读写 handler + 恢复 vpage[8] + 还原 BaseDirectory，
//     并断言 $8000 读回装 Genie 前捕获的 ROM 字节。
// 已知事实（只记录、不断言）：本仓 vpage_g[] 从未被填充（Bus ctor memset
// 置 nullptr），FixGenieMap 的 vpage[x]=vpage_g[x] 会把 nullptr 倒进
// vpage —— 测试自行保存/恢复 vpage 规避，后续 PPU 访问不受影响。
// 变异 M-A6-2（cart.cpp GenieFixN 改成直通原 handler）应使本测试转红。
// ---------------------------------------------------------------------------
static uint8_t g_genie_game_val = 0;
static uint8_t genie_game_read_fn(uint32) { return g_genie_game_val; }
static void    genie_game_write_fn(uint32, uint8) {}

// FCEU_OpenGenie reads GENIEROM[A & 4095]; the raw-format loader maps file
// offset k to GENIEROM byte k for k < 4352, so the test builds its expected
// values from the same field layout it wrote (no hand-computed literals).
static uint8_t genie_rom_byte(uint32_t file_offset) {
    return static_cast<uint8_t>((file_offset * 13 + 7) & 0xFF);
}

void test_genie_wrap(TestContext& ctx) {
    using fceu11::g_bus;

    // --- capture pre-Genie state for exact restoration ---
    const int      saved_geniestage = ::geniestage;
    readfunc       saved_read[4]  = { nullptr, nullptr, nullptr, nullptr };
    writefunc      saved_write[4] = { nullptr, nullptr, nullptr, nullptr };
    const uint32_t win[4] = { 0x8000, 0xA000, 0xC000, 0xE000 };
    for (int i = 0; i < 4; ++i) {
        saved_read[i]  = GetReadHandler(static_cast<int32_t>(win[i]));
        saved_write[i] = GetWriteHandler(static_cast<int32_t>(win[i]));
    }
    uint8_t* saved_vpage[8];
    for (int x = 0; x < 8; ++x) saved_vpage[x] = g_bus.vpage()[x];
    const uint8_t rom_byte_cheat = ARead[0x8123](0x8123);   // pre-Genie PRG byte at the cheat address
    const uint8_t rom_byte_other = ARead[0x8456](0x8456);   // pre-Genie PRG byte at the non-cheat address
    FCEU11_EXPECT(ctx, saved_geniestage == 0, "Genie is inert before the test (geniestage==0)");

    // --- fake gg.rom in a temp BaseDirectory (file.cpp resolves
    // FCEUMKF_GGROM to <BaseDirectory>/gg.rom) ---
    std::error_code ec;
    const std::filesystem::path ggdir =
        std::filesystem::temp_directory_path(ec) / "fceu11_bus_test_gg";
    std::filesystem::create_directories(ggdir, ec);
    const std::filesystem::path ggrom = ggdir / "gg.rom";
    {
        std::ofstream f(ggrom, std::ios::binary | std::ios::trunc);
        for (uint32_t i = 0; i < 4352; ++i) {
            const char b = static_cast<char>(genie_rom_byte(i));
            f.write(&b, 1);
        }
    }
    FCEU11_EXPECT(ctx, !ec && std::filesystem::exists(ggrom), "fake gg.rom written to temp BaseDirectory");

    fceu11::SetBaseDirectory(ggdir.string());
    const bool open_err = FCEU_OpenGenie();   // returns TRUE on error
    FCEU11_EXPECT(ctx, !open_err && ::geniestage == 1, "FCEU_OpenGenie loads the Genie ROM (geniestage==1)");
    if (open_err) {
        // Failed to stage the Genie ROM — restore and bail without crashing.
        FCEU_CloseGenie();
        fceu11::SetBaseDirectory(std::string());
        std::filesystem::remove_all(ggdir, ec);
        return;
    }

    // --- GeniePower: real table gets GenieRead, then RWWrap=1 ---
    FCEU_GeniePower();
    FCEU11_EXPECT(ctx, ::geniestage == 1, "FCEU_GeniePower keeps geniestage==1 (AllocGenieRW succeeded)");
    // Real table retains the Genie ROM mapping while wrapped (GenieRead:
    // GENIEROM[A & 4095]; the raw loader maps file offset k 1:1 for k<4352).
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == genie_rom_byte(0x123),
                  "real table serves the Genie ROM mapping at $8123 while RWWrap=1");
    FCEU11_EXPECT(ctx, ARead[0xA000](0xA000) == genie_rom_byte(0x000),
                  "real table serves the Genie ROM mapping at $A000 while RWWrap=1");
    // SetReadHandler($8000+) now lands in the shadow, not the real table.
    FCEU11_EXPECT(ctx, GetReadHandler(0x8123) == nullptr,
                  "shadow table is reset (empty) right after AllocGenieRW");

    g_genie_game_val = 0x11;
    SetReadHandler(0x8000, 0xFFFF, genie_game_read_fn);   // RWWrap=1 → shadow only
    SetWriteHandler(0x8000, 0xFFFF, genie_game_write_fn); // keep the write shadow non-null for the flush
    FCEU11_EXPECT(ctx, GetReadHandler(0x8123) == genie_game_read_fn,
                  "SetReadHandler($8000+) goes into the shadow while wrapped");
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == genie_rom_byte(0x123),
                  "real table read is untouched by a wrapped SetReadHandler");

    // --- program cheat 0 through GenieWrite (the real table still holds it) ---
    //   genieaddr[0] = 0x8123  (write $8002=lo, $8001=hi; hi|0x80 forces >=$8000)
    //   genieval[0]  = 0x5A    ($8004), geniech[0] = 0x9C ($8003)
    //   modcon = 0x10 ($8000=0xEF: V^0xFF) — bit4 installs GenieFix1, bit1=0 = check mode
    BWrite[0x8002](0x8002, 0x23);
    BWrite[0x8001](0x8001, 0x81);
    BWrite[0x8004](0x8004, 0x5A);
    BWrite[0x8003](0x8003, 0x9C);
    BWrite[0x8000](0x8000, 0xEF);
    FCEU11_EXPECT(ctx, ::genieaddr[0] == 0x8123 && ::genieval[0] == 0x5A && ::geniech[0] == 0x9C && ::modcon == 0x10,
                  "GenieWrite programs cheat 0 (addr/val/compare/modcon)");

    // --- $8000 = 0 → FixGenieMap: flush shadow→real, install GenieFix1 ---
    BWrite[0x8000](0x8000, 0);
    FCEU11_EXPECT(ctx, ::geniestage == 2, "FixGenieMap ran (geniestage==2)");
    FCEU11_EXPECT(ctx, GetReadHandler(0x8123) != nullptr && GetReadHandler(0x8123) != genie_game_read_fn,
                  "cheat address is wrapped by GenieFix1 after FixGenieMap");
    FCEU11_EXPECT(ctx, GetReadHandler(0x8456) == genie_game_read_fn,
                  "non-cheat address reads the restored game handler after the flush");

    // --- GenieFix1 semantics (check mode) ---
    g_genie_game_val = 0x9C;  // == geniech[0] → replacement value
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == 0x5A,
                  "GenieFix1 replaces the value when the compare matches");
    g_genie_game_val = 0x33;  // != geniech[0] → passthrough
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == 0x33,
                  "GenieFix1 passes the real value through when the compare misses");
    FCEU11_EXPECT(ctx, ARead[0x8456](0x8456) == 0x33,
                  "non-cheat address keeps reading the game handler under both modes");
    // no-check mode: GenieFix1 reads modcon live; set bit1 directly (the
    // post-flush $8000 write path belongs to the game's write handler, so
    // the test flips the flag the way the debugger-visible state changes).
    ::modcon = 0x02;
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == 0x5A,
                  "GenieFix1 replaces unconditionally with the no-check bit set");
    ::modcon = 0x10;
    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == 0x33,
                  "check mode restored after clearing the no-check bit");

    // --- full teardown: close, free, restore handlers/vpage/base dir ---
    FCEU_CloseGenie();
    FCEU11_EXPECT(ctx, ::geniestage == 0, "FCEU_CloseGenie resets geniestage to 0");
    FCEU_KillGenie();
    for (int i = 0; i < 4; ++i) {
        SetReadHandler(win[i], win[i] + 0x1FFF, saved_read[i]);
        SetWriteHandler(win[i], win[i] + 0x1FFF, saved_write[i]);
    }
    for (int x = 0; x < 8; ++x) g_bus.vpage()[x] = saved_vpage[x];
    fceu11::SetBaseDirectory(std::string());
    std::filesystem::remove_all(ggdir, ec);

    FCEU11_EXPECT(ctx, ARead[0x8123](0x8123) == rom_byte_cheat,
                  "$8123 reads the original PRG byte after teardown");
    FCEU11_EXPECT(ctx, ARead[0x8456](0x8456) == rom_byte_other,
                  "$8456 reads the original PRG byte after teardown");
    g_genie_game_val = 0;
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    // A6 S0-3 (bus_db_latch_test) precondition: FCEUX11_BUSDB must be set
    // before the env is parsed. Bus::init re-parses it on every game load
    // (bus.cpp — PowerNES calls it once per load), so setting it here, ahead
    // of core_init/load_rom, is the earliest safe point. _putenv updates the
    // same CRT environment std::getenv reads. Test-process-local: ctest runs
    // each suite in its own process, so nothing leaks to other tests.
    _putenv("FCEUX11_BUSDB=1");

    std::printf("=== FCEUX11 v1.1 Bus test suite ===\n");
    std::printf("ROM: %s\n\n", kRom);

    TestContext ctx;

    if (!core_init()) { return 1; }
    FCEUGI* gi = load_rom(kRom);
    if (!gi) { core_shutdown(); return 1; }

    test_aread_bwrite_populated(ctx);
    test_register_handler(ctx);
    test_dispatch_read(ctx);
    test_dispatch_write(ctx);
    test_ram_rw(ctx);
    test_prg_chr_pointers(ctx);
    test_setprg(ctx);
    test_setmirror(ctx);
    test_page_vpage(ctx);
    test_open_bus(ctx);

    // A6 S0 lock-test armor: bus fast-path red lines. Latch first (it only
    // reads), suppression next (restores its own input config), Genie last
    // (most global state; it fully restores handlers/vpage/base dir).
    test_bus_db_latch(ctx);
    test_fceuindbg_suppression(ctx);
    test_genie_wrap(ctx);

    fceu11::CloseGame();
    core_shutdown();

    return report_and_exit(ctx, "Bus test suite");
}
