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

#### (B) 18 个 MMC3 家族 mapper：已注册，但板级 Power 被基类顶掉

> ⚠️ **r9 更正：17 → 18**。本节原名单**漏了 mapper 165**：r9 实施期逐个核对
> `info->Power` 的赋值点时发现 `Mapper165_Init` 设了 `info->Power = M165Power`，
> 且 **165 是已注册的**（`kMapper165Register`）⇒ 它的板级 Power 同样被 forwarder
> 吞掉。名单由 17 更正为 **18**。

`src/boards/mmc3_base_cart.cpp:20-26` 的 `Mmc3BaseCart::on_power()` **只调
`GenMMC3Power()`**；其注释声称「Each MMC3 variant subclass overrides on_power()」，
但 `src/boards/mmc3_variants_carts.h` 里 **26 个 MMC3 家族 Cart 类只有
`Mapper198Cart` 与 `Mapper199Cart` 真的 override 了**（外加 mapper 4 的 `Mmc3Cart`）。

⇒ **18 个已注册、但板级 Power 不执行的 MMC3 家族 mapper**：

```
12 37 44 45 47 49 52 114 115 116 165 195 205 245 249 250 254 406
```

（其余 6 个 `74 / 118 / 119 / 191 / 192 / 194` 的 Init 把 `info->Power` 留在
`GenMMC3Power` ⇒ 基类行为**正好正确**，**不得**给它们加 override。）

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
| **165** | `M165Power`：`EXPREGS[0] = 0xFD`（CHR-RAM 模式的种子） | CHR-RAM 模式永不启用（**r9 补进名单**） |
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
| **A1** | 补 42 个 mapper 的注册，恢复板级 Power（**r2 更正为 75 个号**） | **本仓缺陷** | **高** | 中 | **高** | ✅ **已完成（r2，A1-a）**：修复 + 门禁 S1–S4 + 变异验证 + `ctest 41/41`；`f09e284` 已推送 |
| **A2** | MMC3 家族 17 个变体的 `on_power` 接线 | **本仓缺陷** | **高** | 中 | **高** | ✅ **已完成（r9）**——**实际是 18 个号**（计划漏了 165，见 §1.4(B) 更正）：18 个 `on_power` override + 17 条板专属行为锁 + 双向变异验证 |
|  **A3**  |  矫正活表补 22 条 + 修 2 条  |  上游优势 U1  |  中  |  低  |  **高**  |  ✅ **已完成（r10）**：上游 2.8.0 全表 278 行重生成（活表 256 → 278）；**实到 22 条缺失 + 2 条过时**，另发现上游**有 23 条**带 C 表达式（多出的那条是 mirror 字段的 `0x10 \| 4`）⇒ 见 r10  |
| **A4** | mapper 45 取上游窗口（与 A2 同批，防越界） | 上游优势 U4 | 中 | 低 | **高** | ✅ **已完成（r9，与 A2 同批）**：写窗口 `$5000-$7FFF` → `$6000-$7FFF`（上游 2.8.0 同款）+ 越界锁；上游的 CHR/PRG mask 重写**本轮未取**（证据档位只有「上游这么写」，见 r9 ⑤） |
| **A5** | mapper 4 的 NES 2.0 PRG-RAM | 上游优势 U3 | 中 | 低 | 中 | ✅ 插队做（一处改动） |
| **A6** | mapper 210 Namco 175/340 | 上游优势 U2 | 中 | 中 | 中 | ✅ 做（证据档位见 §三） |
| **A7** | MMC5 / bandai / mapper 72 / mapper 19 四项小修 | 上游优势 U6 | 中 | 低～中 | 中 | ✅ 做 |
| **A8** | 上游新增 mapper 471 / 451 / 413 | 上游优势 U7 | 低 | 低～高 | 低～中 | 471 ✅ 做；451/413 见 §三 D 级 |

### 2.2 线 B：长尾纳入项（筛选：性价比 ∈ {中,高} ∧ 难度 ∈ {中,低}）

| 长尾号 | 条目 | 线 | 重要性 | 难度 | 性价比 | 是否纳入 | v2.0.5 的动作 |
|---|---|---|---|---|---|---|---|
| **LT-03** | 3GO.nes 的 CRC 矫正条目 | NES | 中 | 低 | 高 | ✅ **纳入** | 已闭合（无代码改动）；本轮只需**更正归因**（§七） |
| **LT-04** | PGO 一组可复现读数 | NES | 低 | 中 | 中 | ✅ **纳入** | 读数已闭合（+32%~37%）；尾巴是「**PGO 是否默认 ON**」⇒ **裁决项 D2** |
| **LT-08** | DMC 计费范围界定 | NES | 中 | 中 | 中 | ✅ **纳入** | **B1 ✅ r11 收口**：判定句「一次 sample fetch ⇒ 4 CPU 周期」+ 三方交叉 + 12 条新锁（formats 19→31） |
| **LT-09** | 人工验收未闭合（NES 侧 7 项） | NES | 中 | 低 | 高 | ✅ **纳入** | **B2 ⚠️ r11 部分闭合**：V14/V11 已给读数、**V15 实测不通过 ⇒ 另开缺陷**（见 §9.24）；V13/V7/V25+V22/V6 待业主真机 |
| **LT-10** | 人工验收未闭合（GBA 3 张卡带） | GBA | 低-中 | 低 | 高 | ✅ **纳入** | **B3 ⚠️ r11 流程就绪**：探针链自证通过；**本机 0 实体卡 ⇒ 待业主出卡**（脚本已备好） |
| **LT-16** | `start_delay = 2` 的独立来源取证 | GBA | 低-中 | 中 | 中 | ✅ **纳入** | **B4 ⚠️ r11 待裁决**：GBATEK 零覆盖 + 四套参照实现均无延迟 ⇒ 记为待裁决（低优先、不阻塞），未改实现 |
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

### 2.4 线 C：加载卡顿（**计划外 · 业主 2026-11-16 实测报上来**）

> 本线**不在 r1 的 mapper 主题内**，是业主用 v2.0.4_hotfix1 日常用 ROM 合集时
> 撞上的**加载卡顿**，本轮一并落地（见 §三 C1/C2 与 §八 r5）。

| # | 任务 | 类型 | 重要性 | 难度 | 性价比 | 建议动作 |
|---|---|---|---|---|---|---|
| **C1** | **加载一个 ROM 卡住约 8~10 秒**（音频设备探测在 GUI 持锁期间同步执行） | **本仓缺陷（driver 层）** | **高** | **低** | **高** | ✅ **已落地（r5）**：实测 **9.49 s → 0.29 s** |
| **C2** | **命令行加载中文名 ROM 必然失败**（`main()` 的 argv 是本地 ANSI(GBK)，文件层按 UTF-8 处理） | **本仓缺陷** | **高** | **低** | **高** | ✅ **已落地（r5）** |
| **C3** | `--no-console` 让命令行 ROM 被静默忽略 | 本仓缺陷 | 中 | 中 | 中 | ✅ **已修（r7，BUG-V205-02 §9.19）**：登记式跳过，不动 `Config` 类布局 |

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
| 21 条 `mapper\|0x1000`（256/257/258/259/260/406 等） | `256\|0x1000` … | **一条都没有** | 这些 ROM 拿不到 mapper 矫正 ⇒ 按头里的错 mapper 装载 |
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

### C1 / C2 · 加载卡顿与中文名路径（✅ r5 已落地，提交 `36af048`）

**为什么在 v2.0.5 线内**：业主的 ROM 合集（`Desktop\roms`，3903 个 .nes / 7806 个文件）
**绝大多数是中文文件名**，且日常是「双击 ROM 打开」⇒ C1（卡顿）与 C2（中文名打不开）
落在**同一个使用路径**上。

#### C1 · 加载 ROM 卡 8~10 秒

**现象**：业主 2026-11-16 实测报告「最新 exe 加载 ROM 很慢，要卡住十秒左右才能充分加载」。

**根因（已实证，业主机上复现）**：每次加载 ROM 都会走
`DriverInitialize()` → `InitSound()` → `SDL_OpenAudio()`；而本机 **WASAPI 后端找不到
音频端点**，`SDL_OpenAudio()` 要**整整 8 秒**才返回失败。**这 8 秒发生在 GUI 线程
持有 `FCEU_WRAPPER_LOCK` 期间** ⇒ 模拟线程被挡在锁外、窗口无响应 ⇒ 用户体感即
「卡十秒」。

硬证据（stderr 无缓冲 ⇒ 时间戳可靠；同一台机器、同一份 v2.0.4_hotfix1 包）：

```
t=  0.70s  Initializing video... Video Mode: 256 x 224 x 32 bpp
t=  8.70s  WASAPI can't find requested audio endpoint: 找不到元素
```

交叉验证：把 `SDL_AUDIODRIVER` 换成 `directsound` / `winmm` 后，**8 秒空档完全消失**
⇒ 归因在 WASAPI 后端本身，不在 ROM、不在 loader。

**改法**（`src/drivers/Qt/sdl-sound.cpp`，四条）：

1. **设备打开搬到后台线程** —— 调用方不再等待设备探测（核心修复）；
2. **已打开且采样率一致则复用** —— 旧代码每次加载都重开一次设备（**必失败**，
   SDL 只允许一个音频设备），且重开会把音频回调线程与环形缓冲一起拆掉重建；
3. **打开失败后退避 60 秒** —— 坏掉的端点不再每加载一次 ROM 就重付一次探测代价；
4. **驱动回退** —— WASAPI 失败后依次试 `directsound` / `winmm`。

⚠️ 第 4 条在本机**也失败**（三个后端都打不开）⇒ **本机 FCEUX11 本来就没有声音**；
这是**系统音频端点问题，不是本次改动造成的**（改动前同样无声，只是每次加载要多等 8 秒）。
现在失败会**明确报错**而不是静默。

**实测读数**（同机、同一个 ROM、判据 = 配置文件 `SDL.LastOpenFile` 被更新）：

| | ROM 加载完成时刻 |
|---|---|
| 旧版 v2.0.4_hotfix1 | **9.49 s** |
| 修复版 | **0.29 s** |

#### C2 · 命令行加载中文名 ROM 必然失败

**现象**（同批实测发现）：命令行传中文名 ROM ⇒ 装载失败（`SDL.LastOpenFile` 不被更新）；
英文名 ROM 同一份包 ⇒ 正常。

```
Joust (J).nes  -> cfg 的 SDL.LastOpenFile 被更新  => 加载成功
打氣球.nes     -> cfg 的 SDL.LastOpenFile 不变    => 加载失败
```

**根因**：Windows 的 `main()` 收到的是**本地 ANSI 代码页**（本机 936/GBK）的参数，
而 FCEUX 的文件层一律按 **UTF-8** 处理（`FCEUD_UTF8_fopen` 等在入口处把 UTF-8
转成本地代码页）⇒ 命令行传入的中文路径**永远打不开**。
⇒ **双击 ROM 打开（走命令行）这条路径，对中文名合集基本不可用**；
GUI 文件对话框不受影响（Qt 给的是 Unicode 路径）。

