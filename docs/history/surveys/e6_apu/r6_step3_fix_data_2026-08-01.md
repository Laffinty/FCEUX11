# E-3 APU R6 Step 2b — Defect 1 两次修复尝试失败记录 (2026-08-01)

> **目的**：诚实记录 R6-2b（缺陷 1：帧计数器相位）的两次失败尝试 + 反汇编/probe 发现，避免后续重复踩坑。
>
> **状态**：两次尝试均已回滚。R6-2a（缺陷 2）修复保留在 `b710c68`。缺陷 1 仍未修复。
> **分支**：`wip_1.16`（HEAD `b710c68`，回滚后工作树仅余诊断注释）

---

## 1. 缺陷 1 概述（R6 Step 1 probe 实证）

- `FCEUSND_Reset` 设 `fcnt=0`；`FrameSoundUpdate` 在 `fcnt==0` 置 IRQ
- 上电后首个 quarter-frame（~7457 cyc）就置 IRQ，硬件要求 ~29828 cyc（第 4 个）
- 影响 5/7 APU FAIL：`apu_single_4/5/6` + `apu_reset_4017_timing/written`

## 2. 尝试 1：`FCEUSND_Reset` 设 `fcnt=1`（已回滚）

**假设**：硬件上电等效一次 $4017=$00 写（Write_IRQFM 设 fcnt=1），所以 Reset 也应 fcnt=1。

**结果**：
| 项 | 结果 |
|---|---|
| apu_single_4/5/6 | 仍 FAIL 0x02（未修复） |
| apu_reset_4017_timing | 0x02 → 0x03（**退化**） |
| apu_reset_4017_written | 仍 FAIL 0x02 |
| ctest | **golden_savestate_test + savestate_regression_test FAIL**（12 ROM 哈希全变，fcnt 是 savestate chunk 运行期值） |
| apu_single_3（R6-2a） | PASS（保留） |

**结论**：fcnt 起始值不是缺陷 1 根因。且 fcnt 变更碎 golden savestate（须重生，代价高）。

## 3. 尝试 2：IRQ 置位位置 `if(!fcnt)` → `if(fcnt==3)`（已回滚）

**假设**（§十 R6 方案 A 字面）：IRQ 应在 4-step 序列"末位"（fcnt==3）置，而非"首位"（fcnt==0）。

**结果**：
| 项 | 结果 |
|---|---|
| apu_single_4/5/6 | 仍 FAIL 0x02（未修复） |
| apu_reset_4017_timing | 0x02 → 0x03（**退化**） |
| apu_reset_4017_written | 仍 FAIL 0x02 |
| ctest | **34/34 PASS**（意外：fcnt==3 置位未碎 golden，因 IRQ 位最终状态相同） |
| apu_single_3（R6-2a） | PASS（保留） |

**结论**：IRQ 置位位置（fcnt==0 vs fcnt==3）也不是缺陷 1 根因。两次尝试 IRQ 实际都在第 4 个 quarter 置位，结果相同。

## 4. 反汇编发现（apu_single_4_jitter.nes）

### 4.1 测试核心（CPU 0xE200-0xE21F）

```
E202: LDA #$40
E204: STA $4017        ; 写 inhibit (清 IRQ)
E207: LDA #$00
E209: STA $4017        ; 写 enable (计时起点)
E20C-E219: (定时: LDA #$74 → JSR E358 + LDA #$4D → JSR E342)
E21A: LDA $4015        ; 读 IRQ 标志
E21D: AND #$40         ; 测 bit6
E21F: RTS
```

### 4.2 PASS/FAIL 判定（CPU 0xE249-0xE250）

```
E249: LDA $4015
E24C: AND #$40
E24E: BEQ $E253   ; bit6=0 (IRQ 未置) → E253 (PASS 路径)
E250: JMP $E51F   ; bit6=1 (IRQ 已置) → FAIL "too soon"
```

**blargg 期望：enable 写后经定时器，读 $4015 时 IRQ 尚未置位。**

### 4.3 定时器长度估算

- E20E `LDA #$74`(116) → JSR E358：E358 循环 116 次 × E342($D7=215)
- E342(215) 走 E440 分支（A≥7）≈ ~1740 cyc
- 总 ≈ 116 × 1740 ≈ **202,000 cyc ≈ 27 quarters**

⚠️ 反汇编器对 E342/E440 的分支计算不可靠（多级 BCS/BCC 嵌套），此估算仅供参考。

## 5. probe 数据决定性证据

enable 写（L41 `W4017 V=0x00`）后，FSU 序列 fcnt=1→2→3→0，**IRQ 在第 4 个 quarter（fcnt=0）置位**（约 29828 cyc）。这与 NES 标准 4-step 帧 IRQ 语义一致。

**但 blargg 仍 FAIL** → 两种可能：
1. blargg 定时器 < 4 quarters，期望 IRQ 未置，但我们第 4 quarter 置了（若是，fcnt=1 或 fcnt==3 应能修，但都 FAIL → 排除）
2. **缺陷 1 的根因不在 fcnt/IRQ 位置**，而在别处——最可能是：
   - `FCEU_SoundCPUHook` 的 fhcnt 增量粒度（`cycles*48` 的舍入误差累计）
   - `Write_IRQFM` 的 `fhcnt=fhinc` 重置使首个 quarter 起点偏差
   - length counter 的 clock 时机（`FrameSoundStuff` 内部）

