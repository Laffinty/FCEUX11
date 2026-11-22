# FCEUX11 v2.0.8 — 许可证合规 + 崩溃韧性 + 观测能力 · 构建计划

> **STATUS: 在途**（2026-11-22 立项，r1）。
> 本文件是**构建计划**，不是盘点报告：回答「v2.0.8 做什么、怎么做、怎么验、什么不做」。
> 立项来源 = 第三方调研报告
> `docs/third_party_report/FCEUX11模拟器深度调研报告_基于v2.0.7_2026‑11‑22.md`
> （⚠️ 文件名连字符为 U+2011）的 P0/P0/P1 三条主张。
> **报告本身有事实偏差，一切以本文件 §一 实测事实底座与 §六 偏差附录为准，别照报告抄。**
>
> **坐标口径**：本文件所引行号以建档时的 `main`（`b7944c4`）为准；**过期以源码为准**。
> 收口后移入 `docs/history/plans/`，内容冻结不再续写。

---

## 〇、立项来源与业主裁决

### 0.1 来源

第三方报告给出三条与本版本直接相关的主张（原文见报告「P0/P1 问题清单」节）：

1. **P0 关键合规风险**：大量源码文件缺失许可证标注（报告称 332 个），建议补 GPL2 头。
2. **P0 错误/异常处理不足**：核心崩溃或输入异常时无回退策略，可能数据丢失。
3. **P1 观测能力缺失**：缺运行时日志、性能指标与错误跟踪（报告建议日志库 / Sentry）。

### 0.2 业主裁决（2026-11-22，AskUserQuestion 四项）

| # | 裁决项 | 结论 |
|---|---|---|
| D1 | 崩溃收集方案 | **本地 minidump**（SEH + `MiniDumpWriteDump`，纯离线不外发，零新依赖） |
| D2 | 诊断类菜单归属 | **新增 Diagnostics 二级菜单**（Advanced 下第七个） |
| D3 | Rust 侧许可证范围 | **只补 Cargo.toml 元数据**（.rs 不逐文件盖头） |
| D4 | 计划批准 | 同意，创建本计划文档 |

报告建议的其余项（Sentry 云端遥测、clang-tidy 静态分析、覆盖率工具、跨平台、
插件架构等）**均不纳入 v2.0.8**（§九）。

### 0.3 权威口径（联机信源，报告主张 1 的定性依据）

- **FSF GPL FAQ**（gnu.org/licenses/gpl-faq.en.html，「Why should I put a license
  notice in each source file?」）：按文件许可证声明是**强烈建议**（防止代码脱离
  许可上下文被单独复制），**不是 GPL 的法律强制条件**——GPL 对**整体作品**生效，
  本仓已有 `COPYING`（GPLv2 全文）+ `DERIVATIVE_WORK_NOTICE.txt` + `COPYRIGHT_AUDIT.md`。
  ⇒ **定性：合规卫生问题，不是燃眉法律风险**；但既然修就按 SPDX 口径一次修到位
  （spdx.dev/learn/handling-license-info：SPDX 标识符 + 保留完整许可声明）。
- **Sentry**：sentry-native 为 MIT（许可层面兼容 GPLv2），但云端遥测对离线模拟器
  是隐私与信任负担 ⇒ D1 裁决不接，本地 minidump 提供同等诊断力。若将来接入，
  必须 opt-in 且另立评审。

---

## 一、事实底座（2026-11-22 实测，全部逐条验证过）

### 1.1 许可证现状（主张 1 → 属实，数字过时）

- 全仓 C/C++（`.cpp/.h/.c`，排除 `src/rust/target`）共 **660** 个；
  含 `GNU General Public License` 文本的 **319**；**无该文本的 341**（报告快照
  2026-10-07 时为 332，其后新增项目文件继续裸奔）。
- 341 个的分布：`src/drivers` **166**（上游 mjbudd77 的 Qt 移植，多数无头）/
  `src/utils` 28 / `src/boards` 26 / `src/archived` 7 / `src/palettes` 6 /
  `src/input` 5 / `src/tests` 5 / `src/platform` 4 / 零散核心文件若干。
