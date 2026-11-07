# kgmqa-037 中断轮询下沉 + hijack（D1 专线）—— S0 调查页

> **STATUS: OPEN**（S0+S1+S2 完成 2026-11-07：取证落档、INTLEDGER 探针+锁测试、两拍模型+向量级 hijack；S3–S5 未动。S2 实施记录见 §10）
> 落地：2026-11-07（v2.0.2 计划 r12 的 D1 专线 PLAN，S0 阶段）
> 上位卡：`docs/plans/long-term-evolution/T2-中断时序.md`（分项计划 T2 第 1 步）
> 设计裁定（r12）：**线时刻账本**——本仓无每访问周期步进，Mesen2 两拍不可照抄。
>
> 证据强度：`[实测]` = 本轮 E-1 探针实跑读数；`[读码]` = 读源码推断；
> `[外部]` = nesdev wiki / Mesen2 参照，未在本仓复核。

---

## 1. 用例身份与失败现状

| 字段 | 值 | 出处 |
|---|---|---|
| kgmqa id | `kgmqa-037-cpu-int-2-nmi-brk-blargg` | `tests/tests.json` |
| ROM | `blargg/cpu/cpu_interrupts_v2_2-nmi_and_brk.nes`（`tests/fixtures/` 已 vendor） | tests.json `mirror_path` |
| 帧预算 / 协议 | `--frames 300` / `$6000` | tests.json |
| 当前失败码 | **0x01**（advisory；2026-11-07 基线复跑 EXIT=1） | `[实测]` |
| diag_string 开头 | `NMI BRK 00 / 27 36 00 / 26 36 00 ×5 / 27 36 00 …`（NMI×BRK 交互周期对齐差 1） | `[实测]` |
| known_limit 原文 | `CPU interrupt polling only at instruction boundary (x6502.cpp:515-579); hijack/branch-delay not modeled` | tests.json `provenance` |

同族：054 cli_latency（PASS 监视）· 040 vbl_05 nmi_suppression（PASS 监视）·
L2 `cpu_int_2/3/4/5` 全族。037 的出口判据见 T2 卡（金标 048/032/057/053/054 不回退）。

## 2. hijack 状态机（四交互矩阵，T2 卡第 1 步）

硬件规则 `[外部]`（nesdev wiki「CPU interrupts」）：轮询发生在**每条指令倒数第二周期
的 φ2 末**；分支指令在**末周期**二次采样；中断序列经 opcode 通道启动（opcode 槽为
dummy 读），因此可被劫持。

| 注入时刻 ↓ / 在飞实体 → | BRK（执行中） | NMI 序列（执行中） | IRQ 序列（执行中） | branch（执行中） |
|---|---|---|---|---|
| **NMI 在 BRK 轮询点后置位** | **hijack**：序列继续、向量=$FFFA、B=0（BRK 变 NMI 服务）（⚠ S2 实测更正：B=1 保留——向量级劫持，见 §10） | —（按排队） | 向量劫持同右 | 末周期窗口：派发提前 1 周期、PC=目标 |
| **IRQ 在 BRK 轮询点后置位** | 与 BRK 同向量 $FFFE、B=0（等价劫持，仅 B 位差） | 同左 | — | 同右（branch-delay） |
| **NMI 在 IRQ/BRK 序列执行中置位** | 向量劫持：本序列读 $FFFA、B 保持 0 | 第二次 NMI 排队、首次返回后立即服务 | 向量劫持：$FFFE→$FFFA | — |
| **branch 已取指、线在末周期前置位** | — | — | — | **branch-delay**：轮询点后移到末周期，目标首指令前派发 |

本仓现状 `[读码]`（`src/x6502.cpp`）：派发是**独立代码路径**（`X6502_RunDebug` 边界
判定 → `ADDCYC(7)+PUSH×3+向量读`，x6502.cpp:587-627），不经过 opcode 派发器 ⇒ 上表
**全部 hijack/向量劫持/branch-delay 行为零实现**；BRK（ops 路径）与 NMI/IRQ（边界路径）
互不可见，NMI 只能在 BRK **完整执行后**再派发（037 的差 1 即此）。

## 3. 差距面 G1–G5（PLAN §1.3）

| # | 差距 | 现状锚点 | 归属步骤 |
|---|---|---|---|
| G1 | **边界轮询**：只在指令边界 `if(_IRQlow)` 判定（x6502.cpp:555） | 硬件 = 每指令倒数第二周期末 φ2 采样 | S2（轮询下沉） |
| G2 | **无 hijack**：中断派发不走 opcode 通道，无劫持 | Mesen2：DummyRead 强制 opcode=$00 + ProcessInterrupt(hijack) | S2/S3 |
| G3 | **无 branch-delay**：分支末周期二次采样不存在 | 同一 loop-top 判定 | S5 |
| G4 | **服务序列无 dummy 读**：直接 PUSH×3+向量读，无 opcode/pad 槽 | 硬件 7 周期含 opcode 槽 dummy 读 | S2（随 hijack） |
| G5 | **NMIDELAY=8 近似**：VBL 块 `X6502_Run(8)` 后才置线，帧边界量化 | 硬件 sl241 dot2 /NMI；相位逐帧漂 1/3 周期（vbl_05 X 表根因） | 线时刻账本吸收（设计裁定） |

## 4. E-1 取证（现有 `FCEUX11_E1_TRACE=1`，零新增；2026-11-07 实测）

三 ROM 各跑一遍 `build/tests/f11qa_blargg_runner.exe`（帧预算同 tests.json），
收集 stderr 的 NMI 事件链（NMI_LATCH → NMI_SET → NMI_DEFER → NMI_DISPATCH）。
`abs` = `e1_cpu_abs()`（CPU 周期）；**delta = NMI_DISPATCH.abs − NMI_SET.abs**
=「线锁存 → 派发」间隔，即两拍模型要下重的那个量。

