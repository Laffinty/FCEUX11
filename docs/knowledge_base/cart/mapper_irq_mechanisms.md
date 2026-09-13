<!--
source: https://www.nesdev.org/wiki/MMC3 (+ MMC6/MMC5/VRC4/VRC6/VRC7/Sunsoft FME-7/JY Company/GTROM/PPU_OAM/PPU_rendering/Tricky_to_emulate_games,
        forum.nesdev.org threads, furrtek/VGChips MMC3C, SourMesen/Mesen2 source, christopherpow/nes-test-roms readmes)
retrieved: 2026-09-13
conversion: compiled research note (facts paraphrased with attribution; Mesen2 items are behavioral facts extracted from GPL source, no code copied)
-->

# Mapper IRQ 机制分类与 MMC3 族 A12 时序（多信源汇编）

> 本页是多信源汇编（NESdev Wiki、官方论坛实测帖、VGChips 硅片逆向、Mesen2 参照实现、blargg 测试套件 readme），与逐字存档的 `mapper_mmc3.md` 互补。目标读者：A12 watcher 与 IRQ 计数器的实现者。
> 说明：本环境直接抓取 nesdev.org 被 Cloudflare 403 拦截，Wiki 内容经 web.archive.org 2024 快照与 nesdev-wiki.nes.science 镜像核验，检索日 2026-09-13。

## 1. IRQ 机制分类（谁需要 PPU 侧时序）

| 机制 | Mapper | 计数源 | 实现含义 |
|---|---|---|---|
| **A12 边沿（滤波）** | MMC3 / MMC6 及其修订 | 过滤后的 PPU A12 上升沿 | 必须实现取指地址级 A12 watcher |
| **A12 边沿（无滤波/多源）** | J.Y. Company ASIC（90/209/211 等） | $C001 可选源：CPU M2 上升、**PPU A12 上升（无滤波，每行约 8 次）**、PPU 读（每行 170 次）、CPU 写；另有可编程预除器 | 与 MMC3 滤波完全不同，不能复用 |
| **A12 ÷8 预除器** | Acclaim MC-ACC（MMC3 克隆） | 原始 A12 下降沿 ÷8 预除器（非滤波） | 不仿真预除器则 Mickey's Safari in Letterland 渲染错误 |
| **取指侦听（无 IRQ）** | MMC2（Punch-Out!!）、MMC4（Fire Emblem） | CHR 取指地址落进特定范围触发 bank 锁存（MMC4: $0FD8/$0FE8/$1FD8/$1FE8 系列，下一次 VRAM 地址通知时生效） | 需要 per-fetch 地址可见；MMC2 需要仿真第 34 个 tile 取指 |
| **真扫描线计数（无 A12）** | MMC5 | 侦听"连续 3 次 $2xxx 命名表读 + 后续属性取指"判行；In Frame 标志 $5204 bit6；IRQ 在 **PPU dot 4**（MMC3 典型 dot 260） | 需要 per-fetch 地址可见 + 渲染启停侦听 |
| **CPU 周期定时器** | VRC4、VRC6、VRC7、Sunsoft FME-7 | 每 CPU 周期递减（VRC6 的"扫描线模式"= CPU 周期倍数 ≈113.66/行，会与真实扫描线漂移） | 只需逐周期 CPU 调度，不需要 A12 |
| **无 IRQ** | VRC2、GTROM(111)、DxROM/Namco 108、MMC1 | 无 | 无 |
| **MMC1 A12 特例（无 IRQ）** | MMC1（部分板） | 寄存器 2 的 WRAM 禁用**只在精灵 pattern 取指（A12=1）期间生效**，$6000 读可当软件扫描线计数器 | 忽略此行为会在 MMC1_A12 测试中冻结 |

## 2. MMC3 / MMC6 详解

### 2.1 过滤电路（硬件依据）

