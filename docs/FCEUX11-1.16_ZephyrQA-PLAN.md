# FCEUX11 v1.16「ZephyrQA 独立测试系统」构建计划

> **报告性质**：工程化构建计划（P0 转译）。基于桌面《FCEUX11 v1.16 独立测试系统可行性与方向性评估》方向性报告，经代码级交叉验证后转译为可执行方案。
> **分支**：`wip_1.16`（全流程在此分支进行）
> **框架命名**：ZephyrQA
> **核心命题**：把 FCEUX11 现有但散落的测试资产收编为一个独立存在的、清单驱动的、双 oracle 的、机器可判定的测试框架——既是 FCEUX11 的质量防线，也是 AI 代理的"阅卷机"，且框架核心可迁移到其他多种项目。
> **本阶段（P0）范围**：仅产出本计划文档 + ZephyrQA 占位目录。零代码改动。

---

## 〇、TL;DR

**可行。** 方向性报告的三个核心判断经代码级验证全部成立，并补充两个工程化校正：

1. **测试系统独立化是"收编"而非"新建"**——FCEUX11 已有 30 项 CTest、golden master、bench 协议、i18n 门禁构成的"影子测试系统"，资产齐全，缺的是统一清单、统一 runner、统一报告、统一生命周期。
2. **双 oracle 分离是系统灵魂**——现有 oracle 全是"回归等价"（回答"与昨天是否一致"），无一个"硬件一致性"（回答"与真实硬件是否一致"）。ZephyrQA 的核心动作是补上 Oracle B。
3. **独立测试系统是精度对齐与 runppu 重批的前置条件**——依赖序为：测试系统 → 精度基线 → 精度攻关 → runppu 重批，而非三线并进。

**两个工程化校正**（不构成否决项，决定分阶段策略）：

- **headless"首付"有补丁**：报告称 v1.11 DriverCallbacks 解耦已付首付——属实，但 `src/fceu.cpp` 对 `drivers/Qt/nes_shm.h` 仍有残留正向依赖，headless 接口的"全款"待 P1 补齐。
- **`golden_hashes.json` 是"错误固化"实证样本**：几乎所有 ROM 的帧 hash 都是 `2072644093`（0x7B7B7B7D，疑空帧），仅 nestest 有值变化。收编阶段必须审计。

**需求升级回应**：原命题针对 FCEUX11。现要求框架独立后须适应其他多种项目、且 AI 友好。评估结论：原方案前四项设计已天然 AI 友好，**唯一缺口是跨项目可迁移性**——引入 `SutAdapter` 抽象层，把"框架核心（被测物无关）"与"FCEUX11 adapter（被测物特定）"分层即可解决，不推翻原方案。

---

## 一、元信息与约束

### 1.1 依赖序

```
P0 报告转译（本次）
  └─ P1 收编 + headless 全款
       └─ P2 Oracle B 接入（零代码修改）
            └─ P3 软件侧输入通道（Lua 解耦）
                 └─ P4 精度攻关（双 oracle 护栏下）
                      └─ P5 runppu 重批（条件解冻）
```

### 1.2 七条方向性约束（钉死，落地不得自由发挥）

源自方向性报告 §八，本计划在此边界内设计细节：

1. **先收编后新建**：现有 30 项 CTest 与全部脚本门禁必须先迁移进清单体系，禁止绕开它们另起炉灶。
2. **Oracle 类型是清单一等字段**：每个测例必须声明属于 A（回归等价）还是 B（硬件一致性）；无 oracle 类型的测例不得入库。
3. **接入精度判据的阶段不允许修改任何模拟器源代码**；已知失败清单是独立交付物。
4. **runppu 重批的开工条件**写成显式门禁：Oracle A 全绿 + Oracle B 清单稳定 + 收益预期重估通过，三者缺一不可。
5. **报告格式先于 runner 定稿**（JSON 迁移矩阵为最小内核），因为报告是人与 AI 共同的接口。
6. **基线更新走与代码变更同级的评审**，并在报告中标红每次基线漂移。
7. **AI 不得修改已入库的 `expected` 值**，只能新增条目；基线更新走与代码同级评审（反 gaming）。

### 1.3 三条明确非目标（写进系统文档首页，防止典范野心吃掉主线）

1. **不做跨平台二进制可移植**：MSVC-only 是 ABI 与字节级 savestate 兼容的正当约束。"可移植性"定义为数据与协议层可移植（清单、基线、报告格式、框架核心 Rust 代码），而非二进制可移植。
2. **不做多被测物框架**（一期）：ZephyrQA 一期只服务 FCEUX11。跨项目迁移能力通过 `SutAdapter` 抽象层**预留**，但不在 v1.16 实际接入第二个被测物。
3. **不强行纳入 UI 层**：headless 天然绕开 GUI。Qt UI 层走独立 offscreen smoke 通道（hotfix4 D-1 教训），不纳入核心 harness。

---

## 二、现状盘点（代码级实证）

### 2.1 影子测试系统全貌

