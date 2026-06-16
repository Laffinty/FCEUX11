# i18n_update.ps1 - v0.3.15 PR-B
# Run lupdate on src/drivers/Qt/lang/translations.pro to refresh all .ts
# source files with the current set of tr() calls.
#
# Usage:
#   powershell scripts/i18n_update.ps1
#
# Tooling version: locked to Qt 6.8 LTS (per plan §3.1)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"

# Find lupdate-qt6 / lupdate from vcpkg Qt 6.8
$Lupdate = $null
$Candidates = @(
    "lupdate-qt6.exe",
    "lupdate.exe"
)
foreach ($name in $Candidates) {
    $found = Get-Command $name -ErrorAction SilentlyContinue
    if ($found) {
        $Lupdate = $found.Source
        break
    }
}

if (-not $Lupdate) {
    Write-Error "lupdate not found on PATH. Install Qt 6.8 LTS LinguistTools via vcpkg."
    exit 1
}

Write-Host "Using lupdate: $Lupdate"
Write-Host "Working dir: $LangDir"

Push-Location $LangDir
try {
    & $Lupdate -no-obsolete -project translations.pro
    if ($LASTEXITCODE -ne 0) {
        throw "lupdate failed with exit code $LASTEXITCODE"
    }
    Write-Host "lupdate completed successfully. .ts files updated."
}
finally {
    Pop-Location
}
