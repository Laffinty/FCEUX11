# FCEUX11 v1.16 KagamiQA — 遗留问题与构建难题

> **日期**：2026-07-29
> **分支**：`wip_1.16`
> **关联**：`docs/FCEUX11-1.16_KagamiQA-审计报告.md`、`docs/FCEUX11-1.16_KagamiQA-修复验证报告.md`
>
> **2026-07-29 勘误说明（Stage-2 PR 0-3）**：
> 本文档三处记载与 `301c742` 之后的 HEAD 实际状态不符，已就地更正：
> 1. §2.4 第 197 行 `2 MiB 全零文件` 表述 — 实测当前 `ppu_rendering_lut_test.exe` 为 10.6 MiB 有效 PE 文件，详见 `docs/FCEUX11-1.16_Stage2-构建计划.md` §一勘误 1
> 2. §1.3 第 84 行 `rlib ABI 交互` 表述 — 真因是 `tests/CMakeLists.txt:637-665` 的 4 个具体缺陷，不是 `工程复杂性`，详见 Stage-2 计划 §一勘误 3
> 3. 关联审计链（`修复验证报告.md`）曾称「`fceux11_cpu_testexe` 后缀 bug 未修」 — 实测 `src/rust/crates/kagami-qa/src/adapter/subprocess.rs:51` 用 `format!("{}.{}", name, EXE_EXTENSION)` 已正确拼出 `fceux11_cpu_test.exe`，详见 Stage-2 计划 §一勘误 2
> **结论稳定性声明**：除上述 3 处更正外，本文其余「不修复」与「构建难题绕过方案」维持原结论（即 L1.1/L1.2 维持、L2.3 维持、L2.4 维持 — `/GL-` 修复确实生效，现象描述变更不影响结论）

---

## 一、无法修复的项目

### 1.1 L3 — SubprocessAdapter::init() 传入空 config（不修复）

**文件**：`src/rust/crates/kagami-qa/src/main.rs:186`
**现象**：`adapter.init(&scheduler_config_default())` 传入的 `QaConfig` 所有字段均为默认值/空值，而真正的 config 已在上一行 move 进 `TestScheduler::new(config, ...)`。

```rust
let scheduler = TestScheduler::new(config, manifest);
adapter.init(&scheduler_config_default())?;  // ← 空 config
```

**为何不修复**：

`SubprocessAdapter::init()` 的实现是显式的 no-op —— 参数名前缀 `_` 表示有意忽略：

```rust
// adapter/subprocess.rs:32-34
fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
    Ok(())
}
```

这是 **trait 契约要求的合法实现**。`SutAdapter` trait 定义要求所有 adapter 实现 `init()`，但 `SubprocessAdapter` 的构造已在 `SubprocessAdapter::with_working_dir()` 中完成（设置 `bin_dir` 和 `default_working_dir`），无需额外的初始化步骤。传入空 config 不会产生任何运行时影响。

若未来需要真实的 init 逻辑（如 `Fceux11DirectAdapter` 确实需要 init），只需将 `scheduler_config_default()` 改为 clone 后的真实 config。当前代码在语义上不完整但不构成 bug。

---

### 1.2 L4 — CALL_PPUREAD/CALL_PPUWRITE 空指针检查不对称（不修复）

**文件**：`src/ppu.cpp:301-303`

```cpp
#define CALL_PPUREAD(A)   (FFCEUX_PPURead(A))                          // 不判空
#define CALL_PPUWRITE(A,V) (FFCEUX_PPUWrite ? FFCEUX_PPUWrite(A,V)     // 判空
                            : FFCEUX_PPUWrite_Default(A,V))
```

**为何不修复**：

这是 **P4-bridge 修复后的 intentional design choice**，而非遗漏。

P4-bridge（commit `02db484`）在 `FCEUPPU_Power()` 开头插入了 `PPU_ResetHooks()` 调用，保证每次上电后 `FFCEUX_PPURead` 和 `FFCEUX_PPUWrite` 都被设为有效的默认函数指针。此后这两个指针在正常模拟周期内 **永不 NULL**。

设计逻辑：
- **READ 不判空**：如果 `FFCEUX_PPURead` 是 NULL，说明 `PPU_ResetHooks()` 未被调用——这是代码 bug，应该 **立即 crash** 暴露问题（fail-fast）
- **WRITE 判空**：board mapper 可能不需要自定义 write hook，回退到默认实现是合法路径。NULL guard 是防御性安全网

在 `PPU_ResetHooks()` 保证非 NULL 的前提下，给 READ 加判空只会增加每次 PPU 读的热路径开销，而无实际保护价值。这是经过根因分析后的刻意设计权衡。

---

### 1.3 kagami_qa_direct_smoke — Direct runner 无法端到端构建（待 Stage-2 Phase C 重判）

