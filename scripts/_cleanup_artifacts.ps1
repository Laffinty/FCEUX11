$ErrorActionPreference = "Continue"
Set-Location "Z:\Project\FCEUX11"

# Delete debug DATA artifacts (regenerable traces/logs). Keep the
# diagnostic scripts (scripts/_*.ps1, tools/_*.py) for the next session.
$patterns = @(
    "trace_*.csv",
    "__*.txt",
    "__*.log",
    "ins_log.txt",
    "blargg_result.json",
    "blargg_stderr.txt",
    "usage_err.txt"
)
$removed = 0
foreach ($pat in $patterns) {
    Get-ChildItem -File -Filter $pat -ErrorAction SilentlyContinue | ForEach-Object {
        Remove-Item $_.FullName -Force -ErrorAction SilentlyContinue
        $removed++
    }
}
Write-Host "removed=$removed files"
Write-Host "--- remaining root files ---"
Get-ChildItem -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match "^(__|trace_|ins_log|blargg_|usage_err)" } |
    Select-Object Name | Format-Table -AutoSize
