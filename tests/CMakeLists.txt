
# Disable C++20 module scanning for test targets (not needed, avoids
# MSVC standard-library header path issues in some environments).
set(CMAKE_CXX_SCAN_FOR_MODULES OFF)

# Shared link configuration for test executables
function(fceux11_add_test_executable target_name)
    add_executable(${target_name} ${ARGN})

    target_link_libraries(${target_name} PRIVATE
        fceux11_drivers_qt
        fceux11_drivers_common
        fceux11_core
        fceux11_boards
        fceux11_utils
    )

    if(FCEUX11_RUST_ENABLED AND FCEUX11_RUST_STATIC_LIB)
        add_dependencies(${target_name} fceux11_rust_build)
        target_link_libraries(${target_name} PRIVATE
            ${FCEUX11_RUST_STATIC_LIB}
        )
    endif()

    if(WIN32)
        target_link_libraries(${target_name} PRIVATE ntdll)
    endif()

    target_include_directories(${target_name} PRIVATE
        ${CMAKE_SOURCE_DIR}/src
        ${CMAKE_SOURCE_DIR}/src/drivers
    )

    # v0.3.9: tests link against fceux11_core, which transitively pulls
    # in version.h. Without __QT_DRIVER__ defined, version.h hits the
    # MSVC-only `#include "scmrev.h"` branch (pre-existing build
    # infrastructure gap; the generator script for scmrev.h was
    # removed in the v0.2.x `vc/` cleanup). Defining __QT_DRIVER__ on
    # test targets mirrors src/CMakeLists.txt:82 — the canonical
    # escape hatch the header was designed for. See plan v3 §5
    # v0.3.9 acceptance gate 2 (ctest pass) — unblocking this is a
    # test-infrastructure fix, not a v0.3.9 API change.
    target_compile_definitions(${target_name} PRIVATE __QT_DRIVER__)
endfunction()

