# FCEUX11 Dependency Deployment Script
#
# Produces a directory that runs on a machine with no Visual Studio, no vcpkg
# and no VC++ Redistributable:
#   * Qt plugins (platforms/styles/imageformats) -- the platform plugin is
#     mandatory, the exe aborts at startup without it;
#   * the full dependency closure of the exe and of every shipped DLL,
#     iterated until stable (ICU data, brotli, jpeg, ...);
#   * the VC++ runtime, deployed app-local;
#   * the license texts the distribution must carry (vcpkg port copyrights,
#     the vendored GBA core's MIT LICENSE, the project's own COPYING).
#
# Requires dumpbin on PATH (a Visual Studio developer environment); it fails
# loudly rather than shipping a package whose dependencies are unknown.
#
# v2.0.6: the three packaging defects above (Qt plugins, closure depth, VC++
# runtime) plus the silent-empty-dependency-analysis path were fixed here; the
# v2.0.6 release package was the first built with this version.
param(
    [Parameter(Mandatory=$true)]
    [string]$ExecutablePath,

    [string]$OutputDir = (Split-Path $ExecutablePath -Parent),

    [string]$VcpkgRoot = $env:VCPKG_ROOT
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ExecutablePath)) {
    throw "Executable not found: $ExecutablePath"
}

# Resolve vcpkg installed bin directory
$vcpkgBin = $null
if ($VcpkgRoot) {
    $candidate = Join-Path $VcpkgRoot "installed\x64-windows\bin"
    if (Test-Path $candidate) {
        $vcpkgBin = $candidate
    }
}

# Fallback: search common locations
if (-not $vcpkgBin) {
    $candidates = @(
        "$PSScriptRoot\..\build\vcpkg_installed\x64-windows\bin"
        "$PSScriptRoot\..\vcpkg_installed\x64-windows\bin"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) {
            $vcpkgBin = $c
            break
        }
    }
}

if (-not $vcpkgBin) {
    throw "vcpkg installed bin directory not found. Specify -VcpkgRoot or ensure build/vcpkg_installed exists."
}

Write-Host "Using vcpkg bin: $vcpkgBin" -ForegroundColor Gray

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# dumpbin is a HARD requirement, not a nice-to-have.
#
# It only exists on PATH inside a Visual Studio developer environment. The old
# version swallowed its absence (`Get-Command` returned nothing, the analysis
# produced an empty list) and the script then "succeeded" while copying zero
# DLLs -- a package that cannot start on any machine, with no error printed
# anywhere. Fail loudly instead.
$dumpbin = (Get-Command dumpbin -ErrorAction SilentlyContinue).Source
if (-not $dumpbin) {
    throw @"
dumpbin not found on PATH, so dependency analysis cannot run.

Run this script from a Visual Studio developer prompt, or call vcvars64.bat
first, e.g.:

  cmd /c "call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && powershell -NoProfile -ExecutionPolicy Bypass -File scripts\copy_dependencies.ps1 -ExecutablePath build\src\fceux11.exe -OutputDir dist"

Without it the output directory would contain a binary whose dependencies are
unknown, which is worse than no package at all.
"@
}

function Get-PeDependencies {
    param([string]$Path)
    $dlls = @()
    $output = & $dumpbin /nologo /DEPENDENTS $Path 2>$null
    foreach ($line in $output) {
        if ($line -match '^\s+([\w\-\.]+\.dll)\s*$') {
            $dlls += $matches[1]
        }
    }
    return $dlls | Sort-Object -Unique
}

