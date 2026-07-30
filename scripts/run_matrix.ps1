$ErrorActionPreference = "Continue"
$ROOT = 'D:\Project\FCEUX11'
Set-Location $ROOT
$env:PATH = "$ROOT\build-c1\tests;$ROOT\vcpkg_installed\x64-windows\bin;$ROOT\vcpkg_installed\x64-windows\debug\bin;" + $env:PATH
# NOTE (S-4): the git revision is stamped at COMPILE time by
# src/rust/crates/kagami-qa/build.rs (report/matrix.rs uses option_env!).
# Setting FCEUX11_GIT_REV here would be too late to affect the report, so it is
# deliberately not set. Rebuild the runner to re-stamp:
#   cargo build --release -p kagami-qa --bin kagami-qa-runner

& "$ROOT\src\rust\target\x86_64-pc-windows-msvc\release\kagami-qa-runner.exe" `
    --manifest "$ROOT\tests\tests.json" `
    --bin-dir  "$ROOT\build-c1\tests" `
    --baseline  "$ROOT\build-c1\kagamiqa_baseline.json" `
    --output    "$ROOT\build-c1\kagamiqa_migration_matrix.json"
Write-Host ("EXIT=" + $LASTEXITCODE)
