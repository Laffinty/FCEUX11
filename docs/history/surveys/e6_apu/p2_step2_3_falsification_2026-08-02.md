# E-3 APU P2 Phase 2 Step 2.3 — 5-step 立即 clock 触发条件 证伪记录 (2026-08-02)

> **目的**：按 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 2 Step 2.3 执行改法
> （`src/sound.cpp:1105` `if(V&0x2)` → `if(V&0x1)`），instrument-first 实测证伪。
>
> **依据**：NESdev Wiki「APU Frame Counter」side effects——**bit7（5-step mode）写触发立即
> quarter+half clock，bit6（inhibit）写只清 IRQ 标志、不触发 clock**；blargg `apu_test` #01/#02
> 源码语义（$80 写应立即 clock length，$C0 亦应 clock）。
> **分支**：`wip_1.16`（HEAD `364928f`，工作树直接改）
> **作者**：独立执行（2026-08-02）

---

## 1. 位映射事实（方案前提的错误所在）

`Write_IRQFM` 现有代码：

```cpp
const uint8 raw = V;
V=(V&0xC0)>>6;          // $00→0x0, $40→0x1, $80→0x2, $C0→0x3
fcnt=0;
if(V&0x2)               // = 原始 bit7（5-step mode 位），非 inhibit
 FrameSoundUpdate();
fcnt=1;
```

`(V&0xC0)>>6` 是自然映射：**原始 bit6（0x40, inhibit）→ 缩减值 bit0（0x1）**；
**原始 bit7（0x80, 5-step mode）→ 缩减值 bit1（0x2）**。因此：

- 当前 `if(V&0x2)` 触发条件 = 原始 bit7 = **5-step 写** → 与 NESdev 硬件语义一致；
- 方案 Step 2.3 声称"当前 `V&0x2` 是 inhibit"——**与代码事实相反**；改为 `V&0x1` 反而会让
  `$40`（inhibit）触发 clock、`$80`（5-step）不触发，正好颠倒硬件行为。
- 方案注中"`$80` 写同时置 bit6+bit7"亦误：`$80=0b10000000` 只置 bit7；同时置二者的是 `$C0`。

`IRQFrameMode` 消费方同样印证自然映射：`FrameSoundUpdate` 中 `IRQFrameMode&0x2` 用于 5-step
额外周期（`sound.cpp:518`），即 bit1 = 原始 bit7。**不存在方案所担心的 bit 映射 swap。**

## 2. 实测结果（同构建二进制，`FCEUX11_E3_TRACE=1`，apu_01_len_ctr × 600 帧）

### 2.1 W4017 → 立即 FSU 行为对比

| 写值 | 基线（`V&0x2`） | 改动版（`V&0x1`） | 硬件期望 |
|---|---|---|---|
| `$40`（inhibit） | IN → OUT 之间**无** FSU（下一 FSU 在 ts=7473 正常 quarter） | IN → **立即** `E3 FSU ts=14 fcnt=0` → OUT | 不 clock，仅清 IRQ |
| `$80`（5-step） | IN → **立即** `E3 FSU ts=425 fcnt=0` → OUT | IN → OUT 之间**无** FSU | **立即 clock** length+sweep |

结论：当前实现（`V&0x2`）符合硬件；方案改法（`V&0x1`）使行为**反转**。

### 2.2 APU 全量回归（52 ROM，改动版）

| 结果 | 数量 | 说明 |
|---|---|---|
| PASS | 36 | 含 apu_01~11、apu_single_2/3/7/8、pal_apu_*（10）等 |
| FAIL（harness 参数类，非本改动引入） | 11 | mixer ×4（未传 `--frames 2400`，0x80）、reset ×5（未传 `--reset-after`，0x81）、sprdma ×2（已知 FAIL） |
| FAIL（既有已知限制，未变） | 3 | `apu_single_5/6` 0x04（hook 量化）、`apu_reset_4017_timing` 0x03、`apu_reset_4017_written` 0x02、`apu_test` 0x01 |
| **FAIL（本改动真实回归）** | **1** | **`apu_single_1_len_ctr` 0x00 → 0x04** |

**关键回归**：`apu_single_1_len_ctr` 基线 PASS（0x00，本实验前后两次复核），改动版 FAIL 0x04
（diag "Channel: 0"）。改动后回滚重建，该 ROM 复回 PASS 0x00。

> 注：apu_01/02/06 等 ROM 改动版仍 PASS，是因为它们几乎不写 $4017（apu_01 全 600 帧仅 5 次
> $4017 写），未覆盖"$80 立即 clock"路径——**PASS 不构成对该行为的背书**，硬件语义 + single_1
> 回归才是判据。

## 3. 结论

1. **Step 2.3 前提证伪**：当前 `if(V&0x2)` 已是"bit7（5-step）写触发立即 clock"的硬件正确实现，
   方案所述的"当前是 inhibit、需改 V&0x1"不成立。
2. **方案改法方向相反**：落地 `V&0x1` 会令 `$40`（inhibit）错误触发 clock、`$80`（5-step）错误
   不触发，并实测使 `apu_single_1_len_ctr` 翻红；streemerz 等依赖 `$80` 立即 sweep clock 的
   游戏亦会受影响。
3. **本步无代码改动可落地**：方案 Phase 2 Step 2.3 判为**证伪/无需执行**（与 Phase 1 Step 1.1
   同类处置），不产生独立 commit，不留已知错误触发条件需要"拿掉"。

## 4. 处置

- ✅ 已回滚：`src/sound.cpp` 恢复 HEAD 状态（工作树 `git status` 干净，UTF-8 + CRLF 与 autocrlf
  归一化后 diff 为空）。
- ✅ 已重建 `fceux11_blargg_runner.exe`（回滚后），apu_single_1/01/02/06/single_3/04 复核 PASS。
- 📝 方案文档 §2/§4 Step 2.3 已就地标注证伪。

## 5. 修订后的 Phase 2 方向

| 步骤 | 内容 | 依据 |
|---|---|---|
| ~~Step 2.3~~ | ~~`V&0x1` 触发条件修正~~ | **证伪**（本文档）：当前 `V&0x2`=bit7 已符合硬件；改法反向并回归 single_1 |
| **Step 2.2 深化（唯一硬骨头）** | 消除 hook 量化方差（cycle-accurate quarter crossing），闭合 single_5/6 + reset_4017_timing | `p2_step2_2_data` §2.3：offset/fhinc/分数/tsdelta 全部证伪，参数级不可收敛 |
| **Phase 1 Step 1.3（同族）** | CPU 侧 NMI/读采样时序模型 | E-1 剩余 6 ROM 同源（帧边界 CPU↔PPU 偏移 ~2-3 dot） |

**建议**：Step 2.2 深化与 Phase 1 Step 1.3 同为"消除指令级量化"工作（cycle-accurate 采样模型），
按方案 §2 合并评估、独立 PR 落地；之后再进 Step 1.4 → Phase 3。

---

*证伪记录完。Step 2.3 无需执行，下一步为 Step 2.2 深化 / Phase 1 Step 1.3 合并评估。*
