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

---
*调查页完（S0+S1+S2）。S3 起的实施记录记 v2.0.2 计划 §十 变更记录，本页只补数据节。*
