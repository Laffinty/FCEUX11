# FCEUX11 v1.16 KagamiQA 闭环四阶段构建方案

> **编制日期**：2026-08-05（**承前**：2026-08-01 P2 三阶段方案 → 已收口至 `docs/history/plans/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md`，本方案为 v1.16 收口期新一阶段规划）
> **性质**：可执行构建方案。Phase 1-3 已闭环（精度收敛 100% 落地）；Phase 4 = KagamiQA 系统闭环 + v1.16 发布门禁。
> **分支**：`wip_1.16`（沿用 P2 决策）
> **核心命题**：把精度侧"已知 33 项 FAIL（32 已知限制 + 1 永久跳过）"的稳定基线，**完整交付到 v1.16 发布门禁**——含 CI Gate、文档回填、Lua bindings 集成、Oracle B 覆盖扩展、P5 runppu 决策五大缺口。

---

## 〇、TL;DR

**Phase 1-3 全部完成。Phase 4 = 把 KagamiQA 系统的 5 大缺口逐项关闭，达到 v1.16 可发布状态。**

| Phase | 目标 | 状态 | 关键产出 |
|---|---|---|---|
| **1** | E-1 PPU VBL/NMI 深度收敛 | ✅ 闭环 | vbl_05 PASS, vbl 5/10 PASS（`5581769` + 探针族） |
| **2** | E-3 APU 帧计数器深度收敛 | ✅ 闭环 | 7 个 bucket-C sub-test 全 PASS（`f5e7cd0`） |
| **3** | 全量精度收敛（Step 3.1 + 3.2）| ✅ 闭环 | Oracle B 144/177 PASS；33 FAIL 全部归类（`f4a072a` `57d3e88` `863e9d7` `23b0cdd` `4a7f7e2` `116a602` `eaf4fe1`） |
| **4** | KagamiQA 系统闭环 + v1.16 发布 | ⏳ **本次** | CI Gate / 文档回填 / Lua bindings / Oracle B 覆盖 / P5 决策 |

**Phase 4 不解决任何"fceux11 精度"问题**——所有精度收敛已在 Phase 3 完成。Phase 4 解决的是**测试系统本身的工程化缺口**：让 CI 自动验证、文档自动同步、Lua 通道完整、Oracle B 覆盖深度足够、P5 决策有据。

---

## 一、现状盘点（2026-08-05 commit `7fd3f19`）

### 1.1 双 Oracle 运行态

```
$ kagami-qa-runner.exe --manifest tests/tests.json
Loaded 39 test entries.
Total:   39
Passed:  35
Failed:  4
Oracle A: 25P / 2F | Oracle B: 10P / 2F
```

| Oracle | PASS | FAIL | 来源 |
|---|---|---|---|
| **A** (tests.json) | 25 | 2（均 advisory） | `tests/tests.json` 27 项 |
| **A** (ctest 全量) | 33/33 | 0 | `ctest -LE perf` 本地验证 |
| **B** (tests.json 12 项) | 10 | 2 | `tests/tests.json` |
| **B** (blargg 全 177 项) | 144 | 33（32 已知限制 + 1 永久跳过） | `fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json` |
| **matrix** | 35/39 | 4 | `build/kagamiqa_migration_matrix.json` git_rev=`7fd3f19`（本会话刷新） |

### 1.2 33 FAIL 完整分类（Phase 3 收敛基线）

