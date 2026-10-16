@echo off
title CamAr Studio - Pro Edition
cd /d "%~dp0"

echo [CamAr Studio] Verificando procesos anteriores...
tasklist /fi "imagename eq camar-engine.exe" 2>NUL | find /i /n "camar-engine.exe">NUL
if "%ERRORLEVEL%"=="0" (
    echo [CamAr Studio] Cerrando instancia previa...
    taskkill /f /im camar-engine.exe >nul 2>&1
    ping 127.0.0.1 -n 2 >nul
)

echo [CamAr Studio] Iniciando CamAr Studio con interfaz original de estudio...
start "" "%~dp0engine-rs\target\release\camar-engine.exe"
exit