- MMC3 的"扫描线计数器"本质是**过滤后的 PPU A12 上升沿计数器**：A12 需保持低电平跨越 **3 个 M2 下降沿**后才接受上升沿（Furrtek 对 MMC3C 的硅片逆向："/C16 是 IRQ 时钟；IRQ 定时器在 PPU_A12 低电平跨越 3 个 M2 下降沿后才被触发"）。
- Talk:MMC3 的实测边界：有人测得"低电平期间含 2 个 M2 上升沿"即可计数（≈2 个 CPU 周期低电平）；lidnariq 给出上限"肯定不超过 64 像素"，且 nes-test-roms 的 MMC3 IRQ 测试 3.4 要求滤波**不超过约 22 个 CPU 周期**。
- A12→/IRQ 引脚还有约 **69 ns** 的模拟传播延迟（SMB3 MMC3B 实测，约 1.5 个主时钟周期，小于 1 dot / 1 CPU 周期）；是否按 1 周期建模对个别对齐敏感的 ROM 有影响。

### 2.2 计数器语义

- 寄存器（$C000-$FFFF 偶/奇）：$C000=IRQ 锁存值；$C001=reload（清计数器并置 reload 标志）；$E000=**禁用并应答**；$E001=使能。
- 每次被时钟：counter==0 或 reload 标志置位 → 从锁存值重载；否则递减。counter==0 且使能时拉 IRQ。相邻 IRQ 间隔 = N+1 条扫描线（N=$C000 值）。
- 锁存值在**下一次 A12 上升沿**才拷入计数器（Wiki 推测在当前扫描线 PPU dot 260 附近）。
- 计数器**无法停止**：$E000/$E001 只屏蔽/放行 IRQ 输出；渲染关闭期间只要有 A12 翻转（含 $2006/$2007 引起的）它照样计数。
- **CPU 发起的 PPU 总线访问会时钟计数器**：blargg 测试 3 验证 $2006 第二次写、$2007 读、$2007 写造成的 A12 0→1 跳变都会计数（1→0 不计），无论渲染是否开启。

### 2.3 修订版差异（Rev A / Rev B）

- **Sharp 产（MMC3C、带粗体 S 的 MMC3B）= Rev B**："counter == 0" 判定。$C000=$00 时**每**扫描线都触发 IRQ；有效范围 1-256。
- **NEC 产（MMC3A、无 S 的 MMC3B）= Rev A**："递减**到** 0" 判定（检查 1→0 转变）。$C000=$00 只触发一次 IRQ（重写 $C001 可再触发）；有效范围 2-256。
- blargg mmc3_test_2 readme 的表述：Rev B（SMB3、Mega Man 3 板验证）写 0 到 $C000 与其他值无差别，计数器到零后每次时钟都重载；Rev A（Crystalis 板）在"清除后重载到零"时触发 IRQ，但"正常递减到零后的重载"不触发。
- Mesen2 的区分方式：NES 2.0 数据库芯片名以 "MMC3A" 开头时置 `_forceMmc3RevAIrqs`；Rev A 判定式为 `(count>0 || reload) && counter==0 && enabled`。
- Star Trek: 25th Anniversary 需要 Sharp（Rev B）行为；Felix the Cat 据称是 MMC3A。$C001 后紧接时钟再 $C001 的病态序列（blargg 的 "N-K" 测试）真实硬件行为未完全成文，**明确不要求仿真**。

### 2.4 MMC6

- StarTropics / Zoda's Revenge（NES 2.0 submapper 1）。IRQ 行为与 MMC3C 相同；差异仅在 1 KiB 内部 PRG RAM 的 512 字节级读写使能（$A001 bit3-0；$8000 bit4/5 总开关），禁用时 mapper 持续复位 $A001。

## 3. A12 线行为细节