| ROM | 结果 | NMI_SET / SET2 / DISPATCH / DEFER | delta(set→dispatch) 分布 | 备注 |
|---|---|---|---|---|
| 037 nmi_and_brk | **FAIL 0x01** | 10 / 0 / 10 / 10 | **[2×8, 7×2]**（帧 3、4 = 7，其余 = 2） | 全部 VBL 路径；delta = DEFER 边界与派发边界之间在飞指令的长度（2 周期指令 ×8、7 周期指令 ×2） |
| 054 cli_latency | PASS 0x00 | **0 / 0 / 0 / 0** | —（无 NMI 事件） | 054 走 **IRQ 线**（FCOUNT）：E3 探针实测每帧 IRQ_BEGIN×2（set/set/clear 拓扑，596 起/298 释）——**CPU 侧 IRQ 派发无任何探针**，即 G1 的取证盲区 |
| 040 nmi_suppression | PASS 0x00 | 2 / 0 / 2 / 2 | **[2, 2]**（abs≈446713、1042330，约帧 15/35） | 其余帧全部 `VBL_SUPPRESSED`/vbl_set_suppressed，抑制路径正确 |

**对照 vbl_05 的 X 表**（`e1_vbl/vbl_step1_3_nmi_sampling_2026-08-02.md`）：期望
X=[4,4,4,3,3,3,3,3,3,2]、转变行 2-3 与 8-9；NMIDELAY 扫参证明任何固定 D 无法同时
满足派发 PC 与转变行——因为 delta 被「边界 + 在飞指令长度」量化（本表 delta∈{2,7}
两档），而硬件相位逐帧漂 1/3 周期。**结论：差值分布是派发模型的直接可观测量，
S1 锁测试的期望值来源。**

NMIDELAY=8 的实际授予量随指令对齐波动（`[实测]` VBL_ENTER→AFTER_NMIDELAY：
帧 1 = 2 周期、帧 2 = 4 周期）——G5 近似的量化噪声面。

## 5. 现有注入时刻 vs 两拍模型期望（S1 锁测试期望值来源）

| 事件 | 本仓现状（实测锚点） | 两拍模型期望 | 本步断言组 |
|---|---|---|---|
| NMI assert（VBL 路径） | 帧边界量化处置线；fresh 推迟一边界；dispatch−set = DEFER 与派发边界间在飞指令长度（2..7 周期，min=2） | cycle N 末采样、N+1 生效；dispatch−assert ≥ 在飞指令余长 | 基线：delta≥2 且 DEFER 记录在 set 同刻 |
| NMI2（$2000 边沿） | 边界 1：NMI2→NMI 转换；边界 2：派发（else-if 链天然两级） | 同上两拍 | 基线：转换记录→派发记录顺序 |
| IRQ（EXT/FCOUNT/DPCM） | 注入后**第一个** loop-top 即派发（无 fresh；set 可等于 dispatch 同刻）；I 旗标置位时挂起不派发 | 倒数第二周期末线值生效 → dispatch ≥ set + 在飞指令余长 | 基线：首边界即派发；目标（S2）：delta≥2 |
| hijack（BRK 在飞 + NMI 置位） | BRK 完整执行（压栈 B=1、向量 $FFFE）**然后** NMI 派发（B=0、$FFFA）= 两次序列、栈 6 字节 | 单序列：opcode 槽 dummy 读、B=0、$FFFA、栈 3 字节（⚠ S2 实测更正：B=1 保留，见 §10） | 目标（S2 fail-to-pass） |
| branch-delay | 分支完整执行后边界派发 | 末周期窗口采样，派发提前 1 周期 | 目标（S5 fail-to-pass） |

## 6. 探针与复跑

```powershell
$env:FCEUX11_E1_TRACE="1"
PATH=vcpkg_installed\x64-windows\bin;$PATH  ./build/tests/f11qa_blargg_runner.exe `
  --rom tests/fixtures/blargg/cpu/cpu_interrupts_v2_2-nmi_and_brk.nes `
  --kgmqa-id kgmqa-037-cpu-int-2-nmi-brk-blargg --frames 300
# stderr: E1 NMI_SET / NMI_DEFER / NMI_DISPATCH、E1B NMI_LATCH(_CALLEE)
# IRQ 侧加 FCEUX11_E3_TRACE=1（sound.cpp FCOUNT 线）
```

原始产物：`.zcode/agent-d1/e1_out/{037_baseline,037_trace,054_trace,054_e3,040_trace}.*`
（会话临时区，非仓库资产）。

## 7. S1 探针 `FCEUX11_INTLEDGER=<dir>`（2026-11-07 落地，x6502.cpp）

中断线时刻账本：七个注入点（TriggerNMI / TriggerNMI2 / X6502_IRQBegin /
X6502_IRQEnd / fceu11::NMI / fceu11::IRQ / 循环顶判定块）+ 二进制账本
`<dir>/intledger.bin`（8 字节头 `INTL1`+24 + 24 字节定长记录
`{u64 abs; i32 count; u32 irqlow; u16 pc; u8 kind; u8 edge; u8 decision; u8 pad}`）
+ `<dir>/summary.log`。kind：0=NMI 1=NMI2 2=EXT 3=EXT2 4=DPCM 5=FCOUNT
6=TEMP 7=RESET 0xF=边界快照；edge：0=release 1=assert 2=snapshot；
decision：0=none 1=reset 2=nmi2转换 3=nmi-defer 4=nmi派发 5=irq派发
6=held（I 旗标/jammed 挂起；连续相同 held 折叠计数 `held_suppressed`）。
64 MiB 上限；不设 env 完全静默零成本（e1_cpu_trace_on 惯例）。

