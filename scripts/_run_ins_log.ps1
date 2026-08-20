$ErrorActionPreference = "Stop"

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

$mode = $args[0]      # "rust" | "off"
$logfile = $args[1]   # e.g. __ins_rust.txt

if ($mode -eq "rust") { $exe = "build-rust-cpu\tests\kagami_qa_cycle_trace.exe" }
else { $exe = "build-off\tests\kagami_qa_cycle_trace.exe" }

$env:FCEUX11_LOG_INS = "1"
cmd /c "`"$exe`" tests\fixtures\blargg\cpu\instr_v5_all.nes 136 trace_ins_$mode.csv 2> $logfile"
Write-Host "exit=$LASTEXITCODE"
Remove-Item Env:FCEUX11_LOG_INS
Write-Host "log_lines=$((Get-Content $logfile -ErrorAction SilentlyContinue).Count)"
