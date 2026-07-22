# FCEUX11 v1.15 LTS hotfix4 PLAN

## 〇、目标、审计方法与基线

**目标**：hotfix4 面向"软件界面与菜单可达功能"做回归排查与修复，重点回答一个问题——漫长的优化/重构迭代是否破坏了原版 FCEUX 的既有功能。同时补齐 UI 层测试缺口，防止同类回归再次发生。

**审计方法**（已于 2026-07-22 完成，只读）：

1. 主菜单全部约 90 个 QAction 的 connect ↔ 槽声明 ↔ 槽实现逐一交叉核对（`ConsoleMenu.cpp` / `ConsoleActions.cpp` / `ConsoleMenuBar.cpp` / `ConsoleWindow.h`）；
2. 与上游 `TASEmulators/fceux@master` Qt 驱动逐段对照（菜单 tr() 标题集合 diff：上游 228 项 vs 本地 190 项，逐项归因）；
3. 全部 17 个菜单可达对话框 + 9 个单例工具窗口的打开链路（构造早退、单例复位、重复打开守卫）；
4. i18n（12 语言 .ts/.qm、retranslate 覆盖）与 53 个 HK 热键枚举的接线核对；
5. 遗留文档（hotfix1-3 PLAN、remaining_work.md、CHANGELOG）、`output/` 运行日志、近 20 条 git 提交。

**已验证基线**（2026-07-22）：`cmake --build build` 增量构建 `BUILD_EXIT=0`；CTest 注册 29 项测试。构建需在 Developer PowerShell（或先 `call vcvars64.bat`）中进行；Git Bash 直接调 `cmake --build` 会因缺 MSVC 环境失败。

**审计后确认无问题的区域**（不再列入修复项）：17 个对话框与 9 个单例窗口的打开/关闭链路全部完好；约 130 处 `setEnabled(false)` 均为正常 UI 状态逻辑；`fceu_callbacks.cpp` 的 DUMMY 回调与上游逐字一致；checkable 动作的初始状态同步全部到位；`output/*.err` 中的失败记录均为手动脚本工作目录错误，非代码回归。

---

## 一、确认缺陷清单（修复对象）

### P0 — 确认功能失效

**D-1　FDS → Load BIOS 菜单完全失效（重构回归）**
- 证据：`src/drivers/Qt/ConsoleMenu.cpp:846` 连接到不存在的槽：
  ```cpp
  connect(fdsLoadBiosAct, SIGNAL(triggered()), this, SLOT(fdsLoadBIOS(void)) );
  ```
  真实槽名为 `fdsLoadBiosFile`（声明 `src/drivers/Qt/ConsoleWindow.h:444`，实现 `src/drivers/Qt/ConsoleEmuControl.cpp:529`）。上游同位置连的就是 `fdsLoadBiosFile`。
- 运行时证据：`output/fceux11_run2.err`：`QObject::connect: No such slot consoleWin_t::fdsLoadBIOS(void)`。
- 影响：用户无法通过菜单安装 FDS BIOS（disksys.rom）。
- 修复：一行，把 `SLOT(fdsLoadBIOS(void))` 改为 `SLOT(fdsLoadBiosFile(void))`。

