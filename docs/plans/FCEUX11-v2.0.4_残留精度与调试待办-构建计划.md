# FCEUX11 v2.0.4 — 残留精度与调试待办 · 构建计划

> **STATUS: 在途**（2026-11-12 立项为「遗留项计划」，同日按业主指令**升格为 v2.0.4 构建计划**：
> 删除不纳入 v2.0.4 的项目，对确认进入的 V1–V8 逐项研究解决方案。工作分支 `wip204`。）
> **来源**：`docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（2026-11-08 产出，快照冻结）
> **复核基准**：`main` @ `101026b`（v2.0.3 发布提交 `45fec33` 之后）
> **本文件的性质**：v2.0.4 的**构建计划**——每项带「目标 / 现状坐标 / 解决方案步骤 / 验收 / 风险与非目标」。
> 前身「遗留项计划」的复核结论与归档整理记录保留在 §七 变更记录（r1/r2），**内容冻结不再续写**。
> **范围裁决（2026-11-12 业主指令）**：不纳入 v2.0.4 的项目（A7 本体 / D1/037 / D3/049 / D7/081 /
> D9/107 / 038 / D8/099，原 §四 L1–L7）**已从本计划删除**；其不进理由与重新立项条件见 git 历史
> （本文件 r2 及之前的版本）与 review 报告。
> **追加批（2026-11-13 追记，r7）**：本计划现含 **V1–V8（精度/调试/工具面）**
> + **V9–V25（内存安全/线程安全/mapper 正确性/输入正确性面）**。后者由
> `docs/plans/FCEUX11-v2.0.4_追加.md`（静态分析技术方向稿）逐条核验后并入，
> 该稿已归档至 `docs/history/plans/FCEUX11-v2.0.4_追加.md`（内容冻结）。

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

### 0.4 追加批（V9–V25）的来源与核验口径（2026-11-13 追记，r7）

> **来源**：`docs/plans/FCEUX11-v2.0.4_追加.md`（2026-11-13 产出的**静态分析技术方向稿**，
> 自上游 `fceux-fceux-2.7.0` + `fceux-master` 对标而来，原文自陈「**未实测**」）。
> 该稿自述的下一步是「回开发环境验证后按 §一体例追加进本计划」⇒ 本轮即执行该步：
> **逐条核实 → 研究方案 → 并入本计划**，并把原稿归档到 `docs/history/plans/`。

- **核验方式**：三路**只读**核验并行（按文件域切分，互不重叠），逐条回源码取 file:line 证据；
  上游对照走 **GitHub API + raw 快照**（仓库 `TASEmulators/fceux`，tag `fceux-2.7.0` 与 master），
  经本机代理 `127.0.0.1:7897` 取回；commit diff 与 issue 原文逐条核对，**不凭记忆写上游内容**。
- **核验基线**：`wip204` @ `c47047a`；**全程未改任何生产代码**。
- **本批的性质**：与 V1–V8 **零重叠**——V1–V8 是精度/调试/工具面，V9–V25 是
  **内存安全 / 线程安全 / mapper 正确性 / 输入正确性**面。⚠️ 因而**判红体系收益同样为 0**
  （本批不在 `tests/tests.json` 的判定面内）⇒ 决策依据仍是「用户可感 / 解锁 / 防呆 / 成本」，
  而 `fail_to_fail_max = 0` 仍然成立：**本批任何一项做坏都是红灯**。
- **证据口径**：本批全部为**静态分析（读源码 + 上游对照）**，**未实跑**；凡涉及运行期行为的
  结论（越界 / 撕裂 / panic）均为**源码路径推导**，逐条标注。⚠️ 落地前**必须先复现 + 建基线**。
- **核验结论总览**：原稿 12 项中 **7 项属实**（V9 / V10 / V11 / V13 / V14 / V15 / V19）、
  **5 项部分属实**（V12 / V16 / V20 / V22 / V23，其中 **V12 的做法不成立**、**V23 的定性被推翻**）；
  核验中**新发现 2 项**（V24 / V25）⇒ 本批共 **17 项**（V9–V25，其中 V17/V18/V21 为原稿留的空号）。
  ⇒ 原稿登记 **15 处文档偏差**（路径写错 / 行号漂移 / 坐标口径不匹配 / 事实性前提被推翻），
  逐条登记在 §二各节的「**文档偏差**」小节里。

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
| V3 护栏登记 | ✅ **完成**（r8） | A4-1 否决护栏进 `src/video.cpp`（64 KiB/帧拷贝处，写明「勿再做消费计数门控」+ chunk-8 任意帧异步消费 ⇒ 金标漂移/谓词恒真的否决理由）；`FCEU_SoundCPUHook` 的「有意省略探点」登记进 `src/sound.cpp`（并把探点口径修正为 **6 个**，实测坐标见 §二 V3）。**纯注释、零行为变更** |
| V4 文本修订批 | ✅ **完成**（r8） | 5 条 `known_limit` 改写（037/038/056/078/107，逐条回源码复核）；`docs/tech/{cpu,F11QA,f11qa-accuracy-backlog}.md` 过期措辞订正；死文件 `src/tests/CMakeLists.txt` **整份删除**（零构建引用 + 覆盖未丢失已核）；v1.8 计划的 3 处活引用改指 `docs/history/plans/`（**生成器与产物逐字一致已验**）。⚠️ `expected` / `failure_means` / `pass_to_fail` **零改动** |
| V5 PGO | ⬜ 未开始 | |
| V6 麦克风 | ⬜ 未开始 | |
| V7 profiler 菜单 | ⬜ 未开始 | |
| V8 A7 实验 | ⬜ 未开始 | |
| **计划外 A** | ✅ **完成** | 断点关键字死代码回归（v0.3.5 `704078f` 引入）修复 + 5 断言锁测试（r5④）——走查 NSF-1 带出 |
| **计划外 B** | 🔴 **新缺陷立项** | GUI 下任何 yielding Lua 脚本运行中崩溃（0xC00000FD/0xC000041D，时长随机 2s～68s+，堆 ~4MB/s 增长，突发深递归）；V1 修复所暴露，阻断 LUA-1/2 GUI 转绿（r5⑤）；需 cdb/WinDbg 或 mlua 内部分析 |
| **计划外 C** | 🔴 **回归候选挂起** | S-2：Step Over 的 watchpoint[64] 长程不消费（r4③）；静态调查榨干（r5⑧），§6.3 门回退变异测试排队（GUI 崩溃缺陷关闭后做） |
| **追加批 V9–V25** | 🔧 **9/14 完成 + V14 半**（r11；明细见 §1.2） | **V9 ✅**（变异腿实跑 + #851 真 ROM 验证，r10 双尾巴全清）、**V24 ✅**、**V11 ✅**（跨进程人工验证待做）、**V13 ✅**（压测待人工）、**V15 ✅**（压测待人工）、**V25 ✅**（r11：KEY_TO_NAME 改 SDL scancode 基准——96 键名错位修复 + 掩码改非零判定；真机 Lua 读值待做）、**V22 ✅**（r11：click 位测试 + 驱动中键 0x04 + 数值字段；真机待做）、**V19 ✅**（双腿变异实测）、**V23 ✅**（r11：CDL FDS-only 对齐上游，**.cdl 输出变化**；端到端上游对比登记不做）；**V14 🔧 半**（r11：TraceLogger 环形缓冲已 atomic 化 10 访问点，CDL 计数器半边待做）；其余 V10 / V12 ⬜ 未开始 |

**交付批次视角（r11 同步）**：批 1（V1→V6）= ✅✅（r11：V6 落地）；批 2（V3→V4）= ✅✅；批 3（V5/V7/V8）= ⬜⬜⬜；插入 V2 = ✅；
追加批 4 = **6/7**（余 V10）；批 5 = **3/5**（✅ V19/V22/V23，🔧 V14 半，⬜ V12）；批 6 = ⬜。

### 1.2 追加批任务总表（V9–V25，2026-11-13 核验后并入，r7）

> 判定列 = 本轮只读核验结论（**属实 / 部分属实 / 坐标有误·问题属实**），非原稿自述。
> 「原稿主张 → 实际情况」的偏差逐条登记在 §二各节末尾。

| # | 项 | 面 | 核验判定 | 一句话现状（实测坐标） | 危害 | 级 | 批 |
|---|---|---|---|---|---|---|---|
| **V9** | mapper 218 VPageR 越界 | Mapper·安全 | **属实** | `src/boards/datalatch.cpp:518` 与上游修复前**逐字相同**；`mapping[]` 取值域 {0,1}，PPU 用全 12 位地址索引 ⇒ 越 `NTARAM[0x800]` | 读越界 + **写越界**（`PPUCHRRAM=0xFF`）⇒ 踩坏 `vnapage_[]` 指针表 | **P0** | 4 |
| **V10** | CHR-RAM <8K 堆溢出 + 掩码钳制回归 | 安全·Mapper | **属实** | `src/ines_init.cpp:51` 仅 1024 垫层；`src/bus.cpp:578-584` 四个 `chr_mask*` 下溢成 `0xFFFFFFFF`（**上游 2.7.0/master 都有钳制，被 `ad2914d` 从本仓删掉**） | NES2.0 且 CHR-RAM <8K ⇒ `$0400-$1FFF` 写越堆最多 7 KB | **P0** | 4 |
| **V24** | GUID 解析的 UTF-8 字节边界 panic | 安全·Rust | **新发现** | `src/rust/crates/fceux11-utils/src/guid.rs:144-161` 用**字节**切片 `String`，且是 `extern "C"` | 畸形 `.fm2` 的 `guid` 字段 ⇒ panic ⇒ **进程 abort** | **P1** | 4 |
| **V11** | VRC7 读档 OPLL 悬空指针 | 安全·存档 | **属实** | `src/boards/vrc7.cpp:51` 以 `FCEUSTATE_INDIRECT` 落盘含 12 个裸指针的 `OPLL`；`:198-200` `StateRestore` 只调 `Sync()`，**零 `OPLL_forceRefresh`** | 跨进程读档 ⇒ 悬空指针解引用（`emu2413.c:778`） | **P1** | 4 |
| **V25** | Lua 键名表用 Windows VK 码索引 SDL scancode | 输入·Lua | **新发现** | `src/rust/crates/fceux11-lua/src/bindings/input.rs:9-12` 的键名表是 VK 码；`src/drivers/Qt/input.cpp:1366` 的 `g_keyState` 是 SDL scancode 索引 | 键名与按键**整体错位**（`capslock`→'Q'、`backspace`→'E'），且 `&0x80` 恒 0 ⇒ 绝大多数键恒 false | **P1** | 4 |
| **V15** | JSreturn 原子转移 | 多线程·输入 | **属实**（**漏合并上游现成修复**） | `src/drivers/Qt/input.cpp:151` 裸 `uint32 JSreturn`；**GUI 线程写**（`:1584`）／**模拟线程读**（`src/input.cpp:231-249` 解引用 `joyports[].ptr`） | 形式数据竞争（编译器可重排该 load）；上游 2.7.0/master 早已修 | **P1** | 4 |
| **V13** | AVI 环形缓冲四索引原子化 | 多线程 | **属实**（路径/行号有误；**漏合并上游现成修复**） | 真实路径 `src/drivers/Qt/AviRecord.cpp:63-68`（六个裸 `int`）/ `AviVideoCodec.h:49-54`；**AVI 功能真实在用** | emu 写／磁盘线程读无同步 ⇒ 消费端可能空转、尾帧丢失 | **P1** | 4 |
| **V12** | 3GO.nes 矫正条目（Mapper 0→252） | Mapper | **部分属实·做法不成立** | 机制对（活表 `ines_data.rs` `ROM_CORRECTIONS` **256 条**）；但原稿给的 `0xF3C85DC3` 是**整文件 CRC**，本仓查表键是 **padded PRG+CHR CRC**（不含头） | 按原稿加条目 ⇒ **静默查不中** | **P1** | 5 |
| **V22** | Lua 鼠标键位掩码判断 | Lua | **部分属实** | `input.rs:135-141` 等值判断（`==1`/`==2`）消费 `GetMouseData` 的位掩码；驱动侧无中键 `0x04`；且本仓 Lua 表**无数值 `click` 字段** | 双键同按（值 3）两个布尔都读不到；`middleclick` 恒 false | **P2** | 5 |
| **V14** | volatile 跨线程共享量原子化 | 多线程 | **属实** | TraceLogger 环形缓冲 `src/drivers/Qt/TraceLogger.cpp:114-118` 仍 `volatile`（**上游 2.7.0/master 早已改 atomic**）；CDL 六计数器 `src/debug.cpp:469` / `src/ppu.cpp:204` 等仍 `volatile` 且 `debug.cpp:481-513` 非原子 RMW | 形式数据竞争；上游有现成写法可抄（CDL 那半无上游背书） | **P2** | 5 |
| **V19** | GUID 失败置零 + 删死代码 | 安全 | **属实**（路径有误） | 真实路径 `src/utils/guid.cpp`（**无 `src/guid.cpp`**）；`guid.cpp:22-27` `fromString` 失败后 `data` 为栈垃圾；Rust `guid.rs:116/126` 两处早退**不置零** | 坏 GUID ⇒ 误报「savestate 属于另一个 movie」；重存时把栈垃圾写进 `.fm2` | **P2** | 5 |
| **V23** | CDL 行为差异文档化 | 调试器 | **部分属实·定性被推翻** | 差异事实成立（Rust `debug.rs:257-273` 不排除 4C/6C；`:274-286` **无条件清零**，上游只清 FDS）；但「F11 站在正确一侧」**不成立**——上游维护者明确拒绝 revert | F11 的无条件清零会**误清 ROM CDL**（mapper 写指令每次抹一处）⇒ 建议对齐上游 | **P2** | 5 |
| **V16** | StringToBytes hex 越读 | 安全 | **部分属实·当前不可达** | `src/utils/xstring.cpp:367` `str.size()/2` 把 `0X` 前缀算进字节数（应 `(str.size()-2)/2`）；但两个调用点（`drivers/common/config.cpp:306`、`movie_fm2.cpp` 的 `installValue`）**都不在编译/调用路径上** | 潜伏越读 1 字节；实际解码器是 Rust `decode_hex`（安全，且不认 `0X`） | **P3** | 6 |
| **V20** | movie length 纵深 | 安全 | **部分属实** | `src/rust/crates/fceux11-formats/src/movie.rs:315` 照收负值；`src/movie_fm2.cpp:243-246` `truncateAt` 无负值护栏（5 个调用点全传非负 ⇒ 潜伏） | 负 length 绕过 `.fm3` 只读判定；上游 #858 的 abort 路径**已被 Rust 重写消除** | **P3** | 6 |

> **V17 / V18 / V21**：原稿裁撤后留的空号（保持与原稿的编号连续），**不启用**。
> **批 4/5/6** 的批次定义见 §三；**出口判据**见 §五。

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

### 追加批 · 批 4：P0/P1（崩溃与内存安全）

> 以下 V9–V25 由 `docs/history/plans/FCEUX11-v2.0.4_追加.md` 逐条核验后并入（r7）。
> 每节的「现状」坐标均为**本轮只读核验实测**（`wip204` @ `c47047a`），
> 偏差登记在节末「**文档偏差**」。

#### V9 · mapper 218 的 VPageR 越界（P0）

**目标**：消除 mapper 218 的越界读写（上游 issue #851 同款段错误）。

**现状（本轮实测，坐标精确）**：
- `src/boards/datalatch.cpp:511-518`：`mapping[]` 取值域 **{0,1}**（1K **页号**语义），
  旧写法 `VPageR[i] = &NTARAM[mapping[info->mirrorAs2Bits*8+i]];` 把它当**字节偏移**用。
- `NTARAM` 实为 `uint8_t (&)[0x800]`（`src/ppu_class.cpp:194`），底层 `ntaram_[0x800]`
  （`src/ppu_class.h:285`），**紧随其后就是 `vnapage_[4]` 指针表**（`:286`）。
- PPU 取图样用**全 12 位地址**索引、**不掩码**：`src/ppu.cpp:133`
  `#define VRAMADR(V) &VPage[(V) >> 10][(V)]`；写路径 `ppu.cpp:182-183`、`:1457-1458`。
