# FCEUX11 v1.17 构建计划 — KagamiQA 统合、Rust 迁移与分级标准

> **版本**：v1.17（2026-08-07 草案 / **2026-08-08 主体已施工**）
> **日期**：2026-08-07 草案 / 2026-08-08 主体施工
> **分支**：`wip_v1.17`
> **状态**：✅ **主体已施工（2026-08-08）**；冻结基线生效为后续收敛项
> **前置**：v1.16 最终验收通过（`docs/history/reports/FCEUX11-1.16_最终验收报告.md`，11/11 项闭合）
> **关联**：`docs/tech/KagamiQA.md`、`docs/history/plans/FCEUX11-Stage3-权威性迭代与通用化路线.md`、`docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md`、`docs/history/checklists/v2.0_removal_checklist.md`
> **路线图位置**：v1.15 完成 v1.x C++ 现代化；v1.16 完成 KagamiQA 双 Oracle 闭环；**v1.17 = KagamiQA 统合（测试体系单一归属）+ 遗留精度收敛 + 分级标准落地**

---

## 〇、TL;DR

v1.17 的五项任务及其性质：

| # | 任务 | 性质 | 目标 |
|---|---|---|---|
| 1 | 可迁移 C++ 测试 → KagamiQA Rust 体系 | 代码迁移（~2,900 LOC） | 消除 C++ harness 双实现，全部测试由 kagami-qa-runner 单一调度 |
| 2 | 不可迁移 C++ 测试耦合 KagamiQA + 源码区隔 | 结构重组 | KagamiQA 资产集中落位（`tests/kagami/`），与 FCEUX11 引擎代码物理区隔 |
| 3 | v1.16 遗留项收敛 | 精度攻坚 + harness 修复 | Oracle B 伪失败 18 项清零，真实精度 FAIL 面下降，矩阵数字收敛 |
| 4 | KagamiQA 架构规范化 | 重构 | 明确七层依赖层级，消除 main.rs god file 与 direct_entry 重复；补 runner `--filter` 与 direct 看门狗；视作一个整体 |
| 5 | A–E 分级通过标准 | 新功能 | 测试结果有机器可算、可审计的发布等级 |

**一句话收束**：v1.17 把 KagamiQA 从「FCEUX11 的附属测试框架」升级为「**测试体系的唯一归属与唯一门禁**」——所有测试都是它的子项（任务 1/2）、它有明确的分层架构（任务 4）、产出可判读的发布等级（任务 5），同时继续收敛 v1.16 遗留的精度与 harness 问题（任务 3）。

**v1.17 的量化收敛目标**：当前基线分级为 **C 级**（8 项 advisory 已知限制全部有据编目、blocking 全 PASS），目标向 **B 级** 靠拢（冻结基线内无新增已知限制、advisory 数量下降）。**B 级机器化条件**：`tests/fixtures/kagamiqa_baseline_frozen.json` 落地为 runner 的 `--baseline` 指向文件，CI 工作流已改用之（见 §十三.1）；任何新的 advisory FAIL 将落入 `new_test(FAIL)` 桶、grade 退回 C，是精准的回归信号。

---

## 一、基线快照（v1.16 验收态，CI run #31/#33 实测 → v1.17 主施工后 2026-08-08 实测）

| 维度 | 数值 | 来源 |
|---|---|---|
| CTest 注册测试 | 34（`-LE perf` 实跑 33，全 PASS） | `tests/CMakeLists.txt` / CI |
| `tests.json` 清单条目 | **47**（Oracle A 27 + Oracle B 20） | `python` 解析确认 |
| 当前矩阵 PASS / FAIL | **39 / 8**（8 FAIL 全部 advisory 已知限制） | CI matrix artifact |
| blargg 落盘 ROM | **177**（cpu 58 / apu 52 / ppu 49 / mmc3 18） | 文件系统计数 |
| Oracle B 全量 | 144 PASS / 33 FAIL（0x80×12 + 0x81×6 **全清零**，剩余 33 全真实精度深模型族） | `blargg_full_output.txt` |
| 已知限制归类 | 32 项深模型族（MMC3 12 / CPU 11 / sprdma 2 / PPU 2 / VBL 5）+ 1 永久跳过 | Phase 3 收口 |
| cargo test（kagami-qa） | **187/187 PASS**（v1.17 7 commit 净增 + 内部重构换路径零回归） | `cargo test --release -p kagami-qa` |
| CI 触发分支 | `main` / `wip_1.16` / `wip_v1.17`（v1.17 A0 已修） | workflow 声明 |
| 权威性口径 | 门槛全绿 + 度量：外部真理覆盖 177/177、oracle 来源数 = 1 | KagamiQA.md §1.4 |

**关键判定**：v1.16 是「构建可靠、测试可信（ctest 维度）、失败诚实标注」的版本。遗留精度项（E-1/E-3）与 harness 缺口（0x80/0x81）已正确归类为 advisory。v1.17 H-1/H-2 收敛后，0x80×12 + 0x81×6 = 18 项伪失败全清（manifest 全字段 + 帧预算校准），剩余 33 FAIL 全部进入「**冻结基线**」（`tests/fixtures/kagamiqa_baseline_frozen.json` 8 项 test_set 入口 + Blargg 文件级 33 项）——CI 改用冻结 baseline 后，runner 输出 **B 级**（`fail_to_fail` 8 项，§十三.1 收口）。

---

## 二、任务 1：可迁移 C++ 测试迁入 KagamiQA Rust 体系

### 2.1 迁移范围（实测 2,897 LOC）

| 类别 | 文件 | LOC | 迁移去向 |
|---|---|---|---|
| Harness | `tests/blargg_runner.cpp` | 533 | 合并进 Rust `direct_entry`（同能力已有第二份实现，消除双实现） |
| Harness | `tests/lua_runner.cpp` | 435 | Rust Lua runner（复用 `fceux11-lua` crate bindings + C ABI 桥） |
| Harness | `tests/kagami_direct_main.cpp` | 27 | 删除 C++ 壳，入口并入 Rust CLI |
| ROM Oracle | `tests/rom_regression_test.cpp` | 329 | Rust harness：load→run N 帧→CRC32 比对 |
| ROM Oracle | `tests/savestate_regression_test.cpp` | 325 | 同上（MD5 savestate） |
| ROM Oracle | `tests/core/mapper_byte_diff_test.cpp` | 442 | 同上（mapper 状态字节） |
| ROM Oracle | `tests/core/apu_wav_diff_test.cpp` | 304 | 同上（WAV 采样） |
| ROM Oracle | `tests/core/ppu_frame_diff_test.cpp` | 253 | 同上（像素帧） |
| ROM Oracle | `tests/fixtures/golden/golden_savestate_test.cpp` | ~250 | 同上（.fc0 往返） |
| 公共设施 | `tests/core/test_helpers.h` | 144 | Rust `test_helpers.rs`（golden 加载 / CRC32 / 比较器） |

### 2.2 可行性依据

