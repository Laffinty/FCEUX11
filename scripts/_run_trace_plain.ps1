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

$mode = $args[0]
$outcsv = $args[1]

if ($mode -eq "rust") { $exe = "build-rust-cpu\tests\kagami_qa_cycle_trace.exe" }
else { $exe = "build-off\tests\kagami_qa_cycle_trace.exe" }

cmd /c "`"$exe`" tests\fixtures\blargg\cpu\instr_v5_all.nes 300 $outcsv"
Write-Host "exit=$LASTEXITCODE csv_size=$((Get-Item $outcsv -ErrorAction SilentlyContinue).Length)"
