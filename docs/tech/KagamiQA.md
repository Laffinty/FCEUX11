# KagamiQA — FCEUX11 双 Oracle 质量防线

> **版本**：v1.16  
> **性质**：双 Oracle（Oracle A 回归 + Oracle B 硬件一致性）自动化测试系统  
> **覆盖率（CI 产物快照 — commit `78a9d7f`，`engine.git_rev=78a9d7f`，`kagami-qa.yml` run #31）**：
> Phase 4.2 CI Gate（R4 通过）后按路径 A 统一刷新（2026-08-06）。
>
> | 维度 | 数值 | 来源 |
> |---|---|---|
> | CTest 注册测试 | 34（全 PASS） | `ctest -N` 输出（`build-c1/CTestTestfile.cmake`） |
> | `tests/tests.json` 清单条目 | 47 | `python -c "import json; print(len(json.load(open('tests/tests.json'))))"` |
> | blargg 落盘 ROM | 177 | `find tests/fixtures/blargg -name '*.nes' \| wc -l` |
> | `blargg_manifest.json` 条目 | 177（与落盘 1:1，死条目 0） | Stage-2 S-1 清掉 3 个重复死条目后 180 → 177 |
> | 当前矩阵 PASS / FAIL | 39 / 8 | 最近一次 `kagamiqa_migration_matrix.json`（`engine.git_rev = 78a9d7f`，CI run #31 2026-08-05；Phase 4.4 commit 锚待 CI 验证后回填） |
>
> **CI 状态**：每次 push 到 `main` / `wip_1.16` 自动触发，产出迁移矩阵 artifact。

> **§0. CI 数字回填纪律（Stage-2 P2-5 → Phase 4.2 R4 通过后已 CI 同步）**
>
> 上面 4 行数字的**唯一可信来源**是 CI 产物：
> - CTest / manifest 数字 = `ci.yml` 的 ctest 步骤输出
> - blargg 落盘数字 = `D-1` 清单（`docs/history/checklists/FCEUX11-1.16_blargg_接入清单.md`）
> - 矩阵 PASS/FAIL = `kagami-qa.yml` 的 `kagamiqa_migration_matrix.json` artifact
>
> **本文档数字以 CI artifact 为准**。Phase 4.2 R4 Gate 已闭环（run #31 `engine.git_rev=78a9d7f`，2026-08-05），无需手动同步。
> 下次刷新前可按以下流程重跑 runner：
> ```powershell
> # 1. 重生 matrix
> & src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe `
>   --manifest tests\tests.json --bin-dir build-c1\tests --working-dir . `
>   --output build-c1\kagamiqa_migration_matrix.json
> # 2. 跑 ctest 列注册测试
> ctest --test-dir build-c1 -N | Select-String 'Test #' | Measure-Object
> # 3. 数落盘 ROM
> (Get-ChildItem -Recurse tests\fixtures\blargg\*.nes).Count
> # 4. 在同一 commit 里刷新本表 + 更新表头 commit 锚
> ```

---

## 一、原理 / Principles

### 1.1 什么是 KagamiQA

KagamiQA（「鏡」QA）是一个**双通道、零耦合的模拟器精度验证系统**。它的设计出发点是：**模拟器的 Bug 有两种完全不同的来源**——

| 来源 | 示例 | 检测方式 |
|------|------|----------|
| **内部逻辑错误** | C++ 重构引入的 reg 读写错位、边界溢出、空指针 | CTest 单元/回归测试（Oracle A） |
| **硬件行为偏差** | PPU VBL 时序偏差 1 个 cycle、APU 长度计数器时钟对齐错误 | blargg $6000 协议 ROM（Oracle B） |

这两个通道**完全解耦**、**互不污染**。Oracle A 是传统的软件回归测试（对比 golden hash），Oracle B 是对照真实 NES 硬件的金标准（「硬件怎么说，我们就得怎么做」）。

### 1.2 双 Oracle 架构