**双通道交叉验证 `[实测]`**（037 三百帧，INTLEDGER vs E-1 同跑）：
assert/defer/dispatch 计数 10/10/10 两通道一致；E-1 delta [2×8, 7×2] 与
INTLEDGER delta [9×8, 14×2] 恒差 +7——E-1 的 DISPATCH 印在 `ADDCYC(7)` 之前、
INTLEDGER 的边界快照在判定块尾（`ADDCYC(7)` 已入账），两通道互为印证。
037 的 FCOUNT 线 assert=894 / release=10、DPCM release=6（APU 帧 IRQ 的
set/set/clear 拓扑在本 ROM 也活跃）；NMI release=334 = ppu.cpp 的
「cancel pending VBL NMI」路径（vbl_05 抑制机制的本体）。

## 8. S1 锁测试 `cpu_interrupt_poll_test`（tests/f11qa/core/cpu_test.cpp）

**独立参照模型**（纪律 5/12）：期望值按字段拼装自本页 §4/§5 标定表 +
服务序列结构（3 push：PCH/PCL/P；派发 7 周期），不调用被测代码。
**双组断言**（`FCEU11_EXPECT` 基线 / `FCEU11_EXPECT_FTP` 目标，FTP 自建
计数器不进退出码——ctest 36 项全绿硬约束不受影响）：

| 组 | 断言 | S1 状态 |
|---|---|---|
| 基线 B0-B7 | 线锁存即时性、fresh 一边界推迟、切片周期 2+7、单/双序列栈形、NMI2 两级转换、NMI 无视 I、I 置位挂起、TEMP 一次性、BRK 先于 NMI（B=1/B=0 字节序） | 838 项全绿 |
| 目标 T-D1-2 | 两拍：边界注入 EXT 首切片不派发、派发落在轮询点后边界 | 3 条 RED（S2 转绿） |
| 目标 T-D1-3 | hijack：单序列（S−3）、B=0、NMI 向量；BRK 槽 PC+2 形状 pin（⚠ S2 实测更正：B=1 保留，见 §10） | 2 条 RED + 2 条两态绿 pin（S2 转绿） |
| 目标 T-D1-4 | branch-delay：分支切片内不派发、压分支目标 PC | 4 条 RED（S5 转绿） |

S1 实测 **FTP RED=9 / GREEN=2**（T-D1-4d 现红机理：现行为 stale 边界先派发
→ CPU 进 ROM IRQ 处理器 RTI 弹栈 ⇒ S 回不到 $FA；S2/S5 后才成立）。

## 9. 变异验证（纪律 3，红→还原→绿留档）

| 变异 | 施加点 | 预期 | 实测 | 还原 |
|---|---|---|---|---|
| **M-D1-0** | `g_e1_nmi_fresh = true` 置位处短路（TriggerNMI） | 基线 fresh 推迟断言红 | **11 项红**（B1/B1a/B5a/B5b + B5c/B5d/B6-2/B2/B3/B3b/B4b——stale 边界先派发把 CPU 送进 ROM NMI 处理器，后续栈形/向量断言全部失准） | 838 全绿复现 ✓ |
| **M-D1-1** | 参照模型 `kSeqCycles` 7→9（期望值污染） | 周期计费断言红 | **2 项红**（B5b、I2a-2，恰为预测面；FTP 计数 9/2 不动） | 838 全绿复现 ✓ |

## 10. S2 实施记录（2026-11-07，数据节）

**落地形态**：① 采样点＝指令 dispatch 后（poll point），派发＝下一 loop-top；`g_intl_pending`（0 无/1 NMI/2 IRQ）承载采样结果，不入 savestate（闸门 #1）。② `g_e1_nmi_fresh` 退休（两拍采样天然产生同一时刻；连带 E1 NMI_DEFER 输出消失、INTLEDGER decision=3 恒 0、M-D1-0 失效→由下表 M-D1-2/2b 替代）。③ BRK hijack＝向量级（见下）。

**⚠ 模型更正（B 位，纪律 12 第四例）**：S0/S1 期判「hijack 压栈 B=0」有误。blargg README 原文（christopherpow/nes-test-roms 镜像；nesdev.org 全程 403 未取到）：

```
36  00  00 NMI interrupting BRK, with B bit set on stack
```

`2-nmi_and_brk.s` 实证：`nmi:` 处理器 `pla` 取的正是 NMI 序列压栈的 P（`sta <nmi_flag` ＝行首字节）⇒ 行 4-8 期望 `36 00 00` ＝ 压栈 P 含 B=1（BRK 自身 push 保留）＋ irq_flag=00（$FFFE 处理器未运行）＋ X=00（BRK 的 PC+2 槽形状）。**正确模型＝向量级劫持**：push 序列原样、仅向量读 $FFFE→$FFFA、消线。S1 断言 B3/T-D1-3b 与首版实现同源于该误读 ⇒ 锁测试全绿不证明此语义，靠外部参照暴露。另两处 harness 暴露的实现修正：压栈显式 `_P|U_FLAG|B_FLAG`（首版 `_P|U_FLAG` 的 B 位随 _P 残留浮动——真机偶然正确、power-on P=$34 的 harness 立刻暴露）；B4b/T-D1-3d 预算 30→7（30 会让 nestest NMI 处理器 RTI 弹栈、PC 离开向量；7＝边界持有 0＋劫持序列 7＝架构精确态）。

**037 行级对比**（`--frames 3000`；行 4-8 ＝ NMI 落在 BRK 7 周期窗口内的 5 档）：

| 行 | blargg 参照 | S0 基线 | S2 | 判定 |
|---|---|---|---|---|
| 1 | 27 36 00 | 27 36 00 | 27 36 00 | ✓ |
| 2-3 | 26 36 00 | 26 36 00 | 26 36 00 | ✓ |
| 4-5 | 36 00 00 | 26 36 00 | **36 00 00** | ✓（S2 修正） |
| 6-8 | 36 00 00 | 26（行6）/ 27（行7-8） | 27 36 00 | ✗（G5 残差，见下） |
| 9-10 | 27 36 00 | 27 36 00 | 27 36 00 | ✓ |

