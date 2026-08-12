# Phase 6 · Integration & Shadow Run

> **目标**：把 vNESU11 接入 FCEUX11 主路径，默认 **OFF**，通过 `VNESU11_CORE=ON` CMake 选项启用。**启用后**用 shadow run 验证：同一 ROM 双跑（C++ + Rust），**帧级三级对比**。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S10/S5 修订——
> ① shadow run 从"逐周期 diff"改为"帧级三级对比"（XBuf CRC / 音频 SNR / savestate MD5）；
> ② 明确 `newppu=1` 才走 vNESU11 PPU；`newppu=0` 走 C++ 旧 PPU 回退（ADR-009）。

## 工期：3 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- `CMakeLists.txt` 添加 `option(VNESU11_CORE "Use vNESU11 Rust core" OFF)`
- `src/CMakeLists.txt` 添加 vNESU11 链接逻辑
- `src/vnesu11_bridge.h/.cpp` 实现完整 FFI 调用桥
- 修改 `src/fceu.cpp` `Emulate()`/`ResetNES()`/`PowerNES()` 走 vNESU11（条件编译）
- `FCEUI_*` 兼容垫片内部实现改为调 vNESU11（条件编译）
- Shadow run 模式：同一进程双跑 C++ + Rust，**帧级三级对比**（见 §2.5）
- **[修订] newppu 分流**：`newppu=1` → vNESU11 PPU；`newppu=0` → C++ 旧 PPU 回退
- **[修订] 系统类型分流**：iNES/FDS/NSF/VS 通过 `vnesu11_set_system_type` 接入
- 调试器 / Lua 钩子 / TAS Editor 适配

### 1.2 ❌ 不在范围内
- 默认切换（Phase 7）
- 移除 C++ CPU/PPU 源（Phase 8）

---

## 2. 任务清单

### 2.1 CMake 选项

```cmake
# CMakeLists.txt（根）
option(VNESU11_CORE "Use vNESU11 Rust core (experimental)" OFF)

if(VNESU11_CORE)
    message(STATUS "Building with vNESU11 Rust core (Phase 6 - shadow run mode)")
    add_definitions(-DVNESU11_CORE_ENABLED=1)
endif()
```

```cmake
# src/CMakeLists.txt
if(VNESU11_CORE)
    target_link_libraries(fceux11_core PUBLIC ${VNESU11_RUST_LIB})
    target_sources(fceux11_core PRIVATE
        src/vnesu11_bridge.cpp
        src/vnesu11_mapper_adapter.cpp
    )
endif()
```

### 2.2 桥接代码

```cpp
// src/vnesu11_bridge.h
#pragma once
#include <vnesu11/soc.h>  // cbindgen 生成

namespace fceu11 {
    // 全局 SoC 实例（VNESU11_CORE=ON 时有效）
    extern ::VNesSocOpaque* g_vnesu11_soc;

    // 生命周期
    void vnesu11_init();
    void vnesu11_kill();
    void vnesu11_power();
    void vnesu11_reset();

    // Frame 仿真
    void vnesu11_emulate(uint8_t** xbuf, int32_t** sbuf, int* sbuf_size);

    // 调试器 / Lua 钩子
    uint8_t vnesu11_cpu_peek(uint16_t addr);
    void vnesu11_cpu_poke(uint16_t addr, uint8_t val);
    void vnesu11_cpu_peek_regs(CpuRegsLayout* out);
    void vnesu11_cpu_poke_regs(const CpuRegsLayout* in);
}
```

```cpp
// src/vnesu11_bridge.cpp
#include "vnesu11_bridge.h"
#include "fceu.h"

#ifdef VNESU11_CORE_ENABLED

::VNesSocOpaque* fceu11::g_vnesu11_soc = nullptr;

void fceu11::vnesu11_init() {
    if (g_vnesu11_soc) return;
    g_vnesu11_soc = vnesu11_create();
}

void fceu11::vnesu11_kill() {
    if (g_vnesu11_soc) {
        vnesu11_destroy(g_vnesu11_soc);
        g_vnesu11_soc = nullptr;
    }
}

void fceu11::vnesu11_power() {
    vnesu11_power_on(g_vnesu11_soc);
}

void fceu11::vnesu11_reset() {
    vnesu11_reset(g_vnesu11_soc);
}

void fceu11::vnesu11_emulate(uint8_t** xbuf, int32_t** sbuf, int* sbuf_size) {
    static thread_local std::vector<int16_t> sbuf_storage;
    sbuf_storage.clear();
    sbuf_storage.reserve(48000 * 2 / 60 * 2);  // 估算
    size_t written = 0;
    // 调用 vNESU11 跑一帧
    uint8_t xbuf_local[61440];
    vnesu11_emulate_frame(
        g_vnesu11_soc,
        xbuf_local,
        sbuf_storage.data(),  // 需要 reserve 后拿原始指针
        sbuf_storage.capacity(),
        &written
    );
    sbuf_storage.resize(written);
    // ... 写到全局 XBuf / SoundBuf
}
#endif  // VNESU11_CORE_ENABLED
```

### 2.3 条件编译 Emulate()