```
┌──────────────────────────────────────────────────┐
│                  KagamiQA Runner                  │
│  (Rust crate kagami-qa → kagami-qa-runner.exe)   │
├──────────────────────────────────────────────────┤
│                                                   │
│  ┌──────────────┐       ┌──────────────────────┐ │
│  │  Oracle A     │       │  Oracle B             │ │
│  │  (软件回归)    │       │  (硬件一致性)          │ │
│  ├──────────────┤       ├──────────────────────┤ │
│  │ • 34 CTest   │       │ • 177 blargg ROM     │ │
│  │ • 单元测试    │       │ • $6000 协议         │ │
│  │ • 回归测试    │       │ • CPU/PPU/APU/MMC3   │ │
│  │ • 边界测试    │       │ • headless 批处理     │ │
│  │ • 驱动:       │       │ • 驱动:               │ │
│  │   Subprocess  │       │   Subprocess 或       │ │
│  │   Adapter     │       │   Direct Adapter      │ │
│  └──────┬───────┘       └────────┬─────────────┘ │
│         │                        │                │
│         ▼                        ▼                │
│  ┌──────────────────────────────────────────────┐ │
│  │          Report Generation                    │ │
│  │  • 迁移矩阵 (FAIL→PASS / PASS→FAIL)           │ │
│  │  • 已知失败清单交叉验证                         │ │
│  │  • 基线漂移检测 (PASS→FAIL 自动 PR 警报)       │ │
│  │  • 精度对照表 (Markdown)                       │ │
│  └──────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

### 1.3 $6000 协议（Oracle B 核心）

Blargg 的测试 ROM 使用 **内存映射结果协议**：

```
$6000 = 0x00  →  PASS (所有子测试通过)
$6000 = 0x01+ →  FAIL (错误码指示具体失败类别)
$6001-$6003   →  诊断字节（可选，用于定位失败操作码/寄存器）
$6004+        →  ASCII 诊断字符串（blargg instr_test 错误详情）
```

ROM 的典型运行模式：
1. **自动测试**：ROM 启动后自动运行全部子测试，在 NMI / 主循环中完成
2. **写结果**：完成后将结果写入 $6000-$6003，然后进入死循环
3. **Runner 读取**：emulate N 帧后，通过 `ARead[0x6000]` 读取结果

### 1.4 权威性口径（Stage-2 §十·五 修订：已废止单一乘积分数）

> ⚠️ **旧公式已作废**，保留于此仅供追溯：
>
> ```
> 权威性 = ROM覆盖率 × Oracle独立性 × CI常驻因子
> v1.16:  1.00 × 0.50 × 0.50 ≈ 0.25
> v1.15:  0.13 × 1.00 × 0.00 = 0.00
> ```
>
> **废止理由**（Stage-2 §十·五）：三个因子里只有 ROM 覆盖率是测量量，另两个是二元自评 ——
> 同一份代码、同一天，作者自评得 `1.00`、审计复评得 `0.25`，**一个能被自己写成满分的数不是度量**。
> 且「Oracle 独立性」「CI 常驻」本质是**卫生条件**（不满足则结论无效，满足了也不增加真理含量），
> 把它们乘进权威性等于主张「跑得勤 = 更接近真理」。乘积形式还会让任一因子为 0 时总分归零，
> 掩盖其余部分的真实进展。

**现行口径：门槛 + 度量分离陈述**

```
【卫生门槛】（二元；不满足则以下度量无效）
  ☑ Oracle A/B 判定通道物理隔离
  ☑ 判定逻辑与 manifest schema 声明一致
  ☑ 迁移矩阵不含结构性失真（new_test 桶 + test_set_diff）
  ☑ 产物可追溯：matrix 带真实 engine.git_rev
  ☑ CI 常驻，指标由 CI 产物回填而非手写  ← Phase 4.2 R4 Gate 闭环（run #31/#32/#33 success）

【权威性度量】（仅在全部门槛满足时有意义）
  外部真理覆盖率 = 177 / 177 blargg ROM（manifest 与落盘 1:1）
  已知失败清单   = 47 项中 8 项 FAIL（Phase 4.4 扩展后），每条含 $6000 码 / 诊断串 / 分类 tag
  oracle 来源数  = 1（blargg）
