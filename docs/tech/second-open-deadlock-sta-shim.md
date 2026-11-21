# Windows 11 IFileDialog「二次打开死锁」与 STA 引用垫片（BUG-V205-03）

> 本文收纳自本仓 wiki 技术文章《The‑Second‑Open Deadlock: How a Deliberately
> Unbalanced CoInitializeEx Tamed a Windows 11 IFileDialog Hang》，按本仓 tech 文档
> 体例（中文正文、代码/文档锚点、坑与教训体例）重排；事实内容以 wiki 原文为准，
> 代码锚点已按**当前树**更新（wiki 写作时垫片只覆盖 2 处，现全部 8 处落地）。
>
> 关联：BUG-V205-03 · v2.0.4_hotfix2（r7 推迟装载 + r8 STA 垫片）·
> `docs/history/plans/FCEUX11-v2.0.5_构建计划.md` §9.11/E13（取证）与 §9.20（A/B/C/D 实验）

## §〇 TL;DR

- **症状**：Windows 11（build 26100）上，「开 ROM → 关 → 再开 → 关 → 第三次开」
  的原生文件对话框（QFileDialog 背后的 IFileDialog，经 windows.storage.dll）
  **永久挂死**——不是崩溃，是 "(Not Responding)" 的冻结窗口。挂死总发生在前
  两个关/开周期内，且**仅鼠标点击 Open 触发**（键盘自动化从不复现）。
- **修复**：每次打开原生对话框之前调用一次**故意不配对**的
  `CoInitializeEx(NULL, COINIT_APARTMENTTHREADED)`——不调 `CoUninitialize`，
  进程生命周期内持有一个「纯计数」的额外 STA 引用。BUG 编号 **BUG-V205-03**。
- **现状**：8 个对话框调用点全覆盖（见 §五 锚点）；逃生门保留
  （`SDL.UseNativeFileDialog=0`）。

## §一 症状与取证

**复现特性**：固定序列（开→关→开→关→开），前两个周期内必挂，仅鼠标路径。

**取证工具链**（全部为当时自建）：
- **响应探针**：每 250 ms 对主窗口 `SendMessageTimeoutW(hwnd, WM_NULL, ...,
  SMTO_ABORTIFHUNG)`——挂死判定不用猜。
- **WCT 等待链**：`OpenThreadWaitChainSession`。⚠️ **Flags 参数必须是 0**，
  否则 `ERROR_INVALID_PARAMETER`（踩过才写下来）。
- **裸栈扫描**：`ReadProcessMemory` 按「模块+偏移」打印——不搞 unwinder，
  偏移足够定位阻塞点。

**两条字节级相同的栈**（两次独立复现）钉死了一个经典死锁环：
- **对话框宿主线程**：阻塞在 `NtUserEnableWindow`/SendMessage 指向 GUI 线程
  （COMDLG32 → windows.storage.dll → combase → RPCRT4）；
- **GUI 线程**：停在 `CoWaitForMultipleHandles`——一个**不泵窗口消息**的 COM
  专用等待——等一个经 shell 代理/存根的远程调用。

**三个排除性结论**（同样来自栈）：对话框跑在宿主线程而非调用者的 STA（GUI 线程
上没有 COMDLG32 帧）；死锁环成立；**栈里没有任何应用自身帧**——wrapper 锁理论
被当场否掉。

## §二 两个被证伪的假设

1. **对话框析构里有重活**——用 `QTimer::singleShot(0, ...)` 推迟
   `CloseGame()`/`LoadGame()`：是卫生改善，但栈字节级相同 ⇒ 否。
2. **COM apartment 错配**——探针用 `CoInitializeEx` 结果分类
   （S_OK = 之前无 apartment、S_FALSE = 已是 STA、RPC_E_CHANGED_MODE = MTA），
   每个检查点都显示 STA ⇒ 否。

## §三 意外即数据：探针构建停止复现

排查中途探针构建**不再挂死**。它有个 quirk：每次打开前调 `CoInitializeEx`，
但只在 S_OK 时配对 `CoUninitialize`——S_FALSE 时不配（违反 COM 契约）。结果
每次打开净留**一个未释放的 STA 引用**，而它恰好是修复。

「它停止复现」本身就是数据——这条与 AGENTS.md 既有教训同族（BUG-V205 当时的
原话），而且后续实验专门为它补了一条对照腿（§四 的 C 腿）。

## §四 A/B/C/D 四腿对照实验（同机 / 同操作员 / 同 ROM）

