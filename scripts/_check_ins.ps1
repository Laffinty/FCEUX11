$a = (Get-Item "__ins_rust.txt" -ErrorAction SilentlyContinue).Length
$b = (Get-Item "trace_ins_rust.csv" -ErrorAction SilentlyContinue).Length
Get-Process kagami_qa_cycle_trace -ErrorAction SilentlyContinue |
    Select-Object Id, CPU, StartTime | Format-Table -AutoSize
Start-Sleep -Seconds 20
$a2 = (Get-Item "__ins_rust.txt" -ErrorAction SilentlyContinue).Length
$b2 = (Get-Item "trace_ins_rust.csv" -ErrorAction SilentlyContinue).Length
Write-Host "ins_rust: $a -> $a2 bytes (log), csv: $b -> $b2"
