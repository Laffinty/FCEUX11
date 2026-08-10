# vNESU11 — Rust crate CMake registration.
#
# Phase 0: produces vnesu11.lib (staticlib) for C++ linking when
# VNESU11_CORE=ON. Mirrors the pattern used by the root fceux11-rust
# crate (see src/rust/CMakeLists.txt).

set(VNESU11_CRATE_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(VNESU11_MANIFEST   ${VNESU11_CRATE_DIR}/Cargo.toml)
set(VNESU11_TARGET_TRIPLE "x86_64-pc-windows-msvc")
set(VNESU11_TARGET_DIR  ${CMAKE_CURRENT_BINARY_DIR}/target)

# Same pattern as src/rust/CMakeLists.txt — explicit OUTPUT so CMake/ninja
# don't schedule C++ compilation of vnesu11_ffi.h consumers in parallel
# with cargo build.
set(VNESU11_STATIC_LIB  ${VNESU11_TARGET_DIR}/${VNESU11_TARGET_TRIPLE}/release/vnesu11.lib)
set(VNESU11_MERGED_HEADER ${VNESU11_CRATE_DIR}/../../fceux11_rust.h)  # future

file(GLOB_RECURSE VNESU11_SOURCE_FILES ${VNESU11_CRATE_DIR}/src/*.rs)

add_custom_command(
    OUTPUT ${VNESU11_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${VNESU11_TARGET_DIR}
        cargo build --manifest-path ${VNESU11_MANIFEST}
                    --target ${VNESU11_TARGET_TRIPLE}
                    --release
    DEPENDS ${VNESU11_SOURCE_FILES} ${VNESU11_MANIFEST}
    WORKING_DIRECTORY ${VNESU11_CRATE_DIR}
    COMMENT "Building vNESU11 Rust crate (target: ${VNESU11_TARGET_TRIPLE})..."
    VERBATIM
)

add_custom_target(vnesu11_build DEPENDS ${VNESU11_STATIC_LIB})

set(VNESU11_ENABLED ON PARENT_SCOPE)
set(VNESU11_RUST_LIB ${VNESU11_STATIC_LIB} PARENT_SCOPE)