**改法**（`src/drivers/Qt/main.cpp`）：在 `QApplication` 建好之后，用
`QApplication::arguments()`（Windows 上从**宽字符命令行**解析）重建一份 **UTF-8**
的 `argv` 再交给驱动解析；缓冲区用 `static`（指针在整个进程生命周期内有效），
且**先填完所有 `std::string` 再取 `c_str()`**（避免 vector 扩容后指针失效）。

#### C1/C2 的门禁与验证

- `ctest --test-dir build --build-config Release -LE perf` **41/41 全绿**（全量重建后复跑）；
- `cargo check --workspace` EXIT=0；
- 改动只落在 **driver 层**（`sdl-sound.cpp` / `fceuWrapper.cpp` / `main.cpp`），
  **未触碰 core / Rust / mapper 逻辑** ⇒ v2.0.5 的 mapper 线不受影响；
- CHANGELOG `[Unreleased]` 已登记（C1、C2 两条 + C3 的 Known limitations）。

#### ⚠️ 未实证 / 留尾巴（如实登记）

1. **GUI 点击路径未端到端验证**：本轮的执行会话是**无交互桌面**（`SendKeys` 被拒、
   截屏失败），⇒ 只能验证命令行路径；**请业主在自己的桌面上双击一个中文名 ROM
   复核**（命令行路径已验证通过）。
2. **本机音频三后端全失败 ⇒ 本机无声**（与本次改动无关，详见 C1 的 ⚠️）。
3. **启动后仍有一个非主线程满载单核**（旧版同样存在，**与加载慢无因果关系**）：
   已用线程起始地址定位到本仓 exe 内的线程，但**具体函数未定位** ⇒ 登记为观察项，
   待业主机上确认是否复现。
