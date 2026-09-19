# PPU（2C02）— 原理、FCEUX11 设计与经验知识

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 素材来源：NESdev Wiki（CC BY-SA，`nesdev.org/wiki/PPU_rendering`、`PPU_frame_timing`、
> `PPU_scrolling`、`PPU_registers`、`PPU_sprite_evaluation`、`OAM`、`PPU_power_up_state`）
> + FCEUX11 源码逆向记录。引用格式：`文件:行号`（以 v1.17 为准，行号可能漂移，以符号名为锚）。

---

## 一、硬件原理（Wiki 综合）

### 1.1 总体参数（模数）

| 参数 | NTSC | PAL | Dendy |
|---|---|---|---|
| 每帧 scanline | 262 | 312 | 312 |
| 可见 scanline | 0–239 | 0–239 | 0–239 |
| VBL 期 | 241–260 | 241–310 | 241–310 |
| 每 line dot（NTSC） | 341（0–340） | 341 | 341 |
| PPU:CPU 时钟比 | 3:1 | 3:1* | 3:1 |
| 帧率 | 60.0988 | 50.0070 | 50.0070 |
| 主频 | 21.47727 MHz ÷ 4 = 5.369318 MHz | 26.601712 MHz | 同 PAL |

*PAL 的 PPU:CPU 比同为 3:1，但 PAL CPU 主频低（1.662607 MHz vs 1.789773 MHz），且
PAL/Dendy 在 pre-render 与 render 行上每周期只跑 1 次 CPU tick 相关逻辑（见 Wiki
`PPU_frame_timing` 与 `CPU` 页）。FCEUX11 以 `PAL?15:16`（每个 PPU dot 的内部时钟单位数）
区分两区（`src/x6502.cpp` `X6502_Run`）。

### 1.2 帧时序（NTSC，Wiki `PPU_frame_timing`）

| 事件 | 位置 | 说明 |
|---|---|---|
| VBL flag（$2002 bit7）置位 | scanline 241, dot 1 | 同一时刻若 $2000 bit7=1 则触发 NMI |
| NMI 触发 | 置位后 +7~8 dot 边沿 | 硬件 NMI 低电平传播延迟 |
| pre-render 行 | 261（= -1） | 可见行 0 的准备行 |
| VBL flag / sprite0 / overflow 清除 | pre-render, dot 1 | 三标志同时清零 |
| odd frame dot skip | 奇数帧 pre-render, dot 339→0 | 仅当 BG 使能（$2000-1 的 bit3，渲染期评估）；跳过 (339,0) 的取址而非整 dot |
| 帧长 | 89342 dots（262×341−0.5 平均） | 60.0988 fps 的来源 |

### 1.3 背景渲染流水线（Wiki `PPU_rendering`）

可见行每 8 dots 取一个 tile，四步各 2 dots（dot 1-256 与 321-336 跑）：

1. **NT byte**（nametable）
2. **AT byte**（attribute，1 字节管 4×4 tile，按 tile 位置取 2bit 象限）
3. **pattern 低平面**
4. **pattern 高平面**

关键时序点：

- **dot 256**：v 的 fine-Y/coarse-Y 垂直增量（`increment_vs`）。
- **dot 257**：v 的水平分量从 t 恢复（`copy horizontal`）。
- **dots 280–304**（仅 pre-render 行）：v 从 t 全量恢复（`copy vertical`）。
- **dot 337/339**：两个「垃圾 NT 取址」（硬件也会做，mapper 计数可能感知）。
- **odd skip 的本质**：奇数帧省掉的是 pre-render 行 dot 339 的那次垃圾 NT 读，不是整 dot。
- v 的水平递增发生在**每 tile 第 8 dot**（dot 8,16,…,256, 328, 336），垂直递增只发生在
  **dot 251**（在 fetch 周期内而非行尾）——这是把「v++ at dot 251」写死而非「行尾 v++」的依据。

### 1.4 Loopy scroll（Wiki `PPU_scrolling`）

- `v`（15bit 当前 VRAM 地址）位布局：`yyy NN YYYYY XXXXX` = fine-Y(14-12) NT(11-10)
  coarse-Y(9-5) coarse-X(4-0)；`t` 同布局；`x`（fine-X，3bit）；`w`（写翻转触发器）。
