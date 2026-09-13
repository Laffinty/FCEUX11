<!--
source: christopherpow/nes-test-roms（readme 与目录清单）、blargg 套件 readme（Wayback 存档）、
        https://www.nesdev.org/wiki/Emulator_tests（Wayback）、本仓 tests/fixtures/blargg_manifest.json 与 2026-09-13 实测
retrieved: 2026-09-13
conversion: compiled research note（套件事实转述 + 本仓集成现状）
-->

# 精度测试套件目录与本仓集成现状

> 用途：blargg 系测试 ROM 的覆盖范围、预期结果、下载与再分发注意，以及本仓（tests/fixtures/blargg_manifest.json + kagami_qa_blargg_runner）的集成现状。实现排障时先查本页定位"哪个套件考察哪个行为"。

## 1. 本仓集成现状（2026-09-13 实测）

- **清单**：`tests/fixtures/blargg_manifest.json`（177 ROM，类别 apu/cpu/ppu/mmc3/other，字段 frames / reset_after / probe_addr）。
- **运行器**：`build-rust-ppu/tests/kagami_qa_blargg_runner.exe --manifest fixtures/blargg_manifest.json`（在 tests/ 目录下运行；注意需把 `vcpkg_installed/x64-windows/bin` 加入 PATH，否则 runner 报缺 z.dll）。全量批处理输出 JSON（rom/addr/value/diag/status/duration_ms），exit code 非零表示存在 FAIL。
- **结果协议**（blargg 套件通用，readme 原文）：状态字节在 $6000（$80=运行中，$81=需 reset，$00-$7F=结果码）；$6001-$6003 签名 $DE $B0 $61；$6004 起文本；音频音调 0=通过、≥2=错误码。
- **当前结果（v2.1.2 引擎，2026-09-13 全量实测）**：**138 PASS / 39 FAIL**。失败簇与 v2.1.3 批次的对应：

| 失败簇 | 项数 | 计划批次（docs/plans/v2.1.3_ppu_accuracy_plan.md） |
|---|---|---|
| MMC3 族（mmc3_1..6、mmc3_v2_1..6） | 12 | 批次 1（A12 watcher） |
| VBL/NMI 1 周期精度（ppu_vbl_nmi、vbl_02/03/04/06/07/08/10） | 8 | 批次 3（逐周期调度） |
| CPU 中断/定时簇（cpu_int_2..5、cpu_interrupts、instr_timing*、instr_misc*、cpu_reset_regs） | 11 | 批次 3 |
| 假写/执行区副作用（cpu_dummy_writes_oam/ppu、cpu_exec_space_ppuio、sprdma_dmc_dma*） | 5 | 批次 3+4 |
| $2007 读缓冲（ppu_read_buffer） | 1 | 批次 5 |
| OAM 渲染期行为（oam_stress） | 1 | 批次 4 |
| 开放总线（ppu_open_bus） | 1 | 批次 5 |

- 已全绿的 PPU 相关套件（回归保护）：sprite_hit 01-11、sprite_overflow 1-5、ppu_vram_access、scroll/scanline 类、vbl_01/05/09、ppu_palette_ram、ppu_power_up_palette、ppu_sprite_ram、full_palette。
- 历史基线：`tests/fixtures/blargg_known_fail.json`（P5，v1.16 时代）记录了当时的失败与原因分类；其治理条款（AI 不得改 expected 值、基线更新需与代码同 review）继续有效。

## 2. 套件目录

### PPU：VBL/NMI 时序

- **ppu_vbl_nmi**（10 项：vbl basics/set/clear、NMI control/timing/on/off、$2002 读抑制、even/odd 帧）：单 PPU 时钟精度的 VBL 标志与 NMI 行为。
- **vbl_nmi_timing**（7 项）：同域，硬件验证过的时序常量。本仓清单中对应 vbl_timing_1..7。
- 本仓 vbl_02（set time）、vbl_10（even/odd timing）等失败直接对应调度粒度（逐指令 vs 逐周期），见上表。

### PPU：精灵

- **ppu_sprite_hit**（新版 10 项）/ **sprite_hit_tests_2005.10.05**（nes-test-roms 收录的旧版 11 项）：sprite-0 hit 判定与时序；逐项预期见 `../ppu/ppu_sprite_hit.md` §5。
- **ppu_sprite_overflow**（新版 5 项）/ **sprite_overflow_tests**（旧版同内容）：overflow 标志含硬件 bug 复刻；逐项预期见同页。
- **oam_read**：$2004 按 $2003 地址读 OAM；真机上电有四种结果（三种损坏签名）。
- **oam_stress**：$2003/$2004 随机序列压力测试；**NTSC 上只在四种 CPU-PPU 上电对齐之一通过**——门禁解读必须带此前提。