S0 5/10 → S2 **7/10**。**行 6-8 根因（S2 模型不可达，需独立 RE）**：`ppu_rendering.cpp` 的 VBL 块＝先 `X6502_Run(8)` 跑完 8 周期预算（CLC＋BRK 都完整执行）**再** `TriggerNMI()` ⇒ 线断言时刻量化在指令边界、对该 5 档时序总落在 BRK 完成之后；硬件要求线在 BRK 序列中段（1–5 周期窗口）置位并劫持向量。r5 的 NMIDELAY 扫参已证明无固定 D 同时满足 vbl_05 的派发 PC 与转变行（040 调参不可动）⇒ 周期级线断言精度是 G5 量化近似的独立缺口，不是 poll/hijack 模型问题。**037 最终判据＝`check_crc $B0D44BB7`（对 10 行打印流 CRC），行 6-8 不匹配 ⇒ CRC 失败 ⇒ FAIL 0x01。**

**变异验证**（纪律 3；apply→目标构建→红→revert→构建→绿；红面全部与预判一致）：

| 变异 | 施加点 | 红面（cpu_test 实测） |
|---|---|---|
| M-D1-2 | poll point 的 `g_intl_pending=1` 赋值短路 | 9 红：B1b/B1c/B5b/B5c/B5d/C4/C5/B6-1/B6-2（全部 NMI 派发基线；T-D1-2/3、T-D1-4a/b 不依赖该路径保持绿） |
| M-D1-2b | poll point 的 `g_intl_pending=2` 赋值短路 | 7 红：I2a-5/6/7、B7-3、T-D1-2c、T-D1-4d |
| M-D1-3 | hijack 向量 $FFFA→$FFFE | 2 红：B4b、T-D1-3d |
| M-D1-14 | `X6502_NmiHijacked` 消线去除 | 1 红：B2b（预算 7 下第二次派发不发生，可观测量＝线状态） |

其余读数（cpu_test 850/850、FTP 9红/2绿→1红/10绿、哨兵 6/6、ctest 36/36、金标三件套零漂移、闸门 #2 实际形态与 T-D1-4c 的 $204/$205 常数疑点）见 v2.0.2 计划 §十 **r18**。工具：`build/d1_s2_patch2.py`、`build/d1_s2_mutation.py`。

---
## 11. L2 同族摸底（S2 后，2026-11-07，数据节）

方法：`f11qa_blargg_runner --rom tests/fixtures/blargg/cpu/cpu_int_{3,4,5}_*.nes --frames 3000`（进程级并行，~5 s/ROM）。S0 基线＝`git checkout f9cbb1e -- src/x6502.cpp src/ops.inc src/ops_table.inc` ＋ `--target f11qa_blargg_runner` 目标域重建（~2 分钟）复跑，之后还原重建（还原后 037 行 4-5 复验 `36 00 00` ✓）。此节同时补齐 S0/S1 期未留档的 int_3/4/5 diag 基线。

### int_3（nmi_and_irq）——S0 **8/12** → S2 **7/12**，互有进退

| 行 | 期望（README） | S0 | S2 | 判定 |
|---|---|---|---|---|
| 1-4 | 23 00 / 21 00 / 21 00 / 20 00 | 同 | 同 | 两态 ✓ |
| 5-6 | 20 00（hijack：IRQ 处理器未运行） | **20 00 ✓** | 25 21 ✗ | **S2 回归** |
| 7-9 | 20 00 | 25 21 / 25 20 / 25 21 ✗ | 25 21 / 25 20 / 25 20 ✗ | 两态同错（G5） |
| 10-12 | 25 20 ×2（+1 行） | ✓/✗/✓ | ✓/✓/✓ | S2 得行 11 |

（两态均输出 12 行 vs README 11 行——行数差为两态共有，与 S2 无关；按位置对齐计分。）

**行 5-6 回归机理**：两拍模型下 NMI 在边界（`X6502_Run` 调用之间）置位时不被任何轮询点采样，已挂起的 IRQ（pending=2）照常派发；轮询点规则「NMI 不降级已挂起 IRQ」只在采样时刻生效。硬件语义＝NMI 在 IRQ 序列**向量化前置位则劫持向量**（§2 矩阵第 3 行：$FFFE→$FFFA、$FFFE 处理器不运行、irq_flag=00）。修复＝边界派发 IRQ 前对 NMI 线做一次向量级劫持检查——**属 S3 模型工作，非 G5**。行 7-9＝NMI 在 IRQ 序列执行中置位（硬件 7 时钟窗口）——本仓线断言无法落在序列中段，**G5**。

### int_4（irq_and_dma）——S0/S2 **逐字节一致**（校验和同为 `FB7832D0`）

全表统一提前一条指令（首列整体 −1：`0 +0 / 0 +1 / 1 +2 / 1 +3 / 2 +4 …` vs 期望 `0 +0 / 1 +1 / 1 +2 / 2 +3 / 2 +4 …`；末行 +526/+527 收敛）。⇒ **S2 零影响、缺口既有**。机理候选：线断言相位（G5）或 DMA 停止期间的周计入——**留 G5 RE 首步取证**。

### int_5（branch_delays_irq）——S0/S2 **逐字节一致**（校验和同为 `EF4967C7`）

test_jmp 10 行中 4 个对齐行提前一条指令（T+00/02/05/07：CK 各少 2/3/2/3、PC 各少 1），其余 ✓；分支特例行（test_branch_taken 的 `LDA $100`＝S5 目标）未到达（test_jmp 先失败，既有）。⇒ 对齐行提前＝断言/时序精度（G5 族）；特例行＝S5。

### 结论

- **G5（线断言周期级精度）是 int_2/3/4/5 四 ROM 的共同 PASS 阻塞**：int_2 行 6-8、int_3 行 7-9、int_4 全表、int_5 对齐行。
- S3 模型工作（NMI 劫持 IRQ 序列）预估把 int_3 提到 ~9/12，仍不能 PASS。
- **建议：G5 RE 先立项**（首步取证＝int_4 机理判定；S0 基线方法已留档本节）；S3 模型工作随后或并行设计。

