' launch.vbs - open Dreamride with no console window. The desktop shortcut points here; it runs launch.ps1 hidden,
' which rebuilds first when any source is newer than the app (see launch.ps1). Same pattern as Consonance's launcher.
Option Explicit
Dim sh, fso, ps1
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
ps1 = fso.BuildPath(fso.GetParentFolderName(WScript.ScriptFullName), "launch.ps1")
' 0 = hidden window, False = do not wait
sh.Run "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File """ & ps1 & """", 0, False
