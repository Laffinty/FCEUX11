# i18n_coverage.ps1 - v0.3.15 PHASE-1
# Parse .ts files and report the unfinished-translation percentage per
# language. CI gate: zh_CN and zh_TW must both be >= 90% translated.
#
# Usage:
#   powershell scripts/i18n_coverage.ps1
#
# v0.3.15 PHASE-1 fix: rewritten to use XPath so attribute access works
# correctly on XmlElement (the previous version relied on PowerShell's
# automatic child-element text extraction, which returned String objects
# instead of XmlElement and broke GetAttribute calls).

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"

$Languages = @{
    "zh_CN" = 90
    "zh_TW" = 90
}

$ExitCode = 0
$XmlNsMgr = New-Object System.Xml.XmlNamespaceManager(@(New-Object System.Xml.NameTable))

foreach ($lang in $Languages.Keys) {
    $tsPath = Join-Path $LangDir "fceux11_$lang.ts"
    if (-not (Test-Path $tsPath)) {
        Write-Warning "$tsPath not found; skipping."
        $ExitCode = 1
        continue
    }

    $xmlDoc = New-Object System.Xml.XmlDocument
    $xmlDoc.Load($tsPath)

    $totalMessages = 0
    $unfinished = 0

    # Select all <message> elements anywhere in the doc
    $messageNodes = $xmlDoc.SelectNodes("//message")
    foreach ($msg in $messageNodes) {
        $totalMessages++
        $tr = $msg.SelectSingleNode("translation")
        $isUnfinished = $false
        if ($null -eq $tr) {
            $isUnfinished = $true
        } else {
            # Check the 'type' attribute on <translation>
            $trType = $tr.GetAttribute("type")
            if ($trType -eq "unfinished" -or $trType -eq "needs-review") {
                $isUnfinished = $true
            }
            # Check inner text is empty/whitespace
            $innerText = ""
            if ($null -ne $tr.InnerText) { $innerText = $tr.InnerText.Trim() }
            if ($innerText.Length -eq 0) {
                $isUnfinished = $true
            }
        }
        if ($isUnfinished) { $unfinished++ }
    }

    $translated = $totalMessages - $unfinished
    if ($totalMessages -gt 0) {
        $pct = [math]::Round(($translated / $totalMessages) * 100, 2)
    } else {
        $pct = 0
    }

    $gate = $Languages[$lang]
    if ($pct -ge $gate) {
        $status = "PASS"
    } else {
        $status = "FAIL"
        $ExitCode = 1
    }

    Write-Host ("[{0}] {1}: {2}/{3} translated ({4}%) -- gate {5}%" -f `
        $status, $lang, $translated, $totalMessages, $pct, $gate)
}

if ($ExitCode -ne 0) {
    Write-Host ""
    Write-Error "i18n coverage gate failed. Add more translations or run lupdate."
}

exit $ExitCode