- 可达性：`src/ines_bmap.h:248` `{"", 218, Mapper218_Init}` ⇒ `src/ines_init.cpp:30,38-39,68`
  走 `bmap[]` 数字匹配 ⇒ `Mapper218_Init` **必被调用**。
- `Mapper218_Init` 末尾 `PPUCHRRAM = 0xFF`（`datalatch.cpp:520`）⇒ 8 个 CHR 页全被标为 RAM
  ⇒ `$2007` 写**越界写**，落点直接覆盖 `vnapage_[]` 指针表 ⇒ 后续命名表访问变野指针。

**根因**：页号当偏移 ⇒ `VPage[b][A] = NTARAM + (0|1) + A`，`A` 可达 `0x1FFF`
⇒ 越出 0x800 边界最多 `0x1801` 字节（上游 #851 栈帧 `GetCHRAddress (A=21845)` = `0x5555` 与此吻合）。

**解决方案步骤**：
1. **单行**替换为上游修复行（commit `6738471`，2026-10-18，1 file / +1 −1，逐字照抄）：
   `VPageR[i] = &NTARAM[mapping[info->mirrorAs2Bits*8+i] * 0x400] - i * 0x400;`
2. **无第二处需同步**（已核）：`mapping[]` 只在此处使用；修复后 `VPage[b][A]` 恒落在 `NTARAM[0..0x800)`。
3. ⚠️ F11 注意点：F11 的 `NTARAM` 是 `g_ppu.ntaram_`（对象**首成员**），
   `&NTARAM[0] - i*0x400` 会构造**对象首地址之前**的指针（C++ 严格属 UB；上游 `NTARAM` 是全局数组故实践无害）。
   **建议照抄上游**（改索引模型会偏离上游语义、放大回归面）。
   ⚠️ **r8 更正**：正因为指针本体带这个偏移，「`VPageR[i]` 落在 NTARAM 区间内」这种**指针区间断言不成立**
   （上游写法下 `i>0` 的指针就在 `NTARAM` 之前）⇒ 锁测试必须断言**有效地址**（第 4 条）。
4. 补常驻锁测试：**断言「有效地址落在 NTARAM 内」**——对每个 12 位 CHR 地址 `v ∈ [0, 0x2000)`，
   PPU 实际访问的字节地址 `VPageR[v >> 10] + v` 必须落在 `[&NTARAM[0], &NTARAM[0] + 0x800)`。
   ⚠️ 该断言**不复述指针公式** ⇒ 不会出现「测试与实现同错」的闭环；
   还原旧写法 ⇒ 夹具所在映射行（`mirrorAs2Bits=0` ⇒ `0,1,0,1,0,1,0,1`）**8192 个地址中 6145 个越界**
   （最深处越出 0x1800 字节）⇒ 必红。

**验收**：① 装载 mapper 218 夹具后锁绿
（**r8 已达成**：新建夹具 `tests/fixtures/mapper_mapper218.nes`——生成器 `INES_MAPPERS` 登记
`("mapper218", 218)` 并**只产出这一个文件**；header `4e 45 53 1a 01 00 a0 d0 …` ⇒ mapper `0xDA`=218，
16400 字节，PRG 16 KB（mapper 218 的 `setprg32` 在 `Bus::setprg32` 的掩码分支下 16 KB 镜像铺满 0x8000-0xFFFF，
不越界）；接进 `tests/f11qa/boards/mapper_load_test.cpp` 的 `tests[]` + 独立检查函数
`check_mapper218_chr_mapping()`）；
② 变异腿（还原 `:518` ⇒ 该锁必红）**未实跑**——本轮只做了数值推导（6145/8192 越界、最深 0x1800 字节），
⚠️ 实施者补做或以推导为证；③ `ctest -LE perf` 全绿；
④ 上游 #851 复现 ROM **仍需外部取**（本机无，**未验**）。

**风险与非目标**：风险极低（单行、只影响 218 初始化）；非目标：不补 mapper 218 的 `Power` 语义
（见 §二末「其他真实缺陷」②）、不做板级名匹配。

**文档偏差**：无（坐标与上游 commit 均核实为真）。

---

#### V10 · CHR-RAM <8K 堆溢出 + 掩码钳制回归（P0）

**目标**：消除 NES 2.0 且 CHR-RAM <8K 的卡带 `$2007` 写越堆；并补回上游的掩码钳制。

**现状（本轮实测，三处缺一不可）**：
- **分配点** `src/ines_init.cpp:49-59`：
  ```c
  int mCHRRAMSize = (CHRRAMSize < 1024) ? 1024 : CHRRAMSize;   // :51  仅 1K 垫层
  UNIFchrrama = VROM = (uint8*)FCEU_malloc(mCHRRAMSize);
  FCEU_MemoryRand(VROM, CHRRAMSize);                           // :53  只填真实尺寸
  ```
  ⚠️ 本仓 `FCEU_malloc` 自身 `memset(ret,0,size)`（`src/utils/memory.cpp:95-100`）⇒ 垫层实际已清零；
  零尺寸分支在 `:57-59`（`else { VROM = NULL; }`），**保留不动**。
- **钳制缺失** `src/bus.cpp:559-585`：`chr_mask8_[0] = (1024 >> 13) - 1` = `0xFFFFFFFF`（**下溢**），
  `chr_mask2_/chr_mask4_` 同；上游 2.7.0 与 master 都有四行
  `if (CHRmask* [chip] >= (unsigned int)(-1)) CHRmask* [chip] = 0;`（`cart.cpp:135-138`）。
  ⚠️ **回归有 git 证据**：本仓**曾经有**这四行，被 `ad2914d`（hotfix1 Phase 4，2026-07-14）删掉
  ⇒ 这是**本仓独有的回归**，不是「上游从来没有」。
- **写越界路径**：Rust `src/rust/crates/fceux11-formats/src/cart.rs:119-139`
  （`battery_vram_size + vram_size`，**无 8K 下限**，测试 `:459-466` 期望 12288）
  → `src/ines_init.cpp:42-44` Rust 桥 → `:51` 分配点 ⇒ **确认点 1 成立**。
  再经 `SetupCartCHRMapping` → `src/bus.cpp:578-584` → `src/cart.cpp:212-224` `setchr8r`
  （8 槽共指同一线性 8K 窗口）+ `src/bus.cpp:486-494` `Bus::setchr8`
  ⇒ `V=0` 时 8 槽全 = `VROM + 0` ⇒ PPU 写 `VPage[tmp>>10][tmp]`（`src/ppu.cpp:182-183`）、
  `tmp ∈ 0x400..0x1FFF` ⇒ **越出 1 KB 堆块最多 7168 字节**。
  钳制缺失还额外放大：任何 `setchrN(V)` 且 **V≠0** 的 mapper 会算出 `chr_ptr + V*8192` 的野指针
  （上游钳制会把 V 归零）。

**影响面（须如实登记）**：非 NES2.0 的默认分支给 8192（`cart.rs:134`）⇒ 零影响；
mapper 13/6/29/30/45/96/176 ⇒ 16K–128K ⇒ 零影响；FDS 固定 8192（`src/fds.cpp:293`）⇒ 零影响；
UNIF 路径 `src/unif_load.cpp:85-89` 虽有独立分配点，但 Rust `unif.rs:694-704 chrram_size_kb`
最小返回 8 KB ⇒ **当前不可触发** ⇒ 降为注记、不必改。

**解决方案步骤**：
1. `src/ines_init.cpp:51`：`(CHRRAMSize < 1024)` → `(CHRRAMSize < 8192)`；
   并在分配后补 `memset(VROM + CHRRAMSize, 0, mCHRRAMSize - CHRRAMSize);`（对齐上游语义；本仓 `FCEU_malloc` 已清零，属双保险）。
2. ⚠️ **保留 `FCEU_MemoryRand(VROM, CHRRAMSize)`（真实尺寸）不动** ——
   上游 PR #859 的 diff 是「**垫层** memset，**不是**整个缓冲区 memset」。
   若照字面把整块改成 memset，会丢掉 `RAMInitOption=0` 的 `00 00 00 00 FF FF FF FF` 图案 ⇒ **必然 movie 漂移**。
3. **保留** `SetupCartCHRMapping(0, VROM, CHRRAMSize, 1)` 与 `AddExState(VROM, CHRRAMSize, …)` 用真实尺寸
   ⇒ **存档格式不变**。
4. `src/bus.cpp:559-585`：在 `size == 0` 早退之后、计算四个 mask 之后补回上游四行钳制
   （顺序必须是：先算 mask，再钳制）。
5. 同步修正 `src/bus.cpp:560-566` 的注释（它对上游钳制语义的推理是**错的**，见 §二末「其他真实缺陷」①）。
6. `src/unif_load.cpp:85-89`：加一行注释说明「依赖 Rust 侧 ≥8K 保证」，本轮不改。

**验收**：
① ASan 下 `nes-test-roms other/oam3.nes`（mapper 7、1K CHR-RAM）3000 帧无越界
（⚠️ **该 ROM 不在本机**，`Research_only/` 已不存在 ⇒ 需外部取；
**ASan 开关本仓已有**：`CMakeLists.txt:202` `option(FCEUX11_ASAN …)`，默认 OFF ⇒
`-DFCEUX11_ASAN=ON -DCMAKE_BUILD_TYPE=RelWithDebInfo` 即可）；
② 新增单测：`Bus::setup_chr_mapping(0, buf, 1024, 1)` ⇒ `chr_mask8_ == 0`（钳制生效）；
③ **≥8K 卡既有 movie 零漂移**；
⚠️ **口径限定**：**<8K 卡画面会变**（此前越界读到相邻堆垃圾，修复后读到 0）⇒ 该类卡的既有 movie 会漂移。
**不能笼统承诺「所有既有 movie 零漂移」**，验收口径必须写明「≥8K 卡」；
④ `ctest -LE perf` 全绿。

**风险与非目标**：风险**中**（触及 RAM 初始化路径）；非目标：不实现真 1K 芯片的地址镜像（上游明确不做）、
不改 UNIF 路径、不动存档格式。

**文档偏差**：① 零尺寸分支行号 `59-62` → 实为 **`ines_init.cpp:57-59`**；
② 「垫层 `memset 0`，**不走 `FCEU_MemoryRand`**」的措辞**易被照字面执行而致 movie 漂移**
（上游实为「保留 MemoryRand（真实尺寸）+ 只对垫层 memset」）；
③ 「F11 独有回归」结论正确，但文档未给 git 证据（实为 `ad2914d` 删除）；
④ 「`unif_load.cpp:86` 同样无下限 ⇒ 同样垫」—— 该路径当前不可触发（Rust 侧 ≥8K），应降为注记。

---

#### V24 · GUID 解析的 UTF-8 字节边界 panic（P1，**本轮新发现**）

**目标**：消除「加载畸形 `.fm2` ⇒ 进程 abort」。

**现状（本轮实测）**：
- `src/rust/crates/fceux11-utils/src/guid.rs:108`
  `pub unsafe extern "C" fn fceux11_rust_guid_scan(guid: *mut FceuGuid, str: *const c_char)`。
- `:124-127`：`clean` = 去掉 `-` 后的串，`if clean.len() != 32 { return; }` —— `len()` 是**字节数**。
- `:129-141` 三个 `parse_hex_u32/u16/byte` 按**字节**切片 `&s[start..end]`；
  调用点 `:144-161` 的切点为 0 / 8 / 12 / 16 / 20…。

**根因**：`clean` 是 `String`（UTF-8），长度恰为 32 **字节**但含多字节字符时，
切点会落在字符中间 ⇒ `&clean[0..8]` panic（`byte index … is not a char boundary`）。
该函数是 **`extern "C"`** ⇒ Rust 在 C ABI 帧内不可 unwind ⇒ **直接 abort**（release 亦是）。

**可达性**：`src/movie_fm2.cpp:446` `movieData.guid = FCEU_Guid::fromString(guid);`
（→ `src/utils/guid.cpp:22-27` → `:42-45` `scan()` → Rust）；
`guid` 串来自 Rust 对 `.fm2` 的解析（`src/rust/crates/fceux11-formats/src/movie.rs:300`
`"guid" => md.guid = val.to_string()`，**原样保留文件里的任意字符串**）
⇒ **加载一个 `guid` 行含多字节 UTF-8、且去 `-` 后恰为 32 字节的 `.fm2` 即可中止进程**。

**解决方案步骤**：
1. `guid.rs`：把字节切片改成**按字节处理**（`clean.as_bytes()`）或先做纯 ASCII 校验：
   `if !clean.bytes().all(|b| b.is_ascii_hexdigit()) { 失败路径 }` —— 两种写法都**不做 `&str` 边界切片**。
   失败路径按 V19 一并**置零**（`guid.data = [0u8;16]`）。
2. 加 `#[cfg(test)]` 锁测试：输入 32 字节含多字节 UTF-8 ⇒ 必须**返回失败**而**不是 panic**
   （变异：还原成 `&clean[..]` ⇒ 转红）。

**r8 实施结果（已落地）**：
- 做法：`clean` 由 `String` 改为 `Vec<u8>`（`s.bytes().filter(|b| *b != b'-')`），
  新增 `hex_digit_value()` 与 `parse_hex_group(&[u8], start, len)`（**按字节**取组，越界返回 `None` 而非 panic），
  函数内**再无任何 `&str` 区间切片**；三条失败路径（非 UTF-8 / 长度≠32 / **含非 ASCII 字节**）统一置零。
- 语义保持：合法 36 位 GUID 的字节布局**逐位不变**（big-endian 解析 + little-endian 落盘）；
  ASCII 非法 hex 字符仍按旧语义「只把所在组置 0」（**未收紧**，属 V19 的范围）。
- ⚠️ **一处裁决（已定）**：含**非 ASCII** 字节的输入 ⇒ **整串置零**（而非旧实现的「部分解析」）。
  理由：非 ASCII 输入根本不是 GUID；旧行为是「恰好没 panic 时的偶然产物」，且与 V19 的「失败置零」目标一致。
  ⇒ **唯一可观察行为变化**：32 字节含多字节 UTF-8 的输入由「部分结果」变「全零」。
- 锁测试：**+5 条**（46 → 51 passed），含「多字节不 panic」「长度/非 UTF-8 置零」「非法 hex 保持旧语义」
  「合法 GUID 逐字节往返（期望值独立推导，不取自被测函数）」。
- 变异验证（**已实测**）：还原成 `&clean[..]` ⇒ 新用例报
  `end byte index 12 is not a char boundary` ⇒ `thread caused non-unwinding panic` ⇒ 进程 abort（`0xC0000409`）；
  还原后文件 SHA256 与变异前一致（逐字还原），复跑 51/0 全绿。

