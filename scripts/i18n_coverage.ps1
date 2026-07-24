# i18n_coverage.ps1 — v1.15 hotfix5
# CI gate: parse all 11 non-English .ts files and reject translations
# that are unfinished, empty, or identical-to-EN (unless whitelisted).
#
# Usage:
#   powershell scripts/i18n_coverage.ps1
#
# Exit code 0 = all pass, 1 = at least one language below threshold.

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"
$AllowPath = Join-Path $LangDir "en_keep_allowlist.txt"

# --- Load allowlist (normalized source strings that may stay English) ---
$AllowSet = @{}
if (Test-Path $AllowPath) {
    foreach ($line in Get-Content $AllowPath -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed.Length -gt 0 -and -not $trimmed.StartsWith("#")) {
            $AllowSet[$trimmed] = $true
        }
    }
} else {
    Write-Warning "Allowlist not found at $AllowPath; identical-to-EN will always fail."
}

# --- Language gates: zh_CN/zh_TW >= 95%, others >= 90% ---
$Languages = [ordered]@{
    "zh_CN" = 95
    "zh_TW" = 95
    "ja"    = 90
    "ko"    = 90
    "es"    = 90
    "fr"    = 90
    "de"    = 90
    "vi"    = 90
    "th"    = 90
    "hi"    = 90
    "ar"    = 90
}

# --- Helpers ---
function Normalize-Text([string]$s) {
    # Collapse whitespace and trim, same as Python norm()
    return ($s -replace '\s+', ' ').Trim()
}

function Unescape-Xml([string]$s) {
    return $s.Replace("&amp;","&").Replace("&apos;","'").Replace("&quot;",'"').Replace("&lt;","<").Replace("&gt;",">")
}

$ExitCode = 0

foreach ($lang in $Languages.Keys) {
    $tsPath = Join-Path $LangDir "fceux11_$lang.ts"
    if (-not (Test-Path $tsPath)) {
        Write-Warning "$tsPath not found; skipping."
        $ExitCode = 1
        continue
    }

    $xmlDoc = New-Object System.Xml.XmlDocument
    $xmlDoc.Load($tsPath)

    # No need to load en.ts — for non-English languages,
    # translation == source text is sufficient to detect identical-to-EN fakes.

    $totalMessages = 0
    $failed = 0

    $messageNodes = $xmlDoc.SelectNodes("//message")
    foreach ($msg in $messageNodes) {
        $srcNode = $msg.SelectSingleNode("source")
        $trNode  = $msg.SelectSingleNode("translation")

        if ($null -eq $srcNode -or $null -eq $srcNode.InnerText) { continue }
        $srcText = $srcNode.InnerText.Trim()
        if ($srcText.Length -eq 0) { continue }

        $totalMessages++

        # Check 1: missing or unfinished
        $isFail = $false
        if ($null -eq $trNode) {
            $isFail = $true
        } else {
            $trType = $trNode.GetAttribute("type")
            if ($trType -eq "unfinished" -or $trType -eq "needs-review") {
                $isFail = $true
            }
            $trText = ""
            if ($null -ne $trNode.InnerText) { $trText = $trNode.InnerText.Trim() }
            # Check 2: empty
            if ($trText.Length -eq 0) {
                $isFail = $true
            }
            # Check 3: identical-to-EN and NOT in allowlist
            if (-not $isFail) {
                $normSrc = Normalize-Text (Unescape-Xml $srcText)
                $normTr  = Normalize-Text (Unescape-Xml $trText)
                if ($normTr -eq $normSrc) {
                    # For non-English: translation == source means it stayed English.
                    # Only OK if the source is in the allowlist.
                    if (-not $AllowSet.ContainsKey($normSrc)) {
                        $isFail = $true
                    }
                }
            }
        }
        if ($isFail) { $failed++ }
    }

    $passed = $totalMessages - $failed
    if ($totalMessages -gt 0) {
        $pct = [math]::Round(($passed / $totalMessages) * 100, 2)
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

    Write-Host ("[{0}] {1}: {2}/{3} passed ({4}%) -- gate {5}% | failed={6}" -f `
        $status, $lang, $passed, $totalMessages, $pct, $gate, $failed)
}

if ($ExitCode -ne 0) {
    Write-Host ""
    Write-Error "i18n coverage gate failed. Run i18n_audit.py for details, then fix translations."
}

exit $ExitCode
