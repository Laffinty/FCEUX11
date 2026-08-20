# Build the cycle_trace harness under FCEUX11_RUST_CPU=OFF.
# Writes outputs to Z:\Project\FCEUX11\build-off\.
$ErrorActionPreference = "Stop"
$ProjectRoot = "Z:\Project\FCEUX11"
$BuildDir = Join-Path $ProjectRoot "build-off"

# Clean any stale build-off directory.
if (Test-Path $BuildDir) {
    Remove-Item -Recurse -Force $BuildDir
}
New-Item -ItemType Directory -Path $BuildDir | Out-Null

# Load MSVC environment via cmd /c shell-out (vcvars requires cmd semantics).
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
$vcvarsOut = & cmd /c "`"$vcvars`" >nul 2>&1 && set"
$vcvarsOut -split "`r?`n" | ForEach-Object {
    if ($_ -match "^([^=]+)=(.*)$") {
        $name = $matches[1]; $value = $matches[2]
        Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
    }
}

# Add Ninja + vcpkg bin to PATH.
$ninjaDir = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"
$vcpkgBin = Join-Path $ProjectRoot "vcpkg_installed\x64-windows\bin"
$env:PATH = "$ninjaDir;$vcpkgBin;$env:PATH"

# Configure build-off with Rust enabled but Rust CPU OFF.
Set-Location $BuildDir
& cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER=cl -DCMAKE_CXX_COMPILER=cl `
    -DCMAKE_PREFIX_PATH=$ProjectRoot\vcpkg_installed\x64-windows `
    -DVCPKG_MANIFEST_MODE=OFF `
    -DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE `
    -DFCEUX11_ENABLE_I18N=OFF `
    -DFCEUX11_RUST_CPU=OFF `
    -DFCEUX11_BUILD_TESTS=ON `
    -DFCEUX11_ENABLE_RUST=ON `
    $ProjectRoot
if ($LASTEXITCODE -ne 0) { throw "cmake configure failed" }

# Build ONLY the cycle_trace target (skip pre-existing broken test targets).
& cmake --build . --target kagami_qa_cycle_trace
if ($LASTEXITCODE -ne 0) { throw "build failed" }

# Confirm the binary exists.
$bin = Join-Path $BuildDir "tests\kagami_qa_cycle_trace.exe"
if (-not (Test-Path $bin)) { throw "binary missing: $bin" }

Write-Host "[OK] $bin ($(((Get-Item $bin).Length) / 1KB)KB)"
