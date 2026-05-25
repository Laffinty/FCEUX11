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

# Track Rust source files so CMake rebuilds the staticlib when they change.
file(GLOB RUST_SOURCE_FILES ${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs)

add_custom_command(
    OUTPUT ${RUST_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CARGO_TARGET_DIR}
        cargo build --manifest-path ${CARGO_MANIFEST}
                    --target ${RUST_TARGET}
                    --release
    DEPENDS ${RUST_SOURCE_FILES} ${CARGO_MANIFEST}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    COMMENT "Building Rust crate (target: ${RUST_TARGET})..."
    VERBATIM
)

add_custom_target(fceux11_rust_build DEPENDS ${RUST_STATIC_LIB})

set(FCEUX11_RUST_ENABLED ON PARENT_SCOPE)
set(FCEUX11_RUST_STATIC_LIB ${RUST_STATIC_LIB} PARENT_SCOPE)
