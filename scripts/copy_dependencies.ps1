param(
    [Parameter(Mandatory=$true)]
    [string]$Executable,

    [Parameter(Mandatory=$true)]
    [string]$OutputDir
)

$env:PATH = "D:\msys64\mingw64\bin;D:\msys64\usr\bin;$env:PATH"

if (-not (Test-Path $Executable)) {
    Write-Error "Executable not found: $Executable"
    exit 1
}

Write-Host "Analyzing dependencies..." -ForegroundColor Cyan

$objdumpPath = "D:\msys64\mingw64\bin\objdump.exe"
$dlls = & $objdumpPath -x $Executable | Select-String "DLL Name:" | ForEach-Object {
    ($_.Line -replace ".*DLL Name:\s*", "").Trim()
} | Sort-Object -Unique

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$copiedCount = 0
$skippedCount = 0

Write-Host "`nRequired DLLs:" -ForegroundColor Yellow
Write-Host ("=" * 50)

foreach ($dll in $dlls) {
    $sourcePath = "D:\msys64\mingw64\bin\$dll"
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
    $sourcePath = "D:\msys64\mingw64\bin\$dll"
    if (Test-Path $sourcePath) {
        $indirect = & $objdumpPath -x $sourcePath | Select-String "DLL Name:" | ForEach-Object {
            ($_.Line -replace ".*DLL Name:\s*", "").Trim()
        }

        foreach ($ind in $indirect) {
            if ($ind -notin $dlls -and $ind -notin $indirectDlls) {
                $indirectDlls += $ind
            }
        }
    }
}

Write-Host "`nIndirect DLLs:" -ForegroundColor Yellow
foreach ($dll in $indirectDlls | Sort-Object -Unique) {
    Write-Host "  $dll" -ForegroundColor Gray
}

Write-Host "`nCopying indirect DLLs..." -ForegroundColor Cyan
foreach ($dll in $indirectDlls | Sort-Object -Unique) {
    $sourcePath = "D:\msys64\mingw64\bin\$dll"
    $destPath = Join-Path $OutputDir $dll

    if ((Test-Path $sourcePath) -and (-not (Test-Path $destPath))) {
        Copy-Item $sourcePath -Destination $destPath -Force
        Write-Host "[COPY] $dll" -ForegroundColor Green
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

$batContent = "@echo off`nset PATH=D:\msys64\mingw64\bin;D:\msys64\usr\bin;%PATH%`ncd /d `"$exePath`"`nstart $exeName`n"
[System.IO.File]::WriteAllText($batPath, $batContent)

Write-Host "Launch script: $batPath" -ForegroundColor Green
Write-Host "`nRun the bat file to start the program" -ForegroundColor Cyan