# Satisfied by Windows itself: the api-ms-win-* API sets (virtual since Win10,
# resolved by the loader rather than read from disk) and anything that really
# does ship in System32 on a stock Windows install.
function Test-SystemDll {
    param([string]$Name)
    if ($Name -like 'api-ms-win-*') { return $true }
    return (Test-Path (Join-Path $env:SystemRoot ("System32\" + $Name)))
}

$copied = 0

Write-Host "`nDeploying dependencies..." -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# 1. Qt plugins. The platform plugin is not optional: without
#    platforms\qwindows.dll the exe aborts at startup with "could not find or
#    load the Qt platform plugin". styles and imageformats carry the shipped
#    look and the image loaders. This step used to be missing entirely.
# ---------------------------------------------------------------------------
$qtPlugins = Join-Path (Split-Path $vcpkgBin -Parent) "Qt6\plugins"
if (Test-Path $qtPlugins) {
    foreach ($sub in @("platforms", "styles", "imageformats")) {
        $srcDir = Join-Path $qtPlugins $sub
        if (-not (Test-Path $srcDir)) { continue }
        $dstDir = Join-Path $OutputDir $sub
        New-Item -ItemType Directory -Path $dstDir -Force | Out-Null
        Get-ChildItem $srcDir -Filter *.dll | ForEach-Object {
            Copy-Item $_.FullName -Destination $dstDir -Force
        }
        $n = (Get-ChildItem $dstDir -Filter *.dll).Count
        Write-Host "  [PLUGINS] $sub -> $n dll(s)" -ForegroundColor Green
    }
} else {
    Write-Host "  [WARN] Qt plugin directory not found at $qtPlugins" -ForegroundColor Yellow
}

# ---------------------------------------------------------------------------
# 2. Dependency closure.
#
#    One level of transitives is not enough. Real examples from this tree:
#    Qt6Core -> icuuc78 -> icudt78 (ICU data); freetype -> brotlidec ->
#    brotlicommon; the Qt jpeg plugin -> jpeg62. The old two-pass version
#    labelled all of those "system" because they were simply not looked at
#    again, and the resulting package died with 0xC0000135 (DLL not found)
#    before main().
#
#    Iterate until nothing new is required. Roots are the executable plus every
#    DLL already in the output directory (including the Qt plugins just
#    deployed); everything they need is copied to the output ROOT, which is
#    where the Windows loader looks for it when the exe starts.
# ---------------------------------------------------------------------------
$unresolved = [ordered]@{}
for ($iter = 0; $iter -lt 10; $iter++) {
    $roots = @($ExecutablePath) + @(Get-ChildItem $OutputDir -Recurse -Filter *.dll | ForEach-Object { $_.FullName })
    $have = @{}
    Get-ChildItem $OutputDir -Recurse -Filter *.dll | ForEach-Object { $have[$_.Name.ToLower()] = $true }

    $need = @()
    foreach ($root in $roots) {
        foreach ($d in (Get-PeDependencies -Path $root)) {
            if ($have.ContainsKey($d.ToLower())) { continue }
            if (Test-SystemDll $d) { continue }
            if ($d -notin $need) { $need += $d }
        }
    }

    if (-not $need) {
        Write-Host "  [CLOSURE] complete after $iter iteration(s)" -ForegroundColor Gray
        break
    }

    $progress = $false
    foreach ($d in $need) {
        $src = Join-Path $vcpkgBin $d
        if (Test-Path $src) {
            Copy-Item $src -Destination (Join-Path $OutputDir $d) -Force
            if ($iter -eq 0) { Write-Host "  [COPY] $d" -ForegroundColor Green }
            else             { Write-Host "  [COPY] $d (transitive)" -ForegroundColor Green }
            $copied++
            $progress = $true
        } else {
            $unresolved[$d] = $true
        }
    }
    if (-not $progress) { break }
}

# ---------------------------------------------------------------------------
# 3. VC++ runtime, deployed app-local.
#
#    MSVCP140.dll / VCRUNTIME140.dll / VCRUNTIME140_1.dll exist in System32 on
#    any machine that ever installed Visual Studio or a redistributable -- which
#    includes every build box -- so a self-check run there passes even when the
#    package does not contain them. On a clean Windows install they are absent
#    and the exe dies with 0xC0000135. Shipping them is what makes
#    "unzip and run, no installation" true (app-local CRT deployment is
#    permitted by the VC++ redistributable license).
# ---------------------------------------------------------------------------
$crtMirror = $null
foreach ($root in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
    if (-not $root) { continue }
    $redist = Join-Path $root "Microsoft Visual Studio"
    if (-not (Test-Path $redist)) { continue }
    # Only the Redist mirror counts. `VC\Tools\...\bin\Hostx86\x64` also holds
    # msvcp140.dll but is a tool directory: it has an incomplete set (no
    # vcruntime140_1.dll), and the leftover file would then be silently
    # "resolved" from System32 on a build box and missing on the user's.
    $hit = Get-ChildItem $redist -Recurse -Filter msvcp140.dll -ErrorAction SilentlyContinue |
           Where-Object { $_.FullName -match '\\VC\\Redist\\MSVC\\' -and $_.FullName -match '\\x64\\' } |
           Sort-Object FullName -Descending | Select-Object -First 1
    if ($hit) { $crtMirror = $hit.DirectoryName; break }
}

if ($crtMirror) {
    foreach ($n in @("msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll")) {
        $src = Join-Path $crtMirror $n
        if (Test-Path $src) {
            Copy-Item $src -Destination (Join-Path $OutputDir $n) -Force
            Write-Host "  [CRT] $n" -ForegroundColor Green
        } else {
            Write-Host "  [CRT][WARN] $n not found in $crtMirror" -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "  [CRT][WARN] VC redistributable mirror not found; the package will require the VC++ Redistributable" -ForegroundColor Yellow
}

# ---------------------------------------------------------------------------
# 4. Self-check: every dependency of everything we ship must resolve either to
#    the output directory or to Windows. A hole here is exactly the class of
#    defect this script used to produce silently, so it is fatal.
# ---------------------------------------------------------------------------
$missing = @()
$roots = @($ExecutablePath) + @(Get-ChildItem $OutputDir -Recurse -Filter *.dll | ForEach-Object { $_.FullName })
$have = @{}
Get-ChildItem $OutputDir -Recurse -Filter *.dll | ForEach-Object { $have[$_.Name.ToLower()] = $true }
foreach ($root in $roots) {
    foreach ($d in (Get-PeDependencies -Path $root)) {
        if ($have.ContainsKey($d.ToLower())) { continue }
        if (Test-SystemDll $d) { continue }
        if ($d -notin $missing) { $missing += $d }
    }
}

if ($missing.Count -gt 0) {
    throw ("Unresolved DLL dependencies: " + ($missing -join ", ") +
           ". Add them to the output directory (check vcpkg bin) before shipping.")
}
Write-Host "  [CHECK] all dependencies resolve (output dir or Windows)" -ForegroundColor Green

# Third pass: deploy the license texts a distribution must carry.
#  * vcpkg ports ship a per-port `copyright` under <installed>/<triplet>/share;
#  * the vendored GBA core is MIT — its full text must accompany the
#    distribution (src/rust/crates/gba-core/LICENSE);
#  * the project's own GPL text (COPYING) travels with the binary.
$licenseDir = Join-Path $OutputDir "licenses"
New-Item -ItemType Directory -Path $licenseDir -Force | Out-Null
$copiedLicenses = 0

$vcpkgShare = Join-Path (Split-Path $vcpkgBin -Parent) "share"
if (Test-Path $vcpkgShare) {
    Get-ChildItem $vcpkgShare -Directory | ForEach-Object {
        $copyright = Join-Path $_.FullName "copyright"
        if (Test-Path $copyright) {
            Copy-Item $copyright -Destination (Join-Path $licenseDir ($_.Name + ".txt")) -Force
            $copiedLicenses++
        }
    }
}

foreach ($pair in @(
    @{ src = Join-Path $PSScriptRoot "..\src\rust\crates\gba-core\LICENSE"; name = "gba-core-LICENSE.txt" },
    @{ src = Join-Path $PSScriptRoot "..\COPYING";                          name = "COPYING.txt" }
)) {
    if (Test-Path $pair.src) {
        Copy-Item $pair.src -Destination (Join-Path $licenseDir $pair.name) -Force
        $copiedLicenses++
    } else {
        Write-Host "  [WARN] license source missing: $($pair.src)" -ForegroundColor Yellow
    }
}

Write-Host "`nDone! Copied $copied DLLs (plus Qt plugins and the VC++ runtime)." -ForegroundColor Green
Write-Host "Deployed $copiedLicenses license files to $licenseDir" -ForegroundColor Green
Write-Host "Output: $OutputDir" -ForegroundColor Cyan
