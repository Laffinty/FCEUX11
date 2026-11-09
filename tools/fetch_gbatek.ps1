# FCEUX11 v2.0.3 - fetch the GBATEK snapshot used as the bit level format reference.
#
# Why this script exists
# ----------------------
# GBATEK (https://mgba-emu.github.io/gbatek/) is the authoritative reference for
# GBA bit level formats: EEPROM command protocol, EEPROM addressing and chip
# capacity, DMA control registers, cartridge header layout, ...
# The v2.0.3 plan (the GBA cartridge compatibility plan under docs/plans/,
# appendix A) quotes GBATEK by line number, so the snapshot has to be
# reproducible instead of being a file that somebody downloaded once by hand.
#
# The snapshot never enters the repository: both outputs land under .zcode/,
# which is git ignored.  Only the excerpts quoted in the plan are committed.
#
# The HTML -> plain text conversion uses the Python standard library only
# (html.parser): no new build dependency, no third party package.
#
# The conversion rules were reverse engineered from the 2026-11-08 snapshot so
# that a re-fetch reproduces the previous plain text byte for byte, which is
# what keeps the line numbers cited in the plan valid:
#   * every start tag, end tag, comment and doctype emits exactly one newline
#   * text is kept verbatim
#   * HTML entities are NOT expanded (&amp; stays "&amp;")
#
# Usage
# -----
#   powershell -ExecutionPolicy Bypass -File tools\fetch_gbatek.ps1
#   powershell -ExecutionPolicy Bypass -File tools\fetch_gbatek.ps1 -SkipFetch
#   powershell -ExecutionPolicy Bypass -File tools\fetch_gbatek.ps1 `
#       -OutputHtml .zcode\gbatek_probe_test.html `
#       -OutputText .zcode\gbatek_plain_test.txt
#
# Exit codes
# ----------
#   0  success
#   2  bad usage, or python not available
#   3  download failed (proxy unreachable, timeout, HTTP status != 200)
#   4  HTML -> plain text conversion failed

param(
    [string]$Url = "https://mgba-emu.github.io/gbatek/",
    [string]$Proxy = "http://127.0.0.1:7897",
    [string]$OutputHtml = ".zcode/gbatek_probe.html",
    [string]$OutputText = ".zcode/gbatek_plain.txt",
    [int]$TimeoutSec = 120,
    [switch]$SkipFetch
)

$ErrorActionPreference = "Stop"

# temp files of the current run (removed again when the run fails)
$script:tmpHtml = ""
$script:errFile = ""

# ---------------------------------------------------------------------------
# HTML -> plain text converter (Python standard library only).
# Written to a temporary file at run time, so no extra file in the repository.
# ---------------------------------------------------------------------------
$Converter = @'
"""GBATEK HTML -> plain text (one entry per line).

Usage: gbatek_html2txt.py <input.html> <output.txt>

Rules (reverse engineered from the 2026-11-08 snapshot, so that a re-fetch
reproduces that file byte for byte and the line numbers cited in the v2.0.3
plan stay valid):

  * every start tag, end tag, comment and doctype emits exactly one newline
  * text is kept verbatim
  * HTML entities are NOT expanded

Standard library only.
"""

import hashlib
import os
import sys
from html.parser import HTMLParser

ANCHORS = [
    'GBA Cart Backup IDs',
    'GBA Cart Backup EEPROM',
    'Lifetime: 100,000 writes per address',
    'Addressing and Waitstates',
    'Data and Address Width',
    'Set Address (For Reading)',
    'Read Data',
    'Write Data to Address',
    'Using DMA',
    'Pin-Outs',
    'Notes',
]


