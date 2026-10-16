@echo off
title CamAr Studio - Portable Edition
cd /d "%~dp0"

echo ========================================================
echo       CamAr Studio - Edicion Portable (v0.1.0 Beta)
echo ========================================================
echo.

:: Cerrar instancias previas si existen
tasklist /fi "imagename eq CamAr-Studio.exe" 2>NUL | find /i /n "CamAr-Studio.exe">NUL
if "%ERRORLEVEL%"=="0" (
    echo [Info] Cerrando sesion previa de CamAr-Studio.exe...
    taskkill /f /im CamAr-Studio.exe >nul 2>&1
    ping 127.0.0.1 -n 2 >nul
)

tasklist /fi "imagename eq camar-engine.exe" 2>NUL | find /i /n "camar-engine.exe">NUL
if "%ERRORLEVEL%"=="0" (
    echo [Info] Cerrando sesion previa de camar-engine.exe...
    taskkill /f /im camar-engine.exe >nul 2>&1
    ping 127.0.0.1 -n 2 >nul
)

echo [Info] Iniciando CamAr Studio...
start "" "%~dp0CamAr-Studio.exe"
exit