4. **本轮踩到的两个测量陷阱**（写给下一轮，避免重复踩）：
   - **子进程 stdout 是全缓冲**（重定向到文件/管道时）⇒ 进程被 kill 会丢缓冲 ⇒
     「日志里没有某行」**不能**当作「代码没执行」的证据（判据应改用 stderr、
     或进程存活期间的配置项 mtime）；本轮曾据此两次误判。
   - **别在构建/链接进行中复制 exe**（link.exe 会先移除旧文件）⇒ 本轮拿到过半成品，
     误判成「加载彻底不工作」。

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
8. **线 C（加载卡顿 / 中文名路径）已落地并经同一门禁覆盖**：C1 实测 **9.49 s → 0.29 s**、
   C2 中文名 ROM 命令行可加载（提交 `36af048`）；C3（`--no-console`）**已随 r7 修复**
   （BUG-V205-02 §9.19，红绿双腿 E2E 实证），CHANGELOG 相应条目已从 Known limitations 改为 Fixed。
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
| r4 | 2026-11-16 | **BUG-V205-01 修复落地（v2.0.4_hotfix1）**：① 改法 = **边界检查与拷贝改用头里的原始 CHR 数量**，**分配/映射/哈希仍用舍入值**（上游同款；⚠️ 不是去掉 `uppow2`——映射与 ROM 矫正表的 CRC 键都依赖舍入后的缓冲）；落点 = `cart.rs`（新增 `vrom_size_raw`）+ `ines.rs`（两个 FFI 结构体透传 + 边界检查改 raw + 哈希传 `(raw, padded)`）+ `ines_load.cpp`（CHR 拷贝用 raw 长度，尾部按上游语义补 0xFF）。② 门禁 **3 条新锁**：`fceux11-formats` 基线 **154 → 156**（CHR=6 可装 + 短文件仍被拒）+ `fceux11_mapper_load_test` 端到端（装载成功 + 文件字节在位 + 尾部 0xFF）；**变异验证双向**：边界检查改回舍入值 ⇒ Rust 锁转红（155/1）+ 端到端锁转红 + 「短文件仍被拒」保持绿。③ 实测：三个 Nokoh ROM 全部能装（修复前三个都是 Unknown ROM file format）；`ctest -LE perf` **41/41**；Rust 八条全绿（根 299/0/20 · vendor 249 · debug 94 · media 52 · formats 156 · utils 51 · lua 54 · 两态 check EXIT=0）。④ 版本口径：`FCEU_HOTFIX_TAG = "_hotfix1"` ⇒ `v2.0.4_hotfix1`，⚠️ **`FCEU_VERSION_NUMERIC` 保持 20004**（hotfix 冻结数值）⇒ v2.0.4 的存档/录像仍可读；产物内字符串已回查确认。⑤ **§9.6 方向 2（Qt 弹框防呆）本轮未做**——「反复弹框」机制仍未实证（假设已被最小复现证伪）⇒ 无证据改动不做，降级为观察项。⑥ **本 hotfix 只取 BUG-V205-01 这一项**，A1 属 v2.0.5 线，不并入本次发布 |
| r5 | 2026-11-16 | **线 C 落地（计划外 · 加载卡顿）**：① **C1 根因**——每次加载 ROM 都走 `DriverInitialize → InitSound → SDL_OpenAudio`，本机 WASAPI 后端**找不到音频端点**，`SDL_OpenAudio` 要**整整 8 秒**才失败，而**这 8 秒在 GUI 线程持有 `FCEU_WRAPPER_LOCK` 期间** ⇒ 模拟线程被挡、窗口无响应；证据 = stderr 时间戳 `0.70s → 8.70s`，交叉验证 = 换 `SDL_AUDIODRIVER=directsound/winmm` 后 8 秒空档**完全消失**。② **改法四条**（`sdl-sound.cpp`）：设备打开**搬到后台线程**（调用方不再等探测）/ 已开且采样率一致则**复用**（旧代码每次加载都重开一次、必失败，还会拆掉重建音频回调线程与环形缓冲）/ 失败后退避 **60 秒** / WASAPI 失败后**回退 directsound、winmm**。③ **实测：ROM 加载完成时刻 9.49 s → 0.29 s**（判据 = cfg 的 `SDL.LastOpenFile` 被更新，不用 stdout——**stdout 重定向时是全缓冲**，进程被 kill 会丢缓冲，本轮据此误判过两次）。④ **C2**：命令行中文名 ROM 此前**必然失败**（`main()` 的 argv 是本地 ANSI(GBK)，文件层按 UTF-8 处理）⇒ 改用 `QApplication::arguments()`（宽字符命令行）重建 UTF-8 `argv`（`main.cpp`）。⑤ **C3 未修**：给配置解析器加「无值开关」注册机制 ⇒ **改 `Config` 类布局后 exe 在参数解析前异常退出（原因未查明）**，已回退；改用「从 argv 剔除该开关」⇒ **下标错位会加载错文件**，已回退 ⇒ 登记 **BUG-V205-02**（本仓原有行为，非本轮引入）。⑥ 门禁：全量重建 EXIT=0、`ctest -LE perf` **41/41**、`cargo check --workspace` EXIT=0；改动**只在 driver 层**，未触碰 core/Rust/mapper 逻辑。⑦ **未实证**：本轮执行会话是**无交互桌面**（`SendKeys` 被拒、截屏失败）⇒ **GUI 点击路径未端到端验证**，待业主机上复核；**本机三个音频后端全失败 ⇒ 本机 FCEUX11 无声**（与改动无关，改动前同样无声）；**启动后仍有一个非主线程满载单核**（旧版同样存在、与加载慢无因果，函数未定位，登记为观察项）。⑧ 提交 `36af048`（**已推送到 `main`**）。 |
| r6 | 2026-11-16 | **新增 §九 · 缺陷登记：BUG-V205-03（关闭 ROM 后立刻打开 ROM ⇒ GUI 永久卡死）**——业主人工实测 v2.0.4_hotfix1 发布包，触发样本 `mapper21/格拉米世界2.nes`；业主补充触发条件：**先「关闭 ROM」再「立刻打开 ROM」**。① **同日 21:33 完成真复现**（业主机，有交互桌面 + 正常 GL：NVIDIA 582.28 / GL 3.3.0）：业主手动操作、机器全程记录 ⇒ **卡死复现**（`HANG at t+128.6s`，app `Responding=False`，连续 6 次快照等待链逐字一致 ⇒ 死锁稳定非瞬态）。② **根因已实证（等待链 + 双方线程栈 + 窗口枚举 + app 日志四路互证）**：**原生（IFileDialog / COMDLG32）文件对话框的 COM 工作线程 tid=8512 卡在 `EnableWindow` 的 `SendMessage` → GUI 线程**（栈：`COMDLG32 → windows.storage → combase → RPCRT4`），而 **GUI 线程 tid=4380 卡在 `IFileDialog::Show()` 的 COM/RPC 等待**（栈：`KERNELBASE → ntdll!ZwWaitForMultipleObjects → combase → RPCRT4`）⇒ **互等死锁**；物证：**「打开 ROM 文件」窗口仍存在但已隐藏**（`exec()` 永不返回）+ **第二次 ROM 装载已完成**（stdout 有第二次 `Loading … mapper21` → `Power on` → `Initializing video`）⇒ 卡住的是 GUI 线程，不是装载。③ **推翻了本条初版登记的根因**：初版基于静态分析写成「包装锁协议失衡」（全局计数 / 跳过解锁 / 抢锁失败即放弃 / 无 preempt 让位 —— 与上游 2.8.0 的四处差异）——**等待链证明 GUI 线程节点里没有任何锁**（无 CriticalSection / Mutex）⇒ 那四处差异**独立存在但与本条无因果**，已降级为顺带修项。④ **仍缺**：为什么「必须先关闭 ROM」尚未做对照实验（决定修复面大小）；「每线程 CPU」仍缺（`GetThreadTimes` 对更高权限进程 `ACCESS_DENIED`）；上游 2.8.0 是否同病未核。⑤ **本轮建成取证装置**（`Research_only/w205/probe/`，gitignored）：`launch_watch.py`（启动 + 全程记录：等待链 / 线程栈 / 截图 / app 日志）、`stack_scan.cpp`（不依赖展开器的栈扫描）、`pe_export.py`（地址→导出符号）、`wct_dump.exe`、`shot.ps1`。⑥ **修复方向**：先做对照实验 ⇒ 主修 = **不要在文件对话框的 COM 回调里同步装载 ROM**（推迟到对话框完全关闭之后）；规避 = `SDL.UseNativeFileDialog` 默认改 0（属产品决定）；兜底 = GUI 取包装锁带超时。⑦ 本轮**只登记不修** |
| r7 | 2026-11-16 | **BUG-V205 三条收口（业主指示「完成 BUG-V205 部分收口」）**。① **BUG-V205-02 已修**（§9.19）：`configSys.cpp` 新增**文件级静态登记表** + 自由函数 `fceux11_config_register_pass_through_flag`（**零头文件改动**，规避 r5 类布局翻车），`_parseArgs` 长参数未知分支先查表 `continue`（不吞值、不动下标）；`fceuWrapperPreInit` 的 `--no-console` 分支登记。**E2E 红绿双腿**（探针 `v205_e2e.py`，判据 = 独立 `FCEUX_CONFIG_DIR` 生成 cfg 的 `SDL.LastOpenFile`）：修复后 exe 上 A（`--no-console <rom>` 装载成功）/ B（无 flag 逐字一致）/ C（中文名回归完好）/ D（无 ROM = 默认值，`config.cpp:773` 预期）全绿；**修复前红腿用 v2.0.4_hotfix1 发布包 exe 当场复现**（A 用例 LastOpenFile=默认值 ⇒ ROM 被吞）。② **BUG-V205-03 主修 #3 落地**（§9.20）：`openROMFile` / `loadNSF` 的装载从对话框调用栈内改为 `QTimer::singleShot(0, this, …)`（栈完全出栈后主事件循环执行；调用面核实：全部调用方为 fire-and-forget 槽）；`SDL.UseNativeFileDialog` 默认值不动（业主产品决定）、包装锁超时兜底与 D1–D4 维持登记（E10：与卡死无因果）。⚠️ **死锁场景自动化无法触发（E12）⇒ 修复效果待业主人手复核**（复刻「关闭→立刻打开」鼠标路径）。③ **BUG-V205-01 尾巴（§9.6 方向 2 防呆）落地**：`ConsoleWindow.cpp` `update()` 改为先清 `errorMsgValid` 再弹框（防叠框 / 防丢错；「反复弹框」机制仍未实证，防呆性质不变）。④ **同机制观察项登记**：`loadStateFrom`（对话框后同步 `FCEUSS_Load`）理论上同属暴露面，不在报告路径，未随本次改动。⑤ 门禁：全量构建 EXIT=0（65 步）+ `ctest -LE perf` **41/41**；改动全在 driver 层（`drivers_common` + `drivers_qt`），Rust 八条不在影响面；`git diff --numstat` 无行尾重写。⑥ 未做（超出 BUG-V205 范围）：版本号 / `[2.0.5]` CHANGELOG 段 / push 均待业主指示 |
| r8 | 2026-11-17 | **BUG-V205-03 终形态修复（STA 引用垫片）+ v2.0.4_hotfix2 发布**。① r7 落地后业主实测死锁仍在（发布包与修复版哈希一致）——第二轮手操复现 + 升级取证（双栈扫描）证明**环完全在 Windows 侧对话框实现内部**：对话框机制跑在内部宿主线程上（GUI 栈无 COMDLG32 帧），其收尾 `EnableWindow(owner)` 为跨线程 SendMessage，与 GUI 线程在 Show() 转发调用里的 COM-only 等待互等；两栈零本仓帧、零本仓锁。② 套间假说（BeeWare #930 同型）经 APTPROBE 全程实测排除（全程 STA）。③ **意外发现**：探针版不再复现（业主指出）——探针的 S_FALSE 不配对 = 每次对话框打开净 +1 STA 引用。④ **A/B/C/D 对照**：B（无垫片）5 次观察全部 ≤2 轮死锁；A/D（保持 +1）合计 18 开零死锁；C（净 0）>5 轮才死 ⇒ 垫片达工程决策标准（P≈0.1%），同时 C 证伪「泄漏至拆除」最简机制（如实登记为未闭合）。⑤ 正式落地：`openROMFile`/`loadNSF` 在 exec 前 `CoInitializeEx(STA)` 不配对；研究探针移除；`SDL.UseNativeFileDialog` 默认 1 不变（预授权的「改 0」不再需要）。⑥ 版本 `v2.0.4_hotfix2`（NUMERIC 20004 冻结不变，hotfix1 口径）；坑 6 复现并处置（version.h 改后 5 个 TU 未重编致产物残留 hotfix1 字符串，touch 强制后回查干净）。⑦ 门禁：全量构建 EXIT=0 + `ctest -LE perf` **41/41**；业主人手终验（循环多轮）无卡死，确认可发。⑧ 发布产物内容如实口径：本 hotfix2 段只记 BUG-V205-02/03 + V205-01 防呆尾巴；C1/C2（hotfix1 二进制已含、未入发布口径）与 A1（v2.0.5 线）维持原口径不动，由业主定稿时取舍 |
| r9 | 2026-11-17 | **A2 + A4 落地（18 个 MMC3 家族板级 Power 接线 + mapper 45 写窗口收窄）**。① **实施期更正 §1.4(B)：17 → 18 个号**——`Mapper165_Init` 设了 `info->Power = M165Power` 且 mapper 165 **是已注册的** ⇒ 它的板级 Power 同样被 forwarder 吞掉。② **改法** = `src/boards/mmc3_variants_carts.h` 里 18 个类各加一个 `on_power() noexcept override`（用宏 `FCEUX11_MMC3_VARIANT_CART_WITH_POWER` 展开），实现放在 `src/boards/mmc3.cpp`（与既有的 `Mapper198Cart`/`Mapper199Cart` override 同一位置、同一写法）；16 个板级 Power 函数去掉 `static`（`M12Power`…`M406_Power`），mapper 116 的 `UNLSL12Power` 在 `src/boards/116.cpp` 同样去 `static` + 在 mmc3.cpp 加 `extern` 声明（先例：`M195Power`/`M199Power`）。**`74 / 118 / 119 / 191 / 192 / 194` 明确不加 override**——它们的 Init 把 `info->Power` 留在 `GenMMC3Power`，基类行为正好正确，加了反而跑错 Power。③ **门禁** = `fceux11_mapper_load_test` 新增 A2 段：**18 号逐号断言 + 1 条 mapper 45 的 A4 窗口锁**；断言一律用「板专属可观测」，三种技术并用——`EXPREGS`/`MMC3_cmd`/`DRegBuf`（**装载前投毒 `0xA5`**，只有板级 Power 会复位它们 ⇒ 板级 Power 没跑就留毒）、handler 身份（对比通用 `MMC3_CMDWrite` / `CartBR` / `CartBW`）、以及 mapper 195 的 WRAM 窗口（写-读之间**毒化开总线锁存**，避开 r2 发现的「写-回读无牙」陷阱）。**对照组**：mapper 116 的「通用 handler 已不在」断言配一条「普通 MMC3 的 `$8000` 必须仍是 `MMC3_CMDWrite`」的控制断言，否则该断言会空转。④ **变异验证双向**（纪律 3）：**M1**（18 个 override 改回 `GenMMC3Power()`）⇒ **17 条锁 + A4 锁全部转红**，红点逐条落在毒值 / 通用 handler 上（`EXPREGS` 停在 `0xA5`、116 的 handler 变回 `MMC3_CMDWrite`、195 的 handler 消失且读回 `0x00`、250/406 的 `MMC3_cmd`/`DRegBuf` 落到通用路径的值）；**M2**（A4 窗口改回 `$5000-$7FFF`）⇒ **只有 A4 锁转红**，其余 16 条保持绿 ⇒ 两类锁互相独立。还原走备份 + **显式刷新 mtime**（`Copy-Item` 保留源时间戳 ⇒ 还原腿会假红，AGENTS.md 已登记的坑）。⑤ **A4 的范围收窄（本轮刻意只取一半）**：**取上游的写窗口**（`$5000-$7FFF` → `$6000-$7FFF`，上游 2.8.0 `src/boards/mmc3.cpp:555` 同款；本仓 `M45Write` 执行 `WRAM[A - 0x6000]` ⇒ `$5000-$5FFF` 段是**负下标越界写**，A2 接线会把它从休眠态唤醒，**先接 A2 不取 A4 = 把越界写变成活的**）；**上游的 `M45CW`/`M45PW` mask 重写与奇地址锁位复位分支本轮未取**——证据档位只有「上游这么写」，无权威出处，且会改这些卡的 CHR/PRG 银行规则 ⇒ **登记为待立项项**（需真 mapper 45 ROM + 基线评审，见 §9.22 ②）。⑥ **mapper 245 如实登记为「无锁」**：`M245Power` 只种 `EXPREGS[0]=0` 再调 `GenMMC3Power()`，而 `GenMMC3Power` → `MMC3RegReset` → `FixMMC3CHR` → `cwrap` → `M245CW` **立刻把 `EXPREGS[0]` 覆写成 CHR 银行值（`0x07`）** ⇒ 板级路径与基类路径**可观测状态完全相同**，写不出有牙的锁（写出来的必是空转断言）。override 仍保留（语义正确、无副作用），但**没有锁覆盖** ⇒ 如实登记，不假装覆盖。⑦ **门禁读数**：全量构建 EXIT=0、`ctest -LE perf` **41/41**、A2 段（对照组 + 17 条锁 + A4 锁）全绿。⑧ **新登记的观察项（只登记不修）**：这 18 个板的**板级 Reset** 同样被 `CartInfo_ResetForward` 顶掉（`Mmc3BaseCart::on_reset()` 只调 `MMC3RegReset`）⇒ 复位时板的私有寄存器 / 开关语义（mapper 12 的中英语言位、mapper 45 的 `EXPREGS[5]` 锁位、mapper 205 的焊盘位）不会按上游走 ⇒ **A2 本轮只做 Power**，Reset 侧转下一项（见 §9.22）。 |
| r10 | 2026-11-17 | **A3 落地（矫正活表补 22 条 + 修 2 条）**。① **取数**：上游快照 `src/ines-correct.h`（2.8.0）**278 行**；本仓活表 `src/rust/crates/fceux11-formats/src/ines/ines_data.rs::ROM_CORRECTIONS` 实为 **256 条** ⇒ 差 **22 条**。② **根因确认（计划 §三 A3 的判断成立）**：v0.2.19 的 Python 抽取器**解析不了 C 表达式** ⇒ 带表达式的行被**整行丢掉**。新写抽取器（`Research_only/w205/a3_regen.py`，gitignored）能求值 `\|` 表达式，`256\|0x1000 → 4352`、`51\|0x800 → 2099`、`0x10\|4 → 20`；并带**安全门**：只允许「+22 行 / 改 2 个值 / 挪 1 行」三种差异，其余差异一律中止写文件。③ **计划未预见的三点（如实更正）**：**(a) 上游的表达式行是 23 条不是 22**——21 条在 **mapper** 字段（`mapper\|0x1000`）+ 1 条在 **mapper** 字段（`51\|0x800`）+ **1 条在 mirror 字段**（`0x243a8735` 的 `0x10\|4`）；后者**不是被整行丢掉，而是表达式被静默截断成 0** ⇒ Major League 的单屏镜像退化成水平（这才是「修 2 条」里的第 2 条的真正来源）；**(b) 上游把 Urusei Yatsura 那条从 mapper-101 区块挪到了 mapper-87 区块**（因为它把 mapper 从 101 纠正成 87）⇒ 活表里该行**位置**随之更新（查找是按 CRC 线性查 ⇒ **行为中性**，只让未来与上游的 diff 变简单）；**(c) 本仓活表与上游 2.8.0 的**其余** 256 行**逐字相同、且顺序相同** ⇒ 本仓活表是上游的严格子集，无第三方私货。④ **两侧既有语义已核对，不需改逻辑**：Rust 侧 `ines.rs:309-326` 已认 `0x1000`（mask 取 `0xFFF`）与 `0x800`（`clear_vrom`）；C++ 侧 `src/ines_load.cpp:306-307` 的 `Mirroring >= 0x10 ⇒ SetupCartMirroring(2 + (Mirroring & 1), 1, 0)` 与**上游 2.8.0 `src/ines.cpp:1060-1061` 逐字一致** ⇒ mirror 写 20 即可生效。⑤ **门禁**：`fceux11-formats` 新增 **2 条锁**——`the_c_expression_correction_rows_reach_the_loader`（22 条逐条断言**查找结果**：mapper 号 / mask / clear_vrom / mirror，期望值**手抄自上游**、不由同一套表达式推导）+ `the_two_corrected_correction_rows_match_upstream`（Major League mirror=20、Urusei mapper=87+mirror=1，外加**负向守卫** `0xF3C85DC3`（整文件 CRC，不是表键）必须继续查不中）。⑥ **变异验证双向**：**M1**（删掉 `0x1d75fd35` 一行）⇒ 第一条锁红（`crc 0x1d75fd35: mapper left -1 right 256`）；**M2**（把两条被修正的值改回旧值）⇒ **第二条**锁红（`Major League: mirror left 0 right 20`）⇒ **两条锁互相独立**。还原走备份 + **显式刷新 mtime**（cargo 按 mtime 指纹，不刷新会跑变异版二进制）。⑦ **⚠️ 行尾事故与新坑**：第一次重生成把新片段写成 LF ⇒ 整个文件变成**混合行尾**（CRLF 1055 + LF 1391）——**git 看不出来**（`core.autocrlf` 规范化，`git diff --numstat` 仍是 +116/−6）⇒ 已修正为**全 CRLF**（2446/0）并把备份挪出源码树 ⇒ **登记为坑 11 的新变体：脚本改仓库文件必须沿用原文件行尾，且改完立刻回查 CRLF/LF 计数**。⑧ **业主 ROM 合集的端到端核对（新增，非计划要求）**：按 loader 的哈希规则（PRG+CHR，**补 0xFF 到 2 幂**后算 CRC32）扫 `Desktop\roms` 的 **3903 个 .nes**（跳过 3 个非 iNES + 1 个 NES 2.0）⇒ **22 条新键里命中 1 条、2 条被修正的里命中 1 条，且两处都精确对上上游注释里的游戏名**：**`mapper51\球类11合1.nes`**（CRC `0xa912b064` = 上游的 “11-in-1 Ball Games”）⇒ 现在按 mapper 51 + **清掉本不该存在的 CHR ROM**（`clear_vrom` ⇒ `vrom_size_8kb = 0`）装载；**`mapper32\职棒大联盟.nes`**（CRC `0x243a8735` = 上游的 “Major League”）⇒ **单屏镜像**（20）取代水平（0）。⇒ **这两张卡在修复前后画面应当不同，是业主可立即手工复核的两个样本**。⑨ **读数**：`fceux11-formats` **158/0**（156 → 158）；Rust 八条全绿（根 **299/0/20** · vendor **249** · debug **94** · media **52** · **formats 158** · utils **51** · lua **54** · 两态 `cargo check` EXIT=0）；`ctest -LE perf` **41/41**。 |
| r11 | 2026-11-17 | **线 B 四条并行落地（LT-08 / LT-09 / LT-10 / LT-16，四个 agent 并行切分，按文件独占）**。① **B1 · LT-08 DMC 计费范围界定 ⇒ 已界定 + 已落锁**：判定句「**DMC 的一次 sample fetch ⇒ 占 4 个 CPU 周期**」（口径 = CPU 周期非 master；不含 DMA 抢占仲裁——本仓无仲裁模型故恒为 4；扣账时点 = `$4015`(bit4=1) 写之后的**下一条指令 opcode fetch 之后**）；**三方交叉全部达成**——权威（NESdev `APU_DMC`「The CPU is stalled for 1-4 CPU cycles to read a sample byte」+ `DMA`「…taking 3 or 4 cycles」；⚠️ 直连+代理 5 条路径全被 Cloudflare 403、Wayback 0 字节 ⇒ **正文经检索索引转录，已如实降档**）／参照实现（上游 **2.8.0 与 2.6.6 的 6 个 DMC 区域逐字相同** ⇒ 上游无 DMC 改动；本仓与上游仅 2 处差异且**均不在计费路径**）／实测（blargg `dmc_tests/latency.nes` PASS，**blargg 自己的测例源码写死 `; 4 DMC wait-states`**（`sync_dmc.s:44`）⇒ 与权威数字对上；来源＝本仓 `scripts/download_blargg_roms.ps1:24` 同一镜像）。**本仓已符合**（4 项主口径全符合），5 处亚周期差异按 LT-08 非目标**登记不改**（恒 4 而非 3-or-4／取样晚 0–6 周期／无 RDY halt／无 DMC-OAM DMA 仲裁重叠／page-cross 惩罚晚一条指令计入）；**未借用 042 用例的 DMA 仲裁结论**（反向守卫守住）。**门禁** = `tests/f11qa/core/apu_test.cpp` 新增 **12 条断言**（19 → **31**，**纯插入 +112/−0**），用 `cpu_instance().timestamp()` **实测差值**断言。**变异双向**：腿 A（`==4`→`==3`）红 `FAIL one DMC sample fetch steals exactly 4 CPU cycles`（30/1）、腿 B（`==per`→`==per+1`）红 `FAIL DMCacc wraps forward by exactly one DMC`（30/1）、还原 31/0 绿；**父 agent 另补子 agent 因文件归属没做的「强变异腿」**（改**模拟器**而非锁：`src/sound.cpp:745-748` 的 DMC 取样 4 读 → 3 读 ⇒ 锁**转红** 30/1 ⇒ 证明锁读的是模拟器真实路径，不是空转断言）。② **B2 · LT-09 人工验收 7 项 ⇒ 部分闭合 + 挖出一个真缺陷**：**V14**（CDL 一致性）已给读数（6 fixture × 120 帧）但**计划字面等式 6/6 不成立**（差值恰为 `code_and_data`）⇒ **并集式** `code+data-both+undef==size` 6/6 **精确成立** ⇒ **只登记判据口径更正、不改基线**（§9.24）；**V11**（VRC7 跨进程读档）机器侧全闭合（读档后 60 帧音频 47917 样本逐字节相同、60 行帧哈希全同、OPLL 指针重建经变异证明 `OPLL_forceRefresh` 在跑）⇒ 只剩「音色」需业主人耳；**V15 movie 回放 ⇒ 🔴 实测不通过**（录出的 `.fm2` **长度对但全 0**、回放 180/180 帧不一致）⇒ **父 agent 逐行核实根因并另立缺陷条目（§9.23）**：`src/movie_fm2.cpp:414` 写完就 `fread`，而 Rust `EmuFileMem::read_into` 是**位置读** ⇒ 读回 0 字节、零初始化的 `tmp` 被整段写出 ⇒ **修法一行**（`fceux11_rust_emufile_mem_fseek(mem,0,0)`，该 API 已存在且 Rust 侧自带测试即此范式）⇒ **本轮只登记不修**（影响面：所有用户录制的 movie 都不可用 ⇒ 建议优先于批 2/3）；**V13** 校验器已备好并正反自测（36000 帧合成 AVI → PASS；尾部截 40 字节 → FAIL）；**V7 / V25+V22 / V6** 需业主在有桌面的机器上做（步骤/判据/误判排除已写全）。③ **B3 · LT-10 GBA 三卡 ⇒ 流程就绪，待业主出卡**：用 **3 张合成夹具卡 + 否定性对照** 自证探针链（执行了 ROM 机器码／读数随 ROM 变化／SWI 按 ARM 约定解码 93972 calls NOT SERVED／走了 stub BIOS 向量 281914 次／记录器非无条件打印／两次运行读数一致）⇒ **六条断言全过**；**挖出两个真坑**：① `steps/frame` **不是**预算上限（帧边界由 LCD 决定，判 wedge 要看 `frames that exhausted the budget`）② **`GBA_PROBE_ONLY` 是子串匹配**（`src/rust/src/gba/cartprobe.rs:80-81` 原文）⇒ 脚本改为**每卡独立工作目录**；**更正派工书一处**：`waitprobe.rs` 是 AGS 等待态测量工具，与卡带分诊无关 ⇒ 未使用；**本机两次全盘搜索 0 实体卡** ⇒ 本条不能闭合，交付「一键脚本 + 读数模板 + 自证报告」。④ **B4 · LT-16 `start_delay = 2` ⇒ 待裁决（低优先、不阻塞）**：GBATEK 当前版**零覆盖**（定时器章只写 reload 语义，全文 412 万字符关键词穷举无一条讲使能→计数延迟；仓内快照 `.zcode/gbatek_plain.txt:4782-4865` 与当前版逐字一致 ⇒ 缺口是「GBATEK 没写」不是「我们没抄」）+ **四套独立参照实现全部「无延迟」**（mGBA master `190a68d6` `src/gba/timer.c:134-151`、`timer.h:22-27` 无延迟字段；VBA-M `GBA.cpp:83 timerOnOffDelay` 推迟使能沿但无「空 N 周期」常量；neser `gba/bus/timer.rs:100-104`；rustboyadvance-ng `core/src/timer.rs:143,195`）⇒ 参照实现层信号**与 AGS 方向相反** ⇒ 按 LT-16 反向守卫**记为待裁决、不做静默取舍**；**未改实现、未改任何 AGS 期望值**；并钉死「若裁决为无延迟会变红的三个 AGS 锚点」（`TIMER PRESCALER` 的 `timers.rs:432-445`／`WAIT STATE WAIT CONTROL` `bus.rs:1893-1905`／`CARTRIDGE RAM WAIT CONTROL` `bus.rs:1888-1891`；`TIMER CONNECT` 不受影响）⇒ **建议路径 = 真机/FPGA 最小测量**（唯一能把档位提到「硬件规格级」的路），次优 = 给 GBATEK 作者提缺口反馈。⑤ **重新盘点（代码复核）**：**线 A 4/8**（A1/A2/A3/A4 ✅；A5/A6/A7/A8 ⬜ —— 逐项 0 命中复核）、**线 B 4 条 = 1 条收口 + 1 条部分闭合 + 2 条待外部条件**、**线 C 3/3 ✅**、**裁决项 D1/D2/D3 全未决**、**§九 BUG-V205 3/3 ✅** ⇒ 计划内 15 项完成 **7 项 ≈ 47%**。⑥ **读数**：`fceux11-apu_test` **31/31**、Rust 八条全绿（根 299/0/20 · vendor 249 · debug 94 · media 52 · formats **158** · utils 51 · lua 54 · 两态 check EXIT=0）、`ctest -LE perf` **41/41**。 |