- $2005 第一写：t 的 coarse-X ← data 低 5 位，`x` ← data 低 3 位；第二写：t 的 coarse-Y/fine-Y。
- $2006 第一写：t 高 6 位（bit 8-13），**同时 t 的 NT 位(10-11)被 $2000 的 bit0-1 覆盖**；
  第二写：t 低 8 位，然后 `v = t`。
- $2002 读清 `w`（vtoggle）。
- $2007 读写：渲染期走 loopy 增量（+1/+32），非渲染期按 $2000 bit2 走 ±1/+32。

### 1.5 精灵（Wiki `OAM`、`PPU_sprite_evaluation`）

- OAM 256 字节 = 64 精灵 × 4 字节（Y、tile、attribute、X）。
- 评估发生在 dots 65–256（偶 dot 读 OAM，奇 dot 写 secondary OAM），渲染在**下一行**。
- **8 精灵/行上限**：第 9 个起置 overflow 标志。硬件 bug：评估第 9 精灵时按「2 tile 内
  对角线比较」继续扫描（m 计数错误递增），导致 overflow 可能在不足 9 个可见精灵时误置。
- **OAM decay**：OAM 内部是动态存储，渲染停止 8 秒左右会缓慢衰减（discarded data 倾向于
  沉降到 $2C 位型）；模拟器通常不模拟，但 OAMADDR≠0 时写入 OAM 会「渗漏」到 sprite
  评估（见 §1.6）。
- sprite 0 hit 条件：不透明 sprite 像素 与 不透明 BG 像素重叠且 X < 255（X=255 永不触发）。

### 1.6 寄存器与 quirk（Wiki `PPU_registers`）

- **$2007 读缓冲**：读 $2007 返回上一次缓冲的值，palette 区（$3F00-$3FFF）旁路缓冲
  直接返回，且**下一**次非 palette 读返回的是 `addr - 0x1000` 处（palette 镜像里的 NT 数据）。
- **palette 读高 2 位**：palette 数据只有 6 位，读出的高 2 位来自**当前 PPU open bus 锁存**。
- **open bus decay**：PPU 内部数据总线残留值约 600 ms 衰减（±25% 概率浮动，blargg 实测）。
- **$2004**：OAMADDR≠0 时写 OAM 会把写入「漏」到评估中（bug 级别行为）；渲染期读 $2004
  返回的是**评估中的 secondary OAM** 数据而非 OAMADDR 处。
- **$2002 读抑制**：恰在 VBL 置位同 dot 读 $2002 会抑制该帧 NMI（read-suppression）。
- **$2000 写延迟**：渲染期写 $2000 改变 NT 选择位会连带改 t 的 bit10-11；VBL 期间写
  $2005/$2006 不应清锁存（VBL 期写是安全的，重置 w 的是 $2002 读）。
- **OAMADDR 帧首行为**：硬件在 pre-render 的 sprite 评估前把 OAMADDR 置 0（blargg
  实测：每帧 sprite 评估从 OAM 头开始），但**只在渲染开启时**。
- **greyscale（$2001 bit0）**：调色板索引 AND 0x30 后再输出。
- **emphasis（$2001 bit5-7）**：色调加权，NTSC/PAL 行为不同（PAL 红/蓝交换）。

### 1.7 上电状态（Wiki `PPU_power_up_state`，blargg 2008 逆向）

- 上电 `PPUCTRL=$00, PPUMASK=$00, PPUSTATUS=$A0(±)，OAMADDR=$00`；帧以「上电后第一帧
  无法渲染、部分寄存器写被忽略」开始（ppudead 期）。
- reset **不**清 VRAM/OAM/palette，只重置寄存器与 w 触发器。

---

## 二、FCEUX11 具体设计

### 2.1 双引擎架构

`int newppu` 切换（`src/ppu.cpp:314` 附近；分发在 `src/ppu_rendering.cpp:1175-1179`）：