**验收**：`cargo test -p fceux11-utils --lib` 全绿（含新锁）；畸形 `.fm2` 加载不中止（只报错）；
`ctest -LE perf` 全绿。

**风险与非目标**：风险低（只在畸形输入上行为变化）；非目标：不收紧 GUID 格式兼容性
（不改成上游那种 36 位严格格式 —— 会拒绝本仓现有写法，另案）。

**文档偏差**：原稿未涵盖（**新增项**）。

**r8 附带登记**：`src/utils/guid.cpp:22-27` 的 `FCEU_Guid ret;` 是**默认初始化**
（`ValueArray<uint8,16>` 是含 `T data[N]` 的聚合体）⇒ 解析失败时 `ret.data` 是**未初始化栈内存**，
且会被 `src/movie_fm2.cpp:327` 的 `guid.toString()` **写进保存的 `.fm2`** ⇒ 信息外泄路径。
本项（Rust 侧置零）已消掉其中一半，**C++ 侧兜底（值初始化 `FCEU_Guid ret{};`）与 `hexToByte` 死代码删除仍属 V19**。

---

#### V11 · VRC7 读档 OPLL 悬空指针（P1）

**目标**：消除跨进程读档后 OPLL 内部裸指针悬空导致的崩溃/杂音（上游 issue #821）。

**现状（本轮实测）**：
- `src/boards/emu2413.h:49` `uint16 *sintbl;`（`OPLL_SLOT` 的裸指针成员），
  `:104` `OPLL_SLOT slot[6 * 2];` ⇒ 结构内共 **12 个**该指针；
  指针目标是模块内 static 表（`src/boards/emu2413.c:168-170` `halfsintable` / `fullsintable`）
  ⇒ 其地址随进程加载基址变化 ⇒ **跨进程恢复即悬空**。
- 落盘：`src/boards/vrc7.cpp:51`
  `{ (void**)VRC7Sound_saveptr, sizeof(*VRC7Sound) | FCEUSTATE_INDIRECT, "VRC7" }`
  ⇒ 整个 `OPLL` 按**原始字节**写出（`src/state.cpp:178-180`）⇒ 12 个指针值原样进文件；
  读回 `:237-239`（文件）/:280-281（缓冲）`memcpy` 原始字节 ⇒ 指针被逐字还原。
- `src/boards/vrc7.cpp:198-200` `StateRestore(int) { Sync(); }`；`Sync()`（`:102-117`）只做
  `setprg8/setchr1/setmirror`（纯 bank 映射，**不碰 OPLL**）；全仓 `OPLL_forceRefresh`
  **只有定义（`emu2413.c:613`）与声明（`emu2413.h:126`），零调用点**。
- 解引用点：`src/boards/emu2413.c:778`（另 `:794`/`:796`）。
- 可达性：mapper 85 已注册（`src/ines_bmap.h:115`、`src/boards/vrc7.cpp:231-234`）；
  存盘链 `src/state.cpp:372 addSformatChunk(0x10, SFMDATA)` ⇒ **每个 VRC7 存档都含 "VRC7" 块**。

**根因**：`FCEUSTATE_INDIRECT` 把含裸指针的结构原样落盘；读回后指针指向**别的进程**的地址空间。
同进程内存取时指针值恰好相同 ⇒ **不崩** ⇒ 该缺陷长期未暴露。

**解决方案步骤**：
1. 最小修复（不动物 ABI、不改存档格式）：在 `src/boards/vrc7.cpp:198-200` 的 `StateRestore` 内补一次刷新：
   ```cpp
   static void StateRestore(int version) {
       Sync();
       if (VRC7Sound) OPLL_forceRefresh(VRC7Sound);   // 重建 sintbl 等派生量
   }
   ```
   ⚠️ **放在 `StateRestore` 而非上游 issue #821 建议的 `Sync()`**：`Sync()` 每次 PRG/CHR 写都触发
   （`vrc7.cpp:132/136/137/138/140`）⇒ 徒增开销；`StateRestore` 是唯一会覆盖该结构体的时机，
   而 `forceRefresh` 只重算派生量（`dphase/tll/rks/sintbl/eg_dphase`），不触碰已恢复的 `phase/patch` ⇒ 音色连续。
2. `OPLL_forceRefresh` 代价：约 60 次查表、无分配、无 I/O，且对 NULL 早退（`emu2413.c:616-617`）⇒ NULL 安全。
3. 更彻底方案（**本仓已有骨架但从未接线**）：`src/state.h:55-60` 注释所述的「Gateway Phase 6」——
   用 `FCEU_SetStatePreSave`（`src/state.cpp:909-912`，**零调用方**）把 OPLL 拷进无指针快照缓冲，
   彻底移除 `FCEUSTATE_INDIRECT`。⚠️ 若走此路需一并接线 `src/cart_class.h:96-97` 的
   `on_save_pre`/`on_load_post`（`Vrc7Cart`（`src/boards/simple_carts.h:38`）当前未覆写）。
   **本轮只做第 1 步**；第 3 步留作长尾。

**验收**：① 跨进程判据：进程 A 保存 VRC7 存档 → 进程 B 加载 → 不崩（可临时加断言比对
`sintbl` 是否等于本进程 `waveform[]` 的地址）；② `ctest -LE perf` 全绿；
③ 静态审查：`grep -rn "FCEUSTATE_INDIRECT" src/boards/` 仅余 `state.cpp:131` 与本处；
④ 人工：Lagrange Point 等 VRC7 游戏读档后音色连续。

**风险与非目标**：风险低（单点重建调用）；非目标：不做「无指针快照」大改（那是 Gateway Phase 6 的完整工程）。

**文档偏差**：上游建议的落点是 `Sync()`（不是 `StateRestore`），原稿未指出这处差异。

---

#### V25 · Lua 键名表用 Windows VK 码索引 SDL scancode（P1，**本轮新发现**）

**目标**：让 Lua 的 `input.get().<键名>` 与实际按键**一一对应**（当前整体错位）。

**现状（本轮实测）**：
- `src/rust/crates/fceux11-lua/src/bindings/input.rs:9-12` 的 `KEY_TO_NAME` 是
  **Windows VK 码表**（含 `leftclick/rightclick/middleclick`）。
- `:119-128` 的键盘扫描路径用 `keys[]` **索引查表** + `& 0x80` 掩码规则（`:120-124`：
  仅 `0x14/0x90/0x91` 用 `0x01`，其余 `0x80`）。
- `src/drivers/Qt/input.cpp:171-178` `g_keyState[event.key.keysym.scancode] =
  (event.type == SDL_KEYDOWN) ? 1 : 0;` ⇒ **值只有 0/1**，索引基准是 **SDL scancode**。

**根因**：VK 码表被拿去索引 SDL scancode 数组 ⇒ 键名与按键**整体错位**
（`capslock` 落在 scancode 20 = 'Q'、`backspace` 落在 8 = 'E'）；
且 `& 0x80` 与 0/1 **永不相交** ⇒ 除三条例外项外，所有键恒为 false。

**解决方案步骤**：
1. 二选一（**建议 a**）：
   (a) 把 `KEY_TO_NAME` 改写为 **SDL scancode 基准**（与本仓 SDL 后端一致，删掉一张无用表），
       掩码判定从 `& 0x80` 改为「值 ≠ 0」；
   (b) 保留 VK 表，另建 SDL→VK 的翻译层（要维护第二张表，且 SDL 侧拿不到 VK 码）。
2. 与 V22 一并改（同一文件、同一绑定面）：鼠标位掩码 + 中键 `0x04`。
3. 键名集合会变化 ⇒ 提交说明必须附**改名对照表**。

**验收**：① `cargo test -p fceux11-lua --lib` 全绿 + 新增锁测试（scancode→键名映射逐项断言，
覆盖 caps lock / backspace / 方向键 / F1–F12）；② 真机 Lua 脚本读 `input.get().capslock` 等与实际按键一致；
③ `ctest -LE perf` 全绿。

**风险与非目标**：风险**中**（键名集合变化会影响既有 Lua 脚本）；非目标：不改 Lua API 的字段命名。

**文档偏差**：原稿未涵盖（**新增项**）。

---

#### V15 · JSreturn 的原子转移（P1）

**目标**：消除 GUI 写 / emu 读手柄状态的跨线程数据竞争（输入是 TAS 立身之本）。

**现状（本轮实测 · 线程归属已逐点落实）**：
- `src/drivers/Qt/input.cpp:151` `static uint32 JSreturn = 0;`（裸 `uint32`）。
- **写者 = Qt 主线程**：`input.cpp:1584` `JSreturn = JS;` 位于 `UpdateGamepad()`，
  调用链 `FCEUD_UpdateInput()`（`:1653`）→ `:1732 UpdateGamepad()`；
  `FCEUD_UpdateInput` 全仓只有两处调用，**两处都在 GUI 线程**：
  `src/drivers/Qt/ConsoleWindow.cpp:657`（`emuFrameFinish`，`:204` 以 `Qt::QueuedConnection` 从 emu 线程投递）
  与 `:680`（120 Hz QTimer）。⚠️ `FCEUD_UpdateInput`（`:1653-1747`）**不取 wrapper 锁**
  ⇒ emu 线程持锁跑核心时，GUI 仍可改写 `JSreturn`。
- **读者 A（GUI 线程，非竞态）**：`GetGamepadPressedImmediate()`（`:213-217`，读在 `:216`）——
  唯一调用者 `src/drivers/Qt/TasEditor/recorder.cpp:174`（TAS 编辑器 GUI），与写者同线程。
- **读者 B（模拟线程，真竞态）**：核心在 `$4016` 读时解引用指针 `src/input.cpp:231-249`
  `joy[0] = *static_cast<uint32*>(joyports[0].ptr);`；
  指针由 GUI 线程在 `input.cpp:1805` `InputDPtr = &JSreturn;` → `:1822 fceu11::SetInput(...)`
  → `src/input.cpp:604 joyports[port].ptr = ptr;` 登记；核心读取发生在模拟线程。

**根因**：裸 `uint32` 跨线程共享 ⇒ 形式 UB（x86 对齐 32 位读写实际不撕裂，但**编译器可重排/缓存该 load**）。

**解决方案步骤（照上游 1:1 移植；上游 2.7.0 与 master **都已修**）**：
1. `src/drivers/Qt/input.cpp:151` → `static std::atomic<uint32_t> JSreturnGui{0};` +
   `static uint32_t JSreturnEmu = 0;`（补 `#include <atomic>`）。
2. `:1584` → `JSreturnGui.store(JS, std::memory_order_relaxed);`
3. `:213-217` → `return JSreturnGui.load(std::memory_order_relaxed);`
4. `:1805` → `InputDPtr = &JSreturnEmu;`
5. 新增 `void emuThreadPollInputs()`（声明入 `src/drivers/Qt/input.h`），在
   `src/drivers/Qt/fceuWrapper.cpp:1398`（`if (GameInfo) {` 块首）调用一次
   ⇒ 等价上游 `fceuWrapper.cpp:1593-1595`。
6. 顺带核对 movie 回放路径：`input.cpp:1736` 的「Don't get input when a movie is playing back」
   只挡 `t & 2`（扩展设备）分支，**手柄分支 `t & 1` 仍会跑 `UpdateGamepad`** ⇒ 需确认回放时
   核心读到的 `joyports[].ptr` 应为 movie 数据（上游同构，移植时一并确认）。

**验收**：① 线程模型断言（探针/单测）：`UpdateGamepad` 与 `emuThreadPollInputs` 的
`std::this_thread::get_id()` 必不同；② 压测：狂按 A/B 录 movie → 回放逐帧输入序列一致；
TAS 编辑器琴卷指示灯与实际按键一致；③ `ctest -LE perf` 全绿；④ Rust 侧门禁不回归。

**风险与非目标**：风险低-中（输入延迟最多 1 帧：GUI 写 → 下一帧 emu 快照，上游已接受此代价）；
非目标：不新增线程、不改输入管线架构、不改 TAS 编辑器交互。

**文档偏差**：① 原稿提到的 `FCEU_UpdateJoypad` / `InputScan` **在本仓不存在**（属上游/X11 驱动时代遗留描述）；
② 原稿把 `input.cpp:1805` 列为「读点」——它是**指针登记**（GUI 线程），真正的读在核心侧
`src/input.cpp:231-249`（模拟线程）。

---

#### V13 · AVI 环形缓冲四索引原子化（P1）

**目标**：消除 emu 线程（生产）与 AVI 磁盘线程（消费）之间的索引数据竞争。

**现状（本轮实测 · 真实路径与行号）**：
- 真实路径在 `src/drivers/Qt/` 下：`AviRecord.cpp` / `AviVideoCodec.h` / `AviRecordDiskThread.cpp`。
- 六个裸 `int` 在 `src/drivers/Qt/AviRecord.cpp:63-68`：`vbufHead` 63、`vbufTail` 64、
  `vbufSize` 65、`abufHead` 66、`abufTail` 67、`abufSize` 68；
  extern 声明在 `src/drivers/Qt/AviVideoCodec.h:49-54`（六个）。
  **需原子化的是 4 个索引**（`vbufHead/vbufTail/abufHead/abufTail`）；
  `vbufSize/abufSize` 初始化后只读（`:502/505`），保持普通 `int` 即可。
- **写者（模拟线程）**：`AviRecord.cpp:558` `vbufHead = head;`、`:597` `abufHead = next;`；
  忙等读对端 `:533`/`:543` `availSize = (vbufTail - vbufHead);`。
- **读者（Qt 磁盘线程）**：`src/drivers/Qt/AviRecordDiskThread.cpp:113`
  `while ((numPixelsReady < numPixels) && (vbufTail != vbufHead))`、`:117` `vbufTail = …`；
  音频对 `:167-183`/`:214-230`。
- 线程创建：`src/drivers/Qt/ConsoleWindow.cpp:291`；启动 `src/drivers/Qt/ConsoleRecording.cpp:122`/`:229`。
- ⚠️ **「使用度确认」（原稿的前置条件）结论 = 真实在用**：
  菜单 `src/drivers/Qt/ConsoleMenu.cpp:1176-1185` → 槽 `ConsoleRecording.cpp:115`；
  帧生产链 核心 `src/video.cpp:271/390/395` → `snapAVI()`（`:429-434`）→
  `src/drivers/Qt/sdl-video.cpp:547-553` → `aviRecordAddFrame()`（`AviRecord.cpp:517`）；
  音频 `src/drivers/Qt/fceuWrapper.cpp:1152`；
  编译进构建 `src/CMakeLists.txt:530-532`（无条件列入 Qt target）。
  ⇒ **不满足「无人用 ⇒ 降级为潜伏项」的前提 ⇒ 本项要做。**

**根因**：**这是本仓丢失上游修复的回归**，不是待新造写法 ——
上游 2.7.0 的 `AviRecord.cpp:128-132` 已是 `static std::atomic<int> vbufHead(0); …`；
上游 master 进一步用 acquire/release。
上游把磁盘线程与生产端放在**同一 TU**，索引能保持 `static`；本仓拆成独立 TU
⇒ 需 extern 声明 ⇒ 移植时把 `std::atomic<int>` **降成了裸 `int`**。

