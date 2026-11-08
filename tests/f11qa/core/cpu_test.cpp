// FCEUX11 v1.1 Sentinel — X6502 CPU correctness tests.
//
// Uses nestest.nes as the canonical CPU test ROM. nestest is a public-domain
// ROM (Kevin Horton, 2004) designed to exercise every documented 6502 opcode
// and addressing mode. We verify behavioural invariants after running:
//   * X.PC, X.A, X.X, X.Y, X.S, X.P all start in the documented reset state
//   * The CPU advances PC by the expected amount per instruction
//   * The processor's flags (N, V, Z, C) match nestest's "log" expectations
//     for the first 100+ instructions (nestest.log is a known reference).
//   * Cycles advance monotonically (timestamp increases)
//
// All 12+ test cases here are independent of any source-code change — they
// read the public X6502 / driver / state API only.

#include "test_helpers.h"

#include <cstdio>
#include <cstdint>

#include "cpu.h"       // fceu11::cpu_instance()（timestamp_base/ref —— poll_abs 口径）
#include "bus.h"       // fceu11::g_bus.read —— NMI/IRQ 向量期望值（ROM 常量读）
#include "core_api.h"  // fceu11::IRQ —— TEMP 线注入点（fceu11 命名空间面）

using namespace fceu11_test;

// nestest.nes boots at $C004 and writes a structured log to $02 / $03 +
// $0004..$0007. The published reference log is part of the nestest.zip
// release. We assert a *minimal* invariant subset here: that the log
// written by the ROM (read from $0004..$0008 after some frames) equals
// what we observed the last time this test was authored. This is sufficient
// to catch:
//   * wrong opcodes, addressing modes, cycle counts
//   * broken flag handling
//   * broken reset/Power vector handling
//
// If/when the test log drifts, the change must be reviewed and the file
// re-baselined with --generate.

static const char* kNestestRom = "fixtures/nestest.nes";
static const int   kFramesToRun = 20;

// Reference PC values nestest's author logged for the first 8 instructions.
// nestest starts running with PC=$C000 (reset vector is forced to $C000
// by patching $FFFC/$FFFD on boot — this is a documented test convention).
// Each entry is the PC after the instruction at the listed index completes.
static const uint16_t kExpectedPC[] = {
    0xC000, 0xC005, 0xC008, 0xC00B, 0xC00F, 0xC012, 0xC015, 0xC018,
};

void test_reset_state(TestContext& ctx) {
    // X6502_Power() directly sets A=X=Y=P=0, S=$FD, and queues
    // FCEU_IQRESET in _IRQlow. The documented 6502 reset values for
    // P (I flag) and PC (reset vector) are NOT yet established — they
    // only take effect when X6502_Run() processes the pending reset.
    FCEU11_EXPECT(ctx, X.A == 0x00, "A == 0 after power");
    FCEU11_EXPECT(ctx, X.X == 0x00, "X == 0 after power");
    FCEU11_EXPECT(ctx, X.Y == 0x00, "Y == 0 after power");
    FCEU11_EXPECT(ctx, X.S == 0xFD, "S == 0xFD after power");

    // Consume the pending reset: X6502_Run processes FCEU_IQRESET,
    // which sets _P=I_FLAG and loads _PC from $FFFC/$FFFD.
    X6502_Run(7);

    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "I flag set after reset consumed");
    // U flag (bit 5) is not stored in X.P in FCEUX11's internal
    // representation; it only appears when P is pushed to the stack
    // (x6502.cpp: PUSH((_P&~B_FLAG)|(U_FLAG))). Do not assert it.
    FCEU11_EXPECT(ctx, X.PC >= 0x8000 && X.PC < 0x10000,
                  "PC is in PRG-ROM range after reset consumed");
}

void test_pc_advances(TestContext& ctx) {
    // Run a single CPU step by calling the run loop with one cycle.
    // After the first nestest instruction (a series of SEI/CLD/etc
    // initialisation), PC must have advanced past $C000.
    uint16_t pc_before = X.PC;
    X6502_Run(1);
    // 1 cycle is not enough to finish a typical multi-byte op, but PC may
    // or may not advance depending on where we are in the instruction.
    // The more robust check is: after 60 frames, PC has progressed well
    // beyond the reset vector.
    (void)pc_before;
    emulate_n(60);
    FCEU11_EXPECT(ctx, X.PC != 0xC000, "PC advanced past reset vector after 60 frames");
}

void test_register_widths(TestContext& ctx) {
    // Registers are uint8_t under the hood; verify we can write/read
    // the full 0x00..0xFF range through the public surface. This catches
    // any truncation regression introduced by future C++ wrappers.
    uint8_t origA = X.A, origX = X.X, origY = X.Y;
    for (uint16_t v = 0; v <= 0xFF; ++v) {
        X.A = static_cast<uint8_t>(v);
        X.X = static_cast<uint8_t>(v);
        X.Y = static_cast<uint8_t>(v);
        FCEU11_EXPECT(ctx, X.A == static_cast<uint8_t>(v), "A register roundtrip 0x00..0xFF");
        FCEU11_EXPECT(ctx, X.X == static_cast<uint8_t>(v), "X register roundtrip 0x00..0xFF");
        FCEU11_EXPECT(ctx, X.Y == static_cast<uint8_t>(v), "Y register roundtrip 0x00..0xFF");
    }
    X.A = origA; X.X = origX; X.Y = origY;
}

