# FCEUX11 v1.16 二阶段构建计划（Stage-2）

> **日期**：2026-07-29（**收官状态回填：2026-07-30**）
> **分支**：`wip_1.16`
> **基线 HEAD**：`301c742`（工作树干净）
> **前置文档**：`docs/FCEUX11-1.16_KagamiQA-遗留问题与构建难题.md`（下称「遗留文档」）
> **关联**：`docs/history/FCEUX11-1.16_KagamiQA-{审计报告,修复验证报告,PLAN,P5-权威性构建计划}.md`、`docs/BuildGuide.md`
> **约束**：每 Phase ≤ 6 PR（沿用 hotfix3 起的注意力窗口约定）；新文件只落 `scripts/` / `tests/` / `src/` / `docs/`，不落项目根目录

---

## 收官摘要（2026-07-30）

| Phase | 完成度 | 说明 |
|---|---|---|
| 0 事实校准 | **4/4 ✅** | 0-2 的 matrix 产物已于 S-4 在 HEAD 上重生 |
| 0.5 判定链路 | **4/4 ✅** | — |
| A 构建环境 | **6/6 ✅** | 裸 PowerShell 一次成功，自动选 Ninja，四个构建难题全部不复现 |
| B Lua bit | **4/4 ✅** | — |
| C direct runner | **5/5 ✅** | C-2 / C-4 被 C-1 的推荐解法吸收，无需单独实施 |
| D Oracle B / CI | **4/4 ✅** | D-2 残留的 3 个「缺失 ROM」经查为命名错配，S-1 已清零 |
| E 精度收尾 | **1/3** | E-2 ✅；**E-1 未收敛**（两次尝试被证伪）；**E-3 仅完成实测分桶** |

**实测数字**：`ctest` **34/34 (100%)**、`cargo test -p kagami-qa` **40/40**、
migration matrix **35/39**（`git_rev=623dd39`），4 项 FAIL 全部是有分类依据的已知失败。

**收官阶段（S 系列）额外发现并修复的 3 项缺陷**（均不在原 30 PR 内）：

| 编号 | 缺陷 | 影响 |
|---|---|---|
| S-1 `bc7c1d8` | D-1 判为「ROM 缺失」的 3 条是**指向不存在路径的重复条目**；真正压红的是 300 帧预算不足 | 覆盖率分母口径失真；两条核心 CPU 权威测试被无依据地 demote |
| S-2 `623dd39` | `InputSpec::from_manifest` 把 `frames` 写死 300，**direct 模式从不读 manifest 的 `--frames`** | direct/subprocess 判定不对等；长跑 ROM 一律误报 `0x80`，掩盖 E-1 的真实 `0x01` 诊断 |
| S-4 `ceed00e` | `git_rev` 用 `option_env!`（编译期），而脚本在**运行期**赋值 → 恒为 `"unknown"` | 产物无法追溯，废快照与新产物不可区分 —— 正是本次开工时踩到的坑 |

**真剩余**：仅 E-1（PPU VBL/NMI 边沿时序）与 E-3（APU 桶 C 精度），两者均为模拟精度问题，
与构建、判定链路、覆盖率口径无关，需独立 PR 推进。

---

---

## 〇、执行摘要

对遗留文档所列 **4 项「不修复/暂不修复」** 与 **4 项「构建难题」** 逐条做了代码级复核（而非仅采信文档结论）。核心结论：

> **8 项中有 6 项可以根治，其中 4 项的「不可修复」判定基于错误的根因分析。**

| 遗留文档结论 | 复核后判定 | 依据 |
|---|---|---|
| §1.1 L3 空 config「不修复」 | ✅ **维持**（但建议 1 行加固） | trait no-op 论证成立 |
| §1.2 L4 CALL_PPUREAD 不判空「不修复」 | ✅ **维持**（建议加 debug assert） | fail-fast 论证成立 |
| §1.3 direct_smoke「工程复杂性，不修复」 | ❌ **推翻 — 可修复** | 根因被误判；真因是 30 行 CMake 缺陷 |
| §1.4 Lua bit 库「5 个真实 bug，暂不修复」 | ❌ **推翻 — 4/5 不是 bug** | 4 项是测试期望值写错，1 项是 1 行实现修复 |
| §2.1 Bash vs MSVC | ✅ **可根治** | `scripts/do_build.ps1` 已实现，仅探测逻辑有缺口 |
| §2.2 PDB C1041 | ✅ **可根治** | 切 Ninja 即消除；CI 早已无此问题 |
| §2.3 LNK1104 文件锁 | ⚠️ **可大幅缓解**，无法 100% 根除 | OBJECT 库 + 重试；属 Windows/AV 环境问题 |
| §2.4 LTCG c2.dll 崩溃 | ✅ **已修复**，需补保险 | `/GL-` 已落；`bus.cpp` 是同类潜在风险点 |

**一句话**：遗留文档把「我没能在 Git Bash 里把它构建出来」记录成了「这个问题不可解」。实际阻塞点绝大多数是**本地构建环境配置**与**测试期望值错误**，而非工程复杂性。CI（Ninja + Release）从未出现这四个构建难题中的任何一个——这本身就是最强的反证。

### 补充：判定链路自身的可信性缺陷（本计划新增发现）

在复核过程中另发现 **4 项此前未被任何审计或计划文档记录的缺陷**，全部位于 KagamiQA 的判定链路本身：生产判定只看退出码而忽略 `stdout_contains`、实现了正确判定的 `oracle/regression.rs` 是死代码、`timeout_seconds` 从不生效、`fail_to_pass` 可被新增测试灌水。

其中最后一条直接击穿框架的反 gaming 设计。**这些缺陷的严重性高于原「遗留问题」清单中的任何一项**——构建失败是显性的（跑不出来），判定失真是隐性的（跑出来了，但结论是错的）。已单列为 **Phase 0.5**，与 Phase A 并列为硬阻塞。

---

## 一、对遗留文档的勘误（必读）

在制定计划前，必须先纠正遗留文档及其上游审计报告中**与 HEAD 实际状态不符**的记载。以下 3 项是我实测验证的：

### 勘误 1：`ppu_rendering_lut_test.exe` 已是有效 PE，不是 2 MiB 全零文件

审计链路多处仍称该文件为「2,097,152 字节全零、无 PE 头」。实测：

```
$ ls -la build/tests/fceux11_ppu_rendering_lut_test.exe
-rwxr-xr-x 10614784 Jul 29 00:54 ...

$ xxd -l 16 build/tests/fceux11_ppu_rendering_lut_test.exe
00000000: 4d5a 9000 0300 0000 0400 0000 ffff 0000  MZ..............
```

10.6 MB，`MZ` 头正常。**§2.4 的 `/GL-` 修复确实生效了**，遗留文档 §三的表格记载正确，但仍在流传的「2MB 全零」说法已过期。

### 勘误 2：migration matrix 的 `.exe` 后缀 bug 已在 `2d15b66` 修复

审计报告称 S1 修复产出了 `fceux11_cpu_testexe`（缺点号）。实测 `src/rust/crates/kagami-qa/src/adapter/subprocess.rs:53`：

