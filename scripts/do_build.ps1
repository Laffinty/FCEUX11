# FCEUX11 Build Script (v0.2.5)
# Pure PowerShell — no MSYS2 / MinGW / POSIX dependencies
# Lives under scripts/; resolves ProjectRoot via the parent of $PSScriptRoot
# so it works regardless of the current working directory.
param(
    [ValidateSet("Debug", "Release", "RelWithDebInfo")]
    [string]$Config = "Release",

    [string]$BuildDir = "build",
    [switch]$Clean,
    [string]$Target = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($BuildDir)) {
    $BuildDir = Join-Path $ProjectRoot $BuildDir
}

if ($Clean -and (Test-Path $BuildDir)) {
    Write-Host "[CLEAN] Removing $BuildDir" -ForegroundColor Yellow
    Remove-Item -Recurse -Force $BuildDir
}

# Ensure vcpkg toolchain is discoverable
$vcpkgToolchain = $null
if ($env:VCPKG_ROOT) {
    $vcpkgToolchain = Join-Path $env:VCPKG_ROOT "scripts\buildsystems\vcpkg.cmake"
}

# Discover Visual Studio installations once for both Ninja and MSVC.
# Prefer vswhere.exe (works across drive letters and editions), then fall back
# to known install roots for edge cases where vswhere is unavailable.
$vsInstallPaths = @()
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vswhere) {
    try {
        $vsInstallPaths += @(& $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath 2>$null)
    } catch {}
}
$vsInstallPaths += @(
    "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools"
    "C:\Program Files\Microsoft Visual Studio\2022\BuildTools"
    "C:\Program Files\Microsoft Visual Studio\2022\Enterprise"
    "C:\Program Files\Microsoft Visual Studio\2022\Professional"
    "C:\Program Files\Microsoft Visual Studio\2022\Community"
)
$vsInstallPaths = @($vsInstallPaths | Where-Object { $_ -and (Test-Path $_) } | Select-Object -Unique)

