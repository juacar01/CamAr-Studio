@echo off
title CamAr Studio - WinUI 3 Pro
cd /d "%~dp0"

echo [CamAr Studio] Verificando motor nativo multimedia...
tasklist /fi "imagename eq camar-engine.exe" 2>NUL | find /i /n "camar-engine.exe">NUL
if "%ERRORLEVEL%"=="1" (
    echo [CamAr Studio] Iniciando motor nativo en segundo plano a 60 FPS...
    start "" /D "%~dp0engine-rs" "%~dp0engine-rs\target\release\camar-engine.exe" --server-only
    ping 127.0.0.1 -n 2 >nul
)

echo [CamAr Studio] Lanzando interfaz moderna C# WinUI 3...
start "" /D "%~dp0ui-winui\bin\Debug\net11.0-windows10.0.26100.0\win-x64" "%~dp0ui-winui\bin\Debug\net11.0-windows10.0.26100.0\win-x64\CamArStudio.WinUI.exe"
exit