- 渲染开启时，总线按取指时刻表承载真实地址：BG/NT 取指 dots 1-256 与 321-336，dummy NT 取指 337-340，**精灵 pattern 取指 257-320（即使没有精灵也取，空槽取 tile $FF，MMC3 照样计数）**；dot 0 空闲，呈现的是 dot 5 将用的 CHR 地址。
- **vblank/强制消隐期没有取指，但 v 寄存器的值直接输出在 PPU 地址引脚上**——因此消隐期每次 $2006 写 / $2007 读写都会让 mapper 看到 A12 翻转（blargg 测试 3 验证；Mesen2 在 $2007 递增后与渲染关闭时的 $2006 写后把 v 推给 SetBusAddress）。
- **精灵 pattern 表选择**：8x8 模式用 $2000 bit3；8x16 模式忽略 bit3，由**精灵 tile 索引 bit0** 逐精灵决定（下半块 tile N+1 取自同一表）。同一行不同精灵可用不同表 → 257-320 窗口可产生多个 A12 边沿（Wiki：每行最多 4 次），8x16 下必须显式跟踪 A12，精灵取指最多把计数器时钟 4 次/行，IRQ 可提前/推迟 8 像素的整数倍。
- 标准配置（BG $0000 + 精灵 $1000）→ 每行恰好一次振荡，**每帧 241 次计数**（blargg 测试 2 验证）。反配置（BG $1000 + 精灵 $0000）→ 计数发生在**上一条扫描线的 dot 324 附近**，且奇数帧跳 dot 会造成第一条扫描线偶发双计数（Wario's Woods 的草地铁丝网右端 48 像素闪烁即此）。
- 渲染中途开关渲染时，读地址由"锁存的低 8 位 + 当前高 6 位"拼成（混合地址），Battletoads / Bill & Ted's Excellent Adventure 这类中途开关渲染换 CHR 的游戏会触及。

## 4. 参照实现行为（Mesen2，GPL——仅提取行为事实，未复制代码）

- `Core/NES/Mappers/A12Watcher.h`：通用 watcher，`UpdateVramAddress<minDelay=10>`——A12 低电平持续**超过 minDelay（默认 10 个 PPU dot）**才上报 Rise；Fall 检测跨帧累计低电平时长。minDelay 是模板参数，逐 mapper 定制（RAMBO-1 用 30）。
- `Core/NES/Mappers/Nintendo/MMC3.h`：MMC3 **不用**通用 watcher，内联滤波 `IsA12RisingEdge` 要求 A12 低电平 ≥3 个 master clock（Mesen2 主时钟 21.47 MHz：12 tick/CPU 周期、4 tick/dot，即 0.75 dot 的最小间隔）。上升沿时：counter==0 或 reload → 重载，否则递减；RevB 判 `counter==0 && enabled`，RevA 判递减到零。
- `Core/NES/Mappers/Tengen/Rambo1.h`：RAMBO-1（mapper 64）用 minDelay=30 的 watcher，IRQ 延迟 2 CPU 周期断言，reload 补 +1/+2（"Fixes Hard Drivin'"）。
- 侦听点：`NesPpu::SetBusAddress(addr)` 在渲染期每次取指地址变化时通知 mapper；渲染关闭时在 $2007 递增后与 $2006 写后把 v 推上总线。
- MMC3 克隆族在 `Core/NES/Mappers/Mmc3Variants/`（MC-ACC、114/115/121/182/187/198 等 46 个头文件）。

> 与本仓滤波参数的换算：硬件 3 个 M2 下降沿 ≈ 3 个 CPU 周期 ≈ 9 PPU dot；blargg 上限约 22 CPU 周期 ≈ 66 dot；Mesen2 MMC3 = 0.75 dot、通用 watcher 默认 10 dot。本仓取值应落在 [≈2 dot, 66 dot] 区间内并以 blargg 套件逐项校准（见 §6）。

## 5. 依赖此精度的游戏（Tricky-to-emulate games 摘录）

