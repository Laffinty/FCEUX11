$f = Get-Item "__ins_rust.txt" -ErrorAction SilentlyContinue
$p = Get-Process kagami_qa_cycle_trace -ErrorAction SilentlyContinue |
    Where-Object { $_.StartTime -gt (Get-Date).AddHours(-1) }
Write-Host "ins_rust_bytes=$($f.Length) lines=$((Get-Content $f -ErrorAction SilentlyContinue).Count)"
$p | Select-Object Id, CPU | Format-Table -AutoSize