```rust
let candidate_exe = self.bin_dir.join(format!("{}.{}", name, std::env::consts::EXE_EXTENSION));
```

`EXE_EXTENSION` == `"exe"`（无点），`format!` 补了点号 → `fceux11_cpu_test.exe`。**代码是对的。**

但仓库根目录的 `kagamiqa_migration_matrix.json` 仍是 **修复前**的快照：

```json
"run_id": "20260728-060755-71ef94",
"summary": {"total": 39, "passed": 1, "failed": 38, "skipped": 0}
```

即：**代码已修，产物未重跑**。这份废数据是「权威性 0.25」的直接来源，必须重新生成才能证伪或证实。

### 勘误 3：direct runner 的根因判定完全错误

遗留文档 §1.3 列了三条阻塞点，逐条核对：

| 遗留文档的说法 | 实际情况 |
|---|---|
| 「NMake `add_dependencies` 不传递 Rust 构建失败信号」 | ❌ 失败的 custom target 命令**会**中断构建。真正的依赖缺陷是另一回事（见 C-5） |
| 「rlib 与 MSVC COFF/LTCG 的 ABI 交互需精确协调」 | ❌ 误判。rlib 不是 ABI 问题——它**根本不是可被 `link.exe` 消费的产物** |
| 「MSVC 14.51 LTCG 已知不稳定性」 | ❌ 与本项无关。`link.exe` 原生支持混合 `/GL` 与普通 COFF 输入 |

真因是 `tests/CMakeLists.txt:637-665` 这 30 行里的 4 个具体缺陷，详见 Phase C。**Rust 侧完全没问题**——已实测 `cargo build --release -p kagami-qa --features direct-adapter --lib` 退出码 0，8 个 FFI 符号与 C++ 侧一一对应。

---

## 二、新增发现（不在任何现有文档中）

复核过程中发现 3 项此前未被记录的问题：

### N-1 🔴 `tests/tests.json` 硬编码了作者本机的 Python 绝对路径

`menu_slot_check` 条目（`tests/tests.json`，provenance `hotfix4 T-1`）：

```json
"input": {
  "binary": "C:/Users/ikrx2/AppData/Local/Programs/Python/Python313/python.exe",
  "args": ["scripts/check_menu_slots.py", "."]
}
```

这条目在**任何其他机器和 CI 上都必然 setup_error**。它是 migration matrix「38 failed」中确定无疑的一员，且与 `.exe` 后缀 bug 无关——即使重跑 matrix，这条仍会红。必须改为 `python` / `py -3` 并由 runner 走 PATH 解析。

### N-2 🟡 `_FCEUX11_CORE_LIBS` / `_FCEUX11_OPENGL_LIBS` 是全仓库未定义变量

```
$ grep -rn "_FCEUX11_CORE_LIBS\|_FCEUX11_OPENGL_LIBS" --include=*.txt --include=*.cmake .
./tests/CMakeLists.txt:663:            ${_FCEUX11_CORE_LIBS}
./tests/CMakeLists.txt:664:            ${_FCEUX11_OPENGL_LIBS}
```

仅有的两处出现就是这两处引用点，**定义处不存在**。CMake 对未定义变量静默展开为空，所以 `ntdll` / `userenv` / `ws2_32` / `dbghelp`（Rust std 必需）从未被链接。注释写着「matching `fceux11_add_headless_test_executable`」，但那个 helper 在 `tests/CMakeLists.txt:66` 是显式写 `ntdll` 的——这里是照抄注释没照抄代码。

### N-3 🟡 Oracle B 的 ROM 覆盖率远低于文档宣称

| 口径 | 数量 |
|---|---|
| `tests/fixtures/blargg/` 磁盘上的 ROM | **177** |
| `tests/tests.json` 中 blargg 相关条目 | **5**（`blargg_smoke` / `cpu_instrs` / `cpu_timing` / `ppu_vbl_nmi` / `blargg_suite`） |
| P5 计划宣称目标 | ≥140（≥80%） |

`blargg_suite` 是聚合条目，实际驱动的 ROM 数需在 Phase D 实测确认，但「5 个 manifest 条目 vs 177 个 ROM」的结构性缺口是确定的。

---

## 三、Phase 划分总览

7 个 Phase，共 30 个 PR。Phase 之间**串行为主**（后者依赖前者的构建与判定能力），Phase 内部 PR 可并行。每 Phase ≤6 PR。

```
Phase 0    事实校准与基线重建        4 PR   ← 先证伪废数据，否则后续全在猜
   │
Phase 0.5  判定链路可信性修复        4 PR   ← 判定逻辑本身不可信，则一切测量无意义
   │
Phase A    构建环境根治              6 PR   ← 解决 §2.1~§2.4，这是一切的前提
   │
   ├─ Phase B  Lua bit 库归零        4 PR   ← 独立，可与 C 并行
   ├─ Phase C  direct runner 端到端   5 PR   ← 独立，可与 B 并行
   │
Phase D    Oracle B 覆盖率与 CI 权威性 4 PR   ← 依赖 A（构建）+ C（可选）
   │
Phase E    精度遗留与收尾            3 PR   ← 依赖 A + D 的可复现构建
```

**两条关键路径**：

- **Phase A 是构建侧的硬阻塞**。在 A 完成前，B/C/D/E 的任何验证都不可信——当前本地构建（NMake + Debug）本身就是四个构建难题的产地，而 CI（Ninja + Release）是绿的。
- **Phase 0.5 是判定侧的硬阻塞**。在 0.5 完成前，即使构建成功、测试全跑，**产出的 PASS/FAIL 与迁移矩阵也不能作为决策依据**——因为判定逻辑与 manifest 声明不一致（详见下节）。

两者互不依赖，可并行推进；但 D（权威性指标）必须同时等两者。

---

## 四、Phase 0 — 事实校准与基线重建

**目标**：用实测数据取代过期文档结论，建立可信基线。**不写任何功能代码。**

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **0-1 ✅ `22aea5a`** | 修复 `menu_slot_check` 的硬编码 Python 路径（N-1） | `tests/tests.json` | binary 改为 `python`；在无该路径的机器上不再 setup_error |
| **0-2 ✅（S-4 重生）** | 重新生成 `kagamiqa_migration_matrix.json`，用 `2d15b66` 后的 runner | 产物（gitignored） | 记录真实 passed/failed；若仍大面积 failed，逐条归因 |
| **0-3 ✅ `b53a7a0`** | 校正遗留文档 §三与审计链路中已过期的 3 项记载（本文档 §一） | `docs/FCEUX11-1.16_KagamiQA-遗留问题与构建难题.md` | 「2MB 全零」「后缀 bug 未修」「rlib ABI」三处更正 |
| **0-4 ✅ `fee5c81`** | 修复 `docs/BuildGuide.md` §8.4 的死引用 | `docs/BuildGuide.md` | §8.4 指向的 `scripts/_find_vcvars.bat` 实际在 `scripts/archive/`；改为指向 `do_build.ps1` |

**Phase 0 的意义**：遗留文档的多数「不可修复」结论，是在**一个坏掉的本地构建环境 + 一份废数据 matrix** 上得出的。不先校准，后面每个 Phase 都会重蹈覆辙。