# Auto-detect available generator. Visual Studio bundles Ninja outside PATH in
# ordinary PowerShell/Git Bash sessions, so probe those installations as well.
$generator = $null
$ninjaPath = $null
$ninjaCommand = Get-Command ninja -ErrorAction SilentlyContinue
if ($ninjaCommand) {
    $ninjaPath = $ninjaCommand.Source
} else {
    $ninjaPath = $vsInstallPaths |
        ForEach-Object { Join-Path $_ "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
}

$ninjaOk = $false
if ($ninjaPath) {
    try {
        & $ninjaPath --version | Out-Null
        $ninjaOk = ($LASTEXITCODE -eq 0)
    } catch {}
}
if ($ninjaOk) {
    $generator = "Ninja"
    $ninjaDir = Split-Path -Parent $ninjaPath
    $env:PATH = "$ninjaDir;$env:PATH"
    Write-Host "[ENV] Using Ninja from $ninjaPath" -ForegroundColor Gray
}

# Load the MSVC environment when invoked from a non-Developer shell.
if (-not (Get-Command cl -ErrorAction SilentlyContinue)) {
    $vcvars = $vsInstallPaths |
        ForEach-Object { Join-Path $_ "VC\Auxiliary\Build\vcvars64.bat" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
    if ($vcvars) {
        Write-Host "[ENV] Loading VS environment from $vcvars" -ForegroundColor Yellow
        $envVars = (& cmd /c "`"$vcvars`" >nul 2>&1 && set") -split "`r?`n"
        foreach ($line in $envVars) {
            if ($line -match "^([^=]+)=(.*)$") {
                $name = $matches[1]
                $value = $matches[2]
                Set-Item -Path "Env:$name" -Value $value -ErrorAction SilentlyContinue
            }
        }
    }
    if (-not (Get-Command cl -ErrorAction SilentlyContinue)) {
        throw "Failed to locate MSVC. Install Visual Studio Build Tools with the C++ workload."
    }
}

if (-not $generator) {
    if (Get-Command nmake -ErrorAction SilentlyContinue) {
        Write-Host "[WARN] Ninja was not found; falling back to legacy NMake." -ForegroundColor Yellow
        Write-Host "[WARN] Ninja is the only supported local generator and matches CI. Install the Visual Studio CMake component." -ForegroundColor Yellow
        $generator = "NMake Makefiles"
    } else {
        throw "No supported build generator found (Ninja or NMake). Install Ninja (recommended) or ensure nmake is on PATH."
    }
}

$cmakeArgs = @(
    "-S", $ProjectRoot
    "-B", $BuildDir
    "-G", $generator
    "-DCMAKE_BUILD_TYPE=$Config"
    "-DCMAKE_C_COMPILER=cl"
    "-DCMAKE_CXX_COMPILER=cl"
)

# Prefer existing local vcpkg_installed over system vcpkg to avoid re-building packages
$localVcpkg = Join-Path $ProjectRoot "vcpkg_installed\x64-windows"
if (Test-Path $localVcpkg) {
    $cmakeArgs += "-DCMAKE_PREFIX_PATH=$localVcpkg"
    Write-Host "[INFO] Using local vcpkg_installed: $localVcpkg" -ForegroundColor Gray
    # If qttools (LinguistTools) is missing, disable i18n to avoid configure failure
    $linguistDir = Join-Path $localVcpkg "share\Qt6LinguistTools"
    if (-not (Test-Path $linguistDir)) {
        $cmakeArgs += "-DFCEUX11_ENABLE_I18N=OFF"
        Write-Host "[WARN] Qt6LinguistTools not found; disabling i18n support" -ForegroundColor Yellow
    }
    # Disable vcpkg manifest mode to prevent rebuilding packages from source
    $cmakeArgs += "-DVCPKG_MANIFEST_MODE=OFF"
    # Map missing debug configs to release (some vcpkg Qt debug tool wrappers are absent)
    $cmakeArgs += "-DCMAKE_MAP_IMPORTED_CONFIG_DEBUG=RELEASE"
    $cmakeArgs += "-DCMAKE_MAP_IMPORTED_CONFIG_RELWITHDEBINFO=RELEASE"
} elseif ($vcpkgToolchain -and (Test-Path $vcpkgToolchain)) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$vcpkgToolchain"
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

$maxBuildAttempts = 3
$preexistingBuildProcessIds = @(Get-Process -Name cl, nmake, link -ErrorAction SilentlyContinue |
    ForEach-Object { $_.Id })
for ($attempt = 1; $attempt -le $maxBuildAttempts; $attempt++) {
    Write-Host "[BUILD] cmake --build $BuildDir --config $Config (attempt $attempt/$maxBuildAttempts)" -ForegroundColor Cyan
    $buildOutput = @()
    $savedErrorActionPreference = $ErrorActionPreference
    try {
        # Windows PowerShell promotes redirected native stderr to ErrorRecord;
        # keep it capturable without letting the global Stop policy abort here.
        $ErrorActionPreference = "Continue"
        $targetArg = ""
        if ($Target -ne "") { $targetArg = "--target $Target" }
        & cmake --build $BuildDir --config $Config $targetArg 2>&1 |
            ForEach-Object { $_.ToString() } |
            Tee-Object -Variable buildOutput
        $buildExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $savedErrorActionPreference
    }
    if ($buildExitCode -eq 0) { break }

    $buildText = ($buildOutput | Out-String)
    if ($attempt -eq $maxBuildAttempts -or $buildText -notmatch "LNK1104") {
        throw "CMake build failed"
    }

    # LNK1104 is locale-independent, but the surrounding diagnostic text is
    # not. Extract quoted .lib paths instead of matching "cannot open file".
    $lockedLibNames = @([regex]::Matches(
        $buildText,
        '(?im)LNK1104:[^\r\n]*[''"]([^''"\r\n]+\.lib)[''"]') |
        ForEach-Object { $_.Groups[1].Value.Trim() } |
        Select-Object -Unique)
    $lockedLibPaths = @()
    foreach ($lockedLibName in $lockedLibNames) {
        $exactCandidates = @()
        if ([System.IO.Path]::IsPathRooted($lockedLibName)) {
            $exactCandidates += $lockedLibName
        } else {
            # Linker diagnostics may be relative to either the build tree or
            # the cmake caller's working directory (the project root).
            $exactCandidates += Join-Path $BuildDir $lockedLibName
            $exactCandidates += Join-Path $ProjectRoot $lockedLibName
        }
        $matchesForName = @($exactCandidates |
            Where-Object { Test-Path $_ -PathType Leaf } |
            ForEach-Object { Get-Item $_ } |
            Sort-Object FullName -Unique)

        if ($matchesForName.Count -eq 0) {
            $leafName = Split-Path $lockedLibName -Leaf
            $matchesForName = @(Get-ChildItem -Path $BuildDir -Filter $leafName -File -Recurse -ErrorAction SilentlyContinue)
            if ($matchesForName.Count -gt 1) {
                throw "CMake build failed (LNK1104 library path is ambiguous: $lockedLibName)"
            }
        }
        $lockedLibPaths += $matchesForName
    }
    $lockedLibPaths = @($lockedLibPaths | Sort-Object FullName -Unique)
    if ($lockedLibPaths.Count -eq 0) {
        throw "CMake build failed (LNK1104 referenced a missing library, not a locked build output)"
    }

    Write-Host "[RETRY] LNK1104 detected; clearing residual compiler/linker processes." -ForegroundColor Yellow
    $residualProcesses = @(Get-Process -Name cl, nmake, link -ErrorAction SilentlyContinue |
        Where-Object { $preexistingBuildProcessIds -notcontains $_.Id })
    if ($residualProcesses.Count -gt 0) {
        Write-Host "[RETRY] Stopping $($residualProcesses.Count) process(es) started during this build attempt." -ForegroundColor Yellow
        $residualProcesses | Stop-Process -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 500

    $buildRoot = [System.IO.Path]::GetFullPath($BuildDir).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    foreach ($lockedLib in $lockedLibPaths) {
        $lockedLibPath = [System.IO.Path]::GetFullPath($lockedLib.FullName)
        if (-not $lockedLibPath.StartsWith($buildRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Write-Host "[WARN] Refusing to delete library outside build directory: $lockedLibPath" -ForegroundColor Yellow
            continue
        }
        $removed = $false
        for ($deleteAttempt = 1; $deleteAttempt -le 3; $deleteAttempt++) {
            try {
                Remove-Item -Force $lockedLibPath
                Write-Host "[RETRY] Removed locked output: $lockedLibPath" -ForegroundColor Yellow
                $removed = $true
                break
            } catch {
                if ($deleteAttempt -lt 3) {
                    Start-Sleep -Milliseconds (250 * $deleteAttempt)
                }
            }
        }
        if (-not $removed) {
            Write-Host "[WARN] Could not remove locked output before retry: $lockedLibPath" -ForegroundColor Yellow
        }
    }
}

Write-Host "[TEST] ctest --test-dir $BuildDir --output-on-failure" -ForegroundColor Cyan
# Ensure vcpkg DLLs are on PATH for test execution
$env:PATH = "$localVcpkg\bin;$env:PATH"
& ctest --test-dir $BuildDir --output-on-failure
if ($LASTEXITCODE -ne 0) { throw "CTest failed" }

Write-Host "[SUCCESS] Build complete: $BuildDir" -ForegroundColor Green
