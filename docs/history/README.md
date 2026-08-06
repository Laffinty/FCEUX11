# FCEUX11 内部文档归档（docs/history）

> **定位**：FCEUX11 各版本（v1.4 ~ v1.16）已完成/在途的构建计划、执行报告、审计验收、
> 调查实验数据与清单的**历史归档**。当前**在途工作**的构建方案见 `docs/` 根目录（如
> `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md`），技术参考文档见 `docs/tech/`。
> **归档整理日期**：2026-08-01（依据 Diátaxis 文档架构思想 + Rust RFC 状态机惯例）

---

## 1. 归档规范（如何阅读与维护）

### 1.1 目录语义（按用途归档）

| 目录 | 用途 | 示例 |
|------|------|------|
| `plans/` | 历史构建/实施计划与路线图（已完成 🔵 或暂缓 🟡） | hotfix1~5 PLAN、Stage-2、v1.x Roadmap |
| `reports/` | 阶段完成、验证、审计、验收报告 | hotfix2/3 phase 报告、KagamiQA 审计/验收、最终验收报告 |
| `surveys/` | 调查与实验数据（两条在途链：`e1_vbl/` PPU VBL/NMI、`e6_apu/` APU 帧计数器） | vbl_step*、r6_step* |
| `checklists/` | 接入清单、移除清单、问题清单、手工场景 | blargg 接入清单、v2.0 移除清单 |
| `obsolete/` | **已被取代**的文档（仅作审计链保留，文件顶部有 🔴 OBSOLETE 横幅） | 翻译质量复测报告（v2） |
| `history.md` | v1.4 ~ v1.13 综合构建史（早期单文件已合并于此） | — |

### 1.2 状态标注（文件顶部横幅）

沿用 RFC 状态机惯例，每个文档用下列状态之一标注（`obsolete/` 与暂缓文档必须带横幅）：

| 状态 | 含义 | 横幅 |
|------|------|------|
| 🟢 ACTIVE | 当前有效/在途（调查链、待办清单、最新交接文档） | `> **STATUS: ACTIVE**` |
| 🔵 COMPLETED | 已完成归档的历史记录（占绝大多数） | `> **STATUS: COMPLETED**` |
| 🟡 DEFERRED | 暂缓（如 P3 权威性提升） | `> **STATUS: DEFERRED**`（附原因与重启条件） |
| 🔴 OBSOLETE | 已过时/被取代，置于 `obsolete/` | `> [!WARNING] **STATUS: OBSOLETE**`（附取代者路径） |

### 1.3 命名规范

- 计划/报告/清单：`FCEUX11-<版本>-<主题>-<类型>`，类型 ∈ `PLAN` / `报告` / `清单`
- 调查数据：链前缀 + 步骤 + 日期，如 `vbl_step2_instrument_data_2026-08-01.md`、`r6_step3_fix_data_2026-08-01.md`
- 涉及文件:行号的引用，统一写作 `` `docs/history/<子目录>/<文件名>.md:<行号>` ``

### 1.4 维护约定

1. **新文档先定用途再落位**：计划→`plans/`、报告→`reports/`、数据→`surveys/<链>/`、清单→`checklists/`；
   被取代文档移入 `obsolete/` 并加 🔴 横幅，不直接删除（git 已留痕，审计链需完整）。
2. **引用必须带完整路径**：从 `docs/` 起，含子目录（本归档整理已统一修复全部交叉引用）。
3. **重复内容合并**：同一主题的多轮报告只保留最终结论文档，中间轮次进 `obsolete/`。

---

## 2. 文档清单（按子目录）

### 2.1 `plans/` — 构建计划与路线图

| 文件 | 状态 | 说明 |
|------|------|------|
| `FCEUX11-1.15_LTS-hotfix1-PLAN.md` | 🔵 | hotfix1 修复执行计划（42 PR，P0~P3） |
| `FCEUX11-1.15_LTS-hotfix2-PLAN.md` | 🔵 | hotfix2 算法级 REVIEW + 性能优化（22 PR，PPU 热路径） |
| `FCEUX11-1.15_LTS-hotfix3-PLAN.md` | 🔵 | hotfix3 细致 REVIEW 修复（26 PR / 5 phase） |
| `FCEUX11-1.15_LTS-hotfix4-PLAN.md` | 🔵 | hotfix4 UI 回归排查（D-1~D-16） |
| `FCEUX11-1.15_LTS-hotfix5-PLAN.md` | 🔵 | hotfix5 i18n 重做（执行结果见 `reports/FCEUX11-1.15_翻译质量最终复测报告.md`） |
| `FCEUX11-1.16_KagamiQA-PLAN.md` | 🔵 | KagamiQA 双 Oracle 测试系统总构建计划 |
| `FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md` | 🔵 | P5 权威性构建（口径后续修订为 177 ROM） |
| `FCEUX11-1.16_P3-KagamiQA闭环四阶段构建方案.md` | 🔵 | P3 闭环四阶段构建（Phase 1-3 精度收敛 100% + Phase 4 KagamiQA 系统闭环 + v1.16 发布门禁） |
| `FCEUX11-1.16_Stage2-构建计划.md` | 🔵 | Stage-2 二阶段构建（含收官状态回填） |
| `FCEUX11-Stage3-权威性迭代与通用化路线.md` | 🟡 | Stage-3 路线草案（**P3 暂缓**，顶部有 DEFERRED 横幅） |
| `v1.x_Modernization_Roadmap.md` | 🔵 | v1.x 现代化改造路线图（v1.1~v1.14，已收官） |

