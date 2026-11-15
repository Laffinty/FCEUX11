# FCEUX11 v2.0.5 — 上游 mapper 能力对齐 + 长尾纳入 · 构建计划

> **STATUS: 在途**（2026-11-15 立项）。
> 本文件是**构建计划**，不是盘点报告：它回答「v2.0.5 做什么、怎么做、怎么验、什么不做」。
> 盘点与逐条评估的事实源是 `docs/plans/FCEUX11-长尾项盘点与性价比评定-2026-11-15.md`（**在途**）；
> 上游 mapper 的比对取证在本文件 §一 与 `Research_only/w205/`（gitignored）。
> 收口后移入 `docs/history/plans/`，**内容冻结不再续写**。
>
> **坐标口径**：本文件所引行号以建档时的 `main`（`eaa937a`）为准；**过期以源码为准**
> （`AGENTS.md` 坑 11 的行尾/坐标纪律）。上游坐标以快照
> `C:\Users\ikrx2\Desktop\新建文件夹 (2)\fceux-master` 为准。

---

## 〇、判定口径与前提

### 0.1 ⚠️ 上游快照的准确身份（**先更正，再往下读**）

业主指示里说的是「上游 fceux **2.7** 的最新源代码」，但**该快照实际是 FCEUX master = 2.8.0**：

```
C:\Users\ikrx2\Desktop\新建文件夹 (2)\fceux-master\src\version.h:63-72
  FCEU_VERSION_MAJOR  2
  FCEU_VERSION_MINOR  8
  FCEU_VERSION_PATCH  0
  FCEU_VERSION_STRING "2.8.0"
```

⇒ 本计划的一切"上游优势"结论都是相对 **2.8.0 master** 而言的，**不是**相对 2.7.0。
这不改变结论方向（2.8.0 相对 2.6.6 的 mapper 增量极小，见 §1.3），但**引用时不能写成"2.7.0"**。

⚠️ **快照无 `.git`** ⇒ 上游每处改动的引入版本、作者意图、配套测试**均不可考**；
所有"上游新增"判定只能靠静态比对 + 本仓 2.6.6 基线反查（`git show 1c35642`）。

### 0.2 判断「某个 mapper 能不能用」的唯一口径

本仓有**两条并存**的加载路径，缺一不可：

| 路径 | 位置 | 作用 |
|---|---|---|
| **旧静态表** | `src/ines_bmap.h` 的 `BMAPPINGLocal bmap[]` | 装载时调 `tmp->init(&iNESCart)`（`src/ines_init.cpp:78`），装 PRG/CHR 映射、分配 RAM、设 `info->Power` |
| **新注册表** | `src/boards/registry.cpp` 的 `MapperEntry`（`find_mapper`） | 决定装载后 `g_cart` 是不是一个**真的 Cart 对象**，以及 `GI_POWER` 走谁 |

⇒ **「bmap 里有」不等于「能用」**：只有新注册表命中，`g_cart` 才会是具体 Cart 子类；
否则 `g_cart` 停在 `CartPlaceholder` 上，`GI_POWER` 落进**空实现**。
本仓库自己的测试文件已写明这一机制（`tests/f11qa/boards/mapper_load_test.cpp:232-249`），
并为 191 / 199 补了注册（GitHub issue #12）。

### 0.3 本仓的架构差异清单（**先把它们划掉，否则会把改造误判成上游优势**）

| # | 本仓改造 | 上游对应写法 | 是否算「上游优势」 |
|---|---|---|---|
| 1 | `src/boards/mapinc.h` 拆成 `mapinc_base.h` / `mapinc_bus.h` / `mapinc_state.h` / `mapinc_audio.h` / `mapinc_mmc3.h` | `#include "mapinc.h"` | ❌ **否**（占全部"上游独有 1 行"差异中的 **99 条**） |
| 2 | `g_cpu.set_map_irq_hook(X)` | `MapIRQHook = X;` | ❌ 否 |
| 3 | RAII：`P_owner.reset()` | `if (P) FCEU_gfree(P); P = NULL;` | ❌ 否 |
| 4 | `g_cpu.native_layout().DB` | `X.DB` | ❌ 否 |
| 5 | `fceu11::g_bus.chr_ptr()` | `CHRptr[0]` | ❌ 否 |
| 6 | 多态 `fceu11::ExpansionAudio` 类 | `GameExpSound.RChange/Fill/HiFill/HiSync` 函数指针 | ❌ 否（**本仓更现代**） |
| 7 | `MapperEntryRegister` 注册表 + `MapperStrategyA` 捕获 legacy `info->Power` | 直接用 `info->Power` | ❌ 否（**本仓架构**） |
| 8 | `src/sound.cpp` 的**帧计数器周期精确模型**（`FrameCounterTick` / `FcNextEventPos`）+ **APU 对齐抖动**（写落在偶/奇周期 ⇒ 3 或 4 周期后成熟） | 经典 `fhcnt -= cycles*48; if (fhcnt<=0) {...}` 循环 | ❌ 否（**本仓领先**，见 §1.5） |

### 0.4 证据纪律（沿用本仓纪律 3 / 4 / 10）

1. **每条"上游优势"必须给出上游文件:行 + 本仓文件:行 + 本仓是否有等价实现**；
   只说"上游有"不说"本仓有没有"的条目，一律不进本计划。
2. **不凭记忆写位级格式**（纪律 4）：上游的 N340 镜像表、上游 134 的 CNROM 分支、
   上游 210 的 CRC32 判定表**都没有权威出处**，只能作为"上游这么写"的证据，
   不能作为"真机这么工作"的证据 ⇒ 凡依赖它们的移植项都标注证据档位。
3. **门禁里没有的＝不存在**（纪律 10）：本计划每条新锁测试都必须落在
   §六 的某条**已存在**的门禁命令上，并在实施时**当场看到它的测试计数**。
4. **变异验证双向**（纪律 3）：每条新锁都要能"改回修复前的值 ⇒ 转红"。

### 0.5 长尾纳入的筛选口径（业主 2026-11-15 指示）

> 「将 `docs/plans/FCEUX11-长尾项盘点与性价比评定-2026-11-15.md` 里面
> **性价比为中和高**，且**难度为中和低**的长尾任务纳入 v2.0.5 的任务」

即：**性价比 ∈ {中, 高}** ∧ **难度 ∈ {中, 低}**。逐条筛结果见 §2.2。

---

## 一、结论摘要：上游 mapper 到底比不比我们强

### 1.1 三句话

1. **在"支持哪些 mapper 号"上，上游不比我们强**——本仓覆盖 **270** 个号，
   上游覆盖 **273** 个，差集只有上游新增的 **413 / 451 / 471** 三个（§1.2）。
2. **在"单个 mapper 的实现新度"上，上游只在 6 处更强**，且每一处都有前提
   （§1.3）；**172 个共有板子文件里 99 个的差异只是 `#include "mapinc.h"` 样板**。
3. ⚠️ **但本仓自己有一批 mapper 是"静默失效"的——这比"上游更强"严重得多**：
   **75 个 mapper 号的板级 Power 从不执行**（缺注册；r2 更正：r1 的 42 号名单
   漏了 `Latch_Init` 家族与委托 Init），**17 个 MMC3 家族变体的
   板级 Power 被基类顶掉**（§1.4）。**上游 master 在这些 mapper 上是正常工作的。**
   ⇒ 对这些 mapper 而言，**上游确实比我们强，而且是数量级上的强**。

### 1.2 覆盖面：号多 ≠ 兼容强

> ⚠️ **r2 更正**：下表数字为**剥掉注释条目后**的实测（coverage.py 的旧读数把
> 36 个被注释掉的条目也算进去了）。方向性结论不变。

| | 条目数 | 覆盖的 mapper 号 | 独有号 |
|---|---|---|---|
| 上游 2.8.0 `src/ines.cpp` 的 `bmap[]`（剥注释） | 235 | **235** | 413 / 451 / 471 |
| 本仓 `src/ines_bmap.h` 的 `bmap[]`（剥注释，不含终止符） | 234 | **234** | 101 / 151 |
| 交集 | — | **233** | — |

- **上游独有的 3 个号全部是 2024 年新增的板子**（`src/boards/{413,451,471}.cpp`），
  本仓**编译单元、bmap、注册表、CMake 源文件列表四处皆无**。
- 本仓独有的 2 个号（101「Deprecated, dupe」上游注释掉、151）与本仓**更完整**的
  结论（§1.5）一致——**本仓在覆盖面上不落后上游，反而多 2 个号**。
- ⇒ **「号」的层面本仓不落后上游。** 上游领先的是那 3 个新板子的**实现**。

### 1.3 实现新度：上游只在 6 处更强