| 资产类别 | 现有内容 | 组织形态 | 缺口 |
|---|---|---|---|
| 单元/集成测试 | 30 项 CTest 全绿（cpu/ppu/apu/bus/mapper_core/cart_class/savestate_core/core_state/fds_load/expected_api/config_store/pixbuf_pool/driver_callbacks/core_driver_boundary 等） | `tests/` 子目录，`add_subdirectory(tests)` 挂主构建 | 无统一清单元数据；与主构建耦合 |
| 回归等价 oracle | `mapper_byte_diff_test`（169 mapper）、`apu_wav_diff_test`（8 金色 WAV）、`ppu_frame_diff_test`、`golden_savestate_test`、`savestate_regression_test`、`rom_regression_test` | CTest 用例内嵌 | 基线无版本化/provenance；无"基线何时允许更新"治理规则 |
| 性能协议 | `bench_tolerance_test` + 5 项 bench（cpu/ppu/full/apu_frame/bus_dispatch） | bench 与 test 混编 | 基线陈旧（v1.10 时代）；门禁为 advisory |
| 静态门禁 | `menu_slot_check`、`i18n_regression_test`、core-driver 边界 grep 断言 | 部分注册 CTest、部分 PS/Python 脚本 | 入口分散；CI 与本地行为不完全一致 |
| i18n 机器门禁 | `i18n_audit.py`、`i18n_coverage.ps1`（已含 identical-to-EN 反伪装）、`en_keep_allowlist.txt`（341 条） | 独立脚本 | 未与主测试入口统一 |
| 构建/工具链 | MSVC 锁定、vcpkg 清单、ASan/UBSan 开关、PGO/LTCG | `CMakeLists.txt` + `do_build.ps1` | sanitizer 跑批未常态化；无"测试构建"配置 |

### 2.2 Rust workspace（关键支撑，决定框架本体语言）

项目已有完整 Rust workspace，Rust 已是一等公民：

- **位置**：`src/rust/`，Cargo workspace（resolver = "3"，edition 2024）
- **7 个 crate**：`fceux11-core` / `fceux11-debug` / `fceux11-formats` / `fceux11-lua` / `fceux11-media` / `fceux11-utils` + 顶层 `fceux11-rust` lib
- **CMake 集成**：`src/rust/CMakeLists.txt` 用 `add_custom_command` 跑 `cargo build --release --target x86_64-pc-windows-msvc`，产出 `fceux11_rust.lib` 静态库；`FCEUX11_RUST_ENABLED` 链接进 `fceux11_utils`
- **cbindgen**：`build.rs` + `cbindgen 0.29.3` 自动生成 C 头文件，`cpp_compat = false`
- **release profile**：`panic = "abort"` + `lto = true`
- **ffi_stubs 机制**：`fceux11-lua` crate 的 `ffi_stubs.rs`（`#[cfg(any(test, feature = "ffi-stubs"))]`）让 Rust 测试脱离 C++ 独立链接——这正是 ZephyrQA 框架核心可独立测试的范式
- **结论**：ZephyrQA 作为新 crate 加入此 workspace，复用 staticlib + cbindgen + CMake 模式，**零新工具链**。

### 2.3 Lua 引擎（关键支撑，决定软件侧对接语言）

- **已是 Rust 实现且为默认**：`FCEUX11_LUA_RUST_ENABLED` 默认 ON（`src/CMakeLists.txt:125`），`fceux11-lua` crate 用 `mlua` 实现
- **全绑定覆盖**：emu / memory / joypad / movie / savestate / ppu / sound / gui / zapper / debugger / bit / rom / input（`src/rust/crates/fceux11-lua/src/bindings/`）
- **C++ 侧退化为 FFI 转发层**：`src/lua-engine.cpp` 以 `extern "C"` 导出 `FCEU_LuaStop` / `FCEU_LoadLuaCode` / `fceux11_lua_*` 系列
- **样本脚本**：`tests/lua_scripts/` 已有 16 个 Lua 测试脚本（test_emu / test_memory / test_joypad / test_movie / test_savestate / test_ppu 等）
- **约束**：`lua-engine.cpp` 当前 include 了 `drivers/Qt/sdl.h` / `fceuWrapper.h` / TasEditor 系列，硬依赖 Qt——Lua 引擎当前无法 headless 独立运行。**P3 解耦**后 Lua 成为软件侧动态测例通道。

### 2.4 双 oracle 缺失实证

现有 oracle 全是回归等价（golden master / characterization testing）：

- `mapper_byte_diff_test`：169 mapper 字节级 memcmp
- `apu_wav_diff_test`：8 金色 WAV 0-sample 容差
- `ppu_frame_diff_test`：5 ROM 0-pixel 容差
- `golden_savestate_test`：savestate 字节级
- `savestate_regression_test` / `rom_regression_test`

**无一个 oracle 回答"与真实硬件是否一致"**。这就是 readme 里"继承 FCEUX 卓越模拟精度"目前尚属营销语言的根因——没有可证伪的判据。

### 2.5 `$6000` 协议落地路径（Oracle B 的工程基础）

blargg 硬件测试 ROM 的现代协议是 **$6000 内存映射结果协议**：测试 ROM 把 PASS/FAIL 与状态码写入 $6000–$6003，headless 运行 N 帧后读取内存即可判定，**完全不需要截图比对或人工读屏**。

