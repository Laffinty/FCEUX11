# E-1 Step 1.2 — NMI 取消时序缺口小修：实测 + 回滚（2026-08-03）

> **分支**：`wip_1.16`（HEAD `3f3ce1c`）
> **性质**：按闭合评估 §4 实施的"独立小修"实验——**实测证伪其可独立落地性**，已回滚（方案 §0.2：任一红即回滚）。
> **结论**：`(241,0-1)` 读取消 NMI 的语义缺口**真实存在**且修复机制本身正确（探针证实取消生效），
> 但**无法独立落地**——模拟器 ~2-3 dot 帧边界偏移把 vbl_04/05 的 VBL 轮询读也错放到 (241,0)，
> 取消会误伤其 NMI（vbl_04 PASS→0x03、vbl_05 PASS→0x01）。**与深模型（CPU 侧读时序）绑定**，
> 记录为有据已知限制，为深模型铺路。

---

## 1. 目标缺口（闭合评估 §4 复述）

`A2002` 在 `(241,0-1)` 执行 `X6502_IRQEnd(FCEU_IQNMI)` 是 no-op：
读发生在 VBL 块 `X6502_Run(nd)` 期间（`ppu_rendering.cpp:1642`）、**早于** `TriggerNMI()`（`:1648`），
NMI 尚未 latch，无事可清；随后 `TriggerNMI` 照常 latch → NMI 照发。硬件语义：读置位点拉高 /NMI，
CPU 下一边界不采样。

## 2. 基线探针实证（零改动，`FCEUX11_E1_TRACE=1`，vbl_06 × 600 帧）

日志：`build/e1_baseline_vbl06_trace.log`（303,244 行，gitignored）。

```
172601: E1 P2002_READ abs=3067426 sl=241 cycle=0      ← (241,0) 读，X6502_Run(8) 期间
172602: E1 VBL_AFTER_NMIDELAY abs=3067426 sl=241 cycle=0 delay=8
172603: E1 NMI_SET abs=3067426 (path=VBL)             ← TriggerNMI 照常 latch
172604: E1 NMI_DEFER … E1 NMI_DISPATCH                 ← NMI 照常派发
```

统计：10 帧 `VBlankON=1`（10 行测量帧，每行一帧）→ 10 次 `NMI_SET`，**0 次被取消**。缺口确认。

## 3. 修复实施（已回滚，代码保持基线）

- `src/ppu.h`：声明 `fceu11_ppu_mark_nmi_read_cancelled()` / `fceu11_ppu_take_nmi_read_cancelled()`
- `src/ppu.cpp`：`(241,0-1)` 分支由 `X6502_IRQEnd` 改为置位帧级标记（保留 `X6502_IRQEnd` 为防御性 no-op）+ `E1 P2002_READ_CANCEL` 探针
- `src/ppu_rendering.cpp`：VBL 块 `X6502_Run(nd)` 后取标记，`if (!vbl_set_suppressed && !nmi_read_cancelled) TriggerNMI();`
  探针 `E1 VBL_AFTER_NMIDELAY` 增加 `nmi_cancel` 字段

## 4. 修复后探针（同环境，vbl_06 × 600 帧）

日志：`build/e1_fixed_vbl06_trace.log`（gitignored）。

```
44829: E1 VBL_ENTER … VBlankON=1
44830: E1 VBL_AFTER_NMIDELAY abs=565837 sl=241 cycle=0 delay=8 nmi_cancel=1
44831: E1 W2000 …                                ← 无 E1 NMI_SET！取消生效
```

统计：10 帧全部 `nmi_cancel=1`，**0 次 `NMI_SET`**（基线 10 次）。取消机制**正确生效**。

## 5. 实测结果与回归（关键数据）

### vbl_06 逐行（$6000 仍 0x01，FAIL 面移动）

| 行 | 期望 | 基线 | 修复后 | 说明 |
|---|---|---|---|---|
| 00-03 | `- N` | `- N` ✅ | `- -` ❌ | 期望 NMI 触发，被取消 |
| 04 | `- -` | `V N` ❌ | `- -` ✅ | 置位抑制 |
| 05-06 | `V -` | `V N` ❌ | `V -` ✅ | **目标达成** |
| 07-09 | `V N` | `V N` ✅ | `V -` ❌ | 期望 NMI 触发，被取消 |

