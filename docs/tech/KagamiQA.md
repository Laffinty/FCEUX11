# KagamiQA — FCEUX11 双 Oracle 质量防线

> **版本**：v1.16  
> **性质**：双 Oracle（Oracle A 回归 + Oracle B 硬件一致性）自动化测试系统  
> **覆盖率**：39 CTest 回归 + 180 blargg $6000 协议 ROM（CPU/PPU/APU/MMC3）  
> **CI 状态**：每次 push 到 `main` / `wip_1.16` 自动触发，产出迁移矩阵 artifact  

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
│  │ • 39 CTest   │       │ • 180 blargg ROM     │ │
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

### 1.4 权威性公式

```
权威性 = ROM覆盖率 × Oracle独立性 × CI常驻因子

v1.16:  1.00  × 1.00  × 1.00  = 1.00  (防线)
v1.15:  0.13  × 1.00  × 0.00  = 0.00  (原型)
```

**权威性不要求 Oracle B 全部 PASS**。精确知道什么失败，比「全绿但不测」更权威。已知失败清单本身就是防线的一部分。

---

## 二、使用帮助 / Usage

### 2.1 快速开始

```powershell
# 1. 下载 blargg 测试 ROM（一次性，约 180 个 ROM，~8 MB）
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
Total:  180
Passed: 120
Failed: 60
```

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
| `kagamiqa_migration_matrix.json` | SWE-bench 同构迁移矩阵：`fail_to_pass` / `pass_to_pass` / `pass_to_fail` / `fail_to_fail` |
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
| blargg 测试 ROM | `tests/fixtures/blargg/` | 180 个公共领域 ROM，$6000 协议 |
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
| blargg ROM 套件 | `tests/fixtures/blargg/` | 无 | 公共领域，直接复制 |
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
│   ├── tests.json                          ← 39 CTest + 9 blargg + 5 Lua 测试清单
│   ├── fixtures/
│   │   ├── blargg/                         ← 180 blargg ROM (cpu/ppu/apu/mmc3/)
│   │   ├── blargg_manifest.json            ← ROM 清单（name/path/frames/probe_addr）
│   │   ├── blargg_known_fail.json          ← 已知失败分类（60 条，含 runppu 标记）
│   │   ├── blargg_full_baseline.json       ← P5 全量基线（120 PASS / 60 FAIL）
│   │   └── nestest.nes                     ← smoke test ROM
│   ├── blargg_runner.cpp                   ← Oracle B 执行器（C++, headless）
│   ├── lua_runner.cpp                      ← Lua 脚本执行器（C++, headless, P5 断言捕获）
│   └── kagami_direct_main.cpp              ← Direct runner C++ 入口（P5）
├── src/
│   ├── kagami_bridge.h                     ← C ABI 桥接头文件
│   ├── kagami_bridge.cpp                   ← C ABI 桥接实现（编译进 fceux11_core）
│   └── rust/crates/kagami-qa/
│       ├── src/
│       │   ├── main.rs                     ← CLI runner 二进制
│       │   ├── lib.rs                      ← Rust 库（含 C-callable direct 入口）
│       │   ├── adapter/
│       │   │   ├── trait_def.rs            ← SutAdapter trait + InputSpec/TESTResult
│       │   │   ├── subprocess.rs           ← SubprocessAdapter（fork 子进程）
│       │   │   └── direct.rs              ← Fceux11DirectAdapter（C ABI FFI）
│       │   ├── manifest/                   ← 测试清单加载（tests.json 解析）
│       │   ├── oracle/
│       │   │   ├── regression.rs           ← Oracle A 判定（exit code + stdout）
│       │   │   └── hardware.rs             ← Oracle B 判定（$6000 协议解析）
│       │   ├── runner/scheduler.rs         ← 测试调度器
│       │   └── report/
│       │       ├── matrix.rs               ← 迁移矩阵生成
│       │       └── baseline.rs            ← 基线加载/保存/漂移检测
│       └── Cargo.toml
├── scripts/
│   ├── download_blargg_roms.ps1            ← ROM 下载（177 条目，幂等）
│   ├── generate_blargg_manifest.ps1        ← 清单生成器
│   └── analyze_blargg_results.ps1          ← 结果分析/分类
├── .github/workflows/
│   ├── ci.yml                              ← 主 CI（build + ctest + benchmark）
│   └── kagami-qa.yml                       ← KagamiQA CI（Oracle A+B + 矩阵 + 漂移检测）
└── docs/tech/
    └── KagamiQA.md                         ← 本文档
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

**A:** CI 日常用 **Subprocess**（已足够快，180 ROM 约 2 分钟）。精度调试（如 runppu 重批）用 **Direct**——可以在帧间插入 oracle probe、调整 PPU 参数、捕获中间状态，这是 subprocess 做不到的。

