@echo off
echo ========================================================
echo   Instalando CamAr Studio Virtual Camera Driver...
echo ========================================================

:: 1. Desregistrar versiones previas
regsvr32.exe /u /s "%~dp0CamAr_VCam64.dll"
regsvr32.exe /u /s "%~dp0CamAr_VCam32.dll"

:: 2. Registrar oficialmente CamAr Studio Virtual Camera (64-bit y 32-bit)
regsvr32.exe /s "%~dp0CamAr_VCam64.dll" "/i:UnityCaptureName=CamAr Studio Virtual Camera"
if exist "%~dp0CamAr_VCam32.dll" (
    regsvr32.exe /s "%~dp0CamAr_VCam32.dll" "/i:UnityCaptureName=CamAr Studio Virtual Camera"
)

echo [OK] CamAr Studio Virtual Camera registrada exitosamente.
exit /b 0
