@echo off
echo ========================================================
echo   Desinstalando CamAr Studio Virtual Camera Driver...
echo ========================================================

regsvr32.exe /u /s "%~dp0CamAr_VCam64.dll"
regsvr32.exe /u /s "%~dp0CamAr_VCam32.dll"

echo [OK] CamAr Studio Virtual Camera desregistrada.
exit /b 0