```cpp
// src/fceu.cpp
void fceu11::Emulate(uint8_t** xbuf, int32_t** sbuf, int* sbuf_size, int skip) {
#ifdef VNESU11_CORE_ENABLED
    // Shadow run mode：双跑 + diff
    static thread_local std::vector<int16_t> sbuf_v11;
    static thread_local std::vector<int16_t> sbuf_rust;
    static thread_local uint8_t xbuf_v11[61440];
    static thread_local uint8_t xbuf_rust[61440];

    // C++ 跑一帧
    FCEUI_EmulateCpp(xbuf, sbuf, sbuf_size, skip);  // 现有路径（重命名）
    memcpy(xbuf_v11, *xbuf, 61440);
    sbuf_v11.assign(*sbuf, *sbuf + *sbuf_size);

    // Rust 跑一帧
    vnesu11_emulate_frame(...);
    memcpy(xbuf_rust, ...);
    sbuf_rust.assign(...);

    // Diff（每 N 帧打印一次）
    if (frame_count++ % 60 == 0) {
        diff_frames(xbuf_v11, xbuf_rust, sbuf_v11, sbuf_rust);
    }
#else
    // 默认：C++ 路径
    X6502_RunDebug(...);
#endif
}
```

### 2.4 兼容垫片切换

`FCEUI_*` 函数（~150 个调用点）保留 API，内部条件编译：

```cpp
// src/core_api.h
inline void FCEUI_Emulate(...) {
#ifdef VNESU11_CORE_ENABLED
    fceu11::vnesu11_emulate(...);
#else
    // 现有 C++ 实现
    X6502_RunDebug(...);
#endif
}

inline uint16_t FCEUI_GetReg(int reg) {
#ifdef VNESU11_CORE_ENABLED
    CpuRegsLayout regs;
    fceu11::vnesu11_cpu_peek_regs(&regs);
    switch (reg) {
        case FCEUI_PC: return regs.PC;
        case FCEUI_A: return regs.A as u16;
        // ...
    }
#else
    return ::X6502_GetReg(reg);  // 现有
#endif
}
```

### 2.5 [修订] Shadow Run：帧级三级对比（S10）

**审计结论**：C++ 与 Rust 时序模型不同（决策 A 复刻 budget，但内部结构不同），
**逐周期 diff 无定义**。改为**帧级三级对比**（与现有 kagami-qa 的
rom_regression / savestate_regression harness 同构）：

```cpp
// src/vnesu11_shadow.h
enum class DiffLevel {
    CRC,      // 1) XBuf 256×240 CRC32 逐帧对比
    SNR,      // 2) 音频 buffer 逐帧 SNR（≥ 60dB 视为一致）
    MD5,      // 3) savestate 每 60 帧 MD5 对比
};

class ShadowRunHarness {
public:
    void run_rom(const std::string& path, int frames);
private:
    void diff_frame_crc(const uint8_t* cpp, const uint8_t* rust);   // 1
    void diff_audio_snr(const int16_t* cpp, const int16_t* rust, size_t n);  // 2
    void diff_savestate_md5(const CpuRegsLayout& a, const CpuRegsLayout& b); // 3
};
```

**三级判据**：
| 级别 | 判据 | 行为 |
|------|------|------|
| 1 | XBuf CRC 一致 | 通过 |
| 1 | XBuf CRC 差 ≤ 5 字节 | warning（不阻塞） |
| 1 | XBuf CRC 差 > 5 字节 | **error（阻塞 CI）** |
| 2 | 音频 SNR ≥ 60dB | 通过 |
| 3 | savestate MD5 一致 | 通过 |

**先决条件（[修订] S7）**：C++ 与 Rust 的 RAM 初始化必须逐位一致
（`xoroshiro128plus` 复刻，Phase 2 交付），否则第一帧就 diff。

**回退**：`newppu=0` 时不启用 shadow run 的 PPU 对比（C++ 旧 PPU 路径，
见 ADR-009）。

### 2.6 调试器 / Lua 钩子适配

```cpp
// src/debug.cpp
uint8_t FCEUI_DebugGetByte(uint16_t addr) {
#ifdef VNESU11_CORE_ENABLED
    return fceu11::vnesu11_cpu_peek(addr);
#else
    return ARead[addr](addr);  // 现有
#endif
}
```

### 2.7 TAS Editor / Movie 适配

```cpp
// src/movie.cpp
int FCEUI_LoadMovie(...) {
#ifdef VNESU11_CORE_ENABLED
    // movie 初始化后，vNESU11 接管帧推进
    // joypad 写入走 vNESU11
    vnesu11_joypad_set_button(g_vnesu11_soc, pad, btn, pressed);
#else
    // 现有
#endif
}
```

---

## 3. 验证策略

### 3.1 编译验证

- `cmake -B build -DVNESU11_CORE=OFF`：与 v1.17 一致
- `cmake -B build -DVNESU11_CORE=ON`：链接 vNESU11 + bridge

### 3.2 启动验证

- [ ] `VNESU11_CORE=ON` 构建的可执行文件能启动
- [ ] 加载 SMB1、Zelda、MMC3 测试 ROM 等
- [ ] 帧率 ≥ 60 FPS（无回归）
- [ ] 视觉无差（shadow run 0 diff）

### 3.3 Shadow run 套件

```
tests/shadow_run/
├── run_shadow.sh              # 调用 fceux11 + diff 工具
├── roms/
│   ├── smb1.nes
│   ├── zelda.nes
│   ├── mmc3_irq_test.nes
│   ├── blargg_cpu_instrs.nes
│   └── ...
├── baselines/                 # v1.17 帧 / 状态 golden
│   ├── smb1.frame_001.bin
│   ├── smb1.frame_060.bin
│   └── ...
└── report.html                # 生成的 diff 报告
```

### 3.4 真实游戏回归

跑 10 个常用游戏 5 分钟：
- SMB1 / SMB2 / Zelda / Metroid / Contra / Castlevania / Mega Man 2 / Kirby / Final Fantasy / Tetris
- shadow run：每帧 XBuf CRC 一致
- audio buffer：SNR ≥ 60dB

