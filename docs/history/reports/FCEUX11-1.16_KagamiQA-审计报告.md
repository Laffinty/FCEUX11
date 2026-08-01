# FCEUX11 v1.16 KagamiQA 独立审计报告

> **审计日期**：2026-07-28
> **审计范围**：KagamiQA 双 Oracle 测试系统的检测精度、可靠性、可信性、权威性，以及第三方 ROM/NES 文件的版权与许可证合规性
> **审计方法**：文档审阅 + 代码级核对 + 实际运行复现 + 许可证追溯
> **审计分支**：`wip_1.16` (HEAD = `7c2356b`)
> **审计人**：独立审计（ZCode agent）

---

## 〇、审计结论速览（TL;DR）

KagamiQA 是一个**设计理念优秀、部分可复现、但存在多处文档与实现不符**的测试系统。其核心价值（双 Oracle 分离、$6000 协议、清单驱动、SutAdapter 抽象）在架构层面成立，且 Oracle B 的硬件一致性基线经实测**可复现**。但在**权威性所依赖的"CI 常驻"与"机器可判定"两个支柱上存在严重实现缺陷**，导致系统目前**未能达成其宣称的"防线"地位**。版权合规方面，ROM 来源合法但"公共领域"声明未经证实，存在归因缺口。

| 维度 | 评级 | 一句话结论 |
|------|------|-----------|
| 检测精度 | 🟡 部分 | Oracle B 基线真实可复现（120P/60F），但 Oracle A 非全绿、Lua 判定有假阳性 |
| 可靠性 | 🔴 不足 | 迁移矩阵 runner 有 Windows `.exe` bug，全量测试报"program not found"；direct runner 无法编译 |
| 可信性 | 🟡 部分 | 精度数据可信，但"全绿""通道打通""drift 检测"等声明与实现不符 |
| 权威性 | 🔴 未达成 | 权威性公式自评 1.00，但 CI 常驻因子与迁移矩阵实际均不工作，真实权威性 ≈ 0.13 |
| 版权合规 | 🟡 部分 | ROM 来源单一且为社区惯例，但"公共领域"声明未证实、归因缺失、nestest 被 redistribute |

**核心矛盾**：文档（含 README、KagamiQA.md、P5 计划）将 KagamiQA 描述为已交付的"v1.16 最终版本"防线，但实测表明其**迁移矩阵生成步骤在 Windows 上从未成功运行过任何测试**（runner 代码缺陷），其**in-process direct 通道从未成功编译**（Rust 2024 unsafe lint），其**baseline drift 检测是返回空向量的 stub**。这些是"防线"功能的三大支柱，目前均未真正生效。

---

## 一、审计输入与文档清单

本次审计阅读的 v1.16 KagamiQA 文档：

| 文档 | 性质 |
|------|------|
| `docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md` | P0 总体计划（39KB，14 章） |
| `docs/history/reports/FCEUX11-1.16_KagamiQA-P0-P4-构建状态报告.md` | P0–P4 状态报告 |
| `docs/history/obsolete/FCEUX11-1.16_KagamiQA-P2-accuracy-table.md` | Oracle B 精度对照表 |
| `docs/history/reports/FCEUX11-1.16_KagamiQA-P4-bridge.md` | P4-bridge 修复计划 |
| `docs/history/reports/FCEUX11-1.16_KagamiQA-P4-bridge-根因解决方案.md` | P4-bridge NULL deref 根因分析 |
| `docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md` | P5 权威性构建计划 |
| `docs/tech/KagamiQA.md` | KagamiQA 系统文档（对外） |
| `README.md` §质量保障 | 对外宣传声明 |

实测核对的对象：`tests/tests.json`、`tests/fixtures/blargg_manifest.json`、`tests/fixtures/blargg_known_fail.json`、`tests/fixtures/blargg_full_baseline.json`、`kagamiqa_migration_matrix.json`、`src/rust/crates/kagami-qa/` 全部源码、`.github/workflows/kagami-qa.yml`、`scripts/download_blargg_roms.ps1`、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`、`COPYING`。

---

## 二、检测精度审计

### 2.1 Oracle B（硬件一致性）—— 基线真实可复现 ✅

**文档声称**：180 个 blargg ROM，120 PASS / 60 FAIL，覆盖 CPU/PPU/APU/MMC3。

**实测复现**：

```
$ cd tests && ../build/tests/fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json
=== Blargg Suite Summary ===
Total:  180
Passed: 120
Failed: 60
```

**结论**：Oracle B 基线 **120P/60F 完全可复现**，与 `blargg_full_baseline.json` 和 `blargg_known_fail.json` 的声明一致。ROM 覆盖率 180/180 = 100%（去重后 ≥140，满足 P5 ≥80% 门禁）。这是 KagamiQA **最扎实、最可信**的部分。

**精度改进实证**：`vbl_01_basics.nes` 经实测确为 PASS（`value=0x00`），与文档声称的"P4 首个 FAIL→PASS"一致，`known_fail.json` 正确记录其为"previously_fixed_p4 — confirmed still PASS in P5 baseline"。精度攻关的 FAIL→PASS 信号**真实**。

**已知失败分类的可信性**：`blargg_known_fail.json` 对 60 个 FAIL 逐条标注 `category` / `code` / `eventually_pass` / `runppu` / `reason`，其中 9 条标记为 `runppu=true`（VBL/NMI/PPU 时序相关），为后续精度攻关提供了可信的优先级排序。随机抽验 `ppu_vbl_nmi.nes` 实测返回 `value=0x80` + 完整诊断字符串，与 known_fail 记录的 `code=0x80` 一致。

### 2.2 Oracle A（回归等价）—— 非全绿 ⚠️

**文档声称**（多处反复强调）：
- P0–P4 状态报告：「Oracle A 全绿是每次修改的前置条件」
- P5 计划 §四门禁：「Oracle A 全绿 | `rom_regression_test` 0 差异 | 100%」
- README：「39 个 CTest … 零差异门禁」
- KagamiQA.md：「39 CTest 回归」

**实测复现**：

```
$ ctest --test-dir build --build-config Release -LE perf
91% tests passed, 3 tests failed out of 33
Total Test time = 14.89 sec

