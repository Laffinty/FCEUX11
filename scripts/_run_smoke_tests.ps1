# Test runner — executes a suite of FCEUX11 test executables and reports results.
$tests = @(
  'fceux11_enum_class_bitflags_test.exe',
  'fceux11_smoke_test.exe',
  'fceux11_rom_regression_test.exe',
  'fceux11_savestate_regression_test.exe',
  'fceux11_pixbuf_pool_test.exe',
  'fceux11_i18n_regression_test.exe',
  'fceux11_config_store_test.exe',
  'fceux11_expected_api_test.exe',
  'fceux11_bus_test.exe',
  'fceux11_cpu_test.exe',
  'fceux11_ppu_test.exe',
  'fceux11_apu_test.exe',
  'fceux11_mapper_load_test.exe',
  'fceux11_mapper_reset_test.exe',
  'fceux11_mapper_core_test.exe'
)
$report = @()
foreach ($t in $tests) {
  $exe = "D:\Project\FCEUX11\build\tests\$t"
  if (-not (Test-Path $exe)) { continue }
  $out = "D:\Project\FCEUX11\output\$([IO.Path]::GetFileNameWithoutExtension($t)).out"
  $err = "D:\Project\FCEUX11\output\$([IO.Path]::GetFileNameWithoutExtension($t)).err"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $exe -PassThru -RedirectStandardOutput $out -RedirectStandardError $err -NoNewWindow
  $done = $p.WaitForExit(90000)
  if (-not $done) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    $report += "$t  TIMEOUT(90s)"
  } else {
    $sw.Stop()
    $rc = $p.ExitCode
    $elapsed = ('{0:0.0}s' -f $sw.Elapsed.TotalSeconds)
    $report += "$t  exit=$rc  time=$elapsed"
  }
}
$report | ForEach-Object { Write-Host $_ }
