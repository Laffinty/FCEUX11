param(
    [Parameter(Mandatory=$false)]
    [string]$VcpkgRoot = "$PSScriptRoot\..\vcpkg",

    [Parameter(Mandatory=$false)]
    [ValidateSet("x64-windows", "x64-windows-static")]
    [string]$Triplet = "x64-windows"
)

$ErrorActionPreference = "Stop"

Write-Host "FCEUX11 vcpkg Setup Script" -ForegroundColor Cyan
Write-Host "===========================" -ForegroundColor Cyan
Write-Host ""

if ($Triplet -eq "x64-windows-static") {
    Write-Host "[INFO] Using static triplet: $Triplet" -ForegroundColor Yellow
    Write-Host "[INFO] Static linking will reduce deployment dependencies" -ForegroundColor Gray
}

$VcpkgExe = Join-Path $VcpkgRoot "vcpkg.exe"

if (-not (Test-Path $VcpkgExe)) {
    Write-Host "[STEP 1] Cloning vcpkg repository..." -ForegroundColor Cyan
    Write-Host "  Target: $VcpkgRoot" -ForegroundColor Gray

    if (Test-Path $VcpkgRoot) {
        Write-Host "[WARNING] Directory exists. Pulling latest..." -ForegroundColor Yellow
        Push-Location $VcpkgRoot
        git pull
        Pop-Location
    } else {
        git clone https://github.com/microsoft/vcpkg.git $VcpkgRoot
    }

    Write-Host "[STEP 2] Bootstrapping vcpkg..." -ForegroundColor Cyan
    Push-Location $VcpkgRoot
    .\bootstrap-vcpkg.bat
    Pop-Location
} else {
    Write-Host "[INFO] vcpkg already installed at: $VcpkgRoot" -ForegroundColor Green
}

Write-Host "[STEP 3] Integrating vcpkg with Visual Studio..." -ForegroundColor Cyan
& $VcpkgExe integrate install

Write-Host "[STEP 4] Installing FCEUX11 dependencies for $Triplet..." -ForegroundColor Cyan
& $VcpkgExe install --triplet $Triplet qtbase:x64-windows sdl2:x64-windows libarchive:x64-windows zlib:x64-windows liblzma:x64-windows

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To use vcpkg with CMake, set the environment variable:" -ForegroundColor Yellow
Write-Host '  $env:VCPKG_ROOT = "' + $VcpkgRoot + '"' -ForegroundColor White
Write-Host ""
Write-Host "Or add to your CMakeLists.txt:" -ForegroundColor Yellow
Write-Host '  if(DEFINED ENV{VCPKG_ROOT})' -ForegroundColor White
Write-Host '    set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake")' -ForegroundColor White
Write-Host '  endif()' -ForegroundColor White
Write-Host ""
Write-Host "Built artifacts will be in: $VcpkgRoot\installed\$Triplet" -ForegroundColor Gray
Write-Host "========================================" -ForegroundColor Cyan
