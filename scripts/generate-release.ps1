<#
.SYNOPSIS
    Script automatizado de publicacion y versionamiento de CamAr Studio.
.DESCRIPTION
    Flujo de trabajo oficial:
    1. Si estamos en una rama feature/*, hace commit y la fusiona en develop.
    2. Fusiona develop en beta.
    3. Calcula el consecutivo de version (SemVer: vX.Y.Z-beta.N o vX.Y.Z).
    4. Genera una copia de respaldo limpia del proyecto en .zip (sin target ni node_modules).
    5. Compila UI y motor en modo Release.
    6. Ensambla y comprime la version Portable en .zip.
    7. Compila el instalador oficial (.exe) usando Inno Setup 7.
    8. Crea el tag Git de la version y hace push de develop, beta y tags a GitHub.
    9. Regresa a la rama develop listo para el siguiente feature.
#>

param (
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$rootDir = "d:\proyectos\CamAr Studio"
Set-Location $rootDir

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "      CamAr Studio - Protocolo Oficial 'Generar Version'  " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Identificar rama actual y fusionar feature en develop
$currentBranch = (git branch --show-current).Trim()
Write-Host "[1/8] Rama actual: $currentBranch" -ForegroundColor Green

# Commit de cambios pendientes si existen
$status = git status --porcelain
if ($status) {
    Write-Host "   -> Guardando cambios pendientes..." -ForegroundColor Yellow
    git add -A
    git commit -m "chore: guardado previo a generacion de version"
}

if ($currentBranch.StartsWith("feature/")) {
    Write-Host "   -> Fusionando feature '$currentBranch' en 'develop'..." -ForegroundColor Green
    git checkout develop
    git merge --no-ff $currentBranch -m "merge: fusionar feature '$currentBranch' en develop"
} elseif ($currentBranch -ne "develop") {
    Write-Host "   -> Cambiando a rama 'develop'..." -ForegroundColor Green
    git checkout develop
}

# 2. Fusionar develop en beta
Write-Host "[2/8] Fusionando 'develop' en rama 'beta'..." -ForegroundColor Green
git checkout beta
git merge --no-ff develop -m "merge: actualizacion de develop en beta"

# 3. Determinar el consecutivo de version
if ([string]::IsNullOrWhiteSpace($Version)) {
    $latestTag = git tag -l "v*" --sort=-v:refname | Select-Object -First 1
    if ($latestTag -match '^v?([0-9]+)\.([0-9]+)\.([0-9]+)(-beta\.?([0-9]+))?') {
        $major = [int]$matches[1]
        $minor = [int]$matches[2]
        $patch = [int]$matches[3]
        if ($matches[5]) {
            $betaNum = [int]$matches[5] + 1
            $Version = "$major.$minor.$patch-beta.$betaNum"
        } else {
            $patch = $patch + 1
            $Version = "$major.$minor.$patch-beta.1"
        }
    } else {
        $Version = "0.1.1-beta.1"
    }
}

Write-Host "[3/8] Consecutivo de version asignado: v$Version" -ForegroundColor Green

# 4. Generar copia de respaldo del proyecto en .zip (código fuente limpio)
$releaseDir = "$rootDir\release"
if (-not (Test-Path $releaseDir)) { New-Item -ItemType Directory -Path $releaseDir | Out-Null }

$projectZip = "$releaseDir\CamAr-Studio-v$Version-Project.zip"
Write-Host "[4/8] Creando copia de seguridad del proyecto en .zip: $projectZip..." -ForegroundColor Green
if (Test-Path $projectZip) { Remove-Item $projectZip -Force }

$tempSourceDir = "$rootDir\temp_source_export"
if (Test-Path $tempSourceDir) { Remove-Item $tempSourceDir -Recurse -Force }
New-Item -ItemType Directory -Path $tempSourceDir | Out-Null

robocopy "$rootDir" "$tempSourceDir" /E /XD target node_modules .git release temp_source_export /XF *.log *.zip *.exe /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
Compress-Archive -Path "$tempSourceDir\*" -DestinationPath $projectZip -CompressionLevel Optimal
Remove-Item $tempSourceDir -Recurse -Force
Write-Host "   -> Copia del proyecto generada: $((Get-Item $projectZip).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ })" -ForegroundColor Gray

# 5. Compilar UI (Vite) y Motor Rust en modo Release
Write-Host "[5/8] Compilando UI (Vite) y Motor Nativo (Cargo Release)..." -ForegroundColor Green
Set-Location "$rootDir\ui"
npm run build
if ($LASTEXITCODE -ne 0) { throw "Fallo en compilacion de UI" }

Set-Location "$rootDir\engine-rs"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Fallo en compilacion de engine-rs" }

Set-Location $rootDir

# 6. Generar carpeta Portable y .zip Portable
$portableFolder = "$releaseDir\CamAr-Studio-v$Version-Portable"
Write-Host "[6/8] Ensamblando version Portable en '$portableFolder'..." -ForegroundColor Green
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

# Ejecutable principal
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

# 7. Crear Instalador (.exe) con Inno Setup
Write-Host "[7/8] Creando Instalador oficial con Inno Setup..." -ForegroundColor Green
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
}

# 8. Tag Git y Push a GitHub
Write-Host "[8/8] Creando Tag Git 'v$Version' y sincronizando con GitHub..." -ForegroundColor Green
git add -A
git commit -m "release: publicacion v$Version"
git tag -a "v$Version" -m "Publicacion version v$Version"

git push origin beta --tags
git push origin develop

# Retornar a develop para continuar el flujo de desarrollo
git checkout develop

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " [EXITO] Version v$Version publicada y empaquetada:" -ForegroundColor Cyan
Write-Host "   * Rama Beta:      Actualizada y sincronizada en GitHub" -ForegroundColor White
Write-Host "   * Tag Git:        v$Version" -ForegroundColor White
Write-Host "   * Proyecto Zip:   $projectZip" -ForegroundColor White
Write-Host "   * Portable Zip:   $portableZip" -ForegroundColor White
Write-Host "   * Instalador Exe: $releaseDir\CamAr-Studio-v$Version-Setup.exe" -ForegroundColor White
Write-Host "   * Rama actual:    develop (lista para nuevos features)" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Cyan