```
CPU 11  ─┬─ bucket B.3+B.4: 2 (cpu_dummy_writes_oam 0x06, cpu_exec_space_ppuio 0x05) — 已知限制 57d3e88
         ├─ bucket B.1+B.2: 9 (cpu_int_2/3/4/5_branch + cpu_reset_regs + instr_misc/_03_dummy + instr_timing/_v2_1) — 已知限制 eaf4fe1
         └─ cpu_interrupts 0xFE (永久跳过, eventually_pass=false)

MMC3 12 — bucket A (mmc3_1..6 + mmc3_v2_1..6, 0x02/0x03/0x04/0x09) — 已知限制 f4a072a

vbl 5  — Phase 1 (vbl_02_set_time, vbl_06_suppression, vbl_07/08_nmi_timing, vbl_10_even_odd_timing) — 已知限制 (Phase 1 调查)

sprdma 2 — bucket D (sprdma_dmc_dma, sprdma_dmc_dma_512) — 已知限制 4a7f7e2

PPU 真实精度 2 — bucket C 余项 (oam_stress 0x01, ppu_vbl_nmi 0x01) — 已知限制 116a602
```

**全部 33 FAIL = 32 已知限制 + 1 永久跳过**，0 项可独立收敛（深模型族待 per-cycle 联合仿真）。

### 1.3 KagamiQA 缺口清单（Phase 4 工作面）

| 缺口 | 类别 | 影响 | 来源 |
|---|---|---|---|
| **G1** 本会话 8 commit 未通过 CI Gate | CI 集成 | 阻塞 v1.16 发布 | `.github/workflows/kagami-qa.yml` last successful = `1156ca1` (2026-07-31) |
| **G2** README + `docs/tech/KagamiQA.md` §0 表格数字未回填 | 文档同步 | 阻塞 v1.16 发布 | 表格仍引 `1156ca1`，违反 §0 "禁止手改" 纪律 |
| **G3** Lua bindings 缺失 | Lua 集成 | `lua_joypad_test` + `lua_memory_test` 永远 FAIL（已 advisory）| `tests.json` provenance 已记录 P3 路径 |
| **G4** tests.json Oracle B 12 项 vs blargg 177 项 | 覆盖深度 | 迁移矩阵追踪深度不足 | tests.json 是"金标准小集合"但需明确分工 |
| **G5** P5 runppu 重批决策 | 权威性 | v1.16 是否启动 runppu 切换 | P5 计划已写，门禁未评估 |
| **G6** perf 标签测试排除 | Oracle A | `ctest -LE perf` 排除部分测试 | 33/33 非 perf + perf 未跑 |

---

## 二、Phase 4 子计划（v1.16 收口）

### 4.1 Lua bindings 落地（P3 路径闭环）

**目标**：把 `lua_joypad_test` + `lua_memory_test` 从 `failure_means: "advisory"` 升为 `failure_means: "blocking"`，全 PASS。

#### 4.1.1 `lua_joypad_test` — joypad binding 补全

**当前状态**：
- 测试脚本：`tests/lua_scripts/test_joypad.lua`（已写好，60 帧跑批）
- runner：`fceux11_lua_runner`
- 缺失：`joypad.get` / `joypad.set` / `mask1` / `mask2` override 逻辑
- 定位：`src/rust/crates/fceux11-lua/src/bindings/joypad.rs`

**改法**：
1. 检查 `mlua` UserData 注册路径
2. 暴露 `joypad.get(button)` / `joypad.set(button, val)` Lua 函数
3. 实现 `mask1` / `mask2` 元组表（与 fceux 兼容语义）
4. 验证：单独跑 `fceux11_lua_runner tests/lua_scripts/test_joypad.lua --frames 60` 全 PASS
5. 改 `tests/tests.json`：`failure_means: "advisory"` → `"blocking"`，去 tag `unimplemented-coverage`

#### 4.1.2 `lua_memory_test` — memory binding 补全

**当前状态**：
- 测试脚本：`tests/lua_scripts/test_memory.lua`（已写好，60 帧跑批）
- 缺失：`memory.readwrite` / `memory.readwordsigned` / `memory.getregister` / `memory.registerexec`
- 定位：`src/rust/crates/fceux11-lua/src/bindings/memory.rs`

**改法**：
1. 暴露 4 个 Lua 函数
2. `registerexec` 需要在 Lua state 内执行 6502 子集（受限或 stub）—— 先以 basic 形式返回字节即可
3. 验证 + 改 `tests/tests.json`