```

**Phase 4.5 runppu 决策**（2026-08-06，本会话正式签发；详见 `docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md`）：

> **P5 runppu 重批推迟到 v1.17+**。理由：8 项已知限制全部归类于深模型族（CPU/PPU 寄存器/DMA 层），与渲染路径解耦，runppu 切换**零精度收益** + 引入新回归风险。Phase 1-4 已闭环（双 Oracle 稳定 + 33 FAIL 全部归类 + Lua 集成完整 + R4 Gate 验证），满足 P5 设定的"维持稳定基线"目标。
>
> v1.17+ 重新评估 runppu 的 3 个重启条件：(a) 深模型族突破；(b) 新独立外部 oracle 引入；(c) per-cycle 联合仿真就绪。

**「oracle 来源数」为什么单列**：当前唯一的外部真理来源是 blargg ROM 套件，因此
**权威性上限 = 该套件对真实硅片的保真度**。ROM 覆盖率从 13% 提到 100%，只是把这一个来源用尽，
不会突破它。继续提升权威性的路径是**引入相互独立、可彼此证伪的新来源**（NESdev 其他套件、
TASVideos 精度表、第二个模拟器差分、真机采集），而非在同一来源里继续堆测试数量。

**权威性不要求 Oracle B 全部 PASS**。精确知道什么失败，比「全绿但不测」更权威。已知失败清单本身就是防线的一部分。

### 1.5 分级标准（v1.17，Task 5）

每次运行产出一个**机器可算、可审计的发布等级**（`report/grade.rs` → 矩阵 JSON `grade` 字段），
判定基于同一份不可篡改的矩阵数据，不新增判定通道：

| 级 | 名称 | 判定规则（全部满足才升级） | 含义 |
|---|---|---|---|
| **A** | 完美通过 | 无任何 FAIL ∧ 无 PASS→FAIL ∧ 无 skipped | 所有测试通过，无已知限制 |
| **B** | 符合发布标准 | 无 blocking FAIL ∧ 无 PASS→FAIL ∧ 剩余 FAIL 全部在**冻结基线**内（`fail_to_fail`，无 `new_test` FAIL） | 与上版同口径，可发布 |
| **C** | 可接受的发布标准 | 无 blocking FAIL ∧ 无 PASS→FAIL ∧ 所有 FAIL 均为 advisory（有据已知限制） | 有已知限制但全部有据编目，可发布 |
| **D** | 不允许发布 | 任一 blocking FAIL ∨ 任一 PASS→FAIL 回归 | 有回归或门禁测试失败 |
| **E** | 基本功能受损 | `smoke_test` / `headless_smoke_test`（或带 `engine-boot` tag）FAIL | 引擎本身坏了 |

要点：

1. **单调门限**：等级是累积满足关系。B 需要能证明 FAIL 在冻结基线内（即 runner 带 `--baseline`
   且 transition 显示 `fail_to_fail`）；**无 baseline 时保守封顶 C**——这正是当前 CI 口径
   （不带 `--baseline`）下 v1.16 基线 = **C 级**的原因，且 `grade_reasons` 会明说差在哪。
2. **E 级判定源**：`smoke_test` / `headless_smoke_test` 是唯一测「引擎能启动」的测试，
   按其 id 或 `engine-boot` tag 识别。
3. **R4 Gate 扩展**（v1.17）：`grade` 为 D/E 时 CI 判红禁止合并；`grade` 从 C 升 B 是
   v1.17 收敛目标的机器化体现（要求以冻结 baseline 运行并保持无新增已知限制）。
4. **与反 gaming 兼容**：等级基于同一矩阵数据，新增测试进 `new_test` 桶的纪律不变；
   「新增 FAIL 即 C」的设计使「用新测试刷等级」无利可图。

---

## 二、使用帮助 / Usage

### 2.1 快速开始

```powershell
# 1. 下载 blargg 测试 ROM（一次性，177 个 ROM，~8 MB）
.\scripts\download_blargg_roms.ps1

# 2. 生成 ROM 清单
.\scripts\generate_blargg_manifest.ps1

# 3. 编译（确保已构建）
.\scripts\do_build.ps1 -Config Release

# 4. 运行 Oracle B — blargg 全量批处理
cd tests
..\build\tests\fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json
```

预期输出（截断）：
```
  [apu_01_len_ctr] 600 frames... PASS (0x00) 917ms
  [apu_02_len_table] 600 frames... PASS (0x00) 908ms
  ...
  [vbl_02_set_time] 300 frames... FAIL (0x01) 463ms
  ...
