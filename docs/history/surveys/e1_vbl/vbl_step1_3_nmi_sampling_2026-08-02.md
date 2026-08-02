# E-1 PPU VBL/NMI Step 1.3 — CPU 侧 NMI 采样模型（2026-08-02）

> **目的**：按 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 1 Step 1.3
> 修复 CPU 侧 NMI/读采样时序与 PPU 帧边界的对齐（vbl_02/05/06/07/08/10 共 6 ROM 的
> 同一深层根因），实测评估。
>
> **分支**：`wip_1.16`（HEAD `f5e7cd0` 之上，工作树直接改）
> **作者**：Codex（2026-08-02）

---

## 1. 本轮改动（保留）

1. **`src/x6502.cpp`**：
   - `TriggerNMI()`（VBL 路径）新增 **NMI-latch freshness 延迟一指令边界**：
     VBL 块在两次 CPU run 之间置 latch，CPU 下一个 loop-top 检查点其实是
     **上一条指令结束的边界**（latch 尚未生效），立即派发会早 1 指令。
     按 6502 "指令结束采样" 语义，fresh latch 先推迟一个边界、下一边界才派发。
     `TriggerNMI2()`（$2000 写边沿）已有等价的 NMI2→NMI 一级转换，未改动。
   - env-gated `E1 NMI_SET / NMI_SET2 / NMI_DEFER / NMI_DISPATCH` 绝对周期探针
     （`FCEUX11_E1_TRACE=1`），与 PPU 侧探针共用同一开关。
   - 新增 `fceu11_e1_last_pc()`：记录当前边界 CPU 指令流位置（PPU 侧 VBL 探针用）。
2. **`src/ppu_rendering.cpp`**：
   - `E1 VBL_ENTER` 探针扩展为打印 `abs / cycle / count（_count 残量）/ lastpc /
     suppressed / VBlankON`，`VBL_AFTER_NMIDELAY` 打印 NMI 延迟点。
   - NMI 延迟仍走 R5 Step 3 的 `runppu(3)`（默认），未改帧结构（6820 VBL 周期、
     89342 帧长不变，vbl_01/03/04/09 保持 PASS）。
3. **`src/ppu.cpp`**：`E1 P2002_READ` 改为绝对周期；`E1 W2000` 新增 $2000 写探针。

## 2. 回归结果

| 检查 | 结果 |
|---|---|
| Oracle A `ctest -LE perf` | **33/33 PASS**（config_store_test 为 Qt 环境问题，与核心无关）|
| vbl_01 / 03 / 04 / 09（PASS 基线）| 全部保持 PASS ✓ |
| vbl_05_nmi_timing | 0x01，X 序列 **[2,2,2,2,2,1,1,1,1,1] → [3,3,3,3,2,2,2,2,2,2]**（首行 +1）|
| vbl_02 / 06 / 07 / 08 / 10 | 0x01 / 0x01 / 0x01 / 0x01 / 0x03（未翻红，亦未闭合）|

## 3. 决定性探针数据（vbl_05，D=NMIDELAY 扫参）

`E1 VBL_ENTER`（VBlankON=1 帧）逐行：count 为 `_count` 残量（16 单位 = 1/3 CPU 周期），
lastpc 为边界处指令流位置；`NMI_DISPATCH pc` 为派发边界（X = pc-2 指令的立即数）。

### 3.1 默认 D=3（本轮）— 全 10 行
```
count  lastpc  DPC    X
-80    E350    E354   3
  0    E34E    E354   3
-16    E34E    E354   3
-32    E34E    E352   2
...
```
→ X = [3,3,3,3,2,2,2,2,2,2]；期望 [4,4,4,3,3,3,3,3,3,2]。

### 3.2 D 扫参（fresh 延迟固定 1 边界）
| D | X 序列（10 行）| 4→3 转变行 | 3→2 转变行 |
|---|---|---|---|
| 0 | 2,2,2,2,2,2,1,1,1 | 5-6 | 8-9 |
| 3 | 3,3,3,3,2,2,2,2,2,2 | 3-4 | 8-9 |
| 6 | 4,3,3,3,3,3,3,2,2,2 | 0-1 | 7-8 |
| 7 | 4,4,3,3,3,3,3,3,2,2 | 1-2 | 7-8 |
| 8 | 3,3,3,2,2,2,2,2,2,1 | 2-3 | 8-9 |
| 9 | 4,4,4,4,3,3,3,3,3,3 | 3-4 | 9-10(无) |

**期望 [4,4,4,3,3,3,3,3,3,2]** 的转变行是 2-3 与 8-9；任何固定 D 都无法同时满足
"派发 PC 正确（=期望 X）"与"转变行正确"——D 同时移动两者。

## 4. 关键机制发现（Step 1.3 深层根因）

1. **fresh 延迟 1 边界是对的**：`NMI_SET` 与 `NMI_DEFER` 同一 abs（边界 B），派发在
   B+2（E352 完成后）。去掉延迟（实验 NODEFER）使全部 X 再 -1，确认该一级延迟
   符合 6502 "指令结束采样"。
2. **flag 置位点被帧边界锚死**：`PPU_status |= 0x80` 在 sl240→sl241 边界（cycle 0），
   vbl_01 要求 set→clear = 6820。任何把置位点移入块内（FLAGDELAY/dot1/phase 模型）
   的实验都使 cycle 每帧累积、sync_vbl 永不退出或 vbl_01 翻红（"period too long"）——
   即 **FCEUX 的 CPU/PPU 预算结构不允许 flag 置位落在"指令中间"**。
3. **硬件 vs FCEUX 的相位差**：RustyNES 参考文档（`build/rustynes_ppu-2c02.md`）确认
   flag 置位在 sl241 dot1、/NMI 在 dot2，CPU 下一指令边界采样。硬件 CPU 自由运行，
   flag 落在指令流中的**相位每帧漂移 1/3 周期**（vbl_05 每行 = 1 PPU dot）。
   FCEUX 的 `_count` 残量确实按 1/3 周期/帧漂移（VBL_ENTER count 逐帧 -16），
   **但派发被 VBL 块 runppu 预算量化吸收**，同一 D 下 DPC 只随 lastpc 粗变
   （E350/E34E/E34B 三档），无法还原 [4,4,4,3,...,2] 的逐行漂移形状。
4. 方向结论：Step 1.3 的完成需要 **CPU 侧采样模型携带子周期相位**（与 Phase 2
   cycle-position APU 帧计数器同族），即让 NMI 派发点由 `_count` 残量相位直接决定，
   而不是由 VBL 块固定预算 + fresh 延迟近似。此改动面在 x6502 采样模型，超出本轮
   "延迟 1 边界"的近似范围。

## 5. 本轮净效果与遗留

- **净效果**：vbl_05 首行 X 2→3（仍差 1 迭代）；其余 ROM 状态不变；Oracle A 零回归。
- **遗留**：vbl_02/05/06/07/08/10 仍未闭合；flag 置位相位漂移的 CPU 侧建模是下一步。
  探针（VBL_ENTER lastpc/count、NMI_SET/DEFER/DISPATCH）与扫参数据保留在
  `build/e1_vbl05*.err`、`build/e1_d*.err`。

---

*数据文档完。*
