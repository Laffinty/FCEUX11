<!--
source: SourMesen/Mesen2, quietust/nintendulator, ares-emulator/ares, punesemu/puNES, 0ldsk00l/nestopia, TASEmulators/fceux（各仓库源码与文档）
retrieved: 2026-09-13
conversion: compiled research note（全部为行为事实/设计决策的转述与摘录；未复制任何源码。许可约束见 §6）
-->

# 逐 dot PPU 参照设计综述（Mesen2 / Nintendulator / ares / puNES / Nestopia / FCEUX）

> 用途：为 v2.1.3 逐 dot 渲染管线与逐周期调度提供**交叉验证**（README §6 预告的来源类型）。不作唯一依据——硬件事实以 `ppu/`、`cart/` 目录的 Wiki 存档为准。
> 检索日 2026-09-13。本环境 nesdev.org 直接抓取被 Cloudflare 403 拦截，论坛引文经 web.archive.org 快照核验。

## 1. 三类 CPU-PPU 同步架构

| 架构 | 代表 | 说明 | mid-scanline 副作用能力 |
|---|---|---|---|
| catch-up-on-access | 旧 FCEUX（`FCEUPPU_LineUpdate` 由寄存器处理器触发）、Nestopia（CPU 内存钩子里 `cpu.Update()` + PPU `Sync` 钩子，内部仍逐 dot `Run()`） | CPU 跑一段，寄存器访问时把 PPU 追到当前时刻 | 取决于内部粒度 |
| call-per-CPU-cycle | puNES `ppu_tick`（累加 cpu_divide，内部 while 循环逐 dot）、FCEUX new PPU（`runppu(1)`，34 个 BGData 记录 × 每 record 8 次） | 每 CPU 周期调用一次，内部是真逐 dot | CPU 周期粒度 |
| master-clock lockstep | Mesen2（`Run(runTo)` 驱动逐 dot `Exec()`）、ares（每 dot `Thread::synchronize(cpu)` 协同切换）、Nintendulator（逐 dot switch 嵌在 CPU 执行内） | 主时钟统一驱动，无追赶 | 亚 CPU 周期（唯一能表达"$2006 第二写延迟 3 dot"这类 dot 级延迟） |

>Quietust（Nintendulator 作者，论坛 t=20782）：门级每 dot 重算是"当今微处理器不可行"的，精确模拟器都按 dot 粒度建模但缓存派生值提速（例如 `(Reg2001 & 0x18) && (Scanline < 240)` 只在换行或写 $2001 时更新）。这与本仓 v2.1.2 的门控缓存做法一致。

## 2. 各实现速览

### 2.1 Mesen2（SourMesen/Mesen2，**GPL-3.0，只读学习**）

- 核心：`Core/NES/NesPpu.cpp/.h`（+ `BaseNesPpu.*`）。逐 dot `Exec()`；主时钟 21.47 MHz：12 tick/CPU 周期、4 tick/dot。
- 取指：`LoadTileInfo`/`LoadSpriteTileInfo` 在精确 dot 做 NT/AT/PT 取指；`ShiftTileRegisters` 逐 dot 推进移位寄存器；`IncHorizontalScrolling`/`IncVerticalScrolling` 按 loopy 语义。
- **寄存器写时序**（对实现最有参考价值的一组决策）：
  - $2000/$2005：**当前 dot 立即生效**；写落在可见扫描线 dot 257 时把 open bus 位拼进活跃 `v`（scroll-glitch 路径，`ProcessTmpAddrScrollGlitch`）。
  - $2006：第一写立即；**第二写延迟 3 个 PPU dot 生效**（`_updateVramAddrDelay = 3`，注释归功 VisualNES 的发现）；另有按 dot 条件的 AND 掩码 quirk。
  - $2007：写当前 dot 生效但 **v 递增推迟到下一 dot**（`_needVideoRamIncrement`）；6 dot 内连续 $2007 读被忽略（`_ignoreVramRead`）。