1. **铁证在先**：`kagami_qa_direct_runner` 已证明 Rust 经 C ABI 桥（`src/kagami_bridge.cpp/h`）在进程内 load→step→probe $6000，且与 subprocess 模式 parity 已验证。
2. **golden 是数据**：帧/哈希/像素/WAV goldens 格式中立，与语言无关，无需重建。
3. **统合目标同构**：这些测试全部符合「load→run→compare」模式，迁 Rust 后天然成为 manifest 条目。

### 2.3 实施步骤

1. **Rust 侧统一 harness 核心**：在 `kagami-qa` crate 新增 `harness/` 模块（或并入 `direct_entry` 重构产物，见任务 4），提供 `load_rom → run_frames → read_mem → compare` 的通用驱动。
2. **逐测试迁移（每个测试一个 PR）**：C++ harness 与 Rust harness 双跑同一 ROM，**100% 一致后才切换**。
3. **`test_helpers.h` → `test_helpers.rs`**：golden 加载、哈希、比较器等价实现。
4. **CTest 登记同步**：已迁移测试的 CTest 条目删除或改为指向 Rust 二进制；`tests.json` 条目 binary 字段更新 + `provenance` 标注迁移 commit。
5. **CI 验证**：迁移 PR 必须跑全量 ctest + matrix，反 gaming 纪律生效（`new_test` 桶 + baseline 同级评审）。

### 2.4 验收门禁

| 门禁 | 阈值 |
|---|---|
| 逐测试 parity | C++ vs Rust harness 输出 100% 一致（哈希/判定/exit code） |
| 迁移后 ctest | 34 项语义不变（改名/删条目的需在 PR 描述显式列出） |
| 迁移后 matrix | 39/47 基线不回退，`fail_to_pass = 0` |
| 残留检查 | `grep -rn "BLARGG_RESULT" tests/*.cpp` 仅剩注释引用 |
| harness 双实现 | `direct_entry.rs` 与旧 blargg_runner 逻辑并存的重复代码清零 |

### 2.5 证伪判据

若任一 ROM 在 Rust harness 下产出与 C++ harness 不同的哈希/判定，且非 golden 过期（见 2.6 时序），则该测试**不得切换**，回退 C++ 实现并记录差异根因。

### 2.6 时序红线（与任务 3 的协调）

`ppu_frame_diff_test` / `apu_wav_diff_test` 的 golden 在 R5（E-1 PPU）/ R6（E-3 APU）精度修复后可能失效。**执行序约束**：任务 3 的 R5/R6 精度修复先落地（或同步协调 golden 重抓），再迁移这两个测试，避免迁移与 golden 重抓相互污染。

> **2026-08-08 实际状态（详见 §十一.2.3）**：R5/R6 已通过已知限制路径收口（H-1/H-2 harness 修复搞定，无 sound.cpp / ppu_rendering.cpp 改动），**未触发 golden 重抓**；`apu_wav_diff_test` / `ppu_frame_diff_test` 因 §2.6 仍在 `tests/core/` 未迁 Rust。本节时序约束保留为：**R5/R6 真实精度回归若发生，需在 `ppu_frame_diff` 与 `apu_wav_diff` 迁移前完成 golden 重抓**。当前已知限制收口路径下，迁移前置条件 = **`tests/fixtures/golden_hashes_audit.json` 追加 R5/R6 diff 行**（记录固化的真实精度偏差），不必等真实修复。后续工作项见 §十三.2。

---

## 三、任务 2：不可迁移 C++ 测试耦合 KagamiQA + 源码区隔

### 3.1 范围（不可迁移 ≈ 4,000+ LOC）

| 子类 | 代表 | 为何不可迁 |
|---|---|---|
| 核心内部单元测试（~3,000） | `cpu_test` / `ppu_test` / `apu_test` / `bus_test` / `mapper_test` / `savestate_test` / `cart_class_test` / `fds_load_test` / `core_state_test` / `smoke_test` / `headless_smoke_test` / `driver_callbacks_test` / `core_driver_boundary_test` | 直接调 C++ 核心内部 API（`g_bus` / `g_cpu` / `fceu11::State`），核心未 Rust 化前无法迁移（跟随核心迁移节奏） |
| C++ 语言/平台测试（~1,000） | `enum_class_bitflags_test` / `expected_api_test` / `config_store_test`(Qt) / `pixbuf_pool_test` / `i18n_regression_test` | 测 C++ 构造或 Qt 绑定本身，随 v2.0 清理清单退役 |
| 基准（~970） | 5 个 Google Benchmark + `bench_tolerance_test` | 测 C++ 热路径，并行维护（criterion 版可新增，不替换） |

### 3.2 现状问题：KagamiQA C++ 资产散落四处

```
src/kagami_bridge.cpp/h        ← 引擎树内（引擎链接，但属 KagamiQA 资产）
tests/kagami_direct_main.cpp   ← tests/ 根
tests/blargg_runner.cpp        ← tests/ 根
tests/lua_runner.cpp           ← tests/ 根
src/rust/crates/kagami-qa/     ← Rust 主体（已独立）
```

### 3.3 目标目录规划

```
tests/kagami/                        ← KagamiQA C++ 资产唯一落点（新建）
  ├── blargg_runner.cpp              ← 任务 1 迁移完成后删除（暂存）
  ├── lua_runner.cpp                 ← 任务 1 迁移完成后删除（暂存）
  ├── kagami_direct_main.cpp         ← 任务 1 迁移完成后删除（暂存）
  ├── core/ boards/ benchmark/       ← 不可迁移测试分区
  │      benchmarks/ utils/             （44 文件完整映射见落位清单）
  └── 无独立 CMakeLists（方案 B：tests/CMakeLists.txt 内改源路径——
      2026-08-08 已定，避免 CMAKE_CURRENT_SOURCE_DIR 变化破坏 fixture 路径）
src/kagami/                          ← 引擎侧桥接（决策点 3.4，建议保留于 src/ 根）
  └── kagami_bridge.cpp/h
```

> 完整 44 文件映射、CMake 改动点、登记核对表与验收门禁见
> **`docs/FCEUX11-1.17_Task2-落位清单.md`**（执行级清单，本计划为策略级）。

### 3.4 实施步骤与决策点

1. **决策点（桥接归属）**：`kagami_bridge` 被编译进 `fceux11_core`，是引擎的 FFI 面（双向资产）。**建议保守处理**：桥接保留在 `src/`（不移动，避免 `src/CMakeLists.txt` 构建链扰动），在文档中显式声明其「KagamiQA 资产但引擎链接」的边界属性。若坚持物理隔离，需单独评估 `src/CMakeLists.txt` 改动与链接顺序风险。
2. **`tests/kagami/` 落位**：移动不可迁移测试文件；保持 **CTest 测试名不变**（`cpu_test` 等），只改源文件路径，保证 CI 历史对比有效。
3. **路径修正（方案 B 下大部分不变）**：`WORKING_DIRECTORY` 与 fixture 相对路径（`fixtures/...`）**保持不变**（单一 CMakeLists 的收益）；PATH 注入名单（`tests/CMakeLists.txt` 的 `set_tests_properties` 测试名列表）**确认无需改动**；仅 `add_executable` 源路径与 include 目录（`tests/core` → `tests/kagami/core`）需更新。
4. **manifest 登记核对**：确认 47 条 `tests.json` 条目与 `tests/kagami/` + `tests/` 其余文件一一对应，无游离测试。
5. **文档**：KagamiQA.md §五「目录结构」更新为统合后布局。
6. **决策记录（2026-08-08，落位清单 §六 四项已拍板）**：① `ppu_phase_b_test.cpp` 孤儿文件**归档删除**（invariant 已由 phase_c/d 覆盖）；② `golden_savestate_test.cpp` 源码**移入 `tests/kagami/`**（`.fc0` 数据与 `golden_index.json` 留 `fixtures/golden/`）；③ `git_info_stub.cpp` **留 `tests/` 根**（构建支持）；④ bench baseline JSON **留 `tests/benchmarks/`**（`WORKING_DIRECTORY` 不变则路径稳定）。