## 12. G5 RE 三路调查（S2.5，2026-11-07，数据节）

> 三个并行 agent（架构/取证/约束）只读调查的收口记录；全程零构建。完整叙述见计划 §十 r20。

### 12.1 逐线量化清单（架构路）

| 线 | 硬件断言时刻 | 本仓断言时刻 | 粒度 | 偏差来源 |
|---|---|---|---|---|
| NMI（VBL） | sl241 dot1 flag set 后 ~1 CPU 周期 | VBL 块 `X6502_Run(8)` 预算内整数条指令跑完之后（ppu_rendering.cpp:1678-1701） | 指令边界，随 count 残量逐帧漂 | 8/3 周期预算实际消耗 = Σ(整条指令)∈[2.67,~7] 周期（过冲 E∈[0,4.3]）；断言与指令流耦合 |
| NMI2（$2000 边沿） | 写后下一周期 | store 指令体内（ppu.cpp:1304）+ loop-top 转换 | 指令边界 | 含 NMI2→NMI 转换（非 G5 缺口） |
| EXT 经 GameHBIRQHook（MMC3 系） | A12 上升沿（dot 精确） | PPU 点流中点级精确（ppu_rendering.cpp:2064-2078） | **点精确** | 观测侧等下一指令 poll |
| EXT 经 MapIRQHook（VRC/FDS 等） | mapper 计数器到 0 的任意周期 | 指令边界（x6502.cpp:930；hook 内部逐周期、对外折叠窗口起点） | 指令边界 | 窗口内相位存在但被丢弃 |
| DPCM | 样本耗尽的精确周期 | 同上（sound.cpp:747/846） | 指令边界 | 同 MapIRQHook |
| FCOUNT | fhcnt==29828/29829 精确周期 | 同上（sound.cpp:620-622） | 指令边界 | 事件落指令末 1-2 周期时**提前一条指令**派发（int_4 头部根因） |

### 12.2 延迟公式（S2 两拍模型）

- NMI（VBL）：总延迟 = 8/3 + E + L(I₁)，E=过冲 ∈[0,4.3] 逐帧漂移——cpu_int_5 对齐行提前的来源。
- IRQ（hook 侧）：事件窗口相位 p∈[0,L(W))，断言折叠为窗口起点 B；总延迟 = L(W)−p+L(I₁)。p≥L(W)−1 时硬件晚一条而本仓不会 ⇒ 统一提前一条（int_4 全表形态）。
- NMI（BRK 劫持特例）：VBL 线在 BRK 跑完后才断言 ⇒ 永远赶不上劫持在飞 BRK（int_2 行 6-8 / int_3 行 7-9 的直接机制）。

### 12.3 int_4 机理判定（取证路）

**首列语义**（4-irq_and_dma.s 解码）：handler 前 `tsx/dex/sta $100,x` 在栈上预写标记，IRQ 压栈的 **PCL 恰落标记位** ⇒ 首列 = 返回 PC 相对 `landing` 的偏移 = IRQ 在哪条指令后被取。IRQ 源 = APU frame counter（mode 0，`$4017` 写 0 启用）。+N = delay 参数（`delay 532-(dly)`）。

**判定：(a)+(b) 组合、(c) 排除**：

| 项 | 机理 | 证据 |
|---|---|---|
| (a) 线断言相位 | hook 窗口半开 (B_k, B_k+temp]：恰落指令结束边界的事件记入本指令、本指令尾 poll 可见 ⇒ 提前一条派发 | 头部奇偶档模式逐档吻合；18 档唯一拟合 T_ours=T_hw=−1（frame counter 绝对常数无错） |
| (b) DMA 周计入 | 本仓 B4014（ppu.cpp:1484-1516）恒 512 周期、无 dummy/对齐、无写周期奇偶（硬件 513/514） | INTLEDGER delta=519=512+7 实测钉死；尾部 +524..527 偏差 |
| (c) 派发记账 | —排除 | 压栈 PC 与 poll 所见逐档一致；assert→dispatch 同边界（delta=7） |

**交叉预测**：只修 (a)=16/18、只修 (b)=14/18、双修=**18/18**——实施后行级验收用。(b) 修复（ppu.cpp 补 513/514 **按写周期奇偶**，恒 514 会让 +527 反着错）会平移全部 sprite-DMA 时序敏感测试 ⇒ 独立立项、全量 ctest 复跑；DMC 仲裁（sprdma_and_dmc_dma）另立。(a) 修复归入方案 A 的线可见性门控（`X6502_IRQBegin`/`fceu11::IRQ`/`fceu11::NMI`/map_irq_hook 调用点打触发时刻戳，poll 点只采样「触发时刻严格早于本指令结束边界」的线）；注意不得推迟 `FrameSoundEvent` 的 APU 内部时钟（否则 kgmqa-121 与 APU golden 会动）。

### 12.4 候选设计对比（架构路）

| 方案 | 要点 | 改动面 | r12 合规 | 威胁面 | 量级 |
|---|---|---|---|---|---|
| **A. 线携带断言时刻（推荐）** | `_IRQlow` 电平语义不变，旁设 per-line `assert_at`（CPU 绝对周期）；VBL 块/hook 侧登记时刻；poll point/边界/BRK 劫持统一 `assert_at < now` 门控（now=指令记账末尾，timestamp 已预支；窗口 [起始,结束) 半开） | x6502.cpp（判定块/poll/TriggerNMI/IRQBegin 伴生表）+ ops.inc（劫持判定）+ ppu_rendering.cpp（VBL 块传时刻）+ 可选 sound/fds/boards hook 传相位 | ✓ 判定仍每指令一次 | 断言时刻改写后 NMI 派发相位整体移动 ⇒ 040/037/04-nmi 全套复跑（INTLEDGER 已就位可对账）；NMIDELAY 退化为断言时刻偏移旋钮 | ~150-250 行核心 |
| B. VBL 块重排 | 删 `X6502_Run(nd)`，NMI 断言登记为纯 PPU 时刻事件 | 同 A + VBL 块结构 | ✓ | 最大：r5 调参资产（X 表）全重采 | 中-大 |
| C. 指令内分段采样 | 长指令插到期检查点 | dispatch 结构 + ops_table.inc | ✗ 擦边（指令内第二观测点，与两拍模型精神冲突，需单独裁定） | 现有全绿面回归风险高；cpu_int_4 修复不保证 | 中 |

