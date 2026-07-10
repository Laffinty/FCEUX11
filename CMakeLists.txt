
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

project(FCEUX11 VERSION 1.13 LANGUAGES CXX C)

# v0.2.1: Enforce MSVC toolchain; MinGW-w64 / MSYS2 support removed.
# v0.3.1: Locked to MSVC 19.36+ (Visual Studio 17.6+) to avoid C++23
# preview behavior drift. v1.0 continues this lock — see
# docs/tech/13_构建系统与CI矩阵.txt §1 for the toolchain policy.
# v0.3.6.5: MSVC lock stays in place for ABI / byte-level savestate
# reasons (plan §3.1, §1.3 "工具链锁定"). This is NOT because cl
# cannot drive sanitizers — it can: /fsanitize=address has shipped
# since VS 2019 v16.9 and is fully wired below. The only sanitizer
# MSVC does not implement is /fsanitize=undefined (clang-only);
# v0.3.6.5 substitutes that with native /RTC1 + /sdl + /GS + /guard:cf
# (see UBSan block below). Full clang-style UBSan is a v0.4.x item.
if(NOT MSVC)
    message(FATAL_ERROR
        "FCEUX11 v0.2.1+ requires the MSVC 2022+ toolchain for ABI and "
        "byte-level savestate compatibility (plan §3.1). MinGW-w64 / "
        "MSYS2 / clang-cl / gcc paths have been removed. Do NOT change "
        "CMAKE_C_COMPILER / CMAKE_CXX_COMPILER to bypass this guard.")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# v0.2.1: MSVC security and conformance options
# v0.3.1: /W4 soft-start with /WX- (warnings are not errors yet; /WX pushed to v0.3.16)
# v0.3.16 (LTS closure): /WX activated (warnings as errors). Residual
#   baseline warnings from v0.3.0~v0.3.15 (C4100 unreferenced param,
#   C4267 size_t→int in pre-v0.3.10 call sites, C4456 variable shadowing
#   in TasEditor auto-generated code, C4200 zero-length array in cbindgen
#   output, C4244 narrowing in legacy Lua bindings) are explicitly /wd'd
#   with documented rationale — these are pre-existing categories that
#   pre-date the /WX upgrade and are not addressed in v1.0.
#   New code must compile clean under /WX.
# v1.0: continues the v0.3.16 LTS /WX policy unchanged.
# v0.3.1: /wd4996 is transitional — will be removed after strcpy/sprintf/vsprintf cleanup in v0.3.5
add_compile_options(/W4 /WX /permissive- /guard:cf /GS /sdl
    /utf-8  # v1.0: tell cl.exe to interpret source as UTF-8 (not system code page 936)
    /wd4996  # transitional: deprecated POSIX/CRT functions (strncpy/strncat in safe_string.h)
    /wd4100  # pre-existing: unreferenced parameter (event handlers, virtuals)
    /wd4267  # pre-existing: size_t→int in mappers / TasEditor / file.cpp (pre-v0.3.10)
    # /wd4456 removed in v1.13 Purify I §11
    /wd4200  # pre-existing: zero-length array in cbindgen-generated fceux11_rust.h
    /wd4244  # pre-existing: narrowing conversion in ops_table.inc / x6502.cpp / vsuni.cpp
    # /wd4514 removed in v1.13 Purify I §11
    # /wd4710 removed in v1.13 Purify I §11
    /wd4820  # pre-existing: padding added (Q_OBJECT vtables, Rust #[repr(C)] structs)
    # /wd4866 removed in v1.13 Purify I §11
    # /wd4868 removed in v1.13 Purify I §11
    # /wd5039 removed in v1.13 Purify I §11
)
add_link_options(/GUARD:CF /CETCOMPAT /IGNORE:4098)  # LNK4098 LIBCMT conflict in test linking

# v0.3.1: ccache probe. v1.0: scan PATH first (CMake default) so ccache
# in any user-writable location (scoop, winget, custom) is picked up.
# The PATHS list below is a fallback hint for choco-installed ccache.
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
# v0.3.16 LTS closure / v1.0 carries over: project-wide deprecation
# control. Per plan v3 §6.3, the FCEUI_* compat shims (250+ inline
# wrappers in core_api.h / io_api.h / net_api.h / diag_api.h / movie.h)
# plus fceuScopedPtr / FCEU_malloc* / etc. are marked
# [[deprecated("use fceu11::Foo")]] at the source level. The default
# during v0.3.x / v1.0 is OFF (no warnings emitted) so the in-tree
# build compiles clean (matching the documented "macro 抑制" policy).
# External users who want to see the deprecation warnings can pass
# -DFCEUX11_SHOW_DEPRECATION_WARNINGS=ON to opt out. Removal of the
# shims themselves is scheduled for v2.0 (one full major-version
# migration window per plan v3 §6.1 phase 3).
option(FCEUX11_SHOW_DEPRECATION_WARNINGS "Emit [[deprecated]] warnings on FCEUI_* / FCEU_malloc* / fceuScopedPtr use (off by default during v0.3.x; on for v0.4.0+ migration audit)" OFF)
if(NOT FCEUX11_SHOW_DEPRECATION_WARNINGS)
    add_definitions(-DFCEUX11_NO_DEPRECATION_WARNINGS)