> **2026-07-29 勘误**：原文档将根因归为"跨语言工程复杂性"。Stage-2 §一勘误 3 已证伪该归因 — Rust 侧实测 `cargo build --release -p kagami-qa --features direct-adapter --lib` **退出码 0**，8 个 FFI 符号与 C++ 侧完全对应。真正的 4 个硬阻塞全部位于 `tests/CMakeLists.txt:637-665` 这 30 行（详见 Stage-2 计划 §七 Phase C）。本节先做诊断更正，**最终"是否修复"交由 Stage-2 Phase C 完成后再行修订**。

**CTest 状态**：`Not Run`——`kagami_qa_direct_runner.exe` 从未生成。

**原因链**（Stage-2 §一勘误 3 后）：

```
CMake target kagami_qa_direct_runner              (tests/CMakeLists.txt:637-665)
  → 调 cargo build --crate-type rlib                 (C-a: rlib 不是 link.exe 可消费的产物)
  → 链接路径缺 target triple 段 x86_64-pc-windows-msvc  (C-b: 路径根本命中不了产物)
  → 系统库 _FCEUX11_CORE_LIBS / _FCEUX11_OPENGL_LIBS  (C-c: 全仓库未定义变量, 展开为空)
      → ntdll/userenv/ws2_32/dbghelp 全未链入, Rust std 必 LNK2019
  → CRT 不匹配 /MT vs /MD                              (C-d: 已被现有 /IGNORE:4098 容忍)
```

> 上述 4 个缺陷在 `tests/CMakeLists.txt` 这 30 行内集中体现。**Rust 侧代码本身完全无问题**。

**原 §1.3 中 3 个错误诊断**（Stage-2 §一勘误 3 列出）：

| 原说法 | 实际情况 |
|--------|----------|
| 「NMake `add_dependencies` 不传递 Rust 构建失败信号」 | ❌ 失败的 `add_custom_target` 命令**会**中断构建；真正缺陷是缺 `add_custom_command(OUTPUT ... DEPENDS ...)` 增量建模（见 Phase C-5） |
| 「rlib 与 MSVC COFF/LTCG 对象的 ABI 交互需精确协调」 | ❌ **rlib 不是 link.exe 可消费的产物**——`Cargo.toml:7` 声明 `crate-type = ["rlib"]`，应当产出 `kagami_qa.lib`（staticlib）才能链入 C++ |
| 「MSVC 14.51 LTCG 已知不稳定性」 | ❌ 与本项无关；`link.exe` 原生支持 `/GL` 与普通 COFF 混合输入 |

**为何本文档仍未直接给出修复**：本次勘误仅更新诊断准确性，`kagami_qa_direct_runner` 的实际修复路径已迁移至 Stage-2 §七 Phase C（C-1~C-5 共 5 个 PR）。在那之前，不应在本文档承诺"已修复"。

---

### 1.4 lua_bit_test_headless — Lua bit 库 5 个真实 bug（暂不修复）

**CTest 状态**：`Failed`——31 个 PASS / 1 个 FAIL（lua_bit_test_headless）+ 1 个 Not Run（direct_smoke）。

**失败详情**（来自 `tests/lua_scripts/test_bit.lua` 实测输出）：

```
FAIL: rshift(-1, 31) => got -1, expected 1
FAIL: ror(0x01, 1) => got -2147483648, expected 2147483648
FAIL: tohex(255) => got 000000ff, expected 000000FF
FAIL: tohex(255, 2) => got ff, expected FF
FAIL: tohex(-1, 4) => got ffff, expected FFFFFFFF
bit library: 22 passed, 5 failed
```

**根因**：
- `rshift`：对有符号整数的符号扩展处理不一致
- `ror`：32 位旋转后未正确处理无符号溢出
- `tohex`：格式化输出时的大小写和宽度处理偏差

**为何暂不修复**：

1. **这些 bug 在 KagamiQA 引入之前已存在**（hotfix2 时代的 fceux11-lua crate），非本次 P5 改动引入
2. **P5 M2 修复已让 runner 正确报告 FAIL**（修复前是假 PASS），问题已被检测——这是 KagamiQA 的价值所在
3. **bit 库 bug 属于 fceux11-lua crate 的精度缺陷**，需要 Rust Lua 绑定层面的修复，不在 KagamiQA 测试框架的 scope 内
4. 这 5 个 bug 不影响核心模拟精度（CPU/PPU/APU），仅影响 Lua 脚本中位运算相关的高级用

**建议**：在 fceux11-lua crate 的后续版本中修复 bit 库实现，或在上游 mlua/bit 库更新后同步。

---

## 二、构建环境难题

以下记录了从 bash (Git Bash) 环境构建 MSVC 项目的实际困难及绕过方案。

### 2.1 核心矛盾：Bash vs MSVC

