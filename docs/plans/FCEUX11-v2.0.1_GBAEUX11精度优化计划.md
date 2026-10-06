# FCEUX11 v2.0.1 — GBAEUX11 精度优化计划（AGS 外部测试驱动）

> **STATUS: OPEN**（按 Phase 独立立项；每阶段带独立验收门禁，无一次性关闭日）
> **进度：P0 ✅（2026-10-06，见 §十一 r1）**；P2 → P3 → P1/P4、P5 待做。
> **日期**：2026-10-06
> **依据**：外部测试报告 `docs/tech/gba-external-test-report-2026-10.md`
> （jsmolka 13 ROM + AGS 老化卡 v7.0 成绩单；harness 与全部证据在
> `Research_only/gbatech/`）
> **治理**：本计划全部改动限于 `src/rust/crates/gba-core/` 与
> `src/rust/src/gba/`（GBA 侧），**NES 侧零触碰**（不变式 9 / 用户裁决
> 2026-10-01）。GBA 不接入任何质量检测系统：AGS/jsmolka 是**验收测量**，
> 不是门禁；回归门禁仍为 `ctest -LE perf`（33 项全绿）+ GBA 锁测试全绿。
> 每处 gba-core 改动 = vendor 本地补丁集变更，须逐处登记
> `src/rust/crates/gba-core/ATTRIBUTION.md` §3.2。
>
> **纪律**：先记变更记录再改文档；每条新锁测试做变异验证（改回旧值必须变红）；
> 不凭记忆写位级格式——本计划引用的时序常数全部给出处（mGBA 0.10.5 源码只读
> 参照，MPL-2.0 只读不抄；AGS 测试逻辑以 Normmatt/ags_aging 反编译为第一手）。

## 一、基线（2026-10-06 外测）

| 测量面 | 结果 |
|---|---|
| jsmolka 13 ROM | 10 全过；arm/bios 因 ARM SWI 编号缺陷挂死（隔离后 arm 仅 t234、bios 全为 open-bus 设计差异） |
| AGS v7.0（强制 SRAM） | MEMORY 8/9 ✗、TIMER 2/3 ✗、LCD 1/7 ✗、DMA 1–2/8 ✗；KEY INPUT、INTERRUPT、CARTRIDGE TYPE FLAG 过 |
| AGS 原样运行 | CHECK SUM ERROR 死循环（EEPROM_V 误检；mGBA 同病；强制 SRAM 解锁） |
| SMA4/MKSC 冒烟 | SMA4 经 harness 启动至语言选择画面（与 r51 手测一致） |

**统一根因图景**：失败集中在**总线/外设时序层**——我们有一个近似时序模型
（`bus.rs::access_cycles` 已含 WAITCNT 分区等待与 32 位折算），但四处偏差
叠加使所有"测量时间"的测试失败：S/N 全局启发、预取"乐观近似"、DMA 无专属
时序、定时器计数边界。此外有一个独立的一行级缺陷（SWI 编号字节）。

## 二、Phase P0 — ARM 模式 SWI 编号修复（最高优先，独立立项）

**缺陷**：`gba-core/src/cpu/arm7tdmi.rs:686-696`，ARM 分支取 `swi_instr & 0xFF`；
真机约定（三重佐证）：THUMB SWI 编号 = 指令 bits 0-7，**ARM SWI 编号 =
指令 bits 16-23**。规范编码 `swi 0x060000`（Div，jsmolka/AGS 风格）被读成
0x00 = SoftReset → 任何 ARM 模式 BIOS 调用触发软复位循环。
佐证：① jsmolka 全部 ARM 编码按 bits 16-23；② mGBA 0.10.5 对同指令解码为 06
（外测报告 §五）；③ devkitPro/libgba `src/Div.s` 为 `.code 16`（THUMB）用
`swi 6`、而 ARM 汇编惯用 `swi 0x60000`——两种约定并存且都依赖正确分支。