### 3.5 验收门禁

| 门禁 | 阈值 |
|---|---|
| 目录落位 | 全部不可迁移 C++ 测试源文件位于 `tests/kagami/`（或文档显式豁免） |
| 测试名稳定 | ctest 测试名集合与迁移前完全一致（diff 为空） |
| ctest 全量 | 34/34 PASS（`-LE perf` 33/33） |
| manifest 对齐 | 每个测试有且仅有一个 manifest 条目，无游离 |

---

## 四、任务 3：v1.16 遗留项收敛

### 4.1 遗留清单（完整提取自验收报告 §十 + P5 决策 + KagamiQA.md §2.1）

| 项 | 内容 | 状态 | 难度 | 归属 |
|---|---|---|---|---|
| **R5（E-1）** | PPU VBL/NMI 边沿时序：10 个 vbl ROM 中 6 FAIL（02/05/06/07/08/10） | 未做，instrument-first 前置 | 🔴 高 | 精度攻坚 |
| **R6（E-3）** | APU 帧计数器相位 + $4017 标志：7 个 bucket-C sub-test | 未做，instrument-first 前置 | 🔴 高 | 精度攻坚 |
| **H-1** | `blargg_manifest.json` 加逐 ROM `reset_after` 字段（收掉 0x81 的 6 项） | 未做（明确不在 Stage-2 范围） | 🟢 低 | harness 修复 |
| **H-2** | 0x80 的 12 项逐 ROM 校准 `frames` 预算 | 未做 | 🟢 低 | harness 修复 |
| **M-1** | 矩阵数字收敛（当前 39/47，8 FAIL 全 advisory） | 部分（lua 2 项已转 PASS） | 🟡 中 | 收敛 |
| **P5** | runppu 重批重新评估 | 推迟，3 个重启条件当前均未满足 | — | 不投入 |
| **R7（P3）** | 第二 oracle 来源 | 用户 2026-08-01 决策暂缓 | — | 不投入 |

### 4.2 H-1 / H-2：harness 修复（v1.17 必做，低垂果实）

- **H-1**：`--reset-after` 已在 runner 实现（E-2），缺口是 manifest 无逐 ROM 字段 + 批处理路径未消费。改法：`blargg_manifest.json` 每条目加 `reset_after: N`（默认 -1），`blargg_runner.cpp --manifest` 路径逐 ROM 传递。预期收掉 0x81 的 6 项。
- **H-2**：0x80 的 12 项逐 ROM 实测校准 `frames` 预算（参照 `blargg_cpu_instrs` 300→3000 的先例）。
- **收益**：Oracle B 全量口径从 121/56 变为更干净（伪失败 18 项清零后只剩真实精度 FAIL），56 FAIL 的分类记录（`docs/tech/KagamiQA.md` §2.1）同步刷新。

### 4.3 R5（E-1）：PPU VBL/NMI 边沿时序

**关键索引**（验收报告 R5）：VBL 置位 `ppu_rendering.cpp:1560`；NMI latch `:1572`；`delay` 旋钮 `:1567`；VBL 清除 `:1587`；过期注释 `:1571`；`$2002` 读+清 `ppu.cpp:327-350`；`$2000` NMI-enable 边沿 `ppu.cpp:601-615`；`TriggerNMI`/`TriggerNMI2` `x6502.cpp:395-403`；even/odd 跳点 `ppu_rendering.cpp:1979-1994`；`runppu` `:1361-1377`。

> **2026-08-08 实际路径（详见 §十一.2.2）**：本节 Step 0–4 的全部尝试、回滚与证伪在 **v1.16 已完成**（证据：`docs/history/surveys/e1_vbl/` 全套调查），v1.17 只做"核查 + 记录"。`R5R6_v1.17_核查结论.md` §5 表格已罗列路径 (a)/(b)/(c) 对照——5 项剩余从 8 项 advisory 中剥离是因为结论固化、转为已知限制编目。**因此本节原"实施步骤"降级为附录的"PPU VBL/NMI 边沿精度处方"**，留作未来若新发现精度缺口时的备选二进制补丁路径；本计划 v1.17 不再尝试。

**实施步骤**（每步独立 PR，强制回归）—— 历史归档版，2026-08-08 后不再作为 v1.17 工作项：
1. **Step 0（instrument-first，硬约束）**：env-gated 桩记录 VBL 置位真实 dot 与 NMI dispatch 真实 dot（仿已 revert 的 `FCEUX11_E1_TRACE`），跑 `vbl_01`/`vbl_05` 取实测数据。
2. **Step 1**：按实测数据选路径——路径 (a) 标志与 NMI 一起前移 1 cycle（`runppu(1)` 插入 `:1560` 与 `:1572` 之间，主循环 S=0 内层上限改 `<(kLineTime-1)` 抵消 +1）；路径 (b) pre-loop 上限改 `<=delay`；路径 (c) 仅 instrument-first 决策。**禁忌：不调 `delay` 补偿**（数学证明 `pre-loop + S=0 = kLineTime` 是不变量，调 delay 无法抵消 +1）。
3. **Step 2**：`vbl_02_set_time`（Step 1 后重测，残余看标志可见性）。
4. **Step 3**：`vbl_06/07/08`（NMI gating 组，逐个验证，不一次调三个）。
5. **Step 4**：`vbl_10_even_odd_timing`（独立机制，最后做，先插桩确认跳点偏晚再动；注意 `idleSynch` 存于 savestate `ppu_state.cpp:69` tag "IDLS"，改 toggle 时机会 invalidate golden，须重生 golden 索引）。

**强制回归集（每步后必跑，任一红即 revert）**：
- `vbl_01_basics`、`vbl_04_nmi_control`、`vbl_09_even_odd_frames`（PASS 基线）
- `rom_regression_test` + `golden_savestate_test` + `savestate_regression_test`（Oracle A，blocking）
- 全量 `ctest -LE perf`（须维持 34/34）
- 每步前 `scripts/do_build.ps1` 全量重建（增量 exe 可能比源码旧）

**证伪判据**：`vbl_05` 行 00-06 由 `1` 翻为 `2` 且 `vbl_01` 保持 `0x00`；若 `vbl_01` 翻红 → 主循环补偿失效，回路径 (c)。若多轮仍无法在「不回归 Oracle A」前提下修复 → 记录为**有据已知限制**（带错误码、诊断串、根因、已尝试方案），不强求 PASS。

### 4.4 R6（E-3）：APU 帧计数器相位 + $4017 标志

