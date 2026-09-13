<!--
source: https://www.nesdev.org/wiki/PPU_sprite_evaluation (+ OAMADDR 页，经 web.archive.org 2024 快照核验；本机直抓被 Cloudflare 403 拦截)
retrieved: 2026-09-13
conversion: compiled research note（事实转述并逐条标注来源页；许可见 ../../README.md §1 与文末）
-->

# PPU 精灵评估（sprite evaluation）与 OAM 渲染期行为

> 来源页：NESdev Wiki《PPU sprite evaluation》（该时序模型来自精确定时的 $2004 读测量，由 Visual 2C02 晶体管级仿真确认）与《OAMADDR》。
> 适用范围：NTSC 2C02。精灵 0 hit 的判定规则与 blargg 套件预期另见 `ppu_sprite_hit.md`；MMC3 计数器对精灵取指窗口的依赖见 `../cart/mapper_irq_mechanisms.md`。

## 1. 逐 dot 时序（可见扫描线）

### dots 1-64：清 secondary OAM

- Secondary OAM（当前扫描线的 32 字节缓冲）被初始化为 $FF——此窗口读 $2004 返回 $FF。
- 内部实现仍是"从 primary OAM 读、写入 secondary OAM"的常规周期，只是有一个信号强制读值恒为 $FF。逐 dot 精确的模拟器应保留读/写周期。

### dots 65-256：评估循环

- **奇数周期从 primary OAM 读，偶数周期写入 secondary OAM**（secondary OAM 已满时改为读 secondary OAM）。
- 计数器 n（精灵 0-63）、m（字节 0-3），OAM[n][m] = $2004 地址 4n+m：
  1. 从 n=0 开始：读 Y（OAM[n][0]），复制到下一个空 secondary OAM 槽（找到 8 个后写被忽略）。若 Y 在范围内，把该精灵剩余字节 OAM[n][1..3] 一并复制。
  2. n 递增。n 回卷到 0（64 个全部评估完）→ 跳 4。**找到的精灵不足 8 个 → 回 1**。**恰好 8 个 → 关闭 secondary OAM 写**（后续合格精灵"落选"）。
  3. overflow 检查：从 m=0 开始，把 OAM[n][m] 当作 Y 判范围。在范围内 → 置 $2002 的 overflow 标志，并继续读后续 3 个 OAM 条目（m 每字节递增、m 溢出时 n 递增；m=3 时 n 递增）。不在范围内 → **n 与 m 同时递增（无进位）**；n 回卷到 0 → 跳 4，否则回 3。
  4. （已满/扫完）尝试（并失败）把 OAM[n][0] 复制进下一个空槽，n 递增，重复直到 hblank。
- 评估从 **OAMADDR 决定的地址**开始（见 §2），不是 0。
- 步骤 3 的"n 与 m 同时递增"是硬件 bug，见 §3。

### dots 257-320：精灵 pattern 取指

- 每 8 个 dot 一组处理一个已选精灵：前 4 dot 从 secondary OAM 读 Y、tile 号、属性、X；随后**读 4 次 X**（同时 PPU 取精灵 tile 数据：garbage NT ×2、pattern 低、pattern 高）。
- garbage 取指与真实 pattern 取指都会驱动 PPU 地址总线（mapper 侦听相关，含 MMC3）。
- **不足 8 个精灵时空槽对 tile $FF 做 dummy 取指**（secondary OAM 的 $FF 假数据所致）：第一个空槽读出精灵 #63 的 Y + 3 个 $FF，之后的空槽读出 4 个 $FF。MMC3 对这些 $FF 取指照样计数。
- 精灵属性与 X 的装载发生在第二个 garbage NT 取指期间：属性在第一个 tick、X 在第二个 tick。

### dots 321-340（+0）

- 读 secondary OAM 第一个字节（同时 PPU 为下一条扫描线取前两个背景 tile）。

## 2. OAMADDR（$2003）quirks / "OAM rot"

来源页：《OAMADDR》。

