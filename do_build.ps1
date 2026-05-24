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

# Auto-detect available generator
$generator = $null
$ninjaOk = $false
try { & ninja --version | Out-Null; $ninjaOk = ($LASTEXITCODE -eq 0) } catch {}
if ($ninjaOk) {
    $generator = "Ninja"
} elseif (Get-Command nmake -ErrorAction SilentlyContinue) {
    $generator = "NMake Makefiles"
} else {
    # Attempt to auto-load VS 2022+ BuildTools environment
    $vcvarsPaths = @(
        "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
        "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
        "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
        "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
        "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
    )
    $vcvars = $vcvarsPaths | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($vcvars) {
        Write-Host "[ENV] Loading VS environment from $vcvars" -ForegroundColor Yellow
        $envVars = (& cmd /c "`"$vcvars`" >nul 2>&1 && set") -split "`r?`n"
        foreach ($line in $envVars) {
            if ($line -match "^([^=]+)=(.*)$") {
                $name = $matches[1]
                $value = $matches[2]
                Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
            }
        }
        if (Get-Command nmake -ErrorAction SilentlyContinue) {
            $generator = "NMake Makefiles"
        } else {
            throw "Failed to load nmake after running vcvars64.bat"
        }
    } else {
        throw "No supported build generator found (Ninja or NMake). Install Ninja (recommended) or ensure nmake is on PATH."
    }
}

$cmakeArgs = @(
    "-S", $ProjectRoot
    "-B", $BuildDir
    "-G", $generator
    "-DCMAKE_BUILD_TYPE=$Config"
    "-DCMAKE_C_COMPILER=cl"
    "-DCMAKE_CXX_COMPILER=cl"
)

# Prefer existing local vcpkg_installed over system vcpkg to avoid re-building packages
$localVcpkg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows"
if (Test-Path $localVcpkg) {
    $cmakeArgs += "-DCMAKE_PREFIX_PATH=$localVcpkg"
    Write-Host "[INFO] Using local vcpkg_installed: $localVcpkg" -ForegroundColor Gray
    # If qttools (LinguistTools) is missing, disable i18n to avoid configure failure
    $linguistDir = Join-Path $localVcpkg "share\Qt6LinguistTools"
    if (-not (Test-Path $linguistDir)) {
        $cmakeArgs += "-DFCEUX11_ENABLE_I18N=OFF"
        Write-Host "[WARN] Qt6LinguistTools not found; disabling i18n support" -ForegroundColor Yellow
    }
    # Disable vcpkg manifest mode to prevent rebuilding packages from source
    $cmakeArgs += "-DVCPKG_MANIFEST_MODE=OFF"
    # Map missing debug configs to release (some vcpkg Qt debug tool wrappers are absent)
    $cmakeArgs += "-DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE"
    $cmakeArgs += "-DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE"
} elseif ($vcpkgToolchain -and (Test-Path $vcpkgToolchain)) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$vcpkgToolchain"
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

Write-Host "[BUILD] cmake --build $BuildDir --config $Config" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config
if ($LASTEXITCODE -ne 0) { throw "CMake build failed" }

Write-Host "[TEST] ctest --test-dir $BuildDir --output-on-failure" -ForegroundColor Cyan
# Ensure vcpkg DLLs are on PATH for test execution
$env:PATH = "$localVcpkg\bin;$env:PATH"
& ctest --test-dir $BuildDir --output-on-failure
if ($LASTEXITCODE -ne 0) { throw "CTest failed" }

Write-Host "[SUCCESS] Build complete: $BuildDir" -ForegroundColor Green
