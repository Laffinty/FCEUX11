option(FCEUX11_ENABLE_RUST "Enable Rust module integration" ON)

if(NOT FCEUX11_ENABLE_RUST)
    message(STATUS "Rust module disabled (FCEUX11_ENABLE_RUST=OFF)")
    return()
endif()

# Detect Rust target triple based on the active C++ compiler
if(MSVC)
    set(RUST_TARGET "x86_64-pc-windows-msvc")
elseif(MINGW)
    set(RUST_TARGET "x86_64-pc-windows-gnu")
else()
    message(FATAL_ERROR "Rust module: Unsupported C++ compiler. Only MSVC and MinGW-w64 are supported.")
endif()

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
set(CARGO_TARGET_DIR ${CMAKE_CURRENT_BINARY_DIR}/rust)

if(MSVC)
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/fceux11_rust.lib)
else()
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/libfceux11_rust.a)
endif()

add_custom_command(
    OUTPUT ${RUST_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CARGO_TARGET_DIR}
        cargo build --manifest-path ${CARGO_MANIFEST}
                    --target ${RUST_TARGET}
                    --release
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    COMMENT "Building Rust crate (target: ${RUST_TARGET})..."
    VERBATIM
)

add_custom_target(fceux11_rust_build DEPENDS ${RUST_STATIC_LIB})

set(FCEUX11_RUST_ENABLED ON PARENT_SCOPE)
set(FCEUX11_RUST_STATIC_LIB ${RUST_STATIC_LIB} PARENT_SCOPE)