| 构建 | 每次打开的额外 STA 引用 | 结果 |
|---|---|---|
| B（release） | 无 | 1–2 周期内挂（5 次独立观测） |
| A（探针，不配对） | +1 保留（每次两次） | 连开 4 次，无挂 |
| C（探针，配对） | +1 再 −1，净 0 | 5+ 周期无挂，之后挂 |
| D（最终垫片） | +1 保留 | 连开 14 次，无挂 |

- A+D 合计 **18 次连续打开零挂**，对照 5 次 ≤2 周期挂——同分布下巧合概率 ~0.1%。
- **C 腿是牙齿**：净引用数与 D 相同、存活表现却不同 ⇒ 当时最受青睐的机制
  （「对话框实现丢弃调用侧 apartment 引用 → STA 被拆除」）被证伪 ⇒
  **机制只被部分理解，如实登记未闭合**——不要把假话写成结论。
- 教训：单变量实验要带**变异腿**（C）——与 AGENTS.md 纪律 3 的变异验证同族，
  只是这里的变异是「净等价」的引用策略。

## §五 最终修复与本仓锚点

**形态**：每次打开**一次**（`exec()` 之前），而非探针的每次两次；「泄漏」是纯
整数簿记——计数背后没有资源，OLE 也不要求进程退出时归零；「理论上更整洁」的
配对形态被 C 腿证伪后弃用（**发布你实测过的形态**，§七）。

**代码锚点（当前树，8 处全带 BUG-V205-03 注释）**：

| 文件 | 位置 | 覆盖的对话框 |
|---|---|---|
| `src/drivers/Qt/ConsoleFile.cpp` | :290 附近 | openROMFile（ROM 打开）——首发 |
| `src/drivers/Qt/ConsoleFile.cpp` | :408 附近 | loadNSF（NSF 打开）——首发 |
| `src/drivers/Qt/ConsoleEmuControl.cpp` | ×3 | follow-up 扩展 |
| `src/drivers/Qt/ConsoleRecording.cpp` | ×2 | follow-up 扩展 |
| `src/drivers/Qt/ConsoleDebugWindows.cpp` | ×1 | follow-up 扩展 |

⚠️ wiki 写作时只有前两处、其余登记为 follow-up——**当前树里 follow-up 已全部
落地**，以代码为准。

⚠️ **位置纪律**：垫片必须在 `exec()` **之前**。loadNSF 一处曾放在 `exec()`
之后，保护了零东西（2026-11-19 位置纠正，注释里有记录）。

**文档锚点**：`docs/history/plans/FCEUX11-v2.0.5_构建计划.md` §9.11/E13（取证
记录）与 §9.20（A/B/C/D 实验与开放问题）。发布载体：v2.0.4_hotfix2
（r7 推迟装载 + r8 STA 垫片）。

## §六 开放问题（如实登记未闭合）

- windows.storage.dll 为什么在关/开周期后不再把对话框托管在调用者 STA；
  一个瞬态配对为什么能移动概率。
- build 26100 以外的 Windows 行为；经典 `GetOpenFileNameW` 是否免疫。
- 微软是否将其归类为 bug（找到的最接近类比：BeeWare briefcase issue #930）。

机制理解**只算部分闭合**——C 腿证伪了最简机制，但没有更好的机制被证实。

## §七 工具箱与教训

工具三件：`SendMessageTimeoutW`+`SMTO_ABORTIFHUNG` 响应探针；WCT 等待链
（Flags=0）；`ReadProcessMemory` 裸栈扫描（模块+偏移）。

教训（wiki 原文要点，按体例重排）：
1. **「它停止复现」是数据**——不是侥幸，去 diff 你改了什么。
2. **字节级相同的两条栈把轶事变成事实**。
3. **栈里没有自己的帧 = 证据**——直接排除自己代码的嫌疑。
4. **单变量实验带变异腿**（C 腿）。
5. 第一幸存假设不要急着翻转用户可见的默认值。
6. **Ship the shape you measured**——不配对形态实测 18 连开存活，就发它，
   而不是理论上更整洁的配对形态。

## §八 与本仓其它文档的衔接

- 活体冻结取证（本文：WCT/裸栈/响应探针）与无头冻结复现
  （`gba-compat-methodology.md` §五 的 FFI 模式）是**两类冻结的两件武器**：
  前者管 Windows/COM/对话框层的真机取证，后者管模拟核心路径的无头复现。
- 「如实登记未闭合」体例：机制被证伪但未被替换时，写成开放问题而非结论
  （本仓计划文档的惯用体例）。
- 逃生门（`SDL.UseNativeFileDialog=0`）写在配置项层面——修不了的时候给用户
  一条可执行的退路。