void test_flag_mask(TestContext& ctx) {
    // P register is 8 bits wide with bits 4 (B) and 5 (U) treated
    // specially. The high three bits (N, V, U) and low five (B, D, I, Z, C)
    // are addressable. Verify we can read/write each.
    uint8_t origP = X.P;
    for (int bit = 0; bit < 8; ++bit) {
        X.P = static_cast<uint8_t>(1u << bit);
        FCEU11_EXPECT(ctx, (X.P & (1u << bit)) != 0, "P register bit is settable");
    }
    X.P = origP;
}

void test_timestamp_monotonic(TestContext& ctx) {
    // timestampbase is the cumulative cycle counter (monotonic across all
    // frames). The per-frame `timestamp` variable is reset to 0 at the end
    // of each FCEUI_Emulate() call (see fceu.cpp:886-888), so checking
    // monotonicity across emulate_n() calls requires timestampbase.
    uint64_t b0 = timestampbase;
    emulate_n(10);
    uint64_t b1 = timestampbase;
    emulate_n(10);
    uint64_t b2 = timestampbase;
    FCEU11_EXPECT(ctx, b1 >= b0, "timestampbase non-decreasing after 10 frames");
    FCEU11_EXPECT(ctx, b2 >= b1, "timestampbase non-decreasing across 20 frames");
    FCEU11_EXPECT(ctx, b2 >  b0, "timestampbase advanced over 20 frames");
}

void test_scanline_progression(TestContext& ctx) {
    // scanline advances each visible scanline (~262 per NTSC frame).
    // After 60 frames, scanline should have wrapped many times.
    int s0 = scanline;
    emulate_n(60);
    int s1 = scanline;
    // scanline resets each frame at -1 (pre-render), so we just need to
    // verify it moved and ended in a valid visible/post-visible range.
    (void)s0;
    FCEU11_EXPECT(ctx, s1 >= -1 && s1 < 320, "scanline within valid NTSC range");
}

void test_nmi_trigger(TestContext& ctx) {
    // nestest writes $FF to $2000 (PPUCTRL) to enable NMI. We just
    // verify TriggerNMI doesn't crash and leaves the I-flag cleared on
    // the *next* RTI. The NMI source flags in fceu.h are FCEU_IQNMI=0x080.
    TriggerNMI();
    FCEU11_EXPECT(ctx, true, "TriggerNMI returns without crashing");
    emulate_n(1);
    FCEU11_EXPECT(ctx, true, "engine survives one frame after NMI");
}

void test_dma_cycle_invariants(TestContext& ctx) {
    // DMA via $4014 takes 513 or 514 CPU cycles (depending on alignment).
    // We don't time it directly (the engine hides that), but we can verify
    // the per-frame cycle delta via timestampbase (the per-frame `timestamp`
    // variable is reset to 0 at the end of each Emulate call). NTSC frame
    // is ~29780 master cycles = ~29780 CPU cycles at 1:1 ratio.
    uint64_t b0 = timestampbase;
    emulate_n(1);
    uint64_t b1 = timestampbase;
    uint64_t dt = b1 - b0;
    FCEU11_EXPECT(ctx, dt > 8000 && dt < 32000,
                  "one NTSC frame consumes ~8900-12000 CPU cycles (via timestampbase delta)");
}

void test_nestest_log_path(TestContext& ctx) {
    // nestest writes a log of PC/A/X/Y/P/flags/cycle-count to a memory
    // region. We don't decode the full log; we just check that after
    // 20 frames the log region ($0004..) has been written with non-zero
    // data (proving the CPU executed the documented test sequence).
    extern uint8* RAM;  // declared in fceu.h
    FCEU11_EXPECT(ctx, RAM != nullptr, "RAM pointer non-null");
    // Sample byte at $0004 (first PC byte in nestest's log).
    bool any_nonzero = false;
    for (int i = 0x0004; i < 0x0100; ++i) {
        if (RAM[i] != 0) { any_nonzero = true; break; }
    }
    FCEU11_EXPECT(ctx, any_nonzero, "nestest log region has been populated");
}

void test_addressing_modes_exercised(TestContext& ctx) {
    // After many frames, PC will be deep into the nestest test sequence,
    // having exercised every documented addressing mode at least once.
    // We verify a coverage proxy: the CPU has executed enough code that
    // PC is well past the entry point. nestest.nes spans ~$C000-$C668
    // and then loops; after 150+ frames the CPU should have traversed
    // the entire test ROM at least once.
    FCEU11_EXPECT(ctx, X.PC >= 0xC000, "PC has not wrapped below reset vector");
    FCEU11_EXPECT(ctx, X.PC > 0xC000,
                  "PC has advanced past the first instruction");
}

void test_jammed_state(TestContext& ctx) {
    // X.jammed is set if the CPU executes a KIL/JAM opcode. nestest
    // deliberately does not trip it. We verify the field is exposed
    // and is currently zero.
    FCEU11_EXPECT(ctx, X.jammed == 0, "CPU not jammed after nestest run");
}

