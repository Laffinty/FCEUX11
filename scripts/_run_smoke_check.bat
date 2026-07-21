@echo off
cd /d "D:\Project\FCEUX11"
echo === Direct smoke test run from project root with test exe path ===
.\build\tests\fceux11_smoke_test.exe
echo === ps exit: %ERRORLEVEL% ===
echo.
echo === cpu_test with project root CWD ===
.\build\tests\fceux11_cpu_test.exe
echo === ps exit: %ERRORLEVEL% ===
