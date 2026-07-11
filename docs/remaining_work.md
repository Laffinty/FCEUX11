# FCEUX11 遗留项清单

> **生成日期**：2026-07-11（v1.14 Anvil 收官后）
> **来源**：`docs/v1.x_Modernization_Roadmap.md` 中所有未勾选 `- [ ]` 项
> **目的**：为后续版本开发和社区贡献提供明确的待办指引

---

## 总览

| 类别 | 项数 | 优先级 | 说明 |
|------|------|--------|------|
| A. 文档债务 | ~28 | 低 | 代码已完成，roadmap checkbox 未同步 |
| B. v1.8 Masonry 未实现 | ~15 | **高** | 唯一未交付的代码版本 |
| C. 已完成版本的推迟项 | 2 | 中 | v1.6 遗留的子类化和 Rust 滤波器路径 |
| D. v1.14 运行时验证 | 2 | 中 | 需专用硬件环境 |

---

## A. 文档债务（§3 v1.3 Legion + §4 v1.4 Gateway）

以下 checkbox 未勾选，但对应代码**已实现并交付**。原因是 roadmap
在这些版本交付时未同步更新。建议一次性勾选。

### A.1 §3 v1.3 Legion — CPU 状态对象化（roadmap 行 169-242）

代码已在 v1.3 交付（commit 见 CHANGELOG v1.3 段），以下为实际状态：

| 行 | 内容 | 实际状态 |
|----|------|----------|
| 169 | 新建 `src/cpu.h` / `src/cpu.cpp` | ✅ 已存在 |
| 170 | `fceu11::Cpu` 类 | ✅ `src/cpu.h` 定义，`alignas(64)` |
| 209 | `static_assert(offsetof(Cpu, layout_) == 0)` | ✅ `src/cpu.cpp:14` |
| 210 | `alignas(64)` 缓存行对齐 | ✅ `src/cpu.h:35` |
| 211 | 补全寄存器访问器 `db()` / `pi()` | ✅ 已实现（v1.4 后端审查补遗） |
| 217 | 全局迁入 Cpu 类成员 | ✅ `timestamp_`, `sound_timestamp_`, `scanline_`, `map_irq_hook_`, `overclocking_` |
| 223 | `timestampbase` 集中管理 | ✅ `fceu11_core_types.h` |
| 224 | 保留 inline 引用别名 | ✅ `x6502.h` 5 个别名 + `cpu.h` g_cpu |
| 228 | 逐文件迁移调用点 | ✅ 已完成 |
| 232 | `X6502_Run` 接受 `Cpu&` 参数 | ✅ `X6502_RunDebug(fceu11::Cpu&, int32)` |
| 233 | `ADDCYC(x)` → `cpu.add_cycles(x)` | ✅ `x6502.cpp:54` |
| 234 | 兼容宏保留 | ✅ `x6502.h:57` |
| 238-242 | 验收标准 | ✅ ctest 通过，savestate 兼容 |

### A.2 §4 v1.4 Gateway — 内存分发总线重构（roadmap 行 252-332）

代码已在 v1.4 交付：

| 行 | 内容 | 实际状态 |
|----|------|----------|
| 252 | 新建 `src/bus.h` / `src/bus.cpp` | ✅ 已存在 |
| 253 | `fceu11::Bus` 类 | ✅ `src/bus.h` 定义，`__forceinline` read/write |
| 313 | 全局 ARead/BWrite 引用 | ✅ `bus.cpp:408-433` |
| 314 | SetReadHandler/SetWriteHandler 转发 | ✅ 已实现 |
| 315 | setprg*/setchr*/setmirror 转发 | ✅ 已实现 |
| 316 | board 文件迁移 | ✅ 171 个文件已迁移 |
| 317 | Bus::init() 接入 PowerNES | ✅ 已实现 |
| 321-325 | 分批迁移 board 文件 | ✅ 已完成 |
| 329-332 | 验收标准 | ✅ ctest 通过 |

---

## B. v1.8 Masonry — 未实现版本（roadmap 行 709-758）

**这是唯一未交付的代码版本**。v1.8 Masonry 的目标是清理 board/ 目录架构。
以下为全部待办项：

### B.1 mapinc.h 拆分（§8.1）

- [ ] 将 `mapinc.h` 拆分为按需包含的子头文件：
  ```
  src/boards/mapinc_base.h   — 仅 <cstdint> + 宏定义
  src/boards/mapinc_bus.h    — Bus 访问（setprg*/setchr*/setmirror*）
  src/boards/mapinc_state.h  — Savestate 注册宏
  src/boards/mapinc_mmc3.h   — MMC3 衍生 mapper 共享状态（原 mmc3.h）
  ```
- [ ] 逐文件更新 `#include "mapinc.h"` → 按需包含
- [ ] 优先处理最常用 mapper（mmc3.cpp 等）

**影响范围**：`src/boards/` 下 171 个 .cpp 文件

### B.2 Mapper 子类化批量迁移（§8.2）

按 v1.7 定义的 `fceu11::Mapper` 基类分批迁移：

- [ ] 批次 1：标准 mapper（0-4: NROM, MMC1, UNROM/CNROM, MMC3）
- [ ] 批次 2：常用 mapper（5-9: MMC5, FFE, ANROM, PPU 钩子类）
- [ ] 批次 3：VRC 系列（vrc2and4, vrc6, vrc7）
- [ ] 批次 4：Namco/Sunsoft/其他日系 mapper
- [ ] 批次 5：冷门 mapper（coolgirl 等）
- [ ] 每批次完成后运行对应 mapper 回归测试

