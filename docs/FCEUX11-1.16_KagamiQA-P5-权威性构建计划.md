# FCEUX11 v1.16 KagamiQA P5 权威性构建计划

> **性质**：P5 runppu 重批 + 权威性加固，交付一个覆盖率 ≥80%、CI 常驻、frame-by-frame 可驱动的独立双 oracle 测试系统。
> **前提**：P0–P4 已交付（39 条清单、blargg 22 ROM 基线、Oracle A 全绿、Oracle B 17/22 PASS、01-vbl_basics FAIL→PASS）。
> **分支**：`wip_1.16`

---

## 〇、TL;DR

KagamiQA 当前是一个**可工作的原型**：清单驱动、双 oracle 分离、迁移矩阵产出均已实现。但它还不是一个**权威的质量防线**——原因有三：

1. **ROM 覆盖率仅 ~13%**（22/174 blargg ROM），大量精度风险不可见
2. **CI 未集成**，每次都是手动运行 → §十"平行王国化"风险已兑现
3. **in-process adapter 未接入 runner**，P5 需要的 frame-by-frame oracle probe 路径未打通

本次构建将 KagamiQA 从"原型"升级为"防线"，验收标准：

- **blargg 覆盖率 ≥80%**（≥140 ROM，覆盖全部子类）
- **CI 常驻**（每次 push 触发，迁移矩阵自动产出）
- **in-process 通道打通**（runner 经 C ABI 直接驱动 core，不再 fork 子进程）
- **Oracle A 全绿不退化**
- **Oracle B 基线完整版本化**

---

## 一、权威性定义与度量

### 1.1 权威性 = 信号可信度

```
权威性得分 = ROM覆盖率 × Oracle独立性 × CI常驻因子

当前:  0.13 × 1.0 × 0.0 = 0.00  ← 原型
目标:  0.80 × 1.0 × 1.0 = 0.80  ← 防线
```

| 因子 | 含义 | 测量方法 |
|---|---|---|
| ROM 覆盖率 | Oracle B 覆盖的 blargg ROM 比例 | `coverage = tested_roms / 174` |
| Oracle 独立性 | A/B 两个 oracle 是否完全解耦、互不污染 | 二元：双通道分离 = 1.0 |
| CI 常驻因子 | 是否每次 push 自动运行 | 二元：接入 CI = 1.0 |

### 1.2 权威性不要求的事

- **不要求 Oracle B 全绿**：已知失败清单本身就是权威性的一部分——精确知道什么失败，比"全绿但不测"更权威
- **不要求跨项目迁移**（§1.3 非目标 2）：SutAdapter 抽象层已预留，但 v1.16 不接入第二个被测物
- **不要求 100% ROM 覆盖**：~174 个 ROM 中有部分属于同一测试的不同版本（如 instr_test-v3/v4/v5），去重后有效覆盖 ~140 个即可 ≥80%

---

## 二、差距分析

### 2.1 blargg ROM 覆盖率

当前状态（22 ROM）：

| 类别 | 已覆盖 | 应覆盖 | 缺口 |
|---|---|---|---|
| CPU 指令 | 4 (all_instrs, official_only, cpu_timing_test, cpu_interrupts) | ~16 | branch_timing, dummy_reads, dummy_writes, instr_misc, instr_timing, 各指令族细分 |
| PPU Sprite Overflow | 5/5 ✅ | 5 | — |
| PPU Sprite 0 Hit | 11/11 ✅ | 11 | — |
| PPU VBL NMI | 1 | ~8 | vbl_clear_timing, nmi_suppression, vbl_nmi_timing 各变体 |
| PPU Open Bus | 0 | ~3 | ppu_open_bus |
| PPU OAM | 0 | ~5 | oam_read, oam_stress |
| PPU Scanline | 0 | ~3 | scanline_timing |
| APU | 1 | ~30 | dmc, noise, square, triangle 各子类 |
| **合计** | **22** | **~140** | **~118** |

### 2.2 CI 集成

当前：零。runner 完全手动调用。

目标：
- GitHub Actions workflow：每次 push 到 `wip_1.16` / `main` 触发
- Job 矩阵：`KagamiQA (Oracle A)` + `KagamiQA (Oracle B)`
- 报告上传为 workflow artifact
- baseline drift 检测 → PR comment（红色警报）

### 2.3 in-process adapter

当前状态：
- C ABI 桥接 (`src/kagami_bridge.h/cpp`) 已实现、已编译进 `fceux11_core`
- Rust FFI adapter (`src/adapter/direct.rs`) 已实现
- 但 `kagami-qa-runner` 仍使用 `SubprocessAdapter`（fork 子进程）

