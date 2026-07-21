$env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;$env:PATH"
$env:LIB = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\lib\x64;$env:LIB"
$env:INCLUDE = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.44.35207\include;$env:INCLUDE"

# Find rc.exe
$windowsKitsPath = "C:\Program Files (x86)\Windows Kits\10\bin"
$windowsKitVersion = Get-ChildItem $windowsKitsPath -Directory | Select-Object -First 1 -ExpandProperty Name
if ($windowsKitVersion) {
    $rcPath = Join-Path $windowsKitsPath "$windowsKitVersion\x64\rc.exe"
    if (Test-Path $rcPath) {
        $env:PATH = "C:\Program Files (x86)\Windows Kits\10\bin\$windowsKitVersion\x64;$env:PATH"
        Write-Host "Found Windows SDK RC at: $rcPath"
    }
}

$buildDir = "C:\Users\ikrx2\Desktop\project\FCEUX11\build_nmake"

Write-Host "Configuring..."
cmake -G "NMake Makefiles" -S C:\Users\ikrx2\Desktop\project\FCEUX11 -B $buildDir 2>&1 | Select-Object -Last 40

if ($LASTEXITCODE -ne 0) {
    Write-Host "Configure failed"
    exit 1
}

Write-Host "Building..."
cmake --build $buildDir --config Release 2>&1 | Select-Object -Last 100