**D-2　两处 PowerShell 续行符 `` `n `` 源码残留**
- 证据：`src/drivers/Qt/ConsoleActions.cpp:111`、`src/drivers/Qt/ConsoleTranslation.cpp:111`：
  ```cpp
  #include "Qt/ConsoleActions.h"`n#include "Qt/ConsoleWindow.h"
  ```
  由提交 `2540f2b` 引入；与 hotfix1 C-12（`ConsoleVideo.cpp:111`）同类，hotfix1 只修了一处。当前编译可过的原因仅是两文件在第 71 行已提前包含 `ConsoleWindow.h`；MSVC 对此报 C4067 警告，且 `src/CMakeLists.txt:96` 的 `/WX-` 把 src 全树的 warnings-as-errors 关掉了（见 D-16）。
- 修复：两处 `` `n `` 替换为真实换行；修后全仓 grep `` `n `` 确认 src 下无同类残留。

**D-3　4 个热键在 Hotkey Config 中可绑定但完全无功能**
- 证据：`HK_FDS_EJECT` / `HK_VS_TOGGLE_DIPSWITCH` / `HK_TOGGLE_SUBTITLE` / `HK_LOAD_LUA` 在 `src/drivers/Qt/config.cpp:93,173,179,185` 注册（Hotkey Config 对话框可见、可绑定），但业务代码引用全部被注释：`src/drivers/Qt/input.cpp:908,1145,1118,1072`。
- 修复（分两类处理）：
  - `HK_FDS_EJECT`：菜单动作 `fdsEjectAct`（`ConsoleMenu.cpp:838-842`）本身有效，只是没接热键。参照 `HK_FDS_SELECT` 的既有模式（`ConsoleMenu.cpp:833-834`）补 `setAction` + 轮询/连接接线；
  - 其余 3 个上游同为死代码：从 `config.cpp` 的注册表中移除（Hotkey Config 不再出现死键），并在 CHANGELOG 记录"上游继承的未实现热键已隐藏"。

**D-4　切换语言后修改热键 → 全部菜单标签回退为启动时语言（本项目 i18n 回归）**
- 证据：`src/drivers/Qt/input.cpp:319-321` `hotkey_t::setAction` 在菜单构建时缓存一次文本快照 `actText`；`input.cpp:275` `readConfig()` 每次刷新热键都用这份陈旧快照重写 action 文本。`readConfig` 由 `setHotKeys()`（`fceuWrapper.cpp:936`、`HotKeyConf.cpp:260,493,516`）触发。
- 复现：中文启动 → 切换英文 → 打开 Hotkey Config 改任意键 → 所有带热键的菜单项变回中文。
- 修复：废弃 `actText` 快照。给 `hotkey_t` 增加 `refreshText()`：取 `act->text()`，剥离末尾已有 `\t` 后缀（`lastIndexOf('\t')`），再用当前 shortcut 重新拼接；`readConfig()` 改调 `refreshText()`。

**D-5　切换语言后菜单项的热键提示后缀（`\tF5` 等）永久丢失**
- 证据：快捷键不靠 `QAction::setShortcut` 显示，而是 `setAction` 拼接进文本（`input.cpp:321`）；`retranslateUi()`（`src/drivers/Qt/ConsoleWindow.cpp:761-942`）全部用纯 `setText(tr(...))` 覆盖，之后无任何代码重新拼接。
- 修复：与 D-4 同一处——`retranslateUi()` 末尾遍历 53 个 `Hotkeys[]`，对已 `setAction` 的统一调 `refreshText()`。D-4/D-5 必须同 PR 修复。

### P1 — 确认缺陷 / 功能缺失

**D-6　三个子菜单条目永远不可翻译（`tr(运行时变量)`）**
- 证据：`src/drivers/Qt/ConsoleMenu.cpp:256`（`Slot &%i`，Change State Slot）、`:477`（`&%ix`，Window Resize 1x-4x）、`:725`（`%i On, %i Off`，AutoFire Pattern）。lupdate 只提取字符串字面量；`retranslateUi` 中同模式代码（`ConsoleWindow.cpp:855-873, 925-933`）同样无效。已验证 `src/drivers/Qt/lang/fceux11_zh_CN.ts` 中无对应条目。
- 修复：改为可提取形式，如 `tr("Slot &%1").arg(i)`、`tr("&%1x").arg(i)`、`tr("%1 On, %2 Off").arg(on).arg(off)`；菜单构建与 `retranslateUi` 两处同步改；随后运行 `scripts/i18n_update.ps1` 更新 12 语言 .ts 并重新生成 .qm。

**D-7　`hotkey_t::getString` 把热键名截断到 7 字符**
- 证据：`src/drivers/Qt/input.cpp:340` `FCEU_strlcpy(s, sizeof(s), ...)`——`s` 是 `char*` 形参，`sizeof(s)` 恒为 8；调用方 `src/drivers/Qt/GamePadConf.cpp:2341,2529` 传入的是 128 字节缓冲。
- 影响：GamePad 配置界面热键名显示截断（如 `Ctrl+Sh`）。
- 修复：`getString` 增加 `size_t size` 参数并传给 `FCEU_strlcpy`；两处调用方传 `sizeof(缓冲)`。

**D-8　NetPlay（联机对战）整体失效 — 转为正式裁剪**
- 证据：`src/drivers/Qt/fceuWrapper.cpp:365` `//FCEUD_NetworkConnect();`（ROM 加载后不再发起联机）；无 NetPlay 菜单（上游有顶层 `&NetPlay` 菜单）；`src/drivers/Qt/config.cpp:588` 官方自注 `// network play options - netplay is broken`，但 `--net/--port/--user` CLI 选项仍注册；`src/drivers/Qt/QtNetplay.cpp` 仅剩回调接线，`FCEUI_NetplayStart` 无 UI/CLI 可达路径；`src/archived/fceux-server/` 已归档。
- 决策：hotfix4 **不恢复联机**（工程量远超 hotfix 范畴），执行正式裁剪：
  1. 移除 `config.cpp` 中 netplay 相关 CLI 选项注册；
  2. 删除 `QtNetplay.cpp` 中无消费者的死代码（保留 `fceu_callbacks.cpp` 回调桩，核心侧 `netplay.cpp` 不动，避免 ABI/savestate 风险）；
  3. `readme.md` 与 CHANGELOG 明示"NetPlay 自 hotfix4 起正式移除（上游本就 broken），如有需求见 v2.0 路线"。