**解决方案步骤（照抄上游 master，不自创写法）**：
1. `AviRecord.cpp:63-68`：四个索引改 `std::atomic<int>`；`AviVideoCodec.h:49-54` 同步改类型。
2. 写端 `vbufHead.store(head, std::memory_order_release);`；
   读端 `int head = vbufHead.load(std::memory_order_acquire);`
   （上游 master `AviRecord.cpp:2758-2767` / `:2434/2459` 的写法）。
   本仓同族先例：`src/drivers/common/nes_shm.h:169-188` 的 `sndBuf_t` + `push_sound_sample` 注释
   （明确写着「避免 plain `head` 读写的 RMW 数据竞争」）。
3. ⚠️ `std::atomic<int>` **不可拷贝/赋值** ⇒ 重置处（`AviRecord.cpp:508-511`）改
   `.store(0, std::memory_order_relaxed)`（上游 2.7.0 `:2400-2403` 即如此）。

**验收**：① 录 AVI ≥10 分钟：文件大小/帧数正常、尾部不丢帧、能正常关闭；
② `grep -rn "vbufHead\|vbufTail\|abufHead\|abufTail" src/drivers/Qt/` 全部访问点均为 `.load()/.store()`；
③ ASan/TSan 下录制 60 秒无 data-race 报告（⚠️ 本仓 TSAN 未落地 ⇒ 用 ASan + 静态审查替代）；
④ `ctest -LE perf` 全绿。

**风险与非目标**：风险低（类型替换，ABI 内部）；非目标：不改录制格式、不改编码器、
不引入 libav 依赖、不处理 `AviRecord.cpp:577-595` 已有的音频环 overrun 护栏（另一件事）。

**文档偏差**：① 路径 `src/AviRecord.cpp` / `src/AviVideoCodec.h` / `src/AviRecordDiskThread.cpp`
→ 实为 `src/drivers/Qt/` 下同名文件；② 行号 `:63-67`（四个）→ 实为 `:63-68`（六个，其中四个是索引）；
extern `:49-52` → 实为 `:49-54`；③ 「照抄 `nes_shm.h` 注释策略、**不新造写法**」——
真正的现成写法在**上游自己**（2.7.0 `:128-132` / master `:2758-2767`）；
④ 原稿的「若确认无人用则降为注记」前提**不成立**。

---

### 追加批 · 批 5：正确性与防御性修正

#### V12 · 3GO.nes 矫正条目（Mapper 0→252）（P1 · **做法须重取证**）

**目标**：让 iNES 头误标 Mapper 0 的《三国志中原之霸者》按 Mapper 252 装载（上游 issue #844/#845）。

**现状（本轮实测）**：
- 活表结构 `src/rust/crates/fceux11-formats/src/ines/ines_data.rs:1050-1055`
  `pub struct RomCorrection { pub crc32: u32, pub mapper: i32, pub mirror: i32 }`；
  表体 `:1055-2336`，实测 **256 条**（唯一 CRC 252 个）。
- 查询链 `src/rust/crates/fceux11-formats/src/ines.rs:308-327`：**纯按 CRC 匹配**，
  `e.mapper >= 0` 才改 mapper、`e.mirror >= 0` 才改镜像；无 mapper/头校验、无「仅 mapper 0 生效」限制。
- 应用链 `src/ines_load.cpp:113` `fceux11_rust_ines_check_hinfo` → `:135` `…_apply_corrections`
  → `:139` `corrected_mapper` → `:280` `FCEU_EffectiveMapperNo` → `MapperNo = 252`
  → `src/ines_bmap.h:282` `{"SAN GUO ZHI PIRATE", 252, Mapper252_Init}` → `src/boards/252.cpp`（实现完好）。
  日志锚点 `ines_load.cpp:286` ` Mapper corrected by CRC table: %d`。
- ⚠️ **CRC 口径（关键）**：本仓查表键 = **padded(PRG) ‖ CHR 的 CRC32，不含 16 字节 iNES 头**
  （`src/ines_load.cpp:261` `iNESGameCRC32 = cart.crc32;` ← `ines.rs:1067-1075`
  `fceux11_rust_ines_compute_hash` ← `ines.rs:759-781 feed_padded`；与上游 `ines.cpp:992-998` 同口径）。
- ⇒ **原稿给的 `0xF3C85DC3` 查不中**：它是**整文件 CRC**
  （上游 issue #845 附 MD5 `50a88a24…`、SHA1 `0f47f98e…`、文件大小 393,232 字节 = 16 + 262144 + 131072）。

**上游状态**：2.7.0 与 master 的 `src/ines-correct.h` 均**无** 3GO / `F3C85DC3`
（CHINF 段 255 条）⇒ **上游至今未修**（issue #844/#845 仍 open）。
另：issue #845 评论（loong2snow，2026-06-08）称该文件是 **SMYNES 加密版**（「no standard emulator can run」）；
报告者 2026-06-19 又称**找到另一个能在 FCEUX 跑、画面不乱码的 3GO.nes** ⇒ **另一个 dump、另一个 CRC**。

**解决方案步骤**：
1. **不要**写 `0xF3C85DC3`。按本仓口径重算：`CRC32(PRG ‖ CHR)`，PRG/CHR 各自补到 2 幂（补 `0xFF`），不含头；
   3GO.nes 的 PRG = 16×16KB、CHR = 16×8KB **均为 2 幂 ⇒ 无填充** ⇒ 键 = `CRC32(file[16..393232])`。
2. **更稳的取法**：用本仓自己算（加载该 ROM 读日志 CRC，或写一次性探针调
   `fceux11_rust_ines_compute_hash`）⇒ 避免手工 CRC 口径再错一次（本项目已栽过两次）。
3. 条目写法：`RomCorrection { crc32: <本仓口径 CRC>, mapper: 252, mirror: -1 }`
   （`mirror: -1` = 不改镜像，保留头里的垂直镜像）。
4. **必须同时处理「另一个 dump」**：若目标是让用户手上那份能跑，条目要按**实际持有的文件**建，
   可能一条变两条。
5. 在 `ines.rs` 补一条单测：「padded CRC → 命中 ROM_CORRECTIONS → mapper 变 252」。

**验收**：① 加载时日志出现 ` Mapper corrected by CRC table: 252`（`ines_load.cpp:286`）
+ `iNES  Mapper number: 252`；② 画面正常（不再 256KB PRG 不分页）；
③ **反向锚点**：把 `0xF3C85DC3` 写进表**必须查不中**（证明口径认知正确）；
④ `cargo test -p fceux11-formats --lib` 全绿（**本轮实测基线 = 153 passed / 0 failed / 0 ignored**）。

**风险与非目标**：查表项、风险低；**真正的风险是「以为修好了」**（条目不生效 ⇒ 静默无效）；
非目标：不实现 SMYNES 加密 ROM 的解密（上游明确无人实现）；不改死表 `src/ines-correct.h`。

**文档偏差**：① 「约 326 条」→ 实为 **256 条**（326 是文件内 `crc32:` 字段总数，含 `INPSEL_CRC` 的 72 条）；
② 做法 `{crc32: 0xF3C85DC3, …}` **口径不匹配，照抄无效**；
③ 「板级名匹配 `SAN GUO ZHI PIRATE` 对 iNES 无从命中」——结论对但**理由错**：
iNES 路径**根本不做板级名匹配**（`src/ines_init.cpp:38-39` 只比 `num == tmp->number`），
`bmap[].name` 在 iNES 路径是死字段。

---

#### V22 · Lua 鼠标键位掩码判断（P2）

**目标**：修复 Lua `input.get()` 的鼠标键读值（双键同按丢失、中键恒 false）。

**现状（本轮实测）**：
- `src/rust/crates/fceux11-lua/src/bindings/input.rs:135-141`
  ```rust
  if click != 0 {
      if click == 1 { table.set("leftclick", true)?; }
      else if click == 2 { table.set("rightclick", true)?; }
  }
  ```
  ⇒ `click == 3`（双键同按）两个分支都不命中 ⇒ **leftclick/rightclick 全读不到**。
- 取值链：`input.rs:157 fceux11_lua_GetMouseState(&mut x,&mut y,&mut click)`
  → `src/lua-engine.cpp:565-573` → `src/drivers/Qt/input.cpp:1233-1260 GetMouseData`：
  `b |= 0x01`（左，`:1246`）、`b |= 0x02`（右，`:1250`）⇒ **驱动侧确无 `0x04` 中键位**
  （且仅当 `consoleWindow && viewport_Interface` 存在时才非零，`:1240`）。
- `middleclick`（`input.rs:12`，属 `KEY_TO_NAME`）在键盘路径也恒 false：
  `input.rs:120-124` 的 `0x80` 掩码与 SDL 键盘状态值（0/1）永不相交 ⇒ 两条路径都恒 false。
- ⚠️ **本仓 Lua 表没有数值 `click` 字段**（上游 `lua-engine.cpp:2766-2776` 有
  `lua_pushinteger(L, click)`）⇒ 原稿「从读不到变读到 3」**不成立**：按位修正后得到的是两个布尔 true。

**解决方案步骤**：
1. `input.rs:135-141` 改位测试：`click & 0x01` / `& 0x02` / `& 0x04`。
2. `src/drivers/Qt/input.cpp:1244-1251` 补中键：`Qt::MiddleButton` → `b |= 0x04`
   （需覆盖 QWidget / SDL / GL 三种 viewport 实现）。
3. 建议同时补数值字段 `table.set("click", click)`，对齐上游 `lua-engine.cpp:2775-2776`，
   否则上游脚本移植到 F11 会拿到 nil。
4. 与 V25 同批改（同一绑定面、同一文件）。

**验收**：`cargo test -p fceux11-lua --lib` 全绿（现有两条用例只覆盖 1 与 2 ⇒ 补
`click==3 → leftclick && rightclick 同真`、`click==4 → middleclick`）；真机脚本读值正确；
`ctest -LE perf` 全绿。

**风险与非目标**：风险低（Lua 绑定层，不动输入管线）；⚠️ 唯一行为变化：
双键同按时两个布尔由 false 变 true、`middleclick` 由恒 nil 变可能 true ⇒ **提交说明必须写明**；
非目标：不动 zapper 读值路径。

**文档偏差**：① 行号 `:133-140` → 实为 **`:135-141`**；
② 「读到 3」不成立（本仓无数值 `click` 字段）；③ `middleclick` 恒 false 的原因有两半（驱动侧缺 `0x04`
**且**键盘路径掩码恒 0）。

---

#### V14 · volatile 跨线程共享量统一原子化（TraceLogger 环形缓冲 + CDL 六计数器）（P2）

**目标**：让跨线程共享的索引/计数器有内存序保证（**两类缺口性质不同，分开处理**）。

**现状（本轮实测 · 线程归属已逐点落实）**：
- 线程模型：模拟线程 `EmulationThread`（`src/drivers/Qt/ConsoleEmulatorThread.cpp:11-13`，`:41` 调
  `fceuWrapperUpdate()`）；Qt 主线程 `MainThread`（`src/drivers/Qt/ConsoleWindow.cpp:128-133`）；
  TraceLogger 写盘线程 `TraceLogDiskThread`（`src/drivers/Qt/TraceLogger.cpp:2126-2129`，`run()` `:2154`，
  由 GUI 线程在 `:574` 拉起）。
- **环形缓冲确实跨线程**：写者在**模拟线程**
  （`src/x6502.cpp:1058` → `src/debug.cpp:986` → `src/driver_callbacks.cpp:176`
  → `src/drivers/Qt/TraceLogger.cpp:1130` → `:1135` → `src/trace_common.cpp:587-600`
  → `TraceLogger.cpp:1105 pushToLogBuffer` → `:1003 logBuf[logBufHead] = rec;`、`:1017 logBufHead = nextHead;`）；
  读者在**写盘线程**（`TraceLogger.cpp:2206 while (logBufHead != logBufTail)`、`:2217`）。
  声明 `TraceLogger.cpp:114-118`（`volatile int logBufHead/logBufTail`）。
  ⚠️ **这一半是「漏合并上游」**：上游 2.7.0 已改 `std::atomic<unsigned int>`
  （commit `44d3e45d`，2026-10-12），master 再加 acquire/release（commit `0a1ef89e`，2026-11-02）。
- **CDL 六计数器确实跨线程**：写者在**模拟线程**（`src/debug.cpp:481-488`、`:499-515` 经 Rust FFI
  `fceux11_rust_debug_log_cd_data`；`src/ppu.cpp:251-252/269-270/1153-1154`；`src/sound.cpp:194-196`）；
  读者在**GUI 线程**（`src/drivers/Qt/CodeDataLogger.cpp:365-408 updatePeriodic`（QTimer，`:93` connect）、`:581`）。
  定义 `src/debug.cpp:469`、`src/ppu.cpp:204`、`src/debug.h:85`、`src/ppu.h:149`、`src/sound.h:43`。
  `src/debug.cpp:481-513` 的 RMW **只涉及 3 个**计数器（codecount/datacount/undefinedcount），确为非原子。
  ⚠️ **CDL 这半无上游背书**（上游 master `debug.cpp:495` 仍是 `volatile int`）⇒ 是本仓自创提案。
- `src/core_state.h:174-183` 的六个 `volatile int&` 访问器**全仓无调用者**
  （仅 `src/core_state.cpp:129-134` 定义；`tests/f11qa/core_state_test.cpp:162-172` 未断言）
  ⇒ 改返回类型是**零功能影响**改动。

**解决方案步骤**：
1. **环形缓冲（照抄上游 master）**：`TraceLogger.cpp:114-118` 改
   `static std::atomic<unsigned int> logBufHead{0}, logBufTail{0};`；
   写端 `load(relaxed)` → 写缓冲 → `store(release)`；读端 `load(acquire)`；
   `% logBufMax` 改无符号运算；同步修 `:114-116` 那段「volatile」过时注释。
2. **CDL 六计数器（本仓自创）**：声明/定义改 `std::atomic<int>`；
   `src/core_state.cpp:129-134` 返回类型改 `std::atomic<int>&`（零调用者 ⇒ 零风险）。
3. `debug.cpp:481-488`、`:499-515` 的 RMW：**不可简单换 `fetch_add`**（Rust 侧是复合逻辑）⇒
   改为 `load(relaxed)` → FFI 计算 → `store(relaxed)`；若要真正原子化，用 C++20 `std::atomic_ref<int>`
   （Rust FFI 签名 `*mut i32` 保持不变，用 `atomic_ref` 包一层最省事；**先确认工程的 `-std` 版本**）。
4. GUI 读点 `CodeDataLogger.cpp:368-373` 改 `load(relaxed)`。

**验收**：① `cargo test -p fceux11-debug --lib` 全绿（若动 FFI 签名）；
② `ctest -LE perf` 全绿；③ TraceLogger：开日志 + 后台写盘长跑 10 分钟，无断流、无 overrun 刷屏；
④ CDL：开 CDL 后 GUI 计数满足 `codecount+datacount+undefinedcount == cdloggerdataSize`（允许 ±1 时序抖动）。

**风险与非目标**：性能上 CDL 计数器在 `DebugCycle()` 热路径，**必须用 relaxed**；
非目标：**原子化修不掉**「写盘线程启动时复位 head/tail」的逻辑竞态（见 §二末「其他真实缺陷」⑥）。

