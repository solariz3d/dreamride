# launch.ps1 - start Dreamride's world, rebuilding first when any source is newer than the built app.
# The desktop shortcut runs this, so the app you open is always the code on disk.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File world\launch.ps1 [args for the app]
#
# What counts as a source: the engine (src/**, Cargo.toml, Cargo.lock) and the world (world/src/**, world/shaders/**).
# A rebuild runs build.bat with CARGO_TARGET_DIR pointed at this repo's own target folder, because build.bat links
# ..\target\release\luminesce.dll.lib and a machine-wide CARGO_TARGET_DIR (set for another project) would put the
# engine elsewhere. If the build fails, the app is NOT started on the old binary: a message names the log instead.
$ErrorActionPreference = 'Stop'
$world = $PSScriptRoot
$repo = Split-Path $world -Parent
$exe = Join-Path $world 'bin\luminesce-world.exe'
$dll = Join-Path $world 'bin\luminesce.dll'
$log = Join-Path $world 'build\launch-build.log'

function Newest([string[]]$paths) {
    $files = foreach ($p in $paths) { if (Test-Path $p) { Get-ChildItem $p -Recurse -File } }
    ($files | Measure-Object -Property LastWriteTimeUtc -Maximum).Maximum
}

$sources = Newest @("$repo\src", "$repo\Cargo.toml", "$repo\Cargo.lock", "$world\src", "$world\shaders")
$built = if ((Test-Path $exe) -and (Test-Path $dll)) {
    @((Get-Item $exe).LastWriteTimeUtc, (Get-Item $dll).LastWriteTimeUtc) | Sort-Object | Select-Object -First 1
} else { [datetime]::MinValue }

if ($sources -gt $built) {
    Add-Type -AssemblyName System.Windows.Forms
    if (Get-Process luminesce-world -ErrorAction SilentlyContinue) {
        [System.Windows.Forms.MessageBox]::Show("Dreamride's code changed and needs a rebuild, but it is still running.`nClose it, then open it again.", 'Dreamride') | Out-Null
        exit 1
    }
    New-Item -ItemType Directory -Force (Split-Path $log) | Out-Null
    $env:CARGO_TARGET_DIR = Join-Path $repo 'target'
    # Its own process, judged by its exit code alone: cargo writes progress to stderr, which Windows PowerShell would
    # otherwise raise as an error under ErrorActionPreference = Stop. Three traps, each hit or measured:
    #  - Start-Process -Wait also waits for the build's children, and MSVC leaves its PDB server (mspdbsrv) running
    #    after compiling, so it never returns (hit here, 2026-11-04);
    #  - Start-Process -PassThru's .ExitCode comes back $null (0 of 5 correct, measured in Consonance's own
    #    consonance/launch.ps1), and $null -ne 0 would refuse every good build;
    #  - [System.Diagnostics.Process]::Start with WaitForExit() gives the real code (5 of 5 there).
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'cmd.exe'
    $psi.Arguments = "/c `"`"$world\build.bat`" > `"$log`" 2>&1`""
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.EnvironmentVariables['CARGO_TARGET_DIR'] = $env:CARGO_TARGET_DIR
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.WaitForExit()
    if ($p.ExitCode -ne 0) {
        [System.Windows.Forms.MessageBox]::Show("Dreamride did not rebuild, so it was not started on the old version.`nThe build log is:`n$log", 'Dreamride') | Out-Null
        exit 1
    }
}
if ($args.Count) { Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) -ArgumentList $args }
else { Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) }