- 341 个里仅 33 个带某种版权行，**308 个完全裸**。
- 三桶分类（实施对象不同，**不可一刀切**）：
  - **桶 A · 本项目自建**（例：`src/apu.cpp` 来自 `23ab9f0` v1.6 脚手架，无头）
    ⇒ 补**项目头**（FCEUX11 Contributors，GPLv2+）。
  - **桶 B · 上游继承**（例：`src/boards/199.cpp` 有 FCE Ultra banner
    「Copyright (C) 2006 CaH4e3」但缺 GPL 正文；drivers 下多数连 banner 都没有）
    ⇒ 补**上游样式标准头**，**保留原有版权行**（GPLv2 §1 保全署名义务）。
  - **桶 C · 真第三方**（例：`src/boards/emu2413.c`（Okazaki YM2413 模拟）、
    `src/utils/unzip.*`（minizip，zlib 许可）、部分 `src/palettes/*`、
    `src/archived/fir/*`（Andrea Mazzoleni 系））
    ⇒ **不得盖 GPL 头**，逐族考证其自身许可并在 `COPYRIGHT_AUDIT.md` 登记；
    **考证不清的按纪律 4 停下登记，不盖章**。
- `COPYRIGHT_AUDIT.md` 的判红规则 = 文本含「GNU General Public License」⇒
  盖章后重扫即应转绿；该文档同步更新新扫描数与 C 族许可登记。
- Rust 侧：177 个 `.rs` 全部无头（D3 裁决本次不动）；`src/rust/Cargo.toml`
  与 `fceux11-*` 七个 crate **无 `license` 字段**；`gba-core` 已有
  `license = "MIT"`（`gba-core/Cargo.toml:24`，vendor 树，**保持 MIT 不动**）。

### 1.2 崩溃与 SRAM 现状（主张 2 → 部分属实，两处真缺口）

- **SRAM 只在关闭游戏时写盘**：`FCEU_SaveGameSave` 定义于 `src/cart.cpp:430`，
  调用点仅 `src/ines_gi.cpp:57`（`GI_CLOSE`）与 `src/unif.cpp:33`（`GI_CLOSE`）。
  ⇒ **进程崩溃 = 丢失自 ROM 装载以来的全部电池存档进度**。报告的「数据丢失」
  这一条是真实的，且是最便宜可修的一条。
- **全仓无任何崩溃处理**：`SetUnhandledExceptionFilter` / `MiniDumpWriteDump` /
  `AddVectoredExceptionHandler` grep 零命中 ⇒ NES 核心崩溃 = 进程静默死亡，
  无转储、无记录、用户无可上报材料。
- **已有设施**（报告不知道的，避免重复建设）：
  GBA 侧 `gba_step_frame` 的 `catch_unwind` 恢复 + 每帧错误码去重日志
  （v2.0.3 r6/r13）；即时存档备份/undo 记账（`FCEUSS_Save` 收敛点）；
  Qt 消息日志 dock。
- **「自愈/降级」的定性**：模拟器核心崩溃后机器状态已损坏，「自愈」不可行也不应
  做（引入半活状态比干净死更糟）。实际等价物 = **崩溃转储（可诊断）+
  SRAM 周期落盘（无数据丢失）+ 既有 GBA 恢复机制**。本条按此口径写进已知限制。

### 1.3 日志现状（主张 3 → 部分失实，真缺口是文件落盘的质量）

- 链路已存在：`FCEUD_Message` / `FCEUD_PrintError`（`src/driver_callbacks.cpp:35`）
  → Qt 回调（`src/drivers/Qt/fceu_callbacks.cpp:186-187`）
  → `msgLog_Message` / `msgLog_PrintError`（`src/drivers/Qt/MsgLogViewer.cpp:318`）
  → `msgLogBuf_t` 环形缓冲（256 行）+ 消息日志 dock。
- **`msgLogBuf_t` 其实已有文件 sink**（`MsgLogViewer.cpp:48-83`），但有三点缺陷：
  1. 写到 **`%TEMP%\fceux.log`**（`GetTempPathA`）——普通用户永远找不到；
  2. `fopen(..., "w+")`——**每次启动截断**，无历史、无轮转；
  3. 文件名是上游的 `fceux.log`，不是本项目的。
- **`addLine` 无锁**（`MsgLogViewer.cpp:99-117` 直接 `fseek/fprintf` 共享 `fp`），
  而 `FCEUD_Message` 可从模拟线程与 GUI 线程并发调用 ⇒ **既有潜在数据竞争**
  （上游继承缺陷），本轮修 sink 时一并上锁。
- 无分级、无指标、无崩溃收集（后者归批 2）。

### 1.4 菜单现状（D2 的落点）