**D-9　仿真速度预设热键（25%–1600% 七档）被删除**
- 证据：上游 `config.h` 有 `HK_SPEED_QUARTER/HALF/NORMAL/2X/4X/8X/16X` 并在 `initHotKeys` 接到 `CustomEmulationSpeed(...)`；本地 `src/drivers/Qt/config.h:20` 无此枚举，全仓 `HK_SPEED` 零引用。`CustomEmulationSpeed()` 本身仍在（`src/drivers/Qt/sdl-throttle.cpp:409`），仅被 Custom 对话框使用。
- 修复：参照上游 `fceux@master` `ConsoleWindow.cpp::initHotKeys` 对应段，恢复 7 个枚举值、`config.cpp` 注册与 `ConsoleHotKeys.cpp` 接线（菜单 Speed 子菜单不动）。

**D-10　Help → Documentation 子菜单缺失，离线帮助成死代码**
- 证据：上游 Help 菜单有 `&Documentation` 子菜单（Online/Local）；本地 Help 仅 About/About Qt/Message Log（`ConsoleMenu.cpp:1255-1277`）。`openOnlineDocs`（`ConsoleHotKeys.cpp:100`）仅剩右键上下文菜单可达（`ConsoleWindow.cpp:606-607`）；`openOfflineDocs`（`ConsoleHotKeys.cpp:109` → `OpenHelpWindow`，`HelpPages.cpp` 整套仍在编译）**无任何调用方**。
- 修复：Help 菜单加回 `Documentation` 子菜单（Online / Local 两项，分别接 `openOnlineDocs` / `openOfflineDocs`），`retranslateUi` 同步加条目；在线链接暂保留 fceux.com 上游文档站，CHANGELOG 记录"链接目标待 v1.16 换成本项目文档"。

**D-11　Recent ROMs 菜单缺 "Clear Recent ROM List" 项**
- 证据：上游 `ConsoleWindow.cpp:2344-2346` 有该项及 `clearRecentRomMenu` 槽；本地 `src/drivers/Qt/ConsoleRecentRom.cpp:124-155` `buildRecentRomMenu()` 只填充 10 条记录，无 Clear 项（内部 `clearRomList()` :113 已存在，无槽包装）。
- 修复：`buildRecentRomMenu()` 末尾加分隔线 + Clear 动作，新增 `clearRecentRomMenu()` 槽调用 `clearRomList()` 并重建菜单；`retranslateUi` 同步。

### P2 — 低优先（同 PR 顺手修或单项小 PR）

**D-12　toggleGameGenie 会话内状态反向（上游继承缺陷，顺手修）**
- 证据：`src/drivers/Qt/ConsoleDebugWindows.cpp:259-270`：config 写入新值 `!gg_enabled`，但 `FCEUI_SetGameGenie(gg_enabled)` 传旧值，`checked` 参数未使用。重启后自愈，会话内核心状态与菜单勾选相反。
- 修复：改用槽的 `checked` 参数统一写 config 与核心。上游同病，本修复属"比上游更好"，CHANGELOG 注明。

