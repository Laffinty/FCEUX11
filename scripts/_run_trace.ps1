$ErrorActionPreference = "Stop"

# Same env setup as scripts/_rebuild_cycle_trace.ps1 (vcvars + vcpkg + ninja).
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

Set-Location "Z:\Project\FCEUX11"

$mode = $args[0]           # "rust" | "off"
$frames = $args[1]
$outcsv = $args[2]
$reglog = $args[3]         # "" to skip reg logging

if ($mode -eq "rust") { $exe = "build-rust-cpu\tests\kagami_qa_cycle_trace.exe" }
else { $exe = "build-off\tests\kagami_qa_cycle_trace.exe" }

if ($reglog) { $env:FCEUX11_LOG_REG = "1" }

# Use cmd native redirection for stderr: opens the file directly in the
# child (no PowerShell pipe), avoiding the pipe-buffer deadlock when the
# harness writes thousands of reg-log lines per frame.
if ($reglog) {
    cmd /c "`"$exe`" tests\fixtures\blargg\cpu\instr_v5_all.nes $frames $outcsv 2> $reglog"
} else {
    cmd /c "`"$exe`" tests\fixtures\blargg\cpu\instr_v5_all.nes $frames $outcsv"
}
Write-Host "exit=$LASTEXITCODE csv_size=$((Get-Item $outcsv -ErrorAction SilentlyContinue).Length)"
if ($reglog) {
    Remove-Item Env:FCEUX11_LOG_REG
    Write-Host "reg_lines=$((Get-Content $reglog -ErrorAction SilentlyContinue).Count)"
}
