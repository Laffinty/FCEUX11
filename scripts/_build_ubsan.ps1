# FCEUX11 v0.3.6.5 — Build with MSVC-native UB runtime checks
#
# MSVC does NOT implement /fsanitize=undefined (clang-only). This
# script enables FCEUX11_UBSAN=ON which maps to /RTC1 (Debug-only)
# + /sdl + /GS + /guard:cf — see CMakeLists.txt comment block.
# Full clang-style UBSan is deferred to v0.4.x.
#
# Build type defaults to Debug so /RTC1 actually engages. Override
# with -Config RelWithDebInfo if you only want /sdl coverage.
param(
    [string]$BuildDir = "build-ubsan",
    [ValidateSet("Debug", "RelWithDebInfo")]
    [string]$Config = "Debug",
    [switch]$KeepCache
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}

if ((-not $KeepCache) -and (Test-Path $BuildDir)) {
    Write-Host "[CLEAN] Removing $BuildDir (stale cache may have buggy /fsanitize:undefined)" -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BuildDir
}

$localVcpkg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows"
if (-not (Test-Path $localVcpkg)) {
    throw "vcpkg_installed\x64-windows not found at $localVcpkg."
}

$cmakeArgs = @(
    "-S", $ProjectRoot
    "-B", $BuildDir
    "-G", "Ninja"
    "-DCMAKE_BUILD_TYPE=$Config"
    "-DCMAKE_C_COMPILER=cl"
    "-DCMAKE_CXX_COMPILER=cl"
    "-DCMAKE_PREFIX_PATH=$localVcpkg"
    "-DVCPKG_MANIFEST_MODE=OFF"
    "-DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE"
    "-DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE"
    "-DFCEUX11_BUILD_TESTS=ON"
    "-DENABLE_LINT=OFF"
    "-DFCEUX11_UBSAN=ON"
)

$linguistDir = Join-Path $localVcpkg "share\Qt6LinguistTools"
if (-not (Test-Path $linguistDir)) {
    $cmakeArgs += "-DFCEUX11_ENABLE_I18N=OFF"
    Write-Host "[WARN] Qt6LinguistTools not found; disabling i18n" -ForegroundColor Yellow
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

$buildLog = Join-Path $BuildDir "_build_ubsan.log"
Write-Host "[BUILD] cmake --build $BuildDir --config $Config (tee → $buildLog)" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config 2>&1 | Tee-Object -FilePath $buildLog
if ($LASTEXITCODE -ne 0) { throw "CMake build failed" }

$d9002 = Select-String -Path $buildLog -Pattern "warning D9002" -SimpleMatch -Quiet
if ($d9002) {
    Write-Host "[FAIL] D9002 warnings detected — a UB-check flag was rejected by cl. Check $buildLog." -ForegroundColor Red
    throw "UB-substitute build produced D9002."
}

Write-Host "[SUCCESS] UB-substitute build complete: $BuildDir (0 D9002 warnings)" -ForegroundColor Green
Write-Host "Next: scripts\_ctest_ubsan.ps1" -ForegroundColor Gray