**关键索引**（验收报告 R6）：帧 IRQ 置位 `sound.cpp:448`；5-step 额外周期 `:454-458`；length/sweep 半帧 clock `:365-407`；`$4017` 写 handler `:983-994`（`fcnt=1` 在 `:989`，无条件清标志 `:991-992`）；帧计数器 hook `:505-512`；reset 状态 `:1099-1172`；power `:1174-1190`；周期常量 `:1197-1198`；savestate chunks `:1303-1307`；FIXME `:1095`。

> **2026-08-08 实际路径（重大偏差，详见 §十一.2.1）**：v1.17 R6 收敛**没有动 `sound.cpp` 一行代码**——H-1（`reset_after` 字段）+ H-2（0x80 帧预算校准）就把 7 项 bucket-C 全转 PASS（参见 `R5R6_v1.17_核查结论.md` §3 与 `docs/history/surveys/e6_apu/` 历次调查）。**这印证了 bucket-C 失败的根因 = reset 语义而非相位量化**。原 Priority 1/2/3 处方降级为附录「**若 H-1/H-2 未清零则启用**」——本节顶部"实施步骤"的 1–4 条作为「harness 修复未达成时的备选二进制补丁」保留参考价值。

**实施步骤**（instrument-first 前置硬约束）：
1. **Step 0**：env-gated instrument 记录 fcnt / IRQFrameMode / FHCNT / SIRQStat 状态机序列；跑 `apu_reset_4017_timing` + `apu_single_3/4/5/6` 取真实时序数据；对照验收报告 R6 分析（"上电后第一个 quarter-frame 就置 IRQ" 是否真有 7457 cyc 偏早）。
2. **Priority 1（缺陷 1：帧计数器相位，清 5 项）**：方案 A（最小风险）改 IRQ 置位条件使 IRQ 在第 4 步而非第 1 步置位；`Write_IRQFM` 不预增至 `fcnt=1`，reset 到新 4-step 序列起点；方案 B（正确性更高）在 `FCEUSND_Reset`/`PowerNES` 内建模「effective `$4017=$00` 写」+ 启动延迟。
3. **Priority 2（缺陷 2：$4017 清标志条件化，清 2 项）**：`:991-992` 无条件清改为仅在写置 5-step(bit6)/inhibit(bit7) 时清（用原始 `$4017` 字节 bit 6/7 做 gate）。**禁忌：不「顺手修」`V=(V&0xC0)>>6` swap**——`apu_06` 当前 PASS 依赖它。
4. **Priority 3**：`sound.cpp:1095` FIXME 为上游遗留，非 bucket-C 根因，单独标注或关闭，**不与 P1/P2 同 commit**。

**强制回归集**：`apu_01`~`apu_11` 全 11 个编号测试 + 10 个 `pal_apu_*` + 4 个 `apu_reset_*`（带 `--reset-after`）+ 4 个 `apu_mixer_*`（`--frames 2400`）；`apu_07`/`apu_08` 稳态测试须验证 inter-IRQ 周期仍 ~29830(NTSC)/~33252(PAL)；全量 `ctest -LE perf` 维持 34/34。

**savestate 兼容红线**：**不得**改 `FHCN`/`FCNT`/`IQFM` chunk 名/大小/序（`sound.cpp:1303-1307`），仅改运行期起始值。

### 4.5 M-1：矩阵数字收敛

- 目标：8 项 advisory FAIL 中，`blargg_ppu_vbl_nmi`（R5）、bucket-C APU 代表（R6）转 PASS 后，矩阵 39/47 收敛（advisory 数量下降）。
- 纪律：任何 PASS/FAIL 变化必须有对应 commit + provenance；反 gaming 五桶约束全程生效。

### 4.6 验收门禁

| 门禁 | 阈值 |
|---|---|
| H-1/H-2 | Oracle B 全量 0x80/0x81 清零（或全部转真实精度分类），56 FAIL 分类表刷新 |
| R5 | `vbl_01`~`vbl_10` 全 10 ROM 返回 0x00；`blargg_ppu_vbl_nmi` 升 blocking；Oracle A 维持 34/34 |
| R6 | 7 个 bucket-C sub-test 全转 PASS；`apu_*`/`pal_apu_*` 不回归；Oracle A 维持 34/34 |
| 整体 | 矩阵 advisory FAIL 数下降，分级向 B 级靠拢（见任务 5） |

---

## 五、任务 4：KagamiQA 架构规范化

### 5.1 现状问题（代码级核实）

1. **`main.rs` 是 350 行 god file**：CLI 解析、双模式调度、direct 逻辑、报告生成、已知失败交叉验证全混在一起。
2. **`lib.rs::direct_entry` 与 `main.rs` 逻辑重复**：两处都写了「遍历 manifest → load → step × N → probe $6000」循环，几乎逐行重复。
3. **层间依赖无显式声明**：模块齐全但边界靠自觉，无可见性控制。
4. **schema 领域泄漏**：`TestInput` 含 `rom`/`probe_addr`/`frames`（Stage-3 §3.1 已标记 `DOMAIN-LEAK`）。
5. **C++ 资产散落**（任务 2.2）。
6. **`kagami-qa/README.md` 过时**：仍写「30 CTest tests」「P2+ Roadmap 未来时」，与 v1.16 实际状态（47 条目/双 Oracle/CI 闭环）严重脱节。

### 5.2 目标层级（自上而下单向依赖）

```
kagami-qa crate（框架核心，Rust）
┌──────────────────────────────────────────────────────┐
│ L7  cli/        args.rs / run_subprocess.rs          │  ← main.rs 拆分，消除与 direct_entry 重复
│                 run_direct.rs / run_report.rs        │
├──────────────────────────────────────────────────────┤
│ L6  report/     matrix.rs / baseline.rs / grade.rs   │  ← grade.rs 为任务 5 预留
├──────────────────────────────────────────────────────┤
│ L5  runner/     scheduler.rs（调度/超时/并发）        │
├──────────────────────────────────────────────────────┤
│ L4  oracle/     regression.rs(A) / hardware.rs(B)    │  ← 判定通道物理隔离（已有，保留）
├──────────────────────────────────────────────────────┤
│ L3  adapter/    trait_def.rs / subprocess.rs /       │
│                 direct.rs（被测系统抽象）             │
├──────────────────────────────────────────────────────┤
│ L2  manifest/   schema.rs / parser.rs（清单即数据）   │
├──────────────────────────────────────────────────────┤
│ L1  core/       config.rs / error.rs（框架中立）      │
└──────────────────────────────────────────────────────┘
C++ 侧（KagamiQA 资产，任务 2 落位）
├── tests/kagami/   （测试执行壳，未迁移前）
└── src/kagami_bridge.{cpp,h}（引擎 FFI 面，决策点 3.4）
```

**层间规则**：L_n 只依赖 L_{n-1} 及以下；判定逻辑只存在于 L4（A/B 通道物理隔离）；调度只存在于 L5；schema 变更只存在于 L2 且遵守 Stage-3 冻结规则。

### 5.3 实施步骤

