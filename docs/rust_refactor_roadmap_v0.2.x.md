# FCEUX11 Rust 渐进式重构路线图 v0.2.x

> **版本范围**：v0.2.2 → v0.2.11（共 10 个 Phase）  
> **目标**：以低风险、高确定性的方式，将 10 个自包含小模块逐步迁移至 Rust，验证混合语言架构的可行性，并为后续核心模块重构建立范式。  
> **制定日期**：2026-05-25

---

## 一、背景与设计理念

### 1.1 为什么渐进式重构？

FCEUX 是一个拥有 20 余年历史的 C/C++ 模拟器核心，其 PPU/APU/CPU 模拟循环属于**高频关键路径**，任何微小的行为偏差都会导致画面/音频异常或游戏崩溃。全面重写风险极高，也不符合项目维护节奏。

学术研究与工业实践（KDAB 2024、OpenHarmony His2Trans 2025、Linux Kernel Rust 驱动）均表明：

> **增量迁移（Incremental Migration）+ ABI 保留（ABI-Preserving）** 是大型 C/C++ 代码库引入 Rust 的最安全策略。

### 1.2 文献与先进实践支撑

| 来源 | 核心观点 | 本项目应用 |
|------|---------|-----------|
| **KDAB "Hybrid Rust/C++ Apps"** | 优先选择"自包含、接口干净、非模板/宏密集"的叶子模块作为首批目标。 | 首批模块全部来自 `src/utils/` 和独立 I/O 模块。 |
| **His2Trans (arXiv 2603.02617)** | "Skeleton-First" + `#[repr(C)]` 类型保留，确保混合构建链接成功、现有测试通过。 | 每个 Rust 模块先保持 C ABI 兼容，C++ 侧保留同名 wrapper，支持条件编译回退。 |
| **Linux Kernel Rust** | 内部 API 可自由演进，但跨语言边界必须使用稳定的 C ABI；Rust 的类型系统可减少意外的 ABI 破坏。 | 统一使用 `extern "C"` + `#[no_mangle]`；禁止在 FFI 边界使用 Rust 特有的 `String`、`Vec`、引用等类型。 |
| **Rust Embedded Book / Corrosion** | `staticlib` 是嵌入现有 CMake 项目的最轻量方式；保持"数据留在各自语言侧"，不跨边界传递所有权。 | Rust crate 类型维持 `staticlib`；Rust 不接管 C++ 分配的内存生命周期，仅做只读计算或返回新分配缓冲区。 |

### 1.3 模块筛选原则（风险由低到高）

```
入选标准 = 自包含 + 接口窄 + 非关键路径 + 无复杂C++特性 + 有Rust生态优势
排除标准 = 高频模拟循环内 + 大量全局状态交互 + 重模板/宏/虚函数 + 函数指针回调注册
```

**特别注意**：`src/boards/` 下的 Mapper 虽然代码行数少（如 VRC1 仅 63 行），但依赖 `mapinc.h` 的复杂宏系统（`DECLFW`、`SFORMAT`、`setprg8` 等）和函数指针注册机制，Rust 侧难以安全表达，且位于核心模拟路径。**不建议在 v0.2.x 阶段重构 Mapper**。

---

## 二、整体架构与集成方式

### 2.1 现有基础（v0.2.1）

- `src/rust/` 已建立独立 Cargo crate，`crate-type = ["staticlib"]`。
- CMake 通过 `add_subdirectory(rust)` 集成，生成 `fceux11_rust.lib`。
- 已有一个成功案例：`crc32fast` 替代了 `src/utils/crc32.cpp` 中的 zlib 计算，通过 `fceux11_rust_crc32()` C ABI 函数暴露。

### 2.2 集成范式："Wrapper-Shim + Feature-Gate"

每个模块重构遵循以下统一模式：

```
┌─────────────────┐      C ABI       ┌─────────────────┐
│   C++ Caller    | <--------------> |  Rust staticlib |
│  (原有代码不变)  |   (extern "C")   |  (新实现 + 优化)  |
└────────┬────────┘                  └─────────────────┘
         |
         ▼
┌─────────────────┐
│  C++ Wrapper    │  ← src/utils/xxx.cpp 保留，但内部 #ifdef FCEUX11_RUST_ENABLED
│  (同名函数转发)  |    时调用 Rust FFI；否则保持原 C++ 实现。
└─────────────────┘
```

