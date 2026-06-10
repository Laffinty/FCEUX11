
cmake_minimum_required(VERSION 4.0)

# v0.2.1: Auto-detect vcpkg toolchain
if(DEFINED ENV{VCPKG_ROOT} AND NOT DEFINED CMAKE_TOOLCHAIN_FILE)
    # Prefer local vcpkg_installed (avoids manifest-mode re-build of Qt6 etc.)
    if(EXISTS "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
        list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
        message(STATUS "Using local vcpkg_installed: ${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
    else()
        set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake"
            CACHE STRING "")
    endif()
endif()

project(FCEUX11 VERSION 0.3.6 LANGUAGES CXX C)

# v0.2.1: Enforce MSVC toolchain; MinGW-w64 / MSYS2 support removed.
# v0.3.1: Locked to MSVC 19.36+ (Visual Studio 17.6+) to avoid C++23
# preview behavior drift — see docs/v0.3.x_Construction_Plan_v3.md §3.1.
# v0.3.6.5: This MSVC lock is **non-negotiable** and applies even to the
# sanitizer driver. Do NOT switch to clang-cl, gcc, MinGW, or any
# non-MSVC toolchain — even with FCEUX11_ASAN=ON. Real ASan coverage
# is a v0.4.x item (plan §10.2), and may require shipping pre-built
# ASan-instrumented binaries in CI rather than changing the local
# toolchain. If you are a future agent and you are reading this
# because cmake configure failed, the correct action is to install
# the MSVC toolchain matching the locked version, NOT to "loosen"
# this check or set CMAKE_C_COMPILER=clang-cl.
if(NOT MSVC)
    message(FATAL_ERROR
        "FCEUX11 v0.2.1+ requires MSVC 2022+ toolchain. "
        "MinGW-w64 / MSYS2 / clang-cl / gcc support has been removed. "
        "Do NOT change CMAKE_C_COMPILER / CMAKE_CXX_COMPILER to bypass "
        "this — the project is byte-level compatible with MSVC only. "
        "See docs/tech/v0.3.x_Checkpoint_6.5.md §2 for why we accept "
        "the ASan tooling limitation rather than switch compilers.")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# v0.2.1: MSVC security and conformance options
# v0.3.1: /W4 soft-start with /WX- (warnings are not errors yet; /WX pushed to v0.3.16)
# v0.3.1: /wd4996 is transitional — will be removed after strcpy/sprintf/vsprintf cleanup in v0.3.5
add_compile_options(/W4 /permissive- /guard:cf /GS /sdl /wd4996)
add_link_options(/GUARD:CF /CETCOMPAT)

# v0.3.1: ccache probe (Windows chocolatey path)
find_program(CCACHE_PROGRAM ccache PATHS "C:/ProgramData/chocolatey/bin" "C:/Program Files/ccache" "C:/msys64/usr/bin")
if(CCACHE_PROGRAM)
    message(STATUS "ccache found: ${CCACHE_PROGRAM}")
    set(CMAKE_C_COMPILER_LAUNCHER   "${CCACHE_PROGRAM}")
    set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
else()
    message(STATUS "ccache not found — incremental builds will use compiler cache only")
endif()

option(FCEUX11_BUILD_TESTS "Build smoke and regression tests" ON)
option(FCEUX11_ENABLE_I18N "Enable internationalization (i18n) support" ON)
option(ENABLE_LINT "Enable static analysis targets (clang-tidy, cppcheck)" OFF)

# v0.3.6.5: B-track checkpoint — AddressSanitizer / UndefinedBehaviorSanitizer
# Both are MSVC 19.36+ options (/fsanitize=address|undefined). On Windows
# they require MSVC's `<experimental/>` ASan runtime and Debug-or-RelWithDebInfo.
#
# UBSan: MSVC 19.36 implements a *subset* of the clang UBSan checks:
#   bounds, enum, float-cast-overflow, float-divide-by-zero,
#   integer-divide-by-zero, null, return, shift,
#   signed-integer-overflow, unreachable, vla-bound, function, pointer-overflow.
#   (alignment / object-size / builtin / vptr are NOT available.)
# Clang-cl provides the full set; for now we expose the MSVC variant.
#
# LSan (LeakSanitizer) is bundled with MSVC ASan — no separate flag required.
#
# ============================================================
# ⚠  FUTURE-AGENT GUARDRAIL — DO NOT SWITCH TO CLANG-CL  ⚠
# ============================================================
# If you are reading this and CMake rejected your attempt to set
# CMAKE_C_COMPILER=clang-cl "to make ASan work", DO NOT modify this
# block to allow clang-cl. The MSVC-only lock above is intentional
# (per plan §1.3 "工具链锁定: MSVC 19.36 + Qt 6.8 LTS 在 v0.3.16
# 之前不升级"). On machines where cl 14.x actually rejects
# /fsanitize:address with D9002 (silently dropping the flag), the
# correct mitigation is one of:
#   1. Document the limitation and ship without real ASan coverage
#      (current v0.3.6.5 baseline — see docs/tech/v0.3.x_Checkpoint_6.5.md)
#   2. Run a SEPARATE CI job with a pre-built clang-cl toolchain
#      (out-of-tree, matrix-style; do NOT change the local default)
#   3. Wait for v0.4.x to revisit the toolchain policy
# Do NOT install LLVM clang-cl as a default system toolchain on the
# assumption it will be picked up — that would silently change every
# agent's CMAKE_C_COMPILER detection and break the MSVC-only stance.
# ============================================================
option(FCEUX11_ASAN  "Enable AddressSanitizer (and LSan) for the v0.3.6.5 checkpoint" OFF)
option(FCEUX11_UBSAN "Enable UndefinedBehaviorSanitizer for the v0.3.6.5 checkpoint" OFF)

if(FCEUX11_ASAN OR FCEUX11_UBSAN)
    if(NOT MSVC)
        message(FATAL_ERROR
            "FCEUX11_ASAN / FCEUX11_UBSAN currently require the MSVC toolchain "
            "(uses /fsanitize=address|undefined). Clang-cl support is a v0.4.x item.")
    endif()
    # Sanitizers require debug-friendly builds — opt-in by user, but recommend RelWithDebInfo.
    if(NOT CMAKE_BUILD_TYPE STREQUAL "Debug" AND
       NOT CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
        message(WARNING
            "FCEUX11_ASAN/UBSAN is most useful with CMAKE_BUILD_TYPE=Debug or "
            "RelWithDebInfo. Current: ${CMAKE_BUILD_TYPE}. Continuing anyway.")
    endif()
endif()

if(FCEUX11_ASAN)
    message(STATUS "FCEUX11: AddressSanitizer ENABLED (/fsanitize:address, LSan bundled)")
    add_compile_options(/fsanitize:address)
    # Use CMAKE_*_LINKER_FLAGS rather than add_link_options — Ninja
    # misinterprets `/fsanitize:address` as a build input when injected
    # through the add_link_options() path (it sees the leading `/` as a
    # file dependency). CMAKE_*_LINKER_FLAGS go directly into the link
    # rule's args, no escaping issues.
    set(CMAKE_EXE_LINKER_FLAGS_DEBUG   "/fsanitize:address ${CMAKE_EXE_LINKER_FLAGS_DEBUG}")
    set(CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize:address ${CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO}")
    set(CMAKE_SHARED_LINKER_FLAGS_DEBUG   "/fsanitize:address ${CMAKE_SHARED_LINKER_FLAGS_DEBUG}")
    set(CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize:address ${CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO}")
    set(ASAN_LDFLAGS "/fsanitize:address" CACHE INTERNAL "ASan link flags")
    add_definitions(-DFCEUX11_ASAN_ENABLED=1)
endif()

if(FCEUX11_UBSAN)
    message(STATUS "FCEUX11: UndefinedBehaviorSanitizer ENABLED (/fsanitize:undefined)")
    add_compile_options(/fsanitize:undefined)
    set(CMAKE_EXE_LINKER_FLAGS_DEBUG   "/fsanitize:undefined ${CMAKE_EXE_LINKER_FLAGS_DEBUG}")
    set(CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize:undefined ${CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO}")
    set(CMAKE_SHARED_LINKER_FLAGS_DEBUG   "/fsanitize:undefined ${CMAKE_SHARED_LINKER_FLAGS_DEBUG}")
    set(CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize:undefined ${CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO}")
    set(UBSAN_LDFLAGS "/fsanitize:undefined" CACHE INTERNAL "UBSan link flags")
    add_definitions(-DFCEUX11_UBSAN_ENABLED=1)
endif()

# v0.3.2: Export compile_commands.json for clang-tidy / IDE integration
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# i18n: Qt Linguist tools
if(FCEUX11_ENABLE_I18N)
    find_package(Qt6 REQUIRED COMPONENTS LinguistTools)
endif()

add_subdirectory( src )

if(FCEUX11_BUILD_TESTS)
    enable_testing()
    add_subdirectory( tests )
    include(CTest)
endif()

# v0.3.2: Optional lint targets (clang-tidy / cppcheck)
if(ENABLE_LINT)
    find_program(CLANG_TIDY_PROG clang-tidy)
    find_program(CPPCHECK_PROG cppcheck)

    if(CLANG_TIDY_PROG AND EXISTS "${CMAKE_BINARY_DIR}/compile_commands.json")
        add_custom_target(lint-clang-tidy
            COMMAND ${CLANG_TIDY_PROG}
                -p ${CMAKE_BINARY_DIR}
                ${CMAKE_SOURCE_DIR}/src/*.cpp
                ${CMAKE_SOURCE_DIR}/src/drivers/Qt/*.cpp
                ${CMAKE_SOURCE_DIR}/src/utils/*.cpp
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Running clang-tidy..."
        )
    endif()

    if(CPPCHECK_PROG)
        add_custom_target(lint-cppcheck
            COMMAND ${CPPCHECK_PROG}
                --enable=warning,style,performance,portability
                --std=c++20 --language=c++ --template=gcc
                --suppress=missingIncludeSystem
                --suppress=unusedFunction
                -I ${CMAKE_SOURCE_DIR}/src
                -I ${CMAKE_SOURCE_DIR}/src/drivers
                ${CMAKE_SOURCE_DIR}/src/
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Running cppcheck..."
        )
    endif()
endif()