=== Blargg Suite Summary ===
Total:  177
Passed: 121
Failed: 56
```

**56 项失败的分类（2026-07-30 实测，`git_rev=623dd39`）** —— 按 §十·五「已知失败清单每条须含
错误码 + 诊断串 + 分类标记」的要求：

| `$6000` | 数量 | 含义 | 性质 |
|---|---|---|---|
| `0x80` | 12 | **测试仍在运行** —— 该 ROM 的帧预算不够 | 🔧 **harness 问题，非精度缺陷** |
| `0x81` | 6 | **"Press RESET"** —— ROM 要求在中途复位 | 🔧 **harness 能力缺口**（E-2 已给 runner 加 `--reset-after`，但 `--manifest` 批处理路径尚未逐 ROM 传递） |
| `0x01`–`0xFE` | 38 | 具体子测试失败（`0x01`×14、`0x02`×11、`0x03`×5、`0x09`×3、其余零星） | ⚠️ **真实精度问题** |

> **重要区分**：56 项里有 **18 项（32%）根本不是模拟精度缺陷**，而是 runner 的帧预算与 reset
> 能力没喂对 —— 与 S-1 修掉的 `instr_v5_all` 属于同一类错误。把它们和真正的精度失败混在一个
> "56 failed" 里报出去，会**高估**缺陷面并淹没真问题。真实精度待修面是 **38 项**。
>
> 按类目：cpu 17 / apu 15 / mmc3 12 / ppu 10 / 其他 2。
>
> **后续（不在 Stage-2 收官范围）**：给 `blargg_manifest.json` 增加逐 ROM 的 `reset_after` 字段并
> 让 `--manifest` 路径消费它，可一次性收掉 `0x81` 的 6 项；`0x80` 的 12 项则需逐 ROM 校准 `frames`。


### 2.2 运行 Oracle A（CTest 回归）

```powershell
# 全量回归（排除性能基准测试）
ctest --test-dir build --build-config Release --output-on-failure -LE perf

# 单个测试
ctest --test-dir build --build-config Release -R rom_regression
```

### 2.3 生成迁移矩阵

```powershell
cd src/rust
cargo run --release -p kagami-qa -- `
  --manifest ../../tests/tests.json `
  --bin-dir ../../build/tests `
  --output ../../build/kagamiqa_migration_matrix.json `
  --accuracy-table ../../build/kagamiqa_accuracy_table.md `
  --known-fail ../../tests/fixtures/blargg_known_fail.json `
  --baseline ../../build/kagamiqa_migration_matrix.json `
  --save-baseline ../../build/kagamiqa_baseline_next.json
```

输出文件说明：

| 文件 | 内容 |
|------|------|
| `kagamiqa_migration_matrix.json` | SWE-bench 同构迁移矩阵：`fail_to_pass` / `pass_to_pass` / `pass_to_fail` / `fail_to_fail` / `new_test` |
| `kagamiqa_accuracy_table.md` | Oracle B 精度对照表（Markdown），每个 ROM 的 PASS/FAIL + 错误码 |
| `kagamiqa_baseline_next.json` | 当前运行快照，保存为下次对比的基线 |

### 2.4 基线漂移检测

```powershell
# 第一次运行 — 无基线，生成基线
cargo run --release -p kagami-qa -- `
  --manifest tests/tests.json --bin-dir build/tests `
  --save-baseline build/kagamiqa_baseline.json

# 后续运行 — 对照上次基线
cargo run --release -p kagami-qa -- `
  --manifest tests/tests.json --bin-dir build/tests `
  --baseline build/kagamiqa_baseline.json `
  --save-baseline build/kagamiqa_baseline_next.json
```

漂移检测逻辑：
- `PASS → PASS`：正常，不计入迁移
- `FAIL → PASS`：✅ 进步！迁移矩阵 `fail_to_pass` 记录
- `PASS → FAIL`：❌ **回归！** 迁移矩阵 `pass_to_fail` 记录，CI 自动 PR 评论警报
- `FAIL → FAIL`：已知失败未变，计入 `fail_to_fail`
- **`new_test`：基线中不存在的 `test_id`，无论当前 PASS / FAIL 均单列此桶**（Stage-2 §四·五 PR 0.5-3）—— 见下方「§反 gaming」

### 2.4.1 反 gaming 与用例集合变更（Stage-2 §四·五 0.5-4）

**问题**：迁移矩阵的 `fail_to_pass` 桶存在被刷分的可能 —— 若 baseline 中无某 test_id，过去会被静默当作"此前 FAIL"，新增通过的测试便计入 `fail_to_pass`。SWE-bench 的 `FAIL_TO_PASS` 是任务实例预固定的清单，不存在此问题；KagamiQA 必须自行锚定。

**对策（4 层叠加）**：

1. **第 5 桶 `new_test`**（PR 0.5-3）：基线中不含的 test_id 全部进入此桶，不分 PASS/FAIL
2. **`test_set_diff` 字段可见性**（PR 0.5-4）：报告顶部字段，列出本次运行的 added/removed test_ids，PR Review 必须能 diff
3. **用例集合变更需与基线更新同级评审**：扩大 manifest 新增 test_id 与 `--save-baseline` 升级必须**同一 PR**，不允许先加测试再改基线（让 diff 双向可见）
4. **manifest 上的 `provenance` 字段必填**：每条 case 必须能追溯来源（hotfix4 T-1、blargg v1 等），禁止无 provenance 的"新增测试"

**新增测试 ≠ 自动通过**：任何含 `test_set_diff.added` 非空的 PR，必须在描述里显式列出新增条目及其预期分类（已知失败 / 新覆盖 / 误报暴露），CI 仍报 `new_test` 桶大小供审计。

### 2.5 In-Process Direct 模式（帧级驱动）

```powershell
# 编译 direct runner（需 CMake + Rust + direct-adapter feature）
cmake --build build --config Release --target kagami_qa_direct_runner