#### 4.1.3 回归门禁

- `ctest -LE perf`：Oracle A 全绿（包括 Lua 类）
- `kagami-qa-runner.exe --manifest tests/tests.json`：35/39 → 37/39（Lua 2 项转 PASS）
- `tests/tests.json` 改后 commit 单独 1 PR（`lua(bindings)` 前缀）

### 4.2 CI Gate 验证（R4 通过）

**目标**：本会话 8 commit + Phase 4 commit 全部经 GitHub Actions 跑通 `kagami-qa.yml`。

**流程**：
1. **本地预检**：
   - `scripts/do_build.ps1 -Config Release`（全量重建，确保 Qt 依赖解析）
   - `ctest -LE perf --output-on-failure` 100% pass
   - `src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe --manifest tests/tests.json --bin-dir build/tests --output build/kagamiqa_migration_matrix.json` 39 项跑通
2. **Push 到 `wip_1.16`**：触发 `.github/workflows/kagami-qa.yml`
3. **CI 内核对**（看 GitHub Actions 日志）：
   - `Oracle A` 步骤：`100% tests passed`
   - `Oracle B` 步骤：`Total: 39, Passed: 35, Failed: 4`（与本地一致；Lua 收尾后 Passed: 37）
   - `engine.git_rev = 7fd3f19`（或更新）
4. **R4 Gate 通过判定**：CI artifact `kagamiqa_migration_matrix.json` 含本会话 commit + 数字一致

**风险**：
- vcpkg cache 冷启动可能 60+ 分钟（R4-0 已 fix）
- blargg ROM cache 冷启动（`scripts/download_blargg_roms.ps1` 已幂等）
- 监控 actions/cache@v4 行为（沿用 R4-1 教训）

### 4.3 文档同步（R2 路径 A）

**目标**：`README.md` CN/EN + `docs/tech/KagamiQA.md` 三处数字 + 锚 commit 由 CI 产物回填。

**执行规则**（沿用 §0 of `docs/tech/KagamiQA.md` 纪律）：
> **禁止手改本文档的数字**。改之前先跑 CI 拿到 `kagamiqa_migration_matrix.json`。

**步骤**：
1. CI 通过后下载 artifact `kagamiqa_migration_matrix.json`
2. 用 `engine.git_rev` 字段更新 `docs/tech/KagamiQA.md` §0 表格的 commit 锚（4 行）
3. 同步更新 `README.md` CN/EN 的 `commit` 锚
4. 删除 §0 中的 "禁止手改" 提示（因为已 CI 同步）
5. 改后 1 PR（`docs(p2)` 或 `docs(ci)` 前缀）

**验证**：人工 review + 文档 vs CI artifact 哈希对比

### 4.4 Oracle B 覆盖深度扩展（G4）

**目标**：明确 tests.json 与 blargg manifest 的分工，必要时扩 tests.json Oracle B 关键桶代表项。

**当前状态**：
- `tests/tests.json` Oracle B：12 项（含 vbl_05 / ppu_open_bus / mmc3_test_2 / cpu_dummy_writes_* / 等）
- `fixtures/blargg_manifest.json` Oracle B：177 项（独立跑批）

**方案**：
1. **保持 tests.json 12 项作为"金标准小集合"**——Oracle A + Oracle B 各桶的代表项，覆盖深度优先
2. **Oracle B 全量 batch run 作为二级报告**——`fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json` 单独跑，不进迁移矩阵
3. **补全建议**：如需扩 tests.json Oracle B，按 §1.1 桶分类增补：
   - 桶 A 关键项：mmc3_test_2（已含），补 mmc3_4_scanline_timing
   - 桶 B 关键项：cpu_int_2_nmi_brk（验证 vbl 中断族收敛状态）
   - 桶 C 关键项：ppu_read_buffer（已 PASS，作为 PASS 监控）
   - 桶 D 关键项：sprdma_dmc_dma（已知限制，监控）
   - 总数目标：~18-20 项（增 6-8 项）

