# FCEUX11 v2.0.8 批 3 — 观测能力（日志落盘 + 诊断对话框 + Diagnostics 菜单）· 构建计划

> **STATUS: 在途**（2026-11-23 立项，r1）。本文件是主计划 §四 的**专项细化文档**
> （业主指示：参照批 2 的开发模式，为批 3 立独立文档）。主计划 =
> `docs/plans/FCEUX11-v2.0.8_许可证合规与崩溃韧性-构建计划.md`（r1–r6）。
> 业主裁决 **D2 = Advanced 下新增 Diagnostics 二级菜单**（§0.2 记于主计划）。
>
> **坐标口径**：以 `main` @ `81ecfcd`（批 2 收口 + 专项进度标注修正）实测为准；
> **过期以源码为准**。本文件的行号均为 2026-11-23 实测值。

---

## 〇、定位与范围

| 项 | 内容 | 现状 |
|---|---|---|
| **O1** | 日志 sink 升级：落点迁移 + 轮转 + **修并发写竞争** + 开关 | 有 sink，但路径不可寻、每次启动截断、无锁 |
| **O2** | 诊断对话框：一屏看清版本/路径/ROM/帧计数/音频/崩溃留痕 | 不存在 |
| **O3** | Advanced 下新增 **Diagnostics 二级菜单**（D2 裁决） | 不存在（现有六个二级菜单） |

**明确排除**：云端遥测 / Sentry（D1 裁决，批 2 已定）、日志分级体系的全链路重构
（§九）、性能指标平台（§九）、CI 覆盖率工具（§九）。

---

## 一、事实底座（2026-11-23 逐条实测）

### 1.1 日志 sink 现状（O1 的对象）

链路：`FCEUD_Message` / `FCEUD_PrintError`（`src/driver_callbacks.cpp:35`）
→ Qt 回调注册（`src/drivers/Qt/fceu_callbacks.cpp:202-203`）
→ `msgLog_Message`（`src/drivers/Qt/MsgLogViewer.cpp:318`）/ `msgLog_PrintError`
（`:334`）→ `msgLogBuf_t`（`:48`）+ 全局实例 `msgLog`（`:188`）。

`msgLogBuf_t` 的四个实测缺陷：

| # | 缺陷 | 坐标 | 后果 |
|---|---|---|---|
| 1 | **路径不可寻**：写 `%TEMP%\fceux.log`（`GetTempPathA`，`:56`） | `:56-69` | 用户永远找不到；且**不是本项目的名字** |
| 2 | **每次启动截断**：`fopen(filename, "w+")` | `:69` | 无历史、无轮转 ⇒ 上次崩溃前的日志全丢 |
| 3 | **无锁**：`addLine` 直接 `fseek/ftell/fprintf` 共享 `fp` | `:99-117` | **并发写竞争**：`FCEUD_Message` 来自模拟线程与 GUI 线程（实测调用点：`src/cheat.cpp:296`、`src/fds.cpp:111-126` 等 core 侧 + 驱动侧），两线程并发 ⇒ `FILE*` 内部状态撕裂/丢行 |
| 4 | **环形缓冲与文件脱节**：`fpOfsList[256]` 只存偏移，文件本身无限增长 | `:46`（`MSG_LOG_MAX_LINES 256`）+ `:117-137` | 文件可涨到任意大小；viewer 只显示最后 256 行的区间 |

**打开时机（O1 的关键约束）**：全局 `msgLog` 是**静态初始化对象** ⇒ 它的 `FILE*`
在 `main()` 之前就打开了；此时 **base 目录尚未初始化**（`InitConfig()` 在
`fceuWrapperInit` 内，`src/drivers/Qt/fceuWrapper.cpp:763` ⇒
`fceu11::SetBaseDirectory`，`src/drivers/Qt/config.cpp:509`）。
⇒ **要让日志落在 base 目录，必须「延迟打开 + 在 config 初始化后重定向」两段**。

### 1.2 诊断对话框可取的数据面（O2 的输入）

**好消息：计数器全都已存在，本项不需要新增埋点。**

