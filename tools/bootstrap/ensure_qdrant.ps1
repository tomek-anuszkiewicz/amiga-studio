<#
.SYNOPSIS
    Ensures that the local Docker-managed Qdrant service is running.

.DESCRIPTION
    Starts the named Qdrant container when it already exists, or creates it with
    a persistent named volume when absent. The caller remains responsible for
    invoking rag_qdrant after this function returns successfully.
#>

function Ensure-AmigaQdrantContainer {
    [CmdletBinding()]
    param()

    $dockerCommand = Get-Command docker -ErrorAction SilentlyContinue
    if (-not $dockerCommand) {
        Write-Error "Docker CLI is required to start the local Qdrant service."
        return $false
    }

    $containerName = if ([string]::IsNullOrWhiteSpace($env:AMIGA_QDRANT_CONTAINER)) { "amiga-rag-qdrant" } else { $env:AMIGA_QDRANT_CONTAINER }
    $imageName = if ([string]::IsNullOrWhiteSpace($env:AMIGA_QDRANT_IMAGE)) { "qdrant/qdrant:latest" } else { $env:AMIGA_QDRANT_IMAGE }
    $volumeName = "$containerName-storage"

    $containerState = & $dockerCommand.Path container inspect $containerName --format "{{.State.Running}}" 2>$null
    $containerState = ($containerState | Out-String).Trim()
    if ($containerState -eq "true") {
        Write-Host "[OK] Local Qdrant container is already running: $containerName" -ForegroundColor Green
        return $true
    }
    if ($containerState -eq "false") {
        Write-Host "Starting local Qdrant container: $containerName" -ForegroundColor Cyan
        & $dockerCommand.Path start $containerName | Out-Null
    } else {
        Write-Host "Creating local Qdrant container: $containerName" -ForegroundColor Cyan
        & $dockerCommand.Path run --detach --name $containerName --restart unless-stopped --publish "6333:6333" --publish "6334:6334" --volume "${volumeName}:/qdrant/storage" $imageName | Out-Null
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Docker could not start the local Qdrant container '$containerName'."
        return $false
    }

    Write-Host "[OK] Local Qdrant container is running: $containerName" -ForegroundColor Green
    return $true
}