取证方法：172 个共有板子文件逐个做**归一化语义 diff**（去注释、折叠空白、
`difflib` 行级 + token 级双比），脚本 `Research_only/w205/diff_boards.py`、
`show_diff.py`。

**归一化后 token 相似度分布**：99 个文件 ≥ 0.99；其余绝大多数落在 0.90–0.98，
且差异 100% 落在 §0.3 的八类本仓改造里。**逐条顶回后，真正成立的上游优势只有 6 类**：

| 编号 | 上游优势（一句话） | 上游坐标 | 本仓现状（已实测复核） | 性质 | 代价 |
|---|---|---|---|---|---|
| **U1** | **ROM 矫正活表整行缺 22 条 + 2 条值过时** | `src/ines-correct.h` 23 行带 `\|` 掩码表达式（`256\|0x1000` / `51\|0x800`） | 本仓**活表**（Rust `ines_data.rs::ROM_CORRECTIONS`）**一条都没有**；另有 `243a8735` mirror 被压成 `0`、`0da5e32e` 仍是旧的 mapper 101 | **本仓自己的回归**（v0.2.19 抽取器解析不了 C 表达式） | **低**（纯数据） |
| **U2** | mapper 210 按 NES 2.0 submapper 1/2 + CRC32 判定 **Namco 175 / 340**，N340 有 4 种镜像，读档恢复镜像 | `src/boards/n106.cpp:441-460`、`:66-73` | 本仓只有二值 `is210`；镜像表 `MI_0/MI_V/MI_H/MI_0`（**case3 与 case0 重复**）；读档不恢复镜像 | 子 mapper 支持 + bug 修复 | 低 |
| **U3** | mapper 4 支持 **NES 2.0 头的 PRG-RAM 尺寸** | `src/boards/mmc3.cpp:366-367` | 本仓 `src/boards/mmc3.cpp:419-429` 无 `ines2` 分支，恒用固定 8KB WRAM | NES 2.0 头适配 | **低**（一处） |
| **U4** | mapper 45 重写：CHR bank mask 重定义 + **写窗口收窄到 `$6000-$7FFF`** + 锁位复位分支 | `src/boards/mmc3.cpp:500-566` | 本仓 `src/boards/mmc3.cpp:554-620` 是 2.6.6 旧版，写窗口是 `$5000-$7FFF` | bug 修复 + **顺带消除本仓潜在越界写**（见 §1.4 注） | 中 |
| **U5** | mapper 134 重写：4 种 PRG 模式 + CNROM + 4 个 `EXPREGS` + 写锁定 | `src/boards/mmc3.cpp:862-916` | 本仓 `src/boards/mmc3.cpp:918-952` 是 2.6.6 旧版（单一 PRG 模式、1 个寄存器、写窗口仅 `$6001`） | 新增寄存器 + 子 mapper 支持 | **高**（上游自己标 `// CNROM mode. Unclear. Untested.`） |
| **U6** | 四项小修：MMC5 上电寄存器默认值（`0xFF`→`0`）/ bandai x24c02 设备地址校验 / mapper 72 写窗口收窄到 `$8000-$FFFF` / mapper 19 的 N163 CHR-RAM 窗口真正映射 | `mmc5.cpp:906-913`、`bandai.cpp:181-186`、`72.cpp:59`、`n106.cpp:131` | 本仓四项分别缺 / 无 / 用 `$6000-$FFFF` / 该调用被注释掉 | bug 修复 + 行为修正 | 低～中 |

**被否证的"上游优势"（不要照做）**：

| 线索 | 核实结论 |
|---|---|
| MMC5 的 `MMC5SPRVPage`/`MMC5BGVPage`、`if (CHRptr[0])` 保护、`PALRAM/UPALRAM`、`MMC5HackVROMPTR` | **本仓全有**（访问路径改为 `fceu11::g_bus.*`），不是上游新增 |
| coolgirl 的 `cfi_data[]`、`get_bits/set_bits`、CFI flash、IRQ 时序（`scanline+1`/`239`/`127`）、`lreset+2` | **本仓逐字已有** |
| `sb-2000.cpp` / `vrc5.cpp` 的"上游独有"内容 | **是上游自己注释掉的死代码**，本仓只是删了注释 |
| `mmc1.cpp` / `datalatch.cpp` / `addrlatch.cpp` / `vrc2and4.cpp` 行数远大于上游 | **本仓架构改造**（Cart 类化），不是上游优势 |
| mapper 7 ANROM 的锁存窗 | **本仓已修**（上游 2.8.0 仍有 `$4020-$FFFF` 吞写洞） |
| mapper 97 / mapper 101 / mmc1 的 NES 2.0 submapper 5 | **本仓更完整**（上游已把 101 注释为 "Deprecated, dupe"） |
| 上游 mapper 218 的 NTARAM 越界修复 | **本仓已逐字包含**（v2.0.4 V9） |
| 上游的 CHR-RAM < 8K 下限保护 / mapper 修正写回 | **本仓已对齐**（V10 / S5-A2） |

### 1.4 ⚠️ 本仓自身的静默失效缺陷（本计划的头号内容）

> 这一节回答的不是"上游比我们好什么"，而是**"我们现在有多少 mapper 其实是坏的"**。
> 结论：**上游 master 在这些 mapper 上是正常工作的** ⇒ 这是我们落后于上游的地方。

#### (A) 75 个 mapper 号：板级 Power **一次都不执行**（缺 `MapperEntry` 注册；r2 从 42 更正）

**完整调用链（已逐行核实）**：

```
src/ines_load.cpp:305        iNES_Init(MapperNo)        → bmap 命中 → MapperNN_Init(&iNESCart)
                                                            → iNESCart.Power = MxxPower      ✅ 板级函数被装上
src/ines.cpp:63              currCartInfo = &iNESCart
src/ines.cpp:65              create_cart_for_mapper(MapperNo, g_bus)
src/cart_class.cpp:53-58     find_mapper(MapperNo) → nullptr（该号没注册）→ 返回 nullptr
src/cart_class.cpp:61-64     assign_cart(nullptr) → g_cart = &g_cart_placeholder
src/ines.cpp:68              iNESCart.Power = CartInfo_PowerForward;   ← 无条件覆盖掉板级 Power ❗
--- 上电 ---
src/ines_gi.cpp:27-28        GI_POWER → iNES_ExecPower() → iNESCart.Power()
src/cart.cpp:69-72           CartInfo_PowerForward → currCartInfo->cart_obj->on_power()
src/cart_class.cpp:30        CartPlaceholder::on_power() {}            ← 空实现 ❗
⇒ 板级 Power 从不执行：它的 SetWriteHandler / SetReadHandler / EXPREGS 初值
  / SRAM 映射 / IRQ hook 全部装不上。
```

**运行时实证**（不是只读代码）：用与 `tests/fceux11_mapper_load_test.exe` **完全相同的
库**链接了一个临时探针 `Research_only/w205/probe/registry_probe.cpp`
（不改动仓库任何文件），实测：

```
total registered in 0..600 = 159
spot checks:
  mapper 191 -> TW MMC3+VRAM Rev. A
  mapper 199 -> Waixing DBZ variant
  mapper 218 -> (NO REGISTRY ENTRY)
  mapper 244 -> (NO REGISTRY ENTRY)
  mapper 252 -> (NO REGISTRY ENTRY)
  mapper 342 -> (NO REGISTRY ENTRY)
  mapper 471 -> (NO REGISTRY ENTRY)
```

> ⚠️ **r2 更正（名单 42 → 75）**：下表为**剥注释后自洽的 derive**（方法：运行时
> 探针取未注册全表 × 逐板 Init 链静态分析，`Research_only/w205/derive_unregistered.py`）。
> r1 的 42 号名单漏了 **`Latch_Init` 家族**（`Latch_Init` 给每个调用者装 `LatchPower`，
> 覆盖 8/29/101/107/113/140/144/152/155/171/174/180/184/200/201/202/203/204/212/
> 213/214/217/227/229/231/240/241/242 等）与**委托 Init**（`Mapper255_Init`→
> `Mapper225_Init`、`AA6023_Init`→`CommonInit`、`Mapper197_Init`/`Mapper171_Init`/
> `Mapper257_Init` 等）。自洽校验：bmap 234 号 = 注册 159 + 未注册 75。

**75 个号**（本仓 `bmap` 里有实现、Init 链装板级 Power、但注册表里没有）：

```
8 29 101 107 113 134 140 144 152 153 154 155 157 159 171 174 178 180 184
196 197 200 201 202 203 204 207 208 209 211 212 213 214 215 216 217 218
219 220 221 222 225 226 227 228 229 230 231 232 233 234 235 238 240 241
242 243 244 246 252 253 255 256 257 258 259 260 261 268 342 354 361 366
470 547
```