if(FCEUX11_BUILD_TESTS)
    # Smoke test (Task 1.0)
    fceux11_add_test_executable(fceux11_smoke_test
        smoke_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME smoke_test COMMAND fceux11_smoke_test)

    # Mapper regression tests (Task 1.1)
    fceux11_add_test_executable(fceux11_mapper_load_test
        boards/mapper_load_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME mapper_load_test
             COMMAND fceux11_mapper_load_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_mapper_reset_test
        boards/mapper_reset_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME mapper_reset_test
             COMMAND fceux11_mapper_reset_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # ROM regression test (v0.2.15 / v0.3.0)
    fceux11_add_test_executable(fceux11_rom_regression_test
        rom_regression_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME rom_regression_test
             COMMAND fceux11_rom_regression_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # v0.3.12.5 — byte-level savestate consistency test
    fceux11_add_test_executable(fceux11_savestate_regression_test
        savestate_regression_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME savestate_regression_test
             COMMAND fceux11_savestate_regression_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # v0.3.3 — tl::expected API tests
    fceux11_add_test_executable(fceux11_expected_api_test
        expected_api_test.cpp
        git_info_stub.cpp
        ${CMAKE_SOURCE_DIR}/src/utils/fceu11_expected.cpp
    )
    add_test(NAME expected_api_test
             COMMAND fceux11_expected_api_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # v0.3.8 — FCEU_ENUM_CLASS_BITFLAGS macro tests.
    # Header-only / pure-C++ test: no engine link, no NES core init.
    # Uses a standalone add_executable rather than
    # fceux11_add_test_executable to skip the heavy engine link.
    add_executable(fceux11_enum_class_bitflags_test
        enum_class_bitflags_test.cpp
    )
    target_include_directories(fceux11_enum_class_bitflags_test PRIVATE
        ${CMAKE_SOURCE_DIR}/src)
    add_test(NAME enum_class_bitflags_test
             COMMAND fceux11_enum_class_bitflags_test)

    # v0.3.15 PHASE-2: i18n regression test (coverage + simp/trad + widget
    # retranslateUi/changeEvent/keyPress static-analysis gate). Standalone
    # executable — does not link fceux11_drivers_qt (which would pull in
    # the full Qt application and require a display server / OpenGL ctx
    # to construct). The test reads .ts files and source files only.
    add_executable(fceux11_i18n_regression_test
        i18n_regression_test.cpp
    )
    target_include_directories(fceux11_i18n_regression_test PRIVATE
        ${CMAKE_SOURCE_DIR})
    add_test(NAME i18n_regression_test
             COMMAND fceux11_i18n_regression_test
             WORKING_DIRECTORY ${CMAKE_SOURCE_DIR})

    # -------------------------------------------------------------------------
    # v0.3.6.5 / v0.3.14 — Windows test runtime PATH injection
    # MSVC ASan-instrumented binaries dynamically import
    # clang_rt.asan_dynamic-x86_64.dll from the MSVC toolchain bin
    # directory. All Windows test executables dynamically import vcpkg
    # DLLs (Qt6Core/SDL2/etc or their `d` debug variants). Without these
    # directories on PATH, every test exits with STATUS_DLL_NOT_FOUND
    # (0xc0000135) before main() runs. The PATH injection is therefore
    # required on Win32 regardless of sanitizer settings.
    # -------------------------------------------------------------------------
    if(WIN32)
        file(TO_NATIVE_PATH
             "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows/bin"
             _FCEUX11_VCPKG_REL_BIN_NATIVE)
        file(TO_NATIVE_PATH
             "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows/debug/bin"
             _FCEUX11_VCPKG_DBG_BIN_NATIVE)

        if(FCEUX11_ASAN OR FCEUX11_UBSAN)
            get_filename_component(_FCEUX11_MSVC_BIN_DIR
                                   "${CMAKE_CXX_COMPILER}" DIRECTORY)
            file(TO_NATIVE_PATH "${_FCEUX11_MSVC_BIN_DIR}"
                                _FCEUX11_MSVC_BIN_NATIVE)
            set(_FCEUX11_TEST_PATH_MODIFICATIONS
                "PATH=path_list_prepend:${_FCEUX11_MSVC_BIN_NATIVE};PATH=path_list_prepend:${_FCEUX11_VCPKG_REL_BIN_NATIVE};PATH=path_list_prepend:${_FCEUX11_VCPKG_DBG_BIN_NATIVE}")
            message(STATUS
                "FCEUX11 tests: PATH prepended for sanitizer runtime — "
                "${_FCEUX11_MSVC_BIN_NATIVE}; ${_FCEUX11_VCPKG_REL_BIN_NATIVE}; ${_FCEUX11_VCPKG_DBG_BIN_NATIVE}")
        else()
            set(_FCEUX11_TEST_PATH_MODIFICATIONS
                "PATH=path_list_prepend:${_FCEUX11_VCPKG_REL_BIN_NATIVE};PATH=path_list_prepend:${_FCEUX11_VCPKG_DBG_BIN_NATIVE}")
            message(STATUS
                "FCEUX11 tests: PATH prepended for vcpkg runtime — "
                "${_FCEUX11_VCPKG_REL_BIN_NATIVE}; ${_FCEUX11_VCPKG_DBG_BIN_NATIVE}")
        endif()

        set_tests_properties(
            smoke_test mapper_load_test mapper_reset_test
            rom_regression_test savestate_regression_test expected_api_test
            enum_class_bitflags_test
            PROPERTIES ENVIRONMENT_MODIFICATION
                "${_FCEUX11_TEST_PATH_MODIFICATIONS}"
        )
    endif()

    # -------------------------------------------------------------------------
    # v0.3.0 — Performance baseline benchmarks (Google Benchmark preferred)
    # -------------------------------------------------------------------------
    find_package(benchmark CONFIG QUIET)
    if(benchmark_FOUND)
        message(STATUS "Google Benchmark found — building benchmark targets")
        add_executable(fceux11_bench_ppu_render benchmark/ppu_render_bench.cpp git_info_stub.cpp)
        target_link_libraries(fceux11_bench_ppu_render PRIVATE
            fceux11_drivers_qt fceux11_drivers_common fceux11_core fceux11_boards fceux11_utils
            benchmark::benchmark
        )
        target_include_directories(fceux11_bench_ppu_render PRIVATE ${CMAKE_SOURCE_DIR}/src ${CMAKE_SOURCE_DIR}/src/drivers)
        # v0.3.9: see comment in fceux11_add_test_executable — bench
        # targets also need __QT_DRIVER__ to bypass the version.h
        # scmrev.h include (pre-existing build infrastructure gap).
        target_compile_definitions(fceux11_bench_ppu_render PRIVATE __QT_DRIVER__)
        if(WIN32)
            target_link_libraries(fceux11_bench_ppu_render PRIVATE ntdll)
        endif()

        add_executable(fceux11_bench_x6502_exec benchmark/x6502_exec_bench.cpp git_info_stub.cpp)
        target_link_libraries(fceux11_bench_x6502_exec PRIVATE
            fceux11_drivers_qt fceux11_drivers_common fceux11_core fceux11_boards fceux11_utils
            benchmark::benchmark
        )
        target_include_directories(fceux11_bench_x6502_exec PRIVATE ${CMAKE_SOURCE_DIR}/src ${CMAKE_SOURCE_DIR}/src/drivers)
        target_compile_definitions(fceux11_bench_x6502_exec PRIVATE __QT_DRIVER__)
        if(WIN32)
            target_link_libraries(fceux11_bench_x6502_exec PRIVATE ntdll)
        endif()

        add_executable(fceux11_bench_apu_mix benchmark/apu_mix_bench.cpp git_info_stub.cpp)
        target_link_libraries(fceux11_bench_apu_mix PRIVATE
            fceux11_drivers_qt fceux11_drivers_common fceux11_core fceux11_boards fceux11_utils
            benchmark::benchmark
        )
        target_include_directories(fceux11_bench_apu_mix PRIVATE ${CMAKE_SOURCE_DIR}/src ${CMAKE_SOURCE_DIR}/src/drivers)
        target_compile_definitions(fceux11_bench_apu_mix PRIVATE __QT_DRIVER__)
        if(WIN32)
            target_link_libraries(fceux11_bench_apu_mix PRIVATE ntdll)
        endif()
    else()
        message(STATUS "Google Benchmark not found — building standalone benchmark executables")
        fceux11_add_test_executable(fceux11_bench_ppu_render   benchmark/ppu_render_bench.cpp   git_info_stub.cpp)
        fceux11_add_test_executable(fceux11_bench_x6502_exec   benchmark/x6502_exec_bench.cpp   git_info_stub.cpp)
        fceux11_add_test_executable(fceux11_bench_apu_mix      benchmark/apu_mix_bench.cpp      git_info_stub.cpp)
    endif()

    # v0.3.11 Phase-2 SIMD probe — standalone, build-only, not registered as CTest.
    add_executable(fceux11_ppu_simd_probe_scalar benchmark/ppu_simd_probe.cpp)
    target_compile_definitions(fceux11_ppu_simd_probe_scalar PRIVATE SCALAR_PROBE)

    add_executable(fceux11_ppu_simd_probe_sse42 benchmark/ppu_simd_probe.cpp)
    target_compile_options(fceux11_ppu_simd_probe_sse42 PRIVATE /arch:SSE2)

    add_executable(fceux11_ppu_simd_probe_avx2 benchmark/ppu_simd_probe.cpp)
    target_compile_options(fceux11_ppu_simd_probe_avx2 PRIVATE /arch:AVX2)
endif()