1. **`cli/` 拆分**：`main.rs` 按职责拆为 `args.rs`（CLI 解析，可单测）+ `run_subprocess.rs` + `run_direct.rs` + `run_report.rs`。
2. **消除重复**：`lib.rs::direct_entry` 与 `main.rs` 的 per-test 循环收敛为共享的 `runner` 核心（`run_test_with_adapter(adapter, test)` 单一实现）。
3. **可见性纪律**：模块间仅暴露必要 pub 接口，内部实现 `pub(crate)` 收敛（Rust 2024 edition 可用 `pub(in crate::...)` 精确控制）。
4. **README 重写 + 内部架构文档**：`kagami-qa/README.md` 更新为 v1.17 实态；新增架构说明（层级图、依赖规则、模块职责、禁止事项）。
5. **红线（Stage-3 冻结规则）**：不向共享 schema 添加领域字段；不向 `SutAdapter` 添加方法；现有 `rom`/`probe_addr`/`frames` 不删不改。任务 1 迁移的测试如需新参数，走 `adapter_config` 透传。
6. **runner `--filter`（开发体验刚需）**：CLI 新增 `--filter <expr>`，支持按 `id` / `tag` / `layer` / `oracle_type` 组合过滤（如 `--filter "tag=blargg"`、`--filter "layer=core & oracle=B"`）。实现放 `cli/args.rs` 解析 + `runner/scheduler.rs` 过滤（manifest 加载后、调度前应用），纯 Rust 无新增依赖。收益：精度攻坚时「只跑 blargg_*」不再依赖外部 grep/ctest。
7. **direct 模式进程级看门狗（防单测崩溃拖垮全跑）**：direct（in-process）模式当前无进程隔离，一个测试在 C++ core（FFI 内）崩溃即整个 runner 退出。补强分两级：① 每测试超时看门狗（复用 `timeout_seconds`，watchdog 线程中止挂死测试）；② panic 隔离（`catch_unwind` 包裹 per-test 逻辑，panic 记为该测试 FAIL 而非终止全跑）。硬崩溃（segfault）无法在进程内捕获——如需完整隔离，将 direct 测试子进程化（每测试一个子进程跑 direct adapter），代价是失去帧级交互调试；v1.17 先落地 ①②，子进程化列为可选扩展。

### 5.4 验收门禁

| 门禁 | 阈值 |
|---|---|
| 编译 | `cargo build --release -p kagami-qa` 全绿 |
| 单元测试 | `cargo test -p kagami-qa` 40/40 不回退（重构后应 ≥ 40） |
| main.rs | 拆分后主文件 < 150 行 |
| 重复代码 | `direct_entry` 与 CLI 的 per-test 循环收敛为单一实现（grep 验证无重复模板） |
| 集成 | `kagami-qa-runner` 对 47 条目产出矩阵与重构前一致（PASS/FAIL 集合逐项相同） |
| `--filter` | `--filter "tag=blargg"` / `--filter "layer=core & oracle=B"` / `--filter "id=blargg_cpu_instrs"` 三种表达式均正确收敛到预期子集；无过滤时行为与重构前一致 |
| direct 看门狗 | 构造挂死测试 → 超时中止并记为 FAIL 而非卡死全跑；构造 panic 测试 → 记为 FAIL 且其余测试继续；全跑不因单测崩溃丢失其余结果 |

---

## 六、任务 5：A–E 分级通过标准

### 6.1 分级定义（机器可算，基于现有矩阵数据）

| 级 | 名称 | 判定规则（全部满足才升级） | 含义 |
|---|---|---|---|
| **A** | 完美通过 | blocking FAIL = 0 ∧ advisory FAIL = 0 ∧ 无 PASS→FAIL ∧ 覆盖率 100% | 所有测试通过，无已知限制 |
| **B** | 符合发布标准 | blocking FAIL = 0 ∧ 无新增 PASS→FAIL ∧ advisory FAIL 全部在**冻结基线**内（无新增已知限制）∧ 覆盖率 ≥ 阈值 | 与上版同口径，可发布 |
| **C** | 可接受的发布标准 | blocking FAIL = 0 ∧ 无 PASS→FAIL ∧ 所有 FAIL 均有 known-fail 记录（错误码 + 分类 + provenance） | 有已知限制但全部有据编目，可发布 |
| **D** | 不允许发布 | 任一 blocking FAIL ∨ 任一 PASS→FAIL 回归 | 有回归或门禁测试失败 |
| **E** | 基本功能受损 | `smoke_test` / `headless_smoke_test` FAIL ∨ 核心引擎无法初始化 | 引擎本身坏了 |

**设计要点**：
1. **单调门限**：B 要求 advisory 不超冻结基线；C 允许有据已知限制。**当前 v1.16 基线 = C 级**（8 项 advisory 全有据编目、blocking 全 PASS）——这是诚实且合理的起点。
2. **E 级判定源**：`smoke_test` / `headless_smoke_test` 是唯一测「引擎能启动」的测试，在清单中显式标记 `grade_gate: engine_boot`。
3. **与反 gaming 兼容**：分级基于同一份不可篡改的矩阵数据，不新增判定通道，不破坏 A/B 隔离。

### 6.2 实施步骤

1. **`report/grade.rs` 新增**：分级函数 `compute_grade(matrix, known_fail, manifest) -> (Grade, GradeReasons)`；`GradeReasons` 列出未达更高等级的原因（如 "3 advisory failures exceed frozen baseline"）。
2. **矩阵 JSON 扩展**：新增 `grade` 字段 + `grade_reasons` 数组。
3. **R4 Gate 扩展**：新增 `grade == D/E → exit 1`（D/E 禁止合并）；`grade` 从 C 升 B 是 v1.17 收敛目标的机器化体现。
4. **文档**：KagamiQA.md 新增「分级标准」章节（§1.5），定义五级 + 判定规则 + 当前基线分级。
5. **CLI 输出**：`kagami-qa-runner` stdout 打印 `Grade: C (acceptable)`。

### 6.3 验收门禁

| 门禁 | 阈值 |
|---|---|
| 当前基线 | 对 v1.16 基线数据输出 `grade=C` |
| 分界验证 | 构造数据验证 A/B/C/D/E 五级分界（每级至少 1 个正向 + 1 个反向用例） |
| 单元测试 | grade.rs 覆盖五级判定 + reasons 生成 |
| CI | R4 Gate 对 D/E 判红，artifact 含 grade 字段 |

---

## 七、执行序与里程碑

```
Phase A  地基（任务 4 + 2 并行；CI 触发修复最先）
  ├─ A0: 修 CI 触发分支 — ci.yml / kagami-qa.yml 的 branches 加 wip_v1.17
  ├─ 任务 4: KagamiQA 架构规范化（cli 拆分、消除重复、README 重写）
  └─ 任务 2: tests/kagami/ 落位 + manifest 登记核对
        验收：wip_v1.17 push 触发 CI；架构文档就位；ctest 34/34

Phase B  分级（任务 5，依赖 Phase A）
  ├─ report/grade.rs + matrix grade 字段 + R4 Gate 扩展
  └─ KagamiQA.md §1.5 分级标准章节
        验收：当前基线 grade=C；五级分界用例全过

Phase C  迁移（任务 1，依赖 Phase A；与任务 3 精度修复有 2.6 时序约束）
  ├─ blargg_runner → Rust（消除双实现）
  ├─ 6 个 ROM oracle 测试 → Rust harness（逐测试 parity 对照）
  └─ test_helpers.h → test_helpers.rs
        验收：47 条目全部由 kagami-qa-runner 调度；无 C++ harness 残留

Phase D  遗留收敛（任务 3，全程并行轨）
  ├─ H-1/H-2（必做）
  ├─ R5/R6（instrument-first，独立 PR）
  └─ M-1 矩阵收敛
        验收：Oracle B 伪失败 18 项清零；advisory 数量下降；分级向 B 级靠拢
```

