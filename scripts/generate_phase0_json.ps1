# Phase 0 — Generate v2.1 baseline JSON files from captured logs.

param(
    [string]$InDir  = "$PWD\build\v2.1_phase0",
    [string]$OutDir = "$PWD\tests\fixtures"
)

$ErrorActionPreference = "Continue"

# --- Parse blargg VBL subset results ---------------------------------------
$blarggFiles = @(
    @{ name = "blargg_ppu_vbl_nmi";        path = "blargg_vbl_nmi.log" },
    @{ name = "blargg_ppu_vbl_clear_time"; path = "blargg_vbl_clear.log" },
    @{ name = "blargg_scanline";           path = "blargg_scanline.log" },
    @{ name = "blargg_scanline_a1";        path = "blargg_scanline_a1.log" }
)

$blarggResults = @{}
foreach ($f in $blarggFiles) {
    $logPath = Join-Path $InDir $f.path
    $content = Get-Content $logPath -Raw -ErrorAction SilentlyContinue
    if ($content -match 'status=PASS') {
        $blarggResults[$f.name] = $true
    } elseif ($content -match 'status=FAIL') {
        $blarggResults[$f.name] = $false
    } else {
        $blarggResults[$f.name] = $null
    }
}

# --- Parse CTest results ----------------------------------------------------
$ctestLog = Get-Content (Join-Path $InDir "ctest.log") -Raw
$ctestResults = @{}
# Lines look like: " 1/33 Test  #1: smoke_test ........................   Passed    0.33 sec"
foreach ($line in ($ctestLog -split "`n")) {
    if ($line -match '^\s*\d+/\d+\s+Test\s+#\d+:\s+(\S+)\s+\.+\s+(Passed|Failed|Skipped)') {
        $name = $matches[1]
        $ctestResults[$name] = ($matches[2] -eq 'Passed')
    }
}

# --- Parse matrix results ---------------------------------------------------
$matrixPath = Join-Path $InDir "kagamiqa_direct_matrix_v2.1.json"
$matrixData = $null
if (Test-Path $matrixPath) {
    $matrixData = Get-Content $matrixPath -Raw | ConvertFrom-Json
}

# --- Compose baseline JSONs -------------------------------------------------
$stamp = (Get-Date).ToUniversalTime().ToString("o")
$metaPath = Join-Path $InDir "metadata.json"
$meta = Get-Content $metaPath -Raw | ConvertFrom-Json

$blarggOut = @{
    generated_at      = $stamp
    run_id            = "phase0-$($meta.git_head.Substring(0,7))"
    branch            = $meta.git_branch
    git_head          = $meta.git_head
    fceux11_exe_sha256 = $meta.fceux11_exe_sha256
    scope             = "v2.1 PPU Rust refactor Phase 0 — VBL/NMI/scanline frozen subset"
    frames_per_rom    = "vbl_nmi=600, vbl_clear=600, scanline=300, scanline_a1=300"
    json_schema       = @{ test_id = "bool: true=PASS, false=FAIL, null=inconclusive" }
    results           = $blarggResults
}
$blarggOut | ConvertTo-Json -Depth 5 | Set-Content `
    (Join-Path $OutDir "v2.1_phase0_blargg_baseline.json") -Encoding UTF8

# CTest baseline — only save what passed; the existing kagamiqa_baseline_frozen
# covers the broader matrix, this is the perf-excluded ctest slice.
$ctestOut = @{
    generated_at      = $stamp
    run_id            = "phase0-$($meta.git_head.Substring(0,7))"
    branch            = $meta.git_branch
    git_head          = $meta.git_head
    fceux11_exe_sha256 = $meta.fceux11_exe_sha256
    scope             = "v2.1 PPU Rust refactor Phase 0 — ctest -LE perf (33 tests)"
    source_log        = "build/v2.1_phase0/ctest.log"
    summary           = @{
        total  = $ctestResults.Count
        passed = ($ctestResults.Values | Where-Object { $_ }).Count
        failed = ($ctestResults.Values | Where-Object { -not $_ }).Count
    }
    results           = $ctestResults
}
$ctestOut | ConvertTo-Json -Depth 5 | Set-Content `
    (Join-Path $OutDir "v2.1_phase0_ctest_baseline.json") -Encoding UTF8

# --- Print summary ----------------------------------------------------------
""
Write-Host "Generated v2.1 Phase 0 baseline JSONs:"
Write-Host "  - tests/fixtures/v2.1_phase0_blargg_baseline.json"
Write-Host "  - tests/fixtures/v2.1_phase0_ctest_baseline.json"
""
Write-Host "blargg VBL subset:"
$blarggResults.GetEnumerator() | Sort-Object Name | ForEach-Object {
    $sym = if ($_.Value -eq $true) { "PASS" } elseif ($_.Value -eq $false) { "FAIL" } else { "?" }
    Write-Host ("  {0,-35}  {1}" -f $_.Name, $sym)
}
""
Write-Host "CTest (perf-excluded):"
Write-Host ("  total={0} passed={1} failed={2}" -f `
    $ctestOut.summary.total, $ctestOut.summary.passed, $ctestOut.summary.failed)
""
if ($matrixData) {
    Write-Host "KagamiQA direct matrix (smoke, 12 cases):"
    Write-Host ("  passed={0} failed={1} total={2}" -f `
        $matrixData.summary.passed, $matrixData.summary.failed, $matrixData.summary.total)
}
