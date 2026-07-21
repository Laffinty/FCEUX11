@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "D:\Project\FCEUX11\scripts\_run_smoke_tests.ps1" > "D:\Project\FCEUX11\output\smoke_run.log" 2>&1
echo === Exit %ERRORLEVEL% ===
endlocal