class GbatekText(HTMLParser):
    def __init__(self):
        HTMLParser.__init__(self, convert_charrefs=False)
        self.chunks = []

    def handle_starttag(self, tag, attrs):
        self.chunks.append('\n')

    def handle_startendtag(self, tag, attrs):
        self.chunks.append('\n')

    def handle_endtag(self, tag):
        self.chunks.append('\n')

    def handle_decl(self, decl):
        self.chunks.append('\n')

    def handle_comment(self, data):
        self.chunks.append('\n')

    def handle_data(self, data):
        self.chunks.append(data)

    def handle_entityref(self, name):
        self.chunks.append('&' + name + ';')

    def handle_charref(self, name):
        self.chunks.append('&#' + name + ';')


def main():
    if len(sys.argv) != 3:
        sys.stderr.write('usage: gbatek_html2txt.py <input.html> <output.txt>\n')
        return 2
    src = sys.argv[1]
    dst = sys.argv[2]
    if not os.path.isfile(src):
        sys.stderr.write('input file not found: %s\n' % src)
        return 2
    with open(src, 'r', encoding='utf-8', errors='replace') as fh:
        raw = fh.read()
    parser = GbatekText()
    parser.feed(raw)
    text = ''.join(parser.chunks)
    with open(dst, 'w', encoding='utf-8', newline='\n') as fh:
        fh.write(text)
    lines = text.split('\n')
    print('plain text : %s' % dst)
    print('lines      : %d' % len(lines))
    print('sha256     : %s' % hashlib.sha256(text.encode('utf-8')).hexdigest())
    print('anchor line numbers (in the plain text, so the plan citations can be re-checked):')
    for anchor in ANCHORS:
        hits = [i + 1 for i, line in enumerate(lines) if line == anchor]
        shown = ', '.join(str(h) for h in hits[:6])
        if len(hits) > 6:
            shown = shown + ', ... (%d hits)' % len(hits)
        print('  %-38s %s' % (anchor, shown if hits else '(not found)'))
    return 0


if __name__ == '__main__':
    sys.exit(main())
'@

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
function Write-Fail {
    param([string]$Message, [int]$Code)
    # a failed run must not leave a half written snapshot behind
    foreach ($f in @($script:tmpHtml, $script:errFile)) {
        if ($f -and (Test-Path -LiteralPath $f)) { Remove-Item -LiteralPath $f -Force }
    }
    [Console]::Error.WriteLine("fetch_gbatek: ERROR: " + $Message)
    exit $Code
}

function Get-RepoRoot {
    if (-not [string]::IsNullOrEmpty($PSScriptRoot)) {
        return (Split-Path -Parent $PSScriptRoot)
    }
    return (Get-Location).Path
}

function Resolve-OutputPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path (Get-RepoRoot) $Path))
}

# ---------------------------------------------------------------------------
# 1. download
# ---------------------------------------------------------------------------
$RepoRoot = Get-RepoRoot
$HtmlPath = Resolve-OutputPath $OutputHtml
$TextPath = Resolve-OutputPath $OutputText

foreach ($dir in @((Split-Path -Parent $HtmlPath), (Split-Path -Parent $TextPath))) {
    if (-not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Host ("created directory : " + $dir)
    }
}

Write-Host ("repo root         : " + $RepoRoot)
Write-Host ("url               : " + $Url)
Write-Host ("proxy             : " + $(if ($Proxy) { $Proxy } else { "(none, direct)" }))