**文档偏差**：① 「emu 侧自增（`CodeDataLogger.cpp:889` 等）」——`:889` 在 `LoadCDLog()` 内，
是 **GUI 侧载入 .cdl** 的计数路径，emu 侧自增实际在 `debug.cpp:481-515` / `ppu.cpp` / `sound.cpp`；
② 「GUI 侧读（…`:842`）」——`:842` 是**写**（`ResetCDLog`）；③ 「`debug.cpp:481-513` 的 RMW」只涉及三个计数器。

---

#### V19 · GUID 失败置零 + 删死代码（P2）

**目标**：解析失败时给出**确定的**全零 GUID，而不是栈垃圾。

**现状（本轮实测）**：
- Rust `src/rust/crates/fceux11-utils/src/guid.rs:108` `fceux11_rust_guid_scan`：
  两处提前返回**不置零** —— `:114-117`（非 UTF-8）、`:124-127`（`clean.len() != 32`）。
- C++ `src/utils/guid.cpp:22-27`
  ```cpp
  FCEU_Guid FCEU_Guid::fromString(std::string str) { FCEU_Guid ret; ret.scan(str); return ret; }
  ```
  `FCEU_Guid` 无用户提供的构造函数（`src/utils/guid.h:8`）⇒ `data[16]` 为**栈上不确定值**；
  `guid.cpp:42-45` `scan()` 直接转调 Rust，失败即返回 ⇒ `data` 保持栈垃圾。
  `src/utils/valuearray.h:24` `T data[N];` —— **无构造零化**。
- **栈垃圾如何进入最终结果**：
  `src/movie_fm2.cpp:446`（**活路径**，`LoadFM2`）→ `src/movie_playback.cpp:186`
  `if (tempMovieData.guid != currMovieData.guid)` ⇒ 弹「savestate 属于另一个 movie」对话框
  （`:189`）或 `:204-205` 直接 `FCEU_PrintError` + `StopMovie()`；
  再经 `src/movie_fm2.cpp:327` `input.guid = guid.toString().c_str();` ⇒ **存盘时把栈垃圾写进 `.fm2` 文件**。
- `src/utils/guid.cpp:29-40` `hexToByte` —— 全仓只有定义与声明，**无调用点** ⇒ 死代码。

**上游参照**：上游 master `src/utils/guid.cpp:53-58` 在 `scan()` 失败时
`memset(data,0,sizeof(data));` + 报错 ⇒ **失败置零**的现成做法；上游校验也更严（36 字符 + 分隔符位置固定）。

**解决方案步骤**：
1. **Rust 侧置零**（唯一写点）：`guid.rs` 两条提前返回前补 `guid.data = [0u8; 16];`
   （与 V24 的字节切片修复合并做）。
   ⚠️ `guid.rs:109-111` 的空指针早退无法置零（无对象可写）⇒ C++ 侧仍需兜底。
2. **C++ 侧兜底**：`src/utils/guid.cpp:22-27` 改**值初始化** `FCEU_Guid ret{};` ⇒ 必为零。
3. **删死代码**：`guid.cpp:29-40` `hexToByte` 与 `src/utils/guid.h:13` 的声明一并删除。

**验收**：① 构造 `guid 0`（或任意非 36 位）的 `.fm2` ⇒ 加载后 GUID 全零、**同一文件连读两次结果一致**；
② 合法 36 位 UUID 往返不变（现有 `guid.rs` 单测仍绿）；
③ `grep -rn "hexToByte" src/` 归零；④ `cargo test -p fceux11-utils --lib` 与 `ctest -LE perf` 全绿。

**风险与非目标**：风险低（错误路径行为由「不确定」变「确定零」；「两个都坏」的场景将不再误报 mismatch
—— 那是修正而非回归）；非目标：不改 GUID 生成算法；不把 Rust 校验收紧到上游 36 位严格格式（兼容性另案）。

**文档偏差**：路径 `src/guid.cpp` → 实为 **`src/utils/guid.cpp`**（本仓**无** `src/guid.cpp`）。

---

#### V23 · CDL 行为差异文档化 + 写路径对齐（P2 · **原稿定性被推翻**）

**目标**：让 F11 的 CDL 语义与上游对齐，并**把两处差异写进文档**，防后人误当 bug 修。

**现状（本轮实测 · 逐点差异）**：

| 行为 | 上游 2.7.0 | 上游 master | F11 Rust |
|---|---|---|---|
| 读标 data 是否排除 4C/6C | **排除**（`debug.cpp:557`） | 同 | **不排除**（`src/rust/crates/fceux11-debug/src/debug.rs:257-273`） |
| 写路径清零范围 | **仅 FDS**（`debug.cpp:577-589`，`:588 cdloggerdata[j] = 0;`） | 同 | **无条件清零**（`debug.rs:274-286`，`:285`）——该 crate **无 FDS 概念** |

- 调用链：模拟线程 `src/debug.cpp:984 LogCDData` → `:503 fceux11_rust_debug_log_cd_data` → Rust。
- F11 已有一条单测**把现行为钉死**：`debug.rs:785-803 lcd_write_clears_slot`。

**⚠️ 原稿的定性（「F11 站在正确一侧」）不成立**：上游 issue **#771 至今 open**，
维护者 parasyte 明确回复「**We can't break PRG ROM writes. Reverting would not be acceptable, either.**」
⇒ 准确表述是「F11 保留了上游 2024-09-13（commit `65f4a482`）已改掉的旧行为」。

**影响面（会误清 ROM CDL，且条件常见）**：写类指令 + `GetPRGAddress(A) != -1`
（`src/debug.cpp:255-279`：非 FDS 时 `A ∈ $8000–$FFFF` 映射为 PRG 偏移）
⇒ `debug.rs:285` 把该 PRG 偏移的 code/data 标记**整体抹掉**并回退计数
⇒ 绝大多数 mapper 寄存器在 `$8000–$FFFF` ⇒ **游戏的 mapper 写指令每次都会抹掉一处 PRG 偏移的 CDL 标记**
（含向量表/热点代码的可能）；连带 `src/drivers/Qt/CodeDataLogger.cpp:704-716 SaveStrippedROM`
按 `cdloggerdata[i] & 3` 决定保留/清零 ⇒ 被误抹的偏移会被当作「未使用」从剥离 ROM 中删掉。

**解决方案步骤**：
1. **写路径对齐上游（FDS-only）**：给 `log_cd_data` / `fceux11_rust_debug_log_cd_data` 增加
   `is_fds: bool`（C++ 侧 `src/debug.cpp:494` 有 `GameInfo->type` 可传），
   把 `debug.rs:274-286` 的清零块包进 `if is_fds { … }`；
   同步把 `debug.rs:785` 单测改成两条（非 FDS 不清零 / FDS 才清零）。
   ⚠️ 这会**改变现有 .cdl 输出**（变得与上游一致）⇒ 提交说明必须写明。
2. **4C/6C：不改代码，改为文档化偏差**。文档措辞从「F11 站在正确一侧」改为
   「F11 保留上游 2024-09-13 前的行为；上游 #771 至今 open，维护者以『不能破坏 PRG ROM 写语义』为由明确拒绝 revert」。
   若决定跟进上游（加回 4C/6C 排除），须先评估既有 CDL 反汇编脚本是否依赖跳转目标 data 位。
3. 补一条 CDL 对比用例：同一 ROM 分别产出 F11 与上游 2.7.0 的 `.cdl`，
   **只对比 data 位**并把差异显式列为白名单（否则两处差异混在一起会误判回归）。

**验收**：① `cargo test -p fceux11-debug --lib` 全绿（含改写后的 FDS 门控用例）；
② 新增单测：非 FDS + 写指令 ⇒ `cdloggerdata[j]` 不变、计数不变；FDS + 写指令 ⇒ 清零 + 计数回落；
③ 端到端：同一 ROM 跑 CDL，F11 的 `.cdl` PRG 段与上游 2.7.0 对比，**剩余差异仅限已登记的 4C/6C data 位**；
④ 差异声明进 `docs/`（含 #771 链接与上游拒绝 revert 的原话）。

**风险与非目标**：风险低-中（改写清零语义会改变现有 `.cdl` 内容，与上游更一致但对既有工具链是新差异）；
非目标：不实现 FDS 之外的可写 PRG；不引入上游维护者私有分支的方案。

**文档偏差**：① 上游 FDS 清零块行号 `:567-583` → 实为 **`:577-589`**；
② Rust 行号 `:255-273` / `:275-292` → 实为 **`:257-273`** / **`:274-286`**；
③ 「F11 站在正确一侧」**被上游维护者的明确拒绝推翻**。

---

### 追加批 · 批 6：潜伏与纵深（可选）

#### V16 · StringToBytes hex 越读（P3 · **当前不可达**）

**目标**：消除 hex 解码对 `0X` 前缀的字节数误算（上游 issue #857），并防止死路径复活。

**现状（本轮实测）**：
- `src/utils/xstring.cpp:363` 前置条件 `if (str.size()>2 && str[0]=='0' && toupper(str[1])=='X')`。
- `:367` `int bytesAvailable = str.size()/2;`（原稿坐标 ✓）；`:370` `const char* cstr = str.c_str()+2;`。
- **越读精确量**：全长 L、hex 位数 N = L-2；`amt = min(len, L/2)`。
  **偶数 L** ⇒ `amt = L/2 = N/2+1` ⇒ 末轮读 `cstr[N]`（NUL，界内）**与 `cstr[N+1]`（越界 1 字节）**；
  奇数 L ⇒ 只读到 NUL，不越界。（上游 #857 原文即「one byte past the string's NUL terminator」。）
- ⚠️ **两个原稿可达点都不可用**：
  ① `src/drivers/common/config.cpp:306` —— **该文件不在编译列表**
  （`src/CMakeLists.txt` 只有 `config.cpp:167`、`drivers/common/configSys.cpp:444`、`drivers/Qt/config.cpp:519`），
  且 `LoadFCEUConfig`/`SaveFCEUConfig` 全仓无调用方 ⇒ 整个旧 CFGSTRUCT 配置系统是死文件；
  ② `src/movie_fm2.cpp:270/296/306` 位于 `MovieData::installValue`（`:248`）内 —— **全仓无调用方** ⇒ 死代码。
- **实际生效的解析器是 Rust**：`src/rust/crates/fceux11-formats/src/movie.rs:96-108` `decode_hex`
  （过滤空白 → 断言偶数字节数 → `chunks_exact(2)`）与 `:110-117` `hex_digit` —— **无越读**，
  且 `hex_digit` 不接受 `'X'` ⇒ Rust 路径**根本不接受 `0X` 前缀**（`movie.rs:294-298` 遇 `0X` 整段丢弃）。

**上游参照**：commit **`683c3f0`**（2026-11-01，Fixes #857）的实码：
`const size_t bytesAvailable = (str.size() - 2) / 2; // exclude the "0X" prefix`
（同时把 `BytesToString/StringToBytes` 与 `valuearray.h` 的长度参数改 `size_t`）。

**解决方案步骤**：
1. `src/utils/xstring.cpp:367` 改上游写法（不动 ABI）：
   `const size_t bytesAvailable = (str.size() - 2) / 2;`
   可选加固：奇数长度直接 `return false`。
2. **防复活**：`src/movie_fm2.cpp:248` `installValue` 与 `src/drivers/common/config.cpp`
   整体是死代码 ⇒ 建议**加护栏注释 + 登记「禁止复活」**（不删，避免牵连 `movie.h:251` 与搬迁文档）。
   ⚠️ 写进验收：**任何人照原稿去改 `drivers/common/config.cpp:306` 都改了个不参与编译的文件**。
3. 若将来复活 `installValue`，须同步更新 `HexStringToBytesLength`（`xstring.cpp:313-318`）的语义。

**验收**：① 为 `StringToBytes("0XABAB", buf, 2)` 加一条针对性测试（ASan 下无越读；修复前应报 1 字节越读，可先复现再修）；
② ASan 跑上游 #857 POC 形态的 `.fm2` ⇒ 不进 C++ 路径；
③ `grep -rn "StringToBytes" src/` 的调用点清单与本节结论一致；
④ `cargo test -p fceux11-formats --lib` 全绿。

**风险与非目标**：风险低（越读非越写，且当前不可达）；非目标：改 Rust 侧 `decode_hex`；
不引入 `0X` 前缀支持（那是功能增强，另案）。

**文档偏差**：① 「偶数长度值末轮读过 NUL」→ 实为「读到 NUL **再越 1 字节**」；
② 可达点 `drivers/common/config.cpp:306` **不在编译路径上**（照此修改无效）；
③ 「`installValue` 已无调用方」**正确**（已 grep 确认）。

---

#### V20 · movie length 纵深（P3）

**目标**：拒绝畸形 movie 头里的负 `length`，并给 `truncateAt` 加护栏。

**现状（本轮实测）**：
- `src/rust/crates/fceux11-formats/src/movie.rs:315`
  `"length" => md.load_frame_count = val.parse().unwrap_or(-1),` —— **负值照收入库**（原稿坐标 ✓）。
- Rust 主路径守卫齐备（原稿 ✓）：`movie.rs:389-391`、`:492`、`:513` 的 `>= 0` 判定；
  `src/movie_playback.cpp:144` `if (currMovieData.loadFrameCount >= 0)`。
- `src/movie_fm2.cpp:243-246` `void MovieData::truncateAt(int frame) { records.resize(frame); }`
  —— **无负值护栏**（`records` 是 `std::vector`，`resize((size_t)-2)` ⇒ `std::length_error`）。
  ⚠️ 但其 **5 个调用点全传非负值**（`movie_playback.cpp:277`、`movie_record.cpp:225`、`:635`、
  `TasEditor/bookmarks.cpp:351`（0）、`TasEditor/splicer.cpp:378`（`frame+1`））⇒ **属潜伏**。
- ⚠️ **上游 #858 的 abort 路径在 F11 已被 Rust 重写消除**：`src/movie_fm2.cpp:422`
  `fceux11_rust_movie_load_fm2`；C++ 侧 `:448-449` 用 Rust `Vec` 的长度（`size_t`，不可能为负）。
  负 length 只会让三处 `>=0` 守卫**失效**（= 取消截断上限），**不产生负 resize**。

**上游参照**：commit **`9f45054e`**（2026-11-01，Fixes #858）在 `LoadFM2_binarychunk`
（master `movie.cpp:639-649`）补 `if (numRecords < 0) numRecords = 0;`。

**真实影响（P3）**：负 length 被存进模型 ⇒ `movie_playback.cpp:144` 的只读判定失效
⇒ 一个本应按「TAS 工程只读」处理的 `.fm3` 被当成普通电影（可编辑、可再录）；
重存时 Rust `movie.rs:635` 的 `if md.load_frame_count >= 0` 不写 `length` ⇒ 负值不落盘（自愈）。

**解决方案步骤**：
1. Rust 解析层拒收负值（`movie.rs:315`）：`md.load_frame_count = if n >= 0 { n } else { -1 };`
   （`-1` 已是「不限制」哨兵 ⇒ 语义无损）。
2. `src/movie_fm2.cpp:243-246` 加护栏：`if (frame < 0) frame = 0;`（与上游「负数归零」同式，防复活）。

**验收**：① 上游 #858 POC（`length -2` + `binary 1`）不 abort、不抛 `length_error`；
② Rust 单测：`length -2` 解析后 `load_frame_count == -1`；
③ `movie_playback.cpp:144` 的只读判定不被畸形 length 绕过；
④ `cargo test -p fceux11-formats --lib` + `ctest -LE perf` 全绿。