**注意**：v1.7 已交付 NROM (0)、MMC1 (1)、MMC3 (4)、VRC6 (24/26) 的 PoC
Cart 子类（commit `881d5f4` / `c580b72`），可作为后续批次的参考模板。

### B.3 FceuMallocPtr RAII 全量采纳（§8.3）

- [ ] 审计所有 board 文件中的 `malloc()/free()` 调用
- [ ] 统一替换为 `FceuMallocPtr` RAII 持有者
- [ ] `FCEU_amalloc` 调用点标记 `[[nodiscard]]`
- [ ] 目标：board/ 目录下零裸 `malloc()/free()` 调用

**注意**：v1.13 Purify 已完成核心文件（src/ 根目录）和 Qt 驱动文件的
malloc/free 清理。board/ 目录是最后的阵地。

### B.4 Board 注册表（§8.4）

- [ ] 新建 `src/boards/registry.h` / `src/boards/registry.cpp`
- [ ] 自动注册机制替代手动 `FCEU_*Mapper` 函数指针注册：
  ```cpp
  namespace fceu11 {
  struct MapperEntry {
      uint32_t mapper_number;
      const char* name;
      std::function<std::unique_ptr<Mapper>(Bus&)> factory;
  };
  const MapperEntry* find_mapper(uint32_t number);
  }
  ```
- [ ] 保留旧注册路径的兼容层

### B.5 验收标准（§8.5）

- [ ] `mapinc.h` 已拆分，不再无条件包含 12 个头文件
- [ ] 批次 1~3 mapper 子类化完成（覆盖 > 80% 常见 ROM）
- [ ] board/ 目录零裸 `malloc()/free()`
- [ ] Mapper 注册表可用，按编号查找 mapper
- [ ] 全部回归测试通过

---

## C. 已完成版本的推迟项

### C.1 v1.6 Resonance — 扩展音频子类化（roadmap 行 570）

- [ ] VRC7 / FDS / MMC5 / Namco163 / Sunsoft5B 子类化推迟到
  **v1.8 Masonry**（避免与 §6.4 cart-class 改动冲突）

**说明**：v1.6 已交付 VRC6 子类化 PoC（`fceu11::Vrc6Audio`），
其余扩展音频的 `ExpansionAudio` 子类化依赖 v1.8 的 Cart/Mapper
架构现代化，需与 B.2 一并推进。

### C.2 v1.6 Resonance — Rust 滤波器路径（roadmap 行 584）

- [ ] `Apu::flush_emulate_sound()` 调用 Rust 滤波器：当前走的是
  `filter.cpp` 自由函数路径，未通过 `g_apu` 成员方法。虚路径激活
  只在 `GameExpSound.expansion != nullptr` 时生效（VRC6 PoC +
  v1.8 后续子类），标准 APU 仍走原函数指针链路 — 这是 §1.4 性能
  预算的设计选择（避免每次 flush 走 Apu 间接调用）

**说明**：这是有意的架构决策，非遗留 bug。仅在 v1.8 完成扩展音频
子类化后，才需要评估是否将标准 APU 路径也走 Apu 成员方法。

---

## D. v1.14 Anvil 运行时验证（roadmap 行 1234-1235）

- [ ] 在与 v1.0 基线相同的硬件/配置下运行 `bench_cpu_frame`,
  `bench_ppu_frame`, `bench_full_frame`
- [ ] 对比 `baseline_v1.0.json`，任何退步 > 2% 必须修复

**执行条件**：
- 需要 Windows Server 2022, MSVC 14.44, x64, Release 配置
  （与 `baseline_v1.0.json` 记录的环境一致）
- 顺序执行（非并行，避免 CPU 竞争），各跑 3 轮取 best-of-3
- 方法详见 roadmap §10.6.6 "性能验证方法"

**当前状态**：`bench_tolerance_test` 已通过（对比工作基线
`bench_baseline.json`），但与 v1.0 原始基线的对比尚未在专用环境执行。

---

## 建议的执行顺序

1. **先修文档债务**（A 类）— 一次性勾选 §3/§4 的 checkbox，工作量最小
2. **v1.8 Masonry**（B 类）— 唯一的代码版本遗留，建议按 B.1 → B.3 → B.2 → B.4 顺序
3. **运行时验证**（D 类）— 在 CI 或专用 runner 上配置基准回归流程
4. **扩展音频子类化**（C 类）— 依赖 v1.8，可在 v1.8 完成后并行推进

---

## 附：v1.x 版本交付状态

| 版本 | 代号 | 状态 | 最终 commit |
|------|------|------|-------------|
| v1.1 | Sentinel | ✅ | — |
| v1.2 | Census | ✅ | — |
| v1.3 | Legion | ✅ (checkbox 未更新) | — |
| v1.4 | Gateway | ✅ (checkbox 未更新) | — |
| v1.5 | Prism | ✅ | `c47fa4e` |
| v1.6 | Resonance | ✅ | `d9879a9` |
| v1.7 | Cartograph | ✅ | `c580b72` |
| **v1.8** | **Masonry** | **❌ 未交付** | — |
| v1.9 | Chronicle | ✅ | — |
| v1.10 | Cryptex | ✅ | `07f0126` |
| v1.11 | Bridge | ✅ | — |
| v1.12 | Scissors | ✅ | — |
| v1.13 | Purify | ✅ | tag `v1.13` |
| v1.14 | Anvil | ✅ | `779d12d` |