**风险**：0-2 重跑 matrix 需要一次成功的 Release 构建——若当前环境跑不通，0-2 顺延至 Phase A 之后执行（这是唯一允许跨 Phase 顺延的 PR）。

---

## 四·五、Phase 0.5 — 判定链路可信性修复

**目标**：让 KagamiQA 的判定逻辑与它自己的 manifest schema 声明相符。

**为什么必须独立成 Phase**：Phase 0 校准的是「数据」，本 Phase 校准的是「判定数据的规则」。规则若与声明不符，重跑再多次也只是把错误结论刷新得更快。以下 4 项均为实测确认，此前**不在任何审计与计划文档中**。

### 0.5-a 🔴 生产判定只看退出码，`stdout_contains` 是装饰性字段

`src/rust/crates/kagami-qa/src/adapter/subprocess.rs:84` —— 生产路径唯一判据：

```rust
let passed = exit_code == test.expected.exit_code;
```

而 `manifest/schema.rs:78` 声明了 `expected.stdout_contains: Option<String>`。该字段**从未被生产代码读取**。任何按 schema 编写、依赖 stdout 断言的用例，都会拿到**静默的假 PASS**。

### 0.5-b 🔴 `oracle/regression.rs` 是死代码

```
$ grep -rn "check_expected\|regression::" src/rust/crates/kagami-qa/src/ | grep -v regression.rs
（无输出）
```

`oracle/regression.rs:7-19` 实现了正确的两因子判定（退出码 + stdout 包含），**全仓库无任何调用点**。它自带 4 个单元测试且全部通过——即**测试覆盖了一个永不执行的函数**，这类绿灯是纯粹的噪声。

架构意图（oracle 层负责判定）与实现现状（adapter 层自行判定）已经背离。

### 0.5-c 🟡 `timeout_seconds` 全链路解析但从不生效

`manifest/schema.rs:14` 与 `core/config.rs:15` 都有该字段，`tests.json` 每条都填了值，但执行走 `Command::output()`——**无超时机制**。挂死的测试将永久阻塞 runner，而不是判 FAIL。这在 CI 上表现为 job 超时（无诊断信息），而非一条明确的失败用例。

### 0.5-d 🔴 `fail_to_pass` 可被「新增测试」灌水 —— 反 gaming 设计的正门

`report/matrix.rs:184`：

```rust
let prev_passed = prev.results.get(&r.test_id).copied().unwrap_or(false);
```

基线中不存在的 test_id **默认视为「此前 FAIL」**。于是：**新增一条通过的测试 → 直接记入 `fail_to_pass`**。

这一条直接击穿框架的核心防御。PLAN §十一.1 明确：「AI 读 FAIL_TO_PASS 即判补丁对错」；§4.3 的反 gaming 规则是「**AI 不得修改已入库的 `expected` 值，只能新增条目**」。而「只能新增」恰恰就是刷分路径——规则堵死了改期望值这扇门，却把灌水留了个正门。

对照 SWE-bench：其 `FAIL_TO_PASS` 是**任务实例预先固定的清单**，不由补丁运行时动态生成。KagamiQA 缺了这层锚定。

### 现状澄清（避免过度恐慌）

`tests/blargg_runner.cpp:425,441` 把判定编码进了退出码（`fail_count > 0 ? 1 : 0` / `r.passed ? 0 : 1`），因此 **Oracle B 当前没有产生误报**。0.5-a/b/c 是**潜伏缺陷**而非正在犯的错误。但 0.5-d 是**当前就在生效**的指标失真。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **0.5-1 ✅ `36bd311`** | `SubprocessAdapter::run_test` 改为调用 `oracle::regression::check_expected`，消除判定逻辑二地并存 | `adapter/subprocess.rs:235`、`oracle/regression.rs:10` | `stdout_contains` 生效；新增一条「退出码 0 但 stdout 不含期望串」的用例，必须判 FAIL |
| **0.5-2 ✅ `7bf7771`** | 实装 `timeout_seconds`：改用带超时的等待（`wait_timeout` 或 spawn + 轮询），超时判 FAIL 并写入 `migration_note` | `adapter/subprocess.rs` | 新增故意挂死 30s 的用例，在 5s 超时下判 FAIL 而非阻塞 |
| **0.5-3 ✅ `dfe0710`** | 迁移矩阵引入第五桶 `new_test`：基线中不存在的 test_id 不得计入 `fail_to_pass` | `report/matrix.rs:184` | 新增通过的测试出现在 `new_test`，`fail_to_pass` 保持为空 |
| **0.5-4 ✅ `39c758b`** | 反 gaming 加固：`tests.json` 的用例集合变更需与基线更新同级评审；矩阵报告显式标注本次运行的「新增/删除用例数」 | `report/matrix.rs`、`docs/tech/KagamiQA.md` §反 gaming | 报告中用例集合变更可见、可 diff、不可静默 |

**0.5-3 的设计要点**：不要简单把 `unwrap_or(false)` 改成 `unwrap_or(true)`——那会把新增的失败测试记成 `pass_to_fail`（假回归警报），只是把偏差换了个方向。**新测试在语义上不属于任何迁移桶**，必须单列。

---



**目标**：让「从任意 shell 一条命令构建成功」成为常态，永久消除 §2.1~§2.4。

### 判定依据

CI（`.github/workflows/ci.yml:68`）用 **Ninja + Release + MSVC**，`ctest -LE perf` 全绿；本地 `build/CMakeCache.txt` 是 **NMake + Debug**。四个构建难题**全部只在本地 NMake 路径出现**。`scripts/do_build.ps1:33-35` 其实已经优先选 Ninja：

```powershell
try { & ninja --version | Out-Null; $ninjaOk = ($LASTEXITCODE -eq 0) } catch {}
if ($ninjaOk) { $generator = "Ninja" }
elseif (Get-Command nmake ...) { $generator = "NMake Makefiles" }
```

但本机 `ninja.exe` 只存在于 VS 安装目录下（`Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/`），**不在裸 PATH 上**，旧探测因此失败 → 回落 NMake → 触发全部四个难题。这里的准确表述是「裸 PATH 不可见」，不是「电脑未安装 Ninja」。

**2026-07-29 A-1 落地后复核**：`vswhere.exe` 确认本机有两套完整、可启动的 Visual Studio 18 C++ 工具链，两套都自带 Ninja 1.13.2 和 CMake 4.2.3-msvc3：

```text
C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
D:\Program Files\Microsoft Visual Studio\18\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
```

因此后续排查不得再以 `Get-Command ninja` / `where ninja` 在裸 shell 无输出为依据声称「Ninja 缺失」。应先运行 `vswhere.exe -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`，再检查每个安装根下的上述相对路径。`do_build.ps1` 现已自动完成这一步，并在实跑中选中 BuildTools 内置 Ninja。