| | 旧 PPU（`FCEUPPU_Loop`） | 新 PPU（`FCEUX_PPU_Loop`） |
|---|---|---|
| 驱动模型 | 逐 scanline，惰性区间渲染 | 逐 dot（`runppu(1)`） |
| CPU 交错 | 每行 `X6502_Run(256)` / `X6502_Run(85)` 分段 | 每 dot 同步放行 CPU（3 dot = 1 cycle） |
| 背景实现 | 伪 shift register（`pputile.inc`，LUT 位展开） | `BGData::Record::Read` 按硬件 fetch 时序逐 dot |
| 精度定位 | 兼容/快速路径 | blargg 收敛主战场（`newppu=1` 为 baseline） |
| 游戏回归锚 | Knight Rider→ppudead、Super Donkey Kong→OAMADDR 清零、3-D WorldRunner→dot257、SMB3/Crystalis→MMC3 hook 时机 | — |

时钟基数：**1 CPU 周期 = 48 内部单位，1 PPU dot = 16 单位**（NTSC；PAL = 15/45）。
`X6502_Run(n)` 的 `n` 实际是「PPU dot 数」而非 CPU 周期（`src/x6502.cpp:498-508`、
`src/cpu.h:105-107`）。`GETLASTPIXEL = (timestamp*48 − linestartts) >> 4` 把 CPU 单位换回 dot。

### 2.2 状态组织

- 全部 PPU 寄存器与渲染热状态集中在 `fceu11::Ppu g_ppu`（`src/ppu_class.h:160-335`），
  旧全局名（`PPU[4]`、`vnapage`、`RefreshAddr`、`XOffset`…）是别名引用；**热字段布局
  曾经导致 bench +15% 回退**，新增字段前先看 `ppu_class.h:276-308` 的缓存行注释。
- Loopy 寄存器用 `PPUREGS`（fv/v/h/vt/ht 五计数器 + 锁存副本，`ppu_class.h:107-156`），
  提供 `increment_hsc`（每 tile）/`increment_vs`（dot 251）/`install_h_latches`（dot 257）/
  `install_latches`（pre-render dot 280-304）。
- $2002/$2004/$2007 等 handler 在 `src/ppu.cpp:599-1151`；CHR/NT 可写性由
  `PPUCHRRAM/PPUNTARAM` 位掩码门控（`ppu.cpp:175-201, 276-298`）。

### 2.3 背景渲染两套实现

- 旧 PPU：`RefreshLine` 是**惰性区间渲染**——CPU 任意读写 PPU 寄存器前先
  `FCEUPPU_LineUpdate()` 补画到当前 dot（`ppu_rendering.cpp:234-245`）。tile 解码走
  `ppulut1/2`（位展开）+ `ppulut3`（fine-X）三张 LUT（makeppulut，`ppu_rendering.cpp:101-124`），
  `pshift[0/1]` 伪移位器每 tile 整字节灌入。模板化复刻在 `src/pputile_template.cpp`。
- 新 PPU：`BGData::Record::Read`（`ppu_rendering.cpp:1414-1493`）逐 dot 记录 NT/AT/平面
  读取，**每个 dot 记录当时的 $2001**（grayscale/deemph 逐像素时间戳，SoA 布局
  `bgdata.ppu1[slot][8]`）；行末 dot 337/339 的两条 prefetch 对应 `bgdata.main[0..1]`，
  渲染循环用 `xt+2` 偏移消费。

### 2.4 精灵管线

- 双缓冲 OAM 乒乓（`oams[2][64][8]`，`ppu_rendering.cpp:1725-1727`）。
- 解码走 65536 项 `kSpriteIdxLUT`（pattern 字节对 → 8 像素 2bit 索引打包 uint64，
  `src/ppu_sprite_lut.h`），H-flip 用字节反转。
- sprite 0 hit：新 PPU 在像素循环内联判定（`ppu_rendering.cpp:1903-1906`，dot 级精度）；
  旧 PPU 在行尾 `CheckSpriteHit`。
- **overflow bug 未复现**（只做正确的 ns>8 判定，`ppu_rendering.cpp:1020`）——若未来
  补做，必须先过 `oam_stress.nes`。
