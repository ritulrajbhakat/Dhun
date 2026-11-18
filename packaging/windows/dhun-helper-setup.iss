; DHUN Local Helper — Windows One-Click Installer (Inno Setup 6)
; Builds: dist-installer-win\DHUN-Local-Helper-Setup.exe
;
; Transparently installs:
;   1. dhun-local-helper.exe + helper scripts into {localappdata}\DHUN\bin
;   2. Custom URI protocol handler `dhun://` so the DHUN web app can launch the helper in one click
;   3. Optional Windows Firewall rule for Private Wi-Fi LAN only (TCP port 47821)
;   4. Optional Windows sign-in auto-start checkbox (user-controlled)

#define MyAppName "DHUN Local Helper"
#define MyAppVersion "2.1.0"
#define MyAppPublisher "DHUN Party DJ"
#define MyAppURL "https://github.com/ritulrajbhakat/Dhun"
#define MyAppExeName "dhun-local-helper.exe"

[Setup]
AppId={{8D6B7D92-4C3B-4A6A-9D8A-47821DHUN001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={localappdata}\DHUN\bin
DisableDirPage=yes
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\dist-installer-win
OutputBaseFilename=DHUN-Local-Helper-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start DHUN Local Helper automatically when I sign in to Windows"; GroupDescription: "Optional startup:"; Flags: unchecked
Name: "firewall"; Description: "Allow guest smartphones on my Private Wi-Fi network (Windows Firewall TCP 47821)"; GroupDescription: "Local Wi-Fi guest QR access:"

[Files]
Source: "..\..\target\release\dhun-local-helper.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "launch-dhun-helper.vbs"; DestDir: "{app}"; Flags: ignoreversion
Source: "start-dhun-helper.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "stop-dhun-helper.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\docs\LOCAL_HELPER_INSTALL.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Start DHUN Local Helper"; Filename: "wscript.exe"; Parameters: """{app}\launch-dhun-helper.vbs"""; IconFilename: "{app}\{#MyAppExeName}"
Name: "{group}\Stop DHUN Local Helper"; Filename: "powershell.exe"; Parameters: "-WindowStyle Hidden -ExecutionPolicy Bypass -File ""{app}\stop-dhun-helper.ps1"""
Name: "{group}\Uninstall DHUN Local Helper"; Filename: "{uninstallexe}"

[Registry]
; Register custom `dhun://` URI protocol in HKCU so the web app can launch the installed helper cleanly
Root: HKCU; Subkey: "Software\Classes\dhun"; ValueType: string; ValueName: ""; ValueData: "URL:DHUN Local Helper Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\dhun"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\dhun\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKCU; Subkey: "Software\Classes\dhun\shell\open\command"; ValueType: string; ValueName: ""; ValueData: "wscript.exe ""{app}\launch-dhun-helper.vbs"" ""%1"""

; Optional user-controlled Windows sign-in startup entry
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "DHUNLocalHelper"; ValueData: "wscript.exe ""{app}\launch-dhun-helper.vbs"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
; Add Private-network-only firewall rule (prompts UAC cleanly via runas if needed)
Filename: "netsh.exe"; Parameters: "advfirewall firewall add rule name=""DHUN Local Helper (Private LAN)"" dir=in action=allow protocol=TCP localport=47821 profile=private"; Tasks: firewall; Flags: runhidden shellexec waituntilterminated; Verb: "runas"
; Start DHUN Local Helper immediately after installation completes
Filename: "wscript.exe"; Parameters: """{app}\launch-dhun-helper.vbs"""; Description: "Start DHUN Local Helper now (ready for your browser)"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-WindowStyle Hidden -ExecutionPolicy Bypass -File ""{app}\stop-dhun-helper.ps1"""; Flags: runhidden
Filename: "netsh.exe"; Parameters: "advfirewall firewall delete rule name=""DHUN Local Helper (Private LAN)"""; Flags: runhidden shellexec; Verb: "runas"
