# FCEUX11 v2.0.3 — GBA 卡带兼容性优化计划

> **STATUS: 在途**（业主 2026-11-08 批准立项范围：**三段式** + **卡带不进仓库、判据换成自建最小夹具**）
> **进度快照（2026-11-12，r19）**：S1 ✅ 收口 · S2 ✅ 收口 · **S3 ✅ 收口（r17——S3-a 第二批 + S3-b/S3-d/S3-e 落地；S3-c 以取证收口；r19：FireRed 的 0x01 强制空白被双向定位为越过 init 竞争的触发点，机器判据「出画面」达成）**
> · S4 ✅ · S5 ✅ ⇒ **12 条夹具全部为常驻绿锁（复现器清零）**；声音家族 12 号 SWI 与 `0x2A` 维持 S2-b 兜底 + 已知限制（纪律 4 拦住，见 §九 r16）。
> **⚠️ r2（2026-11-09）：§一.2.2 的「可触达 panic 点」清单已按源码复核并收窄（6 处更正），§二.2 / §二.4 的夹具清单与出口判据已改写。**
> **⚠️ r3（2026-11-09）：GBATEK 逐位核实 ⇒ 撤回 r2 的「EEPROM 访问窗口过宽」一条，并据此把夹具改成 8 条（7 会红 + 2 绿锁）、出口改成 262 全绿 + 7 条复现器。**
> **⚠️ r4（2026-11-09）：S1-a 的 Step 1（G3 编码器自检）已过变异验收 —— 测出 3 条盲区并修掉，
> 8/8 的目标都能红；但 §2.4 的「只改一个字段 ⇒ 其它 7 条仍然绿」有 **2 行结构性达成不了**
> （`mov_imm` 是唯一的造地址手段、`swi(HALT)` 是每条自检的终止器）⇒ **需裁决**。**
> **✅ r5（2026-11-09）：S1-a 的 Step 0 余账 + Step 2/3/4/5/6 已完成 —— 9 条夹具（7 红 + 2 绿锁）
> 与 3 条守卫落地，**7 条复现器在夹具域过滤下逐条为红**，门禁 **262 passed / 26 ignored**。
> 同时更正三处计划口径：① §四 S3-b 的「flash 状态寄存器按 GBATEK 的状态位定义返回」**
> 无依据**（GBATEK 根本没有状态寄存器）；② SWI 向量字应为 `MOVS PC, LR`（vendor 的 LR 差 4）；
> ③ 夹具是 **9 条**不是 8 条。**
> **✅ r6（2026-11-10）：S1-b 探针改造完成 —— §2.3 六项全部落地，根 crate **264 / 26**（+2 常驻锁），
> 复现器仍 7 红，ctest **36/36**，7 张卡带基线**零 panic、零未实现 SWI**，Dragon Ball 注入实验
> 证明**游戏对输入有响应**（卡死未复现，交 S3-b/S3-c）。附录 A 补 **A.6（GPIO/RTC）**，
> §一.5 行动项至此收口。三个实现层新事实见 §九 r6：**cargo test --release 仍 unwind**、
> **格式化 panic 的 payload 不可 downcast（读数必须走 hook 的 payload_as_str）**、
> **match scrutinee 自死锁**（初版毒锁恢复曾把全套件挂起）。**
> **✅ r7（2026-11-10）：S2-a 落地 —— vendor 的两个可达 panic 点（OAM shape=3 / gfx mode=3）
> 改为映射 `ObjMode::Disabled`（真机语义：精灵不显示），`oam_bad_shape` / `oam_gfx_mode3`
> 转常驻绿锁（根 crate **266 / 24**），复现器 7 → 5，vendor **242** 不变、ctest **36/36**、
> 7 卡基线零回归；ATTRIBUTION §3.2.12（第 28 处、第 6 个源文件）；变异两腿双向可红；
> Lock B 改造为**注入式机制锁**（业主裁决），S1-b 的 OAM 触发器随修复退役。**
> **📋 r8（2026-11-10）：业主裁决扩围 —— GitHub issue #12 的两个 NES 侧缺陷核实属实（只读 RE），
> 作为 **S4 · FC 侧修复批次** 并入本计划（§四之二）。**本轮只写文档不动代码**，下一轮编码作业
> 按 S4 节直接开工。formats crate 门禁已实测登记（**150 项全绿**，AGENTS.md 门禁节同步）。**
> **动 GBA 兼容性代码前先读 §一.2.2 开头的「更正」小节**——原文清单里 13 条有 11 条不可达，
> **真正可达的只有 OAM 的两条**（shape=3 与 gfx mode=3）。
> **🔴 r10（2026-11-10）：S4 的 B4 人工步骤已做（9 张 NES 卡带实测 v2.0.2_hotfix1）+ 上游对照
> 实验 ⇒ 定位到两个真实缺陷（新立 §四之三 S5）：① **hash 作用域**（本仓按 raw PRG 算，
> 上游与 `ines-correct.h` 修正表都按「舍入 + 0xFF 填充」算 ⇒ 非 2 幂 PRG 的 ROM 必然查不中
> 修正表；吞食天地2 的 padded CRC `0x28192599` 命中 `{…, 198, -1}` —— **它实际是 mapper 198**）；
> ② **修正写回缺失**（`ines_apply_corrections` 把 mapper 写进局部 `parseResult`，
> `tofix & 1` 分支是空注释 ⇒ **整张 251 条修正表的 mapper 修正在本仓整体失效**）。
> ⚠️ **S4 的 A3 决策「hash 用 raw」需要复议**（漏了 hash 同时是修正表的查找键）。
> ⚠️ **本仓 fork 的上游基线是 FCEUX 2.6.6，不是 1.15**（1.15 是本项目自己的版本线）。
> **✅ r12/r13（2026-11-11）：S2 止血收口 —— S2-b SWI 兜底落地（SWI 向量 `b .` →
> `MOVS PC, LR`，未实现 SWI「永久自旋」→「返回 + 生产侧计数记录」；复现器 5 → 3，
> 根 crate 266/24 → 270/22）；S2-c C++ 侧不崩落地（每帧错误码静默吞掉 → 去重日志）。
> ⚠️ §3.2「寄存器默认值按 mGBA」口径修正：mGBA 源码不在本机（`Research_only/` 已不存在），
> 改按 GBATEK SWI ABI 自建依据（§九 r12）。门禁全绿：ctest 36/36、vendor 242、
> debug 93 / media 52 / formats 153；7 卡基线逐字一致。S3-a 前置 RE 证据表已存
> `docs/plans/FCEUX11-v2.0.3_S3a前置RE-GBATEK与SWI证据表.md`。**
> **立项目标**：把「商用手写 GBA 卡带跑不起来」这一类问题，从**靠人看**变成**可复现、可证伪、可回归**。
> **基线**：`2960998`（v2.0.2 收口批次 r26 之后）· NES 侧 `ctest -LE perf` **36 项全绿** · F11QA 矩阵 **122 total / 112 passed / 10 failed / grade B / `pass_to_fail` 空** · GBA 锁测试 **249** + vendor **242** + debug **93** + media **52**
> **产出定位**：只立项、排期、定验收；实施按工作项逐个开独立任务，**先 PLAN 后动手**的纪律不变。

---

## 〇、范围与不变式

### 范围内

1. **GBA 卡带兼容性**：让真实商用手写卡带能启动、能出画面、能存读档、不闪退。
2. **可复现门禁**：自建**最小可复现夹具**（合成 ROM），让每一条已定位缺陷都有**一条会红的锁测试**。
3. **探针工程**：把仓库现有的 `cartprobe` / `probe_why_a_game_stalls` / `probe_trace_a_stuck_game`
   升级成能**注入输入 + 长窗口 + 判定卡死点**的取证工具。

### 范围外（排除项）

1. **不碰 NES 侧任何行为**。本轮所有代码改动只落在 GBA 侧
   （`src/rust/src/gba/`、`src/rust/crates/gba-core/`、`src/gba_load.cpp`）。
   ⚠️ **r8 业主裁决（2026-11-10）**：GitHub issue #12 的两个 NES 侧缺陷（iNES loader 硬拒
   非 2 幂 PRG / mapper 191、199 未注册）**核实属实并纳入本计划**，见 §四之二 S4——
   本条口径**不再绝对**：GBA 侧文件域仍按本条执行；FC 侧仅限 S4 明确列出的文件
   （`src/rust/crates/fceux11-formats/`、`src/ines_load.cpp`、`tests/f11qa/boards/`、
   `tests/fixtures/`、`src/boards/mmc3.cpp`、`src/boards/mmc3_variants_carts.h`），
   且 NES 零回归门禁（不变式 1）对 S4 改动**加倍适用**。
2. **不把 7 张商业卡带放进仓库**（业主裁决 2026-11-08）。它们只作**本地取证**用，
   仓库内的判据一律换成**自建合成 ROM**。
3. **不做 GBA 精度对齐**（本轮是**能不能跑**，不是**跑得准不准**）。精度另立项。
4. **外部参考实现只读不抄**：mGBA 是 MPL-2.0、FCEUX 上游与本仓库同源 —— 两者都**只读**。

### 不变式（继承 AGENTS.md，红灯即停）

1. **NES 零回归**：任何 GBA 改动后 `ctest -LE perf` 36 项全绿 + F11QA 矩阵 `pass_to_fail = 0` +
   grade ≥ B。金标三件套零漂移。
2. **GBA 长期 Alpha，不阻塞本体发版**（AGENTS.md 不变式 2）。本计划**不设**本体发版门槛。
3. **extern "C" 出口面必须能被安全调用**（AGENTS.md 纪律 6）：导出函数里**不允许**出现
   会 unwind 的 panic —— 这正是本轮「闪退」的一半原因，见 §一.2。
4. **变异验证**（AGENTS.md 纪律 3）：每条新锁测试都要能把「修复前的值」改回去看到它转红；
   **全绿不算证据**。
5. **不凭记忆写位级格式**（AGENTS.md 纪律 4）：位定义一律取 GBATEK 权威快照（§一.5）。
6. **vendor 树补丁集要登记**：改 `src/rust/crates/gba-core/` 必须同步
   `src/rust/crates/gba-core/ATTRIBUTION.md`（逐处说明 + 重新 vendor 流程）。
7. **instrument-first**（`docs/tech/precision.md` §4）：改行为前先有能看见它的读数。

---

## 一、问题陈述与已确认的事实（取证基线）

> 本节是本计划的**事实底座**。它取代被删除的两份旧计划中与 GBA 兼容性相关的部分。
>
> **路径简写约定**（免得看的人找不到文件）：
> `gba-core/src/...` = `src/rust/crates/gba-core/src/...`；
> `hw/` = `src/rust/crates/gba-core/src/cpu/hardware/`；
> `lcd/` = `hw/lcd/`；`lcd.rs` = `hw/lcd.rs`；`lcd/layers/` = `hw/lcd/layers/`；
> `gba/` = `src/rust/src/gba/`；`swi/` = `src/rust/src/gba/swi/`。

### 1.1 实测样本

来源：业主提供的实测报告 `fceux11_v2.0.3卡带兼容性实测报告.odt`（真实卡带 + 真实运行截图，
报告真实性由业主背书）+ 7 个 `.gba` 卡带本体 + 5 个自动生成的 `.srm` 存档。

| # | 卡带 | 容量 | 存档型号（ROM 内 ID 串） | 结果 | 症状（报告原文） |
|---|---|---|---|---|---|
| 1 | Kirby - Nightmare in Dream Land | 8 MB | `SRAM_V112` | ✅ 完全兼容 | — |
| 2 | Need for Speed - Most Wanted | 8 MB | `EEPROM_V124` | ✅ 完全兼容 | — |
| 3 | Super Mario Advance 2 - Super Mario World | 4 MB | `EEPROM_V122` | ✅ 完全兼容 | — |
| 4 | Mario Kart - Super Circuit | 4 MB | `FLASH_V124` | ❌ 不兼容 | 「启动动画和音乐正常，**马上就卡死**，fceux11 无法操作，然后**自动闪退**」 |
| 5 | Dragon Ball - Advanced Adventure | 16 MB | `EEPROM_V124` | ❌ 不兼容 | 「开场动画和音频正常，**按点击按钮后就卡死**，然后**闪退**」 |
| 6 | Legend of Zelda - The Minish Cap | 16 MB | `EEPROM_V124` | ❌ 不兼容 | 「加载/开场/存档/前情提要正常，**进入主线游戏时黑屏**，按键无反应，**不卡死不闪退**，可正常关闭 ROM」 |
| 7 | Pokemon - FireRed | 16 MB | `FLASH1M_V103` | ❌ 不兼容 | 「**无法加载任何画面**，音频刺耳杂音」 |

**存档文件实测**：5 个 `.srm` **全部是 32768 字节**，与卡带真实芯片容量无关
（详见 §一.2.4）；Mario Kart 与 FireRed **没有生成** `.srm`（二者都没跑到存档那一步）。

**ROM 头实测**：7 张卡的 GBA 头**全部合法**（Nintendo logo 在 0x04–0x9F 逐字节正确、
固定值 `0x96` 在 0xB2、标题/游戏码/厂商码在 0xA0/0xAC/0xB0 全部可读）。
⇒ **ROM 头不是问题**。

### 1.2 已定位的根因

#### 1.2.1「卡死」＝ 未实现的 BIOS SWI 落到 stub BIOS 的 SWI 向量，而那个向量是 `b .`

- 模块 `dispatch` 只认领 **16 个** SWI 号码（`src/rust/src/gba/swi/mod.rs:856-900`
  的锁测试即事实源）：`0x01–0x05, 0x09, 0x0A, 0x0D–0x15`。
- vendor core 另服务 **6 个**（`src/rust/crates/gba-core/src/cpu/arm7tdmi.rs:1134`（`handle_swi_hle` 起点））：
  `0x00, 0x06, 0x07, 0x08, 0x0B, 0x0C`。
- **完全没实现的号码**（模块的 `Swi` 枚举里有名字，但 `dispatch` 没有分支 ⇒
  **有名字 ≠ 有实现**）：`0x16/0x17/0x18`（Diff8bit/Diff16bit 反滤波）、`0x19` SoundBias、
  **`0x1A–0x1E`（SoundDriver 五兄弟）**、`0x1F` MidiKey2Freq、
  **`0x20–0x24`（MusicPlayer 五兄弟）**、`0x25` MultiBoot、`0x26` HardReset、
  `0x27` CustomHalt、`0x28/0x29/0x2A`。
- **未处理 ⇒ 永久自旋**：core 拒绝后走真异常路径 PC = 0x08 ⇒
  `src/rust/src/gba/bios.rs:131-134` 的 SWI 向量是 `BRANCH_SELF`
  （`bios.rs:97` `const BRANCH_SELF: u32 = 0xEAFF_FFFE;` = `b .`）
  ⇒ **CPU 在 BIOS 0x08 永久自旋**。
- 仓库**早就知道这条**：`src/rust/src/gba/frame.rs:375-380` 的 `gba_step_frame` 文档写着
  「Bounded so a machine wedged before it ever reaches VBlank -- **a game looping on an
  unclaimed SWI, say** -- returns instead of hanging the emulator」。
- ⚠️ **这条机制目前被两条锁测试锁成了「预期行为」**：
  `src/rust/src/gba/bios.rs:396-401`（`the_stub_does_not_implement_swi`）与
  `bios.rs:218-223`（`unhandled_vectors_park_instead_of_wandering`）
  都**断言 SWI 向量必须是 `BRANCH_SELF`**。
  ⇒ 修这一条**必然要改这两条锁测试**，且断言必须从「park」改成「返回并记录」，
  否则等于把缺陷重新锁回去（AGENTS.md 纪律 3）。

#### 1.2.2「闪退」＝ vendor 核 panic，而 release 是 `panic = "abort"`

> ⚠️ **更正（2026-11-09，r2）——本小节的「可触达」清单已按源码复核并收窄。**
> **行号全部仍准确**，但清单里 **13 条中有 11 条不可达**：
>
> 1. `bus.rs` 的 11 处 `panic!` 是各 `*_raw` 函数的**兜底分支**，而 `read_raw`(`bus.rs:1077`) /
>    `write_raw`(`bus.rs:1152`) 的 `match` 对各自 I/O 区间**已全覆盖** ⇒ 落不进去。
>    「写一个未建模的 IO 寄存器」今天的行为是 `bus.rs:1203-1211` 的
>    **静默忽略 + debug 日志**，零 panic。
> 2. `internal_memory.rs:660` 同理不可达：`read_at` / `write_at` 的**全部**调用方只有
>    `bus.rs:1088` / `bus.rs:1094` / `bus.rs:1157` 三处，而 `read_at` 的 match 缺口
>    （`0x0400_0000–0x07FF_FFFF` 的 I/O、调色板、VRAM、OAM 区）**恰好由 bus 自己接管**。
> 3. `operations.rs:896` 的 PLD `todo!` 是**死代码**：`arm7tdmi.rs:458-463` 先拦 `Pld` 抛
>    UndefinedInstruction，`psr.rs:74` 的 `NV => false` 再挡一道 ⇒ PLD 的真实后果是
>    「静默 no-op」或「跳 `0x04` 向量自旋」，**都不是 panic**。
> 4. OAM 四处里 **gfx-mode=3 的 panic 现场是 `object_attributes.rs:144` 的 `unreachable!()`**，
>    不是 `262`——`262` 的 `.unwrap()` 只在值 = 4 时才响，而该字段是 2 bit（0..3）⇒ 死分支；
>    `389` / `475` 只向上转发。
>
> ⇒ **真正可达的 panic 面只有两条**：`object_attributes.rs:265`（OAM attr0 **shape = 3**）与
> `object_attributes.rs:144`（OAM attr0 **gfx mode = 3**）。
> ⇒ 另：真机上「禁用精灵」是 **obj mode = 2**（vendor 已正确处理，`layer_obj.rs:234-236` 跳过）；
> **shape = 3 / gfx mode = 3 是「禁止值」（prohibited）**，真机当作「精灵不可见」，vendor 是崩溃。
> ⇒ **S2-a 的修复语义应当是「映射为精灵禁用」**（与 obj mode = 2 的既有处理一致），不是「忽略」。
>
> **逐条证据与复核命令**：
> `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §一（含 §九.3 的复核命令）。
> ⇒ **本节的结论方向不变**（闪退确实来自 vendor 核 panic + `panic = "abort"`），
> **变的是证据清单** ⇒ S2-a 的表要按上面对照重读。

- `src/rust/Cargo.toml:9-11`：`[profile.release] panic = "abort"`。
  在 `extern "C"` 帧里 panic **无法 unwind** ⇒ 直接带走整个进程 ⇒ **闪退**。
- panic 点清单（**已按 r2 复核收窄**；行号为原表，判定列是 r2 的复核结论）：
  | 位置 | 触发条件 | r2 判定 |
  |---|---|---|
  | `gba-core/src/bus.rs:596` | `panic!("Not implemented write memory address: {address:x}")` | ❌ 不可达（兜底分支） |
  | `gba-core/src/bus.rs:291/302/343/422/520/538/558/576` | 各 I/O 区域读写越界 panic | ❌ 不可达（兜底分支） |
  | `gba-core/src/bus.rs:957/1069` | LCD 区域读写越界 panic | ❌ 不可达（兜底分支） |
  | `gba-core/src/cpu/hardware/internal_memory.rs:660` | `unimplemented!("Unimplemented memory region. {address:x}")` | ❌ 不可达（bus 只转交三个区间） |
  | `gba-core/src/cpu/arm/operations.rs:896` | `todo!("implement single data transfer operation")`（PLD 指令） | ❌ 死代码（两道防线） |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:265` | OAM attr0 **shape = 3** ⇒ `.unwrap()` on `Err("Prohibited ObjShape")` | ✅ **可达** |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:144` | OAM attr0 **gfx mode = 3** ⇒ `unreachable!()` in `GfxMode::try_from` | ✅ **可达** |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:262/389/475` | 向上转发 | ❌ `262` 是死分支；`389`/`475` 只转发 |
- 关键性质：**这两个 panic 发生在**渲染中途**（OBJ 属性每扫描线解析一次，
  `lcd/layers/layer_obj.rs:691-693`）⇒ 帧步进中途 abort ⇒ C++ 侧看到 `gba_step_frame()`
  永不正常返回 ⇒ **进程崩**。

#### 1.2.3 FireRed 的开机忙等（已定位到具体指令）

用仓库自带探针实测（`probe_why_a_game_stalls` + `probe_trace_a_stuck_game`）：

- PC 稳定停在 ROM `0x080008C2–0x080008CA`（**Thumb**），指令序列是
  `LDRH R1,[R2,#28]` / `ADD R0,R3,#0` / `AND R0,R1` / `CMP R0,#0` / `BEQ loop`。
- 实测寄存器：**`r2 = 0x030030F0`、`r3 = 1`** ⇒ 它在等
  **IWRAM 的 `*(u16*)0x030030FC` 的 bit0** 变成 1，而该位**永远是 0**。
- 30M 指令内 **distinct frame hashes = 1（画面完全冻结）**、每帧步数恒为 163724
  ⇒ 与「无法加载任何画面、音频刺耳杂音」完全吻合（冻结 + CPU 空转出噪声）。
- **尚未确定**：谁应该写这个标志位。⚠️ **高度可疑但未证实**的一条线索：
  `internal_memory.rs:222` 的 `gpio_present` 初始为 `false`，而
  `internal_memory.rs:688-689` 只在**游戏写入** 0xC4–0xC9 时才置 `true`；
  若游戏的 RTC 探测**先读后写**，读到的就是普通 ROM 字节而不是 GPIO 值
  ⇒ RTC 探测失败 ⇒ 标志位永不置位。
  （S3-c 要用夹具证实或证伪这条，**不得当结论用**。）

> **✅ r16 取证收口（推翻本节两条结论，S3-c 以此结案）**：
> ① **忙等轮询的地址是 `0x0300310C`（gMain+0x1C）不是 `0x030030FC`**——指令解码
> `LDRH R1,[R2,#28]` 的 R2 = gMain `0x030030F0` ⇒ +0x1C；`0x030030FC` 是回调指针
> （帧 14 一次性写入 `0x080EC5B9`）。本节旧读数出自轮询式探针——**方法论发现**：
> `bus.read_half_word` 会加 `cycles_count`（`bus.rs:673/1302`），轮询探针给机器
> 加速时钟并改变行为 ⇒ **探针观测一律 `read_raw`**（新纪律候选，已入 AGENTS.md）。
> ② **「GPIO 先读后写」线索证伪**：300 帧零写、30M 指令 sniff 零 GPIO 指针加载
> （GPIO 字面量在 0x0834F2xx，本窗口未执行）。③ **真机制链**：IE/IME/IRQPTR 全就位、
> PPU 正常走（DISPSTAT bit0 翻转 594 次），但 **DISPSTAT bit3（V-IRQ 使能）300 帧
> 从未置位** ⇒ IF 恒 0 ⇒ 写入方（ROM `0x080007B4` `STRH r1,[r5,#0x1C]`，VBlank IRQ
> 回调，同函数设 BIOS IntrWait 标志）永不执行 ⇒ 忙等死锁；游戏意图已写入 IRQ 管理
> 影子 `0x03000004=0x0008`（step 1403），但「影子→硬件」apply 从未运行（DISPCNT 影子
> 0x0140 同样未上硬件）；**加速时钟 +25% 对照跑 ⇒ apply 执行、V-IRQ 每帧发、游戏活**
> ⇒ **init 时序竞争**（入口：`0x030000C0` 锁 + `0x03000060` 队列 + flusher `0x08000A18`
> 调用方）——属 GBA 精度范畴，超出本轮「能不能跑」范围 ⇒ **转长尾**（AGENTS.md）。
> 旁证：DB/MK/MC 三张卡都自己开好 DISPSTAT bit3 且 vendor VBlank IRQ 正常——
> 「vendor IRQ 模型坏」不成立，FireRed 是唯一不开闸的卡且病在自身 init 时序。
>
> **✅ r19 追记（双向定位，FireRed 在标准探针窗口已活）**：r17 的 `0x01`
> RegisterRamReset 强制空白（GBATEK `:193540-193541`）被实证为让 FireRed 越过
> 该竞争的**确定性触发点**——7 卡基线里 6 张逐字不变、唯独 FireRed 从 FROZEN
> 变 48 变化帧 + 音频 DMA 激活；窗口内它只多踩了 `0x01`（第二批四号零调用）；
> 摘掉 DISPCNT 写入立即回 FROZEN（逐字同 pre-r17），还原即复现 48 哈希。
> 机制：BIOS 侧的强制空白写入改变 init 的状态/时序条件 ⇒ 影子 apply 链得以运行。
> 竞态本身仍在，长尾条目保留。