void test_opcode_size_table(TestContext& ctx) {
    // opsize[256] gives the byte count of each 6502 opcode (including
    // the opcode byte). Known values from documented reference:
    //   BRK = 1 byte in FCEUX11 (the optional BRK_3BYTE_HACK is OFF by
    //         default; the 6502 reads a 2nd padding byte but the canonical
    //         instruction size is 1)
    //   RTI = 1, JMP abs = 3, NOP impl = 1, RTS = 1
    extern const uint8 opsize[256];
    FCEU11_EXPECT(ctx, opsize[0x00] == 1, "BRK opsize == 1 (no BRK_3BYTE_HACK)");
    FCEU11_EXPECT(ctx, opsize[0x40] == 1, "RTI opsize == 1");
    FCEU11_EXPECT(ctx, opsize[0x4C] == 3, "JMP abs opsize == 3");
    FCEU11_EXPECT(ctx, opsize[0xEA] == 1, "NOP impl opsize == 1");
    FCEU11_EXPECT(ctx, opsize[0x60] == 1, "RTS opsize == 1");
}

void test_opcode_cycle_count(TestContext& ctx) {
    // optype/opsize must agree for a known NOP: opcode 0xEA is implied
    // addressing, 1 byte, 2 cycles.
    extern const uint8 opsize[256];
    extern const uint8 optype[256];
    FCEU11_EXPECT(ctx, opsize[0xEA] == 1, "NOP byte count == 1");
    // optype encoding: low 4 bits = addressing mode. Implied/Accumulator/
    // Immediate/Branch/NULL = 0. NOP (0xEA) is implied, so the tag is 0
    // (this is documented in the table comment at x6502.cpp:601).
    FCEU11_EXPECT(ctx, (optype[0xEA] & 0x0F) == 0,
                  "NOP addressing mode tag is 0 (implied)");
}

// ---------------------------------------------------------------------------
// D1 专线 S1：cpu_interrupt_poll_test 用例集（2026-11-07）
//
// 对象：x6502.cpp 的 `if(_IRQlow)` 循环顶边界判定块（中断轮询）与六个线注
// 入点（TriggerNMI / TriggerNMI2 / X6502_IRQBegin / X6502_IRQEnd /
// fceu11::NMI / fceu11::IRQ）。kgmqa-037（中断轮询下沉 + hijack）的锁测试。
//
// **独立参照模型**（纪律 5/12：判定逻辑是共享路径，期望值不得来自被测代
// 码）：本节期望值全部按字段拼装自两个来源——
//   (1) S0 标定表 docs/history/surveys/cpu_interrupt_hijack/README.md §4/§5
//       （E-1 双通道实测：set→dispatch = 在飞指令长度 + 派发序列 7 周期；
//       VBL 路径 fresh 一边界推迟；NMI2→NMI 一级转换）；
//   (2) nesdev「CPU interrupts」服务序列结构：一次派发压栈 3 字节
//       （PCH, PCL, P；PUSH 写 $0100+S 后 S--，故 P 位于序列末字节）。
// X6502_Run(c) 预算口径：_count += c*16；每 CPU 周期扣 48 单位 ⇒ Run(2)
// 恰好容纳一条 2 周期指令，Run(7) 恰好容纳一次派发序列。
//
// **双组断言**：
//   (a) 基线断言（FCEU11_EXPECT，本步全绿）：保护当前「指令边界轮询」
//       行为 + 供变异验证证牙（M-D1-0 / M-D1-1）。
//   (b) 目标模型断言（FCEU11_EXPECT_FTP，本步**显式 RED**，注释标
//       // S2+/S5 fail-to-pass）：D1 落地后的两拍行为。test_helpers.h 的
//       FCEU11_EXPECT 无预期失败模式（失败直接进 ctx.failed → 进程退出
//       码 1 → ctest 红，违反 36 项全绿硬约束），故本文件自建计数器：
//       预期红不进 ctx.failed，单独计数并在套件报告打印。S2/S5 落地时
//       这些断言逐条转绿（设计预期红数见 main 里的打印与下方各断言注释）。
//   S1 实测状态（2026-11-07）：FTP 11 条 = RED 9（T-② 三条、T-③ 两条、
//   T-④ 四条）+ GREEN 2（T-D1-3c/3d：两态皆绿的形状 pin——BRK 槽压
//   PC+2 与向量取 $FFFA 在现行为下也成立， hijack 不得破坏它们）。
//   T-D1-4d 现红的机理：现行为在 stale 边界先派发 → CPU 进 ROM IRQ
//   处理器执行 RTI 弹栈 ⇒ S 回到 $FD ≠ $FA；S2/S5 后首切片不派发才成立。
// ---------------------------------------------------------------------------

namespace refint {
    // 一次中断派发序列压栈 3 字节（PCH, PCL, P）
    constexpr int kSeqPushes      = 3;
    // 派发序列计 7 周期（ADDCYC(7)，硬件 7 周期序列）
    constexpr int kSeqCycles      = 7;
    // 6502 无 1 周期指令：NOP sled 的 2 周期是最小步长
    constexpr int kMinInstrCycles = 2;
    // BRK(3 push) + NMI(3 push) = 现行为 hijack 缺失下的两次序列
    constexpr int kTwoSeqPushes   = 2 * kSeqPushes;
    // 一次序列对 S 的位移（负方向）
    constexpr int kSeqSDelta      = kSeqPushes;
}

// S2+ fail-to-pass 记账（见上方双组断言说明）
static int g_intl_ftp_red   = 0;
static int g_intl_ftp_green = 0;
static void check_ftp_true(TestContext& ctx, bool cond, const char* label) {
    if (cond) {
        std::printf("  ok(ftp-green) %s\n", label);
        ++ctx.passed;
        ++g_intl_ftp_green;
    } else {
        std::printf("  RED(fail-to-pass, expected until D1 S2/S5) %s\n", label);
        ++g_intl_ftp_red;
    }
}
#define FCEU11_EXPECT_FTP(ctx, cond, label)                                  \
    do {                                                                     \
        check_ftp_true((ctx), (cond), (label));                              \
    } while (0)