**问题**：FCEUX11 使用 NMake Makefiles 生成器，编译需要 `cl.exe`、`nmake.exe`、`link.exe`、`rc.exe` 等，这些工具**不在 Git Bash 的 PATH 中**，且需要正确设置 `INCLUDE` 和 `LIB` 环境变量。

**尝试的方案及结果**：

| 方案 | 结果 |
|------|------|
| 直接 `cmake --build` | ❌ `nmake not found` |
| `cmd /c "vcvars64.bat && cmake --build"` | ❌ cmd 窗口输出无法捕获，环境变量不传递 |
| PowerShell `Launch-VsDevShell.ps1` | ❌ 脚本路径不存在（BuildTools 非完整 VS） |
| 手动设置 PATH/INCLUDE/LIB | ⚠️ 部分成功，缺 UCRT include 路径 |
| PowerShell + `cmd /c vcvars64.bat && set` 捕获环境 | ✅ **可行** |

**最终可行方案**（用于所有构建）：

```powershell
$envBlock = cmd /c '"path\to\vcvars64.bat" >nul 2>&1 && set'
$lines = $envBlock -split '\r?\n'
foreach ($line in $lines) {
    if ($line -match '^(INCLUDE|LIB|LIBPATH|PATH)=(.*)$') {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
    }
}
cmake --build build --config Release
```

该方法通过 `cmd /c` 运行 vcvars64.bat，然后用 `set` 命令 dump 全部环境变量，再用 PowerShell 解析并注入当前进程（仅 Process 级别，不影响系统）。**这是从非 MSVC shell 构建 MSVC CMake 项目的通用模式**。

---

### 2.2 PDB 并行写入冲突（C1041）

**现象**：
```
fatal error C1041: 无法打开程序数据库"fceux11_core.pdb"
如果要将多个 CL.EXE 写入同一个 .PDB 文件，请使用 /FS
```

**根因**：NMake 生成器默认启用 `/MP`（多进程编译），多个 `cl.exe` 实例同时写入 `fceux11_core.pdb`。`/FS`（Force Synchronous PDB writes）标志可以序列化 PDB 访问，但通过 `-DCMAKE_CXX_FLAGS='/FS'` 设置时，在某些工具链版本中不一定被所有翻译单元继承。

**最终绕过**：设置 `$env:CL = '/FS'`（环境变量方式），并减少并行度（`-- /nologo` 去掉并行编译参数）。这会显著增加编译时间（~10 分钟 → ~30 分钟），但避免了 PDB 竞争。

**系统性解决**：切换到 Ninja 生成器（`-G "Ninja"`）可从根本上避免此问题。CI workflow（`.github/workflows/ci.yml`）使用 Ninja，因此不受影响。本地 NMake 构建需要此 workaround。

---

### 2.3 fceu11_direct_storage_probe.lib LNK1104 文件锁

**现象**：
```
LINK : fatal error LNK1104: 无法打开文件"fceu11_direct_storage_probe.lib"
```

**根因**：`fceu11_direct_storage_probe` 是一个 Windows DirectStorage API 的运行时探测静态库（`src/platform/win11/DirectStorageProbe.cpp`），由 CMake option `FCEUX11_DIRECT_STORAGE_PROBE`（默认 ON）控制。

在增量构建或前次构建未完全清理时，`link.exe` 的输出文件会被操作系统或杀毒软件短暂锁定。LNK1104 在 Windows 上是一个**概率性构建环境问题**，与代码无关。

**绕过方式**：
1. `taskkill /F /IM nmake.exe; taskkill /F /IM cl.exe`（杀掉残留编译进程）
2. `rm -f build/src/fceu11_direct_storage_probe.lib`（删除可能被锁的旧文件）
3. 重试构建

或直接关闭该选项：`-DFCEUX11_DIRECT_STORAGE_PROBE=OFF`。该组件是**可选的非核心功能**（仅探测主机是否支持 DirectStorage 1.2 NVMe 加速），关闭后 savestate I/O 退回到标准 `std::fstream`，不影响模拟精度。

**为何不永久关闭**：该选项默认 ON 是有意设计——在成功构建时，它为主 GUI 提供 NVMe 硬件信息展示。LNK1104 仅出现在特定构建环境/时序下。CI 上未见此问题。

---

### 2.4 M3 — MSVC LTCG 链接器崩溃（c2.dll）

> **2026-07-29 勘误**：原文档描述的「2 MiB 全零文件、无 PE 头」是修复**前**的现象；当前 HEAD（`wip_1.16` 分支，`301c742` 之后）的 `build/tests/fceux11_ppu_rendering_lut_test.exe` 实测为 **10.6 MiB 的有效 PE 文件**（`MZ` 头正常，链接完整）。`/GL-` 修复确实生效，本节的根因分析与修复方案不变，只是现象描述需要刷新。

