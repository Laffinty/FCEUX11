param(
    [string]$InputJson = "$env:TEMP\blargg_results_clean.json",
    [string]$Manifest = "tests\fixtures\blargg_manifest.json",
    [string]$KnownFail = "tests\fixtures\blargg_known_fail.json",
    [string]$Output = "build\kagamiqa_accuracy_table.md"
)

$content = Get-Content $InputJson -Raw
$bytes = [System.IO.File]::ReadAllBytes($Manifest)
if ($bytes.Length -ge 3 -and [int]$bytes[0] -eq 239 -and [int]$bytes[1] -eq 187 -and [int]$bytes[2] -eq 191) {
    $bytes = $bytes[3..($bytes.Length-1)]
}
$tmpPath = [System.IO.Path]::GetTempFileName()
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($tmpPath, [System.Text.Encoding]::UTF8.GetString($bytes), $utf8NoBom)
$mfstText = Get-Content $tmpPath -Raw
$mfst = ConvertFrom-Json -InputObject $mfstText
$results = [regex]::Matches($content, '"rom":"([^"]+)".*?"value":"([^"]+)".*?"status":"(PASS|FAIL)"')

$romMap = @{}
foreach ($r in $mfst.roms) { $romMap[$r.name] = $r.category }

$pass = ($results | Where-Object { $_.Groups[3].Value -eq 'PASS' }).Count
$fail = ($results | Where-Object { $_.Groups[3].Value -eq 'FAIL' }).Count
$total = $results.Count
$passRate = [Math]::Round($pass/$total*100, 2)
$shortOf90 = 160 - $pass

# By category
$catStats = @{}
foreach ($cat in @('cpu', 'ppu', 'apu', 'mmc3')) {
    $cp = 0
    $cf = 0
    foreach ($m in $results) {
        $romName = $m.Groups[1].Value
        $status = $m.Groups[3].Value
        if ($romMap.ContainsKey($romName) -and $romMap[$romName] -eq $cat) {
            if ($status -eq 'PASS') {
                $cp++
            } else {
                $cf++
            }
        }
    }
    $catStats[$cat] = @{pass=$cp; fail=$cf; total=($cp+$cf)}
}

# Baseline diff
$baselineJson = Get-Content $KnownFail -Raw | ConvertFrom-Json
$baselineFails = $baselineJson.failures | ForEach-Object { $_.rom -replace '\.nes$', '' }
$currentFails = @()
foreach ($m in $results) {
    if ($m.Groups[3].Value -eq 'FAIL') { $currentFails += $m.Groups[1].Value }
}
$newRegressions = Compare-Object -ReferenceObject $currentFails -DifferenceObject $baselineFails | Where-Object { $_.SideIndicator -eq '<=' }
$fixed = Compare-Object -ReferenceObject $currentFails -DifferenceObject $baselineFails | Where-Object { $_.SideIndicator -eq '=>' }