根因：全部测量读在模拟器量化到 **(241,0)**（行 0-3 在 Δ=0 落点、行 4-9 在 Δ=+4 落点，仅 2 个离散点）。
修复使 (241,0) 的取消真实生效 → 行 4-9 全部取消。行 0-3/7-9 的 `N` 基线是"读错位 + 取消失效"的**巧合正确**。

### 硬基线回归（方案 §0.2 → 触发回滚）

| ROM | 基线 | 修复后 | 影响 |
|---|---|---|---|
| vbl_01_basics | PASS 0x00 | PASS 0x00 | ✅ |
| vbl_03_clear_time | PASS 0x00 | PASS 0x00 | ✅ |
| vbl_04_nmi_control | PASS 0x00 | **FAIL 0x03** | ❌ 回归 |
| vbl_05_nmi_timing | PASS 0x00 | **FAIL 0x01**（diag `00 255` = NMI 未触发） | ❌ 回归 |
| vbl_09_even_odd_frames | PASS 0x00 | PASS 0x00 | ✅ |

其余：vbl_02/06/07/08/10 维持 FAIL（码不变）。Oracle A（`ctest -LE perf`）修复前后均 **34/34**。

## 6. 根因：缺口修复与深模型绑定

vbl_04/05 的 VBL 轮询读同样落在 (241,0)（模拟器 ~2-3 dot 帧边界偏移，评估 §2/§3 同源）。
硬件上这些读发生在置位**之前**（sl240 段），不触发取消；模拟器偏移使其错放 (241,0)，
取消一旦生效即误伤本应触发的 NMI（vbl_05 X=255 = 测量帧 NMI 全程未触发）。

**推论**：`(241,0-1)` 取消语义在"读落点正确"时是硬件正确的，但当前模拟器所有 VBL 附近读
都被量化到同一指令边界 → 无法区分"应取消"（vbl_06 行 5-6）与"不应取消"（vbl_04/05 轮询）。
修复的落地点位依赖深模型（CPU 侧亚指令级读时序 / 帧边界偏移修正），**收敛无保证**。

## 7. 处置

1. **回滚**（已执行）：`git checkout -- src/ppu.cpp src/ppu.h src/ppu_rendering.cpp`，工作区干净。
   重建后 vbl_04/05 恢复 PASS（待验证，见 §8）。
2. **记录为有据已知限制**：NMI 取消缺口 = 真实正确性缺口，修复方案已设计并验证机制正确，
   落地前置 = 深模型（与 vbl_02/06 读落点量化同族）。
3. **保留**：探针日志（gitignored，供深模型调查复用）；方案 §3 Step 1.2 与闭合评估文档
   标注"取消缺口已实测验证机制、回滚待深模型"。

## 8. 回滚验证（2026-08-03）✅

回滚（`git checkout -- src/ppu.cpp src/ppu.h src/ppu_rendering.cpp`）→ 全量重建 → 复跑：

| ROM | 基线 | 修复后 | 回滚后 |
|---|---|---|---|
| vbl_01_basics | PASS | PASS | PASS ✅ |
| vbl_02_set_time | FAIL 0x01 | FAIL 0x01 | FAIL 0x01 ✅ |
| vbl_03_clear_time | PASS | PASS | PASS ✅ |
| vbl_04_nmi_control | PASS | **FAIL 0x03** | **PASS** ✅ 恢复 |
| vbl_05_nmi_timing | PASS | **FAIL 0x01** | **PASS** ✅ 恢复 |
| vbl_06_suppression | FAIL 0x01 | FAIL 0x01（行模式变） | FAIL 0x01（`- N` 恢复）✅ |
| vbl_07_nmi_on_timing | FAIL 0x01 | FAIL 0x01 | FAIL 0x01 ✅ |
| vbl_08_nmi_off_timing | FAIL 0x01 | FAIL 0x01 | FAIL 0x01 ✅ |
| vbl_09_even_odd_frames | PASS | PASS | PASS ✅ |
| vbl_10_even_odd_timing | FAIL 0x03 | FAIL 0x03 | FAIL 0x03 ✅ |

Oracle A（`ctest -LE perf`）：回滚后 **34/34**。回归归因确认 = NMI 取消修复本身。

---
*实验完。结论：缺口真实、机制正确、落地绑定深模型 → 有据已知限制，不排入当前序列。*
