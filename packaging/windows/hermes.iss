[Setup]
AppId=app.hermes.dictation
AppName=Hermes
AppVersion={#Version}
DefaultDirName={localappdata}\Programs\Hermes
DefaultGroupName=Hermes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\..\dist
OutputBaseFilename=Hermes-setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\hermes.exe
WizardStyle=modern

[Files]
Source: "..\..\target\release\hermes.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Hermes"; Filename: "{app}\hermes.exe"
Name: "{group}\Uninstall Hermes"; Filename: "{uninstallexe}"
