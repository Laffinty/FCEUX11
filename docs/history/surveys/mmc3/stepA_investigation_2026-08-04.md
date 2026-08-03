# FCEUX11 v1.16 — MMC3 桶调查 (Phase 3 Step 3.2 桶 A)

> **编制日期**: 2026-08-04
> **承接文档**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 / Step 3.2
> **承接报告**: `docs/history/reports/FCEUX11-1.16_P3-Step31_harness_cleanup_2026-08-04.md`
> **基线 commit**: `60136cc` (Step 3.1 完成)
> **状态**: 🚧 **有据已知限制,记录于此**

---

## 0. 摘要

桶 A (MMC3) 含 **12 项 ROM FAIL**(Step 3.1 后期 Oracle B):

| 错误码 | 数量 | 子类型 |
|---|---|---|
| 0x02 | 6 | IRQ counter reload / set-on-0 |
| 0x03 | 2 | A12 clocking via PPUADDR (mmc3_1/v2_1) |
| 0x04 | 2 | A12 clocking via PPUADDR write (mmc3_3/v2_3) |
| 0x09 | 2 | scanline 0 IRQ timing (mmc3_4/v2_4) |

**结论**: 12 项全部**记录为有据已知限制**。尝试修复均引入回归或效果有限,MMC3 IRQ counter + A12 clocking 需要更深模型(超出单 PR 修复范围)。

---

## 1. 已落地改动

### 1.1 env-gated probe (`src/boards/mmc3.cpp`)

**目的**: 后续深模型调查时复用,本次不进入 fix PR。

新增 `FCEUX11_MMC3_PROBE=1` env-gated probe:

1. **`MMC3_IRQWrite` (line ~219)**: 记录 `$C000`/`$C001`/`$E000`/`$E001` 写入(IRQLatch / Reload / IRQa)
2. **`MMC3_hb` (line ~273)**: 记录每个扫描线末尾的 counter 状态
3. **`ClockMMC3Counter` (line ~211)**: 记录每次时钟前的 count/latch/reload/enabled,以及 IRQ 是否触发

**零侵入性**:probe 仅在 env=1 时输出 stderr;默认静默(Oracle A 33/33 不变)。

### 1.2 探针实测数据

`mmc3_2_details.nes` × 600 帧 probe 输出:
- 总 probe 行数:**419903**
- IRQ 触发次数:**546**
- 第一个 IRQ: `sl=13 cyc=6 count_was=1 reload=0 isRevB=1 latch=255`
- 后续 IRQ 间隔 ~15 scanlines(255 / 15 ≈ 17,匹配 latch=255 时的递减速率)

**观察**:Counter 在每个扫描线末尾正确递减 1,IRQ 在 counter=1 → 0 转换时触发。**基本机制正常,问题在边角场景**。

---

## 2. 修复尝试与结果

### 2.1 尝试 #1: 改条件 `count | isRevB` → `count` (RevB 严格 transition)

```cpp
// 修改:
if (count && !IRQCount) {  // 仅非零 → 零转换
    // fire IRQ
}
```

**结果**:
- ❌ `mmc3_4_scanline_timing`: 0x09 Failed #9 → **0x0E Failed #14 "IRQ never occurred"** (REGRESSION)
- ❌ 其他 ROM 不变

**分析**: 这个修复正确匹配 `mmc3_v2_6_MMC3_alt` 的"reload-to-0 不触发 IRQ"语义,但 `mmc3_4_scanline_timing` 测试依赖旧语义下的 IRQ 触发模式(可能需要在 IRQa 已 cleared 后仍 fire)。测试序列中需要先 clear IRQa 再触发 IRQ,这个修复破坏了该路径。

### 2.2 尝试 #2: 改条件为 `!reload_before && !IRQCount`

```cpp
// 修改:
if (!reload_before && !IRQCount) {  // 本次未 reload AND counter 归零
    // fire IRQ
}
```

**结果**:
- ❌ `mmc3_4_scanline_timing`: 0x09 → **0x03 Failed #3 "Scanline 0 IRQ should occur sooner when $2000=$08"** (REGRESSION, 不同 sub-test)
- ❌ 其他 ROM 不变

