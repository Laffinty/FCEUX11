# FCEUX11 v2.0.4 — 残留精度与调试待办 · 遗留项计划

> **STATUS: DRAFT**（2026-11-12 立项 · 业主指令：「核对 v2.0.2 残留 review 报告里的老问题，
> 在 v2.0.3 完成之后是否仍然存在；仍存在且仍有攻关价值的，逐项记录下来，留到 v2.0.4 解决」）
> **来源**：`docs/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（2026-11-08 产出）
> **复核基准**：`main` @ `101026b`（v2.0.3 发布提交 `45fec33` 之后；`45fec33` 本身是版本口径提交）
> **本文件只做两件事**：① 逐项判定旧问题在 v2.0.3 之后**是否仍然存在**；
> ② 对仍存在的项做**性价比裁决**——进 v2.0.4 的写在 §三，留长尾/待裁决的写在 §四（**附不进的理由**）。
> 本文件**不排期、不写攻关步骤**（那是每个工作项自己的事）。

---

## 〇、判定口径与性价比框架

### 0.1 「是否仍然存在」的三档

| 档 | 判据 |
|---|---|
| **仍存在** | 源码事实与报告描述一致，或报告描述失真但**缺口本身仍在** |
| **部分变化** | 缺口的一部分已消失/已实现，另一部分仍在 |
| **已消失** | 该项所描述的缺口在当前仓库里**找不到对应物** |

> ⚠️ **「报告描述失真」≠「问题已解决」**——本轮查出 4 处失真（D7 / D8 / 077 / D10 的 git 证据），
> 其中 3 处失真的**同时**还带着真缺口。失真与缺口**分开记账**（§五）。

### 0.2 性价比的四把尺子

| 尺子 | 含义 |
|---|---|
| ① **判红体系** | 修它会不会改 F11QA 的 `grade` / `pass_to_fail` / 冻结基线？ |
| ② **用户可感** | 真机上有没有可见的行为差异（画面/声音/输入/不崩）？ |
| ③ **解锁** | 修它是否解锁别的项？ |
| ④ **防呆** | 不修它，后人会不会被过期描述误导、重复踩坑？ |

**成本** = 实现工时（人日）+ 外部依赖（硬件 / 业主裁决 / 协议文档 / 真机介质）。

### 0.3 ⚠️ 本计划最重要的一条前提

> **本计划里几乎所有精度项的「判红体系收益 = 0」。**
> `kgmqa-037` / `038` / `049` / `077` / `081` / `099` / `107` / `108` 在 `tests/tests.json`
> 里**全是 `failure_means: advisory` 且冻结基线为 `false`** ⇒ **修它们不改 grade、不改
> `pass_to_fail`**（AGENTS.md 业主 2026-11-08 裁决的原文）。
> ⇒ **v2.0.4 的决策依据只能是 ②③④ 与成本，不是「红不红」**。
> ⇒ 反过来，`policy.r4_gate_thresholds.fail_to_fail_max = 0` ⇒ **任何一项做坏了都是红灯**，
> 这就是「成本」一栏要写得那么细的原因。

---

## 一、复核方法与证据纪律

- **复核基线**：`main` @ `101026b`；v2.0.3 期间的 24 个提交全部是 GBA 卡带兼容性线（S1–S3）、
  FC 侧 loader/mapper 修复（S4/S5：`src/ines_load.cpp` / `src/boards/` / `fceux11-formats/`）
  与文档 ⇒ **NES 侧的这些残留项基本未被触碰**（这是推测，下面每项都有独立证据）。
- **取证方式**：逐项回源码（file:line）+ `git log` 提交史 + 清单原文抄录；
  **不接受「读文档得出结论」**，也不接受「推测 v2.0.3 没碰所以还在」。
- **一处实跑**（本轮唯一的行为级取证，见 §三 V1）：F-1 的复现用无头 Lua runner 实跑，
  **门禁里的两个 Lua 用例同时跑通作阴性对照** ⇒ 证明「门禁对它全盲」而不是「脚本写错了」。
- ⚠️ **本轮未做**：F11QA 迁移矩阵的**全量实跑**（本机无矩阵入口；现存产物
  `build/f11qa_migration_matrix.json` 的 `run_id = 20261110-060102-3d4144`、
  `git_rev = 502150e`（v2.0.3 r5），**早于发布提交 `45fec33`** ⇒ 它**不是 HEAD 的读数**，
  本文件引用它时一律注明这一点。

---

## 二、总表：17 个待核点的现状

| # | 报告里的项 | 现状 | 一句话结论 |
|---|---|---|---|
| 1 | **A1** 手工 GUI 验收 | **仍存在** | 21 行勾选表**自建档起从未被改过**；走查单也在、结果列全空 |
| 2 | **A4-1** 否决结论的护栏 | **仍存在** | `src/video.cpp` 里**没有任何** A4/否决/消费计数 的注释；64 KB/帧 memcpy 仍无门控 |
| 3 | **A7** 指令 handler 别名 | **仍存在** | `ops_table.inc` 的 `(void)xp;` 精确计数 **256** |
| 4 | **B2(b)** `FCEU_SoundCPUHook` 探点 | **仍存在** | 该函数体内**零** `FCEU_PROFILE_FUNC`；省略**仍未登记** |
| 5 | **B2(d)** profiler 的 Qt 菜单项 | **仍存在** | Qt 侧只有 include 与 `FCEU_profiler_log_thread_activity()`，**无 QAction/菜单** |
| 6 | **D1/037** 中断模型 S3–S5 | **仍存在** | S3/S4/S5 **零实现痕迹**；`src/x6502.cpp` 末次提交停在 S2.5 阶段 3 |
| 7 | **D3/049** `$2007` 读缓冲 M1–M3 | **仍存在** | U1/U2 均未实现（渲染期零 `VRAMBuffer` 赋值；衰减仍是整字节归零） |
| 8 | **D7/081** FDS IRQ 状态机 | **仍存在**（⚠️ 报告描述失真） | **实现已在**（含锁测试），缺的是**调查页/边沿探针/过测** |
| 9 | **D9/107** SNES 鼠标端到端 0xFC | **仍存在** | 协议四缺陷修复在位；0xFC **零实现零测试** |
| 10 | **D10/108** FC 麦克风协议 | **仍存在** | `src/input.cpp:130-140` **逐字未改**，仍是 1996 年的粗实现；全仓无麦克风锁测试 |
| 11 | **D8/099** PAL/Dendy | **仍存在**（⚠️ 报告描述失真） | 上游制式机制**已在**；缺的是**业主裁决 + 统一制式表 + per-region PPU 点数表** |
| 12 | **PGO** 从未评估 | **仍存在** | 两个 CMake 开关在、默认 OFF；**无任何本地读数**（v1.14 只建过选项） |
| 13 | **038** instr_misc | **仍存在** | `set_db()` **仍零调用点**；`LDA/STA (z,x)` 仍无 dummy read / 跨页周期 |
| 14 | **077** holy_mapperel | **仍存在**（⚠️ 报告描述失真） | 聚合 runner 与 47/47 合取判定 **v1.8 就有**；缺的是 **ROM 本体不在库** + 子组度量 |
| 15 | **R5** 清单/文档过期描述 | **部分存在** | 5 条 `known_limit` 逐字未改；基线仍 2026-09-26；死代码 `add_test` 还在 |
| 16 | **R5** v2.0.2 计划的 4 处过期行 | **已消失** | 归档重组时已修（a–d 四处） |
| 17 | **R5** `long-term-evolution/` 残留引用 | **已消失** | 目录整体不存在；「108P-11F」文本无第二处残留 |

**外加一条报告未列、但实测仍在的项** ⇒ 进 §三 V1：

| # | 项 | 现状 |
|---|---|---|
| 18 | **F-1** Lua `emu.frameadvance()` abort | **仍存在，且本轮实跑复现**（`0xC0000409`）；**门禁对它全盲** |

---

## 三、进 v2.0.4 的项（逐项）

> 排序 = **建议的执行顺序**（不是工期表）。

### V1 · F-1：Lua `emu.frameadvance()` 的 abort —— 建议**第一个做**

- **现状（本轮实跑）**：用仓库现成的无头 runner 实跑**从未接入门禁**的脚本
  `tests/lua_scripts/test_frameadvance.lua` ⇒ **退出码 `0xC0000409`**
  （`STATUS_STACK_BUFFER_OVERRUN`，即 `panic in a function that cannot unwind` 的 abort），
  且脚本**一句输出都没有** ⇒ 挂在第一次 `emu.frameadvance()`。
- **阴性对照（证明不是脚本写错）**：同一 runner 跑**已接进门禁**的两个脚本
  （`tests/lua_scripts/test_emu.lua`、`test_bit.lua`）⇒ **exit 0 全绿**。
  差别在于门禁那份 `test_emu.lua:48` **只断言 `type(emu.frameadvance) == "function"`，
  从不调用它** ⇒ **门禁对这个 abort 完全瞎**。
- **根因**：`emu.frameadvance()` 的绑定在 **Rust** 侧
  （`src/rust/crates/fceux11-lua/src/bindings/emu.rs:85`，走 `mlua::Thread::yield`），
  而它是被 **C++ 跨边界调进来**的 ⇒ unwind 不能穿过 `extern "C"` 帧。
  2026-06-21 起未改（业主 2026-11-08 裁决转长尾）。
- **成本**：**中**（改 yield 的发生位置，让「让出协程」发生在能 unwind 的一侧；
  Rust 侧改动，不触碰 NES 行为）⇒ **验收脚本现成**（上面那个未接线的脚本）。
- **成效**：① 消掉一个 **abort 级崩溃**（任何用户 Lua 脚本都可能直接崩掉模拟器）；
  ② **解锁 V2**（A1 的 `break_asap` 门腿与 A1-b 反向项都卡在它）；
  ③ 把一个**门禁盲区变成覆盖**（把 `test_frameadvance.lua` 接进 F11QA 矩阵）。
- **判据**：脚本实跑 exit 0 且 `LUA_RESULT` 标记为通过；**且该脚本进入门禁路径**
  （`tests/tests.json` 增一条用例，`failure_means: blocking`）⇒ 盲区转覆盖。

### V2 · A1：手工 GUI 验收（21 行 checklist + 18 项走查单）

- **现状**：`docs/tech/debug-regression-checklist.md:727-747` 的勾选表 **21 行全部未勾**
  （每行「结果」列仍是 `□绿 □红 □未验证`，日期/备注列全空；全文 `[x]` 计数 = **0**）；
  `git log --all -S '| B-1 | 普通断点'` **只命中建档那一次提交** ⇒ **自建档起从未被改过**。
  配套走查单 `docs/tech/a1_gui_walkthrough.md`（96 行）存在，各组结果列同样全空。
  ⚠️ **一处计数滑动**：checklist `:774` 自称「合计 **22** 个可勾选项」，表里实际 **21** 行
  （`a1_gui_walkthrough.md:5` 已记录此事）⇒ 本文件按 **21 行**计。
- **成本**：**业主人工一次 GUI 会话 2–3 小时**（不占开发工时）；**前置 = V1**
  （LUA-1/LUA-2 被 F-1 阻断，走查时按「因 F-1 阻断」标注，不计为本项失败）。
- **成效**：代码已 100% 落地并验证，缺的是**人工验收**；ctest **抓不到**这类 GUI 回归。
- **判据**：21 行全部有「绿/红 + 日期 + 测试人」；走查单 18 项逐项结论。
- ⚠️ 走查时**不应判红**的一处已知行为变化：`vblankScanLines` / `vblankPixel` 的计算位于门内
  （`src/debug.cpp:921-929`）⇒ 零断点时调试器 PPU 页签的 vblank 读数停在旧值。

### V3 · A4-1 与 B2(b)：两处「否决结论没进代码」的护栏

- **现状**：① `src/video.cpp` 全文 grep `A4|否决|盘点|chunk|谓词|消费` **只命中一条无关的
  PNG chunk 注释**（`:502`）；而 `:253-254` 仍是 `if(!fceu11::IsEmulationPaused())` +
  `memcpy(XBackBuf, XBuf, 256*256);`（**64 KB/帧、无消费计数门控**）⇒ **它看起来该被优化**，
  而 A4-1 当初被盘点否决（理由：savestate chunk-8 任意帧异步消费 ⇒ 金标漂移、谓词恒真）
  **只活在文档里**。② `src/sound.cpp` 的 `FCEU_SoundCPUHook`（`:767`）函数体**零探点**
  （每指令热路径，按 R4 裁决**有意省略**）⇒ 但计划里仍列着 5 个探点，后人会去找。
- **成本**：**分钟级纯文本**（两处注释/登记）。
- **成效**：④ 防呆 —— 阻止两轮「看起来该做」的重复劳动。

### V4 · R5：清单与文档的过期描述（一批纯文本修订）

- **仍存在的具体条目**：
  - `tests/tests.json` 的 **5 条 `known_limit` 逐字未改**（037 / 038 / 056 / 078 / 107），
    其中 037 那条「hijack not modeled」**与事实相反**（hijack 早已落地：
    `src/x6502.cpp:400` `g_intl_pending`、`:848` `X6502_NmiHijacked()`）；
  - **`docs/tech/cpu.md:73-74`** 也留着同一句过期描述，**且与同文件 `:30-33` 自相矛盾**
    （报告未点名这一处）；
  - 冻结基线仍是 `generated_at = 2026-09-26` / **121** 条，而 `tests/tests.json` 已是 **122** 条；
    049/050/051 三条 FAIL→PASS 漂移的 `approved` 位**仍在矩阵产物里是 `false`**；
  - `docs/tech/F11QA.md:12` 与 `docs/tech/f11qa-accuracy-backlog.md:3` 仍写
    「内部逻辑检测 **42P / 0F**」⇒ **该条不变式本身已过时**（实测 A 类 42 pass / 1 fail，
    那 1 个是 `kgmqa-115-bench-tolerance`，按业主 2026-11-07 裁决已移出精度判定、不判红）
    ⇒ **接手人若按字面「红灯即停」，会对着一个设计上就不该判红的用例反复排查**；
  - `src/tests/CMakeLists.txt:65` 仍声明 `add_test(NAME rom_regression_test ...)`，
    而 `src/CMakeLists.txt` **无 `add_subdirectory(tests)`** ⇒ 该夹具是**死代码**
    （同批 780 条金标由 Rust 夹具 `rom_regression_rust_smoke`（ctest #35）覆盖且在门禁路径上）。
- **成本**：**纯文本**；且 `known_limit` **不参与任何机械判定**
  （`grade.rs::compute_grade` 只读 `failure_means` / `pass_to_fail` / `new_test` /
  `test_set_diff` / `summary.failed`）⇒ **改动不触碰门禁读数**。
- ⚠️ **不得并入本批**：049/050/051 的**基线翻转**（`approved`）——那属期望值变更，按 AGENTS.md
  r15 口径需同级评审；且 **049 的 PASS 建立在帧预算放宽而非精度修复之上**，翻它会让 049 看起来
  像「精度已达标」。
- **成本**：分钟级；**成效**：④ 防呆（这是本批唯一的收益，但成本近乎为零）。

### V5 · PGO：把「从未评估」变成读数

- **现状**：`CMakeLists.txt:99-100` 两个开关在（`FCEUX11_PGO` / `FCEUX11_PGO_USE`，默认 OFF），
  整块逻辑 `:95-120`（含互斥 `FATAL_ERROR`、`/GENPROFILE` 与 `/USEPROFILE` 两段）**逐字未动**；
  仓库里关于 PGO 的记录**全是「从未动/从未评估」的转述，无任何性能读数**。
  ⚠️ **历史出处补注**：v1.14（`779d12d`）曾把「建立 PGO 训练场景 + PGO 作为可选构建配置」
  标为已完成 ⇒ 准确表述是「**选项已建、训练场景已建，但本仓从未产出过任何 PGO 性能读数**」。
- **成本**：**低**（instrumented 构建 → 训练场景（v1.14 的 SMB3 60s / Akumajou 30s / Zelda 30s
  已有清单）→ use 构建 → CPU 基准对比）。⚠️ 代价是**两轮 Release 构建 + 基准机时**。
- **成效**：CPU 性能可能显著改善（v2.0.2 的 A6 被裁决关闭时，PGO 与 A7 被并列为
  「从未动过的两个更便宜替代」⇒ A6 关闭后它是剩下的最便宜选项）；且**把「从未评估」这句永久消掉**。
- **判据**：产出**一组可复现的读数**（训练场景、构建命令、基准表、结论：采用/不采用/条件采用）。

### V6 · D10/108：FC 麦克风协议（性价比最高的「实体功能」项）

- **现状**：`src/input.cpp:130-140` **逐字未改**（末次提交 `88dd595` / 2026-07-10）：
  ```c
  // Not verified against hardware.
  if (replaceP2StartWithMicrophone) {
      if (joy[1]&8) {
          microphone = !microphone;
          if (microphone) { ret|=4; …
  ```
  逻辑 = 按 `joy[1]&8` 翻一个布尔量再 `ret|=4`（**单一 bit2**，非硬件采样位 2/3 协议）；
  **全仓无麦克风锁测试**（`tests/f11qa/core/` 下只有 `input_snesmouse_test.cpp` 等，无 mic 相关）。
- **成本**：**中**，但**有完整先例**：SNES 鼠标那次（`05bfc1f`）= 协议四缺陷修复
  （`src/input/snesmouse.cpp`）+ 桥注入面（`src/f11qa_bridge.*`、manifest `sut_input`）
  + T1–T6 锁测试（`tests/f11qa/core/input_snesmouse_test.cpp`）**一次提交的量**。
- **成效**：② 用户可感 —— FC 麦克风是**真实周边**（自带的实现连硬件都没验过）；
  ① 判红体系收益为 0（advisory + 基线 False）。
- **判据**：协议按硬件文档实现 + 锁测试（T1–T6 同款）+ 桥注入面接进 manifest。

### V7 · B2(d)：profiler 的 Qt 菜单项

- **现状**：Qt 侧只有 `#include "../../profiler.h"`（11 处）与
  `FCEU_profiler_log_thread_activity()`（`ConsoleWindow.cpp:742-743`、
  `fceuWrapper.cpp:1535-1536/1551-1552`）⇒ **无 QAction / 无菜单项**；
  `FCEU_profiler_report(FILE*)`（`src/profiler.h:161`）全仓只有 `src/fceu.cpp:707` 一处调用
  （退出时打 stdout）⇒ **没有 UI 触发点**。
- **成本**：**小**（一处菜单动作 + 信号槽）。
- **成效**：② 开发期可按需产出 profiler 报告（现在只能在退出时看 stdout）。
- ⚠️ 报告的「探点 4/5」口径**已过时**：现状是 **6 个探点**
  （`fceu.cpp:804` / `ppu_rendering.cpp:301,686` / `sdl-sound.cpp:376` /
  `ConsoleWindow.cpp:629,666`）⇒ 本项的缺口只剩「Qt 菜单」与「SoundCPUHook 探点（有意省略）」。

### V8 · A7 的「收益上限实验」（只做实验，不做本体）

- **现状**：`src/ops_table.inc` 的 `(void)xp;` **精确计数 256**（全部 handler）；
  `src/x6502abbrev.h:11` `#define _X g_cpu.native_layout().X` 仍在；
  `src/cpu.cpp:14-15` 的 `static_assert(offsetof(Cpu, layout_) == 0 …)` 仍在
  ⇒ **savestate 布局不得动**这条硬约束不变。
- **本项只做**：在一个**隔离分支**上把部分 handler 改别名 ⇒ 跑 CPU 基准 ⇒ **量出收益上限**
  ⇒ 给出「值得做 / 不值得做 / 只做 X 子集」的**读数结论**。
- **成本**：**低–中**（一次分支 + 基准机时），**不动主线**。
- **成效**：把「A7 值不值得」从**猜测**变成**读数**（报告与归档里它一直是「从未动工」的定性描述）。
- ⚠️ **本体（A7 全面别名化）不进 v2.0.4**（见 §四 L1）——它要改 256 个 handler、
  且 savestate 布局不得动，**在收益上限未知之前不值得**。

---

## 四、不进 v2.0.4 的项（逐项，附理由）

> 「不进」不是「永远不做」——每条都写了**什么条件成立时它才该被重新立项**。

| # | 项 | 不进的理由 | 重新立项的条件 |
|---|---|---|---|
| **L1** | **A7 本体**（256 个 handler 别名） | 成本**大**（256 个 handler × savestate 布局不得动 × 全量回归）；**收益至今没有任何读数** | 先做完 V8 的收益上限实验；读数为正且能定位到「哪些 handler 值得改」时再立项 |
| **L2** | **D1/037** 中断模型 S3–S5 | 成本**大**（三个模型扩展 + DMC 计费章程 + branch-delay）；①判红体系**零收益**（advisory + 基线 False）；且 **077 依赖它** ⇒ 更该等 | 出现「NES 精度专项攻坚」的独立立项，或有人愿意为 blargg 中断类 ROM 的精度单独投入 |
| **L3** | **D3/049** `$2007` 读缓冲 M1–M3 | 同上；且该用例的 PASS **来自帧预算放宽**（600 → 2000，manifest provenance 已写明）⇒ 「PASS」不等于「读缓冲做对了」 | 同 L2；或有人愿意把 049 的 PASS 换成真精度 PASS（需同时处理基线翻转的评审） |
| **L4** | **D7/081** FDS IRQ（Path B 的 B1/B2/B3） | ① **实现已在**（`src/fds.cpp:97,134-141` + `src/rust/crates/fceux11-formats/src/fds.rs:299-317`，含锁测试）⇒ 缺的是**调查页 / IRQ 边沿探针 / 真机过测**；② 真机过测**需要 FDS 介质**（外部依赖）；③ 判红体系零收益 | 拿到 FDS 介质与真机环境；或业主认为 FDS 支持值得正式投入 |
| **L5** | **D9/107** SNES 鼠标端到端 0xFC | 需要**真实鼠标硬件**交互 ⇒ **无法无人值守验收**；协议四缺陷已修，剩下的是端到端 | 拿到 SNES 鼠标（或可模拟 0xFC 握手的输入注入面） |
| **L6** | **038** instr_misc | 业主已 defer；根因明确（`set_db()` 零调用点 ⇒ open bus 是死的；`LDA/STA (z,x)` 无 dummy read / 跨页周期）但成本中、判红体系零收益 | 同 L2；或有人做「(z,x) 寻址的 dummy read + 跨页周期」专项（它同时会牵动 038 的 `GetIX`） |
| **L7** | **D8/099** PAL/Dendy | **待业主产品裁决**（主题外扩）；且报告「全部未做」的描述**不准确**（上游机制已在）⇒ 先更正描述再谈做不做 | 业主明确「正式支持 PAL/Dendy」；届时仍缺的是**统一制式表接到 core 配置**与 **per-region PPU 点数表**（现各区固定 341 点/行） |

---

## 五、报告自身需要更正的地方（连带事实偏差）

> 这些**不是 v2.0.4 的工作项**，但「不修就会误导下一个接手的人」⇒ 并入 V4 的纯文本批次。

| # | 报告的位置 | 偏差 |
|---|---|---|
| 1 | §1.7 D7「Path B —— FDS IRQ 状态机（磁盘传输 / 定时器建模）**未做**」 | **不准确**：完整状态机与锁测试已在仓库里（见 §四 L4） |
| 2 | §1.10 D8「制式表（CPU Hz / PPU 点 / 扫描线 / 帧）… **全部未做**」 | **不准确**：扫描线（`ppu_core.cpp:69-81` 262/312）、CPU Hz 常量（`x6502.h:90-92`）、region UI + `SDL.PAL` 配置项（`ConsoleVideoConf.cpp:139-143`、`config.cpp:531`）**都已存在**；真正缺的是产品裁决 + 统一制式表 + per-region PPU 点数表 |
| 3 | §二 行 190「077 = **未开始**（47 子 ROM 聚合）」 | **措辞不精确**：聚合 runner 与 47/47 合取判定 **v1.8 就有**（`rom_protocol.rs:17,20`、`rom_runner.rs:308-386`）；缺的是 **47 个 ROM 本体不在库**（本地聚合是 exit-2 环境失败）+ 子组度量 |
| 4 | §1.9「`src/input.cpp` 自 2026-09-28 起唯一提交 `05bfc1f`」 | **提交号错**：`05bfc1f` 没有改 `src/input.cpp`；该文件末次提交是 `88dd595`（2026-07-10）。**结论不变**（麦克风代码零改动） |
| 5 | §3.2 起的「v2.0.2 计划 §二/§四/§十」行号坐标 | **已失效**：v2.0.2 计划已被重组为 `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`（§〇–§六 + 附录 A，**无 §十**）⇒ 坐标须重映射到归档 §2.1 / §2.3 / §三 |
| 6 | §1.1 提到的 `kgmqa-081-fds-irq-bisqwit` | **树内无此 id**；实际是 `kgmqa-081-fds-irq-tests-sour`，且其 `known_limit` 已在 2026-11-07 改写成「BIOS-gated … NOT an FDS IRQ accuracy defect」 |
| 7 | §1.9 之外：D1 的「(C) DMC 计费范围界定」 | 仍**未落地**（v2.0.3 期间零 CPU 侧提交）；⚠️ `src/ppu.cpp:1506` 的 “DMC arbitration … separate charter” 属 **DMA 仲裁**（042 用例），**与本章程不是同一件事** ⇒ 别被它误导成「已有人做」 |
| 8 | 报告 §三.3 的定位 | 049/050/051 的 `baseline_drift.approved` **不在** `f11qa_baseline_frozen.json` 里，而在**矩阵产物** `build/f11qa_migration_matrix.json:651-671`（三条均 `approved: false`）⇒ 「在哪个文件里」这一点要说准 |

---

## 六、与 v2.0.3 线的衔接

- v2.0.3 线（GBA 卡带兼容性）**自身**仍有未完项，**不在本计划范围内**，此处只做指针：
  - BIOS SWI 声音家族 `0x1A`–`0x1E` / `0x20`–`0x24` / `0x28` / `0x29` / `0x2A` 维持兜底，
    `0x26` HardReset 结构障碍挂起；
  - S3-b / S3-c / S3-d / S3-e 已在 v2.0.3 收口（r16–r19），其中 **FireRed 的 init 竞态根因未解**
    （是**越过**不是修好，长尾保留）；
  - 4 张失败卡带里**仅 FireRed 经业主 2026-11-12 人工复核确认兼容**，其余 3 张
    （Mario Kart SC / Dragon Ball / Minish Cap）**机器判据全过、人工未确认**。
  - 详见 `docs/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md` §九 r16–r20。
- ⚠️ **两条线的红线不要混**：本计划全部是 **NES 侧**；v2.0.3 线的 GBA 侧工作**不阻塞**本计划，
  反之亦然。

---

## 七、v2.0.4 出口判据

- [ ] **V1**：`tests/lua_scripts/test_frameadvance.lua` 实跑 exit 0（不再 `0xC0000409`），
      **且该脚本进入门禁路径**（manifest 增一条、`failure_means: blocking`）；
- [ ] **V2**：A1 的 21 行勾选表**全部**有「绿/红 + 日期 + 测试人」；
      走查单 18 项逐项结论；F-1 阻断项按 V1 的结论标注；
- [ ] **V3**：`src/video.cpp` 的 A4-1 否决护栏注释在位；
      `FCEU_SoundCPUHook` 探点的**有意省略**已在计划/CHANGELOG 登记；
- [ ] **V4**：`tests/tests.json` 的 5 条 `known_limit` 改为与事实相符；
      `docs/tech/cpu.md` / `F11QA.md` / `f11qa-accuracy-backlog.md` 的过期措辞已改；
      `src/tests/CMakeLists.txt:65` 的死代码 `add_test` 已处理；
      **049/050/051 的基线翻转未被顺手做掉**（需同级评审）；
- [ ] **V5**：PGO 产出**一组可复现读数**（训练场景 + 构建命令 + 基准表 + 结论）；
- [ ] **V6**：FC 麦克风协议按硬件实现 + 锁测试 + 桥注入面接进 manifest；
- [ ] **V7**：Qt 侧 profiler 菜单动作在位，且能产出报告；
- [ ] **V8**：A7 收益上限实验给出**读数结论**（值得/不值得/只做哪子集）；
- [ ] **NES 零回归**：`ctest -LE perf` **36 项全绿** + F11QA 矩阵 `pass_to_fail = 0` + grade ≥ B
      （矩阵**本地无入口** ⇒ 以 CI 复核为准）；
- [ ] 四条 Rust 锁测试门禁全绿（root / vendor / debug / media）。

---

## 八、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（不变式 1）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# V1 的行为级复现（当前必红：0xC0000409）
& build\tests\f11qa_lua_runner.exe tests\lua_scripts\test_frameadvance.lua --frames 60
# 阴性对照（当前必绿）
& build\tests\f11qa_lua_runner.exe tests\lua_scripts\test_emu.lua --frames 60

# 四条 Rust 锁测试
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"

# F11QA 迁移矩阵（122 用例；pass_to_fail 必须为空）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo build --release -p f11qa --bin f11qa-runner"
& src\rust\target\x86_64-pc-windows-msvc\release\f11qa-runner.exe --manifest tests\tests.json --bin-dir build\tests --output build\f11qa_migration_matrix.json --accuracy-table build\f11qa_accuracy_table.md --known-fail tests\fixtures\blargg_known_fail.json --baseline tests\fixtures\f11qa_baseline_frozen.json --save-baseline build\f11qa_baseline_next.json

# PGO 评估（V5）：三段式
cmake -S . -B build-pgo-gen -DCMAKE_BUILD_TYPE=Release -DFCEUX11_PGO=ON
cmake --build build-pgo-gen --config Release
#   （用训练场景跑出 profile）
cmake -S . -B build-pgo-use -DCMAKE_BUILD_TYPE=Release -DFCEUX11_PGO_USE=ON
cmake --build build-pgo-use --config Release
```

> ⚠️ `f11qa_lua_runner.exe` 在 `build\tests\`（CMake 目标），与 `f11qa-rom-runner`（Cargo bin，
> 在 `src\rust\target\…\`）**不是同一个东西**。
> ⚠️ 跑 `ctest` 时不要与构建/其它测试并行（会污染测量，AGENTS.md r54②）。

---

## 九、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-12 | **立项（v2.0.3 收口后的残留项复核 + 性价比裁决）—— 本轮只写文档，未动任何生产代码。** ① **复核对象**：`docs/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（2026-11-08）的 17 个待核点 + 1 条报告未列但实测仍在的项（F-1）。② **复核结论**：**13 项仍存在**（A1 / A4-1 / A7 / B2(b) / B2(d) / D1 / D3 / D7 / D9 / D10 / D8 / PGO / 038 / 077）、**1 项部分存在**（R5 清单与文档过期描述）、**2 项已消失**（v2.0.2 计划的 4 处过期行已随归档重组修掉；`long-term-evolution/` 目录整体不存在）；**外加 F-1 仍复现**（本轮实跑 `0xC0000409`，且已接线的两个 Lua 用例 exit 0 ⇒ 门禁对它全盲）。③ **性价比裁决**：**8 项进 v2.0.4**（V1 F-1 → V2 A1 → V3 护栏 → V4 文档批次 → V5 PGO → V6 麦克风 → V7 profiler 菜单 → V8 A7 收益上限实验），**7 项不进**（L1 A7 本体 / L2 D1 / L3 D3 / L4 D7 / L5 D9 / L6 038 / L7 D8），每条附不进理由与重新立项条件。④ **裁决的关键前提**：本计划几乎所有精度项的**判红体系收益 = 0**（advisory + 冻结基线 False）⇒ 决策依据只能是「用户可感 / 解锁 / 防呆 / 成本」。⑤ **报告自身的 8 处事实偏差**已登记（§五），其中 D7 / D8 / 077 三处属「描述失真但缺口仍在」⇒ 失真与缺口**分开记账**。⑥ **一处时效性提醒**：现存矩阵产物 `build/f11qa_migration_matrix.json` 的 `git_rev = 502150e`（v2.0.3 r5），**早于发布提交 `45fec33`** ⇒ 它不是 HEAD 的读数，引用时须注明。⑦ **未做**：F11QA 矩阵全量实跑（本地无入口）；F-1 的**修复方案设计**（本计划只立项，不写攻关步骤）。 |