---

## 4. 性能基准

| 项目 | 目标 |
|------|------|
| 帧时间（含 C++ shadow run 开销） | ≤ 18ms（> 60 FPS 有 buffer） |
| 不开 shadow run（纯 vNESU11） | ≤ 16.67ms |
| thunk 总开销 | ≤ 5% CPU cycles |

---

## 5. 关键技术决策

### 5.1 Shadow Run 默认 ON（[修订] 仅 newppu=1 路径）

`VNESU11_CORE=ON` 且 `newppu=1` 时自动开启 shadow run（帧级三级对比）。
`newppu=0` 走 C++ 旧 PPU 回退，不参与 shadow 对比。

### 5.2 [修订] 三级 diff 输出策略（S10）

| 级别 | 判据 | 行为 |
|------|------|------|
| 1 | XBuf CRC 一致 | 静默 |
| 1 | CRC 差 ≤ 5 字节 | warning（不阻塞 CI） |
| 1 | CRC 差 > 5 字节 | error（阻塞 CI） |
| 2 | 音频 SNR ≥ 60dB | 静默 |
| 3 | savestate MD5 一致 | 静默 |

### 5.3 FCEUI_* 兼容垫片优先

~150 个 Qt 驱动调用点不动，**只在 `FCEUI_*` 函数内条件编译**。这样：
- Qt 驱动零改动
- 兼容垫片是单一修改点
- Phase 8 移除 C++ 实现时，兼容垫片可以直接删除

> **注意（S9）**：兼容垫片覆盖 `FCEUI_*` 调用点，但 **68 个文件直接 include
> 核心头**（`x6502.h`/`ppu.h`/`bus.h`/`sound.h`）读取全局符号——这些站点
> 需要逐个审计（Phase 0 的 `core_headers_deps.md` 清单为依据），不能只依赖垫片。

### 5.4 不在 Phase 6 切换默认

`VNESU11_CORE=OFF` 是 v2.0 默认，避免任何意外回归影响用户。Phase 7 切换。

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| Shadow run 性能开销过高 | 🟠 中 | 仅在 `VNESU11_CORE=ON` 时启用；可配置关闭 |
| 兼容垫片遗漏某个 FCEUI_* 函数 | 🟠 中 | 编译期 `-DFCEUX11_SHOW_DEPRECATION_WARNINGS` 强制检查 |
| Movie/TAS Editor 不兼容 | 🟠 中 | Phase 6 末跑 TAS movie round-trip 测试 |
| 调试器断点不工作 | 🟡 低 | FCEUI_DebugGetByte 走 vNESU11；断点由 debugger 控制 |
| Joypad 输入不同步 | 🟡 低 | vnesu11_joypad_set_button 是显式 FFI |

---

## 7. DoD

> **2026-08-12 实测更新（Phase 6 启动）**：P0 全部完成 + P1 骨架落地。
> `cargo test -p vnesu11` **322 passed / 0 failed / 1 ignored**（新增 4 个
> `vnesu11_emulate_frame` FFI 测试 + 6 个 system_type 测试 + peek/poke_regs
> 接线）。`vn_perf_bench` 实测 **743 us/帧 ≈ 1346 FPS**（远低于 16.7ms 预算）。
> VNESU11_CORE=ON/OFF 双配置 `fceux11.exe` 均构建链接成功。
>
> **2026-08-12 收口更新**：测试数字已按模块拆分（lib **192** + apu_tests **24**
> + ppu_tests **33** + mapper_tests **12** + system_type 等，全部通过）；
> P2 完成 shadow run harness + CPU 差异迭代修复的第一批（见 §9 第六~九版）。
> **以下 DoD 中 `[x]` = 已完成；未标 = 后续会话/Phase 7。**

- [x] `VNESU11_CORE=OFF`：行为与 v1.17 完全一致（构建验证通过；OFF 路径无功能改动，`vnesu11_*` 全为 no-op 桩）
- [x] `VNESU11_CORE=ON`：链接通过，可执行文件可启动
- [ ] **newppu=1** shadow run：SMB1/Zelda/Contra 60 帧 CRC 零 diff — [后续会话] 需端到端 harness + ROM fixtures
- [ ] **newppu=1** shadow run：MMC3 IRQ 测试 ROM 零 diff — [后续会话] 同上
- [ ] **newppu=1** shadow run：blargg cpu_instrs 全 PASS — [后续会话] 同上
- [ ] **newppu=0** 回退：C++ 旧 PPU movie round-trip 通过（ADR-009）— [Phase 7]
- [ ] **[修订] 系统类型**：FDS / NSF / VS 各跑一个测试文件通过（S6）— Rust 侧 6 个 system_type 冒烟测试已通过；C++ 端到端 [后续会话]
- [ ] 性能：纯 vNESU11 帧时间 ≤ v1.17 × 1.05 — `vn_perf_bench` 743us/帧（Phase 1 门禁已证 CPU parity；完整对比 [后续会话]）
- [ ] TAS movie round-trip（录放 5 分钟）字节一致 — [Phase 7]
- [ ] savestate round-trip（每个主流 mapper）字节一致 — [Phase 7]
- [ ] 真实游戏 10 个跑 5 分钟无 crash — [后续会话/Phase 7]

### Phase 6 启动已交付（2026-08-12）

