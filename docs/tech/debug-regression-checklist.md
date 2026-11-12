# 调试器手工回归验收清单（A1 门控三合一）

> **STATUS: ACTIVE**（2026-11-06 建档）
> **来源**：v2.0.2 计划 §十 r5 记录的「A1 未闭合项」——手工九场景待用户 GUI 实测。
> **背景**：A1（提交 `55d82d0`）给 NES 调试器加了**快速门**：零断点时每条 6502 指令
> 不再执行 `DebugCycle()` 的调试开销。**ctest 抓不到这批回归**（见 §三），因此
> 本清单是 A1 验收的**唯一**闭合手段。
> **本文件只描述「怎么点、看到什么算过」，不重复计划的收益读数。**

---

## 一、怎么用这份清单

1. **前置**：用**带调试器的 Release 构建**（`FCEUDEF_DEBUGGER` 全程 define，
   `src/CMakeLists.txt:84`），Qt 驱动。本清单全部是 GUI 交互，命令行跑不了。
2. **每项独立做**：一次只开一个调试特性。理由见 §四——A1 的门是**单条 OR 条件**，
   多个特性同时开会互相掩盖「哪一条把门撑开了」。
3. **每项记一条**：在 §六的勾选表里填日期、结果、备注。
4. **判红**：任一项红 ⇒ 记 §五要求的失败记录，**不要**自行改代码（改动归主会话决策）。
5. **回写**：本清单每项的通过与否应回写到
   `docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（A1 条目，主会话负责写）。
   回写约定见 §六末尾。

### 术语与门控速查

| 术语 | 含义 | 代码坐标 |
|---|---|---|
| **门（gate）** | `DebugCycle()` 入口的早退条件。门**关** ⇒ 整个调试器每指令逻辑被跳过 | `src/debug.cpp:918` |
| **A1-a** | `DebugCycle()` 入口早退。门 = `breakpoint()` 的 9 路 OR + `debug_loggingCD` + `trace_logging` | `src/debug.cpp:918-919` |
| **A1-b** | `e1_last_pc` 写入移入 `e1_cpu_trace_on()` 门；`map_irq_hook` 热路径 load acquire→relaxed | `src/x6502.cpp:611-614`、`src/cpu.h:140-142` |
| **A1-c** | viewer 回调链门条件改为**窗口状态**（`nt_view_open` / `ppu_view_open` / `trace_logging`），**不是**调试器状态 | `src/driver_callbacks.h:113-126` |
| **9 路 OR** | `numWPs` \| `step` \| `runline` \| `stepout` \| `watchpoint[64].flags` \| `badopbreak` \| `break_on_cycles` \| `break_on_instructions` \| `break_asap` | `src/debug.cpp:918`、`:970` |

> ⚠️ **A1-c 的门条件是窗口状态**。因此「**零断点时打开 viewer 仍必须刷新**」是一条
> 极易漏的项——若照抄计划原文「同门」（用调试器状态当门），开窗就会黑屏。
> 这条单列为 §四的 **V-2**，务必做。

---

## 二、通用准备

### 2.1 测试 ROM

用**任意一个能稳定跑到主循环、且画面有内容**的 NES ROM。本仓库的 fixture 生成器
在 `tests/fixtures/generate_test_roms.py`。**注意**：该脚本生成的 PRG 是
`0xEA` NOP sled（不写任何 APU 寄存器），**音频全程静音**——用它测 NSF/声音类项目
会得到假阴性。若要测声音（§四的 L-1 除外），另找一个真实 ROM。

**记录用到的 ROM 路径**（失败记录里要写）：

```
ROM 路径：D:\Project\FCEUX11\tests\fixtures\nestest.nes（2026-11-13 走查会话；
NSF-1 另用 D:\Project\FCEUX11\tests\fixtures\test_nsf.nsf——其 play 例程为
BRK+NOP sled，静音是夹具构造使然，见 §6.1 NSF-1 行）
```

### 2.2 打开调试器

主窗口菜单：**Advanced → Debug → Debugger...**
（`src/drivers/Qt/ConsoleMenu.cpp:1043-1048` → `ConsoleDebugWindows.cpp:170-186`）

调试器窗口顶部有五个页签（`ConsoleDebugger.cpp:1603/1624`）：**CPU / PPU /
Breakpoints / Bookmarks**（Bookmarks 默认在第二组）。反汇编区在上，寄存器区在右。

### 2.3 「零断点」的确认方式

跑任何 A1-c（viewer）项之前，**必须**确认处于「零断点」状态：

- Breakpoints 页签的列表**为空**（无任何条目）；
- 调试器 Debug 菜单 → Break On... 下的五个勾选项（Bad Opcodes / Unlogged Code /
  Unlogged Data / Cycle Count Exceeded / Instruction Count Exceeded）**全部未勾选**
  （`ConsoleDebugger.cpp:781-841`）；
- 调试器 Debug 菜单 → Step Back（F9）**灰色不可用**——灰色即代表 TraceLogger 没在跑
  （`ConsoleDebugger.cpp:4501-4510` 判 `FCEUD_TraceLoggerRunning()`）。

---

## 三、为什么这批项必须手工（不可自动化）

`ctest` 的 33 项门禁全部是**无头进程**：跑 `.exe`、比对哈希或内存快照，**不建 Qt
窗口、不派发键盘/鼠标事件**。而本清单的每一项都要求「打开某个窗口 → 点某个菜单项
→ 在某个 QLineEdit 里填值 → 观察某个 QLabel 的文本」。这些交互**在 headless 下
无法驱动**：

1. **菜单与对话框只在 GUI 路径被构造**。`ConsoleDebugger::buildMenuBar()`
   （`ConsoleDebugger.cpp:423-950`）与 `DebuggerBreakpointEditor`（`:1874-2090`）
   只在 `ConsoleDebugger` 构造函数里被调用，而 `ConsoleDebugger` 只由
   `consoleWin_t::openDebugWindow()`（`ConsoleDebugWindows.cpp:170`）创建——
   后者由 Qt 菜单动作 `debuggerAct`（`ConsoleMenu.cpp:1043-1046`）触发。
   **无头路径从不进入这条链**，所以 `DebuggerBreakpointEditor` 的字段校验
   （`checkDataValid`，`:2148-2213`）、`DebugBreakOnDialog` 的阈值换算
   （`closeWindow`，`:8104-8156`）在 ctest 里**一次都没被执行过**。

2. **`break_asap` 走 Lua 回调 → 真实事件循环**。`debugger.hitbreakpoint()`
   （`src/rust/crates/fceux11-lua/src/bindings/debugger.rs:15-21`）最终调
   `fceux11_lua_debugger_hitbreakpoint()`（`src/lua-engine.cpp:969-971`）置
   `break_asap = true`。**这个标志只能被 `breakpoint()` 消费**
   （`src/debug.cpp:659-663`），而 `breakpoint()` 只在 `DebugCycle()` 里被调
   （`:970-971`）——**即必须有一条真实的 6502 指令流在跑**。脚本侧还要靠
   `emu.frameadvance()` 让出主协程、由模拟线程在帧边界
   （`fceu.cpp:869` → `fceux11_lua_frame_boundary()`）续跑，
   这条「模拟线程 ↔ Lua 协程」的往返在 ctest 里没有对应物。

3. **三个 viewer 的刷新是「窗口开了没有」决定的**，而窗口的存在性由 Qt 窗口
   生命周期决定：`nt_view_open` 在 `ppuNameTableViewerDialog_t` 构造时置 1
   （`NameTableViewer.cpp:172`）、析构时置 0（`:606`）。这个状态位**没有 ctest
   覆盖**——无头测试用 `fceu11::register_driver(fceu11::DriverCallbacks{})`
   注册**全空**回调表（`src/f11qa_bridge.cpp:40-43`，注释原文「Register null
   driver callbacks (headless — no Qt, no GUI)」），
   `update_nt_view` / `update_ppu_view` 在 ctest 里**恒为 nullptr**。

4. **NSF 播放是 GUI 侧的独立加载路径**：`File → Play NSF`
   （`ConsoleMenu.cpp:199-204` → `ConsoleFile.cpp:311`）弹出 `QFileDialog` 选
   `.nsf` 文件，`NSFGI(GI_POWER)` 才调 `NSF_init()`（`nsf.cpp:82-85`）。
   NSF 的帧驱动还与 `GameInfo->type == GIT_NSF` 的分支耦合
   （`ppu_rendering.cpp:1227-1228`、`debug.cpp:931-934`），**无头路径不进这条链**。

**结论**：ctest 能证明「NES 侧数值没回归」（金标哈希、golden savestate），
**不能**证明「调试器还能用」。这两件事互补，不重叠。

---

## 四、逐项清单

> 每项格式：**步骤**（照着点）→ **判据**（绿/红）→ **A1 关联**（哪条线、门开没开）。

### 4.1 断点类（A1-a）

#### B-1　普通断点（Breakpoints 列表 / Add）

**步骤**

1. 载入测试 ROM（§2.1），按 F5 或点 Run 让它跑起来。
2. `Advanced → Debug → Debugger...` 打开调试器。
3. 点 Debug 菜单 → **Run**（F5）让模拟继续跑。
4. 在反汇编区找到一条已知指令（建议选主循环里的一条 `LDA`/`NOP`），记下它的地址。
   - 地址来源不确定时：用 View → **Go to Address**（Ctrl+A）输入已知地址跳转。
5. 点 Breakpoints 页签 → 点 **Add** 按钮 → 弹出「Add Breakpoint」对话框。
6. **Address** 填上一步记的地址（十六进制，**不带 `$`**，如 `8000`）；
   **Memory** 组勾 **Execute**；其余（Read / Write / Enable）按需，**Enable 必须勾上**；
   **Condition** 留空；点 **OK**。
7. 点 Debug 菜单 → **Run**（F5）。

**判据**

- **绿**：模拟**在第 4 步那个地址停下**（反汇编区 PC 行高亮跳到该行），
  调试器状态标签显示 **"Emulator Stopped / Paused at Breakpoint"**，
  寄存器区显示的是该指令处的寄存器值。
- **红**：不停；或状态标签停在 "Emulator is Running"；
  或停在**别的**地址（说明门开得不对/地址算错）。

**A1 关联**：A1-a。门由 `numWPs`（`src/debug.cpp:918`）撑开——
设了断点 `numWPs` 递增（`ConsoleDebugger.cpp:2353`），门开，`breakpoint()`
被调（`:970-971`），命中后 `BreakHit()` 暂停（`debug.cpp:636-643`）。

**注意**：`Address` 字段的十六进制解析走 `offsetStringToInt()`
（`debug.cpp:38-117`），它**认符号名与关键字**（`NMI`/`IRQ`/`RST`/`VBL`），
也认符号表里的名字。填错格式会静默得到 `conversionOk=false`，
对话框会显示错误消息——**填地址时只填纯十六进制数字**。

---

#### B-2　watchpoint（内存读/写断点）

**步骤**

1. 同 B-1 步骤 1-3（ROM 已跑、调试器已开、模拟在跑）。
2. Breakpoints 页签 → **Add**。
3. **Address** 填一个会被频繁访问的 RAM 地址（**建议 `$0000`-`$001F`** 这段
   零页，或 `$0100`-`$01FF` 栈区；填 `0000` 便于记忆）。
4. Memory 组勾 **Read**（和/或 **Write**），**Enable** 勾上。
5. 点 **OK**，然后 Debug → **Run**（F5）。

**判据**

- **绿**：模拟在**很短时间内**停住（因为选了高频地址），状态标签显示
  "Emulator Stopped / Paused at Breakpoint"（编号为该断点在列表中的序号）。
- **红**：永不停（⇒ 门没被 `numWPs` 撑开，或读断点判定被跳过）。

**A1 关联**：A1-a，门由 `numWPs` 撑开。命中判定在 `breakpoint()` 的
CPU mem 分支（`debug.cpp:803-806`，`watchpoint[i].address == A`）。

**补充读数**：断点列表的 **Flags** 列会显示 `E`（Enable）、`R`（Read）、
`W`（Write）、`X`（Execute）——用来确认勾选真的落库
（渲染于 `ConsoleDebugger.cpp:2455-2483`）。

---

#### B-3　Break on PC（断在指定 PC）

> ⚠️ **当前代码中无名为 "Break on PC" 的菜单项**（全仓 grep 零命中；
> `git log --all -S "Break on PC"` 只命中计划文档本身，见 §七）。
> r5 里这一项对应的是**「在一个指定 PC 地址上打断点」**这一功能，
> 实际入口有两个，都测：

**入口 (a)：Breakpoints 页签 → Add → Execute（等价于 break on PC）**

1. 同 B-1 步骤 1-3。
2. Breakpoints 页签 → **Add** → **Address** 填 `8000`（或你 ROM 里确知会被执行到
   的地址）；Memory 组只勾 **Execute** + **Enable**；**OK**。
3. Debug → **Run**（F5）。

**入口 (b)：反汇编区右键 → "Run To &Cursor"（运行到光标处）**

1. 同 B-1 步骤 1-3。
2. 在反汇编区**右键**点击目标那一行 → 上下文菜单选 **Run To &Cursor**。
   （菜单项仅在光标行是合法地址时出现，`ConsoleDebugger.cpp:6396-6402`。）
3. 观察。

**判据**

- **绿**：模拟**恰好停在那个地址**（PC 行高亮到该行），寄存器区 PC 值 == 设定地址。
- **红**：停不到（⇒ `watchpoint[64].flags` 这条门没撑开，或地址从没被执行）；
  或停在邻近地址。

**A1 关联**：A1-a。入口 (a) 靠 `numWPs`；入口 (b) 靠
`watchpoint[64].flags`（`debug.cpp:918`）。入口 (b) 走的是
`watchpoint[64]` 这个**保留槽**（`debug.h:68` 注释「+ 1 reserved for step over」），
命中后由 `debug.cpp:709-714` 清槽并停下。

---

#### B-4　Bad Op（非法opcode 断点）

**步骤**

1. 同 B-1 步骤 1-3。
2. 调试器 **Debug 菜单 → Break On... → Bad Opcodes**（勾选）。
3. Debug → **Run**（F5），跑几秒。

**判据**

- **绿**：如果你的 ROM 真的会执行到非法 opcode，模拟停下，状态标签显示
  **"Emulator Paused on Bad Opcode"**。
- **判定说明**：**正常 ROM 不会触发**，所以「不停」在正常 ROM 上是**正确**的，
  不算红。要真正验证这条，需要一个会跑非法 opcode 的 ROM。
  **若手上没有这样的 ROM，把本项标为「未验证」而不是「红」**——
  这是一条「需要特殊夹具」的项。

**A1 关联**：A1-a，门由 `dbgstate.badopbreak` 撑开（`debug.cpp:918`）。
判定在 `breakpoint()`（`:671-672`，条件 `size == 0` 即非法 opcode）。
注意 `size` 来自 `opsize[opcode[0]]`（`:937`），这个查表在门**之内**。

**顺带确认**：再把 Bad Opcodes 取消勾选，Debug → Run，
确认状态标签回到 "Emulator is Running"（取消勾选会走
`breakOnBadOpcodeCB(false)`，`ConsoleDebugger.cpp:2775-2781`）。

---

#### B-5　cycles 超限（Cycle Count Exceeded）

**步骤**

1. 同 B-1 步骤 1-3。
2. 调试器 **Debug 菜单 → Break On... → Cycle Count Exceeded**（勾选）。
   弹出「**Break on CPU Cycle Exceedance**」对话框
   （`DebugBreakOnDialog`，`ConsoleDebugger.cpp:7876`）。
3. 对话框里：
   - **Mode** 组选 **Continuous**（连续模式；要一次性的选 One-Shot）；
   - **Reference** 组选 **Absolute**（绝对）或 **Relative**（相对）；
   - **Threshold:** 输入框填一个**小的**阈值，例如 `100000`。
     （点 `+1k` / `+1m` 按钮也能调，`:7990-8049`。）
   - 点 **Ok**。
4. Debug → **Run**（F5）。

**判据**

- **绿**：模拟在跑过设定阈值后停下，状态标签显示
  **"Emulator Paused on Cycle Count Exceedance"**；
  调试器 **CPU** 页签的 **CPU Cycles:** 字段显示的累计值 ≥ 你设的阈值。
- **红**：不停；或状态标签显示的是别的暂停原因。

**A1 关联**：A1-a，门由 `break_on_cycles` 撑开（`debug.cpp:918`）。
判定在 `breakpoint()`（`:665-666`）。**注意阈值语义**：
`Absolute` 模式下 `break_cycles_limit = threshold`（`:8132`），
`Relative` 模式下换算成 `totalCount + (relVal - deltaCount)`（`:8138`）——
两种模式的**触发时机不同**，测的时候别混。

**顺带确认**：CPU 页签的 **CPU Cycles:** 字段（`ConsoleDebugger.cpp:1228/1255`）
在跑动时**应持续增长**。若它卡住不动 ⇒ 计数器被误伤（见 §4.5 的 A1-b 反向项）。

---

#### B-6　instructions 超限（Instruction Count Exceeded）

**步骤**

1. 同 B-1 步骤 1-3。
2. **Debug → Break On... → Instruction Count Exceeded**（勾选）。
   弹出「**Break on CPU Instruction Exceedance**」对话框。
3. **Threshold:** 填 `100000`（小阈值），**Mode** 选 Continuous，
   **Reference** 选 Absolute，点 **Ok**。
4. Debug → **Run**（F5）。

**判据**

- **绿**：停下，状态标签 **"Emulator Paused on Instruction Count Exceedance"**；
  CPU 页签 **Instructions:** 字段的值 ≥ 阈值。
- **红**：不停。

**A1 关联**：A1-a，门由 `break_on_instructions` 撑开（`debug.cpp:918`）。
判定在 `breakpoint()`（`:667-668`，判 `total_instructions > break_instructions_limit`）。

---

### 4.2 单步类（A1-a）

> **共同前置**：模拟**已暂停**（调试器打开时通常自动暂停；否则按 Pause F6）。
> 单步动作都在**调试器窗口**里做，且窗口要有焦点。

#### S-1　Step Into（F11）

**步骤**

1. 同 B-1 步骤 1-2（ROM 已跑、调试器已开）。
2. 确认模拟处于**暂停**（状态标签 "Emulator Stopped / Paused"）。
3. 记下当前 **PC** 值（寄存器区）。
4. 按 **F11**（或 Debug 菜单 → **Step &Into**）。**重复 3 次**。

**判据**

- **绿**：每按一次 F11，**PC 前进一条指令**（连续按 3 次，PC 走 3 步而非原地不动），
  反汇编区的 PC 高亮行同步下移。
- **红**：按 F11 后 PC **不动**（⇒ `dbgstate.step` 这条门没撑开，
  或 `breakpoint()` 的 step 分支没跑到）。

**A1 关联**：A1-a，门由 `dbgstate.step` 撑开（`debug.cpp:918`）。
置位在 `debugStepIntoCB`（`ConsoleDebugger.cpp:3061-3069`），
消费在 `breakpoint()`（`:689-693`）。

#### S-2　Step Over（F10）

**步骤**

1. 同 S-1 步骤 1-2。
2. 把光标/PC 移到一条 **JSR（$20）** 指令上——Step Over 对 JSR 有专门路径。
   （在反汇编区找 `JSR` 字样；若 ROM 里没有 JSR，退而选任意非 JSR 指令。）
3. 按 **F10**（或 Debug → **Step &Over**）。

**判据**

- **绿**：模拟停下，且 PC 落在 **JSR 的返回地址之后**（即跳过了子程序体）。
  非 JSR 指令时，PC 前进一条指令。
- **红**：不停；或 PC 停进了 JSR 的**子程序内部**（说明走的是 Step Into 路径）；
  或弹出 "Step Over is currently in process." 的提示
  （`ConsoleDebugger.cpp:3124-3128`，说明上一次的 Step Over 残留没收尾）。

**A1 关联**：A1-a。Step Over 对 JSR 走 **`watchpoint[64]` 保留槽**
（`ConsoleDebugger.cpp:3129-3130`），门由 `watchpoint[64].flags` 撑开；
非 JSR 时走 `dbgstate.step`（`:3134`）。

#### S-3　Step Out（Shift+F11）

**步骤**

1. 同 S-1 步骤 1-2。
2. 把 PC 移到**子程序内部**（一个 JSR 已经进去、还没出来的位置）。
3. 按 **Shift+F11**（或 Debug → **&Step Out**）。

**判据**

- **绿**：模拟停下，且 PC 落在**调用者的下一条指令**（即已从子程序返回）。
- **红**：不停；或弹出 "Step Out Already Active" 对话框
  （`ConsoleDebugger.cpp:3080-3082`，说明上一次 Step Out 没结束）。

**A1 关联**：A1-a，门由 `dbgstate.stepout` 撑开（`debug.cpp:918`）。
嵌套层数靠 `dbgstate.jsrcount` 计数（`debug.cpp:675-686`）：
JSR 加一、RTS 减一，归零时切到 `dbgstate.step` 停下。

#### S-4　Run Line（F7） / Run 128 Lines（F8）

**步骤**

1. 同 S-1 步骤 1-2。
2. 按 **F7**（或 Debug → **Run &Line**）。
3. 重复步骤 2 三次；再按一次 **F8**（Debug → **Run &128 Lines**）。

**判据**

- **绿**：每按一次 F7，扫描线**前进一条**（调试器 PPU 页签的
  **Scanline** 字段变化，字段定义在 `ConsoleDebugger.cpp:1412`）；
  F8 一次前进 128 条。
- **红**：不停；或扫描线**不动**（⇒ `dbgstate.runline` 这条门没撑开）。

**A1 关联**：A1-a，门由 `dbgstate.runline` 撑开（`debug.cpp:918`）。
置位在 `debugRunLineCB`（`ConsoleDebugger.cpp:3159-3173`），
判定在 `breakpoint()`（`:696-706`，按时间戳比较 `runline_end_time`）。

---

### 4.3 日志器类（A1-a 的 `debug_loggingCD` / `trace_logging` 两条腿）

#### L-1　Code/Data Logger（CD Logger）

**步骤**

1. 载入测试 ROM，跑起来。
2. `Advanced → Debug → Code/Data Logger...`（`ConsoleMenu.cpp:1091-1096`）。
3. 窗口标题「**Code Data Logger**」，中间有 **Start** 按钮（`CodeDataLogger.cpp:236`）。
4. 点 **Start** → 按钮变 **Pause**，状态标签变
   "Logger is Running: Press Pause to Stop"（`CodeDataLogger.cpp:379-380`）。
5. 让它跑 **10 秒以上**。
6. 看 **Code/Data Log Status** 组的六个读数（`CodeDataLogger.cpp:166-171`）：
   PRG Logged Code / PRG Logged Data / PRG not Logged / CHR Logged Code /
   CHR Logged Data / CHR not Logged。
7. 点 **Pause** → 状态标签回 "Logger is Paused: Press Start to Run"。

**判据**

- **绿**：跑动期间六个百分比读数**从 0% 上升**（PRG Logged Code 尤其明显——
  主循环代码必然被取指）；暂停后读数冻结。
- **红**：读数**始终 0.00%**（⇒ `LogCDData` 没被调到，
  即 `debug_loggingCD` 这条门没撑开）；或按钮点了没反应。

**A1 关联**：A1-a 的 `debug_loggingCD` 腿（`debug.cpp:918`）。
`LogCDData()` 在门**之内**被调（`:973-974`）。
置位在 `StartCDLogging()`（`CodeDataLogger.cpp:920-928`，走 `FCEUI_SetLoggingCD(1)`）。

#### L-2　Trace Logger

**步骤**

1. 载入测试 ROM，跑起来。
2. `Advanced → Debug → Trace Logger...`（`ConsoleMenu.cpp:1083-1088`）。
3. 窗口标题「**Trace Logger**」。点 **Start Logging** 按钮
   （`TraceLogger.cpp:253`），按钮应变 **Stop Logging**。
4. 在 **Log Options** 组里勾上 **Log State of Registers** 与
   **Log Instructions Count**（`TraceLogger.cpp:302/311`）。
5. 让它跑 **5 秒以上**，看下方日志列表区。
6. 点 **Stop Logging**。

**判据**

- **绿**：日志列表**持续出现新行**（不是空白），且行内有反汇编文本；
  勾了 "Log Instructions Count" 时能看到指令计数列。
- **红**：日志区**空白**或**不再增长**（⇒ `trace_logging` 这条门没撑开）。

**A1 关联**：A1-a 的 `trace_logging` 腿（`debug.cpp:918`）。
Qt 侧在 `toggleLoggingOnOff()` 里把开关镜像到 core 侧状态位
`g_driver().trace_logging`（`TraceLogger.cpp:589`）——
**这是 A1-c 新增的三个状态位之一**（`driver_callbacks.h:124`）。

> ⚠️ **这条是 A1-a 与 A1-c 的交界**：core 侧的门读 `g_driver().trace_logging`，
> Qt 侧写的是 `logging` 局部变量（`TraceLogger.cpp:97`）。两者靠
> `TraceLogger.cpp:589` 这一行保持同步。**这一行若漏了，Trace Logger
> 会「按了 Start 但没记录」**——而这**不是** ctest 能抓的。

**顺带确认（Step Back 依赖同一条记录流）**：在 Trace Logger **窗口开着**的情况下，
调试器 Debug 菜单 → **Step &Back**（F9）应当**变灰**（未暂停时禁用，
`ConsoleDebugger.cpp:4501-4510`）。若开着 Trace Logger 但 Step Back **始终灰**，
说明 `FCEUD_TraceLoggerRunning()` 没真起来（`TraceLogger.cpp:1086-1089`）。

#### L-3　Code/Data Logger 与 Trace Logger **同时**开

**步骤**

1. 同 L-1 步骤 1-2，**Code/Data Logger 保持 Start 状态**（不暂停）。
2. 再 `Advanced → Debug → Trace Logger...` → 点 **Start Logging**。
3. 跑 5 秒，看两个窗口的输出。

**判据**

- **绿**：两个窗口**都**有输出（CD 百分比在涨、Trace 日志在长行）。
- **红**：只有一个有输出（说明两个 logger 抢同一个门，互相把对方挤掉——
  门是单条 OR，两条腿都撑开时**应该**都能工作）。

**A1 关联**：A1-a 的两条腿是 **OR 关系**，不是互斥。
这条专门验「OR 语义」没被写成「互斥」。

---

### 4.4 viewer 刷新类（A1-c —— 门条件是**窗口状态**，不是调试器状态）

> ⚠️ **本节是整份清单里最容易漏的一节。** A1-c 的门条件**不是**「调试器有没有在活动」，
> 而是「**窗口开没开**」。零断点时打开 viewer，**必须仍然刷新**。
> 若把门写成调试器状态 ⇒ 开窗即黑屏 ⇒ 整项红。

#### V-1　Name Table Viewer 在**零断点**下刷新

**步骤**

1. 载入测试 ROM，跑起来（画面应有内容）。
2. **先不要**打开调试器，也**不要**设任何断点（保持 §2.3 的「零断点」）。
3. `Advanced → Debug → Name Table Viewer...`（`ConsoleMenu.cpp:1075-1080`）。
4. 窗口标题「**Name Table Viewer**」。等 **2 秒以上**（内部 30Hz 定时器，
   `NameTableViewer.cpp:582-586`）。
5. 看窗口里的 name table 图块区。

**判据**

- **绿**：name table **图块内容随 ROM 运行而变化**（或至少**不是全黑/全空**——
  静止画面也合法，但**必须画出图块**，不能是一块黑）。
- **红**：图块区**全黑/全空**，且在 ROM 明显在动画面时**始终不变**
  （⇒ `nt_view_open` 没置位，或门误用了调试器状态）。

**A1 关联**：A1-c。`nt_view_open` 在窗口构造时置 1（`NameTableViewer.cpp:172`）、
析构时置 0（`:606`）；core 侧在 `ppu_rendering.cpp:793` 读它：
`DEBUG(if (fceu11::g_driver().nt_view_open) FCEUD_UpdateNTView(sl, 0));`。

**关键读数**：这条的**唯一正确实现**就是「窗口开了 → 刷新」。
任何「零断点所以不刷新」的实现都是错的。

> ⚠️ **PPU 引擎的选择会改变本项测的是哪条路径。**
> `nt_view_open` 的**唯一** core 侧读点是 `ppu_rendering.cpp:793`，它在
> `DoLine()` 里，而 `DoLine()` **只被旧 PPU 主循环调用**
> （`ppu_rendering.cpp:1321`，即 `FCEUPPU_Loop`）。
> 新 PPU 主循环（`FCEUX_PPU_Loop`，`ppu_rendering.cpp:1557`）走
> `ppu_rendering.cpp:1778-1779` 的 `FCEUD_UpdatePPUView` / `FCEUD_UpdateNTView`
> ——**那两处没有任何门**。
> `SDL.NewPPU` 默认 **0**（旧 PPU，`src/drivers/Qt/config.cpp:884`），
> 所以**默认配置下本项测的正是被门控的那条路径**。
> ⇒ **跑本项前不要改 PPU 引擎设置**（`Options` 里的新 PPU 开关）；
> 若你的配置已开了新 PPU，本项**测不到 A1-c 的门**，请在备注里写明。

#### V-2　PPU Viewer 在**零断点**下刷新

**步骤**

1. 同 V-1 步骤 1-2。
2. `Advanced → Debug → PPU Viewer...`（`ConsoleMenu.cpp:1059-1064`）。
3. 窗口标题「**PPU Viewer**」，左上是 pattern table（背景图块），右侧是精灵区。
4. 等 2 秒以上，看 pattern table 是否随游戏更新。

**判据**

- **绿**：pattern table **有内容且会变**（游戏在改 CHR 时能看出来）。
- **红**：全黑/空白且不变。

**A1 关联**：A1-c。`ppu_view_open` 由 `sync_ppu_view_flags()`
（`ppuViewer.cpp:60-64`）在 PPU 窗口与 Sprite 窗口的构造/析构处维护，
合并表示「两个窗口任一开着」。

#### V-3　Sprite Viewer 在**零断点**下刷新

**步骤**

1. 同 V-1 步骤 1-2。
2. `Advanced → Debug → Sprite Viewer...`（`ConsoleMenu.cpp:1067-1072`）。
3. 等 2 秒以上，看精灵/OAM 区。

**判据**

- **绿**：OAM 区**有精灵数据**（或随游戏变化）。
- **红**：空白且不变。

**A1 关联**：A1-c，与 V-2 共用 `ppu_view_open`（`ppuViewer.cpp:62` 把两个窗口
指针合并成一个状态位；Sprite 窗口的析构在 `ppuViewerSpriteViewer.cpp:285-295`）。

#### V-4　viewer 与调试器**同时**开（零断点）

**步骤**

1. 同 V-1 步骤 1-2，但**同时**把调试器窗口也开着
   （`Advanced → Debug → Debugger...`），**不设任何断点**。
2. Name Table Viewer / PPU Viewer 保持打开。
3. 跑 5 秒，观察两个 viewer 是否仍在刷新。

**判据**

- **绿**：viewer **照常刷新**（调试器开着不影响它）。
- **红**：viewer 卡死（⇒ 门被「调试器窗口开着」这种状态污染了）。

**A1 关联**：A1-c。**这一项专门防「门条件混用」**：
调试器窗口开着 ≠ 调试器在活动。门必须只看**viewer 窗口自己**的状态。

---

### 4.5 Lua 与 NSF 类

#### LUA-1　Lua `debugger.hitbreakpoint()`（break_asap）

**步骤**

1. 载入测试 ROM，跑起来。
2. 准备一个 Lua 脚本文件（内容如下，存成 `bp_test.lua`）：

   ```lua
   while true do
       emu.frameadvance()
       debugger.hitbreakpoint()
   end
   ```

3. 主窗口 **File → Load &Lua Script**（`ConsoleMenu.cpp:302-310`，
   弹「Lua Script Control」窗口）。
4. **Browse** 选中 `bp_test.lua` → 点 **Start**。
5. 等 1-2 秒。

**判据**

- **绿**：模拟**停下**，调试器状态标签显示
  **"Emulator Paused on Lua Breakpoint"**
  （`ConsoleDebugger.cpp:4451-4452`，`BREAK_TYPE_LUA = -5`，`debug.h:29`）。
- **红**：不停（⇒ `break_asap` 这条门没撑开，或标志没被消费）。

**A1 关联**：A1-a，门由 `break_asap` 撑开（`debug.cpp:918`）。
`break_asap` **在门里**（r5 记的实施修正①：9 路 OR 实含 `break_asap`）。
标志由 `fceux11_lua_debugger_hitbreakpoint()` 置位（`lua-engine.cpp:969-971`），
由 `breakpoint()` 消费并清零（`debug.cpp:659-663`）。

**注意**：`hitbreakpoint()` 的 Lua 名是 **`debugger.hitbreakpoint()`**
（`src/rust/crates/fceux11-lua/src/bindings/debugger.rs:15-21`）。
r5 文案里写的 `break_asap` 是 **core 侧的 C++ 变量名**，不是 Lua API 名。

#### LUA-2　Lua `debugger.getinstructionscount()` 必须**持续增长**（A1-b 反向项）

> **这是 A1-b 特意留的反向项。** r5 记录：「`IncrementInstructionsCounters`
> **不门控**——实施核对发现 `lua getTotalInstructions` 与 TraceLogger `instrCount`
> 在零调试特征时也消费，门控=读数停滞，故保留每指令记账」。
> **换句话说：如果有人日后把 `IncrementInstructionsCounters` 也塞进门里，
> 这条会立刻转红。**

**步骤**

1. 载入测试 ROM，跑起来。
2. 准备 `instr_test.lua`：

   ```lua
   local a = debugger.getinstructionscount()
   while true do
       emu.frameadvance()
       local b = debugger.getinstructionscount()
       emu.print(string.format("instr: %d -> %d (delta %d)", a, b, b - a))
       a = b
   end
   ```

3. **File → Load &Lua Script** → 选中该文件 → **Start**。
4. 看 **Output Console**（Lua Script Control 窗口的输出区）里连续几行读数。
5. 再看调试器 **CPU** 页签的 **Instructions:** 字段（若开了调试器）。

**判据**

- **绿**：每帧的 `delta` **> 0 且持续增长**（不是恒为 0）；
  调试器 CPU 页签的 **Instructions:** 字段**也在持续增长**。
- **红**：delta **恒为 0**，或 Instructions 字段**卡住不动**
  （⇒ 指令计数器被误门控了）。

**A1 关联**：A1-b 的**反向验证**。`IncrementInstructionsCounters()`
（`debug.cpp:602-606`）在 `src/x6502.cpp:608` 被调，**在门之外**——
它不在 `DebugCycle()` 里。Lua 侧读取走
`fceux11_lua_debugger_get_instructions_count()`（`lua-engine.cpp:982-984`）
→ `total_instructions`。

**顺带确认**：TraceLogger 的 **Log Instructions Count**
（`TraceLogger.cpp:311`）也是同一个计数器（`trace_common.cpp:436`
读 `total_instructions`）——L-2 里勾上它时也应看到计数在涨。

#### NSF-1　NSF 播放

**步骤**

1. 主窗口 **File → Play &NSF**（`ConsoleMenu.cpp:199-204`）。
2. 弹出「Load NSF File」对话框（`ConsoleFile.cpp:318`），选一个 `.nsf` 文件。
3. 等 2 秒以上。

**判据**

- **绿**：**能听到声音**（NSF 的主功能）；屏幕上出现 NSF 播放器
  的可视化画面（音频波形柱 + 曲名/作者/曲目信息，来自 `nsf_ui.cpp:31-127`）。
- **红**：无声；或画面全黑无信息。

**A1 关联**：A1-a 的**间接**关联——NSF 播放走的是自己的 6502
（`NSF_init()`，`nsf.cpp:82-85`），但**共用同一条 `X6502_Run` →
`DebugCycle()` 路径**。`debug.cpp:931-934` 对 NSF 有专门分支
（`_PC` 在 `0x3801`-`0x3824` 时直接 return，避开 NSF 初始化代码）。
这条分支在门**之内**，所以门关时它也不执行——**但它本来也不产生任何副作用**，
所以 NSF 播放不该受影响。**本项就是为了确认这一点。**

**替代验证（无 .nsf 文件时）**：也可以用 NSF 的**调试器断点关键字**
验证同一分支未被破坏——在 Breakpoints 页签 → Add → **Address** 填
`PLAY`（`debug.cpp:95` 认这个关键字，展开为 NSF 头里的 Play 地址），
Memory 勾 Execute + Enable，OK 后 Run。**能停在 PLAY 地址**即说明
NSF 断点路径完好。

---

## 五、失败时必须记录什么

任一项判红，**必须**记录以下五项（缺一项就无法复现）：

| # | 记录项 | 说明 |
|---|---|---|
| 1 | **症状** | 实际看到的现象，原样抄界面文字。例如「按 F11 三次，PC 停在 $8010 不动」。 |
| 2 | **复现步骤** | 从 §4 的步骤编号起写，例如「B-1 步骤 1-7」。不要重写成叙述。 |
| 3 | **ROM / 参数** | ROM 完整路径（失败记录里要写）；断点地址、阈值、条件表达式等**实际填入的值**。 |
| 4 | **环境** | 构建类型（Release）、提交号（`git rev-parse --short HEAD`）、Qt 版本。 |
| 5 | **门控状态** | 失败时的调试器状态：Breakpoints 列表内容、Break On... 五个勾选项状态、Trace Logger / CD Logger 是否在跑、viewer 窗口是否开着。 |

**填写模板**：

```
项号：______    日期：__________    结果：□绿  □红  □未验证

