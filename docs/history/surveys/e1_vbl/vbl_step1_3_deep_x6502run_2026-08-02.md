# E-1 PPU VBL/NMI Step 1.3 深化 — X6502_Run 帧修复 + NMIDELAY 校准（2026-08-02）

> **目的**：按 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 1 Step 1.3 深化方向
> （"让 NMI 派发点由 `_count` 相位直接决定"）落地第一增量：**消除 runppu(3) 帧畸变 + 校准
> NMI 派发预算**，使 `vbl_05_nmi_timing` 转 PASS（$6000=0x00）。
>
> **分支**：`wip_1.16`（HEAD `7ea1d0c` 之上，工作树直接改）
> **作者**：Codex（2026-08-02）

---

## 1. 机制发现（instrument-first，改动前探针数据）

### 1.1 runppu(3) 帧畸变（决定性数据）

`vbl_05` 的 `E1 VBL_ENTER` 探针（改动前，runppu(3) 版本）逐测试帧：

| 帧 | abs | cycle | count | lastpc |
|---|---|---|---|---|
| 0 | 595615 | **0** | -80 | E350 |
| 1 | 1161447 | **3** | 0 | E34E |
| 2 | 1727281 | **6** | -16 | E34E |
| 3 | 2382457 | **9** | -32 | E34E |
| 4 | 2948291 | **12** | -48 | E34E |
| ... | ... | **+3/帧** | -16/帧 | ... |

**`cycle` 逐帧 +3**：NMI-on 帧的 VBL 块 = runppu(3) + 20×341 = **6823 dots**（硬件为
6820）——R5 Step 3 遗留的帧畸变。硬件上 NMI 开/关帧长一致，此畸变使每帧边界相位
额外 +3 dots，扭曲 blargg nmi_timing 测量的逐行漂移形状。

### 1.2 派发点量化（与 survey §4 一致）

NMI 派发 PC 只取离散值：lines 0-3 → E354（X=3）、lines 4-9 → E352（X=2），
无法还原期望 `[4,4,4,3,3,3,3,3,3,2]` 的逐行漂移。派发 = f(lastpc, count@latch)，
CPU 流位置（lastpc ∈ {E350,E34E,E34B}）与相位（count）分离。

## 2. 改动（保留）

1. **`src/ppu_rendering.cpp` VBL 块**：NMI 使能路径 `runppu(nd)` → `X6502_Run(nd)`。
   - runppu(nd) = 推进 PPU nd dots + 给 CPU nd×16 单位预算；X6502_Run(nd) 只给
     CPU 预算、**不推进 PPU** → NMI-on 帧恢复 6820-dot VBL（硬件真值）。
   - CPU 行为完全一致（预算相同），仅帧长归一。改动后探针证实 `cycle` 恒为 0。
2. **`e1_nmi_delay()` 默认 3 → 8**：`X6502_Run(8)` = 8 dots ≈ 2.67 CPU 周期预算
   于 latch 前，把 CPU 流位置推到正确 LDX 处。Env `FCEUX11_E1_NMIDELAY` 可覆盖。

## 3. 校准扫描（帧修复后，NMIDELAY 扫参）

| NMIDELAY | vbl_05 X 序列 | 结论 |
|---|---|---|
| 6 | [3,3,3,3,3,3,2,2,2] | 派发偏早 |
| 7 | [4,3,3,3,3,3,3,2,2] | 首行 4，其余偏早 |
| **8** | **[4,4,4,3,3,3,3,3,3,2]** | **✅ 与期望完全一致** |
| 9 | [4,4,4,4,3,3,3,3,3,3] | 转变行晚 1（4→3 在行 4） |
| 12 | [4,4,4,4,4,4,3,3,3] | 转变行更晚 |
| 15 | [5,5,5,4,4,4,4,4,4] | 整体 +1 |

**8 是唯一甜点**（7 偏早 1 指令、9 偏晚 1 行）；±1 dot 即翻转，属测试自身 1-dot 分辨率。

## 4. 回归结果

### 4.1 vbl 全 10 ROM（NMIDELAY=8）

| ROM | 改动前 | 改动后 | 期望 |
|---|---|---|---|
| vbl_01_basics | PASS | **PASS** | PASS（基线保持）|
| vbl_02_set_time | FAIL 0x01 | FAIL 0x01（`04 - V`，期望 `- -`）| 抑制窗口读偏移 |
| vbl_03_clear_time | PASS | **PASS** | PASS（基线保持）|
| vbl_04_nmi_control | PASS | **PASS** | PASS（基线保持）|
| **vbl_05_nmi_timing** | **FAIL 0x01** | **✅ PASS** | 主修复目标达成 |
| vbl_06_suppression | FAIL 0x01 | FAIL 0x01（`04-06` 抑制未达成）| 同上 |
| vbl_07_nmi_on_timing | FAIL 0x01 | FAIL 0x01（转变行晚 1）| $2000 边沿采样 |
| vbl_08_nmi_off_timing | FAIL 0x01 | FAIL 0x01（转变行早 2）| $2000 边沿采样 |
| vbl_09_even_odd_frames | PASS | **PASS** | PASS（基线保持）|
| vbl_10_even_odd_timing | FAIL 0x03 | FAIL 0x03（`08 07`）| Step 1.4 |

**零回归**：4 个 PASS 基线全部保持，vbl_05 转 PASS。

### 4.2 vbl_07/08 对 NMIDELAY 不敏感

NMIDELAY ∈ [5,12] 全扫，vbl_07/08 的转变行**不随 NMIDELAY 移动**——其根因在
$2000 写边沿的 NMI 采样窗口（NMI-enable/disable 与 VBL 置位/清除边界的相对采样），
与 VBL 派发预算无关，属另一子问题。

## 5. 剩余 FAIL 根因分类

| ROM | 根因 | 归属 |
|---|---|---|
| vbl_02 / vbl_06 | $2002 读落在 (sl240,c340) 抑制窗口外（CPU 侧读时序偏移 ~2-3 dots，survey §1.2 同源）| Step 1.3 读侧 |
| vbl_07 / vbl_08 | $2000 NMI-enable/disable 边沿采样窗口（指令边界 vs VBL 置位/清除边界）| Step 1.3 边沿 |
| vbl_10 | even/odd 跳点（(339,261)→(0,0)）位置 | Step 1.4 |

## 6. 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | instrument-first：改动前 VBL_ENTER 探针证实 cycle +3/帧畸变 | ✅ |
| 2 | 改动后探针证实 cycle 恒 0（帧修复生效）| ✅ |
| 3 | vbl_05 主修复目标 PASS | ✅ |
| 4 | vbl PASS 基线（01/03/04/09）零回归 | ✅ |
| 5 | Oracle A（ctest -LE perf，全量重建后）33/33 | ✅ |
| 6 | golden savestate 无需重生（无运行期起始值变更，nestest 哈希已在上提交重生）| ✅（待全量确认）|
| 7 | 未开新分支、savestate chunk 结构未动 | ✅ |

---

*数据文档完。下一步：vbl_02/06 读侧抑制窗口（A2002 读采样相位）→ vbl_07/08 边沿采样 →
Step 1.4 even/odd。*