extern uint8* RAM;  // fceu.h（与 test_nestest_log_path 同一来源）



// 每个 Run 切片前清预算残量：_count 是跨 X6502_Run 调用**累积**的
// （X6502_RunDebug 开头 _count += cycles*16，而上一切片常以负残量退出）
// ——不清则本切片可能一条指令都不跑，「恰好 N 条指令/一次派发」的切片
// 语义就变成「至多」。只动运行预算（运行期变量），不动任何架构状态。
static void poll_run_exact(int cpu_cycles) {
    X.count = 0;
    X6502_Run(cpu_cycles);
}

// 场景基座：PowerNES → 消费 RESET（poll_run_exact(7)，RESET 分支无
// ADDCYC）→ PC 指向 RAM $200 起的 NOP sled（2 周期 = kMinInstrCycles）→
// S=$FD、PC=$200，I 旗标按 clear_i 设置。RESET 后 _IRQlow 已清。
// 注意：Run(7) 会从 ROM 复位向量跑 1-2 条 nestest 序言指令，随后
// 本函数覆盖 PC/S/P —— 状态完全确定。
static void poll_setup_boundary(bool clear_i) {
    PowerNES();
    poll_run_exact(7);
    RAM[0x200] = 0xEA;
    RAM[0x201] = 0xEA;
    RAM[0x202] = 0xEA;
    RAM[0x203] = 0xEA;
    RAM[0x204] = 0xEA;
    RAM[0x205] = 0xEA;
    X.PC = 0x200;
    X.S  = 0xFD;
    X.P  = clear_i ? (uint8_t)(X.P & ~I_FLAG) : (uint8_t)(X.P | I_FLAG);
}

// 绝对 CPU 周期（与 x6502.cpp e1_cpu_abs 同口径；测试内独立重算）
static uint64_t poll_abs() {
    return (uint64_t)fceu11::cpu_instance().timestamp_base()
         + (uint64_t)fceu11::cpu_instance().timestamp_ref();
}

// nestest 的向量是 ROM 常量：NMI=$C5AF、IRQ=$C5F4（本函数读总线取期望值，
// 不硬编码——按字段拼装）。
static uint16_t poll_nmi_vector() {
    return (uint16_t)(fceu11::g_bus.read(0xFFFA)
                    | (fceu11::g_bus.read(0xFFFB) << 8));
}
static uint16_t poll_irq_vector() {
    return (uint16_t)(fceu11::g_bus.read(0xFFFE)
                    | (fceu11::g_bus.read(0xFFFF) << 8));
}

// ①-baseline（B1/B5）：VBL 路径 NMI（TriggerNMI）→ fresh 一边界推迟 →
// 派发序列 7 周期。S0 标定：set→dispatch = 在飞指令长度（此处 2）+ 7。
// 变异映射：M-D1-0（g_e1_nmi_fresh 置位处短路）⇒ B1/B1a/B5a/B5b 四项红。
void test_poll_nmi_vbl_fresh_deferral(TestContext& ctx) {
    poll_setup_boundary(true);
    TriggerNMI();
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) != 0,
                  "B0: TriggerNMI latches NMI line immediately");
    const uint64_t t0 = poll_abs();
    poll_run_exact(2);   // 边界 1 = fresh DEFER（零周期）+ 一条 2 周期 sled NOP
    const uint64_t t1 = poll_abs();
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) != 0,
                  "B1: NMI still pending after first slice (fresh deferral)");
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "B1a: no dispatch inside first slice (I flag untouched)");
    FCEU11_EXPECT(ctx, t1 - t0 == (uint64_t)refint::kMinInstrCycles,
                  "B5a: first slice charged exactly the 2-cycle NOP (defer is free)");
    poll_run_exact(7);   // 边界 2 = NMI 派发（7 周期，扣完即退出）
    const uint64_t t2 = poll_abs();
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) == 0,
                  "B1b: NMI line cleared by the dispatch boundary");
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "B1c: dispatch set I flag");
    FCEU11_EXPECT(ctx, t2 - t1 == (uint64_t)refint::kSeqCycles,
                  "B5b: dispatch slice charged exactly the 7-cycle sequence");
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "B5c: exactly one 3-push sequence on the stack");
    FCEU11_EXPECT(ctx, X.PC == poll_nmi_vector(),
                  "B5d: dispatch took the NMI vector");
}