---

## 九、缺陷登记（待修 BUG · 只登记不修）

> **本节登记「已实证的根因 + 修复方向」，但本轮不动代码**（业主指示：等下次修复）。
> 每条按「现象 / 实证链 / 根因分层 / 影响面 / 未实证部分 / 修复方向 / 验收 / 非目标」记，
> 纪律 4 适用：**位级格式与上游行为以代码为准，不凭记忆**。

### BUG-V205-01 · loader 误拒「CHR 数量非 2 幂」的合法 iNES 1.0 ROM ⇒ 错误弹框

**状态**：✅ **已修（2026-11-16，v2.0.4_hotfix1）** —— 见 §9.8；本条同时保留为**回归锁**的来源
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

#### 9.8 ✅ 修复实施记录（2026-11-16 · v2.0.4_hotfix1）

**改法**（= §9.6 的方向 1；方向 2 的 Qt 防呆**本轮未做**，见下）：

| 落点 | 改动 |
|---|---|
| `src/rust/crates/fceux11-formats/src/cart.rs` | `FceuRomSizes` 新增 `vrom_size_raw`；`compute_rom_sizes` 把它算成**头里的原始 CHR 银行数**（iNES 1.0 取 `header[5]`；NES 2.0 取「高位拼装」或指数记法的精确值）。`vrom_size_8kb` 仍是**舍入值**（分配/映射用） |
| `src/rust/crates/fceux11-formats/src/ines.rs` | `FceuInesParseResult` / `FceuInesCartResult` 各自新增 `vrom_size_raw` 并透传；**边界检查改用 `chr_raw_bytes`**（`offset + chr_raw_bytes > file_len`）；**哈希改传 `(chr_raw_bytes, chr_size_bytes)`** ⇒ 与 PRG 侧同款（读 raw、补 0xFF 到 padded） |
| `src/ines_load.cpp` | CHR 拷贝改用 `cart.vrom_size_raw << 13`（文件实有字节数），**尾部保持 `memset` 的 0xFF**（= 上游语义） |

