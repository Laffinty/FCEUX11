# Test runner — executes from tests/ directory so fixtures/ is resolvable
Set-Location D:\Project\FCEUX11\tests
$tests = @(
  'fceux11_enum_class_bitflags_test.exe',
  'fceux11_smoke_test.exe',
  'fceux11_pixbuf_pool_test.exe',
  'fceux11_i18n_regression_test.exe',
  'fceux11_config_store_test.exe',
  'fceux11_expected_api_test.exe',
  'fceux11_cpu_test.exe',
  'fceux11_ppu_test.exe',
  'fceux11_apu_test.exe',
  'fceux11_bus_test.exe',
  'fceux11_mapper_load_test.exe',
  'fceux11_mapper_reset_test.exe',
  'fceux11_mapper_core_test.exe',
  'fceux11_rom_regression_test.exe',
  'fceux11_savestate_regression_test.exe'
)
$report = @()
foreach ($t in $tests) {
  $exe = "D:\Project\FCEUX11\build\tests\$t"
  $tag = [IO.Path]::GetFileNameWithoutExtension($t)
  if (-not (Test-Path $exe)) {
    $report += "$t  MISSING"
    continue
  }
  $out = "D:\Project\FCEUX11\output\${tag}_v2.out"
  $err = "D:\Project\FCEUX11\output\${tag}_v2.err"
  if (Test-Path $out) { Remove-Item $out -Force }
  if (Test-Path $err) { Remove-Item $err -Force }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $exe -WorkingDirectory 'D:\Project\FCEUX11\tests' -PassThru -RedirectStandardOutput $out -RedirectStandardError $err -NoNewWindow
  $done = $p.WaitForExit(120000)
  $sw.Stop()
  if (-not $done) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    $report += "$t  TIMEOUT(120s)"
  } else {
    $rc = $p.ExitCode
    $elapsed = ('{0:0.0}s' -f $sw.Elapsed.TotalSeconds)
    $report += "$t  exit=$rc  time=$elapsed"
  }
}
$report | ForEach-Object { Write-Host $_ }