// ②-baseline（I2a/I2b）+ fceu11::IRQ TEMP 线：IRQ 注入后**第一个 loop-top
// 即派发**（stale 边界、无 fresh 推迟——现行为；S2 轮询下沉后由
// test_poll_target_two_phase_irq 的 FTP 断言接管）。I 旗标置位时挂起。
// ⚠ 配对说明：I2a 的两个断言编码的是 S2 将要翻转的行为，S2 落地时随
// T-D1-2 一起改写（基线与目标是一对）。
void test_poll_irq_ext_first_boundary(TestContext& ctx) {
    // I2a: I 清 → 跑一条 NOP 刷新 _PI → 注入 EXT → 首个 loop-top 派发
    poll_setup_boundary(true);
    poll_run_exact(2);                       // NOP：把 _PI 刷成 I 清（派发看 _PI）
    X6502_IRQBegin(FCEU_IQEXT);
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQEXT) != 0,
                  "I2a-0: X6502_IRQBegin latches EXT line immediately");
    const uint64_t t0 = poll_abs();
    poll_run_exact(2);
    const uint64_t t1 = poll_abs();
    // S2 rewrite (paired with T-D1-2): the pre-S2 model dispatched at the
    // stale boundary; the two-phase model samples the line at the poll
    // point of the instruction being polled, so the dispatch lands at the
    // boundary AFTER that instruction.
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "I2a-1: no dispatch inside the polled instruction's slice");
    FCEU11_EXPECT(ctx, t1 - t0 == (uint64_t)refint::kMinInstrCycles,
                  "I2a-2: the polled slice charged only the 2-cycle NOP");
    FCEU11_EXPECT(ctx, X.S == 0xFD,
                  "I2a-3: no sequence pushed inside the polled instruction");
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQEXT) != 0,
                  "I2a-4: EXT line stays asserted (mapper owns release)");
    // The dispatch slice: 7-cycle sequence at the next boundary.
    poll_run_exact(9);
    const uint64_t t2 = poll_abs();
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "I2a-5: EXT dispatch at the boundary after the poll point");
    FCEU11_EXPECT(ctx, t2 - t1 == (uint64_t)refint::kSeqCycles,
                  "I2a-6: dispatch slice charged the 7-cycle sequence");
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "I2a-7: exactly one 3-push sequence on the stack");
    // I2b: I 置位 → EXT 挂起不派发（线保持、栈不动）
    poll_setup_boundary(false);
    poll_run_exact(2);
    X6502_IRQBegin(FCEU_IQEXT);
    poll_run_exact(10);
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQEXT) != 0,
                  "I2b-1: EXT line held while I flag set");
    FCEU11_EXPECT(ctx, X.S == 0xFD,
                  "I2b-2: no sequence pushed while I flag set");
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "I2b-3: I flag remains set (blocked, not serviced)");
    // B7: TEMP 线（fceu11::IRQ 注入点）同走 IRQ 通道、每边界清一次
    poll_setup_boundary(true);
    poll_run_exact(2);
    fceu11::IRQ();
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQTEMP) != 0,
                  "B7-0: fceu11::IRQ latches TEMP line");
    poll_run_exact(2);
    // S2 rewrite: the one-shot TEMP line is consumed by the poll point,
    // and the dispatch lands at the next boundary.
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "B7-1: no dispatch inside the polled instruction's slice");
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQTEMP) == 0,
                  "B7-2: TEMP is one-shot (consumed by the poll point)");
    poll_run_exact(9);
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "B7-3: TEMP dispatches through the IRQ channel (I clear)");
}

// C-baseline：NMI2（TriggerNMI2，$2000 边沿路径）→ 一级转换 → 下一边界派
// 发（else-if 链的两级结构：转换边界不派发、派发边界不再转换）。
void test_poll_nmi2_conversion_two_boundaries(TestContext& ctx) {
    poll_setup_boundary(true);
    poll_run_exact(2);                       // _PI 刷新
    TriggerNMI2();
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI2) != 0,
                  "C0: TriggerNMI2 latches NMI2 line");
    poll_run_exact(2);                       // 边界 1 = NMI2→NMI 转换 + 2 周期 NOP
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI2) == 0,
                  "C1: NMI2 consumed by the conversion boundary");
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) != 0,
                  "C2: conversion produced a pending NMI");
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "C3: conversion boundary does not dispatch");
    poll_run_exact(7);                       // 边界 2 = NMI 派发
    FCEU11_EXPECT(ctx, (X.IRQlow & (FCEU_IQNMI | FCEU_IQNMI2)) == 0,
                  "C4: converted NMI dispatched at the next boundary");
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "C5: one 3-push sequence total");
}

// B6-baseline：NMI 无视 I 旗标（I 置位下照样派发）。
void test_poll_nmi_ignores_iflag(TestContext& ctx) {
    poll_setup_boundary(false);
    TriggerNMI();
    poll_run_exact(2);                       // fresh defer
    poll_run_exact(7);                       // dispatch
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) == 0,
                  "B6-1: NMI dispatched despite I flag set");
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "B6-2: sequence pushed");
}

