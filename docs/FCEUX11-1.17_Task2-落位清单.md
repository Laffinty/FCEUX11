# FCEUX11 v1.17 Task 2 执行清单 — `tests/kagami/` 落位

> **版本**：v1.17（执行草案，待 CI 绿后执行）
> **日期**：2026-08-08
> **分支**：`wip_v1.17`
> **状态**：📋 清单（未执行）
> **关联**：`docs/FCEUX11-1.17_计划.md` §三（任务 2：不可迁移 C++ 测试耦合 KagamiQA + 源码区隔）
> **前置**：CI 首跑确认绿（A0 验收：ctest 34/34、矩阵 47/39/8、grade=C、R4 Gate 通过）

---

## 〇、原则（固定不变式）

| 不变式 | 说明 |
|---|---|
| **CTest 测试名不变** | `cpu_test` 等 34 个名字不变，CI 历史对比有效 |
| **`tests.json` 不变** | 47 条目引用的 binary 名（`fceux11_*_test`）不随源码路径变 |
| **`WORKING_DIRECTORY` 不变** | 全部保持 `tests/`，fixture 相对路径（`fixtures/...`）继续解析 |
| **数据不动** | `tests/fixtures/`（ROM/golden/hash）、`tests/lua_scripts/`、`tests/benchmarks/*.json` 留在原地 |
| **单一 CMakeLists（方案 B）** | 不拆独立子构建，只在 `tests/CMakeLists.txt` 内改源路径——避免 `CMAKE_CURRENT_SOURCE_DIR` 变化破坏 fixture 路径 |

**类别标注**：
- 🅐 = Task 1 可迁移（暂驻 `tests/kagami/`，Task 1 迁 Rust 后删除）
- 🅑 = 永久驻留（核心内部单元测试 / 门禁）
- 🅒 = 随 v2.0 退役（C++ 语言/平台绑定测试，不翻译）

---

## 一、文件移动映射（44 个 C++ 文件）

### A. `tests/kagami/` 根（19 个）

| 文件 | 类别 | 备注 |
|---|---|---|
| `blargg_runner.cpp` | 🅐 | Task 1 迁移后删除（Rust `direct_entry` 合并） |
| `lua_runner.cpp` | 🅐 | Task 1 迁移后删除 |
| `kagami_direct_main.cpp` | 🅐 | Task 1 迁移后删除（27 行壳并入 Rust CLI） |
| `rom_regression_test.cpp` | 🅐 | Task 1 迁移后删除 |
| `savestate_regression_test.cpp` | 🅐 | Task 1 迁移后删除 |
| `smoke_test.cpp` | 🅑 | 引擎启动门禁（grade E 判定源） |
| `headless_smoke_test.cpp` | 🅑 | 引擎启动门禁（grade E 判定源） |
| `core_state_test.cpp` | 🅑 | |
| `enum_class_bitflags_test.cpp` | 🅒 | 测 C++ 宏构造 |
| `expected_api_test.cpp` | 🅒 | 测 `tl::expected` |
| `config_store_test.cpp` | 🅒 | Qt TypedConfig |
| `pixbuf_pool_test.cpp` | 🅒 | 随 v2.0 退役 |
| `i18n_regression_test.cpp` | 🅒 | 静态分析门禁 |
| `ppu_phase_c_test.cpp` | 🅑 | hotfix2 Phase C |
| `ppu_phase_d_test.cpp` | 🅑 | hotfix2 Phase D |
| `ppu_rendering_lut_test.cpp` | 🅑 | hotfix2 P0-1 |
| `ppu_phase_b_test.cpp` | ⚠️ **孤儿决策点** | git 跟踪但 CTest 未注册；头注释已过期（声称 CMakeLists 注册但实际无）。**建议归档删除**（invariant 已由 phase_c/d 覆盖）；或补注册 |
| `golden_savestate_test.cpp`（来自 `fixtures/golden/`） | 🅐 | **决策点**：源码移入 kagami/；`.fc0` 数据与 `golden_index.json` 留 `fixtures/golden/`（数据与代码分离） |

### B. `tests/kagami/boards/`（2 个）

| 文件 | 类别 |
|---|---|
| `mapper_load_test.cpp` | 🅑 |
| `mapper_reset_test.cpp` | 🅑 |

### C. `tests/kagami/core/`（15 个）

| 文件 | 类别 | 备注 |
|---|---|---|
| `cpu_test.cpp` | 🅑 | |
| `ppu_test.cpp` | 🅑 | |
| `apu_test.cpp` | 🅑 | |
| `bus_test.cpp` | 🅑 | |
| `mapper_test.cpp` | 🅑 | CTest 名 `mapper_core_test` |
| `savestate_test.cpp` | 🅑 | CTest 名 `savestate_core_test` |
| `cart_class_test.cpp` | 🅑 | |
| `fds_load_test.cpp` | 🅑 | |
| `driver_callbacks_test.cpp` | 🅑 | |
| `core_driver_boundary_test.cpp` | 🅑 | |
| `apu_wav_diff_test.cpp` | 🅐 | Task 1 迁移后删除 |
| `mapper_byte_diff_test.cpp` | 🅐 | Task 1 迁移后删除 |
| `ppu_frame_diff_test.cpp` | 🅐 | Task 1 迁移后删除 |
| `_phase_d_tests.h` | 🅑 | 头文件 |
| `test_helpers.h` | 🅐 | 共享设施；Task 1 迁 → `test_helpers.rs` 后删除 |

