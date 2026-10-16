@echo off
title CamAr Studio - Core Rust Engine
echo ========================================================
echo  Iniciando CamAr Studio Engine [RUST]
echo  Video Processing, Live Effects & Virtual Camera
echo ========================================================
cd /d "%~dp0\..\engine-rs\target\release"
camar-engine.exe
pause
