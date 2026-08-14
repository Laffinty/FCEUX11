# Phase 7a · v2.0 默认切换（精度冻结 80.8%）

> **目标**：把 `VNESU11_CORE` 默认改为 **ON**，把 Phase 6 收口的 `T1=143/177=80.8%` 冻结为 v2.0 baseline 交付，**不等** `T1≥85%` 的硬门槛（推迟到 Phase 7b）。
>
> **2026-08-14 路径 B 决议（PHASE6 收口会话确认）**：原 `phase_7_default_switch.md` 的 DoD（含 T1 ≥ 85%）过于乐观——A12 clocking deep model、PPU dot 粒度时序、DMA 仲裁均需多周 deep model 改动，不应阻塞 v2.0 发布。**精度攻关与发布解耦**：Phase 7a 发 v2.0（80.8% 冻结），Phase 7b 独立排期推 85%。

---

## 工期：2-3 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- `VNESU11_CORE` 默认改 **ON**
- `VNESU11_CORE=OFF` 仍可编译（CI 双构门禁）
- kagami-qa Oracle A 27 项 + Oracle B 21 项：新 FAIL = **0**
- blargg 177 ROM：新 FAIL = **0**（与 Phase 6 收口态持平）
- mapper_byte_diff 175 case：新 FAIL = **0**
- 真实游戏 smoke 脚本就绪（NROM/MMC3 共 8 游戏 + Castlevania/Contra 共 10 游戏）
- 性能 baseline 确认：`vn_perf_bench` ≤ v1.17×1.05（实测 743us vs 724us，ratio 1.026）
- Release 构建完整通过（PGO 可选，shadow run 必须可关）
- 文档同步（BuildGuide、ChangeLog、README）

### 1.2 ❌ 不在范围内（推迟到 Phase 7b / 7b+）
- **T1 ≥ 85% 精度攻关**（MMC3 A12 clocking + PPU dot 粒度 + DMA 仲裁 + CPU 中断时序）
- 完整 PPU/APU 5 通道 state mirror 扩展
- MapperMetaVtable::fill_audio（VRC6/FDS/N163 扩展音频）
- 真实游戏 smoke ROM 投放（owner 责任，等 owner 提供）
- C++ newppu=1 CPU/PPU 源删除（Phase 8）
- `VNESU11_CORE=OFF` 编译路径移除（v2.1 才做）

---

## 2. 任务清单

### 2.1 CMake 默认切换

```cmake
# CMakeLists.txt（根）
option(VNESU11_CORE "Use vNESU11 Rust core (default ON since v2.0)" ON)  # 改默认

if(NOT VNESU11_CORE)
    message(WARNING "VNESU11_CORE=OFF: legacy C++ CPU/PPU. Phase 7a keeps this for CI gate; will be removed in v2.1.")
endif()
```

### 2.2 Shadow Run 切换为可选

```cmake
# shadow run 默认开（开发期回归工具），Release build 必须关
option(VNESU11_SHADOW_RUN "Run C++ and vNESU11 side-by-side for equivalence check" ON)

if(NOT VNESU11_SHADOW_RUN)
    add_definitions(-DVNESU11_DISABLE_SHADOW=1)
endif()
```

Release 构建建议：
```powershell
cmake -B build-release -DVNESU11_CORE=ON -DVNESU11_SHADOW_RUN=OFF -DFCEUX11_PGO=ON
```

### 2.3 精度基线断言（与 Phase 6 收口态一致）

```bash
# 全量 blargg 177 ROM：T1 ≥ 143/177（80.8%）且 0 新 FAIL
cd tests
$env:VNESU11_RUST_PRIMARY="1"
..\build\tests\kagami_qa_blargg_runner.exe --manifest fixtures\blargg_manifest.json

# 期望：Passed=143 Failed=34（与 commit d7a7a19 一致），且与 blargg_known_fail.json 的 60 条失败 + v2.0 verified pass 27 条 + 新加 instr_misc = 60+27+1= 88 条对得上
```

### 2.4 真实游戏 smoke（脚本就绪 + dry-run 验收）

- `scripts/smoke_run_games.ps1`（Phase 6 已交付）保持可用
- 新增 CI gate：`scripts/ci_smoke_dryrun.ps1`（无需 ROM，调 `--help` / `--dry-run`，验证脚本逻辑 + 报告 SKIPPED）
- 真实 ROM 投放后激活：8 NROM/MMC3 + Castlevania + Contra 共 10 游戏 × 18000 帧 × 5 spot-check
- **Phase 7a 验收硬指标**：脚本逻辑 + dry-run 通过；ROM 投放 **不在 Phase 7a 范围**（owner 责任）

### 2.5 性能对比