### PPU：寄存器/总线

- **blargg_ppu_tests_2005.09.15b**（palette_ram、power_up_palette、sprite_ram、vbl_clear_time、vram_access）：基础读写行为。本仓对应 ppu_palette_ram 等条目，已全绿。
- **ppu_read_buffer**（"Mammoth test pack"）：$2007 读缓冲的完整行为（缓冲更新时机、渲染期读、调色板直读）。当前 FAIL，批次 5 目标。
- **ppu_open_bus**：open bus 残值与衰减。当前 FAIL，批次 5 目标。
- **full_palette**：全色调色板/强调位输出。

### CPU / APU / DMA

- **instr_timing / instr_test-v5 / instr_misc / cpu_timing_test6 / cpu_interrupts_v2 / cpu_dummy_reads / cpu_dummy_writes / cpu_exec_space / cpu_reset / branch_timing_tests**：指令时序、中断时序、假读写副作用、执行区读副作用（cpu_exec_space_ppuio 要求对 $2000-$2007 的"执行读"产生真实寄存器副作用）。
- **sprdma_and_dmc_dma / dmc_dma_during_read4**：OAM DMA 与 DMC DMA 的交互时序（本仓 sprdma_dmc_dma 当前 FAIL）。
- **apu_test / blargg_apu_2005.07.30**：帧计数器、长度计数器、DMC、混音（本仓已大规模集成且多数通过）。

### Mapper

- **mmc3_test / mmc3_test_2 / mmc3_irq_tests**：MMC3/MMC6 扫描线计数器与 IRQ；结构与"测试 3-5 依赖未完全成文的硬件时序，1、2 通过为硬性要求；rev_A/rev_B 真机互斥"的注意事项见 `../cart/mapper_irq_mechanisms.md` §6。
- **MMC1_A12**（Bregalad）：MMC1 寄存器 2 的 A12 相关行为。
- **mmc5test_v2 / exram**：MMC5；**fdsirqtests**：FDS IRQ。本仓尚未集成。
- 无 VRC 专用 IRQ 套件。

### 其他

- **scanline / scanline-a1**：扫描线计数行为（本仓已集成、当前 PASS）。
- **nmi_sync**：NMI 同步。**240pee**（= tepples 的 240p Test Suite NES 移植，GPL-2.0-or-later）：显示/缩放器测试，非模拟精度套件。
- PeterLemon/NES（注意仓库名是 `PeterLemon/NES`）：2C02 各寄存器/行为的汇编演示与测试，仓库页未声明许可——只作事实参考。
- 总目录：Wiki《Emulator_tests》页；nes-test-roms 仓的 `status.txt` 是 NESICIDE/Nestopia 1.40/Nintendulator 0.975 的历史通过矩阵，可作横向参照。

## 3. 下载与再分发注意

- 聚合仓 **christopherpow/nes-test-roms** 未声明许可（GitHub license 字段为 null）；blargg 各套件 readme 也无许可声明（作者 Shay Green / gblargg@gmail.com，套件长期免费公开分发）。实践约定：**运行测试无碍；再分发需保留对原作者的署名与原始链接，不暗示超出原始分发意图的权利**。
- 新版 ppu_sprite_hit / ppu_sprite_overflow **不在** nes-test-roms 仓里；blargg 原站链接已失效，可用 Wayback 存档的 zip（`web.archive.org/web/2019id_/http://blargg.8bitalley.com/parodius/nes-tests/ppu_sprite_hit.zip` 等）。
- 本仓 `*.nes` 一律 gitignore（AGENTS.md 惯例），由 `scripts/download_blargg_roms.ps1` 下载到 fixtures。

## 4. 来源与许可

- christopherpow/nes-test-roms 各套件 readme（事实转述；许可状态见 §3）。
- blargg 套件 readme 原文（Shay Green；Wayback 存档核验，2026-09-13）。
- 《Emulator_tests》《Sprite overflow games》：NESdev Wiki，公有领域（核实记录见 ../README.md §1）。
- 本仓实测数据：build-rust-ppu 2026-09-13 全量批处理（blargg_v212_result.json，本地工件不入库）。