#### 1.2.4 存档芯片模型不完整

GBATEK 权威（摘录见 §附录 A）：

| ID 串 | 真实芯片 | 本仓库映射 | 判定 |
|---|---|---|---|
| `EEPROM_Vnnn` | EEPROM **512 B 或 8 KB** | `BackupType::Eeprom`，缓冲 32 KB（`internal_memory.rs:97`） | ❌ **无真模型** |
| `SRAM_Vnnn` | SRAM **32 KB** | `BackupType::Sram`，`0x8000` | ✅ |
| `FLASH_Vnnn` | FLASH **64 KB** | `BackupType::Flash64`，`0x10000` | ✅ |
| `FLASH512_Vnnn` | FLASH **64 KB** | `BackupType::Flash64`，`0x10000` | ✅ |
| `FLASH1M_Vnnn` | FLASH **128 KB** | `BackupType::Flash128`，`0x20000` | ✅ |

- **容量映射是对的**（⚠️ 本轮我曾按「1M = 1MB」误判，被 GBATEK 否证 —— 见 §一.5）。
- ⚠️ **r2 更正**：**位级 EEPROM 模型是存在的** —— `internal_memory.rs:523-611` 有
  `is_eeprom_access` / `eeprom_write` / `eeprom_read` / `eeprom_interpret_command`。
  **真正缺的是下面两条**（原表把「无真模型」写得太宽）：
  - **E2 · 芯片型号与容量不建模**：`BackupType::detect`（`internal_memory.rs:75-89`）把
    `EEPROM_V`（**512 B** 芯片）与 `EEPROM_V124`（**8 KB** 芯片）**混成同一个
    `BackupType::Eeprom`** ⇒ 型号信息丢失；而 `buffer_size()`（`:92-98`）对 `Eeprom`
    返回 `0x8000`（32 KB），真机是 **512 B 或 8 KB** ⇒ `.srm` 尺寸恒为 32768
    （实测 5 个 `.srm` 全是 32768 字节，与此对上）。
  - **E3 · `0x0E` 区域对 Eeprom 一律回 `0xFF`**：`internal_memory.rs:636`
    （读）与 `:724`（写丢弃）—— EEPROM 内容实际存放在别处（`0x0D` 窗口的位流接口）。
  - **Dragon Ball / Minish Cap / NFS 三张卡都是 `EEPROM_V124`（8 KB）** ⇒
    它们读到的存档是全 `0xFF`，写进去的偏移也是错的。
  - ⭐ **r3 追加（2026-11-09，GBATEK 逐位核实）—— 原 r2 里的「E1 · 访问窗口过宽 16 MB」
    **撤回****：GBATEK 快照 `.zcode/gbatek_plain.txt:8692-8694` 明文
    「**On carts with 16MB or smaller ROM, eeprom can be alternately accessed anywhere at
    D000000h-DFFFFFFh**」⇒ `is_eeprom_access()` 对 ≤16 MB 的卡返回
    `0x0D000000–0x0DFFFFFF` **与 GBATEK 一致**，**不是缺陷**。
    同时核实：**EEPROM 的命令协议 vendor 是对的** —— GBATEK `:8710-8786` 与
    `internal_memory.rs:566-614` 逐位一致（请求位 `"11"`/`"10"`、6 或 14 位地址、
    64 位数据、读回 68 位 = 4 忽略 + 64 数据、MSB first、Ready 位、
    以 64 位为单位；写分支宽度判据 `buffer.len() >= 81` 也对）。
    ⇒ **存档芯片侧真正缺的只有 E2 与 E3 两条**，且 E2 的根因是**型号丢失**而不是窗口。
    ⇒ **教训**：本条与 r1 的「DMA 起始时序」「FLASH1M 容量」是**同一类错误**
    （凭记忆写位级/协议），本项目**已犯第四次**。**位级与协议一律查快照原文。**
- **Flash 状态寄存器根本没有**：读 `0x0E000006/7` 落到 `internal_memory.rs:639-653`
  的「普通读」分支 ⇒ 返回**存档原始字节**，而不是芯片状态（ready / write-enable / erase-complete）。
  ⇒ 任何轮询 Flash 状态的商用卡都会死等。
  > **✅ r16 重定性（四源一致——「没有状态寄存器」的正确读法）**：GBATEK FLASH 全章
  > `:8858-9146` 的命令集里**没有 `0x70` 读状态命令**（全集只有 90/F0/80/10/30/A0/B0），
  > `E000006` 全文零命中，轮询一律是**回读比较**（`:8981`/:8915/:8936/:8959）；最强证据 =
  > **真卡带里在跑的任天堂 SDK 驱动反编译**（pret `agb_flash_1m.c:51-86`：PollFlashStatus
  > = 普通阵列字节读 + DQ5 超时位）；mGBA `savedata.h:38-49` 无 0x70、VBA-M 同。
  > ⇒ **`0x0E000006/7` 的真硬件语义 = 阵列字节 offset 6/7（vendor 现状本来就对）**，
  > 上句「返回存档原始字节 ⇒ 死等」的推断不成立（游戏等的正是回读值）。原复现器的
  > 「ready=bit0」约定作废，`flash_status_reg` 已按**回读语义**反转为绿锁（r17）；
  > 「忙窗口」增强不做（`:9119` verify-retry + SDK 即时成功 + MKSC 实测可玩）。
- **`.srm` 尺寸 = `buffer_size()`**（`src/rust/src/gba/frame.rs:787`
  `gba_battery_size` → `battery_data().len()`）⇒ EEPROM 卡存出来**永远 32 KB**
  （实测 5 个 `.srm` 全是 32768 字节，正好对上）。

#### 1.2.5 ROM 窗口没有镜像（次要，但确实存在）

`internal_memory.rs:377-404` 的 `read_rom()`：对 `address >= rom.len()` 返回
**开放总线垃圾**（`(address >> 1) & 0xFFFF` 的低/高字节），
**而不是镜像卡带**。真机上 ≤32 MB 的卡在 `0x08000000–0x09FFFFFF` 是镜像的。
⇒ 任何访问超过自己容量的地址（8 MB 卡访问 `0x08800000+`）拿到的是垃圾。

> **✅ r16/r17 重写（前提证伪——本条标题的「缺陷」不成立）**：GBATEK `:14046`
> 原文 **"Normal ROM Cartridges ... are NOT mirrored"**；`:14029-14030` 的开放总线
> 公式（`Address/2 AND FFFFh`）是 GBATEK 唯一成文的行为；mGBA 0.10.5 越界读 =
> 开放总线（仅 1 MiB 卡 4 倍镜像与烧录卡补零两个例外，本仓不跟）；melonDS 返回 0
> 且注释 "CHECKME: does ROM mirror?"。三来源无一支持「按容量镜像」⇒ 本条是 r1
> 凭记忆写入的结论（与 §1.5 登记的两次同类），**vendor 现状正确**。
> ⇒ `rom_window_mirror` 已改写为**开放总线绿锁**（字节/半字/字四腿 + 双哨兵，
> 期望值由夹具内独立拼装的 GBATEK 公式给出，r17）；顺带修掉
> `try_read_word` 越界臂的快路径契约（r17 W5b，总线级值前后逐位相同）。

#### 1.2.6 显示侧基本是全的（**不是**黑屏主因）

| 能力 | 状态 | 坐标 |
|---|---|---|
| 视频模式 0–5 | ✅ 全部实现 | `lcd/layers/layer_0.rs`、`lcd/layers/layer_1.rs`、`lcd/layers/layer_2.rs:149/173/212`、`lcd/layers/layer_3.rs:105` |
| OBJ / 仿射 / 旋转缩放 | ✅ | `lcd/layers/layer_obj.rs:140-154/166-201/420-462` |
| WIN0 / WIN1 / WINOUT / WINOBJ | ✅ | `lcd/registers.rs:523-573`、`lcd.rs:543-556` |
| 混合（alpha/brighten/darken）、BLDY 衰减、半透明 OBJ | ✅ | `lcd.rs:400-528`、`layer_obj.rs:487-490` |
| **强制空白渲染成白色**（真机也是白） | ✅ | `lcd.rs:303-307` |

⇒ **Minish Cap 的「黑屏」不是强制空白**（强制空白是白屏），也不是显示模式缺失。
剩余显示侧缺陷只有两条，都在 S3-e：mode 4 的 palette index 0 被误当透明
（`lcd/layers/layer_2.rs:193-196`，真机 mode 4 的 index 0 **不是**透明）、
**mode 5 的页面翻转（DISPCNT bit 4）没实现**（`lcd/layers/layer_2.rs:212-237`，
`render_mode4` 有 `frame_offset` 而 `render_mode5` 完全没有）。

> **✅ r16/r17 收口（两条「缺陷」一条证伪、一条确认已修）**：mode 4 index 0 被
> GBATEK `:3198` 证伪——原文 "Color 0 (backdrop) is transparent, and OBJs may be
> displayed behind the bitmap"，mGBA 同构 ⇒ **vendor `layer_2.rs:193-196` 正确，
> 改了才是回归**（已加 OBJ-behind-BG2 优先级技巧的防回归锁）；BG2 门控（旧表第三条）
> 同为误报（`:1944-1952`：BG2 在 mode 0–5 全存在，vendor 分发逐格一致）——
> 本小节这两处是 r1 对注释「BG2 (all modes)」的误读。**mode 5 页面翻转确认真缺陷**
> （`:1918` bit 4 适用 modes 4,5、`:3204-3206` Frame 1 = 0600A000）⇒ r17 已修
> （`frame_offset = bit4 ? 0xA000 : 0`，与 mode 4 同式）+ 2 条 vendor 单元锁 + 端到端锁。

### 1.3 被否证的两个假设（**不要按这两个方向立项**）

1. **「DMA 起始时序 3 没实现」——不是原因。**
   - GBATEK 权威：起始时序在 **bits 12–13**（`gba-core/src/cpu/hardware/dma.rs:22-26`
     的 `timing()` **解码是对的**）；地址控制才是 bits 5–6 / 7–8。
   - **DMA1/DMA2 的时序 3（Sound FIFO）已经实现了**，走**定时器溢出驱动**的
     `run_fifo_dma`（`gba-core/src/bus.rs:700-745`），不是 `run_event_dma(3)`。
   - ⚠️ **仓库自带 `cartprobe` 的判词本身是错的**：
     `src/rust/src/gba/cartprobe.rs:338-345` 的 verdict 只看 `seen_timing[3]`
     （**通道 3**）判断「SPECIAL armed」，却在逐通道清单里把 DMA1/DMA2 的时序 3
     也印成「never fired by this core」。**用它的判词下结论会掉进这个坑。**
     真正没实现的只有 **DMA3 的时序 3（Video Capture）**。
2. **「Flash 芯片容量映射错」——不是原因**（GBATEK 已证，见 §1.2.4 表）。

### 1.4 仍未定位 / 尚未复现的部分（诚实标注）

| 现象 | 状态 |
|---|---|
| Minish Cap 进主线黑屏 | **未复现**。探针窗口 240 帧（4 秒）内画面一直在变（50 个不同帧哈希），故障点没跑到 |
| Dragon Ball 按键后卡死 | **未复现**。探针**不送按键**，窗口内 32 个不同帧哈希 |
| Mario Kart「马上就卡死」 | **未复现**。探针窗口内 128 个不同帧哈希，仍在跑 |
| FireRed 那个标志位的写入方 | **未定** |
| RTC 探测「先读后写」这条线索 | **高度可疑，未证实** |
| 4 张失败卡各自实际调用了哪些 SWI | **未知**（无 SWI 调用日志） |

### 1.5 取证纪律（本轮的教训，必须固化）

**位级格式一律取 GBATEK 权威快照，不凭记忆。** 本轮我**两次**凭记忆写位级格式都写错：

1. 把 DMA 起始时序记成 bits 5–6（实际 **12–13**）；
2. 把 `FLASH1M` 记成 1 MB（实际 **128 KB**）。

两次都是被 GBATEK 否掉的 —— AGENTS.md 纪律 4 说「已犯过两次」，**本轮又犯两次**。

⇒ **行动项**：GBATEK 快照固化成本地取证基线（`.zcode/gbatek_plain.txt`，
经代理 `127.0.0.1:7897` 抓取 `https://mgba-emu.github.io/gbatek/`），
**关键条目的摘录写进本计划 §附录 A**，并给仓库加一条**抓取脚本**，
免得每次重新抓、更免得每次靠记忆。

> ⚠️ **r2 追加**：附录 A 当前**只有三节**（DMA 控制寄存器 / 存档 ID 串 / 卡带头布局），
> **缺 EEPROM 命令协议与 GPIO/RTC 两章** ⇒ 这是 S1-a 的**前置条件**（见 §二.4 最后一条）。
> **取不到就停下来记录，不许凭记忆写**（纪律 4，本项目已因此翻车两次）。
>
> ⚠️ **r3 追加**：**快照本来就在本地** —— `.zcode/gbatek_plain.txt`（4.0 MB）+ `gbatek_probe.html`（5.3 MB），
> 2026-11-08 抓取 ⇒ **不必联网**。已就地核实：EEPROM 命令协议（`:8710-8786`）与
> vendor **逐位一致**；EEPROM 寻址（`:8677-8694`）**推翻了 r2 的「E1 窗口过宽」**；
> GPIO/RTC 章节（`:9287-9353`）**没有**定义「未写入前」的行为 ⇒ 夹具 10 降级为绿锁。
> ⇒ **附录 A 仍需补 A.4（EEPROM 协议）与 A.5（EEPROM 寻址/容量）两节摘录**，
> 并加一条**可重跑的抓取脚本**；快照本身**不入库**。

---

## 二、S1 阶段 · 可复现门禁（**必须先做**）

### 2.1 为什么必须先做

**仓库现有的探针复现不出 4 个失败中的 3 个**：

| 探针 | 局限 | 后果 |
|---|---|---|
| `cartprobe`（`src/rust/src/gba/cartprobe.rs:348-369`） | 不注入按键、只跑 240 帧、判词逻辑有错（§1.3） | Dragon Ball / Minish Cap / Mario Kart 的故障点跑不到 |
| `probe_why_a_game_stalls`（`frame.rs:2680+`） | 每 1M 指令打一次快照，不归因到具体子系统 | 只知道「PC 在哪」，不知道为什么 |
| `probe_trace_a_stuck_game`（`frame.rs:~2800+`） | 只打印寄存器快照，不看**谁该写那个变量** | FireRed 的标志位写入方查不出 |
| `probe_what_one_frame_changes`（`frame.rs:2929+`） | **要求机器 halt 才开窗** | FireRed 是**忙等**，永不安抚 ⇒ 没有窗口 |

⇒ **不先解决这个，后面每个修复都无法证伪** —— 这正是 AGENTS.md 纪律 3
（变异验证）与纪律 10（门禁没跑 ≠ 通过）的同一个坑。

### 2.2 S1-a 最小可复现夹具（自建合成 ROM，**Rust 内建，不入库二进制**）

业主裁决：卡带不进仓库，判据换成**自建最小夹具**。

> ⚠️ **r2 改写（业主 2026-11-09 拍板 D1）——载体形态与清单都改了。**
>
> **载体**：夹具改为 **Rust 内建**（`src/rust/src/gba/fixtures.rs` 里的函数，现场拼出 ROM 镜像），
> **不再**放 `tests/fixtures/gba/`、**不再**由 `generate_test_roms.py` 生成、**不产生任何二进制入库**。
> `.gitignore` 相应**只加一条 `*.gba`（全局忽略，不加例外）** —— 机械保证商业卡带永远进不了仓库。
> 理由三条：① 纪律 5「测试向量按字段拼装」在二进制 blob 上无法满足；② 门禁链路上**没有 cargo 环节**
> （CI 侧），「测试依赖仓库里一个二进制文件」是**无监控的依赖**；③ ARM 编码器只能有一份实现。
>
> **清单**：删 `gba_io_unmapped.gba`（该缺陷**不可达**，夹具今天就是绿的 ⇒ 无牙门禁）；
> EEPROM 拆成三条独立缺陷 ⇒ **8 条变 9 条**（详见 r2 与
> `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §五）。

| # | 夹具 | 红/绿 | 触发什么 | 对应缺陷 | 期望断言 | 归属 |
|---|---|---|---|---|---|---|
| 1 | `swi_unimpl_1a` | 🔴 | 执行 SWI `0x1A`（SoundDriverInit） | §1.2.1 | **不永久自旋**；机器能继续跑下一条指令；读数里报出「SWI 0x1A 未实现」 | S2-b |
| 2 | `swi_diff8bit_unfilter_wram` | 🟢 | 执行 SWI `0x16`（Diff8bitUnFilterWram），源块是一段**差分流** | §1.2.1 | 落点必须**逐字节等于 GBATEK 自己的 unfiltered 行**（`:192826-192827` 的 10 11 … 19） | S3-a（**r15 转正**：原 `swi_unimpl_16` 的「兜底返回」锁被**改写**为「真实现」锁） |
| 2′ | `swi_diff8bit_unfilter_vram` | 🟢 | 执行 SWI `0x17`（Diff8bitUnFilterVram），落点是 **VRAM** | §1.2.1 | 同上的字节行 + **半字配对顺序**（第一个解码字节在低半字） | S3-a（r15 新增） |
| 2″ | `swi_diff16bit_unfilter` | 🟢 | 执行 SWI `0x18`（Diff16BitUnFilter），源单元是**半字** | §1.2.1 | 每个半字单元都等于 GBATEK 的 unfiltered 值 | S3-a（r15 新增） |
| 3 | `oam_bad_shape` | 🔴 | 用 `STRH` 把 OAM 精灵 0 的 attr0 写成 **shape = 3** | §1.2.2（`:265`） | **不 panic**；该精灵按「禁用」处理；帧正常步进 | S2-a |
| 4 | `oam_gfx_mode3` | 🔴 | 同上，但 attr0 写成 **gfx mode = 3** | §1.2.2（`:144`） | 同上 | S2-a |
| 5 | `eeprom_chip_capacity` | 🔴 | ROM 含 `EEPROM_V124`（8 KB 芯片）；走一遍存档通路 | §1.2.4 **E2** | `.srm` 尺寸 = **8192**（8 KB 芯片）/ **512**（512 B 芯片）——今天**恒 32768** | S3-b |
| 6 | `flash_status_reg` | 🔴 | ROM 含 `FLASH_V124`；先往 `0x0E00_0000` 写 `0x00`，再读 `0x0E00_0006` | §1.2.4 **E3** | 读回的是**芯片状态**（ready 位 bit0 = 1），不是存档原始字节 | S3-b |
| 7 | `rom_window_mirror` | 🔴 | ROM 只有 `0x200` 字节；读 `0x0800_0040` 与 `0x0800_0240` | §1.2.5 | 两个地址读到**同一个绝对金标准值**（不是开放总线垃圾） | S3-d |
| 8 | `gpio_read_before_write` | 🟢 | ROM 含 `EEPROM_V124`；**先读后写** GPIO 0xC4–0xC9 | §1.2.3 线索 | **维持现状**：「写入后读得到 GPIO 值、写入前读到 ROM 字节」——GBATEK **未定义**「未写入前」的语义（r3），故**不可断言别的** | S3-c（降级） |
| — | `eeprom_protocol_lock` | 🟢 | 按 **GBATEK 协议**读写 8 KB EEPROM | §1.2.4 | **维持现状**：vendor 与 GBATEK **逐位一致**（r3），此条是**防 S3-b 改坏协议**的回归锁 | S3-b（回归锁） |

> ⚠️ **r3（2026-11-09）**：**8 条夹具里只有 7 条会红**（🔴），另 2 条是**绿色回归锁**（🟢）。
> ⚠️ **r5（2026-11-09）更正：夹具实为 9 条（7 红 + 2 绿锁），不是 8 条** —— 7 + 2 = 9。
> **门禁计数 262 不受影响**（绿锁只算 2 条常驻）。
> 原因是 GBATEK 逐位核实（§1.2.4 的 r3 追加）：**EEPROM 的命令协议 vendor 本来就是对的**，
> **GPIO「未写入前」的行为 GBATEK 未定义** ⇒ 这两条不能写成会红的断言。
> ⇒ **`-- --ignored` 下的复现器集合 = 7 条**。
> ⇒ 详见 `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §1.7.1 / §1.8 / §五。
>
> ✅ **r15（2026-11-11）更正**：S3-a 落地后夹具 **10 → 12 条**，红绿分布
> **3 红 + 9 绿**（r12 时是 3 红 + 7 绿）。改动三处：① `swi_unimpl_16`
> **改名为 `swi_diff8bit_unfilter_wram` 并从「兜底返回」锁改写为「真实现」锁**
> ——`0x16` 有了实现，它作为「未实现 SWI」的锁名与前提都不再成立；
> ② 新增 `swi_diff8bit_unfilter_vram`（`0x17`）与 `swi_diff16bit_unfilter`（`0x18`）；
> ③ 「兜底」这件事本身仍由 `swi_unimpl_1a`（`0x1A` **故意**不实现）锁着。
>
> **✅ r17 收口（本表的历史使命结束——复现器清零）**：三条红复现器全部转正为
> **常驻绿锁（函数名不变）**，断言按 r16 证伪后的语义反转/改写：
> `eeprom_chip_capacity` = 动态容量（512 B 起步 / 14 位访问扩到 8 KB 且经 14 位
> 回读种值 / 裸 `EEPROM_V` 卡同样到 8 KB——钉住「ID 串不携带容量」）；
> `flash_status_reg` = **回读语义**（四源重定性，r16）；`rom_window_mirror` =
> **开放总线公式**（镜像前提证伪，r16）。另有 S3-e 两条新绿锁
> `mode5_page_flip_reads_the_selected_frame` /
> `mode4_index0_is_transparent_under_a_lower_priority_obj`
> （新文件 `fixtures/lcd.rs`，已注册进 G2 扫描集合，归属 S3-e）。
> ⇒ **夹具集合 12 → 14 条，全部常驻绿锁；`REPRODUCERS = []`；每条保持变异验证
> 双向可红的纪律**（r17 的 S3-b 路 13 腿 + S3-a/S3-e 路 15 腿全部实测）。
**夹具的纪律**：
- 每条夹具**必须能单独触发一条缺陷**，一个夹具不混两件事；
- 夹具的期望值**按字段拼装，不手写字面量**（AGENTS.md 纪律 5）——
  **二进制 blob 无法满足这一条**，这是 r2 改载体形态的首要理由；
- 每条夹具配套**变异验证**：把被断言的行为改回去 ⇒ 该锁测试必须转红；
- ⚠️ **OAM 必须用半字/字写**：`bus.rs:1231-1234` 把 `STRB` 写 OAM **忽略**了，
  用字节写造不出非法 shape/gfx ⇒ 夹具会「测了个寂寞」；