## 6. 结论与下一步建议

### 6.1 已排除的假设

| 假设 | 结果 |
|---|---|
| fcnt 起始值（0 vs 1） | ❌ 非根因 |
| IRQ 置位位置（fcnt==0 vs fcnt==3） | ❌ 非根因 |

### 6.2 未排除的假设（按可能性排序）

1. **`fhcnt` 增量/重置粒度**：`FCEU_SoundCPUHook` 用 `cycles*48` 递减 fhcnt，`Write_IRQFM` 用 `fhcnt=fhinc` 重置。若 `cycles` 的单位或 48 因子与硬件 quarter 周期（7457.5 cyc）有舍入误差，累计到 blargg 的 27-quarter 窗口会显著偏移
2. **length counter clock 相位**：`apu_single_5_len_timing` 报 "First length of mode 0 is too soon"——length clock 在 fcnt=0 时过早触发，可能需在 fcnt==3（序列末）才 clock
3. **FrameSoundStuff 内部**：具体哪个 counter 在哪个 fcnt step clock，需逐项核对

### 6.3 建议

- **暂缓 R6-2b**：缺陷 1 根因比 §十 R6 处方描述的深，需要 instrument-first 深入 fhcnt 时序（记录每次 FCEU_SoundCPUHook 的 fhcnt 精确轨迹 + blargg 定时器实际周期）
- **保留 R6-2a**（已 push 前 commit `b710c68`）：apu_single_3 修复有效且无回归
- **golden savestate 未碎**（回滚后），无需重生

## 7. 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | 两次尝试均已回滚（`if(!fcnt)` 恢复） | ✅ |
| 2 | ctest 34/34 PASS（回滚后重建） | ✅ |
| 3 | R6-2a apu_single_3 仍 PASS | ✅ |
| 4 | apu_single_4 回到基线 FAIL 0x02 | ✅ |
| 5 | golden savestate 未碎（无需重生） | ✅ |
| 6 | 反汇编 + probe 发现已记录 | ✅ |
| 7 | 未 commit 任何失败尝试 | ✅ |

---

*诊断记录完。缺陷 1 需更深入调查（fhcnt 时序），非本次 R6-2b 可闭合。*
---

## 8. 尝试 3：移除 Write_IRQFM 的 fhcnt=fhinc 重置（已回滚，2026-08-01 下午）

**假设**（R6 Step3 probe 深挖后）：`Write_IRQFM` 每次写 $4017 都 `fhcnt=fhinc` 重置，强制写后第一个 FSU 再等满 7457 cyc → IRQ 固定在写后 4 quarters（fcnt 1→2→3→0）置。硬件上帧计数器周期连续，写只重排 fcnt 相位 → IRQ 应更早/按自然相位。

**新 probe**（本轮新增，诊断用）：
- `E3 HOOK_SAMPLE`：每 4096 次 hook 调用记录 fhcnt（看长期漂移）
- `E3 HOOK_TRIG`：每次 quarter 触发记录 fhcnt_before/after（看边界稳定）
- `E3 R4015`：每次 $4015 读记录 fcnt/fhcnt/sirq（定位 blargg 读时刻）

**fhcnt 漂移分析结论**：HOOK_TRIG 的 fhcnt_before range [24, 336], mean=82 → quarter 边界非常稳定，**无累积漂移**（否定了 fhcnt 漂移假设）。

**结果**：
| 项 | 结果 |
|---|---|
| apu_single_4 | 仍 FAIL 0x02（未修复） |
| apu_single_3（R6-2a） | PASS（保留） |
| ctest | golden_savestate + savestate_regression FAIL（fhcnt 是 FHCN chunk 运行期值） |
| 回滚 | ✅ fhcnt=fhinc 恢复 |

**决定性 probe 数据（R4015）**：
```
FSU fcnt=3  sirq=0x0      ← 第 3 个 FSU
FSU fcnt=0  fhcnt=0 sirq=0x0  ← 第 4 个 FSU（IRQ 内置）
R4015 fcnt=1  fhcnt=357960  sirq=0x40  ← blargg 读时 IRQ 已置
```
enable 写后 IRQ 在 ~4 quarters 置，blargg 在 ~4.5 quarters 读 → 已置 → FAIL。

**结论**：三次假设（fcnt 起始值 / IRQ 置位位置 / fhcnt 重置）全部证伪。缺陷 1 根因在 **blargg wait_n 定时器确切语义**层面——需精确反汇编 E342/E358/E440 循环算 blargg 期望的 enable→IRQ 时延，超出当前会话可靠范围。**暂停缺陷 1**，保留 R6-2a（apu_single_3 PASS）为 R6 的净成果。

## 9. R6 当前净成果与状态

| 项 | 状态 |
|---|---|
| 缺陷 2（$4017 清标志） | ✅ 修复（R6-2a，apu_single_3 PASS，apu_test 推进越过 irq_flag） |
| 缺陷 1（帧计数器相位） | ⏸ 暂停（3 次假设证伪，根因在 blargg 定时器语义） |
| Oracle A ctest | ✅ 34/34（probe 静默时） |
| golden savestate | ✅ 未碎（回滚后） |
| wip_1.16 HEAD | `f66b042`（诊断记录）+ 本轮 probe（未 commit） |
