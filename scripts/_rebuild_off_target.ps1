$ErrorActionPreference = "Stop"

# Load MSVC env via cmd.exe (vcvars64.bat is .bat, must run in cmd shell)
$vcvarsOut = & cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat`" >nul 2>&1 && set"
foreach ($line in ($vcvarsOut -split "`r?`n")) {
    if ($line -match "^([^=]+)=(.*)$") {
        $name = $matches[1]; $value = $matches[2]
        Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
    }
}

# Add Ninja + vcpkg bin to PATH.
$ninjaDir = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"
$vcpkgBin = "Z:\Project\FCEUX11\vcpkg_installed\x64-windows\bin"
$env:PATH = "$ninjaDir;$vcpkgBin;$env:PATH"

# Wipe cmake_pch state so the new x6502.cpp compiles cleanly.
$cacheDir = "Z:\Project\FCEUX11\build-off\src\CMakeFiles\fceux11_core.dir"
foreach ($f in @("cmake_pch.hxx.pch", "cmake_pch.cxx.pch", "cmake_pch.cxx.obj")) {
    $p = Join-Path $cacheDir $f
    if (Test-Path $p) { Remove-Item -Force $p }
}

# Build the OFF cycle_trace target.
Set-Location "Z:\Project\FCEUX11\build-off"
& cmake --build . --target kagami_qa_cycle_trace
if ($LASTEXITCODE -ne 0) { throw "build failed" }

$bin = "Z:\Project\FCEUX11\build-off\tests\kagami_qa_cycle_trace.exe"
if (-not (Test-Path $bin)) { throw "binary missing: $bin" }
Write-Host "[OK] $bin ($(((Get-Item $bin).Length) / 1KB)KB)"