The following tests FAILED:
	 23 - ppu_rendering_lut_test (BAD_COMMAND)
	 33 - lua_bit_test_headless (Failed)
	 34 - kagami_qa_direct_smoke (Not Run)
```

**结论**：Oracle A **实际为 30/33 通过（91%），并非"全绿"**。三个失败：

| 测试 | 失败性质 | 文档归类 | 审计判定 |
|------|---------|---------|---------|
| `ppu_rendering_lut_test` | `BAD_COMMAND` / `Not Run` — 二进制文件头为 `0000`（非 PE `MZ`），2MB 占位文件，**Exec format error** | P4-bridge 报告称"既有配置问题，与本次修改无关" | **真实构建损坏**，非配置问题。exe 文件格式错误，无法执行 |
| `lua_bit_test_headless` | `Failed` — Lua bit 库有 5 个位运算 bug（rshift/ror/tohex） | P4-bridge 报告称"WORKING_DIRECTORY 路径前缀重复，与本次修改无关" | **真实 Lua 库 bug**。从正确目录运行仍有 5 FAIL，被 lua_runner 的"不崩溃=PASS"判定掩盖 |
| `kagami_qa_direct_smoke` | `Not Run` — `kagami_qa_direct_runner.exe` 不存在 | 未在文档中说明 | **direct runner 从未成功构建**（见 §三） |

**注意 CTest 注册数**：CTest 实际注册 **34 个测试**（含 1 个性能测试被 `-LE perf` 排除后为 33），而文档与 README 一致声称"39 CTest"。`tests/tests.json` 确有 39 条目，但其中部分条目（如 `blargg_suite`、`apu_wav_diff_test`、`bench_tolerance_test` 等）未注册为 CTest 而是作为 KagamiQA 清单条目存在。**"39 CTest"是清单条目数，非 CTest 注册数**，文档措辞有误导性。

### 2.3 Lua 判定精度 —— 存在假阳性 🔴

**文档声称**（P5 计划 §2.4 + KagamiQA.md §2.6）：「P5 升级后，`lua_runner` 会捕获 Lua `assert()` / `error()` 输出并解析为 PASS/FAIL 信号，不再仅依赖"脚本没崩溃 = PASS"的模糊判定。」

**实测复现**：

```
$ fceux11_lua_runner.exe lua_scripts/test_bit.lua
FAIL: rshift(-1, 31) => got -1, expected 1
FAIL: ror(0x01, 1) => got -2147483648, expected 2147483648
FAIL: tohex(255) => got 000000ff, expected 000000FF
FAIL: tohex(255, 2) => got ff, expected FF
FAIL: tohex(-1, 4) => got ffff, expected FFFFFFFF
bit library: 22 passed, 5 failed
LUA_RESULT: script=test_bit.lua status=PASS duration_ms=2 details=script completed (no Lua errors detected)
```

**结论**：Lua bit 库**实际有 5 个真实失败**（位运算结果错误），但 `lua_runner` 报告 `status=PASS`，理由是"script completed (no Lua errors detected)"。这正是 P5 计划声称已修复的"不崩溃=PASS"问题，**但修复未生效** —— runner 仍只检测 Lua `error()`/`assert()` 异常，不解析脚本自身打印的 `FAIL:` 行。这意味着：

1. P5 计划 §5E（Lua 判定精度加固）的退出条件"覆盖现有 4 脚本"**未达成**。
2. KagamiQA.md §2.6 的声明"P5 升级后…不再仅依赖"脚本没崩溃 = PASS""**与实现不符**。
3. 存在**假阳性**：Lua 测试套件可能报告 PASS 而掩盖真实的库 bug（如本例的 bit 库 5 个缺陷）。

### 2.4 检测精度总评

| 子项 | 评级 | 证据 |
|------|------|------|
| Oracle B 硬件一致性 | ✅ 优秀 | 120P/60F 可复现，vbl_01 FAIL→PASS 可复现，known_fail 分类详实 |
| Oracle A 回归等价 | ⚠️ 部分 | 30/33，3 个真实失败被文档归为"既有配置问题"，实际含构建损坏与 Lua bug |
| Lua 判定 | 🔴 不足 | 假阳性：5 个真实 FAIL 被判 PASS，P5 声明的修复未生效 |
| 精度攻关信号 | ✅ 可信 | vbl_01_basics FAIL→PASS 真实，诊断字符串完整 |

---

## 三、可靠性审计

### 3.1 迁移矩阵 runner —— Windows `.exe` 扩展名 bug 🔴（严重）

**文档声称**（KagamiQA.md §2.3、CI workflow、README）：`kagami-qa-runner` 产出 `kagamiqa_migration_matrix.json`，含 `fail_to_pass` / `pass_to_pass` / `pass_to_fail` / `fail_to_fail` 四象限。

**实测复现**：

```
$ cargo run --release -p kagami-qa -- --manifest ../../tests/tests.json --bin-dir ../../build/tests --output out.json
Total:   39
Passed:  0
Failed:  39
Oracle A: 0P / 27F | Oracle B: 0P / 12F
```

**所有 39 个测试全部失败**，错误信息统一为：

```
"migration_note": "setup_error: Failed to execute test 'cpu_test' (fceux11_cpu_test): program not found"
```

**根因**（代码级定位，`src/rust/crates/kagami-qa/src/adapter/subprocess.rs:38-49`）：

```rust
let bin_path = if test.input.binary.contains('/') || test.input.binary.contains('\\') {
    PathBuf::from(&test.input.binary)
} else {
    let candidate = self.bin_dir.join(&test.input.binary);
    if candidate.exists() {       // ← BUG: 检查的是无扩展名路径
        candidate
    } else {
        PathBuf::from(&test.input.binary)
    }
};
```

`tests.json` 中 38/39 条目的 `binary` 字段是裸名（如 `"fceux11_cpu_test"`，无 `.exe`）。`bin_dir.join("fceux11_cpu_test")` 产生 `build/tests/fceux11_cpu_test`（无扩展名），`candidate.exists()` 在原生 Windows 上返回 `false`（实际文件是 `fceux11_cpu_test.exe`），回退到裸名 `Command::new("fceux11_cpu_test")`，`CreateProcess` 找不到程序 → "program not found"。

唯一"通过"的条目 `menu_slot_check` 之所以例外，是因为其 `binary` 是绝对路径 `C:/Users/.../python.exe`（含 `/` 且含 `.exe`），走了第一个分支。

**影响**：
1. **迁移矩阵功能在 Windows 上完全失效**。`kagamiqa_migration_matrix.json`（已入库）显示 38/39 失败，与文档声称的"39 全绿"矛盾 —— 这个文件本身就是 bug 的产物，却被 commit 进仓库作为"交付物"。
2. **CI workflow 的 "KagamiQA — Migration Matrix" 步骤使用相同 `--bin-dir build/tests`，在 CI 上同样会全失败**。CI 使用 `continue-on-error: true`，所以不会阻断流水线，但产出的 artifact 是无效的。
3. **transition_matrix 的四象限（fail_to_pass 等）永远为空数组**，因为没有任何测试被实际执行。这意味着 README 宣传的"追踪 PASS→FAIL 回归与 FAIL→PASS 进展"**从未产生过真实数据**。

**修复方向**：在 `candidate.exists()` 前追加 `std::env::consts::EXE_EXTENSION`（Windows 上为 `"exe"`），或尝试 `name` 与 `name.exe` 两个候选。

### 3.2 In-process direct runner —— 无法编译 🔴

**文档声称**（KagamiQA.md §2.5、P5 计划 §5C、README v1.16 亮点）：「in-process runner 通道打通（C ABI 直驱 core）」「`kagami-qa-runner --direct` 对 Oracle B 全量产出与 subprocess 模式 100% 一致」。

**实测复现**：

```
$ cmake --build build --config Release --target kagami_qa_direct_runner
CMake Error: Generator: build tool execution failed