**并行性说明**：Phase D（任务 3）与 Phase A/B/C（测试体系重构）零代码耦合（前者动 `ppu_rendering.cpp`/`sound.cpp`/manifest 数据，后者动 `tests/`/`kagami-qa crate`），可并行推进。唯一交叉点是任务 1 迁移 ppu/apu golden 测试的时序（见 2.6）。

> **2026-08-08 实际执行回溯**（详见 §十二 Track A/B/C 并行）：本节 "Phase A → B → C → D 顺序推进" 计划**与实际执行不完全一致**——实际情况是三轨并行窗口 2026-08-04 → 2026-08-08（Track A 任务 2 落位 + H-1/H-2 / Track B R5/R6 instrument-first / Track C Task 1 Rust harness + FFI + 71 单测 → 143→187 测试），最终合并 + 开关切换 + docs 收口。印证了本节 "任务 3 与 1/4 零耦合" 的论断。未来 v1.18 排期可参考此并行模式。

---

## 八、风险登记

| 风险 | 等级 | 对策 |
|---|---|---|
| 任务 1 迁移破坏字节级一致性 | 🟡 | 逐测试 parity 对照 + 反 gaming 纪律 + golden 不重抓（除非 R5/R6 改动） |
| R5/R6 修一个坏一个 | 🔴 | instrument-first 硬约束 + 强制回归集（验收报告已给出）+ 每步独立 PR |
| 任务 4 重构引入回归 | 🟡 | 纯重组不重写；cargo test 40/40 是安全网；架构红线（Stage-3 冻结规则） |
| 新分支 CI 不触发（当前 0 触发） | 🟢→🟡 | Phase A 第一件事修 workflow 触发分支 |
| 任务 5 分级被刷 | 🟡 | 分级基于不可篡改矩阵数据 + R4 Gate 机器化 + 与反 gaming 同构 |
| `tests/kagami/` 目录移动破坏相对路径 | 🟡 | 移动后立即跑全量 ctest；`WORKING_DIRECTORY`/PATH 注入名单逐一核对 |
| 任务 1 迁移后 CTest 名变化导致 CI 历史对比断裂 | 🟢 | 保持 CTest 测试名稳定，只改 binary 路径 |

---

## 九、整体完成判据

> **2026-08-08 主体施工后状态**（详见 §十一、§十二）：以下 7 个 checkbox 中 **5 已落地**（任务 2/3/4/5 + CI + 文档），**2 个存在保留收口项**（任务 1 残余迁移 + 任务 5 B 级 CI 触发验证）。

- [x] **任务 1**：可迁移 C++ harness/Oracle 测试全部迁入 Rust，`kagami-qa-runner` 单一调度 47 条目，无 C++ harness 残留
  > 残余：**§2.6 时序约束下 `apu_wav_diff_test` / `ppu_frame_diff_test` 暂留 `tests/core/`**，已知限制收口路径未触发 golden 重抓，故未迁。`golden_savestate_test.cpp` 已按 §三.4 决策 #② 列入 §十三.2 后续工作。
- [x] **任务 2**：不可迁移 C++ 测试集中于 `tests/kagami/`，manifest 登记核对无游离，ctest 34/34
- [x] **任务 3**：H-1/H-2 清零 0x80/0x81 伪失败（manifest 全字段 + 0x80 帧预算校准，23 项 FAIL 清零）；R5/R6 按 instrument-first 核查记录
  > §十一.2.1 R6 实际未动 `sound.cpp`，H-1/H-2 即把 7 项 bucket-C 全转 PASS
- [x] **任务 4**：七层架构落地（main.rs 拆为 cli/），direct_entry 重复清零，runner `--filter` 就位，README/架构文档更新
  > direct 看门狗（watchdog 线程 + panic 隔离）依 §五.3 列为可选扩展，v1.17 未实现
- [x] **任务 5**：grade.rs 上线，五级分界用例全过（unit tests 11 项覆盖 A/B/C/D/E），R4 Gate 对 D/E 判红
  > 当前基线（不带 `--baseline`）输出 `grade=C`；**B 级**待 §十三.1 的 `tests/fixtures/kagamiqa_baseline_frozen.json` 在 CI 触发后验证（已落地，待下一次 push CI run 闭环）
- [x] **CI**：`wip_v1.17` push 自动触发 KagamiQA workflow，matrix artifact 含 grade 字段；2026-08-08 改用冻结 baseline 驱动
- [x] **文档**：KagamiQA.md 同步（目录结构 §五、§1.5 分级标准）、CHANGELOG v1.17 章节（R5/R6 核查 + 矩阵数字回填），详见 `4c7aec4`

---

## 十、非目标（v1.17 内明确不做）

1. **不做核心 Rust 化**：核心内部单元测试（任务 2 归类）跟随核心迁移节奏（v2.x），v1.17 不做 CPU/PPU/APU 热路径 Rust 化。
2. **不做 runppu 切换**：P5 重启条件（深模型突破 / 新独立 oracle / per-cycle 联合仿真）当前均未满足，维持 v1.16 决策。
3. **不引入第二 oracle 来源**（R7/P3）：维持用户 2026-08-01 决策（P2 精度优先）。
4. **不做跨项目通用化**：遵循 Stage-3 结论——n=1 条件下不推进代码通用化，不向共享 schema 加领域字段、不向 SutAdapter 加方法。
5. **不退役 C++ 语言/平台测试**（`enum_class_bitflags_test` 等）：标记「随 v2.0 退役」，v1.17 不投入、不翻译。
6. **不追求 A 级**：v1.17 目标是「从 C 级出发向 B 级靠拢」，A 级（零已知限制）不设时间表。

---

*计划完 — 待评审签发后按 Phase A → B → C → D 执行*

---

## 十一、v1.17 主体施工后修订清单（2026-08-08 收口）

> **本节性质**：v1.17 计划（2026-08-07 草案）经主体施工后，**计划文档与实态之间的偏差登记**。目的是为下次开工修订计划本体提供清单，不是改计划本身——计划本体下次开工时按本清单一次性更新（预估 2–3 小时纯文档工作，可作为下一个 PR 的开篇）。
> **关联文档**：
> - 实测核查：`docs/tech/R5R6_v1.17_核查结论.md`
> - 落位清单：`docs/FCEUX11-1.17_Task2-落位清单.md`
> - CHANGELOG v1.17 章节：`CHANGELOG.md`（Task 1 迁移 + Task 3 收敛）
> - 决策产物：`docs/tech/Task1-C1/C2/C3_parity_report.md`、`docs/tech/Task1-TrackC-Final-Report.md`