其中用户可感的至少有 **COOLGIRL（342）**、**252 SAN GUO ZHI PIRATE**、
**253 DRAGON BALL PIRATE**、**244 DECATHLON**、**246 FONG SHEN BANG**、
**218**、**215 / 216 / 219 / 221 / 222**、**BANDAI 家族 153 / 157 / 159**、
**257 PEC-586**、**256 ONE-BUS**、**197 / 255 / 261 / 268（BMC 家族）**。

> **交叉印证**：长尾盘点文档 LT-03 记录「dump B（mapper 252）**145 秒纯蓝屏**」，
> 并归因于「SMYNES 专用版，任何模拟器都跑不出画面」。
> **本条给出一个更早的解释**：本仓的 mapper 252 **板级 Power 从不执行**，
> 写 `$8000-$FFFF` 的寄存器写根本没被接住 ⇒ 蓝屏是**本仓缺陷**的症状之一，
> 与 dump 本身是不是加密版**无关**。
> ⇒ LT-03 的结论「不需要矫正条目」**仍然成立**（矫正表那条独立），
> 但「蓝屏 = ROM 加密」这个归因**必须更正**。**更正登记见 §七。**

**仓库自己知道这个缺陷、但只修了 2 个号**：
`tests/f11qa/boards/mapper_load_test.cpp:232-249` 明写
「An unregistered mapper keeps g_cart on the no-op placeholder, so the power cycle
never runs and the ROM comes up grey」，并为 191 / 199 补了注册（issue #12）。
**其余 42 个号从未被登记进任何门禁** ⇒ 门禁盲区成立。

#### (B) 17 个 MMC3 家族 mapper：已注册，但板级 Power 被基类顶掉

`src/boards/mmc3_base_cart.cpp:20-26` 的 `Mmc3BaseCart::on_power()` **只调
`GenMMC3Power()`**；其注释声称「Each MMC3 variant subclass overrides on_power()」，
但 `src/boards/mmc3_variants_carts.h` 里 **26 个 MMC3 家族 Cart 类只有
`Mapper198Cart` 与 `Mapper199Cart` 真的 override 了**（外加 mapper 4 的 `Mmc3Cart`）。

⇒ **17 个已注册、但板级 Power 不执行的 MMC3 家族 mapper**：

```
12 37 44 45 47 49 52 114 115 116 195 205 245 249 250 254 406
```

各被顶掉的实质内容（`src/boards/mmc3.cpp`）：

| mapper | 被顶掉的板级 Power 内容 | 用户可感后果 |
|---|---|---|
| 12 | `EXPREGS[2] = 1`（**中文默认**）+ `$4100-$5FFF` 读写 handler | 语言/功能都不对 |
| 37 | `$6000-$7FFF` 写 handler | 寄存器写丢失 |
| 44 | `$A000-$BFFF` 写 handler | 同上 |
| 45 | `EXPREGS[0..5] = 0` + `$5000-$7FFF` 写 + `$5000-$5FFF` 读 | 同上 |
| 47 | `$6000-$7FFF` 写 handler | 同上 |
| 49 | `M49Reset()` + `$6000-$7FFF` 写 + 读 handler | 同上 |
| 52 | `M52Reset()` + `$6000-$7FFF` 写 handler | 同上 |
| 114 | `$8000-$FFFF` 写 + `$5000-$7FFF` 写 handler | 同上 |
| 115 | `$4100-$7FFF` 写 + `$5000-$5FFF` 读 handler | 同上 |
| 116 | `UNLSL12Power`：`mode = 0` + `vrc2_chr[0..3]` 初值 | VRC2 变体行为不对 |
| 195 | `M195Power` | WRAM 窗口不对 |
| 205 | `$6000-$7FFF` 写 handler | 同上 |
| 245 | `EXPREGS[0] = 0` | 寄存器初值不对 |
| 249 | `$5000` 写 handler | 同上 |
| 250 | `$8000-$BFFF` + `$C000-$FFFF` 写 handler | 同上 |
| 254 | `$8000-$BFFF` 写 + `$6000-$7FFF` 读 handler | 同上 |
| 406 | `$8000-$BFFF` + `$C000-$FFFF` 写 handler | 同上 |

**⚠️ 越界注记（必须与 A4 同批处理）**：本仓 `M45Power` 装的是
`SetWriteHandler(0x5000, 0x7FFF, M45Write)`，而 `M45Write` 执行
`WRAM[A - 0x6000]` ⇒ `$5000-$5FFF` 段是**负下标越界写**。
**当前不可达**（因为 Power 根本不跑）；**一旦修好接线就会变成活的越界写**
⇒ 必须**同时**取上游收窄后的窗口（U4）。

**正常者**（有 override，勿动）：`0 NromCart` / `1 Mmc1Cart` / `4 Mmc3Cart` /
`24,26 Vrc6Cart` / `198 Mapper198Cart` / `199 Mapper199Cart`。
其余 126 个 mapper 走 `MapperStrategyA`（`src/boards/mapper_strategy_a.h:38-56`），
它在**构造时捕获** legacy `info->Power` 并在 `on_power()` 里调用 ⇒ **正确**。

### 1.5 本仓领先上游的部分（**明确登记为"不动"，防止下轮重复统计**）

| 项 | 本仓 | 上游 2.8.0 |
|---|---|---|
| APU 帧计数器 | **周期精确模型**（`FrameCounterTick` / `FcNextEventPos`，精确落在边界） | 经典 `fhcnt -= cycles*48; if (fhcnt<=0)` 循环 |
| APU 对齐抖动 | 写 `$4017` 按 CPU 周期奇偶**延后 3 或 4 周期**成熟 | 无 |
| 扩展音频 | 多态 `ExpansionAudio`（可保存状态、可测） | `GameExpSound.*` 函数指针 |
| mapper 7 ANROM | 锁存窗 `$8000-$FFFF`（**已修** `$4020` 吞写洞） | 仍是 `$4020-$FFFF` |
| mapper 97 | 4 种镜像 + CHR 位域 | 2 种镜像 |
| mapper 101 | 仍启用并实现 | 已注释 "Deprecated, dupe" |
| mmc1 | NES 2.0 submapper 5 **Fixed PRG** | 无 |
| mapper 218 | NTARAM 越界修复（V9） | 有（同） |
| CHR-RAM < 8K | 8K 下限补齐（V10） | 有（同） |
| mapper 修正写回 | 有效（S5-A2） | 有（同） |
| 存档标签短串 | `AddExState` 按实际长度拷贝（LT-17） | **同款缺陷**（`state.cpp:907` 定长 4 字节） |

---

## 二、任务总表

### 2.1 线 A：上游 mapper 能力对齐

| # | 任务 | 类型 | 重要性 | 难度 | 性价比 | 建议动作 |
|---|---|---|---|---|---|---|
| **A1** | 补 42 个 mapper 的注册，恢复板级 Power | **本仓缺陷** | **高** | 中 | **高** | ✅ 立即做（与 A2 同批） |
| **A2** | MMC3 家族 17 个变体的 `on_power` 接线 | **本仓缺陷** | **高** | 中 | **高** | ✅ 立即做 |
| **A3** | 矫正活表补 22 条 + 修 2 条 | 上游优势 U1 | 中 | 低 | **高** | ✅ 立即做（纯数据） |
| **A4** | mapper 45 取上游窗口（与 A2 同批，防越界） | 上游优势 U4 | 中 | 低 | **高** | ✅ 与 A2 同批 |
| **A5** | mapper 4 的 NES 2.0 PRG-RAM | 上游优势 U3 | 中 | 低 | 中 | ✅ 插队做（一处改动） |
| **A6** | mapper 210 Namco 175/340 | 上游优势 U2 | 中 | 中 | 中 | ✅ 做（证据档位见 §三） |
| **A7** | MMC5 / bandai / mapper 72 / mapper 19 四项小修 | 上游优势 U6 | 中 | 低～中 | 中 | ✅ 做 |
| **A8** | 上游新增 mapper 471 / 451 / 413 | 上游优势 U7 | 低 | 低～高 | 低～中 | 471 ✅ 做；451/413 见 §三 D 级 |

### 2.2 线 B：长尾纳入项（筛选：性价比 ∈ {中,高} ∧ 难度 ∈ {中,低}）

