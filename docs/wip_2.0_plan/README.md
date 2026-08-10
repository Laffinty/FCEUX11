# wip v2.0 — vNESU11 工程方案

> **分支**：`wip_v2.0`
> **目标**：将 FCEUX11 的两个核心模块（C++ CPU 与 C++ PPU）迁移到 Rust，并合并为统一的虚拟 SoC 抽象 **`vNESU11`**；同时调整外层 C++ 框架以适配新的所有权边界。
> **状态**：WIP / 草案（每完成一个 phase 应在本文件更新进度）

---

## 目录

### 设计文档（先读这三份）
- [00_overview.md](./00_overview.md) — 项目目标、范围、成功标准
- [01_feasibility.md](./01_feasibility.md) — 可行性分析（含学术/工程参考）
- [02_architecture.md](./02_architecture.md) — vNESU11 架构与 FFI 边界

### 审计与决策（2026-08-10 再评估）
- [AUDIT_20260810.md](./AUDIT_20260810.md) — **审计报告**：2 个必须修正的设计错误（S1/S2）、5 个架构缺口（S3-S7）、3 处高估（S8-S10）。所有 phase 文件已按此修订。
- [DECISIONS_S3_S5_analysis.md](./DECISIONS_S3_S5_analysis.md) — **决策点理论分析**（联网检索）：S3 时序模型 → 决策 A + 预留 B 接口；S5 newppu → 选项③

### 阶段文件（按顺序执行）
- [phase_0_foundation.md](./phase_0_foundation.md) — FFI 骨架、SFORMAT tag 契约、逐字段布局校验
- [phase_1_cpu.md](./phase_1_cpu.md) — CPU 解释器迁移（budget 驱动）
- [phase_2_bus_and_ram.md](./phase_2_bus_and_ram.md) — 总线矩阵（固定区 match + mapper 区间表）+ 私有内存 + RAM 随机源
- [phase_3_ppu.md](./phase_3_ppu.md) — PPU 核心（newppu=1 段驱动）+ 渲染 + 精灵 LUT
- [phase_4_apu_dma.md](./phase_4_apu_dma.md) — APU 5 通道 + OAM/DMC DMA + IRQ 控制器
- [phase_5_mapper_adapter.md](./phase_5_mapper_adapter.md) — C++ mapper 的 FFI 适配层（per-range handler）
- [phase_6_integration.md](./phase_6_integration.md) — 接入主路径（shadow run 帧级三级对比）
- [phase_7_default_switch.md](./phase_7_default_switch.md) — 切换默认、移除旧 newppu=1 C++ 路径
- [phase_8_cleanup.md](./phase_8_cleanup.md) — 清理 newppu=1 C++ 源（旧 PPU 保留）

### 附录
- [A_performance_model.md](./A_performance_model.md) — 性能模型（修订：持平至 +3%）
- [B_risk_register.md](./B_risk_register.md) — 风险登记册与缓解策略
- [C_references.md](./C_references.md) — 学术论文、技术参考、关键源

---

## 进度跟踪

| Phase | 标题 | 状态 | 关键交付 | 预计工时 |
|-------|------|------|---------|---------|
| 0 | Foundation | ✅ 完成（2026-08-10 本地验证） | FFI 骨架 + cbindgen 生成器、`VNesSoc` skeleton、SFORMAT tag 契约、68 站点依赖图、`CpuRegsLayout` 逐字段校验、per-range mapper 表、CMake 注册。**本地验证**：VNESU11_CORE=ON/OFF 双配置构建成功、ctest 33/33、fceux11.exe 启动正常 | 2 周 |
| 1 | CPU | ✅ 完成（2026-08-10 含门禁） | 6502 解释器 Rust 版（151 官方 + 21 undocumented + budget 驱动）；nestest $02/$04 PASS；blargg cpu_instrs 16/16（Rust）且与 C++ 基线 parity；**性能门禁 PASS**（Rust 0.713 ms/帧 ≤ C++ 0.724×1.05） | 4 周 |
| 2 | Bus + RAM | ⚪ 未启动 | 固定区 `match` + MapperRangeTable 接入、WRAM/VRAM/OAM/Palette、RAM 随机源复刻 | 2 周 |
| 3 | PPU | ⚪ 未启动 | newppu=1 段驱动 PPU + 渲染 + 精灵 LUT | 6 周 |
| 4 | APU + DMA + IRQ | � 未启动 | 5 通道、DMA、外部 IRQ（FDS）、IRQ 控制器、scheduler | 4 周 |
| 5 | Mapper Adapter | ⚪ 未启动 | per-range handler 全量接通、FDS 虚拟 mapper、Game Genie 包装 | 3 周 |
| 6 | Integration | ⚪ 未启动 | `option(VNESU11_CORE)` 默认 OFF，shadow run 帧级三级对比 | 3 周 |
| 7 | Default Switch | ⚪ 未启动 | 默认 ON，CMake 强依赖通过 + newppu=0 回归组 | 2 周 |
| 8 | Cleanup | ⚪ 未启动 | 删除 newppu=1 C++ 路径（旧 PPU 保留） | 1 周 |

**总计预估**：27 周（≈ 6.5 人月），可串行；Phase 1 + Phase 3 可部分并行（不同工程师）。

---

## 与现有项目的关系

