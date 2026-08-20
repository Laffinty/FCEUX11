$a = (Get-Item "trace_instr_r300b.csv" -ErrorAction SilentlyContinue).Length
Get-Process kagami_qa_cycle_trace -ErrorAction SilentlyContinue |
    Select-Object Id, CPU, StartTime | Format-Table -AutoSize
Start-Sleep -Seconds 20
$b = (Get-Item "trace_instr_r300b.csv" -ErrorAction SilentlyContinue).Length
Write-Host "size before=$a after=$b"