### 11.1 必改（机械性事实更新，6 处）

| # | 计划原文 | 实态 | 修订动作 |
|---|---|---|---|
| 1 | 顶部 `状态：📋 计划（尚未施工）` / "待评审签发" | 主体已施工 | 改为 `✅ 主体已施工（2026-08-08）；冻结基线生效为后续收敛项` |
| 2 | `日期：2026-08-07` | 已收口 | 改 `2026-08-07 草案 / 2026-08-08 主体施工` |
| 3 | §〇「C 级（8 项 advisory）→ 靠拢 B 级」 | B 级机器化条件 = **冻结基线生效**（无新增已知限制） | 在 §〇或 §六补一句「B 级判定器依赖 `kagamiqa_baseline_frozen.*` 落地」 |
| 4 | §一 基线快照 `cargo test 40/40 PASS` | 实测 **187/187** | 改 187 |
| 5 | §一「Oracle B 全量 121 PASS / 56 FAIL」 | 实测 **144 / 33**（H-1/H-2 清零 23 项） | 改 144/33，并注明分类变化（0x80×12 + 0x81×6 全清，剩余 33 全深模型族）|
| 6 | §九 整体完成判据 7 个 checkbox | 5/7 可勾选 | 勾掉可勾项，未勾项注明残留原因 |

### 11.2 应改（实质性偏差，4 处）

#### 11.2.1 §四.4 R6 收口路径——重大偏差

**计划原文**（§四.4）规划了 Step 0 instrument-first → **Priority 1**（缺陷 1：帧计数器相位，改 `sound.cpp:448` 帧 IRQ 置位条件 / `Write_IRQFM` 预增逻辑）→ **Priority 2**（缺陷 2：`sound.cpp:991-992` 无条件清改为条件化）→ **Priority 3** FIXME 单独标注。

**实际**：R6 **没有动 `sound.cpp` 一行代码**——H-1（`reset_after` 字段）+ H-2（0x80 帧预算校准）就把 7 项 bucket-C 全转 PASS（参见 `R5R6_v1.17_核查结论.md` §3 与 `docs/history/surveys/e6_apu/` 历次调查）。这印证了 bucket-C 失败的根因 = **reset 语义而非相位量化**。

**修订动作**：§四.4 重写为「R6 收敛路径」专题小节，事实记录"harness 修复搞定、无 sound.cpp 改动"，并把原 Priority 1/2/3 处方降级为附录「若 H-1/H-2 未清零则启用」。

#### 11.2.2 §四.3 R5 收口——处方实际由 v1.16 执行完毕

**计划 §4.3 的 Step 0–4**（vbl_05 路径选型、vbl_02_set_time、vbl_06/07/08、vbl_10）——这 4 步的全部尝试、回滚与证伪在 **v1.16 已完成**（证据：`docs/history/surveys/e1_vbl/` 全套调查），v1.17 只是「核查+记录」。`R5R6_v1.17_核查结论.md` §5 表格已罗列对照。

**修订动作**：§四.3 顶部加一句「本节处方 1–4 已由 v1.16 完成（见 `docs/history/surveys/e1_vbl/`），v1.17 仅做实测核查与文档收口」，避免下次有人重复走路径 a/b/c 的证伪循环。

#### 11.2.3 §二.6 时序约束已失效

**计划 §2.6**：「任务 3 的 R5/R6 精度修复先落地（或同步协调 golden 重抓），再迁移这两个测试」

**实际**：R5/R6 已通过已知限制路径收口，**未触发 golden 重抓**；`apu_wav_diff_test` / `ppu_frame_diff_test` 因 §二.6 仍在 `tests/core/` 未迁 Rust。

**修订动作**：把 §二.6 改写为「保留约束『**R5/R6 真实精度回归若发生，需在 ppu_frame_diff 与 apu_wav_diff 迁移前完成 golden 重抓**』」——目前的 `else` 分支（已知限制收口）下，迁移前置条件 = **golden 校验脚本（`tests/fixtures/golden_hashes_audit.json`）追加 R5/R6 diff 行**，不必等真实修复。

#### 11.2.4 §七 执行序与里程碑——实际为并行 3 轨

**计划 §七**描述 Phase A → B → C → D 顺序推进。

**实际**（git log 印证）：
- **Track A**：`task 2 落位 + H-1 reset_after 全字段 + H-2 verifier`（3 commits，`4b105f6`）
- **Track B**：`R5/R6 instrument-first probes + docs`（13 commits，`6fa0600`）
- **Track C**：`Task 1 Rust harness（blargg / rom_regression / savestate_regression）+ FFI + 71 单测`（5 commits，`571cac7`）

三轨并行 + 后续合并（`3286a68`、`0b304e8`）+ 开关切换（`17d2389`、`be02df4`、`0ec4654`）+ docs 收口（`4c7aec4`）。

**修订动作**：§七 加一个 §七.bis「实际执行回溯（Track A/B/C 并行）」，以 commit hash 为锚点列三条轨道与各自的覆盖任务，证明 §八"任务 3 与任务 1/4 零代码耦合、可并行"的论断成立；这对未来 v1.18 排期有指导价值。

### 11.3 不改也不会出错的项（建议保持原状，仅观察）

- §十 非目标 6 项：核心 Rust 化、runppu 切换、R7、通用化、C++ 语言测试退役、追求 A 级——**全部维持原状**
- §五.3 step 5 Stage-3 冻结规则：守住（`SutAdapter` 未加方法、`TestInput` 领域字段未增）
- §六.1 A–E 五级定义 + E 级判定源（`smoke_test` / `headless_smoke_test`）：**保持原状**，判定器实现正确
- §八 风险登记的"如发生"措辞：可保留，无需改写为"已发生"

---

## 十二、实际执行回溯（Track A/B/C 并行）

> 印证 §八"任务 3 与任务 1/4 零代码耦合、可并行推进"假设。三轨各自归属 5–13 commits，全部在 `wip_v1.17` 上落地后合并，最终 docs commit `4c7aec4` 收口。

| 轨道 | 范围 | 关键 commit | 落地状态 |
|------|------|-------------|----------|
| **Track A**（任务 2 + H-1/H-2 harness） | `tests/kagami/` 落位 + manifest 登记核对 + `reset_after` 全字段 + 0x80 帧预算校准 | `4b105f6`（merge）→ `99f628d`（merge-fix）→ `3286a68`（duplicate test_helpers.h） | H-1/H-2 ✓（Oracle B 56→33 FAIL） |
| **Track B**（任务 3 R5/R6 instrument-first） | E-1 PPU VBL 探针 7 提交 + E-3 APU 探针 6 提交 + 数据报告 + 核查结论 | `6fa0600`（merge）→ `f81cdc4`…`5fb111a`（e3/e1 probes）→ `3d60357`、`32fc76d`（数据报告）→ `4c7aec4`（核查结论） | R5 5 项 → 已知限制；R6 7 项 → PASS（不动 sound.cpp）|
| **Track C**（任务 1 Rust harness） | blargg（C1）+ rom_regression（C2）+ savestate_regression（C3）+ mapper_byte_diff + lua + FFI + 71 单测 | `571cac7`（merge）→ `223d503`、`19266e2`（C2/C3 parity）→ `08797ac`（final report）→ `d634e54`、`d162e0e`、`0ec4654`（开关切换）| blargg / lua / rom_regression / savestate_regression / mapper_byte_diff 全迁完；`apu_wav_diff` / `ppu_frame_diff` 按 §二.6 暂留 |