Mesen2 参照（NesCpu.cpp `EndCpuCycle`，每访问周期采样）只读对照：其断言精确性来自 PPU 被推到精确 master clock；本仓 PPU 已点级推进，量化只来自宿主结构 ⇒ 方案 A 在指令边界上近似其不动点，理论充分。

### 12.5 G5 验收门（约束路；实施 S2.5 时逐门复跑）

| 门 | 判据 | 验证命令（cwd=仓库根，PATH 前缀 vcpkg） | 当前值 | 允许方向 |
|---|---|---|---|---|
| G-1（硬） | 040（5.nmi_suppression）PASS | `f11qa-rom-runner --kgmqa-id kgmqa-040-vbl-05-nmi-timing-blargg --tests-json tests/tests.json --frames 600` | PASS 0x00；对 D 全扫不敏感 | 一字节不动 |
| **G-2（硬，须显式加回）** | **旧 05-nmi_timing PASS 且 X 表逐值不变** | `FCEUX11_E1_TRACE=1 f11qa_blargg_runner --rom tests/fixtures/blargg/ppu/vbl_05_nmi_timing.nes --frames 3000`（看 `E1 NMI_DISPATCH pc` 分布） | X=[4,4,4,3,3,3,3,3,3,2]（pc E356×3/E354×6/E352×1）；D=7/9 翻红 | X 表逐值不许变；⚠ **该 ROM 目前无任何门禁覆盖（见 12.7）** |
| G-3（硬） | ctest -LE perf 全绿 | `ctest -LE perf -j 4` | 36/36 | 计数可增不许红 |
| G-4（硬） | 金标三件套零漂移 | `f11qa_rom_regression_runner` / `fceux11_golden_savestate_test` | 780/0；8/8 | 漂移 ⇒ 停+记录+业主裁决（闸门 #3） |
| G-5（硬） | cpu_test 850/850 + FTP RED≤1 | `cd tests && ../build/tests/fceux11_cpu_test.exe` | GREEN=10/RED=1（4c=S5） | 10 绿不许翻红 |
| G-6（硬） | T2 哨兵 5 项（048/032/057/053/054） | 逐个 rom-runner | 全 PASS | 原样保留 |
| G-7（硬） | vbl_nmi_timing 7 ROM + 旧 ppu 套件 5 PASS | 逐个 blargg_runner | 全 PASS；旧套件 02/06/07/08/10 为有据 FAIL 基线 | PASS 项不许翻红；FAIL 项只许变好 |
| G-8（方向） | 037 行 6-8：`27 36 00`→`36 00 00` | rom-runner 看 diag 十行 | 7/10 | 只许向 `36 00 00` 收敛；行 0-5 不许回退；终判 = `check_crc $B0D44BB7` |
| G-9（方向） | int_3/4/5 G5 残差行收敛 | blargg_runner 直跑 | int_3 7/12、int_4 全表 −1、int_5 4 行 −1 | 只许匹配数增加；int_3 行 5-6 归 S3 模型工作，不许再恶化 |
| G-10（结构） | 新状态不入 savestate；探针 env-gated 默认静默；decoupling 白名单不新增 | 不设 env 跑 G-3/G-4 | 零成本 | 静默性是探针生命线 |

### 12.6 历史陷阱（改断言时刻最先看的 12 条，约束路）

1. **旧 05-nmi_timing 是最灵敏金丝雀（±1 dot 翻红）且当前无门禁**——改断言时刻后第一个跑它。
2. 派发 PC 与转变行被同一 D 同时移动——别再用 D 扫参闭合（r5 已证伪）。
3. **置位点不许动**（Step 1.1 证伪）：`PPU_status|=0x80` 挪 dot1 ⇒ set→clear=6819≠6820，01/03/09 全翻红。不变量是 set→clear=6820，非块总长。
4. **帧长不许动**：VBL 块内推 PPU ⇒ NMI-on 帧 6823/+3 漂移；症状 = `VBL_ENTER cycle` 不再恒 0（E1 trace 30 秒可验）。
5. **ppudead 路径（ppu_rendering.cpp:1570-1598）别顺手改**——有独立 P4-bridge 修复，Super Donkey Kong 依赖。
6. 抑制 marker 交互：`vbl_set_suppressed` 帧级消费；`X6502_Run(nd)` 抑制帧照常执行（帧长一致性）；NMI 取消类逻辑必须与读落点精度一起落地（vbl_04/05 误伤先例）。
7. **fresh-latch 已退休（r18）**——新方案再叠加任何「fresh 推迟一边界」= VBL NMI 多延迟一条（04-nmi_control #11 二次计数）。
8. **(241,0) 坐标失真**：`X6502_Run` 窗口内 PPU cycle 停驻，按 PPU 坐标判定的新代码都在失真坐标上工作——优先用 CPU 侧时间/相位。
9. 读侧与派发侧互不当证据（NMIDELAY 扫参对 vbl_02/06 零影响；07/08 转变行对 D 不敏感）。
10. 增量构建陷阱：动时序前确认 exe 与源码同步（E-1 会话一整张基线表作废的教训）。
11. 变异验证纪律：验收同时看**码值与形状**（X 表/十行 diag）——码值 PASS 而形状漂走 = 下一个「测试与实现同错」候选。
12. 探针名已换代（NMI_DEFER 已随 r18 消失；E1 delta 与 INTLEDGER 恒差 +7）——复用旧文档探针名 grep 会落空。