**风险与非目标**：风险低（主路径已安全，纯纵深）；非目标：不改 FM2 格式、不改 Rust `dump` 的输出规则。

**文档偏差**：① 原稿验收锚点「上游 #858 POC 不 abort」**在 F11 本来就不 abort** ⇒ 只能当「不回归」判据，
不是「修复前红」的判据；② 原稿未说明 `truncateAt` 的 5 个调用点全传非负值 ⇒ 该项属**潜伏**。

---

### 追加批 · 核验中发现的其余真实缺陷（**登记，不立任务**）

> 以下均为本轮核验顺带查出、**不在本批任务面**内的缺陷。登记在此防丢失；
> 是否立项由业主按「用户可感 / 解锁 / 防呆 / 成本」另行裁决。

| # | 坐标 | 现象 | 关联 |
|---|---|---|---|
| ① | `src/bus.cpp:560-566` | 注释对上游钳制语义的**推理是错的**（写成「只在 size 恰为单个 bit 时触发」；实际守卫是 `>=`，`size < 2^shift` 时**必然**触发）。**这段错误推理正是 `ad2914d` 删除钳制的依据** | V10（建议同批改注释） |
| ② | `src/cart_class.cpp:28-33` + `src/ines.cpp:68` | `bmap[]` 有 216 条 mapper 而 `MapperEntry` 工厂只有 159 个 ⇒ 无工厂的 mapper 其 legacy `info->Power` 被 `CartInfo_PowerForward` 覆盖后**永不执行**（mapper 218 的 `Mapper218_Power` 亦然） | 与 v2.0.3 已登记的「17 个 MMC3 变体同因」同族 |
| ③ | `src/ppu.cpp:208-216` | `GetCHRAddress`：`&VPage[A>>10][A] - CHRptr[0]`，CHR 缺失时（`CHRptr[0]==NULL`）是**空指针减法**（UB） | V9 同族 |
| ④ | `src/drivers/common/config.cpp` | **整文件不在编译列表**（活文件里的死代码），却与 `configSys.cpp` / Rust 配置系统三套并存 ⇒ 易误导后续工作 | V16 的可达点误判根源 |
| ⑤ | `src/state.cpp:909-912` + `src/state.h:55-60` | `FCEU_SetStatePreSave`（Gateway Phase 6 为「无指针 savestate」设计的机制）**零调用方**；`src/cart_class.h:96-97` 的 `on_save_pre`/`on_load_post` 已在调，但 `Vrc7Cart`（`src/boards/simple_carts.h:38`）未覆写 | V11 的「彻底方案」骨架已备但未接线 |
| ⑥ | `src/drivers/Qt/TraceLogger.cpp:2196` | 写盘线程 `run()` 启动时 `logBufHead = logBufTail = 0;` 与 emu 生产者并发（上游 master `:2499-2500` 同构）⇒ **逻辑竞态，atomic 化修不掉** | V14 之外的独立项 |
| ⑦ | `src/drivers/Qt/TraceLogger.cpp:1006-1016` | 生产者在环形缓冲满时于**模拟线程内** `SDL_Delay(1)` 自旋最多约 10 秒 ⇒ 阻塞模拟线程（掉帧/长时间卡死） | 与 ⑥ 同域 |
| ⑧ | `src/drivers/Qt/CodeDataLogger.cpp:872-918` | `LoadCDLog()` 在 **GUI 线程且不取 `FCEU_WRAPPER_LOCK`** 地逐字节写 `cdloggerdata`，而模拟线程可能正对同一缓冲做 CDL 标记 ⇒ **缓冲级**数据竞争（比计数器竞态严重） | 与 V14 的 CDL 半边同域，但独立 |
| ⑨ | `src/ines-correct.h` | 死表（全仓无 include）；上游同款表仍在用 ⇒ 移植时不要两边都改 | V12 |
| ⑩ | `tests/tests.json` 的 `provenance` 字段（`:1064` / `:1104` / `:1847` / `:2692` / `:3818`） | **5 条旧的 `known-limit:` 克隆句仍留在 `provenance`**（V4 只改 `known_limit`，未动 `provenance`）⇒ 后人 grep 旧句仍会命中。**建议**：按仓库既有惯例（`kgmqa-081` 的「追加式带日期更正」）在 `provenance` **末尾追加**更正指针，**不改原句**（它是带日期的溯源记录） | V4（**下一批小项**） |
| ⑪ | `src/tests/` 整目录 | `src/tests/CMakeLists.txt` 已于 r8 删除（从不参与构建），但该目录**仍有 5 个 `.cpp` + 旧金标 `fixtures/golden_hashes.json`（5 条，活的在 `tests/fixtures/`，13 条）+ `AGENTS.md` + `Testing/Temporary/CTestCostData.txt`** ⇒ 整目录仍是死资产。r8 只按授权范围删了 CMakeLists 并修了 `src/tests/AGENTS.md:122` 的悬空引用；**整目录清理由业主另行裁决** | V4（待裁决） |
| ⑫ | `docs/tech/precision.md:31` | 同一句过期中断描述仍在（「中断只在指令边界轮询（无 hijack/branch-delay…）」）—— V4 改了 `docs/tech/cpu.md` 的三处，但 precision.md 这一处不在当批所有权内 | V4（下一批小项） |

---

## 三、交付批次建议

| 批 | 内容 | 理由 |
|---|---|---|
| 批 1 | **V1 → V6** | 用户可感/解锁：V1 消 abort + 解锁 V2 走查；V6 有 `05bfc1f` 整套先例、一次提交量 |
| 批 2 | **V3 → V4** | 防呆，分钟级纯文本，成本近零 |
| 批 3 | **V5、V7、V8** | 读数/实验类，互不依赖，可并行（文件级互不重叠） |
| 插入 | **V2**（业主 GUI 会话） | 人工，V1 落地后随时安排 |
| **批 4**（r7 新增） | **V9、V10、V24、V11、V25、V15、V13** | **崩溃与内存安全**：V9/V10 是上游已修而本仓未同步的 P0；V24 可致加载畸形文件即 abort；V11 跨进程读档悬空指针；V25 输入键名整体错位（用户可感）；V15/V13 是**漏合并上游现成修复**（1:1 移植）。建议**每项一个提交簇**，单项独立可回退 |
| **批 5**（r7 新增） | **V12、V22、V14、V19、V23** | **正确性与防御性**：V12 需先重新取证 CRC 口径；V22 与 V25 同文件宜同批；V14 环形缓冲半边照抄上游、CDL 半边是本仓自创提案（建议**分开做**）；V19 与 V24 同文件宜同批；V23 会改变现有 `.cdl` 输出 ⇒ 提交说明须写明 |
| **批 6**（r7 新增） | **V16、V20** | **潜伏/纵深**：当前均不可达或无实害，属「防复活」性质；可与批 4/5 打包提交，也可单独挂起 |

> ⚠️ **批 4/5/6 的执行前提**（沿用 §0.4 的证据口径）：本批**全部是静态分析结论、未实跑** ⇒
> 每一项动手前**先复现 + 建回归基线**，再改；`fail_to_fail_max = 0` ⇒ 做坏即红灯。
> ⚠️ **V10 单独提交**（它触及 RAM 初始化路径，风险中），并**先建「≥8K 卡既有 movie 零漂移」的回归基线**。

**批次进度（r8 追记）**：批 2 = **V3 ✅ / V4 ✅**（完成）；批 4 = **V9 ✅ / V24 ✅**，余 V25 / V15 / V13 / V11 / V10 **⬜ 未开始**
（V10 须先建 movie 零漂移基线 + ASan 构建；V11 / V15 / V13 均为**照抄上游现成修复**，可 1:1 移植）；
批 1 = V1 ✅ / V6 ⬜；批 3 = ⬜⬜⬜；批 5 / 批 6 = ⬜（V12 需实际 ROM 文件）。

**批次进度（r9 追记，2026-11-14）**：批 4 = **V9 ✅ / V24 ✅ / V11 ✅ / V13 ✅ / V15 ✅**，余 **V25 / V10 ⬜ 未开始**
（V10 须先建 movie 零漂移基线 + ASan 构建）；批 5 = **V19 ✅**，余 V12 / V22 / V14 / V23 ⬜；
批 1 = V1 ✅ / V6 ⬜；批 3 = ⬜⬜⬜；批 6 = ⬜。
V9 的变异腿已于 r9 实跑（红→绿，读数与 r8 数值推导逐字一致）；实施模式复用 r8 先例
（多 agent 按文件域并行 + 主会话逐条审 diff + 统一串行门禁）。

**批次进度（r11 追记，2026-11-14）**：**批 1 = ✅✅ 全清（V6 落地）**；批 4 = **6/7**（新增 V25 ✅，余 V10）；
批 5 = **3/5 + V14 半**（新增 V22 ✅ V23 ✅，V14 环形缓冲半落地、CDL 半待做，余 V12）；
批 3 = ⬜⬜⬜；批 6 = ⬜。实施模式：两路 agent/波（本机并发上限 ≈2），波间主会话统一串行门禁
+ 变异腿主会话代跑（合并变异归因：每条目标锁均见红）。门禁终读数：`ctest -LE perf` **38/38**（V6 锁
input_famicom_mic_test 入门禁 37→38）；Rust 七条全绿——root 299/0/20、gba-core 249/0/2、
**debug 94/0**（V23 +1）、media 52/0、formats 153/0、utils 51/0、**lua 54/0**（V25/V22，**crate 首次入门禁**）。

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
- [x] **V3**（2026-11-13，r8）：`src/video.cpp` 的 A4-1 否决护栏注释在位（写明否决理由 +「勿再做消费计数门控」）；
      `FCEU_SoundCPUHook` 的**有意省略**探点已在代码内登记（并把探点口径修正为 **6 个**）；**纯注释、零行为变更**；
- [x] **V4**（2026-11-13，r8）：`tests/tests.json` 的 5 条 `known_limit` 改为与事实相符（037/038/056/078/107）；
      `docs/tech/cpu.md` / `F11QA.md` / `f11qa-accuracy-backlog.md` 的过期措辞已改；
      死文件 `src/tests/CMakeLists.txt` 已删（零构建引用 + 覆盖未丢失已核，其指向该文件的 `src/tests/AGENTS.md:122` 一并修正）；
      v1.8 计划活引用 3 处 + 生成器及产物已改（**逐字一致已验**）；
      **049/050/051 的基线翻转未被顺手做掉**（只登记于 `docs/tech/f11qa-accuracy-backlog.md` §4）；
      ⚠️ `expected` / `failure_means` / `pass_to_fail` **零改动**；
- [ ] **V5**：PGO 产出**一组可复现读数**（训练场景 + 构建命令 + 基准表 + 结论）；
- [x] **V6**（2026-11-14，r11）：FC 麦克风协议按参照语义实现（wiki `Standard controller` Famicom 节 +
      Mesen2 双信源：**bit2 电平直读、仅 $4016、仅 P2**，1996 逐读翻转 hack 退役、键映射保留）
      + 锁测试 `input_famicom_mic_test` 入门禁（**37 → 38**，T1–T6；变异 M-1..M-5 两轮合并实测各归其位）
      + 桥注入面 `kagami_bridge_set_famicom_mic` + sut_input 保留值 0x40；
      `kgmqa-108` 行为读数记录在案（修复前后逐字相同：0x02 稳定——判定逻辑未取证，只登记不判；
      基线翻转与 manifest `sut_input:[1,64]` 留待同级评审）；
      ⚠️ 人工项待做：真机吹气（游戏兼容面实测长尾）；
- [ ] **V7**：Qt 侧 profiler 菜单动作在位，且能产出报告；
- [ ] **V8**：A7 收益上限实验给出**读数结论**（值得/不值得/只做哪子集）；
- [x] **V9**（2026-11-13，r8）：mapper 218 的 `VPageR` 改上游写法（`src/boards/datalatch.cpp`，逐字照抄 commit `6738471`）
      + 新夹具 `tests/fixtures/mapper_mapper218.nes`（生成器登记，单文件生成）
      + 常驻锁测试（**断言「有效地址落在 NTARAM 内」**，不复述指针公式 ⇒ 不会与实现同错）
      ⇒ 实跑读数：`mapper_load_test` 输出 `all 8192 CHR addresses land inside NTARAM`（10/10 PASSED）；
      ⚠️ ~~变异腿未实跑~~ **r9 已实跑（红→绿）**：还原旧写法 ⇒ `FAIL: … 6145 of 8192 CHR addresses fall
      outside NTARAM (first at 0x07FF, worst is 6144 bytes out)`，与 r8 数值推导**逐字一致**；还原后回绿；
      ⚠️ ~~上游 #851 复现 ROM 仍需外部取~~ **r10 已取得并验证（两条尾巴全清）**：业主从 nesdev 论坛帖
      （Drag 楼层，附件 `download/file.php?id=31730`）下载 `mapper218bad.zip`——zip SHA256
      `3fa6f72c8053bb26f7d6d38a5004d169c10838303df3d80188e1c82975f17d93`、ROM `efed56a8e6a1706e89defab75609ad7eef50cb567920567eed339aea3c78be4f`，
      iNES mapper 218 + CHR-RAM + 4-screen 位，文件时间戳与 #851 开立日期吻合 ⇒ **修复版装载 + 600 帧全程无崩溃**
      （`f11qa_blargg_runner --rom --frames 600`，$6000 读数 0x34 = 该 ROM 自身输出）；⚠️ **真 ROM 的变异腿（旧写法）
      在本仓内存布局下不段错误**——越界 0x1800 字节落在相邻活动数组 = 静默损坏（与 Drag 在 FCEUX stable 看到的
      「反复复位」同族；zlago 的段错误是其布局特有）⇒ 布局无关的越界证明仍由合成锁测试承担
      （`mapper_load_test` 断言有效地址域，r9 变异已证红）；证据存 `Research_only/w204/`（gitignored）；
- [ ] **V10**：CHR-RAM 垫层改 8K（`src/ines_init.cpp:51`，**保留 `FCEU_MemoryRand` 真实尺寸**）
      + `src/bus.cpp` 补回上游四行掩码钳制 + `setup_chr_mapping(0, buf, 1024, 1)` 单测
      + **≥8K 卡既有 movie 零漂移**（⚠️ **<8K 卡画面会变、其 movie 会漂移 —— 口径必须写明**）
      （⚠️ **r9 更正：此条原为 `[x]`，系 r7 并入时的误预勾——V10 实际未开始**（看板与 §三 批次进度
      一直是 ⬜），现改回未勾；先建 movie 零漂移基线 + ASan 构建再动）；
- [x] **V24**（2026-11-13，r8）：`guid.rs` 的字节切片改按字节处理（`clean` 改 `Vec<u8>` + 按字节取组，
      函数内**再无 `&str` 区间切片**）+ 三条失败路径统一置零 + 锁测试
      ⇒ 实跑读数：`cargo test -p fceux11-utils --lib` **51 passed / 0 failed**（基线 46，+5）；
      **变异腿已实测**：还原 `&clean[..]` ⇒ `not a char boundary` ⇒ `extern "C"` 帧内 panic ⇒ 进程 abort（`0xC0000409`）
      ⇒ 还原后 SHA256 与变异前一致、复跑全绿；
      ⚠️ 唯一行为变化：含非 ASCII 字节的 GUID 输入由「部分解析」变「全零」（**已裁决**：非 ASCII 输入本就不是 GUID）；