# 运行（in-process，不 fork 子进程）
.\build\tests\kagami_qa_direct_runner.exe `
  --manifest tests/tests.json `
  --output build/kagamiqa_direct_matrix.json
```

Direct 模式通过 C ABI 桥接（`src/kagami_bridge.cpp` → `src/kagami_bridge.h` → Rust FFI `direct.rs`）直接驱动模拟器 core，无需 fork 子进程。帧间可插入 oracle probe、状态快照、runppu 参数调整。

### 2.6 Lua 脚本测试

```powershell
# 运行 Lua 测试（headless）
.\build\tests\fceux11_lua_runner.exe tests/lua_scripts/test_bit.lua
.\build\tests\fceux11_lua_runner.exe tests/lua_scripts/test_emu.lua --frames 120
```

P5 升级后，`lua_runner` 会捕获 Lua `assert()` / `error()` 输出并解析为 PASS/FAIL 信号，不再仅依赖「脚本没崩溃 = PASS」的模糊判定。

### 2.7 CI 自动运行

KagamiQA 在每次 push 到 `main` 或 `wip_1.16` 分支时自动运行（`.github/workflows/kagami-qa.yml`）：

1. **Oracle A**：`ctest --output-on-failure -LE perf`
2. **Oracle B**：`fceux11_blargg_runner --manifest tests/fixtures/blargg_manifest.json`
3. **迁移矩阵生成**：`kagami-qa-runner` 产出 JSON + 精度表
4. **Artifact 上传**：矩阵、精度表、基线作为 workflow artifact 保存 30 天
5. **基线漂移警报**：PASS→FAIL 自动在 PR 下评论红色警报（`gh pr comment`）

也支持 `workflow_dispatch` 手动触发。

---

## 三、独立化运行 / Standalone Operation

KagamiQA 的核心组件可以**独立于 FCEUX11 项目**运行，用于测试其他 NES 模拟器。

### 3.1 独立化所需组件

| 组件 | 路径 | 说明 |
|------|------|------|
| blargg 测试 ROM | `tests/fixtures/blargg/` | 177 个 ROM（作者 Shay Green，社区惯例可自由用于模拟器测试），$6000 协议 |
| ROM 下载脚本 | `scripts/download_blargg_roms.ps1` | 从 GitHub 镜像下载，可独立运行 |
| ROM 清单 | `tests/fixtures/blargg_manifest.json` | JSON 格式，记录每个 ROM 的 path/frames/probe_addr |
| 已知失败基线 | `tests/fixtures/blargg_known_fail.json` | 版本化的已知失败清单 |
| kagami-qa-runner | `src/rust/crates/kagami-qa/` | 纯 Rust，无 C++ 链接依赖（subprocess 模式） |
| 分析脚本 | `scripts/analyze_blargg_results.ps1` | 结果分类统计 |

### 3.2 对第三方模拟器运行 Oracle B

**前提**：第三方模拟器必须提供一个可执行文件，接受 `--rom <path> --frames N` 参数，运行后通过 stdout 输出 `BLARGG_RESULT:` 行。

```
┌─────────────────────────┐
│  kagami-qa-runner       │  (纯 Rust, 无 C++ 链接)
│                         │
│  → spawn 子进程:         │
│    your_emu_blargg_runner │  ← 第三方模拟器的 blargg 包装
│    --rom fixtures/...    │     (需实现相同的 CLI 接口)
│    --frames 300          │
│                         │
│  → 解析 stdout:          │
│    BLARGG_RESULT:         │
│    rom=all_instrs.nes    │
│    value=0x80            │
│    status=FAIL           │
│                         │
│  → 产出迁移矩阵 JSON      │
└─────────────────────────┘
```

**最简实现**（C 伪代码）：
```c
// your_emu_blargg_runner.c
int main(int argc, char** argv) {
    const char* rom = parse_arg(argc, argv, "--rom");
    int frames = parse_arg_int(argc, argv, "--frames", 300);

    emu_init();
    emu_load_rom(rom);
    emu_run_frames(frames);

    uint8_t result = emu_read_byte(0x6000);
    uint8_t diag1  = emu_read_byte(0x6001);
    uint8_t diag2  = emu_read_byte(0x6002);
    uint8_t diag3  = emu_read_byte(0x6003);

    printf("BLARGG_RESULT: rom=%s addr=0x6000 value=0x%02X "
           "diag=[0x%02X,0x%02X,0x%02X] status=%s duration_ms=%lld\n",
           rom, result, diag1, diag2, diag3,
           result == 0x00 ? "PASS" : "FAIL",
           elapsed_ms());

    return result == 0x00 ? 0 : 1;
}
```

### 3.3 独立运行命令

```powershell
# 1. 下载 blargg ROM（通用，不依赖 FCEUX11）
git clone https://github.com/christopherpow/nes-test-roms.git
# 或
.\scripts\download_blargg_roms.ps1 -OutDir .\my_blargg_roms\