```bash
# Phase 6 收口已实测：vNESU11 743us/帧 vs C++ 724us/帧，ratio 1.026（< 1.05 gate）
# Phase 7a 验证：Release 配置（PGO=ON，shadow=OFF）下重测，结论应一致
build-release\src\fceux11.exe -rom smb1.nes -bench 60
```

### 2.6 CI 工作流

```yaml
# .github/workflows/ci.yml 新增 Phase 7a 任务
- name: Build with vNESU11 (default ON)
  run: cmake -B build -DVNESU11_CORE=ON && cmake --build build

- name: Build with vNESU11 OFF (legacy gate)
  run: cmake -B build-legacy -DVNESU11_CORE=OFF && cmake --build build-legacy
  continue-on-error: false  # OFF 路径必须也构建通过（兼容垫片覆盖验证）

- name: Run kagami-qa Oracle A+B
  run: ./build/tests/kagami_qa_blargg_runner.exe --manifest tests/fixtures/blargg_manifest.json
  # 期望：Passed ≥ 143 Failed ≤ 34（与 Phase 6 收口一致）

- name: Run shadow run regression
  run: ./build/tests/kagami_qa_shadow_run_runner.exe
  # 期望：cpu_match ≥ 5（ADR-011 下限），实测 ≥ 46

- name: Cargo unit tests
  run: cargo test --release -p vnesu11 && cargo test --release -p kagami-qa
  # 期望：203/0/0 + 187/0/0

- name: Mapper byte diff
  run: ./build/tests/kagami_qa_mapper_byte_diff_runner.exe
  # 期望：175/175 PASS
```

### 2.7 文档同步

- `docs/BuildGuide.md`：更新构建步骤（默认 `VNESU11_CORE=ON`，`VNESU11_CORE=OFF` 为兼容回退）
- `docs/ChangeLog.md`：增补 `[2.0.0]` 条目
- `README.md`：vNESU11 章节标记"默认 ON"，ADR-011 引用 Phase 7b 排期说明
- `docs/wip_2.0_plan/`：每个 phase 文件标记完成，phase_7a/7b 列在目录
- `docs/tech/KagamiQA.md`：T1 门槛 85% 标注"Phase 7b 目标"，80.8% 标注"v2.0 baseline"

---

## 3. 验证策略

### 3.1 精度不回归（vs Phase 6 收口 commit `d7a7a19`）

| 指标 | Phase 6 收口 | Phase 7a 期望 |
|---|---|---|
| Rust-primary T1 | 143/177 = 80.8% | **≥ 143/177**（禁止新 FAIL） |
| shadow cpu_match | 46/59 | **≥ 46/59**（禁止退化） |
| cargo test (vnesu11) | 203/0/0 | **≥ 203/0/0** |
| cargo test (kagami-qa) | 187/0/0 | **≥ 187/0/0** |
| vn_perf_bench 帧时间 | 743us | **≤ 760us**（5% gate） |

任何新 FAIL = 阻塞 v2.0 发布。

### 3.2 默认切换回归（OFF 路径必须仍能编译）

- `cmake -B build-legacy -DVNESU11_CORE=OFF` 必须构建通过
- OFF 路径下 kagami-qa 行为与 v1.17 完全一致（兼容垫片覆盖度 = 100%）

### 3.3 真实游戏 smoke（dry-run 验收）

- 脚本可通过 `--help` / dry-run
- 无需 ROM 即报告 SKIPPED（不算 FAIL）
- ROM 投放到位后再跑 10/10 PASS = **Phase 7b 验收**（不是 7a）

---

## 4. 关键技术决策

### 4.1 精度冻结 80.8% 为 v2.0 baseline（路径 B 决议）

**理由**：
1. T1 80.8% 已超过 v1.17 frozen 67.8% 净改善 +13pp，达成"vNESU11 独立实现的诚实验证"。
2. 剩余 7 PASS 缺口（MMC3 A12 + PPU dot + DMA 仲裁）需 deep model 改动，风险高且耗时 4-5 周。
3. v2.0 用户视角：默认 ON 切换 + 80.8% 精度已是有意义的 release；85% 留待 v2.0.1 hotfix。
4. 不阻塞 Phase 8（C++ newppu=1 源删除）的 release 节奏。

**冻结算账**：80.8% 进入 `kagamiqa_baseline_v2.0.json`（v2.0 frozen），不再手动修改。

### 4.2 Shadow Run 默认 ON，但 PGO Release 必须 OFF

- 开发期 / CI：shadow run = ON（cpu_match ≥ 5 = PASS，ADR-011 下限）
- Release 构建（PGO=ON）：shadow run = OFF（否则 PGO 数据被 shadow 污染）

### 4.3 兼容垫片覆盖率保持 100%

- Phase 6 已达 100% FCEUI_* 覆盖（`src/core_api.h` 81 个 inline shim + 53 文件 × 366 调用点）
- Phase 7a 不修改兼容垫片（保持现状，Phase 8 再删除）

