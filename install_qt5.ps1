# PowerShell script to install MSYS2 dependencies
$msysPath = "D:\msys64\mingw64.exe"
$command = "pacman -Sy --noconfirm mingw-w64-x86_64-qt5-base mingw-w64-x86_64-qt5-tools mingw-w64-x86_64-SDL2 mingw-w64-x86_64-libarchive"

Write-Host "Installing dependencies using MSYS2..."
Write-Host "This may take several minutes..."

# Run MSYS2 pacman non-interactively
$process = Start-Process -FilePath $msysPath -ArgumentList "bash", "-c", $command -NoNewWindow -Wait -PassThru

if ($process.ExitCode -eq 0) {
    Write-Host "Dependencies installed successfully!" -ForegroundColor Green
} else {
    Write-Host "Failed to install dependencies. Exit code: $($process.ExitCode)" -ForegroundColor Red
}

Write-Host "Press any key to exit..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
