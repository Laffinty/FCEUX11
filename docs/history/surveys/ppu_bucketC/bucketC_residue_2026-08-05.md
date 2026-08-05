# FCEUX11 v1.16 — PPU 桶 C 余项定案（深模型族已知限制）

> **编制日期**: 2026-08-05
> **承接文档**: `docs/history/surveys/ppu_bucketC/stepC_investigation_2026-08-04.md`
> **承接方案**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 桶 C
> **状态**: ✅ **桶 C 余项 2/2 记入有据已知限制**（深模型族）

---

## 0. 摘要

桶 C PPU 真实精度 4 项中前 2 项已 PASS（`ppu_read_buffer` + 顺带 `cpu_dummy_writes_ppu` `863e9d7`；`ppu_open_bus` `23b0cdd`）。剩余 2 项经评估后记入**有据已知限制**：

| 错误码 | ROM | 状态 |
|---|---|---|
| 0x01 | `oam_stress` | 🚧 **有据已知限制**（深模型族 — PPU 隐式 OAM 改写） |
| 0x01 | `ppu_vbl_nmi` | 🚧 **有据已知限制**（深模型族 — CPU 侧读采样量化） |

**桶 C 最终状态**：3/4 PASS（ppu_read_buffer + ppu_open_bus + cpu_dummy_writes_ppu 顺带）+ 2/4 已知限制（oam_stress + ppu_vbl_nmi）。

---

## 1. `oam_stress` 0x01 — 根因与处置

### 1.1 测试来源
blargg `oam_stress`（`christopherpow/nes-test-roms` 仓库，无源码）— 测试 OAM 在 PPU 渲染期间的稳定性，触发条件为 `PPUON` 开启 + 长帧 OAM 写入压力。

### 1.2 根因诊断
桶 C 调查记录（`stepC_investigation_2026-08-04.md` §5 探针实测）：
- `A2004` newppu 路径已实现 `PPUGenLatch = SPRAM[PPU[3]]`（本次 23b0cdd 顺手修复）
- 桶 B 调查（`stepB_investigation_2026-08-04.md`）发现 `cpu_dummy_writes_oam` 与本项同源——**PPU 隐式 OAM 改写**：
  - 渲染期间，PPU 自动写入 OAM 的路径（secondary OAM 初始化、精灵求值、属性 byte 写入）未完全建模
  - 探针数据：`A2004 ret == SPRAM[PPU[3]] == oam36_was` 已确认地址同一性
  - 根因在 PPU 渲染期对 OAM 的写时序未对齐硬件真值（次级 OAM 在 cycle 0-63 初始化为 0xFF、spr_read.found 自动填充、attribute byte 在 cycle 256-319 写入等）

### 1.3 处置依据
- 方案 §5 桶 C 余项评估：`oam_stress` 与桶 B `cpu_dummy_writes_oam` 同族
- 桶 B.3+B.4 已记录 `cpu_dummy_writes_oam` 为已知限制（`57d3e88`）
- 完整修复需 PPU 渲染期 OAM 写入的 cycle-accurate 建模 + 桶 B 探针已识别为"RMW 宏 abs/abs,X/abs,Y/(zp,X)/(zp),Y 都有 dummy write"——非"改动面小"桶
- 与 Mesen2/Nestopia 等准确模拟器对比，需要每条 PPU 渲染 cycle 的 OAM 操作精确化

### 1.4 决策
记入**有据已知限制**（深模型族），与桶 B.3+B.4 模式一致。修复路径：待深模型族突破后统一处理（与桶 D 同 Mesen2 风格 per-cycle PPU/CPU 联合仿真改造）。

---

## 2. `ppu_vbl_nmi` 0x01 — 根因与处置

### 2.1 测试来源
blargg `ppu_vbl_nmi` 同一 ROM（包含多个 sub-test）。**注意**：`ppu_vbl_nmi.nes` 的 0x01 错误码与 Phase 1 的 `vbl_02_set_time` 0x01 同源——blargg 在两个 ROM 中分别测试 NMI 时序的不同子场景。

### 2.2 根因诊断
方案 §5 桶 C 余项评估明确：
> "`ppu_vbl_nmi` 0x01:manifest frames=300 不足(需 3000);3000 frames 下为 0x01 (测试 2 of 10 vbl_set_time,与 Phase 1 vbl_02 同族——CPU 侧读采样量化)"

详细机制见 `docs/history/surveys/e1_vbl/vbl_step1_2_closure_assessment_2026-08-03.md`：
- 1 dot/行漂移来自 `sync_vbl_delay(A=行号)`（A 循环每迭代等效延迟 1 PPU dot = 1/3 CPU 周期，亚指令粒度）
- `A2002` 派发被指令边界量化 → 10 行仅 2 个离散落点
- 测试源码实证漂移为亚指令粒度 + 残量在差值中抵消（漂移被帧边界重同步摧毁）
- 闭合需亚指令级读时序（CPU/PPU 联合仿真改造），收敛无保证

### 2.3 处置依据
- 与 Phase 1 `vbl_02_set_time` 同根（CPU 侧读采样量化）
- Phase 1 已记录 `vbl_02_set_time` 为有据已知限制（`vbl_step1_2_closure_assessment_2026-08-03.md`）
- 完整修复需亚指令级 CPU↔PPU 联合仿真——超出"改动面小"预算
- 用户决策（2026-08-03 Phase 1）："有时间再推进"

### 2.4 决策
记入**有据已知限制**（深模型族，与 Phase 1 vbl_02/vbl_06 同族）。修复路径：待深模型族突破后统一处理。

---

## 3. 桶 C 全表收口

| 错误码 | ROM | 状态 | 提交 |
|---|---|---|---|
| 0x0E | `ppu_read_buffer` | ✅ PASS | `863e9d7` |
| 0x09 | `cpu_dummy_writes_ppu` | ✅ PASS（顺带） | `863e9d7` |
| 0x03 | `ppu_open_bus` | ✅ PASS | `23b0cdd` |
| 0x01 | `oam_stress` | 🚧 已知限制（深模型族） | `c(known-limit)`（本 commit） |
| 0x01 | `ppu_vbl_nmi` | 🚧 已知限制（深模型族） | `c(known-limit)`（本 commit） |

**桶 C 总评**：4 项中 3 项 PASS + 2 项已知限制（含顺带 1 项 PASS），收敛率 60%（3/5）；剩余 2 项明确归类为深模型族。

---

## 4. 探针保留

`FCEUX11_OPENDECAY_PROBE` env-gated 探针继续保留（含 OPENDECAY W2000/PPUGenLatch 写入/读取/衰减检查 + OPENDECAY SPRDMA 事件日志），供未来深模型族调研复用。

---

*调查完。桶 C 余项已记入已知限制，与桶 B.3+B.4 + Phase 1 vbl_02 + 桶 D 同族（深模型族）。*