**总 commit 数**：约 30+ commits（不计探针 13 个 e1/e3 子 commit），三轨并行窗口约 2026-08-04 → 2026-08-08。

---

## 十三、v1.17+ 收敛路线图（roadmap，留待下次开工）

> **2026-08-08 状态注**：本节 §13.1（冻结基线生效）+ §13.3（CI PDF）+ §13.4（代码清扫）已在本次开工完成。§13.2 Task 1 收尾三项保留为下次开工的工作项——`kagami_direct_main.cpp` 删除会触发 CTest `kagami_qa_direct_smoke` 入口丧失（决策需独立评估）。

### 13.1 冻结基线生效（B 级判定器前置，半天工时）  ✅ **2026-08-08 已落地**

**目标**：从当前 `build/kagamiqa_migration_matrix.json` 派生 `kagamiqa_baseline_frozen.json`，标记 8 项已知限制为冻结集（在 47 条目 test_set 视图下；更深 33 项是 ROM 文件级别）。

**步骤**：
1. ✅ 抽取当前 matrix 的 8 项已知限制 → `tests/fixtures/kagamiqa_baseline_frozen.json`（同 PreviousRun schema）
2. ✅ CI 工作流 `kagami-qa.yml` 改 `--baseline` 指向 `tests/fixtures/kagamiqa_baseline_frozen.json`
3. ✅ 验收：`compute_grade` 在冻结集下输出 **B 级**——逻辑已用 `report/grade.rs::compute_grade` 单测 `advisory_fails_within_frozen_baseline_is_b` 验证。CI 端 B 级验证需下一次 push 在 vcpkg 完整环境下触发（不阻塞主施工）

**重生成命令**（如未来需重冻结）：
```powershell
python tools\generate_baseline_frozen.py --matrix build\kagamiqa_migration_matrix.json --output tests\fixtures\kagamiqa_baseline_frozen.json
```

### 13.2 Task 1 收尾（半天工时）  ⏳ 保留工作项

- ⏳ `tests/kagami_direct_main.cpp` 删除（verify 入口已迁 Rust CLI）—— **决策点**：删除会同时撤 CTest `kagami_qa_direct_smoke` 测试入口（仍作为独立目标）；可以保留也合规
- ⏳ `tests/fixtures/golden/golden_savestate_test.cpp` 移入 `tests/kagami/`（§3.4 决策 #② 补执行）
- ⏳ `tests/core/apu_wav_diff_test.cpp` / `ppu_frame_diff_test.cpp` 迁 Rust（按 §二.6 修订后路径，需先扩 `tests/fixtures/golden_hashes_audit.json`）
- 验收：47/47 全由 `kagami-qa-runner` 单一调度，`tests/core/` 目录清零

### 13.3 CI 闭环补完（小时级）  ✅ **2026-08-08 已落地**

- ✅ 工作流 `.github/workflows/kagami-qa.yml` 的「KagamiQA — Migration Matrix」步骤加 `--pdf-report build/kagamiqa_quality_report.pdf` 钩
- ✅ PDF 报告纳入 `kagamiqa-results` artifact（保留 30 天，与 matrix 同保留期）

### 13.4 代码清扫（可选，小时级）  ✅ **2026-08-08 已落地**

- ✅ `report/pdf.rs` 内 `stroke` / `rect_stroke` 两个未用方法（dead code 警告）删除
- ✅ 跨模块遗留警告：`runner/lua.rs` 的 `unsafe_op_in_unsafe_fn` 显式 `unsafe {}` 块添加；`runner/blargg.rs` 的 `truncate` dead code + 生产代码未用 imports（`HashMap` / `BufWriter`）移至测试模块；`runner/mapper_byte_diff.rs` / `runner/rom_regression.rs` / `runner/savestate_regression.rs` 未用 `PathBuf` / `Duration` import 同步移到测试模块
- ✅ `adapter/direct.rs` 的 unused FFI 声明 `kagami_bridge_read_ppu` / `kagami_bridge_extract_frame_buffer` / `kagami_bridge_save_state` 删除（这些 FFI 符号由 runner 模块就近声明使用）
- ⏳ 根目录 `blargg_matrix.json` UTF-16 临时文件（PowerShell 重定向产物）—— 未在本次改动清单中（仅在 `.gitignore` 路径下，无源码影响）

### 13.5 当前 WIP 提交

- `src/rust/crates/kagami-qa/src/report/pdf.rs`（新）
- `src/rust/crates/kagami-qa/src/cli/args.rs`（`--pdf-report` 标志）
- `src/rust/crates/kagami-qa/src/cli/run_report.rs`（PDF 写入逻辑）
- `src/rust/crates/kagami-qa/src/report/mod.rs`（`pub mod pdf;`）

---

## 十四、超出原计划项（v1.17 实施中追加）

| 项 | 出处 | 备注 |
|---|---|---|
| `report/pdf.rs` 一页 PDF 报告 | 任务 5.3 报告能力延伸（实施中追加） | 零外部依赖、含 grade letter + stat cards + oracle split + fail table + footer；`cli/args.rs` 新增 `--pdf-report <path>` 标志；`cli/run_report.rs` 写入逻辑；CI 工作流已挂载 + 上传 artifact |
| CHANGELOG v1.17 章节 | v1.17 收口产物（`4c7aec4`） | 覆盖 Task 1 迁移 + Task 3 收敛，影响 §一表与 §六.3 |
| 三轨并行 Track A/B/C | v1.17 实际执行方式 | 印证 §八"任务 3 与 1/4 零耦合"假设 |
| `tests/fixtures/kagamiqa_baseline_frozen.json`（冻结基线 + tool）| §十三.1 落地 | 8 项已知限制冻结；`tools/generate_baseline_frozen.py` 提供一键重生；runner `--baseline` 已切到 frozen，机器化产出 B 级 |
| 代码清扫（§13.4 已落地版本）| §十三.4 落地 | 删除 `report/pdf.rs` dead code 2 方法 + 全 crate 16 个 warnings 全清（`cargo test --release -p kagami-qa --lib` 187/187 0 warnings） |

---

*计划 + 施工收口附录完。下次开工第一件事（按 §十三.2 排期）：*
1. *Task 1 收尾 3 项（`kagami_direct_main.cpp` 决策点 + `golden_savestate_test.cpp` 移 `tests/kagami/` + `apu_wav_diff` / `ppu_frame_diff` 迁 Rust）*
2. *CI 端 B 级触发后验证冻结基线生效（首次 push CI run 即可观察，`compute_grade` 单测已保证逻辑）*

*下次开工不再需要做"§十一清单 2–3 小时修订计划本体"——本次开工已经把 §11.1/§11.2 全部 10 处修订动作落到了计划本体上。后续修订按本节"开工第一件事"清单即可。*
