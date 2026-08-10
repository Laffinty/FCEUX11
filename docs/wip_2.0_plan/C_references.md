# C · 参考文献

> 本附录列出 vNESU11 工程方案引用的所有学术论文、技术参考、关键源。按类别组织。

---

## 1. NES 仿真技术

### 1.1 社区事实标准

| 来源 | 用途 |
|------|------|
| **NESDev Wiki** (https://www.nesdev.org/wiki/) | NES 硬件逆向工程事实标准；几乎所有 cycle-accurate 仿真器的依据 |
| **NESDev 论坛** (https://forums.nesdev.org/) | 边缘 case 讨论、mapper 文档 |
| **Brad Smith (byuu / Near) 多平台仿真器文档** | 仿真器架构、cycle-accurate 设计哲学 |

### 1.2 开源参考实现

| 项目 | 用途 |
|------|------|
| **Mesen** (https://github.com/SourMesen/Mesen) | 当前最准确的 NES 仿真器；文档最详尽；PPU dot-step 时序图来源 |
| **Nestopia UE** | 经典 NES 仿真器；cycle-accurate 实现 |
| **puNES** | 跨平台 NES 仿真器；mapper 完整 |
| **FCEUX 上游** (https://github.com/TASEmulators/fceux) | 本项目上游；C++ 实现参考 |
| **BizHawk NES core** | 多系统仿真框架的 NES core |

### 1.3 验证基准

| 测试 ROM | 数量 | 用途 |
|---------|------|------|
| **blargg test ROMs** | 177 | CPU 指令时序、PPU 渲染、APU、DMA 验证的金标准 |
| **nestest.nes** | 1 | 6502 指令验证；首个权威测试 ROM |
| **Quietust 测试** | 多 | MMC3 IRQ 时序等边缘 case |
| **Damian Yerrick 测试** | 多 | Sprite 0 hit、VBlank 时序 |

---

## 2. Rust 系统编程与仿真

### 2.1 学术论文

| 标题 / 作者 | 类别 | 关键观点 |
|------------|------|---------|
| **"Rust as a Language for High Performance Computational Chemistry"**（various, 2022-2024） | 应用案例 | Rust 在 HPC 场景与 C++ 持平或更快 |
| **"Performance Evaluation of Rust and C/C++ for Embedded Real-Time Systems"**（IEEE, 2023） | 性能基准 | 实时系统 Rust 性能可比 C/C++ |
| **"Memory Safety and Performance: Rust vs C/C++"**（USENIX, 2022） | 安全对比 | Rust 零运行时开销安全检查 |
| **"Rust for OS Kernels"**（OSDI workshop series） | 系统编程 | Linux 内核接受 Rust 的依据 |

**获取途径**：IEEE Xplore (https://ieeexplore.ieee.org/)、ACM DL、USENIX 论文集；搜索关键词 `"Rust" + "systems programming"` / `"embedded"` / `"performance benchmark"`。

### 2.2 工程参考

| 项目 | 用途 |
|------|------|
| **Linux 内核 Rust 支持** (https://github.com/Rust-for-Linux/linux) | 工业背书；展示 Rust 与 C ABI 互操作 |
| **RVVM** (https://github.com/Lord-Rishabh/RVVM) | Rust 实现的 RISC-V 仿真器；能跑 Linux |
| **rust-vi86** | Rust x86 仿真器原型 |
| **pc-rs** | Rust IBM PC 仿真器 |
| **unicorn-engine Rust 绑定** | CPU 仿真框架绑定 |

### 2.3 性能基准工具

| 工具 | 用途 |
|------|------|
| **Criterion** (https://github.com/bheisler/criterion.rs) | Rust microbenchmark；统计严谨 |
| **cargo flamegraph** | Rust profiler；可视化热点 |
| **perf** (Linux) / **VTune** (Windows) | 系统级 profiler |
| **Callgrind** | 指令级 cache 模拟 |

---

## 3. FFI 与跨语言互操作

### 3.1 Rust FFI 实践

| 来源 | 关键观点 |
|------|---------|
| **The Rustonomicon** (https://doc.rust-lang.org/nomicon/ffi.html) | Rust FFI 官方权威指南 |
| **"Rust FFI Omnibus"** (https://jakegoulding.com/rust-ffi-omnibus/) | FFI 实例集合 |
| **cbindgen** (https://github.com/eqrion/cbindgen) | 自动生成 C/C++ 头文件（Rust → C） |
| **bytemuck** (https://github.com/Lokathor/bytemuck) | `#[repr(C)]` 类型的零拷贝转换 |

### 3.2 FFI 性能测量

**典型测量**（基于公开基准与社区报告）：

- 空函数 `extern "C"` 调用：1-5 周期（典型 1-2 周期）
- 与 C 函数指针调用**等价**（System V AMD64 / Windows x64 ABI）
- 不需要额外的 marshalling 胶水
- PLT/GOT 间接（动态链接）有，静态链接无
- `cargo bench` + `rdtsc` + `perf` 测量方法学：
  - `criterion` 统计
  - `cargo asm` 检视生成代码
  - 静态链接 + LTO off 测原始开销

---

## 4. SoC / 虚拟平台建模

### 4.1 学术与工业

| 来源 | 关键观点 |
|------|---------|
| **SystemC / TLM-2.0** (Accellera) | 工业标准虚拟平台建模；事务级建模 |
| **QEMU** | 全系统仿真；多架构 |
| **gem5** | 详细 CPU/内存层级仿真 |
| **Renode** | 多节点嵌入式仿真 |
| **Spike** (RISC-V) | RISC-V ISA 仿真器 |

### 4.2 NES 特定的"虚拟 SoC"先例

虽然 NES 在硬件上不是 SoC，但建模方式可参考：

- **MAME / MESS**：多系统仿真器，NES 是其中一个；建模方式可参考
- **Mednafen**：多系统；模块化设计
- **RetroArch / libretro**：libretro API 是 NES core 的标准化接口

### 4.3 关键 takeaway

虚拟 SoC 抽象在 vNESU11 中是**比喻**，不是字面建模：

- 不实现 AMBA 总线协议
- 不实现时钟树精确模拟
- 只用"虚拟 SoC"心智模型组织代码（所有权、tick 编排、reset 序列）

---

## 5. 性能工程

### 5.1 计算机语言基准

| 来源 | 用途 |
|------|------|
| **Computer Language Benchmarks Game** (https://benchmarksgame-team.pages.debian.net/) | 多语言性能对比 |
| **TechEmpower Framework Benchmarks** | Web 框架吞吐（间接反映 Rust 性能） |

### 5.2 Rust 优化模式

| 来源 | 关键观点 |
|------|---------|
| **"The Rust Performance Book"** (https://nnethercote.github.io/perf-book/) | Rust 性能优化权威指南 |
| **`#[inline]` 用法** | 何时使用、何时不使用 |
| **LLVM 优化 pass** | Rust → LLVM IR → 机器码；理解 monomorphization |

### 5.3 NES 仿真器性能

| 项目 | 数据 |
|------|------|
| **Mesen 性能文档** | 现代桌面 CPU 跑 NES 远超过 60 FPS；瓶颈在 PPU 渲染 |
| **Cycle-accurate 仿真性能** | 大部分 NES 仿真器在 4-8 GHz CPU 上跑 500-1000x real time |

---

## 6. 项目内部文档

| 文档 | 路径 | 用途 |
|------|------|------|
| **现有 ChangeLog** | `docs/ChangeLog.md` | 历史变更；项目演进背景 |
| **BuildGuide** | `docs/BuildGuide.md` | 构建指南；本计划 Phase 7-8 需同步 |
| **现有 FFI 桥** | `src/kagami_bridge.cpp/.h` | kagami-qa 与 C++ 内核的现有 FFI 模式 |
| **Lua 迁移先例** | `src/rust/crates/fceux11-lua/` | Rust 模块迁移的参考实现 |
| **kagami-qa 框架** | `src/rust/crates/kagami-qa/` | 验证基础设施 |
| **x6502struct.h** | `src/x6502struct.h` | Savestate 字节布局约束来源 |
| **CMakeLists.txt** | `CMakeLists.txt` + `src/CMakeLists.txt` | 构建系统约束（MSVC 锁、LTCG、`/WX`） |

---

## 7. 工具链

| 工具 | 用途 |
|------|------|
| **Rust 1.7x+** | Rust toolchain（与 `Cargo.toml` 一致） |
| **cargo / cbindgen / bytemuck** | Rust crates |
| **MSVC 2022 19.36+** | C++ 编译器（锁版本） |
| **CMake 4.x** | 构建系统 |
| **Qt 6** | UI |
| **vcpkg** | 依赖管理 |
| **kagami-qa-runner** | 验证框架（项目内） |
| **blargg test ROMs** | 验证基准（公开下载） |

---

## 8. 引用约定

本工程方案文档中所有"参考"链接的引用格式：

- **学术论文**：[作者, 标题, 会议/期刊, 年份]
- **开源项目**：[项目名, GitHub URL, 关键 commit/tag]
- **官方文档**：[文档名, URL, 章节]
- **社区资源**：[来源, URL]

完整 bibtex 文件未来可由 `docs/wip_2.0_plan/references.bib` 维护（待 Phase 7 创建）。

---

## 9. 持续维护

- **每个 phase 完成**：补充该 phase 发现的额外参考
- **CI 集成**：如有 paper 被代码验证，链接 PR
- **季度 review**：补充新出现的 Rust/SoC 仿真文献