- ⚠️ **断言必须能区分「状态」与「数据」**（AGENTS.md 纪律 13）：
  夹具 8 若直接读 `0x0E00_0006`，今天的返回值 `0xFF` 的 bit0 **恰好也是 1** ⇒ 无牙门禁；
  必须先往存档区写 `0x00` 再读状态位才能区分。

### 2.3 S1-b 探针改造（在**现有**探针上加，不另起炉灶）

| 改造 | 落点 | 目的 |
|---|---|---|
| **修 cartprobe 的 verdict 逻辑** | `cartprobe.rs:338-345` | 现在只看通道 3 却印 DMA1/2 的「never fired」⇒ 改成按通道分别判，别再输出误导判词 |
| **加 SWI 陷阱记录** | `swi/mod.rs` dispatch 入口 | 记录每个 SWI 号码与「是否已实现」⇒ 4 张失败卡各自踩了哪些号码，**直接可读** |
| **加 panic 捕获** | `frame.rs:375` `gba_step_frame` 外层 | `catch_unwind` 包住帧步进，把 panic 转成**可读读数**（哪个地址、什么消息），而不是带走进程 |
| **加输入注入** | `frame.rs` 已有 `gba_set_buttons` | 让探针能在指定帧按指定键 ⇒ 复现 Dragon Ball 的「按键后卡死」 |
| **加长窗口 + 卡死点判定** | `cartprobe.rs` | 窗口从 240 帧拉到可配置；判「卡死」用「PC 连续 N 帧不变」而不是「帧预算耗尽」 |
| **让 `probe_what_one_frame_changes` 能对付忙等** | `frame.rs:2929+` | 现在必须 halt 才开窗 ⇒ 改成「忙等也能开窗」（按指令窗口而非 halt 窗口） |

### 2.4 S1 出口判据（**可证伪**）

> ⚠️ **r2 改写（业主 2026-11-09 拍板 D2）——前两条改了口径与数字。**
>
> **原第一条与「§〇 范围内第 2 条：每条缺陷都要有一条会红的锁测试」互相矛盾**：
> 一条断言「SWI 0x1A 之后机器不永久自旋」的测试，在 S2-b 修复之前**必然是红的**，
> 「会红」与「全绿」不可能同时成立。
>
> ⇒ **裁决：复现器标 `#[ignore]`** —— 断言写的是**目标行为**（不是缺陷行为），
> 修复落地时转成常驻锁测试（当场先红后绿 + 变异验证）；
> **另加 3 条常驻守卫**（孤儿 `#[ignore]` / 夹具完整性 / 复现器存在性）防止 ignore 腐烂。
> ⇒ **不采用「断言写成缺陷当前行为以求全绿」**：那等于把缺陷锁成预期行为，
> 与本计划 §1.2.1 自己对 `bios.rs:396-401` / `bios.rs:218-223` 的批评冲突。
>
> ✅ **2026-11-11 逐条核对后勾选**（依据 = 当日实测读数 + r4–r13 的变更记录）。
> 括号里的数字是**当时的实际读数**，不是事后追认；口径与 §2.2 的「9 条」表、
> §五 编排图（「S1-a 最小夹具（9 条）」）存在**已知不一致**——那两处仍写 9 条，
> 而 r12 后夹具已增至 **10 条**（7 条常驻绿锁 + 3 条红复现器）。

- [x] **夹具全部就位**（`src/rust/src/gba/fixtures/*.rs`，按缺陷域分四个文件
      `swi` / `oam` / `backup` / `rom`，公共底座在 `fixtures.rs`；**Rust 内建、无二进制入库**）；
      **r15 后共 12 条** = **9 条常驻绿锁**（swi ×5 / oam ×2 / backup ×1 / rom ×1）
      + **3 条红复现器**（`rom_window_mirror` / `eeprom_chip_capacity` / `flash_status_reg`
      —— 归属 S3-d / S3-b / S3-b，**未修 ⇒ 按设计为红**）；
- [x] `cargo test -p fceux11-rust --no-default-features --features gba --lib`
      **从 249 项增加到 280 项，全绿**（2026-11-11 实测 `280 passed / 22 ignored`）
      = 249 + **8** 条编码器端到端自检 + **3** 条守卫 + **2** 条绿色回归锁（r5 的 262）
      + **2** 条探针锁（r6）+ **2** 条 OAM 转正（r7）
      + **4** 条（r12：两条 SWI 转正 + Thumb 绿锁 + 生产记录器锁）
      + **8** 条单元锁 + **2** 条端到端夹具（r15 的 diff-unfilter 三号）；
      另有 **`#[ignore]` 复现器**（r5 时 7 条 ⇒ r7 后 5 条 ⇒ **r12 后 3 条**：
      两条 OAM（r7）与两条 SWI（r12）复现器已转常驻绿锁），
      在 `-- --ignored` 下运行，**它们必须逐条为红**（红 = 缺陷在场）——
      2026-11-11 实测 `0 passed / 3 failed`，红的正是 S3-b ×2 + S3-d ×1；
      **✅ r17 更新：三条复现器已随 S3-b/S3-d 收口全部转正（`REPRODUCERS = []`），
      现为 `0 tests / 0 failed`（无可跑项）——上文读数为当时的实测口径，保留不追改**；
- [x] 每条复现器都带 `#[ignore = "S2-x / S3-x: 修复后转常驻锁测试"]` 的**指名归属**
      （G1 守卫强制：孤儿 `#[ignore]` 的原因串必须指名工作项）；
- [x] **每条绿锁今天就是绿的**（EEPROM 协议逐位对 GBATEK、GPIO 机制事实）
      —— 9 条常驻绿锁（`eeprom_protocol_lock` / `gpio_read_before_write` /
      `oam_bad_shape` / `oam_gfx_mode3` / `swi_unimpl_1a` /
      `thumb_swi_unimpl_1a_returns_and_continues_in_thumb` /
      `swi_diff8bit_unfilter_wram` / `swi_diff8bit_unfilter_vram` /
      `swi_diff16bit_unfilter`）全绿；
      它们的作用是**防 S2/S3 的修复改坏本来正确的东西**；
- [x] 每条新锁测试的**变异验证双向可红**（改回修复前的值 ⇒ 转红；还原 ⇒ 复绿）
      —— r5 逐条双向、r7 两腿、r12 的 M1–M4、**r15 的 M1–M5**（各腿其余测试保持绿）。
      **r15 五腿读数**（基线 280 全绿；执行器 `.zcode/mutation_s3a.ps1`，**不入库**）：

      | 腿 | 改动 | 转红的锁 |
      |---|---|---|
      | M1 | `0x16` 读成 16 位单元 | `swi_diff8bit_unfilter_wram`（**只红这一条**） |
      | M2 | `0x17` 改成字节写 | `swi_diff8bit_unfilter_vram`（**只红这一条**） |
      | M3 | `0x18` 读成字节单元 | `swi_diff16bit_unfilter`（**只红这一条**） |
      | M4 | 半字配对顺序颠倒 | `swi_diff8bit_unfilter_vram` + `swi_diff16bit_unfilter`（两条共用同一段半字写路径） |
      | M5 | size 字段读成 16 位 | `the_size_field_is_twenty_four_bits_wide`（**只红这一条**） |

      每腿还原后 **280 passed / 0 failed**。
      ⚠️ **M5 首轮是「无牙」的**：当时的夹具 size 只有 10 / 20 字节，16 位与 24 位两种读法
      **结果完全相同** ⇒ 补了 `the_size_field_is_twenty_four_bits_wide`
      （用 64 KiB 的 size，bit 16 置位）才让这一腿有牙。
      ⇒ 这与 r4 的三条盲区是**同一类**：断言的值太小，两种读法都过得去。
- [x] **编码器端到端自检逐条变异可红** —— 编码器是全部夹具的**共因**，
      它错了 ⇒ 夹具都在测别的东西，而且照样「全绿」（AGENTS.md 纪律 12 的同族，更隐蔽）。
      r4 的矩阵 **8/8 的目标都能红**（首轮测出 3 条盲区并修掉）；
      ⚠️ 其中 M1（`mov_imm`，唯一的造地址手段）与 M7（`swi(HALT)`，每条自检的终止器）
      两行是**结构性共因** ⇒ 改坏编码器会**响亮地全局失败**，代价是该行无法定位；
- [x] `cartprobe` 在 4 张失败卡带本地跑出**明确读数**（哪张卡、卡在哪个 PC、
      踩了哪些未实现 SWI、有没有 panic）；**进仓库的是夹具读数，不是卡带读数**
      —— r6 / r7 / r12 三轮 7 卡基线（240 帧窗口）**逐字一致**、零 panic、零未实现 SWI；
      Dragon Ball 的输入注入实验证明游戏对输入有响应；
- [x] `cartprobe` 的 verdict 判词修正后**不再出现**误导性表述
      —— `describe_timing(channel, timing)` 通道化（DMA1/2 = sound FIFO（本核心已服务）
      / DMA3 = video capture（从未触发）），并由常驻锁
      `special_timing_wording_is_channel_aware` 钉住措辞；
- [x] **NES 零回归**：`ctest -LE perf` **36/36 全绿**（2026-11-11 实测，Release 构建 EXIT=0）；
      ⚠️ **F11QA 矩阵 `pass_to_fail = 0` / grade ≥ B 待 push 后由 CI 复核**
      （本地无矩阵入口，只有 `ctest` 这个近似门禁）；
- [x] ⚠️ **前置条件**：GBATEK 快照须补齐 **EEPROM 命令协议** 与 **GPIO/RTC** 两章
      —— **r5 补 A.4（EEPROM 命令协议 `:8710-8786`）+ A.5（EEPROM 寻址与容量 `:8661-8706`）**，
      **r6 补 A.6（GPIO/RTC `:9218-9406`）**；另有 `tools/fetch_gbatek.ps1` 可重跑且
      **逐字节复现**现有快照 ⇒ 附录 A 的行号引用在重抓后仍成立
      ⇒ 夹具 5 与夹具 10 的协议级断言**已有权威依据**（AGENTS.md 纪律 4）。
      ⚠️ **遗留**：**Flash 状态寄存器 `0x0E000006/7` 在 GBATEK 全文零命中**
      （GBATEK 的 flash 轮询是**回读比较** `:8981`）⇒ `flash_status_reg` 复现器断言的是
      **本计划自定的约定**，不是权威来源 ⇒ **S3-b 开工前须另找来源或用真卡带重新推导**。

---

## 三、S2 阶段 · 止血（**先不崩**）

> 目标不是「跑对」，是**把「闪退」变成「画面不对但还在跑」** —— 只有这样，后面的
> 定位才能继续做下去。

### 3.1 S2-a vendor 核的 panic → 开放总线 / 忽略写

把**游戏可触达路径**上的 panic 全部改成「读回开放总线 / 写被忽略 + 记一次日志」：

| 位置 | 改成 |
|---|---|
| `bus.rs:596` | 写未建模 DMA 寄存器 → 忽略 + 计数 |
| `bus.rs:291/302/343/422/520/538/558/576/957/1069` | 读越界 → 回开放总线；写越界 → 忽略 |
| `internal_memory.rs:660` | 读未映射区域 → 回开放总线（`0x0D`/`0x0E` 区域按芯片类型处理） |
| `arm/operations.rs:896` | PLD 指令 → 按 NOP 语义实现（不是 `todo!`） |
| `object_attributes.rs:262/265/389/475` | OAM 非法 shape/gfx-mode → **映射为「精灵禁用」**，不 `.unwrap()` |

**纪律**：**内部不变量**（如 `unreachable!()` 覆盖的 2 bit 字段越界）**保留** ——
只改**外部输入可达**的那些。改完要在注释里写明「这是外部可达路径，改为容错」。

### 3.2 S2-b 未实现 SWI 的兜底

> ✅ **已落地（2026-11-11，r12）**：向量字 = `MOVS PC, LR`（`0xE1B0F00E`），未实现 SWI
> 返回续接点（ARM/Thumb 双路夹具实证），生产侧 `note_unimplemented_swi` 每号首次日志 +
> 常驻计数（vendor 自答 6 号不计入）。⚠️ 本节「寄存器返回值按 mGBA 的做法给合理默认」
> **口径已修正**：mGBA 源码不在本机，改按 GBATEK SWI ABI 自建依据（R2,R4-R14 保持不变
> ⇒ 全寄存器保留、零编造返回值），见 §九 r12。两条被 §1.2.1 预告的锁测试已改写并更名；
> 复现器 5 → 3；夹具 9 → 10（新增 Thumb 绿锁）。

- 把 `bios.rs:134` 的 SWI 向量从 `BRANCH_SELF` 改成「**返回 + 记录 SWI 号码**」。
- ⚠️ **必然要改两条锁测试**：`bios.rs:396-401`、`bios.rs:218-223`
  —— 断言从「park」改成「返回并记录」（§1.2.1 已预警）。
- **兜底语义**：SWI 的寄存器返回值按 mGBA 的做法给「合理默认」，而不是随机值。
- ⚠️ 这是**降级**不是**修复**：真正的修复是 S3-a 补齐实现。兜底只是把
  「永久卡死」变成「功能缺失但游戏能跑」。

### 3.3 S2-c C++ 侧不崩

> ✅ **已落地（2026-11-11，r13）**：`gba_load.cpp` 每帧步进错误码 → `note_step_error`
> 去重日志（`FCEU_printf`＝stdout + 消息日志 dock；**刻意避开 `FCEU_PrintError`**——
> 它下游是弹窗）。不弹窗/不崩/不卸载；同码 streak 首帧报、码变再报、干净帧与新会话清闩。
> 详见 §九 r13。

- `src/gba_load.cpp` 的每帧步进包一层错误处理：Rust 侧返回错误码时
  **不弹窗、不崩**，只记日志并继续（对齐 AGENTS.md 纪律 6）。

### 3.4 S2 出口判据

- [x] **9 条夹具**全部**从「会崩/会卡死」变成「能跑完」**（r12 收口）：会崩/会卡死的四条
      （swi 自旋 ×2、OAM panic ×2）全部能跑完——swi 两条已转常驻绿锁；其余红复现器
      不崩不挂（断言的是 S3 待修值）；r12 另增 Thumb 绿锁，夹具 9 → 10；
- [x] `gba_step_frame` 在所有夹具下**不返回 panic**（r6 catch_unwind 兜底 + r7 OAM 修复 +
      r12 变异 M1–M4 背书）；
- [x] 4 张失败卡带本地跑：**不再闪退**（r6/r7/r12 三轮 7 卡基线零 panic；MKSC r56 已可玩；
      卡死/黑屏仍在——那是 S3 的事）；
- [x] NES 零回归（r12/r13 终轮：`ctest -LE perf` 36/36；F11QA 矩阵待 push 后 CI 复核）。

---

## 四、S3 阶段 · 修根因

### S3-a SWI `0x16–0x2A` 补齐

> ✅ **第一批已落地（2026-11-11，r15）：`0x16` / `0x17` / `0x18` 三个 diff-unfilter**
> —— 新的 bus-free 模块 `src/rust/src/gba/swi/unfilter.rs` + `swi/mod.rs` 的
> `serve_diff_unfilter`。依据 = **GBATEK 快照**（`:192251-192253` 号段、
> `:192804-192867` 契约与算术、`:192826-192827` **它自己的示例向量**）
> **+ mGBA 的 `_unFilter`**（`src/gba/bios.c:903-946`，**只读格式、不抄代码**——
> 它是唯一说清「哪一号读 8 位单元、哪一号存半字」的来源）。
> ⇒ 夹具 `swi_unimpl_16` **改写**为 `swi_diff8bit_unfilter_wram`，
> 新增 `swi_diff8bit_unfilter_vram`（`0x17`）与 `swi_diff16bit_unfilter`（`0x18`）；
> 期望值用 **GBATEK 公布的 10 11 … 19** 那一行，**不是**拿实现的同一套算法重算
> （测试与实现同错是本项目栽过两次的坑）。
> ⇒ 详见 §九 r15。

| 优先 | 号码 | 名称 | 备注 |
|---|---|---|---|
| P0 | `0x16/0x17/0x18` | Diff8bitUnFilterWram / Vram / Diff16bitUnFilter | 压缩数据加载常用，商用卡命中率最高 —— ✅ **r15 已落地** |
| P0 | `0x1A–0x1E` | SoundDriverInit / Mode / Main / VSync / ChannelClear | **官方 SDK 声音驱动**，Nintendo 自家卡几乎必用 —— ⚠️ **r16 判定：mGBA HLE（无真 BIOS）对这 12 号也全是 no-op stub**（`bios.c:607-608` default 分支），真实音频产出（官方软合成器）任何模拟器都没有 HLE ⇒ 无权威依据实现真语义，**维持 S2-b 兜底 + 记已知限制**（认领成 no-op 会违背红线 3 的「未实现必须可见」，不做） |
| P1 | `0x1F` | MidiKey2Freq | ✅ **r17 已落地**（GBATEK `:194413-194414` 公式标注推导；freq 在 **WaveData+4**——证据表旧记 +8 已更正；整数八度向量独立拼装） |
| P1 | `0x20–0x24` | MusicPlayerOpen / Start / Stop / Continue / FadeOut | ⚠️ 同声音家族（r16 判定），维持兜底 + 已知限制 |
| P2 | `0x19` | SoundBias | ✅ **r17 已落地**（SOUNDBIAS `0x04000088` vendor 已建模；低 10 位替换、高位保持；「依赖 `0x270` 声学校准表」半句经 r16 复核**维持判伪**） |
| P2 | `0x25/0x26/0x27` | MultiBoot / HardReset / CustomHalt | ✅ r17：`0x25`（r0=1 failed，`:193914` 契约）与 `0x27`（参数在 **r2**，复用 `enter_low_power`）已落地；**`0x26` HardReset 结构障碍维持**（dispatch 拿不到整机所有权）+ 零观测需求 ⇒ **挂起** |
| P2 | `0x28/0x29/0x2A` | SoundDriverVSyncOff / On / GetJumpList | `0x28`/`0x29` 同声音家族维持兜底；**`0x2A` GetJumpList BLOCKED**（36 项函数表 GBATEK 零记载 `:194549`/`:194556`、mGBA 显式放弃）⇒ 维持 S2-b 兜底 + 已知限制 |

**每一号都要有**：
1. **GBATEK 权威依据**（附录 A 补摘录，**不许凭记忆**）；
2. 一条**锁测试**（期望值按字段拼装）；
3. **变异验证**双向可红；
4. 对 `0x19 SoundBias` 这类依赖**只读常量表**的，登记表从哪来（mGBA 的表**只读不抄**，
   数值须另寻权威来源或自行推导并**标注为推导**）。

### S3-b 存档芯片模型

1. **EEPROM 真模型**：支持 512 B 与 8 KB 两种；地址宽度、位序、命令协议按 GBATEK；
   `internal_memory.rs:523-532` 的 `is_eeprom_access()` 窗口**收窄**到真机范围。
2. **Flash 状态寄存器**：`0x0E000006/7` 按 GBATEK 的状态位定义返回；
   擦除/写入的「忙→就绪」状态机要真的走。
   > **⚠️ r5 更正：GBATEK 没有 flash 状态寄存器。** 全 4.0 MB 快照 grep `E000006`
   > **零命中**；GBATEK 的 flash 轮询是**回读比较**（`:8981` "wait until
   > [E00xxxxh]=dat"、`:8915` "[E000000h]=FFh"）⇒ **「按 GBATEK 的状态位定义返回」这句
   > 没有权威依据**。⇒ S3-b 开工前必须先解决 `0x0E000006/7` 与「ready = bit0」的
   > **来源问题**（另找权威来源，或用真卡带实测重新推导）；在补上来源之前，
   > 复现器 `flash_status_reg` 断言的是**本计划**给的这两个约定，不是 GBATEK。
3. **`.srm` 尺寸跟芯片走**：`buffer_size()` 已按芯片给，但 EEPROM 目前是占位 32 KB
   ⇒ 改为 512 B / 8 KB 真实尺寸。
4. **存档类型可覆盖**：`gba_set_save_type` 已有 ⇒ UI 侧接线（属产品决定，另议）。

> **✅ r17 收口（第 1–3 条落地，语义有两处按 r16 证据改写）**：
> ① EEPROM 容量走 **D1 动态模型**（不是按 ID 串定型号——GBATEK `:8521` 否证了
> 「nnn 编码容量」，原第 1 条的「支持 512 B 与 8 KB 两种」改为「512 B 起步、高位
> 访问懒扩 8 KB 封顶」，mGBA 同款；`is_eeprom_access()` 窗口**不动**——r3 已证
> 与 GBATEK 一致）；vendor 3 处（V1 `buffer_size` 0x0200 / V2
> `ensure_eeprom_capacity` / V3 `load_battery` 旧 32 KB `.srm` 前 8 KB 无损迁入）
> + 4 条单元锁，ATTRIBUTION §3.2.13–§3.2.15。② Flash 状态寄存器**不做**——
> r16 四源重定性：真语义 = 阵列字节回读，vendor 本来就对；`flash_status_reg`
> 断言反转为绿锁。③ `.srm` 尺寸随 D1 动态（512 B / 8 KB）；旧 32 KB 档兼容。
> ④ **顺带修掉 `apply_state` 潜伏缺陷**（比 RE 报告更深：读档后 ROM/BIOS/
> backup_type 三者全空 ⇒ 真实游戏读档致命）——修复 = `apply_state` 内三条语句
> + 锁；**新已知限制**：手动 `gba_set_save_type` 覆盖随读档被 redetect 还原
> （vendor doc 既定语义）。⑤ 第 4 条 UI 接线维持挂起（产品决定）。

### S3-c GPIO / RTC 与 FireRed 的标志位

1. **先取证再动手**：用 `gba_gpio_rtc.gba` 夹具 + 探针，**证实或证伪**「先读后写导致
   RTC 探测失败」这条线索（§1.2.3）。
2. 若成立 ⇒ 让 GPIO 在**卡带类型判定为 RTC 卡**时**一开始就可读**，
   而不是「被写过一次才生效」。
3. **FireRed 的标志位写入方**：S1-b 的「忙等也能开窗」改造落地后，用它找出写方。

> **✅ r16 以取证结案（第 1 条的答案是否定的，第 2 条因此不做）**：用探针实测
> （`probe_s3c_forensics`，全在 `#[cfg(test)]`）——**「先读后写」线索证伪**
> （FireRed 300 帧零写 GPIO、30M 指令 sniff 零指针加载）⇒ GPIO 开局可读的改动
> **不解决 FireRed，不做**（不碰 vendor 的 `gpio_present` 机制；`gpio_read_before_write`
> 绿锁维持，作为 GPIO 机制事实的回归锁）。第 3 条的答案已找到：写入方 =
> ROM `0x080007B4`（VBlank IRQ 回调 `STRH r1,[r5,#0x1C]`，r5 = gMain）；
> FireRed 的真根因是其自身 init 时序竞争（影子寄存器 apply 链，
> **见 §1.2.3 的 r16 收口注**）⇒ 属精度范畴，转长尾。取证读数全文见
> `.zcode/v203_stage1/s3c_forensics.md`（探针改动只落在 `cartprobe.rs` 测试模块）。

### S3-d ROM 窗口镜像

`internal_memory.rs:377-404`：`address >= rom.len()` 时**镜像卡带**而不是回开放总线
（≤32 MB 的卡在 `0x08000000–0x09FFFFFF` 是镜像的）。