同时确认：CMake 侧**零处**依赖 NMake（无 `if(CMAKE_GENERATOR MATCHES "NMake")`、无 `CMAKE_MAKE_PROGRAM` 覆盖、无 makefile 专用变量）。切换 Ninja 无需改任何 CMake 代码。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **A-1 ✅ `24e6512`** | `do_build.ps1` 已用 `vswhere.exe` + VS 安装根回退定位内置 `ninja.exe`，并将其目录加入本次构建的 `PATH` | `scripts/do_build.ps1` | 已在裸 Git Bash → 普通 PowerShell 调用链实测选中 Ninja 1.13.2 |
| **A-2 ✅ `2575f33`** | Ninja 已确立为唯一受支持的本地生成器；NMake 仅在穷尽 VS 内置 Ninja 后作 legacy 回退并打印醒目告警 | `scripts/do_build.ps1`、`docs/BuildGuide.md` §4/§7/§8 | 文档与脚本统一为「PATH 不可见 ≠ 未安装」口径 |
| **A-3 ✅ `f7d1994`** | 设 `CMAKE_MSVC_DEBUG_INFORMATION_FORMAT=Embed`（`/Z7`）作为 C1041 的第二道防线 | `CMakeLists.txt` | 即使回落 NMake 也不再 PDB 争用；CMake ≥3.25 要求已满足（项目要求 4.0） |
| **A-4 ✅ `17c9e44`** | `fceu11_direct_storage_probe` 由 `STATIC` 改为 `OBJECT` 库，消除该 `.lib` 落盘 | `src/CMakeLists.txt:570-576` | 不再产出 `fceu11_direct_storage_probe.lib`；消费端 `:639-641` 无需改动 |
| **A-5 ✅ `c3a8383`** | `do_build.ps1` 增加 LNK1104 重试循环（检测 → kill 残留 `cl/nmake/link` → 删锁定 `.lib` → 重试 ≤3 次） | `scripts/do_build.ps1` | 覆盖全部 6 个静态库，不止 probe 库 |
| **A-6 ✅ `99f048f`** | 保险：对 `bus.cpp` 预防性加 `/GL-` | `src/CMakeLists.txt:598-599` | `bus.h:197-198` 的 `aread_[0x10000]`+`bwrite_[0x10000]`（合计 1 MiB）是仅次于 `kSpriteIdxLUT` 的 LTCG 风险点 |

### A-6 的风险评估（为什么值得做）

`kSpriteIdxLUT`（512 KiB，`alignas(64)` + lambda 初始化）触发了 c2.dll 崩溃。全仓库扫描后，唯一同量级的候选是 `src/bus.h:197-198` 的两张函数指针表（合计 1 MiB）。它们由 `bus.cpp` 中的循环赋值填充（`aread_[x] = ANullImpl`），**不是** constexpr lambda 初始化——如果 c2.dll 的 bug 特定于 lambda 物化路径，`bus.cpp` 就是安全的。但若 bug 只是「大 obj + /GL + /LTCG」的泛化问题，`bus.cpp` 就是下一个。1 行 `/GL-` 的成本约等于零，值得买这份保险。

其余候选（`ppulut1/2/3` ~2.5 KiB、`kBitRevLUT` 256 B、`SPRBUF` 384 B）体量差 2~3 个数量级，无风险。

### Phase A 验收门

```powershell
# 从裸 PowerShell（非 Developer Shell）
.\scripts\do_build.ps1 -Config Release
```
必须：① 自动选中 Ninja；② 全程无 C1041 / LNK1104；③ 构建耗时回到 ~10 分钟量级（而非 §2.2 记载的 30 分钟）；④ `ctest -LE perf` 结果与 CI 一致。

---

## 六、Phase B — Lua bit 库归零

**目标**：`lua_bit_test_headless` 转 PASS。

### 关键结论：5 个「真实 bug」里，4 个是测试期望值写错

遗留文档 §1.4 称这是「fceux11-lua crate 的精度缺陷」。对照 **LuaBitOp 1.0.2 规范**（`bit` 库的事实标准，FCEUX Lua 生态沿用）逐条核对 `src/rust/crates/fceux11-lua/src/bindings/bit.rs`：

| # | 断言 | 位置 | 真正的错误方 | 依据 |
|---|---|---|---|---|
| F1 | `rshift(-1,31)` 得 -1 期望 1 | `bit.rs:38` | 🔴 **实现** | `i32::wrapping_shr` 是**算术**右移（符号扩展）；规范要求 `rshift` 是**逻辑**右移 |
| F2 | `ror(0x01,1)` 得 -2147483648 期望 2147483648 | `test_bit.lua:56` | 🟡 **测试** | 规范明示「所有位运算返回**有符号** 32 位数」，`-2147483648` 是正确答案 |
| F3 | `tohex(255)` 得 `000000ff` 期望 `000000FF` | `test_bit.lua:63` | 🟡 **测试** | 规范：默认生成 **8 位小写**十六进制 |
| F4 | `tohex(255,2)` 得 `ff` 期望 `FF` | `test_bit.lua:64` | 🟡 **测试** | 规范：`n` 为正 → 小写；负 → 大写 |
| F5 | `tohex(-1,4)` 得 `ffff` 期望 `FFFFFFFF` | `test_bit.lua:65` | 🟡 **测试** | `n=4` → 取低 16 位、4 位小写 → `ffff`。期望值大小写和宽度都错 |

**即：实现只有 1 处真 bug（F1），另外 4 处是测试自己写错了期望值。** 遗留文档「5 个真实 bug」的定性不成立，「需要 Rust Lua 绑定层面的修复」也过度估计了工作量。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **B-1 ✅ `4f92f60`** | 修复 `rshift` 为逻辑右移：`Ok((x as u32).wrapping_shr(n as u32) as i32)` | `src/rust/crates/fceux11-lua/src/bindings/bit.rs:38` | `rshift(-1,31)==1`；`rshift(256,8)==1` 不回归 |
| **B-2 ✅ `8edb014`** | 修正 4 处测试期望值（F2~F5）为规范值，并在每行加注 LuaBitOp 规范引用 | `tests/lua_scripts/test_bit.lua:56,63,64,65` | 期望值有据可查，避免后人再次误判为实现 bug |
| **B-3 ✅ `f3f1a72`** | 补齐 `tohex` 的负数 `n` → 大写语义（规范要求，当前实现**永不产出大写**） | `bit.rs:56-67` | 新增断言 `tohex(255,-2)=="FF"`、`tohex(-1,-8)=="FFFFFFFF"` |
| **B-4 ✅ `1b0f5dd`** | 对 `bit` 库全 API 做一次规范对照审计（`tobit/bnot/band/bor/bxor/lshift/arshift/rol/bswap`），补测试 | `bit.rs`、`test_bit.lua` | 防止同类「实现对/测试错」或反向问题继续潜伏 |

> **B 阶段附带修复**：`c2a89c0`（B-fix-2）修正 `lua_bit_test_headless` 的 CTest 工作目录/脚本路径错配，
> `362fcf3` 内含 B-fix-1 —— `lua_runner.cpp` 不再把摘要行 "X passed, Y failed" 里的 "failed" 子串当失败信号。
> 这两处才是该测试此前显示 FAIL 的近因，与 bit 库语义无关。

**B-1 的回归风险**：对所有非负 `x`，`x as u32` 保持位模式不变，结果与旧实现完全一致；仅负数路径改变——而负数路径正是规范要求按无符号处理的。现存通过的断言不受影响。