**改动**：ARM 分支改 `(swi_instr >> 16) & 0xFF`（THUMB 分支不动）。
**同时改本仓库三处一手编码器**（`gate.rs` 的 `swi()`、`swi/mod.rs` 的
`arm_swi()` 与 `SWI_HALT_WORD`）——它们此前把编号写进低 8 位，是靠缺陷才绿的，
见 §十一 r1 ③。
**登记**：ATTRIBUTION §3.2（arm7tdmi.rs 本地补丁 10 处 → 11 处）。
**验证**：
1. 新锁测试：ARM 模式构造 `swi 0x060000` 断言 Div 语义生效且 CPSR/寄存器无
   SoftReset 副作用；变异可红（改回 `& 0xFF` 必须转红）。
2. jsmolka arm/bios 复跑：从"挂死"变为正常判卷（arm 应显示 Failed test 234，
   bios 显示 Failed test 001，参照报告 §五/§七）。
3. `ctest -LE perf` 全绿；SMA4/MKSC harness 冒烟。

## 三、Phase P2 — S/N（顺序/非顺序）访问跟踪重构（时序地基）

**现状**：`bus.rs::access_cycles` 用单一全局 `last_used_address` 判顺序性
（`address - last_used_address == size`）。取指与数据访问交错时该启发式失效
——AGS 的测量循环里几乎每次数据访问都被误判为非顺序。

**目标语义**：参照 mGBA `memory.c`（`GBA_BASE_WAITSTATES*` 四张表 +
`GBARomWaitstates`/`GBARomWaitstatesSeq` = 非顺序 {4,3,2,8}、顺序
{2,1,4,1,8,1}，memory.c:37-42）的 S/N 判定方式实现；关键是明确
"上一个总线访问"的跟踪粒度（mGBA 以最近一次访问的地址+宽度为准，取指与
数据访问同流参与）。**只读参照，不抄代码**；语义对齐后按我们的结构实现。

**改动点**：`bus.rs` 的 `last_used_address` 跟踪 → 按 mGBA 语义重实现
（含 `read_opcode_*` 与数据访问同流）。
**验证**：
1. 先行工作项：解码 AGS `sub_800329C`/`sub_8003310` 汇编（Normmatt 仓库
   `src/sub_800326C.arm.s`），提取 WAIT STATE WAIT CONTROL 与 CARTRIDGE RAM
   WAIT CONTROL 的精确期望值（后者已知 0x1C/0x18/0x14/0x2C），写成常量锁测试。
2. AGS 复跑：两个 WAIT 类测试转 0。
3. `ctest -LE perf` + 锁测试全绿 + MKSC/SMA4 冒烟（时序改动可能影响限速感受，
   `bench_tolerance_test` advisory 只看不设卡）。

## 四、Phase P3 — 预取缓冲真模型（替换乐观近似）

**现状**：`bus.rs:1376-1385`——顺序取指在 WAITCNT bit14（预取使能）时按
`size/2` 计（"预取器已跟上"的乐观假设）。真机是 8×16-bit 预取队列：CPU 不占
Game Pak 总线时预取器推进、取指命中则 1 周期/半字、未命中照付等待、数据访问
使缓冲失效。测量循环（数据访问穿插）下乐观假设系统性偏差。

**目标语义**：mGBA 的预取实现（`memory.c:436-452` 的 prefetch 读取、
`gba.c:296` 总线缓冲、ARM 核心侧填充逻辑）——语义级参照。

**验证**：AGS PREFETCH BUFFER 测试转 0；TIMER PRESCALER 的期望值
（{4096,64,16,4}，测量循环 1024×(SUBS+BNE)=4096 周期）依赖预取精确性，
须与 P4 联合验收；`ctest` + 冒烟同上。

## 五、Phase P1 — DMA 传输时序

**现状**：`bus.rs::run_dma_block` 无 DMA 专属时序——逐单元借用
`read_word/write_word` 的 CPU 访问记账；无起始开销、无 DMA 的 S/N 模式。
mGBA 权威语义：**每次 DMA 起始固定 +3 周期**（`dma.c:115/147/175`
`when = now + 3 + cycles`），逐单元按源/目标区域等待周期计算。

**改动点**：`run_dma_block` 增加起始开销与显式逐单元时序（首单元非顺序、
后续顺序；源/目标各自计价），与 CPU 的 `last_used_address` 流解耦。
**验证**：
1. 先行工作项：解码 AGS `TimeDmaToAndFromMemory_U16/U32` 汇编，提取期望
   周期数（内存类测试各含 2 个计时位，是 8/9 内存测试失败的共同成分），
   写成常量锁测试（变异可红）。