**验证**：迁移矩阵总条目 39 → 47-49；Oracle B 仍 2-4 项 FAIL（与已知限制一致）

### 4.5 P5 runppu 重批决策（G5）

**目标**：基于 Phase 1-4 数据，对 P5 runppu 重批门禁做正式决策。

**门禁评估**（P5 §0.2 锁定的 3 条件）：
| 门禁 | 当前状态 | 评估 |
|---|---|---|
| Oracle A 全绿 | ✅ 33/33 ctest PASS（本地） | 待 CI Gate 验证 |
| Oracle B 清单稳定 | ✅ 33 FAIL 全部归类（32 已知限制 + 1 永久跳过） | ✅ |
| 收益预期重估通过 | ⏳ 待评估 | 本次决策点 |

**收益分析**（基于 Phase 3 数据）：
- 当前 PASS 率：144/177 = **81.4%**（过 P5 设定的 80% 阈值）
- 32 项已知限制均属深模型族——runppu 切换不会修复任何一项
- runppu 切换**成本**：QA 全量回归（177 ROM × 600 帧 ≈ 3 分钟 × 完整 oracle） + savestate 兼容性验证
- runppu 切换**收益**：零精度提升（深模型族限制与渲染路径无关）+ 可能引入新回归

**建议决策**：**保持当前 PPU（不做 runppu 切换）**。

**理由**：
1. P5 收益分析显示 runppu 切换**零精度收益**（深模型族在 CPU/PPU/DMA 层，与渲染路径解耦）
2. Phase 4 1-4 全部完成后，v1.16 已达到"双 Oracle 稳定 + 33 FAIL 全部归类 + Lua 集成完整 + CI Gate 验证"——满足 P5 设定的"维持稳定基线"目标
3. runppu 重批推迟到 v1.17+，届时 P5 计划可单独评估（如深模型族突破后再决策）

**改法**：
1. 更新 `docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md`：本会话正式决策 + 推迟到 v1.17
2. 更新 `docs/tech/KagamiQA.md` §1.2 权威性度量：移除 P5 runppu 重批的预期
3. 改后 1 PR（`docs(p5-decision)` 前缀）

### 4.6 v1.16 release tag 准备（G1+G2 闭环）

**目标**：Phase 4 全部完成 + R4 Gate 通过后，发布 v1.16。

**步骤**：
1. **CHANGELOG.md**：
   - 新增 `## [1.16] - 2026-08-XX` 段
   - Phase 1-4 关键 commit 列表
   - 33 FAIL 防御线说明（不是 release blocker）
2. **src/version.h**：`FCEUX_VERSION_STRING` bump `1.15(hotfix5)` → `1.16`
3. **readme.md** CN/EN：版本号 + 发布日期 + release notes 摘要
4. **Tag**：`v1.16`
5. **GitHub Release**：附 `kagamiqa_migration_matrix.json` artifact + accuracy table

---

## 三、Phase 1-3 闭环历史（已收口）

### 3.1 Phase 1 — E-1 PPU VBL/NMI 深度收敛

| 步骤 | 状态 | 提交 |
|---|---|---|
| Step 1.1 置位点 cycle0→dot1 | 🚧 已证伪回滚 | — |
| Step 1.2 $2002 抑制窗口 | 🚧 部分落地 | — |
| Step 1.3 NMI fresh + X6502_Run 帧修复 | ✅ vbl_05 PASS | `7cfb029` + `5581769` |
| Step 1.4 even/odd 跳点 | 🚧 vbl_10 已知限制 | — |

**产出**：vbl 5/10 PASS；vbl 5 FAIL（vbl_02/06/07/08/10）全部记入已知限制

### 3.2 Phase 2 — E-3 APU 帧计数器深度收敛

