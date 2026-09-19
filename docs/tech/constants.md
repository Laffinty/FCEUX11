# 模数与经验常数总表（Modulo & Magic Numbers）

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 跨模块速查表：写代码/做 review 时先查这里，再跳模块文档。行号以 v1.17 为准。

---

## 1. 时钟与频率（一切的基准）

| 常数 | 值 | 含义 | 代码锚点 |
|---|---|---|---|
| NTSC CPU | 1789772.7272727272 Hz | 21.47727 MHz ÷ 12 | `x6502.h:90` |
| PAL CPU | 1662607.125 Hz | 26.601712 MHz ÷ 16 | `x6502.h:92` |
| Dendy CPU | 1773447.467 Hz | Famiclone 变体 | `x6502.h:90` |
| NTSC 帧率 | 60.098813897440515532 | = 216000000/3573662 拍 | MoviePlay.cpp:333 |
| PAL 帧率 | 50.006977968268290849 | — | 同上 |
| PPU:CPU | 3:1（三区一致） | — | — |
| NTSC 帧长 | 89342 PPU dots（262×341，平均 −0.5） | ≈ 29780.5 CPU 周期 | — |
| PAL/Dendy 帧长 | 106392 dots（312×341，PAL 312 行但 odd skip 不同） | — | — |
| 1 CPU 周期 | 48 内部单位；1 PPU dot = NTSC 16 / PAL 15 单位 | **`X6502_Run(n)` 的 n 单位是 PPU dot** | `cpu.h:104-109`、`x6502.cpp:505` |
| scanline 数 | NTSC 262（可见 240）；PAL/Dendy 312（可见 240 + Dendy 290） | — | `ppu_core.cpp:65-81`、`fceu.cpp:146` |

## 2. 帧时序关键点（NTSC）

| (scanline, dot) | 事件 |
|---|---|
| (241, 1) | VBL flag 置位（若 $2000 bit7=1 → NMI） |
| (241, 1)+8 dot | FCEUX11 的 NMI 注入延迟（经验校准 `nd=8`，`FCEUX11_E1_NMIDELAY` 可调） |
| (261, 1) | VBL/sprite0/overflow 三标志清除 |
| (261, 280–304) | v ← t 全量恢复（copy vertical） |
| (任意行, 257) | v 水平分量 ← t（copy horizontal；「3-D WorldRunner 若在 256 做会花屏」） |
| (任意行, 256) | v 垂直增量；**dot 251** 才是 fetch 内 v 递增点 |
| (261, 339) | 奇数帧跳过的垃圾 NT 取址（odd frame skip；仅 BG 使能时） |
| (261, 337/339) | 两个垃圾 NT 读（mapper A12 计数可能感知） |
| 旧 PPU VBL 序列 | `X6502_Run(256+85)` → VBL → `X6502_Run(12)` → NMI（ppu_rendering.cpp:1211-1236） |

## 3. APU 帧序列（NTSC，CPU 周期）

| CPU 周期 | 事件 |
|---|---|
| 7457 / 14913 / 22371 | 4 步与 5 步共有的 quarter/half 时钟 |
| 29828 / 29829 / 29830 | IRQ flag / 半帧+IRQ / 序列重置（4 步） |
| 37281 / 37282 | 5 步的最后 half / 重置（无 IRQ） |
| $4017 写 | 偶周期 +3、奇周期 +4 成熟（`fc_reset_in = (abs_ts&1)?4:3`） |
| PAL | 8313/16627/24939/33252-33254；41565/41566 |

## 4. 混音公式

```
pulse = 95.52 / (8128 / p + 100)          // p = 两脉冲输出和 (0..30)
tnd   = 163.67 / (24329 / t + 100)        // t = tri/8227 + noise/12241 + dmc/22638
```
代码锚点：`sound.cpp:1537-1548`（`wlookup1[32]`、`wlookup2[203]`）。

## 5. 行为性魔数（为何是它）

| 魔数 | 为什么 |
|---|---|
| `PPU_OPEN_BUS_DECAY_CYCLES = 1073864` | blargg `ppu_open_bus` 实测 ≈600ms 衰减（`ppu.cpp:475-495`） |
| NMI 延迟 8 dot | e1_vbl 探针校准；7/9 都有测试反例（`ppu_rendering.cpp:1539-1545`） |
| MMC3 A12 hook 在 sprite fetch `s==2` | Dragon's Lair 依赖 pattern 使能判定而非 sprite 使能（`ppu_rendering.cpp:2060-2074`） |
| OAMADDR 帧首清零 | Super Donkey Kong 反例锁死（`ppu_rendering.cpp:1217/1632`） |
| ATX/OAL 用 `A|0xFF` | blargg 实测 NES 上不稳定输入为 $FF（ops.inc:346-348） |
| XAA 用 `A|0xEE` | 「BIG QUESTION MARK」自认可疑值（ops.inc:490-491） |
| `kLineTime = 341`、`kFetchTime = 2` | 新 PPU 主循环每 tile fetch 2 dot 的来源 |
| sprite LUT 65536 项 | pattern 字节对（2×256）全查表，空间换时间（`ppu_sprite_lut.h`） |
| `nesincsize = (1<<17)·CPU/(SndRate·16)` | 音频以 1/16 子采样累积（sound.cpp:1577） |
| DMC stall = 4 CPU 周期/字节 | 硬件 4 周期 halt 的近似（未做对齐抖动与仲裁，sound.cpp:659-683） |
| `wlcount=2048`、`nreg=1`、`S=0xFD` | 上电初值（APU 波形周期占位 / LFSR seed / 栈指针） |
| 非 2 幂 ROM 白名单 {53,198,228,547} | 头尺寸异常的 mapper 特例（ines_bmap.h:16-19） |

## 6. 性能模式（约束写法）

- PPU 主循环**禁止取模**：`runppu` 用回绕分支替代 `%341`（ppu_rendering.cpp:1366-1375）。
- `Ppu` 类热字段集中缓存行；曾因布局回退 bench +15%（ppu_class.h:276-308）。
- `Cpu::layout_` 必须偏移 0 + 64 字节对齐（savestate 二进制兼容，cpu.cpp:14-26）。
- 调色板整行 `|= 0x40404040` 不能合并为 32 位写（别名/布局陷阱，ppu_rendering.cpp:422-441）。
- `ops_table.inc` 由 `scripts/generate_x6502_dispatch.py` 生成，勿手改。

## 7. 换算速记

- dots → CPU 周期：÷3（NTSC/PAL/Dendy 一致，但 PAL CPU 慢所以 dot 更长）；341 dots = 113⅔ CPU 周期。
- NTSC 一行 = 341 dots ÷ 5.369318 MHz ≈ **63.5 µs**（≈113.67 CPU 周期 × 558.7 ns）；一帧 89342 dots ≈ **16.64 ms**。
- VBL 期 20 行 ≈ **1.27 ms**（NTSC，(241,1) 到 (260,341)）。
- 帧内 CPU 周期：NTSC ≈ 29780.5（旧 PPU 行分段 `256+85` dots = 113.67 周期/行）。