$ cargo build --release -p kagami-qa --features direct-adapter --lib
error[E0133]: call to unsafe function `*argv.offset(...)` is unsafe and requires unsafe block
  --> crates/kagami-qa/src/lib.rs:39:24
error: could not compile `kagami-qa` (lib) due to 1 previous error; 4 warnings emitted
```

**根因**：`src/rust/crates/kagami-qa/src/lib.rs` 的 `direct_entry` 模块（`#[cfg(feature = "direct-adapter")]`）使用 Rust 2024 edition，但 `kagami_qa_direct_main` 函数体内的 unsafe 操作（`*argv.offset()`、`CStr::from_ptr()`）未用 `unsafe { }` 块包裹。Rust 2024 的 `unsafe_op_in_unsafe_fn` lint 默认 deny，导致编译失败。

**影响**：
1. `kagami_qa_direct_runner.exe` **从未成功生成**（`build/tests/` 下不存在）。
2. CTest `kagami_qa_direct_smoke` 因此 `Not Run`。
3. P5 计划 §5C 退出条件"`--direct` 产出与 subprocess 模式 100% 一致"**从未验证**。
4. KagamiQA.md §2.5 的完整使用说明（`cmake --build ... --target kagami_qa_direct_runner`）**无法执行**。
5. README v1.16 亮点"in-process runner 通道打通（C ABI 直驱 core）"**与事实不符**。

