$ErrorActionPreference = "Continue"
$rom = "tests\fixtures\blargg\cpu\instr_v5_all.nes"
$frames = "300"

$env:FCEUX11_LOG_REG = "1"

Write-Host "[1/4] rust reg log + trace"
& .\build-rust-cpu\tests\kagami_qa_cycle_trace.exe $rom $frames trace_instr_r300b.csv 2> __reg_rust2.txt
Write-Host "[2/4] cpp reg log + trace"
& .\build-off\tests\kagami_qa_cycle_trace.exe $rom $frames trace_instr_c300b.csv 2> __reg_cpp2.txt

Remove-Item Env:FCEUX11_LOG_REG

Write-Host "[3/4] trace diff"
python tools\find_irq_full.py trace_instr_r300b.csv trace_instr_c300b.csv

Write-Host "[4/4] reg log line counts"
$r = (Get-Content __reg_rust2.txt).Count
$c = (Get-Content __reg_cpp2.txt).Count
Write-Host "rust=$r cpp=$c"