### 4.4 不在 Phase 7a 删任何 C++ 源

C++ CPU/PPU newppu=1 源保留到 Phase 8。Phase 7a 只翻转开关 + 加 CI，不改任何 C++ 源。

---

## 5. 风险

| 风险 | 严重度 | 缓解 |
|---|---|---|
| `VNESU11_CORE=ON` 默认后某个用户工作流破 | 🟠 中 | CI 双构 + smoke dry-run；Phase 7a 内 24h 监控 |
| OFF 路径在某个 commit break | 🟡 低 | CI 双构门禁 + 兼容垫片 100% 覆盖（Phase 6 已证） |
| 真实游戏 ROM 不到 = smoke 无法激活 | 🟡 低 | Phase 7a 只验收 dry-run；10/10 PASS 推到 Phase 7b / owner |
| PGO Release 性能退化 ≥ 5% | 🟠 中 | Phase 7a 末 PGO 验证；退化 → 触发 `A_performance_model.md §5` 备选 |
| 文档遗漏（BuildGuide / ChangeLog / README） | 🟡 低 | Phase 7a 末强制 review |

---

## 6. DoD

- [ ] `VNESU11_CORE=ON` 默认；可执行文件启动 + 加载 ROM 正常
- [ ] `VNESU11_CORE=OFF` 仍可编译（CI 双构门禁通过）
- [ ] kagami-qa Oracle A 27 项：新 FAIL = 0
- [ ] kagami-qa Oracle B 21 项：新 FAIL = 0
- [ ] blargg 177 ROM（Rust-primary）：**新 FAIL = 0**（T1 ≥ 143/177 = 80.8%）
- [ ] shadow run cpu_match ≥ 5（实测 ≥ 46）
- [ ] mapper_byte_diff 175 case：新 FAIL = 0
- [ ] 真实游戏 smoke 脚本：dry-run 通过
- [ ] FDS / NSF / VS 各 1 文件 smoke 路径验证（可用现有 ROM 集）
- [ ] cargo test -p vnesu11 ≥ 203/0/0
- [ ] cargo test -p kagami-qa ≥ 187/0/0
- [ ] vn_perf_bench ≤ v1.17×1.05（实测 ratio 1.026）
- [ ] PGO 构建：通过
- [ ] 文档全部更新（BuildGuide 默认 ON、ChangeLog `[2.0.0]` 条目、README vNESU11 章节标记默认 ON）
- [ ] CI 全绿（ON + OFF 双构 + kagami-qa Oracle A+B + blargg + shadow run + cargo test + mapper byte diff）
- [ ] **Release tag：`v2.0.0`**
- [ ] ADR-011 在 README 顶部"为什么 Rust 化"小节引用，标注"Phase 7b 推 85%"

---

## 7. 关键文件交付

```
修改：
  CMakeLists.txt                            # VNESU11_CORE=ON 默认 + VNESU11_SHADOW_RUN=ON 默认
  .github/workflows/ci.yml                  # 双构 + kagami-qa Oracle A+B + shadow run + cargo test + mapper byte diff
  docs/BuildGuide.md                        # 默认 ON 说明
  docs/ChangeLog.md                         # [2.0.0] 条目
  README.md                                 # vNESU11 章节标记默认 ON
  docs/wip_2.0_plan/README.md               # phase 7 行拆 7a + 7b
  docs/wip_2.0_plan/phase_7_default_switch.md  # 标记 superseded by 7a/7b
  docs/tech/KagamiQA.md                     # T1 门槛标注 v2.0=80.8% / v2.0.1=85% 目标
  docs/wip_2.0_plan/phase_6_integration.md  # §7.5 标注 85% = Phase 7b
  src/rust/crates/vnesu11/Cargo.toml        # 版本 0.x → 1.0
```

新交付：
```
  docs/wip_2.0_plan/phase_7a_default_switch.md   # 本文件
  docs/wip_2.0_plan/phase_7b_accuracy.md         # 精度攻关独立排期
  scripts/ci_smoke_dryrun.ps1                    # CI 验收脚本（无需 ROM）
```

---

## 8. 工期与排期建议

| 周 | 任务 |
|---|---|
| 第 1 周 | CMake 默认翻转 + CI 双构门禁 + kagami-qa Oracle A+B 全量验证 + blargg 基线断言 |
| 第 2 周 | shadow run 切换为可选 + PGO Release 构建 + 文档同步（BuildGuide / ChangeLog / README / phase docs）+ ADR-011 引用更新 |
| 第 3 周（可选） | MapperMetaVtable::fill_audio 残留 hook + savestate compat 回归 + final review + v2.0.0 tag |

总工期 2-3 周（与 Phase 6 收口 3 周同等优先级）。

下一步：[phase_7b_accuracy.md](./phase_7b_accuracy.md)（独立排期，4-5 周）。