$ErrorActionPreference = "Continue"

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

Set-Location "Z:\Project\FCEUX11\tests"

cmd /c "`"..\build-rust-cpu\tests\kagami_qa_blargg_runner.exe`" --manifest fixtures\blargg_manifest.json > ..\blargg_result.json 2> ..\blargg_stderr.txt"
Write-Host "exit=$LASTEXITCODE"
$json = Get-Content "..\blargg_result.json" -Raw
# Summarize: count PASS/FAIL entries from the JSON
$pass = ([regex]::Matches($json, '"passed"\s*:\s*true')).Count
$fail = ([regex]::Matches($json, '"passed"\s*:\s*false')).Count
Write-Host "json_passed=$pass json_failed=$fail"