- 取指原子性：`ReadVram/WriteVram` 先 `SetBusAddress(addr)` 再访问，同一 dot 内原子；扫描线 0 dot 0 与 vblank 开始时主动置总线地址；渲染中途关闭时总线地址复位为 v。
- **mapper 观察点**：`SetBusAddress()` 检查 `_mapper->HasVramAddressHook()` 后调 `NotifyVramAddressChange(addr)`——每次真实取指、$2007 驱动的地址变化、延迟 v 更新都会触发（MMC3 计数器依赖此路径，含 257/265/… dummy NT 取指与精灵 $FF tile 取指）。
- 精灵：`ProcessSpriteEvaluation` 逐 dot 增量（边界走 `__noinline` 首尾函数）；`LoadExtraSprites` 处理 8 精灵上限与 overflow；**OAM 衰减/损坏建模**（`OamDecayCycleCount = 3000`，`ProcessOamCorruption`）。
- 帧时序：pre-render 标志 dot 1 清、280-304 垂直复制、OAM 行拷贝 quirk；NMI 241（NTSC）/291（Dendy），$2002 早 1 dot 读时 `_preventVblFlag` 抑制；**奇数帧跳点实现为 pre-render 339 → 340**（PAL 无跳点）。
- open bus：逐寄存器 `SetOpenBus/ApplyOpenBus`、$2002 状态位 open bus 拼接。

### 2.2 Nintendulator（quietust/nintendulator，GPL-2.0）

- `src/PPU.cpp`：**逐 dot 硬编码 switch 状态机**（`RunNoSkip/RunSkip`），每个 dot 的取指行为写在对应 case 里；Y 递增在 dot 255、滚动重载在 pre-render 279-303（读取时点语义与本仓 dot 256/257 记法差 1，建模对象相同）。
- 固定 4 项流水 `RenderData[4]`：取指字节在几 dot 后被消费；`TileData` + fine-X `IntX` 构成结构性"取指→出像素"滞后；注释明确 sprite-0 hit"晚 1 像素触发"、VBL 标志"晚 1 周期清"。
- CPU 寄存器写：$2000/$2001/$2005/$2006 **立即**；$2007 经 `IOMode` 倒计时（写=6、读=5 个 PPU 周期）建模内部访问延迟后才上总线并递增。
- 精灵评估：`ProcessSprites()` dot 驱动 FSM（sprstate 0-4）：dots 1-64 清 secondary OAM（含 pre-render 行拷贝 sprite-address 页的硬件 quirk）、65-256 Y 范围检查（复刻 overflow bug）、hblank 前"thrashing"；精灵取指嵌在同一 dot switch 的 257-319。
- `doc/readme.txt`：GPL-2.0 分发；**mapper 接口定义是 public domain**（本仓 C++ 侧的 GameHBIRQHook 形制与之同源）。

### 2.3 ares（ares-emulator/ares，**ISC——许可最宽松**）

- `ares/fc/ppu/ppu.cpp`：`PPU::step(u32)` 每 dot 一次；**每 dot 后 `Thread::synchronize(cpu)` 协同让出**（libco 式），CPU 与 PPU 显式交替。
- 取指状态机在 `scroll.cpp`（cycleScroll）/`sprite.cpp`（cycleSpriteEvaluation）/`render.cpp`；全部以 `enable()`（渲染开启）为条件——渲染关闭时取指循环停止。
- 渲染**关闭**时 $2007 独占总线：直接 v 递增并立即 `cartridge.ppuAddressBus()` 通知；**开启**时递增切换为取指电路共用的 loopy X/Y 路径（同一 v）。
- $2007 读返回缓冲 `latchData`（调色板读不缓冲，与 open bus `io.mdr & 0xC0` 或接）；`blockingRead = 6` 节流连续访问。
- mid-scanline quirk 齐全：$2000 写在 lx==257 拼接 open bus 位进 nametableX；$2005/$2006 第一写在 lx 257 损坏 tileX/nametable；渲染期 $2004 写只做高 6 位 OAMADDR 毛刺递增。
- mapper 总线可见性：每次 CPU 发起的 $2007 访问都发布 `io.busAddress`（`cartridge.ppuAddressBus()`）；逐取指 CHR/NMT 路由在 cartridge 层决策。

### 2.4 puNES（punesemu/puNES，GPL-2.0）

