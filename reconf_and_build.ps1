$ErrorActionPreference = "Continue"
$env:VCPKG_ROOT = 'Z:\Project\FCEUX11\vcpkg'

# Use vswhere to find VS 2022 Build Tools.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null

$vcvars = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
$cmd = "`"$vcvars`" && set"
$envLines = & cmd /c $cmd
foreach ($line in $envLines) {
    if ($line -match "^([^=]+)=(.*)$") {
        Set-Item -Path "Env:$($matches[1])" -Value $matches[2]
    }
}

$ninja = (Get-Command ninja.exe -ErrorAction SilentlyContinue).Source
if ($ninja) {
    $env:Path = "$env:Path;$(Split-Path $ninja -Parent)"
}

Set-Location 'Z:\Project\FCEUX11'

# Configure OFF path (vcvars is loaded into this process's env, so the
# cmake subprocess inherits it).
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release 2>&1 | Out-Null

# Wrap cmake --build in vcvars via cmd /c so the ninja subprocess
# inherits the SDK / MSVC tools include + lib paths. Without this,
# any C/C++ TU that uses MSVC headers (windows.h, stdio.h, etc.)
# fails C1083 because ninja's cl.exe invocation has no SDK search path.
$cmd = "`"$vcvars`" && cd /d `"Z:\Project\FCEUX11`" && cmake --build build --config Release --target fceux11_rust_ppu_smoke_test"
Write-Host "=== Building fceux11_rust_ppu_smoke_test (via vcvars shell) ==="
& cmd /c $cmd 2>&1
$rc = $LASTEXITCODE
Write-Host "=== Build exit code: $rc ==="

if ($rc -eq 0) {
    Write-Host "=== Running ctest ==="
    $cmd2 = "`"$vcvars`" && cd /d `"Z:\Project\FCEUX11`" && ctest --test-dir build --build-config Release --output-on-failure -LE perf"
    & cmd /c $cmd2 2>&1 | Select-Object -Last 10
}
exit $rc