> **✅ r16/r17 重划收口（「镜像」前提被 GBATEK `:14046` 原文证伪）**：真机行为 =
> **不镜像、回开放总线**（公式 `:14029-14030`，mGBA/melonDS 同判）⇒ vendor 现状
> 正确，**不做镜像**。原复现器 `rom_window_mirror` 改写为**开放总线绿锁**：
> 控制腿（ARM `LDR 0x0800_0040` = GOLDEN）+ 字节腿 + 半字腿 + 字腿 + 双哨兵，
> 期望值全部由夹具内独立拼装的 GBATEK 公式给出（不调 vendor 推导——纪律 5/12；
> 「不镜像」由字节/半字腿锁死：将来若有人实现镜像，镜像值 ≠ 公式值，两腿即红）。
> 顺带：`try_read_word` 越界臂（仅整段越界分支）改为半字对公式快路径
> （ATTRIBUTION §3.2.16；⚠️ 定性 = 快路径契约/性能修复——总线级值修复前后
> 逐位相同，设计报告的「0x0101_2120 形态」是端序误读，已实证更正）。
> mGBA 的 1 MiB 4 倍镜像 / 非 2 幂补零两例外**不跟**（可选项未获裁决，默认不做；
> 夹具对 1 MiB 类卡不设防，将来立项单独处理）。

### S3-e 显示侧小修

1. mode 4 的 palette index 0 **不当作透明**（`lcd/layers/layer_2.rs:193-196`）；
2. **mode 5 的页面翻转**（DISPCNT bit 4）实现（`lcd/layers/layer_2.rs:212-237`）；
3. BG2 文本层只在它有意义的模式下渲染（`lcd.rs:343-349`）。

> **✅ r16/r17 收口（三条只剩一条是真缺陷）**：第 1、3 条被 GBATEK 证伪
> （`:3198`：bitmap 模式 Color 0 = backdrop 透明，vendor 正确；`:1944-1952`：BG2
> 全模式存在）——**都按「不改行为」处理**：index 0 加 **OBJ-behind-BG2 防回归锁**
> （直接断言 index0==palette[0] 是无牙的：backdrop 色就是 bg_palette_ram[0]，两种
> 行为同值），BG2 只记证伪不加锁（分发表锁是「测试与实现同源」的低价值锁）。
> 第 2 条已修：`render_mode5` 补 `frame_offset = bit4 ? 0xA000 : 0`（与 mode 4
> 同式；ATTRIBUTION §3.2.17，vendor 补丁第 33 处）+ 2 条 vendor 单元锁 +
> 端到端锁 `mode5_page_flip_reads_the_selected_frame`（两页逐行不同图案、
> bit4 两运行互为对照；M1 摘 offset / M2 基址误写 0x9600 均可红）。

### S3 出口判据

> **✅ r17 收口勾选。⚠️ 两处按证据改写（纪律 2：显式记录，不静默放宽）**：
> ① 夹具名以 §2.2 的 Rust 内建夹具为准（r2 改载体后旧 `.gba` 名全部失效）；
> ② **FireRed 行改写**——r16 取证证明其冻结根因是**游戏自身 init 时序竞争**
> （影子寄存器 apply 链；DISPSTAT bit3 未开闸，与 GPIO/存档芯片均无关），
> 属 GBA 精度范畴、超出本轮「能不能跑」的范围 ⇒ 本计划的判据改为
> 「根因定位 + 转长尾」，原「出画面 + 存档正确」目标转入 AGENTS.md 长尾
> （与 §9.1 八条、DMA 两项 RE 同类：Alpha 阶段按需驱动），**不由本计划背书**。

- [x] S3-a **已认领的**每个 SWI 号都有锁测试 + 变异验证（r15 第一批 3 号 + r17 第二批
      4 号 + `0x01` 搭车；声音家族 12 号与 `0x2A` 按纪律 4 记录为已知限制、维持
      S2-b 兜底——r16 判定，认领成 no-op 会违背红线 3）；
- [x] `eeprom_chip_capacity` / `flash_status_reg` / `rom_window_mirror` 逐条转绿
      （r17；断言按证伪后语义反转/改写，见 §2.2 的 r17 收口注）；
- [x] **4 张失败卡带本地机器判据复核（r16/r17）**：
      | 卡带 | 原目标 | r16/r17 读数 |
      |---|---|---|
      | FireRed | 出画面 + 存档正确 | **✅ r19 更新：机器判据「出画面」在探针窗口内达成**——r17 的 `0x01` 强制空白使它越过 init 竞争（FROZEN → 48 变化帧、音频 DMA 激活；摘掉该写入即回 FROZEN，双向定位实证，见 §九 r19）；进主线观感仍待人工；竞态根因分析不变，长尾条目保留。取证同时证明其存档通路（FLASH1M）与该症无关 |
      | Mario Kart SC | 进游戏不卡死 | ✅ 1300 帧 + 14 组注入：1088 hash、DISPSTAT bit3 正常、DISPCNT 自帧 400 稳定 0x1741（gameplay 形态）、零 panic 零未实现 SWI；进赛观感转人工 |
      | Dragon Ball | 按键后不卡死 | ✅ 1200 帧 + 12 组注入：468 hash、12/12 响应、卡死未复现；EEPROM 读通路 184,821 次访问零写提交（`save_dirty` 永未置位）、VBlankIntrWait ×1242 全 served |
      | Minish Cap | 进主线出画面 | ✅ 1800 帧：797 hash、DISPCNT 四段推进（0x0540→0x1741→0x1F40→0x2640）、黑屏未复现（r6 结论外推 7.5 倍）、10/10 注入响应；进主线观感转人工 |
- [x] **NES 零回归** + GBA 锁测试数 ≥ 257（r17 后 **299 passed / 0 failed / 20 ignored**）
      + vendor **249** / debug 93 / media 52 三条门禁全绿（`ctest -LE perf` 36/36；
      F11QA 矩阵待 push 后 CI 复核）；
- [x] vendor 树改动**全部登记进 `ATTRIBUTION.md`**（§3.2.13–§3.2.17；补丁 **28 → 33 处、
      6 → 7 个源文件**——internal_memory.rs 4 → 8、新增 `lcd/layers/layer_2.rs` 1）。

---

## 四之二、S4 · FC 侧修复批次（r8 业主裁决扩围，**待编码**）

> **批次名称（= GitHub issue #12 标题，逐字）**：
> **「fix iNES loader rejects non-power-of-2 PRG ROMs; mappers 191/199 missing from MapperEntry registry (grey screen)」**
> 来源：GitHub issue #12（报告者 chenkohan，2026-11 报告，涉及 v2.0.1 Windows 版；
> 业主已回复并索取 ROM 用于验证）。**2026-11-10 完成只读核实与预研（本节的全部 file:line
> 都是当日实测），两缺陷均属实**；业主裁决：与 GBA 三段式并行，文件域互斥、随时可插。
> **本节是自足实施计划——下一轮编码作业按此开工，不需重新取证。**
> ⚠️ 提交信息里请引用批次名称与 issue #12。

### S4.0 事实底座（2026-11-10 只读核实，全部 file:line 实测）

**Bug 1 · iNES loader 硬拒非 2 幂 PRG**（复现 ROM 形态：iNES 1.0，PRG 12×16KB、CHR 16×8KB、
mapper 191、文件 327696 字节，头完全自洽）：

| 环节 | 实测位置 | 事实 |
|---|---|---|
| PRG 尺寸舍入 | `fceux11-formats/src/cart.rs:90` | `rom_size_16kb = uppow2(12) = 16`（`rom_size_raw = 12` 同结构体保留） |
| 布局取舍入数 | `fceux11-formats/src/ines.rs:721-731` | `NOT_POWER2 = [53, 198, 228, 547]`（`ines/ines_data.rs:206`）**不含 191** ⇒ `round=true` ⇒ `prg_size_bytes = 16 << 14 = 262144` |
| 硬切片 + 硬拒绝 | `fceux11-formats/src/ines.rs:961-978` | `:963` PRG 文件边界检查与切片用**舍入后**尺寸 ⇒ 吞掉 CHR 前 64KB；`:972` CHR 越界 `262160+131072=393232 > 327696` ⇒ `return false`。**即便文件够大，PRG 切片也会把 CHR 错切进 PRG**（报告者指出的正确性隐患成立） |
| C++ 映射 | `src/ines_load.cpp:148-149` | false → `LOADER_INVALID_FORMAT` ⇒ 用户看到「Unknown ROM file format」 |
| C++ 消费链（预研） | `src/ines_load.cpp:171-186` | **已经是**「`FCEU_malloc(cart.prg_size)`（舍入）+ `memset 0xFF` + `memcpy(ROM, cart.prg_data, cart.prg_size)`」形态——memcpy 源是 Rust 返回的**文件内视图**，PRG 改用 raw 尺寸放行后若 memcpy 仍拷舍入数字节会**越过 memstream 缓冲区越界读** ⇒ 修复必须同步改 memcpy 尺寸 |
| 上游（fork 原版）行为 | fork 初始提交 `1c35642` 的 `src/ines.cpp:869-954` | 无任何文件尺寸硬校验：`malloc(舍入)`+`memset 0xFF`+`FCEU_fread` **不检查短读** ⇒ 装载永不拒绝（这就是「上游能跑」的机制）。⚠️ 两处细节与 issue 表述的偏差（不影响结论）：原版 round 分支 `fread` 的也是舍入数，靠不查短读存活；「expected = 读 raw」是**比原版更正确**的修法而非还原原版 |

**Bug 2 · mapper 191/199 未注册 → 灰屏**：

| 环节 | 实测位置 | 事实 |
|---|---|---|
| 注册表缺席 | `src/boards/mmc3.cpp`（`MapperEntry{...}` 全列） | 实际注册 25 个变体（4,12,37,44,45,47,49,52,**76**,114,115,116,118,119,165,192,194,195,198,205,245,249,250,254,406）——**191、199 均不在**（issue 清单少列 76，无实质影响） |
| 有实现没注册 | `Mapper191_Init` = `mmc3.cpp:1017`；`Mapper199_Init` = `src/boards/199.cpp:83`（上游同构，DBZ 系列注释在文件头） | 191：`GenMMC3_Init(256,256,8,battery)` + `CHRRAMSIZE=2048` + `M191CW: setchr1r((V&0x80)>>3,...)`——issue 描述**逐字属实**；199：`CHRRAMSIZE=8192`、`(V<8)?0x10:0x00` 选 CHR-RAM、`EXPREGS`+`M199PW` 扩展 PRG（issue 的「sub-regs 0x0A/0x0B」未逐字核到，落地时顺手核对） |
| bmap 行号 | `src/ines_bmap.h:221`（191）/ `:229`（199） | 与 issue 引用精确吻合；legacy `iNES_Init`（`ines_init.cpp:29-74`）扫 `bmap[]` ⇒ **legacy Init 确实会跑** |
| 灰屏机制 | `src/cart_class.cpp:53` + `:23-27` | `create_cart_for_mapper` 未命中注册表 → nullptr → **`g_cart` 停在 `CartPlaceholder`（全 no-op）** ⇒ v1.8 管线手里是空 cart ⇒ 灰屏。旁证：195（已注册，`Mapper195Cart`）能渲染 |
| 根因定性 | `src/cart_class.cpp:49` + `src/boards/registry.h` | 注册表迁移目标「all 171 by Phase H」**未完成**——191/199 是缺口，不是有意排除 |
| 兄弟工厂形态 | `src/boards/mmc3_variants_carts.h:44+` | 每个变体 = **一行式** `class MapperNNNCart : public Mmc3BaseCart {}` + `mmc3.cpp` 一条 `MapperEntry{N, "名字", &MapperNNN_Init, [](Bus&){...}}`——S4-b 体量小、先例充分 |

### S4-a · loader 修复（设计已定，照做即可）

**语义**：**分配与映射保持舍入尺寸**（`SetupCartPRGMapping` 要求 2 幂，上游注释在 fork 原版
`ines.cpp:873-875`），**文件边界检查、指针推进、文件拷贝一律用 raw 尺寸**，尾部由既有的
`memset 0xFF` 填充。**hash 用 raw 文件字节**——决策依据：现有可装载 ROM 里 pow2 卡
raw==rounded、`NOT_POWER2` 四 mapper 本来就是 raw ⇒ **对既有 ROM hash 零变化**；新放行的
非 2 幂卡无历史 hash 包袱。**FFI 结构零变更**（`FceuInesCartResult` 已有 `rom_size_raw`，
C++ 侧 `ines_load.cpp:158` 已在读）。

| # | 改动 | 位置 |
|---|---|---|
| A1 | PRG 文件边界检查与切片改 raw：`offset + (prg_banks_raw << 14) > file_len` 才拒绝；指针/offset 按 raw 推进 | `ines.rs:961-967`（`layout.prg_banks_raw` 已存在，`:696`） |
| A2 | CHR 的 offset 由此自然变小（不再被舍入 PRG 吞掉）；CHR 边界检查逻辑不动 | `ines.rs:969-978` |
| A3 | hash 的 PRG 尺寸参数改 raw 字节（CHR 不动） | `ines.rs:986-994`（`fceux11_rust_ines_compute_hash` 调用点） |
| A4 | C++ memcpy 尺寸改 raw：`memcpy(ROM, cart.prg_data, (size_t)cart.rom_size_raw << 14)`；`memset 0xFF` 与 `SetupCartPRGMapping` 保持舍入尺寸**不动** | `src/ines_load.cpp:180`（:176 的 memset、:186 的 mapping 不动） |
| A4′ | **r9 实施时对 A4 的两处更正（照 A4 字面写会出事）**：① `rom_size_raw` 对 iNES 1.0 的「0 = 256 银行」编码是 **0**，字面写法会把**所有 256 银行 ROM 的 PRG 拷成 0 字节** ⇒ 改用 `rom_size_raw ? rom_size_raw : rom_size_16kb`；② 对 NOT_POWER2 mapper（53/198/228/547）+ 256 银行头，`prg_size` 算出来是 **0**，raw 计数会**越界写** ⇒ 拷贝长度取 `min(raw, prg_size)` | `src/ines_load.cpp:182-197` |
| A5 | **锁测试（formats crate 内）**：合成 header（PRG 12 银行 + CHR 16 银行 + mapper 191）断言装载成功、PRG 尾部填充 0xFF、CHR 切片正确、hash 可算；变异验证 = 边界检查改回舍入 ⇒ 红 | `fceux11-formats/src/ines.rs` `#[cfg(test)]` |
| A6 | **ctest 级端到端锁**：`generate_test_roms.py` 增合成 fixture（非 2 幂 PRG + mapper 191），`mapper_load_test.cpp` 的 `tests[]` 增一行断言装载成功且 `mappernum==191`（pre-fix 该 fixture 装载返回 null ⇒ 红） | `tests/fixtures/generate_test_roms.py`、`tests/f11qa/boards/mapper_load_test.cpp`（kgmqa-016） |

**登记为观察项（本轮不修）**：CHR 同构缺陷——`vrom_size_8kb` 同样被 `uppow2` 且无 raw
对应字段（`cart.rs:94-95`），非 2 幂 CHR 会被硬拒/错切（issue 未涉及）。真实用例出现再立项，
**宁窄勿宽**（同一教训：不为不可达路径扩改动面）。

### S4-b · 注册 191/199（照兄弟形态）

| # | 改动 | 位置 |
|---|---|---|
| B1 | `class Mapper191Cart : public Mmc3BaseCart {}` + `class Mapper199Cart : public Mmc3BaseCart {}`（一行式，同 `Mapper195Cart`）；`mmc3_variants_carts.h` 头注释的映射表与「24 variants」计数同步更新 | `src/boards/mmc3_variants_carts.h` |
| B1′ | **r9 实施时对 B1 的一处更正（199 不能是一行式）**：`src/ines.cpp:68-70` 在 `iNES_Init`（= legacy `MapperNN_Init`，它会设 `info->Power`）**之后**把 `iNESCart.Power` 覆写成 `CartInfo_PowerForward` ⇒ **power 周期实际只走 `cart_obj->on_power()`**。基类的 `on_power()` 调 `GenMMC3Power()`：191 正确（它的 Init 没有自定义 power），但 **199 有自己的 `M199Power`**（`src/boards/199.cpp`：播种 DBZ 扩展寄存器 `EXPREGS` + 装 `0x8001` 写处理器）⇒ 一行式基类会让 `EXPREGS` 停在 0、`0xC000/0xE000` 指向 PRG bank 0 ⇒ **游戏跑错代码**。故 199 给一个 `on_power()` 覆写（`Mapper199Cart::on_power() → M199Power()`），并把 `M199Power` 从 `static` 放开 | `src/boards/mmc3_variants_carts.h`（类声明）、`src/boards/mmc3.cpp`（定义 + 注册）、`src/boards/199.cpp`（`M199Power` 去 static） |
| B2 | 两条 `MapperEntryRegister`（191 "TW MMC3+VRAM Rev. A" 用 bmap 同名风格、199 "Waixing DBZ variant"，名字落地时对齐现有条目风格） | `src/boards/mmc3.cpp`（紧跟 198 的条目） |
| B3 | **锁测试**：`mapper_load_test.cpp` 增 `find_mapper(191)/find_mapper(199)` 非空断言（该测试已链 core 库，`boards/registry.h` 直接可用）；变异 = 摘掉一条注册 ⇒ 红 | `tests/f11qa/boards/mapper_load_test.cpp` |
| B4 | 端到端灰屏消验：用 issue 作者提供的真 ROM（业主已索取，**ROM 不入库**——收到后放桌面测试区，参照 `*.gba` 先例考虑给 `.gitignore` 加 `*.nes`）人工确认出画面；本项是**人工步骤**，不进门禁 | 本地 |

### S4 出口判据

- [x] **A1-A4 落地**（2026-11-10，r9）；issue 的复现 ROM 形态（合成 12+16+191）
      **装载成功**（`mapper_load_test` 实测 mapper 191），**画面可用待 B4 人工**；
- [x] **`cargo test -p fceux11-formats --lib` 全绿**（基线 **150 → 151 项**，2026-11-10 实测，
      新增 1 条 A5 锁）——
      ⚠️ **该命令在 S4 之前不存在于任何门禁**（纪律 10 活例），AGENTS.md 门禁节已同步登记，
      以后任何 formats 改动都要跑它；
- [x] **A5/A6/B3 三条锁变异双向可红**（还原腿刷 mtime）——逐条读数见 §九 r9；
- [x] `mapper_load_test`（kgmqa-016）扩展后仍绿；**NES 零回归加倍适用**：
      `ctest -LE perf` **36/36** 全绿 + 金标三件套零漂移
      （**F11QA 矩阵 `pass_to_fail=0` / grade ≥ B 待在 push 后由 CI 复核**——本地无矩阵入口）；
- [x] **GBA 门禁不受伤**（S4 文件域与 GBA 零交集，GBA 计数**逐字不变**：根 **266/24**、
      vendor **242**、debug **93**、media **52**，2026-11-10 实测）；
- [x] **`fceux11_rust.h` / 符号面零变化**——S4-a **没有加字段**（`FceuInesCartResult` 布局逐字不变，
      生成头只在**未跟踪**的注释里补了 `prg_data` / `prg_size` 的语义说明；
      头文件本身不在 git 里 ⇒ 无提交面变化）；
- [x] 变更记 §九（r9 起）；**issue #12 回复待业主发**（回复里需附验证结果）。

---

## 四之三、S5 · ROM 修正表与 hash 作用域（r10 新立，**待编码**）

> **触发**：S4 的 B4 人工步骤（业主用 9 张 NES 卡带实测 v2.0.2_hotfix1）暴露——其中
> 吞食天地2 在**上游 FCEUX 2.6.6 / 2.7.0 能跑出游戏 LOGO**，在本仓全灰。逐环核实后
> 定位到**两个独立缺陷**，都在 S4 的下游（不是 S4 造成的），但**S4 让它们第一次可见**。

### S5.0 事实底座（2026-11-10 实测，全部 file:line 为实测）

| # | 事实 | 证据 |
|---|---|---|
| 1 | 本仓 hash 按 **raw PRG** 算 | `ines.rs:1016-1018`：`fceux11_rust_ines_compute_hash(prg_data, prg_raw_bytes, …)` |
| 2 | 上游 hash 按**舍入 + 0xFF 填充**的缓冲区算 | 上游 `ines.cpp:957/959`（2.6.6 与 fork 基线 `1c35642` 同）：`md5_update(&md5, ROM, ROM_size << 14)` / `CalcCRC32(0, ROM, ROM_size << 14)`；2.7.0 同构（`rom_size_bytes`） |
| 3 | 修正表**以该 CRC 为键** | `src/ines-correct.h`（251 条）→ `ines_data::ROM_CORRECTIONS`；查表在 `ines.rs:309` `ROM_CORRECTIONS.iter().find(\|e\| e.crc32 == crc32)` |
| 4 | 吞食天地2 的两种 CRC | raw `0x50D82653`（**查不到**） vs padded `0x28192599`（命中 **`{0x28192599, 198, -1}`**）⇒ **它实际是 mapper 198**（"TW MMC3+VRAM Rev. E"），不是头里写的 4 |
| 5 | 修正后的 mapper **从未写回** | `ines_load.cpp:100` 的 `ines_apply_corrections(const FceuInesCartResult&, …)` 内 `parseResult` 是**局部**；`:128` 的 `tofix & 1` 分支是**空注释**；`:160` 的局部 `MapperNo` 与 `iNESCart.mapper_number` 都不更新 ⇒ `iNES_Init(MapperNo)`（`:279`）与 `create_cart_for_mapper` 拿到的都是**未修正**的号 |
| 6 | 其余三类修正是写回了的 | `tofix & 2` → `Mirroring`；`tofix & 4` → `head.ROM_type |= 2`；`tofix & 8` → 释放 VROM ⇒ **只有 mapper 修正整体失效** |
| 7 | 影响面 | 表里 **251 条**；凡「头里的 mapper 与修正表不一致」的 ROM 都受影响（不限于非 2 幂 PRG —— 那类只是**必然**失配） |

### S5.1 改动清单

| # | 改动 | 位置 | 备注 |
|---|---|---|---|
| **A1** | hash 作用域改回「按**实际映射缓冲区**算」：raw PRG + `0xFF` 填充到 2 幂（CHR 同理），使 CRC32/MD5 与上游**逐位一致** ⇒ 修正表能命中。**同批**把 `layout.prg_size_bytes` 改成**恒为舍入尺寸**（上游的 `ROM_size = uppow2(not_round_size)`，其注释明说「PRGCartMapping wants ROM_size to be to the power of 2」）⇒ 映射尺寸与 hash 口径一致 | `src/rust/crates/fceux11-formats/src/ines.rs`（`fceux11_rust_ines_compute_hash` 增 `prg_padded_size` / `chr_padded_size` 两个参数 + 新增 `feed_padded` 辅助；`compute_layout` 的 `prg_size_bytes`；`fceux11_rust_ines_load` 的调用点） | ⚠️ **这会推翻 S4 的 A3 决策**（「hash 用 raw」）：A3 的依据「对既有 ROM hash 零变化」成立，但**漏了 hash 同时是修正表与 `SAVIE_WHITELIST` / `CheckBad` / `MasterRomInfo` 的查找键** ⇒ 需按本条重写 A3 的理由，**保留 A1/A2 的 raw 切片** |
| **A2** | 把修正后的 mapper **写回** `iNESCart.mapper_number` 与局部 `MapperNo`（`tofix & 1` 分支），并保证 `create_cart_for_mapper` 拿到的是修正后的号。规则抽成 `FCEU_EffectiveMapperNo()` 便于锁 | `src/ines_load.cpp`（`ines_apply_corrections` 增出参 `corrected_mapper`）+ `src/ines.h`（新增 `FCEU_EffectiveMapperNo`） | 镜像 / 电池 / 清 VROM 三类修正**不动** |
| **A3** | **mapper 198 必须跑自己的 power 函数**（上游 `info->Power = M195Power`：把 16 KB WRAM 映到 `$5000` 并装读写 handler）。本仓基类只跑 `GenMMC3Power()`（WRAM 在 `$6000`）⇒ 该板没有 `$5000` 的工作 RAM | `src/boards/mmc3.cpp`（`M195Power` 去 static + `Mapper198Cart::on_power()`）+ `src/boards/mmc3_variants_carts.h`（`Mapper198Cart` 加 override） | 与 S4 的 B1′（199 → `M199Power`）**同一模式**；其余 16 个同因变体**另立项** |
| **A4** | 锁（Rust 侧）：① 原 A5 锁的 hash 断言改为 padded 口径 + `assert_ne!(raw hash)`（把「不能算 raw」钉住）；② 新锁断言 padded CRC `0x28192599` 命中 `{…, 198, -1}`；③ 新锁断言修正写进 `parseResult.mapper_no` | `fceux11-formats` crate（基线 151 → **153**） | **变异验证**：hash 改回 raw ⇒ ①红；查表项删掉 ⇒ ②红 |
| **A5** | 锁（C++ 侧）：① mapper 198 的 `$5000` **写回读**（写 `0x5A` 再读回：跑 `M195Power` ⇒ `0x5A`；不跑 ⇒ 开放总线 `0x00`）；② 有效 mapper 规则（有修正用修正值、无修正保留头里的号） | `tests/f11qa/boards/mapper_load_test.cpp`（用仓库现成的 `fixtures/mapper_mapper198.nes`） | **变异验证**：A3 还原 ⇒ ①红；A2 写回去掉 ⇒ ②红 |