### 2.2 `reports/` — 执行 / 审计 / 验收报告

| 文件 | 状态 | 说明 |
|------|------|------|
| `v1.15_hotfix2_phase_a_verify.md` | 🔵 | hotfix2 Phase A 验证（oam_bucket_idx 溢出修复，PPU −49.71%） |
| `v1.15_hotfix2_phase_b.md` | 🔵 | hotfix2 Phase B 完成（7 个 P1 微结构 PR） |
| `v1.15_hotfix2_phase_c.md` | 🔵 | hotfix2 Phase C 完成（5 个 P2 微观 PR） |
| `v1.15_hotfix2_phase_d.md` | 🔵 | hotfix2 Phase D 完成（5 个 P3 清理 PR） |
| `v1.15_hotfix2_release_evaluation.md` | 🔵 | hotfix2 发布就绪评估（评级 A） |
| `v1.15_hotfix3_overview.md` | 🔵 | hotfix3 总体审计（91 项诊断索引，含 5 CRITICAL） |
| `v1.15_hotfix3_phase_a_diagnostics.md` | 🔵 | hotfix3 Phase A 诊断（跨线程数据竞争/UAF） |
| `v1.15_hotfix3_phase_c_diagnostics.md` | 🔵 | hotfix3 Phase C 诊断+实施（内存安全） |
| `v1.15_hotfix3_phase_d_diagnostics.md` | 🔵 | hotfix3 Phase D 诊断+实施（性能回退修复） |
| `v1.15_hotfix3_phase_e_diagnostics.md` | 🔵 | hotfix3 Phase E 诊断+实施（质量清理） |
| `FCEUX11-1.15_收官审计报告.md` | 🔵 | v1.15 Finale 收官审计（原名 `remaining_work.md`，整理时更名） |
| `FCEUX11-1.15_翻译质量最终复测报告.md` | 🔵 | 翻译质量最终复测（v3 发布评审 ✅，**本主题唯一有效结论**） |
| `FCEUX11-1.16_KagamiQA-P0-P4-构建状态报告.md` | 🔵 | KagamiQA P0~P4 逐 phase 构建状态 |
| `FCEUX11-1.16_KagamiQA-P4-bridge.md` | 🔵 | P4-bridge headless 初始化修复（ppudead VBL/NMI） |
| `FCEUX11-1.16_KagamiQA-P4-bridge-根因解决方案.md` | 🔵 | P4-bridge 帧 3 崩溃根因（FFCEUX_PPURead NULL deref） |
| `FCEUX11-1.16_KagamiQA-审计报告.md` | 🔵 | KagamiQA 独立审计（S1~S4/M1~M7/L1~L4）；部分结论被修复验证与 Stage-2 勘误 |
| `FCEUX11-1.16_KagamiQA-修复验证报告.md` | 🔵 | 审计修复验证（逐项核销审计报告；个别结论后被 Stage-2 勘误） |
| `FCEUX11-1.16_CI-R4-实跑诊断.md` | 🔵 | CI R4 三轮实跑诊断与整改（1156ca1 闭环） |
| `ci-r4-closure-memory-2026-08-01.md` | 🟢 | R4 闭环结论落档（最新，为下一 PR/任务留对接提示，与上者配合使用） |
| `FCEUX11-1.16_最终验收报告.md` | 🔵 | v1.16 最终验收（通过）+ §十 P0~P3 整改建议（P2 精度收敛的处方来源） |

### 2.3 `surveys/e1_vbl/` — E-1 PPU VBL/NMI 调查链（在途 🟢）

