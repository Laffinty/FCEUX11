FCEUX11 v0.2.8

7 个自包含叶子模块从 C/C++ 渐进式迁移至 Rust，验证混合语言架构的可行性。
Migrated 7 self-contained leaf modules from C/C++ to Rust incrementally, validating the feasibility of a hybrid C++/Rust architecture.

MD5 哈希计算改用 RustCrypto md-5 crate，消除手写算法维护负担，支持 SIMD 优化。
Replaced handwritten MD5 with the RustCrypto md-5 crate, eliminating maintenance burden and enabling SIMD optimizations.

GUID 生成改用标准 UUID v4（uuid crate），消除低质量 rand() 随机性，解析更健壮。
Replaced low-quality rand()-based GUID generation with standard UUID v4 (uuid crate), providing more robust parsing.

通用工具函数 uppow2 改用 Rust 标准库实现，作为端到端流水线验证样板。
Replaced uppow2 with Rust std implementation, serving as the end-to-end pipeline validation template.

WAV 音频导出改用 Rust 安全文件 I/O，消除手动 fputc 字节序操作，文件句柄由静态 Mutex 管理。
Migrated WAV audio export to Rust safe file I/O, eliminating manual fputc byte-order operations with file handles managed by a static Mutex.

OS 工具函数（mkdir / mkpath / file_exists / msleep）改用 Rust 标准库，解耦 Win32 API 硬编码，为未来跨平台构建打基础。
Migrated OS utilities to Rust std, decoupling hardcoded Win32 APIs and laying groundwork for future cross-platform builds.

Unicode 转换（UTF-8/16/32）改用 Rust 安全实现，slice 边界检查替代 2001 年风格的手动指针算术，消除缓冲区溢出风险。
Migrated Unicode conversions to safe Rust, replacing 2001-style manual pointer arithmetic with slice bounds checking to eliminate buffer overflow risks.

时间戳计时改用 std::time::Instant 跨平台纳秒计时器，消除 Windows QPC/TSC 手动校准代码，保留 C++ timeStampRecord 类外壳，调用方零侵入。
Replaced timestamp timing with std::time::Instant cross-platform nanosecond timer, eliminating manual Windows QPC/TSC calibration while preserving the C++ timeStampRecord class shell with zero caller intrusion.

所有模块遵循统一的 Wrapper-Shim + Feature-Gate 范式：#ifdef FCEUX11_RUST_ENABLED 时调用 Rust FFI，否则自动回退到原 C++ 实现。
All modules follow the unified Wrapper-Shim + Feature-Gate pattern: delegate to Rust FFI when FCEUX11_RUST_ENABLED is defined, otherwise automatically fall back to the original C++ implementation.

C ABI 边界保持稳定：#[no_mangle] pub extern C、#[repr(C)] 结构体、所有权隔离（数据留在各自语言侧）。
C ABI boundaries remain stable via #[no_mangle] pub extern C, #[repr(C)] structs, and ownership isolation (data stays on each language side).

Rust cargo test 42 项全部通过；CMake MSVC Release 构建通过；FCEUX11_RUST_ENABLED=ON/OFF 双配置均编译链接成功。
Rust cargo test: 42/42 passed; CMake MSVC Release build passed; both FCEUX11_RUST_ENABLED=ON and OFF configurations compile and link successfully.

软件界面版本号统一升级至 v0.2.8：主窗口标题、关于窗口、Qt 翻译文件（英/简中/繁中）及 CMake/vcpkg/Rust 元数据全部同步。
Unified version bump to v0.2.8 across all UI surfaces: main window title, About dialog, Qt translation files (en/zh_CN/zh_TW), and CMake/vcpkg/Rust metadata.