### S5 出口判据（2026-11-11 实测全勾）

- [x] 吞食天地2 在本仓**跑出与上游一致的游戏标题画面**（「吞食天地 三国外传 PUSH START BUTTON
      豪华中文版」）—— 本地取证夹具，**卡带不进仓库**；
- [x] 9 张卡的对照结果与上游 **2.6.6 / 2.7.0 一致**：8 张逐字一致，剩下 4 张
      （Q Boy / SD高达外传2 / SD高达外传3 两版）**双方都灰屏** ⇒ 上游级缺口；
- [x] `cargo test -p fceux11-formats --lib` **153 项全绿**；`ctest -LE perf` **36/36**；
- [x] 门禁全绿：GBA 根 **266/24**、vendor **242**、debug **93**、media **52**、formats **153**；
- [x] A4 / A5 四条锁**可观测**（各自钉住一条可区分的差异，不是同义反复）；
- [x] S4 的 A3 决策理由在计划与 CHANGELOG 中**同步更正**；
- [x] GBA 三条门禁**逐字不变**（根 266/24、vendor 242、debug 93、media 52）⇒ S5 未碰 GBA 线。

---

## 五、阶段编排与依赖

```
S1 可复现门禁（夹具 + 探针改造）        ← 必须先做，否则后面不可证伪
 ├─ S1-a 最小夹具（r15 后 12 条）
 ├─ S1-b 探针改造（SWI 陷阱 / panic 捕获 / 输入注入 / 长窗口 / 忙等开窗）
 └─ S1-c 修 cartprobe 的 verdict 判词
        │
        ▼
S2 止血（panic 全改容错 + SWI 兜底）    ← 让机器还能跑，才谈得上定位
 ├─ S2-a vendor 核 panic → 开放总线
 ├─ S2-b SWI 兜底（改 bios.rs:134 + 两条锁测试）
 └─ S2-c C++ 侧不崩
        │
        ▼
S3 修根因（可多路并行，文件域互斥）
 ├─ S3-a SWI 0x16-0x2A      → swi/mod.rs（独占）
 ├─ S3-b 存档芯片模型        → internal_memory.rs（独占）
 ├─ S3-c GPIO/RTC + 取证     → internal_memory.rs + frame.rs（**与 S3-b 同文件，须串行**）
 ├─ S3-d ROM 窗口镜像        → internal_memory.rs（**与 S3-b/S3-c 同文件，须串行**）
 └─ S3-e 显示侧小修          → lcd/（独占）
```

**并行切分按文件域**：`swi/` 一路、`lcd/` 一路、`internal_memory.rs` 一路（内部再串行）。
**探针与夹具（S1）独立成路**，可与 S2/S3 的准备并行。
**S4（FC 侧，r8 扩围）独立成路**——文件域（`fceux11-formats` / `ines_load.cpp` /
`boards/` / `tests/f11qa`）与 GBA 线**零交集**，可与 S2-b/S3 任意并行或穿插；唯一共享资源
是构建与门禁（不与构建/测试并行跑的老规矩不变）。

> **✅ r17 实际编排（存档备查）**：本轮按此图双路并行——internal_memory 路（S3-b +
> S3-d 重划，真树）与 swi+lcd 路（S3-a 第二批 + S3-e，隔离副本 `FCEUX11_w23`，
> junction 指回 C++ 树喂 decoupling 守卫）文件域互斥、变异验证各自闭环，合并后
> 全量门禁复跑。S3-c 无代码改动（取证结案）。

---

## 六、风险与红线

1. **NES 零回归**（不变式 1）：GBA 改动后必跑 `ctest -LE perf` + F11QA 矩阵。
2. **「把 panic 改成容错」会掩盖真实缺陷**：每个容错点必须**留一个计数/日志**，
   否则「不再闪退」可能只是「缺陷换了个地方」。⇒ S2 出口要求读数里能看到容错计数。
3. **兜底 SWI 会掩盖「未实现」**：S2-b 的兜底必须让未实现号码**可见**（日志 + 计数），
   否则 S3-a 就没有「还差哪些」的清单。
4. **改锁测试的红线**：`bios.rs:396-401` / `bios.rs:218-223` 断言的是**当前行为**；
   改它们时必须在 commit message 里写清「为什么原来断言的行为是缺陷」，
   且**必须先有一个会红的夹具**证明新行为是对的。
5. **vendor 树**：本轮要改 `gba-core` 若干处 ⇒ ATTRIBUTION.md 逐处登记；
   记不清的改动**不做**（宁可不修，不可乱改 vendor）。
6. **外部参考只读不抄**（mGBA = MPL-2.0，与本项目许可不相容）。
7. **GBATEK 取证**：位级格式一律查快照，不凭记忆（本轮已两次翻车）。
8. **GBATEK 快照不进仓库**（5.4 MB）⇒ 只把**关键条目摘录**写进 §附录 A，
   并给一条**可重跑的抓取命令**。

---

## 七、整包出口判据

> **✅ r17 状态更新**（机器可达项已勾；**剩余三项全部是业主动作**：push 后的 CI
> 复核、issue 回复、观感人工——本地无法推进，如实保留未勾）：

- [x] `ctest -LE perf` **36 项全绿**（r17 终验）；F11QA 矩阵 **`pass_to_fail = 0`**、
      **grade ≥ B** ⬜ —— **待 push 后由 CI 复核**（本地无矩阵入口）
- [x] GBA 锁测试 **≥ 262 项全绿**（r17 后 **299 passed / 0 failed / 20 ignored**，
      其中 19 提交口径 = 测量工具；vendor **249** / debug **93** / media **52** 全绿）
- [x] **夹具逐条绿**（r17 后 **14 条**全为常驻绿锁，`REPRODUCERS = []`；每条变异
      验证双向可红的纪律保持——S3-b 路 13 腿 + S3-a/S3-e 路 15 腿实测）
- [x] **4 张失败卡带本地机器判据复核**（r16/r17 读数见 §四 S3 出口表；
      ⚠️ FireRed 判据按证据改写为「根因定位 + 转长尾」，显式记录于该表）；
      **卡带不进仓库**，仓库内判据一律是夹具
- [x] `cartprobe` 的 verdict 判词**已修正**，不再输出误导性表述（r6；r16 探针
      进一步以 `read_raw` 观测避免加速时钟）
- [x] vendor 树改动**全部登记进 `ATTRIBUTION.md`**（§3.2.13–§3.2.17，
      **28 → 33 处、6 → 7 个源文件**）
- [x] **S4（FC 侧，r8 扩围）**：loader 修复 + 191/199 注册落地，S4 出口判据（§四之二）全勾
      （r9–r11）；issue #12 报告者验证闭环 ⬜ —— **回复待业主发**
- [ ] 已知限制**如实写进发版说明**：本轮只解决「能跑」，**不声称 GBA 精度已达标**；
      未实现的 SWI（声音家族 12 号 + `0x2A`，r16 判定依据）、容错点、未复现的症状
      逐条列出 ⬜ —— **随下次发版执行**
- [ ] ⚠️ **仍需人工**：卡带的**最终画面观感**（有没有音乐、动画对不对）——
      机器判据只能证明「不崩、不卡、能出画面」，**观感仍要人看**；
      FireRed 的机器判据「出画面」已由 r17 的 `0x01` 强制空白达成（r19 双向定位，
      见 §四 S3 出口表），竞态本身的长尾条目保留

---

## 八、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（NES 零回归）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# GBA 锁测试（含新增夹具）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib

# FC loader/formats crate（r8 登记的新门禁——S4 之前无任何门禁跑它；基线 150 项）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-formats --lib

# 夹具生成（新增后）
python tests\fixtures\generate_test_roms.py

# 本地取证：4 张失败卡带（**不进仓库**，只在本地跑）
$env:FCEUX11_GBA_PROBE_DIR = "C:\Users\ikrx2\Desktop\v2.0.3实测依据"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --release --no-default-features --features gba --lib -- --ignored --nocapture cartprobe"

# 单卡定点取证
$env:GBA_PROBE_ROM = "C:\Users\ikrx2\Desktop\v2.0.3实测依据\Pokemon - FireRed Version (USA, Europe) (Rev 1).gba"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --release --no-default-features --features gba --lib -- --ignored --nocapture probe_why_a_game_stalls"