| 数据 | 来源 | 坐标 |
|---|---|---|
| 版本号 | `FCEU_DISPLAY_VERSION` | `src/version.h:155` |
| 编译器/构建串 | `FCEUD_GetCompilerString()` | `src/diag_api.h` |
| base 目录 | `fceu11::GetBaseDirectory()` | `src/io_api.h:138` |
| 当前 ROM | `GameInfo->name` / `->filename` / `->MD5` | `src/git.h:243`（`FCEUGI`） |
| 渲染/呈现帧计数 | `nes_shm->render_count` / `blit_count`（原子） | `src/drivers/common/nes_shm.h:135-136` |
| **音频欠载** | `nes_shm->sndBuf.starveCounter`（原子） | `nes_shm.h:193`；现成 UI 读它：`src/drivers/Qt/ConsoleSoundConf.cpp:386` |
| 标称帧率 | `getBaseFrameRate()` | core API（`AviRecord.cpp:437` 等在用） |
| 崩溃留痕 | `<base>/crashes/` 下的 `.dmp` 数量 + 最新一份 | 批 2 C2 已落（`src/crash_handler.cpp`） |

**对话框范式**：`MsgLogViewDialog_t`（`src/drivers/Qt/MsgLogViewer.h:37`，
`QDialog` + `Q_OBJECT` + `changeEvent(LanguageChange)` 重译）——**新对话框照它抄**。
**打开目录的现成设施**：`QDesktopServices::openUrl(QUrl::fromLocalFile(...))`
（先例：`src/drivers/Qt/AboutWindow.cpp:123`）。

### 1.3 菜单与 i18n 现状（O3 的约束）

| 事实 | 坐标 | 对 O3 的约束 |
|---|---|---|
| Advanced 下现有**六个**二级菜单（Emulation / Movie / Debug / Memory Tools / Misc Tools / Advanced Settings） | `src/drivers/Qt/ConsoleMenu.cpp:137-146` | 新增第七个 `Diagnostics` |
| `AdvancedSub` 枚举（`COUNT = 6`）+ `advancedSubTitle()` + `MenuPathId` | `src/drivers/Qt/MenuCatalog.h:53-60` / `:77-87` / `:88-104` | 三处各 +1 |
| **`MenuPathId` 目前零消费者**（全仓 grep 无引用） | — | 无需登记任何清单；但它是「为将来 CI 守卫预留」的稳定标识 ⇒ 新子菜单仍要给它一个合法值 |
| 菜单 slot 必须声明在 `slots:` 段 | `scripts/check_menu_slots.py`（CI 门禁：SIGNAL/SLOT 目标未声明 ⇒ 静默失效 ⇒ 脚本报红） | 新 slot 落 `ConsoleWindow.h` 的 `public slots:` 段（先例 `:424` `openMsgLogWin`） |
| **i18n 源串冻结**：新增 `tr()` 需走 `[i18n-string-add]` 热修流程（lupdate 重扫 → 重新翻译 → 重跑覆盖率门禁） | `src/drivers/Qt/lang/README.md`「Source string freeze」；`MenuCatalog.h` 头部「Iron rule」 | O3 会新增约 **5–9 个源串** ⇒ **必须走完整流程**，否则覆盖率门禁掉线 |
| 覆盖率门禁：**zh_CN / zh_TW ≥ 95%，其余 10 种 ≥ 90%** | `scripts/i18n_coverage.ps1` | 12 个 `.ts` 都要补新串，否则门禁红 |
| 简繁分离检查：zh_CN 禁繁体、zh_TW 禁简体 | `scripts/check_simp_trad.ps1` | 新中文串要用词表里的规范译法（`glossary.txt`） |
| 术语硬规则：`TAS / ROM / NES / mapper / CPU / PPU / APU / FPS / RAM / PRG / CHR` **不译**；`&` 助记符保留 | `lang/README.md` | 新串命名须遵守 |

---

## 二、O1 · 日志 sink 升级

### 2.1 设计

1. **路径**：`<base>/logs/fceux11_log.txt`（base 未就绪时回落 `%TEMP%`）。
   命名去掉上游的 `fceux.log` ⇒ 一眼看出是本项目产物。
2. **延迟打开 + 重定向**（解 §1.1 的时机约束）：
   `msgLogBuf_t` 的构造改为**不打开文件**；首次 `addLine` 时按当前已知路径打开；
   `InitConfig()` 之后（`fceuWrapper.cpp:763` 之后）调用一次
   `msgLog_reopen_to_base_dir()`——**已有内容先落旧文件再切换**（不丢启动早期日志）。