2. AGS 复跑：DMA 类转 0；内存类 8 项中仅剩等待/预取相关失败。
3. `ctest` + 冒烟。

## 六、Phase P4 — 定时器计数边界对齐

**现状**：`timers.step(elapsed)` 按累计周期推进；mGBA 以 prescaleBits
{0,6,8,10}（timer.c:124）在事件调度器上精确对齐预分频边界与级联链。
AGS 期望值是**精确数**：PRESCALER 期望 {4096,64,16,4}（1024×4 周期循环）、
CONNECT 期望 512（sub_8009294：TM0 装载 65534、四级级联后读 TM3）。

**改动点**：`timers.rs` 的预分频推进边界（计数发生在分频窗口末端）与
级联递增的瞬时语义对齐 mGBA `timer.c`。
**验证**：AGS TIMER PRESCALER/CONNECT 转 0；把 {4096,64,16,4} 与 512 写成
核心级锁测试（不依赖 AGS ROM）；`ctest` + 冒烟。

## 七、Phase P5 — LCD DISPSTAT 窗口时序

**现状**：`lcd.rs` 在 pixel_index==240 置 HBlank 标志、行界清除；VBlank
窗口同理。AGS H BLANK STATUS 测试在已知窗口内采样 DISPSTAT 失败——
疑似置位/清除的周期粒度差 1–2 周期（V COUNTER/INTR FLAG 类已过，
说明大窗口正确，边界偏差在 STATUS 采样上现形）。

**改动点**：对齐 mGBA `io.c`/`video.c` 的 DISPSTAT 位窗口语义（先解码
AGS `sub_8003A1C` 取期望窗口）。
**验证**：AGS LCD 7 项全 0；`ctest` + 冒烟。

## 八、实施顺序与依赖

```
P0（独立，可先行）
P2（S/N 地基） → P3（预取） → P1（DMA，依赖 P2 的 S/N）
                            → P4（定时器，依赖 P2/P3 的总线周期正确性）
                            → P5（LCD，独立于 P1–P4，可在 P2 后任意插入）
```

每个 Phase 一个独立小任务立项：只读调查 → 变更记录（§十一）→ 实现 →
锁测试（变异验证）→ AGS/jsmolka 复跑 → ctest → 汇报。**任何一步 NES 侧
出现触碰即停**。

## 九、验收基线与测量方法（对执行者）

```powershell
# AGS 复跑（强制 SRAM；4800 帧足够稳定到成绩单）
$env:GBA_FORCE_SAVE_TYPE = "1"
Research_only\gbatech\gbaharness.exe <ags.gba> 4800 <outdir> <label>
# jsmolka 复跑（同 harness，不带 GBA_FORCE_SAVE_TYPE）
# 判卷：BMP → PIL 按 8px 行带裁剪放大（模板 Research_only/gbatech/bmp2png.py）
```

- AGS 各类别（§3.3 表）为逐 Phase 验收基线；目标：P2 后两个 WAIT 类转 0、
  P3 后 PREFETCH 转 0、P1 后 DMA 转 0、P4 后 TIMER 转 0、P5 后 LCD 转 0、
  P0 后 jsmolka arm/bios 不再挂死。
- 最终目标态：AGS 全类通过（COM 除外，需硬件）；MEMORY 类是唯一预期保留
  不确定性的类别（若 P2/P3 落地后仍失败，则须解码 EWRAM/IWRAM 等测试的
  计时位期望值再立下一轮，见 §十）。

## 十、开放问题（不阻塞立项）

1. AGS MEMORY 类各测试含多个计时子位，失败位组合（8 项全败）无法从屏幕
   细分——P2/P3 落地后复跑，仍失败者再解码对应反编译提取期望值。
2. DMA 行 8 列对 9 项测试表的 1 列缺口（显示代码未反编译）——P1 期间以
   mGBA 屏幕对照或反编译补齐。
