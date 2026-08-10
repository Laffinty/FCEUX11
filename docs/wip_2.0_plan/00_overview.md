# 00 · 项目概述

## 1. 项目目标

将 FCEUX11 当前分散在 C++ 端的 **CPU 解释器**（`x6502.cpp` + `cpu.cpp`，约 955 行）与 **PPU 核心**（`ppu*.cpp`，约 3 814 行）合并迁移到一个统一的 Rust crate **`vnesu11`** 中，作为 **虚拟 SoC** 抽象运行。

### 1.1 一句话目标

> **把"两个独立 C++ 模块 + 跨 TU 全局状态"换成"一个 Rust 单一所有权 SoC + 窄 FFI 边界"，让 LLVM 拿到跨模块内联的物理寄存器分配权。**

### 1.2 三个具体目标

1. **架构统一**：CPU/PPU/APU/DMA/IRQ/Clock 收归 `VNesSoc` 单结构体；mapper 留在 C++ 走 `*mut c_void` + vtable 适配
2. **性能收益**：消除 C++ 端跨 TU 间接调用（`ARead[]` 函数指针表、跨头文件的 inline 失败），预期 5-15% hot path 提速（见 `A_performance_model.md`）
3. **兼容性冻结**：savestate 二进制格式不变（`x6502struct.h` 64 字节布局）、~250 个 mapper 不重写、Qt/Lua/TAS 链路不破坏

---

## 2. 范围

### 2.1 ✅ 在范围内

| 类别 | 内容 |
|------|------|
| **新代码** | `crates/vnesu11/` 全套实现 |
| **FFI 表面** | `vnesu11_create/destroy/power_on/reset/emulate_frame/cpu_peek/cpu_poke/...` |
| **Mapper 适配** | **[修订]** `SetReadHandler`/`SetWriteHandler` 转发（per-range 注册，非 vtable），见 `AUDIT_20260810.md` S4 |
| **构建系统** | `src/rust/Cargo.toml` 加 `vnesu11` member；`src/rust/CMakeLists.txt` 加 build target |
| **CMake 选项** | `option(VNESU11_CORE "Use vNESU11 Rust core" OFF)`（Phase 6）→ ON（Phase 7） |
| **测试** | 现有 kagami-qa 双 Oracle 不动；新增 `vnesu11` 子 crate 单测 + shadow run harness |
| **文档** | 本目录全部文件；CHANGELOG/BuildGuide 同步更新 |
| **系统类型** | **[修订]** iNES + FDS + NSF + VS 四类（共用同一核心），见 §2.4 |

### 2.2 ❌ 不在范围内

| 类别 | 内容 | 原因 |
|------|------|------|
| **重写 mapper** | ~250 个 C++ mapper | 工程量爆炸，价值低（mapper 不在 hot path） |
| **dynarec / JIT** | 动态重编译 6502 | 当前 C++ 解释器性能足够；dynarec 复杂度 > 收益（见 ADR-006） |
| **改时序模型** | **[修订]** 维持 scanline-budget 模型（决策 A，ADR-008）；dot 级重写（决策 B）单独立项 | 审计发现现状是 budget 模型，改动 = 行为重写（S3） |
| **旧 PPU 迁移** | **[修订]** `newppu=0` 的旧 PPU 路径**不迁移**，保留为 movie 兼容回退 | 审计发现双 PPU + movie 记录 PPUflag（S5，ADR-009） |
| **新 mapper 模型** | 改 mapper IRQ 时序 | 同上 |
| **跨平台** | 把 Rust 核心移植到 Linux/macOS | 项目锁 MSVC（见 `CMakeLists.txt:28-37`） |
| **替代 Qt 驱动** | 用 egui/iced 重写 GUI | 工程量 5-10x 本项目，纯属另立项目 |
| **加速器硬件** | AVX/NEON SIMD 优化 PPU | 当前性能不是瓶颈；引入会破坏 64 字节布局假设 |

### 2.3 🟡 视情况（Phase 8 决策点）

- **Lua 内存钩子重写**：当前用 thunk；如性能不足，迁移到 `Arc<LuaHook>` 直调
- **TAS Editor 兼容性**：可能需要小调整 Lua 内存钩子的 hook 时序精度
- **调试器符号表 API**：是否要 expose Rust 端给 debugger；现有 `fceulua.h` 兼容层可保留
- **[修订] 旧 PPU 最终去向**：v2.0 保留 C++ 旧 PPU；是否在 v2.1 也迁移到 Rust（双 PPU 都迁）是 v2.1 决策

### 2.4 [修订] 系统类型矩阵（S6）

vNESU11 替换 CPU/PPU 后，必须支持 FCEUX11 的全部四种系统类型（共用同一
`FCEUPPU_Loop → X6502_Run` 核心）：

| 系统类型 | 与普通 iNES 的差异 | vNESU11 支持方式 | 交付阶段 |
|---------|-------------------|-----------------|---------|
| **iNES** | 基准 | 默认路径 | 全程 |
| **FDS** | 磁盘 IRQ（`FCEU_IQEXT/EXT2`）+ 额外 RAM | `IrqController` 外部 IRQ 源 + FDS 虚拟 mapper | Phase 4 + Phase 5 |
| **NSF** | 仅 CPU+APU，PPU 空转 | `Scheduler` 的"无 PPU"模式（PPU 空转 stub） | Phase 3 |
| **VS UniSystem** | coin 输入 + 特定 PPU 行为 | `Joypad` 扩展 + PPU 行为开关 | Phase 4 |