endif()
# v0.3.6.6: toolchain policy — MSVC-only. The prior `ENABLE_LINT` option was
# implemented as a clang-tidy + cppcheck toggle; the clang-tidy half has been
# removed (no .clang-format / .clang-tidy / clang-tidy in the toolchain per
# v0.3.6.5 errata). cppcheck is a separate, non-clang tool and is retained.
# Use ENABLE_LINT_CPPCHECK to wire the cppcheck custom target into the build.
option(ENABLE_LINT_CPPCHECK "Enable cppcheck static analysis custom target (clang-tidy removed per v0.3.6.6 toolchain policy)" OFF)

# v0.3.6.5: B-track checkpoint — AddressSanitizer (real, MSVC-native)
# + UndefinedBehavior-substitute (MSVC-native runtime checks).
#
# ASan — fully supported by cl since VS 2019 v16.9 via /fsanitize=address.
# The runtime DLLs (clang_rt.asan_dynamic-x86_64.dll) and import libs
# ship with every MSVC install in <VC>\bin\Hostx64\x64\ and
# <VC>\lib\x64\. No extra component install is required.
#
# NOTE on flag syntax: MSVC accepts the `=` form ONLY. The earlier
# v0.3.6.5 attempt used `/fsanitize:address` (colon), which cl silently
# drops as D9002 "unknown option" — that's the original PARTIAL-PASS
# root cause documented in docs/tech/v0.3.x_Checkpoint_6.5.md.
#
# UBSan — Microsoft has not implemented /fsanitize=undefined (the
# Microsoft Learn /cpp/sanitizers/ landing page lists only
# AddressSanitizer). v0.3.6.5 therefore substitutes a MSVC-native
# set of UB-catching runtime checks under the FCEUX11_UBSAN switch:
#
#   /RTC1   — Debug-only: stack-frame integrity, uninitialized
#             variable use, value truncation on cast (covers a
#             subset of clang's `bool`/`enum`/`integer-divide-by-zero`).
#   /sdl    — already on globally: extends /GS, adds checks for
#             buffer overruns, integer overflow in `new[]`, etc.
#   /GS     — already on globally: stack buffer security cookie.
#   /guard:cf — already on globally: Control Flow Guard.
#
# These do NOT cover the full clang UBSan check set (alignment,
# object-size, vptr, signed-integer-overflow on plain arithmetic,
# float-divide-by-zero, etc.) — full clang-style UBSan is a v0.4.x
# toolchain workstream item. The MSVC subset is treated as a
# functional substitute for v0.3.6.5's "catch UB before v0.3.7" goal,
# not as clang-UBSan parity.
#
# LSan caveat: MSVC Windows ASan does NOT implement LeakSanitizer.
# `ASAN_OPTIONS=detect_leaks=1` is rejected by cl's ASan runtime, so
# leak detection is deferred to v0.4.x clang-cl CI matrix. See
# scripts/_ctest_asan.ps1 (no `detect_leaks` in ASAN_OPTIONS) and
# docs/tech/v0.3.x_Checkpoint_6.5.md §4.4 for the empirical proof.
option(FCEUX11_ASAN  "Enable AddressSanitizer (ASan only — LSan is MSVC-Windows-disabled) for the v0.3.6.5 checkpoint" OFF)
option(FCEUX11_UBSAN "Enable MSVC-native UB runtime checks (/RTC1 + /sdl); clang-style UBSan deferred to v0.4.x" OFF)