FCEUX11 的内存读取机制（`src/bus.h`）：

```c
// readfunc 是函数指针数组；ARead[addr](addr) 即读取 addr 处一字节
extern readfunc  (& ARead )[0x10000];
extern writefunc (& BWrite)[0x10000];
```

Oracle B 的判定核心即 `ARead[0x6000](0x6000)` ——读取 blargg 写入的结果码。这条路径已存在，P2 接入时无需改模拟器源码。

### 2.6 两个工程化校正点

#### 2.6.1 headless"首付"有补丁

报告称 v1.11 DriverCallbacks 解耦已为 headless 付首付——属实：

- `src/driver_callbacks.h`：POD 结构体（43 函数指针字段）+ 进程级单例 `g_driver()`（`__forceinline`）+ `register_driver()` 注册
- `static_assert(std::is_trivially_copyable_v<DriverCallbacks>)` 保证 POD 语义
- 所有 `FCEUD_*` 回调 nullptr 时安全 no-op（`driver_callbacks.cpp`：`if (auto* fn = g_driver().X) fn(...);`）

**但**：`src/fceu.cpp`（核心）对 `drivers/Qt/nes_shm.h` 仍有残留正向依赖（`nes_shm` / `open_nes_shm`）。这是核心对 Qt 驱动层的残留耦合，headless 接口的"全款"待 P1 补齐——把 `nes_shm_t` 从 Qt 驱动层下沉到 core 或 `drivers/common`。

#### 2.6.2 `golden_hashes.json` 疑似错误固化样本

`tests/fixtures/golden_hashes.json` 中，几乎所有 ROM 的帧 hash 都是 `2072644093`（0x7B7B7B7D），仅 nestest 有值变化：

```json
"nrom":    { "frames": [2072644093, 2072644093, ...] },  // 全相同
"mmc1":    { "frames": [2072644093, 2072644093, ...] },  // 全相同
"mmc3":    { "frames": [2072644093, 2072644093, ...] },  // 全相同
"nestest": { "frames": [2072644093, ..., 1785016444, 632066529, ...] }  // 有变化
```

强烈暗示 nrom/mmc1/mmc3 等的 golden 帧是**空/未渲染状态被刻成基准**。这是方向性报告 §六"错误固化"风险的真实样本。P1 收编阶段必须审计：若确认是错误基线，则在基线治理规则下重建，或标注为"已知错误基线，允许被精度修复打破"。

### 2.7 core 可被独立 link

`fceux11_core` 静态库（`src/CMakeLists.txt:585`）链接 `fceux11_boards + fceux11_utils + zlib + SDL2`，**不链接 Qt**。ZephyrQA 的 FCEUX11 adapter 经 C ABI link core（与现有 CTest 链接路径一致，`fceux11_add_test_executable` 也是 link core + drivers_common）。SDL2 依赖源于音频子系统，headless 场景可 dummy 化。

---

## 三、技术选型论证

### 3.1 框架本体语言：Rust（顺势）

| 候选 | 评估 | 结论 |
|---|---|---|
| **Rust** | 项目已有成熟 workspace（7 crate + cbindgen + CMake + ffi_stubs）。ZephyrQA 作为新 crate 加入，复用全部既有基建。Rust 的 serde（JSON）、错误处理（Result）、trait 抽象（SutAdapter）、无 UB 保证，恰好匹配测试框架"序列化 + 判定 + 抽象"的核心需求。 | ✅ 选定 |
| C++ | 与现有 30 CTest 同语言，link core 直接。但 C++ 缺乏 Rust 的序列化/错误处理/并发安全网，且与项目正在推进的 Rust 化方向相背。 | ✗ |
| Python | 开发快、生态丰富。但需 ctypes/cffi 调 C ABI，性能与类型安全弱，引入新运行时依赖，与 MSVC-only 工具链锁定不一致。 | ✗ |

**顺势论据**：项目已在 `src/rust/` 建立完整 Rust 基建，`ffi_stubs` 机制已证明"Rust 测试可脱离 C++ 独立链接"。ZephyrQA 不是引入新范式，而是复用既有范式。

### 3.2 软件侧对接语言：Lua + 声明式 tests.json（双通道）

被测物暴露的是 C ABI，能跨 C ABI 的语言均是候选。

| 候选 | 评估 | 结论 |
|---|---|---|
| **Lua + JSON** | 双通道：常规测例用声明式 `tests.json` 清单（输入 + oracle 类型 + 预期 + 超时，零代码，AI 友好）；动态测例用 Lua 脚本（复用既有 mlua 引擎 + 全绑定 + 16 样本脚本）。Lua 轻量、沙箱化、与 NES 模拟器社区惯例一致。AI 生成 Lua 无需重编译、迭代快。 | ✅ 选定 |
| 纯 JSON 清单 | 最简、最解耦、AI 最易生成，但无法表达"跑 N 帧后读 $6000 再分支"这类动态控制，覆盖面受限。 | 作为子集纳入 |
| Rust 测例代码 | 类型最安全、与框架同语言，但每条测例需重编译、AI 生成迭代慢、违背"新增测试 = 新增清单条目而非改 runner 代码"原则。 | ✗ |
| Python 脚本 | 生态丰富、AI 生成快，但引入 Python 运行时、性能/类型安全弱、与既有 Lua 引擎能力重叠造成双轨。 | ✗ |

