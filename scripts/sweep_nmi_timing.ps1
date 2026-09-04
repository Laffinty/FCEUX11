<#
.SYNOPSIS
Phase 6.6.quater.3.5 - multi-path parallel NMI/timing sweep.

Sweeps 4 combinations of (VBL set dot, NMI delay) and records
per-combination results via ctest -V (which sets the env vars
on the test process).

Tests captured:
  - kagami_qa_direct_smoke   (12 cases, including the 9 deferred)
  - rust_ppu_vbl_nmi_timing_test (02-vbl_set_time hash)
#>
$ErrorActionPreference = 'Stop'
Set-Location Z:\Project\FCEUX11

$BuildDir    = 'Z:\Project\FCEUX11\build-rust-ppu'
$ResultsFile = Join-Path $BuildDir 'sweep_nmi_timing_2026-09-04.json'
$env:VCPKG_ROOT = "Z:\Project\FCEUX11\vcpkg"
$env:PATH = "Z:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"

# Sweep matrix (4 groups per owner-approved scope)
$Matrix = @(
    @{ vbl_set_dot = 0; nmi_delay = 0; label = 'dot0_delay0' },
    @{ vbl_set_dot = 0; nmi_delay = 3; label = 'dot0_delay3_NESdev' },
    @{ vbl_set_dot = 1; nmi_delay = 0; label = 'dot1_delay0_current' },
    @{ vbl_set_dot = 1; nmi_delay = 3; label = 'dot1_delay3' }
)

$AllResults = @()
foreach ($m in $Matrix) {
    Write-Host ""
    Write-Host "=== Sweep: $($m.label) (VBL=$($m.vbl_set_dot), NMI_DELAY=$($m.nmi_delay)) ===" -ForegroundColor Cyan
    $env:FCEUX11_VBL_SET_DOT = "$($m.vbl_set_dot)"
    $env:FCEUX11_NMI_DELAY   = "$($m.nmi_delay)"

    Set-Location $BuildDir
    $ctestOut = cmd /c "ctest --output-on-failure -V -R ^(kagami_qa_direct_smoke^|rust_ppu_vbl_nmi_timing_test^) 2>&1" | Out-String
    Set-Location Z:\Project\FCEUX11

    # Extract the per-test result lines
    $directLines = @($ctestOut -split "`r?`n" | Where-Object { $_ -match '^\[direct\]' -or $_ -match 'Direct run complete' })
    $blarggLine  = ($ctestOut -split "`r?`n" | Where-Object { $_ -match '^BLARGG_RESULT:' } | Select-Object -First 1)
    $testSummary = ($ctestOut -split "`r?`n" | Where-Object { $_ -match '^[0-9]+/[0-9]+ Test' -or $_ -match 'Test Passed|Test Failed' })

    foreach ($l in $directLines) { Write-Host "  $l" }
    if ($blarggLine) { Write-Host "  $blarggLine" }
    foreach ($l in $testSummary) { Write-Host "  $l" }

    $AllResults += [pscustomobject]@{
        label        = $m.label
        vbl_set_dot  = $m.vbl_set_dot
        nmi_delay    = $m.nmi_delay
        blargg       = $blarggLine
        direct_lines = $directLines
        test_summary = $testSummary
    }
}

# Restore defaults
$env:FCEUX11_VBL_SET_DOT = $null
$env:FCEUX11_NMI_DELAY = $null

# Persist
$AllResults | ConvertTo-Json -Depth 5 | Out-File -FilePath $ResultsFile -Encoding UTF8
Write-Host ""
Write-Host "Results written to: $ResultsFile" -ForegroundColor Green