3. **加锁**：`addLine`/`loadTextViewer`/`clear` 全部进同一把互斥量
   （`std::mutex` + `std::lock_guard`；**不用 `FCEU_WRAPPER_LOCK`**——那是
   wrapper 锁，会与 GUI/模拟线程互锁 ⇒ 死锁风险）。
4. **轮转**：单文件超 **5 MB** ⇒ 轮转 `fceux11_log.1.txt`（保留 3 代）。
   轮转在锁内完成，**先关句柄再改名**（Windows 上打开的文件不能改名）。
5. **级别与刷新**：错误级（`msgLog_PrintError`）即时 `fflush`；普通消息走缓冲。
   **不引入日志分级体系**（§九 明确不做）——只加「错误级即时落盘」这一条。
6. **开关**：配置 `SDL.LogToFile`（默认 **1**）；关掉时只留 dock 不写文件
   （写文件失败时**静默降级**：关 sink + dock 提示一次，**绝不影响模拟**）。

### 2.2 验收与变异腿

| # | 锁 | 断言 | 变异腿 |
|---|---|---|---|
| O1a | ctest `log_sink_test`（**新建**，纯标准库可测的部分） | 落点路径推导：base 已知 ⇒ `<base>/logs/...`；base 未知 ⇒ `%TEMP%` 回落 | 改回落分支 ⇒ 红 |
| O1b | 同 | **并发写不撕裂**：N 线程各写 M 行 ⇒ 行数与内容完整（断言捕获内容非空且条数正确，纪律 13） | 去掉锁 ⇒ 红（竞争可见） |
| O1c | 同 | 轮转：写超阈值 ⇒ 生成 `.1.txt` 且当前文件变小、旧代被清 | 去掉轮转 ⇒ 红 |
| O1d | 同 | 开关关闭 ⇒ 不产生文件 | — |
| 人工 | GUI | 跑一次游戏 ⇒ `logs/` 下有文件；重启 ⇒ **不截断**（追加） | — |

> **可测性设计**：把「路径推导 / 加锁 / 轮转 / 开关」抽成
> `src/drivers/Qt/log_sink.{h,cpp}` 的**无 Qt 依赖**部分（路径与文件操作用标准库），
> ctest 直接驱动它；`msgLogBuf_t` 只做「把文本交给它」的胶水。

---

## 三、O2 · 诊断对话框

### 3.1 设计

- **新文件** `src/drivers/Qt/DiagnosticsDialog.{h,cpp}`（`QDialog` + `Q_OBJECT`，
  照 `MsgLogViewDialog_t` 的形制，含 `changeEvent(LanguageChange)` 重译）。
- **内容**（只读既有状态，**不反哺 core**——纪律 8）：
  版本/构建串 · base 目录 · 当前 ROM（名 + mapper + CRC32/MD5）· 帧计数
  （render/blit）· 标称帧率 · **音频欠载计数**（`starveCounter`）· 崩溃留痕
  （`crashes/` 下 `.dmp` 数量 + 最新一份名）· 日志文件路径与大小。
- **按钮**：复制报告到剪贴板 · 打开日志目录 · 打开崩溃目录（`QDesktopServices`
  先例）。目录不存在时按钮置灰（不弹错）。
- **落点**：驱动层，只读 core 状态；**三态都要能开**（无游戏 / NES 会话 / GBA 会话）。

### 3.2 验收

| # | 项 | 断言 |
|---|---|---|
| O2a | ctest（**新建**，只测纯数据组装） | 报告文本组装函数在「无 ROM」态不崩、含版本串与 base 目录 | 
| O2b | 人工 | 三态各开一次，界面无乱码、数值合理（欠载计数随播放变化） |
| O2c | 人工 | 「打开日志目录」「打开崩溃目录」在有/无目录两种情形下行为正确 |

---

## 四、O3 · Diagnostics 二级菜单（D2 裁决）

### 4.1 设计

- `src/drivers/Qt/MenuCatalog.h`：`AdvancedSub` 增 `Diagnostics`（`COUNT` 6→7）+
  `advancedSubTitle()` 增 case。