- `src/drivers/Qt/ConsoleMenu.cpp:137-146`：`&Advanced` 下现有六个二级菜单
  （Emulation / Movie / Debug / Memory Tools / Misc Tools / Advanced Settings）。
- 目录化结构在 `src/drivers/Qt/MenuCatalog.h`：`MenuRoot` / `AdvancedSub` 枚举 +
  `advancedSubTitle()`（`MenuCatalog.h:62-73`）+ `MenuPathId`（菜单路径稳定标识，
  CI 守卫用）⇒ **新增二级菜单是结构化动作**：枚举 +1、title 函数 +1、
  ConsoleMenu 构建点 +1、`MenuPathId` 登记 +1、翻译 key +N。
- 12 语言翻译：新增 key 未翻译的语言自动回落英文源文；本次补 zh-CN（业主语言）
  与 en（源文），其余语言按现仓库惯例后续补。

### 1.5 实施期需钉死的坐标（本节只列不猜）

- base 目录推导（logs/ 与 crashes/ 落点）：复用 `.sav` / `.snaps` 同层的既有
  目录推导设施，实施时在 `src/file.cpp` / `FCEU_MakeFName` 现有类型上择路。
- `fceux11_rust_cart_battery_save` 的写文件实现（formats crate）——原子写改造点。
- 音频欠载计数的既有计数器（A1 线的 `audio_probe_gate` 体系内有无现成读数）。

---

## 二、批 1 · 许可证头（零行为变更）

### L1 分类 pass（先分类、业主抽检、再盖章）

- **目标**：341 个文件逐一归类 A/B/C，产出**分类 manifest**（JSON：文件、桶、依据）。
- **步骤**：
  1. `scripts/license_headers/classify.py`：规则初分（含 `Copyright (C) 2006 CaH4e3`
     等上游署名 → 桶 B 候选；git 首次入库即本项目提交 → 桶 A 候选）。
  2. **第三方族逐一 RE**（纪律 4：不凭记忆写许可结论）：`emu2413` 族 / `unzip`
     (minizip) 族 / `archived/fir` 族 / `palettes` 族 / 其余发现族——每族查源
     文件内文与上游出处，许可不清白的进 C 桶白名单并登记「考证未完成」。
  3. manifest 交业主抽检后再进入 L2。
- **验收**：manifest 三桶数量之和 = 341；桶 C 每族有一句许可依据或「未考证」登记。
- **反向守卫**：`check.py`（L3）把桶 C 白名单当例外集，白名单外的裸文件即红。

### L2 盖章脚本

- **目标**：桶 A/B 文件获得许可头；**行尾与 BOM 分毫不动**（AGENTS 坑 11）。
- **模板**：
  - （r2 修订：取消 SPDX 行——全仓 660 文件零 SPDX 先例；A/B 统一用仓库既有
    「FCE Ultra」banner 形状，差异只在版权段。）桶 A 版权段 = `Copyright (C)
    2026 FCEUX11 Contributors`；桶 B = **原文件既有版权行原样保留**（现文缺失时
    从 fork 基线 `git show 1c35642:<path>` 回溯提取；两处皆无则省略版权段，零编造）。
- **实现约束（r2 修订）**：**字节级前置插入**——读字节、原内容逐字节不动、
  EOL 取首行行尾、检测 UTF-8 BOM 并保留（插在 BOM 之后）；幂等（首 2 KB 已含
  GPL 文本即跳过）；既有前导 banner 含版权行的文件做**整块替换**（防标题/版权行
  双重堆叠），形状不识别则退化为纯 GPL 块前插；
  **`src/rust/crates/gba-core/**` 绝对不碰**（vendor 树，ATTRIBUTION 红线）。
- **验收**：全量构建 EXIT=0；`git diff --numstat` 抽查 ≥20 个文件——
  增删行数 = 头行数，无整文件重写特征；重扫「无 GPL 文本」计数 = 白名单大小。
- **风险**：桶 C 误盖 GPL 头 ⇒ manifest 业主抽检 + 逐族依据；行尾污染 ⇒
  脚本读写口径 + numstat 判据（坑 11 的 r10 变体）；**`src/archived/**`
  一并盖桶 B 头**（仍在发布树内）。

### L3 `--check` 进 CI（防回归）

- `scripts/license_headers/check.py`：扫 `src/` C/C++，凡不含 GPL 文本且不在
  桶 C 白名单者，打印清单并**非零退出**。
