# A1 手工清单 G 项业主走查单（18 项）

> 落档：2026-11-08（r24）。由 A1 清单无头自动化收口 agent 产出（会话产物 → 入库），
> 配套权威源 `docs/tech/debug-regression-checklist.md`（步骤编号引用不重写）。
> 口径更正：勾选表实际 **21 行**（文档自称 22——计数滑动）；L-2 核心路径已无头验证 PASS（r24），
> 本单 = 剩余 18 个真 GUI 项；LUA-1/LUA-2 被缺陷 F-1（emu.frameadvance panic，r24 记档）阻断，
> 单内已含 GUI 复现取证步骤。

> 配套：`docs/tech/debug-regression-checklist.md`（步骤权威源，本单引用其步骤编号不重写）。
> 本次无头收口结果：L-2 核心路径已自动化验证 PASS；LUA-1/LUA-2 被缺陷 F-1 阻断
> （见报告），本单对这两项给出「GUI 复现 F-1」的附加取证步骤。
> 每项判红 ⇒ 按清单 §五 模板记录，不要自行改代码。

## 0. 通用准备（整个会话做一次）

1. 启动命令（Git Bash，PATH 前置 vcpkg 运行时）：
   ```bash
   export PATH="/d/Project/FCEUX11/vcpkg_installed/x64-windows/bin:$PATH"
   /d/Project/FCEUX11/build/Release/fceux11.exe &
   ```
   （或直接双击；用 **Release + FCEUDEF_DEBUGGER** 构建，即 build/Release 产物。）
2. ROM：`tests/fixtures/nestest.nes`（有真实代码流，跑得稳）；
   备选任意可跑到主循环的 ROM。**不要**用 fixtures 里 `__tmp_apu_square.nes` 之外
   的静音 sled ROM 测声音类项（本单无声音项，NSF-1 另需 .nsf 文件）。
3. File → Open ROM 载入，按 F5 跑起来。
4. 「零断点」确认（清单 §2.3）：Breakpoints 列表为空；Debug → Break On... 五项全不勾；
   Debug → Step Back（F9）灰色。
5. **PPU 引擎保持默认**（SDL.NewPPU=0，旧 PPU）——V-1 只在被门控的旧 PPU 路径上有效
   （清单 §4.4 V-1 警示框）。
6. 会话顺序建议：断点组 → 单步组 → 日志组 → viewer 组 → Lua 组 → NSF，
   每组做完把开的状态复位（取消勾选、关窗、Stop Logging）。

## 1. 断点组（门腿：numWPs / watchpoint[64] / badopbreak / break_on_cycles / break_on_instructions）

| 项 | 前置状态 | 操作（清单步骤号） | 绿 | 红 | 备注 |
|---|---|---|---|---|---|
| B-1 普通断点 | ROM 在跑，调试器开，Run 中 | 4-7：记一个主循环地址 → Breakpoints/Add → 填纯十六进制地址（**不带 $**），勾 Execute+Enable → Run | PC 停在该地址，标签 "Paused at Breakpoint" | 不停/停错地址 | 地址解析走 offsetStringInto（清单 B-1 注意框） |
| B-2 watchpoint | 同上 | 1-5：Add → 地址 `0000`，勾 Read（+Write）+Enable → Run | 数秒内停住 | 永不停 | Flags 列应显示 E/R/W |
| B-3a Execute 断点 | 同上 | 入口(a) 1-3：Add → 地址 + 只勾 Execute+Enable → Run | 恰停在该 PC | 停不到/停邻近 | 等价 r5 的 "Break on PC"（该菜单项不存在，清单 §七.1） |
| B-3b Run To Cursor | 同上 | 入口(b) 1-3：反汇编区右键目标行 → Run To &Cursor | 恰停在光标行 | 停不到 | 走 watchpoint[64] 保留槽 |
| B-4 Bad Opcodes | 同上 | 1-3：Break On... → Bad Opcodes → Run 几秒 | 正常 ROM 不停=**正确** | —— | 无非法 opcode ROM ⇒ 记「未验证」并写明缺什么（清单允许） |
| B-5 cycles 超限 | 同上 | 1-4：Break On... → Cycle Count Exceeded → Continuous+Absolute+阈值 100000 → Ok → Run | 停下，标签 "Cycle Count Exceedance"，CPU 页签 Cycles ≥ 阈值 | 不停/停错原因 | 顺带看 Cycles 字段在跑动时持续增长（A1-b 旁证） |
| B-6 instructions 超限 | 同上 | 1-4：Instruction Count Exceeded 同上 | 停下，标签 "Instruction Count Exceedance" | 不停 | Instructions 字段持续增长 |

