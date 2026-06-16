# i18n_release.ps1 - v0.3.15 PR-B
# Run lrelease to compile .ts -> .qm and emit them to the build dir.
#
# Usage:
#   powershell scripts/i18n_release.ps1
#
# Tooling version: locked to Qt 6.8 LTS (per plan §3.1)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"
$OutDir = Join-Path $ProjectRoot "build\i18n"

# Find lrelease-qt6 / lrelease from vcpkg Qt 6.8
$Lrelease = $null
$Candidates = @(
    "lrelease-qt6.exe",
    "lrelease.exe"
)
foreach ($name in $Candidates) {
    $found = Get-Command $name -ErrorAction SilentlyContinue
    if ($found) {
        $Lrelease = $found.Source
        break
    }
}

if (-not $Lrelease) {
    Write-Error "lrelease not found on PATH. Install Qt 6.8 LTS LinguistTools via vcpkg."
    exit 1
}

if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

Write-Host "Using lrelease: $Lrelease"
Write-Host "Output dir: $OutDir"

$tsFiles = @(
    "fceux11_en.ts",
    "fceux11_zh_CN.ts",
    "fceux11_zh_TW.ts"
)

Push-Location $LangDir
try {
    foreach ($ts in $tsFiles) {
        $qm = [System.IO.Path]::ChangeExtension($ts, ".qm")
        $qmPath = Join-Path $OutDir $qm
        & $Lrelease $ts -qm $qmPath
        if ($LASTEXITCODE -ne 0) {
            throw "lrelease failed for $ts with exit code $LASTEXITCODE"
        }
        Write-Host "Compiled: $ts -> $qmPath"
    }
    Write-Host "All .qm files generated."
}
finally {
    Pop-Location
}