- **评估起点 = dot 65 时的 OAMADDR 值**。若 OAMADDR 没指向 Y 字节，则它指向的内容（tile 号/属性/X）会被**重新解释为 Y 坐标**，其后字节依此错位。
- **OAM rot / 精灵消失**：评估到 OAM 末尾（精灵 63）即停止且不回卷——起点之前的精灵在本行不可见。这就是游戏的 OAM 轮转手法。
- **精灵 0 别名**：非 0 OAMADDR 可让 OAMADDR 处的精灵**被当作精灵 0**（sprite-0 hit 与优先级皆受影响）。
- 任天堂建议：发起 OAMDMA 前先向 OAMADDR 写 0，保证对齐并避免损坏。
- **取指窗口自清**：pre-render 与可见扫描线的 dots 257-320（精灵 tile 装载区间）内 OAMADDR 被置 0——一帧正常渲染结束时 OAMADDR 总会回到 0。
- **渲染开始时 OAMADDR ≥ 8 的损坏 bug（2C02G/2C02H）**：渲染开始若 OAMADDR ≥ 8，从 OAMADDR & $F8 开始的 8 字节被拷贝到 OAM 的前 8 字节（Dendy 上该 bug 是 2C02 兼容所必需）。
- **渲染期写 $2004（OAMDATA）**：不修改 OAM 值，只做毛刺递增——**仅递增高 6 位**（低 2 位视评估状态可能也被碰）。OAMDMA 走 $2004 写，同样受影响。Wiki 建议模拟器对渲染期的写一律忽略。
- **渲染期写 $2003 的 OAM 损坏（2C02G）**：通常表现为把精灵 8、9（地址 $20）拷贝到目标地址所在 8 字节行，源内容疑似来自之前的 CPU 总线值；2C03/2C04/2C05/2C07 无此行为。早年"$2004 读不可靠"的结论现归因于此损坏。
- **OAM 衰减（DRAM）**：渲染开启时每扫描线刷新一次（$2001 任一渲染位开启即可）；NTSC 上渲染关闭期间**不刷新**。数值可保持约一个 vblank 略多。PAL 2C07 从扫描线 265 起刷新且渲染关闭时也刷——**PAL 上不存在 OAM 衰减**。多数模拟器不仿真衰减且无兼容性后果。

## 3. Sprite overflow 硬件 bug（步骤 3 的实现要求）

来源页：《PPU sprite evaluation》§ Sprite overflow bug / § Cause of the sprite overflow bug。

- **bug 本体**：第 3 步"n 与 m 同时递增（无进位）"——若只有 n 递增，overflow 标志就会在"同扫描线 >8 精灵"时如预期置位。
- **对角线扫描**：第一次（正确的）超员检查后，逻辑开始**对角线**扫 OAM——后续条目的 tile 号/属性/X 被当作 Y 判范围，产生假阳性与假阴性混杂的不一致行为。blargg 4.Obscure 钉死了精确映射：精灵 1-8 在线上而 9 不在时，**精灵 10 的第 2 字节、11 的第 3 字节、12 的第 4 字节、13 的第 1 字节……被当作各自的 Y**；扫描在某个误读 Y 落入范围或 64 个全扫完时停止（全不中则不置位）。
- **根因（Visual 2C02 分析）**：找到 8 个精灵后置位的"写禁止"信号把 secondary OAM 写变成读，读到的恰是第一个被复制精灵的 Y（恒"in range"），逻辑错误让它影响了精灵地址递增；再多找到一个精灵后另一信号封锁比较，bug 停止。
- **OAMADDR 依赖**：2C02G/H 上评估起点 OAMADDR ≠ 0 会先触发 §2 的 8 字节拷贝 bug，因此 overflow 结果还依赖 OAMADDR。
- 依赖此 bug 的游戏清单见 Wiki《Sprite overflow games》专页。

## 4. 与 $2001 / 帧结构的关系

- **评估条件**：$2001 的精灵层或背景层**任一开启**即运行精灵评估。BG 关闭时背景像素全透明，精灵 0 hit 不可能发生。
- 扫描线中途写 $2001 两渲染位全关：渲染立即停止；只关精灵位则"仅隐藏精灵渲染"，评估继续（这也正是 OAM 刷新的来源）。
- **评估中途关闭渲染**：OAMADDR 损坏逻辑可能导致下一帧 OAMDMA 丢精灵。安全关闭点：无精灵的行 x=192、有精灵的行 x=240。
- **同帧关再开渲染**：至少重开后的第一条扫描线有额外损坏。
- **pre-render 行不进行精灵评估**；因第 N 行的精灵在 N-1 行评估，**扫描线 0 永远没有精灵**——这也是精灵 Y 坐标"整体偏一行"显示的根源。

## 5. blargg 测试套件对应

- `oam_read`：验证 $2004 按 $2003 地址读 OAM；实测真机上电有四种结果（全过或三种不同损坏签名）——$2003 写损坏所致。
- `oam_stress`：$2003/$2004 随机序列压力测试；**NTSC 上只在四种 CPU-PPU 上电对齐之一通过**——解读本仓门禁结果时必须带此前提。
- sprite_hit/sprite_overflow 套件的逐项预期见 `ppu_sprite_hit.md` §5。

## 6. 来源与许可

- 《PPU sprite evaluation》《OAMADDR》《PPU OAM》《PPU rendering》《PPU frame timing》：NESdev Wiki，公有领域（wiki General disclaimer 原文："Any information posted on this wiki is considered public domain"；核实记录见 ../../README.md §1）。
- 论坛引用（t=626 精灵 hit、t=1308 overflow、t=6424/t=10189/p=179676/t=11041 为 OAMADDR 页引用的实测帖）：作者版权保留，仅短引。
- blargg 套件 readme（oam_read/oam_stress/sprite hit/sprite overflow）：Shay Green (blargg)，套件免费分发。