# 2. 生成清单
.\scripts\generate_blargg_manifest.ps1 -RomDir .\my_blargg_roms\ -OutFile .\my_manifest.json

# 3. 构建你的模拟器的 blargg 包装器（需实现上述 CLI 接口）
# ...

# 4. 运行 kagami-qa-runner（使用你的模拟器包装器）
cargo run --release -p kagami-qa -- `
  --manifest my_manifest.json `
  --bin-dir path\to\your\emu\binaries `
  --output my_migration_matrix.json `
  --accuracy-table my_accuracy_table.md
```

---

## 四、跨项目移植 / Cross-Project Migration

### 4.1 SutAdapter 抽象层

KagamiQA 的 `SutAdapter` trait（`src/rust/crates/kagami-qa/src/adapter/trait_def.rs`）定义了被测系统与 QA runner 之间的抽象接口：

```rust
pub trait SutAdapter {
    // Subprocess 模式 — 包装已有 CLI 二进制
    fn init(&self, config: &QaConfig) -> Result<(), QaError>;
    fn run_test(&self, test: &TestManifest) -> Result<TestResult, QaError>;

    // In-process 模式 — 帧级驱动（用于 runppu / 精度调试）
    fn load(&mut self, input: &InputSpec) -> Result<(), QaError>;
    fn step(&mut self) -> Result<(), QaError>;
    fn read_oracle_probe(&self, addr: u32) -> Result<u8, QaError>;
    fn snapshot(&self) -> Result<Vec<u8>, QaError>;
    fn reset(&mut self) -> Result<(), QaError>;
}
```

已有实现：

| Adapter | 模式 | 适用场景 |
|---------|------|----------|
| `SubprocessAdapter` | 子进程 fork | 任何实现了 CLI 接口的模拟器 |
| `Fceux11DirectAdapter` | In-process C ABI FFI | FCEUX11 专用，帧级调试 |

### 4.2 接入新模拟器（Subprocess 模式）

**步骤 1**：为你的模拟器创建一个 blargg 包装器二进制，实现 CLI：

```
your_emu_blargg_runner --rom <path> --frames <N>
```

stdout 输出格式：
```
BLARGG_RESULT: rom=<name> addr=0x6000 value=0x<XX> diag=[0x<XX>,0x<XX>,0x<XX>] status=PASS|FAIL duration_ms=<N>
```

**步骤 2**：在 `tests.json` 中添加测试条目：

```json
{
  "my_emu_blargg_suite": {
    "id": "my_emu_blargg_suite",
    "description": "Full blargg suite on MyEmu",
    "oracle_type": "B",
    "layer": "core",
    "input": {
      "binary": "my_emu_blargg_runner",
      "args": ["--manifest", "fixtures/blargg_manifest.json"],
      "working_dir": "."
    },
    "expected": { "exit_code": 0 },
    "timeout_seconds": 300,
    "tags": ["blargg", "oracle-b", "myemu"],
    "failure_means": "advisory"
  }
}
```

**步骤 3**：运行 kagami-qa-runner：

```powershell
cargo run --release -p kagami-qa -- `
  --manifest my_tests.json `
  --bin-dir path/to/my/emu/binaries `
  --output my_results.json
```

**无需任何 Rust 代码修改**。`SubprocessAdapter` 是纯进程通信的，对任何模拟器透明。

### 4.3 接入新模拟器（Direct/In-Process 模式）

如果你的模拟器提供了 C ABI（共享库），可以实现一个 `SutAdapter` 来帧级驱动：