3. EEPROM_V 误检的产品层解法：AGS 场景已由手动覆盖解决；"多签名共存 ROM
   的检测启发式"记录为 open 问题，不在本计划承诺。
4. mGBA 屏幕级对照缺失（本地 lib-only 构建）——如需强对照，可另行构建
   mGBA 前端或在 mGBA 上复跑同一 harness 判卷流程（单独小任务）。

## 十一、变更记录

> 沿用 v2.0 计划 §十四 的纪律：执行期若发现计划与实态不符，**先记入本表再改**，
> 不静默偏离 §八 的阶段出口标准与 §十 的不变式。

| **r1** | 2026-10-06 | **P0 开工前的只读调查：缺陷判定成立，但 P0 的范围比 §二 写的宽一层 —— 本仓库自建门禁从未覆盖 ARM 的 SWI 编号约定，因为门禁自己的编码器和缺陷是同一个错误。**<br>**① 缺陷复核通过**，位置与外测报告一致（`arm7tdmi.rs:683-696`，ARM 分支 `swi_instr & 0xFF`）。规范编码 `swi 0x060000` 的编号在 bits 16-23，低 8 位是注释字段，本核心读出 `0x00` = SoftReset。<br>**② 调查中发现的三处一手代码，全部用「低 8 位」编码 ARM 的 SWI：**<br>· `src/gba/gate.rs:132` `swi()` = `0xEF00_0000 \| (number & 0x00FF_FFFF)` —— S2-c 自建门禁的唯一指令构造器；<br>· `src/gba/swi/mod.rs:783` `arm_swi()` = `0xEF00_0000 \| (n & 0xFF)`；<br>· `src/gba/swi/mod.rs:1316` `SWI_HALT_WORD = 0xEF00_0002`。<br>**③ 所以 S2-c 那 5 条往返锁测试与 S1a-1 的「真实路径」测试，是靠这个缺陷才绿的** —— 它们的编号写在错误的那 8 位里，核心读错误的那 8 位，闭环自洽，全套 244 项无一报警。**这是 r33 与 r49 同族的错误：门禁被写成了核心错误信念的镜像**，而门禁的存在意义恰恰是抓这种信念错误。<br>**④ 裁决：只认 bits 16-23，不做低 8 位兼容。** 依据是三方独立一致 —— mGBA 0.10.5 对 `0xEF060000` 解码为 SWI 06（外测报告 §五 证据 2）、jsmolka 全部 ARM 编码按 bits 16-23、devkitARM 的 ARM 汇编惯用 `swi 0x60000`。真机 BIOS 只读 bits 16-23，兼容低 8 位等于凭空发明一个不存在的硬件行为，**并且会让门禁继续绿着**——那正好抵消这次修复的全部价值。<br>**⑤ 由此 P0 的出口范围增加三项**：上述三处编码器随核心一并改正（`frame.rs:943` 的 `0xEF00_0000` 是 SoftReset、两种编码下编号都是 0，**不动**）。这不是「顺手修」，而是 P0「锁测试全绿」出口标准的必要条件：不改这三处，核心修好后门禁反而转红。<br>**⑥ 与 §二 的差异已在此登记**：§二 只写了核心那一行 + ATTRIBUTION 登记 + 一条新锁测试；实施范围是「核心 1 行 + 一手编码器 3 处 + 新锁测试 + ATTRIBUTION/CHANGELOG/AGENTS.md」。NES 侧零触碰。 | 只读调查 |

**r1 实施结果**（2026-10-06）：① 核心 `arm7tdmi.rs` 的 ARM 分支改 `(swi_instr >> 16) & 0xFF`，THUMB 分支未动 —— vendor 树第 18 处本地补丁，登记 `ATTRIBUTION.md` §3.2.3（含「**上游是错的、重新 vendor 要第一个核对**」的提示）与 §4 的 17→18 记账。② 一手编码器三处随改：`gate.rs` 的 `swi()`（并加 `assert!(number < 0x100)`）、`swi/mod.rs` 的 `arm_swi()`、`SWI_HALT_WORD` 0xEF000002→0xEF02_0000。**`frame.rs:943` 的 `0xEF00_0000` 是 SoftReset、两种编码下编号都是 0，确认不动。** ③ 新增 2 条锁测试（`gate.rs::arm_swi_numbering`）。

