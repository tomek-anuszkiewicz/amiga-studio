<#
.SYNOPSIS
    Bootstrap the Amiga documentation RAG scopes through the rag_qdrant CLI.

.DESCRIPTION
    This compatibility entry point delegates directly to rag_qdrant. It indexes
    repository docs plus Design and Reference below Obsidian/Amiga. The CLI
    reuses its SHA-256 cache unless -Reindex is supplied.

.PARAMETER CheckOnly
    Report Qdrant status without indexing documents.

.PARAMETER Reindex
    Force both supported scopes to be rebuilt, bypassing the SHA-256 cache.
#>

[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$Reindex
)

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$RagQdrantCommand = Get-Command rag_qdrant -ErrorAction SilentlyContinue

if (-not $RagQdrantCommand) {
    Write-Error "rag_qdrant CLI is not found on PATH. Install it before indexing Amiga documentation."
    exit 1
}

if ($CheckOnly) {
    & $RagQdrantCommand.Path --status
    exit $LASTEXITCODE
}

$AmigaDocumentationRoot = Join-Path $RepoRoot "Obsidian\Amiga"
$IndexCommands = @(
    @($RepoRoot, "--source", "amiga", "--include-dirs", "docs"),
    @($AmigaDocumentationRoot, "--source", "amiga", "--include-dirs", "Design", "Reference")
)

foreach ($IndexArguments in $IndexCommands) {
    if ($Reindex) {
        $IndexArguments += "--reindex"
    }

    & $RagQdrantCommand.Path @IndexArguments
    if ($LASTEXITCODE -ne 0) {
        Write-Error "rag_qdrant indexing failed with exit code $LASTEXITCODE."
        exit $LASTEXITCODE
    }
}

& $RagQdrantCommand.Path --status
exit $LASTEXITCODE