**D-13　Advanced 顶层菜单未接"菜单打开时暂停"信号**
- 证据：`src/drivers/Qt/ConsoleMenu.cpp:136-153` 创建 `advMenu` 后未 connect `aboutToShow/aboutToHide`；而 fileMenu(:158-159)、optMenu(:345-346)、emuMenu(:551-552)、helpMenu(:1252-1253) 及六个 Advanced 子菜单都接了 `mainMenuOpen/mainMenuClose`。
- 修复：补两行 connect，行为与其他顶层菜单一致（仅影响 `SDL.PauseOnMainMenuAccess` 开启时）。

**D-14　TraceLogger 析构最长阻塞约 16.7 分钟**
- 证据：`src/drivers/Qt/TraceLogger.cpp:409-411` `diskThread->wait(1000000)`（1000000ms）。线程若卡在 Windows 无缓冲磁盘 I/O 未响应中断，关闭窗口冻结 GUI。
- 修复：超时改为 5000ms；超时后 `terminate()` + 短 `wait()`（参照 hotfix3 A-4 `ConsoleWindow.cpp` 析构的既有模式）。

**D-15　statusTip 与 MsgLogViewer 不重译**
- 证据：`retranslateUi()`（`ConsoleWindow.cpp:761-942`）全文 0 处 `setStatusTip`；`MsgLogViewer.cpp:210-211` 只在构建时 `tr()` 一次，无 `changeEvent`/retranslate（其余 33 个对话框均有）。
- 修复：`retranslateUi` 补主要动作的 `setStatusTip(tr(...))`；`MsgLogViewer` 补 `changeEvent` 重译（参照任一已有对话框的实现模式）。

**D-16　【评估项，不强制】`/WX` 政策名存实亡**
- 证据：顶层 `CMakeLists.txt:53` `/W4 /WX`（注释宣称 v0.3.16 起 /WX 激活），但 `src/CMakeLists.txt:96` 追加 `/WX-` 把 src 全树的警告即错误关掉——D-2 的 C4067 残留能长期存活正因此。
- 处理：hotfix4 只做一次试验性编译（移除 `/WX-` 后全量构建）。若警告仅零星几条则顺手修掉并移除 `/WX-`；若大面积爆发（Qt 头文件噪音），保留 `/WX-` 并在 `src/CMakeLists.txt` 注释中写明与顶层政策的矛盾及原因，归入 v2.0 债务清单。**禁止**为消除警告做大规模代码改动。

---

## 二、测试补网（防同类回归）

**T-1　新增 headless 菜单接线静态测试（核心交付物）**

D-1 类缺陷（SLOT 名拼错，运行时静默失败）现有 29 项 CTest 全部抓不到——`tests/` 中唯一两个 Qt 邻近测试（`i18n_regression_test`、`config_store_test`）都刻意不触 UI（见 `tests/CMakeLists.txt:115-119` 注释）。

新增 `scripts/check_menu_slots.py`（Python3，无第三方依赖，与 `tools/transform_v036.py` 同风格）：

1. 扫描 `src/drivers/Qt/*.cpp` 中所有 `SLOT(<name>(...))`，筛选 receiver 为 `this`（即 `consoleWin_t`）的连接；
2. 解析 `src/drivers/Qt/ConsoleWindow.h` 的 slots 区段声明，核对每个 `<name>` 存在；
3. 反向核对：所有菜单 `connect` 的 SIGNAL 侧 action 均已 `addAction` 进菜单（防止"动作存在但没挂进菜单"）；
4. 任一核对失败输出 `文件:行号` 并非零退出。

在 `tests/CMakeLists.txt` 注册（参照 `i18n_regression_test` 的注册方式，`WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}`）：

```cmake
find_package(Python3 COMPONENTS Interpreter)
if(Python3_Interpreter_FOUND)
    add_test(NAME menu_slot_check
        COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/scripts/check_menu_slots.py
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR})
endif()
```

验收：测试在修复前的 D-1 代码上应 FAIL（可先注释修复验证一次），修复后 PASS。

**T-2　运行时冒烟（手动验证步骤，不进 CTest，需显示环境）**

```powershell
.\build\src\fceux11.exe 2> smoke.err
# 断言：smoke.err 中无 "QObject::connect: No such slot" / "No such signal"
```

每个 Phase 完成后执行一次。

---

## 三、分阶段执行计划

