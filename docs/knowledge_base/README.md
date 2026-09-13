# FCEUX11 知识库（docs/knowledge_base）

本目录汇总本项目实现所依据的权威硬件/格式资料，供实现、排障与评审时查阅。
当前内容以 NESdev Wiki 页面（由 owner 提供的离线网页存档）为第一批来源，整理为 Markdown。

## 1. 来源与许可

- 来源：NESdev Wiki（https://www.nesdev.org/wiki/ ）。
- 许可：据 owner 说明，NESdev Wiki 内容为公有领域（public domain），可自由用于本项目（GPLv2）。2026-09-13 复核：Wiki 的《Nesdev wiki:General disclaimer》原文写明 "Any information posted on this wiki is considered public domain … You can use the information from this wiki any way you want"，现行页脚也无 CC 声明（社区常误传为 CC BY-SA，经一手核验不成立）。
- 每个文件头部都保留了原始页面 URL 与抓取日期，便于回溯与核对；如需引用第三方资料，请同样注明来源与许可。
- 第三方许可红线（2026-09-13 研究核实，详见 reference/dot_ppu_designs.md §4）：**Mesen2 是 GPL-3.0，与本项目 GPLv2 不兼容——只可学习事实，禁止复制代码**；puNES/Nestopia/Nintendulator/FCEUX 为 GPL-2.0、ares 为 ISC，代码复用合法但须署名；blargg 测试 ROM 与聚合仓未声明许可，再分发需保留署名。

## 2. 目录结构与命名规范

- 目录按硬件子系统划分：@ppu/@、@apu/@、@cart/@、@video/@，以及 @reference/@（参照实现设计综述与测试套件目录，2026-09-13 新增）（后续可加 @cpu/@、@system/@）。
- 文件名统一使用小写 snake_case：
  - 子系统页面：@<subsystem>_<topic>.md@（例：@ppu_rendering.md@、@apu_frame_counter.md@）
  - 卡带 mapper：@mapper_<name>.md@（例：@mapper_mmc3.md@）
  - 文件格式：@<format>_header.md@（例：@ines_header.md@、@nes2_header.md@）
- 每个文件必须以如下头部注释块开头（YAML 风格，放在 HTML 注释里以免影响渲染）：

  `
  <!--
  source: <原始 URL>
  retrieved: <抓取日期 YYYY-MM-DD>
  conversion: <整理方式，例如 automated from the owner-provided HTML archive>
  -->
  `

- 正文保留原文的标题层级；表格转为 Markdown 表格；图片不随仓库存放，必要时在正文中标注原页面图名。
- 若同一主题有多份来源，优先使用原始硬件资料（NESdev Wiki / 官方手册），并在文末列出交叉来源。


## 3. 索引

