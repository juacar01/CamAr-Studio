; Inno Setup Script para CamAr Studio (Beta)
#define MyAppName "CamAr Studio"
#define MyAppVersion "0.1.1-beta"
#define MyAppPublisher "CamAr Studio"
#define MyAppURL "https://github.com/CamArStudio"
#define MyAppExeName "CamAr-Studio.exe"
#define SourceDir "d:\proyectos\CamAr Studio\release\CamAr-Studio-v0.1.1-beta-Portable"

[Setup]
AppId={{58B8682F-E22A-4D3B-A87A-CC6B8B1328E0}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=d:\proyectos\CamAr Studio\release
OutputBaseFilename=CamAr-Studio-v0.1.1-beta-Setup
SetupIconFile=d:\proyectos\CamAr Studio\ui-winui\Assets\AppIcon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "installvcam"; Description: "Registrar driver de Camara Virtual (CamAr Studio Virtual Camera para OBS/TikTok/Zoom)"; GroupDescription: "Componentes Adicionales:"; Flags: checkedonce

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
Name: "{group}\Desinstalar {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\app.ico"

[Run]
; Si seleccionó la cámara virtual, registrar el driver
Filename: "{sys}\regsvr32.exe"; Parameters: "/s ""{app}\driver\CamAr_VCam64.dll"" ""/i:UnityCaptureName=CamAr Studio Virtual Camera"""; StatusMsg: "Registrando CamAr Studio Virtual Camera (64-bit)..."; Flags: runhidden; Tasks: installvcam
Filename: "{sys}\regsvr32.exe"; Parameters: "/s ""{app}\driver\CamAr_VCam32.dll"" ""/i:UnityCaptureName=CamAr Studio Virtual Camera"""; StatusMsg: "Registrando CamAr Studio Virtual Camera (32-bit)..."; Flags: runhidden; Tasks: installvcam
; Ejecutar aplicación tras terminar
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Desregistrar driver al desinstalar
Filename: "{sys}\regsvr32.exe"; Parameters: "/u /s ""{app}\driver\CamAr_VCam64.dll"""; Flags: runhidden
Filename: "{sys}\regsvr32.exe"; Parameters: "/u /s ""{app}\driver\CamAr_VCam32.dll"""; Flags: runhidden