| 长尾号 | 条目 | 线 | 重要性 | 难度 | 性价比 | 是否纳入 | v2.0.5 的动作 |
|---|---|---|---|---|---|---|---|
| **LT-03** | 3GO.nes 的 CRC 矫正条目 | NES | 中 | 低 | 高 | ✅ **纳入** | 已闭合（无代码改动）；本轮只需**更正归因**（§七） |
| **LT-04** | PGO 一组可复现读数 | NES | 低 | 中 | 中 | ✅ **纳入** | 读数已闭合（+32%~37%）；尾巴是「**PGO 是否默认 ON**」⇒ **裁决项 D2** |
| **LT-08** | DMC 计费范围界定 | NES | 中 | 中 | 中 | ✅ **纳入** | **B1**：三方交叉取证 + 判定句 + 锁测试 |
| **LT-09** | 人工验收未闭合（NES 侧 7 项） | NES | 中 | 低 | 高 | ✅ **纳入** | **B2**：本机可做的 5 项 + 需真机的 2 项 |
| **LT-10** | 人工验收未闭合（GBA 3 张卡带） | GBA | 低-中 | 低 | 高 | ✅ **纳入** | **B3**：探针读数 + 业主人眼确认 |
| **LT-16** | `start_delay = 2` 的独立来源取证 | GBA | 低-中 | 中 | 中 | ✅ **纳入** | **B4**：纯取证，不阻塞 |
| **LT-17** | SFORMAT 短标签定长 `memcpy` 越界 | NES | 中 | 低 | 中 | ✅ **纳入** | 已修（v2.0.5 前的 `7fbd31a`）；**登记为已闭合** |
| LT-01 | GUI 下 yielding Lua 脚本崩溃 | NES | **高** | **高** | 中 | ❌ **不纳入**（难度=高，落在筛选外） | **D1 建议纳入其"第一步"**（低成本可复现化） |
| LT-05 | 彻底移除 `FCEUSTATE_INDIRECT` | NES | 中 | 中高 | 中 | ❌ 不纳入 | 暂缓 |
| LT-06 | 已完成项的验证尾巴 | NES | 低 | 低 | **低** | ❌ 不纳入 | 已闭合 |
| LT-07 | 049/050/051 基线翻转 + R4 gate 口径 | NES | 中 | 中 | **低** | ❌ 不纳入 | 前置（133 处漂移）未裁决 |
| LT-02 | Step Over 的 watchpoint 长程不消费 | NES | 中低 | 中 | **低** | ❌ 不纳入 | 依赖 LT-01 |
| LT-11 | BIOS SWI 声音家族 12 号 + `0x2A` | GBA | 中 | 高 | **低** | ❌ 不纳入 | GBATEK 材料不全 |
| LT-14 | FireRed 的 init 竞态根因 | GBA | 中 | 高 | 低-中 | ❌ 不纳入 | 现方案有效，不可回退 |
| LT-15 | GBA DMA 剩余两项机制 RE | GBA | 中 | 高 | 中 | ❌ 不纳入 | 难度=高 |
| LT-13 | GBA GPIO / RTC | GBA | 中 | 中高 | 中 | ❌ 不纳入 | 难度=中高 |
| LT-12 | BIOS SWI `0x26` HardReset | GBA | 低 | 中高 | **低** | ❌ 不纳入 | 暂缓 |

### 2.3 裁决项（**需业主点头**，不自行决定）

| # | 裁决事项 | 背景 | 我的建议 |
|---|---|---|---|
| **D1** | **LT-01 的"第一步"是否纳入 v2.0.5** | LT-01 重要性是全表最高档（**高**），但整条难度=**高**，落在业主筛选口径外；而它的**第一步**（`scripts/repro/lua_gui_yield_repro.ps1` 把随机 2–68 秒变成固定触发）成本是**低**，且脚本已在树里 | ✅ **建议纳入第一步**（不含第二步的回溯/根因）—— 它是全表唯一"用户可触发的崩溃"，而第一步不依赖装调试器 |
| **D2** | **PGO 是否设为默认 ON**（LT-04 的尾巴） | 实测收益 +32%~37% 吞吐；但默认 ON 要求发布流程三步化、`.pgc` 入库或可复现、CI 两套配置（两个开关互斥） | ⏸ **建议 v2.0.5 不改默认**，把它作为**发布策略**单独立项；本轮只把读数与脚本固化 |
| **D3** | **mapper 92 的硬件模型是否换成上游版** | 上游把 92 从 latch 模型换成 JF-17 寄存器模型；两种模型**互斥**，且**改存档格式**（`LATC` → `PREG`/`CREG`），本仓版源自 2.6.6 基线 | ⏸ **建议 v2.0.5 不动**——无 JF-19 实测 ROM 时属"拿推测换现状"；**登记为长尾**，等业主提供 ROM 再立项 |

---

## 三、逐项方案

### A1 · 补 42 个 mapper 的注册（恢复板级 Power）—— ✅ r2 已按 A1-a 落地（名单更正为 75）

> **r2 落地摘要**：修复 = `src/ines.cpp` 条件化（命中 ⇒ forwarder；未命中 ⇒ 保留
> 板级 Power/Reset/Close，与 UNIF 同款）；A1-b 否决（`find_mapper` 上限 512 而
> 547 在名单里）。门禁 = `fceux11_mapper_load_test` 的 S1/S2/S3/S4（见下）。
> 实测：sweep 75/75 保留 + 252 锚点 0xFC + `ctest -LE perf` 41/41；
> 变异腿（还原无条件覆盖）sweep 75 红 + S3 红 + S4 保持绿。

**目标**：让这 75 个号的 `GI_POWER` 走到板级自己的 Power 函数。

**做法（二选一，择一执行；r2 已选 A1-a）**：

- **A1-a（推荐，改动最小、风险最低）**：把 `src/ines.cpp:68-70` 的**无条件覆盖**改成
  「注册表命中 ⇒ 走 forwarder；未命中 ⇒ **保留板级 `info->Power`**」。
  改动 1 处，**一次性修好 42 个号**，且**不新增任何 Cart 类**。
  ⚠️ 须确认未命中的号走 legacy 路径时不会与 `CartPlaceholder` 冲突
  （`cart_obj` 为 placeholder 时 `on_power()` 是空实现 ⇒ legacy `info->Power` 才是唯一有效路径）。
- **A1-b（与本仓架构一致，但工作量大）**：为 42 个号各补一个 `MapperEntry` +
  一个 `MapperStrategyA` 派生的 Cart 类（`MapperStrategyA` 已在构造时捕获 legacy
  `info->Power`，几乎零成本）。参照 issue #12 对 191 / 199 的做法。

**验收命令**（r2 实施后的实际形态；探针读数已并入正式门禁）：
```powershell
# ① S1 分类完备断言（fceux11_mapper_load_test）：bmap 里每个有 Init 的号
#    必须 ∈ 注册表 ∪ 75 名单（自维护：新板子未分类即红）
# ② S2 sweep：75 个号各造一张合成 ROM（16KB PRG + iNES1/2 头，按
#    generate_test_roms.py 字段拼装）实装载，断言未命中号保留板级 Power 指针
# ③ S3 mapper 252 执行锚点：CartBR 读 handler 身份（$6000/$8000）+ PRG 标签字节
#    ⚠️ 不得用「写-回读」当判据——开总线回声（ANullImpl 返回 DB 锁存）会让它无牙
# ④ S4 控制组：0/4/191/199 必须仍走 forwarder（防分支写反）
# 运行时读数（探针，保留作研究侧复核）
Research_only\w205\probe\registry_probe.exe   # 期望：159 注册不变；75 个号由 S2 sweep 盯
```

**反向守卫**：
- 断言必须写「**bmap 里有实现**」的号（由 `MapperNN_Init` 存在性判定），
  **不是**「0..600 全部有注册」——后者会把 300+ 个根本不存在的号也要求注册。
- **不得**为了让门禁变绿而给没有实现的号编造空 Cart。
- A1-a 与 A1-b **只能选一个**；同时做 ⇒ 板级 Power 可能被执行两次。

**非目标**：不改任何板子的寄存器语义；不碰 GBA；不新增 mapper 号。

---

### A2 · MMC3 家族 17 个变体的 `on_power` 接线

**目标**：让这 17 个号的板级 Power 真正执行。

**做法**：在 `src/boards/mmc3_variants_carts.h` 里为这 17 个类各加一个
`on_power() noexcept override`，转调对应的板级函数
（`M12Power` / `M37Power` / `M44Power` / `M45Power` / `M47Power` / `M49Power` /
`M52Power` / `M114Power` / `M115Power` / `M195Power` / `M205Power` / `M245Power` /
`M249Power` / `M250_Power` / `M254_Power` / `M406_Power`），
**与 `Mapper198Cart` / `Mapper199Cart` 的既有写法同款**（`mmc3.cpp:1610-1620`）。
这些板级函数目前是 `static`，需与 `M195Power` / `M199Power` 一样去掉 `static`
或加内部声明（`M195Power` 已有 `extern` 声明的先例，`mmc3.cpp:1545/1549`）。