# GBATEK 取证快照（重新抓，需代理）
curl.exe -x http://127.0.0.1:7897 -o .zcode\gbatek_probe.html https://mgba-emu.github.io/gbatek/
```

---

## 九、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-08 | **初稿**。业主提供 7 张真实商用手写卡带 + 实测报告（真实性背书）。主会话以 4 个并行只读 agent + 自跑探针完成取证：**① 读出 ODT 报告全文（4 个失败症状 + 截图位置）；② 解析 7 个 ROM 头（全部合法）与存档 ID 串；③ 实跑仓库自带 `cartprobe`（7 卡 × 240 帧窗口）；④ 实跑 `probe_why_a_game_stalls` + `probe_trace_a_stuck_game` 定位 FireRed 卡死指令与寄存器；⑤ 抓 GBATEK 权威快照核对位级格式（**本轮据此否证了自己两条记忆**）；⑥ 代码审计：SWI 覆盖、存档芯片模型、显示管线、panic 面。**结论：三条独立根因（未实现 SWI→`b .` 自旋 / vendor 核 panic + `panic="abort"` ⇒ 闪退 / 存档芯片模型不完整）+ 一条次要缺陷（ROM 窗口无镜像）+ 一条显示侧小修。** 业主拍板范围：**三段式**（S1 可复现门禁 → S2 止血 → S3 修根因）+ **卡带不进仓库、判据换成自建最小夹具**。本条为计划立项，未动任何生产代码 |
| r2 | 2026-11-09 | **S1-a 开工前的只读复核 —— 计划的事实底座有 6 处需要更正，夹具清单与出口判据随之改写。未动任何生产代码。** **① §1.2.2 的「可触达 panic 点」清单收窄**：行号全部仍准确，但 **13 条里 11 条不可达** —— `bus.rs` 的 11 处 `panic!` 是各 `*_raw` 的兜底分支，而 `read_raw`(`bus.rs:1077`)/`write_raw`(`bus.rs:1152`) 的 match 对各自 I/O 区间**已全覆盖**（「写未建模 IO」今天 = `bus.rs:1203-1211` 的**静默忽略 + debug 日志**，零 panic）；`operations.rs:896` 的 PLD `todo!` 是**死代码**（`arm7tdmi.rs:458-463` 先拦 `Pld` 抛 UndefinedInstruction，`psr.rs:74` 的 `NV => false` 再挡一道）；`internal_memory.rs:660` 同样不可达（`read_at`/`write_at` 的**全部**调用方只有 `bus.rs:1088/1094/1157` 三处，缺口区间恰由 bus 自己接管）。**② OAM 的真现场是两条不是一条**：gfx-mode=3 的 panic 现场是 **`object_attributes.rs:144` 的 `unreachable!()`**，不是 `262`（`262` 的 `.unwrap()` 只在值=4 时才响，而该字段是 2 bit ⇒ 死分支）；shape=3 才是 `265`。另更正「真机禁用值」的表述：真机禁用精灵是 **obj mode=2**（vendor 已正确处理），**shape=3 / gfx mode=3 是「禁止值」**，真机当作精灵不可见、vendor 是崩溃 ⇒ **S2-a 的修复语义应是「映射为精灵禁用」**。⇒ **可达 panic 面收窄为 OAM 两条**（`:265` shape=3、`:144` gfx=3）⇒ §1.2.2 加「更正」小节。**结论方向不变**（闪退仍来自 vendor 核 panic + `panic="abort"`），**变的是证据清单** ⇒ S2-a 的表须按对照重读。**③ §1.2.4 的「EEPROM 没有真模型」不准确**：位级模型**存在**（`internal_memory.rs:523-611` 的 `is_eeprom_access`/`eeprom_write`/`eeprom_read`/`eeprom_interpret_command`），真正缺的是三条 —— **E1 访问窗口过宽 16 MB**（`is_eeprom_access` 对 ≤16 MB 的卡返回 `0x0D00_0000–0x0DFF_FFFF`，而真机 EEPROM 只有 64 KB ⇒ `bus.rs:1638/1674` 会把 **`0x0D01_0000–0x0DFF_FFFF` 的半字读写劫持进 EEPROM 位流**，那是卡的 ROM 区 ⇒ **EEPROM 卡读自己的 ROM 会拿到位流垃圾**，比原表的「窗口宽度错」更严重）、**E2 芯片容量不建模**（`.srm` 恒 32768）、**E3 Flash 状态寄存器不存在**。**④ 业主拍板 S1-a 的三个决策**：**D1 载体 = Rust 内建**（`src/rust/src/gba/fixtures.rs` 现场拼 ROM 镜像，**不产生二进制入库**；`.gitignore` **只加一条 `*.gba`，不加例外** —— 例外规则是它的漏洞；理由：纪律 5「按字段拼装」在二进制 blob 上做不到 + 门禁链路上**没有 cargo 环节**，「测试依赖仓库里一个二进制文件」是无监控的依赖 + ARM 编码器只能有一份实现）；**D2 门禁口径 = 复现器标 `#[ignore]` + 3 条常驻守卫**（消解「§〇 说会红」与「§2.4 说全绿」的矛盾；**断言写目标行为，不写缺陷行为** —— 写成缺陷行为就是把缺陷锁成预期行为，与 §1.2.1 自己对 `bios.rs:396-401` 的批评冲突）；**D3 夹具条数 = 9 条**（删不可达的 `gba_io_unmapped.gba`，EEPROM 拆协议/窗口/尺寸三条）⇒ **§2.2 与 §2.4 相应改写**（出口计数 **249 → 269 全绿** + **9 条 `#[ignore]` 复现器**，且复现器在 `--ignored` 下必须逐条为红）。**⑤ 新增前置条件**：GBATEK 快照须补齐 **EEPROM 命令协议** 与 **GPIO/RTC** 两章（附录 A 当前只有三节）⇒ 夹具 5 与夹具 10 的协议级断言**取不到权威依据就不许写**。**⑥ 登记不修的观察项**：vendor 的 `OperandKind` 判别与真 ARM 相反（`cpu/arm/instructions.rs:796`）⇒ `bios.rs:78` 的 `ADD_LR_PC` 在真 ARM 上是 `ADD lr, pc, r0`；vendor 的 `SingleDataTransfer` 立即数偏移**不乘 4**。实施计划见 `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md`（Step 0..7） |
| r3 | 2026-11-09 | **GBATEK 逐位核实 ⇒ 撤回 r2 的一条结论，并据此改写夹具清单与出口计数。仍只动文档，未动任何生产代码。** ① **撤回「E1 · 访问窗口过宽 16 MB」**：GBATEK 快照（**本来就在本地**：`.zcode/gbatek_plain.txt`，4.0 MB，2026-11-08 抓取）`:8692-8694` 明文「**On carts with 16MB or smaller ROM, eeprom can be alternately accessed anywhere at D000000h-DFFFFFFh**」⇒ `internal_memory.rs:523-532` 的 `is_eeprom_access()` **与 GBATEK 一致**，**不是缺陷**。② **EEPROM 的命令协议 vendor 本来就是对的**：GBATEK `:8710-8786` 与 `internal_memory.rs:566-614` 逐位一致（请求位 `"11"`/`"10"`、6 或 14 位地址、64 位数据、读回 68 位 = 4 忽略 + 64 数据、MSB first、Ready 位、以 64 位为单位；写分支宽度判据 `buffer.len() >= 81` 也对）⇒ **存档芯片侧真正缺的只有 E2（型号丢失 ⇒ `.srm` 恒 32768）与 E3（Flash 状态寄存器不存在）**。③ **GPIO 侧取证只到位一半**：GBATEK 有 GPIO/RTC 章节（`:7984`/`:9189`/`:9287-9353`，含 Boktai 的引脚表），但**没有**定义「游戏尚未写入时读 `0x080000C4` 读到什么」⇒ 按纪律 4 **不许硬写断言** ⇒ 该条**降级为绿色回归锁**。⇒ **夹具清单：9 条 → 8 条**（**7 条会红** + **2 条绿锁**），**D3（业主原裁决「9 条」）相应改为 8 条**；⇒ **出口判据：常驻 249 → 262 全绿**（+8 G3 编码器自检 +3 守卫 +2 绿锁），**`-- --ignored` 另有 7 条复现器**（必须逐条为红），**S3 全修完 269 项全绿**。④ **教训**：本条与 r1 的「DMA 起始时序」「FLASH1M 容量」是**同一类错误**（凭记忆写位级/协议），本项目**已犯第四次** ⇒ **位级与协议一律查快照原文**。⑤ **Step 0 取证已完成**（快照在本地，不必联网），只剩「把 EEPROM / GPIO-RTC 两章的关键条目摘录写进附录 A + 加一条可重跑的抓取脚本」。 |
| r4 | 2026-11-09 | **S1-a 的 Step 1（G3 编码器自检）已过变异验收 —— 测出 3 条盲区并修掉；8/8 的目标都能红，但 §2.4 的「只改一个字段 ⇒ 其它 7 条仍然绿」有 2 行结构性达成不了 ⇒ 需裁决。** ① **做法**：8 个编码器各一处**单字段**变异，逐次跑 8 条 G3 自检记逐条红绿（执行器 `.zcode/mutation_matrix.ps1`，不入库）。② **首轮测出 3 条盲区**（目标测试在自己的编码器被改坏时**照样绿** ⇒ 无牙门禁）：`g3_orr_imm`（立即数不需要旋转 ⇒「忘记旋转」不可观测）、`g3_strh`（存的值**高半字为 0** ⇒ `STR` 与 `STRH` 写出的字节完全相同）、`g3_ldrh`（源字高半字为 0 + 内存往返抹掉差异）⇒ **已修**：改用需要旋转的立即数 / **预置哨兵 `0xDEAD_BEEF`** / 源字改 `0xC5C5C500`，并把断言从「内存往返」改为**读寄存器**（否则自检会依赖别的编码器，矩阵无法定位）。③ **修后矩阵**：**8/8 的目标都红**；**6/8 行只红目标**（M2/M3/M4/M5/M6/M8）；**M1（`mov_imm`）红 5 条 = 恰好是「用 `mov_imm` 造地址」的自检集合**（1/3/4/5/6）—— 它是**唯一**能造地址的手段 ⇒ 结构性共因；**M7（`swi`）红 8 条** —— `swi(HALT)` 是**每条自检的终止器** ⇒ 结构性共因（一个坏 SWI 编码器 = 全局失败，**响亮地失败是对的**，代价是该行无法定位）。⇒ **要消掉这两行需改夹具脚手架**（基址寄存器预置 + 终止器改用 `gate.rs` 参考实现）⇒ **设计变更，超出 S1-a 授权 ⇒ 交业主裁决**；⚠️ 附带风险：改用「跑到预算耗尽」当终止器会**假绿**（程序没跑也读到预置值）⇒ 终止器必须可判定。④ **顺带登记一个坑**：变异验证的**还原腿会假红** —— 还原源文件不更新 mtime ⇒ cargo 判定「无需重建」⇒ 跑的是上一轮变异版的二进制 ⇒ **反向腿必须确认二进制是新的**。⑤ 门禁：根 crate **257**、vendor **242**、debug **93**、media **52**，全绿；**未动任何生产代码**。逐格读数见 `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §十一 r4 与 §七 Step 1 验收记录 |
| r5 | 2026-11-09 | **S1-a 完成 Step 0 余账 + Step 2/3/4/5/6 —— 9 条夹具与 3 条守卫落地，7 条复现器逐条为红，门禁命中 262；同时更正三处计划口径。** ① **Step 0 余账销清**：附录 A 补 **A.4（EEPROM 命令协议 `:8710-8786`）/ A.5（EEPROM 寻址与容量 `:8661-8706`）**（照抄原文、带行号）；新增 `tools/fetch_gbatek.ps1`（抓 HTML → 转纯文本，**逐字节复现**现有快照 ⇒ 附录 A 的行号引用在重抓后仍成立；错误路径实测：代理不可达 / HTTP 404 / 超时均 exit 3）。⚠️ **未销清**：§一.5 要求的 **GPIO/RTC 两章摘录（A.6）仍缺** ⇒ 该行动项未完全收口。② **Step 2**：9 条夹具按**缺陷域**拆成 `src/rust/src/gba/fixtures/{swi,oam,backup,rom}.rs`（公共底座留在 `fixtures.rs`）⇒ 文件域互斥、可并行；**7 条会红 + 2 条绿锁**，每条做过**双向变异验证**（变异腿 + 还原腿，还原后刷新 mtime）。③ **Step 3**：3 条常驻守卫 `fixtures/guards.rs`（G1 孤儿 `#[ignore]` / G2 夹具集合完整性 / G4 复现器存在性与归属）。④ **Step 4**：`.gitignore` 加 `*.gba`（**无例外**）⇒ 商业卡带机械地进不了仓库。⑤ **读数**：根 crate **262 passed / 26 ignored**（= 249 + 8 G3 + 2 绿锁 + 3 守卫）；**7 条复现器逐条为红**（`0 passed; 7 failed`）；`gba-core` **242** / `debug` **93** / `media` **52** 全绿；`ctest -LE perf` **36/36**。⚠️ **复现器必须带夹具域过滤**（`… --lib gba::fixtures:: -- --ignored`），不带过滤会拉进 19 个需要真卡带的测量工具。⑥ **三处口径更正（由本轮取证得出）**：**(a) §四 S3-b 第 2 条「`0x0E000006/7` 按 GBATEK 的状态位定义返回」不成立** —— GBATEK 快照全文 grep `E000006` **零命中**，GBATEK 的 flash 轮询是**回读比较**（`:8981` "wait until [E00xxxxh]=dat"）⇒ **GBATEK 没有 flash 状态寄存器**；S3-b 开工前须另找权威来源或用真卡带重新推导（现有夹具断言的是**本计划**给的 `0x0E000006/7` + ready=bit0）。**(b) §1.2.1 的兜底配方要改**：vendor 的 LR 约定与真机差 4 ⇒ SWI 向量若照抄 IRQ 向量的 `SUBS PC, LR, #4` 会**退回 SWI 自身重入**，正确的向量字是 **`MOVS PC, LR`**。**(c) 0x16 的命名三处不同**（GBATEK `Diff8bitUnFilterWrite8bit` / 仓库 `Diff8BitUnFilterWram` / 计划 `Diff8bitUnFilterWram`），号码相同。⑦ **计数口径**：夹具是 **9 条**（7 红 + 2 绿），**不是**「8 条（7 会红 + 2 绿锁）」⇒ 7 + 2 = 9；**门禁计数 262 不受影响**（绿锁只算 2 条常驻）。⑧ **两个坑**：① 变异验证的**还原腿会假红**（还原源文件不更新 mtime ⇒ cargo 跳过重建 ⇒ 跑的是上一轮变异版的二进制）；② **多个 worker 并发改同一个 vendor 文件会互撞** ⇒ 共享 vendor 树的变异验证必须串行或用隔离副本。⑨ **未动任何生产代码**（vendor 树零改动，逐文件哈希核对）。 |
| r6 | 2026-11-10 | **S1-b 探针改造完成——§2.3 六项全部落地，新增 2 条常驻锁（根 crate 262 → 264 / 26 ignored），复现器 7 条仍逐条红；三个实现层新事实（工具链 ×2、并发 ×1），其中一个是全 suite 挂起级死锁的根因。改动只落在 `cartprobe.rs` / `frame.rs` / `swi/mod.rs` 三个文件；fixtures/ 与 vendor 树零改动。** ① **§2.3 六项**：(a) cartprobe verdict **通道化**——`describe_timing(channel, timing)`：DMA1/2 时序 3 标「sound FIFO，核心已服务」、DMA3 标「video capture —— never fired」、DMA0 标 prohibited；verdict 行按通道分别判，逐通道「STILL ENABLED」读数保留；(b) `swi/mod.rs` `dispatch` 拆为薄壳 + `dispatch_inner`，壳记录 `(号码, 是否认领)` 进 **`#[cfg(test)]`** thread-local 计数表（`take_swi_tallies` 排空 + `swi_name` / `vendor_core_serves` 判读辅助；vendor 自答集 {0x00,0x06–0x08,0x0B,0x0C}）——**release 生产零开销**（C3 同口径）；cartprobe 每卡打印「踩了哪些号码、谁服务、第一个未实现号之后 CPU 停在 0x08 所以之后的 SWI 不可能发生」；(c) `gba_step_frame` 外层 `catch_unwind` ⇒ 命中返回 **GBA_ERR_STATE**（**不新增错误码**——新码要动 build.rs，超出授权；语义对：机器已不可信），读数经 panic hook 记录；(d) 输入注入 `GBA_PROBE_INPUT="帧:键;…"`（1 基于测量窗、全状态语义、解析错即 panic 不静默）+ `GBA_PROBE_ONLY` 文件名过滤；(e) 窗口可配 `GBA_PROBE_FRAMES`/`GBA_PROBE_WARMUP`（默认 180/60 = 原 240）与 `GBA_PROBE_STUCK_FRAMES`（默认 30），verdict 新增「PC 连续 N 帧不变」读数（逐帧 modal PC 比对，报**终局**连击、不报历史最大）；(f) `probe_what_one_frame_changes` **忙等开窗**——halt 等不到时降级为 PC 采样稳定判定 + 按指令数开窗（FireRed 忙等从此可开窗）。② **两条常驻锁（264 = 262 + 2）**：`special_timing_wording_is_channel_aware`（cartprobe，钉 §1.3 判词措辞）与 `a_core_panic_in_step_frame_is_caught_and_readable`（frame.rs，断言**完整恢复周期**：返回 STATE / 读数含 "Prohibited ObjShape" / 下一帧 NO_ROM / **毒锁下重载成功且新机器跑通** / unload）。触发程序用 fixtures **公开**积木复刻 oam 夹具（attr0 = 0xC001 引自 `fixtures/oam.rs:109-110`；fixtures/ 未动）。③ **变异三腿双向可红**：腿 1 describe_timing 改回通道盲 ⇒ Lock A 红；腿 2 catch 改 `resume_unwind` ⇒ **0xC0000409 fail-fast**（panic 穿越 `extern "C"` 帧被判 foreign unwind 直接 abort——**生产「闪退」机制的活体演示**，AGENTS.md 纪律 6 的实证）；腿 3 摘掉 catch 处理器的机器丢弃 ⇒ 「torn machine must be gone, got 5」红。三腿还原后**刷 mtime** 复绿。④ **工具链事实 ×2（实测，有读数）**：(a) **`cargo test --release` 仍是 unwind**——`[profile.release] panic="abort"` 对测试目标不生效（仓库外微型实验实证）⇒ 探针/锁在 `--release` 取证跑法下 catch_unwind 与 panic hook **都工作**；⇒ **fixtures/oam.rs 注释「release 下 catch_unwind 无法挽救」被否证**（该文件本轮不动，仅在此记录；也意味着 OAM 复现器将来可在 --release 下跑）。(b) **格式化 panic 的 payload 拿不到字符串**——rustc 1.96 上 `Result::unwrap` 对 `Err(&'static str)` 的 panic，`is::<&str>()` 与 `is::<String>()` **双双为 false**（type_id 诊断实测）⇒ 读数必须走 **panic hook** 的 `PanicHookInfo::payload_as_str()`（fixtures/oam.rs 的 `payload_message` 有同一盲区，其 message 恒为 "<non-string panic payload>"、靠 location 工作——该文件本轮不动）。⑤ **并发事实 ×1（挂起级）**：**match scrutinee 自死锁**——`match m.lock() { Err(_) => m.lock() }` 里 scrutinee 临时 `Result`（**内含已持有的 guard**）存活到整个 match 结束 ⇒ Err 臂内对同一互斥量再 lock = 自己等自己。初版 with_machine 的毒锁恢复正中此坑：Lock B 第二次 step_frame 挂死 ⇒ `exclusively` 的 TEST_LOCK 永久被占 ⇒ 全套件其后 12+ 个 exclusively 测试排队挂起（首次全量门禁 60+ 秒无响应的根因）；这也解释了**旧代码 `try_lock` 恢复为何是死代码**（try_lock 同时被「毒」与「scrutinee 自持」双重挡死，r5 之前从未有人踩到）。修复 = **毒锁透明恢复**（`into_inner` 无条件恢复并使用；毒对进程终身有效，std 无 unpoison），撕毁态的丢弃收敛到 catch 处理器这个**唯一捕获点**；`set_machine` / `set_machine_unload` 同步恢复（否则毒锁后机器永久无法重载——初版 `if let Ok(...)` 在毒锁下静默不做事）。⑥ **门禁读数**：根 crate **264 passed / 26 ignored**（9.7 s）；复现器（`gba::fixtures::` 域过滤 + `--ignored`）**0 passed / 7 failed**；vendor **242** / debug **93** / media **52** 全绿；`ctest -LE perf` **36/36**（Release 全链重链，`-j 1`，BUILD_EXIT=0、零 LNK1104）。⑦ **卡带实测（本地取证，不进仓库）**：**7 张卡基线（默认 240 帧窗口）零 panic、零 NOT-SERVED SWI**——未实现 SWI 与 OAM panic 在开机窗口内都不触发，与「4 张失败卡的故障点在更深处」一致；判词无误导措辞（Dragon Ball 的 DMA1/2 FIFO 行正确标注 serviced）。画像：Dragon Ball/Minish Cap/Mario Kart 各自稳定在片头循环 PC（`0x08115ad0` 连击 135 帧 / `0x080b1500` 连击 180 帧 / `0x0806138c` 连击 176 帧）= **都在等输入**；FireRed「picture FROZEN」复现 §1.2.3 忙等画像；Kirby/NFS/SMA2 正常跑。**Dragon Ball 注入两轮（Start/A，300 帧 / 600 帧连打）**：**游戏对输入有响应**（每次有效按键后 30 帧内画面变化，600 帧 185 个不同帧哈希、预算零耗尽、零 panic、零未实现 SWI）——**「按键后卡死」在本时程/键位下未复现**；差异留给 S3-b/S3-c 用同一工具追（首要嫌疑：菜单后的存档通路 = §1.2.4 E2/E3 的 EEPROM_V124）。⑧ **生产 release 行为不变的边界**：`panic="abort"` 下 catch_unwind 不可能返回（abort 前原 diag hook 照记），本轮 catch/恢复只在 unwind 可用的构建（cargo test 全系、debug）改变行为——「panic 带走进程」变「错误码 + stderr 读数」。⑨ **bat 编码坑（补录）**：`.bat` 里写中文路径（`v2.0.3实测依据`）会被 cmd 按 OEM 码页读成乱码 ⇒ 探针目录/过滤/注入等含非 ASCII 的环境变量一律**从 bash 侧传入**（Git Bash 正确转 UTF-16），bat 内不放 `set` 行。AGENTS.md 已同步（v2.0.3 进度块 + 计数 + scrutinee 自死锁登记）。 |
| r7 | 2026-11-10 | **S2-a 落地——vendor 核的两个可达 panic 点改为「精灵禁用」，商业卡的「闪退」根因（§1.2.2）消除；OAM 两条复现器转常驻绿锁。** ① **范围按 r2 收窄（业主裁决）**：只修 `object_attributes.rs` 的两条可达 panic——attr0 **shape = 3**（原 `:265` `.unwrap()`）与 **gfx mode = 3**（原 `:144` `unreachable!()`）；§3.1 表其余条目（bus.rs / internal_memory.rs / PLD）r2 已证不可达，**不动**——为不可达路径改容错无法用变异证明价值，只会膨胀 vendor 补丁面。② **修法**：`GfxMode::try_from` 对禁止值返回 `Err("Prohibited GfxMode")`（`4 => Err` 死分支并入）；`ObjAttribute0::try_from` 对 shape/gfx 的 `Err` 映射到 vendor 既有的 **`ObjMode::Disabled`** 表示（shape 占位 `Square`）——`process_sprites_scanline` 与 `process_winobj_sprites_scanline` 本来就 `continue` 跳过它（`layer_obj.rs:234-236/520-525`），可观测行为与真机完全一致（精灵不显示、帧继续）；字段零新增 ⇒ serde 无影响；真内部不变量（字段宽度不可能的值）保留 panic（§3.1 纪律）。③ **登记**：ATTRIBUTION **§3.2.12**（#30，vendor 补丁 **27 → 28 处、5 → 6 个源文件**，`object_attributes.rs` 入列）；AGENTS.md vendor 表加一行。④ **复现器转常驻**：`oam_bad_shape` / `oam_gfx_mode3` 摘 `#[ignore]`（断言本来就是目标行为，`assert_treated_as_disabled` 的双对照设计——显式禁用对照必须不可见 + 合法对照必须画出——修复后直接成立）；guards 的 `REPRODUCERS` **7 → 5**（G1/G2/G4 逻辑不动）；oam.rs 文档补记。⑤ **Lock B 处置（业主裁决：注入式机制锁）**：S2-a 修掉触发器后原锁必然转红 ⇒ 重构为 `a_frame_panic_is_caught_and_the_machine_recovers`——`#[cfg(test)]` thread-local `INJECT_FRAME_PANIC` 注入标志，在 `gba_step_frame` 的 machine body **内部**（vendor panic 同一条 unwind 路径、机器锁同样在持）触发 `panic!("injected frame panic")`，非测试构建编译期消失、release 零痕迹；断言链不变（STATE / 读数 / NO_ROM / 毒锁下重载成功且新机器跑通）；「不再 panic」这半边由两条常驻 oam 锁负责——机制锁与行为锁各管一段，互不重复。⑥ **门禁读数**：根 crate **266 passed / 24 ignored**（264 + 2 转正 − 2 ignore）；复现器 `gba::fixtures::` 域过滤 `--ignored` **0 passed / 5 failed**（仍逐条红）；vendor **242**（上游测试无扰）/ debug **93** / media **52** 全绿；`ctest -LE perf` **36/36**（BUILD_EXIT=0）。⑦ **变异两腿双向可红（单字段、单锁定位）**：腿 1 shape 的 Err 映射改回 `panic!` ⇒ `oam_bad_shape` 红（panic 被 `catch_panic_reading` 转成 reading，不炸进程）；腿 2 gfx 的 Err 映射改成「只回 Normal、不置 Disabled」⇒ `oam_gfx_mode3` 红（双对照逮住「画出来了」）；两腿还原后刷 mtime 复绿。⑧ **卡带复跑**：7 张基线（release，240 帧）verdict 与 r6 完全一致（6 changing + FireRed FROZEN 忙等画像）、**零 panic、零未实现 SWI**——S2-a 在本窗口无行为回归；其对商业卡的真实收益由两条常驻 oam 锁背书（窗口内本就无 panic 触发，观测面不变）。⑨ 文件面：`object_attributes.rs` / `ATTRIBUTION.md` / `fixtures/oam.rs` / `fixtures/guards.rs` / `frame.rs` + 计划与 AGENTS.md；**未碰** `bios.rs` / `swi/` / `gba_load.cpp`。下一步：S2-b（SWI 兜底）——先例预警见 §1.2.1（必改 `bios.rs:396-401` / `bios.rs:218-223` 两条锁）与 r5 ⑥(b)（向量字必须 `MOVS PC, LR`）。 |
| r8 | 2026-11-10 | **业主裁决扩围：GitHub issue #12 的两个 NES 侧缺陷并入本计划为 §四之二「S4 · FC 侧修复批次」——本轮只写文档、不动任何代码，下一轮编码作业按 S4 节直接开工。** ① **只读核实（当日完成，S4.0 的全部 file:line 为实测）**：两缺陷**均属实**——Bug 1（iNES loader 对非 2 幂 PRG 先舍入后切片 ⇒ 硬拒 + 潜在 CHR 错切）机制链逐环核实（`cart.rs:90` / `ines.rs:721-731` / `ines.rs:961-978` / `ines_load.cpp:148-149`），C++ 消费链预研发现 **`ines_load.cpp:171-186` 已是 rounded-padded 拷贝形态 ⇒ 修 Rust 切片后 C++ memcpy 必须同步改 raw 字节数，否则越界读 memstream 缓冲区**；Bug 2（mapper 191/199 未注册 → 灰屏）核实注册表 25 项确无 191/199、legacy Init 会跑但 `g_cart` 停在 `CartPlaceholder`、根因是 registry 迁移（目标 all 171）未完成、兄弟形态是一行式 Cart 子类。**两处与 issue 表述的偏差如实记录**（原版 round 分支 fread 也用舍入数靠不查短读存活，故「读 raw」是比原版更正确的修法；issue 清单少列 mapper 76）。② **S4 设计已定稿在 §四之二**：A1-A6（loader raw 切片 + hash 用 raw 的决策依据 + formats 锁 + ctest fixture 锁）、B1-B4（191/199 注册 + registry 断言锁 + 真 ROM 人工验灰屏）、**CHR 同构缺口登记为观察项不修**、**FFI 结构零变更**（`rom_size_raw` 已在 `FceuInesCartResult` 且 C++ 已在读）。③ **门禁登记（纪律 10 活例）**：`fceux11-formats` crate 此前**不在任何门禁命令里**——本轮实测 `cargo test -p fceux11-formats --lib` **150 项全绿**，命令登记进 AGENTS.md 门禁节与本计划 §八；`mapper_load_test`（kgmqa-016）确认可扩展（8 个合成 fixture 遍历装载）。④ **范围口径**：§〇 范围外第 1 条加 r8 注——GBA 文件域口径不变，FC 侧仅限 S4 列出的文件，NES 零回归门禁对 S4 **加倍适用**；S4 文件域与 GBA 线零交集（§五 编排已加独立支线），可与 S2-b/S3 任意并行。⑤ **验证依赖**：issue 作者的真 ROM（业主已邮件索取）——**ROM 不入库**，收到放桌面测试区，编码时按 `*.gba` 先例考虑给 `.gitignore` 加 `*.nes`。 |
| r9 | 2026-11-10 | **S4 落地（FC 侧修复批次全部编码完成）——issue #12 的两个缺陷都修好了；新增 1 条 Rust 锁 + 2 条 ctest 级断言，5 腿变异逐条双向可红。** ① **A1-A3（Rust 侧 loader）**：`fceux11_rust_ines_load` 的 PRG 切片 / 文件边界检查 / hash 全部改用 **raw** 字节数（`rom_size_raw << 14`；iNES 1.0 的「0 = 256 银行」编码走 `rom_size_16kb` 兜底）⇒ 12 银行 PRG 的卡不再被硬拒、CHR 不再被 PRG 吞掉、hash 覆盖的是 raw PRG。**分配与映射仍用舍入尺寸**（`SetupCartPRGMapping` 要 2 幂）。② **A4（C++ 侧 memcpy）**：拷贝长度改 raw，并**取 `min(raw, prg_size)`** —— 照 A4 字面写会踩两个坑（256 银行 ROM 拷 0 字节 / NOT_POWER2 mapper 越界写），见 §四之二 A4′。③ **A5**：formats crate 新增 1 条锁 `a_non_power_of_two_prg_rom_loads_with_raw_slices`（合成 12+16+191 的 327696 字节镜像；断言装载成功 / PRG 切片 / **CHR 起点在 raw PRG 末端而非舍入末端** / md5+crc32 = raw PRG+CHR）⇒ **基线 150 → 151**。④ **A6 + B3（ctest 级端到端）**：`generate_test_roms.py` 新增非 2 幂 PRG + mapper 191 的 fixture（`tests/fixtures/mapper_mmc3_191_nonpow2.nes`，327696 字节，与 issue 复现 ROM 同形）；`mapper_load_test.cpp` 增 ① registry 断言（`find_mapper(191/199)` 非空）② 表里增 191 一行（装载成功且 `mappernum==191`）③ **字节级断言**（`X6502_DMR(0x8000)`=0xEA 是文件 PRG；`0xC000/0xE000`=0xFF 是填充 ⇒ CHR 没被吞进 PRG）④ **199 的 power 断言**（`EXPREGS` 必须 = FE FF 01 03 ⇒ `M199Power` 真的跑了）。⑤ **B1/B2**：191/199 进 registry；**199 不能用一行式基类**（见 §四之二 B1′）——`M199Power` 从 `static` 放开、由 `Mapper199Cart::on_power()` 调用。⑥ **变异验证 5 腿逐条双向可红**（还原腿刷 mtime）：**M1** Rust 切片回合入 ⇒ A5 红（load 失败）；**M2** Rust hash 回合入 ⇒ A5 红（md5 不符）；**M3** C++ memcpy 回合入 ⇒ A6 红（`0xC000` 变 0xEA = CHR 被吞）；**M4** 摘掉 191 注册 ⇒ B3 红（`FAIL: mapper 191 has no registry entry`）+ A6 红；**M5** 199 的 power 换成 `GenMMC3Power()` ⇒ EXPREGS 断言红（`00 00 00 00`）。**每腿只红该腿该红的那一条**，还原后复绿。⑦ **门禁**：`ctest -LE perf` **36/36**；formats **151**；GBA 三条**逐字不变**（根 **266/24**、vendor **242**、debug **93**、media **52**）⇒ S4 未碰 GBA 线。⑧ **如实登记三条**：(a) **B4 人工步骤未做**（真 ROM 未到，出画面的观感仍待人工）；(b) **F11QA 矩阵待 push 后由 CI 复核**（本地只有 `ctest` 这个近似门禁）；(c) **新发现、本轮不修**：已注册的**带自定义 power 的 MMC3 变体**（12/37/44/45/47/49/52/114/115/165/195/205/245/249/250/254/406 等）**同样只跑 `GenMMC3Power()`、它们自己的 `MxxPower` 从不执行**（与 199 同因：v1.15 B.1 把 power 统一成了基类实现）⇒ **单独立项**，不在 S4 范围（宁窄勿宽）。⑨ 顺带更正三处计划口径：§2.4 的复现器条数（9 → 5）、§五 的夹具条数（8 → 9）、本节 A4 的字面写法（见 A4′）。 |