- 接入 `.github/workflows/ci.yml` 新 step（Python 零依赖，runner 自带）。
- **验收**：本地跑 EXIT=0；人为删一个头 → 红并点名文件 → 还原 → 绿。

### L4 COPYRIGHT_AUDIT.md 更新

- 新扫描日期 2026-11-22、新总数/分布、盖章记录（指向本计划 §二）、
  桶 C 逐族许可登记表。

### L5 Rust 元数据（D3）

- `src/rust/Cargo.toml`（workspace）+ `fceux11-{core,debug,formats,lua,media,utils}`
  + `f11qa` 七处补 `license = "GPL-2.0-or-later"`（无 `[package]` 的 crate 用
  各自现有结构）。
- **`gba-core` 保持 `MIT` 不动**（vendor 树署上游，`ATTRIBUTION.md` 管辖）。
- **验收**：`cargo check --workspace` 两态 EXIT=0；各 crate 测试计数不变
  （纯元数据，不触代码）。

---

## 三、批 2 · 崩溃韧性

### C1 SRAM 周期落盘（数据丢失缺口的根治）

- **目标**：电池存档进度最多滞后 N 秒，崩溃不再清零。
- **现状坐标**：§1.2（`cart.cpp:430` 仅 GI_CLOSE 调用）。
- **步骤**：
  1. 新增 `void FCEU_FlushGameSave(void)`（`src/cart.cpp`）：按 `GameInfo->type`
     分派到 `FCEU_SaveGameSave(&iNESCart)` / UNIF 同款；`GameInfo == NULL`
     （含 GBA 会话）直接返回 ⇒ **GBA 会话天然短路，解耦不破**。
     （备选：新增 `GI_SAVESRAM` 动作走既有 `FCEU_GI` 分派——实施期二选一，
     倾向前者，少一层枚举。）
  2. 原子写：`fceux11_rust_cart_battery_save`（formats crate）改为
     **tmp 文件 + rename**，写一半崩溃不再留下截断 `.sav`；单点改造，
     关闭路径与周期路径同时受益。
  3. GUI 周期触发：`ConsoleWindow` 既有周期回调处挂秒级计时器，间隔读配置
     `SDL.SRAMFlushInterval`（**默认 30，0 = 关**），到点且模拟运行中才调。
     头less 路径（ctest runner）不经过 GUI ⇒ 测试零影响。
- **验收**：新 ctest——装载带电池 ROM → 调 `FCEU_FlushGameSave()` → 断言 `.sav`
  生成且字节非空 → 再调一次断言内容更新；formats crate 新锁——写中途注入失败
  ⇒ `.sav` 原文件不损（tmp+rename 语义，变异腿：还原直写实现即红）。
- **反向守卫**：关 `SDL.SRAMFlushInterval=0` 时行为与改前逐字相同（无非必要写盘）。
- **行为变更登记**：游戏运行时 `.sav` 的 mtime 会周期更新（CHANGELOG Known
  limitations/行为说明各一句）。

### C2 崩溃转储（诊断缺口的根治，D1 裁决）

- **目标**：任何未处理异常留下本地可分析的 `.dmp` + 一行摘要；不外发、零新依赖。
- **步骤**：
  1. 新文件 `src/crash_handler.{h,cpp}`（属 core，不依赖 Qt——纪律 8）：
     `SetUnhandledExceptionFilter` + CRT 补丁族（`_set_invalid_parameter_handler` /
     `_set_purecall_handler` / `std::set_terminate` / `signal(SIGABRT)`）统一汇入
     一个写入口；`MiniDumpWriteDump`（链 `dbghelp`）写 `MiniDumpNormal` 级转储到
     `crashes/fceux11_<时间戳>_<pid>.dmp`；`crashes/crash_log.txt` 追加
     时间 / 版本号 / 异常码 / 异常地址 / 当前 ROM 文件名（各读取均 try-guard，
     filter 内**零 UI、最小分配**）；`EXCEPTION_CONTINUE_SEARCH` 交还 WER。
  2. 安装点：`main()` 最早处（`src/drivers/Qt/main.cpp`），先于 Qt 构造。
  3. 探针：`FCEUX11_TEST_CRASH=1` 时在启动后注入一次受控崩溃
     （`#ifdef` 不设限 release 也可用，供人工验收）⇒ 产出 dmp 即验收证据。
  4. 一次性标志防 filter 重入递归。
