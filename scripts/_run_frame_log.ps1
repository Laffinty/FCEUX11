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
$frame = $args[1]     # e.g. 135
$logfile = $args[2]   # e.g. __fr_rust2.txt

if ($mode -eq "rust") { $exe = "build-rust-cpu\tests\kagami_qa_cycle_trace.exe" }
else { $exe = "build-off\tests\kagami_qa_cycle_trace.exe" }

# Run only up to frame+1 frames (divergence is deterministic given the
# identical prior history; frame N happens during a N+1-frame run).
$frames = [int]$frame + 1

$env:FCEUX11_LOG_FRAME = "$frame"
cmd /c "`"$exe`" tests\fixtures\blargg\cpu\instr_v5_all.nes $frames trace_fr_$mode.csv 2> $logfile"
Write-Host "exit=$LASTEXITCODE"
Remove-Item Env:FCEUX11_LOG_FRAME
Write-Host "log_lines=$((Get-Content $logfile -ErrorAction SilentlyContinue).Count)"
