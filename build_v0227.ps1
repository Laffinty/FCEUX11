$vsPath = 'C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools'
$vcvarsBat = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
$buildDir = 'C:\Users\ikrx2\Desktop\project\FCEUX11\build_vs'

# Call vcvars64.bat to set up MSVC environment
$env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;$env:PATH"
$env:LIB = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\lib\x64;C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\lib\x64;$env:LIB"
$env:INCLUDE = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\include;C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\include;$env:INCLUDE"

# Run cmake configure and build
Write-Host "Configuring..."
cmake -G 'Visual Studio 17 2022' -A x64 -S C:\Users\ikrx2\Desktop\project\FCEUX11 -B $buildDir 2>&1 | Select-Object -Last 30
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "Building..."
cmake --build $buildDir --config Release 2>&1 | Select-Object -Last 100