**B-3 的必要性**：当前 `tohex` 用 `format!("{:0>width$x}", ...)` 硬编码小写，规范中「负 `n` → 大写」这半条语义**不可达**。这是真实的功能缺失（虽然当前无测试覆盖），属于 F3~F5 误判的连带产物——正因为实现不支持大写，写测试的人才误以为大写是默认行为。

---

## 七、Phase C — direct runner 端到端联合构建

**目标**：`kagami_qa_direct_runner` 成功链接，`kagami_qa_direct_smoke` 从 `Not Run` 转 PASS。

### 判定：可修复。真因是 CMake，不是「跨语言工程复杂性」

已实测：`cargo build --release -p kagami-qa --features direct-adapter --lib` **退出码 0**（仅 2 个无害 warning），8 个 FFI 符号与 C++ 侧完全对应。Rust 侧不是阻塞点。

四个硬阻塞，全在 `tests/CMakeLists.txt:637-665`：

**C-a｜产物类型错误（rlib ≠ 可链接产物）**
`src/rust/crates/kagami-qa/Cargo.toml:7` 声明 `crate-type = ["rlib"]`，cargo 产出 `libkagami_qa.rlib`——这是 Rust 内部归档格式，`link.exe` **无法消费**。而 CMake 在 `:661` 链接 `.../kagami_qa.lib`，该文件**从不存在** → 必然 LNK1104。已实测验证修法：

```
cargo rustc --release -p kagami-qa --features direct-adapter --lib --crate-type staticlib
→ target/x86_64-pc-windows-msvc/release/kagami_qa.lib   (3,780,368 bytes) ✅
```

**C-b｜产物路径错误（缺 target triple 段）**
`src/rust/.cargo/config.toml:2` 设了 `[build] target = "x86_64-pc-windows-msvc"`，cargo 写入 `target/<triple>/release/`，而 CMake `:661` 写的是 `target/release/`（无 triple 段）。注意 `src/rust/CMakeLists.txt:36` **写对了**——照抄那份即可。

**C-c｜系统库完全缺失**（即 N-2）
`cargo rustc -- --print native-static-libs` 输出：
```
kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:libcmt
```
`:663-664` 意图用两个**未定义变量**提供它们 → 展开为空 → Rust std 必然 LNK2019。MSVC 只自动链 `kernel32`。

**C-d｜CRT 不匹配（/MT vs /MD）**
`.cargo/config.toml:5` 的 `target-feature=+crt-static` 让 Rust 请求 `libcmt`（静态 CRT），而 CMake 未设 `MSVC_RUNTIME_LIBRARY`，C++ 侧默认 `/MD`（`msvcrt`）→ 经典 LNK4098。**注意**：`CMakeLists.txt:68` 已有 `add_link_options(... /IGNORE:4098)`，注释明写「LNK4098 LIBCMT conflict in test linking」——项目早就撞过并压制了它，且 `fceux11_rust.lib` 今天就在同样的不匹配下正常链入主程序。所以这是**已被容忍的既有状态**，不是新增阻塞（LNK2038 不会触发，因为 Rust 不发 `/FAILIFMISMATCH` 指令）。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **C-1 ✅ `b2f92f7`** | 产出 staticlib：`crate-type = ["rlib","staticlib"]`（保留 rlib 供根 crate 依赖）**或**在 CMake 用 `cargo rustc --crate-type staticlib`（无需改 Cargo.toml） | `src/rust/crates/kagami-qa/Cargo.toml:7` 或 `tests/CMakeLists.txt` | 产出 `kagami_qa.lib` |
| **C-2 ⊘ 被 C-1 吸收** | 修正产物路径，补 triple 段；复用 `src/rust/CMakeLists.txt:36` 的既有写法 | `tests/CMakeLists.txt:661` | 走推荐解法后不再有独立 cargo 产物，路径问题消失 |
| **C-3 ✅ `362fcf3`** | 删除两个未定义变量，改为字面量 `ntdll userenv ws2_32 dbghelp`；补 `fceux11_drivers_common` 与 `src/`、`src/drivers/`、`src/drivers/null` 头文件目录（对齐 `fceux11_add_headless_test_executable`，`tests/CMakeLists.txt:47-73`） | `tests/CMakeLists.txt:663-664` | 无 LNK2019 |
| **C-4 ⊘ 被 C-1 吸收** | 消除 Rust std 符号重复风险（见下方「首要风险」） | `src/rust/Cargo.toml`、`tests/CMakeLists.txt` | 推荐解法（复用根 crate 唯一 staticlib）从根上排除 LNK2005，无需去重 |
| **C-5 ✅ `362fcf3`** | 依赖关系改为增量正确形式：`add_custom_command(OUTPUT ... DEPENDS ...)` + `add_custom_target(... DEPENDS ...)`，替换裸 `add_custom_target`；统一 `CARGO_TARGET_DIR` 到 `src/rust` 用的那个 | `tests/CMakeLists.txt:637-643` | 独立 cargo invocation 已整体删除，两处 target dir 合一 |

> **C-1 实测补充（计划外发现）**：cargo 生成 staticlib 时**不会**把 rlib 依赖中未被根 crate 引用的
> 符号带出来 —— 即使 feature 已开启且 LTO 生效。因此必须在根 crate 侧写 `#[no_mangle]` 包装函数
> 再导出，`b2f92f7` 即此实现。计划 §七 推测「加个 feature flag 即可」低估了这一层。

### C-4：首要残余风险（LNK2005 重复 std）

`src/CMakeLists.txt:713` 把 `fceux11_rust.lib` 以 `PUBLIC` 链入 `fceux11_utils`；而 `kagami_qa_direct_runner` 链接 `fceux11_utils`（`tests/CMakeLists.txt:659`）。若再单独链一个 `kagami_qa.lib`，就会引入**两份 Rust staticlib**，每份都自带完整 `std`，且根 crate `fceux11-rust` 本身就依赖 `kagami-qa`（`src/rust/Cargo.toml:29`）——相同 `-Cmetadata` 哈希意味着符号名完全相同。

**推荐解法（可完全绕开该风险）**：不产出独立的 `kagami_qa.lib`，改为给根 crate `fceux11-rust` 加 `direct-adapter = ["kagami-qa/direct-adapter"]` feature，复用**既有**的那一个 staticlib。

**强证据表明这本就是原设计**：`src/rust/fceux11_rust.h:3976` **已经声明了** `kagami_qa_direct_main`——cbindgen 早就把它导出到聚合头文件里了。采用此解法后，C-1/C-2 可简化为「加一个 feature flag」，整个 Phase C 收敛为「1 个 feature + 4 个系统库」。

**建议**：C-1 先按推荐解法实现；若 feature 传递遇阻，再回落到独立 staticlib 方案并在 C-4 处理符号去重。

### 其他残余风险

