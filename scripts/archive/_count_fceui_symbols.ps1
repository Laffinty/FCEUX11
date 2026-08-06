# FCEUX11 v0.3.10 — Repeatable FCEUI_* symbol counter
#
# Mirrors the baseline manual command:
#   grep -rn "FCEUI_" src/ | wc -l
#
# Outputs total occurrences, unique files, per-extension breakdown,
# and the top contributing files. Designed for P1 metrics baseline
# and the P4 convergence gate (< 600 in src/).
param(
    [string]$Root = "src",
    [switch]$IncludeTests,
    [int]$FailThreshold = 0,
    [int]$TopFiles = 15,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot

function Resolve-RelativePath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) { return $Path }
    return Join-Path $ProjectRoot $Path
}

$scanPaths = @((Resolve-RelativePath $Root))
if ($IncludeTests) {
    $scanPaths += (Resolve-RelativePath "tests")
}

# Prefer real grep when available so the count matches the documented
# baseline exactly. Fall back to Select-String for environments without Git.
$grep = Get-Command grep -ErrorAction SilentlyContinue
$useGrep = $null -ne $grep

$rawMatches = [System.Collections.Generic.List[object]]::new()
$baselineRawCount = 0

foreach ($p in $scanPaths) {
    if (-not (Test-Path $p)) {
        Write-Warning "Path not found: $p"
        continue
    }

    if ($useGrep) {
        # First: exact baseline command (includes binary-file summary lines).
        $baselineOutput = & $grep -rn "FCEUI_" $p 2>$null
        if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1) {
            throw "grep failed with exit code $LASTEXITCODE"
        }
        $baselineRawCount += ($baselineOutput | Measure-Object).Count

        # Second: text-only analysis for meaningful per-file breakdown.
        # -I skips binary files so every line is file:line:content.
        $output = & $grep -rnI "FCEUI_" $p 2>$null
        if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1) {
            throw "grep -I failed with exit code $LASTEXITCODE"
        }
        foreach ($line in $output) {
            # grep -n format: [drive:]/path/file:line:content
            # Handles both absolute Windows paths (C:/...) and relative paths.
            if ($line -match '^((?:[A-Za-z]:)?[^:]+):(\d+):(.*)$') {
                $filePath = $matches[1].Replace('\', '/')
                # Normalize to a path relative to the project root.
                $prefix = ($ProjectRoot -replace '\\', '/')
                if ($filePath.StartsWith($prefix)) {
                    $filePath = $filePath.Substring($prefix.Length).TrimStart('/')
                }
                $rawMatches.Add([PSCustomObject]@{
                    File = $filePath
                    Line = $matches[2]
                    Text = $matches[3]
                })
            }
        }
    }
    else {
        $files = Get-ChildItem -Recurse -File -Path $p
        $ssMatches = $files | Select-String -Pattern "FCEUI_"
        foreach ($m in $ssMatches) {
            $rawMatches.Add([PSCustomObject]@{
                File = $m.Path.Replace('\', '/')
                Line = $m.LineNumber
                Text = $m.Line
            })
        }
        $baselineRawCount = $rawMatches.Count
    }
}

$total = $rawMatches.Count
$uniqueFiles = $rawMatches | Select-Object -ExpandProperty File -Unique | Sort-Object
$fileCounts = $rawMatches | Group-Object File | Sort-Object Count -Descending
$extCounts = $rawMatches | ForEach-Object {
    [System.IO.Path]::GetExtension($_.File).ToLower()
} | Group-Object | Sort-Object Count -Descending

# Determine the relative path of the primary scan root for reporting.
$displayRoot = ($Root -replace '\\', '/')

$result = [PSCustomObject]@{
    version       = "v0.3.10"
    phase         = "P1"
    root          = $displayRoot
    includeTests  = [bool]$IncludeTests
    tool          = if ($useGrep) { "grep" } else { "Select-String" }
    baselineRaw   = $baselineRawCount
    total         = $total
    uniqueFiles   = $uniqueFiles.Count
    extensions    = @($extCounts | Select-Object Name, Count)
    topFiles      = @($fileCounts | Select-Object -First $TopFiles | Select-Object Name, Count)
}

if ($Json) {
    $result | ConvertTo-Json -Depth 4
}
else {
    Write-Host "=== FCEUX11 v0.3.10 FCEUI_* Symbol Count ===" -ForegroundColor Cyan
    Write-Host "Scan root     : $displayRoot$(if ($IncludeTests) { " + tests/" })"
    Write-Host "Tool          : $($result.tool)"
    Write-Host "Baseline raw  : $baselineRawCount  (matches 'grep -rn `"FCEUI_`" $displayRoot | wc -l')"
    Write-Host "Text count    : $total  (binary files excluded; use for gates)"
    Write-Host "Unique files  : $($uniqueFiles.Count)"
    Write-Host ""
    Write-Host "--- By file extension ---" -ForegroundColor Yellow
    foreach ($e in $extCounts) {
        Write-Host ("  {0,-8} {1,6}" -f $e.Name, $e.Count)
    }
    Write-Host ""
    Write-Host "--- Top $TopFiles files ---" -ForegroundColor Yellow
    foreach ($f in ($fileCounts | Select-Object -First $TopFiles)) {
        Write-Host ("  {0,6}  {1}" -f $f.Count, $f.Name)
    }
    Write-Host ""
    if ($FailThreshold -gt 0) {
        if ($total -gt $FailThreshold) {
            Write-Host "[FAIL] text count $total > threshold $FailThreshold" -ForegroundColor Red
            exit 1
        }
        else {
            Write-Host "[PASS] text count $total <= threshold $FailThreshold" -ForegroundColor Green
        }
    }
    Write-Host "Baseline commands:" -ForegroundColor Gray
    Write-Host "  grep -rn `"FCEUI_`" $displayRoot | wc -l    -> $baselineRawCount" -ForegroundColor Gray
    Write-Host "  grep -rnI `"FCEUI_`" $displayRoot | wc -l   -> $total" -ForegroundColor Gray
}

exit 0