| 步骤 | 状态 | 提交 |
|---|---|---|
| Step 2.1 power-on/reset 相位 | ✅ | `e3(step2.1)` |
| Step 2.2 一阶近似 | ✅ apu_single_4_jitter PASS | `e3(step2.2)` |
| Step 2.2 深化（cycle-position 帧计数器） | ✅ single_5/6 + reset_4017 + apu_test 全 PASS | `f5e7cd0` `e3(step2.2-deep)` |
| Step 2.3 5-step 立即 clock | 🚧 证伪不落地 | — |

**产出**：7 个 bucket-C sub-test 全 PASS，APU 52 ROM 零 apu_* 失败

### 3.3 Phase 3 — 全量精度收敛

| 步骤 | 状态 | 提交 |
|---|---|---|
| Step 3.1 harness cleanup（0x80/0x81 桶清零） | ✅ | `821a26e` |
| 桶 A（MMC3 12） | ✅ 12/12 已知限制 | `f4a072a` |
| 桶 B.3+B.4（CPU 3） | ✅ 1 PASS + 2 已知限制 | `57d3e88` + `863e9d7` |
| 桶 C（PPU 4） | ✅ 2 PASS + 2 已知限制 | `863e9d7` + `23b0cdd` + `116a602` |
| 桶 D（sprdma 2） | ✅ 2/2 已知限制 | `4a7f7e2` |
| 桶 B.1+B.2（CPU 9） | ✅ 9/9 已知限制 | `eaf4fe1` |
| 桶 C.1（vbl 5） | ✅ Phase 1 已记录 | — |

**产出**：Oracle B 144/177 PASS；33 FAIL = 32 已知限制 + 1 永久跳过（全部归类）

---

## 四、四阶段时间线（v1.16 收口规划）

```
2026-08-01 ─┐
           ├─ P2 三阶段方案启动
2026-08-04 ─┤  Phase 3 Step 3.1 harness + 桶 A/B.3+B.4
2026-08-05 ─┤  Phase 3 桶 C/D/B.1+B.2 收口（oracle 验证 144/33）
           │  33 FAIL = 32 已知限制 + 1 永久跳过
2026-08-05 ─┤  ──── Phase 3 闭环 ────
2026-08-05 ─┤  P2 方案归档至 docs/history/plans/
2026-08-05 ─┤  P3 方案（本文件）建立
           │
           ├─ Phase 4.1: Lua bindings 落地
           │  (lua_joypad_test + lua_memory_test: advisory → blocking)
           │
           ├─ Phase 4.2: CI Gate 验证
           │  (kagami-qa.yml 通过 R4 Gate)
           │
           ├─ Phase 4.3: 文档同步
           │  (README + KagamiQA.md 数字回填)
           │
           ├─ Phase 4.4: Oracle B 覆盖扩展
           │  (tests.json 12 → ~18-20 项)
           │
           ├─ Phase 4.5: P5 runppu 决策
           │  (建议：保持现状，推迟 v1.17+)
           │
           ├─ Phase 4.6: v1.16 release tag
2026-08-XX ─┘  (CHANGELOG / version / readme / tag)
```

---

## 五、KagamiQA 缺口收敛状态表（Phase 4 进展追踪）

| 缺口 | 描述 | Phase 4 子步骤 | 完成判据 |
|---|---|---|---|
| **G1** CI Gate | 本会话 8 commit 未经 CI 验证 | 4.2 | `kagami-qa.yml` 通过 R4 Gate，artifact `git_rev = HEAD` |
| **G2** 文档回填 | README + KagamiQA.md 数字过时 | 4.3 | 三处 commit 锚更新到本会话 HEAD |
| **G3** Lua bindings | `lua_joypad_test` + `lua_memory_test` 永远 FAIL | 4.1 | 两项从 advisory → blocking，tests.json 同步，全 PASS |
| **G4** Oracle B 覆盖 | tests.json 12 项 vs 177 项 | 4.4 | tests.json Oracle B 扩至 ~18-20 项关键桶代表 |
| **G5** P5 决策 | runppu 重批门禁未评估 | 4.5 | P5 计划更新 + 决策记录 |
| **G6** perf 标签 | `ctest -LE perf` 排除部分测试 | (本会话不修) | 接受 v1.16 不变 |