- **验收**：新 ctest `crash_dump_test`——直调转储写函数（fabricated
  `EXCEPTION_POINTERS`），断言 dmp 存在且 > 0 字节、摘要行落盘；
  人工探针实跑一例。
- **风险**：LTCG 链接病态（坑 13）——新 TU 若中招按先例加 `/GL-`；
  dbghelp 在含 Qt 的进程内正常（微软自带组件）；转储只落本地 ⇒ 与
  `Research_only/` 同口径不进 git（`.gitignore` 加 `crashes/`）。

### C3 GBA `.srm` 周期落盘（独立小项）

- **目标**：GBA 电池存档同享 C1 的保护；**与 C1 零共享代码**（不变式 3，
  不把 NES 泛化物接进 GBA）。
- **步骤**：`src/gba_load.cpp` 自身的会话状态处挂同款周期触发，调其既有
  `.srm` 写出函数；间隔复用同一配置项读值（**读配置不算泛化**——四个允许
  耦合外的共用仅此一个 ini 键，如守卫扫红则各自读键、注释说明）。
- **验收**：GBA 锁测试计数不减；decoupling 三守卫全绿；ctest 全绿。
- ⚠️ 新 C/C++ 文件与改动文件**不得出现 "GBA" 字串**（解耦守卫扫描面），
  GBA 相关改动只落在 `gba_load.cpp` 既有白名单文件内。

---

## 四、批 3 · 观测能力

### O1 日志 sink 升级

- **目标**：日志落到用户找得到、能留存的地方；顺手修掉既有无锁竞争（§1.3）。
- **步骤**（改造 `msgLogBuf_t`，不新建体系）：
  1. 落点迁到 base 目录 `logs/fceux11_log.txt`（§1.5 钉坐标），带轮转
     （单文件超 5 MB → `fceux11_log.1.txt` … 保留 3 代）。
  2. `addLine` 加互斥锁（修竞争）；错误级消息（`msgLog_PrintError`）
     即时 `fflush`，普通消息走缓冲。
  3. 配置 `SDL.LogToFile`（默认 **1**；纯本地文件，无外发，磁盘失败时
     **静默降级**：关 sink + dock 提示一次，不影响运行）。
- **验收**：ctest 全绿；手工验收——日志出现于期望路径、跨两次启动追加不截断、
  开关生效；竞争面以代码评审 + 锁为守卫（构造并发写探针不设 CI）。
- **风险**：锁内不得做 GUI/IO 以外的事（死锁面）；路径推导失败回落 Temp
  并提示。

### O2 诊断对话框

- **目标**：一屏看清「版本/环境/运行状态/留痕位置」，支撑用户报障与自检。
- **内容**：版本与构建信息、exe/config/base 路径、当前 ROM（名 / mapper /
  CRC）、帧时统计（既有 FPS 口径 + 滑动平均）、音频欠载计数（§1.5 钉坐标，
  无现成计数器则最小埋点）、`crashes/` 转储计数与最新一份、日志路径；
  按钮：打开日志目录 / 打开崩溃目录 / 复制报告到剪贴板。
- **落点**：驱动层（Qt），只读既有状态，不反哺 core。
- **验收**：手工验收清单；无游戏时（GBA 会话/NES 会话/空会话三态）可开不崩。

### O3 Diagnostics 二级菜单（D2 裁决）

- **步骤**：`MenuCatalog.h` `AdvancedSub` 增 `Diagnostics`（`COUNT` 6→7）+
  `advancedSubTitle` + `MenuPathId` 登记；`ConsoleMenu.cpp` 构建
  `advDiagMenu`（置于 Misc Tools 之后）；条目：诊断信息…（O2）/ 打开日志目录 /
  打开崩溃目录 / 文件日志开关（勾选 → `SDL.LogToFile`）。
- **翻译**：新增 key 的 zh-CN 与 en 全量补齐；其余 10 语言回落英文，
  按现仓库惯例后续补译（不阻塞收口）。
- **验收**：`MenuPathId` CI 守卫绿；四条入口手工逐一点验；
  `ctest -LE perf` 全绿。

---

## 五、门禁与验收总表

