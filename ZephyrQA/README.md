# ZephyrQA

> **状态**：P0 占位（构建计划已落于 `docs/FCEUX11-1.16_ZephyrQA-PLAN.md`，本目录暂无源码）
> **分支**：`wip_1.16`
> **定位**：FCEUX11 v1.16 主线工程——一个独立存在的软件测试框架，既是 FCEUX11 的质量防线，也是 AI 代理的"阅卷机"。

---

## 这是什么

ZephyrQA 是 FCEUX11 v1.16 引入的**独立测试系统**。它不是又一个测试目录，而是把 FCEUX11 现有但散落的测试资产（30 项 CTest、golden master、bench 协议、i18n 门禁）**收编**为一个统一的、清单驱动的、双 oracle 的、机器可判定的测试框架。

方向性来源：`docs/FCEUX11-1.16_ZephyrQA-PLAN.md`（基于桌面《v1.16 独立测试系统可行性与方向性评估》报告的工程化转译）。

## 设计意图：框架核心与被测物解耦

ZephyrQA 的 crate 内部分两层，这是它"能迁移到其他多种项目"的架构前提：

```
┌─────────────────────────────────────────────────────────┐
│  软件侧输入（人或 AI 编写）                              │
│  ┌────────────────────┐    ┌──────────────────────────┐ │
│  │  tests.json 清单    │    │  Lua 脚本（动态测例）     │ │
│  │  声明式 · 零代码    │    │  复用 fceux11-lua/mlua    │ │
│  └─────────┬──────────┘    └────────────┬─────────────┘ │
└────────────┼────────────────────────────┼──────────────┘
             │                            │
┌────────────▼────────────────────────────▼──────────────┐
│  ZephyrQA 框架核心层（被测物无关 · 可迁移）              │
│  manifest 解析 · runner 调度 · 双 oracle 判定           │
│  迁移矩阵报告 · 基线治理                                 │
│  仅依赖 SutAdapter trait，不依赖任何具体被测物           │
└────────────┬───────────────────────────────────────────┘
             │  SutAdapter trait
             │  load() / step() / read_oracle_probe() / snapshot()
┌────────────▼───────────────────────────────────────────┐
│  SUT adapter 层（被测物特定）                            │
│  ┌──────────────────┐  ┌──────────────┐  ┌──────────┐  │
│  │ FCEUX11 adapter  │  │ 未来项目 A    │  │ 未来项目B│  │
│  │ C ABI link core  │  │ ...           │  │ ...      │  │
│  │ ARead[$6000]     │  │               │  │          │  │
│  └──────────────────┘  └──────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────┘
```

- **框架核心层**：与被测物无关。清单 schema、调度、判定、报告、基线治理全部在此。迁移到新项目时这层原样复用。
- **SUT adapter 层**：被测物特定。FCEUX11 提供一个 adapter（经 C ABI 链接 `fceux11_core`，`ARead[0x6000]` 读取 blargg 测试结果）。未来迁移到其他项目，只需写一个新 adapter。

这一分层使"典范性"目标可证：**可迁移的是框架核心 + 方法论，不是 FCEUX11 代码**。

## 技术选型

| 维度 | 选型 | 依据 |
|---|---|---|
| 框架本体语言 | **Rust** | 顺势——FCEUX11 已有成熟 Rust workspace（7 crate + cbindgen + CMake 集成 + ffi_stubs），Rust 已是一等公民。ZephyrQA 作为新 crate 加入，零新工具链。 |
| 软件侧对接 | **Lua + 声明式 tests.json** | 双通道：常规测例用 JSON 清单（零代码，AI 友好）；动态测例用 Lua 脚本（复用既有 mlua 引擎与 16 个样本脚本，AI 生成 Lua 无需重编译、迭代快）。 |
| 被测物对接 | **C ABI link core** | 经 `SutAdapter` trait 抽象；FCEUX11 adapter 链接 `fceux11_core` 静态库。 |

## 目录规划（P1+ 实施）

```
ZephyrQA/
├── README.md                 ← 本文件
├── crates/
│   └── zephyr-qa/            ← Rust crate（P1 落地）
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── core/         ← 框架核心（被测物无关）
│           ├── manifest/     ← tests.json schema + 解析
│           ├── runner/       ← 调度
│           ├── oracle/       ← 双 oracle 判定（A 回归 / B 硬件）
│           ├── report/       ← 迁移矩阵 JSON
│           └── adapter/      ← SutAdapter trait + FCEUX11 实现
├── manifests/                ← tests.json 清单（P1 收编现有 30 CTest）
├── lua/                      ← Lua 测试脚本（P3 软件侧输入通道）
└── docs/                     ← 框架自身文档
```

## 当前状态

- ✅ 方向性报告已审阅并代码级交叉验证
- ✅ 技术选型已确定（Rust + Lua + JSON）
- ✅ AI 友好性与跨项目可迁移性已评估
- ✅ 构建计划已落于 `docs/FCEUX11-1.16_ZephyrQA-PLAN.md`
- ⬜ P1 收编与 crate 骨架（下一阶段）

详见构建计划：`docs/FCEUX11-1.16_ZephyrQA-PLAN.md`