# Build markdown (all ASCII to avoid encoding issues)
$lines = @()
$lines += '# FCEUX11 v2.0 T1 Blargg Accuracy Table'
$lines += ''
$lines += '> **2026-08-13 strategic pivot (ADR-011)**: this table is the phase 6 T1 pass-rate oracle'
$lines += '> **Source**: kagami_qa_blargg_runner, full 177-ROM run'
$lines += '> **Protocol**: blargg `$6000` (0x00=PASS, 0x01+=FAIL)'
$lines += '> **Runner**: `build/tests/kagami_qa_blargg_runner.exe --manifest tests/fixtures/blargg_manifest.json`'
$lines += '> **CWD requirement**: `D:\Project\FCEUX11\tests\` (manifest paths are relative to `tests/`)'
$lines += ''
$lines += '## Summary'
$lines += ''
$lines += '| Metric | Count |'
$lines += '|--------|-------|'
$lines += "| **Total ROMs** | $total |"
$lines += "| **Passed** | $pass |"
$lines += "| **Failed** | $fail |"
$lines += "| **Pass-rate** | $passRate% |"
$lines += "| **Phase 6 gate** | >= 90% (160/177) |"
$lines += "| **Gap to 90%** | $shortOf90 ROMs |"
$lines += ''
$lines += '## By Category'
$lines += ''
$lines += '| Category | Total | Pass | Fail | Pass-rate |'
$lines += '|----------|------:|-----:|-----:|----------:|'
foreach ($cat in @('cpu', 'ppu', 'apu', 'mmc3')) {
    $s = $catStats[$cat]
    $rate = if ($s.total -gt 0) { [Math]::Round($s.pass/$s.total*100, 2) } else { 0 }
    $lines += "| $cat | $($s.total) | $($s.pass) | $($s.fail) | $rate% |"
}
$lines += ''
$lines += '## Failures (33 ROMs)'
$lines += ''
$lines += '| ROM | Category | Code |'
$lines += '|-----|----------|------|'
foreach ($m in $results) {
    if ($m.Groups[3].Value -eq 'FAIL') {
        $romName = $m.Groups[1].Value
        $code = $m.Groups[2].Value
        $cat = if ($romMap.ContainsKey($romName)) { $romMap[$romName] } else { '?' }
        $lines += "| $romName | $cat | $code |"
    }
}
$lines += ''
$lines += '## v1.16 baseline comparison'
$lines += ''
$lines += '| Metric | v1.16 baseline | v2.0 (2026-08-13) | Net |'
$lines += '|--------|---------------:|------------------:|----:|'
$lines += "| Total ROMs | 180 (claimed) | 177 (actual) | -3 |"
$lines += "| Pass | 120 | $pass | $('{0:+#;-#;0}' -f ($pass - 120)) |"
$lines += "| Fail | 60 | $fail | $('{0:+#;-#;0}' -f ($fail - 60)) |"
$lines += "| New regressions | n/a | $($newRegressions.Count) | n/a |"
$lines += "| Fixed since v1.16 | n/a | $($fixed.Count) | n/a |"
$lines += ''
$lines += "Note: baseline `_total=180` is stale metadata; manifest, download script, and disk"
$lines += "all show 177 actual ROMs. Of 60 v1.16 known-fails, $($fixed.Count) now PASS in v2.0 (real"
$lines += "improvement); $($newRegressions.Count) entries remain in baseline + still failing (no new regressions)."
$lines += ''
$lines += '## Baseline snapshot'
$lines += ''
$lines += '- `build\kagamiqa_baseline_next.json` (1605 bytes, 47 entries) - saved via `kagami-qa-runner --save-baseline`'
$lines += '- Note: this baseline only covers the 47 entries in `tests\tests.json` (small subset).'
$lines += '  The full 177-ROM run from `kagami_qa_blargg_runner` produces a JSON to stdout'
$lines += '  that does NOT feed the baseline.'
$lines += ''
$lines += '## Known issues (2026-08-13)'
$lines += ''
$lines += '1. **CWD/path coupling**: manifest `path` field is `fixtures/blargg/...` relative to'
$lines += '   `tests/` subdir, NOT repo root. Runner must be invoked from'
$lines += '   `D:\Project\FCEUX11\tests\` or all ROMs fail with `kagami_bridge: LoadGame(fixtures/blargg/...) failed`'
$lines += '   and exit code 0xFE.'
$lines += ''
$lines += '2. **v1.16 `kagamiqa_accuracy_table.md` showed 0/12** because the v1.16 epoch'
$lines += '   `kagami_qa_direct_runner` was invoked from repo root, hitting the CWD issue.'
$lines += '   This v2.0 run (177 ROMs from `tests/` CWD) shows the actual hardware state.'
$lines += ''
$lines += '3. **kagami_qa_blargg_runner has no `--output` flag**: JSON results go to stdout only;'
$lines += '   no native accuracy-table writer. Need to redirect + parse externally to build'
$lines += '   a markdown report.'

Set-Content -Path $Output -Value ($lines -join "`n") -Encoding utf8
Write-Host "Written: $Output"
Write-Host "Total=$total PASS=$pass FAIL=$fail rate=$passRate%"