**双通道设计**：

- **tests.json 清单**（常规测例，约 80%）：声明式，AI 与人类共读。新增测试 = 新增 JSON 条目，不改 runner 代码。
- **Lua 脚本**（动态测例，约 20%）：需帧序列控制、条件分支、内存读写的复杂场景。复用 `fceux11-lua` crate 的 mlua 引擎。

### 3.3 被测物对接：C ABI link core + SutAdapter trait

ZephyrQA 框架核心通过 `SutAdapter` trait 与被测物解耦（详见 §五）。FCEUX11 adapter 经 C ABI 链接 `fceux11_core` 静态库，调用 `fceu11::Initialize` / `fceu11::LoadGame` / `fceu11::Emulate` / `ARead[addr](addr)`。

---

## 四、AI 友好性评估

### 4.1 五判据对照

"AI 友好"= 让 AI 能"读懂意图、生成测例、获得机器判定"，无需人类在场。拆成五个可工程化判据：

| 判据 | 含义 | 本计划落地 |
|---|---|---|
| 清单驱动 | AI 读 JSON 生成测例（数据非代码），可 schema 校验 | ✅ tests.json 清单 |
| oracle as data | 预期值是清单字段而非代码 assert，可版本化 | ✅ `expected` 字段 |
| 机器可判定输出 | 迁移矩阵 JSON，AI 读 FAIL_TO_PASS 即判补丁对错 | ✅ §六报告格式 |
| failure_means 设防 | 每条清单回答"失败意味什么"，防 AI 生成空转测试 | ✅ `failure_means` 字段 |
| 跨项目可迁移 | 框架核心与被测物解耦，FCEUX11 只是 adapter 之一 | ✅ §五 SutAdapter |

### 4.2 Lua 脚本通道的 AI 价值

AI 生成 Lua 脚本比生成 Rust 测试代码迭代快得多（无需重编译），且 Lua 语法简单到主流 LLM 几乎不犯错。这强化了"Lua + JSON 双通道"的选择：

- 简单测例 → JSON 清单（AI 生成，schema 校验，零编译）
- 复杂测例 → Lua 脚本（AI 生成，即时运行，无需编译）
- 框架本体 → Rust（人类 + AI 共建，类型安全，但变更频率低）

三种形态按"变更频率"分层：高频的测例用数据/脚本（AI 友好），低频的框架用强类型语言（正确性优先）。

### 4.3 反 gaming 治理

调研指出 AI agent 可能"改测试让自己通过"（reward hacking——editing tests or removing failing assertions）。清单体系的天然防御：

- 测例是数据（JSON/Lua），AI 改清单会产生 diff，可被 review 审计
- `expected` 值的修改受 §1.2 约束 7 管控："AI 不得修改已入库的 expected 值，只能新增条目"
- 基线更新走与代码同级评审，报告标红每次基线漂移

显式治理规则写入框架文档首页，防止 AI 在长期协作中渐进侵蚀基线。

### 4.4 AI 可读的清单自描述

清单 schema 含 `description`（人类 + AI 共读的测例意图）与 `failure_means`（失败语义），让 AI 仅凭清单即可理解全量验证意图。这是方向性报告原则 1（清单驱动）的强化——清单不只是给 runner 读的，也是给 AI 代理读的"验证意图说明书"。

---

## 五、跨项目可迁移性设计

### 5.1 核心思想：框架核心与被测物解耦

ZephyrQA crate 内部分两层：