> 每阶段独立 commit；构建与测试命令统一为（Developer PowerShell）：
> ```powershell
> cmake --build build                                   # 增量构建
> ctest --test-dir build --output-on-failure            # 全量测试
> ```
> 或一键 `.\scripts\do_build.ps1 -Config Release`（自动探测 MSVC 环境并跑测试）。

**Phase A — 一行级确认修复（D-1, D-2, D-7, D-13, D-14）**
均为低风险点修。完成后：构建通过、29 项 CTest 全绿、T-2 冒烟无 connect 警告（D-1 修复的直接证据）。

**Phase B — i18n 与热键（D-4, D-5, D-6, D-3）**
D-4/D-5 同 PR（`hotkey_t::refreshText()` 方案）；D-6 需跑 `scripts/i18n_update.ps1` 并确认 12 语言 .qm 重新生成；D-3 的 `HK_FDS_EJECT` 接线参照 `HK_FDS_SELECT` 模式。完成后人工验证：中文启动 → 切英文 → 菜单热键后缀仍在（D-5）；改热键后菜单不回退中文（D-4）；State Slot / Window Resize / AutoFire 子菜单显示中文（D-6）。

**Phase C — 功能补齐（D-9, D-10, D-11, D-12, D-15）**
D-9 参照上游 master `initHotKeys`；D-10/D-11 菜单新增项需同步 `retranslateUi`。完成后人工验证对应菜单项可达可用。

**Phase D — 裁剪与评估（D-8, D-16）**
D-8 按 §一 决策执行裁剪 + 文档；D-16 只做试验与记录，不强制改动。

**Phase E — 测试补网与总验证（T-1, T-2）**
落地 `check_menu_slots.py` 并注册 CTest；最终 `ctest --test-dir build --output-on-failure`（应含新测试共 30 项）全绿；T-2 冒烟通过；更新 `CHANGELOG.md`（新增 `[1.15(hotfix4)]` 段）与 `readme.md` 版本徽标。

---

## 四、验收标准

- P0（D-1~D-5）、P1（D-6~D-11）全部修复落地；P2 中 D-12~D-15 修复落地，D-16 有明确试验结论记录；
- Release 构建通过，无新增警告（特别注意 D-2 修后 C4067 消失）；
- CTest 30 项（29 + T-1）全部通过；
- T-2 冒烟：`smoke.err` 无任何 `No such slot` / `No such signal` 警告；
- i18n：`scripts/i18n_update.ps1` 执行后 12 语言 .ts 无新增 `unfinished` 条目，.qm 全部重新生成；
- 菜单层级、快捷键默认值、savestate 字节格式不变；不改变任何核心模拟行为。

## 五、执行约束

- 最小改动：点修为主，禁止顺手重构无关代码；
- 保持现有菜单重组结构（Tools/Debug/Movie 并入 Advanced 五个子菜单是本项目设计，不回退）；
- 每阶段独立 commit，消息遵循 `cliff.toml` 约定（`fix(ui): ...` / `fix(i18n): ...` 等）；
- 凡与上游有意的行为差异（D-12 等），在 CHANGELOG 中注明。

---

## 六、二次审计意见（执行人：Claude / 2026-07-22）

> 本节为执行前最后一轮独立核验。结论：**16 个缺陷全部经源码/日志核对为真实**（详见末尾核验表），计划整体质量高、按 Phase A → E 顺序推进即可。下文为本执行人后续开工必须遵守的补充约定。

### 6.1 核验结论摘要

- **D-1 ~ D-16 全部真实**。每条都能用 `Grep` / `Read` 直接复现文件:行号级证据；D-1 还有 `output/fceux11_run2.err:5` 的运行时警告作为独立佐证。
- **基础设施存在性已确认**：`scripts/do_build.ps1`、`scripts/i18n_update.ps1` 在位；`tools/transform_v036.py` 在位（T-1 风格参照）；`tests/CMakeLists.txt` `add_test(` 计数 = 29（与计划一致）。`scripts/check_menu_slots.py` 按计划应于 T-1 新增，**开工前不存在是预期**。
- **Git 卫生**：仓库当前在 `main`、干净。每次 Phase 必须独立 commit；不得 squash/fixup 多个 Phase。

### 6.2 开工前必做的 3 件事