if ($SkipFetch) {
    if (-not (Test-Path -LiteralPath $HtmlPath)) {
        Write-Fail ("-SkipFetch was given but the html file does not exist: " + $HtmlPath) 2
    }
    Write-Host ("html              : " + $HtmlPath + " (kept, -SkipFetch)")
} else {
    $tmpHtml = $HtmlPath + ".part"
    $errFile = $HtmlPath + ".curlerr"
    $script:tmpHtml = $tmpHtml
    $script:errFile = $errFile
    foreach ($f in @($tmpHtml, $errFile)) {
        if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force }
    }

    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($curl) {
        $curlArgs = @("-sS", "-L", "--max-time", "$TimeoutSec", "--stderr", $errFile,
                      "-w", "%{http_code}", "-o", $tmpHtml)
        if ($Proxy) { $curlArgs += @("-x", $Proxy) }
        $curlArgs += $Url

        $saved = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $status = (& $curl.Source @curlArgs)
        $rc = $LASTEXITCODE
        $ErrorActionPreference = $saved
        $status = ($status | Out-String).Trim()

        $curlErr = ""
        if (Test-Path -LiteralPath $errFile) {
            $curlErr = ((Get-Content -LiteralPath $errFile -Raw) | Out-String).Trim()
            Remove-Item -LiteralPath $errFile -Force
        }

        if ($rc -ne 0) {
            $reason = switch ($rc) {
                5  { "could not resolve the proxy host (is the proxy address correct?)" }
                6  { "could not resolve the target host" }
                7  { "could not connect to the proxy (is the proxy running?)" }
                28 { "timed out after $TimeoutSec seconds" }
                35 { "TLS handshake failed" }
                60 { "TLS certificate problem" }
                default { "curl exit code $rc" }
            }
            Write-Fail ("download failed: " + $reason + " [" + $Url + "] " + $curlErr) 3
        }
        if ($status -ne "200") {
            Write-Fail ("download failed: HTTP status " + $status + " (expected 200) for " + $Url) 3
        }
    } else {
        Write-Host "curl.exe not found, falling back to Invoke-WebRequest"
        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            $params = @{ Uri = $Url; UseBasicParsing = $true; TimeoutSec = $TimeoutSec }
            if ($Proxy) { $params["Proxy"] = $Proxy }
            Invoke-WebRequest @params -OutFile $tmpHtml
        } catch {
            Write-Fail ("download failed: " + $_.Exception.Message + " [" + $Url + "]") 3
        }
    }

    if (-not (Test-Path -LiteralPath $tmpHtml)) {
        Write-Fail "download failed: no output file was produced" 3
    }
    $size = (Get-Item -LiteralPath $tmpHtml).Length
    if ($size -lt 100000) {
        Write-Fail ("download failed: the fetched page is only $size bytes, that cannot be GBATEK") 3
    }
    if (Test-Path -LiteralPath $HtmlPath) { Remove-Item -LiteralPath $HtmlPath -Force }
    Move-Item -LiteralPath $tmpHtml -Destination $HtmlPath -Force
    Write-Host ("html              : " + $HtmlPath + " (" + $size + " bytes)")
}

# ---------------------------------------------------------------------------
# 2. convert
# ---------------------------------------------------------------------------
$py = Get-Command python -ErrorAction SilentlyContinue
$pyPrefix = @()
if (-not $py) {
    $py = Get-Command py -ErrorAction SilentlyContinue
    if ($py) { $pyPrefix = @("-3") }
}
if (-not $py) {
    Write-Fail "python not found in PATH: the html -> plain text conversion needs it (standard library only)" 2
}

$pyFile = Join-Path ([System.IO.Path]::GetTempPath()) ("gbatek_html2txt_" + $PID + ".py")
[System.IO.File]::WriteAllText($pyFile, $Converter, (New-Object System.Text.UTF8Encoding($false)))

$saved = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$convOut = (& $py.Source @($pyPrefix + @($pyFile, $HtmlPath, $TextPath)) 2>&1)
$rc = $LASTEXITCODE
$ErrorActionPreference = $saved
if (Test-Path -LiteralPath $pyFile) { Remove-Item -LiteralPath $pyFile -Force }

foreach ($line in $convOut) { Write-Host $line }
if ($rc -ne 0) {
    Write-Fail ("conversion failed: python exit code " + $rc) 4
}
if (-not (Test-Path -LiteralPath $TextPath)) {
    Write-Fail "conversion failed: the plain text file was not produced" 4
}

Write-Host ""
Write-Host ("text              : " + $TextPath + " (" + (Get-Item -LiteralPath $TextPath).Length + " bytes)")
Write-Host "both outputs live under .zcode/ which is git ignored: the snapshot itself never enters the repository"
exit 0
