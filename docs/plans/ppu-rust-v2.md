# PPU 模块 v2.1 — Rust 迁移与 CPU/PPU 主时钟协同优化

**Status:** Draft · **Branch:** `wip2.1` · **Author:** WIP · **Date:** 2026-08-23

## 0. 目标与非目标

**Primary goal.** 将 legacy/newppu 双路径的 C++ PPU 实现（`src/ppu.cpp` + `src/ppu_class.{h,cpp}` + `src/ppu_rendering.cpp` + `src/ppu_core.cpp` + `src/ppu_state.cpp`，约 4.2k 行实现）迁移至 `fceux11-core` 的 Rust 模块，以**单 PPU 点（1/3 CPU cycle）精度**重建寄存器与渲染时序；同时重构 CPU↔PPU 协同模型，把 v2.0 遗留的「指令步进 + `runppu(x)` 批量授权」模式替换为主时钟（master-clock）驱动，关闭 vbl/NMI、MMC3 IRQ、精灵 DMA/DMC 等 known-fail 家族，目标 KagamiQA **Grade A**。

**Non-goals.**
- 不做渲染后端重写：仍输出 256×240 XBuf（保持 `golden_frames/*.xbuf` 字节兼容），不做滤镜/缩放/着色器。
- 不做 APU 迁移：本计划只把 APU 作为主时钟 tick 链上的一个被 tick 节点（APU 仍为 C++），APU Rust 化另立计划。
- 不做从零重写：以现有 `newppu` 行为为语义基线（R5 深度校准后已接近目标），逐块搬运 + 精度修正，而非凭文档重推全部状态机。
- 不做 Mesen2 克隆：借鉴其主时钟 / 总线 tick 链架构（chippy ADR 0004），但保留 FCEUX11 的 FFI + 64 字节 CPU blob + SFORMAT savestate 契约。

## 1. 为什么可行

| 约束 | 现状 |
|---|---|
| Rust workspace 已就绪 | `src/rust/` 已通过 CMake 构建 staticlib；`fceux11-core` 已有完整 `cpu/` 模块（state/addressing/execute/tick/ffi/bus），PPU 可复用同一 FFI 与 `CppBus` 模式 |
| CPU 协同桥已验证 | Phase 7 后 C++ CPU 已删除；`fceux11_cpu_run` + `tick_cycles` 预/后回调 + `nmi_fresh_bridge` + IRQ bridge 全部落地，PPU 是下一个自然迁移对象 |
| 测试 oracle 齐全 | tests.json 47 项：ppu_test / ppu_frame_diff_test / ppu_phase_c,d_test / golden_savestate / savestate_regression / blargg vbl_nmi 族 / mmc3 族 / sprdma_dmc_dma / ppu_read_buffer |
| 权威参考已确认 | blargg 的 vbl_nmi 单 PPU 时钟精度声明、742 CPU/PPU 相位、Mesen2 主时钟 ADR、NESdev wiki 262×341 规范（详见 §2） |
| 增量迁移模式成熟 | CPU 项目已跑通「FFI 竖切 → 逐 Phase 等价替换 → 删 C++ 收口」流程，可直接复用 |

**Verdict: 可行，但精度门槛高于 CPU。** PPU 的风险在于「每点状态机 × 总线观测」的组合爆炸：寄存器端口、滚动计数、精灵扫描、VBL/NMI、MMC3 A12、DMA 子周期相互耦合，任何一处 ±1 dot 都会在 blargg 精度测试上显形。因此本计划把 VBL/NMI 与主时钟协同拆成独立 Phase，且每个 Phase 都挂不可回退的 gate。

## 2. 权威参考（联网检索，实施期以官方/原厂行为为准）

