<#
.SYNOPSIS
    Script automatizado de generacion de version de CamAr Studio.
.DESCRIPTION
    1. Calcula el consecutivo de version o toma el especificado.
    2. Crea una nueva rama Git con el consecutivo de version.
    3. Genera una copia completa del proyecto en .zip (excluyendo temporales pesados).
    4. Compila UI y motor en modo Release.
    5. Empaqueta la version Portable y genera el archivo .zip portable.
    6. Compila el instalador oficial (.exe) usando Inno Setup.
#>

param (
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$rootDir = "d:\proyectos\CamAr Studio"
Set-Location $rootDir

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "      CamAr Studio - Protocolo de Generacion de Version   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Determinar el consecutivo de version
if ([string]::IsNullOrWhiteSpace($Version)) {
    # Buscar ultima version en git tags o installer.iss
    $issContent = Get-Content "$rootDir\installer.iss" -Raw
    if ($issContent -match '#define MyAppVersion "([0-9]+)\.([0-9]+)\.([0-9]+)(-[a-zA-Z0-9]+)?"') {
        $major = [int]$matches[1]
        $minor = [int]$matches[2]
        $patch = [int]$matches[3] + 1
        $suffix = if ($matches[4]) { $matches[4] } else { "-beta" }
        $Version = "$major.$minor.$patch$suffix"
    } else {
        $Version = "0.1.1-beta"
    }
}

Write-Host "[1/6] Version a generar: $Version" -ForegroundColor Green

# 2. Crear nueva rama Git con el consecutivo de version
$branchName = "v$Version"
Write-Host "[2/6] Creando y cambiando a la rama Git '$branchName'..." -ForegroundColor Green
try {
    # Commit cambios pendientes si existen
    $status = git status --porcelain
    if ($status) {
        git add -A
        git commit -m "chore: preparación para release $Version"
    }
    git checkout -b $branchName
} catch {
    Write-Host "[Aviso] Rama '$branchName' ya existia o se cambio directamente: $_" -ForegroundColor Yellow
}

# 3. Generar copia del proyecto en .zip (excluyendo carpetas gigantes temporales)
$releaseDir = "$rootDir\release"
if (-not (Test-Path $releaseDir)) { New-Item -ItemType Directory -Path $releaseDir | Out-Null }

$projectZip = "$releaseDir\CamAr-Studio-v$Version-Project.zip"
Write-Host "[3/6] Creando copia de seguridad del proyecto en .zip: $projectZip..." -ForegroundColor Green
if (Test-Path $projectZip) { Remove-Item $projectZip -Force }

$tempSourceDir = "$rootDir\temp_source_export"
if (Test-Path $tempSourceDir) { Remove-Item $tempSourceDir -Recurse -Force }
New-Item -ItemType Directory -Path $tempSourceDir | Out-Null

# Copiar archivos esenciales excluyendo build caches
robocopy "$rootDir" "$tempSourceDir" /E /XD target node_modules .git release temp_source_export /XF *.log *.zip *.exe /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
Compress-Archive -Path "$tempSourceDir\*" -DestinationPath $projectZip -CompressionLevel Optimal
Remove-Item $tempSourceDir -Recurse -Force
Write-Host "   -> Copia del proyecto generada: $((Get-Item $projectZip).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ })" -ForegroundColor Gray

# 4. Compilar UI y Motor Rust en Release
Write-Host "[4/6] Compilando UI (Vite) y Motor Nativo (Cargo Release)..." -ForegroundColor Green
Set-Location "$rootDir\ui"
npm run build
if ($LASTEXITCODE -ne 0) { throw "Fallo en compilacion de UI" }

Set-Location "$rootDir\engine-rs"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Fallo en compilacion de engine-rs" }

Set-Location $rootDir

# 5. Generar carpeta Portable y .zip Portable
$portableFolder = "$releaseDir\CamAr-Studio-v$Version-Portable"
Write-Host "[5/6] Ensamblando version Portable en '$portableFolder'..." -ForegroundColor Green
if (Test-Path $portableFolder) { Remove-Item $portableFolder -Recurse -Force }
New-Item -ItemType Directory -Path $portableFolder | Out-Null

$portableFiles = @(
    "$rootDir\engine-rs\target\release\camar-engine.exe",
    "$rootDir\engine-rs\target\release\camar_tracker.dll",
    "$rootDir\engine-rs\target\release\DirectML.dll",
    "$rootDir\engine-rs\target\release\onnxruntime.dll",
    "$rootDir\engine-rs\target\release\onnxruntime_providers_shared.dll"
)
foreach ($f in $portableFiles) {
    if (Test-Path $f) { Copy-Item $f -Destination $portableFolder -Force }
}

# Copiar ejecutable principal con nombre de marca
Copy-Item "$rootDir\engine-rs\target\release\camar-engine.exe" -Destination "$portableFolder\CamAr-Studio.exe" -Force

# Carpetas de datos y recursos
$foldersToCopy = @(
    @{ Src = "$rootDir\models"; Dst = "$portableFolder\models" },
    @{ Src = "$rootDir\driver"; Dst = "$portableFolder\driver" },
    @{ Src = "$rootDir\engine-rs\assets\sprites"; Dst = "$portableFolder\sprites" },
    @{ Src = "$rootDir\presets"; Dst = "$portableFolder\presets" }
)
foreach ($item in $foldersToCopy) {
    if (Test-Path $item.Src) {
        Copy-Item -Path $item.Src -Destination $item.Dst -Recurse -Force
    }
}

# Configuraciones activas y metadata
if (Test-Path "$rootDir\engine-rs\target\release\camar_hardware.json") {
    Copy-Item "$rootDir\engine-rs\target\release\camar_hardware.json" -Destination $portableFolder -Force
}
if (Test-Path "$rootDir\engine-rs\target\release\camar_beauty.json") {
    Copy-Item "$rootDir\engine-rs\target\release\camar_beauty.json" -Destination $portableFolder -Force
}
if (Test-Path "$rootDir\ui-winui\Assets\AppIcon.ico") {
    Copy-Item "$rootDir\ui-winui\Assets\AppIcon.ico" -Destination "$portableFolder\app.ico" -Force
}

# Batch launcher portable
$batContent = @"
@echo off
title CamAr Studio - Portable Edition (v$Version)
cd /d "%~dp0"
taskkill /f /im CamAr-Studio.exe >nul 2>&1
taskkill /f /im camar-engine.exe >nul 2>&1
start "" "%~dp0CamAr-Studio.exe"
exit
"@
$batContent | Set-Content -Path "$portableFolder\Iniciar_CamAr_Studio.bat" -Encoding ASCII

# Comprimir Portable a .zip
$portableZip = "$releaseDir\CamAr-Studio-v$Version-Portable.zip"
if (Test-Path $portableZip) { Remove-Item $portableZip -Force }
Compress-Archive -Path "$portableFolder\*" -DestinationPath $portableZip -CompressionLevel Optimal
Write-Host "   -> Version Portable comprimida: $((Get-Item $portableZip).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ })" -ForegroundColor Gray

# 6. Crear Instalador (.exe) con Inno Setup
Write-Host "[6/6] Creando Instalador oficial con Inno Setup..." -ForegroundColor Green

# Actualizar version y rutas en installer.iss
$issFile = "$rootDir\installer.iss"
$issContent = Get-Content $issFile -Raw
$issContent = $issContent -replace '#define MyAppVersion ".*?"', "#define MyAppVersion `"$Version`""
$issContent = $issContent -replace '#define SourceDir ".*?"', "#define SourceDir `"$portableFolder`""
$issContent = $issContent -replace 'OutputBaseFilename=.*', "OutputBaseFilename=CamAr-Studio-v$Version-Setup"
$issContent | Set-Content -Path $issFile -Encoding UTF8

$isccPath = "C:\Program Files\Inno Setup 7\ISCC.exe"
if (-not (Test-Path $isccPath)) {
    $isccPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
}

if (Test-Path $isccPath) {
    & $isccPath $issFile
    $setupExe = "$releaseDir\CamAr-Studio-v$Version-Setup.exe"
    if (Test-Path $setupExe) {
        Write-Host "   -> Instalador generado con exito: $((Get-Item $setupExe).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ })" -ForegroundColor Green
    }
} else {
    Write-Host "[Aviso] No se encontro ISCC.exe en las rutas estandar. El instalador debe compilarse manualmente con Inno Setup." -ForegroundColor Yellow
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " [EXITO] Version $Version generada completamente:" -ForegroundColor Cyan
Write-Host "   1. Rama Git:      $branchName" -ForegroundColor White
Write-Host "   2. Proyecto Zip:  $projectZip" -ForegroundColor White
Write-Host "   3. Portable Zip:  $portableZip" -ForegroundColor White
Write-Host "   4. Instalador:    $releaseDir\CamAr-Studio-v$Version-Setup.exe" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Cyan