**验收命令**：
```powershell
# 每条至少一个「写-回读」锁（照 check_mapper198_power 的模式）
# 例：mapper 12 —— 写 $4100，读回；mapper 45 —— 写 $6000，读回；mapper 165 —— EXPREGS[0]==0xFD
ctest --test-dir build --build-config Release --output-on-failure -LE perf
```

**反向守卫**：
- **不得**用「`GenMMC3Power()` 跑通」当验收——它对 17 个号全都"通过"，是典型无牙门禁；
- 断言必须指向**该板自己的**可观测差异（EXPREGS 初值 / 专属窗口的写回读）；
- **不得**改 `Mmc3BaseCart::on_power()` 的默认行为（74/76/118/119/192/194/197
  依赖 `GenMMC3Power` 默认值，改了会波及 7 个正常 mapper）。

**非目标**：不改 MMC3 的寄存器语义（除 A4 的 mapper 45 窗口）。

---

### A3 · 矫正活表补 22 条 + 修 2 条（U1）

**目标**：把上游 2.8.0 `src/ines-correct.h` 的 277 条数据**对齐**到本仓**活表**
`src/rust/crates/fceux11-formats/src/ines/ines_data.rs::ROM_CORRECTIONS`。

**缺失根因（已实测定位）**：上游表里 22 行的 mapper 字段写成 C 表达式
（`256|0x1000` / `51|0x800` / `0x10|4`），
v0.2.19 的 Python 抽取器解析不了表达式 ⇒ **整行被丢掉**。

| 缺失/过时 | 上游值 | 本仓活表现值 | 影响 |
|---|---|---|---|
| 21 条 `mapper|0x1000`（256/257/258/259/260/406 等） | `256|0x1000` … | **一条都没有** | 这些 ROM 拿不到 mapper 矫正 ⇒ 按头里的错 mapper 装载 |
| 1 条 `a912b064` | `51\|0x800` | **没有** | 丢掉「CHR ROM 本不该存在」的清 VROM 语义 |
| `243a8735`（Major League） | mirror `0x10\|4` = **20** | mirror **0** | 单屏镜像退化为水平 |
| `0da5e32e`（Urusei Yatsura） | mapper **87**, mirror **1** | mapper **101**, mirror -1 | 装成错的板子 |

**做法**：
1. 用**能求值 C 表达式**的抽取器，从上游 `src/ines-correct.h` 重新生成整张表
   （`256|0x1000` → `4352`、`51|0x800` → `2099`、`0x10|4` → `20`）；
2. **Rust 侧逻辑不用改**——`src/rust/crates/fceux11-formats/src/ines/ines.rs:310-320`
   已经认 `0x1000`（mask=0xFFF）与 `0x800`（clear_vrom）两个标志位，只是没有数据喂它；
3. 本仓 C++ 侧已支持 `Mirroring >= 0x10`（`src/ines_load.cpp:296-297`
   ⇒ `SetupCartMirroring(2 + (Mirroring & 1), 1, 0)`）⇒ mirror 写 20 即可生效；
4. 加一条锁测试：断言这 22 个 CRC 在活表中可命中，且 mapper/mirror 逐字等于上游。

**验收命令**：
```powershell
cargo test -p fceux11-formats --lib          # 期望 154/0（含新锁）
# 反查：上游 22 个带掩码的 CRC 在活表里必须全部命中
Select-String -Path src\rust\crates\fceux11-formats\src\ines\ines_data.rs `
  -Pattern '0x1d75fd35|0xb02fcb57|0xa912b064|0x243a8735|0x0da5e32e'   # 期望：全部命中