```
┌─────────────────────────────────────────────────────────┐
│  ZephyrQA 框架核心层（被测物无关 · 可迁移）              │
│  manifest 解析 · runner 调度 · 双 oracle 判定           │
│  迁移矩阵报告 · 基线治理                                 │
│  仅依赖 SutAdapter trait，不依赖任何具体被测物           │
└────────────┬───────────────────────────────────────────┘
             │  SutAdapter trait
┌────────────▼───────────────────────────────────────────┐
│  SUT adapter 层（被测物特定）                            │
│  ┌──────────────────┐  ┌──────────────┐  ┌──────────┐  │
│  │ FCEUX11 adapter  │  │ 未来项目 A    │  │ 未来项目B│  │
│  │ C ABI link core  │  │ ...           │  │ ...      │  │
│  └──────────────────┘  └──────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 5.2 SutAdapter trait（方向性，P1 定稿）

```rust
/// 被测物适配器 trait。框架核心通过此 trait 与具体被测物交互。
/// FCEUX11 提供一个实现；未来迁移到新项目时只需写一个新 adapter。
pub trait SutAdapter {
    /// 加载输入（ROM 文件路径 / 测试输入等）
    fn load(&mut self, input: &InputSpec) -> Result<()>;
    /// 推进一步（一帧 / 一个事件循环 / 一次 tick）
    fn step(&mut self) -> Result<()>;
    /// 读取 oracle 探针（Oracle B 的 $6000 结果码；Oracle A 的快照点）
    fn read_oracle_probe(&self, addr: u32) -> Result<u8>;
    /// 快照当前状态（用于 golden master / savestate 比对）
    fn snapshot(&self) -> Result<Vec<u8>>;
    /// 重置到初始状态
    fn reset(&mut self) -> Result<()>;
}
```

**设计约束**：

- trait 方法是**被测物无关**的抽象——`step()` 对 FCEUX11 是"一帧"，对其他项目可能是"一次事件循环"或"一个 tick"
- FCEUX11 adapter 实现里才出现 `fceu11::Emulate` / `ARead[0x6000]` 等被测物特定调用
- 框架核心的 runner / oracle / report 模块只调 `SutAdapter`，不出现任何 FCEUX11 符号

### 5.3 典范性可证性

这一分层使方向性报告 §七的"典范性"目标可证：**可迁移的是框架核心 + 方法论，不是 FCEUX11 代码**。迁移到新项目时复用的是：

- tests.json schema 与解析器
- runner 调度逻辑
- 双 oracle 判定逻辑
- 迁移矩阵报告生成
- 基线治理规则

新项目只需：写一个 `SutAdapter` 实现 + 填清单 + 提供 golden 基线。

### 5.4 跨项目迁移的非目标（v1.16 内）

- 不在 v1.16 实际接入第二个被测物（§1.3 非目标 2）
- 跨项目能力通过 `SutAdapter` 抽象层**预留并验证**（FCEUX11 adapter 是该 trait 的参考实现），但不分散主线精力去适配其他项目

---

## 六、架构设计

### 6.1 ZephyrQA crate 结构

```
src/rust/crates/zephyr-qa/
├── Cargo.toml
└── src/
    ├── lib.rs                  ← 公共入口
    ├── core/                   ← 框架核心（被测物无关）
    │   ├── mod.rs
    │   ├── config.rs           ← 运行配置
    │   └── error.rs            ← 统一错误类型
    ├── manifest/               ← tests.json schema + 解析
    │   ├── mod.rs
    │   ├── schema.rs           ← serde 结构（TestCase / TestSuite）
    │   └── parser.rs
    ├── runner/                 ← 调度
    │   ├── mod.rs
    │   ├── scheduler.rs        ← 按 oracle_type / tag / layer 分组调度
    │   └── watchdog.rs         ← 超时看门狗
    ├── oracle/                 ← 双 oracle 判定
    │   ├── mod.rs
    │   ├── regression.rs       ← Oracle A：回归等价（memcmp / hash）
    │   └── hardware.rs         ← Oracle B：硬件一致性（$6000 协议）
    ├── report/                 ← 迁移矩阵 JSON
    │   ├── mod.rs
    │   ├── matrix.rs           ← FAIL_TO_PASS / PASS_TO_PASS / PASS_TO_FAIL
    │   └── baseline.rs         ← 基线漂移检测与标红
    └── adapter/                ← SutAdapter trait + FCEUX11 实现
        ├── mod.rs              ← SutAdapter trait 定义
        └── fceux11.rs          ← FCEUX11 adapter（C ABI link core）
```

### 6.2 双 Oracle 并列结构

```
        ┌──────────────── 测例清单（tests.json / Lua）────────────────┐
        │  oracle_type: "A"  ────→  回归等价判定                     │
        │  oracle_type: "B"  ────→  硬件一致性判定                   │
        └──────────────────────────┬──────────────────────────────────┘
                                     │
                    ┌────────────────┴────────────────┐
                    │                                 │
              ┌─────▼─────┐                     ┌─────▼─────┐
              │ Oracle A  │                     │ Oracle B  │
              │ 回归等价  │                     │ 硬件一致性 │
              │           │                     │           │
              │ 现有 golden│                     │ blargg    │
              │ master 全 │                     │ $6000 协议│
              │ 收编      │                     │ (新增)    │
              └─────┬─────┘                     └─────┬─────┘
                    │                                 │
                    └──────────────┬──────────────────┘
                                   │
                          ┌────────▼────────┐
                          │  迁移矩阵报告   │
                          │  (JSON)         │
                          │  FAIL_TO_PASS   │
                          │  PASS_TO_PASS   │
                          │  PASS_TO_FAIL   │
                          └─────────────────┘
```

**为什么必须分离**：Oracle A 要求"行为不变"，Oracle B 的攻关恰恰要求"行为改变"（向真实硬件靠拢）。若混在一个判定通道，每次精度修复都会被 golden diff 误判为回归。分离后，精度攻关的完成定义清晰：**目标 ROM 在 Oracle B 下 FAIL→PASS，且 Oracle A 全集保持 PASS→PASS**。

### 6.3 headless Null Driver

整个独立化唯一真正的架构动作。FCEUX11 已为它付过首付（v1.11 DriverCallbacks 解耦）。Null Driver = 一个"无 Qt、全空回调、脚本可驱动"的 test-driver 目标：

- 注册一个全 nullptr 的 `DriverCallbacks`（所有 `FCEUD_*` 安全 no-op）
- 不启动 QGuiApplication
- 由 ZephyrQA runner 经 C ABI 驱动 `fceu11::Initialize` / `LoadGame` / `Emulate`
- **P1 补齐全款**：下沉 `nes_shm_t` 解除 `fceu.cpp` 对 Qt 的残留依赖

### 6.4 `$6000` 协议（Oracle B 落地）

```
ZephyrQA runner                  FCEUX11 adapter
     │                                  │
     │  load(blargg.nes)                │
     ├─────────────────────────────────►│ fceu11::LoadGame
     │                                  │
     │  step() × N frames               │
     ├─────────────────────────────────►│ fceu11::Emulate × N
     │                                  │ (blargg ROM 运行，写 $6000-$6003)
     │  read_oracle_probe(0x6000)       │
     ├─────────────────────────────────►│ ARead[0x6000](0x6000)
     │  ← result code                   │
     │◄─────────────────────────────────┤
     │                                  │
     │  判定: 0x00=PASS, 其他=FAIL+码   │