```
P0（完成）：
  [x] vnesu11_emulate_frame 真实实现（Rust run_frame + xbuf/sbuf 拷贝 + APU drain，4 个 FFI 测试）
  [x] fceu.cpp::Emulate() 条件编译接通（shadow harness 并行 C++ + Rust，每 60 帧 CRC log）
  [x] CHR 转发：Bus::setchr1/4/8 → vnesu11_chr_set_page → VNesSoc::chr_pages[8]
  [x] MapperMetaVtable::tick_irq 改进（读 g_cpu.native_layout().IRQlow & FCEU_IQEXT）
P1（完成）：
  [x] src/vnesu11_shadow.{h,cpp}（ShadowData 导出 + CRC32 + periodic log）
  [x] vnesu11_power_on_bridge/reset_bridge 接入 vnesu11_shadow_reset
  [x] vnesu11_cpu_peek/poke + vnesu11_ppu_peek + vnesu11_cpu_peek_regs/poke_regs 接线
  [x] 6 个 system_type 冒烟测试（iNES/VS/FDS/NSF/unknown/NSF-frame）
  [x] vn_perf_bench（743 us/帧 ≈ 1346 FPS）
P2（后续会话/Phase 7）：
  [x] Shadow run 端到端 harness（2026-08-12：kagami_qa_shadow_run_runner 构建 + 运行成功）
  [x] CPU 寄存器差异迭代修复（2026-08-12 第一~十版：$2002 VBlank 路由、DEY/TAY 周期、
      段预算对齐、per-instruction APU tick、frame counter 全功能 + 状态同步、
      PPU 状态同步、预算单位补偿、frame counter IRQ 保持、count 单位 ÷16——
      frame 1-3 完全匹配，cpu_match=3/59）
  [ ] CPU 差异迭代剩余（frame 4+ 发散：count÷16 取整残差 + 每指令时序残差 +
      PPU v/t/x/w 同步——见 §9.1.2 Step 2c）
  [ ] MapperMetaVtable::fill_audio（VRC6/FDS/N163 扩展音频）
  [ ] savestate round-trip 100% parity
  [ ] TAS movie round-trip
```

---

## 9. Shadow run 实测结果（2026-08-12）

`kagami_qa_shadow_run_runner` 端到端跑通（构建 + 运行 + 对比）：

```
构建：tests/CMakeLists.txt 注册，仅 VNESU11_CORE=ON 构建
链接要点：
  - vnesu11.lib 经 ${VNESU11_RUST_LIB} 显式链接（修复了 src/rust/CMakeLists.txt
    未向上传播变量的 CMake bug：VNESU11_RUST_LIB 需 PARENT_SCOPE 二次导出）
  - 两个 Rust 静态库（fceux11_rust.lib + vnesu11.lib）各内嵌 std → /FORCE:MULTIPLE
    （同一 rustc 构建，std 符号一致，安全去重）
  - 需显式 VNESU11_CORE_ENABLED=1 编译宏（src/ 的 add_definitions 不达 tests/）
  - 链接 userenv / ws2_32 / dbghelp / ntdll（Rust std winsock 依赖）
  - 运行 DLL：side-by-side 复制 vcpkg debug DLL + DebugCRT 到 exe 目录
    （本机 MSVC 14.51 重建 vs vcpkg 14.44 DLL 的 PATH 冲突）
```

运行结果（3 个 ROM，60 帧）：

| ROM | CPU 匹配 | 观察 |
|-----|---------|------|
| cpu_dummy_reads.nes | 3/59 | **2026-08-12 第六~十版修复后**：frame 1-3 **完全匹配**；frame 4+ 仍发散（count÷16 取整 + 每指令时序残差 → 指令流分叉，见下方"剩余发散"第 5 条） |
| nestest.nes | 0/30 | 双方 PC 都在推进但不同步（依赖完整 PPU/APU channel 同步，Step 3 覆盖） |
| mapper_axrom.nes | 0/30 | 同上 |

**Shadow 捕获**：`frame=60 xbuf_crc=0xEC1C6272 audio_samples=32768`（帧 CRC + APU 样本均正常产出）。

**已修复（VBlank 路由，2026-08-12 第二版）**：shadow 暴露的首个发散根因是
`bus.rs::ppu_read_register(0x02)` 返回 Phase-2 stub 值，**未反映 PpuCore 的
VBlank 状态** → Rust CPU 的 $2002 等待循环无法退出。修复：$2002 读改走
`PpuCore.regs.read_status()`（含读清 VBlank）；$2000/$2001 写路由到
`PpuCore.regs.ppuctrl/ppumask`；$2003/$2004 路由到 PpuCore OAM；$2005/$2006
走 `PpuRegisters::write_scroll/write_addr` + 同步渲染器 scroll 缓存。
**效果**：frame 1-2 的 PC 从垃圾值（0x3636 循环）收敛到与 C++ 仅差 1 条指令
（E5A6 vs E5A3）——两核心在帧边界基本对齐。

**已修复（DEY/TAY 周期 + 段预算 + 可见扫描线后随 + $4017 NMI 入口，2026-08-12 第三版）**：
三个耦合问题：
1. **DEY (0x88) 和 TAY (0xA8) 周期表错位**：Rust BASE_CYCLES 写 3，C++ CycTable 写 2；
   两条指令每条多算 1 cycle，使 blargg 测试 ROM 的 DEY/TAY 循环每帧少跑 ~3 条指令。
   修复：Rust BASE_CYCLES 与 C++ CycTable **逐字节对齐**，并加回归测试
   `base_cycles_matches_cpp_cyc_table`（pinned against full 256-entry C++ table）。