```

**反向守卫**：
- **矫正表只加不改**既有条目（LT-03 的原话仍然有效）；
- `0xF3C85DC3` 必须继续**查不中**（那是整文件 CRC，不是查表键）；
- **不得**改 `src/ines-correct.h`——它是**死表**（全仓无 include），改它不解决问题；
- **不得**改 Rust 的查找逻辑（数据补齐后不需要）。

**非目标**：不重算其它卡的 CRC；不重钉冻结基线。

---

### A4 · mapper 45 取上游窗口（与 A2 同批）

**目标**：在修好 A2 的接线**之前或同时**，把 `M45Power` 的写窗口从
`$5000-$7FFF` 改成上游的 `$6000-$7FFF`，并取上游的 CHR bank mask 规则。

**理由**：本仓 `M45Write` 执行 `WRAM[A - 0x6000]`；窗口含 `$5000-$5FFF` 时
是**负下标越界写**。当前因 A2 的缺陷不可达；**先修接线后不取上游窗口 = 把休眠缺陷激活**。

**验收命令**：同 A2 的写-回读锁；另加一条「写 `$5000` 不应改动 WRAM 邻域」的越界锁。

**反向守卫**：**不得**在 A2 接线落地**之后**再做本条（那是把越界写变成活的）。

---

### A5 · mapper 4 的 NES 2.0 PRG-RAM（U3）

**做法**：`src/boards/mmc3.cpp:419-429` 加 `info->ines2` 分支
（`ws = (info->wram_size + info->battery_wram_size) / 1024`）。
本仓字段已有（`src/cart.h:85,87,88`），单位与上游一致（字节）。

**反向守卫**：**不得**改非 NES 2.0 路径的 `ws = 8`；Low-G-Man 的 CRC32 分支保持原样。

---

### A6 · mapper 210 的 Namco 175 / 340（U2）

**做法**（只改 `src/boards/n106.cpp` 一个文件）：把二值 `is210` 换成
上游的三态 `NamcoType { N163, N175, N340 }` + `Mapper210_DetectType()`
（NES 2.0 submapper 1/2 优先，CRC32 兜底），并把 N340 的镜像表改成上游那 4 项
（`MI_0/MI_V/MI_1/MI_H`），读档恢复镜像态。

**⚠️ 证据档位（纪律 4）**：上游的 N340 镜像表**没有权威出处**，
上游的 CRC32 兜底表**只有 5 个游戏且 default 一律判 N340**（未收录的 N175 会被误判）。
⇒ 本条必须**先做取证**：以 NESdev wiki / 权威资料定镜像语义，
**上游只作为参照，不作为依据**。

**验收命令**：写-回读 + 镜像锁；NES 2.0 submapper 1/2 各一条。

**反向守卫**：**不得**无条件把 mapper 210 当 N340（上游的 default 就是这样，会误判）；
**不得**把上游那行 `FCEU_printf("Submapper detected by CRC32...")` 当成必需输出照抄。

---

### A7 · MMC5 / bandai / mapper 72 / mapper 19 四项小修（U6）

| 子项 | 做法 | 注意 |
|---|---|---|
| MMC5 上电默认值 | `src/boards/mmc5.cpp` 三处 `0xFF` → `0`（`CHRBanksA/B`、`NTAMirroring/NTFill/ATFill`） | **会改变 MMC5 开机画面** ⇒ 必须跑 F11QA 回归；这是**行为变更**，需登记 |
| bandai x24c02 | `src/boards/bandai.cpp` 加设备地址校验：`(x24c02_addr & 0x78) != 0x50` ⇒ 转 STANDBY | 纯 bug 修复 |
| mapper 72 | `src/boards/72.cpp` 写窗口 `$6000-$FFFF` → `$8000-$FFFF` | 防呆 |
| mapper 19（N163） | `src/boards/n106.cpp` 启用 `SetupCartCHRMapping(0x10, NTARAM, 0x800, 1)` 与 `setchr1r` 路径 | **需真 N163 ROM 才能确认对错** ⇒ 无 ROM 前只登记不实施 |

---

### A8 · 上游新增 mapper 471 / 451 / 413（U7）

| 号 | 板子 | 代价 | 建议 |
|---|---|---|---|
| **471** | Impact Soft IM1（≈ mapper 201 + 扫描线 IRQ），65 行 | **低**：新文件 + `src/CMakeLists.txt` 的 `SRC_BOARDS` + `src/ines_bmap.h` + `MapperEntry` | ✅ **做** |
| **451** | Haratyler HP/MP（异形 MMC3 + CFI flash 存高分），216 行 | **中**：机制齐备（CFI 有 `coolboy.cpp` 先例），但**新增 5 个 SFORMAT 标签**（`FLSH/FLST/FLMD/FLBA/FLBV`）⇒ **存档格式变更** | ⏸ **建议转长尾**（为 1 款 homebrew 扩存档格式不划算；且 C++ 侧是 `addSaveGameBuf` 而 Rust 侧还缺 `flash_data` 的 savestate 通路） |
| **413** | Super Russian Roulette（4 寄存器 + 串行 PCM），147 行 | **高**：依赖本仓**不存在**的 `MiscROM` 全局与 misc ROM 区装载 ⇒ 等于**新增一条 ROM 数据通道** | ⏸ **转长尾**（需先立"misc ROM 区装载"子任务） |

**注意**：上游这三个号都**不在**本仓 `bmap` 里 ⇒ 移植必须**同时**登记
`src/ines_bmap.h` 与 `MapperEntry` 两条路径（否则就是 §1.4(A) 那种"装了但不工作"）。

---

### B1 · LT-08：DMC 计费范围界定

**为什么现在做**：业主筛选口径（性价比 ∈ {中,高} ∧ 难度 ∈ {中,低}）命中本条。

**取证坐标**（本仓）：`src/sound.cpp:741`（`DMCDMA()`）、`:869`（`FCEUSound` 里的
`DMCacc -= cycles` 循环）、`:872-899`（`while(DMCacc<=0)` 的取样与 `DMCacc+=DMCPeriod`）；
DMC 状态已迁到 `src/apu.cpp` / `src/apu.h`（`fceu11::g_apu`）。

**⚠️ 一条重要的取证结论（本轮新得）**：上游 master 的 `src/sound.cpp`
**与本仓的 DMC 模型同构**（同样的 `DMCDMA()` + `DMCacc` 循环），
且上游**没有** DMC 相关的改动（其 `changelog.txt` 最新条目停在 2013 年）。
⇒ **「参照上游实现」这条路在 LT-08 上拿不到新信息**，
本条必须靠 **NESdev wiki + 经典测试 ROM 的可观测行为**两路取证。

**做法**：① 取权威边界；② 写「DMC 触发 ⇒ 占多少 CPU 周期」的判定句；
③ 落在 APU/DMA 计费模型里并配锁测试。

**反向守卫**：**不得**用「042 用例的 DMA 仲裁结论」代替 DMC 结论；
结论必须带**三方交叉取证**（wiki / 测试 ROM / 参考实现），不接受单一来源。

**非目标**：不改 APU 混音模型；不追 DPCM 采样精度。

---

### B2 · LT-09：NES 侧人工验收 7 项

**本机可做 5 项**（一次会话做完）：V11 跨进程读档 / V13 录 AVI ≥10 分钟 /
V15 movie 回放 / V14 CDL 一致性读数 / V7 profiler 点击。
**需业主真机 2 项**（并行发起，不阻塞其余）：V6 麦克风吹气 / V25+V22 真机 Lua 与鼠标。

**反向守卫**：人工验收**不得**反过来修改 `expected` / `failure_means` / `pass_to_fail`
——人工项是**确认**，不是**调判据**。

---

### B3 · LT-10：GBA 3 张卡带人工确认

用已有的 `cartprobe` / 诊断探针跑出读数（哪张卡、卡在哪个 PC、踩了哪些未实现 SWI、
有无 panic）；**进仓库的是夹具读数，不是卡带读数**；再由业主肉眼确认画面与音效。

**反向守卫**：**不得**因为「人工看起来没问题」就把 GBA 的**兜底语义**
（未实现 SWI 返回并记录）改成「已实现」——`swi_unimpl_1a` 这条锁**故意**钉住「不实现」。

---

### B4 · LT-16：`start_delay = 2` 的独立来源取证

**已知缺口（长尾盘点 §五 已核对）**：v2.0.3 计划附录 A 的 GBATEK 快照
**对 TMxCNT_H 写入时序零覆盖** ⇒ **本条当前仍缺权威材料**。
⇒ **按纪律 4，取不到就停下来记录，不要写**。本条在 v2.0.5 的定位是：
① 先把「缺 TMxCNT_H 写入时序」登记进 §五 的阻塞表；② 若能取到第二套参照实现
（另一套模拟器的定时器语义）则交叉验证；③ **不动实现**。

---

### D1 / D2 / D3 · 裁决项

见 §2.3。**在业主点头前不动任何代码。**

---

## 四、交付批次建议（按性价比，不是日程）

- **批 1 · 零依赖、立刻可做**（都是"本仓缺陷"，收益最实）
  - **A3** 矫正活表补 22 条 + 修 2 条（纯数据 + 1 条锁）
  - **A1** 补 42 个 mapper 的注册（**先定 A1-a / A1-b 走哪条**）—— ✅ r2：已按 A1-a 落地（名单更正 75）
  - **A2 + A4** MMC3 家族 17 个变体接线 + mapper 45 窗口（**同批，防越界激活**）
  - **A5** mapper 4 的 NES 2.0 PRG-RAM（一处，插队）
  - **B2 / B3** 人工验收（与业主真机项并行发起）
- **批 2 · 有取证前置**
  - **A6** mapper 210（先做镜像语义取证）
  - **A7** 四项小修（MMC5 默认值需 F11QA 回归；mapper 19 等真 ROM）
  - **B1** LT-08 DMC（两路取证）
- **批 3 · 低优先 / 需裁决**
  - **A8-471** 上游新增 mapper 471
  - **A8-451 / A8-413** ⇒ 转长尾（存档格式 / 新 ROM 通道）
  - **B4** LT-16 取证
- **暂缓（等裁决 / 等材料）**：D1 / D2 / D3、LT-01 第二步、LT-05、LT-07、LT-02

---

## 五、v2.0.5 出口判据

1. **NES 零回归（不变式 1）**：`ctest --test-dir build --build-config Release --output-on-failure -LE perf` **41/41**（当前基线）+ 本计划新增的 mapper 锁全部进该命令。
2. **Rust 八条全绿**：`fceux11-rust`（GBA 根 299/0/20）、`gba-core`（249）、
   `fceux11-debug`（93）、`fceux11-media`（52）、`fceux11-formats`（**154 → 155**，
   A3 加 1 条）、`fceux11-utils`（51）、`fceux11-lua`（54）；
   两态 `cargo check --workspace` EXIT=0。
3. **新增强制门禁**：
   - **注册表分类完备断言**（r2 依 A1-a 口径改写）：bmap 里有实现的号必须有
     **有效 Power 路径**——注册表命中（forwarder→strategy 捕获）∨ 75 号名单
     （legacy 保留路径，S2 sweep 锁死）；两集合均被
     `fceux11_mapper_load_test` S1 钉死并自维护；
   - **板级 Power 行为锁**：每条 A2 的 mapper 至少一条行为锁，
     **断言必须指向该板自己的可观测差异**（不得用 `GenMMC3Power()` 当判据）。
     ⚠️ **r2 方法论警告：「写-回读」锁在本仓会被开总线回声欺骗**
     （未映射读返回 CPU 数据总线锁存 `g_cpu.db()`，而 `X6502_DMW` 恰好把写入值
     锁进 DB ⇒ 无 handler 也"读得回"）——A2 的锁优先用 **handler 身份
     （`GetReadHandler(...) == &CartBR`）或 EXPREGS 值**，用回读必须先毒化 DB 并证明非回声（同族风险：S5-A3 的 `check_mapper198_power` 未复核，见 §八 r2 ④）。
4. **变异验证双向**（纪律 3）：每条新锁都能"改回修复前的值 ⇒ 转红"，且**当场看到红**。
5. **F11QA 矩阵不回归**：`pass_to_fail` 必须为空；
   A7 的 MMC5 默认值属**行为变更** ⇒ 若金标漂移，必须走**基线评审**（LT-07 的地盘），
   **不得**在本计划里直接改 `expected`。
6. **版本口径**：`FCEU_VERSION_PATCH` 4 → 5，`FCEU_VERSION_NUMERIC` 20004 → **20005**
   （v2.0.4 的口径是整版递增，见 CHANGELOG `[2.0.4]`）；
   FCSX/FM2 版本字段信息性 ⇒ 旧存档/录像仍可读。
7. **CHANGELOG `[2.0.5]` 段** + Known limitations 草稿（人工验收未闭合项照旧登记）。

---

## 六、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 全量构建（Ninja 树必须串行，见 AGENTS.md 坑 12）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release -j 1"

# NES 门禁（不变式 1）
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 --output-on-failure -LE perf

# Rust 八条
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-formats --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-utils --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-lua --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace --no-default-features"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace"

# A1 的运行时读数（临时探针；实施后须并入正式门禁）
Research_only\w205\probe\registry_probe.exe

# A3 的数据反查
Select-String -Path src\rust\crates\fceux11-formats\src\ines\ines_data.rs `
  -Pattern '0x1d75fd35|0xb02fcb57|0xa912b064|0x243a8735|0x0da5e32e'