if(FCEUX11_ASAN OR FCEUX11_UBSAN)
    if(NOT MSVC)
        message(FATAL_ERROR
            "FCEUX11_ASAN / FCEUX11_UBSAN require the MSVC toolchain.")
    endif()
    # Sanitizers require debug-friendly builds — opt-in by user, but recommend RelWithDebInfo for ASan.
    if(NOT CMAKE_BUILD_TYPE STREQUAL "Debug" AND
       NOT CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
        message(WARNING
            "FCEUX11_ASAN/UBSAN is most useful with CMAKE_BUILD_TYPE=Debug or "
            "RelWithDebInfo. Current: ${CMAKE_BUILD_TYPE}. Continuing anyway.")
    endif()
endif()

if(FCEUX11_ASAN)
    message(STATUS "FCEUX11: AddressSanitizer ENABLED (/fsanitize=address /Zi, LSan bundled)")
    # /Zi: emit symbols so ASan stacks resolve to source lines (silences C5072).
    add_compile_options(/fsanitize=address /Zi)
    # Use CMAKE_*_LINKER_FLAGS rather than add_link_options — Ninja
    # misinterprets `/fsanitize=address` as a build input when injected
    # through the add_link_options() path (it sees the leading `/` as a
    # file dependency). CMAKE_*_LINKER_FLAGS go directly into the link
    # rule's args, no escaping issues. See [[test-runs-and-pitfalls]].
    set(CMAKE_EXE_LINKER_FLAGS_DEBUG   "/fsanitize=address ${CMAKE_EXE_LINKER_FLAGS_DEBUG}")
    set(CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize=address ${CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO}")
    set(CMAKE_SHARED_LINKER_FLAGS_DEBUG   "/fsanitize=address ${CMAKE_SHARED_LINKER_FLAGS_DEBUG}")
    set(CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO "/fsanitize=address ${CMAKE_SHARED_LINKER_FLAGS_RELWITHDEBINFO}")
    # v0.3.6.5 followup: Release defaults omit /DEBUG, so PDBs aren't
    # emitted and ASan's bundled llvm-symbolizer can't resolve stack
    # frames (the F-1 finding shows up as <exe>+0xRVA instead of
    # file:line). Force /DEBUG on Release + ASan; /DEBUG flips OPT
    # defaults to NOREF/NOICF which bloats the exe — restore Release
    # size with explicit /OPT:REF /OPT:ICF. Only affects ASan Release;
    # Debug/RelWithDebInfo already carry /DEBUG via CMake defaults.
    set(CMAKE_EXE_LINKER_FLAGS_RELEASE
        "/fsanitize=address /DEBUG /OPT:REF /OPT:ICF ${CMAKE_EXE_LINKER_FLAGS_RELEASE}")
    set(CMAKE_SHARED_LINKER_FLAGS_RELEASE
        "/fsanitize=address /DEBUG /OPT:REF /OPT:ICF ${CMAKE_SHARED_LINKER_FLAGS_RELEASE}")
    set(ASAN_LDFLAGS "/fsanitize=address" CACHE INTERNAL "ASan link flags")
    add_definitions(-DFCEUX11_ASAN_ENABLED=1)
endif()

if(FCEUX11_UBSAN)
    # MSVC-native UB-substitute. /RTC1 is Debug-only (cl rejects it
    # with C4348 / C4651 against /O2). /sdl /GS /guard:cf already on.
    if(CMAKE_BUILD_TYPE STREQUAL "Debug")
        message(STATUS "FCEUX11: UB-substitute ENABLED (/RTC1 + /sdl, Debug — full subset)")
        add_compile_options(/RTC1)
    else()
        message(STATUS
            "FCEUX11: UB-substitute partial — /RTC1 requires Debug build "
            "(current: ${CMAKE_BUILD_TYPE}), falling back to /sdl + /GS + "
            "/guard:cf already-on globally. Use -DCMAKE_BUILD_TYPE=Debug "
            "for full coverage.")
    endif()
    # No linker-side flags for MSVC native UB checks.
    set(UBSAN_LDFLAGS "" CACHE INTERNAL "UBSan link flags (MSVC native: none)")
    add_definitions(-DFCEUX11_UBSAN_ENABLED=1)
endif()

# Export compile_commands.json for IDE integration (Visual Studio Code,
# CLion, Qt Creator, etc.). The v0.3.2 wording said "for clang-tidy / IDE
# integration" but clang-tidy has been removed from the toolchain per the
# v0.3.6.6 policy; compile_commands.json is still useful for IDEs.
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

# v0.3.6.6: Optional lint target — cppcheck only.
# The v0.3.2 `if(ENABLE_LINT)` block had two halves: a clang-tidy branch
# (now removed — project is MSVC-only with no clang toolchain) and a
# cppcheck branch. cppcheck is a separate, non-clang static analyzer and
# stays in the build. Invoke with:
#   cmake -S . -B build -DENABLE_LINT_CPPCHECK=ON
#   cmake --build build --target lint-cppcheck
if(ENABLE_LINT_CPPCHECK)
    find_program(CPPCHECK_PROG cppcheck)
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
    else()
        message(STATUS "cppcheck not found in PATH — lint-cppcheck target disabled")
    endif()
endif()