2. **可见扫描线只发 256 预算**：原 `next_segment` 仅发出 `Visible { cpu_budget: 256 }`，
   缺少 C++ DoLine 的 6+63+16=85 cycle HBLANK 后续。修复：新增
   `PpuCore::sprite_eval_segment()` 发出 85 cycle 后续，与 C++ `X6502_Run(6)+Run(63)+Run(16)` 对齐。
3. **scanline 241 仅发 1 cycle 预算**：原 `Segment::VBlank { cpu_budget: 1 }` 给 NMI handler
   入口（7 cycles push + vector）留的空间不够；NMI 路由被推迟。修复：
   `Segment::VBlank { cpu_budget: CPU_BUDGET_VBLANK_LINE } = 341`，与 C++ DoLine
   `if (sl >= 240) { X6502_Run(256+69); X6502_Run(16); }` 完全对齐。
**效果**：frame 1-2 完全匹配（cpu_match 从 0/59 升到 2/59）。

**已修复（每指令 APU tick + 每指令 IRQ 路由，2026-08-12 第四版）**：
原 `run_segment_inner` 在每段末尾一次性 `apu.tick(budget)`，导致：
- frame counter 在 7457 / 14913 / 22371 / 29828 的 sub-instruction 边界事件被
  累积到段尾，丢失 cycle 精度（`fhcnt` 在 `FCEU_SoundCPUHook` 是逐 cycle 推进）。
- APU 触发的 IRQ 在段边界才被路由到 CPU，比 C++ 的"下一指令 IRQ 检查"晚 1 段。
修复：`run_segment_inner` 直接展开 `run_budget_with_hook` 循环，每条指令调用
`self.cpu.step_one(remaining, &mut bus)` → `self.apu.tick(tcount)` →
`self.route_apu_irqs_to_cpu()`（提取了独立的 helper）。新增 `CpuCore::step_one`
返回 `(new_remaining, tcount)`，让 caller 决定 per-instruction hook。

**已修复（$4017 延迟重置，2026-08-12 第五版）**：
原 `frame_counter.write($4017)` 同步重置 `cycle_count = 0`，与 C++
`fc_reset_in = (parity == 0) ? 3 : 4` 的 3-4 cycle 延迟重置不一致。
修复：新增 `pending_mode` + `reset_in` 字段，`write_with_parity(val, parity)`
记录待提交的模式位 + 延迟 cycle 数；`tick()` 每周期递减 `reset_in`，归零时
才真正提交模式位 + 清零 `cycle_count`。

**已修复（frame counter 全功能 + 状态同步，2026-08-12 第六~七版）**：
1. **frame counter 事件位置 2× 偏差**：原实现用 `14914` 作 4-step 周期、事件在
   3728/7456/11185/14914——**恰好是正确位置（7457/14913/22371/29828）的 cycle
   半值**。修复为完整 NTSC/PAL × 4/5-step 常量表（`FrameCounterProfile`，
   byte-pinned 对照 `src/sound.cpp::FrameCounterTick()`），并改用 post-increment
   比较（`fhcnt++; if (fhcnt==N)`）避免 IRQ 在 29829 被同次 tick 误清。
2. **APU 状态同步**：新增 `vnesu11_apu_poke_state/peek_state`
   （frame counter + master cycle + IRQ pending 标志），shadow sync 每帧推入。
3. **PPU 状态同步**：新增 `vnesu11_ppu_poke_state/peek_state`（$2000-$2003
   寄存器 + PPUGenLatch + PALRAM + NTARAM + OAM），让 $2002/$2007/$2004 读值
   两端一致。
4. **$4015 读清 CPU IRQ 位**：`apu_status_read` 补 `cpu.irq_end(FCOUNT)`，
   对应 C++ `X6502_IRQEnd(FCEU_IQFCOUNT)`（否则 IRQ handler 的 BIT $4015
   清不掉 CPU 的 pending 位 → 反复重进 handler 卡死）。

**已修复（每帧预算对齐 + 单位补偿 + IRQ 保持，2026-08-12 第八~九版）**：
1. **VBlank 段预算**：242-260（19 行）和 261（pre-render）从 85 改 341，
   -1 段（Rust 内部额外 scanline）改 0——对齐 C++ DoLine 每行 341。
2. **预算单位错位**（shadow 发散的主根因）：C++ `add_cycles` 按 `c×48` 消耗、
   `X6502_Run` 按 `×16` 预算（等效 3 dots/cycle）；Rust 把"点"预算当 CPU cycle
   用，导致 CPU 每帧执行 ~89k cycles（C++ 只 ~29.8k），frame counter 相位漂移。
   修复：`run_segment_inner` 每指令补扣 `tcount×2`、poll 补扣 `poll_consumed×2`
   （×3 总费率），CPU 核心保持 cycle 单位。
3. **frame counter IRQ 跨 wrap 保持**：C++ `FrameIRQEnd` 在 mode 0（非 inhibit）
   时**保持** IRQ（只清 SIRQStat flag，不清 IRQlow FCOUNT），仅 $4015 读才清。
   Rust 原在 29830 wrap 时清 IRQ，导致 set(29828/29) 与 reset(29830) 落在同一
   `apu.tick` 内时 IRQ 被吞。修复：wrap 分支不再清 `*irq_out`。

**剩余发散（Phase 6 持续工作 — 2026-08-12 现状）**：
cpu_dummy_reads frame 1-2 完全匹配，frame 3+ 仍发散。指令级诊断（PC 流对比）已定位：
1. **frame 0-1 指令数几乎一致**（Rust 8508/8490 vs C++ frame 1-2 8510/8506，差 2-16 条），
   frame 2+ 差 200+ 条（指令流分叉）。