| 文件 | 说明 |
|------|------|
| `FCEUX11-1.16_E-1-VBL调查记录.md` | E-1 调查总记录（第 2 轮，含「不要照抄」清单） |
| `vbl_baseline_2026-07-30.txt` | 10 ROM × 600 帧基线原始输出 |
| `vbl05_disasm_2026-07-30.md` | `vbl_05_nmi_timing.nes` 反汇编与时序诊断 |
| `vbl_step1_instrument_data_2026-08-01.md` | Step 1 dot 级探针数据（env-gated probe） |
| `vbl_step1_1_falsification_2026-08-01.md` | Step 1.1 证伪记录（置位点 cycle0→dot1 已回滚） |
| `vbl_step1_2_data_2026-08-01.md` | Step 1.2 $2002 读抑制实测（改动保留） |
| `vbl_step2_instrument_data_2026-08-01.md` | Step 2 CPU 侧插桩数据（NMI 派发 / $2002 读 / $2000 写） |
| `vbl_step3_fix_data_2026-08-01.md` | Step 3 runppu(3) 实测（fix 保留，R6 文档称之为 R5 fix） |

### 2.4 `surveys/e6_apu/` — E-3 APU 帧计数器调查链（在途 🟢）

| 文件 | 说明 |
|------|------|
| `r6_step1_instrument_data_2026-08-01.md` | R6 Step 1 探针数据（$4017/fcnt/IRQ 状态机，两个根因假设实证） |
| `r6_step2_fix_data_2026-08-01.md` | R6 缺陷 2 修复实测（仅 inhibit 写清 IRQ，保留于 `b710c68`） |
| `r6_step3_fix_data_2026-08-01.md` | R6 缺陷 1 两次修复尝试失败记录（已回滚，**未修复**） |
| `p2_step2_1_fix_data_2026-08-01.md` | **P2 Phase 2 Step 2.1** power/reset 相位分离实测（fcnt=1,2,3,0 证实；零回归；未闭合项指向 Step 2.2） |

### 2.5 `checklists/` — 清单

| 文件 | 状态 | 说明 |
|------|------|------|
| `FCEUX11-1.16_blargg_接入清单.md` | 🟢 | blargg ROM 统一 177 基线（被 CI 文档沿用） |
| `FCEUX11-1.16_KagamiQA-遗留问题与构建难题.md` | 🔵 | 遗留问题与构建难题清单（4 项不修复 + 4 项难题，含就地勘误） |
| `v1.15_hotfix2_manual_scenarios.md` | 🔵 | hotfix2 手动冒烟场景清单 |
| `v2.0_removal_checklist.md` | 🟢 | v2.0 前置移除清单（FCEUI_* 105 个、bmap[] 等） |

### 2.6 `obsolete/` — 已过时（🔴，仅审计链保留）

| 文件 | 取代者 |
|------|--------|
| `FCEUX11-1.15_翻译质量复测报告.md` | `reports/FCEUX11-1.15_翻译质量最终复测报告.md`（v3 ✅ 通过） |
| `FCEUX11-1.16_KagamiQA-P2-accuracy-table.md` | 22 ROM 快照 → 177 ROM 统一基线（`checklists/FCEUX11-1.16_blargg_接入清单.md`） |

### 2.7 根目录

| 文件 | 说明 |
|------|------|
| `history.md` | v1.4 ~ v1.13 综合构建史（早期单文件已合并于此并移除原件） |

---

## 3. 已知例外与历史引用说明

以下引用在历史文档中**刻意保留原样**（属历史陈述，非断链；整理时已逐条核实）：

- **`docs/继续任务.txt`**：已移除的输入文件（其内容已转录进 `docs/tech/P2_precision_instrument_handoff.md` 与
  `reports/ci-r4-closure-memory-2026-08-01.md`），历史文档中仍保留对它的引用。
- **`docs/internal/*`**：旧目录名（现为 `docs/history/`）。早期单文件已合并入 `history.md`；
  `v1.x_Modernization_Roadmap.md` 内的 `docs/internal/global_state_audit.md`、
  `docs/internal/v1.6_resonance_build_plan.md`、`docs/internal/phase6_vrc7_bench_regression.md`
  均指已合并归档内容。
- **`docs/FCEUX11-1.15_LTS-隐患审计报告.md`**、**《FCEUX11_各语种翻译质量审计报告.md》**：上游输入文档，
  已不单独存在（结论已并入 hotfix1-PLAN 与翻译复测链）。
- **`docs/FCEUX11-1.16_KagamiQA-P5-accuracy-table.md`**：P5 计划声明的**未来产物**，由
  `kagami-qa-runner` 自动生成，当前不存在属预期。
- **`docs/tech/KagamiQA-方法论.md`**：Stage-3 建议产出的未来文档（与被测物无关的规范）。
- **`v1.15_hotfix3_phase_b_diagnostics.md`**：从未单独生成（Phase B 诊断并入
  `reports/v1.15_hotfix3_overview.md`，hotfix3-PLAN 内有说明）。
- **`docs/blob/docs/...`**（在 `docs/tech/microsoft_ui_migration_feasibility_report.md` 中）：
  微软官方文档的外部链接，非本仓库文件。
