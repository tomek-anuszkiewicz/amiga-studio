<#
.SYNOPSIS
    Bootstrap the Amiga documentation RAG scopes through the rag_qdrant CLI.

.DESCRIPTION
    This compatibility entry point delegates directly to rag_qdrant. It indexes
    repository docs plus Markdown below Obsidian/Amiga, including Design and
    Reference while excluding .obsidian and other technical directories. The
    CLI requires RAG_INDEX_JSON to identify its shared incremental state file.

.PARAMETER CheckOnly
    Report Qdrant status without indexing documents.

#>

[CmdletBinding()]
param(
    [switch]$CheckOnly
)

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$RagQdrantCommand = Get-Command rag_qdrant -ErrorAction SilentlyContinue

if (-not $RagQdrantCommand) {
    Write-Error "rag_qdrant CLI is not found on PATH. Install it before indexing Amiga documentation."
    exit 1
}

$RagIndexJson = $env:RAG_INDEX_JSON
if ([string]::IsNullOrWhiteSpace($RagIndexJson)) {
    Write-Error "RAG_INDEX_JSON must name the shared rag_qdrant state file before indexing Amiga documentation."
    exit 1
}

if ($CheckOnly) {
    & $RagQdrantCommand.Path --status --index-json $RagIndexJson
    exit $LASTEXITCODE
}

$AmigaDocumentationRoot = Join-Path $RepoRoot "Obsidian\Amiga"
$RepositoryDocumentationRoot = Join-Path $RepoRoot "docs"
$IndexCommands = @(
    @($RepositoryDocumentationRoot, "--source", "amiga", "--index-json", $RagIndexJson),
    @($AmigaDocumentationRoot, "--source", "amiga", "--index-json", $RagIndexJson)
)

foreach ($IndexArguments in $IndexCommands) {
    & $RagQdrantCommand.Path @IndexArguments
    if ($LASTEXITCODE -ne 0) {
        Write-Error "rag_qdrant indexing failed with exit code $LASTEXITCODE."
        exit $LASTEXITCODE
    }
}

& $RagQdrantCommand.Path --status --index-json $RagIndexJson
exit $LASTEXITCODE