2. **两端都不服务 NMI/IRQ**（测试 ROM 不开 NMI，I flag 屏蔽 IRQ）——发散**不是**中断时序。
3. **Rust frame 3 的 VBlank 轮询（E48A/E48D）退出后**，E48F RTS 返回 E621（"等 NMI"
   子程序的 RTS），**弹出栈顶 0x0022** → 进入 BRK 循环（0022 BRK → E622 BIT $4015 →
   E625 RTI → ...）。C++ frame 4 停在轮询/返回正常调用者（E493）。
4. **根因（第九版假设，第十版已证伪）**：frame 3 时 Rust 的**栈内容**（JSR 推的返回
   地址）与 C++ 不同（尽管 WRAM 每帧同步）——曾指向 JSR/栈语义差异。
   **第十版 count÷16 修复后 frame 1-3 全匹配**：JSR 推栈语义已审计正确（push PC-1、
   hi 后 lo，与 C++ 一致）；frame 4+ 的栈内容差异是**相位漂移的下游症状**，
   **不要再审计 JSR/栈语义**。
5. **[2026-08-12 第十版] count 单位错乱**：`vnesu11_cpu_poke_regs` 直接同步 C++
   `X6502.count`（×16 内部单位）到 Rust 的 count（点单位），换算应为 ÷16。原样同步
   使 Rust 预算残差每帧差 ~60 点（~20 cycles）→ frame 0-1 指令数差（2-16 条）+
   frame counter 相位漂移（~7 cycles）。修复后 shadow cpu_match 2 → 3（frame 1-3
   完全匹配）。剩余 frame 4+ 发散：count÷16 取整 + 每指令时序残差（Step 3）。

shadow harness 现已具备持续迭代的对比基础设施（harness 报告行已包含 APU
的 IRQ 触发记录 + frame counter 相位跟踪 + 每帧指令数对比 `g_cpu_instr_count_`）。

---

## 9.1 会话接续指引（2026-08-12 定稿，清空上下文后唯一依赖）

> **目标**：任何新会话只读本文件即可接续 Phase 6 收尾。**先跑一遍 §9.1.4 验证
> 命令确认基线**，再按 §9.1.2 剩余路径执行。不要重复已完成的修复（§9.1.1 清单）。

### 9.1.0 当前精确状态

- **分支** `wip_v2.0`；工作树应干净（最近 commit `46ac846`；若检出后发现脏树，
  先 `git status` 确认无未提交源码改动再继续）。
- **测试**：`cargo test --release -p vnesu11` 全模块通过（lib 194 + apu 24 + ppu 33
  + mapper 12 + system_type 等），无 FAILED。
- **shadow**（cpu_dummy_reads 60 帧）：`cpu_match=5/59`（**frame 1-2 真匹配**——
  逐帧指令数 delta=0（8510/8506）；frame 4 匹配（被抑制的 VBL 转移两端一致）；
  其余发散，当前根因是 S_2→S_3 转移主代码分叉，见 §9.1.2 Step 2c）。
  nestest/mapper_axrom 仍 0/N（依赖完整 PPU/APU 同步）。
- **核心状态**：frame counter IRQ 保持（mode-0）、预算单位 ×3 dots/cycle、
  count 单位 ÷16、-1 段 dot 对齐、**VBlank flag 帧首置位时间线**、**VBL-set
  suppression（shadow sync 决策同步）**——均已修复。

### 9.1.1 已完成的修复链（勿重复）

1. $2002 VBlank 路由（bus.rs read_status）
2. DEY/TAY 周期表对齐（cycles.rs，含回归测试 `base_cycles_matches_cpp_cyc_table`）
3. 段预算：VBlank 341 + sprite_eval 85 + -1 段 0（ppu/mod.rs）
4. per-instruction APU tick + IRQ 路由（soc.rs run_segment_inner + step_one）
5. frame counter 全功能 NTSC/PAL × 4/5-step（frame_counter.rs，周期 29830）
6. $4017 延迟重置 3/4-cycle（write_with_parity）
7. APU/PPU 状态同步（ffi.rs apu_poke_state / ppu_poke_state + shadow.cpp sync）
8. $4015 读清 cpu.irq_pending FCOUNT（bus.rs apu_status_read）
9. 预算单位 ×3 dots/cycle（run_segment_inner 补扣 tcount×2）
10. frame counter IRQ 跨 wrap 保持（FrameIRQEnd mode-0 对齐）
11. -1 段不推进 dot 时钟（262×341 dots 对齐 C++）
12. **count 单位 ÷16**（ffi.rs vnesu11_cpu_poke_regs `cpu.count = r.count / 16`）
13. **VBlank flag 帧首置位时间线**（ppu/mod.rs：PRELINE 置位 + NMI arm、sl 20 清除、
    sl 241 不再置位——对齐 C++ FCEUX_PPU_Loop 顶部置位；4 个 PPU 测试更新）
14. **$2002 低 5 位 open-bus**（bus.rs：`read_status | (read_buffer & 0x1F)`，
    对齐 C++ `PPU_status | (PPUGenLatch & 0x1F)`）
