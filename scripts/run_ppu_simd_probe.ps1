# Run the v0.3.11 Phase-2 SIMD probe variants and emit a Markdown report.
param(
    [string]$BuildDir = "build",
    [string]$Config = "Release",
    [string]$OutReport = "docs/tech/v0.3.x_SIMD_Probe_Report.md"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root $BuildDir

function Run-Probe($name) {
    $exe = Join-Path (Join-Path (Join-Path $build "tests") $Config) "${name}.exe"
    if (!(Test-Path $exe)) {
        throw "Probe executable not found: $exe (build the 'tests/$name' target first)"
    }
    $output = & $exe
    if ($LASTEXITCODE -ne 0) { throw "Probe $name failed: $output" }
    return $output
}

function Parse-Result($line) {
    if ($line -match 'SCALAR_MS_PER_FRAME=([\d.]+)\s+AUTO_MS_PER_FRAME=([\d.]+)\s+SPEEDUP=([\d.]+)') {
        return @{
            scalar_ms = [double]$matches[1]
            auto_ms   = [double]$matches[2]
            speedup   = [double]$matches[3]
        }
    }
    return $null
}

$configs = @(
    @{ Name = "fceux11_ppu_simd_probe_scalar"; Arch = "scalar (no auto-vectorization)" }
    @{ Name = "fceux11_ppu_simd_probe_sse42";  Arch = "SSE4.2-class (/arch:SSE2)" }
    @{ Name = "fceux11_ppu_simd_probe_avx2";   Arch = "AVX2 (/arch:AVX2)" }
)

$rows = foreach ($c in $configs) {
    $line = Run-Probe $c.Name
    $r = Parse-Result $line
    if (!$r) { throw "Could not parse output from $($c.Name): $line" }
    [PSCustomObject]@{
        Variant = $c.Arch
        Scalar_ms_per_frame = $r.scalar_ms
        Auto_ms_per_frame = $r.auto_ms
        Speedup = $r.speedup
    }
}

$avx2 = $rows | Where-Object { $_.Variant -like "*AVX2*" }
$threshold = 1.20
$recommendation = if ($avx2.Speedup -ge $threshold) {
    "AVX2 speedup >= 20% threshold; SIMD optimization may be reconsidered in v0.4.x with a full PPU kernel rewrite."
} else {
    "AVX2 speedup < 20% threshold for this representative PPU workload; SIMD PPU optimization remains off the v0.3.x/v0.4.x roadmap per §3.4."
}

$date = Get-Date -Format "yyyy-MM-dd"
$md = @(
    "# v0.3.11 Phase-2 SIMD Probe Report"
    ""
    "- Date: $date"
    "- Workload: palette-index -> RGBA conversion over a 256x240 frame (PPU output stage)"
    "- Iterations per run: 100 (after 10 warmup frames)"
    ""
    "| Variant | Scalar ms/frame | Auto-vectorized ms/frame | Speedup |"
    "|---------|----------------:|-------------------------:|--------:|"
)
foreach ($row in $rows) {
    $md += "| $($row.Variant) | $($row.Scalar_ms_per_frame.ToString('F4')) | $($row.Auto_ms_per_frame.ToString('F4')) | $($row.Speedup.ToString('F3')) |"
}
$md += @(
    ""
    "## Conclusion"
    ""
    $recommendation
    ""
    "Note: this probe is intentionally isolated from the main emulator; per v0.3.11 spec, SIMD code does not enter the main branch."
)

$reportPath = Join-Path $root $OutReport
$reportDir = Split-Path -Parent $reportPath
if (!(Test-Path $reportDir)) { New-Item -ItemType Directory -Path $reportDir -Force | Out-Null }
$md | Set-Content -Path $reportPath -Encoding UTF8
Write-Host "Wrote report: $reportPath"
$rows | Format-Table -AutoSize
