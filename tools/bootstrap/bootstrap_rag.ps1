<#
.SYNOPSIS
    Start the shared RAG service and optionally index Amiga documentation.

.DESCRIPTION
    Use the rag_qdrant CLI on PATH from the configured shared Compose project.
    Before startup, ask the CLI to confirm that Compose owns the service.

.PARAMETER CheckOnly
    Start the shared RAG service if needed and report its status.

.PARAMETER Index
    Start the shared service, index docs and the Amiga Design and Reference
    scopes, then report status. RAG_INDEX_JSON must name the canonical shared
    state file. This action updates the shared index and incremental state.

.EXAMPLE
    .\tools\bootstrap\bootstrap_rag.ps1 -CheckOnly

.EXAMPLE
    .\tools\bootstrap\bootstrap_rag.ps1 -Index
#>

[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$Index
)

function Show-Usage {
    Write-Host 'Usage: .\tools\bootstrap\bootstrap_rag.ps1 -CheckOnly | -Index'
    Write-Host '  -CheckOnly  Start the shared RAG service and report its status.'
    Write-Host '  -Index      Also update the shared index and state for Amiga documentation.'
    Write-Host 'Both actions require a Compose-mode CLI. Indexing also requires RAG_INDEX_JSON.'
}

if (-not ($CheckOnly -or $Index)) {
    Show-Usage
    exit 0
}
if ($CheckOnly -and $Index) {
    Show-Usage
    Write-Error "Choose one action: -CheckOnly or -Index."
    exit 1
}

$RagQdrantCommand = Get-Command rag_qdrant -CommandType Application -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $RagQdrantCommand) {
    Write-Error "rag_qdrant CLI is not found on PATH. Use the launcher from the shared RAG Compose project."
    exit 1
}

$RagIndexJson = $env:RAG_INDEX_JSON
if ($Index -and [string]::IsNullOrWhiteSpace($RagIndexJson)) {
    Write-Error "RAG_INDEX_JSON must name the canonical shared state file before indexing Amiga documentation."
    exit 1
}

$RagServiceScript = Join-Path $PSScriptRoot "ensure_qdrant.ps1"
if (-not (Test-Path -LiteralPath $RagServiceScript -PathType Leaf)) {
    Write-Error "RAG service helper is missing: $RagServiceScript"
    exit 1
}
$RagServiceMode = & $RagQdrantCommand.Path service mode
if ($LASTEXITCODE -ne 0 -or @($RagServiceMode).Count -ne 1 -or $RagServiceMode -cne "compose") {
    Write-Error "rag_qdrant service mode must report compose. Use the current launcher from the configured shared Compose project."
    exit 1
}

. $RagServiceScript
Start-SharedRagService -RagCommand $RagQdrantCommand.Path
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if ($Index) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $DocumentationScopes = @("docs", "Obsidian\Amiga\Design", "Obsidian\Amiga\Reference")

    foreach ($Scope in $DocumentationScopes) {
        $DocumentationRoot = Join-Path $RepoRoot $Scope
        & $RagQdrantCommand.Path $DocumentationRoot --repo-root $RepoRoot --source amiga --index-json $RagIndexJson
        if ($LASTEXITCODE -ne 0) {
            Write-Error "rag_qdrant indexing failed with exit code $LASTEXITCODE."
            exit $LASTEXITCODE
        }
    }
}

& $RagQdrantCommand.Path --status
exit $LASTEXITCODE
