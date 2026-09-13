<!--
source: https://www.nesdev.org/wiki/PPU_OAM (§ Sprite 0 hits) + https://www.nesdev.org/wiki/PPU_rendering + https://www.nesdev.org/wiki/PPU_sprite_priority
        （经 web.archive.org 2024 快照核验；本机直抓被 Cloudflare 403 拦截）+ blargg ppu_sprite_hit / sprite_overflow 套件 readme
retrieved: 2026-09-13
conversion: compiled research note（事实转述并标注来源页；blargg 套件预期逐条列出）
-->

# Sprite-0 hit 精确规则与 blargg 套件预期

> 说明：NESdev Wiki 没有名为 "PPU sprite hit" 的独立页面——判定规则分布在《PPU OAM》§ Sprite 0 hits 与《PPU rendering》；本页把它们与 blargg 套件的逐项预期汇编在一起，供 `sprite0_hit` 实现与测试对照。本仓当前实现（帧缓冲逐像素命中记录 + 精确 dot 锁存）已通过 sprite_hit 01-11 全套，本页作为回归保护的行为规格。

## 1. 触发条件（《PPU OAM》§ Sprite 0 hits）

- 触发：**精灵 0 的不透明像素与背景的不透明像素重叠**。
- **不触发**的情形：
  - x=0..7，当左裁剪窗口开启（$2001 的 bit1（BG）或 bit2（精灵）任一为 0）；
  - **x=255 永不触发**（与像素流水线相关的怪癖）；
  - 背景 or 精灵像素透明（CHR 图案的 2bit 色号为 %00）——只看图案位，不看实际颜色（黑对黑也算命中）、不看调色板；
  - 本帧 sprite-0 hit 已置位——**每帧只检测第一次命中**。
- **不受影响**的情形：
  - 精灵优先级位（精灵 0 完全在背景后面照样命中）；
  - PAL PPU 在 x=0、x=1、x=254 的左右边缘消隐。
- 硬件机制：精灵 0 在下一行范围内时**恒被分配第一个精灵输出单元**；命中条件 = "精灵 0 在范围内 AND 第一个精灵输出单元正在输出非零像素 AND 背景绘制单元正在输出非零像素"；用两个标志防止并发评估践踏第二标志。

## 2. 精确时序（《PPU rendering》）

- "Sprite 0 hit 表现为图像从 cycle 2 开始"——即移位寄存器第一次移位的那个周期，**最早**在该点置位；实际像素输出因内部渲染流水线进一步延迟，第一个像素在 cycle 4 输出。逐 dot 实现应把命中判定挂在像素流水线处理到该 x 的那个 dot，而不是"第 x 像素 = dot x+1"这种线性近似。
- 标志在 pre-render 行 dot 1 清零（《PPU OAM》）。
- blargg 09-timing / 10-timing_order 考到 PPU 时钟精度：包括"VBL 末尾清得太早/太晚"，以及命中时刻取决于**屏幕位置**而非"是哪个精灵像素"，且不把左裁剪下、x=255、右边缘外的像素计入。

## 3. 永不显示的精灵（《PPU rendering》/ blargg 07）

- 精灵 Y ≥ 239 只会渲染到屏幕外的行：Y=239 的像素在屏幕行 239 miss、238 hit；Y=255 永远 miss。y 范围判定见 `ppu_oam.md`。

## 4. 8x16（《PPU OAM》/ blargg 08）

- 8x16 模式忽略 $2000 bit3，由 tile 索引 bit0 逐精灵选表；下半块用 tile N+1（同一表）。垂直翻转时两个 subtile 上下互换（奇数 tile 画在上面）。上下 tile 边缘的命中/不命中由 08-double_height 分别考察。

## 5. blargg 套件逐项预期

### ppu_sprite_hit（新版 10 项；zip 原址已失效，用 Wayback 存档；nes-test-roms 仓里是 2005.10.05 版 11 项）

| ROM | 关键预期 |
|---|---|
| 01-basics | 完全被背景压住也命中；BG 渲染关 → miss；全透明精灵 miss；**只有图案低 2 位相关**；对其他精灵永不命中 |
| 02-alignment | 精灵与背景图案对齐组合 |
| 03-corners | tile 角落像素 |
| 04-flip | 翻转下的命中 |
| 05-left_clip | **左裁剪只在 X=0 时挡住命中**（X=1..7 不受左裁剪影响的组合另测） |
| 06-right_edge | **X=255 永远 miss**；精灵像素落在 254 则命中 |
| 07-screen_bottom | Y≥239 永远 miss；Y<239 可命中；屏幕行 239 miss、238 hit |
| 08-double_height | 8x16 上下 tile 的边缘 |
| 09-timing | PPU 时钟精度的置位/清除时刻（含 VBL 末尾清除） |
| 10-timing_order | 命中时刻取决于屏幕位置，不取决于哪个精灵像素；左裁剪/x=255/右边缘外像素不计入 |

### sprite_overflow / ppu_sprite_overflow（5 项）

| ROM | 关键预期 |
|---|---|
| 1-basics | 9 个精灵置位；**VBL 开头不清、VBL 末尾清**；$2001=$00 不置位；$2001=$08/$10 正常置位 |
| 2-details | 左裁剪下的精灵也算数；Y=239 置位、Y=240/255 不置位；每行 ≤7 个不置位；8x16 处理不正确（硬件如此） |
| 3-timing | PPU 时钟精度（第 9 个精灵远后于第 8 个时的置位早晚） |
| 4-obscure | 对角线 bug 的精确映射（见 `ppu_sprite_evaluation.md` §3）；搜索止于第 64 个精灵 |
| 5-emulator | 反作弊项：**未读 $2002 也要置位**；关渲染/改 OAM/改精灵高度必须重新计算置位时刻——禁止用"帧末统一判一次"的捷径 |

- 旧版时序窗口（论坛 t=1308，blargg 给出）：清除 ≈ CPU 2272；首行置位 2429-2465；第二行起 2542；末行起 29595。

## 6. 精灵优先级 MUX（独立于 hit，但同一实现域）

来源：《PPU sprite priority》。

- **OAM 索引低者在前**——精灵对精灵的遮挡与优先级位无关。
- 优先级位只在"精灵 vs 背景"间仲裁：最前不透明精灵的优先级位为 1 且背景像素不透明 → 背景在前。
- 硬件事实：8 个精灵输出单元**按编号接线，编号最小的不透明输出恒胜出**——无论优先级位、无论背景像素。因此一个"垫底"优先级的精灵只要有不透明像素，就会遮住编号更大的精灵（SMB3 的道具从音块里钻出即用此特性）。
- 实现模型：取指窗口（257-320）内按"前（低索引）→ 后"处理 8 个精灵，逐 X 只取**第一个**不透明像素并保存其优先级位；输出时"精灵像素替换背景"当且仅当（精灵不透明且前优先级）或（背景透明）。**不要**实现"前/景/背三层"模型——那是 NDS 的行为。

## 7. 来源与许可

- 《PPU OAM》《PPU rendering》《PPU sprite priority》《PPU sprite evaluation》：NESdev Wiki，公有领域（核实记录见 ../../README.md §1）。
- blargg `ppu_sprite_hit` / `ppu_sprite_overflow` / `sprite_hit_tests_2005.10.05` / `sprite_overflow_tests` readme 与套件本体（Shay Green，免费分发；新版套件原站已失效，Wayback 存档可用；nes-test-roms 仓收录旧版）。
- 论坛 t=626（Disch/tepples 关于 x=255 永不命中的讨论）、t=1308（overflow 时序表）：作者版权保留，仅短引。