```

无需截图比对、无需人工读屏。全量 blargg 套件（174 ROM / 30+ 套件）跑批 <3 分钟（AprNes 公开方法论佐证）。

### 6.5 软件侧双输入通道

```
┌─────────────────────┐     ┌──────────────────────────┐
│  tests.json 清单     │     │  Lua 脚本（动态测例）     │
│  (常规测例 ~80%)     │     │  (动态测例 ~20%)          │
│                     │     │                          │
│  声明式 · 零代码     │     │  复用 fceux11-lua/mlua    │
│  AI 生成 · schema 校 │     │  AI 生成 · 无需重编译     │
│  例:                 │     │  例:                     │
│  {                  │     │  emu.frameadvance()      │
│    "oracle":"B",    │     │  local v = memory.read(  │
│    "rom":"blargg/..",│    │     0x6000)              │
│    "frames":300,    │     │  assert(v == 0x00)       │
│    "expected":0x00  │     │                          │
│  }                  │     │                          │
└─────────┬───────────┘     └────────────┬─────────────┘
          │                              │
          └──────────┬───────────────────┘
                     │
          ┌──────────▼──────────┐
          │  ZephyrQA runner    │
          │  (统一调度)         │
          └─────────────────────┘
```

---

## 七、tests.json schema v1（方向性，P1 定稿）

```jsonc
{
  "schema_version": 1,
  "suite": "fceux11_v1.16",
  "provenance": {
    "generated_by": "zephyr-qa",
    "engine_version": "1.16.0",
    "toolchain": "msvc-19.x",
    "generated_at": "2026-07-26T00:00:00Z"
  },
  "tests": [
    {
      "id": "blargg_cpu_dummy_reads",
      "description": "blargg CPU dummy reads test — verifies CPU read timing",
      "oracle_type": "B",                    // A=回归等价 B=硬件一致性（一等字段，必填）
      "layer": "core",                       // core / ui / i18n / perf
      "input": {
        "rom": "ZephyrQA/roms/blargg/cpu_dummy_reads.nes",
        "frames": 300
      },
      "expected": {
        "probe_addr": 0x6000,
        "value": 0                            // 0x00=PASS
      },
      "timeout_seconds": 30,
      "tags": ["blargg", "cpu", "accuracy"],
      "failure_means": "CPU 在 dummy read 时序上与真实硬件不一致",
      "provenance": {
        "source": "blargg test ROM suite",
        "baseline_version": "v1.16.0-P2"
      }
    },
    {
      "id": "regression_ppu_frame_nrom",
      "description": "PPU frame regression for NROM — 0-pixel diff vs golden",
      "oracle_type": "A",
      "layer": "core",
      "input": {
        "rom": "tests/fixtures/mapper_nrom.nes",
        "frames": 60
      },
      "expected": {
        "golden": "ZephyrQA/baselines/frames/nrom.xbuf",
        "tolerance": 0
      },
      "timeout_seconds": 60,
      "tags": ["ppu", "frame", "regression"],
      "failure_means": "PPU 渲染输出发生非预期漂移",
      "provenance": {
        "source": "v1.5 Prism golden",
        "baseline_version": "v1.5.0"
      }
    }
  ]
}
```

**字段说明**：

| 字段 | 必填 | 说明 |
|---|---|---|
| `id` | ✅ | 测例唯一标识 |
| `description` | ✅ | 人类 + AI 共读的测例意图（AI 友好） |
| `oracle_type` | ✅ | "A" 或 "B"（一等字段，无值不得入库） |
| `layer` | ✅ | core / ui / i18n / perf |
| `input` | ✅ | 输入描述（rom / frames / 等） |
| `expected` | ✅ | 预期值（oracle as data） |
| `timeout_seconds` | ✅ | 超时看门狗 |
| `tags` | ✗ | 过滤标签 |
| `failure_means` | ✅ | 失败语义（防空转测试，AI 友好） |
| `provenance` | ✅ | 基线来源与版本（基线治理） |

---

## 八、JSON 报告格式（方向性，P1 定稿）

报告格式先于 runner 定稿（约束 5），因为报告是人与 AI 共同的接口。

```jsonc
{
  "report_version": 1,
  "run_id": "20260726-143000-abc123",
  "engine": { "version": "1.16.0", "toolchain": "msvc-19.x", "git_rev": "..." },
  "summary": {
    "total": 204,
    "passed": 198,
    "failed": 4,
    "skipped": 2
  },
  "transition_matrix": {                  // 迁移矩阵（核心字段，AI 读此判补丁对错）
    "fail_to_pass": [                     // 修复了的（攻关目标）
      { "id": "blargg_mmc3_irq", "prev": "FAIL", "curr": "PASS" }
    ],
    "pass_to_pass": [ ... ],              // 没破坏的（稳定基线）
    "pass_to_fail": [                     // 破坏了的（回归警报）
      { "id": "regression_ppu_frame_mmc3", "prev": "PASS", "curr": "FAIL" }
    ],
    "fail_to_fail": [ ... ]               // 仍失败的（已知失败，未攻关）
  },
  "baseline_drift": [                     // 基线漂移（标红，约束 6）
    {
      "id": "regression_mapper_byte_mmc5",
      "field": "expected.golden_hash",
      "old": "a1b2c3...",
      "new": "d4e5f6...",
      "approved": false                   // 未走评审的漂移 = 红色警报
    }
  ],
  "oracle_breakdown": {
    "A_regression":       { "pass": 30, "fail": 1 },
    "B_hardware":         { "pass": 168, "fail": 3 }
  },
  "details": [ ... ]                      // 每条测例的完整结果
}
```

**核心字段**是 `transition_matrix`——这是 SWE-bench 同构的 FAIL_TO_PASS / PASS_TO_PASS 判据，让 AI 读报告即可判断"补丁对不对"。

---

## 九、分阶段实施计划

### P0：报告转译（本次，已完成）
- ✅ 方向性报告审阅 + 代码级交叉验证
- ✅ 技术选型（Rust + Lua + JSON）
- ✅ AI 友好性 + 跨项目可迁移性评估
- ✅ 本计划文档 + ZephyrQA 占位目录
- **退出条件**：本文档入库 `wip_1.16`

### P1：收编 + headless 全款
- 迁移现有 30 CTest + 脚本门禁进 tests.json 清单体系
- 建 `zephyr-qa` crate 骨架（六模块）+ runner 骨架 + JSON 报告最小内核
- **审计 `golden_hashes.json`**：确认错误固化样本，重建或标注"已知错误基线"
- **下沉 `nes_shm_t`**：解除 `fceu.cpp` 对 Qt 的残留依赖，补齐 headless 全款
- 实现 `SutAdapter` trait + FCEUX11 adapter（C ABI link core）
- 实现 Null Driver（全 nullptr DriverCallbacks）
- **不接入 blargg、不改模拟器源码**
- **退出条件**：现有 30 CTest 全部能经 ZephyrQA runner 跑通并产出迁移矩阵 JSON；headless 可在无 Qt 环境运行

### P2：Oracle B 接入（零代码修改）
- 建 blargg ROM 套件清单 + 已知失败清单基线
- headless 跑批读 `$6000`，产出 FAIL/PASS 矩阵
- 产出"FCEUX11 精度对照表"（对标 TASVideos 表格格式）
- 对照 TASVideos 上游 FCEUX 预期失败分布核对
- **不修模拟器代码**（约束 3）
- **退出条件**：blargg 全套件跑批 <3 分钟；精度对照表入库；已知失败清单版本化

### P3：软件侧输入通道
- 解耦 `lua-engine.cpp` 对 Qt 的依赖（下沉 Qt 特定 includes）
- 确立 Lua 脚本通道：ZephyrQA runner 可加载并执行 Lua 测试脚本
- 确立 tests.json + Lua 双通道统一调度
- **退出条件**：Lua 测试脚本可经 ZephyrQA headless 运行并产出判定

### P4：精度攻关（双 oracle 护栏下）
- 在 Oracle A 全绿 + Oracle B 清单稳定前提下，修模拟器
- 目标 ROM 在 Oracle B 下 FAIL→PASS 即胜利
- 全绿非 v1.16 承诺（"清单收窄即胜利"）
- **退出条件**：至少一项 FAIL→PASS；Oracle A 全集保持 PASS→PASS

### P5：runppu 重批（条件解冻）
- **开工门禁**（约束 4）：Oracle A 全绿 + Oracle B 清单稳定 + 收益预期重估通过
- runppu 重批是 timing rewrite，双 oracle 齐备才能区分"时序改对了"与"改得和以前不一样但都错"
- 测试系统成人礼：若能在 runppu 这类改动上给出可信 FAIL/PASS 信号，系统通过"典范"成人礼
- **退出条件**：runppu 重批完成且双 oracle 给出可信信号；或收益预期重估未通过则继续冻结

---

## 十、风险登记册（工程级）

| 风险 | 机制 | 概率 | 影响 | 工程对策 | 阶段 |
|---|---|---|---|---|---|
| 基线资产腐化 | golden/bench 基线无人维护，失败变常态噪声 | 中 | 高 | 基线版本化 + 更新同评审级（约束 6）；bench 用统计协议（中位数 + 阈值） | P1 |
| 性能测试抖动污染信任 | bench 本质噪声测量，与确定性测试共用判定语义 | 高 | 中 | 单独通道、统计判定、重复验证；现有 R4 协议（warmup3+测7丢极值）已可收编 | P1 |
| 错误固化 | golden master 把当前（可能错误）行为刻成基准 | 中 | 高 | 双 oracle 分离；Oracle B 判 FAIL 的行为在 Oracle A 中标记"已知错误基线"；P1 审计 golden_hashes | P1 |
| 平行王国化 | 独立系统脱离 CI 与发布流程，无人运行 | 中 | 高 | 从第一天起接入 CI；报告发布到 GitHub Pages（项目已有 Pages 管线） | P1+ |
| 范围膨胀 | "典范"野心导致过度工程（多平台、多被测物、插件） | 中 | 中 | 三条非目标（§1.3）写进首页；跨项目能力仅预留不实接 | 全程 |
| Qt UI 层测不到 | headless 天然绕开 GUI | 高 | 中 | 承认边界：UI 层走 Qt offscreen smoke 独立通道，不强行纳入核心 harness | P1 |
| AI gaming | AI agent 改测试让自己通过 | 中 | 中 | 测例是数据可 diff 审计；约束 7 禁改 expected；基线更新同级评审 | 全程 |

---

## 十一、AI 协作维度

### 11.1 阅卷机定位

ZephyrQA 在功能上与 SWE-bench harness 同构：每个 PLAN 里的 PR 就是一个"任务实例"，测试系统就是它的阅卷机。建成后的工作流：

```
当前：  AI 声明 → 人工抽查 → 发布评估文档
建成后：AI 声明 → harness 迁移矩阵 → 人工只看 diff 与清单变化
```

hotfix5/6 的三轮翻译审计（209→122→47）证明多层验证价值，但那是人工发起、按次付费；机器阅卷把同样安全网变成每次提交自动生效的常数成本。

### 11.2 两个必须设防的失败模式（清单字段级落地）

**空转测试**：AI 生成的测试通过但不断言有意义的东西。对策——清单条目入库时必须回答"这条测例失败时意味着什么"（`failure_means` 字段），无答案者不得入库。这是 SWE-bench Verified 人工过滤"可解性"做法的清单化翻版。

**错误固化**：golden master 把当前行为刻成基准，而当前行为可能错。对策——Oracle A 与 Oracle B 分离，任何被 Oracle B 判 FAIL 的行为，其在 Oracle A 的 golden 基线必须标记"已知错误基线，允许被精度修复打破"。这两个设防都不增代码量，只增清单字段与治理规则。

### 11.3 建设期的 AI 甜蜜点

建设 ZephyrQA 恰好是 AI Coding 的甜蜜点任务：大量机械性工作（清单条目编写、报告解析器、历史测试元数据补录）、判据客观（runner 能跑、报告字段齐全）、反馈即时（每条清单改动立即可验证）。多代理编排也已被 v1.15 验证（hotfix3 三代理 REVIEW），可复用。

---

## 十二、验收标准

| 阶段 | 退出条件（可判定、可复现） |
|---|---|
| P0 | 本文档 + ZephyrQA 占位目录入库 `wip_1.16` |
| P1 | 现有 30 CTest 全部经 ZephyrQA runner 跑通；迁移矩阵 JSON 产出；headless 无 Qt 运行；golden_hashes 审计完成；nes_shm 下沉完成 |
| P2 | blargg 全套件跑批 <3 分钟；精度对照表入库；已知失败清单版本化；零模拟器代码修改 |
| P3 | Lua 测试脚本可经 ZephyrQA headless 运行并产出判定；双通道统一调度 |
| P4 | 至少一项 Oracle B FAIL→PASS；Oracle A 全集 PASS→PASS |
| P5 | runppu 重批完成且双 oracle 给出可信信号；或收益重估未过则继续冻结（附理由） |

---

## 十三、与方向性报告的映射

| 报告原则/约束 | 本计划落地 |
|---|---|
| 原则 1 清单驱动 | §七 tests.json schema |
| 原则 2 双 oracle 分离 | §6.2 + oracle_type 一等字段 |
| 原则 3 headless 被测接口 | §6.3 Null Driver + P1 nes_shm 下沉 |
| 原则 4 机器可判定输出 | §八 迁移矩阵 JSON |
| 原则 5 基线版本化与治理 | §1.2 约束 6 + provenance 字段 + baseline_drift 标红 |
| 约束 1 先收编后新建 | P1 |
| 约束 2 oracle 类型一等字段 | §七 schema |
| 约束 3 接入不改模拟器 | P2 |
| 约束 4 runppu 显式门禁 | P5 |
| 约束 5 报告先于 runner | §八 先定稿 |
| 约束 6 基线更新同级评审 | §1.2 约束 6 |
| 约束 7 明确非目标 | §1.3 |

---

## 十四、一句话收束

**v1.16 这个命题的可行性不是"能不能建一个测试系统"，而是"敢不敢承认现有散件已经是系统的零件"。** ZephyrQA 的工作是收编它们、补上硬件 oracle、把判定权交给机器——精度对齐和 runppu 重批会从"高风险愿望"变成"护栏内的普通工作"，而这套系统本身，会成为 FCEUX11 在 AI Coding 时代留下的最有复用价值的工程资产：它的框架核心可迁移，它的方法论可陈述，它的人机接口无需信任。
