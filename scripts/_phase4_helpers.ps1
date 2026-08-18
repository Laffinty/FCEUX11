param([string[]]$Cmd)
$vsInstall = (& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null)
$vcvars = Join-Path $vsInstall "VC\Auxiliary\Build\vcvars64.bat"
$cmdLine = "`"$vcvars`" >nul && " + ($Cmd -join ' ')
cmd.exe /c $cmdLine