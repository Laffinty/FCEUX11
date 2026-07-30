$ErrorActionPreference = "Continue"
$ROOT = 'D:\Project\FCEUX11'
Set-Location "$ROOT\build-c1\tests"
$env:PATH = "$ROOT\build-c1\tests;$ROOT\vcpkg_installed\x64-windows\bin;$ROOT\vcpkg_installed\x64-windows\debug/bin;" + $env:PATH

$roms = @(
  @{n="01_basics"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_01_basics.nes"},
  @{n="02_set_time"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_02_set_time.nes"},
  @{n="03_clear_time"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_03_clear_time.nes"},
  @{n="04_nmi_control"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_04_nmi_control.nes"},
  @{n="05_nmi_timing"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_05_nmi_timing.nes"},
  @{n="06_suppression"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_06_suppression.nes"},
  @{n="07_nmi_on_timing"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_07_nmi_on_timing.nes"},
  @{n="08_nmi_off_timing"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_08_nmi_off_timing.nes"},
  @{n="09_even_odd_frames"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_09_even_odd_frames.nes"},
  @{n="10_even_odd_timing"; r="$ROOT\tests\fixtures\blargg\ppu\vbl_10_even_odd_timing.nes"}
)

foreach ($t in $roms) {
  Write-Host ("=== vbl_" + $t.n + " ===")
  $p = Start-Process -FilePath ".\fceux11_blargg_runner.exe" `
       -ArgumentList @("--rom", $t.r, "--frames", "600") `
       -RedirectStandardOutput "stdout_$($t.n).txt" `
       -RedirectStandardError  "stderr_$($t.n).txt" `
       -PassThru -Wait -NoNewWindow
  Write-Host ("  exit=" + $p.ExitCode)
  if (Test-Path "stdout_$($t.n).txt") {
    Get-Content "stdout_$($t.n).txt" | ForEach-Object { Write-Host ("  | " + $_) }
  }
}
