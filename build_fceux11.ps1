$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$vsPath = 'C:\Program Files\Microsoft Visual Studio\18\Community'
$buildDir = 'C:\Users\ikrx2\Desktop\project\FCEUX11\build'

# Call vcvars64.bat to set up MSVC environment
$vcvarsBat = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
$null = & "$vcvarsBat" *> $null 2>&1

# Now build
cmake --build $buildDir --config Release 2>&1 | Select-Object -Last 150