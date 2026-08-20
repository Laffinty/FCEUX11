Get-Process kagami_qa_cycle_trace, kagami_qa_blargg_runner, fceux11 -ErrorAction SilentlyContinue |
    Select-Object Id, ProcessName, StartTime | Format-Table -AutoSize
Write-Host "--- root artifacts ---"
Get-ChildItem -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match "^(__|trace_|ins_log|blargg_|trace_test)" } |
    Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize
Write-Host "--- git status short ---"
git -C Z:\Project\FCEUX11 status --short