- **`FCEUX11_RUST_ENABLED` 在 `tests/` 作用域不可见**：它在 `src/rust/CMakeLists.txt:55` 以 `PARENT_SCOPE` 设置，只到 `src/` 层，因此 `tests/CMakeLists.txt:18,58` 的 Rust 分支**从不触发**。当前无害（库经 `fceux11_utils` 的 `PUBLIC` 传递而来），但是个潜在陷阱。`:633` 处用的 `FCEUX11_ENABLE_RUST` 是 cache 变量，全局可见——统一用它。
- **CI 将新增门禁**：`kagami_qa_direct_runner` 没有 `EXCLUDE_FROM_ALL`，`ctest -LE perf` 也不过滤 `kagami_qa_direct_smoke`。一旦链接成功，**`ci.yml` 就开始把它当门禁跑**——而 `ci.yml` **没有** Rust 工具链安装步骤（只有 `kagami-qa.yml:44-48` 有），且 `FCEUX11_ENABLE_RUST` 默认 `ON`、缺 rustup 时 configure 直接 fatal（`src/rust/CMakeLists.txt:13-15`）。目前 CI 靠 runner 预装的 Rust 侥幸存活。**必须在 Phase D 给 `ci.yml` 补显式工具链步骤**（D-3）。
- **运行时行为未验证**：以上全部关于**链接**。`kagami_bridge_*` 是否真能在进程内正确驱动模拟器、direct 模式输出是否与 subprocess 模式一致（这正是该测试的目的），仍需 C-5 后实测。

---

## 八、Phase D — Oracle B 覆盖率与 CI 权威性

**目标**：让权威性指标建立在真实数据上，而非废 matrix。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **D-1 ✅ `bc2875c`** | 实测 `blargg_suite` 实际驱动的 ROM 数，产出「177 个 ROM 的接入状态清单」 | `tests/tests.json`、`docs/` | 得到真实覆盖率分母/分子，取代「22/174」等口径不一的说法 |
| **D-2 ✅ `bc2875c` + `bc7c1d8`** | 按 D-1 清单扩充 manifest 接入，目标 ≥80% ROM 覆盖 | `tests/tests.json` | 已知失败的 ROM 显式标注为 expected-fail，不得静默跳过 |
| **D-3 ✅ `c36c4f4`** | 给 `ci.yml` 补 `dtolnay/rust-toolchain@stable` 步骤（对齐 `kagami-qa.yml:46`） | `.github/workflows/ci.yml` | 消除「靠 runner 预装 Rust 侥幸通过」的脆弱性 |
| **D-4 ✅ `c36c4f4`** | 打通 `kagami-qa.yml` 的 matrix 步骤，产出可信 migration matrix 并作为 CI artifact 上传 | `.github/workflows/kagami-qa.yml` | matrix 不再是本地手工产物；权威性指标可被 CI 复算 |

> **D-1 结论修订（S-1 `bc7c1d8`）**：D-1 把 3 个 manifest 条目判为「ROM 不在仓库、需重新下载」，
> 该结论**错误**。3 个 ROM 早已在磁盘上，只是用了 `download_blargg_roms.ps1` 的命名
> （`instr_v5_all` / `instr_v5_official` / `cpu_timing_test6`），且 manifest 中**已有对应的活条目** ——
> 那 3 条是指向不存在路径的**纯重复条目**。删除后 manifest 180 → 177，与磁盘完全一致，死条目归零。
> `blargg_cpu_instrs` / `blargg_cpu_timing` 已实测 PASS 并转回 `blocking`：真正压红它们的是
> **300 帧预算不足**（返回 `0x80` 仍在运行），不是缺 ROM。这同时解释了 E-2 记录的
> 「`official_only` 停在 4/16」。

**口径纪律**：`README.md` 与 `docs/tech/KagamiQA.md` 中的测试数量、覆盖率、权威性数值，必须在 D-4 完成后**统一由 CI 产物回填**，禁止手写。当前中英文行数字不一致（34 vs 39）的问题，根源就是手工维护。

---

## 九、Phase E — 精度遗留与收尾

**目标**：处理需要可复现构建才能推进的模拟精度问题。**必须在 Phase A 之后**——这些问题的调试离不开快速可靠的 Release 构建。

| PR | 内容 | 文件 | 验收 |
|---|---|---|---|
| **E-1 🔴 未收敛** | `ppu_vbl_nmi` 的 `02-vbl_set_time` 「VBL period too long #8」：P4-1 的 VBL cycle 0→1 调整疑似过冲，重新平衡周期计数 | `src/ppu_rendering.cpp:1547-1582` | `01-vbl_basics` 不回归的前提下 `02` 转 PASS |
| **E-2 ✅ `ab2ea5d`** | 补齐 blargg 诊断输出：`official_only.nes`（停在 4/16）与 `all_instrs.nes` 的 $6004 诊断区为空，需增加运行帧数或调整参数以捕获失败操作码 | `tests/blargg_runner.cpp` | 失败时能打印具体子测试名与操作码 |
| **E-3 🟡 仅完成实测分桶** | APU `1-len_ctr` 子测试 #7 + `src/sound.cpp:1095` 的 reset 寄存器 FIXME 联合排查（两者疑似同源） | `src/sound.cpp` | 至少产出根因结论；能修则修，不能修则记录为有依据的已知限制 |

### E-1 / E-2 / E-3 的现状（2026-07-30 实测，本表的题干已部分被推翻）

- **E-1**：题干描述的「周期过冲」**不成立** —— 插桩实测 VBL `period=6820`，398/398 帧完全一致。
  真正的失败机制是 **VBL/NMI 边沿相位与 CPU 观察点不对齐**，且 `vbl_02~10` 每个 ROM 对应
  **独立的时序参数**，不是单一线性偏移。已有两次失败尝试（1-cycle NMI delay 被证伪并 revert）。
  fresh 构建实测：`01/03/04/09 PASS`，`02/05/06/07/08 FAIL 0x01`，`10 FAIL 0x03`。
  完整记录见 `docs/FCEUX11-1.16_E-1-VBL调查记录.md` 与 `docs/e1_survey/`。
- **E-2**：已落地。runner 新增 `--reset-after` 与 double-sample 诊断。
  **附带发现**：`official_only` 停在 4/16 的真因是帧预算不足，非诊断区缺失（见 S-1）。
- **E-3**：题干「`1-len_ctr` 子测试 #7 fail」**不成立** —— `apu_01_len_ctr.nes` 当前整体 PASS。
  28 个 APU ROM 全量实测：15 PASS / 13 FAIL，13 项分 3 桶（runner reset 能力 / mixer 输出
  帧预算 / 具体子测试精度）。桶 A + 桶 B 共 8 项已由 E-2 的 `--reset-after` 解决，
  真剩余是桶 C 的 7 项精度问题（含 `$4017` write timing #2、`irq_flag` #6）。

### L3 / L4 的最终裁定

复核后**维持遗留文档的「不修复」结论**，但各补一处零成本加固：

- **L3（`main.rs:186` 空 config）**：`SubprocessAdapter::init()` 确为显式 no-op（`subprocess.rs:31-33` 参数名前缀 `_`），trait 契约论证成立。但传空 config 是**语义噪声**——未来若有 adapter 真需要 init，此处是隐蔽的坑。建议在 E 阶段顺手改为传真实 config 的 clone（1 行，零风险），或至少加注释说明为何可以传空。
  > **✅ 已落地（S-2）**：`adapter.init(&config)` 上移到 `TestScheduler::new(config, ...)` 之前，
  > 直接传真实配置，占位函数 `scheduler_config_default()` 整体删除（`QaConfig` 已 derive `Clone`，
  > 但调序后连 clone 都不需要）。