症状：
（抄界面上的实际文字）

复现步骤：
（§4 的步骤编号 + 你的实际操作）

ROM：____________________  参数：____________________

环境：commit __________  构建 Release  Qt ______

门控状态：
  Breakpoints 列表：____________________
  Bad Opcodes      ：□勾 □未勾
  Unlogged Code    ：□勾 □未勾
  Unlogged Data    ：□勾 □未勾
  Cycle Count Exc. ：□勾 □未勾
  Instr Count Exc. ：□勾 □未勾
  Trace Logger     ：□开 □关
  CD Logger        ：□开 □关
  NT Viewer        ：□开 □关
  PPU/Sprite Viewer：□开 □关
```

> **「未验证」是合法结论**，但必须写清**缺什么**（例如「无非法 opcode ROM」）。
> 把「没条件测」记成「绿」是撒谎；把「没条件测」记成「红」是误报。

---

## 六、勾选表

### 6.1 逐项记录

| # | 项 | 场景（对应 r5 九场景） | 关键判据（一句话） | 日期 | 结果 | 备注 |
|---|---|---|---|---|---|---|
| B-1 | 普通断点 | watchpoint / 断点 | 在设定地址停下，标签 "Paused at Breakpoint" | 2026-11-13 | **绿** | C28F 命中，"Paused at Breakpoint: 0"，列表 EC--X--（ZCode 代走查） |
| B-2 | watchpoint（读/写） | watchpoint | 高频地址上很快停住 | 2026-11-13 | **绿** | 01FD R+W 命中（0000 在该 ROM 循环未被访问，换栈址）；Flags=ECRW-- |
| B-3a | Break on PC（Add+Execute） | Break on PC | 恰好停在设定 PC | 2026-11-13 | **绿** | 纯 Execute C28F 精确命中（A 寄存器新值证新命中） |
| B-3b | Break on PC（Run To Cursor） | Break on PC | 右键运行到光标处停下 | 2026-11-13 | **绿** | 右键 C291 → 恰停光标行；断点表空 = watchpoint[64] 保留槽 |
| B-4 | Bad Opcodes | Bad Op | 需非法 opcode ROM，否则记「未验证」 | 2026-11-13 | **未验证** | nestest 勾选跑 5s 不停（无误触发=正确，负半验证绿）；正向缺非法 opcode ROM |
| B-5 | Cycle Count Exceeded | cycles 超限 | 停下，标签 "Cycle Count Exceedance" | 2026-11-13 | **绿** | 连续+绝对 100000，「模拟器因周期计数超限暂停」，周期字段高亮 2392684496 |
| B-6 | Instruction Count Exceeded | instructions 超限 | 停下，标签 "Instruction Count Exceedance" | 2026-11-13 | **绿** | 「模拟器因指令计数超限暂停」，指令字段高亮 799083789 |
| S-1 | Step Into（F11） | Step Into | PC 每次前进一条指令 | 2026-11-13 | **绿** | ×4 每步恰 +1 指令（799083792→795），PC 循环内推进 |
| S-2 | Step Over（F10） | Step Over | JSR 时跳过子程序体 | 2026-11-13 | **红** | JSR@C07E 三次激活均不停在返回地址 C081（直达主循环）；隔离态（零断点）15s 无停机。子程序有 RTS（ROM 反汇编实证）。**按 §6.3 处理，见 v2.0.4 计划 r4** |
| S-3 | Step Out（Shift+F11） | Step Out | 从子程序返回到调用者之后 | 2026-11-13 | **绿** | NMI 处理器内步出：RTI+外层 RTS → 停 C084=调用者下一条（+74 指令与处理器长度吻合） |
| S-4 | Run Line / Run 128 Lines | Run Line | 扫描线按 1 / 128 前进 | 2026-11-13 | **绿** | F7×3 各+1（243→246），F8 一次 +128 跨帧回绕精确（246+128 mod 262=112） |
| L-1 | Code/Data Logger | CD Logger | 六个百分比读数从 0% 上升 | 2026-11-13 | **绿** | PRG Logged Code 0→1.01%、Data 0.02%；暂停后 3s 零变化 |
| L-2 | Trace Logger | TraceLogger | 日志区持续出现新行 | 2026-11-13 | **绿** | 行持续增长（计数列+寄存器+反汇编），1,000,000 行，按钮翻转停止记录 |
| L-3 | CD Logger + Trace Logger 同时 | CD Logger + TraceLogger | 两个窗口**都**有输出 | 2026-11-13 | **绿** | 双开：CD 重置归零后在 trace 开启下 4s 重涨至 1.01%（OR 语义无互斥） |
| V-1 | NT Viewer（零断点） | viewer 刷新 | 图块**有内容**，不黑屏 | 2026-11-13 | **绿** | 图块完整渲染（无调试器、旧 PPU 门控路径 = A1-c 核心项） |
| V-2 | PPU Viewer（零断点） | viewer 刷新 | pattern table 有内容 | 2026-11-13 | **绿** | 图案表 0 字体图块完整 |
| V-3 | Sprite Viewer（零断点） | viewer 刷新 | OAM 区有数据 | 2026-11-13 | **绿** | nestest 不写 OAM，空 OAM 是夹具真实状态；查看器在渲染（槽位0/信息面板/调色板） |
| V-4 | viewer + 调试器同时开 | viewer 刷新 | viewer 照常刷新 | 2026-11-13 | **绿** | 三 viewer 调试器同开照常渲染（门污染黑屏故障模式排除） |
| LUA-1 | Lua `hitbreakpoint()` | Lua break_asap | 停下，标签 "Paused on Lua Breakpoint" | 2026-11-13 | **红（双重阻断）** | F-1 复现（abort 0xC0000409，事件日志）后 **V1 已修**（纯 Lua 包装，runner exit 0）；GUI 侧仍被新缺陷阻断（yielding 脚本运行中随机崩溃，见 v2.0.4 计划 r5⑤）；新缺陷关闭后复走 |
| LUA-2 | Lua `getinstructionscount()` | **A1-b 反向项** | 每帧 delta > 0 且增长 | 2026-11-13 | **红（双重阻断）** | 同上（00:47:58）；同上 |
| NSF-1 | NSF 播放 | NSF 播放 | 有声 + 可视化画面 | 2026-11-13 | **未验证（声音侧）** | 可视化绿（Test/Song 渲染）；夹具 PLAY=BRK+NOP sled 静音且 NSF CPU 坠入 $0000 BRK 循环。**另发现：PLAY 关键字被断点对话框判「起始地址无效」**（§4.5 替代验证前提不成立） |

### 6.2 覆盖对照（对照 r5 §十 记录）

r5 原文：「手工九场景（watchpoint / Step Into / Run Line / Step Out / Step Over /
Break on PC / Bad Op / cycles / instructions 超限、CD Logger + TraceLogger、
Lua break_asap、NSF 播放、viewer 刷新）」。

| r5 场景 | 本清单对应项 | 覆盖情况 |
|---|---|---|
| watchpoint | B-2 | ✅ |
| Step Into | S-1 | ✅ |
| Run Line | S-4（F7 / F8） | ✅ |
| Step Out | S-3 | ✅ |
| Step Over | S-2 | ✅ |
| Break on PC | B-3a + B-3b（两个入口都测） | ✅ |
| Bad Op | B-4 | ⚠️ 需非法 opcode ROM，否则记「未验证」 |
| cycles 超限 | B-5 | ✅ |
| instructions 超限 | B-6 | ✅ |
| CD Logger | L-1 | ✅ |
| TraceLogger | L-2 | ✅ |
| CD Logger + TraceLogger | L-3 | ✅ |
| Lua break_asap | LUA-1 | ✅ |
| NSF 播放 | NSF-1 | ✅ |
| viewer 刷新 | V-1 / V-2 / V-3 / V-4 | ✅ |
| **（r5 未列）A1-b 反向项** | LUA-2 | ✅ **本清单新增** |

**合计 22 个可勾选项**，覆盖 r5 全部 15 个场景关键词 + 1 个 A1-b 反向项。

### 6.3 回写约定

**本清单每项的通过与否，应回写到
`docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md` 的 A1 条目**（主会话负责写）。

回写时按下列格式：

- **一项绿**：在 §十 追加一条 rN 记录，写明
  「A1 未闭合项：**手工验收完成**（<日期>，<测试人>）。清单见
  `docs/tech/debug-regression-checklist.md`，22 项中 <绿数> 绿 / <红数> 红 /
  <未验证数> 未验证。<若有未验证项，逐项写明原因>。」
- **一项红**：先按 §五 写失败记录，再在 §十 追加一条 rN 记录，写明
  「A1 未闭合项：**发现回归** —— <项号> <场景>：<症状一句话>。
  复现：<ROM / 参数>。根因待查。」
  **红项不得静默关闭**——A1 的验收判据是「调试功能不回归」，有红项即验收未闭合。
- **多项红**：按不变式 1（NES 零回归）的口径处理——先回退 A1 门控
  （`src/debug.cpp:918`）验证红项是否消失，**再**决定是否修。
  回退验证是一次**变异验证**，结果要一并记进 §十。

**本文件本身不回写 §十**——§十 属计划文档，由主会话统一更新（避免并发冲突）。

---

## 七、读码过程中发现的「入口不存在」与「命名不符」（如实记录）

> 这一节记的是**清单编写时的读码结论**，供后续接手人参考。
> 都不是 A1 引入的缺陷，但会让人按 r5 文案找不到入口。

1. **无 "Break on PC" 菜单项**。全仓 grep `Break on PC` 零命中；
   `git log --all -S "Break on PC"` 只命中计划文档 `892f402` 本身。
   r5 的 "Break on PC" 指的是**「断在指定 PC 地址」这一功能**，
   实际入口是 B-3a / B-3b 两个。**清单已按功能而非文案命名。**

2. **Lua 侧无 `getTotalInstructions` 这个名字**。r5 文案里的
   `lua getTotalInstructions` 是**描述性说法**。当前代码里：
   - core 侧 C++ 全局变量叫 `total_instructions`（`debug.h:115`）；
   - Lua API 名叫 **`debugger.getinstructionscount()`**
     （`src/rust/crates/fceux11-lua/src/bindings/debugger.rs:31-37`），
     C 桥是 `fceux11_lua_debugger_get_instructions_count()`
     （`lua-engine.cpp:982-984`）。
   清单 LUA-2 用的是**实际 API 名**。

3. **Lua 的 `break_asap` 也是 core 侧变量名**，不是 Lua API 名。
   Lua API 是 **`debugger.hitbreakpoint()`**
   （`debugger.rs:15-21`），core 变量 `break_asap` 在 `debug.h:108`。

4. **A1-c 的 `ppu_view_open` 在 core 侧只有「写」没有「读」**。
   写方：`ppuViewer.cpp:62`（`sync_ppu_view_flags()`）。
   读方：全仓 grep `ppu_view_open` 只有 `driver_callbacks.h` 的声明与
   `ppuViewer.cpp:62` 的写——**core 侧没有任何一处读它**。
   实际生效的 viewer 门是 `nt_view_open`（`ppu_rendering.cpp:793`）。
   ⇒ **A1-c 实际只剪掉了 NT viewer 那条链**；PPU/Sprite viewer 的
   `FCEUD_UpdatePPUView` 调用（`ppu_rendering.cpp:1319`、`:1778`）
   **仍是每帧间接调用**。
   ⇒ **V-2 / V-3 的 PPU/Sprite viewer 项因此几乎不可能因 A1 变红**——
   它们测的是「A1 没顺手弄坏 viewer」，而不是「A1 的门在 viewer 上生效」。
   **V-1（NT Viewer）才是真正测 A1-c 门条件的那一项。**

   **补充（引擎相关）**：`nt_view_open` 的唯一 core 侧读点
   （`ppu_rendering.cpp:793`）在 `DoLine()` 里，而 `DoLine()` 只被**旧 PPU**
   主循环调用（`:1321`）。新 PPU 主循环（`FCEUX_PPU_Loop`，`:1557`）在
   `:1778-1779` 调 `FCEUD_UpdatePPUView` / `FCEUD_UpdateNTView`，**那两处没有门**。
   `SDL.NewPPU` 默认 0（旧 PPU，`config.cpp:884`）⇒ **默认配置下 V-1 测的
   就是被门控的那条路径**。**跑 V-1 前不要改 PPU 引擎设置**；
   若已开新 PPU，V-1 测不到门，须在备注里写明。

5. **`vblankScanLines` / `vblankPixel` 的更新被 A1 门住了**。
   它们的计算在 `debug.cpp:921-929`（门内），消费者是调试器 PPU 页签的
   扫描线/像素显示（`ConsoleDebugger.cpp:4346-4361`）。
   ⇒ **零断点时调试器 PPU 页签的 vblank 读数会停在旧值**。
   这是 A1 的**可观察行为变化**（不是崩溃，但读数会不对）。
   S-4（Run Line）测的就是扫描线读数——若 S-4 的扫描线读数在 vblank 区间
   显示异常，**先别急着判 S-4 红**，那是本条已知现象，不是门控缺陷。

6. **`break_on_unlogged_code` / `break_on_unlogged_data` 不在门的 9 路 OR 里**
   （`debug.cpp:918` 没有这两项）。它们在 `LogCDData()` 内被消费
   （`debug.cpp:508-515`），而 `LogCDData()` 只在 `debug_loggingCD` 为真时
   被调（`:973-974`）——所以它们**间接**被 `debug_loggingCD` 这条腿覆盖。
   ⇒ **「Break On... → Unlogged Code / Unlogged Data」必须配合 CD Logger
   正在运行才有意义**。本清单未单列这两项（L-1 已覆盖其门控面）。

---

## 八、相关文档

- `docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md` §1.1（A1：改法、风险、
  与「验收未闭合」的现状）与
  `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md` §2.1（A1 的代码落点）
- `docs/tech/cpu.md`（6502 指令与周期模型）
- `docs/tech/precision.md`（精度治理与禁忌清单）
- `docs/tech/gba-traps.md` §7（写检查之前必读的三问）
- `src/rust/crates/gba-core/ATTRIBUTION.md`（vendor 树与本地补丁集）
