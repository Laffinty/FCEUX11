# _phase5_rebuild_rust_cpu.ps1
# Phase 5 cleanup — rebuild build-rust-cpu with FCEUX11_RUST_CPU=ON and run ctest.
# Loads MSVC vcvars64 (no auto-detection on plain PowerShell), then delegates to
# cmake/ninja/ctest. Reads existing CMakeCache for build type.
$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$BuildDir = Join-Path $ProjectRoot "build-rust-cpu"
$Config = "Release"
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
$ninja = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe"

if (-not (Test-Path $vcvars)) { throw "vcvars64.bat missing: $vcvars" }
if (-not (Test-Path $ninja)) { throw "ninja.exe missing: $ninja" }

Write-Host "[ENV] Loading MSVC from $vcvars" -ForegroundColor Yellow
$envVars = (& cmd /c "`"$vcvars`" >nul 2>&1 && set") -split "`r?`n"
foreach ($line in $envVars) {
    if ($line -match "^([^=]+)=(.*)$") {
        $name = $matches[1]
        $value = $matches[2]
        Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
    }
}
$env:PATH = (Split-Path -Parent $ninja) + ";" + $env:PATH

if (-not (Test-Path $BuildDir)) {
    Write-Host "[CONFIGURE] $BuildDir not present; running cmake configure" -ForegroundColor Cyan
    & cmake -S $ProjectRoot -B $BuildDir -G Ninja `
        "-DCMAKE_BUILD_TYPE=$Config" `
        "-DCMAKE_C_COMPILER=cl" `
        "-DCMAKE_CXX_COMPILER=cl" `
        "-DCMAKE_PREFIX_PATH=$ProjectRoot\vcpkg_installed\x64-windows" `
        "-DVCPKG_MANIFEST_MODE=OFF" `
        "-DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE" `
        "-DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE" `
        "-DFCEUX11_ENABLE_I18N=OFF" `
        "-DFCEUX11_RUST_CPU=ON" `
        "-DFCEUX11_BUILD_TESTS=ON" `
        "-DFCEUX11_ENABLE_RUST=ON"
    if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }
} else {
    Write-Host "[CONFIGURE] Using existing $BuildDir; verifying FCEUX11_RUST_CPU=ON" -ForegroundColor Cyan
    $cacheText = Get-Content (Join-Path $BuildDir "CMakeCache.txt") -Raw -ErrorAction SilentlyContinue
    if (-not $cacheText -or $cacheText -notmatch "(?m)^FCEUX11_RUST_CPU:BOOL=ON\s*$") {
        throw "build-rust-cpu cache does not have FCEUX11_RUST_CPU=ON. Re-run cmake configure or remove the directory."
    }
}

Write-Host "[BUILD] cmake --build $BuildDir --config $Config" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config 2>&1 | Tee-Object -Variable buildOutput
$buildExit = $LASTEXITCODE
if ($buildExit -ne 0) {
    Write-Host "[BUILD] exit=$buildExit" -ForegroundColor Red
    throw "Build failed"
}

# Ensure vcpkg DLLs are on PATH for test execution
$localVcpkg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows"
if (Test-Path $localVcpkg) {
    $env:PATH = "$localVcpkg\bin;$env:PATH"
}

Write-Host "[TEST] ctest --test-dir $BuildDir --output-on-failure -E perf" -ForegroundColor Cyan
& ctest --test-dir $BuildDir --output-on-failure -E perf
$testExit = $LASTEXITCODE
Write-Host "[DONE] build exit=$buildExit ctest exit=$testExit" -ForegroundColor Green
exit $testExit