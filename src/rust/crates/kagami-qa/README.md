# KagamiQA — FCEUX11 测试体系唯一归属与唯一门禁

KagamiQA（「鏡」QA）是 FCEUX11 的**双通道、零耦合模拟器精度验证系统**，也是本项目测试体系的
唯一编排层：全部测试（CTest 回归、blargg 硬件一致性、Lua 脚本、基准）都以 `tests.json`
清单条目的形式成为它的子项，由 `kagami-qa-runner` 统一调度、判定、报告与门禁。

> v1.17 定位（`docs/FCEUX11-1.17_计划.md`）：把 KagamiQA 从「FCEUX11 的附属测试框架」升级为
> 「测试体系的唯一归属与唯一门禁」。本文档对应 v1.17 实态（47 条清单 / 双 Oracle / CI 闭环 /
> 七层架构）。

## 双 Oracle 架构

```
┌──────────────────────────────────────────────────┐
│              kagami-qa-runner（CLI, L7）          │
│  cli/args.rs · cli/run_subprocess.rs             │
│  cli/run_direct.rs · cli/run_report.rs           │
├──────────────────────────────────────────────────┤
│  Oracle A（软件回归）     Oracle B（硬件一致性）   │
│  • CTest 单元/回归测试    • 177 blargg $6000 ROM  │
│  • exit code + stdout    • load→step×N→probe     │
│  • oracle/regression.rs  • oracle/hardware.rs    │
├──────────────────────────────────────────────────┤
│  报告：迁移矩阵（SWE-bench 同构五桶）· 精度表      │
│  基线漂移检测 · R4 Gate（CI 机器化门禁）          │
└──────────────────────────────────────────────────┘
```

- **Oracle A**（回归等价）：「和上一版一样吗」——判定通道 `oracle/regression.rs`。
- **Oracle B**（外部真理）：「和真实硬件一致吗」——blargg `$6000` 协议，判定通道
  `oracle/hardware.rs`。
- 两个通道**物理隔离、互不污染**；判定逻辑只存在于 L4。

## 七层架构（L1 → L7 单向依赖）

```
L7  cli/        args.rs · run_subprocess.rs · run_direct.rs · run_report.rs
L6  report/     matrix.rs · baseline.rs（grade.rs 为任务 5 预留）
L5  runner/     scheduler.rs · direct.rs（共享 in-process 执行核心）
L4  oracle/     regression.rs(A) · hardware.rs(B)   ← 判定通道隔离
L3  adapter/    trait_def.rs · subprocess.rs · direct.rs
L2  manifest/   schema.rs · parser.rs（oracle-as-data）
L1  core/       config.rs · error.rs（框架中立）
```

**层间规则**：L_n 只依赖 L_{n-1} 及以下；判定只存在于 L4；调度只存在于 L5；
schema 变更只存在于 L2，且遵守 Stage-3 冻结规则（不向共享 schema 加领域字段、
不向 `SutAdapter` 加方法）。

### 共享执行核心（`runner/direct.rs`）

`run_direct_rom_tests` 是 in-process 模式的**单一执行核心**：CLI `--direct` 与 C-ABI
入口 `direct_entry::kagami_qa_direct_main`（被 `tests/kagami_direct_main.cpp` 消费）
共用同一份 load→step→probe 循环，杜绝两处实现漂移（任务 4 去重产物）。

## 快速开始

```powershell
# 1. 下载 blargg 测试 ROM（一次性，177 个，~8 MB）
.\scripts\download_blargg_roms.ps1

# 2. 构建 runner（CI 同款）
cd src/rust
cargo build --release -p kagami-qa

# 3. 全量运行 + 产出迁移矩阵
..\..\src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe `
  --manifest tests\tests.json --bin-dir build\tests `
  --output build\kagamiqa_migration_matrix.json `
  --accuracy-table build\kagamiqa_accuracy_table.md `
  --known-fail tests\fixtures\blargg_known_fail.json
```

CLI 参数（`kagami-qa-runner --help` 语义，完整语法见 `cli/args.rs`）：

| 参数 | 默认 | 说明 |
|------|------|------|
| `--manifest` | `tests/tests.json` | 测试清单（oracle-as-data） |
| `--bin-dir` | `build/tests` | 测试二进制目录 |
| `--output` | `kagamiqa_migration_matrix.json` | 迁移矩阵 JSON |
| `--working-dir` | 当前目录 | 子进程工作目录 |
| `--accuracy-table` | — | Oracle B 精度表（Markdown） |
| `--known-fail` | — | 版本化已知失败清单 |
| `--baseline` | — | 上轮快照（对比迁移） |
| `--save-baseline` | — | 保存本轮快照 |
| `--filter` | — | 按子集运行，如 `--filter "tag=blargg"`、`--filter "layer=core & oracle=B"`、`--filter "id=blargg_cpu_instrs"`（`&` 为 AND，key ∈ id\|tag\|layer\|oracle） |
| `--direct` | — | in-process 驱动（stderr 诊断；权威报告仍走 subprocess） |

## 反 gaming 纪律（AI 协作设计）

- 迁移矩阵五桶：`fail_to_pass` / `pass_to_pass` / `pass_to_fail` / `fail_to_fail` / `new_test`。
  新增测试无论 PASS/FAIL 一律进 `new_test`，**不得灌水 `fail_to_pass`**。
- 每条 manifest 条目强制 `provenance`（来源可追溯），无来源不得入库。
- 基线更新与用例集合变更必须同级评审；PASS→FAIL 自动 PR 警报。

## 开发

```powershell
# 单元测试（当前 44+，含判定链路、反 gaming、CLI 解析）
cargo test -p kagami-qa
# 编译检查（含 direct-adapter feature 的 FFI 路径）
cargo check -p kagami-qa --features direct-adapter
```

> 注：`cargo build`（debug，无 feature）无法链接 bin——`adapter/direct.rs` 的 FFI 符号
> 需 C++ 侧提供，release 构建经 LTO 消除未用路径后方可链接（CI 即用 release）。
> 完整 CMake 链路（`kagami_qa_direct_runner`）见 `docs/BuildGuide.md` §10。