- **L4（`CALL_PPUREAD` 不判空）**：fail-fast 设计论证成立，`PPU_ResetHooks()`（P4-bridge `02db484`）确保上电后指针非 NULL，热路径加判空确实是纯开销。建议加 `assert(FFCEUX_PPURead)`（Debug 构建生效、Release 零成本），把「刻意的 fail-fast」变成「有诊断信息的 fail-fast」——崩在 assert 上比崩在空指针解引用上好排查得多。
  > **✅ 已落地（S-2）**：`src/ppu.cpp` 与 `src/ppu_rendering.cpp` 两处宏定义统一改为
  > `(assert(FFCEUX_PPURead != nullptr), FFCEUX_PPURead(A))`。逗号表达式保持宏的**表达式语义与返回类型**
  > 不变（全部 10 处调用点都是简单赋值），`NDEBUG` 下 `assert` 展开为 `((void)0)` → Release 零成本。

> ⚠️ 需与 `docs/tech/null_pointer_defects_v1.15_audit.md` 的 5 项空指针缺陷区分：那些覆盖 `currCartInfo` / `XBackBuf` / `GameInfo->type`，与 L4 的论证**不冲突**，且各自只需 1 行 guard。它们被标记为 v2.0，本计划**不纳入**，但若 Phase E 有余量可低成本顺带处理。

---

## 十、验收标准（Stage-2 完成定义）

| # | 标准 | 验证方式 | 状态（2026-07-30 实测） |
|---|---|---|---|
| 1 | 裸 PowerShell 下 `.\scripts\do_build.ps1 -Config Release` 一次成功 | 无 C1041 / LNK1104；自动选 Ninja | ✅ 自动选中 BuildTools 内置 Ninja；`C1041/LNK1104/LNK2019/LNK2005` 命中数 **0** |
| 2 | 本地 `ctest -LE perf` 与 CI 结果一致 | 逐项比对 | ✅ `ctest` **34/34 = 100%**（build-c1, Release） |
| 3 | `lua_bit_test_headless` PASS | Phase B | ✅ PASS |
| 4 | `kagami_qa_direct_smoke` PASS（不再 `Not Run`） | Phase C | ✅ PASS（S-2 修好帧预算奇偶后 6.99s） |
| 5 | migration matrix 由 CI 产出且 passed 率有据可查 | Phase D | 🟡 本地 **35/39**、`git_rev=623dd39` 可追溯（S-4）；CI 侧 workflow 已就绪但本会话未触发 |
| 6 | Oracle B ROM 覆盖 ≥80%，失败项显式标注 | Phase D | ✅ **177/177 = 100%**（manifest 与磁盘 1:1，死条目 0）；失败项带 `$6000` 码与分类 tag |
| 7 | README / KagamiQA.md 的数字由 CI 产物回填，中英一致 | Phase D | ✅ `bc72b20` 已统一 |
| 8 | 遗留文档的 3 处过期记载已更正 | Phase 0 | ✅ `b53a7a0` |
| 9 | **判定链路与 schema 声明一致**：`stdout_contains` 生效、超时生效、`regression.rs` 在调用链上 | Phase 0.5；负向用例（退出码 0 但 stdout 不符）必须判 FAIL | ✅ `36bd311` + `7bf7771`；`cargo test -p kagami-qa` **40/40** |
| 10 | **`fail_to_pass` 不含新增测试** | Phase 0.5；新增通过用例落入 `new_test` 桶 | ✅ `dfe0710` + `39c758b` |
| 11 | **权威性按修订口径分离陈述**，不再输出单一乘积分数 | 见 §十·五 | ✅ 见下方「收官口径」 |

**Oracle A 目标**：33 项中 33 PASS（当前 31 PASS / 1 FAIL / 1 Not Run）。Phase B 解决 FAIL，Phase C 解决 Not Run。
→ **已达成**：`ctest` 34/34 全 PASS。

### 收官口径（2026-07-30，按 §十·五 的「门槛 + 度量分离」陈述）

```
【卫生门槛】
  ☑ Oracle A/B 判定通道物理隔离
  ☑ 判定逻辑与 manifest schema 声明一致        （0.5-1 / 0.5-2）
  ☑ 迁移矩阵不含结构性失真                      （0.5-3 / 0.5-4）
  ☑ 产物可追溯：matrix 带真实 git_rev            （S-4；此前恒为 "unknown"）
  ☐ CI 常驻、指标由 CI 产物回填                  （workflow 已就绪，尚未在 CI 上实跑一轮）

【权威性度量】
  外部真理覆盖率 = 177 / 177 blargg ROM（manifest 与磁盘 1:1，死条目 0）
  已知失败清单   = 迁移矩阵 39 项中 4 项 FAIL，全部带分类：
                   · blargg_ppu_vbl_nmi  E-1 未收敛，$6000=0x01，逐 ROM 诊断已归档
                   · blargg_suite        聚合项，含 E-1/E-3 的已知失败
                   · lua_joypad_test / lua_memory_test  unimplemented-coverage
  oracle 来源数  = 1（blargg）—— 覆盖率已到顶，继续提升权威性必须引入新来源
```

**`blargg_suite` 全量实测（177 ROM，`git_rev=623dd39`）：121 PASS / 56 FAIL**，
56 项按 `$6000` 分类后有一个关键结论：

| `$6000` | 数量 | 性质 |
|---|---|---|
| `0x80`（仍在运行，帧预算不足） | 12 | 🔧 harness 问题 |
| `0x81`（"Press RESET"，批处理路径未传 `--reset-after`） | 6 | 🔧 harness 能力缺口 |
| 具体子测试失败码 | 38 | ⚠️ 真实精度问题 |

> **56 项里有 18 项（32%）不是模拟精度缺陷**，而是与 S-1 同类的「喂错参数」——
> 真实精度待修面是 **38 项**，不是 56 项。这个区分必须保留在任何对外数字里，
> 否则会高估缺陷面并淹没真问题。

> **注意**：覆盖率 177/177 指「全部磁盘 ROM 都被 `blargg_suite` 驱动」，**不等于**全部 PASS。
> 按 §十·五 保留的判断：「精确知道什么失败，比『全绿但不测』更权威。」

---

## 十·五、权威性口径修订

现行公式（`docs/tech/KagamiQA.md` §1.4、P5 计划 §1.1）：

```
权威性 = ROM覆盖率 × Oracle独立性 × CI常驻因子
```

**建议在 Stage-2 内废止这个单一乘积分数**，理由有二：

### 理由一：它已被证明可被自评膨胀 4 倍

同一份代码、同一天，文档自评 `1.00 × 1.00 × 1.00 = 1.00`，审计复评 `1.00 × 0.50 × 0.50 = 0.25`。三个因子里只有 ROM 覆盖率是测量量（`tested_roms / 174`），另两个是二元自评。**一个作者能自己写成 1.00 的分数，不是权威性度量，是自检清单。**

### 理由二：它把卫生条件当成了权威来源