```rust
// 示例：为 MyEmu 实现 DirectAdapter
unsafe extern "C" {
    fn myemu_init() -> i32;
    fn myemu_load_rom(path: *const c_char) -> i32;
    fn myemu_emulate_frame() -> i32;
    fn myemu_read_byte(addr: u16) -> u8;
    fn myemu_kill();
}

pub struct MyEmuDirectAdapter { /* ... */ }

impl SutAdapter for MyEmuDirectAdapter {
    fn load(&mut self, input: &InputSpec) -> Result<(), QaError> {
        unsafe { myemu_init(); myemu_load_rom(...); }
        Ok(())
    }
    fn step(&mut self) -> Result<(), QaError> {
        unsafe { myemu_emulate_frame(); }
        Ok(())
    }
    fn read_oracle_probe(&self, addr: u32) -> Result<u8, QaError> {
        Ok(unsafe { myemu_read_byte(addr as u16) })
    }
    // ...
}
```

C ABI 桥接模式参考 `src/kagami_bridge.h` + `src/rust/crates/kagami-qa/src/adapter/direct.rs`。

### 4.4 移植清单

| 可移植组件 | 路径 | 依赖 | 说明 |
|-----------|------|------|------|
| blargg ROM 套件 | `tests/fixtures/blargg/` | 无 | 社区惯例可自由用于模拟器测试（上游无 LICENSE 声明），直接复制 |
| ROM 清单格式 | `tests/fixtures/blargg_manifest.json` | 无 | JSON 格式，手工或脚本生成 |
| 已知失败格式 | `tests/fixtures/blargg_known_fail.json` | 无 | JSON 格式，手动维护 |
| kagami-qa crate | `src/rust/crates/kagami-qa/` | Rust + serde | 纯 Rust 库，可发布到 crates.io |
| SubprocessAdapter | `adapter/subprocess.rs` | 无 | 任何实现 CLI 的模拟器 |
| SutAdapter trait | `adapter/trait_def.rs` | 无 | 接口定义，复制到任何项目 |
| 迁移矩阵 schema | `report/matrix.rs` | serde | SWE-bench 同构 JSON 格式 |
| 下载脚本 | `scripts/download_blargg_roms.ps1` | PowerShell | 独立运行 |

### 4.5 自定义 Oracle 扩展

KagamiQA 的 Oracle 架构可以扩展以支持**非 blargg 测试 ROM**：

1. **Oracle C（性能基准）**：已有 `bench_tolerance_test`（Oracle B / benchmark layer）
2. **Oracle D（内存泄漏）**：可接入 AddressSanitizer / Valgrind 输出
3. **Oracle E（确定性）**：同一 ROM 连续运行 2 次，比对 savestate hash 是否一致

只需在 `tests.json` 中添加条目并实现对应的 adapter（或复用 `SubprocessAdapter` + 新的 CLI 工具）。

---

## 五、目录结构 / Directory Layout