⚠️ **不是「去掉 `uppow2`」**：上游 2.6.6 / 2.8.0 也舍入，且**映射与 ROM 矫正表的 CRC 键都依赖舍入后的缓冲区** ⇒
要改的是「**边界检查用哪个数**」，不是「**分配用哪个数**」。

**门禁（3 条新锁）**：
- `fceux11-formats`（**基线 154 → 156**）：
  - `a_non_power_of_two_chr_rom_loads_with_raw_slices` —— 头 CHR=6 的 ROM **能装**；
    `chr_size` = 8 银行（舍入、分配用）、`vrom_size_raw` = 6（文件用）；CHR 切片 = 文件的 6 银行；
    哈希 = **raw CHR + 0xFF 补到舍入值**（= 上游口径，且**必须**≠ raw-only 哈希）。
  - `a_truncated_chr_rom_is_still_rejected` —— 头说 6 银行、文件只有 4 银行 ⇒ **仍被拒**
    （防呆：修复不得把硬错误变成读越界）。
- `fceux11_mapper_load_test`（端到端，C++ 侧）：CHR=6 的合成 ROM（mapper 140）**装载成功**
  + 文件自己的 CHR 字节在位（`0xEB`）+ **尾部是 0xFF**（`pad[0]=pad[last]=0xFF`）。

**变异验证（纪律 3）**：把边界检查改回舍入值 ⇒
`a_non_power_of_two_chr_rom_loads_with_raw_slices` **转红**（155 passed / 1 failed），
`a_truncated_chr_rom_is_still_rejected` **保持绿**（正确：短文件两种实现下都该被拒），
C++ 端到端锁**转红**（同源变异）⇒ 三条锁都有牙。

**实测读数**：
- 三个 Nokoh ROM 全部 `ines_load -> 1` 且 `LoadGame` 返回非 null（修复前三个都是 "Unknown ROM file format."）。
- `ctest -LE perf` **41/41** 全绿；Rust 八条全绿（根 299/0/20 · vendor 249 · debug 94 ·
  media 52 · **formats 156** · utils 51 · lua 54 · 两态 `cargo check` EXIT=0）。
- 版本口径：`FCEU_HOTFIX_TAG = "_hotfix1"` ⇒ `FCEU_DISPLAY_VERSION = "v2.0.4_hotfix1"`；
  ⚠️ **`FCEU_VERSION_NUMERIC` 保持 20004**（hotfix 冻结数值，与 v2.0.2_hotfix1 同一口径）
  ⇒ v2.0.4 产出的存档/录像在本版**仍可正常读取**。产物内字符串已回查确认
  （`FCEUX11 v2.0.4_hotfix1`）。

**未做 / 留尾巴**：
- ~~**§9.6 方向 2（Qt 弹框防呆）本轮未做**~~ → **✅ 已随 r7 落地**（BUG-V205 三条收口时一并处理，见 §9.8 末尾的 r7 注）：`ConsoleWindow.cpp` 的 `update()` 改为**先清 `errorMsgValid` 再弹框**——防呆性质不变（「反复弹框」的机制仍未实证），但消除了两类确定的缺陷面：模态框存续期间重入的 `update()` 会看到仍为真的标志而**叠出第二个模态框**；以及模态框存续期间新入队的错误被返回后的清零**静默丢弃**。清零提前后语义变为「每个入队错误恰好弹一次：不叠框、不丢错」（错误文本本身由 `showErrorMsgWindow()` 在包装锁下先行取出并清空，时序不受影响）。
- **mapper 140 的板级正确性仍未验证**（业主已明确暂不考虑）：本条只保证「能装载」。

### BUG-V205-02 · `--no-console` 让命令行 ROM 被静默忽略

**状态**：✅ **已修（2026-11-16，r7）** —— 实施记录见下方 §9.19
**报告来源**：线 C 的 C1 排查过程中顺带实测发现（同批次的 C2 中文名问题已修）

#### 现象

- `fceux11.exe --no-console <rom>` ⇒ **ROM 根本没加载**（配置项 `SDL.LastOpenFile`
  不被更新 ⇒ 可据此判定）；不带 `--no-console` 的同一条命令**正常**。

#### 根因（已实证）

`Config::parse()` → `Config::_parseArgs()` 遇到**不认识的参数直接 `return -1`**
（`src/drivers/common/configSys.cpp`），而 `--no-console` 是**驱动自己消费**的开关
（`fceuWrapperPreInit`，v0.3.15.x PHASE-3 引入），**不在配置项表里** ⇒ 解析失败 ⇒
解析器把「ROM 参数」当成不存在。

⚠️ 这是本仓**原有**行为，**不是本轮引入的回归**；本轮只是把它暴露并登记。

#### 本轮试过的两种修法（**都已回退**，勿重复踩）

| 修法 | 结果 | 为什么不采用 |
|---|---|---|
| 给解析器加「无值开关」注册机制（`Config::addFlag()`，改 `Config` 类布局） | ❌ **exe 在参数解析前异常退出**（stdout 只到 DirectStorage 探测、探针未打印、无 diag log）⇒ **原因未查明** | 不把未查清的改动带进发布；**改公共类布局有 ODR/ABI 风险**（部分 TU 未重编 ⇒ 对象布局不一致） |
| 把该开关从 `argv` 里剔除后再交给解析器 | ❌ **下标错位**：`parse()` 返回的是**参数下标**，而 `argv[romIndex]` 取的是**原数组** ⇒ 加载错文件 | 同上；正确做法必须是「不改变下标」的识别方式 |

#### 修复方向（下次立项）

给配置解析器加一个**不改变 `Config` 类布局**的开关注册接口（例如把开关集合放在类外的
静态表，或由调用方传入一个 `std::set`），使解析器认得 `--no-console`、跳过它、
**既不消费下一个参数也不改变任何下标**。

#### 验收

- `fceux11.exe --no-console <rom>` ⇒ ROM 被加载（`SDL.LastOpenFile` 被更新为该 ROM）；
- 不带 `--no-console` 的行为**不变**；
- `ctest -LE perf` **41/41** 全绿。

#### 9.19 ✅ 修复实施记录（2026-11-16 · r7）

**改法**（= 上面「修复方向」的原话落地；两处落点，**零头文件改动**）：

| 落点 | 改动 |
|---|---|
| `src/drivers/common/configSys.cpp` | 新增**文件级静态表** `fceux11_passThroughFlags()` + 登记函数 `fceux11_config_register_pass_through_flag(const char*)`（外部链接自由函数）；`_parseArgs` 的长参数分支在 `return -1` 之前先查该表，命中则 `continue`——**不吞下一个参数、不改变任何下标**（`parse()` 返回的是原 argv 的下标） |
| `src/drivers/Qt/fceuWrapper.cpp` | `fceuWrapperPreInit` 顶部的 `--no-console` 分支补一行登记（外部函数用 `extern` 声明引入，与 `main.cpp` 的 `extern bool g_noConsole` 同款先例） |

⚠️ **为什么不用类成员/头文件**：r5 第一次回退的教训（改 `Config` 类布局 ⇒ exe 在参数解析前异常退出，原因未查明）——本方案**不触碰 `configSys.h`**，`Config` 的对象布局逐字不变，ODR/ABI 面为零。
⚠️ 只登记 `--no-console`：`--help`/`-h`/`--version`/`--no-gui` 在 `fceuWrapperPreInit` 里**直接 exit**，永远到不了 `parse()`，登记它们是死代码。

**门禁与双向验证**（判据 = 独立 `FCEUX_CONFIG_DIR` 下生成的 `fceux.cfg` 的 `SDL.LastOpenFile` 行；探针 `Research_only/w205/probe/v205_e2e.py`，gitignored）：

| 用例 | 修复后（`build/src/fceux11.exe`） | 修复前（v2.0.4_hotfix1 发布包 exe） | 判定 |
|---|---|---|---|
| A `--no-console <ASCII 名 ROM>` | **LastOpenFile = 该 ROM** ⇒ 装载成功 | LastOpenFile = **默认值（配置目录）** ⇒ ROM 被吞 | ✅ 修复生效，**红腿当场实证** |
| B `<ASCII 名 ROM>`（无 flag） | 装载成功 | 装载成功（值逐字一致） | ✅ 行为不变 |
| C `<中文名 ROM>`（无 flag） | 装载成功 | 装载成功 | ✅ C2 回归完好 |
| D `--no-console`（无 ROM） | LastOpenFile = 默认值（`config.cpp:773` 的缺省就是基准目录，**预期行为**） | 同左 | ✅ 无假装载 |

⇒ A 用例**修复前红 / 修复后绿，双腿齐**（纪律 3）；B/C/D 修复前后逐字一致 ⇒ 无行为回归。
⇒ 全量构建 EXIT=0（65 步）+ `ctest -LE perf` **41/41** 全绿；改动全部在 driver 层（`drivers_common` + `drivers_qt`），Rust 八条不在影响面。

**验收残留**：无。本条验收命令三行全部当场实测通过。

### BUG-V205-03 · 「关闭 ROM」后立刻「打开 ROM」⇒ GUI 永久卡死（**原生文件对话框 × COM 回调死锁**）

**状态**：✅ **已修（2026-11-17，r8 终形态：r7 推迟装载 + r8 主线程 STA 引用垫片），随 v2.0.4_hotfix2 发布**；实施与 A/B/C/D 对照实验记录见 §9.20/§9.21；业主人手终验通过（循环多轮无卡死）
**报告来源**：业主人工实测 —— 发布包 `C:\Users\ikrx2\Desktop\fceux11-v2.0.4_hotfix1-windows-amd64\fceux11.exe`（= 本仓 `main` @ `ba35b0f` 之后）
**业主补充的触发条件**：**先「关闭 ROM」，再「立刻打开 ROM」**；触发样本 = `C:\Users\ikrx2\Desktop\roms\mapper21\格拉米世界2.nes`（mapper 21 = Konami VRC2/VRC4 A）。

