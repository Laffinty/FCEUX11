<!--
source: https://www.nesdev.org/wiki/PPU_scrolling / PPU_registers / PPU_rendering / PPU_sprite_evaluation
        （经 web.archive.org 2024 快照核验）+ Mesen2/puNES/Nintendulator 已验证的写时序决策（见 ../reference/dot_ppu_designs.md）
retrieved: 2026-09-13
conversion: compiled research note（硬件事实转述 + 参照实现的时序决策对照；游戏实践清单来自 Tricky_to_emulate_games 与论坛存档）
-->

# 渲染期寄存器写语义（mid-frame writes）与分屏滚动

> 用途：v2.1.3 逐 dot 渲染管线的写语义规格。渲染期（渲染开启）写 $2000/$2005/$2006/$2007/$2003/$2004 的硬件行为、"何时生效"的参照实现取值、以及依赖这些行为的游戏清单。
> 基础：loopy v/t/x/w 的位定义与常规更新见 `ppu_scrolling.md` 与 `ppu_registers.md`；本页只讲**渲染期写入**这个特例。

## 1. 各寄存器渲染期写行为

### $2000（PPUCTRL）

- 渲染期写立即改变 t 的 bit10-11（nametable 选择）；pattern 表选择位影响**下一次取指**。
- **dot 257 竞态**：写在可见扫描线 dot 257 落下时，open bus 的低位会拼进活跃 v 的 nametable 位（Mesen2 `ProcessTmpAddrScrollGlitch`；ares 在 lx==257 同语义）。

### $2005（PPUSCROLL）

- 渲染期写与 vblank 期同构（第一写→fine X + t 低位；第二写→fine Y + t 高位），但效果立刻被渲染流水线"看到"：**X 分量只能在水平消隐前写入才完整生效**——因为每条可见扫描线的 dot 257 都执行 t→v 的水平复制。
- dot 257 写下有 open-bus 拼接竞态（同 $2000）。
- 中途改**垂直**滚动：来自 t 的垂直复制只发生在 pre-render 行 dots 280-304，因此渲染期写 $2005/$2006 的垂直分量**下一帧才可见**（`ppu_registers.md` 已载）。

### $2006（PPUADDR）

- **第二写不是立即生效**：Mesen2 把第二写排队 3 个 PPU dot 后应用（`_updateVramAddrDelay=3`，出处 VisualNES）；puNES 用 second_write.delay 倒计时 + race 影子字段（对着 split_scroll_test_v2.nes 校准）。Nintendulator 的 $2000/$2005/$2006 立即生效、$2007 单独延迟。
- $2006 写本身会把 v 推上 PPU 地址总线（渲染关闭时 mapper 看得到，MMC3 计数器被时钟——见 `../cart/mapper_irq_mechanisms.md` §3）。
- 完整 X/Y 分屏惯用法：$2006/$2005/$2005/$2006 四连写，最后两写安排在 hblank 内（`ppu_scrolling.md`）。
- 中途写 $2006 的效果被取指流水线**滞后 1-2 个 tile** 才可见（取指超前运行约 2 tile）。

### $2007（PPUDATA）

- 渲染期访问"不推荐"：会造成图形错乱，写会落到**不可预测的 VRAM 地址**（`ppu_registers.md` 已载：v 的 coarse X 被来自 fine Y 表的值污染 + 递增行为异常）。
- 已知滥用：渲染期 $2007 **读**会改变滚动位置（The Young Indiana Jones Chronicles、Zelda II 的画面抖动即此）；$3F00-$3FFF 区间的读立即返回调色板、底下仍对真实总线做一次读进缓冲。
- v 递增：渲染期 $2007 访问会使 v 同时走 coarse-X 与 Y 递增（双重效应）。
- **递增/生效时点**（参照实现一致地取 5-6 PPU 周期量级）：Mesen2 写立即但 v 递增推迟 1 dot（`_needVideoRamIncrement`），6 dot 内连续读忽略（`_ignoreVramRead`）；Nintendulator 用 IOMode 倒计时（写=6、读=5 个 PPU 周期）才做真实总线访问与递增；ares 用 blockingRead=6 节流。

### $2003 / $2004（OAMADDR / OAMDATA）

- 渲染期行为汇总见 `ppu_sprite_evaluation.md` §2：$2004 写不改 OAM 只做高 6 位毛刺递增；$2003 写触发 OAM 损坏；dots 257-320 窗口 OAMADDR 自动清 0。