| 主题 | 文件 | 主要用途 |
|---|---|---|
| PPU 渲染流水线、图层/优先级/sprite-0 hit | [ppu/ppu_rendering.md](ppu/ppu_rendering.md) | 渲染器实现与复核 |
| PPU 滚动（loopy v/t/x/w） | [ppu/ppu_scrolling.md](ppu/ppu_scrolling.md) | 扫描线滚动、dot 256/257 语义 |
| PPU 调色板（含 emphasis/灰度） | [ppu/ppu_palettes.md](ppu/ppu_palettes.md) | 颜色、3F10 精灵调色板、别名规则 |
| PPU OAM / 精灵属性 | [ppu/ppu_oam.md](ppu/ppu_oam.md) | 8x16、翻转、优先级、Y 坐标 |
| PPU 寄存器 | [ppu/ppu_registers.md](ppu/ppu_registers.md) | 2000-2007 位定义与写读语义 |
| PPU 帧时序 | [ppu/ppu_frame_timing.md](ppu/ppu_frame_timing.md) | 每 dot 事件（256/257/280-304） |
| PPU 命名表与镜像 | [ppu/ppu_nametables.md](ppu/ppu_nametables.md) | 镜像模式与地址映射 |
| PPU 上电状态 | [ppu/ppu_power_up_state.md](ppu/ppu_power_up_state.md) | power/reset 初值 |
| PPU 开放总线 | [ppu/ppu_open_bus.md](ppu/ppu_open_bus.md) | 2002/2007 读回与 open bus |
| APU 帧计数器 | [apu/apu_frame_counter.md](apu/apu_frame_counter.md) | 帧中断线与 4017 |
| iNES 头 | [cart/ines_header.md](cart/ines_header.md) | ROM 加载 |
| NES 2.0 头 | [cart/nes2_header.md](cart/nes2_header.md) | 扩展头解析 |
| MMC3 | [cart/mapper_mmc3.md](cart/mapper_mmc3.md) | A12 时钟与 IRQ 时序（逐字存档） |
| Mapper IRQ 机制分类与 A12 watcher 参照 | [cart/mapper_irq_mechanisms.md](cart/mapper_irq_mechanisms.md) | MMC3 族/克隆/JY/MMC5/VRC 分类、Rev A/B、滤波取值边界、Mesen2 行为、问题游戏 |
| NTSC 视频信号与调色板生成 | [video/ntsc_video.md](video/ntsc_video.md) | 颜色输出建模 |
| 精灵评估与 OAM 渲染期行为 | [ppu/ppu_sprite_evaluation.md](ppu/ppu_sprite_evaluation.md) | 逐 dot 评估、OAM rot、overflow bug、OAM 衰减 |
| Sprite-0 hit 规则与套件预期 | [ppu/ppu_sprite_hit.md](ppu/ppu_sprite_hit.md) | 命中判定、精确时序、blargg 逐项预期、优先级 MUX |
| 渲染期寄存器写语义 | [ppu/ppu_mid_frame_writes.md](ppu/ppu_mid_frame_writes.md) | mid-frame 写 $2000/$2005/$2006/$2007、分屏滚动、游戏实践 |
| 逐 dot PPU 参照设计综述 | [reference/dot_ppu_designs.md](reference/dot_ppu_designs.md) | Mesen2/Nintendulator/ares/puNES/Nestopia/FCEUX 设计与许可红线 |
| 精度测试套件目录与集成现状 | [reference/accuracy_test_suites.md](reference/accuracy_test_suites.md) | blargg 套件覆盖、运行协议、当前 138/177、失败簇→批次映射 |

## 4. 与实现/计划文档的关系

- 实现细节与验收记录：[docs/history/v2.1.2_mmc3_irq_clock_archived_2026-09-13.md](../history/v2.1.2_mmc3_irq_clock_archived_2026-09-13.md)（PPU Rust 迁移期间的 IRQ、滚动、精灵修复）。
- 本知识库只放硬件/格式事实，不放实现结论；实现结论写在 plan/commit 中。
- 引用硬件行为时，建议在代码注释里同时写出本知识库的相对路径，便于溯源。


## 5. 新增条目的流程

1. 先判断归属目录（ppu/apu/cart/video/...）；没有合适目录时新建，并在本节补一行说明。
2. 文件名按第 2 节的命名规范。
3. 写入头部注释块（source/retrieved/conversion），正文保留原始章节结构。
4. 在本 README 的索引表补一行（主题 / 文件 / 用途）。
5. 若资料与现有实现冲突，先在本知识库记录事实，再到 docs/plans 下开条目记录差异与处置。

## 6. 待补充来源（下一步）

- 2026-09-13 研究新增：PPU sprite evaluation / OAMADDR / sprite-0 hit / mid-frame 写语义 / MMC3 族 IRQ 机制 / 参照实现综述 / 测试套件目录（ppu/、cart/、reference/ 本轮新增条目）。
- NESdev Wiki 尚未抓取的页面：APU DMC / APU length counter、CPU 内存映射、MMC1/VRC 系列 Mapper 页。
- 其他权威资料：loopy 滚动文档与 Brad Taylor 2C02 技术参考的独立条目化（两文均无版权声明，转述需署名；摘要已并入 reference/dot_ppu_designs.md）。
- 官方文档：NES 开发手册（如 Nintendo 的 2A03/2C02 手册扫描件，若可获得）。


## 7. engineering 目录（工程经验，非硬件事实）

该目录收录本项目自己的排障与工程经验，命名用 <topic>.md（与硬件页面的命名规则区分开）。

| 主题 | 文件 | 用途 |
|---|---|---|
| PPU 画面回归排障方法论 | [engineering/debugging_ppu_regressions.md](engineering/debugging_ppu_regressions.md) | 症状到检查点映射、导帧/ASCII/探针/A-B 工具链 |
| Rust 与 C++ 共享状态所有权 | [engineering/rust_state_ownership.md](engineering/rust_state_ownership.md) | X6502 blob 整块回写陷阱（IRQ 行案例） |
| 测试门禁的陷阱与设计 | [engineering/test_gate_pitfalls.md](engineering/test_gate_pitfalls.md) | 弱基准陷阱、反向验证、基准再生成纪律 |