```
FCEUX11/
├── tests/
│   ├── tests.json                          ← 47 条测试清单（27 Oracle A + 20 Oracle B；Phase 4.4 扩 8 项）
│   ├── fixtures/
│   │   ├── blargg/                         ← 177 blargg ROM (cpu/ppu/apu/mmc3/)
│   │   ├── blargg_manifest.json            ← ROM 清单（name/path/frames/probe_addr/reset_after；v1.17 H-1 全条目含 reset_after）
│   │   ├── blargg_known_fail.json          ← 已知失败分类（60 条，含 runppu 标记）
│   │   ├── blargg_full_baseline.json       ← P5 全量基线（120 PASS / 60 FAIL）
│   │   ├── golden/                         ← golden savestate 数据（.fc0 + golden_index.json；v1.17 决策：数据留此处）
│   │   └── nestest.nes                     ← smoke test ROM
│   ├── kagami/                             ← KagamiQA C++ 资产唯一落点（v1.17 Task2-A1 落位）
│   │   ├── *.cpp                           ← 🅑 永久驻留：核心单元/门禁/平台测试
│   │   │                                      （smoke/headless_smoke/core_state/enum_class/
│   │   │                                       expected_api/config_store/pixbuf_pool/i18n_regression/
│   │   │                                       ppu_phase_c/d/ppu_rendering_lut）
│   │   ├── boards/                         ← mapper_load/reset_test.cpp
│   │   ├── core/                           ← cpu/ppu/apu/bus/mapper/savestate/cart_class/fds_load/
│   │   │                                      driver_callbacks/core_driver_boundary_test.cpp
│   │   ├── benchmark/                      ← 5 Google Benchmark + ppu_simd_probe
│   │   ├── benchmarks/                     ← bench_tolerance_test.cpp（baseline JSON 留 tests/benchmarks/）
│   │   └── utils/                          ← xstring_microbench.cpp
│   ├── blargg_runner.cpp                   ← Oracle B 执行器（C++, headless；v1.17 待 Task1 迁移后删除）
│   ├── lua_runner.cpp                      ← Lua 脚本执行器（C++, headless；待迁移）
│   ├── kagami_direct_main.cpp              ← Direct runner C++ 入口（待迁移）
│   ├── rom_regression_test.cpp             ← ROM 回归（C++, 待迁移）
│   ├── savestate_regression_test.cpp       ← savestate 回归（C++, 待迁移）
│   ├── core/                               ← 🅐 待迁移：ppu_frame/apu_wav/mapper_byte_diff_test.cpp + test_helpers.h
│   └── git_info_stub.cpp                   ← 构建支持（留 tests/ 根，v1.17 决策③）
├── src/
│   ├── kagami_bridge.h                     ← C ABI 桥接头文件（v1.17 Track C：+extract_frame_buffer/save_state）
│   ├── kagami_bridge.cpp                   ← C ABI 桥接实现（编译进 fceux11_core）
│   └── rust/crates/kagami-qa/
│       ├── src/
│       │   ├── main.rs                     ← CLI runner 入口（v1.17 Task4 拆分后 <150 行）
│       │   ├── lib.rs                      ← Rust 库（含 C-callable direct 入口 + blargg/rom_regression/savestate C-ABI）
│       │   ├── cli/                        ← L7：args/run_subprocess/run_direct/run_report
│       │   ├── report/                     ← L6：matrix/baseline/grade（v1.17 Task5 A-E 分级）
│       │   ├── runner/                     ← L5：scheduler + direct（看门狗）+ blargg/rom_regression/savestate harness
│       │   ├── oracle/                     ← L4：regression.rs(A) / hardware.rs(B)
│       │   ├── adapter/                    ← L3：trait_def/subprocess/direct
│       │   ├── manifest/                   ← L2：schema/parser/filter（--filter 表达式）
│       │   └── core/                       ← L1：config/error
│       └── Cargo.toml
├── tools/
│   ├── add_reset_after.py                  ← v1.17 H-1：manifest reset_after 批量补全
│   ├── verify_manifest.py                  ← v1.17 H-1：manifest 完整性校验
│   └── verify_no_0x80.py                   ← v1.17 H-2：0x80/0x81 校准状态检测
├── scripts/
│   ├── download_blargg_roms.ps1            ← ROM 下载（177 条目，幂等）
│   ├── generate_blargg_manifest.ps1        ← 清单生成器
│   └── analyze_blargg_results.ps1          ← 结果分析/分类
├── .github/workflows/
│   ├── ci.yml                              ← 主 CI（build + ctest + benchmark）
│   └── kagami-qa.yml                       ← KagamiQA CI（Oracle A+B + 矩阵 + grade + 漂移检测）
└── docs/tech/
    ├── KagamiQA.md                         ← 本文档
    ├── R5_instrument_first_data.md         ← v1.17 R5 (E-1) PPU VBL/NMI 探针数据（Track-B）
    └── R6_instrument_first_data.md         ← v1.17 R6 (E-3) APU 帧计数器探针数据（Track-B）
```

---

## 六、FAQ

### Q: 为什么 Oracle B 有 60 个 FAIL 还算「防线」？

**A:** 精确知道 60 个 ROM 失败（有错误码、有诊断字符串、有分类标记），比「假装全绿但只测了 22 个 ROM」权威得多。已知失败清单是**版本化的**——每个新版本可以精确回答「哪些以前失败现在通过了（FAIL→PASS）」和「哪些以前通过现在回归了（PASS→FAIL）」。

### Q: 我可以只运行 Oracle B 而不用 Oracle A 吗？

**A:** 可以。`kagami-qa-runner` 的 `--manifest` 参数指向任何只包含 Oracle B 条目的 JSON 清单。双 Oracle 不是耦合的——它们是独立通道。

### Q: 如何把 KagamiQA 用于我自己的模拟器项目？

**A:** 见 §四。最简路径：① 下载 blargg ROM → ② 写一个包装器 CLI 二进制 → ③ 添加到 tests.json → ④ 运行 kagami-qa-runner。无需修改 KagamiQA 本身。

### Q: Direct 模式和 Subprocess 模式什么时候用哪个？

**A:** CI 日常用 **Subprocess**（已足够快，177 ROM 约 2 分钟）。精度调试（如 runppu 重批）用 **Direct**——可以在帧间插入 oracle probe、调整 PPU 参数、捕获中间状态，这是 subprocess 做不到的。