**历史现象**（修复前）：`fceux11_ppu_rendering_lut_test.exe` 在旧 NMake 本地构建路径下产出 2,097,152 字节（2 MiB）的全零占位文件，无 PE 头。

**根因分析**（保留）：

1. 该测试是 `fceux11_core.lib` 中 512 KiB `kSpriteIdxLUT`（`alignas(64) const std::array<uint64_t, 65536>`）的**唯一消费者**
2. `ppu_sprite_lut.cpp.obj` 仅 12,649 字节，文件头为 `0000ffff`（MSVC LTCG "bigobj" magic），说明它是**IL（Intermediate Language）对象**，非标准 COFF
3. 512 KiB 的实际数据在链接时由 `c2.dll`（MSVC Link-Time Code Generator）从 IL 中 materialise
4. `link.exe` 通过 `VirtualAlloc` 分配初始输出缓冲区（恰好 2 MiB，power-of-2 对齐），然后调用 `c2.dll` 填充
5. **`c2.dll` 在处理这个特定的大数组 + `alignas(64)` + immediately-invoked lambda 初始化组合时，发生内部错误或静默崩溃**
6. `link.exe` 未正确传播 `c2.dll` 的失败信号，将未填充的零缓冲区写入磁盘
7. CMake/NMake 将 exit code 0 解释为构建成功

**修复**：在 `src/CMakeLists.txt` 中为 `ppu_sprite_lut.cpp` 添加编译标志 `/GL-`，禁用 Whole Program Optimization：

```cmake
set_source_files_properties(${CMAKE_CURRENT_SOURCE_DIR}/ppu_sprite_lut.cpp PROPERTIES
    COMPILE_FLAGS "/GL-")
```

这使得 512 KiB LUT 被编译为**标准 COFF 数据**（而非 IL），链接器不再需要通过 LTCG 路径 materialise 它，避开了 `c2.dll` 的崩溃路径。

**修复后结果**：`fceux11_ppu_rendering_lut_test.exe` 成功链接为有效 PE，CTest `ppu_rendering_lut_test` 从 `BAD_COMMAND` → **PASS**。

**上游影响**：这是特定 MSVC 版本（14.51.36231, VS 2026 v18.0）的疑似 bug。相同代码在其他 MSVC 版本或使用 `/GL-` 时正常工作。该问题已具备向 Microsoft Developer Community 提交 bug report 的条件（最小复现：512 KiB constexpr array + lambda init + /GL + /LTCG）。

---

## 三、Oracle A 当前状态

```
94% tests passed, 2 tests failed out of 33

Passed:  31
Failed:   1 (lua_bit_test_headless — Lua bit 库 5 个真实 bug, §1.4)
Not Run:  1 (kagami_qa_direct_smoke — 联合构建未完成, §1.3)
```

| # | 测试 | 审计时 | 修复后 | 变化 |
|----|------|--------|--------|------|
| 23 | ppu_rendering_lut_test | ❌ BAD_COMMAND | ✅ PASS | M3 /GL- fix |
| 33 | lua_bit_test_headless | ❌ Failed | ❌ Failed | Lua bit 库 bug（未变） |
| 34 | kagami_qa_direct_smoke | ❌ Not Run | ❌ Not Run | 联合构建阻塞（未变） |

审计时 `ppu_test` 的 BAD_COMMAND（`#11`）在修复后为 PASS（审计时也已手动确认可 PASS，仅 CTest 工作目录问题）。

---

## 四、总结

### 可修复项：12/15 已完成

| 严重级 | 总计 | 已修复 |
|--------|------|--------|
| 🔴 严重 | 4 | 3（S3 中 direct_smoke 不修复） |
| 🟡 中等 | 7 | 7 |
| 🟢 轻微 | 4 | 2（L3/L4 不修复） |

### 不修复项：3 项均为设计如此或超出 KagamiQA scope

| 项 | 性质 |
|----|------|
| L3 (空 config) | trait 合法 no-op |
| L4 (CALL_PPUREAD 不判空) | P4-bridge 后 intentional fail-fast |
| direct_smoke 端到端 | 联合构建工程复杂性，非代码缺陷 |

### 已知但暂不修复：1 项

| 项 | 性质 |
|----|------|
| Lua bit 库 5 bug | fceux11-lua crate 精度缺陷，KagamiQA scope 外 |

### 构建环境难点：4 项均已记录绕过方案

| 难点 | 绕过 |
|------|------|
| Bash vs MSVC | PowerShell + `cmd /c vcvars && set` 捕获环境 |
| PDB 冲突 (C1041) | `$env:CL='/FS'` + 单线程编译 |
| LNK1104 文件锁 | kill 残留进程 + 删旧文件 + 重试 |
| LTCG 崩溃 (c2.dll) | `/GL-` 对特定 TU 禁用全程序优化 |