- [x] **V11**（2026-11-14，r9）：`src/boards/vrc7.cpp` 的 `StateRestore` 补
      `if (VRC7Sound) OPLL_forceRefresh(VRC7Sound);`（落 StateRestore 非 Sync，避免每次 PRG/CHR 写触发；
      `OPLL_forceRefresh` 的 NULL 早退 / 无分配无 I/O 已核实 `emu2413.c:613-626`）；
      `FCEUSTATE_INDIRECT` 数据条目修复后恰余 `state.cpp:131`（RAM）与本处两处；
      ⚠️ 人工项待做：Lagrange Point 跨进程读档不崩 + 音色连续（构建门禁已全绿）；
- [x] **V25**（2026-11-14，r11）：Lua 键名表改 **SDL scancode 基准**（96 个字段名全部不变、读取下标
      全换——改名对照表 = 索引对照表，逐项进提交说明）+ 掩码改「值 ≠ 0」+ 逐项映射锁测试
      （capslock/backspace/方向键/F1–F12 + 双侧修饰键 + 旧 VK 位反锁，49 → 54 项，**crate 首次入门禁**）；
      ⚠️ **计划偏差（r11 实测改正）**：「`&0x80` 恒不相交」半句不成立——FFI 桥把按下键归一化 0x80，
      生产上多数键其实能读到位；主要缺陷是 96 键名整体错位 + 三个 0x01 例外键恒 false；
      ⚠️ 人工项待做：真机 Lua 读 `input.get()` 与实际按键一致；
- [x] **V15**（2026-11-14，r9）：`JSreturnGui/JSreturnEmu` 1:1 移植上游（与 master 快照逐点一致，
      计划五坐标**零漂移**）+ `emuThreadPollInputs` 接进 `fceuWrapper.cpp` 帧首
      + 线程模型断言以**常驻轻量探针**落地（同线程命中 ⇒ 去重 `FCEU_printf` 告警一次，预期静默）；
      ⚠️ 人工项待做：狂按 A/B 录 movie → 回放逐帧一致（验收②）；
- [x] **V13**（2026-11-14，r9）：AVI 四个索引改 `std::atomic<int>` + acquire/release
      （内存序规律照上游 master：对端索引 acquire 读 / 自有索引 relaxed 读 / 写端一律 release 存 / 重置 relaxed；
      消费端 `AviRecordDiskThread.cpp` 也已原子化——计划 §二现状 `:742` 本就列了三路径）
      + 可变访问点 27 处全部 `.load/.store`（grep 全量核对）；hotfix1 P1-8 的 overrun 护栏结构保留；
      ⚠️ 人工项待做：录 AVI ≥10 分钟压测记录（验收①）；
- [ ] **V12**：3GO.nes 的矫正条目按**本仓 CRC 口径（padded PRG+CHR，不含头）**重建
      + 加载日志出现 `Mapper corrected by CRC table: 252`
      + **反向锚点：`0xF3C85DC3` 必须查不中**；
- [x] **V22**（2026-11-14，r11）：Lua 鼠标改位测试（`click & 0x01/0x02/0x04`，click==3/4 锁测试）
      + 驱动补中键 `0x04`（⚠️ 计划预期逐 viewport 补，实测三种实现已各自支持，只需 `GetMouseData`
      一处）+ 数值字段 `table.set("click", click)`（对齐上游，行为增加已声明）；
- [ ] **V14**：TraceLogger 环形缓冲改 atomic（照抄上游 master 的 release/acquire）
      + CDL 六计数器原子化 + `core_state` 访问器改类型 + CDL 计数一致性读数
      （🔧 **r11：上半已落地**——环形缓冲 10 访问点全 `.load/.store`，上游 `44d3e45d`/`0a1ef89e`
      diff 逐段对照，读端取 V13 形（局部快照 + 出循环一次 release store）；**下半（CDL 六计数器 +
      core_state + GUI 读点）待做**——与 V23 同文件（debug.cpp），错峰执行）；
- [x] **V19**（2026-11-14，r9）：GUID 失败置零双保险达成（Rust 半边已随 V24 落地，本轮核实三条失败路径
      均置零；C++ 半边 = `fromString` 的 `FCEU_Guid ret{};` 值初始化）+ `hexToByte` 死代码删除
      （删前零调用已核、删后 `grep` 归零）+ 新测试 `tests/f11qa/utils/guid_test.cpp` 注册进
      `tests/CMakeLists.txt`（ctest 名 `guid_test`，无 LABELS ⇒ 自动进 `-LE perf` 门禁，**36 → 37**）；
      ⚠️ **关键发现：单腿变异测不出 C++ 半边**（两层零化在 C++ 可达输入上完全重叠、互相掩盖）⇒
      **双腿变异实测**（`ret{}`→`ret` + 摘 `guid.rs` len≠32 分支置零）：Rust 锁
      `test_guid_scan_invalid_length_zeroes_guid` 红（0xA5 预填残留）+ `guid_test` 两条垃圾断言红
      （栈垃圾 + repeated mismatch；非 UTF-8 腿仍绿 = 只拆一腿的精确对照）⇒ 还原后全绿；
- [x] **V23**（2026-11-14，r11）：CDL 写路径对齐上游 FDS-only（FFI 增 `is_fds`，C++ 判据
      `GameInfo->type == GIT_FDS`——⚠️ 计划写的 grepFDS 惯例名不成立，本仓惯例是 GIT_FDS 比较）
      + 差异声明进 `docs/tech/debug-regression-checklist.md` §七.7（含 #771 与上游拒绝 revert 原话逐字）
      + 锁测试 93 → 94（判别腿变异已实测红）；**⚠️ 行为变更：非 FDS 卡带的 .cdl 输出自本版改变**
      （写指令不再误抹 PRG 槽）；验收③「同一 ROM 与上游 2.7.0 端到端 .cdl 对比」**登记不做**
      （需构建上游，留待后续立项）；
- [ ] **V16**：`xstring.cpp` 的 `bytesAvailable` 改 `(str.size()-2)/2` + 死路径护栏注释；
- [ ] **V20**：Rust 侧拒收负 `length` + `truncateAt` 负值归零 + `#858` POC 不 abort；
- [ ] **NES 零回归**：`ctest -LE perf` **全绿**（r11 实跑：**38/38**；37 → 38 为
      input_famicom_mic_test 入门禁）
      + F11QA 矩阵 `pass_to_fail = 0` + grade ≥ B（矩阵**本地无入口** ⇒ 以 CI 复核为准
      ⇒ 该子项**保持未勾**并待 CI；⚠️ 另注：`docs/tech/F11QA.md` 里 R4 gate 的 `total==120` 口径
      已随 kgmqa-123 落地变为 123 ⇒ 属期望值口径变更，**须同级评审**，本轮只登记未改）；
- [x] **六条 Rust 锁测试门禁全绿**（r8 实测；**r11 扩为七条并逐字复核**）：root **299 passed / 0 failed / 20 ignored**、
      vendor（gba-core）**249 / 0 / 2**、debug **94 / 0**（r11：V23 +1，基线 93）、media **52 / 0**、formats **153 / 0**、
      utils **51 / 0**（基线 46 ⇒ V24 +5）、**lua 54 / 0**（r11：V25/V22，**crate 首次入门禁**，基线 49）；
      两态 `cargo check --workspace --no-default-features` 与 `cargo check --workspace` 均 **EXIT=0**。

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

# 追加批（V9–V25）相关门禁与工具
# 1) ASan 构建（V10 的越界验收；本仓已有开关，CMakeLists.txt:202）
cmake -S . -B build-asan -DCMAKE_BUILD_TYPE=RelWithDebInfo -DFCEUX11_ASAN=ON
cmake --build build-asan --config RelWithDebInfo   # ⚠️ Ninja 归档并发会 LNK1104 ⇒ 用 -j 1

# 2) 本批涉及的两个 crate 的锁测试
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-formats --lib"   # V12 / V16 / V20（本轮基线 153 passed / 0 failed）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-utils --lib"      # V19 / V24
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-lua --lib"       # V22 / V25
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"      # V14 / V23