真正承载权威性的**只有 Oracle B**——因为只有它对照外部真理（真实 NES 硬件，经 blargg ROM 的 `$6000` 协议中介）。Oracle A 全套（`mapper_byte_diff` / `apu_wav_diff` / `ppu_frame_diff` / `golden_savestate`）本质是 characterization testing，只能回答「和上一版一样吗」，**永远无法回答「对吗」**。PLAN §2.4 自己写得很清楚：「无一个 oracle 回答『与真实硬件是否一致』」。

而「Oracle 独立性」和「CI 常驻」是**卫生条件**：不满足则结论无效，满足了也不增加真理含量。把 CI 常驻算作权威性的三分之一，等价于主张「跑得勤 = 更接近真理」——这不成立。乘积形式还有个副作用：任一因子为 0 则总分为 0，这掩盖了「其余部分其实已经做好」的事实，无法反映渐进改善。

### 修订口径：门槛 + 度量分离陈述

```
【卫生门槛】（二元，不满足则以下度量无效）
  □ Oracle A/B 判定通道物理隔离
  □ 判定逻辑与 manifest schema 声明一致      ← Phase 0.5 新增
  □ 迁移矩阵不含结构性失真                    ← Phase 0.5 新增
  □ CI 常驻，指标由 CI 产物回填而非手写

【权威性度量】（仅在全部门槛满足时有意义）
  外部真理覆盖率 = 已接入的 blargg ROM / 177
  已知失败清单   = 每条含错误码、诊断串、分类标记
  oracle 来源数  = 当前 1（blargg）
```

### 保留原设计中正确的部分

KagamiQA.md §1.4 的这句判断应当保留并前置：

> **「权威性不要求 Oracle B 全部 PASS。精确知道什么失败，比『全绿但不测』更权威。已知失败清单本身就是防线的一部分。」**

这是成熟的工程认识论——把已知失败清单当作防线的组成部分，而非污点。修订口径把它落成了可检查的字段要求（每条已知失败必须带错误码、诊断串、分类标记），而不只是一句态度。

### 关于「oracle 来源数」这一新增项

当前 `oracle 来源数 = 1`。这意味着 **KagamiQA 的权威性上限 = blargg ROM 套件对真实硅片的保真度**——ROM 覆盖率从 13% 提到 100%，也只是把这一个来源用尽，不会突破它。若未来要继续提升权威性，路径是**增加相互独立、可以彼此证伪的 oracle 来源**（NESdev 其他测试套件、TASVideos 精度表、第二个模拟器的差分比对、真机采集），而非继续增加同一来源的测试数量。此项列入度量是为了让这个天花板**在指标上可见**，避免用覆盖率的增长掩盖来源的单一。

---



诚实标注本计划中**尚未实测验证**的部分（**2026-07-30 收官回填：多数已转为实测结论**）：

| 风险 | 等级 | 说明 | 收官判定 |
|---|---|---|---|
| C-4 的 LNK2005 重复 std | 🔴 高 | 这是唯一未能通过只读手段排除的链接风险（未运行链接器）。推荐解法（复用根 crate staticlib）可绕开，但需实测 | ✅ **已排除**。推荐解法落地后 `LNK2005` 命中数 0；C-4 无需单独实施 |
| direct 模式的**运行时**正确性 | 🟡 中 | Phase C 全部论证只覆盖链接成功。进程内驱动模拟器是否与 subprocess 模式等价，完全未验证 | ⚠️ **实测发现并修复一处不对等**：direct 模式无视 manifest 的 `--frames`（恒 300 帧），见 S-2。修复后 4 条 Oracle B 用例 direct 与 subprocess 结论一致 |
| E-1 VBL 周期调整 | 🟡 中 | 精度问题存在「修好一个测试、弄坏另一个」的经典风险，必须回归全部 `ppu_vbl_nmi` 子测试 | 🔴 **风险已兑现**：1-cycle NMI delay 尝试使 `05` 的行 00-01 转好、02-07 仍错，证明不是单参数问题，已 revert |
| A-3 `/Z7` 的产物膨胀 | 🟢 低 | 嵌入式调试信息会增大 .obj；对 `fceux11_core.lib` 中 512 KiB LUT 的影响需实测确认可接受 | ✅ 可接受：`fceux11_core.lib` 102 MB、构建与 ctest 全程正常 |
| c2.dll bug 的真实触发条件 | 🟢 低 | A-6 是基于「可能泛化」的保险，成本近零。若上游 MSVC 修复，`/GL-` 变成无害的历史包袱 | 维持原判 |
| Phase D 的覆盖率目标 | 🟡 中 | 「≥80%」继承自 P5 计划，但在真实分母（D-1）产出前，该目标是否现实未知。D-1 后可能需要下调 | ✅ **无需下调**：实际 177/177 = 100%（清掉 3 个重复死条目后 manifest 与磁盘 1:1） |
| **历史结论的追溯污染** | 🟡 中 | 0.5-d（`fail_to_pass` 灌水）自 matrix 实现之日起就存在。此前所有基于迁移矩阵得出的「本次修复了 N 项」结论都可能虚高。Phase 0.5 完成后应重算一次历史基线，但**已发布文档中的历史声明无法追溯更正**——只能在 KagamiQA.md 加注说明该口径变更 | 维持：新基线已用 `ceed00e` 后的 runner 重算并带 `git_rev` |

> **收官新增的一条风险认识**：产物**可追溯性**本身就是一道防线。S-4 之前 `git_rev` 恒为
> `"unknown"`，导致一份两天前的废 matrix 与新产物在字段上完全无法区分 —— 本次开工时正是被它
> 误导。凡是要拿来做判断的产物，都必须能自证其来源 commit。


**上游动作（异步，不阻塞任何 Phase）**：向 Microsoft Developer Community 提交 c2.dll 的 LTCG 崩溃报告。最小复现已具备（512 KiB constexpr array + lambda init + `/GL` + `/LTCG`，MSVC 14.51.36231），见遗留文档 §2.4。这是未来能移除 `/GL-` 的唯一路径。

---

## 十二、与既有文档的关系

- 本文档**取代**遗留文档中 §1.3、§1.4、§2.1、§2.2 四节的结论部分（其现象记录仍然有效且有价值）。
- 遗留文档 §1.1、§1.2、§2.3、§2.4 的结论**予以维持**，本文档仅补充加固措施。
- Phase 0-3 完成后，遗留文档应加一段前言，指向本文档作为后续处置方案。
- **后续路线**见 `docs/FCEUX11-Stage3-权威性迭代与通用化路线.md`——回答「能否迭代趋于权威、再追求通用化」。其中 §3.2 的「冻结泄漏纪律」**在 Stage-2 期间即生效**：不再向共享 schema 添加领域专用字段、不再向 `SutAdapter` 添加方法。该纪律零成本，但决定了未来抽象重构的可行性。
- 未纳入本计划的开放项（v2.0 清理项 E 系列、i18n 债务 H 系列、GUI/movie 层 TODO F 系列、5 项空指针缺陷 D 系列）已在调研中完整清点，建议单独立项，不塞进 Stage-2 以免超出注意力窗口。