| 游戏 | 依赖点 | 失真表现 |
|---|---|---|
| Crystalis (MMC3) | 扫描线 IRQ 做移动分割 | 分割线接缝游走（blargg Rev A 参照板） |
| Jurassic Park (MMC3) | IRQ 延迟敏感 | OCEAN logo 滚动时损坏 |
| Wario's Woods (MMC3) | 反配置 + 奇偶帧对齐 | 右端约 48 px 草地闪烁 |
| G.I. Joe / Mickey in Letterland (MMC3) | 精灵关闭时精灵取指仍须时钟计数器 | 计数丢失 |
| Star Trek: 25th Anniversary (MMC3) | 扫描线 0 触发 IRQ + Sharp 修订行为 | 依赖 Rev B 判定 |
| StarTropics (MMC6) | 精确 IRQ 时机（渲染关闭改调色板后行中重开精灵）+ OAM 损坏交互 | 中途渲染状态错 |
| Super Mario Bros. 3 (MMC3, Rev B 参照板) | 精灵优先级/OAM 索引交互；帧中连续 4 次 $2006 写（调色板指针 + 滚动复位），依赖渲染关闭时 backdrop=v 处调色板色 | 状态栏与背景 bank 错、底色错 |
| Marble Madness / Mother (J) / Pirates | **扫描线中途**切换 CHR bank 画文本框 | 文本框贴图错（取指约超前 2 tile，写入在后续 tile 边界生效） |
| Bill & Ted's Excellent Adventure / Battletoads | 中途开关渲染（Battletoads 另需 sprite-0 + 精确 CPU/PPU 时序） | 画面撕裂/冻结 |
| The Young Indiana Jones Chronicles / Zelda II | 渲染期 $2007 读副作用改变滚动 | Y 滚动抖动 |
| Burai Fighter (U) | 渲染期 $2007 写画记分条 | 记分条被裁半 |
| Balloon Fight / Micro Machines / Daydreamin' Davey / Stunt Kids / Rollerblade Racer | 渲染期 $2007 读 / $2004 读 / 精灵-only 状态栏 / 渲染关闭 backdrop=v 花招 | 各类状态栏与星点效果异常 |

社区流传但**未找到 nesdev 一手信源**的：Bases Loaded(1)、720 Degrees 的行中分割写；Metal Storm 的帧中换 bank 说法可追溯到 AI 生成文章，不可靠（Metal Storm 实为 MMC3 板）。

## 6. 测试 ROM（christopherpow/nes-test-roms）

- `mmc3_irq_tests/`（1.Clocking、2.Details、3.A12_clocking、4.Scanline_timing、5.MMC3_rev_A、6.MMC3_rev_B）：
  - 1：$2006 翻转 A12 递减计数器；仅写 $C000/$C001 不重载；到零重载；IRQ 标志仅在使能时置位。
  - 2：reload=255 边界；IRQ 禁用时计数器照走；**标准配置每帧恰好 241 次时钟**。
  - 3：只在 0→1 跳变计数（$2006 写、$2007 读/写）。
  - 4：扫描线 0/1/239 的 IRQ 时机不早不晚。
  - 5/6：修订版判定，**真机上互斥**（一张卡只能过其一）；readme 明言测试 3-5 依赖"未完全成文的硬件时序"，只要 1、2 通过，3-5 失败也算合理。
- `mmc3_test/`（旧版 1-6）与 `mmc3_test_2/`（含 §2.3 硬件结论、$C000=$00 与 $2000=$00 时的手动时钟等）。
- `MMC1_A12/`（Bregalad）：MMC1 寄存器 2 WRAM 禁用的 A12 相关行为。
- 结果约定：$6000=状态（$80 运行中，$00-$7F 结果），$6004 起文本（签名 $DE $B0 $61），画面文字或音频音调。
- 无 VRC 专用 IRQ 测试套件。

### 6.1 源码级校准事实（2026-09-13 补充，v2.1.3 批次 1 实现依据）

以下事实直接从 blargg 套件的**汇编源码**（christopherpow/nes-test-roms 的 `mmc3_irq_tests/source/*.asm`、`mmc3_test/source/*.s`、`mmc3_test_2/source/*.s` + `common/`）推导，比 readme 更精确，是滤波取值与计数点校准的一手依据：

