# Simulate the v1.4 CI drift checks locally.
# Mirrors the PowerShell logic in .github/workflows/ci.yml.

$ErrorActionPreference = "Stop"
Set-Location "D:\Project\FCEUX11"

function Test-Check {
    param([string]$Name, [string]$Cmd)
    Write-Host ""
    Write-Host "=== $Name ===" -ForegroundColor Cyan
    Write-Host $Cmd
    Invoke-Expression $Cmd
}

# Bus dispatch site drift check
Test-Check "Bus dispatch site drift check" {
    $changed = git diff --name-only HEAD~5..HEAD | Where-Object {
        $_ -match '^src/boards/.*\.cpp$' -or
        $_ -match '^src/bus\.(cpp|h)$' -or
        $_ -match '^src/cart\.(cpp|h)$'
    }
    if ($changed) {
        Write-Host "::warning::Bus-relevant files changed:"
        $changed | ForEach-Object { Write-Host "  - $_" }
    } else {
        Write-Host "No bus-surface sources changed."
    }
}

# Hot-path reference-alias drift check
Test-Check "Hot-path reference-alias drift check" {
    Set-Location "src"
    $hits = git grep -n "ARead\[\|BWrite\[\|VPage\[\|MMC5SPRVPage\[\|MMC5BGVPage\[" -- x6502.cpp ppu.cpp cart.cpp cheat.cpp debug.cpp nsf.cpp lua-engine.cpp 2>$null
    Set-Location ".."
    if ($hits) {
        $hotHits = $hits | Where-Object {
            $_ -notmatch 'fceu\.cpp' -and
            -not ($_ -match 'ppu\.cpp' -and $_ -match 'VPage\[|MMC5SPRVPage\[') -and
            -not ($_ -match 'ppu\.cpp:17[5-7]\d:' -and $_ -match 'ARead\[|BWrite\[')
        }
        if ($hotHits) {
            Write-Host "::error::Direct reference-alias indexing in hot-path files:" -ForegroundColor Red
            $hotHits | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
            return $false
        } else {
            Write-Host "PASS: only documented exceptions present."
        }
    } else {
        Write-Host "PASS: no direct ARead[]/BWrite[]/VPage[]/MMC5SPRVPage[]/MMC5BGVPage[] in hot-path files."
    }
    return $true
}

# Board-file direct-array drift check
Test-Check "Board-file direct-array drift check" {
    Set-Location "src"
    $hits = git grep -n "ARead\[\|BWrite\[\|Page\[\|VPage\[\|VPageG\[\|PRGptr\[\|CHRptr\[\|MMC5SPRVPage\[\|MMC5BGVPage\[\|VPageR\[" -- boards/ 2>$null
    Set-Location ".."
    if ($hits) {
        $boardHits = $hits | Where-Object {
            $_ -notmatch 'boards/datalatch\.cpp' -and
            $_ -notmatch 'boards/fns\.cpp:20[78]:' -and
            $_ -notmatch 'boards/fns\.cpp:21[08]:' -and
            $_ -notmatch 'boards/n106\.cpp.*NTAPage\['
        }
        if ($boardHits) {
            Write-Host "::error::Direct array indexing in board files:" -ForegroundColor Red
            $boardHits | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
            return $false
        } else {
            Write-Host "PASS: only documented exceptions present."
        }
    } else {
        Write-Host "PASS: no direct array indexing in board files."
    }
    return $true
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Green
Write-Host "All 3 CI drift checks simulated. Push to main → ci.yml build-windows will run these."
