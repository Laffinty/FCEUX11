# FCEUX11 Dependency Deployment Script (v0.2.1)
# Copies required runtime DLLs from vcpkg installed directory.
param(
    [Parameter(Mandatory=$true)]
    [string]$ExecutablePath,

    [string]$OutputDir = (Split-Path $ExecutablePath -Parent),

    [string]$VcpkgRoot = $env:VCPKG_ROOT
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ExecutablePath)) {
    throw "Executable not found: $ExecutablePath"
}

# Resolve vcpkg installed bin directory
$vcpkgBin = $null
if ($VcpkgRoot) {
    $candidate = Join-Path $VcpkgRoot "installed\x64-windows\bin"
    if (Test-Path $candidate) {
        $vcpkgBin = $candidate
    }
}

# Fallback: search common locations
if (-not $vcpkgBin) {
    $candidates = @(
        "$PSScriptRoot\..\build\vcpkg_installed\x64-windows\bin"
        "$PSScriptRoot\..\vcpkg_installed\x64-windows\bin"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) {
            $vcpkgBin = $c
            break
        }
    }
}

if (-not $vcpkgBin) {
    throw "vcpkg installed bin directory not found. Specify -VcpkgRoot or ensure build/vcpkg_installed exists."
}

Write-Host "Using vcpkg bin: $vcpkgBin" -ForegroundColor Gray

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# Analyze dependencies using dumpbin (requires VS environment) or fallback to heuristics
function Get-PeDependencies {
    param([string]$Path)
    $dlls = @()
    try {
        $dumpbin = (Get-Command dumpbin -ErrorAction SilentlyContinue).Source
        if ($dumpbin) {
            $output = & $dumpbin /DEPENDENTS $Path 2>$null
            foreach ($line in $output) {
                if ($line -match '^\s+([\w\-]+\.dll)\s*$') {
                    $dlls += $matches[1]
                }
            }
        }
    } catch {}
    return $dlls | Sort-Object -Unique
}

$exeDeps = Get-PeDependencies -Path $ExecutablePath
$copied = 0
$skipped = 0

Write-Host "`nDeploying dependencies..." -ForegroundColor Cyan

# First pass: copy direct dependencies
foreach ($dll in $exeDeps) {
    $src = Join-Path $vcpkgBin $dll
    $dst = Join-Path $OutputDir $dll
    if (Test-Path $src) {
        Copy-Item $src -Destination $dst -Force
        Write-Host "  [COPY] $dll" -ForegroundColor Green
        $copied++
    } else {
        Write-Host "  [SKIP] $dll (system)" -ForegroundColor DarkGray
        $skipped++
    }
}

# Second pass: copy transitive dependencies (one level)
$allDlls = Get-ChildItem $OutputDir -Filter "*.dll" | Select-Object -ExpandProperty Name
$transitive = @()
foreach ($dll in $allDlls) {
    $dllPath = Join-Path $OutputDir $dll
    $deps = Get-PeDependencies -Path $dllPath
    foreach ($d in $deps) {
        if ($d -notin $allDlls -and $d -notin $transitive) {
            $src = Join-Path $vcpkgBin $d
            if (Test-Path $src) {
                $transitive += $d
            }
        }
    }
}

foreach ($dll in $transitive) {
    $src = Join-Path $vcpkgBin $dll
    $dst = Join-Path $OutputDir $dll
    if (-not (Test-Path $dst)) {
        Copy-Item $src -Destination $dst -Force
        Write-Host "  [COPY] $dll (transitive)" -ForegroundColor Green
        $copited++
    }
}

Write-Host "`nDone! Copied $copied DLLs, skipped $skipped system DLLs." -ForegroundColor Green
Write-Host "Output: $OutputDir" -ForegroundColor Cyan