### 12.7 ⚠ kgmqa-040 门禁挂载漂移（治理发现，待业主裁决）

tests.json 的 kgmqa-040：legacy_id=`blargg_vbl_05_nmi_timing`、mirror_path=`blargg/vbl_nmi_timing/5.nmi_suppression.nes`、**blocking**。runner 按 mirror_path basename 派发 ⇒ **实跑 5.nmi_suppression.nes**（md5 `ef34f799`），与名字所示 ROM 不同。原始定义（370a3af，2026-08-06）确是旧 `vbl_05_nmi_timing.nes`（provenance 写明「Phase 1 commit 5581769 fixed vbl_05 to PASS…added as PASS monitor」）；9-24 镜像重整（6fa6b2f）换挂到 vbl_nmi_timing 套件 5 号 ROM，legacy 名未改，此后 r18/r19 里「040 调参不可动」的表述沿用了旧名（**对一半**：真正被 D=8 钉死的旧 05 已离开门禁）。

影响：① NMIDELAY 校准的唯一直接可观测量（旧 05 X 表）**无门禁覆盖**——S2.5 实施若替换 VBL 块结构，可以绿灯通过所有现行门禁同时静默丢掉 vbl_05 PASS；② 040 现跑的 5.nmi_suppression 对 NMIDELAY 不敏感（D∈{0,6,7,9,12} 全 PASS），它监控的是 VBL 块宏观形状，本身仍是有效门禁。

处置选项（**涉及 tests.json 期望值治理，业主裁决**）：(i) kgmqa-040 换回旧 05-nmi_timing（恢复门禁原意）；(ii) gate id 改名为 nmi_suppression 并**新增**一个 kgmqa 项挂旧 05（门禁+1）；(iii) 维持现状 + 文档声明 + G-2 作为 S2.5 的人工验收门。三路调查的约束 agent 建议至少做到 (iii)，(ii) 最完整。

## 12.8 S2.5 第一轮实施记录（2026-11-07，数据节）

**落地**：per-line 断言时刻表 + 可见性门控（poll point / BRK 劫持，严格 <）+ 精确注入器（`TriggerNMIAt` / `X6502_IRQBeginAt` / `fceu11_cpu_abs_ts`，x6502.h）+ legacy 注入器 `intl_stamp_legacy`（now−1，可见性与 S2 逐字节同）。锁测试 **T-D1-5 组 11 条**；cpu_test **861/861**；FTP 1红/10绿不变。**零生产行为变化实证**：037 `B1222C16`、int_3 `BBA6CEF0`、int_4 `FB7832D0`、int_5 `EF4967C7` 与 S2 轮逐字节一致；金标三件套零漂移。

**尝试 (a)：VBL 时刻化——回退**。`TriggerNMIAt(flag点 + δ)` 于预算授予前登记（δ=`FCEUX11_NMI_ASSERT_DELTA`，整数 CPU 周期）。δ 扫参（旧 05，600 帧）：

| δ | X 表 | 判定 |
|---|---|---|
| 0/1 | [3,2,2,2,2,2,1,1,1,1] | ✗ |
| 2/3 | **[4,3,3,3,3,3,3,2,2,2]** | ✗ 转变间距 6 行＝期望一致，**整体早 2 行** |
| 4 | [5,4,4,4,4,4,4,3,3,3] | ✗ |

期望 [4,4,4,3,3,3,3,3,3,2]。**结论：无整数 δ 可复现**——2 行早 = **2-dot 点级相位差**，整数周期粒度的 assert_at 表达不了；与 02-vbl_set_time 的已知 1-dot C₀ 偏差合流。037 在 δ=1 下行级交易（7/10→4/10：行 6-7 得 `36 00 00` 劫持、行 2-5 位移）——同一 2-dot 问题的另一面。**kgmqa-122 当轮转红咬中**（金丝雀首次实战）⇒ 回退。**下一轮章程**：点级时刻——`now` 的点粒度可从 count 残量导出（`3*timestamp + count/16`，grant 流位置），但须先厘清 grant 位相 vs 执行位相的 slack 语义（e1_vbl closure_assessment §3 的「残量抵消」），并与 02-vbl_set_time 的 C₀ 修正合议。

**尝试 (b)：APU FCOUNT 精确戳——回退**。`X6502_IRQBeginAt(FCOUNT, fsu_event_abs())`（fhcnt 派生事件周期：`win_end − cycles + (fhcnt_event − fhcnt_entry)`）。054 保持 PASS；int_4 头部行**非均匀位移**（`1+0/2+1/2+2/4+3/4+4/7+5...` vs S2 的统一早一条）——整体晚一条 + DMA 区异常 = APU 时序器相位 vs 测试 sync_apu 对齐的独立问题。⇒ 回退。**下一轮章程**：INTLEDGER 对照 dly=0 档的 assert-abs 与 landing 边界的距离，判定对齐差 1 周期的方向与归属（本仓 $4017 写同步 vs sync_apu 的偶周期假设）。

**变异验证**（apply→target 构建→红→revert→绿）：

| 变异 | 施加点 | 红面 |
|---|---|---|
| ms25a | `intl_nmi_live` 的时刻门控短路 | 1 红：T-D1-5a-5（门控消失 ⇒ 派发早一边界、压栈 PC=$201 ≠ $202） |
| ms25b | EXT 门控严格 < → ≤ | 1 红：T-D1-5b-3（边界恰落事件改记本条指令 ⇒ 压栈 PC=$201 ≠ $202） |
| ms25d | `X6502_IRQBeginAt` 戳折叠到 now（pre-S2.5 行为） | 1 红：T-D1-5b-3（登记时刻提前为调用时刻）。⚠ 5c 对该变异不敏感：精确戳与折叠戳都先于首次 poll 生效、派发点相同——已如实记录 |

## 12.9 S2.5 第二轮三章程 PLAN（2026-11-08 收口，待拍板实施）

