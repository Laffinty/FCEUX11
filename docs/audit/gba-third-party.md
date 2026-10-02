# 第三方代码审计：GBAEUX11 的 clementine 使用情况

> 审核日期：2026-10-02 · 审核人：构建 agent · 分支：`wip2.0`（`0d791e8`）
> 结论一句话：**硬件模拟核心用的是 clementine 的代码，MIT 许可且合规；
> BIOS、系统调用与全部系统集成是我们自己写的。两者是分层混合，不是「参考重实现」。**

本文件回答三个问题：用了谁的代码、用在哪、改了多少、许可是否合规。
所有数字为**实测**（`git diff` / `dumpbin` / 文件计数），不是引用既有文档。

---

## 1. 上游出处

| 项 | 值 |
|---|---|
| 项目 | [clementine](https://github.com/RIP-Comm/clementine) |
| 版权 | Copyright (c) 2024 RIPsters |
| 许可 | **MIT** |
| 引入提交 | `6f11daf` *feat(gba): v2.0 S0 — vendor GBA 核心并接通 CMake→cargo→staticlib→header 链路* |
| 本地路径 | `src/rust/crates/gba-core/` |
| 上游原路径 | `emu/` |

## 2. 规模：一手 vs 上游

| 树 | 行数 | 性质 |
|---|---:|---|
| `src/rust/crates/gba-core/src/` | **20,084** | clementine 上游，16 处本地补丁 |
| `src/rust/src/gba/` | **11,485** | **100% 本项目一手** |

一手代码 18 个文件：`mod` `bios` `ffi` `frame` `audio` `overlay` `gate` `save` `rtc`
`decoupling` + `swi/` 下 8 个（`wait` `trig` `decompress` `affine` `bitunpack`
`ram_reset` `checksum` + `mod`）。

**代码总量里上游占 63.6%。** 这个比例本身不是问题 —— 重新实现一个 ARM7TDMI +
LCD 控制器既无必要也不划算 —— 但它决定了「哪些缺陷算上游的、哪些算我们的」。

## 3. 模块级归属对照

| 领域 | 归属 | 说明 |
|---|---|---|
| ARM7TDMI CPU 核心 | **上游** | `cpu/arm/`、`cpu/thumb/`、`arm7tdmi.rs` |
| 总线与地址译码 | **上游** | `bus.rs` |
| 内存（WRAM/IWRAM/VRAM/卡带） | **上游** + 2 处补丁 | 补丁只加了 RTC 访问器 |
| PPU / LCD / 精灵 | **上游** | `cpu/hardware/lcd/**` |
| 定时器、DMA、串口、声音 | **上游** | `cpu/hardware/{timers,dma,serial,sound}.rs` |
| 实时时钟 S3511 | **上游** + 5 处补丁 | 芯片实现完整；补丁只加「注入固定时刻」的能力 |
| BIOS | **一手** | `bios.rs`：我们自建的 16 KB stub，含复位向量、异常向量、IRQ 分派 |
| 系统调用（SWI） | **一手** | `swi/`：已认领 16 个号码，逐个按 GBATEK 语义实现 |
| C ABI 接口面 | **一手** | `ffi.rs` `frame.rs` 的 34 个导出 |
| 音频采样链 | **一手** | `audio.rs`：分数累加器、饱和、音量 |
| 即时存档 | **一手** | `save.rs`：编解码、指纹、侧车、16 MB 工作线程 |
| 电池存档 | **一手** | `frame.rs` 的 6 个导出 + `internal_memory.rs` 的 2 处补丁 |
| 解耦守卫 | **一手** | `decoupling.rs` |

## 4. 补丁集：16 处，跨 3 文件

`git diff --numstat 6f11daf..HEAD` 实测：

| 文件 | 增 | 删 |
|---|---:|---:|
| `cpu/arm7tdmi.rs` | 118 | 2 |
| `cpu/hardware/internal_memory.rs` | 52 | 0 |
| `cpu/hardware/rtc.rs` | 32 | 2 |
| **合计** | **202** | **4** |

**「16 处」与「202 行」是两个数，说的不是一回事**，报审核时两个都要给：
16 是**逻辑改动点**（`ATTRIBUTION.md` §3.2/§3.2.1 逐条记录），202 是**物理行数**。
前者更能说明侵入性，后者更能说明 diff 规模。

16 处分为三组：

**① SWI 注入缝（5 处，`arm7tdmi.rs`）** —— 核心的 `handle_swi_hle` 是私有方法，
外部无法提供自己的 SWI 实现。补丁加了一个 `Option<fn(...)>` 钩子字段，让钩子
优先于核心自己的 `match`。**不改变不装钩子时的行为。**

**② halt 机制（4 处，`arm7tdmi.rs`）** —— 唯**确实改变上游行为**的一组：
`step()` 顶部加了停机判断，`Arm7tdmi::new` 清 CPSR 的 I 位。这是 S1a-1 引入的
R15 已知代价。

**③ RTC 固定时刻（5 处，`rtc.rs` + 2 处 `internal_memory.rs`）** —— 上游
`current_unix_secs()` 直接读 `SystemTime::now()`，**没有注入点**，导致核芯自己的
RTC 测试是拿同一刻的主机时钟当期望值（自比较）。补丁加 `time_override: Option<i64>`，
`None` 时逐位等同上游。

**④ 电池存档访问器（2 处，`internal_memory.rs`）** —— `BackupType::detect()` 与
`buffer_size()` 私有，且无 setter。

## 5. MIT 合规核对

MIT 的义务只有两条：**保留版权与许可声明**、**随分发提供许可全文**。

| # | 义务 | 状态 | 证据 |
|---|---|---|---|
| 1 | 保留上游版权声明 | ✅ | 各文件头部的 `//!` 文档块完整保留 |
| 2 | 保留许可声明 | ✅ | `crates/gba-core/LICENSE`，17 行 MIT 全文，首行 `MIT License`、次行 `Copyright (c) 2024 RIPsters` |
| 3 | 随分发提供许可全文 | ⚠️ **见下** | 文件在树里，但打包流程未核对 |
| 4 | 声明 `license` 字段 | ✅ | `Cargo.toml`: `license = "MIT"` |
| 5 | 标注来源仓库 | ✅ | `Cargo.toml`: `repository = "https://github.com/RIP-Comm/clementine"` |
| 6 | 逐处说明本地修改 | ✅ | `ATTRIBUTION.md` §3.2 / §3.2.1，16 处逐条 |
| 7 | 登记进版权审计 | ✅ | `COPYRIGHT_AUDIT.md:1386` |

### ⚠️ 第 3 条需要动作

仓库根 `COPYING` 是 **GPLv2 全文**（FCEUX 本体，281 行），`gba-core/LICENSE` 是
**MIT 全文**。二者可在同一发行物中共存 —— MIT 明确允许再分发与商业使用，
不要求衍生作品采用同一许可 —— **但 MIT 的「随分发提供许可全文」是硬义务**。

现有 `scripts/copy_dependencies.ps1` 只处理 DLL 拷贝，**不处理许可文件**。
发布前需确认 `gba-core/LICENSE` 进了发行包。这条没有代码可以替代，只能靠流程，
因此写在这里而不是假装已经解决。

## 6. 与 GPLv2 的关系

FCEUX11 整体是 GPLv2 的衍生作品（`DERIVATIVE_WORK_NOTICE.txt`）。GBA 侧引入
MIT 代码**不改变**这个结论：MIT 允许其代码被 GPLv2 项目使用，被包含的部分
继续按 MIT 授权。结论是「同一发行物内含 GPLv2 与 MIT 两种许可的组件」，
需要的是**分别标注**，而不是统一许可。

## 7. 上游同步

**当前状态：未同步，纯 vendor 快照。** 上游仍在活跃开发（R1 风险）。

`ATTRIBUTION.md` §5 记录了重新 vendor 的流程。需要注意：

- vendor 树已在 **3 个文件**上有本地修改（不是全部集中在一个文件，自 S2-b3 起
  就不再是了）。重新 vendor 前必须逐个文件比对，不能整目录覆盖。
- 基线 SHA 记录在 `ATTRIBUTION.md` §1，但本文件未复制该值 —— 单一事实源原则，
  审核时以 `ATTRIBUTION.md` 为准。
