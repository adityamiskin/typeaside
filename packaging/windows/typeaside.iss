[Setup]
AppId=app.typeaside.dictation
AppName=TypeAside
AppVersion={#Version}
DefaultDirName={localappdata}\Programs\TypeAside
DefaultGroupName=TypeAside
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\..\dist
OutputBaseFilename=TypeAside-setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\typeaside.exe
WizardStyle=modern
CloseApplications=yes
CloseApplicationsFilter=typeaside.exe
RestartApplications=no

[Files]
Source: "..\..\target\release\typeaside.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\TypeAside"; Filename: "{app}\typeaside.exe"
Name: "{group}\Uninstall TypeAside"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\typeaside.exe"; Flags: nowait postinstall skipifnotsilent