## 2. 单步组（门腿：step / stepout / runline / watchpoint[64]）

| 项 | 前置状态 | 操作 | 绿 | 红 | 备注 |
|---|---|---|---|---|---|
| S-1 Step Into | 已暂停（F6） | S-1 步骤 3-4：记 PC → 按 F11 ×3 | 每按一次 PC 前进一条 | PC 不动 | 门=dbgstate.step |
| S-2 Step Over | 已暂停 | S-2 步骤 2-3：PC 移到 JSR → F10 | 停在 JSR 返回地址之后 | 停进子程序体/弹 "in process" | JSR 走 watchpoint[64]；无 JSR 退化为单步 |
| S-3 Step Out | 停在子程序内部 | S-3 步骤 2-3：Shift+F11 | 停在调用者下一条 | 不停/弹 "Already Active" | jsrcount 计数 |
| S-4 Run Line / 128 | 已暂停 | S-4 步骤 2-3：F7 ×3 再 F8 | 扫描线按 1 / 128 前进（PPU 页签 Scanline 字段） | 不动 | **vblank 区间读数异常是已知现象**（清单 §七.5：vblankScanLines 在门内，A1 可观察行为变化），别误判红 |

## 3. 日志组（门腿：debug_loggingCD、trace_logging；L-3 验 OR 语义）

| 项 | 前置状态 | 操作 | 绿 | 红 | 备注 |
|---|---|---|---|---|---|
| L-1 CD Logger | ROM 在跑，**不开其他特性** | L-1 步骤 2-7：Advanced → Debug → Code/Data Logger → Start → 跑 10 秒 → Pause | 六个百分比从 0% 上升，暂停后冻结 | 恒 0.00% | **核心路径无头等价物尚无**（debug_loggingCD 无 Lua/env 置位途径），GUI 唯一闭合手段 |
| L-3 同时开 | L-1 保持 Start | L-3 步骤 1-3：再开 Trace Logger → Start Logging → 跑 5 秒 | 两个窗口**都**有输出 | 只有一个有输出 | 专门验两条腿 OR 不互斥 |
| L-2 Trace Logger（GUI 壳部分） | ROM 在跑 | L-2 步骤 2-6：Trace Logger → Start Logging → 勾 Registers+Instr Count → 跑 5 秒 | 日志区持续新行、有反汇编、有计数列 | 空白/不增长 | **核心路径已无头验证 PASS**（本日 FCEUX11_TRACE 实验）；GUI 壳剩「按钮→Qt 状态位同步（TraceLogger.cpp:589）→列表控件渲染」这一段 |

## 4. viewer 组（门腿：nt_view_open / ppu_view_open——窗口状态，非调试器状态）

| 项 | 前置状态 | 操作 | 绿 | 红 | 备注 |
|---|---|---|---|---|---|
| V-1 NT Viewer | **零断点，调试器不开** | V-1 步骤 2-5：Advanced → Debug → Name Table Viewer → 等 2 秒+ | 图块有内容/随画面变化 | 全黑且不变 | **整份清单里唯一真正测 A1-c 门条件**的项（清单 §七.4） |
| V-2 PPU Viewer | 同上 | V-2 步骤 2-4 | pattern table 有内容 | 全黑 | ppu_view_open core 侧只写不读 ⇒ 几乎不可能因 A1 变红（§七.4） |
| V-3 Sprite Viewer | 同上 | V-3 步骤 2-3 | OAM 区有数据 | 空白 | 同上 |
| V-4 viewer+调试器同开 | 零断点 + 调试器窗口开着 | V-4 步骤 1-3 | viewer 照常刷新 | viewer 卡死 | 防「门条件混用」 |