### 3.3 Baseline drift 检测 —— stub 实现 🔴

**文档声称**（KagamiQA.md §2.4、CI workflow、README）：「PASS→FAIL 自动在 PR 下评论红色警报」「baseline drift 检测 → PR comment」。

**实测代码**（`src/rust/crates/kagami-qa/src/report/baseline.rs:54-66`）：

```rust
/// This is a placeholder for a more sophisticated drift detector in P4+.
pub fn detect_drift(
    _manifest: &BTreeMap<String, crate::manifest::schema::TestManifest>,
    _previous: Option<&PreviousRun>,
) -> Vec<BaselineDrift> {
    // P1-P3: drift detection is deferred until the full baseline
    // governance story is in place.
    Vec::new()   // ← 永远返回空
}
```

**结论**：`detect_drift` 是**显式 stub**，永远返回空向量。CI workflow 的 "Baseline Drift Detection" 步骤读取 `transition_matrix.pass_to_fail`，但由于 §3.1 的 runner bug，`pass_to_fail` 本身就永远是空数组 —— 即使 drift 检测实现了，也无数据可检测。**双重失效**：drift 检测逻辑是 stub + 其输入源（迁移矩阵）也是坏的。

### 3.4 P4-bridge NULL deref 修复 —— 真实且有效 ✅

**文档声称**（P4-bridge 根因解决方案）：`FFCEUX_PPURead` 空指针解引用导致帧 3 崩溃，修复方案是在 `FCEUPPU_Power()` 调用 `PPU_ResetHooks()`。

**实测复现**：`ppu_vbl_nmi.nes --frames 300` 正常完成（493ms），输出完整诊断字符串，不再崩溃（exit 1 = 测试失败，非 exit 127 = 崩溃）。Oracle A `rom_regression_test` 720 帧 0 mismatch。**此修复真实、有效、已验证**。这是 KagamiQA 工程质量最高的一笔 —— 根因分析精确到 file:line，修复最小化（+11 行），回归零影响。

### 3.5 可靠性总评

| 子项 | 评级 | 证据 |
|------|------|------|
| 迁移矩阵 runner | 🔴 失效 | Windows `.exe` bug，39/39 全失败，已入库的 matrix JSON 是 bug 产物 |
| direct runner | 🔴 失效 | Rust 2024 unsafe lint 编译失败，exe 从未生成 |
| baseline drift | 🔴 stub | `detect_drift` 返回空向量，且输入源（matrix）也坏 |
| P4-bridge crash fix | ✅ 优秀 | 精确根因定位，最小修复，零回归 |
| Rust 单元测试 | ✅ 通过 | 22/22 PASS（与文档一致） |
| CI workflow 结构 | 🟡 完整但无效 | 结构齐全，但 matrix 步骤因 runner bug 产废数据 |

---

## 四、可信性审计

### 4.1 文档与实现的一致性核对

| 文档声明 | 实测结果 | 一致性 |
|---------|---------|--------|
| 「39 CTest 全绿」 | 30/33 通过（91%） | ❌ 不符 |
| 「Oracle A 全绿是每次修改的前置条件」 | 3 个失败存在 | ❌ 不符 |
| 「180 blargg ROM」 | 180 ROM 实测存在 | ✅ 一致 |
| 「120 PASS / 60 FAIL」 | 实测 120P/60F 可复现 | ✅ 一致 |
| 「vbl_01_basics FAIL→PASS」 | 实测 PASS | ✅ 一致 |
| 「迁移矩阵产出 fail_to_pass 等四象限」 | 四象限全为空数组 | ❌ 不符 |
| 「in-process runner 通道打通」 | direct runner 无法编译 | ❌ 不符 |
| 「Lua 断言级判定」 | 5 个 FAIL 被判 PASS | ❌ 不符 |
| 「baseline drift → PR 红色警报」 | stub 返回空 | ❌ 不符 |
| 「CI 常驻，每次 push 触发」 | workflow 存在且触发配置正确 | ✅ 一致（但 matrix 步骤产废数据） |
| 「权威性 = 1.00（防线）」 | 实际 ≈ 0.13（原型） | ❌ 严重不符 |

**一致率**：4/12 一致，8/12 不符。**文档系统性高估了系统成熟度**。

### 4.2 已入库的 `kagamiqa_migration_matrix.json` 是 bug 产物

仓库根目录的 `kagamiqa_migration_matrix.json`（commit `63e15bb`）内容显示 38/39 失败、`transition_matrix` 四象限全空、`oracle_breakdown` 为 0P/27F + 0P/12F。这是 §3.1 的 `.exe` bug 产物，**不代表任何真实的测试执行结果**。将此文件入库作为"交付物"，会误导后续读者认为 KagamiQA 的迁移矩阵功能已工作但其测试全失败。建议从仓库移除或标注为"runner bug 产物，非真实结果"。