- `src/core/ppu.c`：CPU 周期调用 + 内部逐 dot while；寄存器状态带**竞态影子字段**（`r2000.race.ctrl`、`r2006.race.value`、`r2006.second_write.delay`）——第二写延迟倒计时、dot 257 的 $2000 竞态、取指期间 `r2006.race` 替换、254/255 特判（对着 `split_scroll_test_v2.nes` 调出来的）。
- 取指相位按 `pixel_tile` 0-7：AT 在 1、PT 低在 3、PT 高在 5 + v 递增；Y 递增 dot 253；253-319 精灵取指逐 dot 更新 $2004；320-340 下行预取 + 337/339 garbage NT；奇帧 SHORT_SLINE_CYCLES。
- 精灵评估 4 相 FSM（dots 64-255；相 1+2 收集 ≤8 个；相 3 = 硬件 overflow bug 扫描；相 4 = 2 周期重复）；OAM 损坏（`OAMADDR & 0xF8` 对拷）；dot 319 of scanline 238 强制 $2003 清零。
- **mapper 按 dot 窗口挂钩**：`extcl_ppu_000_to_34x / 000_to_255 / 256_to_319 / 320_to_34x / after_rd_chr`；`irqA12.c` 就是独立的 A12 边沿 MMC3 计数器，`irql2f.c` 是 L2F 变体。
- `ppu_alignment` 设置（默认/随机/每次复位递增）——对 CPU-PPU 上电对齐问题（见 §4）的直接回应。

### 2.5 Nestopia UE（0ldsk00l/nestopia，GPL-2.0）

- `source/core/NstPpu.hpp`：硬件块结构体（Regs/Scroll/Tiles/Chr/Nmt/Palette/Oam/Output/Io）+ dot 时钟（`cycles.hClock/vClock`）；渲染逐 dot（`RenderPixel/FetchName/FetchAttribute/FetchPattern` 强制内联）；精灵评估 10 相成员函数指针 FSM（含奇偶周期 OAM 行为、`CorruptOam`）。
- open bus 逐寄存器时间戳衰减（`decay.timestamp[8]`、`OpenBusDecayCycles`）。
- 事件回调式交错：`ppu.BeginFrame → cpu.ExecuteFrame → ppu.EndFrame`，CPU 时间基内做周期级 catch-up。

### 2.6 FCEUX（上游，GPL-2.0，本项目上游）

- 上游同时存在两套 PPU（`src/ppu.cpp`；**没有** `src/ppu_rendering.cpp`，那是本仓自己的文件名）：
  - **旧 PPU**：`FCEUPPU_Loop` + `DoLine`/`RefreshLine`，寄存器处理器里 `FCEUPPU_LineUpdate()` 追赶渲染已过的像素——本仓继承的算法血统（`pputile.inc` 的 `pshift[2]`/`atlatch` 函数级 static，跨扫描线存续）。
  - **new PPU**：`FCEUX_PPU_Loop` 每扫描线读 34 个 `BGData::Record`，每 record 调 `runppu(1)` 八次——**本质是逐 CPU 周期推进 `ppur.status.cycle`**，并非"每扫描线一调用"。
- mapper 钩子：`PPU_hook(uint32 A)`（PPU 总线地址，读写取指都报）、`GameHBIRQHook()`（旧 PPU 在 DoLine，$(PPU[0]&0x38)==0x18 时抑制；new PPU 在精灵取指 s==2）、`MMC5_hb(scanline)`。
- 上游代码内自认的精度局限（原注释转述）：旧 $2004 读只回 `PPUGenLatch`；emphasis 处理"clunky"；行尾 dummy NT 取指未实现；VBL/NMI 时序"probably off"（new PPU 用 20 周期延迟因为 12 会坏游戏）；渲染期 $2007 递增永远垂直（Tecmo Super Bowl / P'radikus Conflict 修正）；扫描线计数按 Crystalis/Kirby/SMB3 经验调参；Star Trek: 25th Anniversary、SDF 分屏等游戏特例。
- **atlatch quirk 的出处与定案**：`pputile.inc` 的 `pixdata |= ppulut3[XOffset | (atlatch << 3)]` 与 `atlatch >>= 2; atlatch |= cc << 2`——渲染 tile 的属性是**上一次取指**的属性（半 tile 延迟效应），且 static 跨行存续。**上游无任何注释记载此行为**；本仓 `rendering.rs` 头注释与 `ppu_class.h`（`bg_latch_`）是目前唯一的成文记录。**是否硬件行为的定案（2026-09-13，交叉验证）：不符合**——本仓存档的 Wiki《PPU rendering》明确"NT/AT/PT 四次取指针对同一 tile ID、在 8th dot 一并装载（属性进 2 个 1-bit 锁存喂 8-bit 移位寄存器），每个 8 像素串被迫共享同一属性"；Mesen2/Nintendulator/ares/puNES 四家参照实现均为均匀属性模型。FCEUX 的 atlatch 是其 tile 循环算法的历史近似，在属性象限边界处有 4 像素级的属性错位。v2.1.3 批次 2 移除该 quirk（golden 基线重生须 owner 批准）。

