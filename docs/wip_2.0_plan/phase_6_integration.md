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

- [ ] `VNESU11_CORE=OFF`：行为与 v1.17 完全一致
- [ ] `VNESU11_CORE=ON`：链接通过，可执行文件可启动
- [ ] **newppu=1** shadow run：SMB1/Zelda/Contra 60 帧 CRC 零 diff
- [ ] **newppu=1** shadow run：MMC3 IRQ 测试 ROM 零 diff
- [ ] **newppu=1** shadow run：blargg cpu_instrs 全 PASS
- [ ] **newppu=0** 回退：C++ 旧 PPU movie round-trip 通过（ADR-009）
- [ ] **[修订] 系统类型**：FDS / NSF / VS 各跑一个测试文件通过（S6）
- [ ] 性能：纯 vNESU11 帧时间 ≤ v1.17 × 1.05
- [ ] TAS movie round-trip（录放 5 分钟）字节一致
- [ ] savestate round-trip（每个主流 mapper）字节一致
- [ ] 真实游戏 10 个跑 5 分钟无 crash

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
