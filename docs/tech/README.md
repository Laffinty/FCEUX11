# docs/tech — FCEUX11 技术知识库

> **STATUS: ACTIVE**（2026-09-19 建库）
> 定位：**长期有效**的技术参考——硬件原理、FCEUX11 具体设计、历史积累的经验性
> 知识（模数、魔数、兼容 hack、已证伪假设）。过程性数据（调查记录、任务报告、
> 一次性审计）一律归档 `docs/history/`，不留在本目录。

---

## 1. 知识库文档

| 文档 | 内容 | 读者场景 |
|---|---|---|
| [cpu.md](cpu.md) | 6502/RP2A03 原理、指令表生成架构、周期账目、中断模型、非官方指令、已知近似 | 改 CPU、排查 CPU 时序 |
| [ppu.md](ppu.md) | 2C02 原理、双引擎（scanline/dot）架构、渲染管线、Loopy scroll、quirk 与魔数 | 改渲染、调 PPU 精度 |
| [apu.md](apu.md) | 五通道原理、帧计数器精度模型、DMC、混音公式、扩展音频 ABI | 改音频、调 APU 时序 |
| [cartridge.md](cartridge.md) | iNES/NES 2.0/UNIF、mapper 双注册制、MMC1/MMC3/UxROM 实现要点、MMC3 IRQ 与 A12 | 加 mapper、改加载器 |
| [constants.md](constants.md) | **模数与经验常数总表**：时钟频率、帧时序、APU 序列、混音公式、行为性魔数、性能约束 | 写代码/review 前速查 |
| [precision.md](precision.md) | 精度治理：已知失败面及根因、黄金回归体系、**禁忌清单**、纪律规则、调查数据索引 | 动任何时序代码**之前必读** |
| [F11QA.md](F11QA.md) | F11QA 测试框架：双 Oracle、口径、CI 数字回填纪律、迁移指南 | 测试体系、CI 门禁 |

阅读顺序建议：新人先 [constants.md](constants.md) 建立量纲，再按任务读对应模块文档；
任何时序/精度改动先过 [precision.md](precision.md) §3 禁忌清单。

## 2. 内容组织约定（每个模块文档四段式）

1. **一、硬件原理**：NESdev Wiki 综合的机器事实（来源页在文档头注明）。
2. **二、FCEUX11 具体设计**：本仓库怎么实现，为什么这样分层，附 `文件:行号` 锚点。
3. **三、经验与魔数**：历史积累的经验值、兼容 hack、性能教训，每条带出处。
4. **四、相关文档**：模块间交叉引用与归档数据链接。

## 3. 维护规则

- 引用源码用 `文件:行号`，行号漂移时以符号名为锚修正；引用归档文档必须用
  自 `docs/` 起的完整路径。
- 新增事实必须能落到四段式之一；纯过程性记录直接进 `docs/history/`
  （规则见 [../history/README.md](../history/README.md) §1）。
- Wiki 内容按 CC BY-SA 引用与改写；行内数值以 Wiki 原页 + 本库实测（blargg、探针）
  双源校对，冲突时以测试数据为准并在此注明。
- 与文档冲突的代码优先怀疑文档过期；发现即修。

## 4. 来源声明

- NESdev Wiki（nesdev.org，CC BY-SA）：PPU rendering / frame timing / scrolling /
  registers / sprite evaluation / OAM / power-up state；CPU / CPU interrupts /
  unofficial opcodes / power-up state / Cycle reference chart；APU 及其子页；
  iNES / NES 2.0 / UNIF / MMC1 / MMC3 / UxROM。
- FCEUX11 源码与历史调查数据（`docs/history/`），仓库内部，随代码演进。