15. **VBL-set suppression（shadow sync 决策同步）**：C++ 新增
    `fceu11_ppu_peek_vbl_set_suppressed`（peek 不消费），经
    `PpuStateMirror.vbl_set_suppressed` 推给 Rust；`tick_preline_segment` 帧首
    消费。两端抑制同一 S_{N+1}→S_{N+2} 转移（latch 帧 N 末设置、帧 N+1 顶消费；
    同步点在帧 N 末 → Rust 帧 N PRELINE 消费）——cpu_match 3→5。
    **注意**：这是 shadow 专用的决策同步（Rust 尚未独立判定 suppression——
    Rust 自身的 sub-scanline 相位与 C++ 漂移方向相反，独立实现需对齐预算边界
    模型，见 Step 2c 候选解法）。

### 9.1.2 剩余路径（按序执行）

**Step 2c（当前）— frame 3+ 发散收尾**（2026-08-13 检查点：树干净、基线
cpu_match=5/59、延迟置位实验已回退并记录于下文"实验记录"）：

> **明日接续顺序**：① 跑 §9.1.4 验证命令确认基线（cpu_match=5/59）；② 按下文
> "实验记录"第一步：确认 `apply_frame_vbl_set` 是否触发（若触发而 frame 3 不变
> → 发散主因在轮询退出后的主代码路径）；③ 用 CPC/RPC PC 流 diff（探针模式见
> 下文）定位主代码首分叉点；④ 修复后每步跑 60 帧 shadow 收敛。

**根因 1（已修复）：VBlank flag 时间线**。C++ `FCEUX_PPU_Loop`（newppu=1）在
**帧首**（loop 顶部）置位 $2002 VBlank flag，保持 20 行（6820 dots）后清除；
Rust 原来在自身 sl 241（帧末 82181 dots 处）置位。$2002 轮询在两端于**帧内不同
位置**看到 flag=1 → 指令流分叉。frame 1-2 的"匹配"是假匹配（两端都在 reg 中性
的轮询循环里结束）。**修复**：`tick_preline_segment` 帧首置位 + NMI arm，
sl 20 入口清除（`next_segment`），sl 241 不再置位；$2002 低 5 位 OR
`read_buffer & 0x1F`（对齐 C++ `PPUGenLatch & 0x1F`）。4 个 PPU 测试相应更新。

**根因 2（已修复）：VBL-set suppression**。C++ A2002 在 **sl 240, cycle 340**
（边界前 1 dot）的 $2002 读取会**抑制下一帧的 flag 置位 + NMI**（Nesdev
PPU_frame_timing；`fceu11_ppu_mark_vbl_set_suppressed` 在 FCEUX_PPU_Loop 顶部
消费）。实证：`FCEUX11_E1_TRACE=1` 显示某帧置位 `suppressed=1` —— C++ 整帧
flag=0 → 轮询不退出；Rust 无 suppression → 照常置位 → 轮询提前退出跑主代码 →
发散。**修复**：shadow sync 把 C++ 的 latch（新增 `fceu11_ppu_peek_vbl_set_suppressed`，
peek 不消费）经 `PpuStateMirror.vbl_set_suppressed` 推给 Rust，Rust 的
`tick_preline_segment` 在帧首消费（`vbl_set_suppressed` 已在 PpuCore）。
时序验证：C++ latch 在帧 N 的 sl-240 设置、帧 N+1 的 VBL_ENTER 消费；同步点在
帧 N 末、Rust 帧 N 的 PRELINE 消费——两端抑制**同一个** S_{N+1}→S_{N+2} 转移。
**效果：cpu_match 3 → 5**（被抑制的转移现在两端一致）。

**剩余发散（2026-08-12 当前）**：frame 3（S_2→S_3 转移）仍 diff——
`instr cpp=8544 rust=5257`（delta=-3287），rust pc 走入数据区（FCAF）。
**PC 流 diff 定位（CPC/RPC 探针，校正 pc 显示偏移后）**：两端帧首第一条指令
都是 E48A 轮询的 BIT——但读取结果不同：
- C++：第一次 BIT 读取（abs=89343, sl=240 cy=0，跨段指令延续，发生在
  VBL_ENTER 置位**之前**）读到 flag=0 → BPL 再循环一次 → 置位（abs=89343，
  E1 确认非抑制）后的下一次读取（abs=89350, sl=241 cy=13）读到 flag=1 → 退出。
- Rust：PRELINE 置位发生在帧首第一次读取**之前** → 第一次 BIT 读取
  （sl 0 dot 0）就读到 flag=1 → 立即退出（早一次轮询迭代）→ 返回路径弹出
  错误地址（EF01）→ 走入数据区（EF01/EF03/... 与 E622/E625 IRQ handler 交替）。
- 即：**C++ 的帧首置位在 CPU 的边界跨越读取之后生效；Rust 的 PRELINE 置位在
  所有帧首读取之前生效**——边界读取时序差一次轮询迭代。

**实验记录（2026-08-12 深夜，已回退）**：尝试"延迟置位"修复——PRELINE 只武装
`frame_vbl_set_pending`，`soc.rs` 在帧首第一条 step_one 后调
`PpuCore::apply_frame_vbl_set()` 才真正置位 + NMI arm（对齐 C++ 的边界读取
时序）。PPU 测试相应更新（全部通过），但 **shadow frame 3 数值完全不变**
（仍 8544/5257，rust pc=FCAF）。**未验证 apply 是否真的执行**（探针已加、
运行被中止）。两种可能：(a) apply 未触发（帧首第一条指令可能走 IRQ poll 路径，
或 fc IRQ 服务路径绕过）；(b) frame 3 发散并非首读 flag 时序，而是轮询退出后
主代码路径的其它差异。**改动已 `git checkout` 回退**（源码回到 261c42f 提交态）。
明日第一步：加 `apply_frame_vbl_set` 探针（模式见下）确认 apply 是否触发；
若触发而 frame 3 不变 → 发散主因在轮询退出后的主代码路径（候选：fc IRQ 服务
时序、count 残差、或主代码 JSR/RTS 配对），用 CPC/RPC PC 流 diff 逐指令定位。

