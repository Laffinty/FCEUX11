$ErrorActionPreference = 'Stop'
# Load VS 2022 BuildTools vcvars64
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (Test-Path $vcvars) {
  cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
      [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
    }
  }
} else {
  throw "vcvars64.bat not found at $vcvars"
}

# Verify cl is now on PATH
$cl = (Get-Command cl.exe -ErrorAction SilentlyContinue)
if (-not $cl) { throw 'cl.exe still not on PATH after vcvars load' }
Write-Host "[cl] $($cl.Source)"

# We compile each modified C++ source file with /Zs (syntax check only).
# Including path is inferred from the locations of standard fceu11 sources.
$projectRoot = 'D:\Project\FCEUX11'
$includes = @(
  "/I`"$projectRoot\src`"",
  "/I`"$projectRoot\src\drivers\Qt`"",
  "/I`"$projectRoot\src\utils`"",
  "/I`"$projectRoot\src\boards`""
)
$defines = @(
  '/D_WIN32',
  '/D_WIN64',
  '/D_MSC_VER=1939',
  '/D_USE_MATH_DEFINES',
  '/D_CRT_SECURE_NO_WARNINGS',
  '/D_HAS_EXCEPTIONS=1',
  '/D_SCL_SECURE_NO_WARNINGS',
  '/DUNICODE',
  '/D_UNICODE',
  '/DNDEBUG',
  '/DQT_NO_DEBUG',
  '/DQT_CORE_LIB',
  '/DQT_GUI_LIB',
  '/DQT_WIDGETS_LIB'
)
$cflags = @('/nologo', '/Zs', '/EHsc', '/std:c++20', '/W3') + $includes + $defines

$targets = @(
  "$projectRoot\src\fceu.cpp",
  "$projectRoot\src\ppu_state.cpp",
  "$projectRoot\src\drivers\Qt\ConsoleVideo.cpp",
  "$projectRoot\src\state.cpp",
  "$projectRoot\src\cheat.cpp"
)

$ok = $true
foreach ($src in $targets) {
  Write-Host ''
  Write-Host "=== syntax-only: $src ==="
  $logPath = [System.IO.Path]::GetTempFileName()
  & cl.exe @cflags /c "$src" /Fo"$projectRoot\build\dummy.obj" 1> $logPath 2>&1
  $exitCode = $LASTEXITCODE
  if ($exitCode -eq 0) {
    Write-Host "[OK] $src"
  } else {
    Write-Host "[FAIL exit=$exitCode] $src"
    Get-Content $logPath
    $ok = $false
  }
  Remove-Item $logPath -ErrorAction SilentlyContinue
}

if ($ok) {
  Write-Host ''
  Write-Host '[RESULT] All 5 patched files pass cl.exe /Zs (syntax check).' -ForegroundColor Green
  exit 0
} else {
  Write-Host ''
  Write-Host '[RESULT] Some files failed syntax check.' -ForegroundColor Red
  exit 1
}