- `src/drivers/Qt/ConsoleMenu.cpp`：`advDiagMenu = advMenu->addMenu(tr("&Diagnostics"))`
  （置于 Misc Tools 之后），条目四项：
  「诊断信息…」 / 「打开日志目录」 / 「打开崩溃目录」 / 「文件日志」（勾选项，
  映射 `SDL.LogToFile`）。
- `src/drivers/Qt/ConsoleWindow.h`：**新增 slot 落在 `public slots:` 段**
  （`check_menu_slots.py` 的硬要求）——`openDiagnosticsDialog` /
  `openLogDirectory` / `openCrashDirectory` / 勾选处理。
- **i18n 全流程**（§1.3 的冻结规则）：
  1. `scripts/i18n_update.ps1`（lupdate）重扫 12 个 `.ts`；
  2. 新串在 **12 种语言**全部补译（zh_CN/zh_TW 用 `glossary.txt` 的规范译法，
     且遵守简繁分离规则；其余 10 种按仓库既有做法「机器翻译 + 事后复核」）；
  3. `scripts/i18n_coverage.ps1` 与 `scripts/check_simp_trad.ps1` 必须绿。
- **条数与措辞**：菜单标题与条目共约 **5–9 个新源串**（含 status tip）⇒ 覆盖率
  影响面很小，但**必须 12 语言补齐**才能守住 ≥95%/≥90% 门禁。

### 4.2 验收

| # | 项 | 断言 |
|---|---|---|
| O3a | `scripts/check_menu_slots.py` | EXIT=0（slot 声明到位） |
| O3b | `scripts/i18n_coverage.ps1` | 各语言覆盖率达标 |
| O3c | `scripts/check_simp_trad.ps1` | 简繁分离通过 |
| O3d | 人工 | Advanced → Diagnostics 四项逐一点验；切语言后文本随之更新 |

---

## 五、门禁与验收总表

| 门禁 | O1 | O2 | O3 |
|---|---|---|---|
| 全量 Release 构建（`--parallel 1`） | ✔ | ✔ | ✔ |
| `ctest -LE perf`（基线 **52**，以 `ctest -N -LE perf` 为准） | 52 → **54**（+`log_sink_test`） | → **55**（+`diagnostics_report_test`） | 不变 |
| Rust 各 crate 计数 | 不变 | 不变 | 不变 |
| `scripts/check_menu_slots.py` | — | — | ✔ |
| `scripts/i18n_coverage.ps1` + `check_simp_trad.ps1` | — | — | ✔ |
| `check.py`（批 1 门禁） | ✔（新文件必须带头） | ✔ | ✔ |
| 人工 | 日志落点/追加/轮转 | 三态对话框 | 菜单四项 + 切语言 |

---

## 六、不变式与红线

1. **NES 零回归**（不变式 1）：O1 改的是消息回调链（`FCEUD_Message` 的下游），
   **不得改变消息的投递语义**（dock 行为逐字不变）。
2. **GBA 解耦**：O1/O2/O3 的新代码**零 GBA 引用**；GBA 会话下对话框只显示
   「无 NES 游戏」态即可（**不新增 GBA 专属指标**——那属 GBA 线）。
3. **纪律 8 分层**：日志 sink 与对话框都在**驱动层**；core 侧零改动。
4. **不死锁**（O1 的头号风险）：**不得**在 sink 里取 wrapper 锁；sink 自带
   独立互斥量，且**任何持锁路径都不得回调 GUI**。
5. **i18n 冻结流程**：新增 `tr()` 必须走 `[i18n-string-add]` 流程（§1.3）。
6. **坑 11**：改仓库文件用编辑工具原地替换；改后 `git diff --numstat` 核对。

---

## 七、风险与止损

| 风险 | 处置 |
|---|---|
| **加锁引入死锁**（sink 被 GUI 与模拟线程同时调用） | 独立互斥量，锁内**只做文件 IO**；若实测有死锁 ⇒ 退回「单线程写 + 无锁」并在文档登记竞争残留（**不假装修好**） |
| **i18n 覆盖率掉线** | 新串 12 语言补齐；若某语言实在补不出 ⇒ 该语言回落英文源串（Qt 默认行为）**并如实登记**，不伪造覆盖率 |
| **日志路径变更影响既有用户**（原 `%TEMP%\fceux.log`） | 新路径另起文件；**旧文件不再写**，在 CHANGELOG 明确说明（用户可自行删除旧文件） |
| **对话框取数触发 core 状态不一致** | 只读原子计数器与不可变字符串；**不调用任何会改状态的 core API** |
| 止损线 | O1 若锁改造实测出现死锁 ⇒ 停止，回退到「仅路径迁移 + 轮转」，竞争问题登记为长尾 |