```

---

## 七、不做 / 已封口 / 明确登记

### 7.1 明确**不做**（避免下轮重复统计）

| 项 | 结论 | 依据 |
|---|---|---|
| 「上游 172 个板子文件都比我们新」 | ❌ **不成立**。99 个文件的差异只是 `#include "mapinc.h"`；其余落在本仓八类架构改造里 | §0.3 / §1.3 |
| MMC5 的 `MMC5SPRVPage` / `PALRAM` / `MMC5HackVROMPTR` | ❌ 本仓全有 | §1.3「被否证」表 |
| coolgirl 的 CFI flash / IRQ 时序 | ❌ 本仓逐字已有 | 同上 |
| `sb-2000.cpp` / `vrc5.cpp` 的"上游独有" | ❌ 是上游注释掉的死代码 | 同上 |
| mapper 7 ANROM / 97 / 101 / mmc1 submapper 5 / mapper 218 的 NTARAM 修复 | ❌ 本仓领先上游 | §1.5 |
| 上游的 CHR-RAM 下限保护 / mapper 修正写回 | ❌ 本仓已对齐（V10 / S5-A2） | 同上 |
| mapper 92 硬件模型替换 | ⏸ v2.0.5 不做（D3），登记为长尾 | §2.3 |
| mapper 210 电池存档（8KB `.sav` 新块） | ⏸ v2.0.5 不做（存档格式变更），登记为长尾 | §三 A6 注 |
| A8 的 mapper 451 / 413 | ⏸ 转长尾（存档格式 / 新 ROM 通道） | §三 A8 |

### 7.2 🔴 **必须更正的一条既有结论**（LT-03）

长尾盘点文档 §三 LT-03 记录：「dump B（mapper 252）**145 秒纯蓝屏**」，
并把它归因为「SMYNES 专用版，任何模拟器都跑不出画面」。

**本轮实测给出更早的解释**：本仓的 mapper 252 **板级 Power 从不执行**
（§1.4(A)：252 无 `MapperEntry` 注册 ⇒ `CartPlaceholder::on_power()` 空实现）
⇒ 写 `$8000-$FFFF` 的寄存器写**根本没被接住** ⇒ **蓝屏是本仓缺陷的症状**。

⇒ **更正口径**：
- LT-03 的**主结论不变**（"不需要 CRC 矫正条目"仍然成立，矫正表那条独立）；
- 但「蓝屏 = ROM 加密」这个**归因**必须改成「**本仓 mapper 252 未接线**」；
  ⇒ 本条的 A1 修完后，dump B 应重新实跑一次，看蓝屏是否消失
  （若消失 ⇒ 缺陷坐实；若仍在 ⇒ 回到「ROM 本身跑不动」的结论）。
- 这条更正**先改本文件与长尾盘点文档**，**不回头改归档计划**（AGENTS.md 纪律 2）。

### 7.3 已封口（沿用，不重启）

V8 · A7 别名化本体 · V7 的变异树全量门禁 · V23 的端到端上游对比 ·
GBA 侧「BIOS SWI 全部实现」—— 详见长尾盘点文档 §四。

---

## 八、变更记录

| r | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-15 | **首版**：① 上游快照身份核实为 **master 2.8.0**（非 2.7.0）；② 全量比对 172 个共有板子文件 + 装载基础设施层；③ **发现本仓两批静默失效缺陷**：42 个 mapper 号缺注册（**运行时实证**）、17 个 MMC3 家族变体 `on_power` 被基类顶掉；④ 归纳上游真优势 6 类（U1–U7）；⑤ 按业主口径纳入长尾 7 条（LT-03/04/08/09/10/16/17），登记 10 条排除；⑥ 给出 3 个裁决项（D1 LT-01 第一步 / D2 PGO 默认 ON / D3 mapper 92 模型）；⑦ 登记 LT-03 的归因更正 |
| r2 | 2026-11-16 | **A1 落地（A1-a）**：① **实施期更正：缺注册且板级 Power 非空的号是 75 个，不是 42**——r1 的名单漏了 `Latch_Init` 家族（它给每个调用者装 `LatchPower`）与委托 Init（`Mapper255_Init`→`Mapper225_Init`、`AA6023_Init`→`CommonInit` 等）；derive 方法 = 运行时探针（未注册全表）× 逐板 Init 链静态分析（剥注释后自洽：bmap 234 号 = 注册 159 + 未注册 75，**§1.2 的「270/273」也把 36 个注释条目算进去了，一并更正**）。② 修复 = `src/ines.cpp` 条件化（注册命中 ⇒ forwarder；未命中 ⇒ 保留板级 Power/Reset/Close，与 UNIF 路径同款）；**A1-b 否决**——`find_mapper` 上限 512 而 547 在名单里。③ 门禁落在 `fceux11_mapper_load_test`：S1 分类完备（bmap 每号 ∈ 注册表 ∪ 75 名单，自维护）+ S2 sweep（75 号合成 ROM 实装载断言保留，变异腿 75/75 红）+ S3 mapper 252 执行锚点（CartBR handler 身份 + PRG 标签字节）+ S4 控制组（注册号必须仍走 forwarder）。④ **实施期方法论发现（纪律 3 活例）**：「写-回读」锁在本仓**无牙**——未映射读返回 CPU 数据总线锁存（`Bus::ANullImpl` → `g_cpu.db()`）而 `X6502_DMW` 恰好锁存写入值 ⇒ 开总线回声让 0x5A 假绿；S3 第一版正中此坑，靠变异腿当场抓出后改为 handler 身份断言。⚠️ **同族风险登记：`check_mapper198_power`（S5-A3，$5000 写-回读）结构与旧版 S3 相同，其变异腿当年如何避开回声未复核——登记为观察项，不在本轮处置**。⑤ 门禁实测：目标级重建后 `fceux11_mapper_load_test` PASSED（75/75 + 锚点 0xFC）、`ctest -LE perf` **41/41** 全绿；变异腿（还原无条件覆盖）sweep 75 红 + S3 红 + S4 保持绿。⑥ **验收①口径适配**：A1-a 下「bmap 里每个号都必须在注册表里」不可达（75 个号按设计保持未注册）⇒ 语义改写为「bmap 里有实现的号必须有有效 Power 路径：注册表命中（forwarder→strategy 捕获）∨ 75 名单（legacy 保留，S2 sweep 锁死）」——S1 把两个集合钉死并自维护。⑦ §7.2 的 dump B 复跑（人工）仍待业主 |
| r3 | 2026-11-16 | **新增 §九 · 缺陷登记：BUG-V205-01（loader 误拒 CHR 非 2 幂 ROM ⇒ 错误弹框）**——业主人工验证 A1 时用 v2.0.4 发布包打开 `打砖块 [Nokoh汉化].nes`（mapper 140）触发反复错误弹框。**已实证的根因**：本仓 Rust loader 对 CHR 数量做 `uppow2` 舍入（与上游同）**并新增了上游没有的文件边界检查** ⇒ 「CHR 数量非 2 幂」（头 6 → 布局 8）的合法 iNES 1.0 ROM 被判为 "Unknown ROM file format."；**上游 2.6.6 / 2.8.0 不做该边界检查**（直接 `FCEU_fread`，读不足就少读）⇒ 上游能装、本仓不能 = **本仓独有的收紧（回归）**；v2.0.3 的 S4（`fb26feb`）修了 PRG 侧同类问题但 **CHR 侧未同步**。**影响面不止 mapper 140**：任何 CHR 数量非 2 幂的 iNES 1.0 ROM 一律被拒。**「反复弹框」的确切机制尚未实证**（已排除一个假设：模态框嵌套事件循环 ⇒ 定时器重入，最小复现证伪）；代码上一处可疑时序已登记（`ConsoleWindow.cpp:747-751` 的 `errorMsgValid` 在含模态 `exec()` 的 `showErrorMsgWindow()` **返回之后**才清零）⇒ 修复方向已明确，**本轮只登记不修**，待下次立项 |

---

## 九、缺陷登记（待修 BUG · 只登记不修）

> **本节登记「已实证的根因 + 修复方向」，但本轮不动代码**（业主指示：等下次修复）。
> 每条按「现象 / 实证链 / 根因分层 / 影响面 / 未实证部分 / 修复方向 / 验收 / 非目标」记，
> 纪律 4 适用：**位级格式与上游行为以代码为准，不凭记忆**。

### BUG-V205-01 · loader 误拒「CHR 数量非 2 幂」的合法 iNES 1.0 ROM ⇒ 错误弹框