| r10 | 2026-11-10 | **S4 的 B4 人工步骤已做（业主用 9 张 NES 卡带实测 v2.0.2_hotfix1）+ 上游对照实验完成 ⇒ 定位到两个真实缺陷：hash 作用域与 ROM 修正表写回。** ① **版本口径更正**：本仓 fork 的上游基线是 **FCEUX 2.6.6**（`1c35642:src/version.h` = MAJOR 2 / MINOR 6 / PATCH 6），「FCEUX 1.15」是**本项目自己的版本线**——两者极易混；上游 GitHub 发布只回溯到 2.2.x、1.15 的二进制在 SourceForge（实测 404）⇒ 对照实验取 **2.6.6（精确基线）+ 2.7.0（最新）**。② **对照方法**：三个模拟器同一批 9 张卡、同参数（启动→`SetWindowPos(HWND_TOPMOST)`→等 6 s→抓模拟器窗口→1.5 s 后再抓一次），按**画面区**做 4bit 量化，取「不同颜色数 / 主色占比 / 主色 RGB / 两帧差异率」；⚠️ 本仓 exe **不吃命令行 ROM**（Qt 前端从不调 `Config::_parseArgs`，`_parseArgs` 只属于旧的 SDL 前端）⇒ 改走 GUI 自动化（Alt+F→O→**剪贴板粘贴**路径→Enter，中文路径无法用 SendKeys 直输），已验证真的装载（银白色沙加跑出标题画面）。③ **读数**：**上游 2.6.6 与 2.7.0 对 9 张卡的结论逐字相同**；其中 8 张与本仓一致，**1 张不同** —— 吞食天地2：**上游跑出游戏自己的「吞食天地」LOGO（能用），本仓全灰**。④ **根因（两处，均已实测闭环）**：**(a) hash 作用域** —— 本仓 `fceux11_rust_ines_compute_hash` 收的是 **raw PRG 字节数**（`ines.rs:1016-1018`），而上游 2.6.6 / 2.7.0 都按**舍入 + 0xFF 填充**后的 PRG 缓冲区算（`ines.cpp:957/959`：`md5_update(&md5, ROM, ROM_size << 14)`、`CalcCRC32(0, ROM, ROM_size << 14)`）；`ines-correct.h` 的修正表**以该 CRC 为键** ⇒ **非 2 幂 PRG 的 ROM 必然查不中**（吞食天地2：raw `0x50D82653` 查不到、padded `0x28192599` 命中 `{0x28192599, 198, -1}` —— **它实际是 mapper 198，不是头里写的 4**）。**(b) 修正写回缺失** —— `ines_load.cpp:100` 的 `ines_apply_corrections(const FceuInesCartResult&, …)` 把修正后的 mapper 写进**局部** `parseResult.mapper_no`，`tofix & 1` 分支是**空注释**（`:128`），从未写回 `iNESCart.mapper_number`（`:160` 的局部 `MapperNo` 也不更新）⇒ **整张 251 条修正表的 mapper 修正在本仓整体失效**（镜像 / 电池 / 清 VROM 三类修正是写回了的）。⑤ **与 S4 的关系**：**S4 没有失败** —— 它的两个修复都真实生效（registry 让 3 张 191/199 卡能出画面；loader 让非 2 幂 PRG 卡从「被拒」变成「能装载」），**是 S4 把非 2 幂 ROM 第一次放进流水线，从而暴露了下一层的两个缺陷**；⚠️ **S4 的 A3 决策「hash 用 raw」需要复议**（其依据「对既有 ROM hash 零变化」成立，但**漏了 hash 同时是修正表的查找键**）⇒ 与 §四之二 A3 直接冲突，必须按本条重写。⑥ **其余 8 张**：两种 CRC 均**不在**修正表 ⇒ 无修正可用，行为与上游一致（Q Boy / SD高达外传2 / SD高达外传3 两版 = 上游级缺口；大盗伍佑卫门外传 / 银白色沙加 = 双方都正常；拉迪亚战记 = 本仓比上游多一点活动但仍黑屏；SD高达外传1 = 双方都黑屏）。⑦ **下一步（新工作项 S5）**：① hash 改回「按实际映射缓冲区算」（raw PRG + 0xFF 填充到 2 幂，CHR 同理）以对齐上游表键；② 把 `parseResult.mapper_no` 写回 `iNESCart.mapper_number` 与局部 `MapperNo`；③ 各配一条锁（formats crate 侧断言 padded CRC 与上游一致；C++ 侧断言修正后的 mapper 真的到达 `create_cart_for_mapper`）；④ 用这 9 张卡作**本地取证夹具**回归（卡带不进仓库）。 |
| r11 | 2026-11-11 | **S5 落地（三个缺陷全修，吞食天地2 跑出与上游一致的游戏标题画面）——门禁全绿，formats 151 → 153。** ① **A1（hash 作用域，Rust）**：`fceux11_rust_ines_compute_hash` 增 `prg_padded_size` / `chr_padded_size` 两个参数，新加 `feed_padded()` 辅助（raw 字节 + `0xFF` 填充到 2 幂，哈希覆盖**实际映射缓冲区**）；同批把 `compute_layout` 的 `prg_size_bytes` 改成**恒为舍入尺寸**（上游的 `ROM_size = uppow2(not_round_size)`，其注释明说映射要求 2 幂）⇒ 映射尺寸与 hash 口径一致。**⚠️ 这一条推翻 S4 的 A3 决策**（详见 r10 与 §四之三 A1 的备注）：A3 的依据「对既有 ROM hash 零变化」成立，但**漏了 hash 同时是修正表 / `SAVIE_WHITELIST` / `CheckBad` / `MasterRomInfo` 的查找键**；**A1/A2 的 raw 切片保留不动**（issue #12 的核心修复不受影响）。② **A2（修正写回，C++）**：`ines_apply_corrections()` 增出参 `corrected_mapper`（此前 `tofix & 1` 分支是**空注释**，修正后的 mapper 烂在局部 `parseResult` 里）⇒ 修正后的号进入 `iNES_Init()` 与 v1.8 cart factory；规则抽成 `ines.h` 的 `FCEU_EffectiveMapperNo()` 以便锁。③ **A3（mapper 198 的 power，C++）**：上游 `Mapper198_Init` 是 `info->Power = M195Power`（把 16 KB WRAM 映到 `$5000` 并装读写 handler），本仓基类只跑 `GenMMC3Power()`（WRAM 在 `$6000`）⇒ 该板**没有 `$5000` 的工作 RAM**；已按 S4 的 B1′ 同一模式加 `Mapper198Cart::on_power()` → `M195Power()`（`M195Power` 去 static）。**其余 16 个同因变体另立项**。④ **锁（+2 Rust、+2 C++）**：Rust 侧把原 A5 锁的 hash 断言改为 padded 口径并加 `assert_ne!(raw hash)`（把「不能算 raw」钉住）、新增「padded CRC `0x28192599` 命中 `{…,198,-1}`」与「修正写进 `parseResult.mapper_no`」两条；C++ 侧（`mapper_load_test`）新增 mapper 198 的 **`$5000` 写回读**（写 `0x5A` 再读回：跑 `M195Power` ⇒ `0x5A`，不跑 ⇒ 开放总线 `0x00`）与「有效 mapper 规则」两条。⑤ **门禁（实测）**：`ctest -LE perf` **36/36**；GBA 根 **266/24**、vendor **242**、debug **93**、media **52** 全绿；formats **151 → 153** 全绿 ⇒ **未碰 GBA 线**。⑥ **回归（本地取证，9 张卡 GUI 自动化实测）**：吞食天地2 **跑出「吞食天地 三国外传 PUSH START BUTTON 豪华中文版」**（上游同卡同画面）⇒ **S5 的目标达成**；9 张里 **8 张与上游 2.6.6 / 2.7.0 行为一致**，剩下 4 张（Q Boy / SD高达外传2 / SD高达外传3 两版）**双方都灰屏** ⇒ **上游级缺口**，非本仓回归，另立项。⑦ **本轮顺带确立的两个可复用事实**：① **本仓 exe 不吃命令行 ROM**（Qt 前端从不调 `Config::_parseArgs`，`_parseArgs` 只属于旧的 SDL 前端）⇒ 自动化必须走 GUI（Alt+F→O→**剪贴板粘贴**路径→Enter；中文路径无法用 SendKeys 直输）；② **上游基线是 FCEUX 2.6.6**，「1.15」是本项目自己的版本线（上游 GitHub 发布只回溯到 2.2.x，1.15 的二进制在 SourceForge 且已 404）。 |
| r12 | 2026-11-11 | **S2-b 落地——stub BIOS 的 SWI 向量从 `b .` 自旋改为 `MOVS PC, LR`，未实现 SWI 从「机器冻结在 BIOS 0x08」变「返回 + 记录」。** ① **兜底语义（口径修正）**：§3.2 原文「寄存器返回值按 mGBA 的做法给合理默认」**无法按字面执行——mGBA 源码不在本机**（`Research_only/` 目录已不存在，实测）；改按 **GBATEK 自建依据**：SWI 通用 ABI「Outgoing registers R0,R1,R3 are typically containing either garbage, or return value(s). All other registers (R2,R4-R14) are kept unchanged.」（`.zcode/gbatek_plain.txt:192058-192062`）⇒ 调用了未实现调用的游戏本就无权期待返回值，**向量字 = `MOVS PC, LR`（`0xE1B0F00E`）、全寄存器保留、零编造**。② **返回地址证据链**：GBATEK「For Returning from SWI use MOVS PC,R14」`:185672-185674`；ARM 态 LR=PC+4 `:185686`、Thumb 态 LR=PC+2 `:189222` ⇒ LR 恰好停在 SWI 之后那条指令，两种状态都直落续接点；`SUBS PC,LR,#4` 会退回 SWI 自身重入（r5 ⑥(b) 预警的实证确认）。③ **落点**：`bios.rs` 的 `MOVS_PC_LR` 常量（`:115`）进 `build()` 的 0x08 槽位；`BRANCH_SELF` 保留给 abort 向量 0x04/0x0C/0x10/0x14（真异常响亮地停仍是对的）；vendor 侧**零改动**（`operations.rs:121-151` 的 S 位返回路径被 IRQ 返回实际走通，只读核实）。④ **生产侧记录（红线 §六.3）**：`swi/mod.rs` 的 `note_unimplemented_swi`（decline 臂调用）——thread-local `BTreeMap` 每号累计计数、每号只报首次 `eprintln!`（格式 `[gba] swi 0xNN …`），后续静默计数不刷屏；过滤器 `vendor_core_serves` 从 `#[cfg(test)]` 转常驻（vendor 自答的 0x00/0x06-0x08/0x0B/0x0C 不计入）；**未新增 C ABI 导出**（跨出须动 build.rs + ffi.rs + drift guard + 生成头，登记为延后项）。⑤ **锁面**：两条被 §1.2.1 预告的锁按预告改写并更名——`abort_vectors_park_and_the_swi_vector_returns`（原 `unhandled_vectors_park_instead_of_wandering`：abort 向量仍 park、0x08 = MOVS_PC_LR）与 `an_unclaimed_swi_returns_and_is_recorded`（原 `the_stub_does_not_implement_swi`：端到端断言 SWI 0x1A 后机器到 Halt、续接指令真跑、ABI 保留 r4、tallies 记 (0x1A, served=0)、生产计数恰 [(0x1A,1)]）；新增 `the_production_recorder_counts_unimplemented_calls_and_skips_served_ones`；`swi_unimpl_1a`/`swi_unimpl_16` 摘 `#[ignore]` 转常驻绿锁（照 r7 ④ 先例）；新增 Thumb 绿锁（见 ⑦）；REPRODUCERS 5 → 3、夹具 9 → 10。⑥ **变异 M1–M4 逐腿双向可红**（还原刷 mtime）：M1 向量改回 park ⇒ 恰 5 条红；M2 记录器摘掉 ⇒ 恰 2 条红；M3 vendor 过滤摘掉 ⇒ 恰 1 条红；M4 Thumb 编码器 rd 字段挪错 ⇒ 恰 1 条红；各腿其余测试保持绿（各断言各管一段）。⑦ **Thumb 路径完整成立**：夹具 `thumb_swi_unimpl_1a_returns_and_continues_in_thumb`——ARM 入口 BX 进 Thumb，`swi 0x1A` 后 post-call 标记 0xA5 ≠ pre-call 0x5A（只能由异常返回后真继续执行 Thumb 置位），halt 时仍为 Thumb 态（MOVS 从 SPSR 恢复 THUMB 态，GBATEK `:189203-189210` 在 vendor 上实证）。⚠️ **新坑（已写进编码器文档）**：ARM `BX` 基字 bits 7-4 必须是 `0001`（`0xE12FFF10|rm`）——首版写 `0xE12FFF00` 被 vendor 的 `is_msr_flg` 特判劫走解码成 MSR，机器停 0x08、dispatcher 零调用的假象；「文档注释写对位段、常量写错」靠 disasm 通道实证才抓到。⑧ **门禁（终轮复跑）**：根 crate **270 passed / 22 ignored**（266/24 → +2 转正 + 记录器锁 + Thumb 锁；22 = 19 测量工具 + 3 复现器）；复现器域过滤 `--ignored` **0 passed / 3 failed**（eeprom_chip_capacity / flash_status_reg / rom_window_mirror，归属 S3）；vendor **242**（= 未碰 vendor）；debug 93 / media 52 / formats 153 / 两态 `cargo check` 全绿；Release 构建 EXIT=0、`ctest -LE perf` **36/36**；**7 卡 release 基线逐字一致**（6 changing + FireRed FROZEN；零 panic、零 not-implemented 日志、tallies 无 NOT SERVED——兜底路径在 7 卡窗口内未被触发，触发面由 M1–M4 背书）；探针日志存 `.zcode/s2b/baseline_s2b.log`（gitignored）。⑨ **S3-a 前置 RE 同轮完成（只读）**：15 个目标号码逐号证据表（GBATEK 行号逐条、mGBA 列全部 n/a）、**8 条「GBATEK 未指定」清单**（0x1F 公式、0x20–0x24 全部语义、0x19 渐变表、0x1A 写入效果、无参声音 SWI 的指针来源、0x2A 的 36 项函数表等——开工时按纪律 4 停下记录）、0x26 HardReset 的**结构性实现障碍**（dispatch 拿不到整机所有权）、SOUNDBIAS 是 `0x04000088` 不是 `0x04000208`（那是 IME）；证据表存 `docs/plans/FCEUX11-v2.0.3_S3a前置RE-GBATEK与SWI证据表.md`。⑩ 文件面：`bios.rs` / `swi/mod.rs` / `fixtures.rs` / `fixtures/swi.rs` / `fixtures/guards.rs` / `frame.rs`（注释）/ `cartprobe.rs`（判词措辞随行为更新：「CPU parks at BIOS 0x08」的旧表述在本修复后成为虚假陈述）；未碰 `gba_load.cpp`（r13）与 vendor 树。 |
| r13 | 2026-11-11 | **S2-c 落地——`gba_load.cpp` 每帧步进的错误码从「静默吞掉」改为去重日志（不弹窗/不崩/不卸载）。** ① **现状（只读调查）**：改动前 `gba_step_frame()` 的非零返回被 `if (… != GBA_OK) return;` 完全吞掉——无日志、无弹窗、不崩、不卸载，静默跳过该帧的帧缓冲拉取 / 重绘信号 / 音频排空 / 电池落盘；对照一次性路径（装载 / savestate 读写 / 电池读取）全部已有报告，**规律 = 一次性路径全报告、每帧路径全静默** ⇒ §3.3 要求的「记日志」与红线 2 的容错读数缺失。② **改动（只动 `gba_load.cpp`，4 处）**：会话态去重闩 `g_reportedStepError` + `step_error_name`（6 码全名映射）+ `note_step_error`（同码 streak 只报首帧、码变化即再报、干净帧清闩、`GbaLoad` 新会话清闩——一次 panic 共约 3 行日志，最坏有界）；步进点 rc 入变量、非零 `note_step_error` + return（跳过集与改前逐字一致，电池 dirty 标志不被取走）。③ **日志设施 = `FCEU_printf`**：实现在 core（`fceu.cpp:1132`）、经 driver 回调表分发到 stdout + 消息日志 dock，F11QA 四个只链 core 的可执行文件不受影响（纪律 8）；**刻意不用 `FCEU_PrintError`**——它经 `QueueErrorMsgWindow` 下个 GUI tick 变**弹窗**，正是每帧路径禁止的行为。④ **GBA_ERR_STATE 语义**：日志附一行「the core caught an internal panic and dropped the machine; loading a ROM again is what recovers」——对齐 r6 ⑤ 的恢复机制（机器已丢、毒锁透明恢复、重载即重建），**不发明新恢复动作**；release（`panic="abort"`）下该码不可达，日志注释写明读者是 debug/探针构建。⑤ **边界（如实登记）**：release 生产的真 panic 仍带走进程（C++ 侧无法拦——S2-c 对 release 的真实保障是 S2-a/S2-b 消灭可达 panic，本改动是 unwind 构建的安全网 + 红线 2 可见性）；相邻每帧静默点（`gba_frame_buffer*` / `gba_render_audio` / 电池 flush 两导出）**刻意未包**（step 成功后不可达 + diff 最小，要全覆盖接同一个 `note_step_error` 即可）；`gba_last_error` 是恒返 GBA_OK 的 stub，文本化待其被填充。⑥ **门禁**：与 r12 同轮终验——Release 构建 EXIT=0、`ctest -LE perf` **36/36**；`git diff --numstat` 与真实改动一致、文件保持 CRLF 纯 ASCII。 |
| r14 | 2026-11-11 | **账面修正（§2.4 的 S1 出口判据勾选 + CHANGELOG 补记 S2-b/S2-c）—— 未动任何生产代码。** ① **§2.4 的 10 条 S1 出口判据逐条勾选**，每条附**当日实测读数**（`ctest -LE perf` **36/36**；GBA 根 **270 passed / 22 ignored**；vendor **242** / debug **93** / media **52** / formats **153**；复现器 `--lib gba::fixtures:: -- --ignored` **0 passed / 3 failed**，红的正是 S3-b ×2 + S3-d ×1）——**不是事后追认**，是拿当日读数逐条对出来的。② **两处口径按实际修正**：夹具数 **9 → 10**（**7 条常驻绿锁** + **3 条红复现器**，r12 新增 Thumb 绿锁）；门禁计数 **262 → 270**（r6 +2 探针锁、r7 +2 OAM 转正、r12 +4）。⚠️ **§2.2 的「9 条」表与 §五 编排图仍写 9 条**（已知不一致，已在 §2.4 明示，留待下一轮统一）。③ **两处勾选时保留「未完成」字样**：F11QA 矩阵 `pass_to_fail = 0` / grade ≥ B **待 push 后由 CI 复核**（本地只有 `ctest` 这个近似门禁）；**Flash 状态寄存器 `0x0E000006/7` 在 GBATEK 全文零命中**（GBATEK 的 flash 轮询是回读比较 `:8981`）⇒ `flash_status_reg` 复现器断言的是**本计划自定的约定** ⇒ **S3-b 开工前须另找来源或用真卡带重新推导**。④ **CHANGELOG 的 `[Unreleased] v2.0.3` 段补记 S2-b / S2-c**（此前只记了 S1-a 与 S4 ⇒ 两个**有生产行为变更**的批次漏记：S2-b 改向量字、S2-c 改 C++ 每帧错误路径）+ 一节「S2 收口后的门禁读数」。**未动任何生产代码**（两个改动文件都是 Markdown）。 |
| r15 | 2026-11-11 | **S3-a 第一批落地 —— BIOS SWI `0x16`/`0x17`/`0x18`（diff unfilter）有真实现了**；夹具集合 **10 → 12**，GBA 根 crate **270 → 280**。**未动 vendor 树**（改动只在 `src/rust/src/gba/`），**未动任何 C/C++** ⇒ NES 零回归面不受影响。① **实现**：新 bus-free 模块 `src/rust/src/gba/swi/unfilter.rs`（解码核）+ `swi/mod.rs` 的 `serve_diff_unfilter`（接线）—— `0x16` = 8 位单元/逐**字节**写、`0x17` = 8 位单元/逐**半字**写、`0x18` = 16 位单元/逐**半字**写；源块 = 32 位头部（bits 0-3 数据大小 / 4-7 类型 / 8-31 **字节数**）+ 数据单元；累加器按单元宽度回绕（8 位 mod 256、16 位 mod 65536）；源地址按 GBATEK 要求对齐到 4；**拒绝**零长度 / 超 16 MiB / 半字调用遇奇数长度 / 半字调用遇奇地址（**落点一律不动**，不留半写的目的地）。② **依据（两份独立）**：GBATEK 快照 `:192251-192253`（号段）、`:192804-192867`（契约/头部/算术）、**`:192826-192827` 它自己的示例向量**；**mGBA 的 `_unFilter`**（`src/gba/bios.c:903-946`，经代理 `127.0.0.1:7897` 取回，**只读格式不抄代码**——MPL-2.0 与本项目许可不相容）—— 它是唯一说清「哪一号读 8 位单元、哪一号存半字」的来源（调用点 `bios.c:578`），也是「size 字段按**字节**计」的第二依据。③ **锁**：三个端到端夹具（`swi_diff8bit_unfilter_wram`——**由 `swi_unimpl_16` 改写**、`swi_diff8bit_unfilter_vram`、`swi_diff16bit_unfilter`）+ `unfilter.rs` 内 **8 条单元锁**；期望值取 **GBATEK 公布的那一行**（10 11 … 19），**不是**拿实现的同一套算法重算——「测试与实现同错」是本项目栽过两次的坑。④ **落地点的选择有讲究**：`0x17`/`0x18` 的落点用 **VRAM** —— vendor 对 VRAM 字节写会复制/丢弃（`bus.rs:1235-1262`）⇒ **写宽度可观测**；`0x16` 的落点是 WRAM ⇒ **它的写宽度不可观测**（已如实登记，宽度由实现与 mGBA 参照背书）。⑤ **变异 M1–M5 逐腿双向可红**（基线 280 全绿；每腿还原后 **280 / 0**）：M1 `0x16` 读成 16 位单元 ⇒ 只红 `swi_diff8bit_unfilter_wram`；M2 `0x17` 改字节写 ⇒ 只红 `swi_diff8bit_unfilter_vram`；M3 `0x18` 读成字节单元 ⇒ 只红 `swi_diff16bit_unfilter`；M4 半字配对顺序颠倒 ⇒ 红两个共用该路径的；M5 size 字段读成 16 位 ⇒ 只红 `the_size_field_is_twenty_four_bits_wide`。⚠️ **M5 首轮是「无牙」的**（夹具 size 只有 10/20 字节，16 位与 24 位两种读法结果完全相同）⇒ 补了 64 KiB 的字段宽度锁才有牙——与 r4 的三条盲区同类。⑥ **两条被新认领号打破的老锁已修**：`dispatch_claims_exactly_what_is_implemented` 与 `declining_a_number_has_no_side_effects`（两条都把「认领集合」写死了）——它们**在实现落地后立刻变红**，是「认领集合一变，老锁必须同步」的又一实例。⑦ **门禁**：GBA 根 **280 passed / 22 ignored**（22 = 19 个测量工具 + **3 条红复现器**，逐条仍红：S3-b ×2 + S3-d ×1）；vendor **242** / debug **93** / media **52** / formats **153** 全绿；`ctest -LE perf` **36/36**。⑧ **如实登记三点**：size 按字节在 GBATEK 里**没有明说**（依据 = 与 LZ77 同款头部 + mGBA 的 `remaining = header >> 8`）；mGBA 会把 `r0`/`r1` 写成走过的游标，本仓**不动**（GBATEK：No return value；SWI ABI 里这两个寄存器对无返回值调用未定义）；`0x16` 的写宽度不可观测。⑨ **本轮未做**：`0x19`–`0x2A`（声音驱动等）与 S3-b/c/d/e 全部未动。 |
| r16 | 2026-11-12 | **S3 收口前的大规模只读 RE（4 路并行 agent）—— 计划的四条前提被权威来源证伪，两个新潜伏缺陷浮出。生产代码零改动**（探针侧 +1005 行全在 `cartprobe.rs` 的 `#[cfg(test)]`），报告存 `.zcode/v203_stage1/{s3a_re,s3b_re,s3c_forensics,s3de_design}.md`。**① S3-a 余项 18 号逐号判定（0x19–0x2A）**：READY 4（`0x19` SoundBias——SOUNDBIAS `0x04000088` vendor 已建模、GBATEK `:194291-194301` 语义完备，「0x270 校准表」复核维持判伪；`0x1F` MidiKey2Freq——GBATEK `:194413-194414`+`:194243-194245` 闭式可推导、mGBA `bios.c:266-273` 同式；`0x25` MultiBoot——`:193914` 返回契约 + vendor SIO 无从机 ⇒ 恒 failed；`0x27` CustomHalt——`:193426` 参数在 **r2**）· PARTIAL 13（**声音家族 12 号在 mGBA HLE 下也全是 no-op stub**（`bios.c:607-608` default 分支）——任何模拟器都没有官方软合成器的 HLE，可实现内容只剩「认领 + 文档化 no-op」= 零功能收益，且会违背红线 3 的「未实现必须可见」⇒ 不做，记已知限制；0x26 HardReset 结构障碍确认仍成立、建议挂起）· **BLOCKED 1（`0x2A` SoundGetJumpList：36 项函数表 GBATEK 零记载、mGBA 显式放弃转抛真 BIOS）⇒ 维持 S2-b 兜底**。**② S3-b 来源问题：`flash_status_reg` 的前提被否证**——GBATEK FLASH 全章 `:8858-9146` 命令集无 `0x70`、`E000006` 全文零命中、轮询 = 回读比较（`:8981`）；**最强证据 = 真卡带里在跑的任天堂 SDK 驱动反编译**（pret `agb_flash_1m.c:51-86`：`PollFlashStatus` 是 WRAM 里的**普通阵列字节读** + DQ5 超时位；`agb_flash.c:106-116`）；mGBA `savedata.h:38-49` 无 0x70（忙 = dust 定时器，program 650/erase 30000 cycles）；VBA-M 同。⇒ **E3 重定性：vendor 的 flash 阵列读本来就是对的**，复现器断言须反转成绿锁（回读 == 种值）。**E2 设计定型 D1**：ID 串 `nnn` = **库版本号不携带容量**（GBATEK `:8521`——原夹具的「EEPROM_V124 = 8 KB」假设被否证）⇒ 走 mGBA 同款动态容量（默认 512 B + 高位访问懒扩 8 KB；本仓协议层已从位流推地址宽度；`eeprom_block_offset` 保证数据不越 8 KB ⇒ 旧 32 KB `.srm` 前 8 KB 无损迁移）。**③ S3-c 取证（探针，全双通道实证）**：**FireRed 忙等地址更正——轮询的是 `0x0300310C`（gMain+0x1C）不是 §1.2.3 写的 `0x030030FC`**（后者是回调指针，帧 14 一次性写入）；写入方定位 = ROM `0x080007B4` `STRH r1,[r5,#0x1C]`（VBlank IRQ 回调，同函数设 BIOS IntrWait 标志）；**「GPIO 先读后写」线索证伪**（300 帧零写、30M 指令 sniff 零 GPIO 指针加载）；**真机制链**：IE/IME/IRQPTR 全就位、PPU 正常走，但 **DISPSTAT bit3（V-IRQ 使能）300 帧从未置位** ⇒ IF 恒 0 ⇒ 写入方永不执行 ⇒ 忙等死锁；游戏意图已写入 IRQ 管理器影子 `0x03000004=0x0008`，但「影子→硬件」apply 从未运行（DISPCNT 影子 0x0140 同样未上硬件）；**加速时钟 +25% 对照跑 ⇒ apply 执行、V-IRQ 每帧发、游戏活**（106 hash）⇒ **init 时序竞争**（入口：0x030000C0 锁 + 0x03000060 队列 + flusher 0x08000A18 调用方）——属精度范畴，超出本轮「能不能跑」范围，转长尾。**Dragon Ball / Mario Kart / Minish Cap 长窗口（1200–1800 帧）+ 输入注入：卡死/黑屏均未复现**，DB 的 EEPROM 窗口 18 万次访问零写提交、MK 的 DISPCNT 稳定在 gameplay 形态 0x1741 ⇒ 三张卡的机器判据全过，观感转人工。**方法论发现（新纪律候选）**：`bus.read_half_word` 会加 `cycles_count`（`bus.rs:673/1302`）⇒ **轮询式探针给机器加速时钟并改变行为**——§1.2.3 历史读数出自此类探针；⇒ 探针观测一律 `read_raw`（本轮新探针已按此写）。**④ S3-d/S3-e 设计确认**：**镜像前提证伪**（GBATEK `:14046` "Normal ROM Cartridges ... are NOT mirrored"；`:14029-14030` 开放总线公式是唯一成文行为；mGBA 越界 = 开放总线、melonDS 返回 0 且注 "CHECKME"）⇒ §1.2.5 是凭记忆写入的第三次同类案例；**mode 4 index 0 证伪**（GBATEK `:3198` "Color 0 (backdrop) is transparent, and OBJs may be displayed behind the bitmap"；mGBA 同）⇒ vendor `layer_2.rs:193-196` 正确，改了才是回归；**BG2 门控证伪**（`:1944-1952` BG2 在 mode 0–5 全存在，vendor 分发逐格一致）；**mode 5 页面翻转确认是真缺陷**（`render_mode5` 缺 frame_offset；`:1918` bit4 适用 modes 4,5、`:3204-3206` Frame 1 = 0600A000）。⑤ 门禁（探针改动后）：根 **280 passed / 23 ignored**（+1 新探针）、复现器 3 红不变；`ctest -LE perf` 36/36。⑥ **顺带发现两个潜伏缺陷（r17 修）**：`apply_state` 读档后 `backup_type` 丢失（vendor 的 `redetect_backup_type` 全仓零调用）；`try_read_word` 越界臂走慢路径（r17 实测定性为契约/性能修复）。 |
| r17 | 2026-11-12 | **S3 收口实施批 —— S3-a 第二批 + S3-b + S3-d（重划）+ S3-e 落地；三条复现器全部转正为绿锁（复现器清零）；`apply_state` 的潜伏缺陷比 RE 报告更深，一并修掉。** 双路并行实施（internal_memory 路在真树、swi+lcd 路在隔离副本），每项变异验证双向可红；报告存 `.zcode/v203_stage1/{s3bd_impl,s3ae_impl}.md`。**① S3-a 第二批（全在 `swi/`，零 vendor 改动）**：`0x19` SoundBias（SOUNDBIAS 低 10 位替换、高位保持，2 锁）；`0x1F` MidiKey2Freq（**freq 在 WaveData+4——证据表的 +8 是错的，已更正**；公式按 GBATEK 掩 u8+f32、注释**标注为推导**；期望值 = 整数八度向量独立拼装，4 锁）；`0x25` MultiBoot（无从机 ⇒ r0=1 failed，2 锁）；`0x27` CustomHalt（参数在 r2：00h Halt / 80h Stop，复用 `enter_low_power` 并重构为 (swi_num, mode)，reserved 值按 IntrWait 先例返回 + trace，4 锁；⚠️ 「80h 判成 halt」变异在本仓结构上不可观测（Halt/Stop 同一建模）⇒ 以可观测合同四腿替代，如实登记）；**0x01 RegisterRamReset 搭车项**：补 DISPCNT 强制空白（GBATEK `:193540-193541`——RE 报告的 `:193530-193531` 实为空行已更正；整写 `0x0080`，mGBA 只读参照同构，1 锁）。认领集合两条老锁同步（0x19/0x1F/0x25/0x27 进认领集）。**② S3-b**：E2 = vendor 3 处（V1 `buffer_size` Eeprom `0x8000`→`0x0200`；V2 新私有 `ensure_eeprom_capacity` 在命令解码读写两分支调用、按需扩容 `0x2000` 封顶尾填 0xFF——与 mGBA 逐级翻倍的差异 = 中间文件尺寸，定型结果一致；V3 `load_battery` 对 Eeprom 先扩到文件长度再拷入，旧 32 KB `.srm` 前 8 KB 无损迁入、>8 KB 尾段一次性丢弃——旧模型位流写入被 `eeprom_block_offset` 掩在 8 KB 内，尾段只可能是 0xFF）+ 4 条 vendor 单元锁；E3 = `flash_status_reg` 断言反转成绿锁（回读 == 种值，期望值按 8 位总线双通道复制字段拼装 + `assert_ne!(0xFFFF)` 哨兵；「忙窗口」不做——GBATEK `:9119` verify-retry + SDK 即时成功 + MKSC 实测可玩）；`eeprom_chip_capacity` 断言反转成绿锁（512 B 起步 / 14 位访问扩到 8192 且**经 14 位读回读种值** / 裸 `EEPROM_V` 卡同样到 8192——钉住「ID 串不携带容量」；每条写命令后加 Ready clock-out 提交再量尺寸，防「未解释命令 ⇒ 错误原因全绿」）。**③ apply_state 潜伏缺陷（范围扩大）**：先写锁跑出红——**读档后机器的 ROM 与 BIOS 也是空的**（`rom`/`bios_system_rom`/`backup_type` 三者同为 `#[serde(skip)]`，`mem::swap` 把带数据的 CPU 换出去丢弃）；**不还原 ROM 则 redetect 必然 no-op**（从空 ROM 检测只能得 None）⇒ 修复 = `apply_state` 内三条语句（rom / bios / redetect）+ 锁 `the_backup_chip_survives_a_savestate_round_trip`（M6a 摘 redetect / M6b 摘 ROM 还原，双向可红）。**savestate 读档在修复前对真实游戏是致命的**（空 ROM = 取指开总线垃圾；空 BIOS = 首个中断向量进空数组）；既有测试全绿是因为没有任何一条在 load 后 step 过帧。**新已知限制**：手动 `gba_set_save_type` 覆盖随读档被 redetect 还原为检测值（vendor doc 既定语义，修复前连检测值都不保）。**④ S3-d 重划**：镜像前提证伪 ⇒ `rom_window_mirror` 改写为**开放总线绿锁**（控制腿 + 字节腿 + 半字腿 + 字腿 + 双哨兵；期望值全部由夹具内独立拼装的 GBATEK 公式给出，不调 vendor 推导——纪律 5/12；「不镜像」由字节/半字腿锁死）；**W5b** `try_read_word` 越界臂（仅整段越界分支）改半字对公式——⚠️ **定性修正：设计报告的「0x0101_2120 形态」是它自己字节清单的端序误读**，现行慢路径经 4×read_raw 已拼出同形 0x0121_0120 ⇒ W5b 是**快路径契约/性能修复**，总线级值修复前后逐位相同（实证：臂退回 None 跑夹具字腿仍绿）；骑跨分支保持 None 零变化；vendor 锁 `rom_open_bus_word_reads_are_halfword_pairs`。mGBA 的 1 MiB 4 倍镜像 / 非 2 幂补零两例外**不跟**（可选项未获裁决，默认不做——夹具对 1 MiB 类卡不设防，将来立项单独处理）。**⑤ S3-e**：mode 5 页面翻转 = vendor `layer_2.rs` 1 处（`frame_offset = bit4 ? 0xA000 : 0`，与 mode 4 同式）+ 2 条 vendor 单元锁 + 端到端锁 `mode5_page_flip_reads_the_selected_frame`（两页逐行不同图案、bit4 两运行互为对照；M1 摘 offset / M2 基址误写 0x9600 均可红——逐行图案正是 M2 的可观测性保障）；mode 4 index 0 **防回归锁** `mode4_index0_is_transparent_under_a_lower_priority_obj`（OBJ-behind-BG2 优先级技巧——直接断言 index0==palette[0] 是无牙的：backdrop 色就是 bg_palette_ram[0]）；BG2 门控不加锁（证伪记录）。**⑥ 守卫收口**：`REPRODUCERS` 清空（注释记录三次晋升史）；`fixtures/lcd.rs` 注册进 G2 扫描集合（模块 4 → 5、PLANNED 12 → 14，归属 S3-e）。**⑦ 门禁（合并后终读数）**：根 crate **299 passed / 0 failed / 20 ignored**（20 = 19 提交口径测量工具 + 未提交探针 1；**复现器清零**）；vendor **249**（242 + 5 单元锁）；debug **93** / media **52** / formats **153** 全绿；`cargo check` 两态 EXIT=0（E0133 全部是既有 adapter/SIMD 代码的 **warning**，未触碰）；`ctest -LE perf` **36/36**。**⑧ 变异验证汇总**：S3-b 路 3+2+2+2+2+2 腿、S3-a/S3-e 路 15 腿，全部双向可红、还原腿刷 mtime。**⑨ 实施期新坑两条**：① 副本跑门禁缺 C++ 树 ⇒ decoupling 7 守卫环境红——副本以 junction 指回真仓库子目录解决（只读）；② **bus 字写会把未对齐地址向下对齐再写 4 字节** ⇒ fixtures 逐字节 word 写每 4 字节只落 1 个（mode4 夹具曾假红），改按对齐字拼——fixtures.rs 的坑清单本轮未动，待登记。⑩ vendor 补丁 **28 → 33 处、6 → 7 个源文件**（internal_memory.rs 4 → 8：V1/V2/V3/W5b；新增 `lcd/layers/layer_2.rs` 1：mode5）；ATTRIBUTION §3.2.13–§3.2.17。 |
| r18 | 2026-11-12 | **文档收口 + 提交（未动生产代码）**：按纪律 2 先记 §九 r16–r17，再改正文——§1.2.3（FireRed 地址更正 + GPIO 线索证伪 + 真机制与范围裁定）、§1.2.4（E3 重定性 + E2 定型）、§1.2.5（镜像证伪重写）、§1.2.6（mode4/BG2 证伪、mode5 已修）、§2.2（夹具表 12 → 14 条、全绿）、§四 S3 各小节收口注记与 S3 出口判据改写（FireRed 行按根因裁定改写）、§七 整包出口判据更新；AGENTS.md（进度块、vendor 表 33 处/7 文件、长尾新增 FireRed 时序竞争与 1 MiB 例外两条、新坑两条）；CHANGELOG `[Unreleased] v2.0.3`；ATTRIBUTION §3.2.13–§3.2.17；证据表补 0x1F +4 更正与第二批结果。**7 卡基线复跑见 §九 r19**。 |
| r19 | 2026-11-12 | **Stage 5 基线复跑 + 双向定位 —— r17 的 `0x01` 强制空白搭车项被实证为 FireRed 越过 init 竞争的确定性触发点（未动生产代码，变异已还原）。** 7 卡 240 帧基线复跑（release）：6 张与 r6/r12 基线**逐字一致**；**FireRed 从 FROZEN（1 hash、163724 恒定步数、DMA 零块）变 alive（48 变化帧哈希、steps 变动、音频 DMA 186 blocks / 145360 units 激活）**。SWI 清单显示 FireRed 在窗口内只调了 `0x01`(×1)/`0x0B`/`0x0C`/`0x11`/`0x12`——**第二批四个号零调用** ⇒ 唯一被 r17 改变且被 FireRed 踩到的行为 = `0x01` 的 `DISPCNT=0x0080` 强制空白。**双向变异定位**：摘掉该写入 ⇒ FireRed 立即回 FROZEN（与 pre-r17 逐字同）；还原复跑 ⇒ 回 48 哈希且步数与基线逐字一致。⇒ **机制**：r16 定性的 init 时序竞争成立，且已知一个确定性「解楔」——BIOS 侧按 GBATEK `:193540-193541` 的强制空白写入改变了 init 的状态/时序条件，使游戏的影子 apply 链得以运行（音频 DMA 激活 = VBlank IRQ 已开闸）。⇒ **S3 出口表的 FireRed 行据实更新**：机器判据「出画面」在探针窗口内达成（画面在变）；进主线观感仍待人工；竞态根因分析与长尾条目保留（解楔是触发性的，不是修掉了竞态）。门禁复核：根 crate **299/20 全绿**（还原后）。日志：`.zcode/v203_stage1/probe_r17_baseline.log`（7 卡）/ `probe_bisect_no01blank.log`（变异腿）/ `probe_firered_restore.log`（还原腿）。 |