## 5. Lua 组（门腿：break_asap；A1-b 反向项）

> ⚠️ **先看 2026-11-08 的缺陷记录 F-1**（详见收口报告）：
> 无头 runner 里任何 `emu.frameadvance()` 都会让进程 abort（panic in a function
> that cannot unwind, 0xC0000409）。GUI 走同一条 `fceux11_lua_frame_boundary`
> → Rust 回调 → yield 链路，**大概率同样崩**。所以这一组请先按下面步骤做，
> 若模拟器进程**在脚本 Start 后立即消失**，那就是 F-1 在 GUI 复现——按 §五
> 记录（症状=进程退出/闪退），这本身就是有效取证，不算操作失误。

| 项 | 前置状态 | 操作 | 绿 | 红 |
|---|---|---|---|---|
| LUA-1 hitbreakpoint | ROM 在跑 | 准备 `bp_test.lua`（清单 LUA-1 原文脚本）→ File → Load &Lua Script → Browse 选中 → Start → 等 1-2 秒 | 停下，标签 "Paused on Lua Breakpoint" | 不停；**或进程直接退出（=F-1，另记）** |
| LUA-2 getinstructionscount | ROM 在跑 | 准备 `instr_test.lua`（清单 LUA-2 原文脚本）→ Load &Lua Script → Start → 看 Output Console | 每帧 delta > 0 持续增长 | delta 恒 0；**或进程直接退出（=F-1，另记）** |

## 6. NSF-1

| 项 | 前置 | 操作 | 绿 | 红 |
|---|---|---|---|---|
| NSF-1 播放 | 手头有一个 .nsf 文件 | File → Play &NSF → 选文件 → 等 2 秒+ | 有声 + NSF 可视化画面（波形柱+曲名） | 无声/黑屏无信息 |
| NSF-1 替代（无 .nsf 时） | —— | 清单 NSF-1 替代验证：Breakpoints/Add → Address 填 `PLAY` → Execute+Enable → Run（需先 Play NSF 载入……若无 .nsf 此替代也不可用） | 能停在 PLAY 地址 | —— | 无 .nsf ⇒ 记「未验证（缺文件）」 |

## 7. 回写

全部做完后把 §6.1 勾选表填好（日期/结果/备注），交主会话按清单 §6.3 回写
v2.0.2 计划 §十。红项不得静默关闭；多项红按不变式 1 先回退 `src/debug.cpp:918`
的门做变异验证再决策。

---

## 8. 走查结果（2026-11-13 追记）