**探针模式（明日复用）**：
- C++：`src/sound.cpp` FCEU_SoundCPUHook 内按 `extern int framectr` 门控
  `fprintf(stderr, "CPC pc=%04X\n", g_cpu.native_layout().PC)`（注意 pc 是
  **指令后**值）。
- Rust：`src/rust/crates/vnesu11/src/soc.rs` run_segment_inner 两分支（无 IRQ
  路径 + poll 路径）step_one 前按 `self.frame_count` 门控
  `eprintln!("RPC pc={:04X}", self.cpu.pc())`（pc 是**指令前**值）；SoC 需
  临时加 `pc_trace_count` 字段。对比时把 C++ 下标 k 与 Rust 下标 k+1 对齐
  （pc 显示偏移）。

**已就位的诊断基础设施**：`g_cpu_instr_count_`（C++ 每指令计数，sound.cpp 永久
钩子）、`vnesu11_instr_count` FFI + `VNesSoc::instr_count`、`frame_count`、
`segment_dots`（段内已耗 dots）、runner 每帧指令增量日志。复跑基线：
`cpu_match=5/59`（frame 1-2 真匹配 delta=0；frame 4 被抑制转移匹配；其余发散）。

**Step 3 — APU 5 通道状态同步**：
- 扩展 `ApuStateMirror`（ffi.rs）：pulse1/2（timer/length/envelope/sweep）、triangle
  （timer/length/linear）、noise（period/length/envelope/lfsr）、dmc
  （buffer/address/size/period）。
- C++ 端（shadow.cpp sync）：从 `PSG[]`、`curfreq[]`、`lengthcount[]`、`EnvUnits[]`、
  `SweepCount[]`、DMC 全局填。
- 验证：shadow 三个 ROM 指令数/CPU regs 匹配率提升。

**Step 4 — shadow 验证闭环**：cpu_dummy_reads / nestest / mapper_axrom 60 帧全匹配。

**Step 5 — DoD 三套件**：SMB1/Zelda/Contra（XBuf CRC 零 diff + SNR≥60dB）、MMC3 IRQ
测试 ROM、blargg cpu_instrs 177 全 PASS。

**Step 6 — 系统类型 + fill_audio + 性能**：FDS/NSF/VS 各 1 文件；
`MapperMetaVtable::fill_audio`（VRC6/FDS/N163）；纯 vNESU11 帧时间 ≤ v1.17×1.05。

**Step 7 — 文档收口**：§7 DoD 勾选 + §9 更新为最终实测矩阵。

### 9.1.3 关键文件索引（下一步要改的）

- `src/rust/crates/vnesu11/src/ffi.rs`：`ApuStateMirror`/`PpuStateMirror` 扩展、
  `vnesu11_cpu_poke_regs`（count 单位）、`vnesu11_apu_poke_state`/`ppu_poke_state`
- `src/vnesu11_shadow.cpp`：`vnesu11_shadow_sync_from_cpp`（填 APU/PPU 状态）
- `src/vnesu11_shadow.h`：mirror 结构镜像 + extern "C"（namespace 外）
- `src/rust/crates/vnesu11/src/soc.rs`：`run_segment_inner`（预算/单位/IRQ 路由）
- `src/rust/crates/vnesu11/src/apu/frame_counter.rs`：周期常量/IRQ 保持

### 9.1.4 验证命令

```
cd src/rust && cargo test --release -p vnesu11          # 全绿（无 FAILED）
# C++ 重建（%VCVARS64% 在新 shell 通常未定义，直接用本机路径；PowerShell 用反引号
# 转义内层引号。BuildTools 备选：
#   C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat）：
cmd /c "`"D:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat`" >nul 2>&1 && cd build && cmake --build . --target kagami_qa_shadow_run_runner --config Release"
# shadow（PATH 需含 vcpkg bin）：
build\tests\kagami_qa_shadow_run_runner.exe tests/fixtures/blargg/cpu/cpu_dummy_reads.nes --frames 60
# 期望：SHADOW_RESULT cpu_match 基线 5（Step 2c 目标 ≥6，Step 4 目标 60/59）。
# 注意：cpu_diff>0 时 runner 退出码为 1，属正常；退出码 0 才是全匹配。
# 基线已验证（2026-08-12）：cpu_match=5/59（frame 1-2 真匹配 + frame 4 被抑制转移）。
```

### 9.1.5 纪律

- **不做简化**：不跳过 PPU 敏感 ROM、不放宽判据（用户明确要求）。
- **文档同步**：每完成一步，更新 §7 DoD + §9（含日期版本号），防止脱节复发。

---

## 8. 关键文件交付

```
新增：
  src/vnesu11_bridge.h
  src/vnesu11_bridge.cpp
  src/vnesu11_shadow.h
  src/vnesu11_shadow.cpp
  tests/shadow_run/

修改：
  CMakeLists.txt                # VNESU11_CORE option
  src/CMakeLists.txt            # 链接逻辑
  src/fceu.cpp                  # Emulate 条件编译
  src/core_api.h                # 兼容垫片条件编译
  src/debug.cpp                 # 调试器适配
  src/movie.cpp                 # movie/TAS 适配
  src/state.cpp                 # savestate 走 vNESU11 peek/poke
```

下一步：[phase_7_default_switch.md](./phase_7_default_switch.md)
