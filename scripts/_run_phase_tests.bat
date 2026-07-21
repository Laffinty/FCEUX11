@echo off
cd /d "D:\Project\FCEUX11\tests"
echo === ppu_rendering_lut (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_ppu_rendering_lut_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === ppu_phase_c (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_ppu_phase_c_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === ppu_phase_d (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_ppu_phase_d_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === cart_class_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_cart_class_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === driver_callbacks_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_driver_callbacks_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === core_driver_boundary_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_core_driver_boundary_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === ppu_frame_diff_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_ppu_frame_diff_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === golden_savestate_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_golden_savestate_test.exe"
echo exit=%ERRORLEVEL%