> ⚠️ **本条在同一天内被修正过一次**：初次登记时基于静态分析把根因写成「包装锁协议失衡」（见下方 §9.13 的 D1–D4）。
> **该推演已被 21:33 的真复现推翻** —— 等待链与线程栈证明死锁**不在包装锁上**，而在
> **Windows 原生文件对话框（IFileDialog / COMDLG32）的 COM 回调路径**。
> 保留 D1–D4 作为**独立存在的协议缺陷**（仍值得修），但**不是本条卡死的原因**。

#### 9.9 现象

- 已有一个 ROM 在跑 ⇒ 「文件 > 关闭 ROM」⇒ **马上**「文件 > 打开 ROM」⇒ 选同一个 ROM ⇒ 「打开」
  ⇒ **窗口彻底卡死**：菜单点不动、画面停在最后一帧、任务管理器里进程还在（**不是崩溃**）。
- **第二次装载其实完成了**：app 的 stdout 显示第二次 `Loading ... mapper21/格拉米世界2.nes` →
  `Power on` → `Initializing video...` → `NTSC mode set` **全部打印完毕** ⇒
  **ROM 数据没问题，卡住的是 GUI 线程**（画面停在最后一帧 = GUI 线程不再 repaint）。

#### 9.10 触发路径（代码坐标已核对）

| 步 | 动作 | 代码 | 说明 |
|---|---|---|---|
| ① | 文件 > 关闭 ROM | `ConsoleFile.cpp:304-309` `closeROMCB()` | GUI 线程持包装锁跑 `CloseGame()` |
| ② | 文件 > 打开 ROM | `ConsoleFile.cpp:205-293` `openROMFile()` | **原生**文件对话框（业主 cfg `SDL.UseNativeFileDialog = 1`）⇒ `dialog.exec()` 进入 **COM 等待** |
| ③ | 选中文件 → 打开 | `ConsoleFile.cpp:287-290` | **在对话框的 COM 回调里**同步跑 `CloseGame()+LoadGame()`（stdout 证实第二次装载已完成） |

#### 9.11 ✅ 根因（**已实证**）：原生文件对话框的 COM 回调 × GUI 线程 ⇒ 互等死锁

**死锁环（等待链 + 线程栈实测，见 §9.13）**：

```
① 文件对话框的 COM 工作线程（第一次 tid=8512 / 第二次 tid=8800）
   栈：COMDLG32.dll+0x13620 → USER32.dll+0x20f1b → win32u!NtUserEnableWindow
       → COMDLG32.dll+0xc69e0 / +0x6cbf2 / +0x841e4 → windows.storage.dll → combase → RPCRT4
   卡在：关闭对话框时**重新启用 owner 窗口**（= FCEUX 主窗口）
        ⇒ 这是**进程内同步 SendMessage** ⇒ 必须由 GUI 线程处理
   等待：GUI 线程回消息

② GUI（主）线程（第一次 tid=4380 / 第二次 tid=996）
   栈：RPCRT4!NdrClientCall3 → RPCRT4!NDRSContextUnmarshall2
       → combase!CoWaitForMultipleHandles+0x1de9/+0xbe4
       → KERNELBASE!WaitForMultipleObjectsEx+0x123 → ntdll!ZwWaitForMultipleObjects
   卡在：**正在等 Show() 那个 RPC 调用的回包**
       ⇒ `CoWaitForMultipleHandles` 是 STA 消息泵：**只派发 COM/RPC 消息，不派发普通 Win32 消息**
       ⇒ ① 那条 EnableWindow 永远排不上
   等待：对话框结束
```

⇒ **对话��的工作线程等 GUI 线程回消息，GUI 线程等对话框结束 ⇒ 互等 ⇒ 永久死锁。**

**物证（三条互相印证）**：

1. **app 里那个「打开 ROM 文件」窗口仍然存在、但已隐藏**（枚举窗口：`Qt6110QWindowIcon` / 标题 `打开 ROM 文件` / `IsWindowVisible = False`）
   ⇒ **对话框关了，但 `dialog.exec()` 永不返回**（`exec()` 等的是 `Show()` 返回，不是窗口销毁）。
2. **GUI 线程的栈顶是 `combase.dll` + `RPCRT4.dll`** ⇒ 它确实卡在 COM/RPC 里，而不是在 FCEUX 自己的代码里。
3. **等待链里 8512 的节点是 `SendMessage → tid=4380`** ⇒ 对话框线程在等 GUI 线程；而 GUI 线程的节点里**没有指向任何锁**（不是包装锁！）。

**为什么画面停在 X1 logo**：GUI 线程卡住 ⇒ 120 Hz 的 `gameTimer` 不再触发 ⇒
`emuFrameFinish()/updatePeriodic()` 不再跑 ⇒ `transferVideoBuffer()` 不再把帧贴到窗口
⇒ **窗口永远显示最后一帧**（启动时的 FCEUX11 logo）。

**为什么「关闭 ROM 后立刻打开」才触发（当前最可能解释，仍属**推演**）：
原生（现代）文件对话框在打开时会**禁用 owner 窗口、关闭时再 `EnableWindow` 启用**；
这个「启用」是**同步 `SendMessage`** ⇒ 必须由 owner 所在线程处理。
**关闭 ROM 之后**，GUI 线程在对话框的 COM 回调里还要同步做一次完整的 `CloseGame()+LoadGame()`（重活），
于是 `EnableWindow` 恰好落在这段**不泵消息**的窗口里 ⇒ 死锁；
而「不关闭就直接打开」时，ROM 已在跑，回调里的工作量/时序不同 ⇒ 未必命中。
⚠️ **这一条尚未做对照实验**（见 §9.14）。

#### 9.12 对本仓的影响面

- **必现于该路径**：只要「原生文件对话框」+「GUI 线程在 COM 回调里做重活」同时成立，
  Windows 11 的现代文件对话框就有这条死锁路径。
- **用户侧后果**：整个应用变砖（菜单/热键/退出全失效），只能杀进程；
  且此时 `SDL.AutoSaveState = 99` 的业主配置下**连自动存档都不会发生** ⇒ 未存档进度可能丢失。
- **与 ROM/板级无关**：mapper 21 只是本轮命中的样本（mapper 44 未命中 ⇒ 更像**时序**而非板级差异）。

#### 9.13 ✅ 已实证清单（可复核，原始件在 `Research_only/w205/probe/`）

| # | 事实 | 证据 |
|---|---|---|
| E1 | 真复现成功：业主在**有交互桌面 + 正常 GL**（NVIDIA 582.28 / GL 3.3.0）的机器上手动操作 ⇒ 卡死 | `launch_watch.py` 输出：`HANG at t+128.6s`；app `Responding=False` |
| E2 | 第二次 ROM 装载**已完成**（不是装载卡住） | app stdout：`Loading ...mapper21/格拉米世界2.nes` → `Power on` → `Initializing video...` → `NTSC mode set`（共 2 次 `Loading`） |
| E3 | 对话框工作线程卡在 `SendMessage` 给 GUI 线程 | 等待链：`tid=8512 nodes=3` → `[1] SendMessage status=6` → `[2] Thread tid=4380` |
| E4 | 该线程**就是文件对话框的 COM 工作线程** | 栈：`USER32+0x20f1b` → `COMDLG32+0x13620` → `windows.storage+0x4c2bb0` → `combase+0x1c1c8` → `RPCRT4+0xd2b00` |
| E5 | 它卡的具体调用是 `EnableWindow` | RIP = `win32u.dll+0x9604` ⇒ `NtUserEnableWindow`（导出表比对） |
| E6 | GUI 线程卡在 **COM/RPC 等待**，不泵 Win32 消息 | RIP = `ntdll+0x166f34` ⇒ `ZwWaitForMultipleObjects`；栈：`KERNELBASE+0x1ab53` → `ntdll` → **`combase+0xc6259` → `RPCRT4+0xd2b00`** |
| E7 | GUI 线程**没有**在等任何 FCEUX 自己的锁 | 等待链里 tid=4380 的节点只有 `Thread`，**无 CriticalSection / Mutex** ⇒ **推翻「包装锁失衡」假设** |
| E8 | 「打开 ROM 文件」窗口**仍存在但已隐藏** ⇒ `dialog.exec()` 永不返回 | 窗口枚举：`41067c Qt6110QWindowIcon '打开 ROM 文件' vis=False` |
| E9 | 死锁状态**稳定**（不是瞬态） | 连续 6 次快照（间隔 10 s）等待链逐字一致；GUI 线程 `ctxSwitches` 冻结在 205128 不再增长 |
| E10 | 包装锁协议差异（D1–D4）**确实存在**，但与本条卡死无因果 | 源码逐行对照（`fceuWrapper.cpp:1243-1309` vs 上游 `:1443-1606`）；E7 已排除其为死锁点 |
| E11 | **对照 A：不关闭、直接打开** ⇒ **不死锁**（第二次 `Loading` 完成、窗口持续应答 `WM_NULL`） | `control_exp.py`：`RESULT: HANG=False` |
| E12 | **对照 B：连续 3 轮「关闭→打开」**（自动化用**键盘 Enter** 关对话框）⇒ **不死锁** | `cycles.py`：3 轮全部正常 |
| E13 | **第二次独立复现（22:13，仍为业主人手操作）⇒ 栈与第一次逐字相同**（`COMDLG32+0x13620`、`USER32+0x20f1b`、`combase!CoWaitForMultipleHandles+0x1de9`、`RPCRT4!NdrClientCall3` 等偏移全部一致）⇒ **根因确证**（两次独立复现、同一死锁环） | `waitchain_6540_221355.txt` + 手工栈扫描（22:15） |

#### 9.14 ⚠️ 未实证部分（如实登记）

- **「为什么必须先关闭 ROM」已做两轮对照：「关闭」确为必要条件，但触发仍不稳定**：
  - 对照 A（**不关闭、直接打开**）⇒ **不死锁** ⇒ 「关闭」确实是必要条件；
  - 对照 B（**连续 3 轮 关闭→打开**，自动化用**键盘 Enter** 关对话框）⇒ **不死锁**；
  - 业主**人手操作**（鼠标点对话框的「打开」按钮）⇒ **死锁**（21:33 那次）。
  ⇒ **未定的剩余变量**：① 对话框是**鼠标点「打开」**还是**键盘 Enter** 关闭；② 会话状态（业主 cfg `SDL.AutoSaveState = 99` vs 实验用的 `0`）；③ 时序。
  ⇒ **22:13 的第二次业主复现（t+5.4 s 触发）栈与 21:33 逐字相同**，⇒ 死锁环本身已无疑问；剩余变量只决定**触发概率/触发面**，**不影响 §9.15 的三条修法**（三条都与「谁赢了这个竞态」无关）。
