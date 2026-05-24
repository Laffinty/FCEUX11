# FCEUX11 Build Script (v0.2.1)
# Pure PowerShell — no MSYS2 / MinGW / POSIX dependencies
param(
    [ValidateSet("Debug", "Release", "RelWithDebInfo")]
    [string]$Config = "Release",

    [string]$BuildDir = "build",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot

if ($Clean -and (Test-Path $BuildDir)) {
    Write-Host "[CLEAN] Removing $BuildDir" -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BuildDir
}

# Ensure vcpkg toolchain is discoverable
$vcpkgToolchain = $null
if ($env:VCPKG_ROOT) {
    $vcpkgToolchain = Join-Path $env:VCPKG_ROOT "scripts\buildsystems\vcpkg.cmake"
}

$cmakeArgs = @(
    "-S", $ProjectRoot
    "-B", $BuildDir
    "-G", "Ninja"
    "-DCMAKE_BUILD_TYPE=$Config"
    "-DCMAKE_C_COMPILER=cl"
    "-DCMAKE_CXX_COMPILER=cl"
)
if ($vcpkgToolchain -and (Test-Path $vcpkgToolchain)) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$vcpkgToolchain"
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

Write-Host "[BUILD] cmake --build $BuildDir --config $Config" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config
if ($LASTEXITCODE -ne 0) { throw "CMake build failed" }

Write-Host "[TEST] ctest --test-dir $BuildDir --output-on-failure" -ForegroundColor Cyan
& ctest --test-dir $BuildDir --output-on-failure
if ($LASTEXITCODE -ne 0) { throw "CTest failed" }

Write-Host "[SUCCESS] Build complete: $BuildDir" -ForegroundColor Green