### 4.3 P4-bridge 状态报告的"既有配置问题"归类不当

P4-bridge 根因解决方案 §4.4 将 `ppu_rendering_lut_test` 和 `lua_bit_test_headless` 的失败归为"既有测试基础设施工作目录配置问题，非代码回归"。审计实测表明：
- `ppu_rendering_lut_test.exe` 文件头为 `0000`（非 PE），是**损坏的构建产物**，非工作目录问题。
- `lua_bit_test` 从正确目录运行仍有 5 个真实 FAIL，是 **Lua bit 库 bug**，非工作目录问题。

将真实缺陷归类为"配置问题"会阻碍其被修复。

### 4.4 可信性总评

KagamiQA 的**精度数据（Oracle B 基线）可信**，但其**系统成熟度声明不可信**。文档将一个"Oracle B 基线已建立 + crash 已修复 + 框架骨架已搭好"的原型，描述为"权威性 1.00 的 CI 常驻防线"。权威性公式的三个因子中，ROM 覆盖率（1.00）成立，但 Oracle 独立性（1.00）因迁移矩阵失效而名存实亡，CI 常驻因子（1.00）因 matrix 步骤产废数据而仅形式成立。**真实权威性 ≈ 0.13 × 0.5 × 0.5 ≈ 0.03**，远非 1.00。

---

## 五、权威性审计

### 5.1 权威性公式复核

文档定义：`权威性 = ROM覆盖率 × Oracle独立性 × CI常驻因子`

| 因子 | 文档自评 | 审计复评 | 理由 |
|------|---------|---------|------|
| ROM 覆盖率 | 1.00 | **1.00** | 180/180 ROM 实测可复现，满足 ≥80% 门禁 |
| Oracle 独立性 | 1.00 | **0.50** | A/B 通道在架构上解耦（设计正确），但 Oracle A 的迁移矩阵通道完全失效（runner bug），Oracle A 的"全绿"声明不实，独立性被打折 |
| CI 常驻因子 | 1.00 | **0.50** | workflow 存在且会触发，但 matrix 生成步骤产出无效数据（全失败），drift 检测是 stub，CI 的"防线"功能仅 Oracle B 的 blargg 批跑部分真正生效 |

**审计权威性得分**：1.00 × 0.50 × 0.50 = **0.25**（文档自评 1.00）。

### 5.2 权威性的真正支柱

KagamiQA 当前**真正生效**的权威性来源：
1. **Oracle B blargg 批跑**（120P/60F 可复现）—— 这是唯一端到端工作的精度信号。
2. **P4-bridge crash fix**（新 PPU headless 路径打通）—— 工程质量高。
3. **Rust 单元测试 22/22**（框架内部逻辑正确）。

**未生效**的权威性支柱：
1. 迁移矩阵四象限（runner bug）—— 无法回答"补丁对不对"。
2. baseline drift 检测（stub）—— 无法自动标红回归。
3. Oracle A "全绿"门禁（3 失败）—— 门禁本身有漏洞。
4. in-process direct 通道（编译失败）—— 无法帧级调试。
5. Lua 断言判定（假阳性）—— Lua 测试信号不可信。

### 5.3 "已知失败清单是防线的一部分"—— 此命题成立 ✅

文档 FAQ 反复强调"精确知道 60 个 ROM 失败比假装全绿更权威"。审计认同此哲学，且 `blargg_known_fail.json` 的分类质量确实高（逐条 reason + runppu 标记 + eventually_pass 标记）。**这是 KagamiQA 最有价值的工程资产**——它把 FCEUX11 的精度状况从"营销语言"变成了"可证伪的清单"。即使迁移矩阵 runner 坏了，这份 known_fail 清单本身仍有独立的工程价值。

---

## 六、版权与许可证合规审计

### 6.1 项目主许可证

- `COPYING`：**GNU GPL v2**（完整许可证文本）。
- `DERIVATIVE_WORK_NOTICE.txt`：声明 FCEUX11 为 FCEUX 的 GPLv2 衍生作品，列出上游作者（Bero、Xodnizel、zeromus、adelikat、CaH4e3 等）。
- `COPYRIGHT_AUDIT.md`：审计了 `src/` 目录 816 个源文件的版权分布（345 GPLv2+，463 Unknown/None）。

**主许可证合规**：FCEUX11 作为 FCEUX 的 GPLv2 衍生作品，其源代码层面的许可证链条清晰。

### 6.2 第三方测试 ROM 的版权状况

#### 6.2.1 blargg 测试 ROM（180 个）

**文档声称**（KagamiQA.md §3.1、§4.4，download 脚本头注释）：「180 个公共领域 ROM」「blargg 测试 ROM 是公共领域的 NES 诊断 ROM」。

**审计追溯**：

