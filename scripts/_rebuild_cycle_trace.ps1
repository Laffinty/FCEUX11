$ErrorActionPreference = "Stop"
$BuildDir = $args[0]
if (-not $BuildDir) { $BuildDir = "build-rust-cpu" }

# Load MSVC env via cmd.exe.
$vcvarsOut = & cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat`" >nul 2>&1 && set"
foreach ($line in ($vcvarsOut -split "`r?`n")) {
    if ($line -match "^([^=]+)=(.*)$") {
        $name = $matches[1]; $value = $matches[2]
        Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
    }
}
$ninjaDir = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"
$vcpkgBin = "Z:\Project\FCEUX11\vcpkg_installed\x64-windows\bin"
$env:PATH = "$ninjaDir;$vcpkgBin;$env:PATH"

Set-Location "Z:\Project\FCEUX11\$BuildDir"
& cmake --build . --target kagami_qa_cycle_trace
if ($LASTEXITCODE -ne 0) { throw "build failed" }
Write-Host "[OK] kagami_qa_cycle_trace.exe rebuilt in $BuildDir"