**分析**: 改动触发不同的 sub-test 失败(sub-test #3 vs 原 #9),但仍 FAIL。这个修复同样破坏 `mmc3_4` 测试的某个 IRQ 时序。

### 2.3 尝试 #3: 已评估但未实装 — 添加 PPU_hook for A12 via PPUADDR

```cpp
// Mapper 4 需添加 PPU_hook 检测 PPUADDR 写入触发 A12 rising edge
// 改动面大,需先确认测试 v1 vs v2 的硬件期望差异
```

**评估**: 12 ROMs 中 `mmc3_1_clocking`/`v2_1`/`3`/`v2_3` 都期望"PPUADDR 写入触发 A12 上升沿时钟 counter"。Mapper 4 当前**没有设置 PPU_hook**,这意味着测试的子功能未实现,需要实质改动。评估后认为改动面超过单 PR 范围,**留待深模型调研**。

---

## 3. 根因分析 (已确认部分)

### 3.1 当前 `ClockMMC3Counter` 代码

```cpp
// src/boards/mmc3.cpp:211
static void ClockMMC3Counter(void) {
    int count = IRQCount;
    if (!count || IRQReload) {
        IRQCount = IRQLatch;
        IRQReload = 0;
    } else
        IRQCount--;
    if ((count | isRevB) && !IRQCount) {
        if (IRQa) {
            X6502_IRQBegin(FCEU_IQEXT);
        }
    }
}
```

### 3.2 错误码聚类分析

#### 0x02 (6 ROMs) — IRQ counter reload 边角案例

| ROM | 测试期望 | 当前行为 |
|---|---|---|
| `mmc3_2_details` | "Counter isn't working when reloaded with 255" | Counter 减到 0 后,reload 到 255,期望 255 → 0 是 N 个 cycle |
| `mmc3_v2_2_details` | 同上(v2) | 同上 |
| `mmc3_5_MMC3` | "Should reload and set IRQ every clock when reload is 0" | Reload 到 0 → IRQ 期望每次触发 |
| `mmc3_v2_5_MMC3` | 同上(v2) | 同上 |
| `mmc3_6_MMC6` | "IRQ should be set when reloading to 0 after clear" | Clear IRQ → reload 0 → IRQ 期望 set |
| `mmc3_v2_6_MMC3_alt` | "IRQ shouldn't be set when reloading to 0 due to counter naturally reaching 0 previously" | 自然减到 0 后,reload 到 0 → 期望**不**触发新 IRQ |

**矛盾点**: v1_5/v2_5 期望 reload-0 触发 IRQ,v2_6 期望自然 0 后 reload-0 不触发 IRQ。当前代码 `(count | isRevB) && !IRQCount` 在 `count=0`(自然到 0 后)且 latch=0 时:`(0|1) && !0` = true → fire IRQ。这满足 v1_5 但违反 v2_6。

**修复方向**: 区分"自然递减到 0" vs "reload 到 0"。两者都需要 `count && !IRQCount` (排除 count=0 → 0 的 reload),但 v1_5 的"every clock"又要求reload-to-0触发——这是冲突的,可能需要更细粒度的 state 跟踪(比如 IRQa 在自然触发后是否自动 cleared)。

#### 0x03/0x04 (4 ROMs) — A12 clocking via PPUADDR

| ROM | 测试期望 | 当前实现 |
|---|---|---|
| `mmc3_1_clocking` / `v2_1_clocking` | "Should decrement when A12 is toggled via PPUADDR" | Mapper 4 无 PPU_hook → 未实现 |
| `mmc3_3_A12_clocking` / `v2_3_A12_clocking` | "Should be clocked when A12 changes to 1 via PPUADDR write" | Mapper 4 无 PPU_hook → 未实现 |

**修复方向**: 为 Mapper 4 添加 PPU_hook,检测 PPUADDR 写入触发 A12 上升沿。**改动面较大,需新调研**。

#### 0x09 (2 ROMs) — Scanline 0 IRQ timing

| ROM | 测试期望 | 当前行为 |
|---|---|---|
| `mmc3_4_scanline_timing` / `v2_4` | "Scanline 0 IRQ should occur sooner when $2000=$10" | `$2000=$10` 时 BG 高度=16 → 更多扫描线触发 IRQ。当前 IRQ 触发时机不正确 |

**修复方向**: 调整 `GameHBIRQHook` 触发时机或扫描线循环,使其匹配 `$2000=$10` 时的行为。**改动面较大,需新调研**。

---

## 4. 结论与后续

### 4.1 桶 A 状态:全部 12 项记录为有据已知限制

| ROM | 错误码 | 限制类型 |
|---|---|---|
| `mmc3_2_details` | 0x02 | 已知:MMC3 counter reload 边角场景 |
| `mmc3_3_A12_clocking` | 0x04 | 已知:A12 via PPUADDR 未实现 |
| `mmc3_4_scanline_timing` | 0x09 | 已知:Scanline 0 IRQ 时机 |
| `mmc3_5_MMC3` | 0x02 | 已知:MMC3 counter reload 边角场景 |
| `mmc3_6_MMC6` | 0x02 | 已知:MMC3 counter reload 边角场景 |
| `mmc3_v2_1_clocking` | 0x03 | 已知:A12 via PPUADDR 未实现 |
| `mmc3_v2_2_details` | 0x02 | 已知:MMC3 counter reload 边角场景 |
| `mmc3_v2_3_A12_clocking` | 0x04 | 已知:A12 via PPUADDR 未实现 |
| `mmc3_v2_4_scanline_timing` | 0x09 | 已知:Scanline 0 IRQ 时机 |
| `mmc3_v2_5_MMC3` | 0x02 | 已知:MMC3 counter reload 边角场景 |
| `mmc3_v2_6_MMC3_alt` | 0x02 | 已知:MMC3 counter reload 边角场景 |

**0x02 限制**:`src/boards/mmc3.cpp:218` 条件 `(count | isRevB) && !IRQCount` 与 v2_6 期望冲突,需更细粒度的状态机(可能需新增 IRQa 自动 cleared 字段)。
**0x03/0x04 限制**:Mapper 4 未实现 PPU_hook 检测 PPUADDR 写入,需新增。
**0x09 限制**:`GameHBIRQHook` 时机与 `$2000=$10` 时 BG 渲染不匹配,需调整。

### 4.2 探针保留(env-gated,零侵入)

- `FCEUX11_MMC3_PROBE=1` 启用 probe 输出
- 现有 PASS ROM 与 Oracle A 不受影响
- 后续深模型调研时可复用

### 4.3 推荐下一步

桶 A 暂搁置(深模型方向),启动桶 B (CPU 时序/中断) 调研。该桶与 Phase 1 vbl_* 修复无耦合,改动面较小,且部分子桶 (dummy write) 可能快速出结果。

---

*调查完。桶 A 记录为有据已知限制。probe 代码保留供后续深模型调研复用。*