---

## 附录 A · GBATEK 摘录（位级格式取证基线）

> **快照来源**：`https://mgba-emu.github.io/gbatek/`（经代理 `127.0.0.1:7897` 抓取，2026-11-08）
> **本地副本**：`.zcode/gbatek_probe.html` / `.zcode/gbatek_plain.txt`（gitignored，**不进仓库**）
> **用法**：动手前先查这里的摘录；摘录没有的条目**回快照查**，**不许凭记忆**。
> **重抓脚本**：`tools/fetch_gbatek.ps1`（经代理抓 HTML → 转纯文本；转换规则与本快照一致
> ⇒ 重抓后行号不变，摘录的行号引用可复核）。

### A.1 GBA DMA 控制寄存器（`DMA CNT_H`，`40000BAh` 等）

```
Bit   Expl.
0-4   Not used
5-6   Dest Addr Control  (0=Increment,1=Decrement,2=Fixed,3=Increment/Reload)
7-8   Source Adr Control (0=Increment,1=Decrement,2=Fixed,3=Prohibited)
9     DMA Repeat                   (0=Off, 1=On) (Must be zero if Bit 11 set)
10    DMA Transfer Type            (0=16bit, 1=32bit)
11    Game Pak DRQ  - DMA3 only -  (0=Normal, 1=DRQ <from> Game Pak, DMA3)
12-13 DMA Start Timing  (0=Immediately, 1=VBlank, 2=HBlank, 3=Special)
      The 'Special' setting (Start Timing=3) depends on the DMA channel:
      DMA0=Prohibited, DMA1/DMA2=Sound FIFO, DMA3=Video Capture
14    IRQ upon end of Word Count   (0=Disable, 1=Enable)
15    DMA Enable                   (0=Off, 1=On)
```

### A.2 卡带存档 ID 串与真实芯片

```
EEPROM_Vnnn    EEPROM 512 bytes or 8 Kbytes (4Kbit or 64Kbit)
SRAM_Vnnn      SRAM 32 Kbytes (256Kbit)
FLASH_Vnnn     FLASH 64 Kbytes (512Kbit) (ID used in older files)
FLASH512_Vnnn  FLASH 64 Kbytes (512Kbit) (ID used in newer files)
FLASH1M_Vnnn   FLASH 128 Kbytes (1Mbit)
```

### A.3 GBA 卡带头布局（本轮实测 7/7 卡全部吻合）

```
0000-0003  Nintendo Logo (前 4 字节)
0004-009F  Nintendo Logo (156 字节)
00A0-00AB  Game Title (12 字节)
00AC-00AF  Game Code (4 字节)
00B0-00B1  Maker Code (2 字节)
00B2       Fixed Value (96h)
00B3       Main Unit Code
00B4       Device Type
00B5-00BB  Reserved (7 字节)
00BC       Software Version
00BD-00BF  Complement Check
00C0-00FF  Start of ARM7/ARM9 Program
```

### A.4 EEPROM 命令协议

> 快照行号范围：`.zcode/gbatek_plain.txt:8710-8786`
> （GBATEK「GBA Cart Backup EEPROM」章的「Set Address (For Reading)」「Read Data」
> 「Write Data to Address」三小节；**原文空行已略去，其余逐字未改**。）

```
Set Address (For Reading)
Prepare the following bitstream in memory:
  2 bits "11" (Read Request)
  n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)
  1 bit "0"
Then transfer the stream to eeprom by using DMA.
Read Data
Read a stream of 68 bits from EEPROM by using DMA,
then decipher the received data as follows:
  4 bits - ignore these
 64 bits - data (conventionally MSB first)
Write Data to Address
Prepare the following bitstream in memory, then transfer the stream to eeprom
by using DMA, it’ll take ca. 108368 clock cycles (ca. 6.5ms) until the old data
is erased and new data is programmed.
  2 bits "10" (Write Request)
  n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)
 64 bits data (conventionally MSB first)
  1 bit "0"
After the DMA, keep reading from the chip, by normal LDRH [DFFFF00h], until Bit
0 of the returned data becomes “1” (Ready). To prevent your program from
locking up in case of malfunction, generate a timeout if the chip does not
reply after 10ms or longer.
```

> 用途：S3-b「EEPROM 真模型」的命令协议依据（读请求 `"11"` / 写请求 `"10"`、6 或 14 位地址、
> 64 位数据、读回 68 位 = 4 忽略 + 64 数据、Ready 位），同时是 §一.2.4 r3「vendor 协议逐位一致」
> 与绿色回归锁 `eeprom_protocol_lock` 的原文出处。

### A.5 EEPROM 寻址与容量

> 快照行号范围：`.zcode/gbatek_plain.txt:8661-8706`
> （章节头 `:8661`；芯片型号行 `:8665` / `:8669`；「Addressing and Waitstates」`:8677-8694`；
> 「Data and Address Width」`:8698-8706`；**原文空行已略去，其余逐字未改**。）

```
GBA Cart Backup EEPROM
9853 - EEPROM 512 Bytes (0200h) (4Kbit) (eg. used by Super Mario Advance)
9854 - EEPROM 8 KBytes (2000h) (64Kbit) (eg. used by Boktai)
Lifetime: 100,000 writes per address
Addressing and Waitstates
The eeprom is connected to Bit0 of the data bus, and to the upper 1 bit (or
upper 17 bits in case of large 32MB ROM) of the cartridge ROM address bus,
communication with the chip takes place serially.
The eeprom must be used with 8 waitstates (set WAITCNT=X3XXh; 8,8 clks in WS2
area), the eeprom can be then addressed at DFFFF00h..DFFFFFFh.
Respectively, with eeprom, ROM is restricted to 8000000h-9FFFeFFh (max.
1FFFF00h bytes = 32MB minus 256 bytes). On carts with 16MB or smaller ROM,
eeprom can be alternately accessed anywhere at D000000h-DFFFFFFh.
Data and Address Width
Data can be read from (or written to) the EEPROM in units of 64bits (8 bytes).
Writing automatically erases the old 64bits of data. Addressing works in units
of 64bits respectively, that is, for 512 Bytes EEPROMS: an address range of
0-3Fh, 6bit bus width; and for 8KByte EEPROMs: a range of 0-3FFh, 14bit bus
width (only the lower 10 address bits are used, upper 4 bits should be zero).
```

> 用途：S3-b「EEPROM 真模型」的寻址与容量依据（512 B ⇒ 6 位地址 `0-3Fh`、8 KB ⇒ 14 位地址
> `0-3FFh`），也是 §一.2.4 E2「芯片型号与容量不建模 ⇒ `.srm` 尺寸恒 32768」的原文出处，
> 而 `:8692-8694` 又是 r3 撤回「E1 访问窗口过宽 16 MB」的原文依据。

### A.6 GPIO 寄存器与 RTC（S1-a 遗留，r6 补齐）

> 快照行号范围：`.zcode/gbatek_plain.txt:9218-9276`（GPIO 寄存器三件套与写语义）、
> `:9280-9295`（连接示例表）、`:9312-9331`（RTC 芯片与访问方式）、`:9371-9394`（RTC
> Control Register）、`:9402-9406`（Datetime/Time 编码）；
> **原文空行已略去，其余逐字未改。**

```
The I/O registers are mapped to a 6-byte region in the ROM-area at 80000C4h,
the 6-byte region should be zero-filled in the ROM-image. In Boktai, the size
of the zero-filled region is 0E0h bytes - that probably due to an incorrect
definition (the additional bytes do not contain any extra ports, nor mirrors of
the ports in the 6-byte region). Observe that ROM-bus writes are limited to
16bit/32bit access (STRB opcodes are ignored; that, only in DS mode?).
80000C4h - I/O Port Data (selectable W or R/W)
  bit0-3  Data Bits 0..3 (0=Low, 1=High)
  bit4-15 not used (0)
80000C6h - I/O Port Direction (for above Data Port) (selectable W or R/W)
  bit0-3  Direction for Data Port Bits 0..3 (0=In, 1=Out)
  bit4-15 not used (0)
80000C8h - I/O Port Control (selectable W or R/W)
  bit0    Register 80000C4h..80000C8h Control (0=Write-Only, 1=Read/Write)
  bit1-15 not used (0)
In write-only mode, reads return 00h (or possible other data, if the rom
contains non-zero data at that location).
Connection Examples
  GPIO       | Boktai  | Wario
  Bit Pin    | RTC SOL | GYR RBL
  -----------+---------+---------
  0   ROM.1  | SCK CLK | RES -
  1   ROM.2  | SIO RST | CLK -
  2   ROM.21 | CS  -   | DTA -
  3   ROM.22 | -   FLG | -   MOT
  -----------+---------+---------
  IRQ ROM.43 | IRQ -   | -   -
GBA Cart Real-Time Clock (RTC)
S3511 - 8pin RTC with 3-wire serial bus (used in Boktai)
The chip is accessed via 4bit I/O port (only 3bits are used for RTC):
Control Register
  Bit Dir Expl.
  0   -   Not used
  1   R/W IRQ duty/hold related?
  2   -   Not used
  3   R/W Per Minute IRQ (30s duty)        (0=Disable, 1=Enable)
  4   -   Not used
  5   R/W Unknown?
  6   R/W 12/24-hour Mode                  (0=12h, 1=24h) (usually 1)
  7   R   Power-Off (auto cleared on read) (0=Normal, 1=Failure, time lost)
Setting after Battery-Shortcut is 82h. Setting after Force-Reset is 00h.
Datetime and Time Registers
Same as NDS, except AM/PM flag moved from hour.bit6 (NDS) to hour.bit7 (GBA).
```

> 用途：S3-c「GPIO / RTC 与 FireRed 的标志位」的位级依据。**两条对本计划最要紧的原文事实**：
> ① 「**the 6-byte region should be zero-filled in the ROM-image**」—— GPIO 区在 ROM 镜像里
> 应零填充，且 ② 「**In write-only mode, reads return 00h (or possible other data, if the
> rom contains non-zero data at that location)**」—— 这两条**共同**界定了「游戏尚未写入时
> 读 GPIO 读到什么」：GBATEK 没有给出独立的「未初始化」语义，读到的就是 ROM 镜像里的字节
> （应零填充）⇒ 夹具 8（`gpio_read_before_write`，绿锁）断言的「写入前读到 ROM 字节」
> 与此一致，**不得改写成别的语义**。⇒ §一.5 的行动项（GPIO/RTC 两章摘录）至此收口。
> RTC 侧注意：GBATEK 此处只定义了寄存器编码与 3-wire 串行访问方式，**没有**给出
> 位流时序的逐位协议（那在 NDS 章节由引用覆盖）——S3-c 若要实现位级 RTC 对话，
> **先回快照 NDS RTC 章核对**，不许凭此处摘录引申。

---

*本计划由独立 code review 产生（2026-11-08）。实施按工作项逐个开独立任务，先 PLAN 后动手。*