- **不破坏**：savestate 二进制格式（`x6502struct.h` 64 字节布局冻结）
- **不破坏**：~250 个 C++ mapper（通过 FFI 适配继续可用）
- **不破坏**：Lua 引擎（已迁 Rust）、kagami-qa QA 框架
- **改进**：Lua 内存钩子 hot-path（`AtomicBool` flag 守卫，零开销无 hook 场景）
- **改进**：savestate 一致性（SFORMAT tag 语义等价 + golden round-trip，审计 S2 修订）

---

## 决策记录（关键 ADR）

| ID | 决策 | 备选 | 理由 |
|----|------|------|------|
| ADR-001 | crate 名 `vnesu11`，核心 struct `VNesSoc` | `fceux11-core` 沿用、`VNesCore` | 与现有 `fceux11-*` 命名延续，struct 名遵循 Rust 习惯 |
| ADR-002 | Rust 拥有 CPU/PPU(newppu=1)/APU；C++ 拥有 mapper | C++ 拥有全部 / 纯 Rust | 最大化跨模块内联，最小化 ~250 mapper 重写 |
| ADR-003 | FFI 边界用 `*mut c_void` + `#[repr(C)]` vtable | `&mut dyn MapperAgent` | 性能可预测（一次间接跳转），与现状一致 |
| ADR-004 | savestate 格式冻结（**SFORMAT tag 语义**，非字节布局） | 重新设计 | TAS movie + 玩家存档兼容性是硬约束（审计 S2 修订） |
| ADR-005 | Phase 6 引入 `option(VNESU11_CORE)` 默认 OFF | 直接默认 ON | 灰度发布，零回归切换 |
| ADR-006 | 不做 dynarec/JIT（解释器为主） | 加 dynarec 阶段 | 当前 C++ 解释器性能足够；dynarec 复杂度远超性能收益 |
| ADR-007 | **[修订]** vNESU11 核心支持 iNES + FDS + NSF + VS 四类系统 | 仅 iNES | 四类共用同一核心，漏掉 = 功能全废（审计 S6） |
| ADR-008 | **[修订+已定]** 时序模型 = scanline-budget 复刻（决策 A），架构预留 dot 粒度接口（决策 B 于 v2.1+ 可选） | dot 紧交错（决策 B） | 现状是 `FCEUPPU_Loop` + `X6502_Run` budget 模型；重写 = 行为重写（审计 S3，理论分析见 `DECISIONS_S3_S5_analysis.md`） |
| ADR-009 | **[修订+已定]** Rust PPU 只实现 newppu=1；C++ 旧 PPU 保留为 movie 回退（选项③） | 双 PPU 全迁 / 拒绝旧 movie | QA 验证的是 newppu=1；FCEUX 上游实践 + TASVideos 指南均支持双实现并存（审计 S5，理论分析见 `DECISIONS_S3_S5_analysis.md`） |
| ADR-010 | **[修订]** mapper 用 per-range handler 表 | 单 vtable（已废弃） | mapper 是 `SetReadHandler(start,end,fn)` 区间注册（审计 S4） |

---

## 变更日志（每个 phase 完成时追加）

- **v0.1** (2026-08-10)：草案提交，创建 7+1 阶段骨架
- **v0.2** (2026-08-10)：**审计修订**——S1/S2 修正 savestate 与布局策略；S3 时序模型定决策 A；S4 mapper 改区间表；S5 newppu 定 ADR-009；S6 系统类型矩阵；S7 RAM 随机源；S8 性能预期降级；S9 影响面 68 站点；S10 shadow run 三级对比
- **v0.3** (2026-08-10)：**决策点理论分析（联网检索）**——S3 定决策 A + 预留 B 接口（v2.1+）；S5 定选项③（FCEUX 上游实践 + TASVideos 指南）。见 `DECISIONS_S3_S5_analysis.md`
- **v0.4** (2026-08-10)：**Phase 0 骨架完成**——crates/vnesu11 skeleton + FFI 表面（cbindgen 生成 vnesu11_ffi.h，4852 字节）+ 16 个测试全绿（12 unit + 4 layout_check）；savestate_tags.md（mapper-registered chunks 部分需 Phase 5 补充）与 core_headers_deps.md 产出；x6502struct.h 加 8 条 static_assert（S1）；bus.cpp 加 per-range forwarding stub（#ifdef VNESU11_CORE_ENABLED）；fceu.cpp::Emulate 加 #ifdef VNESU11_CORE_ENABLED 路由（hot-path 接通）；CMake 注册（option(VNESU11_CORE OFF) + add_subdirectory(crates/vnesu11)）。**待用户本地验证**：C++ cmake 构建、kagami-qa 行为不变；**Phase 1 末补漏项**：savestate round-trip、golden 二进制对比、mapper chunk 枚举。详见 phase_0_foundation.md
- **v0.5** (2026-08-10)：**Phase 1 完成（解释器部分）**——完整 6502 解释器：`CpuCore` + `BusContext` + `run_budget`（复刻 X6502_Run，决策 A）+ 256 项 decoder + 13 寻址模式 + 151 官方指令 + 21 undocumented（LAX/SAX/DCP/ISB/SLO/RLA/SRE/RRA/ANC/ARR/XAA/LAS/AXS/SHA/SHX/SHY/TAS/KIL）+ decimal ADC/SBC + NMI/IRQ/RESET（penultimate 采样 + SEI/CLI 一周期延迟）+ page-cross/RMW/分支周期。`VNesSoc` 接入 `CpuCore` + `VNesBusContext`。24 个集成测试 + 27 既有测试全绿（51 总计）。**待用户本地**：nestest.nes + blargg ROM 验证、性能门禁。详见 phase_1_cpu.md