> **执行**：ZCode 代业主走查（业主事前拍板 LUA 组「照走查单跑并记录 F-1 取证」+「现在开始」，
> 桌面全程由 ZCode 经 UIA 自动化操控）。**环境**：`build/src/fceux11.exe`
> （Release + `FCEU_DEF_DEBUGGER` + Rust Lua，Qt 6.8.0，zh-CN locale），
> HEAD `bc095a7`（wip204），ROM = `tests/fixtures/nestest.nes`；PPU 默认 `SDL.NewPPU=0`。
> **证据目录**：`C:\Users\ikrx2\Desktop\v204_a1_evidence\`（results.md 全程日志 +
> shots/ 30+ 截图）。逐项结论已回填清单 §6.1（21 行全部有结果+日期）；
> 回写目标改记 v2.0.4 计划 §七 r4（§6.3 指向的 review 报告为冻结快照，未动）。

### 8.1 十八项逐项结论

| 组 | 项 | 结论 | 一句话依据 |
|---|---|---|---|
| 断点 | B-1 | 绿 | C28F 命中，"Paused at Breakpoint: 0" |
| 断点 | B-2 | 绿 | 01FD R+W 命中，Flags=ECRW--（0000 未被该 ROM 循环访问，换栈址） |
| 断点 | B-3a | 绿 | 纯 Execute 精确命中 |
| 断点 | B-3b | 绿 | Run To Cursor 恰停光标行，走 watchpoint[64] 保留槽 |
| 断点 | B-4 | 未验证 | 干净 ROM 勾选跑 5s 不停（负半验证绿）；正向缺非法 opcode ROM |
| 断点 | B-5 | 绿 | 「模拟器因周期计数超限暂停」，周期字段高亮 ≥ 阈值 |
| 断点 | B-6 | 绿 | 「模拟器因指令计数超限暂停」，指令字段高亮 ≥ 阈值 |
| 单步 | S-1 | 绿 | ×4 每步恰 +1 指令 |
| 单步 | **S-2** | **红** | **F10 三次激活均不停在 JSR 返回地址**（详见 8.2） |
| 单步 | S-3 | 绿 | NMI 处理器步出 → 停调用者下一条（+74 指令与处理器长度吻合） |
| 单步 | S-4 | 绿 | F7×3 各 +1 扫描线；F8 +128 跨帧回绕精确（246+128 mod 262=112） |
| 日志 | L-1 | 绿 | PRG Logged Code 0→1.01%，暂停冻结 |
| 日志 | L-2 | 绿 | 日志行持续增长（计数列+寄存器+反汇编），1,000,000 行 |
| 日志 | L-3 | 绿 | 双开：CD 重置归零后在 trace 开启下重涨（OR 语义无互斥） |
| viewer | V-1 | 绿 | NT 图块完整渲染（零断点、无调试器 = A1-c 门条件核心项） |
| viewer | V-2 | 绿 | 图案表 0 字体图块完整 |
| viewer | V-3 | 绿（备注） | nestest 不写 OAM，空 OAM 是夹具真实状态；查看器在渲染 |
| viewer | V-4 | 绿 | 调试器同开三 viewer 照常渲染（门污染黑屏模式排除） |

（LUA-1/LUA-2 与 NSF-1 不在本单 18 项内：前两项按 §5 预期完成 F-1 GUI 复现取证
（事件日志 0xC0000409 ×2，V1 落地后复走）；NSF-1 可视化绿、声音侧未验证
（夹具 play 例程 = BRK+NOP sled），且发现 **PLAY 关键字被断点对话框判
「起始地址无效」**——与本单引用的清单 §4.5 替代验证前提不符，记入
「入口与文档不符」家族。）

### 8.2 S-2 失败记录（按清单 §五 五字段）

1. **症状**：F10（单步跳过，工具栏与调试菜单两入口均试）后标签转「模拟器运行中」，
   从不停在 JSR 返回地址；显示寄存器在运行态不刷新（预期），停止与否以标签 +
   手动暂停后寄存器读数为准。
2. **复现步骤**：B-1 步骤 1-2 → 硬重置（模拟菜单）→ 添加 Execute 断点 C07E
   （nestest init 的 `JSR $C2A7`）→ Run 命中 → F10。
3. **ROM / 参数**：`tests/fixtures/nestest.nes`；断点 C07E Execute+Enable；三次激活：
   ①带断点跑至主循环（暂停读数 +130,378,156 指令、PC=C28F）；②菜单入口同（+212,504,928、
   PC=C291）；③隔离态（命中后删除 C07E、零断点）15s 轮询无停机。
4. **环境**：Release + FCEU_DEF_DEBUGGER，HEAD `bc095a7`，Qt 6.8.0，zh-CN。
5. **门控状态**：第③次激活时断点表为空、Break On 五项全 Off、无 logger、无 viewer；
   截图 `shots/s2_stopped_at_jsr.png` / `s2_isolated_result.png` / `s2_after_pause.png`。

   对照事实：子程序 `$C2A7` 与嵌套 `$C2ED` 均有 RTS（ROM 反汇编实证）⇒ 返回必然发生；
   B-3b（同走 watchpoint[64]、短程）精确命中 ⇒ 特征 = 「watchpoint[64] 长程不消费」。
   **处理**：按 §6.3，红项不静默关闭——先回退 `src/debug.cpp:918` 门做变异验证再决策
   （业主拍板）；记录见 v2.0.4 计划 §七 r4。