### D. `tests/kagami/benchmark/`（6 个）

| 文件 | 类别 |
|---|---|
| `apu_frame_bench.cpp` / `apu_mix_bench.cpp` / `bus_dispatch_bench.cpp` / `ppu_render_bench.cpp` / `x6502_exec_bench.cpp` | 🅑 并行维护 |
| `ppu_simd_probe.cpp` | 🅑 构建探针（非 CTest 注册） |

### E. `tests/kagami/benchmarks/`（1 个）

| 文件 | 类别 | 备注 |
|---|---|---|
| `bench_tolerance_test.cpp` | 🅑 | **决策点**：baseline JSON（`benchmarks/baseline_v1.0.json` 等）留 `tests/benchmarks/`，`WORKING_DIRECTORY` 不变则路径稳定 |

### F. `tests/kagami/utils/`（1 个）

| 文件 | 类别 |
|---|---|
| `xstring_microbench.cpp` | 🅑 |

### 留在 `tests/` 根不动

| 文件 | 说明 |
|---|---|
| `CMakeLists.txt` | 只改路径（§二） |
| `tests.json` | 单一事实源，零改动 |
| `git_info_stub.cpp` | 构建支持（所有测试链接），建议保留——决策点 |
| `fixtures/` | 全部数据（ROM/golden/hash/manifest） |
| `lua_scripts/` | Lua 测试脚本数据 |
| `benchmarks/*.json` | 基准数据（随 §一-E 决策点） |

---

## 二、`tests/CMakeLists.txt` 改动点

| # | 改动 | 位置 |
|---|---|---|
| 1 | ~40 处 `add_executable` / `fceux11_add_test_executable` 源路径加 `kagami/` 前缀（A 类 → `kagami/x.cpp`；C 类 → `kagami/core/x.cpp`；bench → `kagami/benchmark/...`） | 全文件 |
| 2 | 5 处 include 目录 `tests/core` → `tests/kagami/core` | `golden_savestate_test`、`bench_tolerance_test`、`blargg_runner`、`lua_runner`、`kagami_qa_direct_runner` |
| 3 | `benchmark/` 路径 → `kagami/benchmark/`（bench 目标 + simd probe） | benchmark 段 |
| 4 | PATH 注入名单（测试名列表） | **不变** |
| 5 | `WORKING_DIRECTORY` | **全部不变** |

> `ppu_phase_b_test` 若归档删除，CMake 无对应改动（本就未注册）。

---

## 三、登记核对表（34 CTest ↔ 47 manifest ↔ 源文件）

| 核对项 | 现状 | 处置 |
|---|---|---|
| 34 个 CTest 注册 | 源路径更新后全部映射到 `tests/kagami/` 下 | 移动后 `ctest -N` 测试名集合 diff = 空 |
| `ppu_phase_b_test` | CTest 未注册的孤儿源文件（git 跟踪） | 归档删除（建议）或补注册 |
| `headless_smoke_test` | CTest 有、manifest 无 | 记录为双簿记差异；Phase 1 消双簿记时补 manifest 条目 |
| `lua_bit_test_headless`（CTest 名） | vs manifest `lua_bit_test`（同 binary） | 记录为双簿记差异；Phase 1 统一 |
| `menu_slot_check` | Python 脚本（`scripts/check_menu_slots.py`） | 不移动，登记不变 |
| `fixtures/golden/golden_savestate_test.cpp` | 源码位于数据目录 | 决策点见 §一-A |
| 47 manifest 条目 | binary 名全部不变 | `tests.json` 零改动 |

---

## 四、执行步骤（CI 绿后）

```
1. git mv 移动 44 个源文件（保留历史，而非复制+删除）
2. 改 tests/CMakeLists.txt（§二 的 5 类改动）
3. cmake 重新配置 + 增量构建测试目标
4. ctest -LE perf 全量 → 34/34（测试名 diff 为空）
5. 核对 §三 登记表 + 处理决策点（orphan / golden 源码 / bench JSON / git_info_stub）
6. 提交（manifest 无游离、fixtures 零改动）
```

---

## 五、验收门禁

| 门禁 | 阈值 |
|---|---|
| 目录落位 | 全部不可迁移 C++ 测试源文件位于 `tests/kagami/`（或文档显式豁免） |
| 测试名稳定 | `ctest -N` 测试名集合与移动前完全一致（diff 为空） |
| ctest 全量 | 34/34 PASS（`-LE perf` 33/33） |
| manifest 对齐 | 每个测试有且仅有一个 manifest 条目，无游离 |
| 数据不变 | `tests/fixtures/`、`tests/lua_scripts/`、`tests.json` 零改动 |

---

## 六、待拍板决策点

1. **`ppu_phase_b_test.cpp`（孤儿）**：归档删除（建议——invariant 已由 phase_c/d 覆盖）还是补注册？
2. **`golden_savestate_test.cpp` 源码位置**：移入 `tests/kagami/`（数据与代码分离）还是留在 `fixtures/golden/`？
3. **`git_info_stub.cpp`**：保留在 `tests/` 根（构建支持，建议）还是移入 `tests/kagami/`？
4. **bench baseline JSON**：留 `tests/benchmarks/`（路径稳定，建议）还是随代码移动？

---

*清单完 — 待 CI 绿 + 决策点拍板后执行*
