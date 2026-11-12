# FCEUX11 v2.0.4 — 残留精度与调试待办 · 构建计划

> **STATUS: 在途**（2026-11-12 立项为「遗留项计划」，同日按业主指令**升格为 v2.0.4 构建计划**：
> 删除不纳入 v2.0.4 的项目，对确认进入的 V1–V8 逐项研究解决方案。工作分支 `wip204`。）
> **来源**：`docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（2026-11-08 产出，快照冻结）
> **复核基准**：`main` @ `101026b`（v2.0.3 发布提交 `45fec33` 之后）
> **本文件的性质**：v2.0.4 的**构建计划**——每项带「目标 / 现状坐标 / 解决方案步骤 / 验收 / 风险与非目标」。
> 前身「遗留项计划」的复核结论与归档整理记录保留在 §七 变更记录（r1/r2），**内容冻结不再续写**。
> **范围裁决（2026-11-12 业主指令）**：不纳入 v2.0.4 的项目（A7 本体 / D1/037 / D3/049 / D7/081 /
> D9/107 / 038 / D8/099，原 §四 L1–L7）**已从本计划删除**；其不进理由与重新立项条件见 git 历史
> （本文件 r2 及之前的版本）与 review 报告。本计划只含 **V1–V8**。

---

## 〇、判定口径与前提

### 0.1 事实档位（沿用复核口径）

| 档 | 判据 |
|---|---|
| **仍存在** | 源码事实与报告描述一致，或报告描述失真但**缺口本身仍在** |
| **部分变化** | 缺口的一部分已消失/已实现，另一部分仍在 |
| **已消失** | 该项所描述的缺口在当前仓库里**找不到对应物** |

> ⚠️ **「报告描述失真」≠「问题已解决」**——失真与缺口**分开记账**（V4 附表）。
> 升格后新增一处失真更正：V1 的根因（前身文档写「`mlua::Thread::yield` + unwind 穿不过
> `extern "C"` 帧」）经本轮实测**更正**为 Lua 5.1 的 yield-across-C-call 机制 + `panic="abort"`
> （见 V1·根因）；缺口仍在。

### 0.2 ⚠️ 本计划最重要的一条前提

> **本计划里几乎所有精度项的「判红体系收益 = 0」。**
> `kgmqa-037` / `038` / `049` / `077` / `081` / `099` / `107` / `108` 在 `tests/tests.json`
> 里**全是 `failure_means: advisory` 且冻结基线为 `false`** ⇒ **修它们不改 grade、不改
> `pass_to_fail`**（AGENTS.md 业主 2026-11-08 裁决的原文）。
> ⇒ **v2.0.4 的决策依据只能是「用户可感 / 解锁 / 防呆 / 成本」，不是「红不红」**。
> ⇒ 反过来，`policy.r4_gate_thresholds.fail_to_fail_max = 0` ⇒ **任何一项做坏了都是红灯**。

### 0.3 证据纪律（沿用）

- **取证方式**：逐项回源码（file:line）+ `git log` 提交史 + 实跑；**不接受「读文档得出结论」**。
- 位级格式**不凭记忆写**——信源被拦时按「权威参照实现交叉」降级并标注未取证项（V6 即此例）。
- ⚠️ **时效性提醒**：现存矩阵产物 `build/f11qa_migration_matrix.json` 的
  `run_id = 20261110-060102-3d4144`、`git_rev = 502150e`（v2.0.3 r5），**早于发布提交 `45fec33`**
  ⇒ 它**不是 HEAD 的读数**，引用它时一律注明这一点。
- ⚠️ **本轮未做**：F11QA 迁移矩阵的全量实跑（本机无矩阵入口，以 CI 复核为准）。

---

## 一、任务总表（V1–V8）

> 排序 = **建议的执行顺序**（不是工期表）。

| # | 项 | 一句话现状 | 目标 | 验收锚点 |
|---|---|---|---|---|
| V1 | F-1 Lua `emu.frameadvance()` abort | 实跑复现 `0xC0000409`；门禁全盲 | 消掉 abort + 盲区转覆盖 | `test_frameadvance.lua` exit 0 + tests.json 新用例（blocking） |
| V2 | A1 手工 GUI 验收 | 21 行勾选表全空、走查单 18 项结果列全空 | 人工验收落章 | 21 行「绿/红+日期+测试人」+ 18 项结论 |
| V3 | A4-1 / B2(b) 两处护栏登记 | `video.cpp:253-254` 无护栏注释；`sound.cpp` 零探点且省略未登记 | 否决结论进代码 | 注释/登记在位（纯文本） |
| V4 | R5 过期文本修订批 | 5 条 `known_limit` 逐字未改；多处文档过期；死 `add_test` | 文本与事实相符 | diff 门禁 + grep 复核；不碰期望值 |
| V5 | PGO 评估 | 开关在、默认 OFF；**从未产出读数** | 一组可复现读数 + 结论 | 训练场景 + 构建命令 + 基准表 + 采用/不采用 |
| V6 | D10/108 FC 麦克风协议 | `input.cpp:130-140` 1996 年粗实现逐字未改 | 按硬件语义实现 + 测试面 | 锁测试 + 桥注入 + `kgmqa-108` 行为对照 |
| V7 | B2(d) profiler Qt 菜单 | 报告出口只有退出时 stdout；Qt 侧无 QAction | 开发期按需出报告 | 菜单动作在位、能产出报告 |
| V8 | A7 收益上限实验 | `(void)xp;` ×256；别名化收益无读数 | **只做实验**量出上限 | 隔离实验 + 基准读数结论 |

### 1.1 完成度看板（2026-11-13 追记，r6；明细见 §七 r4/r5）

| 项 | 状态 | 依据 |
|---|---|---|
| V1 F-1 | ✅ **完成**（出口判据达成） | 修复落地（纯 Lua 包装）+ runner exit 0 + kgmqa-123（blocking）+ 四门禁 299/249/93/52 + ctest 36/36（r5①②③）。附：GUI 侧暴露新缺陷（r5⑤），LUA-1/2 的 GUI 转绿归新缺陷工作项 |
| V2 A1 手工验收 | ✅ **完成**（出口判据达成） | 21 行全部有「结果+日期+测试人」+ 走查单 18 项逐项结论（16 绿 / 1 红 S-2 / 2 红=F-1 取证 / 2 未验证）（r4） |
| V3 护栏登记 | ⬜ 未开始 | |
| V4 文本修订批 | ⬜ 未开始 | 走查已顺手记录两处失真供本批吸收：走查单 §0.1 exe 路径过期（实际 `build/src/`）；断点关键字回归已修复（r5④）故清单 §4.5 描述恢复有效，§七「入口不符」家族增补已修复注记即可 |
| V5 PGO | ⬜ 未开始 | |
| V6 麦克风 | ⬜ 未开始 | |
| V7 profiler 菜单 | ⬜ 未开始 | |
| V8 A7 实验 | ⬜ 未开始 | |
| **计划外 A** | ✅ **完成** | 断点关键字死代码回归（v0.3.5 `704078f` 引入）修复 + 5 断言锁测试（r5④）——走查 NSF-1 带出 |
| **计划外 B** | 🔴 **新缺陷立项** | GUI 下任何 yielding Lua 脚本运行中崩溃（0xC00000FD/0xC000041D，时长随机 2s～68s+，堆 ~4MB/s 增长，突发深递归）；V1 修复所暴露，阻断 LUA-1/2 GUI 转绿（r5⑤）；需 cdb/WinDbg 或 mlua 内部分析 |
| **计划外 C** | 🔴 **回归候选挂起** | S-2：Step Over 的 watchpoint[64] 长程不消费（r4③）；静态调查榨干（r5⑧），§6.3 门回退变异测试排队（GUI 崩溃缺陷关闭后做） |

**交付批次视角**：批 1（V1→V6）= V1 ✅ / V6 ⬜；批 2（V3→V4）= ⬜⬜；批 3（V5/V7/V8）= ⬜⬜⬜；插入 V2 = ✅。

---

## 二、逐项解决方案

### V1 · F-1：Lua `emu.frameadvance()` 的 abort

**目标**：消掉 abort 级崩溃；把门禁盲区转成覆盖；解锁 V2 的 LUA-1/LUA-2 走查腿。

**现状（已实测，本轮可复现）**：
- 无头 runner 实跑 `tests/lua_scripts/test_frameadvance.lua` ⇒ 退出码 `0xC0000409`
  （`STATUS_STACK_BUFFER_OVERRUN`，`panic in a function that cannot unwind` 的 abort），
  脚本零输出 ⇒ 挂在第一次 `emu.frameadvance()`。
- 阴性对照：同一 runner 跑 `tests/lua_scripts/test_emu.lua` / `test_bit.lua` ⇒ exit 0 全绿。
  门禁那份 `test_emu.lua:48` **只断言 `type(emu.frameadvance) == "function"`，从不调用它**
  ⇒ **门禁对这个 abort 完全瞎**。
- 普通 Lua 运行期错误（`error('boom')`）⇒ runner exit 1 优雅退出 ⇒ **abort 只在 yield 路径**。

**根因（本轮实测钉死；更正前身文档的假设）**：
- `emu.frameadvance` 绑定在 `src/rust/crates/fceux11-lua/src/bindings/emu.rs:84-95`，
  在 Rust 回调内经 mlua `Function::call` 调 Lua `coroutine.yield`。
- mlua 0.10.5 的 `Function::call` → `lua_pcall` → `luaD_call` ⇒ **`nCcalls` 自增**；
  Lua 5.1（lua-src-547.0.0 / lua-5.1.5，`ldo.c`）的 `lua_yield` 有
  `L->nCcalls > L->baseCcalls` 检查 ⇒ **yield 必失败**，返回
  `RuntimeError("attempt to yield across metamethod/C-call boundary")`。
- 同一错误路径在测试 profile（cargo 强制 unwind）下优雅返回 `Err(CallbackError)`（debug/release
  双测过）；真实 runner 在 workspace `[profile.release] panic = "abort"`（`src/rust/Cargo.toml`）
  下**静默 abort** ⇒ `0xC0000409`。⚠️ abort 的精确位点未单步钉死（可选第一步：20 行
  `panic="abort"` 探针 bin 归因；不影响修复方案成立）。
- ⚠️ 前身文档的「走 `mlua::Thread::yield`、unwind 穿不过 `extern "C"` 帧」是**未证实假设**，
  与实测不符（错误是 Lua 层 `RuntimeError`，不是 unwind 穿界）；**缺口仍在**（失真与缺口分开记账）。

**解决方案步骤（已用探针实证可行性）**：
1. 把 yield 移到 Lua 层：注册 `emu.frameadvance` 后执行纯 Lua 包装
   `emu.frameadvance = function() return coroutine.yield() end`。
   实证：3 次挂起 / 续跑 / 收尾 `print(1)` 全部正常（探针两通过）。
2. 落点二选一（推荐 a）：(a) 在 fceux11-lua 注册处（`bindings/emu.rs`）注册完成后用
   `lua.load(...).exec()` 打包装饰，内聚在 crate 内；(b) runner 的 Lua 预载段注入。
3. `frame_advance_waiting` 由 `frame_boundary` 自身维护（`fceux11-lua/src/lib.rs:375/390`）
   ⇒ 包装**无需**再调 native 版本，原绑定体可退化为内部实现或移除。
4. 门禁接线：`tests/tests.json` 增一条用例（形状照 `test_bit`/`test_emu`，`:1352-1396`：
   `binary: f11qa_lua_runner, args: [script, "--frames", "60"]`, `expected.exit_code: 0`），
   `failure_means: blocking` ⇒ 盲区转覆盖。

**验收**：`f11qa_lua_runner tests\lua_scripts\test_frameadvance.lua --frames 60` exit 0 且有
脚本输出；新用例进 tests.json 后全量 `ctest -LE perf` 全绿。

**风险与非目标**：不改 C++；不动 `panic="abort"` profile；不碰 NES 行为；
Lua 包装改变了 yield 的发生层（Rust 回调内 → Lua 层），`coroutine.yield` 的参数透传语义保持。

---

### V2 · A1：手工 GUI 验收（21 行 checklist + 18 项走查单）

**目标**：代码已 100% 落地的部分补上人工验收落章（ctest 抓不到 GUI 回归）。

**现状**：`docs/tech/debug-regression-checklist.md:727-747` 勾选表 21 行全部未勾
（全文 `[x]` 计数 = 0；`git log --all -S '| B-1 | 普通断点'` 只命中建档那次提交）；
配套 `docs/tech/a1_gui_walkthrough.md`（96 行）18 项结果列全空。
⚠️ 计数滑动：checklist `:774` 自称「合计 22 个可勾选项」，表里实际 21 行
（`a1_gui_walkthrough.md:5` 已记录）⇒ 按 **21 行**计。

**解决方案步骤**：
1. **前置 = V1 落地**（LUA-1/LUA-2 被 F-1 阻断；V1 修完再走查这两项，否则按「因 F-1 阻断」标注）。
2. 业主一次 GUI 会话（2–3 小时，不占开发工时）：按 checklist 逐行勾「绿/红 + 日期 + 测试人」，
   走查单 18 项逐项结论。
3. ⚠️ 走查时**不应判红**的一处已知行为变化：`vblankScanLines` / `vblankPixel` 的计算位于门内
   （`src/debug.cpp:921-929`）⇒ 零断点时调试器 PPU 页签的 vblank 读数停在旧值。

**验收**：21 行全部有「绿/红 + 日期 + 测试人」；走查单 18 项逐项结论。

**风险与非目标**：本项是人工验收，不做代码改动；勾选结果落进文档即可。

---

### V3 · A4-1 与 B2(b)：两处「否决结论没进代码」的护栏

**目标**：把「看起来该做但已否决/有意省略」的结论写进代码与登记，阻止重复劳动（④ 防呆）。

**现状（本轮已核实）**：
- ① `src/video.cpp:253-254` 仍是 `if(!fceu11::IsEmulationPaused())` +
  `memcpy(XBackBuf, XBuf, 256*256);`（64 KB/帧、无消费计数门控）；全文 grep
  `A4|否决|盘点|chunk|谓词|消费` 只命中一条无关的 PNG chunk 注释（`:502`）
  ⇒ **A4-1 的否决理由只活在文档里**。⚠️ 前身文本写「有注释写明是有意省略」——
  **实测无任何注释**（审查文本自身的一处失真，已并入 V4 附表 #9）。
  否决理由（供注释引用）：savestate chunk-8 任意帧异步消费 ⇒ 金标漂移、谓词恒真。
- ② `src/sound.cpp` 的 `FCEU_SoundCPUHook`（`:767`）函数体**零** `FCEU_PROFILE_FUNC`
  （本轮全文 grep 实测零命中；每指令热路径，按 R4 裁决**有意省略**）⇒ 省略**仍未登记**，
  且旧「探点 4/5」口径已过时（现状 **6 个探点**，见 V7）。

**解决方案步骤**：
1. `src/video.cpp:253-254` 上方加 A4 否决护栏注释（写明否决理由 + 「勿再做消费计数门控」），
   形态与 fceu.cpp 里 A4 护栏注释同款；如加运行期护栏，与 A4 同形态（实施第一步先对照
   `src/profiler.h` 的 A4 护栏宏体——本轮未逐字取证其宏体，不得凭记忆写）。
2. `src/sound.cpp:767` 处登记「有意省略探点」（注释 + CHANGELOG 一句），并把探点清单口径
   修正为 6 个（V7 落地时一并在 CHANGELOG 提）。

**验收**：两处注释/登记在位；纯文本改动，`ctest -LE perf` 全绿。

**风险与非目标**：不动 `memcpy` 行为本身（A4 否决的正是优化它）；不给 sound.cpp 加探点。

---

### V4 · R5：清单与文档的过期描述（一批纯文本修订）

**目标**：让文本与事实相符，防后人被过期描述误导。**不改任何期望值/基线/判定**。

**解决方案步骤（逐条，坐标已实测落准）**：
1. `tests/tests.json` 的 **5 条 `known_limit`** 改为与事实相符（逐条与 review 报告核对后改写或删除；
   `known_limit` 不参与任何机械判定——`grade.rs::compute_grade` 只读 `failure_means` /
   `pass_to_fail` / `new_test` / `test_set_diff` / `summary.failed` ⇒ 不触碰门禁读数）：
   - `kgmqa-037`（tests.json:1069）：「hijack not modeled」**与事实相反**——hijack 已落地
     （`src/x6502.cpp:400` `g_intl_pending`、`:848` `X6502_NmiHijacked()`）；
   - `kgmqa-038`（:1109）、`kgmqa-056`（:1820）、`kgmqa-078`（:2665）、`kgmqa-107`（:3791）：
     按 review 报告口径改写（107 的 SNES 鼠标协议已由 `05bfc1f` 修掉四缺陷）。
2. `docs/tech/cpu.md:73-74` 的同一句过期描述（**与同文件 `:30-33` 自相矛盾**）删改；
   `docs/tech/F11QA.md:12` 与 `docs/tech/f11qa-accuracy-backlog.md:3` 的「内部逻辑检测 42P/0F」
   改为实测口径（A 类 42 pass / 1 fail，那 1 个是 `kgmqa-115-bench-tolerance`，按业主
   2026-11-07 裁决已移出精度判定、不判红）。
3. **死 `add_test`**：`src/tests/CMakeLists.txt`（68 行）**整份是死文件**——`src/CMakeLists.txt`
   只 `add_subdirectory(rust)`（:718），从不 include 它（本轮实测）；活的是 `tests/CMakeLists.txt`
   （822 行，`add_subdirectory(tests)` 注册）。处理：删 `:65` 的死 `add_test` 或整文件处置
   （同批 780 条金标由 Rust 夹具 `rom_regression_rust_smoke`（ctest #35）覆盖且在门禁路径上）。
4. **v1.8 计划 10 处失效引用**（该文件已移入 `docs/history/plans/`）：活的 3 处改路径——
   `docs/tech/f11qa-vendor-state.md:9`、`scripts/phase2_build_tests_json.py:957`（manifest 生成器）、
   `src/rust/crates/f11qa/src/manifest/parser.rs:39`（doc 注释示例）；另 `tests/tests.json:4098`
   的 `spec_source` 是生成器产物（**生成器与产物一起改**）。
   ⚠️ **不要改**：`docs/history/plans/FCEUX11-v1.8_F11QA-构建计划.md:1018,1061` 与
   `tests/fixtures/_mirror_snapshot/**` 里的同名引用（记录当初那次移动的历史文本）；
   6 处显式标注「已删除」的 v2.0.2 / `long-term-evolution/` 引用（归档文件 :4-5、
   `docs/history/README.md:67`、review 报告 :12-13）是**刻意保留的历史沿革**。
5. **冻结基线的计数差**（`generated_at = 2026-09-26` / 121 条 vs `tests/tests.json` 122 条）
   与 049/050/051 的 `approved: false`（在**矩阵产物** `build/f11qa_migration_matrix.json:651-671`，
   **不在** `f11qa_baseline_frozen.json`）：**只登记，不改**——基线重钉/翻转属期望值变更，
   需同级评审（AGENTS.md r15 口径）。
6. ⚠️ **不得并入本批**：049/050/051 的基线翻转（`approved`）。**049 的 PASS 建立在帧预算放宽
   （600 → 2000）而非精度修复之上**，翻它会让 049 看起来像「精度已达标」。

**验收**：上述条目逐条 grep 复核（过期措辞零命中、新措辞在位）；`ctest -LE perf` 全绿；
tests.json 的 diff **只含 `known_limit`/`spec_source` 文本行**（生成器同改的除外），
`expected` / `failure_means` 零改动。

**附 · review 报告自身的 8 处偏差登记**（更正以本表为准，冻结原文不动）：

| # | 报告的位置 | 偏差 |
|---|---|---|
| 1 | §1.7 D7「FDS IRQ 状态机**未做**」 | **不准确**：完整状态机与锁测试已在（`src/fds.cpp:97,134-141` + `fceux11-formats/src/fds.rs:299-317`） |
| 2 | §1.10 D8「制式表…**全部未做**」 | **不准确**：扫描线（`ppu_core.cpp:69-81`）、CPU Hz（`x6502.h:90-92`）、region UI + `SDL.PAL`（`ConsoleVideoConf.cpp:139-143`、`config.cpp:531`）都在；真正缺的是产品裁决 + 统一制式表 + per-region PPU 点数表 |
| 3 | §二 行 190「077 = **未开始**」 | **措辞不精确**：聚合 runner 与 47/47 合取判定 v1.8 就有（`rom_protocol.rs:17,20`、`rom_runner.rs:308-386`）；缺的是 ROM 本体不在库 + 子组度量 |
| 4 | §1.9「`src/input.cpp` 唯一提交 `05bfc1f`」 | **提交号错**：`05bfc1f` 没改 `src/input.cpp`；末次是 `88dd595`（2026-07-10）。**结论不变**（麦克风代码零改动） |
| 5 | §3.2 起的「v2.0.2 计划 §二/§四/§十」坐标 | **已失效**：v2.0.2 计划已重组为归档（无 §十）⇒ 坐标映射到归档 §2.1 / §2.3 / §三 |
| 6 | §1.1 的 `kgmqa-081-fds-irq-bisqwit` | **树内无此 id**；实际是 `kgmqa-081-fds-irq-tests-sour`，且其 `known_limit` 已于 2026-11-07 改写为「BIOS-gated … NOT an FDS IRQ accuracy defect」 |
| 7 | D1 的「DMC 计费范围界定」 | 仍未落地；⚠️ `src/ppu.cpp:1506` 的 “DMC arbitration … separate charter” 属 **DMA 仲裁**（042 用例），与本章程不是同一件事 |
| 8 | 报告 §三.3 的定位 | 049/050/051 的 `approved` 在**矩阵产物**里（`build/f11qa_migration_matrix.json:651-671`），不在 `f11qa_baseline_frozen.json` |
| 9 | （升格轮新增）前身 V3 文本「sound.cpp/`video.cpp` 有注释写明有意省略」 | **失真**：`video.cpp:253-254` 实测无任何护栏/省略注释（漏配而非有意省略）；`sound.cpp` 全文零探点。缺口 = V3 要补的登记 |

---

### V5 · PGO：把「从未评估」变成读数

**目标**：产出一组可复现的 PGO 性能读数 + 「采用/不采用/条件采用」结论，永久消掉「从未评估」。

**现状（本轮已核实）**：`CMakeLists.txt:95-120` 整块逐字未动——`FCEUX11_PGO` / `FCEUX11_PGO_USE`
两个 option（默认 OFF）、互斥 `FATAL_ERROR`、MSVC `/GL + /LTCG + /GENPROFILE` 与 `/USEPROFILE`
两段。⚠️ 历史出处：v1.14（`779d12d`）把「建立 PGO 训练场景 + 可选构建配置」标为已完成 ⇒
准确表述是「**选项已建、训练场景清单已有（SMB3 60s / Akumajou 30s / Zelda 30s），但本仓从未
产出过任何 PGO 性能读数**」（清单本轮未在仓内找到落盘脚本，实施时固化成脚本或文档）。

**解决方案步骤**：
1. `cmake -S . -B build-pgo-gen -DFCEUX11_PGO=ON` → Release 构建（instrumented）。
2. 跑训练场景出 profile：SMB3 60s / Akumajou 30s / Zelda 30s（ROM 由业主提供，**不入库**）。
3. `cmake -S . -B build-pgo-use -DFCEUX11_PGO_USE=ON` → 重建（/USEPROFILE）。
4. CPU 基准对比：`fceux11_bench_x6502_exec` 等（`tests/CMakeLists.txt:587-591`）+ `full_frame`。
   ⚠️ 基准口径（AGENTS r55）：`bench_tolerance_test` 是 **advisory**，同构建热采都会漂
   （+23% 量级）⇒ **同机同场景多次热采取中位**，只看**大幅持续回退**（历史真回归量级 +100%），
   不拿百分比容差判红。
5. 产出读数记录（构建命令、场景、基准表、结论）进 CHANGELOG/本计划附注。

**验收**：一组可复现读数（训练场景 + 构建命令 + 基准表 + 结论：采用/不采用/条件采用）。
PGO 默认 OFF 不变；不改 `bench_tolerance_test` 判定。

**风险与非目标**：代价 = 两轮 Release 构建 + 基准机时；profile 与机器相关（结论注明机器）；
不承诺收益数值；跑基准时不与构建/测试并行（r54②）。

---

### V6 · D10/108：FC 麦克风协议

**目标**：把 1996 年粗实现换成有信源背书的硬件语义，配齐锁测试与桥注入面。

**现状**：`src/input.cpp:130-140` 逐字未改（末次提交 `88dd595` / 2026-07-10，且那次只是 cast 清理）：
```c
// Not verified against hardware.
if (replaceP2StartWithMicrophone) {
    if (joy[1]&8) {
        microphone = !microphone;
        if (microphone) { ret|=4; …
```
逻辑 = 按 `joy[1]&8` 翻一个布尔量再 `ret|=4`（逐读翻转的方波 hack）；全仓无麦克风锁测试。

**位级依据（信源降级说明）**：NESdev wiki 正文本轮被 Cloudflare 拦（本机代理 `127.0.0.1:7897`
也过不了 JS 挑战）、grep.app 被拦 ⇒ 按「能实测就别只读文本」纪律降级为**权威参照实现**：
**Mesen2**（`SourMesen/Mesen2`，`Core/NES/Input/NesController.h`，取回副本 `.zcode\mesen2_NesController.h`）：
- `:85` `_microphoneEnabled = port == 1 && type == ControllerType::FamicomControllerP2`
  ⇒ 麦克风只在 **P2 口 Famicom 手柄**上；
- `:102-121` `ReadRam`：标准移位数据走 D0（`_stateBuffer & 0x01`），8 位后真手柄读 D=1；
  **`addr == 0x4016` 且麦克风按下时 `output |= 0x04`** ⇒ **bit 2 = 麦克风电平直读
  （按下=1），不参与移位**。
- ⚠️ **未取证不写**：真硬件麦克风的模拟量/多 bit 采样细节无信源；review 报告的
  「位 2/3 协议」说法**未获证实**（Mesen2 只见 bit2）⇒ 实施时以 mict ROM 对照裁决，
  不凭记忆补位。

**解决方案步骤**（完整先例 = SNES 鼠标 `05bfc1f` 一次提交的量）：
1. 协议修复（`src/input.cpp`）：bit2 行为按参照语义改写——P2 Start 按住 = 麦克风开 =
   `$4016` 读 bit2=1（电平直读，不逐读翻转、不移位）；P2 Start → 麦克风的**键映射保留**
   （那是映射问题不是协议问题）。语义修复**不留兼容开关**（业主既定纪律）。
2. 桥注入面（仿 `05bfc1f`）：`src/f11qa_bridge.*` + manifest `sut_input` 增麦克风开/关注入。
3. 锁测试（仿 `tests/f11qa/core/input_snesmouse_test.cpp` 的 T1–T6）：
   新增 mic 测试文件，覆盖「按下置 bit2 / 松开清 / 不影响移位序列 / 只在 $4016 / 只在 P2 口」。
4. 行为对照验收：`kgmqa-108-mict-rainwarrior`（tests.json:3792-3831；rainwarrior mict，
   `spec_source: forums.nesdev.org/viewtopic.php?p=231608`，`mirror_path: rainwarrior/mict/mict.nes`，
   现 `failure_means: advisory` + `known_limit: "Famicom microphone input scan protocol"`）实跑对照。

**验收**：协议按参照语义实现 + 锁测试全绿 + 桥注入面接进 manifest + mict ROM 行为读数记录。
⚠️ **基线翻转不顺手做**：108 从 advisory 转判红 / 删 `known_limit` 属期望值变更，需同级评审
（AGENTS.md r15；049/050/051 同款纪律）。

**风险与非目标**：mict ROM 的判定逻辑未逐字取证（实施第一步先实跑看读数再对症）；
不写未取证的模拟量细节；游戏兼容面（如塞尔达初代 Pols Voice）人工验证属实测长尾。

---

### V7 · B2(d)：profiler 的 Qt 菜单项

**目标**：开发期按需产出 profiler 报告（现在只能退出时看 stdout）。

**现状（本轮已核实）**：`FCEU_profiler_report(FILE*)`（`src/profiler.h:161`）全仓只有
`src/fceu.cpp:707` 一处调用（`fceu11::Kill()` 末尾打 stdout，`:704-708` 注释明言
「模拟线程已停、跨线程读探点记录安全」）；Qt 侧只有 include 与
`FCEU_profiler_log_thread_activity()`（`ConsoleWindow.cpp:742-743`、
`fceuWrapper.cpp:1535-1536/1551-1552`）⇒ **无 QAction / 无菜单项**。
探点现状 **6 个**：`fceu.cpp:804` / `ppu_rendering.cpp:301,686` / `sdl-sound.cpp:376` /
`ConsoleWindow.cpp:629,666`（旧「4/5」口径作废）。

**解决方案步骤**：
1. Qt 菜单动作 + 信号槽（挂 `ConsoleWindow` 菜单区）：**仅 `__FCEU_PROFILER_ENABLE__` 构建注册**
   QAction（非 profiler 构建不出现菜单项）。
2. 触发时产出报告：调 `FCEU_profiler_report(FILE*)` 写临时文件或管道 → 只读对话框展示
   （表列：func / file:line / numCalls / sum / avg / min / max / %total）。
3. 线程语义按 `profiler.h` 注释办：报告读取须在**模拟线程停止后**（或先暂停再读），
   **不得在模拟线程运行中裸读记录**。

**验收**：profiler 构建下菜单可见、点击产出报告表；非 profiler 构建菜单不出现；
`ctest -LE perf` 全绿。

**风险与非目标**：不改探点布局与记录结构；`FCEU_SoundCPUHook` 探点维持有意省略（V3 登记）；
不追求常驻 UI 面板（那是另一形态，不在本项）。

---

### V8 · A7 的「收益上限实验」（只做实验，不做本体）

**目标**：把「A7 值不值得」从猜测变成读数（值得/不值得/只做哪子集）。

**现状（本轮已核实）**：`src/ops_table.inc` 是**生成文件**（`scripts/generate_x6502_dispatch.py`
从 `src/ops.inc` 生成，文件头明言 "Do not edit manually"）；`(void)xp;` 精确计数 **256**
（全部 handler，签名 `static void x6502_op_XX(X6502* xp)`）；`src/x6502abbrev.h:9-20` 的
`_PC/_A/…` 宏路由 `g_cpu.native_layout()`；`src/cpu.cpp:14-15`
`static_assert(offsetof(Cpu, layout_) == 0 …)` ⇒ **savestate 布局不得动**（硬约束不变）。

**实验设计**：
1. **隔离分支/隔离构建**上给生成器加开关：生成「handler 内宏经 `xp` 参数」的版本
   （`_A` → `xp->native_layout().A`）——**256 个 handler 不手改**，生成器一次改完。
2. A/B 对比 CPU 基准（`fceux11_bench_x6502_exec`，`tests/CMakeLists.txt:588`；
   口径同 V5 第 4 步：多次热采取中位，advisory 读数）。
3. 产出读数结论：别名化的收益上限 + 值得/不值得/只做哪子集。

**验收**：读数结论落 CHANGELOG/本计划附注；实验不动主线。

**风险与非目标**：宏路由改动波及所有含 handler 的编译单元（只在隔离构建里做）；
`x6502.h` 的全局 `X` inline 别名保留（兼容未迁移文件）；**savestate 布局不得动**；
⚠️ **A7 本体（256 handler 全面别名化）不在 v2.0.4 范围**（2026-11-12 业主指令已从本计划删除）。

---

## 三、交付批次建议

| 批 | 内容 | 理由 |
|---|---|---|
| 批 1 | **V1 → V6** | 用户可感/解锁：V1 消 abort + 解锁 V2 走查；V6 有 `05bfc1f` 整套先例、一次提交量 |
| 批 2 | **V3 → V4** | 防呆，分钟级纯文本，成本近零 |
| 批 3 | **V5、V7、V8** | 读数/实验类，互不依赖，可并行（文件级互不重叠） |
| 插入 | **V2**（业主 GUI 会话） | 人工，V1 落地后随时安排 |

每批独立可验收；**每批末跑全量门禁**（`ctest -LE perf` + 四条 Rust 锁测试）。

---

## 四、与 v2.0.3 线的衔接

- v2.0.3 线（GBA 卡带兼容性）**自身**仍有未完项，**不在本计划范围内**，此处只做指针：
  - BIOS SWI 声音家族 `0x1A`–`0x1E` / `0x20`–`0x24` / `0x28` / `0x29` / `0x2A` 维持兜底，
    `0x26` HardReset 结构障碍挂起；
  - S3-b / S3-c / S3-d / S3-e 已在 v2.0.3 收口（r16–r19），其中 **FireRed 的 init 竞态根因未解**
    （是**越过**不是修好，长尾保留）；
  - 4 张失败卡带里**仅 FireRed 经业主 2026-11-12 人工复核确认兼容**，其余 3 张
    （Mario Kart SC / Dragon Ball / Minish Cap）**机器判据全过、人工未确认**。
  - 详见 `docs/history/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md` §九 r16–r20。
- ⚠️ **两条线的红线不要混**：本计划全部是 **NES 侧**；v2.0.3 线的 GBA 侧工作**不阻塞**本计划，
  反之亦然。

---

## 五、v2.0.4 出口判据

- [x] **V1**（2026-11-13，r5）：`tests/lua_scripts/test_frameadvance.lua` 实跑 exit 0（不再 `0xC0000409`），
      **且该脚本进入门禁路径**（tests.json kgmqa-123、`failure_means: blocking`）；
- [x] **V2**（2026-11-13，r4）：A1 的 21 行勾选表**全部**有「绿/红 + 日期 + 测试人」；
      走查单 18 项逐项结论；F-1 阻断项按 V1 的结论标注（后因新缺陷改挂，见 r5⑤）；
- [ ] **V3**：`src/video.cpp` 的 A4-1 否决护栏注释在位；
      `FCEU_SoundCPUHook` 探点的**有意省略**已在计划/CHANGELOG 登记；
- [ ] **V4**：`tests/tests.json` 的 5 条 `known_limit` 改为与事实相符；
      `docs/tech/cpu.md` / `F11QA.md` / `f11qa-accuracy-backlog.md` 的过期措辞已改；
      `src/tests/CMakeLists.txt` 的死代码已处理；v1.8 计划活引用 3 处 + 生成器及产物已改；
      **049/050/051 的基线翻转未被顺手做掉**（需同级评审）；
- [ ] **V5**：PGO 产出**一组可复现读数**（训练场景 + 构建命令 + 基准表 + 结论）；
- [ ] **V6**：FC 麦克风协议按参照语义实现 + 锁测试 + 桥注入面接进 manifest；
      `kgmqa-108` 行为读数记录在案（基线翻转留待同级评审）；
- [ ] **V7**：Qt 侧 profiler 菜单动作在位，且能产出报告；
- [ ] **V8**：A7 收益上限实验给出**读数结论**（值得/不值得/只做哪子集）；
- [ ] **NES 零回归**：`ctest -LE perf` **全绿**（以 `ctest -N -LE perf` 的 `Total Tests` 计数为准）
      + F11QA 矩阵 `pass_to_fail = 0` + grade ≥ B（矩阵**本地无入口** ⇒ 以 CI 复核为准）；
- [ ] 四条 Rust 锁测试门禁全绿（root / vendor / debug / media）。

---

## 六、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（不变式 1）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# V1 的行为级复现（修复前必红：0xC0000409；修复后必绿）
& build\tests\f11qa_lua_runner.exe tests\lua_scripts\test_frameadvance.lua --frames 60
# 阴性对照（必绿）
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
#   （用训练场景跑出 profile：SMB3 60s / Akumajou 30s / Zelda 30s）
cmake -S . -B build-pgo-use -DCMAKE_BUILD_TYPE=Release -DFCEUX11_PGO_USE=ON
cmake --build build-pgo-use --config Release
```

> ⚠️ `f11qa_lua_runner.exe` 在 `build\tests\`（CMake 目标），与 `f11qa-rom-runner`（Cargo bin，
> 在 `src\rust\target\…\`）**不是同一个东西**。
> ⚠️ 跑 `ctest` 时不要与构建/其它测试并行（会污染测量，AGENTS.md r54②）。

---

## 七、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-12 | **立项（v2.0.3 收口后的残留项复核 + 性价比裁决）—— 本轮只写文档，未动任何生产代码。** ① **复核对象**：`docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（2026-11-08）的 17 个待核点 + 1 条报告未列但实测仍在的项（F-1）。② **复核结论**：**13 项仍存在**（A1 / A4-1 / A7 / B2(b) / B2(d) / D1 / D3 / D7 / D9 / D10 / D8 / PGO / 038 / 077）、**1 项部分存在**（R5 清单与文档过期描述）、**2 项已消失**（v2.0.2 计划的 4 处过期行已随归档重组修掉；`long-term-evolution/` 目录整体不存在）；**外加 F-1 仍复现**（本轮实跑 `0xC0000409`，且已接线的两个 Lua 用例 exit 0 ⇒ 门禁对它全盲）。③ **性价比裁决**：**8 项进 v2.0.4**（V1 F-1 → V2 A1 → V3 护栏 → V4 文档批次 → V5 PGO → V6 麦克风 → V7 profiler 菜单 → V8 A7 收益上限实验），**7 项不进**（L1 A7 本体 / L2 D1 / L3 D3 / L4 D7 / L5 D9 / L6 038 / L7 D8），每条附不进理由与重新立项条件。④ **裁决的关键前提**：本计划几乎所有精度项的**判红体系收益 = 0**（advisory + 冻结基线 False）⇒ 决策依据只能是「用户可感 / 解锁 / 防呆 / 成本」。⑤ **报告自身的 8 处事实偏差**已登记（§五），其中 D7 / D8 / 077 三处属「描述失真但缺口仍在」⇒ 失真与缺口**分开记账**。⑥ **一处时效性提醒**：现存矩阵产物 `build/f11qa_migration_matrix.json` 的 `git_rev = 502150e`（v2.0.3 r5），**早于发布提交 `45fec33`** ⇒ 它不是 HEAD 的读数，引用时须注明。⑦ **未做**：F11QA 矩阵全量实跑（本地无入口）；F-1 的**修复方案设计**（本计划只立项，不写攻关步骤）。 |
| r2 | 2026-11-12 | **归档整理：除本文件外，`docs/plans/` 下的其余 4 份计划全部移入 `docs/history/plans/`（业主指令）。** ① **移动**：`FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`、`FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md`、`FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md`、`FCEUX11-v2.0.3_S3a前置RE-GBATEK与SWI证据表.md` —— **git 记为 rename（历史可追）**；`docs/plans/` 从此**只留在途计划**。② **引用改写**：全仓 **13 个文件 / 38 处**指向这 4 份文件的路径全部由 `docs/plans/…` 改为 `docs/history/plans/…`（含 `AGENTS.md`、`.github/workflows/f11qa.yml`、`src/rust/src/gba/{mod.rs,fixtures.rs}` 的 doc 注释、`docs/tech/*`、`docs/history/**`）⇒ **改写后零残留**（脚本 `.zcode/rewrite_plan_paths.ps1`，不入库）。③ **语义更新**：AGENTS.md 治理文档表把 v2.0.3 计划改标「**已收口归档**」+ 长尾指向；`docs/history/README.md` 的 `plans/` 清单**新增 4 行**；v2.0.3 计划顶部加**归档横幅**（随 v2.0.3 发布收口、内容冻结、§一 仍是 GBA 线的事实底座）；review 报告加**归档位置**与移交指针（它已是 2026-11-08 快照）。④ **本轮顺带查出并登记两处活体失效引用**（v1.8 计划 10 处引用已失效 ⇒ 活的 3 处 + manifest 生成器 + 其产物）⇒ 已并入 §三 V4；另 6 处 v2.0.2/long-term-evolution 引用是**刻意标注「已删除」的历史沿革**，不动。⑤ **未动**：`tests/tests.json` 的 `spec_source` 与生成脚本**本轮不改**（属 V4 批次，且 manifest 期望值变更需同级评审）。⑥ **未动任何生产代码**。 |
| r3 | 2026-11-12 | **升格：本文件由「遗留项计划」升格为 v2.0.4 构建计划（业主指令），工作分支 `wip204`。** ① **删除不纳入 v2.0.4 的项目**：原 §四 L1–L7（A7 本体 / D1 / D3 / D7 / D9 / 038 / D8）及总表中不进的行整段删除（理由与重新立项条件见 git 历史与 review 报告）；总表改为 V1–V8 任务表。② **V1–V8 逐项解决方案落定**（目标/现状坐标/步骤/验收/风险与非目标）；其中 V1、V5、V6、V7、V8 的代码现状本轮逐条实测核实（file:line 见各项）。③ **V1 根因实测更正**：前身文档「`mlua::Thread::yield` + unwind 穿不过 `extern "C"` 帧」为未证实假设；实测根因 = Rust 回调内经 mlua `Function::call` 调 `coroutine.yield` ⇒ Lua 5.1 `lua_yield` 的 nCcalls 检查必失败（`RuntimeError: attempt to yield across metamethod/C-call boundary`）⇒ 测试 profile 优雅返回 `Err`、真实 runner 在 `panic="abort"` 下静默 abort `0xC0000409`；修复方案（纯 Lua 包装 `emu.frameadvance = function() return coroutine.yield() end`）经探针实证 3 次挂起/续跑正常。④ **V6 位级信源**：NESdev wiki 被 Cloudflare 拦（代理亦不可达）、grep.app 被拦 ⇒ 降级为权威参照实现 **Mesen2 `NesController.h`**（`$4016` 读 bit2 = 麦克风电平直读，仅 P2 口 Famicom 手柄）；模拟量细节未取证不写；review 报告「位 2/3」说法未获证实。⑤ **V4 坐标实测落准**：5 条 `known_limit` 的行号与 kgmqa id 映射（1069/1109/1820/2665/3791）；`src/tests/CMakeLists.txt` **整份是死文件**（68 行，`src/CMakeLists.txt` 从不 include，活的在 `tests/CMakeLists.txt` 822 行）；tests.json 现存 `known_limit` 实测 **15 处**（含新增 `kgmqa-108` 麦克风与 `kgmqa-107` 鼠标两条）。⑥ **V7 口径修正**：探点现状 **6 个**（旧「4/5」作废）；报告出口 `FCEU_profiler_report` 已存在（`profiler.h:161`），缺的只是 Qt 菜单触发点。⑦ **V8 事实**：`ops_table.inc` 是生成文件（`generate_x6502_dispatch.py`），实验走生成器开关而非手改 256 handler。⑧ **交付批次建议**（§三）与出口判据（§五）按构建计划口径重写；复验命令保留并标注修复前后读数。⑨ **本轮未动任何生产代码**；唯一代码改动是移除取证用的临时探针测试（不入库）。 |
| r4 | 2026-11-13 | **V2 执行完毕：A1 手工 GUI 验收 21 行全部走查（ZCode 代业主走查，业主事前拍板「照走查单跑并记录 F-1 取证」+「现在开始」；桌面全程由 ZCode 操控）。结果：16 绿 / 1 红（S-2，真回归候选）/ 2 红=F-1 预期复现（LUA-1/2）/ 2 未验证（B-4、NSF-1 声音侧）。** ① **环境**：`build/src/fceux11.exe`（Release + `FCEU_DEF_DEBUGGER` + Rust Lua，Qt 6.8.0，zh-CN locale），HEAD `bc095a7`（wip204），ROM = `tests/fixtures/nestest.nes` + `test_nsf.nsf`；PPU 保持默认 `SDL.NewPPU=0`（fceux.cfg 实测）。零断点前置确认全过（Break On 五项 Off、断点表空、F9 灰）。② **走查单 §0.1 的 exe 路径已过期**：实际在 `build/src/` 非 `build/Release/`（本行走查已按实际路径执行；文档修正归 V4 批次）。③ **🔴 S-2 红（Step Over 不命中返回地址）**：JSR `$C2A7`（nestest init，经硬重置 + C07E Execute 断点到达）上 F10 三次独立激活均不停在返回地址 `$C081`——第 1/2 次跑了 1.3 亿/2.1 亿指令直达主循环、第 3 次（隔离态：命中后删除断点、零断点）15 秒轮询无停机；子程序 `$C2A7` 与嵌套 `$C2ED` 均有 RTS（ROM 反汇编实证）⇒ 返回必然发生。对照：B-3b 的 Run To Cursor（同为 watchpoint[64] 保留槽、短程）精确命中 ⇒ 缺陷特征为「watchpoint[64] 长程不消费」，隔离域 = 单步跳过路径。**按清单 §6.3：红项不静默关闭——下一步 = 按不变式 1 先回退 `src/debug.cpp:918` 门做变异验证再决策（业主拍板）；S-2 五字段失败记录见走查结果附录 + 证据目录 `results.md`。** ④ **LUA-1/LUA-2：F-1 在 GUI 完整复现**（走查单 §5 预期路径）：两次 Start 均进程 abort，Windows 事件日志 id=1000 `fceux11.exe` 异常 `0xC0000409`（00:45:43 PID 0x2CF8 = bp_test.lua；00:47:58 PID 0x1FC0 = instr_test.lua）——与无头 `f11qa_lua_runner` 同码；V1 落地后复走这两项转绿。⑤ **B-4 未验证**：nestest（干净 ROM）勾选非法操作码跑 5 秒不停（无误触发=正确，负半验证绿）；正向触发缺非法 opcode ROM（桌面 `roms/` 全是商业卡带，无法确认哪张执行非法 opcode）。⑥ **NSF-1 部分验证**：可视化绿（Test/Song 渲染、模拟运行中）；**声音侧未验证 = 夹具构造使然**——`test_nsf.nsf` 的 PLAY($8000) = BRK×3 + 纯 NOP sled（不写 APU），且首帧 PLAY 后 NSF CPU 坠入 `$0000` BRK 死循环（暂停读数 PC=$0000、栈满 BRK 压栈模式）⇒ 音量合成器三连拍无电平条属预期；完整闭合需可发声 .nsf + 人耳。⑦ **新发现（清单「入口与文档不符」家族）**：**断点对话框拒绝 `PLAY` 关键字**——红字「起始地址无效」+ 确定灰化，与清单 §4.5 NSF-1 替代验证声称的「debug.cpp:95 认 PLAY」不符；字面十六进制 `$8000` 可正常建断点（确定可用）。该关键字解析是否本就不在 Qt 校验路径、或 NSF 上下文缺失，归 V4 批次核实。⑧ **V-3 判定备注**：Sprite Viewer 的 OAM 区为空是 nestest 不写 OAM 的真实状态，查看器在渲染（槽位 0 方块/信息面板/调色板均来自真实 OAM 读取）——按「静止画面也合法」口径记绿。⑨ **回写位置说明**：清单 §6.3 指定的回写目标（v2.0.2 review 报告）已是冻结快照 ⇒ 本轮回写改记于本计划（本条）+ 清单 §6.1 勾选表 + 走查单附录结果节；冻结文件未动。⑩ **证据**：`C:\Users\ikrx2\Desktop\v204_a1_evidence\`（results.md 全程日志 + shots/ 30+ 张截图 + scripts/ UIA 自动化脚本库）。测试人 = ZCode（AI 代走查）；业主可抽查截图后补 personally 落章。⑪ **V2 出口判据达成度**：21 行全部有「结果+日期+测试人」✅；走查单逐项结论 ✅（附录）；F-1 阻断项按 V1 结论标注 ✅（两腿=GUI 复现取证，待 V1 后复走）。 |
| r5 | 2026-11-13 | **V1 执行完毕（业主指令「三件事按最优解处理」授权）：F-1 的 Lua 层根因修复 + 走查带出的另一回归（断点关键字死代码）一并修复；发现并立项一个新 GUI 缺陷。** ① **V1 修复落地**：`fceux11-lua/src/bindings/emu.rs` 的 `emu.frameadvance` 原生绑定（Rust 侧 `Function::call(coroutine.yield)`，F-1 根因）整体替换为**纯 Lua 包装**（`lua.load("return function() return coroutine.yield() end").eval()` 后 `emu.set`）；`frame_advance_waiting` 由 `frame_boundary()` 依线程状态自维护（lib.rs:375/390 实读确认），原生置位冗余移除；C ABI 导出面零变化（该绑定本就不经 C ABI）。② **行为级验证**：`f11qa_lua_runner tests/lua_scripts/test_frameadvance.lua --frames 60` **exit 0**（修复前 `0xC0000409`），阴性对照 test_emu exit 0；四条 Rust 锁测试门禁全绿（299/249/93/52，与基线一致）。③ **tests.json 新用例**：`kgmqa-123-lua-frameadvance`（形状照 kgmqa-045，`failure_means: blocking`）⇒ 用例总数 122→123，ctest 计数随增。④ **附带修复：断点关键字死代码回归（走查 r4⑦ 的定性结果）**——`offsetStringToInt`（debug.cpp）的 v0.3.5 缓冲区安全重构（`704078f`）用 `from_chars` 早退替换 `sscanf`（原对非数字串返回 0 ≠ EOF，会继续走到关键字分支），使 **NSF LOAD/INIT/PLAY、FDS NMI1-3、NES NMI/VBL/RST/IRQ/BRK 全部关键字分支变成死代码**；修复 = 十六进制解析失败时落穿（`offset = -1`，BT_P/S/R 分支照旧报 conversionOk=false，与 sscanf 版语义逐点一致）。**锁测试**：`tests/f11qa/core_state_test.cpp` 新增 5 断言（PLAY/LOAD/INIT 经 NSFHeader 解析 + 十六进制 C123 + 垃圾 ZZZZ 拒绝）——变异可红（还原早退即 PLAY 断言失败）。⑤ **🔴 新缺陷立项（V1 修复所暴露，非本次引入）：GUI 下任何 yielding Lua 脚本运行中进程崩溃**。证据链：a) 无头 runner 60 帧 yield/resume 正常（修复正确性）；b) GUI 侧 Start 后崩溃，**时长随机**（2s～68s+，本次最长 4069 帧），两种签名：`0xC00000FD`（栈溢出，__chkstk@Qt6Gui+0x4754C7）与 `0xC000041D`（用户回调致命异常，同偏移）；c) **bisect 定界**：yield_only.lua（无 hitbreakpoint）同样崩 ⇒ 与 break_asap 无关，**任何 yielding 脚本即触发**；d) 探针（load_script/on_script_end/resume-Err/frame_boundary 四点）实测：**load_script=1（无逐帧重载）、on_script_end=0、resume Err=0、frame_boundary 正常逐帧至死** ⇒ Lua 引擎状态机正常，崩溃在引擎之外；e) 堆以 ~4MB/s 增长（152→278MB/30s）；f) 轮询采样器（GetThreadContext，30 样本×27 线程）**无任何线程 RSP 渐进下行** ⇒ 非慢性栈泄漏，为突发深递归或其他机制；g) WER LocalDumps 对栈溢出签名未产出 dump。**嫌疑域**：GUI 帧路径与 mlua 0.10.5 yield/resume 的交互（修复前无任何 yielding 脚本能在 GUI 存活 >2s，此路径从未被真实执行过）；需 cdb/WinDbg 或 mlua 内部分析，立案为独立工作项。⑥ **LUA-1/LUA-2 处置更新**：F-1 阻断已解除（runner 级）；GUI 侧被上述新缺陷阻断 ⇒ 两腿维持「阻断」标注，阻断者从 F-1 改挂新缺陷（清单 §6.1 与走查单 §8 已同步注记）。⑦ **探针为临时代码已全部撤除**（lib.rs 四处；cbindgen 会把 extern 声明漏进生成头与 ucrt 冲突——探针改用纯 std 实现，撤除后生成头已恢复干净）。⑧ **B2（§6.3 门回退变异验证 S-2）本轮未执行**：GUI 崩溃缺陷使 S-2 的 GUI 复测环境不可靠（yield 无关，但同机实验受干扰），优先级让位给新缺陷；S-2 的静态调查结论（r4③ + 只读调查：三处武装代码中 StepOver/StepInto 无 wrapper 锁、RunToCursor 有锁；x86 TSO 下理论不应丢写）已记录，待新缺陷关闭后做门回退实验。⑨ **证据**：`v204_a1_evidence/`（results.md + stdout_probe*.txt 探针流 + stackpoll_out.txt 30×27 线程 RSP 矩阵 + symlookup/stackpoll/stackpeek 工具脚本）。 |
| r6 | 2026-11-13 | **完成度看板落档（§一 1.1，追加）+ §五 出口判据勾选更新（V1/V2 ✅）。** 依据：V1 判据两条均达成（runner exit 0 + kgmqa-123 blocking）；V2 判据三条均达成（21 行结论 + 走查单附录 + 阻断标注）；NES 零回归的 ctest 子项 36/36 ✅ 但 F11QA 矩阵子项以 CI 复核为准 ⇒ 该条保持未勾并待 CI；四条 Rust 门禁 ✅。计划外产物（关键字回归修复、GUI 崩溃新缺陷、S-2 挂起）以看板行显式登记，防下一任只读 §一 总表而漏看。未动任何生产代码（本条为纯文档追加）。 |
