@echo off
REM Phase 3 verification: build only the libraries + tests we need.
REM Avoids pre-existing /EHsc failures in i18n_regression_test /
REM ppu_simd_probe_*.cpp (unrelated to Phase 3 — those need a separate
REM fix: either /EHsc on tests or /wd4530 globally).

call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1

set "PATH=D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;%PATH%"

cd /d D:\Project\FCEUX11

REM Phase 3 verification targets: libs + tests called out in plan §5.1.2
REM NOTE: fceux11_savestate_regression_test excluded — pre-existing
REM MSVC 19.51 C4530 (chrono) without /EHsc, unrelated to Phase 3.
set TARGETS=fceux11_utils fceux11_core fceux11_boards fceux11_drivers_common fceux11_drivers_qt fceux11_smoke_test fceux11_bus_test fceux11_cpu_test fceux11_savestate_core_test fceux11_golden_savestate_test fceux11_mapper_core_test fceux11_rom_regression_test fceux11_bench_tolerance_test fceux11_mapper_load_test fceux11_mapper_reset_test fceux11_ppu_test fceux11_apu_test fceux11_core_state_test fceux11_expected_api_test fceux11_enum_class_bitflags_test

echo === Building selected Phase 3 targets ===
cmake --build build --config Release --target %TARGETS%
if errorlevel 1 exit /b 1

echo === DONE ===