| 门禁 | 批 1 | 批 2 | 批 3 |
|---|---|---|---|
| `ctest -LE perf`（基线 49，以 `ctest -N -LE perf` 为准） | ✔（应 49 不变） | ✔ + C1/C2 新锁 | ✔ |
| 全量 Release 构建 EXIT=0 | ✔ | ✔ | ✔ |
| `cargo check --workspace` 两态 | ✔ | ✔（formats 改动） | ✔ |
| `cargo test -p fceux11-formats --lib`（基线 156） | 不变 | + 原子写锁 | 不变 |
| GBA 根 crate / vendor 计数 | 不变 | C3 后不减 | 不变 |
| `check.py`（CI 新 step） | ✔ | ✔ | ✔ |
| decoupling 三守卫 | 不适用 | ✔（C3） | ✔（O1/O2/O3） |
| 人工验收 | manifest 抽检 | 探针崩溃出 dmp | 菜单四入口 + 三态开对话框 |

- **NES 零回归（不变式 1）加倍适用**：C1 触碰 `cart.cpp`/GI 路径，O1 触碰
  消息回调链——两处改动前后各跑一次全绿。
- 跑 ctest 不与构建/测量并行（r54 纪律）。

---

## 六、第三方报告偏差附录（**别照报告抄**）

| # | 报告表述 | 实测事实（§一 坐标） |
|---|---|---|
| 1 | 「332 个文件未声明许可证」 | 快照过时：2026-11-22 实测 **341**，且含必须区别对待的第三方桶 C |
| 2 | 「几乎无内置日志……未见集成日志库」 | 消息日志 dock + stdout/stderr + `%TEMP%\fceux.log` 文件 sink **均已存在**（`MsgLogViewer.cpp:48`）；真缺口是落盘质量/分级/指标 |
| 3 | 「未实现健壮的错误处理」「无日志记录」 | GBA `catch_unwind` 恢复、错误码去重日志、存档备份/undo 均已存在；真缺口 = SEH/转储/SRAM 周期落盘两条 |
| 4 | 「商业软件要求核心模块具备自愈或降级」 | 本项目非商业定位；且模拟器核心「自愈」语义不成立，落地物为转储 + 落盘（§1.2 定性） |
| 5 | 「建议 Sentry / spdlog」 | D1 裁决：本地 minidump，不引第三方库；spdlog 不引入（现有链路改造即可） |
| 6 | 版本线与发布表述（「1.15 热修版」「SourceForge 发布时间线」「v2.0.7 已发布」） | 与 `AGENTS.md`/CHANGELOG 口径不符——1.15 是本项目自有版本线，v2.0.7 发布状态以业主 CHANGELOG/发版流水线为准 |
| 7 | 「318 文件 GPLv2+」 | 含 GPL 文本 319（口径同 COPYRIGHT_AUDIT 规则），差 1 为其后新增 |

> 报告其余章节（评分表、路线图、竞品分析）不构成本计划的依据；其中可取的
> 工程项（静态分析、覆盖率、CI 矩阵）列 §九 长尾，业主按需另立。

---

## 七、不变式与红线（本版本特别适用项）

1. **NES 零回归**（不变式 1）：`ctest -LE perf` 每批必跑。
2. **GBA 解耦**：新增 C/C++ 文件不得含 "GBA" 字串（decoupling 守卫会变红）；
   GBA 改动仅落 `src/gba_load.cpp`（既有白名单）与 C3 自己的触发点；
   红线清单（帧池/纹理/调色板/调试器映射/cheats/TAS/NSF/录像）零触碰。
3. **vendor 树不动**：`src/rust/crates/gba-core/**` 本版本零改动（批 1 不盖头、
   批 2/3 不接线）；`ATTRIBUTION.md` 补丁集计数不变。
4. **分层**（纪律 8）：`crash_handler` 属 core；诊断对话框与菜单属驱动层。
5. **行尾/BOM**（坑 11 及 r10 变体）：批 1 脚本读写口径 + numstat 判据，先验证
   10 文件小样再全量。
6. **不凭记忆写许可结论**（纪律 4）：桶 C 每族给出处，考证不清白就不盖章。

---

## 八、阶段编排

| 阶段 | 内容 | 依赖 |
|---|---|---|
| P1 | 批 1 全部（L1→L5） | 无；零行为变更，先行落地 |
| P2 | 批 2（C1 → C2 → C3） | 无；C3 可与 C1/C2 并行 |
| P3 | 批 3（O1 → O3 → O2） | O3 的「打开崩溃目录」依赖 C2 的目录约定 |