- **是否与 Windows 版本 / 文件对话框实现强相关**：本轮只在 Win11（26100）的
  **现代 IFileDialog**（`windows.storage` + `combase`）上复现；经典 `GetOpenFileName`
  路径是否也有此问题**未验证**。
- **上游 2.8.0 是否同病**：上游 `openROMFile()` 也用原生 `QFileDialog` + 同步 `LoadGame`
  （`ConsoleWindow.cpp` 同款结构）⇒ **疑似上游共有缺陷**，但**未逐行核对上游、也未实测**。
- **线程级 CPU 仍缺**：本轮 `GetThreadTimes` 对**别的权限级别进程**返回 `ACCESS_DENIED`，
  对本进程也未取到有效值（见 §9.19 的工具说明）⇒ **「哪个线程在空转」这一项仍缺**。

#### 9.15 修复方向（下次立项；按「先钉死、再动刀」）

1. **先做对照实验**（§9.14 第一条）：同一环境跑「不关闭、直接打开」，确认触发面。
2. **规避（零代码）**：把 `SDL.UseNativeFileDialog` 的**默认值从 1 改成 0**（改用 Qt 自带对话框
   ⇒ 不进 COM ⇒ 无此死锁路径）。⚠️ 这会改变所有用户的「打开 ROM」观感，**属产品决定**，
   应由业主拍板（可只作为「已知问题时的临时开关」）。
3. **主修（推荐）· 不要在文件对话框的 COM 回调里做重活**：把「装载 ROM」从对话框回调里
   **推迟到对话框完全关闭之后**（例如 `QFileDialog` 拿到文件名后用 `QTimer::singleShot(0, ...)`
   装载，或改用 Qt 版对话框 + 事件循环），使 GUI 线程在 `Show()` 返回前始终能泵消息。
4. **根治备选**：**在打开原生对话框期间让 GUI 线程继续泵消息**——需要确认 Qt 6 的
   `QFileDialog::exec()` 是否提供该能力（**未验证**）；若没有，则退回第 3 条。
5. **兜底**：GUI 线程取包装锁改带超时（§9.16 原建议，保留）⇒ 即使将来别的路径再失衡，
   也只是「一次操作失败」而不是「整个应用变砖」。
6. **顺手修 D1–D4**（§9.13 E10）：本仓包装锁协议与上游 2.8.0 的四处差异**独立存在**，
   建议在同一条线里对齐上游（`thread_local` 深度 + 无条件解锁 + preempt 让位 + 自旋取锁），
   但**不要**把它当成 BUG-V205-03 的修复。

#### 9.16 验收

- **对照实验**：同环境下「不关闭 → 直接打开」的行为被记录（死锁 / 不死锁），并写进本条；
- **复现脚本**：`launch_watch.py` 在修复后跑完 20 分钟**不再出现 HANG**；
- **回归**：「关闭 ROM → 立刻打开 ROM」能正常装载并继续模拟（stdout 出现**第三次** `Loading`
  之后仍有帧输出、窗口持续应答 `WM_NULL`）；
- **可选锁测试**（若顺手修 D1–D4）：构造「GUI 持锁 → 模拟线程反复抢锁 → GUI 再取锁」的时序，
  断言每轮后**锁深度归零且 QMutex 未锁**；变异腿（把 `fceuWrapperUnLock()` 改回「计数为 0 ⇒
  跳过解锁」）必须转红；
- `ctest -LE perf` **41/41** 全绿（不变式 1）；Rust 八条全绿。

#### 9.17 非目标

- 不改 NES 核心 / Rust / mapper 逻辑（本条纯 driver 层）；
- 不动 GBA 线（`decoupling.rs` 三条守卫照旧）；
- 不借本条改「自动存档」策略（`SDL.AutoSaveState` 语义保持）；
- **不把 §9.11 的机制解释（为什么必须先关闭）当成已证实的根因写进代码注释**（纪律 3）。

#### 9.18 本轮已建成的复现 / 取证装置（`Research_only/w205/probe/`，gitignored）

| 文件 | 作用 |
|---|---|
| `launch_watch.py` | **主装置**：用指定 ROM 启动发布包（stdout/stderr 实时落盘）→ 每 250 ms 探窗口是否应答 → 卡死瞬间自动抓等待链 + 线程 CPU + 截图 + stdout 尾部 → 写 `hang_report_*.txt`；**进程保持存活**供继续观察 |
| `stack_scan.cpp` | 线程栈扫描：读目标进程栈内存，列出所有指向模块的指针（**module+offset**），**不依赖展开器**（x64 展开器在符号处理器配置不对时会静默产出垃圾） |
| `pe_export.py` | 把地址解析成最近的导出符号（如 `ntdll+0x166f34` ⇒ `ZwWaitForMultipleObjects`）；同一 DLL 在各进程 RVA 相同 ⇒ 可用本进程副本解析 |
| `wct_dump.cpp` | 等待链转储器：`wct_dump.exe <pid>`；⚠️ `OpenThreadWaitChainSession` 的 Flags **必须传 0**（传 `WCTP_OPEN_ALL_FLAGS` 报 `ERROR_INVALID_PARAMETER`） |
| `shot.ps1` | 截屏（整桌面 / 指定进程主窗口）⇒ 用于「画面停在最后一帧」与「对话框窗口仍存在但隐藏」的取证 |
| `close_open_probe.cpp` + `close_open_build.bat` + `close_open_run.bat` | **库级探针**：链真实驱动库，在 Qt 主线程复刻「装载 → 关闭 → 打开」全序列，带崩溃栈捕获与看门狗；⚠️ 其原生对话框分支因自动化无法确认对话框关闭而**未跑通**（见 §9.14） |
| `uia_menu.ps1` / `uia_dump.ps1` / `gui_repro2.py` / `gui_repro3.py` | UI Automation 精确点击 Qt 菜单（可访问名带 `&`/`\t` 后缀 ⇒ **必须子串匹配**）+ 驱动原生对话框；**本轮最终改为人手操作、机器记录**（更可靠） |
| `thread_cpu.py` | 线程级 CPU 统计；⚠️ 两处坑：① `OpenThread` 的掩码**不能带 `THREAD_GET_CONTEXT`**（否则对别进程线程一律失败 ⇒ 静默空表）；② `GetThreadTimes` 对**更高权限级别的进程**返回 `ACCESS_DENIED`（本轮对本进程也未取到值 ⇒ 该项仍缺） |

> **本轮真复现的原始件**：`waitchain_8136_213323.txt`（等待链）、`shot_hang_213323.png` + `hang_desktop.png`（现场）、`app_stdout_213323.txt`（app 全程日志）。

#### 9.20 ✅ 修复实施记录（2026-11-16 · r7）

**改法**（= §9.15 第 3 条「主修（推荐）」原话落地；§9.15 第 1 条对照实验已由 E11/E12 完成，不重复）：

| 落点 | 改动 |
|---|---|
| `src/drivers/Qt/ConsoleFile.cpp` `openROMFile()` | `dialog.exec()` 返回并取到文件名之后，**不再在当前调用栈里同步跑 `CloseGame()+LoadGame()`**，改为 `QTimer::singleShot(0, this, lambda)` —— 装载推迟到本调用栈完全出栈、GUI 线程回到主事件循环之后执行。`SDL.LastOpenFile` 的写入时机不变（对话框关闭即写） |
| `src/drivers/Qt/ConsoleFile.cpp` `loadNSF()` | 同款改法（`LoadGame` 同属重活，且走同一个原生对话框路径）——同机制顺手加固 |
| `src/drivers/Qt/ConsoleFile.cpp` | 补 `#include <QTimer>`（此前 `singleShot` 靠传递包含） |

**为什么这样能断环（对应 §9.11 的死锁环）**：环的两条边是「对话框 COM 工作线程等 GUI 线程处理关闭期的 `EnableWindow(owner)`（普通 Win32 消息）」与「GUI 线程卡在 `Show()` 的 `CoWaitForMultipleHandles`（只泵 COM/RPC 消息）」。装载不再发生在对话框调用栈内 ⇒ GUI 线程在对话框收尾窗口内**始终处于正常泵消息的状态**（`exec()` 返回处直接出栈回主事件循环，`EnableWindow` 正常送达）⇒ 两条边都断开。装载与对话框关闭不再重叠，与触发概率/触发面（§9.14 的剩余变量）无关。

**调用面核实**：`openROMFile` 与 `loadNSF` 的全部调用方（菜单动作 + 热键槽，`ConsoleMenuBar.cpp:172/175/202`、`ConsoleWindow.cpp:622`）都是 fire-and-forget 的 Qt 槽，**无任何调用方依赖「函数返回时装载已完成」** ⇒ 推迟装载不改变调用契约。lambda 以 `this` 为上下文对象（主窗口为进程级单例，销毁即失效，无悬垂）。

**同机制观察项（不在本次范围，登记）**：`loadStateFrom`（`ConsoleEmuControl.cpp:113`，对话框后同步 `FCEUSS_Load`）理论上同属「Show() 收尾窗口内做重活」的暴露面——但它不在本条报告路径上、工作量远小于 `CloseGame()+LoadGame()`，**未随本次改动**；若业主人手复核时在该路径也复现卡死，按同款改法单独立项。`saveStateAs` / `fdsLoadBiosFile` 工作量更小，仅记录。

**门禁**：全量构建 EXIT=0 + `ctest -LE perf` **41/41** 全绿（改动全在 driver 层，Rust 八条不在影响面）；命令行装载路径（B/C 用例）修复前后逐字一致 ⇒ 无回归。

**验收残留（如实登记）**：
1. **死锁场景无法自动化触发**——E12 已证自动化（键盘 Enter 关对话框）不触发、业主人手（鼠标）触发 ⇒ **修复效果只能由业主人手复核**：在业主机上复刻「关闭 ROM → 立刻打开 ROM → 选同一个 ROM → 打开」（鼠标操作），确认不再卡死、stdout 出现第三次 `Loading` 且窗口持续应答。
2. **§9.15 第 2 条（`SDL.UseNativeFileDialog` 默认改 0）未做**——业主明确属产品决定，维持默认值不动；主修落地后它不再是必要规避，仅作已知问题时的临时开关保留。
3. **§9.15 第 5 条兜底（包装锁带超时）与第 6 条（D1–D4 对齐上游）未做**——独立于本条，保持登记状态（E10：与本次卡死无因果）。

#### 9.21 ✅ r8：STA 引用垫片（终形态修复）与 A/B/C/D 对照实验

**背景**：r7 落地后业主实测发现死锁**仍在**（发布包 exe 与修复版哈希逐字一致，`UseNativeFileDialog=1`）——触发面变化（此前 1 轮即卡，彼时需 2 轮、且卡在第二次对话框打开）但环未断。第二轮业主手操复现 + 升级取证（双栈扫描、间隔 5 秒）得到决定性证据：**两个线程栈里都没有任何本仓代码帧、WCT 的 GUI 节点没有锁**——环完全在 Windows 侧对话框实现内部：