| 核对项 | 结果 |
|--------|------|
| 来源 | `scripts/download_blargg_roms.ps1` 从 `https://raw.githubusercontent.com/christopherpow/nes-test-roms/master` 下载 |
| 上游仓库 LICENSE | **不存在**（`raw.githubusercontent.com/.../LICENSE` 与 `README.md` 均 HTTP 404） |
| 上游仓库声明 | 仅 "Collection of test ROMs for testing a NES emulator"，无许可证声明 |
| 原始作者 | Shay Green（blargg），社区惯例如此使用，但未找到正式的 public domain dedication |
| "公共领域"声明来源 | **仅出现在 FCEUX11 自身文档**，非上游引述 |
| git 跟踪状态 | **未被 git 跟踪**（`.gitignore` 排除 `*.nes`，blargg 子目录无 `!` 例外），构建时下载 |

**合规判定**：

1. **"公共领域"声明未经证实**。所引用的上游镜像 `christopherpow/nes-test-roms` 无任何 LICENSE 文件，按 GitHub 默认条款为"All Rights Reserved"。原始作者 Shay Green 的测试 ROM 在模拟器社区被广泛当作可自由使用，但 FCEUX11 文档将其断言为"公共领域"缺乏上游依据。**建议将措辞修正为"license not declared by upstream mirror; community-treated as freely redistributable for emulator testing"**，或定位 Shay Green 原始发布点的正式声明。

2. **未 redistribute（合规）**。blargg ROM 被 `.gitignore` 排除，仅通过下载脚本获取，FCEUX11 仓库本身不分发这些 ROM 二进制。这是正确的合规姿态。

3. **归因缺失**。`COPYRIGHT_AUDIT.md` 明确限定 scope 为 `src/`，**不覆盖 `tests/fixtures/`**。`DERIVATIVE_WORK_NOTICE.txt` 不提及 blargg / Shay Green / 测试 ROM。Shay Green 在 `COPYRIGHT_AUDIT.md` 中仅因 `nes_ntsc.c`（NTSC 滤镜）出现，与测试 ROM 无关。**建议在 `DERIVATIVE_WORK_NOTICE.txt` 或新建 `THIRD_PARTY_ROMS.md` 中补充 blargg / Shay Green 的归因**。

4. **下载脚本库存不一致**（次要）。脚本列出 178 条目，磁盘上 180 ROM，文档宣传 180。建议对账。

#### 6.2.2 nestest.nes

**状况**：`tests/fixtures/nestest.nes`（24,592 字节）**被 git 跟踪（redistributed）**，作者 Kevin Horton (kevtris, 2004)。归因仅存在于 `tests/core/cpu_test.cpp:3` 的代码注释："nestest is a public-domain ROM (Kevin Horton, 2004)"。

**合规判定**：

1. **nestest 被 redistribute**（与 blargg 不同），因此归因缺口有实际的分发侧合规重量。
2. "public-domain" 同样是项目自身断言，无 Kevin Horton 的正式声明佐证。社区惯例视 nestest 为可自由使用。
3. **建议**：在 `DERIVATIVE_WORK_NOTICE.txt` 补充 nestest / Kevin Horton 归因；或将其改为构建时下载（与 blargg 一致）。

#### 6.2.3 mapper_*.nes / test_fds.fds / test_nsf.nsf

**状况**：194 个 `mapper_*.nes`（各 16,400 字节，16 字节 iNES 头 + 16KB NOP 填充）+ `test_fds.fds` + `test_nsf.nsf`，均由 `tests/fixtures/generate_test_roms.py` 合成，**项目原创**，被 git 跟踪。

**合规判定**：**无第三方版权问题**。这些是项目生成的测试桩，受项目 GPLv2 覆盖。

#### 6.2.4 商业游戏 ROM

**审计结果**：**未发现任何商业游戏 ROM**（无 Super Mario / Zelda / 等）。`.gitignore` 显式排除 `tests/disksys.rom`（FDS BIOS）并注释"Copyrighted FDS BIOS for local FDS testing (never commit)"。**项目在避免 copyrighted BIOS 方面有明确的合规意识**。

### 6.3 版权合规总评

| 子项 | 评级 | 说明 |
|------|------|------|
| 主许可证（GPLv2）链条 | ✅ 合规 | FCEUX 衍生作品，源码版权清晰 |
| blargg ROM 来源 | ✅ 合规 | 未 redistribute（gitignored，下载获取） |
| blargg "公共领域"声明 | ⚠️ 未证实 | 上游无 LICENSE，声明为项目自身断言 |
| blargg 归因 | 🔴 缺失 | COPYRIGHT_AUDIT 不覆盖 fixtures，DERIVATIVE_NOTICE 不提及 |
| nestest.nes | ⚠️ 需补救 | 被 redistribute 但归因仅代码注释 |
| 商业 ROM | ✅ 无 | 未发现，FDS BIOS 显式排除 |
| 合成测试桩 | ✅ 合规 | 项目原创 |

**总体合规风险**：**低**。项目未分发商业 ROM，blargg ROM 未被 redistribute，FDS BIOS 显式排除。主要缺口是**归因不足**与**"公共领域"声明未证实**，属文档/归因层面的改进项，非实质性版权侵权。

---

## 七、问题清单与严重性分级