- **滤波窗口 [5, 68] PPU dot 的推导**：
  - 下界：精灵取指窗口（dots 257-320）每 8-dot 组有 2 次 garbage NT 取指（`$2xxx`，A12=0），组间低电平恰 4 dot。blargg 2.Details 子测试 7 断言标准配置每帧**恰好 241 次时钟**（= 预渲染行 1 + 可见行 240，即每取指行 1 次）——若 4-dot 低电平可通过滤波，每行会计数 8 次。故 filter > 4 dot。
  - 上界：旧套件 4-scanline_timing 的常数给出 `scanline_0_10 = scanline_0_08 - 256`：$10 模式（BG $1000）下启用渲染后的首个边沿落在预渲染行 **dot 5**（首个 BG pattern 取指，启用前总线 = v 呈低电平、低电平时长不计入限制）；而稳态反配置每行恰 1 次计数、落在上一行预载组 pattern 取指（dot ~325，低电平 68 dot）——行内 BG 边沿的低电平只有 4 dot 必须被拒，预载组边沿的 68-dot 低电平必须被接受。故 filter ≤ 68 dot。
  - 本仓取 **9 dot（= 3 个 M2 下降沿）**，落在窗口内且与 Furrtek 硅片逆向一致。
- **$2006 手动时钟的低电平实测**：`clock_counter` 例程（$0000↔$1000 翻转）在两次写之间的 A12 低电平约 6-26 CPU 周期（18-78 dot），任何 filter ≤ 78 dot 都能通过测试 1/3。
- **$2006 首写不上总线**：测试 3 子测试 3 用"高位写 $10 + 低位写 $00"的配对证明 v 只在第二次写时变化——渲染关闭期只在 `$2006` 第二写、`$2007` 读/写递增后上报 v。
- **8x16 精灵"每行最多 4 次边沿"（Wiki 说法）与 garbage-NT 结构 + >4-dot 滤波矛盾**：在本仓 9-dot 滤波下 8x16 混表每行仍 ~1 次计数。blargg 套件不覆盖此场景；标为**待校准项**（需真机逻辑分析仪数据）。
- **Mesen2 滤波值矛盾（待解，勿照抄）**：Mesen2 MMC3 内联滤波 = 3 master clocks（≈0.75 dot）且精灵窗口逐组上报 garbage NT/AT + pattern 地址（`NesPpu.cpp` ProcessScanlineImpl case 0/2/4）——按此模型标准配置每行应计数 8 次，与 2.Details 子测试 7 的 241 矛盾。要么 Mesen2 未通过该 ROM，要么存在本文未覆盖的机制。本仓以 blargg 源码推导为准。
- **旧套件 4-scanline_timing / mmc3_test_2 的 4 号（±1 PPU dot）测的是 IRQ→中断处理竞态**：判定位 `$21/$22` 取决于处理程序 asl 与主线的 `inc irq_flag` 的相对位置，与 CPU 中断进入时序（逐指令 vs 逐周期）耦合；逐指令 CPU 的可观察量子是 1 CPU 周期（3 dot），无法稳定命中 1-dot 窗口。该两项属于逐周期调度（批次 3）的验收项，不属于 A12 watcher 本身。

## 7. 来源与许可

- NESdev Wiki（MMC3/MMC6/MMC5/VRC4/VRC6/VRC7/Sunsoft FME-7/JY Company/GTROM/PPU_rendering/PPU_OAM/PPU_registers/PPU_scrolling/Sprite-0_hit/Talk:MMC3/Tricky_to_emulate_games）：社区文档，许可为 **CC BY-SA**（见 Wiki 版权页）；本页全部为带出处的释义转述，未整段复制。
- nesdev 论坛实测帖（lidnariq 69ns 测量 p=116018；MMC3 计数器实现 t=23164；IRQ 后滚动 t=20013；Battletoads t607 / SMB3 四次 $2006 写 t401 存档镜像）。
- Furrtek/VGChips MMC3C 硅片逆向（A12 滤波一句引用）。
- SourMesen/Mesen2（GPL）：仅提取行为事实（寄存器语义、阈值、侦听点），未复制代码。
- christopherpow/nes-test-roms 各套件 readme（测试预期；仓库自带汇编源码，readme 未声明再分发许可，随该仓库条款分发）。
