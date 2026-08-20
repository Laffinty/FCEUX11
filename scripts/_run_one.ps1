$p = Start-Process -FilePath "Z:\Project\FCEUX11\build-rust-cpu\tests\kagami_qa_cycle_trace.exe" `
    -ArgumentList "tests\fixtures\blargg\cpu\instr_v5_all.nes", "5", "trace_ps5.csv" `
    -Wait -PassThru -NoNewWindow
Write-Host "ExitCode=$($p.ExitCode)"
$f = Get-Item "trace_ps5.csv" -ErrorAction SilentlyContinue
if ($f) { Write-Host "csv=$($f.Length) bytes" } else { Write-Host "csv missing" }