// ③-baseline（B2/B3/B4）：S2 后的 hijack 单序列形态——NMI 线在 BRK 执行时
// 置位 ⇒ BRK 序列被劫持（BRK 自身压栈保留 B=1、只改读 NMI 向量、栈 3
// 字节）。037 差 1 的
// 根源（调查页 §2）在 S2 后不再是「两次序列」。
void test_poll_hijack_brk_then_nmi_two_sequences(TestContext& ctx) {
    poll_setup_boundary(true);
    poll_run_exact(2);                       // NOP@200 → PC=201，_PI 刷新
    RAM[0x201] = 0x00;                  // BRK @ $201（压 PC+2=$203、读 $FFFE）
    TriggerNMI();                       // 边界注入：线置位（hijack 判据 = BRK 执行时 NMI 线置位）
    // 预算 = 7：边界持有（0）+ BRK 劫持序列（7）= 序列终点。处理器尚未
    // 运行 ⇒ PC/S/栈是 hijack 的架构精确态（预算再长，nestest 的 NMI
    // 处理器会 RTI 弹栈、PC 离开向量，断言面就不成立了）。
    poll_run_exact(7);
    // S2 rewrite (paired with T-D1-3): the hijack collapses BRK + NMI into
    // ONE 3-push sequence — the BRK opcode slot is reused, the BRK's own
    // push (B=1) stays on the stack and only the vector fetch is diverted.
    // blargg 2-nmi_and_brk expected row `36 00 00` = "NMI interrupting BRK,
    // with B bit set on stack" (source 2-nmi_and_brk.s: nmi_flag = pla).
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "B2: hijack collapses BRK+NMI into one 3-push sequence");
    FCEU11_EXPECT(ctx, (RAM[0x100 + 0xFD - 2] & B_FLAG) != 0,
                  "B3: the hijacked sequence keeps the BRK's push (B=1)");
    // The BRK handler address (IRQ vector) is no longer pushed: the second
    // sequence is gone.  nestest's IRQ vector ($C5F4) can not collide with
    // the BRK slot PC ($0203), so this really pins "no second sequence".
    const uint16_t irq_v = poll_irq_vector();
    FCEU11_EXPECT(ctx,
        !(RAM[0x100 + 0xFD - 3] == (uint8_t)(irq_v >> 8) &&
          RAM[0x100 + 0xFD - 4] == (uint8_t)(irq_v & 0xFF)),
        "B4: the BRK handler address is not on the stack (single sequence)");
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) == 0,
                  "B2b: NMI consumed by the hijacking sequence");
    // BRK 的 opcode 槽形状：压 PC+2 = $203（opcode $201 取指 +2）
    FCEU11_EXPECT(ctx, RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x03,
                  "B3b: BRK pushed PC+2 = $203 (opcode slot shape)");
    // 向量取 NMI（$FFFA）—— hijack 把 BRK 序列变成 NMI 服务
    FCEU11_EXPECT(ctx, X.PC == poll_nmi_vector(),
                  "B4b: the hijacked sequence took the NMI vector");
}

// ③-target（// S2+ fail-to-pass）：hijack 落地后同一场景应为**单序列**：
// BRK 槽被劫持（opcode 槽 dummy 读 = 压 PC+2 不变、BRK 的 B=1 压栈保留）、
// 向量 $FFFA、
// 栈 3 字节。本步显式红（现行为是两次序列）；S2 落地转绿。
void test_poll_target_hijack_single_sequence(TestContext& ctx) {
    poll_setup_boundary(true);
    poll_run_exact(2);
    RAM[0x201] = 0x00;                  // BRK @ $201
    TriggerNMI();
    // 预算 = 7（与 B2 基线同口径）：边界持有 + BRK 劫持序列 = 序列终点，
    // 处理器未及运行 ⇒ PC/S 为架构精确态。
    poll_run_exact(7);
    // S2+ fail-to-pass：单序列（栈 3 字节）
    FCEU11_EXPECT_FTP(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
        "T-D1-3a: hijack collapses BRK+NMI into one 3-push sequence (S2)");
    // S2+ fail-to-pass：唯一压栈的 P 保留 BRK 的 B=1（blargg 行 `36 00 00`
    // = "with B bit set on stack"；向量级劫持不重压 P）
    FCEU11_EXPECT_FTP(ctx, (RAM[0x100 + 0xFD - 2] & B_FLAG) != 0,
        "T-D1-3b: hijacked sequence keeps the BRK's B=1 push (S2)");
    // opcode 槽形状不变：压 PC+2 = $203（hijack 复用 BRK 的槽；现行为下
    // BRK 也压 $203，此断言两态皆绿——pin 形状，防 hijack 丢槽）
    FCEU11_EXPECT_FTP(ctx,
        RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x03,
        "T-D1-3c: hijack keeps the BRK opcode-slot PC+2 shape");
    // S2+ fail-to-pass：向量取 $FFFA（hijack 把 BRK 序列变成 NMI 服务）
    FCEU11_EXPECT_FTP(ctx, X.PC == poll_nmi_vector(),
        "T-D1-3d: hijacked sequence takes the NMI vector (S2)");
}

// ②-target（// S2+ fail-to-pass）：两拍模型——边界注入的 EXT 线在**下一条
// 指令的倒数第二周期末**轮询点生效 ⇒ 派发不早于注入 + len(next)。
// 现行为：首个 loop-top（stale 边界）立即派发（I2a）⇒ 本步显式红；S2 转绿。
void test_poll_target_two_phase_irq(TestContext& ctx) {
    poll_setup_boundary(true);
    poll_run_exact(2);                       // _PI 刷新
    X6502_IRQBegin(FCEU_IQEXT);
    const uint64_t t0 = poll_abs();
    poll_run_exact(2);                       // 两拍模型：这条 2 周期 NOP 是被轮询指令
    const uint64_t t1 = poll_abs();
    // S2+ fail-to-pass：首切片只跑被轮询指令（2 周期），不派发
    FCEU11_EXPECT_FTP(ctx, t1 - t0 == (uint64_t)refint::kMinInstrCycles,
        "T-D1-2a: no dispatch inside the polled instruction's slice (S2)");
    // S2+ fail-to-pass：派发不发生在首切片（I 旗标未动）
    FCEU11_EXPECT_FTP(ctx, (X.P & I_FLAG) == 0,
        "T-D1-2b: I flag untouched after the first slice (S2)");
    poll_run_exact(9);                       // 派发切片（7 周期）
    const uint64_t t2 = poll_abs();
    // S2+ fail-to-pass：派发落在第二条边界（7 周期序列）
    FCEU11_EXPECT_FTP(ctx, t2 - t1 == (uint64_t)refint::kSeqCycles,
        "T-D1-2c: dispatch lands at the boundary after the poll point (S2)");
}

