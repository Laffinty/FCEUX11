# FCEUX11 v0.3.6.5 — Build with real MSVC AddressSanitizer
#
# /fsanitize=address (equals form, MSVC official). Earlier v0.3.6.5
# attempt used `:address` (colon) which cl silently dropped via D9002 —
# do NOT regress to that. See docs/tech/v0.3.x_Checkpoint_6.5.md §2.
#
# Output layout: build-asan/ at project root.
# Build type:    Release by default — matches main build/ and avoids
#                vcpkg LibArchive's missing IMPORTED_LOCATION_RELWITHDEBINFO.
#                ASan emits /Zi explicitly (via root CMakeLists.txt) so
#                Release builds still get symbol resolution.
#                Use -Config Debug if you want full /RTC1 + ASan combo.
param(
    [string]$BuildDir = "build-asan",
    [ValidateSet("Debug", "Release", "RelWithDebInfo")]
    [string]$Config = "Release",
    [switch]$Clean,
    [switch]$KeepCache
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}

# Default behavior: wipe the cache, since a prior v0.3.6.5 cache may
# contain the buggy `/fsanitize:address` (colon) flag baked into the
# cached ASAN_LDFLAGS / compile rules. Use -KeepCache to opt in to
# incremental builds once you have a known-good sanitizer cache.
if ((-not $KeepCache) -and (Test-Path $BuildDir)) {
    Write-Host "[CLEAN] Removing $BuildDir (a stale sanitizer cache may bake in the wrong /fsanitize: flag form). Use -KeepCache to opt in to incremental builds." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BuildDir
}

$localVcpkg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows"
if (-not (Test-Path $localVcpkg)) {
    throw "vcpkg_installed\x64-windows not found at $localVcpkg. Run scripts\setup_vcpkg.ps1 first."
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
    "-DENABLE_LINT_CPPCHECK=OFF"
    "-DFCEUX11_ASAN=ON"
)

# i18n probe — same logic as do_build.ps1.
$linguistDir = Join-Path $localVcpkg "share\Qt6LinguistTools"
if (-not (Test-Path $linguistDir)) {
    $cmakeArgs += "-DFCEUX11_ENABLE_I18N=OFF"
    Write-Host "[WARN] Qt6LinguistTools not found; disabling i18n" -ForegroundColor Yellow
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

$buildLog = Join-Path $BuildDir "_build_asan.log"
Write-Host "[BUILD] cmake --build $BuildDir --config $Config (tee → $buildLog)" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config 2>&1 | Tee-Object -FilePath $buildLog
if ($LASTEXITCODE -ne 0) { throw "CMake build failed" }

# Sanity: D9002 ("ignoring unknown option") in the log means a sanitizer
# flag is wrong (likely a regression to /fsanitize: colon form). Fail loud.
$d9002 = Select-String -Path $buildLog -Pattern "warning D9002" -SimpleMatch -Quiet
if ($d9002) {
    Write-Host "[FAIL] D9002 warnings detected — sanitizer flag rejected by cl. Check $buildLog." -ForegroundColor Red
    throw "ASan build produced D9002; sanitizer is NOT instrumented."
}

Write-Host "[SUCCESS] ASan build complete: $BuildDir (0 D9002 warnings)" -ForegroundColor Green
Write-Host "Next: scripts\_verify_asan_instrumentation.ps1 then scripts\_ctest_asan.ps1" -ForegroundColor Gray
