@echo off
cd /d "D:\Project\FCEUX11\tests"
echo === savestate_regression (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_savestate_regression_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === rom_regression (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_rom_regression_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === cpu_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_cpu_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === ppu_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_ppu_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === apu_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_apu_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === bus_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_bus_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === i18n_regression (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_i18n_regression_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === expected_api_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_expected_api_test.exe"
echo exit=%ERRORLEVEL%
echo.
echo === mapper_load_test (CWD=tests) ===
"D:\Project\FCEUX11\build\tests\fceux11_mapper_load_test.exe"
echo exit=%ERRORLEVEL%