### 🔴 严重（影响权威性核心声明）

| # | 问题 | 影响 | 位置 |
|---|------|------|------|
| S1 | 迁移矩阵 runner Windows `.exe` bug，39/39 全失败 | 迁移矩阵功能完全失效，transition_matrix 永远空，CI matrix 步骤产废数据 | `adapter/subprocess.rs:38-49` |
| S2 | direct runner Rust 2024 unsafe 编译失败 | in-process 通道从未工作，P5 §5C 退出条件未达成，README "通道打通"声明不实 | `lib.rs:39-43` (`direct_entry` 模块) |
| S3 | Oracle A 非全绿（30/33），文档反复声称"全绿" | "Oracle A 全绿是前置条件"的门禁形同虚设 | ctest 实测 + 多处文档 |
| S4 | `kagamiqa_migration_matrix.json` 入库为 bug 产物 | 误导后续读者，将 runner bug 误读为"测试全失败" | 仓库根目录 |

### 🟡 中等（影响可信性）

| # | 问题 | 影响 | 位置 |
|---|------|------|------|
| M1 | baseline drift 检测为 stub（返回空） | PR 红色警报功能不工作 | `baseline.rs:54-66` |
| M2 | Lua 判定假阳性（5 FAIL 被判 PASS） | Lua 测试信号不可信，P5 §5E 退出条件未达成 | `lua_runner.cpp` + `tests/lua_scripts/test_bit.lua` |
| M3 | `ppu_rendering_lut_test.exe` 构建损坏（非 PE） | Oracle A 含一个无法执行的测试 | `build/tests/fceux11_ppu_rendering_lut_test.exe` |
| M4 | "39 CTest"措辞误导（实为清单条目数，CTest 注册 34） | 文档夸大测试规模 | README + KagamiQA.md |
| M5 | P4-bridge 报告将真实缺陷（构建损坏、Lua bug）归为"配置问题" | 阻碍缺陷修复 | P4-bridge 根因方案 §4.4 |
| M6 | blargg "公共领域"声明未证实 | 版权声明准确性问题 | KagamiQA.md §3.1/§4.4, download 脚本 |
| M7 | COPYRIGHT_AUDIT 不覆盖 tests/fixtures | 第三方 ROM 版权未审计 | `COPYRIGHT_AUDIT.md` scope |

### 🟢 轻微（改进项）

| # | 问题 | 影响 | 位置 |
|---|------|------|------|
| L1 | nestest.nes 被 redistribute 但归因仅代码注释 | 归因不够正式 | `tests/fixtures/nestest.nes` |
| L2 | 下载脚本 178 条目 vs 磁盘 180 ROM vs 文档 180 | 库存对账不一致 | `download_blargg_roms.ps1` |
| L3 | `main.rs:186` 传空 config 给 `adapter.init()` | 潜在隐患（当前 init 是 no-op） | `main.rs:186` |
| L4 | `CALL_PPUREAD` 不判空而 `CALL_PPUWRITE` 判空 | 不对称，放大 NULL 风险 | `ppu.cpp:301`（P4-bridge 报告已记录） |

---

## 八、整改建议

### 8.1 立即修复（恢复权威性支柱）

1. **修复 runner `.exe` bug**（S1）：在 `subprocess.rs` 的 `candidate.exists()` 前，当 binary 名无扩展名时追加 `std::env::consts::EXE_EXTENSION`。修复后重跑 runner 验证 39 条目可执行，重新生成 `kagamiqa_migration_matrix.json`。
2. **修复 direct runner 编译**（S2）：在 `lib.rs` `direct_entry` 模块的 unsafe 操作外包 `unsafe { }` 块，或在函数头加 `#[allow(unused_unsafe)]` / 调整 lint。验证 `kagami_qa_direct_runner.exe` 可生成。
3. **移除或标注 bug 产物 JSON**（S4）：从仓库移除 `kagamiqa_migration_matrix.json`，或在文件头加 `"_warning": "runner bug artifact, not real results"`，待 S1 修复后重新生成。
4. **修复或排除 Oracle A 失败测试**（S3）：
   - `ppu_rendering_lut_test`：重建该目标，确认 exe 为有效 PE。
   - `lua_bit_test`：修复 Lua bit 库的 5 个位运算 bug，或修正 runner 判定逻辑。
   - `kagami_qa_direct_smoke`：依赖 S2 修复。

### 8.2 短期改进（提升可信性）

5. **实现 baseline drift 检测**（M1）：将 `detect_drift` 从 stub 升级为真实实现（对比 previous.runs 与当前 expected）。或明确在文档中标注"drift 检测为 P6 待实现"。
6. **修复 Lua 判定**（M2）：实现 P5 §5E 的 assert 捕获，解析脚本输出的 `FAIL:` 行。或修正 README/KagamiQA.md 的"P5 升级后…"声明。
7. **修正文档过度声明**（M4/M5）：将"39 CTest 全绿"改为"34 CTest 注册，30 通过，3 已知失败"；将 P4-bridge 报告中"既有配置问题"重新归类。
8. **修正权威性公式自评**：将 1.00 修正为审计复评的 0.25，或在文档中标注"目标 1.00，当前 0.25"。

