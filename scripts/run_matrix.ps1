$ErrorActionPreference = "Continue"
$ROOT = 'D:\Project\FCEUX11'
Set-Location $ROOT
$env:PATH = "$ROOT\build-c1\tests;$ROOT\vcpkg_installed\x64-windows\bin;$ROOT\vcpkg_installed\x64-windows\debug\bin;" + $env:PATH
$env:FCEUX11_GIT_REV = & git rev-parse --short HEAD

& "$ROOT\src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe" `
    --manifest "$ROOT\tests\tests.json" `
    --bin-dir  "$ROOT\build-c1\tests" `
    --baseline  "$ROOT\build-c1\kagamiqa_baseline.json" `
    --output    "$ROOT\build-c1\kagamiqa_migration_matrix.json"
Write-Host ("EXIT=" + $LASTEXITCODE)