验证：每类系统在对应 phase 的 DoD 中有测试 ROM 条目。

---

## 3. 成功标准（DoD）

每个 phase 完成时必须有可验证的 DoD。最终整体 DoD：

### 3.1 功能 DoD
- [ ] 177 个 blargg ROM 全绿（或与 C++ baseline 一致，零回归）
- [ ] 47 个 kagami-qa 清单条目 PASS 数 ≥ v1.17（39P）
- [ ] `mapper_byte_diff` 175-case parity 100%（与 C++ mapper 输出字节一致）
- [ ] savestate round-trip：MD5 与 v1.17 golden 完全一致
- [ ] TAS movie 录放 round-trip：MD5 与 v1.17 一致

### 3.2 性能 DoD（[修订] 见 AUDIT S8——预期降级为持平）

- [ ] blargg `cpu_instrs.nes` 在 Release 构建下跑完时间 ≤ v1.17 baseline × 1.05
- [ ] 真实游戏（如 SMB1、Zelda）跑 60 帧 / 帧 ≤ v1.17 × 1.05
- [ ] PPU 段渲染：每段 CPU cycles 不比 v1.17 多 5%

> **说明**：性能预期已从 5-15% 收益修正为「持平至 +3%，不排除 -2%」。
> 迁移的核心理由是架构统一 + 内存安全；性能是监控项，不是验收项。
> 详见 `A_performance_model.md`。

### 3.3 工程 DoD
- [ ] `cargo test -p vnesu11` 全绿
- [ ] `cargo clippy -p vnesu11` 无 warning（`-D warnings`）
- [ ] C++ 侧 `vnesu11_*` 调用点全部走兼容垫片，无直接字段访问
- [ ] CHANGELOG 增补 `[2.0.0]` 条目
- [ ] `docs/wip_2.0_plan/` 全部 phase 文件标记完成

---

## 4. 风险一览（详见 B_risk_register.md）

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| savestate 字节布局漂移 | 🔴 高 | **[修订]** SFORMAT tag 契约 + 逐 tag golden round-trip（S2） |
| Mapper 适配 thunk 调用频率失控 | 🟠 中 | **[修订]** per-range 区间表，benchmark 门禁（S4） |
| PPU 渲染性能回归 | 🟠 中 | shadow run 30 天逐步放量；性能预期已降级（S8） |
| Lua 内存钩子 hot-path 退化 | 🟡 低 | `AtomicBool` flag 守卫 |
| LTCG c2.dll 物化崩溃复发 | 🟠 中 | 大表拆子 crate（已验证：`fceux11-core` 无大常量） |
| MSVC 工具链依赖 | 🟡 低 | 已锁定；新依赖同样锁 x86_64-pc-windows-msvc |
| 工期 6.5 人月严重低估 | 🟠 中 | Phase 3（PPU）有 1.5 倍 buffer |
| **[修订] 时序模型决策风险** | 🔴 高 | 必须选 A（budget 复刻）；选 B（dot 重写）需单独立项（S3） |
| **[修订] newppu 双 PPU 兼容** | 🟠 中 | 旧 PPU 保留为 movie 回退（S5，ADR-009） |
| **[修订] NSF/FDS/VS 覆盖缺失** | 🟠 中 | 系统类型矩阵（S6，§2.4） |

---

## 5. 与 v1.x 系列的关系

| 版本 | 状态 | vNESU11 角色 |
|------|------|-------------|
| v1.13 Purify | released | Lua 引擎已迁 Rust（先例） |
| v1.14 Anvil | released | LTCG/PGO 优化 |
| v1.16 R4-fix | released | Rust 合并头 CI 修复 |
| v1.17 | released | KagamiQA 统合（QA 框架成熟） |
| **v2.0 (wip)** | **草案（已审计修订）** | **CPU+APU 迁 Rust；PPU 迁 newppu=1 路径；旧 PPU 保留** |
| v2.1 (设想) | — | 旧 PPU 也迁 Rust；vNESU11 完整（可选） |

**注意**：v2.0 的"v"是项目主版本号（与 v1.x 系列并列），不是 Rust crate 版本。crate `vnesu11` 自身用 `0.x` 起步。

**审计记录**：`AUDIT_20260810.md` 记录了 2026-08-10 的再评估（S1-S10），
本文件及 `02_architecture.md`、`phase_0/2/3/5/6`、`A_performance_model.md`
均已按审计结论修订。

---

## 6. 下一步

- 阅读 [01_feasibility.md](./01_feasibility.md) 了解学术/工程可行性依据
- 阅读 [AUDIT_20260810.md](./AUDIT_20260810.md) 了解审计发现与修订依据
- 阅读 [02_architecture.md](./02_architecture.md) 了解 vNESU11 内部结构
- 进入 [phase_0_foundation.md](./phase_0_foundation.md) 开始第一阶段