> 三个只读 RE agent 的蒸馏结论（完整取证在会话产物 .zcode/d1s2/probe_{apu,vbl,dma}/）。实施顺序与耦合关系见拍板单；每阶段独立提交 + 全门禁 + 变异留证。

### 章程 DMA（513/514 奇偶）

- **语义钉死（三方证据：blargg 源码 + 18 行期望表 + INTLEDGER 硬数据 delta=519=512+7）**：OAM DMA = 1 dummy + 1-2 对齐（按写周期栅位奇偶）+ 512 传输；**偶 dly 档 514、奇档 513**；恒 513/恒 514 都错（+527 反着错）。
- **落点**：`ppu.cpp` B4014（:1484-1516），停机 ADDCYC 在 `cycle_before` 采样后、D4 探针 if/else 之前（逐字节等价保持）；校准位 `kOamDma514Parity=0`（偶序数→514，账本 row0 assert_abs=178872 偶 = dly0 landing 推导）；判别行 = int_4 **+526→8**。
- **回归面**：kgmqa-110（blocking PASS，最高风险门）、金标三件套（闸门 #3 停机门）、069（MMC3 IRQ 相位耦合）、122/040/054、int_2/3/5 校验和逐字节不变；sprdma 系读数平移 +1/+2（仍 FAIL，DMC 仲裁独立立项）。

### 章程 APU（frame IRQ 相位对齐）

- **机理（三缺陷叠加，公式本身正确）**：fsu_event_abs = 硬件 flag 置位沿（fb=R00+29831，18/18 实测）；非均匀位移 = ①mode-0 三连火（29828/29829/29830）last-wins 重戳抬高 +1/+2 → ②戳逐 poll 边界追赶 → ③`sta $4014` 的 poll now 被 DMA 计费抬高到 L+522（INTLEDGER 实测）吞戳。候选 (b) sync_apu 相位差、(d) 可见性方向均被证据排除（R00 全偶 18/18；严格 < 正确）。
- **设计**：戳 = fb + **上升沿守卫**（`X6502_IRQBeginAt` 加 `restamp` 缺省参数，重火不重戳）+ **(C) 采样点**（poll 采样取 hook 返回后时刻＝「DMA 停摆周期不属于任何指令的采样窗口」——这是 DMA agent「双修 18/18」缺的第三小块，d11 行的唯一解）。
- **预测表（int_4 18 行）**：仅戳 14/18 → +DMA 17/18 → **+(C) 18/18 → ROM PASS**（check_crc $43571959）。kgmqa-121 零影响（其锁不读 assert_at）；054 PASS 预期。
- **变异预案**：ms26a 去上升沿守卫（重戳回上轮表）/ ms26b 戳改回 win_end−1（回基线）/ ms26c fb→fb−1（d1 翻 0，钉死边界 vs 周期约定）/ ms25b 沿用。
- **⚠ 结构耦合**：(C) 与 DMA 停摆窗口语义必须同轮落地（DMA agent ②2.3 与本章程 2.1 落点 5 互为条件）。

### 章程 VBL（NMI 断言点级时刻）

- **机理改判**：「整体早 2 行」不是常数偏移——需要的阈值**逐行随 flag 边界的 count 残量 s=count/16 变化**（实测 10 行 s∈[−5,0] dots），常数 δ 必然全灭；**legacy（后置 TriggerNMI + NMIDELAY=8）全对的原因是窗口预算量化 n_w 与 s 耦合，代数证明 L_leg = L_hw = 2q+2 对全体 s 成立（K=13）**——NMIDELAY 调参在 s 轴落对了位置。
- **账本恒等式**：`count = 16·G − 48·T` ⇒ `G = 3T + count/16`（grant 流，dots；帧账本零和无漂移）。**G 作锚点对、作比较轴错**（同一 grant 窗口内 G 不变——窗口简并陷阱）；点级信息全部在锚点处被 floor 吸收，**比较留在周期轴**。
- **设计**：flag 置位处一次读 `T_flag` + `s_flag = count>>4`，`TriggerNMIAt(T_flag + floor_div(s_flag+K, 3))`（K=13 标定常数、绑定 D=8，四成分记账：/NMI+2、C₀+1、采样约定、纪元差；floor_div 须真 floor——负 s 实测到 −5）；**删 legacy 后置 TriggerNMI()**（二次戳会用 now−1 覆盖精确戳）；比较点/门控/T-D1-5 十一条**零改动**；ppudead 路径与 NMI2 转换戳不碰。
- **预测**：X 表 [4,4,4,3,3,3,3,3,3,2] **按构造逐值不变**（金丝雀不咬）；040 不动；**037 = 判决性实验**——设计的可见点同等或更晚于 legacy，而行 6-8 需要更早 ⇒ 纯 K=13 大概率修不动 037；若证实 → 升级显式 `phase_debt` 追踪（`阈值 = floor((s+K−debt)/3)`，第二小步）。C₀ 修正（flag 0→1 dot）为 K 的判决性消融实验（13→12 预期）。
- **变异预案**：ms25a/b/d 仍红；新增 K±1（X 表必须可观测移动）与 `count>>4→0`（G-2 必红——防「测试与实现同错」第三例）。

### 集成拍板单

| 阶段 | 内容 | 预测（int_4） | 关键停机门 |
|---|---|---|---|
| 1 | DMA 513/514 | 13/18 | 110 + 金标 + 069 |
| 2 | APU 戳 + 上升沿守卫 | 17/18 | kgmqa-121 + 054 |
| 3 | (C) 采样点（含停摆窗口语义） | **18/18 → PASS** | 4-nmi_and_dma 专项 |
| 4 | VBL 点级锚（K=13） | 不涉 | 122 金丝雀 + G-2 人工门 + vbl_04 |

---
*调查页完（S0+S1+S2+S2.5-RE）。S3 起的实施记录记 v2.0.2 计划 §十 变更记录，本页只补数据节。*