**关键规则**：
1. **ABI 稳定**：FFI 函数使用 `#[no_mangle] pub extern "C"`；结构体使用 `#[repr(C)]`。
2. **零侵入**：C++ 调用方代码尽量不改，仅修改被重构模块的 `.cpp` 实现文件为 wrapper。
3. **可回退**：每个模块通过独立的 CMake option（如 `FCEUX11_RUST_MD5`）或统一宏 `FCEUX11_RUST_ENABLED` 控制，编译失败可随时切回 C++ 实现。
4. **所有权隔离**：C++ 分配的指针传进 Rust 做只读计算；Rust 返回的新缓冲区由 C++ 负责释放（或设计为栈分配 / 静态返回，如 CRC32/MD5 结果）。

### 2.3 头文件与 FFI 组织

推荐采用**统一头文件 + 分模块注释**的方式，避免头文件爆炸：

- 统一头文件：`src/rust/fceux11_rust.h`（已存在）
- 每新增一个模块，在该头文件中追加对应的 `extern "C"` 声明区块。
- 当模块数量超过 15 个或接口复杂度上升时，再考虑拆分为 `fceux11_rust_md5.h` 等分模块头文件。

---

## 三、迭代计划（v0.2.2 → v0.2.11）

### Phase 1 — v0.2.2：MD5（`src/utils/md5.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~205 行 |
| **当前接口** | `md5_starts`, `md5_update`, `md5_finish`, `md5_asciistr` |
| **Rust 方案** | 使用 [`md-5`](https://crates.io/crates/md-5) crate（RustCrypto 生态，纯 Rust，零不安全依赖） |
| **风险等级** | ★☆☆（极低） |
| **收益** | 消除手写 MD5 的维护负担；利用 SIMD 优化（若 crate 支持）；统一哈希接口风格。 |
| **状态** | ✅ 已完成（v0.2.2） |

**实施要点**：
- 保留 `md5_context` 结构体的 C 内存布局（`#[repr(C)]`），或改用 opaque pointer 模式。
- `md5_asciistr` 使用静态缓冲区返回，注意线程安全（原 C++ 实现也是 static buffer，行为保持一致即可）。

---

### Phase 2 — v0.2.3：GUID（`src/utils/guid.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~43 行 |
| **当前接口** | `FCEU_Guid::newGuid`, `toString`, `fromString`, `scan`, `hexToByte` |
| **Rust 方案** | 使用 [`uuid`](https://crates.io/crates/uuid) crate（v4 随机生成 + 解析） |
| **风险等级** | ★☆☆（极低） |
| **收益** | 消除 `rand()` 生成 GUID 的低质量随机性；标准 UUID 解析更健壮。 |
| **状态** | ✅ 已完成（v0.2.3） |

**实施要点**：
- `FCEU_Guid` 继承自 `ValueArray<uint8,16>`，Rust 侧定义为 `#[repr(C)] struct FceuGuid { data: [u8; 16] }`。
- `toString` 返回 `*const c_char` 时，使用 thread-local static buffer，调用方立即复制。
- 内部字节存储使用 little-endian（与 C++ 原有 `FCEU_de32lsb`/`FCEU_en32lsb` 一致）。

**变更文件**：
- `src/rust/src/guid.rs`（新增）
- `src/rust/src/lib.rs`（新增 `mod guid`）
- `src/rust/Cargo.toml`（新增 `uuid = { version = "1.8", features = ["v4"] }`）
- `src/rust/fceux11_rust.h`（新增 GUID FFI 声明）
- `src/utils/guid.cpp`（wrapper 实现）

---

### Phase 3 — v0.2.4：General Utilities（`src/utils/general.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~35 行 |
| **当前接口** | `uint32 uppow2(uint32 n)` |
| **Rust 方案** | 纯 Rust 标准库实现（`u32::next_power_of_two`） |
| **风险等级** | ★☆☆（极低） |
| **收益** | 热身验证：验证整个"修改 Cargo.toml → 写 Rust 函数 → 更新 C wrapper → CMake 构建 → 运行测试"的流水线是否通畅。 |

**实施要点**：
- 这是**最简单的端到端验证**，应在 1-2 小时内完成，作为团队熟悉流程的样板。

---

### Phase 4 — v0.2.5：Wave Audio Export（`src/wave.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~131 行 |
| **当前接口** | `FCEUI_BeginWaveRecord`, `FCEUI_WaveRecordRunning`, `FCEU_WriteWaveData`, `FCEUI_EndWaveRecord` |
| **Rust 方案** | 使用 `std::fs::File` + 手动 WAV header 写入；或用 [`hound`](https://crates.io/crates/hound) crate |
| **风险等级** | ★☆☆（低） |
| **收益** | 非核心模拟路径（仅在用户主动录音时触发）；消除手动 `fputc` 字节序操作，Rust 标准库更安全。 |

**实施要点**：
- 依赖 `FSettings.SndRate` 获取采样率。重构时将该值作为参数传入 Rust FFI（`begin_wave_record(fn: *const c_char, sample_rate: u32)`），**不直接读取 C++ 全局变量**，保持数据所有权隔离。
- 文件句柄用 Rust `Box<File>` 保存在静态 `Mutex` 中，或返回 opaque pointer 给 C++ 保存。

---

### Phase 5 — v0.2.6：OS Utilities（`src/drivers/common/os_utils.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~72 行 |
| **当前接口** | `fceu_mkdir`, `fceu_mkpath`, `fceu_file_exists`, `msleep` |
| **Rust 方案** | `std::fs::create_dir_all`, `std::fs::metadata`, `std::thread::sleep` |
| **风险等级** | ★☆☆（低） |
| **收益** | 当前代码硬编码 Win32 API（`Sleep`, `_mkdir`），Rust std 天然跨平台，为未来跨平台构建打基础。 |

**实施要点**：
- `fceu_mkpath` 涉及 C 风格字符串拼接，Rust 侧接收 `*const c_char` 后转换为 `CStr` → `PathBuf`。
- 这是首次重构 `src/drivers/` 下代码，验证 Rust 模块与 Qt/SDL 前端代码的链接兼容性。

---

### Phase 6 — v0.2.7：Unicode Conversion（`src/utils/ConvertUTF.c`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~499 行 |
| **当前接口** | `ConvertUTF8toUTF16`, `ConvertUTF16toUTF8`, `ConvertUTF8toUTF32`, `ConvertUTF32toUTF8`, `ConvertUTF16toUTF32`, `ConvertUTF32toUTF16`, `isLegalUTF8Sequence` |
| **Rust 方案** | Rust 标准库 `str::encode_utf16` / `String::from_utf16` / `char::encode_utf8` 完全覆盖；或封装为 C ABI 适配层 |
| **风险等级** | ★★☆（中低） |
| **收益** | 消除 Unicode Inc. 的老旧 C 代码（2001 年风格，手动指针算术易出错）；Rust 的 UTF-8 处理是语言级安全特性。 |

**实施要点**：
- 该模块是**纯算法转换器**，不涉及 I/O。重点验证缓冲区溢出防护：Rust 的 slice 边界检查替代原 C 代码的 `sourceEnd` / `targetEnd` 手动比较。
- 保留 `ConversionResult` 枚举的 C 布局，方便调用方错误处理逻辑不变。

---

### Phase 7 — v0.2.8：Time Stamp（`src/utils/timeStamp.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~72 行 + 166 行头文件（类定义） |
| **当前接口** | `FCEU::timeStampRecord` 类（运算符重载、QPC/TSC 校准） |
| **Rust 方案** | `std::time::Instant`（跨平台高精度计时）或 [`quanta`](https://crates.io/crates/quanta) crate |
| **风险等级** | ★★☆（中低） |
| **收益** | 消除手动 Windows QPC / TSC 校准代码；Rust 的计时 API 更精确且跨平台。 |

**实施要点**：
- C++ 侧是类（`timeStampRecord`），包含运算符重载。Rust 侧无法直接暴露类，策略：
  - **选项 A**：C++ 侧保留类外壳，将内部 `readNew()`、`toSeconds()` 等实现转发到 Rust FFI（推荐，侵入最小）。
  - **选项 B**：若后续大量 C++ 代码需要重构，可考虑 CXX crate，但 v0.2.x 阶段保持纯 C FFI，不引入 CXX 复杂度。

---

### Phase 8 — v0.2.9：Profiler（`src/profiler.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~318 行 + 163 行头文件 |
| **当前接口** | `FCEU_PROFILE_FUNC` 宏、`profilerManager`、`profilerFuncMap`、`funcProfileRecord` |
| **Rust 方案** | 纯 Rust 实现性能计数器；或保留 C++ 宏，仅将统计后端（`std::map` / `std::vector` 管理）移至 Rust |
| **风险等级** | ★★☆（中低） |
| **收益** | 仅在 `__FCEU_PROFILER_ENABLE__` 定义时编译，**零影响 Release 构建**；适合作为"较复杂状态管理"的 Rust 重构试验场。 |
| **状态** | ✅ 已完成（v0.2.9） |

**实施要点**：
- `FCEU_PROFILE_FUNC` 宏在 C++ 侧定义不变，因为它生成的是 `thread_local funcProfileRecord` + `profileFuncScoped` RAII 对象。
- Rust 侧替换的是 `profilerFuncMap` 和 `profilerManager` 的底层存储（将 `std::map<std::string, funcProfileRecord*>` 改为 Rust `HashMap`）。
- 这是首次涉及**跨语言的多线程/线程本地存储**交互，需验证 `mutex` 和 `thread_local` 的兼容性。

**变更文件**：
- `src/rust/src/profiler.rs`（新增）
- `src/rust/src/lib.rs`（新增 `mod profiler`）
- `src/rust/Cargo.toml`（版本 `0.2.9`）
- `src/rust/fceux11_rust.h`（新增 Profiler FFI 声明）
- `src/profiler.h`（类定义条件编译：`FCEUX11_RUST_ENABLED` 时用 `void* _rust_handle`）
- `src/profiler.cpp`（wrapper 实现：`#ifdef FCEUX11_RUST_ENABLED` 时调用 Rust FFI）

---

### Phase 9 — v0.2.10：Audio Filter（`src/filter.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~209 行 |
| **当前接口** | `NeoFilterSound`, `MakeFilters`, `SexyFilter`, `SexyFilter2` |
| **Rust 方案** | 直接翻译为 Rust，利用切片边界检查和固定长度数组类型安全；FIR 系数计算可用 `const fn` 在编译期处理 |
| **风险等级** | ★★★（中等） |
| **收益** | 位于音频渲染路径，但仅在 `Sound` 初始化后调用；Rust 的数组索引检查可消除潜在的 FIR 缓冲区溢出。 |
| **状态** | ✅ 已完成（v0.2.10） |

**实施要点**：
- **全局状态处理**：`FSettings.SndRate`, `FSettings.SoundVolume`, `FSettings.soundq`, `FSettings.lowpass`, `PAL` / `NTSC_CPU`, `GameExpSound.NeoFill`。
- **策略**：Rust 不直接读取全局变量；C++ wrapper 在 `filter.cpp` 中读取 `FSettings` 后作为参数传入 Rust FFI。所有 `static` 状态（`sq2coeffs`, `coeffs`, `mrindex`, `mrratio`, `acc1/acc2`）封装在 opaque `FilterState` 中。
- `SexyFilter` 和 `NeoFilterSound` 涉及 `static` 局部变量（`acc`, `mrindex`, `mrratio`），Rust 侧封装在 `struct FilterState` 中通过 opaque pointer 管理。
- FIR 系数表（`fcoeffs.h` + `fir/*.h`）自动转换为 Rust `const` 数组 `src/rust/src/fcoeffs.rs`。

**变更文件**：
- `src/rust/src/filter.rs`（新增）
- `src/rust/src/fcoeffs.rs`（新增，从 C++ 系数表自动转换）
- `src/rust/src/lib.rs`（新增 `mod filter` / `mod fcoeffs`）
- `src/rust/Cargo.toml`（版本 `0.2.10`）
- `src/rust/fceux11_rust.h`（新增 Filter FFI 声明）
- `src/filter.cpp`（wrapper 实现：`#ifdef FCEUX11_RUST_ENABLED` 时调用 Rust FFI）

---

### Phase 10 — v0.2.11：Palette（`src/palette.cpp`）

| 属性 | 详情 |
|------|------|
| **代码规模** | ~589 行 |
| **当前接口** | `FCEU_ResetPalette`, `FCEU_LoadGamePalette`, `FCEU_DrawNTSCControlBars` 等 |
| **Rust 方案** | 将静态调色板数据表和 `CalculatePalette` / `ChoosePalette` / `WritePalette` 逻辑迁移至 Rust；利用 `const` 数组和枚举增强类型安全 |
| **风险等级** | ★★★（中等） |
| **收益** | 大量静态数据（`palette_ntsc`, `lo_levels`, `hi_levels`, `phases` 等）在 Rust 中表达更安全；调色板异常肉眼可验证，便于回归测试。 |

**实施要点**：
- 调色板是**纯计算 + 查表**模块，输出为 `pal[64*8]` 数组。C++ 侧保留 `palo` 全局指针，Rust 侧计算完成后通过 FFI 将结果写入 C++ 预先分配的缓冲区。
- `FCEU_DrawNTSCControlBars` 涉及向 `uint8 *XBuf` 像素缓冲区写入，Rust 侧接收 `*mut u8` + width/height 参数，做边界安全的像素操作。
- 这是**首个涉及图形像素缓冲区的重构**，需仔细验证像素格式和缓冲区步长（stride）。

---

## 四、不纳入 v0.2.x 的高风险模块（及原因）

| 模块 | 位置 | 排除原因 |
|------|------|---------|
| **所有 Mapper** | `src/boards/*.cpp` | 依赖 `mapinc.h` 宏系统（`DECLFW`、`setprg8`、`SFORMAT`）和函数指针注册；Rust 宏与 C 宏不兼容，且位于核心模拟路径。 |
| **PPU** | `src/ppu.cpp` | 每帧执行数万次，任何时序偏差都会导致画面错误；全局状态极多。 |
| **CPU (6502)** | `src/x6502.cpp` | 模拟器核心，指令周期必须精确匹配；中断和标志位行为敏感。 |
| **APU / Sound** | `src/sound.cpp` | 与 Mapper 扩展音频（MMC5/VRC6 等）紧密耦合，时序敏感。 |
| **输入设备核心** | `src/input.cpp` | 涉及 Qt/SDL 事件循环和全局输入状态，与前端高度耦合。 |
| **Memory 管理器** | `src/utils/memory.cpp` | `FCEU_malloc`/`FCEU_gmalloc` 被全代码库调用，替换会导致链接爆炸；且涉及 `FCEU_MemoryRand` 全局初始化。 |
| **Movie / Netplay** | `src/movie.cpp`, `src/netplay.cpp` | 状态机复杂，涉及文件 I/O 和序列化格式兼容性，行为偏差会导致录像不同步。 |

> **建议**：上述模块待 Rust 占比超过 30%、团队对 FFI 边界有充分经验后，再考虑逐步迁移。

---

## 五、测试与回退策略

### 5.1 每个 Phase 的验收标准

1. **构建通过**：`FCEUX11_ENABLE_RUST=ON` 和 `OFF` 两种配置下，CMake 构建均成功。
2. **功能等价**：该模块涉及的现有单元测试（如有）全部通过；若无单元测试，需编写至少一个 Rust 侧 `#[test]` 验证输出与 C++ 原实现一致（以随机输入或已知 fixture 对比）。
3. **性能不退化**：对 `filter`、`md5` 等计算密集型模块，使用 `cargo bench` 或 C++ 侧计时对比，确保 Rust 实现性能不低于原 C++。
4. **ABI 验证**：使用 [`abi-checker`](https://github.com/scrive/abi-checker) 或至少通过链接器成功链接、运行时无符号未找到错误。

### 5.2 回退机制

```cmake
# src/rust/CMakeLists.txt 中（或 src/CMakeLists.txt 逐模块控制）
option(FCEUX11_RUST_MD5 "Use Rust MD5 implementation" ON)
option(FCEUX11_RUST_GUID "Use Rust GUID implementation" ON)
# ...

if(NOT FCEUX11_RUST_MD5)
    # 从 SRC_UTILS 中排除 md5.cpp 的 Rust wrapper，保留原实现
endif()
```

- 若某 Phase 的 Rust 实现出现难以调试的 bug，**立即回退到 C++ 实现**，不阻塞后续 Phase。
- 每个 Phase 独立分支开发，通过 Pull Request 合并，保持 `main` 分支始终可发布。

---

## 六、Rust 依赖管理策略

### 6.1 Crate 选择原则

- **优先标准库**：如 `std::time::Instant`、`std::fs` 可覆盖时，不引入外部依赖。
- **优先成熟生态**：如 RustCrypto（`md-5`）、`uuid` 等经过广泛审计的 crate。
- **避免异步运行时**：本项目为同步模拟器，不引入 `tokio`、`async-std`。
- **MSVC 兼容**：所有 crate 必须支持 `x86_64-pc-windows-msvc` 目标（项目已锁定）。

### 6.2 预计 Cargo.toml 演进

```toml
[dependencies]
crc32fast = "1.4"          # v0.2.1 已引入
md-5      = "0.10"         # v0.2.2
uuid      = { version = "1.8", features = ["v4"] }  # v0.2.3
# hound   = "3.5"          # v0.2.5（可选，若手动写 WAV 则不需要）
# quanta  = "0.12"         # v0.2.8（可选，若 std::time::Instant 足够则不需要）
```

---

## 七、时间线与里程碑（建议）

| 里程碑 | 版本 | 目标 |
|--------|------|------|
| M1 | v0.2.2 | MD5 重构完成，验证 crate 引入和测试流水线。 |
| M2 | v0.2.3 | GUID 重构完成，验证结构体 FFI 传递。 |
| M3 | v0.2.4 | General 重构完成，团队熟悉端到端流程。 |
| M4 | v0.2.5 | Wave 重构完成，验证文件 I/O FFI 和全局状态隔离。 |
| M5 | v0.2.6 | OS Utils 重构完成，验证跨平台（Win→未来 Linux/macOS）潜力。 |
| M6 | v0.2.7 | ConvertUTF 重构完成，验证缓冲区安全迁移。 |
| M7 | v0.2.8 | TimeStamp 重构完成，验证类→C ABI 的适配模式。 |
| M8 | v0.2.9 | Profiler 重构完成，验证多线程/线程本地存储交互。 |
| M9 | v0.2.10 | Filter 重构完成，验证性能敏感路径的 Rust 等效性。 |
| M10 | v0.2.11 | Palette 重构完成，验证图形像素缓冲区 FFI；**v0.2.x 系列收官**。 |

> **v0.2.12 预留**：作为紧急补丁版本号，或用于社区贡献的小模块重构（如 `ConvertUTF` 若分阶段实施）。

---

## 八、长期展望（v0.3.x 及以后）

完成 v0.2.x 的 10 个模块后，项目将具备以下能力：

1. **稳定的 Rust/C++ 混合构建基础设施**：CMake + Cargo 无缝协作。
2. **经过验证的 FFI 设计模式**：opaque pointer、`#[repr(C)]` 结构体、静态状态封装。
3. **团队 Rust 熟练度提升**：为更复杂模块迁移储备人力。

**v0.3.x 候选模块**（需预先进行可行性研究）：

- `src/emufile.cpp`：文件抽象层，替换后可统一压缩/解压缩逻辑（`libarchive` 已在 CMake 中）。
- `src/state.cpp`：存档序列化，纯数据流操作，但体积大（1314 行）。
- `src/ines.cpp` / `src/unif.cpp`：ROM 格式解析，适合 Rust 的 `nom` 解析器组合子。
- `src/cheat.cpp`：游戏作弊码引擎，状态机逻辑清晰。
- **简单 Mapper（如 VRC1、Mapper 34）**：待设计出一套安全的 "Rust Mapper 注册宏/DSL" 后再实施。

---

## 附录 A：参考文献

1. KDAB. "Best Practices for Hybrid Rust/C++ Apps." 2024. https://www.kdab.com/publications/bestpractices/best-practices-hybrid-rust-cpp-apps.html
2. arXiv:2603.02617. "Build-Aware Incremental C-to-Rust Migration via Skeleton-First Translation and Historical Knowledge Reuse." 2026.
3. Weinan Li. "Rust and Linux Kernel ABI Stability: A Technical Deep Dive." 2026. https://weinan.io/2026/02/16/rust-kernel-abi-stability-analysis.html
4. Rust Embedded Working Group. "A little Rust with your C." https://docs.rust-embedded.org/book/interoperability/rust-with-c.html
5. Valerio Viperino. "Rust interop with C++." 2025. https://valerioviperino.me/rust-cpp-ffi/
6. 40tude. "From Source File to Executable: A Gentle Walk Through the Rust Build System." 2025.

---

## 附录 B：现有 Rust 模块参考（v0.2.1）

- `src/rust/src/lib.rs`：暴露 `fceux11_rust_crc32()`。
- `src/rust/Cargo.toml`：版本 `0.2.1`，依赖 `crc32fast = "1.4"`。
- `src/rust/CMakeLists.txt`：通过 `cargo build --target x86_64-pc-windows-msvc --release` 生成 `fceux11_rust.lib`。
- `src/utils/crc32.cpp`：C++ wrapper，在 `FCEUX11_RUST_ENABLED` 时调用 Rust FFI，否则回退到 zlib。

后续模块应严格参照 `crc32` 的成功模式实施。