每阶段开工前先在本文件记 r（纪律 2）；每阶段收口跑 §五 全表。
版本收口（v2.0.8 号段、CHANGELOG、归档）照 v2.0.5/v2.0.7 先例，
**push / tag / 发版一律等业主明确指示**。

---

## 九、明确不做（本版本范围外，长尾按需另立）

- Sentry / 任何云端遥测（D1 裁决；将来接入须 opt-in + 评审）。
- `.rs` 逐文件许可头（D3 裁决，仅做 Cargo.toml 元数据）。
- clang-tidy / CppCheck / PVS-Studio 静态分析、Dependabot/SCA、覆盖率报告
  （报告「工程改进清单」节——与本项目门禁哲学不同，须业主逐项裁决后才立项）。
- 日志分级体系（info/warn/error 全链路重构）与性能指标面板（超出 O2 最小集）。
- 跨平台、插件架构、GBA 精度长尾（各自既有轨道）。
- 「核心自愈/降级」——语义不成立（§1.2），以转储 + 落盘为实际等价物，永久登记。

---

## 十四、变更记录

- **r1（2026-11-22）立项**：第三方报告三条主张逐条核验（§一 实测 + §六 偏差
  附录）；业主四项裁决落档（D1 本地 minidump / D2 新增 Diagnostics 二级菜单 /
  D3 Rust 只补元数据 / D4 批准计划）；本计划创建于 `docs/plans/`。
- **r2（2026-11-22）批 1 实施口径细化（开工前定稿）**：① 取消 SPDX 行——全仓
  660 文件零 SPDX 先例，统一采用既有「FCE Ultra」banner 形状（`206.cpp` /
  `core_api.h` / `471.cpp` 同款）；② `src/archived/fir/*` 7 文件改归桶 B——内容
  在 fork 基线老路径 `src/fir/`（路径成员规则按内容来源修正）；③ 盖章机制改
  **字节级前置插入**（原内容逐字节不动、EOL 取首行行尾、保 BOM）——机制性根绝
  坑 11 行尾污染；既有前导 banner 含版权行 → 整块替换防堆叠；④ 桶 C 白名单定格
  **7 条**：minizip 4（`ioapi.{cpp,h}` / `unzip.{cpp,h}`，zlib 条款文内自声明）+
  emu2413 2（文内无许可声明，纪律 4 不猜）+ 生成文件 `src/rust/fceux11_rust.h`
  （cbindgen 重生成会覆盖）⇒ 盖章 334；⑤ 量化：A ≈128 / B ≈207 / C 7（最终以
  `classify.py` manifest 为准）；⑥ 业主批准批 1 开工并允许多 agent 并行（Rust L5
  与 emu2413 考证外派并行，盖章与构建主线串行）。
- **r3（2026-11-22）批 1 全落地（待业主提交/推送指令）**：
  - **盖章 283 文件**（A=152 项目头 / B=131 上游头保版权行 / C=58 白名单不盖）；
    **字节级前置插入**实证：保真审计 **283/283**（工作树规范化内容 = HEAD blob +
    头，零内容丢失、零行尾混合）；BOM 4 例首行改写为「BOM+头」属正确搬家；
    stamp 幂等（二次运行 0 改动）。
  - **白名单自审的关键捕获**：按报告口径「直接补 GPL 头」会**错误标注 50 个
    第三方文件**——自审抓出四族自带许可：LGPL-2.1（MaxSt hq2x/hq3x + blargg
    nes_ntsc 共 8）· MIT（DeSmuME emufile.h + AnS TAS Editor 36）· BSD-3
    （gwavi/avi 4）· Unicode Inc（ConvertUTF.h）；伴生无文本头按族一致白名单。
    白名单终 **58 条**（第三方 57 + 生成 1）。
  - **emu2413 考证收口**（外派 agent 联机取证）：文内从无许可声明、许可随发行包
    readme 走——2001 一句式自由使用声明（Mednafen 文档逐字背书）→ 2004 zlib 式
    （vst2413 保存原包）→ 作者现仓 MIT（2001-2019）；全部宽松、方向一致、可安全
    再分发；残余「2002 v0.60 当年 readme 原件未找到」按部分未决登记。全文与信源
    见 COPYRIGHT_AUDIT.md §6.5。
  - **L5（外派 agent）**：`license = "GPL-2.0-or-later"` 入 `[workspace.package]`
    + root 包与 7 crate 的 `license.workspace = true`（gba-core 保持 MIT 未动）；
    numstat +9/0 无行尾污染；两态 `cargo check` EXIT=0。
  - **check.py**：660 扫描 / 58 白名单 / EXIT=0；四条变异腿闭合（无头→红、带头→绿、
    白名单键不匹配→红、白名单命中→绿）+ 幂等腿；跨盘 `--root` relpath 缺陷已在
    测试中暴露并修复；已入 `ci.yml`（Checkout 后第一步，vcpkg 之前）。
  - **门禁终读数**：全量 Release 构建 **BUILD_EXIT=0**（710 步，`--parallel 1`）；
    `ctest -LE perf` **49/49 全绿**（CTEST_EXIT=0）；Rust 八条全绿：GBA 根
    **303+20i** / vendor 249 / debug 94 / media 52 / formats **158** / utils 51 /
    lua 54 / f11qa **184**（formats 158 与 f11qa 184 为 v2.0.7 后新基线读数，
    本批 Rust 源码零改动——仅 8 个 Cargo.toml +9 行）。
  - **坑补录**：`cmd /c` 交互陷阱再次实证——Git Bash 下必须 `cmd //c`，否则 cmd
    打印 banner 即退出（首次构建假绿被日志抽查识破，空输出≠通过）。
  - 产出物：`scripts/license_headers/{classify,stamp,check}.py` +
    `third_party_whitelist.json` + `manifest.json`（manifest 为生成物，不入库与
    否随业主）；COPYRIGHT_AUDIT.md §6；AGENTS.md 门禁节补录 + 治理表行更新。
