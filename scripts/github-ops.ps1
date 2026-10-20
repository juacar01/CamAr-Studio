<#
.SYNOPSIS
    Utilidades automatizadas para integracion de Pull Requests con GitHub API.
.DESCRIPTION
    Usa el token autenticado de Git Credential Manager en Windows para crear,
    inspeccionar y fusionar Pull Requests manteniendo la trazabilidad 100% oficial en GitHub.
#>

function Get-GitHubToken {
    $raw = ("protocol=https`nhost=github.com" | git credential fill | Select-String "password=").Line
    if ($raw -match 'password=(.*)') {
        return $matches[1].Trim()
    }
    throw "No se pudo recuperar el token de GitHub desde Git Credential Manager."
}

function New-GitHubPullRequest {
    param (
        [Parameter(Mandatory=$true)][string]$Head,
        [Parameter(Mandatory=$true)][string]$Base,
        [Parameter(Mandatory=$true)][string]$Title,
        [Parameter(Mandatory=$false)][string]$Body = ""
    )

    $token = Get-GitHubToken
    $headers = @{
        Authorization = "Bearer $token"
        Accept        = "application/vnd.github+json"
        "User-Agent"  = "CamAr-Studio-Agent"
    }

    $payload = @{
        title = $Title
        head  = $Head
        base  = $Base
        body  = $Body
    } | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/juacar01/CamAr-Studio/pulls" `
            -Method Post `
            -Headers $headers `
            -Body $payload `
            -ContentType "application/json; charset=utf-8"
        
        Write-Host "   [PR Creado] #$($response.number): $($response.title) ($($response.html_url))" -ForegroundColor Cyan
        return $response
    } catch {
        Write-Host "   [Error al crear PR] $($_.Exception.Message)" -ForegroundColor Red
        throw $_
    }
}

function Merge-GitHubPullRequest {
    param (
        [Parameter(Mandatory=$true)][int]$PullNumber,
        [Parameter(Mandatory=$false)][string]$CommitTitle = "",
        [Parameter(Mandatory=$false)][string]$MergeMethod = "merge" # merge, squash, rebase
    )

    $token = Get-GitHubToken
    $headers = @{
        Authorization = "Bearer $token"
        Accept        = "application/vnd.github+json"
        "User-Agent"  = "CamAr-Studio-Agent"
    }

    $payload = @{
        merge_method = $MergeMethod
    }
    if (-not [string]::IsNullOrWhiteSpace($CommitTitle)) {
        $payload["commit_title"] = $CommitTitle
    }
    $jsonPayload = $payload | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/juacar01/CamAr-Studio/pulls/$PullNumber/merge" `
            -Method Put `
            -Headers $headers `
            -Body $jsonPayload `
            -ContentType "application/json; charset=utf-8"
        
        Write-Host "   [PR Fusionado] #$PullNumber exitosamente ($($response.sha))" -ForegroundColor Green
        return $response
    } catch {
        Write-Host "   [Error al fusionar PR #$PullNumber] $($_.Exception.Message)" -ForegroundColor Red
        throw $_
    }
}