- MMC3 IRQ 的 A12 hook 在 sprite fetch `s==2` 处（`ppu_rendering.cpp:2060-2074`），
  条件是 pattern 使能位（**不是** sprite 使能位——Dragon's Lair 依赖放宽判定）。

### 2.5 Mirroring

- PPU 侧固定 2KB NTARAM + 4 指针 `vnapage[4]`；`Ppu::set_mirror_mode`（0=H、1=V、2/3=单屏，
  `src/ppu_class.cpp:121-141`）+ `set_mirror_pages`（四页任意映射）。
- mapper 改 mirroring 前必须 `notify_line_update()` 刷惰性渲染（`ppu_class.cpp:157-163`）。

### 2.6 调色板 / deemphasis

- PALRAM 32B，写统一 `& 0x3F`；UPALRAM[3] 保存 $3F04/08/0C 读回镜像。
- grayscale：读路径 `& 0x30`（`ppu.cpp:70-71`）；新 PPU 逐像素按当 dot 的 $2001。
- emphasis 用 8bpp 索引高位（0x40/0x80/0xC0）+ `ApplyDeemphasis*` 系列查表
  （`src/palette.cpp:97/184/263/287`，Complete 版走 Rust FFI）；PAL 机型红蓝交换
  `paldeemphswap`（B2001，`ppu.cpp:984-985`）。

---

## 三、经验与魔数（开发者备忘）

| 魔数/事实 | 含义与出处 |
|---|---|
| `PPU_OPEN_BUS_DECAY_CYCLES = 1073864`（`ppu.cpp:475-495`） | ≈600ms open-bus 衰减，blargg `ppu_open_bus` 逐 test 修复产物；写刷新全 8 位，$2004 读用 OAM 字节刷新，$2007 palette 读高 2 位用旧 latch |
| NMI 延迟 `nd=8` dot（`ppu_rendering.cpp:1539-1545`） | 经验校准值（env 探针 `FCEUX11_E1_NMIDELAY` 可调；7/8/9 差异见 §三.2）；是 `vbl_05` 已知 FAIL 的根因之一 |
| VBL 置位在 cy0 而非 cy1（`ppu_rendering.cpp:1609-1627`） | 「Cycle 0→1 shift deferred」；`vbl_02_set_time` FAIL 的根因，修复需平移整个 VBL 事件链 |
| odd skip 只在 sl==0 判定（`ppu_rendering.cpp:2134-2138`） | `vbl_10_even_odd_timing` FAIL 的根因；`ppu_rendering.cpp:2057` 跳点块是**禁忌保留项**（勿改） |
| OAMADDR 帧首清零（`ppu_rendering.cpp:1217/1632`） | 带「breaks Super Donkey Kong」注释的 hack；改动前必跑该游戏锚 |
| $2004 属性位 `V &= 0xE3`、`PPUSPL = V & 7`（`ppu.cpp:1002-1014`) | OAM 未连接位与评估泄漏的近似 |
| 调色板「透明标记」`PALRAM |= 0x40...`（`ppu_rendering.cpp:442-445`） | 不能合并成 32 位写（别名/布局陷阱，热修复注释在 :422-441） |
| `runppu` 回绕分支替代 `%341`（`ppu_rendering.cpp:1366-1375`） | 性能模式：PPU 主循环禁止取模 |
| sprite 评估 X-bucket 预分类（`ppu_rendering.cpp:1739-1810`） | 64→8 筛选的 O(1) 化；新增评估特性需同步两个槽 |
| $4014 SPR DMA 无 DMC 仲裁（`ppu.cpp:1134-1142`） | `sprdma_dmc_dma` FAIL；补齐需 Mesen2 级逐周期 DMA 状态机，见 precision.md |

## 四、相关文档

- 精度已知限制与测试治理：[precision.md](precision.md)
- 模数总表：[constants.md](constants.md)
- MMC3 IRQ 与 mapper 交互：[cartridge.md](cartridge.md) §B
- 调查数据链：`docs/history/surveys/e1_vbl/`（VBL/NMI 探针）、`docs/history/surveys/r5r6_v1.17/`
- KagamiQA 测试框架：[KagamiQA.md](KagamiQA.md)
