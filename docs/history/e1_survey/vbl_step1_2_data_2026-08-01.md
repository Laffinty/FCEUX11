# E-1 PPU VBL/NMI Step 1.2 — $2002 读抑制 实测记录 (2026-08-01)

> **目的**：按 `docs/history/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 1 Step 1.2 实现 $2002 读抑制（NESdev PPU_frame_timing 规则），实测评估。
>
> **分支**：`wip_1.16`（HEAD `f50573a` 之上，工作树直接改）
> **作者**：独立执行（2026-08-01）

---

## 1. 改动内容（保留）

1. **`src/ppu.h`**：声明 `fceu11_ppu_mark_vbl_set_suppressed()` / `fceu11_ppu_take_vbl_set_suppressed()` 访问器。
2. **`src/ppu.cpp`**：
   - 文件级标记 `g_vbl_set_suppressed` + 访问器（savestate 中立，不序列化）
   - `A2002`（$2002 读）：newppu 路径按 NESdev 规则——
     - 读于 `(sl=240, cycle=340)`（置位前 1 dot）→ 标记"本帧 VBL 置位被抑制"
     - 读于 `(sl=241, cycle=0-1)`（置位点/后 1 dot）→ `X6502_IRQEnd(FCEU_IQNMI)` 取消待派发 NMI
   - env-gated `E1 P2002_READ` 探针（过滤边界附近读点）
3. **`src/ppu_rendering.cpp`**：
   - working-config VBL 块入口 `fceu11_ppu_take_vbl_set_suppressed()`；为真则跳过 `PPU_status |= 0x80` 与 `TriggerNMI()`（VBL 周期照常推进）
   - `runppu(3)`（R5 Step 3）在 NMI 开启时无条件执行（帧长一致），仅 `TriggerNMI` 受抑制门控
   - env-gated `E1 VBL_SUPPRESSED` 探针

## 2. 回归结果

| 检查 | 结果 |
|---|---|
| Oracle A `ctest -LE perf` | **34/34 PASS**（含 golden_savestate） |
| Oracle B 全量（177 ROM） | **177 / 121 PASS / 56 FAIL —— 与报告基线逐字一致，零回归** |
| vbl_01 / 03 / 04 / 09（PASS 基线） | 全部保持 PASS ✅ |

## 3. vbl 目标 ROM 实测（600 帧）

| ROM | baseline | Step 1.2 后 | 变化 |
|---|---|---|---|
| vbl_02_set_time | 0x01（rows 00-04 "- V", 05-08 "V -"） | 0x01（**未变**） | ⚪ |
| vbl_05_nmi_timing | 0x01（[2,2,2,2,1,1,1,1,1,1]） | 0x01（未变） | ⚪ |
| vbl_06_suppression | 0x01 | 0x01（rows 00-03 现为 "- N" 与期望一致；04 仍错、05-06 缺 NMI 抑制） | 🔄 部分改善 |
| vbl_07_nmi_on_timing | 0x01 | 0x01（rows 00-05 N / 06-08 -，期望 00-04 N / 05-08 -，差 1 行） | 🔄 部分改善 |
| vbl_08_nmi_off_timing | 0x01 | 0x01（rows 03-04 - / 05-0C N，期望 03-06 - / 07-0C N，差 2 行） | 🔄 部分改善 |
| vbl_10_even_odd_timing | 0x03 | 0x03（未变） | ⚪ |

> ⚠️ 注：vbl_06/07/08 的"部分改善"对照的是 Step 1.1 实验（破坏性）状态，未经 f50573a 真基线逐行确认；
> 但全量 Oracle B 121/56 与报告基线一致，说明抑制改动对非 vbl 套件零影响。

## 4. 探针数据（决定性）

**vbl_02**（300 帧）：
- `P2002_READ` 边界读点：`(241,0) ×322`、`(241,1) ×9`、`(240,340) ×8`、`(241,2-5) ×14`、`(240,335-338) ×5`
- `VBL_SUPPRESSED`：**8 次**（frame 3, 9, 21, 51, 66, 68, 116, 162）——但输出**无任何 "- -" 行**

**vbl_06**（300 帧）：
- `VBL_SUPPRESSED`：2 次（frame 3, 177）

**关键结论**：
1. `(241,0) ×322` 是 **sync_vbl_delay 自旋读**（等 VBL 置位的读循环在 flag 置位点汇聚），非测量读。
2. 抑制在 vbl_02 触发 8 次但**未产生 "- -" 行** → 触发的抑制大多落在 sync 自旋读（硬件上同样合法——"read 1 dot before set suppresses"是文档行为），而**测量行 read1 从未落在 (240,340)**。
3. 测量读位置（`(240,335-338)` 稀疏分布）说明模拟器在帧边界的 CPU↔PPU 读时序**偏移 ~2-3 dot**——与 vbl_05 "NMI 早 ~2 指令"同一根因。

## 5. 结论与决策

1. **抑制机制已实现且硬件语义正确**（NESdev 文档规则），全量零回归。
2. **vbl_02 未修复**：测量读未落在抑制点。真因是**帧边界 CPU↔PPU 相对时序偏移**（step1/step2 文档"CPU 侧相对时序"结论的又一印证），不是抑制窗口本身。
3. **vbl_06 rows 05-06 "V -"（NMI 抑制）未达成**：仿真中 NMI 在指令边界检查早于读指令 → `X6502_IRQEnd` 太晚。需重排 NMI latch 与 CPU 读的时序（CPU 侧模型工作，即 Step 1.3）。
4. **决策**：保留 Step 1.2 代码（零回归、硬件正确、是后续工作的基础）；E-1 剩余（vbl_02/05/06/07/08/10）指向**同一个深层根因**——CPU 侧 NMI/读时序与 PPU 帧边界的对齐，需 Step 1.3 专项，或评估转 R6（APU 根因更明确）。

---

*实测记录完。下一步：Step 1.3（CPU 侧 NMI 模型）或 R6（需用户决策）。*