## 3. 对 v2.1.3 的落地建议（从已验证事实蒸馏）

1. 逐 dot `Exec()` + 扫描线内状态机是四家参照的共同选择；取指在精确 dot、像素输出经移位寄存器管线（取指→出像素结构性滞后必须建模，不能取指即出像素）。
2. 写语义组合：$2000/$2005 立即 + dot 257 open-bus 拼接 quirk；$2006 第二写延迟（Mesen 3 dot）；$2007 立即写 + 递增延迟 1 dot + 连续读节流（6 dot / IOMode 6·5 / blockingRead 6——三家取值一致地落在 5-6 PPU 周期）。
3. A12/mapper 观察点做成"每次总线地址变化"回调（Mesen `NotifyVramAddressChange`、ares `ppuAddressBus`、puNES dot 窗口钩子、上游 FCEUX `PPU_hook`），而不是扫描线级钩子——这是本仓批次 1 的方向。
4. 精灵评估做逐 dot 增量 FSM + overflow bug 计数器 + OAM 衰减/损坏建模；OAM 衰减量级参照 Mesen 的 3000 周期。
5. 渲染关闭路径：$2007 独占总线、v 直递增、总线地址对外可见（ares 语义最清晰），与本仓"快照窗口退化为 blank 路径"的规划一致。
6. CPU-PPU 上电对齐：硬件本身有 4 种对齐（oam_stress readme），精确模拟器或固定一种或暴露选项（puNES）；本仓测试门禁解读 oam_stress 结果时必须记住这一前提。

## 4. 许可与"能做什么"

| 来源 | 许可 | 对本项目（GPLv2） |
|---|---|---|
| Mesen 1 / Mesen2（均已归档只读） | **GPL-3.0** | **只学事实，禁止复制代码**（GPL-3 与 GPLv2 不兼容）；设计转述自由 |
| Nintendulator | GPL-2.0（readme）；mapper 接口定义 public domain | 设计研究自由；移植代码合法（GPLv2+署名）；mapper 接口无约束 |
| ares | **ISC** | 复用代码合法（保留版权与许可声明） |
| higan（历史血统） | 未在本轮核验（历史上 GPLv3） | 按 GPL-3 对待：只学习 |
| puNES / Nestopia UE | GPL-2.0 | 代码复用合法（GPLv2+署名） |
| FCEUX 上游 | GPL-2.0 | 本项目上游，按 GPL 正常复用 |
| 本仓 C++ 遗留参照（ppu_rendering.cpp 等） | 本项目自有 | 字节级基准已退役，行为参照仍有效（v2.1.2 先例） |

## 5. 来源

- Mesen2：https://github.com/SourMesen/Mesen2（Core/NES/NesPpu.cpp/.h、NesCpu.cpp；仓库 GPL-3.0，已归档）
- Nintendulator：https://github.com/quietust/nintendulator（src/PPU.cpp、src/NES.cpp、doc/readme.txt）；官网 http://www.qmtpro.com/~nes/nintendulator/ ；论坛 t=20782（Wayback）
- ares：https://github.com/ares-emulator/ares（ares/fc/ppu/ppu.cpp、memory.cpp、scroll.cpp、sprite.cpp；LICENSE）
- puNES：https://github.com/punesemu/puNES（src/core/ppu.c、ppu_inline.h、irqA12.c、irql2f.c；COPYING）
- Nestopia UE：https://github.com/0ldsk00l/nestopia（source/core/NstPpu.hpp、NstMachine.cpp）
- FCEUX：https://github.com/TASEmulators/fceux（src/ppu.cpp、src/pputile.inc）
- 经典文档：loopy《The Skinny on NES Scrolling》（1999-04-13 邮件；https://3dscapture.com/NES/skinny.txt 等镜像；正文无版权声明，转述需署名）；Brad Taylor《2C02 Technical Reference》（2004-04-23 第 5 版；nesdev.org/2C02%20technical%20reference.TXT；无版权声明，转述需署名）
