# FCEUX11 v0.3.10 — EMUFILE caller inventory
#
# Lists every file that references EMUFILE and categorises the call sites
# by API surface. Used in P1 to build the P2/P3 migration topology and
# in P3 to verify that no deprecated overloads remain.
param(
    [string]$Root = "src",
    [switch]$IncludeTests,
    [switch]$Details,
    [string[]]$Extensions = @("*.cpp", "*.c", "*.h", "*.hpp", "*.cc")
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

# Categories. Order matters for display.
$patterns = [ordered]@{
    "EMUFILE type refs"      = "EMUFILE(_MEMORY|_FILE)?\b"
    ".fread()"               = "->\s*fread\s*\("
    ".fwrite()"              = "->\s*fwrite\s*\("
    "._fread()"              = "->\s*_fread\s*\("
    ".buf()"                 = "->\s*buf\s*\(\s*\)"
    ".get_vec()"             = "->\s*get_vec\s*\(\s*\)"
    "EMUFILE_MEMORY ctor"    = "new\s+EMUFILE_MEMORY|EMUFILE_MEMORY\s*\("
    "EMUFILE_FILE ctor"      = "new\s+EMUFILE_FILE|EMUFILE_FILE\s*\("
    "readAllBytes"           = "readAllBytes\s*\("
    "memwrap"                = "->\s*memwrap\s*\(\s*\)"
}

$allFiles = [System.Collections.Generic.List[System.IO.FileInfo]]::new()
foreach ($p in $scanPaths) {
    if (-not (Test-Path $p)) {
        Write-Warning "Path not found: $p"
        continue
    }
    foreach ($f in (Get-ChildItem -Recurse -File -Path $p -Include $Extensions)) {
        if (-not $allFiles.Contains($f)) {
            $allFiles.Add($f)
        }
    }
}

$inventory = foreach ($file in ($allFiles | Sort-Object FullName)) {
    $rel = $file.FullName.Replace($ProjectRoot, "").TrimStart('\', '/').Replace('\', '/')
    $hits = @{}
    $detailLines = [System.Collections.Generic.List[string]]::new()

    foreach ($cat in $patterns.Keys) {
        $matches = Select-String -Path $file.FullName -Pattern $patterns[$cat]
        $count = ($matches | Measure-Object).Count
        $hits[$cat] = $count
        if ($Details -and $count -gt 0) {
            foreach ($m in $matches) {
                $detailLines.Add("  $($m.LineNumber):$($m.Line.Trim())")
            }
        }
    }

    $any = $hits.Values | Where-Object { $_ -gt 0 } | Measure-Object
    if ($any.Count -eq 0) { continue }

    $row = [PSCustomObject]@{ File = $rel }
    foreach ($cat in $patterns.Keys) {
        $row | Add-Member -MemberType NoteProperty -Name $cat -Value $hits[$cat]
    }
    $row | Add-Member -MemberType NoteProperty -Name Total -Value (($hits.Values | Measure-Object -Sum).Sum)
    $row | Add-Member -MemberType NoteProperty -Name Details -Value $detailLines
    $row
}

$totalFiles = $inventory.Count
$totalHits = ($inventory | Measure-Object Total -Sum).Sum

Write-Host "=== FCEUX11 v0.3.10 EMUFILE Caller Inventory ===" -ForegroundColor Cyan
Write-Host "Scan root : $Root$(if ($IncludeTests) { " + tests/" })"
Write-Host "Files with EMUFILE references: $totalFiles"
Write-Host "Total categorized hits       : $totalHits"
Write-Host ""

# Summary table
$inventory |
    Sort-Object Total -Descending |
    Select-Object File, Total, "EMUFILE type refs", ".fread()", ".fwrite()", "._fread()", ".buf()", ".get_vec()", "EMUFILE_MEMORY ctor", "EMUFILE_FILE ctor", "readAllBytes", "memwrap" |
    Format-Table -AutoSize |
    Out-String -Width 4096 |
    Write-Host

if ($Details) {
    foreach ($item in ($inventory | Sort-Object File)) {
        if ($item.Details.Count -eq 0) { continue }
        Write-Host "--- $($item.File) ---" -ForegroundColor Yellow
        foreach ($dl in $item.Details) { Write-Host $dl }
        Write-Host ""
    }
}

exit 0
