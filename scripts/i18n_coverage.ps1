# i18n_coverage.ps1 - v0.3.15 PR-B
# Parse .ts files and report the unfinished-translation percentage per
# language. CI gate: zh_CN and zh_TW must both be >= 90% translated.
#
# Usage:
#   powershell scripts/i18n_coverage.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"

$Languages = @{
    "zh_CN" = 90
    "zh_TW" = 90
}

$ExitCode = 0

foreach ($lang in $Languages.Keys) {
    $tsPath = Join-Path $LangDir "fceux11_$lang.ts"
    if (-not (Test-Path $tsPath)) {
        Write-Warning "$tsPath not found; skipping."
        $ExitCode = 1
        continue
    }

    [xml]$xml = Get-Content $tsPath
    $totalMessages = 0
    $unfinished = 0
    foreach ($ctx in $xml.TS.context) {
        foreach ($msg in $ctx.message) {
            $totalMessages++
            $tr = $msg.translation
            $isUnfinished = $false
            if ($null -eq $tr) {
                $isUnfinished = $true
            } else {
                # If translation has type="unfinished" attribute
                if ($tr.GetAttribute('type') -eq 'unfinished') {
                    $isUnfinished = $true
                }
                # If translation text is empty
                if ([string]::IsNullOrEmpty($tr.'#text')) {
                    $isUnfinished = $true
                }
            }
            if ($isUnfinished) {
                $unfinished++
            }
        }
    }

    $translated = $totalMessages - $unfinished
    if ($totalMessages -gt 0) {
        $pct = [math]::Round(($translated / $totalMessages) * 100, 2)
    } else {
        $pct = 0
    }

    $gate = $Languages[$lang]
    $status = if ($pct -ge $gate) { "PASS" } else { "FAIL" }
    if ($status -eq "FAIL") { $ExitCode = 1 }

    Write-Host ("[{0}] {1}: {2}/{3} translated ({4}%) -- gate {5}% [{6}]" -f `
        $status, $lang, $translated, $totalMessages, $pct, $gate, $status)
}

if ($ExitCode -ne 0) {
    Write-Host ""
    Write-Error "i18n coverage gate failed. Add more translations or run lupdate."
}

exit $ExitCode