---

## 八、阶段编排

| 阶段 | 内容 | 依赖 |
|---|---|---|
| P1 | O1 步骤 1-3（路径推导 + 延迟打开/重定向 + 加锁）+ O1a/O1b 锁 | 无 |
| P2 | O1 步骤 4-6（轮转 + 错误级即时落盘 + `SDL.LogToFile` 开关）+ O1c/O1d 锁 | P1 |
| P3 | O2 报告数据组装 + `DiagnosticsDialog` + O2a 锁 + 人工三态 | 无（可与 P1/P2 并行） |
| P4 | O3 菜单接线（MenuCatalog + ConsoleMenu + slots）+ i18n 全流程 + 三个脚本门禁 | P3（对话框入口） |
| P5 | 收口：全表门禁 + CHANGELOG/AGENTS 登记 + CI 定谳 | P1-P4 |

每阶段开工前先在本文件记 r（纪律 2）；每阶段收口跑 §五 全表。
**push / tag / 发版一律等业主明确指示**。

---

## 九、明确不做（本批次范围外）

- 云端遥测 / Sentry / crashpad（D1 裁决，批 2 已定）。
- 日志分级体系（info/warn/error 全链路重构）与性能指标平台（超 O2 最小集）。
- 日志文件自动上传 / 远程诊断。
- 转储保留策略（批 2 已登记为长尾：转储无清理策略）。
- CI 覆盖率工具 / 静态分析 / 依赖扫描（报告「工程改进清单」节，需另立项）。

---

## 十四、变更记录

- **r1（2026-11-23）立项**：业主指示参照批 2 模式为批 3 立独立文档。只读调查完成，
  钉死三组坐标：
  ① **日志 sink**（`MsgLogViewer.cpp`）：四缺陷实测——路径 `%TEMP%\fceux.log`
  （`:56`）、`"w+"` 每次截断（`:69`）、`addLine` **无锁**（`:99`，且
  `FCEUD_Message` 确实来自模拟线程与 GUI 线程两方 ⇒ 竞争真实存在）、
  环形缓冲 256 行与文件无限增长脱节（`:46`/`:117`）；**关键约束** = 全局
  `msgLog` 是静态初始化对象 ⇒ 文件在 `main()` 前已打开，而 base 目录要到
  `InitConfig()`（`fceuWrapper.cpp:763`）才就绪 ⇒ **必须延迟打开 + 初始化后
  重定向**。
  ② **诊断数据面**（O2）：计数器**全部已存在**（`nes_shm->render_count`/
  `blit_count`、`sndBuf.starveCounter`、`getBaseFrameRate()`、`GameInfo`）⇒
  **本项无需新增埋点**；对话框范式 = `MsgLogViewDialog_t`；开目录设施 =
  `QDesktopServices`（`AboutWindow.cpp:123` 先例）。
  ③ **菜单与 i18n**（O3）：Advanced 现六个子菜单（`ConsoleMenu.cpp:137-146`），
  `AdvancedSub`/`advancedSubTitle`/`MenuPathId` 三处各 +1；`MenuPathId` **目前
  零消费者**（无须登记清单）；**CI 硬门禁** = `check_menu_slots.py`（slot 必须在
  `slots:` 段）+ `i18n_coverage.ps1`（zh_CN/zh_TW ≥95%、其余 ≥90%）+
  `check_simp_trad.ps1`；**i18n 源串冻结规则** ⇒ O3 的约 5–9 个新源串**必须走
  `[i18n-string-add]` 全流程**（lupdate → 12 语言补译 → 重跑覆盖率）。
  设计要点：O1 抽 `log_sink.{h,cpp}` 无 Qt 依赖部分以便可测；O2 拆「数据组装
  纯函数」与「对话框外壳」以便单测；O3 顺序 O1→O3→O2 调整为 **O1→O2→O3**
  （对话框先落地，菜单才有入口可挂）。
