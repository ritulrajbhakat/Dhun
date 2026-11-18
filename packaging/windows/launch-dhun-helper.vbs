' DHUN Local Helper Silent Launcher (invoked by dhun:// protocol or Start Menu)
' Starts dhun-local-helper.exe cleanly in the background without flashing a console window.
' If dhun-local-helper.exe is already running on port 47821, the duplicate exits cleanly (code 0).

Option Explicit
Dim shell, fso, scriptDir, exePath
Set shell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
exePath = fso.BuildPath(scriptDir, "dhun-local-helper.exe")

If fso.FileExists(exePath) Then
    shell.Run Chr(34) & exePath & Chr(34), 0, False
End If