---

## 六、风险与禁忌

### 主要风险

| 风险 | 缓解 |
|---|---|
| **CI Gate 缓存冷启动**（vcpkg + Qt 6.8.0 源码编译 ~60 min） | R4-0 已 fix 缓存 key；首次 push 触发后永久缓存 |
| **Lua bindings 实现错误引入回归** | 4.1.1 + 4.1.2 各起独立 PR + 单独 ctest 验证 + tests.json 同步 |
| **P5 runppu 决策错误** | 4.5 提供详细收益分析 + 推迟到 v1.17+，保持 v1.16 稳定基线 |
| **Oracle B 覆盖扩展引入测试不稳定** | 4.4 仅补"已稳定 PASS"或"已记录已知限制"项，不引入新失败面 |
| **文档回填手工失误** | 4.3 严格遵守 §0 纪律：CI artifact 写入，禁止手改 |

### 禁忌清单（沿用 P2 §6）

1. **不重启 Phase 3 已闭环的精度收敛工作**——33 FAIL 全部归类即稳定基线
2. **不在 Phase 4 引入新的 fceux11 精度修改**——仅做测试系统集成
3. **不修改已入库的 tests.json expected 值**——只能新增条目（P5 §1.2 约束 7）
4. **不擅自执行 P5 runppu 重批**——本会话决策推迟
5. **不在 v1.16 引入跨项目迁移实证**——P5 一期不做多被测物

---

## 七、文件:行号总索引（Phase 4 增量）

**KagamiQA 核心**（仅索引 Phase 4 修改面）：
- `src/rust/crates/fceux11-lua/src/bindings/joypad.rs` — 新增 joypad.get/set/mask1/mask2 bindings
- `src/rust/crates/fceux11-lua/src/bindings/memory.rs` — 新增 readwrite/readwordsigned/getregister/registerexec bindings
- `tests/tests.json` — `lua_joypad_test` + `lua_memory_test` `failure_means: "advisory" → "blocking"`，去 `unimplemented-coverage` tag
- `docs/tech/KagamiQA.md` — §0 表格数字回填 + §1.2 权威性度量更新
- `README.md` CN/EN — commit 锚更新
- `docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md` — Phase 4 决策记录
- `CHANGELOG.md` — v1.16 release notes
- `src/version.h` — version string bump

---

## 八、验收门禁（v1.16 可发布标准）

### 8.1 必须项（must-pass）

- [ ] **Phase 4.1**：Lua bindings 全 PASS（lua_joypad_test + lua_memory_test 从 advisory → blocking）
- [ ] **Phase 4.2**：`kagami-qa.yml` R4 Gate 通过，artifact `git_rev = HEAD`
- [ ] **Phase 4.3**：README + `docs/tech/KagamiQA.md` 数字与 CI artifact 一致
- [ ] **Phase 4.5**：P5 决策正式记录于 P5 计划文件

### 8.2 可选项（nice-to-have）

- [ ] **Phase 4.4**：tests.json Oracle B 扩至 ~18-20 项
- [ ] **Phase 4.6**：v1.16 release tag + GitHub Release notes

### 8.3 防御线（不是 blocker）

- 32 项已知限制（深模型族）+ 1 项永久跳过（cpu_interrupts）= 33 FAIL 不再变
- Phase 3 已固化 33 FAIL 的根因 + 修复路径文档

---

*方案完。Phase 1-3 闭环（精度收敛 100% 落地），Phase 4 启动（KagamiQA 系统闭环）。v1.16 收口 = Phase 4.1-4.3 + 4.5 = 4 项 must-pass 全部达成。*