**④ 变异验证两轮、三个方向**：核心改回 `& 0xFF` → **两条同时转红**，且各自打出自己的断言消息；编码器改回低字节 → `the_number_lives_at_bits_16_23` 当场转红（`left: 0xEF000006` vs `right: 0xEF060000`），负向测试正确保持绿（它用字面量，不依赖编码器）。

⚠️ **第二轮变异抓出了一条无效测试，是我写完后自己改掉的。** 负向测试初稿用 ARM `SWI` 收尾指令（`Halt`）判断程序是否停机 —— 而收尾指令本身就受被测约定支配：读错字节的核心把**它**读成 `SoftReset`，于是「没有停机」在变异下同样为真，测试在它应当抓住的变异下全绿，**零鉴别力**。第一轮输出「1 passed, 1 failed」正是信号 —— 红的确实是我预期的那条，**绿的那条其实是废的**。改法：判据从「是否停机」换成「`r0` 是否出现过商」—— `Div` 写下商、`SoftReset` 清零并重入卡带，两方向相反，**不需要任何退出条件**，故与被测约定无关。

**教训（与 r33、r49 同族的第三次，且这次多一条）**：变异验证不只用来证明「测试会红」，**也用来发现「某条测试根本不会红」**。同一轮里我还犯过一个更小的错：第一次「变异」**只加了一行注释、根本没改表达式**，却因为测试仍全绿就往下走了一步 —— **「变异后仍全绿」与「变异没生效」在输出上长得一模一样**，必须回 diff 确认改动真的落进了被测路径。两次都靠「回去看 diff / 看哪条红了」发现，没有一条靠「输出是红的」判断。

⑤ 门禁：GBA 锁测试 **246 项全绿**（244 + 2）；两态 `cargo check` 通过；不变式 8 复核（`cargo tree` 计次：关 feature 时 `gba-core`/`rtrb` **0 次**、开 feature 时 3 次）；全量 Release 构建 `BUILD_EXIT=0`；`ctest -LE perf` **33/33**（NES 零回归）。导出面 34 个符号不变（未新增 C ABI）。

⑥ **jsmolka arm / bios 复跑：两处都由「白屏挂死」变为正常判卷，且落在计划预测的测试号上** —— arm 显示 `Failed test 234`（ARM7TDMI 的 MSR 别名怪癖，报告 §六 的冷门设计差异，非本 Phase 范围）、bios 显示 `Failed test 001`（报告 §七：bios 套件 4 项全是 stub BIOS 的 open-bus 设计差异）。**这两个读数与外测报告里「把 ROM 改迁就缺陷」那组变异 ROM 的读数逐字一致** —— 即核心侧修好之后的行为，等价于当初用 ROM 变异证明的因果。

⑦ **SMA4 冒烟无回归**：harness 600 帧仍启动到语言选择画面（与 r51 手测、外测报告一致）。
**MKSC 冒烟无回归，闭环**（用户 2026-10-06 补充提供 ROM 路径，复制到
`Research_only\gbatech\mksc.gba` 后跑 harness）：1200 帧**启动到标题画面**（Mario Kart
Super Circuit logo + 1P 选择提示），逐帧哈希 99fa53fb → ce0662c6 → 660b1a25 → d55214cb
→ 57a8d625 每档都在变，机器在真跑不是冻帧。**顺带核了 BETA 水印完好**：按
`overlay.rs` 的字形表逐点取样，两帧各 55 个字形像素**全部为纯白 (255,255,255)** ——
缩略图上看着像被游戏盖住了，实际是缩略图混叠，若不是逐点取样就会误报一条不存在的
缺陷。
**⚠️ 但「可玩性」仍未验证**：harness 只证明能启动到标题画面，进赛、操控、音频都不在
本次证据范围内，手测仍待用户确认。

⑧ NES 侧零触碰：`git diff` 仅 `src/rust/crates/gba-core/src/cpu/arm7tdmi.rs` + `src/rust/src/gba/`（`gate.rs`、`swi/mod.rs`）+ 四份文档。 | 实施 + 变异验证 + 复跑 |