目标：
- runner 支持 `--direct` flag，走 `Fceux11DirectAdapter` 路径
- Oracle B 测试：load ROM → step() × N frames → read_oracle_probe(0x6000) → 判定
- 不需要 fork 子进程、不需要 blargg_runner 二进制

### 2.4 Lua 判定精度

当前：Lua 脚本不崩溃 = PASS。不解析 assert 结果。

目标：Lua runner 捕获 `assert()` / `error()` 输出并解析为 PASS/FAIL 信号。最小改动——修改 `lua_runner.cpp` 捕获 stderr 中的 `ERROR:` / `FAIL:` 模式。

---

## 三、分阶段实施

### Phase 5A：blargg ROM 覆盖率扩充（核心）

**目标**：≥140 ROM，覆盖 CPU/PPU/APU 全部子类。

| 步骤 | 产出 | 工作量 |
|---|---|---|
| A1. 下载完整 blargg 套件 | `tests/fixtures/blargg/` 下 ~174 ROM | 脚本化（已有 `download_blargg_roms.ps1`，扩参数） |
| A2. 生成 blargg_manifest.json（全量） | 每个 ROM 条目含 name / path / frames / probe_addr | 脚本化 |
| A3. 全量跑批 → Oracle B 原始基线 | `blargg_full_baseline.json`（每个 ROM 的 result_code + diag + duration） | 跑批 <10 分钟 |
| A4. 分类：PASS / KNOWN_FAIL / EXPECTED_FAIL | 标记每个 FAIL 是否 expected_to_eventually_pass | 对照 TASVideos 上游 FCEUX 表格 |
| A5. 更新 tests.json 清单 | 为每个 blargg ROM 添加清单条目（Oracle B） | 脚本生成 |
| A6. 更新 blargg_known_fail.json | 全量已知失败清单，版本化 | 手动审核 |
| A7. 重新生成精度对照表 | `docs/FCEUX11-1.16_KagamiQA-P5-accuracy-table.md` | runner 自动生成 |

**退出条件**：≥140 blargg ROM 进入清单体系，全量跑批通过，已知失败清单版本化，精度对照表完整。

### Phase 5B：CI 集成

**目标**：每次 push 自动运行 KagamiQA，产出迁移矩阵。

| 步骤 | 产出 | 工作量 |
|---|---|---|
| B1. 创建 GitHub Actions workflow | `.github/workflows/kagami-qa.yml` | 单文件 |
| B2. Oracle A job（ctest 封装） | 跑全部 A 类测试，收集 exit code | 复现 `ctest` 行为 |
| B3. Oracle B job（blargg runner） | 跑全量 blargg suite | 调用 `fceux11_blargg_runner --manifest` |
| B4. 迁移矩阵生成 | 调 `kagami-qa-runner` 产出 JSON → upload artifact | 复用 runner CLI |
| B5. baseline drift 检测 | 对照上次 run 的 baseline，标红 PASS→FAIL | Rust baseline.rs 从 stub 升级为真实实现 |
| B6. PR comment 集成 | 若 PASS→FAIL 非空 → 在 PR 下自动评论红色警报 | GitHub API |

**退出条件**：push 到 `wip_1.16` 触发 workflow，Oracle A + Oracle B 均自动运行，迁移矩阵作为 artifact 可下载。

### Phase 5C：in-process runner 打通

**目标**：runner 不再 fork 子进程，直接经 C ABI 驱动 core。

| 步骤 | 产出 | 工作量 |
|---|---|---|
| C1. CMake 目标：`kagami_qa_direct_runner` | 链接 `fceux11_core + fceux11_drivers_null + kagami-qa` 的单一可执行文件 | CMakeLists.txt 修改 |
| C2. main.rs 增加 `--direct` flag | 选择 `Fceux11DirectAdapter` 而非 `SubprocessAdapter` | Rust CLI 扩展 |
| C3. Oracle B 直接驱动路径 | load(rom) → step() × N → read_oracle_probe(0x6000) → 判定 | Rust 侧实现 |
| C4. 对照验证 | `--direct` 产出与 subprocess 模式一致 | 跑 blargg 全量对比 |

**退出条件**：`kagami-qa-runner --direct` 对 Oracle B 全量产出与 subprocess 模式 100% 一致。

### Phase 5D：runppu 精度攻坚（条件触发）

**前置条件**：5A 完成（全量 baseline 已知） + 5C 完成（in-process 可用）。