**状态**：🔴 待修（登记于 2026-11-16）
**报告来源**：业主人工验证 A1 修复时撞上（用 v2.0.4 发布包 `C:\Users\ikrx2\Desktop\fceux11-v2.0.4-windows-amd64\fceux11.exe`
打开 `C:\Users\ikrx2\Desktop\roms\mapper140\打砖块 [Nokoh汉化].nes` ⇒ 反复错误弹框）
**与 A1 的关系**：**无关**（A1 是 mapper 注册接线；本条是 loader 的文件边界检查）——只是人工验证时顺带撞上

#### 9.1 现象

- 打开该 ROM ⇒ **装载失败** ⇒ 弹错误框（弹框文本 = `Unknown ROM file format.`）。
- 业主观察到的是**反复弹框**（本条的「反复」机制**尚未实证**，见 §9.5）。

#### 9.2 实证链（本轮实跑，探针在 `Research_only/w205/probe/b140_load_probe.cpp`）

**先证伪「ROM 是坏头」**——三个 ROM 的头部与文件大小**完全吻合**（delta = 0，无 trainer、无坏头）：

| ROM | 大小 | 头 PRG/CHR | flags6 | mapper | 头部需求 vs 文件大小 |
|---|---|---|---|---|---|
| 哆啦A梦 [Nokoh汉化].nes | 180240 | 8 / 6 | 0xC1（四屏） | 140 | 180240 = 180240 ✅ |
| 太空射击战 [Nokoh汉化].nes | 180240 | 4 / 14 | 0xC1（四屏） | 140 | 180240 = 180240 ✅ |
| 打砖块 [Nokoh汉化].nes | 81936 | 2 / 6 | 0xC0（水平） | 140 | 81936 = 81936 ✅ |

**再分层定位（探针逐层调用 Rust FFI）**：

```
[raw] parse_header   -> 1   mapper=140 nes2=0 prg16k=2 prgraw=2 chr8k=8   ← 头说 6，布局说 8
[raw] compute_layout -> 1   prg_bytes=32768 chr_bytes=65536 round=1
[raw] ines_load      -> 0                                            ← 边界检查在这里失败
[ERR] Unknown ROM file format.
```

- `chr8k=8` 的来源：`fceux11_rust_cart_compute_rom_sizes`
  （`src/rust/crates/fceux11-formats/src/cart.rs:94-96`）对 **CHR 数量也做 `uppow2`**
  ⇒ 头里的 6（= 49152 字节）被舍入成 8（= 65536 字节）。
- `ines_load -> 0` 的原因：`src/rust/crates/fceux11-formats/src/ines.rs:1052`
  的边界检查 `offset + chr_size > file_len`
  ⇒ 16 + 32768 + 65536 = 98320 > 81936 ⇒ **拒绝**。
- 三个 ROM 全中同一机制（180240 那两个同理：CHR 6→8、14→16）。

#### 9.3 根因分层

| 层 | 本仓 | 上游 2.6.6 基线 | 上游 2.8.0 master | 判定 |
|---|---|---|---|---|
| CHR 数量 `uppow2` | ✅ 有 | ✅ 有（`ines.cpp:860`） | ✅ 有（`ines.cpp:882`） | **与上游同** |
| **文件边界检查** | ✅ 有（`ines.rs:1052`，引入于 `1062394`「v1.10 Cryptex 全 Rust 迁移」） | ❌ **无**（直接 `FCEU_fread(VROM, 0x2000, VROM_size, fp)`，读不足就少读，VROM 尾部保持 `memset` 的 0xFF） | ❌ **无**（`ines.cpp:980` 同款） | **本仓独有 ⇒ 收紧即回归** |

⇒ **上游能装载这些 ROM，本仓不能**。这不是「上游比我们强」，是**本仓比上游更严**把合法 ROM 挡在门外。
⇒ **同一类问题的 S4 只修了一半**：v2.0.3 的 `fb26feb`（S4·FC 侧修复批次）把 **PRG** 侧的
边界检查改成「用头里的 raw 字节数」（GitHub issue #12 的非 2 幂 PRG），**CHR 侧没有同步改** ⇒
同一类遗漏，PRG 侧已修、CHR 侧仍在。

#### 9.4 影响面（**不止 mapper 140**）

- 任何 **CHR 数量非 2 幂**（3 / 5 / 6 / 9 / 10 / 12 …）的 **iNES 1.0** ROM 一律被拒 ⇒
  「装不上 → 报错弹框」与板子本身无关（mapper 140 只是本例）。
- 该目录三张 Nokoh 汉化 ROM 全中（CHR = 6 / 6 / 14）。
- 与 mapper 140 的**板级正确性无关**（业主已明确：暂不考虑 mapper 140 的支持）——本条只管
  「能不能装载」。

#### 9.5 未实证部分（⚠️ 如实登记）

- **「反复弹框」的机制未实证**：代码路径上，装载失败 ⇒ `FCEU_PrintError("Unknown ROM file format.")`
  （`src/fceu.cpp:590`）⇒ Qt `msgLog_PrintError`（`MsgLogViewer.cpp:334`）⇒
  `consoleWindow->QueueErrorMsgWindow()`（`ConsoleWindow.cpp:493`）⇒ 主线程 `update()` 里
  `showErrorMsgWindow()` 弹**模态** `QMessageBox`（`ConsoleWindow.cpp:479-491`）。
- **已排除的假设**：「模态框的嵌套事件循环让 gameTimer 重入 ⇒ 无限叠框」——
  最小复现（`Research_only/w205/probe/qt_modal_repro.cpp`，Qt6 + offscreen）**只弹 1 次**，
  假设**未获支持**（offscreen 平台是否派发定时器事件存疑，故不能据此断定真桌面行为）。
- **仍存疑的可疑时序**（修复时必须一并看）：
  `src/drivers/Qt/ConsoleWindow.cpp:747-751`
  ```cpp
  if ( errorMsgValid ) {
      showErrorMsgWindow();     // 内含 msgBox.exec()（模态）
      errorMsgValid = false;    // ← 只在弹框【关闭之后】才清零
  }
  ```
  标志的清零晚于模态框 ⇒ **若**框被关闭期间又有新错误入队，就会再弹一次。
  ⇒ 无论「反复」的真正成因是什么，**「先清标志再弹框」都是低成本防呆**，建议随本条一并修。
- **需要业主补充**（下次修前）：弹框的**确切文字与出现次数**（截图或复现描述）、
  是否**每次点掉都会再弹**、是否伴随 GUI 无响应。

#### 9.6 修复方向（下次立项时执行，本轮只登记）

1. **主修 · CHR 边界检查与 PRG 侧对齐（S4 同款）**：
   - **分配 / 映射 / 哈希**继续用舍入值（`uppow2`，上游同款，映射需要 2 幂）；
   - **边界检查改用头里的原始 CHR 数量**（raw bytes），读不足的部分按上游语义
     **用 0xFF 补齐**（上游 `memset(VROM, 0xFF, ...)` + `FCEU_fread` 少读 ⇒ 尾部即 0xFF）；
   - 落点：`src/rust/crates/fceux11-formats/src/ines.rs:1050-1058`（CHR 段）
     + `src/rust/crates/fceux11-formats/src/cart.rs:94-96`（把 **raw** CHR 数量单独传出）。
   - ⚠️ **不要**简单地把 `uppow2` 去掉——上游也舍入，且映射/哈希依赖舍入后的尺寸；
     要改的是**边界检查用哪个数**，不是**分配用哪个数**。
2. **附带 · 弹框防呆**：`ConsoleWindow.cpp:747-751` 改为**先清 `errorMsgValid` 再弹框**
   （或加 reentrancy guard），使「框未关完又有新错误」不会叠成无限弹框。
3. **验收**：
   - 三张 Nokoh ROM 全部能装载（`LoadGame` 返回非 null）；
   - 新增锁测试（落在 `fceux11-formats` 或 `fceux11_mapper_load_test`）：
     **CHR 数量非 2 幂的合成 ROM 必须能装载**（变异腿：把边界检查改回舍入值 ⇒ 必须转红）；
   - 与上游对照：同一 ROM 在上游 FCEUX 2.6.6 / 2.8.0 能装 ⇒ 本仓必须也能；
   - `ctest -LE perf` **41/41** 全绿（不变式 1）。
4. **非目标**：不追 mapper 140（JALECO JF-11/14）的板级正确性；不改 PRG 侧已修好的 raw 逻辑；
   不动 A1 的接线；不碰 GBA。

#### 9.7 复现步骤（下次修时用）

```powershell
# 探针（一次性，不入库）：分层打印 Rust loader 的每一步判定
Research_only\w205\probe\b140_probe_build.bat
# 期望（修复后）：三个 ROM 的 ines_load -> 1，且 LoadGame 返回非 null
# GUI 侧：Alt+F -> O -> 选 打砖块 [Nokoh汉化].nes -> 应能装载（不再弹 "Unknown ROM file format."）
```