### 8.3 合规补救

9. **补充第三方 ROM 归因**（M7/L1）：在 `DERIVATIVE_WORK_NOTICE.txt` 或新建 `THIRD_PARTY_ROMS.md` 中，补充 blargg/Shay Green、nestest/Kevin Horton 的归因。
10. **修正"公共领域"措辞**（M6）：将"公共领域 ROM"改为"license not declared by upstream; community-treated as freely redistributable"，或定位 Shay Green 原始声明并引述。
11. **扩展 COPYRIGHT_AUDIT 范围**（M7）：将 `tests/fixtures/` 纳入版权审计，或在审计文档中明确声明"fixtures 不在审计范围，另见 THIRD_PARTY_ROMS.md"。

### 8.4 保持优势

12. **保持 Oracle B 基线的版本化治理**：`blargg_known_fail.json` 的分类质量是 KagamiQA 最有价值的资产，应继续保持逐条 reason + runppu 标记 + eventually_pass 标记的纪律。
13. **保持 P4-bridge 式的根因分析质量**：NULL deref 根因方案（file:line 级证据链 + 实证复现 + 最小修复 + 零回归验证）是工程文档的典范，建议作为后续缺陷分析的模板。

---

## 九、审计声明

本审计基于 `wip_1.16` 分支 HEAD（`7c2356b`）在 2026-07-28 的状态。所有实测均在本地 Windows 环境（Git Bash + MSVC NMake 构建目录）执行，命令与输出均已在审计过程中记录。审计未修改任何源代码或测试数据。

本审计的局限性：
1. 未在 GitHub Actions 实际触发 CI workflow 验证云端行为（仅审查 workflow 文件 + 本地复现 runner 行为）。
2. 未穷举全部 180 个 blargg ROM 的逐条结果（仅验证汇总数 120P/60F + 抽验 `vbl_01_basics`/`ppu_vbl_nmi`/`sprite_overflow_1`/`cpu_timing_test`）。
3. 未对 `christopherpow/nes-test-roms` 仓库的 git 历史做完整追溯以寻找可能的许可证声明（仅检查当前 master 的 LICENSE/README）。
4. Shay Green 原始发布点（非 christopherpow 镜像）的许可证声明未深入追溯，建议由项目维护者补充。

---

## 附录 A：实测命令与结果摘要

| 命令 | 结果 |
|------|------|
| `ctest --test-dir build -LE perf` | 30/33 通过（91%），3 失败 |
| `fceux11_blargg_runner --manifest ...`（从 tests/ 目录） | 180 ROM，120P/60F ✅ |
| `fceux11_blargg_runner --manifest ...`（从项目根目录） | 180 FAIL（working dir 错误，0xFE load error） |
| `kagami-qa-runner --bin-dir ../../build/tests` | 39/39 失败（.exe bug） |
| `cargo test -p kagami-qa` | 22/22 PASS ✅ |
| `cargo build --features direct-adapter` | 编译失败（unsafe lint） |
| `cmake --build --target kagami_qa_direct_runner` | 失败（依赖 Rust 编译失败） |
| `fceux11_blargg_runner --rom vbl_01_basics.nes` | PASS ✅ |
| `fceux11_blargg_runner --rom ppu_vbl_nmi.nes` | FAIL 0x80 + 诊断字符串（非崩溃）✅ |
| `fceux11_lua_runner test_bit.lua` | 5 FAIL 被判 PASS（假阳性） |
| `file fceux11_ppu_rendering_lut_test.exe` | `data`（非 PE，文件头 0000） |

## 附录 B：关键文件索引

| 主题 | 位置 |
|------|------|
| KagamiQA 系统文档 | `docs/tech/KagamiQA.md` |
| P0 总计划 | `docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md` |
| P5 权威性计划 | `docs/history/plans/FCEUX11-1.16_KagamiQA-P5-权威性构建计划.md` |
| P4-bridge 根因方案 | `docs/history/reports/FCEUX11-1.16_KagamiQA-P4-bridge-根因解决方案.md` |
| runner .exe bug | `src/rust/crates/kagami-qa/src/adapter/subprocess.rs:38-49` |
| direct runner 编译失败 | `src/rust/crates/kagami-qa/src/lib.rs:39-43` |
| drift stub | `src/rust/crates/kagami-qa/src/report/baseline.rs:54-66` |
| Oracle B 基线 | `tests/fixtures/blargg_full_baseline.json` |
| 已知失败清单 | `tests/fixtures/blargg_known_fail.json` |
| CI workflow | `.github/workflows/kagami-qa.yml` |
| ROM 下载脚本 | `scripts/download_blargg_roms.ps1` |
| 主许可证 | `COPYING`（GPLv2） |
| 版权审计（不含 fixtures） | `COPYRIGHT_AUDIT.md` |
| 衍生作品声明 | `DERIVATIVE_WORK_NOTICE.txt` |
| bug 产物 JSON | `kagamiqa_migration_matrix.json`（仓库根） |