## 2. 渲染开关切换（$2001 bit3/bit4）

- 中途两渲染位全关：渲染立即停止（精灵评估一并停止，见 `ppu_sprite_evaluation.md` §4）；**只关背景或只关精灵**则评估照跑、仅隐藏对应图层。
- **渲染关闭期间 v 直接输出在 PPU 地址引脚**；当 v 落在 $3F00-$3FFF 时，屏幕（背景像素位置）持续显示该调色板色而非 $3F00 的 backdrop——这是被软件有意使用的花招（SMB3 的四次 $2006 写法、Micro Machines、Loopy 的 paltest）。
- 中途关再开渲染：取指/移位寄存器/1-bit 属性锁存的衰减行为（`ppu_rendering.md` 已载）+ OAM 损坏（评估中途关闭）叠加，至少重开后的第一条扫描线有额外损坏。
- 渲染关闭期的 $2007 独占总线语义（ares 的实现表述最清晰）：直接 v 递增并立即对外发布总线地址。

## 3. 游戏实践清单（症状→机制）

| 游戏 | 机制 | 失真表现（引擎精度不足时） |
|---|---|---|
| Super Mario Bros. 3 | 帧中四次 $2006 写（调色板指针 + 滚动复位），依赖渲染关闭期 backdrop=v 花招 | 底色/状态栏 bank 错 |
| Marble Madness / Mother (J) / Pirates | 扫描线中途切 CHR bank 画文本框 | 文本框贴图错（写入在后续 tile 边界生效） |
| Battletoads / Bill & Ted's Excellent Adventure | 中途开关渲染（后者借此换 CHR bank） | 撕裂/冻结 |
| Zelda II / Young Indiana Jones | 渲染期 $2007 读改变滚动 | Y 滚动抖动 |
| Burai Fighter (U) | 渲染期 $2007 写画记分条 | 记分条被裁半 |
| Balloon Fight | 渲染期 $2007 读命名表做星星闪烁 | 星点效果异常 |
| Micro Machines | 渲染期 $2004 读 + palette-space backdrop 花招 | 状态栏/底色错 |
| Daydreamin' Davey / Stunt Kids / Rollerblade Racer | 帧中 OAM DMA / 精灵-only 状态栏（BG 关） | 状态栏丢失或错位 |
| Crystalis / Jurassic Park / Wario's Woods 等 MMC3 族 | IRQ 时机 + 行中换 bank | 见 `../cart/mapper_irq_mechanisms.md` §5 |
| Fire Hawk / Mig 29 / Time Lord | 用 APU DMC IRQ 做分屏 | 分屏位置错（APU 侧精度） |

社区流传但未找到 nesdev 一手信源、不宜引用的：Bases Loaded(1)、720 Degrees 的行中分割写（见 mapper_irq_mechanisms.md §5 末尾说明）。

## 4. 参照实现对照速查

| 行为 | Mesen2 | Nintendulator | ares | puNES |
|---|---|---|---|---|
| $2000/$2005 生效 | 当前 dot 立即 | 立即 | 立即 | 立即（带 race 影子） |
| $2006 第二写 | **延迟 3 dot** | 立即 | 立即 | delay 倒计时 |
| $2007 生效 | 立即写/递增延 1 dot/6 dot 读节流 | IOMode 6·5 周期 | blockingRead=6 | race 字段 |
| dot 257 open-bus 拼接 | 有（$2000/$2005） | 有 | 有（lx==257） | 有 |
| 渲染期 $2007 污染 | 有 | 有（"writes discarded, increments wrong"） | 有 | 有 |
| backdrop=v 花招 | 有 | 有 | 有 | 有 |

## 5. 来源与许可

- 《PPU scrolling》《PPU registers》《PPU rendering》《PPU sprite evaluation》：NESdev Wiki，公有领域（核实记录见 ../../README.md §1）。
- 参照实现行为事实：Mesen2（GPL-3.0，只学事实）、Nintendulator / puNES / ares（GPL-2.0 / GPL-2.0 / ISC）——仅转述，未复制代码，详见 `../reference/dot_ppu_designs.md`。
- 游戏实践：《Tricky_to_emulate_games》（Wiki 镜像）；SMB3 四次 $2006 写的分析见 nesdev 论坛 t401 存档。