```
对话框 COM 工作线程：NtUserEnableWindow ← USER32+0x20f1b ← COMDLG32+0x13620 ← windows.storage ← combase
                     （卡在 EnableWindow(owner) 的 SendMessage，status=6，跨 6 次快照稳定）
GUI 线程：ZwWaitForMultipleObjects ← KERNELBASE ← combase+0xc6259 ← RPCRT4 ← OneCoreUAPCommonProxyStub
         （卡在 Show() 转发调用的 COM-only 等待；ctxSwitches 每 10s 仅 +4 = 冻结）
```

健康 STA 下 `Show()` 不应被转发（GUI 栈里无 COMDLG32 帧）⇒ 顺着「套间状态不对才转发」的线索（BeeWare briefcase #930 同型：宿主把主线程 COM 初始化成错模型 ⇒ 原生对话框冻结）做**套间探针**：`CoInitializeEx(STA)` 分类 S_OK/S_FALSE/MTA，打在 main 入口、QApplication 后、SDL_Init 后、每次对话框打开前/后。**实测：全程 STA（S_FALSE），假说排除。**

**意外发现**：探针版**不再复现**死锁（业主敏锐指出「你是不是改过什么地方？」）。探针唯一的行为副作用：`S_FALSE` 分支**不配对 `CoUninitialize`** ⇒ 主线程 STA 引用计数每次对话框打开净 +1。据此做四腿对照（同机/同操作者/同 cfg/同 ROM）：

| 腿 | 二进制 | 对话框打开时的引用操作 | 结果 |
|---|---|---|---|
| B | 修复版原样（无探针） | 无 | **1–2 轮内死锁**（5 次独立观察一致，含业主最初日常使用报告） |
| A | 探针版（S_FALSE 不配对） | **净 +1×2（before/after 各一次），保持** | 4 开全绿 |
| C | 净减变异（S_FALSE 也配对） | **净 0**（+1−1 × 2） | >5 轮后才死 |
| D | 终形态垫片（同 A 形态） | 净 +1×2，保持 | **14 开全绿、零死锁** |

**判读**：垫片（保持 +1）合计 **18 次对话框打开零死锁** vs 无垫片 5 次观察全部 ≤2 轮死锁（同分布概率 ≈0.1%）⇒ 达工程决策标准，**垫片 = 真实修复**。⚠️ 如实登记两处未闭合：① C 腿与 B 腿在引用计数上**完全等价**（净 0）但行为显著不同（>5 轮 vs ≤2 轮）⇒ 「泄漏至套间拆除」这一最简洁机制解释**被 C 证伪**，瞬态对也改变了触发概率 ⇒ Windows 侧精确机制未钉死（修复按统计证据 + 最坏情况无害落地）；② 修复形态（每次打开一次 +1、不配对）与实测形态（两次 +1）差一次记账——实测定的是「保持性垫片有效」，发布取其一半（exec 前一次），对话框执行期间的保护等价。

**正式落地**（`src/drivers/Qt/ConsoleFile.cpp`）：`openROMFile` / `loadNSF` 在 `dialog.exec()` 前执行 `CoInitializeEx(NULL, COINIT_APARTMENTTHREADED)`，**故意不配对**（每次打开一个计数字，无资源对应；进程退出无需归零）；启动期研究探针全部移除。**`SDL.UseNativeFileDialog` 默认值保持 1**（业主预授权的「默认改 0」因机制级修复成立而不再需要；配置项保留，改 0 仍可作为彻底绕开 COM 的备选）。**同机制观察项保留**：`loadStateFrom` 等其余原生对话框路径未实测、未加垫片，若在其他路径复现卡死按同款处理。

**验证**：全量构建 EXIT=0、产物字符串 `v2.0.4_hotfix2`（回查确认，无 hotfix1 残留——坑 6 复现：version.h 改后 5 个嵌版本串 TU 未自动重编，已 touch 强制）、`ctest -LE perf` **41/41**；业主人手终验（循环多轮）**无卡死**。
### 9.22 · 观察项登记（r9 新发现 · 只登记不修）

> 这两条是 A2 实施期顺带查出来的**同族缺陷**，本轮**只登记**，不在 A2 的范围内动手。

**① 板级 Reset 同样被 forwarder 顶掉**

`info->Reset` 与 `info->Power` 走同一条路：注册表命中 ⇒ `iNESCart.Reset` 被覆盖成
`CartInfo_ResetForward` ⇒ `cart_obj->on_reset()` ⇒ `Mmc3BaseCart::on_reset()` **只调
`MMC3RegReset()`** ⇒ 各板自己的 Reset 函数**从不执行**：

| 板 | 被顶掉的 Reset | 上游语义 | 用户可感后果 |
|---|---|---|---|
| 12 | `M12Reset` | `EXPREGS[0]=EXPREGS[1]=0`；`EXPREGS[2] ^= 1` | **中英语言位不复位**（上游每次复位切语言） |
| 37 | `M37Reset` | `EXPREGS[0]=0` | 复位后扩展寄存器残留 |
| 45 | `M45Reset` | `EXPREGS[0..4]=0`；`EXPREGS[5]++`（锁位计数） | **锁位不复位**（A4 收窄后写入路径仍在） |
| 49 | `M49Reset` | `EXPREGS[0]=0` | 同 37 |
| 52 | `M52Reset` | `EXPREGS[0]=EXPREGS[1]=0` | 同 37 |
| 114 | `M114Reset` | `EXPREGS[0]=0` | 同 37 |
| 205 | `M205Reset` | `EXPREGS[0]=0`；`EXPREGS[1] ^= 2`（焊盘位） | **焊盘位不复位** |

⇒ **建议**：与 A2 同款做法（各 Cart 类 override `on_reset()` 直调板级 Reset），
但**必须自带行为锁**（例如 mapper 12 的语言位在复位后翻转、mapper 45 的
`EXPREGS[5]` 递增、mapper 205 的 `EXPREGS[1]` 翻转）⇒ 这三条的期望值依赖
「复位时到底该不该翻转」这一语义问题 ⇒ **需要业主口径**（上游就是翻转 ⇒
建议照上游），故本轮不自行决定。

**② 上游 mapper 45 的 CHR/PRG mask 重写本轮未取**

上游 2.8.0 的 `M45CW`/`M45PW` 与本仓的 2.6.6 版**结构不同**（上游把 CHR 掩码改成
`((EXPREGS[2] & 0x0F) > 7) ? ((1 << (EXPREGS[2] & 0x0F) << 3) - 1) : 0`，
PRG 改成 `MV |= (EXPREGS[1] & 0x3F & mask) | (EXPREGS[1] & 0xC0)`），上游的
`M45Write` 还多一条**奇地址锁位复位分支**（`EXPREGS[3] &= ~0x40`）。
本轮**只取写窗口**（安全必需项），**未取 mask 重写与锁位分支** ⇒ 依据：
① **证据档位只有「上游这么写」**（无权威出处，纪律 2）；② 上游自己在这一族代码里留有
`// CNROM mode. Unclear. Untested.` 一类的存疑标记；③ 会改这些卡的 CHR/PRG 银行规则，
需真 mapper 45 ROM + F11QA 基线评审才敢动 ⇒ **登记为待立项项**。

### 9.23 · 缺陷登记：录制的 `.fm2` movie **全 0** ⇒ 录制 / 回放功能不可用（r11 由 LT-09 的 V15 机器侧实测发现）

**状态**：🔴 **已实证的缺陷；本轮只登记不修**（修法明确，但「修 + 加门禁」需另立一轮）

**发现途径**：v2.0.5 线 B 的 **B2（LT-09 人工验收）**用无界面测试台实跑 **V15（movie 回放比对）** ⇒
录出来的 `.fm2` 是**长度正确但内容全 0** 的文件（6197 字节全 `0x00`、无 `FCEUXM` 头），回放 **180/180 帧**不一致。

**根因（父 agent 逐行核实，不是子 agent 的猜测）**：

- `src/movie_fm2.cpp:407-416` —— 先把 movie 数据 dump 进一个 **Rust 侧内存文件** `EmuFileMem`，
  再从它读回来写进目标文件：
  ```cpp
  EmuFileMem* mem = fceux11_rust_emufile_mem_create();
  int bytes = fceux11_rust_movie_data_dump(&input, mem, binary, seekToCurrFramePos, currFrameCounter, &currFramePos);
  size_t memSize = fceux11_rust_emufile_mem_size(mem);
  if (memSize > 0) {
      std::vector<uint8> tmp(memSize);                              // ← 零初始化
      fceux11_rust_emufile_mem_fread(mem, tmp.data(), memSize);     // ← 位置读：pos 已在末尾 ⇒ 读回 0 字节
      os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(tmp.data()), memSize));
  }
  ```
- `src/rust/crates/fceux11-formats/src/emufile.rs:84-92` 的 `EmuFileMem::read_into`
  **从 `self.pos` 读**（位置读）⇒ dump 之后 `pos == memSize` ⇒ `fread` 返回 **0** ⇒ `tmp` 保持**零初始化**
  ⇒ 写出去的就是**长度正确、全为 0** 的文件（与实测逐字吻合）。
- ⇒ **修法（一行）**：在 `fread` 前补 `fceux11_rust_emufile_mem_fseek(mem, 0, 0);`
  —— **该 API 已存在**（`emufile.rs:289`），且 **Rust 侧自己的用法就是「fwrite → fseek(0,0) → fread」**
  （`emufile.rs:583-591` 的单元测试即此范式）⇒ **这是移植时漏掉的一次 seek，不是上游缺陷**。
- 另一处可疑点（**本轮未实证**，登记待查）：`src/movie_record.cpp:316-328` 的 `RedumpWholeMovieFile()`
  先 `openRecordingMovie()`（**重开即截断**）再重新 dump ⇒ 重开路径可能把已录内容清空。

**影响面**：**所有用户录制的 movie（`.fm2`）都不可用**（文件全 0）⇒ 其重要性**高于**它作为「人工验收项」的价值
⇒ 建议**另立一条缺陷条目**并**优先于**批 2/3 的 mapper 项。

**验收（修后）**：录一段 movie ⇒ `.fm2` **非全 0** 且含 `FCEUXM` 头 ⇒ 回放与实时推进逐帧一致 ⇒ 进 `ctest -LE perf`。

### 9.24 · 判据口径更正：V14 的 CDL 一致性**字面等式不成立**（r11）

**状态**：📌 **只更正判据口径**（不动基线、不改 `expected` / `failure_means` / `pass_to_fail`）

LT-09 的 V14 判据写的是 `codecount + datacount + undefinedcount == cdloggerdataSize`。
B2 用 **6 个 fixture × 120 帧**实测：**6/6 全部不满足**，且**差值恰等于 `code_and_data`**
（既被计入 code 又被计入 data 的字节数，read 数 2 / 18 / 203 随 ROM 而变）。

⇒ 真正成立的是**并集式**：`code + data - both + undef == size`（**6/6 精确成立**），
计数器与缓冲区位模式逐项吻合。

⇒ **人工验收若照字面等式做，必然误报 FAIL** ⇒ 建议把 V14 的判据改成并集式，并把 `code_and_data` 读出来。
**本轮只登记口径更正，不改任何基线文件**（LT-09 的反向守卫：人工项是**确认**，不是**调判据**）。