1. **重新读一遍本文档**（每次新会话开头），避免凭印象施工。
2. **`git checkout -b hotfix4/<phase>-<id>` 起步**（如 `hotfix4/phase-a-d1-d2-d7-d13-d14`），不要直接在 `main` 上堆 commit，方便 review 与回退。Phase E 验收通过后再合 `main`。
3. **每 Phase 开工前先 `cmake --build build` 跑一次基线**，确认当前未提交改动下基线仍为绿。出现意外基线问题先停下汇报，不要带病施工。

### 6.3 风险点与必须事先确认的细节

> 这些点计划本身已写明，但执行中容易"读完就忘"。**开工对应 Phase 前必须回看本小节相关条目**。

- **D-3 上游截屏**：`HK_LOAD_LUA` / `HK_VS_TOGGLE_DIPSWITCH` / `HK_TOGGLE_SUBTITLE` 这 3 个执行"从 config.cpp 移除"前，必须先拉一次 `TASEmulators/fceux@master` `initHotKeys` 段截屏附在 PR 描述里。若上游其实有实现、只是本地没接，要立刻停下来回报，不要静默删除。`HK_FDS_EJECT` 走"参照 HK_FDS_SELECT 接线"路线无此风险。
- **D-4/D-5 refreshText 副作用**：`hotkey_t::setText` 会触发 `QAction::textChanged`。Phase B 落地前必须 `Grep "textChanged"` 全 Qt 树，确认没有监听者（典型嫌疑：Recent ROM 动态菜单、状态栏文本拼接）。若有，预先在 PR 描述里列出"已 grep 无监听"。
- **D-6 第二落点**：`ConsoleWindow.cpp:855-873, 925-933` 三处同模式 `tr(运行时变量)` 在 `retranslateUi` 内。Phase B 改构建侧（ConsoleMenu.cpp:256/477/725）时必须**同步 diff 这三处**，否则语言切换时菜单中文化、对话框不中文化的双轨 bug 会出现。改完先跑一次 zh_CN/en 切换冒烟，再交 CTest。
- **D-13 手动验收**：Phase A 修 `advMenu` aboutToShow/aboutToHide 后，必须**手动验证一次**"打开 Advanced 顶层菜单时 SDL.PauseOnMainMenuAccess 是否正确触发 mainMenuOpen/mainMenuClose"——仅靠 T-2 抓不到（不会进入 paused 态）。记录在 PR 描述里。
- **D-14 超时 5000ms 仍偏长**：参照 hotfix3 A-4 的析构既有模式，但 5000ms 仍可能在被杀毒软件卡住的磁盘 I/O 上不够。先落地 5000ms；若后续 hotfix5 有用户反馈再降到 1000ms。本次不要拍脑袋降到 1s。
- **D-16 试验性构建**：先 `cmake --build build --clean-first` 走一遍全量构建，记录 warning 总数与归类（Qt 头文件 / 自家代码）。**禁止**为消 warning 改任何非目标代码——计划明文要求"禁止大规模代码改动"。结果无论成败都在 `output/d16_wx_trial.log` 留档并附结论。

### 6.4 T-1 (check_menu_slots.py) 必须满足的硬条件

1. **红→绿验证**：脚本落地后**先在未修的 D-1 代码上跑一次，必须 FAIL 且明确报告 `ConsoleMenu.cpp:846: slot 'fdsLoadBIOS' not declared`**；再修 D-1，确认 PASS。两次结果都贴在 Phase E 的 PR 描述里。
2. **已知边界**：脚本只防"SLOT 名拼错 / receiver 不是 `this`"，不防"SIGNAL 端错"。在脚本头部 docstring 与 PR 描述里都写明。
3. **零第三方依赖**：只允许 Python3 标准库。CI 上若 Python3 缺位，`find_package(Python3 COMPONENTS Interpreter)` 应静默跳过该测试（参照现有 i18n_regression_test 的注册风格）。
4. **不要反向过拟合**：脚本不应该要求 SLOT 都被某个特定 action 显式 addAction 进菜单——`aboutToShow` 类连接用 `this` 而不挂菜单，强制要求会导致误报。

### 6.5 T-2 (smoke) 操作规范