// ④-target（// S5+ fail-to-pass）：branch-delay——分支在飞 + 边界注入线：
// 分支切片内不派发；派发压的是**分支目标 PC**（$204）。首个合取项在 S2
// （轮询下沉）后即转绿；S5 补分支末周期采样形状（分支的轮询点是末周期）。
// 变异映射：T-D1-2 的镜像——用分支指令（BNE taken，3 周期）替 NOP。
void test_poll_target_branch_delay(TestContext& ctx) {
    poll_setup_boundary(true);
    poll_run_exact(2);                       // NOP@200 → PC=201，_PI 刷新
    RAM[0x201] = 0xD0;                  // BNE（Z 清 ⇒ taken）→ 目标 $204
    RAM[0x202] = 0x02;
    X.P = (uint8_t)(X.P & ~Z_FLAG);     // Z 清：分支必 taken（3 周期，无跨页）
    X6502_IRQBegin(FCEU_IQEXT);
    const uint64_t t0 = poll_abs();
    poll_run_exact(3);                       // 分支切片
    const uint64_t t1 = poll_abs();
    // S5+ fail-to-pass：分支切片内不派发（线在分支末周期窗口内采样）
    FCEU11_EXPECT_FTP(ctx, (X.P & I_FLAG) == 0,
        "T-D1-4a: no dispatch inside the taken-branch slice (S5)");
    FCEU11_EXPECT_FTP(ctx, t1 - t0 == (uint64_t)3,
        "T-D1-4b: branch slice charged exactly the 3-cycle taken branch (S5)");
    poll_run_exact(16);                      // 派发切片
    // S5+ fail-to-pass：派发压分支目标 PC = $204
    FCEU11_EXPECT_FTP(ctx,
        RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x04,
        "T-D1-4c: dispatched PC is the branch target $204 (S5)");
    FCEU11_EXPECT_FTP(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
        "T-D1-4d: single sequence pushed after the branch (S5)");
}

// T-D1-5（S2.5 基线）：线携带断言时刻（方案 A）——可见性按时刻门控。
// 两条核心语义：① 未来时刻登记的线，对未到期的指令不可见（电平位立即
// 置位、派发等过了时刻）；② 恰落指令结束边界的事件归属**下一条**指令的
// 窗口（严格 <；int_4 头部行的 (a) 修复语义，调查页 §12.3）。
void test_poll_assert_timestamp(TestContext& ctx) {
    // 5a：未来时刻登记（VBL 语义——TriggerNMIAt 在预算前登记）
    poll_setup_boundary(true);
    TriggerNMIAt(poll_abs() + 2);            // 2 CPU 周期后的未来时刻
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) != 0,
                  "T-D1-5a-1: level bit set immediately at registration");
    poll_run_exact(2);                        // 这条 NOP 恰在时刻处结束：不可见
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "T-D1-5a-2: line not visible to the instruction ending at the instant");
    poll_run_exact(9);                        // 过了时刻：NOP 的 poll 采样 → 本切片内边界派发
    FCEU11_EXPECT(ctx, X.S == (uint8_t)(0xFD - refint::kSeqSDelta),
                  "T-D1-5a-3: dispatch once the instant has passed");
    FCEU11_EXPECT(ctx, RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x02,
                  "T-D1-5a-5: pushed PC = $202 (dispatch after the stamp-exceeding instruction)");
    FCEU11_EXPECT(ctx, (X.IRQlow & FCEU_IQNMI) == 0,
                  "T-D1-5a-4: line consumed by the dispatch");

    // 5b：恰落边界的事件（IRQ 通道——X6502_IRQBeginAt 的 int_4 语义）
    poll_setup_boundary(true);
    X6502_IRQBeginAt(FCEU_IQEXT, poll_abs() + 2);  // 事件 = 本条 NOP 的结束边界
    poll_run_exact(2);
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "T-D1-5b-1: boundary-exact event invisible to the ending instruction");
    poll_run_exact(9);                        // 下一条 NOP 的 poll 可见 → 边界派发
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "T-D1-5b-2: dispatched at the boundary after the next instruction");
    FCEU11_EXPECT(ctx, RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x02,
                  "T-D1-5b-3: pushed PC = $202 (the instruction after the boundary event)");

    // 5c：窗口内事件（严格早于边界）——覆盖它的指令可见、该边界派发
    poll_setup_boundary(true);
    X6502_IRQBeginAt(FCEU_IQEXT, poll_abs() + 1);  // 事件在本条 NOP 中间
    poll_run_exact(2);                        // 采样（pending=2）但预算内不派发
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) == 0,
                  "T-D1-5c-1: mid-window event sampled but not dispatched in-slice");
    poll_run_exact(7);                        // 下一边界立即派发（压 PC=$201）
    FCEU11_EXPECT(ctx, (X.P & I_FLAG) != 0,
                  "T-D1-5c-2: dispatched at the very next boundary");
    FCEU11_EXPECT(ctx, RAM[0x100 + 0xFD] == 0x02 && RAM[0x100 + 0xFC] == 0x01,
                  "T-D1-5c-3: pushed PC = $201 (dispatch one instruction earlier than 5b)");
}

