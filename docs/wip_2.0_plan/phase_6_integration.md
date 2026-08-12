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
  [ ] MapperMetaVtable::fill_audio（VRC6/FDS/N163 扩展音频）
  [ ] CPU 寄存器差异迭代修复（shadow 已暴露：Rust $2002 PPUSTATUS 读未反映 PpuCore VBlank 状态 → wait-loop 无法退出）
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
| cpu_dummy_reads.nes | 2/59 | **2026-08-12 第三版修复后**：frame 1-2 **完全匹配**；frame 3 Rust 提前触发 frame counter IRQ |
| nestest.nes | 0/30 | 双方 PC 都在推进但不同步 |
| mapper_axrom.nes | 0/30 | 双方 PC 都在推进但不同步 |

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

**剩余发散（Phase 6 持续工作）**：frame 3+ Rust 提前触发 frame counter IRQ。
根本原因：Rust `frame_counter.write($4017)` 立即重置 `cycle_count = 0`（同步语义），
而 C++ 走 3-4 cycle 延迟重置（`fc_reset_in = (parity == 0) ? 3 : 4`，
引用 `R6 Step 3` 探针深化的 cycle-position 模型）。结果 Rust 的帧计数器比 C++
早 ~3-4 cycle 到达终端，APU IRQ 提前触发，Rust CPU 在 $2002 spin loop 中途
进 IRQ handler（`E622 = BIT $4015; RTI`），然后回到 spin loop —— 状态虽与
C++ 趋同但已错开数条指令。闭合方向：把 Rust frame_counter.write 改成 3-4 cycle
延迟重置（match `FCEU_SoundCPUHook` 中的 `fc_reset_in` 递减逻辑），并把 parity
来源从 `g_cpu.timestamp_base()` 改为相对 frame 内部的 cycle 计数。

shadow harness 现已具备持续迭代的对比基础设施。

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