| 步骤 | 产出 | 工作量 |
|---|---|---|
| D1. 基于全量 baseline 识别 runppu-relevant 失败 | 分类：哪些 FAIL 是 PPU 时序 → 可被 runppu 修复 | 对照 TASVideos 表格 |
| D2. ppu_vbl_nmi 02-vbl_set_time 修复 | VBL cycle 0→1 shift（已知方向，需精确 cycle 计算） | 代码改动 + 实测验证 |
| D3. 逐项攻破 PPU 时序失败 | 每次改动后跑 Oracle A 全量 + Oracle B 全量 | 迭代 |
| D4. 最终迁移矩阵产出 | FAIL_TO_PASS 列表 + PASS_TO_PASS 全量 | runner 自动生成 |

**退出条件**：≥1 项新 PPU 时序修复（非 VBL period 修复）产生 FAIL→PASS，Oracle A 零回归。

**若重估不通过**（D1 发现无 runppu-relevant 失败）：冻结 D2-D4，附重估理由，P5 以"双 oracle 系统验证通过"结项。

### Phase 5E：Lua 判定精度加固

| 步骤 | 产出 | 工作量 |
|---|---|---|
| E1. lua_runner.cpp 增加 assert 捕获 | 解析 stderr 中的 Lua error/assert 消息 | ~20 行改动 |
| E2. 更新 tests.json Lua 条目 | 添加 `expected.assert_pattern` 字段 | schema 扩展 |
| E3. 回归验证 | 现有 4 个 Lua 测试仍 PASS | 跑 lua_runner |

---

## 四、验收门禁

| 门禁 | 度量方法 | 阈值 |
|---|---|---|
| blargg ROM 覆盖率 | `tested_roms / 174` | ≥80% (≥140 ROM) |
| Oracle A 全绿 | `rom_regression_test` 0 差异 | 100% |
| Oracle B 基线完整 | `blargg_known_fail.json` 含全量 FAIL 条目 | 100% 分类 |
| CI 常驻 | workflow 在每次 push 触发 | ✅ |
| 迁移矩阵 JSON 可获取 | GitHub Actions artifact | 每次 run 产出 |
| in-process 一致性 | `--direct` vs subprocess 对比 | 100% |
| baseline drift 检测 | PASS→FAIL 自动标红 | ✅ |
| Lua 判定可信 | assert 级别捕获 | 覆盖现有 4 脚本 |

---

## 五、风险登记

| 风险 | 概率 | 影响 | 对策 |
|---|---|---|---|
| ROM 下载源不可用 | 低 | 中 | ROM 已入 fixtures——下载脚本是幂等补充，fixtures 内已有 22 ROM 可独立工作 |
| blargg 全量跑批超时 | 中 | 低 | 单 ROM 平均 ~200ms × 174 = ~35s——远低于 3 分钟阈值 |
| CI runner 无 Qt 环境 | 中 | 高 | 用 `fceux11_add_headless_test_executable` + null driver——CI 不需要 GUI |
| in-process runner 链接 C++ core 失败 | 中 | 中 | CMake 目标可 fallback 到 subprocess 模式——CI 不阻塞 |
| runppu 改动触发 Oracle A 回归 | 中 | 高 | 每次改动后立即跑 Oracle A——回归即 revert |

---

## 六、关键约束重申

1. **新 PPU 是唯一方向**：绝不回退到旧 PPU。旧 PPU 无 PPU dot 概念，无法通过 blargg cycle 级测试。
2. **AI 不得修改已入库的 expected 值**：基线更新走与代码同级评审。
3. **Oracle A 全绿是每次修改的前置条件**：任何精度修复前必须先确认回归全绿。
4. **先收编后新建**：全量 blargg ROM 必须先入清单再改代码。
5. **headless 是 Oracle B 的基础**：CI 环境无 GUI，所有测试必须在 headless 下运行。

---

## 七、里程碑

```
5A (ROM 扩充)         ████████████  预计 2-3 轮对话
5B (CI 集成)          ██████████    预计 1-2 轮对话
5C (in-process 打通)  ██████        预计 1 轮对话
5D (runppu 精度)      ████          预计 1-2 轮对话（条件触发）
5E (Lua 判定)         ██            预计 0.5 轮对话
```

先执行 5A → 5B → 5C（并行推进，5B 和 5C 不互斥），然后基于 5A 全量 baseline 评估 5D 开工条件。

---

## 八、一句话收束

**把 KagamiQA 从"22 个 ROM 的原型"升级为"≥140 个 ROM 的 CI 常驻防线"——权威性不在框架代码的多寡，而在覆盖面的广度和信号的不可绕过性。**