- `smoke.err` 输出到 `output/smoke_<phase>.err`，**不要覆盖既有 `output/fceux11_run*.err`**。
- 跑完后立刻 `grep -E "No such slot|No such signal" output/smoke_<phase>.err`，**期望零匹配**。
- 跑完把 exe 进程关掉（GUI 窗口关或 taskkill /IM fceux11.exe /F），避免后续 phase 启动时端口/单例冲突。

### 6.6 报告义务（每 Phase 收尾）

向用户回报时**必须**包含以下 5 项，缺一项视为 Phase 未完成：

1. 本 Phase 涉及哪些 D-id，commit hash 列表；
2. `cmake --build build` 与 `ctest --test-dir build --output-on-failure` 的尾部输出（红/绿结论 + 用时）；
3. T-2 smoke.err grep 结果；
4. 是否有"计划未列、但顺手发现的旁支问题"——有就列出来，**不要当场修**，留给 hotfix5 / 单独 PR；
5. 下一 Phase 的开工计划（一句话即可）。

### 6.7 禁止事项（再强调一次）

- ❌ 不要在 hotfix4 内重构菜单层级（即便觉得 Advanced 五子菜单怪）；
- ❌ 不要顺手"清理"D-2 之外的 ` `n` 残留 warning（D-2 是命中范围，其他发现归 hotfix5）；
- ❌ 不要为 D-16 试验改任何 Qt 头文件引用或 `#pragma warning`；
- ❌ 不要把多个 Phase 合并成一个 commit；
- ❌ 不要跳过 Phase E 的 T-1 红→绿验证直接交 PASS；
- ❌ 不要在 `main` 上直接 commit（先开分支）。

### 6.8 附录：缺陷真实性核验抽样记录

| 编号 | 核验方式 | 关键证据 |
|---|---|---|
| D-1 | Read + 运行时日志 | `ConsoleMenu.cpp:846` 实为 `SLOT(fdsLoadBIOS(void))`；`ConsoleWindow.h:444` 槽实为 `fdsLoadBiosFile`；`output/fceux11_run2.err:5` 含 `QObject::connect: No such slot consoleWin_t::fdsLoadBIOS(void)` |
| D-2 | Grep | `ConsoleActions.cpp:111` 与 `ConsoleTranslation.cpp:111` 两行均含字面 `` `n `` |
| D-3 | Grep | `config.cpp:93,173,179,185` 注册；`input.cpp:908,1072,1118,1145` 均为 `// Hotkeys[HK_...].getRisingEdge()` 注释 |
| D-4 | Read | `input.cpp:319` `actText = act->text();`；`input.cpp:275` 复用 `actText` 重写 text |
| D-5 | Grep | `ConsoleWindow.cpp` 全树零 `setStatusTip`；retranslateUi 段无 refresh 调用 |
| D-6 | Read | `ConsoleMenu.cpp:256/477/725` 均为 `tr(snprintf 后字符串)` |
| D-7 | Read | `input.cpp:340` `FCEU_strlcpy(s, sizeof(s), ...)`，`s` 为 `char*` 形参 |
| D-8 | Grep | `fceuWrapper.cpp:365` 注释调用；`ConsoleMenu.cpp` 全树零 `NetPlay`；`config.cpp:588` 自注 broken |
| D-9 | Grep | `config.h` 枚举无 `HK_SPEED_QUARTER/HALF/NORMAL/2X/4X/8X/16X`；Qt 目录 `HK_SPEED` 零匹配 |
| D-10 | Grep + Read | `ConsoleMenu.cpp:1255-1277` 只 About/About Qt/Message Log；`openOfflineDocs` 全 Qt 树零调用方 |
| D-11 | Read | `ConsoleRecentRom.cpp:124-155` 无 Clear 项 |
| D-12 | Read | `ConsoleDebugWindows.cpp:265-267` config 写新值、`FCEUI_SetGameGenie` 用旧值 |
| D-13 | Read | `ConsoleMenu.cpp:136` 创建 advMenu 后无 connect；`fileMenu:158-159` 反有 connect |
| D-14 | Read | `TraceLogger.cpp:411` `diskThread->wait( 1000000 );` |
| D-15 | Grep + Read | `ConsoleWindow.cpp` 零 `setStatusTip`；`MsgLogViewer.cpp:210-211` 仅构建时 tr() |
| D-16 | Read | `src/CMakeLists.txt:96` `add_compile_options(/W4 /WX- ...)` |


