# Makes Dreamride.lnk on the desktop: it starts the world with the song library open
# (luminesce-world.exe --library), in world\bin, with the icon the build embeds in the exe.
# Run it again after the folder or the exe moves. It will not replace an existing Dreamride.lnk unless
# given -Force.
#   powershell -ExecutionPolicy Bypass -File world\tools\make_shortcut.ps1 [-Force]
param([switch]$Force)
$ErrorActionPreference = 'Stop'

$bin = (Resolve-Path (Join-Path $PSScriptRoot '..\bin')).Path
$exe = Join-Path $bin 'luminesce-world.exe'
if (-not (Test-Path $exe)) { throw "no ${exe}: run world\build.bat first" }

$desktop = [Environment]::GetFolderPath('Desktop')
$lnk = Join-Path $desktop 'Dreamride.lnk'
if ((Test-Path $lnk) -and -not $Force) { throw "${lnk} already exists; run with -Force to replace it" }

$shell = New-Object -ComObject WScript.Shell
$s = $shell.CreateShortcut($lnk)
$s.TargetPath = $exe
$s.Arguments = '--library'
$s.WorkingDirectory = $bin
$s.IconLocation = "$exe,0"
$s.Description = 'Dreamride: music written as functions, played and drawn'
$s.Save()
Write-Output "shortcut: ${lnk} -> ${exe} --library"
