@echo off
cd /d "D:\Project\FCEUX11"
echo === smoke ===
.\build\tests\fceux11_smoke_test.exe
echo exit=%ERRORLEVEL%
echo.
echo === pixbuf_pool ===
.\build\tests\fceux11_pixbuf_pool_test.exe
echo exit=%ERRORLEVEL%
echo.
echo === enum_class_bitflags ===
.\build\tests\fceux11_enum_class_bitflags_test.exe
echo exit=%ERRORLEVEL%
echo.
echo === config_store ===
.\build\tests\fceux11_config_store_test.exe
echo exit=%ERRORLEVEL%
echo.
echo === savestate_regression ===
.\build\tests\fceux11_savestate_regression_test.exe
echo exit=%ERRORLEVEL%
echo.
echo === rom_regression ===
.\build\tests\fceux11_rom_regression_test.exe
echo exit=%ERRORLEVEL%