- **r4（2026-11-22）批 2 独立建档 + C3 前提作废**：业主指示批 2 相对复杂、独立
  立项——专项文档 = `docs/plans/FCEUX11-v2.0.8-批2-崩溃韧性-构建计划.md`（r1）。
  **§三 C3 的「另行实现」前提被调查作废**：`fceu11_gba_flush_battery`
  （`gba_load.cpp:241`）已实现脏标志 + 限频 + 每帧调用 + 关闭强刷 ⇒ C3 转验证项
  （零新代码）。C1 设计定型为五步链路（GI_SAVESRAM 枚举 → iNESGI/UNIFGI case →
  `FCEU_FlushGameSave` → 驱动层 tick（steady_clock + `SDL.SRAMFlushInterval`
  默认 30/0=关）→ cart.rs 原子写 tmp+rename）；C2 = `crash_handler.{h,cpp}`
  （SEH + CRT 补丁族 + `MiniDumpWriteDump` + `FCEUMKF_CRASH` → `crashes/` +
  探针 `FCEUX11_TEST_CRASH=1`）；变异腿 M1–M7、门禁 ctest 49→51 / formats
  158→160、阶段编排 P1–P5、风险与止损线均以专项文档为准。
- **r5（2026-11-22）最新构建情况（批 2 P1/P2 已落地并推送）**：
  - **批 2 P1 已推送 `e0ff625`**（8 文件 +182/−1）：`GI_SAVESRAM=5` 分发 +
    `FCEU_FlushGameSave` 入口 + `sram_flush_test`（六腿 PASS，M4/M5 变异腿双闭合），
    `ctest -LE perf` **49 → 50**。
  - **批 2 P2 已推送 `c5b2a8a`**（2 文件 +110/−7）：cart.rs `atomic_write_file`
    （tmp+rename 原子写，根治 `File::create` 直写截断风险），formats **158 → 160**
    （M1/M2 锁 + M1 变异腿可红）。
  - **最新门禁基线（P2 收口实测）**：全量 Release（`--parallel 1`）BUILD_EXIT=0；
    `ctest -LE perf` **50/50** 全绿；formats **160/0**；其余 Rust crate 计数不变
    （批 1 读数：GBA 根 303+20i / vendor 249 / debug 94 / media 52 / utils 51 /
    lua 54 / f11qa 184）；`check.py` EXIT=0。
  - **进度总览**：批 1 ✅（29560c7）；批 2 = P1 ✅ / P2 ✅ / **P3 ⬜（tick +
    `SDL.SRAMFlushInterval`）/ P4 ⬜（crash_handler）/ P5 ⬜（收口）**。
    CI 读数未人工复核（本机无 gh CLI，Actions 页可看：e0ff625 起首次跑
    `sram_flush_test` + License header check 第二次）。逐阶段细节见批 2 专项文档
    r1–r6。