| 主题 | 来源 | 关键结论（供本计划引用） |
|---|---|---|
| blargg VBL/NMI 测试精度 | [nesdev 论坛 t730](https://archive.nes.science/nesdev-forums/f3/t730.xhtml) | 测试以单 PPU 时钟精度计时，没有出错的余地；vbl_nmi 09 子测试是整套中最难族 |
| CPU/PPU 同步相位 742 | [nesdev 论坛 t9999](https://archive.nes.science/nesdev-forums/f3/t9999.xhtml) | 复位后随机取四种相位之一，模拟器选 "742"；`$2002` 在 VBL set 前 1 个 PPU 点读取会读到 clear 并抑制该帧 NMI；附 NMI/VBL/Hit/Overflow 各 ±1 dot 相位表 |
| 主时钟同步模型 | [chippy ADR 0004（Mesen2 架构）](https://nkane.dev/chippy/adr/0004-v1.2.0/) | 指令步进时序无法通过 blargg/nesdev 精度测试（vbl/NMI 竞态、子周期 DMA、帧计数器）；解法：**每个总线访问前先 tick 链（PPU 3 dots / APU / cart）1 cycle**，全量精度 ROM 通过 |
| PPU 帧结构 | [NESdev Wiki: PPU](https://www.nesdev.org/wiki/PPU) | 262 scanlines × 341 dots；1 CPU cycle = 3 PPU dots；NTSC 奇数帧预渲扫描线缩短 1 dot（vbl_10） |
| MMC3 IRQ | [NESdev Wiki: MMC3](https://www.nesdev.org/wiki/MMC3) | IRQ 在 PPU A12 **上升沿**减计数；A12 连续低满 3 个 M2 下降沿后才重新使能计数（mmc3_4 关键） |
| 周期精确参考实现 | Mesen / SourMesen `Core/PPU.cpp` | 事件式主循环 + 每总线访问 tick 链；可作为行为对照 oracle |

> 注：直接抓取 nesdev.org 一度 403/超时，以上存档帖与镜像内容已足够支撑设计；实施期如遇歧义，以 Mesen 源码与 blargg ROM 实测行为为准。

## 3. 现状基线（必须锁定）

### 3.1 PPU 代码结构（C++，约 4.2k 行实现）

| 文件 | 职责 | 迁移去向 |
|---|---|---|
| `src/ppu.cpp`（1208 行） | 寄存器读写 handler（$2000-$2007 / $4014）、`FFCEUX_PPURead/Write` 函数指针、`newppu` 标志、open-bus decay、legacy `FCEUPPU_Loop` 旧路径 | `ppu/regs.rs` + `ppu/vram.rs` + `ppu/dma.rs` |
| `src/ppu_class.h/.cpp` | `fceu11::Ppu` 类、`PPUREGS`（fv/v/h/vt/ht 五组滚动计数 + 锁存）、`SPRITE_READ`、`PPUPHASE`、`g_ppu` 单例、`NTARAM/vnapage/PPU/SPRAM/XOffset` 别名 | `ppu/state.rs` |
| `src/ppu_rendering.cpp`（2162 行） | `RefreshLine/DoLine/ResetRL`；legacy `FCEUPPU_Loop`（ppu_rendering.cpp:1175）；new `FCEUX_PPU_Loop`（:1551，`runppu(x)` = 推进 `ppur.cycle` + `X6502_Run(x)`，:1365；`runppu1_inline` :1392）；`PPU_MASTER` 分派（:1355） | `ppu/bg.rs` + `ppu/sprite.rs` + `ppu/frame.rs` |
| `src/ppu_core.cpp` | `newppu_get_scanline/dot`、`PPU_ResetHooks`、`scanlines_per_frame` | `ppu/frame.rs` + `ppu/irq.rs` |
| `src/ppu_state.cpp` | savestate SFORMAT 表：NTAR/PRAM/SPRA/PPUR/KOOK/DEAD/PSPL/XOFF/VTGL/RADD/TADD/VBUF/PGEN + newppu 27 项 | `ppu/state.rs`（snapshot/restore） |

### 3.2 Rust CPU 协同现状（v2.0 成果，禁止回归）

- `src/cpu.cpp` 为唯一 facade；FFI 经 `src/rust/fceux11_rust.h`（cbindgen 合并头）。
- bus 回调 `cpu_rust_read/write_thunk` → `g_bus`；`fceux11_cpu_set_tick/tick_cycles` 提供每指令预/后回调，镜像 C++ `hook(temp)` 与 timestamp 推进（R2 修复 `163c9d7`：4*48/15≈12 像素偏移的 timestamp 增量推进）。
- `fceux11_cpu_set_irq_bridge` + `nmi_fresh_bridge` 负责 IRQ/NMI 新鲜度语义。
- 64 字节 X6502 blob（offset 0）与 C++ 布局逐字节兼容；`fceux11_cpu_snapshot/restore` 为纯 memcpy。
- Rust 侧 `fceux11-core/src/cpu/{ffi,bus,tick,execute}.rs`；staticlib 经 `src/rust/CMakeLists.txt` 的 `add_custom_command` + `add_dependencies(fceux11_core fceux11_rust_build)` 注入构建。
- `FCEUX11_RUST_CPU` 强制 ON，OFF 视为配置错误。

### 3.3 测试门面（tests.json 47 项）与 v2.0 终点

- **blocking**：`rom_regression_test`（12 ROM × 60 帧 CRC）、`savestate_regression_test`、`ppu_test`、`savestate_core_test`、`ppu_frame_diff_test`（0 像素）、`ppu_rendering_lut_test`、`ppu_phase_c/d_test`、`golden_savestate_test`、`blargg_cpu_instrs`、`blargg_cpu_timing`、`blargg_vbl_05_nmi_timing`、`blargg_ppu_read_buffer`。
- **advisory**：`blargg_smoke`、`blargg_ppu_vbl_nmi`（现状 03/09 PASS）、`blargg_suite`、`blargg_mmc3_4_scanline_timing`、`blargg_mmc3_v2_4_scanline_timing`、`blargg_cpu_int_2_nmi_brk`、`blargg_instr_misc`、`blargg_oam_stress`、`blargg_sprdma_dmc_dma`。
- **v2.0 终点（2026-08-22 审计）**：CTest 34/34；47 项 39P/8F **Grade B**（pass_to_fail=0、fail_to_fail=8）；blargg_suite 直跑 5P/7F、0 blocking；known-fail 60 项绝大多数 `expected_to_eventually_pass=true`，集中在 PPU/APU/MMC3 时序族——正是本计划的主攻方向。

## 4. 约束锁定

| # | 约束 | 来源 | 落实方式 |
|---|---|---|---|
| C1 | savestate SFORMAT 二进制兼容（NTAR/PRAM/SPRA/PPUR/KOOK/DEAD/PSPL/XOFF/VTGL/RADD/TADD/VBUF/PGEN + newppu 27 项） | `src/ppu_state.cpp` | Rust `snapshot/restore` 逐字段对拍；`golden_savestate_test` + `savestate_regression_test` 为硬 gate |
| C2 | XBuf 256×240 输出逐字节一致 | `tests/golden_frames/*.xbuf` | `ppu_frame_diff_test` 0 像素 + `rom_regression_test` CRC 不变 |
| C3 | `FFCEUX_PPURead/Write` 调用面（bus 路由、debugger、`CALL_PPUREAD/CALL_PPUWRITE` 宏） | `src/ppu.cpp:301-311` | C++ 保留转发壳，函数指针改指 Rust 导出符号，调用点零改动 |
| C4 | open-bus / `PPUGenLatch` / `$2007` 读缓冲语义 | `src/ppu.cpp` + blargg `ppu_read_buffer` | `ppu/vram.rs` 逐端口复刻 decay 与 latch |
| C5 | 主时钟比例：1 CPU cycle = 3 PPU dots；262×341；NTSC 奇数帧 340 dots | NESdev Wiki / vbl_10 | `ppu/frame.rs` 单一 dot 驱动，杜绝硬编码散点 |
| C6 | NMI/VBL 时序：`$2002` 在 VBL set 前 1 点读取 = clear 且抑制本帧 NMI；742 相位；NMI 延迟 = 当前指令完成 + 2 cycles | nesdev t9999 + blargg vbl_nmi | P5 专用 Phase + `nmi_fresh_bridge` 对接 |
| C7 | MMC3 IRQ：A12 上升沿减计数；A12 低满 3 个 M2 下降沿后重使能 | NESdev Wiki MMC3 + blargg mmc3_4 | P6 A12 观测器逐点上报 |
| C8 | CPU 精度零回归：`blargg_cpu_instrs/timing` 保持 PASS；`timestamp_/sound_timestamp/tick_cycles` 桥语义保留（R2 不回归） | `163c9d7` 修复 | 每 Phase 跑 CPU gate；主时钟重构只改 PPU 侧驱动方式 |
| C9 | 双路径过渡规则：过渡期允许 `newppu` 与 legacy 并存，但所有 gate 以 `newppu=1` 判定；P8 删除 legacy | `src/ppu_rendering.cpp:1176`（NSF 例外） | `PPU_MASTER` 最终只指向 Rust 入口 |
| C10 | DMA 子周期：`$4014` 精灵 DMA、DMC DMA 与 CPU 总线冲突行为 | blargg `sprdma_dmc_dma` | P7 经主时钟 tick 链表达，不再依赖批量预算 |

## 5. 架构设计

### 5.1 crate 结构（目标端态）

```
src/rust/crates/fceux11-core/src/ppu/
├── mod.rs          # Ppu 结构、主时钟入口（frame / dot 驱动）
├── state.rs        # PPUREGS 镜像 + 滚动计数 + 锁存 + SFORMAT snapshot/restore
├── regs.rs         # $2000-$2007 / $4014 寄存器端口 + PPUGenLatch / open-bus
├── vram.rs         # NTARAM / pattern / SPRAM 寻址 + $2007 读缓冲
├── bg.rs            # BG 取指管线（nametable / attribute / pattern + shift）
├── sprite.rs        # OAM 扫描 / 取指 / 合成 + sprite0 + 8 精灵限制
├── render.rs        # 像素合成 → XBuf（256×240）
├── dma.rs           # $4014 精灵 DMA + DMC 总线仲裁接口
├── irq.rs           # VBL NMI、MMC3 A12 观测器、帧计数器
├── frame.rs         # 扫描线 / dot 主时钟、奇帧短行、VBL 窗口
└── ffi.rs           # #[no_mangle] extern "C" 入口（PPU_MASTER 替代）
```

### 5.2 主时钟协同模型（替代 runppu 批量授权）

**现状问题**：`FCEUX_PPU_Loop` 逐点推进 `ppur.cycle`，并以 `X6502_Run(x)` 按批授予 CPU 预算（`runppu1_inline` 每 BG 取指 1 cycle）。CPU 在批次内看不到 PPU 状态变化，vbl/NMI 竞态、`$2002` 提前 1 点读、MMC3 A12 上升沿、DMA 子周期无法精确表达——这正是 v2.0 8 个 fail_to_fail 的根源（chippy ADR 0004 已论证同类架构必然失败）。

**目标模型（两条递进路径）**：

- **路径 A（P5-P7 过渡）**：保留 C++ 帧循环，但把「PPU 点推进」与「CPU 预算」解耦为回调：Rust PPU 在任意总线访问点之前调用 `tick_chain`（CPU 1 cycle / APU / cart），使 CPU 逐 cycle 感知 PPU 状态。仅用于 vbl 窗口、sprite eval、DMA、MMC3 观察点等关键区域；普通渲染行仍可用批预算控制开销。
- **路径 B（P8 落地）**：Rust 提供 `fceux11_ppu_frame()` 单函数驱动整帧：每 dot 推进 PPU 状态机，每 3 dots 调 `cpu::tick`（复用 `cpu/tick.rs` 的 `run_with_tick` 内存循环模型），IRQ/DMA/mapper 回调经现有 bridge 转发。C++ `PPU_MASTER` 指向该函数；`runppu` / `runppu1_inline` / `X6502_Run` 批量授权路径删除。

**红线**：CPU 的 `timestamp_ / sound_timestamp / tick_cycles` 桥语义必须保留（R2 修复），即每指令执行后按实际周期增量推进，避免 nestest frame-4 类回归。

### 5.3 FFI 契约（新增符号）

- `fceux11_ppu_init / fceux11_ppu_reset` — 生命周期。
- `fceux11_ppu_frame(skip) -> int` — 替代 `PPU_MASTER` 入口。
- `fceux11_ppu_read / fceux11_ppu_write(addr, v) -> u8` — 接管 `FFCEUX_PPURead/Write`。
- `fceux11_ppu_set_scanline_hook` — `InputScanlineHook` 等价。
- `fceux11_ppu_snapshot / fceux11_ppu_restore` — SFORMAT 兼容。
- `fceux11_ppu_set_a12_observer` — MMC3 A12 逐点上报（P6）。
- 复用现有 `cpu_rust_read/write_thunk` 与 tick bridge；不新增第二套总线回调。

## 6. Phase 构建计划（11 Phase）

> 用户已授权适当加多 Phase。切分原则：**寄存器 → 滚动/BG → 精灵 → VBL/NMI → MMC3 → DMA → 主时钟 → 精度收尾 → 性能 → 清理**，每 Phase 可独立验收、可回滚。

### P1 — 基线快照与契约锁定
- 冻结 `golden_hashes.json`、`golden_frames/*.xbuf`、`golden_savestate` 的 `.fc0` 与哈希。
- 跑全量：CTest 34/34、47 项 39P/8F Grade B、blargg 直跑 5P/7F；把 known-fail 60 项逐条记录当前状态。
- 提取 SFORMAT 契约表（`ppu_state.cpp` 13 组 + newppu 27 项字段偏移），落盘为机器可读契约。
- **验收**：基线 json 入库；任何 gate 状态零漂移。

### P2 — 寄存器端口迁移（$2000-$2007、$4014）
- Rust 实现全部写端口：控制/掩码、滚动写 ×2（`PPUGenLatch` 写序列）、`OAMADDR/OAMDATA`、`VRAMADDR/VRAMDATA`、`$4014` 精灵 DMA 触发；`$2002` 读（VBL/sprite0/overflow + latch clear）。
- open-bus decay、`$2007` 读缓冲端口语义先落地（细节 P9 收尾）。
- `CALL_PPUREAD/CALL_PPUWRITE` 改指 Rust 导出符号（C++ 保留转发壳）。
- **验收**：`ppu_test`、`ppu_phase_c/d_test`、`savestate_core_test`、`golden_savestate_test` 全 PASS；`rom_regression_test` CRC 0 变化。

### P3 — 滚动状态机 + BG 渲染端口
- 五组滚动计数器 fv/v/h/vt/ht + 粗/细 X + 地址锁存/增量；tile/attribute/pattern 取指管线 + shift register。
- `RefreshLine/DoLine` 的 BG 部分以 Rust 复刻，临时双实现 diff 对拍（每行像素级）。
- **验收**：`ppu_frame_diff_test` 0 像素；`ppu_rendering_lut_test` PASS；`rom_regression_test` 0 变化。

### P4 — 精灵流水线
- OAM 扫描（sprite0 命中探测、8 精灵上限 + overflow）、第二周期取指、SPRAM 布局 / pshift、合成优先级。
- **验收**：`ppu_phase_c_test` PASS；`blargg_oam_stress` 由 advisory 提升为 PASS 监控；sprite 相关帧 diff 0。

### P5 — VBL/NMI 点精确循环
- 262×341 dot 主时钟、VBL set/clear、奇数帧 340 dot 短行（vbl_10）、NMI 延迟（当前指令完成 + 2 cycles）。
- `$2002` 提前 1 点读取抑制（742 相位表）；NMI 新鲜度与 `nmi_fresh_bridge` 对接。
- **验收**：`blargg_vbl_05_nmi_timing` 保持 PASS；`blargg_ppu_vbl_nmi` 03/09 → ≥06/09；vbl_02/06/07/08 逐个转 PASS。

### P6 — MMC3 / 扫描线 IRQ 协同
- A12 观测器：每点向 cart 上报 A12 变化；MMC3 IRQ 在 A12 **上升沿**减计数；A12 低满 3 个 M2 下降沿后重使能。
- **验收**：`blargg_mmc3_4_scanline_timing`、`blargg_mmc3_v2_4_scanline_timing` 转 PASS；MMC3 相关 ROM 帧 diff 0。

### P7 — 精灵 DMA / DMC 协同
- `$4014` 精灵 DMA（每 2 CPU cycle 1 字节、odd cycle 读回）+ DMC DMA 总线仲裁 + SPR_DMA 暂停。
- 子周期行为经主时钟 tick 链表达（路径 A），不再依赖批量预算。
- **验收**：`blargg_sprdma_dmc_dma` 转 PASS；blargg_suite 内 dmc_dma 族不回归。

### P8 — CPU↔PPU 主时钟协同重构
- 实现路径 B：`fceux11_ppu_frame()` 单函数主时钟驱动；每总线访问前 tick 链（PPU 3 dots / APU / cart）。
- 删除 `runppu` / `runppu1_inline` / `X6502_Run` 批量授权路径；`PPU_MASTER` 仅指向 Rust 入口；`newppu=1` 为唯一路径（`GIT_NSF` 例外评估后决定）。
- CPU 桥保留：`tick_cycles / timestamp / sound_timestamp / nmi_fresh_bridge` 全部保留，R2 语义零回归。
- **验收**：`blargg_cpu_instrs/timing` PASS 不变；vbl/nmi 族全 PASS；mmc3 族 PASS；`rom_regression_test` 0 变化；CTest 34/34。

### P9 — 精度全量收尾
- `$2007` 读缓冲细节、open-bus decay 全端口、`blargg_cpu_int_2_nmi_brk`、`blargg_instr_misc`、帧计数器（framectr）边界。
- known-fail 60 项重跑：逐条转 PASS，或确认非 PPU 责任（APU / 其他，转出本计划）。
- **验收**：`blargg_ppu_read_buffer` PASS 保持；`blargg_cpu_int_2_nmi_brk`、`blargg_instr_misc` 转 PASS；47 项目标 **Grade A**（0 blocking / advisory fail）。

### P10 — 性能
- SoA 渲染缓冲、bitrev / palette LUT（复用现有 `ppu_phase_c/d` 成果）、每点回调瘦身（热路径内联 tick）、减少 FFI 往返。
- **验收**：单帧耗时 ≤ C++ 基线 105%（bench_tolerance）；CTest 34/34（含 -LE perf）。

### P11 — 清理收口
- 删除已迁移的 C++ PPU 源与 legacy `FCEUPPU_Loop`、`PPU_MASTER`、`newppu` 分派。
- 更新 build/CMake 引用；沿用 v2.0 流程把 plans 归档进 ChangeLog；输出 v2.1 发布 checklist。
- **验收**：`rg "FCEUPPU_Loop|PPU_MASTER|newppu" src/` 零命中（文档除外）；全量矩阵 Grade A；发布 v2.1。

## 7. 风险与回滚

| 风险 | 缓解 |
|---|---|
| 主时钟重构引入 CPU 回归 | 每 Phase 保持 `blargg_cpu_*` 与 R4 gate；P8 前以路径 A 过渡，主时钟仅用于关键窗口 |
| savestate 二进制漂移 | P1 锁 SFORMAT 契约；每 Phase 跑 `golden_savestate` + `savestate_regression`；Rust snapshot/restore 逐字段对拍 |
| 渲染帧漂移 | `ppu_frame_diff_test` 0 像素 + `rom_regression_test` CRC 是硬 gate；任何 Phase 不通过则回滚该 Phase |
| FFI 性能劣化 | P10 单独收口；P2-P4 过渡期可保留 C++ 渲染热路径（只接管寄存器/状态） |
| 双路径分歧（newppu vs legacy） | 以 `newppu=1` 为唯一判定；P8 删除 legacy |
| 官方资料访问受限（nesdev 403） | 已用存档帖 + Mesen 源码 + blargg ROM 实测行为兜底，见 §2 注 |

## 8. 参考资料

- [blargg VBL/NMI 测试（nesdev 论坛 t730）](https://archive.nes.science/nesdev-forums/f3/t730.xhtml)
- [CPU/PPU 同步相位 742（nesdev 论坛 t9999）](https://archive.nes.science/nesdev-forums/f3/t9999.xhtml)
- [chippy ADR 0004：主时钟同步（Mesen2 架构）](https://nkane.dev/chippy/adr/0004-v1.2.0/)
- [NESdev Wiki: PPU](https://www.nesdev.org/wiki/PPU) / [NESdev Wiki: MMC3](https://www.nesdev.org/wiki/MMC3)
- Mesen / SourMesen 源码（`Core/PPU.cpp`）— 周期精确参考实现
- 仓库内：`docs/ChangeLog.md` [2.0.0] 审计记录、`tests/tests.json`、`src/rust/CMakeLists.txt`、`src/rust/crates/fceux11-core/src/cpu/`