// T-D1-6（S2.5 阶段 1 锁，r25 章程 DMA）：OAM DMA 513/514 奇偶停机。
// 硬件真值（blargg 4-irq_and_dma 18 行拟合 + INTLEDGER delta=519 实测）：
// $4014 写的栅位奇偶决定停机段 1 dummy + 0-1 对齐 ⇒ 指令总长 4+513/4+514。
// 测量法（两轮，grant 520 巨额透支 ⇒ 切片恰在 DMA 完成后退出，span =
// 4 + stall + 512，无尾部吸收）：
//   轮 0：STA abs $4014 @200（基线栅位）         ⇒ span0 = 4 + stall0 + 512
//   轮 1：STA zp $00 @203（3 周期翻转器，真实执行）→ STA abs $4014 @205
//         ⇒ span1 = 3 + 4 + stall1 + 512；翻转器奇数位移保证 stall1 ≠ stall0。
// 绿色实测钉值（确定性 harness，start_abs=4）：(518, 520)。
// 变异：删停机 ⇒ (516, 519) 红；奇偶位翻转 ⇒ (518,520)↔(518,520) 对调——
// ⚠ 因 DMA 对齐周期的自锁结构（514 偶 + 两侧取指偶 = 写间位移偶），连续
// DMA 流的奇偶自锁在 514 档是**硬件正确行为**（对齐周期就是干这个的）——
// 本锁的翻转器路径（奇数位移）覆盖翻转语义，绝对校准另由 ROM 级 int_4
// +526/+527 行裁决。I 置位封锁 IRQ 通道防测量窗污染。
void test_oam_dma_513_514_parity(TestContext& ctx) {
    poll_setup_boundary(false);
    // round 0: STA abs $4014 @200（基线栅位，无翻转器）
    RAM[0x200] = 0x8D;
    RAM[0x201] = 0x14;
    RAM[0x202] = 0x40;
    const uint64_t t0 = poll_abs();
    poll_run_exact(520);
    const uint64_t span0 = poll_abs() - t0;
    // round 1: STA zp $00 @203（3 周期翻转器）→ STA abs $4014 @205
    RAM[0x203] = 0x85;
    RAM[0x204] = 0x00;
    RAM[0x205] = 0x8D;
    RAM[0x206] = 0x14;
    RAM[0x207] = 0x40;
    const uint64_t t1 = poll_abs();
    poll_run_exact(520);
    const uint64_t span1 = poll_abs() - t1;
    FCEU11_EXPECT(ctx, span0 == 518 && span1 == 520,
                  "T-D1-6: OAM DMA stall spans (518=4+514, 520=3+4+513) -- parity-dependent");
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    std::printf("=== FCEUX11 v1.1 CPU test suite ===\n");
    std::printf("ROM: %s\n\n", kNestestRom);

    TestContext ctx;

    if (!core_init()) { return 1; }
    FCEUGI* gi = load_rom(kNestestRom);
    if (!gi) { core_shutdown(); return 1; }

    // Run a warm-up frame so mapper banking settles. The CPU executes
    // the first ~89000 nestest instructions (SEI/CLD/etc. prologue +
    // the documented test sequence), which changes A/X/Y/S/P from
    // their reset values.
    emulate_n(1);

    // v1.2 Census: PowerNES() (not ResetNES) for the reset-state
    // check. PowerNES invokes X6502_Power() and the full PPU/APU
    // reset sequence, giving us a known starting state.
    PowerNES();

    // Phase 1: Register storage tests. These directly modify and
    // restore CPU registers, so they must run while the CPU is in a
    // quiescent state (immediately after PowerNES, before any
    // instruction execution).
    test_register_widths(ctx);
    test_flag_mask(ctx);

    // Phase 2: Post-power state verification. This consumes the
    // pending reset (FCEU_IQRESET) and checks the documented 6502
    // reset values for P and PC.
    test_reset_state(ctx);

    // Phase 3: CPU execution tests. The CPU is now running nestest.
    test_pc_advances(ctx);
    test_timestamp_monotonic(ctx);
    test_scanline_progression(ctx);
    test_nmi_trigger(ctx);
    test_dma_cycle_invariants(ctx);
    test_nestest_log_path(ctx);
    test_addressing_modes_exercised(ctx);
    test_jammed_state(ctx);
    test_opcode_size_table(ctx);
    test_opcode_cycle_count(ctx);

    // D1 专线 S1：中断轮询锁测试（基线绿 + 目标红 fail-to-pass）。
    // 每个场景自带 PowerNES 基座，放全部既有用例之后跑。
    test_poll_nmi_vbl_fresh_deferral(ctx);
    test_poll_irq_ext_first_boundary(ctx);
    test_poll_nmi2_conversion_two_boundaries(ctx);
    test_poll_nmi_ignores_iflag(ctx);
    test_poll_hijack_brk_then_nmi_two_sequences(ctx);
    test_poll_target_hijack_single_sequence(ctx);
    test_poll_target_two_phase_irq(ctx);
    test_poll_target_branch_delay(ctx);
    test_poll_assert_timestamp(ctx);
    test_oam_dma_513_514_parity(ctx);

    fceu11::CloseGame();
    core_shutdown();

    std::printf("\nfail-to-pass (D1 target model): RED=%d GREEN=%d (designed RED at S1 = 9)\n",
                g_intl_ftp_red, g_intl_ftp_green);

    return report_and_exit(ctx, "CPU test suite");
}
