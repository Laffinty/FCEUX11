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
# CI failure mode (build-windows run #84481955022, kagami-qa #84481955105):
# [11/1151] Scanning src/utils/general.cpp FAILED C1083
# [16/1151] Building Rust crate (too late — ninja already aborted).
set(RUST_MERGED_HEADER ${CMAKE_CURRENT_SOURCE_DIR}/fceux11_rust.h)

# Track Rust source files so CMake rebuilds the staticlib when they change.
file(GLOB_RECURSE RUST_SOURCE_FILES ${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs ${CMAKE_CURRENT_SOURCE_DIR}/crates/*.rs)

add_custom_command(
    OUTPUT ${RUST_STATIC_LIB} ${RUST_MERGED_HEADER}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CARGO_TARGET_DIR}
        cargo build --manifest-path ${CARGO_MANIFEST}
                    --target ${RUST_TARGET}
                    --release
                    --features direct-adapter
    DEPENDS ${RUST_SOURCE_FILES} ${CARGO_MANIFEST}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    COMMENT "Building Rust crate (target: ${RUST_TARGET}, features=direct-adapter)..."
    VERBATIM
)

add_custom_target(fceux11_rust_build DEPENDS ${RUST_STATIC_LIB} ${RUST_MERGED_HEADER})

set(FCEUX11_RUST_ENABLED ON PARENT_SCOPE)
set(FCEUX11_RUST_STATIC_LIB ${RUST_STATIC_LIB} PARENT_SCOPE)

# v2.0 wip: register the vNESU11 crate as a sibling build target. Its
# staticlib (vnesu11.lib) is linked separately by the parent when
# VNESU11_CORE=ON — see src/CMakeLists.txt.
add_subdirectory(crates/vnesu11)

# v2.0 wip (Phase 6 fix): re-export the vNESU11 static lib path up to
# src/CMakeLists.txt + tests/CMakeLists.txt (the crates/vnesu11
# PARENT_SCOPE only reaches this directory's scope).
set(VNESU11_RUST_LIB ${VNESU11_RUST_LIB} PARENT_SCOPE)
