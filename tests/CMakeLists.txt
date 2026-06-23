
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

    # v0.3.12.5 — byte-level savestate consistency test.
    # 12 ROMs × 60 frames ≈ 720 frames of emulation. On slow CI nodes this
    # Phase 0: per-frame watchdog now aborts hangs early, so the CTest
    # timeout can be tightened to 300s (5min) for fast CI failure.
    fceux11_add_test_executable(fceux11_savestate_regression_test
        savestate_regression_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME savestate_regression_test
             COMMAND fceux11_savestate_regression_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})
    set_tests_properties(savestate_regression_test PROPERTIES TIMEOUT 300)

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

    # v1.2 Census §2.2 — facade wiring test. Pure compile/link check;
    # does not call fceu11::Initialize() so it can run without a Qt event
    # loop. Verifies that every accessor returns a reference whose address
    # matches the underlying extern symbol, and that global_state() is a
    # true singleton.
    fceux11_add_test_executable(fceux11_core_state_test
        core_state_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME core_state_test
             COMMAND fceux11_core_state_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # -------------------------------------------------------------------------
    # v1.1 Sentinel — core unit-test skeleton (§1.1).
    # Each test file has ≥ 10 cases; together they constitute the
    # regression net for the v1.3-v1.6 objectification work.
    # -------------------------------------------------------------------------
    fceux11_add_test_executable(fceux11_cpu_test
        core/cpu_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME cpu_test
             COMMAND fceux11_cpu_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_ppu_test
        core/ppu_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME ppu_test
             COMMAND fceux11_ppu_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_apu_test
        core/apu_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME apu_test
             COMMAND fceux11_apu_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_bus_test
        core/bus_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME bus_test
             COMMAND fceux11_bus_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_mapper_core_test
        core/mapper_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME mapper_core_test
             COMMAND fceux11_mapper_core_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_savestate_core_test
        core/savestate_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME savestate_core_test
             COMMAND fceux11_savestate_core_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # v1.1 Sentinel §1.2 — golden savestate regression test.
    # The .fc0 binary files are NOT generated by the build (the source
    # ROMs in tests/fixtures/*.nes are the canonical input). To bootstrap
    # the goldens, run:
    #   fceux11_golden_savestate_test --generate
    # once, then commit the produced .fc0 files and updated index.
    fceux11_add_test_executable(fceux11_golden_savestate_test
        fixtures/golden/golden_savestate_test.cpp
        git_info_stub.cpp
    )
    # v1.2 Census: golden_savestate_test lives under fixtures/golden/, so it
    # needs an extra include directory to reach core/test_helpers.h.
    target_include_directories(fceux11_golden_savestate_test PRIVATE
        ${CMAKE_SOURCE_DIR}/tests/core
    )
    add_test(NAME golden_savestate_test
             COMMAND fceux11_golden_savestate_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    # v1.1 Sentinel §1.3 — bench tolerance test.
    # Default baseline is benchmarks/baseline_v1.0.json; override with
    # FCEUX11_BENCH_BASELINE=<path>. Run with --generate to write a
    # host-local baseline (not committed).
    fceux11_add_test_executable(fceux11_bench_tolerance_test
        benchmarks/bench_tolerance_test.cpp
        git_info_stub.cpp
    )
    # v1.2 Census: benchmarks/bench_tolerance_test is one directory deep;
    # needs tests/core in the include path to reach test_helpers.h.
    target_include_directories(fceux11_bench_tolerance_test PRIVATE
        ${CMAKE_SOURCE_DIR}/tests/core
    )
    add_test(NAME bench_tolerance_test
             COMMAND fceux11_bench_tolerance_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})
    # v1.4: wall-clock perf benchmarks are advisory on shared CI runners.
    # Inter-runner variance on GitHub Actions (20%+) dwarfs the 2.5%
    # tolerance, so gating the build on them produces flaky failures
    # unrelated to code changes. The "perf" label lets CI exclude this
    # test from the gating ctest run (`ctest -LE perf`) and run it as a
    # continue-on-error advisory step that still reports the numbers.
    set_tests_properties(bench_tolerance_test PROPERTIES LABELS "perf")

    # v0.3.15.x PHASE-4: TypedConfig<T> unit test. Constructs an
    # in-memory QSettings backend via a unique organisation name so
    # the test cannot clobber the user's real fceux11.ini. The
    # test links Core (QSettings, QCoreApplication) but not
    # Widgets/Gui, so it remains headless-runnable on CI.
    find_package(Qt6 REQUIRED COMPONENTS Core)
    add_executable(fceux11_config_store_test
        config_store_test.cpp
    )
    target_link_libraries(fceux11_config_store_test PRIVATE
        Qt6::Core)
    target_include_directories(fceux11_config_store_test PRIVATE
        ${CMAKE_SOURCE_DIR}/src
        ${CMAKE_SOURCE_DIR}/src/drivers)
    add_test(NAME config_store_test
             COMMAND fceux11_config_store_test
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
            enum_class_bitflags_test config_store_test
            core_state_test
            cpu_test ppu_test apu_test bus_test mapper_core_test
            savestate_core_test golden_savestate_test bench_tolerance_test
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
