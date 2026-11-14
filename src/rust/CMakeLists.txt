option(FCEUX11_ENABLE_RUST "Enable Rust module integration" ON)

if(NOT FCEUX11_ENABLE_RUST)
    message(STATUS "Rust module disabled (FCEUX11_RUST_ENABLED=OFF)")
    return()
endif()

# v0.2.1: Rust target locked to MSVC toolchain
set(RUST_TARGET "x86_64-pc-windows-msvc" CACHE STRING "Rust target triple")
set(RUST_TOOLCHAIN "stable" CACHE STRING "Rust toolchain")

find_program(RUSTUP_EXE rustup)
if(NOT RUSTUP_EXE)
    message(FATAL_ERROR "rustup not found. Install Rust: https://rustup.rs/")
endif()

execute_process(
    COMMAND ${RUSTUP_EXE} target list --installed
    OUTPUT_VARIABLE RUST_INSTALLED_TARGETS
    OUTPUT_STRIP_TRAILING_WHITESPACE
    ERROR_QUIET
)

string(FIND "${RUST_INSTALLED_TARGETS}" "${RUST_TARGET}" RUST_TARGET_FOUND)
if(RUST_TARGET_FOUND EQUAL -1)
    message(FATAL_ERROR
        "Rust target '${RUST_TARGET}' is not installed.\n"
        "Please run: rustup target add ${RUST_TARGET}"
    )
endif()

set(CARGO_MANIFEST ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)
# Use a dedicated 'target' subdirectory so Cargo artifacts don't mix with CMake files.
set(CARGO_TARGET_DIR ${CMAKE_CURRENT_BINARY_DIR}/target)

set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/fceux11_rust.lib)

# v1.16 R4-fix (2026-08-07): The merged cbindgen header is a side-product of
# `cargo build` (emitted by build.rs at ${crate_dir}/fceux11_rust.h). It must
# be listed as an explicit OUTPUT of the custom command so CMake / ninja
# know it is produced here — otherwise downstream C++ source files that
# #include "rust/fceux11_rust.h" are scheduled in parallel with cargo build
# and fail with C1083 "Cannot open include file" before the header exists.
# CI failure mode (build-windows run #84481955022, f11qa #84481955105):
# [11/1151] Scanning src/utils/general.cpp FAILED C1083
# [16/1151] Building Rust crate (too late — ninja already aborted).
set(RUST_MERGED_HEADER ${CMAKE_CURRENT_SOURCE_DIR}/fceux11_rust.h)

# Track Rust source files so CMake rebuilds the staticlib when they change.
# v2.0 S0: CONFIGURE_DEPENDS closes risk R10. Plain GLOB_RECURSE is
# evaluated only at configure time, so a .rs file appearing later --
# e.g. the whole crates/f11gba tree -- never reaches DEPENDS, and editing
# it silently reuses the stale fceux11_rust.lib with no error.
# CONFIGURE_DEPENDS re-globs at build time and re-configures on delta.
file(GLOB_RECURSE RUST_SOURCE_FILES CONFIGURE_DEPENDS
     ${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs
     ${CMAKE_CURRENT_SOURCE_DIR}/crates/*.rs)

add_custom_command(
    OUTPUT ${RUST_STATIC_LIB} ${RUST_MERGED_HEADER}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CARGO_TARGET_DIR}
        cargo build --manifest-path ${CARGO_MANIFEST}
                    --target ${RUST_TARGET}
                    --release
                    --features direct-adapter,gba
    DEPENDS ${RUST_SOURCE_FILES} ${CARGO_MANIFEST}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    COMMENT "Building Rust crate (target: ${RUST_TARGET}, features=direct-adapter)..."
    VERBATIM
)

add_custom_target(fceux11_rust_build DEPENDS ${RUST_STATIC_LIB} ${RUST_MERGED_HEADER})

set(FCEUX11_RUST_ENABLED ON PARENT_SCOPE)
set(FCEUX11_RUST_STATIC_LIB ${RUST_STATIC_LIB} PARENT_SCOPE)

# ---------------------------------------------------------------------------
# v2.0.4 r17 (2026-11-15): Rust 的原生静态库契约（native-static-libs）
#
# 背景：任何链接 fceux11_rust.lib 的 C++ 目标，都必须同时链接 rustc 依赖的
# 原生 import library。rustc **不会**为它们在目标文件里写 /DEFAULTLIB 指令，
# 所以 MSVC 的 link.exe 不会自动补上 —— 缺一个就是 LNK2001 + LNK1120。
#
# 权威来源是 rustc 自己打印的清单（不是猜的）：
#     cd src/rust
#     cargo rustc --release --target x86_64-pc-windows-msvc \
#                  --features direct-adapter,gba --lib -- --print native-static-libs
#
# rustc 1.100.0（2026-11-09）的清单：
#     kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib
#     synchronization.lib /defaultlib:msvcrt
#
# 已有接线：kernel32（MSVC 默认）、ntdll / userenv / ws2_32 / dbghelp
# （src/CMakeLists.txt 与 tests/CMakeLists.txt 里已经显式链接）。
# 本次新增：**synchronization**（WaitOnAddress / WakeByAddressSingle /
# WakeByAddressAll 的导入库，SDK 的 um/x64/synchronization.lib）。
#
# 事故经过（CI F11QA job 报红，run 2026-11-14T20:09Z）：
#   CI 的 `dtolnay/rust-toolchain@stable` 自动从 1.99.1 升到 1.100.0，
#   1.100 的 std 改用 WaitOnAddress/WakeByAddress* 做线程挂起，
#   于是 fceux11_rust.lib 里多出三个 __imp_* 未定义符号；
#   链接行没有 synchronization.lib ⇒ 5 个 runner 可执行文件全部 LNK1120
#   ⇒ 构建中断，迁移矩阵根本没产出，R4 gate 随之报「矩阵未产出」。
#   本仓代码零改动，纯粹是工具链漂移打穿了链接契约。
#
# 升级 Rust 时的动作（**必须**照做，否则同一个坑会再踩一次）：
#   1) 跑上面那条 `cargo rustc --print native-static-libs`，拿新清单；
#   2) 把新增/消失的原生库同步到下面的列表；
#   3) 同步更新 .github/workflows/{f11qa,ci}.yml 里的 RUST_TOOLCHAIN 版本号。
# ---------------------------------------------------------------------------
set(FCEUX11_RUST_NATIVE_LIBS "")
if(WIN32)
    # WaitOnAddress / WakeByAddressSingle / WakeByAddressAll
    list(APPEND FCEUX11_RUST_NATIVE_LIBS synchronization)
endif()
set(FCEUX11_RUST_NATIVE_LIBS ${FCEUX11_RUST_NATIVE_LIBS} PARENT_SCOPE)