# 3) 上游参照快照（复核用，只读；直连被拦，必须走本机代理 127.0.0.1:7897）
#    仓库 TASEmulators/fceux；tag fceux-2.7.0；master
curl.exe -s -x http://127.0.0.1:7897 "https://raw.githubusercontent.com/TASEmulators/fceux/fceux-2.7.0/src/ines.cpp"
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/6738471"   # V9
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/8510fb8"   # V10
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/44d3e45d"   # V14
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/0a1ef89e"   # V13 / V14
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/683c3f0"   # V16 / V19
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/commits/9f45054e"   # V20
curl.exe -s -x http://127.0.0.1:7897 "https://api.github.com/repos/TASEmulators/fceux/issues/771"       # V23
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
| r7 | 2026-11-13 | **追加批并入：`docs/plans/FCEUX11-v2.0.4_追加.md`（静态分析技术方向稿）逐条核验后并入本计划 ⇒ 本计划现含 V1–V25（新增批 4/5/6 = V9–V25）。** ① **来源与核验口径**（新增 §0.4）：三路**只读**核验并行（按文件域切分、互不重叠），逐条回源码取 file:line；上游对照走 GitHub API + raw 快照（`TASEmulators/fceux`，tag `fceux-2.7.0` 与 master）经本机代理 `127.0.0.1:7897` 取回，commit diff 与 issue 原文逐条核对。基线 `wip204` @ `c47047a`，**全程未改任何生产代码**。本批全部为**静态分析、未实跑** ⇒ 落地前必须先复现 + 建基线；判红体系收益同样为 0（决策依据仍是「用户可感/解锁/防呆/成本」），而 `fail_to_fail_max = 0` 仍成立。② **判定**：原稿 12 项中 **7 项属实**（V9/V10/V11/V13/V14/V15/V19）、**5 项部分属实**（V12/V16/V20/V22/V23），**核验中新发现 2 项**（V24 GUID 的 UTF-8 字节边界 panic ⇒ `extern "C"` 帧内 abort；V25 Lua 键名表用 Windows VK 码索引 SDL scancode ⇒ 键名整体错位）⇒ 共 17 项（V17/V18/V21 为原稿留的空号，不启用）。③ **原稿 15 处文档偏差**逐条登记在 §二各节末（路径写错：`src/AviRecord.cpp`→`src/drivers/Qt/AviRecord.cpp`、`src/guid.cpp`→`src/utils/guid.cpp`、`src/ines-correct.h` 死表；行号漂移：V10 零尺寸分支 `:59-62`→`:57-59`、V22 `:133-140`→`:135-141`、V23 `:567-583`→`:577-589`、Rust `:255-273/:275-292`→`:257-273/:274-286`；口径不匹配：**V12 的 `0xF3C85DC3` 是整文件 CRC，而本仓查表键是 padded PRG+CHR CRC（不含头）⇒ 照抄静默无效**；事实性前提被推翻：**V13 的「AVI 可能无人用」不成立（真实在用），V23 的「F11 站在正确一侧」被上游维护者明确拒绝 revert 推翻**，V16 的两个可达点**都不在编译/调用路径上**，V20 的上游 #858 abort 路径**已被 Rust 重写消除**；V10 的「垫层不走 `FCEU_MemoryRand`」措辞**照字面执行会造成 movie 漂移**（上游实为「保留 MemoryRand 真实尺寸 + 只 memset 垫层」）。④ **上游修复交叉验证**（GitHub API 取回 diff）：`6738471`（mapper 218 单行）、`8510fb8`（CHR-RAM 8K 垫层）、`44d3e45d`/`0a1ef89e`（TraceLogger + AVI 环形缓冲改 atomic + acquire/release）、`683c3f0`（StringToBytes + GUID 失败置零）、`9f45054e`（movie 负 length）、`84b10058`（GUID 越界+置零）——⇒ **V15/V13/V14 的环形缓冲半边是「漏合并上游现成修复」，本仓修完即领先**；V11 的上游 `vrc7.cpp` 至 2.7.0/master **逐字节相同（SHA256 一致）且未修** ⇒ 本仓修完即领先。⑤ **新增发现的事实底座**：`ROM_CORRECTIONS` 实为 **256 条**（原稿「约 326 条」混算了 `INPSEL_CRC`）；V10 的掩码钳制**本仓曾经有**，被 `ad2914d`（hotfix1 Phase 4，2026-07-14）删掉 ⇒ 「F11 独有回归」有 git 证据；本仓 `FCEU_malloc` 自身清零 ⇒ 垫层已清零；**ASan 开关本仓已有**（`CMakeLists.txt:202` `option(FCEUX11_ASAN …)`，默认 OFF）；`fceux11-formats` 锁测试本轮实测基线 **153 passed / 0 failed**。⑥ **另登记 9 条不立任务的缺陷**（§二末表）：含 `bus.cpp:560-566` 那段**错误推理正是钳制被删的依据**、`bmap[]` 216 mapper vs 159 工厂导致未迁移 mapper 的 `Power` 永不执行（与 v2.0.3 已登记的 17 个 MMC3 变体同族）、`GetCHRAddress` 的空指针减法、`drivers/common/config.cpp` 整文件不在编译列表、`FCEU_SetStatePreSave` 零调用方（Gateway Phase 6 半成品）、TraceLogger 启动复位竞态与模拟线程内 10 秒自旋、`LoadCDLog` 的 GUI 无锁写。⑦ 原稿已按其自述的下一步**归档**至 `docs/history/plans/FCEUX11-v2.0.4_追加.md`（内容冻结）。 |
| r8 | 2026-11-13 | **批 2（V3 + V4）与批 4 的 V24、V9 落地**（业主采纳「先做批 2，再进批 4 的 V24 + V9」的建议）。**门禁终读数**：`ctest -LE perf` **36/36**（9.84 s）；六条 Rust 锁测试 **全绿**（root 299/0/20、gba-core 249/0/2、debug 93/0、media 52/0、formats 153/0、**utils 51/0**（基线 46 ⇒ V24 +5））；两态 `cargo check --workspace [--no-default-features]` 均 EXIT=0。① **实施方式**：四路并行（三个 agent 按文件域切分 + **V3 那路 agent 在落笔前结束回合，由主会话接手落地**）；**主会话逐条审 diff**（含核实 `tests/tests.json` 只动 `known_limit`/`spec_source`、`expected`/`failure_means`/`pass_to_fail` 零改动；抽查 056/078 的基线 `passed=true`、037/038/107 的基线 `false` 与新措辞一致）。② **V3**：A4-1 否决护栏进 `src/video.cpp`（写明 chunk-8 任意帧异步消费 ⇒ 金标漂移/谓词恒真，**勿再做消费计数门控**）+ `FCEU_SoundCPUHook` 的「有意省略探点」登记进 `src/sound.cpp`（探点口径订正为 **6 个**，实测坐标 `fceu.cpp:804` / `ppu_rendering.cpp:301,686` / `sdl-sound.cpp:376` / `ConsoleWindow.cpp:629,666`）；**纯注释、零行为变更**。⚠️ **计划的前提被证伪并已按实测改正**：r7 写的「照 `src/profiler.h` 的 A4 护栏宏体」——**该宏体不存在**（`profiler.h` 与 `fceu.cpp` 均无）⇒ 改用本仓真实惯例（`// 项号: 结论` 式注释，仿 `sound.cpp:1457` 的 A4-2、`input/snesmouse.cpp:82` 的 D9 A4）。③ **V4**：5 条 `known_limit` 逐条回源码复核后改写（037 向量级 hijack **已建模**、038 真根因是 `Cpu::set_db()` 零调用点、056/078 **已 RESOLVED**、107 SNES 鼠标协议**已实现**）；`docs/tech/{cpu,F11QA,f11qa-accuracy-backlog}.md` 过期措辞订正（新增「42 pass / 1 fail」实测口径 + 条数漂移注）；**死文件 `src/tests/CMakeLists.txt` 整份删除**（零构建引用 + 覆盖未丢失已核；其指向该文件的 `src/tests/AGENTS.md:122` 一并修正）；v1.8 计划的 3 处活引用改指 `docs/history/plans/`（**生成器与产物逐字一致已验**）；**049/050/051 只登记不改**（`docs/tech/f11qa-accuracy-backlog.md` §4）。④ **V24**：`guid.rs` 改全程按字节处理（`clean` 改 `Vec<u8>` + 按字节取组，**函数内再无 `&str` 区间切片**）+ 三条失败路径统一置零 + **+5 条锁测试**；**变异腿已实测双向可红**（还原 `&clean[..]` ⇒ `not a char boundary` ⇒ `extern "C"` 帧内 panic ⇒ 进程 abort `0xC0000409`；还原后 SHA256 一致、复跑全绿）；**一处裁决已定**（含非 ASCII 字节的 GUID 输入 ⇒ **整串置零**，非旧实现的「部分解析」）。⑤ **V9**：`src/boards/datalatch.cpp` **逐字照抄上游 commit `6738471`**；新建夹具 `tests/fixtures/mapper_mapper218.nes`（生成器登记 + 单文件生成，header `4e 45 73 1a 01 00 a0 d0 …` ⇒ mapper `0xDA`=218，16400 字节）；新增锁测试 ⇒ 实跑读数 `all 8192 CHR addresses land inside NTARAM`（`mapper_load_test` 10/10 PASSED）。⚠️ **r7 的锁测试写法被本轮更正**：r7 写的「`VPageR[i]` 落在 NTARAM 区间内」**不成立**（上游的偏移基址写法下 `i>0` 的指针本体就在 `NTARAM` 之前）⇒ 改为断言「**有效地址** `VPageR[v>>10] + v` 落在 NTARAM 内」（不复述指针公式 ⇒ 不会与实现同错）；⚠️ **V9 的变异腿未实跑**（旧写法 ⇒ 8192 中 6145 个越界、最深 0x1800 字节 —— 数值推导）；⚠️ **上游 #851 复现 ROM 仍未取**（本机无，**未验**）。⑥ **本计划自身的修正**：§二 V9 小节按上述更正重写；§二末「其他真实缺陷」表增 3 条登记（`provenance` 里 5 条旧 `known-limit:` 克隆句、`src/tests/` 整目录仍是死资产、`docs/tech/precision.md:31` 同一句过期描述）。⑦ `CHANGELOG.md` 新增 **`[Unreleased] — v2.0.4（在途）`** 段（v2.0.4 此前**没有**在途段，发版时并入正式版本段）。⑧ **未做**：V9 变异腿未实跑；上游 #851 ROM 未取；`src/tests/` 整目录清理、`provenance` 5 处旧句、`precision.md:31` 三处**只登记待裁决**；**未推送**（push 需业主逐次明确指示）。 |
| r9 | 2026-11-14 | **第一梯队（批 4 收尾）四项落地 + V9 收尾（业主拍板「第一梯队 5 项并行」）**：V11 / V13 / V15 / V19 代码落地，V9 变异腿实跑闭合。**门禁终读数**：`ctest -LE perf` **37/37**（23.75 s；**36 → 37** = `guid_test` 入门禁）；六条 Rust 锁测试**全绿且与 r8 基线逐字一致**（root 299/0/20、gba-core 249/0/2、debug 93/0、media 52/0、formats 153/0、utils 51/0）；两态 `cargo check` 均 EXIT=0。① **实施方式复用 r8 先例**：五路 agent（四路实施按文件域切分 + 一路 V9 只读取证）+ 主会话逐条审 diff + 统一串行门禁；⚠️ 本机 agent 并发上限实测 ≈2（超出即 `user concurrency limit exceeded`）⇒ 分批续跑。② **V11**：`src/boards/vrc7.cpp` 的 `StateRestore` 补 `if (VRC7Sound) OPLL_forceRefresh(VRC7Sound);`（+6 行，落 StateRestore 非 Sync；上游 `vrc7.cpp` 至 master 未修 ⇒ 修完即领先）；`OPLL_forceRefresh` NULL 早退 / 无分配无 I/O 已核（`emu2413.c:613-626`）；`FCEUSTATE_INDIRECT` 数据条目恰余 `state.cpp:131` + `vrc7.cpp:51` 两处。③ **V13**：四索引改 `std::atomic<int>`，**27 个可变访问点全部 `.load/.store`**；内存序照上游 master（对端 acquire / 自有 relaxed / 写端 release / 重置 relaxed）；消费端 `AviRecordDiskThread.cpp` 一并原子化——⚠️ 计划 §二现状 `:742` 本就列三路径，agent 初判「扩围」系误读，实际零扩围；`AviVideoCodec.h` 是本仓拆 TU 自建头（上游 404，无需参照）；hotfix1 P1-8 overrun 护栏结构保留；`aviRecordClose` 的链式重置拆两条独立 `.store`（atomic store 返回 void）。④ **V15**：1:1 移植上游（`JSreturnGui` atomic + `JSreturnEmu` 快照 + `emuThreadPollInputs` 帧首移交），计划五坐标**零漂移**；验收①线程模型断言以**常驻轻量探针**落地（同线程命中 ⇒ 去重 `FCEU_printf` 告警一次；预期静默）；回放路径证据链核对（回放期核心整体跳过 driver Update，`input.cpp:416-425`）⇒ movie 行为零变化。⑤ **V19**：Rust 半边已随 V24 完成（三条失败路径置零已核实）⇒ 本轮只做 C++ 半边（`ret{}` 值初始化）+ 删 `hexToByte` 死代码（grep 归零）+ 新测试 `tests/f11qa/utils/guid_test.cpp`（`tests/CMakeLists.txt:525` 注册，照 pixbuf_pool_test 模式，无 LABELS ⇒ 自动进 `-LE perf` 门禁）。⚠️ **关键发现：单腿变异测不出 C++ 半边**——两层零化（Rust 失败分支 / C++ `ret{}`）在 C++ 可达输入上完全重叠、互相掩盖 ⇒ **双腿变异实测**（`ret{}`→`ret` + 摘 `guid.rs` len≠32 分支置零）：Rust 锁 `test_guid_scan_invalid_length_zeroes_guid` 红（0xA5 预填残留）+ `guid_test` 两条垃圾断言红（栈垃圾指针残渣 / repeated mismatch；**非 UTF-8 腿仍绿** = 只拆一腿的精确对照）⇒ 还原后全绿。⑥ **V9 收尾**：变异腿实跑达成——`FAIL: … 6145 of 8192 CHR addresses fall outside NTARAM (first at 0x07FF, worst is 6144 bytes out)`，与 r8 数值推导**逐字一致**，还原后回绿；**#851 复现 ROM 全通道取证不可得**（nesdev.org 对 curl / 内置浏览器 / 翻译代理 / Wayback / archive.today 全封锁；issue 本体无附件 0 评论；唯一线索 = 论坛帖 `viewtopic.php?p=310352` 的 zip）⇒ **待业主人工浏览器下载**。⑦ **文档自洽性修正**：§五 **V10 的 `[x]` 系 r7 并入时误预勾**（V10 实际未开始）⇒ 改回 `[ ]`；§一「交付批次视角」行的 r7 旧文与看板矛盾 ⇒ r9 同步；看板追加批 2/14 → **6/14**。⑧ **人工项待做**（不影响判据勾选，逐项已标注）：V11 跨进程读档（Lagrange Point）、V13 录 AVI ≥10 分钟压测、V15 狂按 A/B 录 movie 回放比对。⑨ **上游本地副本**：业主已下载上游最新代码（本轮未消费——GitHub 经代理可达，已按文件取快照；副本可服务下一梯队 V10/V14/V23）。⑩ **未推送**（push 需业主逐次明确指示）。 |
| r10 | 2026-11-14 | **V9 两条尾巴全清（#851 真 ROM 到位 + 双向验证）+ V6 信源取证——业主人工下载 nesdev 材料（本轮零生产代码改动）。** ① **ROM 取证**：`mapper218bad.zip`（383 B，论坛 Drag 楼层 `download/file.php?id=31730`）解出 `mapper218bad.nes`（32784 B，SHA256 `efed56a8…78be4f`；zip SHA256 `3fa6f72c…f17d93`）；iNES 头 `4E 45 53 1A 02 00 A8 D8` ⇒ PRG 2×16KB、byte6=`0xA8`（4-screen 位）、byte7=`0xD8` ⇒ **mapper 0xDA=218 + CHR-RAM**，文件时间戳 Sep 4 04:06 与 #851（2026-09-03T22:16Z）吻合。② **绿腿**：修复版 `f11qa_blargg_runner --rom … --frames 600` 全程无崩溃（770 ms，$6000=0x34 是 ROM 自身输出非 blargg 签名）。③ **红腿（诚实登记）**：变异回旧写法后同 ROM **不段错误**——`VPageR+index` 的越界（最深 0x1800 B）落在**本仓布局的相邻活动数组**里 = 静默损坏；论坛出处 Drag（2026-09-03）原话「写 PPU $0814 触发复位、$0815 跳 ROM 随机位置、$0816 崩溃；ROM 只初始化 + 清空全部 PPU 内存」与旧写法踩坏 `vnapage_[]` 指针表的形态逐点吻合，zlago 同日据此开 #851；**布局无关的越界证明仍由合成锁测试承担**（r9 已证红）。④ **V6 信源**：wiki **无** `Famicom microphone` 独立页（业主实开确认空页）；`Standard controller` 页 Famicom 节载明——`$4016` 读格式 **`xxxx xMES`**（bit0 主手柄 / bit1 扩展口 / **bit2 = 麦克风，仅 $4016**）、**「游戏普遍追踪报告的 0↔1 翻转而非特定电平」**、麦克风音频在卡带边缘前混入 APU 输出、wiki 自注 TODO「个别机型报告疑似反装反相」⇒ **review 报告的「位 2/3」说法永久定性为未获证实**（wiki 只记 bit2），Mesen2 参照实现与 wiki 一致；V6 的「模拟量细节未取证」缺口就此收窄为「wiki 有据 + 数字位语义明确，模拟混音与机型差异不在数字位实现范围」。⑤ 材料（两页 wiki HTML + 论坛帖 HTML + zip + 解出 ROM）存 `Research_only/w204/`（gitignored）。⑥ 未推送（push 需业主逐次明确指示）。 |
| r11 | 2026-11-14 | **波次实施：批 1 全清（V6）+ 批 4/5 再落四项（V25/V22/V23/V14 上半）——业主拍板「执行下一步，每次两个 agent 并行」。** **门禁终读数**：`ctest -LE perf` **38/38**（37→38 = V6 锁入门禁）；Rust **七条**全绿（root 299/0/20、gba-core 249/0/2、**debug 94/0**（V23 +1）、media 52/0、formats 153/0、utils 51/0、**lua 54/0**（crate 首次入门禁，基线 49））；两态 `cargo check` EXIT=0。① **实施方式**：两波 × 两路 agent（按文件域切分，V25+V22 强制同 agent——同一绑定面）；主会话逐条审 diff + 串行门禁 + **变异腿由主会话合并施加**（每条目标锁均见红，归因逐测登记）。② **V25**：KEY_TO_NAME 改 SDL scancode 基准（99 项逐行注明 SDL 常量名，值取自本机 vcpkg SDL_scancode.h）；**Lua 字段名零改名**——变化的是 96 键的读取下标（对照表进提交说明）；掩码改「值 ≠ 0」（双形态 sweep：归一化 0x80 与原始 0/1）。**计划偏差改正**：「&0x80 恒不相交」不成立——`fceWrapper_GetKeyboardState`（`fceu_callbacks.cpp:293-302`）把非零键态归一化 0x80 ⇒ 生产上多数键能读到位；主缺陷 = 96 键名整体错位 + 三个 0x01 例外键恒 false。⚠️ 已知缺口：scancode ≥ 256（MODE/媒体键）在 FFI 256 字节窗口外不可表示。③ **V22**：click 位测试 + 数值字段；**三 viewport 已各自支持中键**（GL/SDL/QWidget 的 `getMouseButtonState` 均含 `Qt::MiddleButton`）⇒ 只补 `GetMouseData` 一处；`GetMouseData` 行号 1233→1272（V15 落地后漂移）。④ **V23**：FFI 增 `is_fds`；FDS 判据 = **`GameInfo->type == GIT_FDS`**（⚠️ 计划的 grepFDS 惯例名不成立）；agent 独立复核上游 2.7.0/master 快照逐行号证实 `:557`/`:577`/`:588`；**.cdl 输出行为变更**（非 FDS 写指令不再误抹 PRG 槽）已声明；文档落 `debug-regression-checklist.md` §七.7（含 #771 原话）；验收③端到端对比**登记不做**（需构建上游）。⑤ **V14 上半**：TraceLogger 环形缓冲 10 访问点全 `.load/.store`，上游 `44d3e45d`/`0a1ef89e` diff 逐段对照；读端取 **V13 形**（局部快照 + 出循环一次 release store）——与 master 消费端的逐次 store 不同，但该形同为上游所出（同批 commit、AviRecordDiskThread 先例），按任务口径取本仓最小结构；复位逻辑竞态（§二末⑥）保持现状为非目标；**无机器红腿**（TSAN 未落地）——证明 = 上游对照 + 静态审查。⑥ **V6**：wiki + Mesen2 双信源落地（bit2 电平直读、仅 $4016、仅 P2；1996 翻转 hack 退役）；新桥导出 `kagami_bridge_set_famicom_mic` + sut_input 保留值 0x40；锁测试 T1–T6（nestest + 真 strobe + ARead 直读）；kgmqa-108 修复前后读数逐字相同（0x02 稳定，只登记不判）；**manifest 行与基线翻转留待同级评审**；变异 M-1..M-5 两轮合并实测（轮 1 = M-1 如实归因为 M-1+M-4 合并施加，T1/T3/T4/T6 红；轮 2 = M-2..M-5 合并，T3×10/T4×2/T5×4/T6×4 红）。⑦ **工具链新事实（坑 10 变体，已录 AGENTS.md）**：FFI 签名变更后的首次构建可与 cbindgen 头重生成**竞态**（ninja 并行下 debug.cpp.obj 读到旧头 ⇒ C2660）⇒ 直接重跑即可，头已重生成。⑧ **管道退出码陷阱（自踩）**：`cmd //c … | tail` 会把构建失败码吃成 0 ⇒ 此后门禁一律「重定向到文件 + echo BUILD_EXIT」；首跑 ctest 曾在陈旧二进制上 37/37（无效），已重跑。⑨ **f11qa-rom-runner 实际路径** = `build\tests\`（非 src/rust/target）；kgmqa id 须全名。⑩ 人工项待做：V25/V22 真机 Lua 读值与鼠标、V6 真机吹气、V13/V15 压测、V11 跨进程读档。⑪ 未推送（push 需业主逐次明确指示）。 |
