param(
    [Parameter(Mandatory=$true)]
    [string]$Executable,

    [Parameter(Mandatory=$true)]
    [string]$OutputDir
)

function Find-MSys64Path {
    $searchLocations = @(
        "D:\msys64",
        "C:\msys64",
        "$env:LOCALAPPDATA\msys64",
        "$env:ProgramFiles\msys64",
        "$env:SystemRoot\Sysnative\msys64",
        "$env:HOME\msys64"
    )

    foreach ($loc in $searchLocations) {
        if (Test-Path "$loc\mingw64\bin") {
            return $loc
        }
    }

    $drives = @("C:", "D:", "E:", "F:")
    foreach ($drive in $drives) {
        $msysPath = "$drive\msys64"
        if (Test-Path "$msysPath\mingw64\bin") {
            return $msysPath
        }
    }

    $regPaths = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )
    foreach ($regPath in $regPaths) {
        $items = Get-ItemProperty $regPath -ErrorAction SilentlyContinue | Where-Object {
            $_.InstallLocation -like "*msys*" -or $_.DisplayName -like "*MSYS*"
        }
        if ($items) {
            foreach ($item in $items) {
                $installPath = $item.InstallLocation
                if ($installPath -and (Test-Path "$installPath\mingw64\bin")) {
                    return $installPath
                }
            }
        }
    }

    return $null
}

$msys64Root = Find-MSys64Path

if (-not $msys64Root) {
    Write-Error "MSYS2 installation not found. Please install MSYS2 or set MSYS2_ROOT environment variable."
    Write-Host "Download MSYS2 from: https://www.msys2.org/" -ForegroundColor Yellow
    exit 1
}

$mingwBin = Join-Path $msys64Root "mingw64\bin"
$msysBin = Join-Path $msys64Root "usr\bin"

Write-Host "Detected MSYS2 at: $msys64Root" -ForegroundColor Green
Write-Host "Using MinGW bin: $mingwBin" -ForegroundColor Gray

$env:PATH = "$mingwBin;$msysBin;$env:PATH"

if (-not (Test-Path $Executable)) {
    Write-Error "Executable not found: $Executable"
    exit 1
}

Write-Host "Analyzing dependencies..." -ForegroundColor Cyan

$objdumpPath = Join-Path $mingwBin "objdump.exe"
if (-not (Test-Path $objdumpPath)) {
    $objdumpPath = "objdump.exe"
}

$dlls = & $objdumpPath -x $Executable 2>$null | Select-String "DLL Name:" | ForEach-Object {
    ($_.Line -replace ".*DLL Name:\s*", "").Trim()
} | Sort-Object -Unique

if (-not $dlls) {
    Write-Warning "No DLL dependencies found or objdump failed."
    $dlls = @()
}

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$copiedCount = 0
$skippedCount = 0

Write-Host "`nRequired DLLs:" -ForegroundColor Yellow
Write-Host ("=" * 50)

foreach ($dll in $dlls) {
    $sourcePath = Join-Path $mingwBin $dll
    $destPath = Join-Path $OutputDir $dll

    if (Test-Path $sourcePath) {
        Write-Host "[COPY] $dll" -ForegroundColor Green
        Copy-Item $sourcePath -Destination $destPath -Force
        $copiedCount++
    } else {
        Write-Host "[SKIP] $dll (System)" -ForegroundColor Gray
        $skippedCount++
    }
}

Write-Host ("=" * 50)
Write-Host "`nAnalyzing indirect dependencies..." -ForegroundColor Cyan

$indirectDlls = @()
foreach ($dll in $dlls) {
    $sourcePath = Join-Path $mingwBin $dll
    if (Test-Path $sourcePath) {
        $indirect = & $objdumpPath -x $sourcePath 2>$null | Select-String "DLL Name:" | ForEach-Object {
            ($_.Line -replace ".*DLL Name:\s*", "").Trim()
        }

        foreach ($ind in $indirect) {
            if ($ind -notin $dlls -and $ind -notin $indirectDlls) {
                $indirectDlls += $ind
            }
        }
    }
}

if ($indirectDlls.Count -gt 0) {
    Write-Host "`nIndirect DLLs:" -ForegroundColor Yellow
    foreach ($dll in $indirectDlls | Sort-Object -Unique) {
        Write-Host "  $dll" -ForegroundColor Gray
    }

    Write-Host "`nCopying indirect DLLs..." -ForegroundColor Cyan
    foreach ($dll in $indirectDlls | Sort-Object -Unique) {
        $sourcePath = Join-Path $mingwBin $dll
        $destPath = Join-Path $OutputDir $dll

        if ((Test-Path $sourcePath) -and (-not (Test-Path $destPath))) {
            Copy-Item $sourcePath -Destination $destPath -Force
            Write-Host "[COPY] $dll" -ForegroundColor Green
        }
    }
}

$finalDlls = (Get-ChildItem $OutputDir -Filter "*.dll").Name | Sort-Object

Write-Host "`n" + ("=" * 50) -ForegroundColor Cyan
Write-Host "Done!" -ForegroundColor Green
Write-Host "Direct dependencies: $copiedCount DLLs" -ForegroundColor Cyan
Write-Host "System DLLs: $skippedCount DLLs" -ForegroundColor Gray
Write-Host "Output: $OutputDir" -ForegroundColor Cyan
Write-Host ("=" * 50)

Write-Host "`nFinal DLL list:" -ForegroundColor Yellow
$finalDlls | ForEach-Object { Write-Host "  $_" }

Write-Host "`nGenerating launch script..." -ForegroundColor Cyan
$exePath = (Get-Item $Executable).DirectoryName
$exeName = (Get-Item $Executable).Name
$batPath = Join-Path $OutputDir "run_$($exeName -replace '\.exe$','').bat"

$batContent = "@echo off`nset PATH=$mingwBin;$msysBin;%PATH%`ncd /d `"$exePath`"`nstart $exeName`n"
[System.IO.File]::WriteAllText($batPath, $batContent)

Write-Host "Launch script: $batPath" -ForegroundColor Green
Write-Host "`nRun the bat file to start the program" -ForegroundColor Cyan
