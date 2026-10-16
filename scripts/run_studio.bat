@echo off
title CamAr Studio [Standalone Desktop App]
echo ========================================================
echo        Iniciando CamAr Studio (Ejecutable Nativo)
echo ========================================================
cd /d "%~dp0\..\engine-rs\target\release"
start "" "camar-engine.exe"
exit
