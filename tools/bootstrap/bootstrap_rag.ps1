<#
.SYNOPSIS
    AI documentation knowledge base and Qdrant RAG vector index bootstrapper.

.DESCRIPTION
    Provisions and updates the local vector database for the Amiga 500 emulator AI RAG
    knowledge system. Indexes Commodore hardware reference manuals, PRMs, and Obsidian
    architecture documentation into the local Qdrant vector database (http://localhost:6333,
    collection: amiga) using tools/rag/bin/amiga_rag.ps1.

    NOTE: Qdrant is optional and only used for AI knowledge retrieval. A bare clone
    compiles and runs the emulator GUI immediately via 'cargo run -p gui'.

.PARAMETER Path
    Custom directory path to index (defaults to Obsidian/Amiga).

.PARAMETER Source
    Knowledge source category: 'amiga' for hardware manuals/specs, or 'obsidian' for
    general architectural guidelines (defaults to 'amiga').

.PARAMETER CheckOnly
    Only probes Qdrant vector database connectivity without performing document indexing.

.EXAMPLE
    .\tools\bootstrap_rag.ps1
    Indexes Obsidian/Amiga documentation into local Qdrant collection ('amiga').

.EXAMPLE
    .\tools\bootstrap_rag.ps1 -CheckOnly
    Probes Qdrant availability on http://localhost:6333.

.EXAMPLE
    .\tools\bootstrap_rag.ps1 -Source obsidian
    Indexes notes under Obsidian/Amiga with the 'obsidian' source category.
#>

[CmdletBinding()]
param(
    [string]$Path,
    [ValidateSet("amiga", "obsidian")]
    [string]$Source = "amiga",
    [switch]$CheckOnly
)

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 AI Knowledge Base & Qdrant RAG Bootstrapper" -ForegroundColor Cyan
    Write-Host "=====================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap_rag.ps1                        : Index Obsidian documentation into Qdrant"
    Write-Host "  .\tools\bootstrap_rag.ps1 -CheckOnly             : Probe local Qdrant database status"
    Write-Host "  .\tools\bootstrap_rag.ps1 -Source obsidian       : Index documentation under 'obsidian' source"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Path <Dir>              : Target directory to index (default: Obsidian/Amiga)"
    Write-Host "  -Source <amiga|obsidian> : Target knowledge source identifier (default: amiga)"
    Write-Host "  -CheckOnly               : Only test Qdrant port reachability"
    Write-Host ""
}

Write-Host ""
Write-Host "Bootstrapping Documentation & AI Knowledge Base..." -ForegroundColor Green
Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray

$AmigaRagScript = Join-Path $RepoRoot "tools\rag\bin\amiga_rag.ps1"
if (-not $Path) {
    $Path = Join-Path $RepoRoot "Obsidian\Amiga"
}

if (-not (Test-Path $AmigaRagScript)) {
    Write-Error "amiga_rag runner not found at: $AmigaRagScript"
    exit 1
}

Write-Host "Probing local Qdrant vector database on http://localhost:6333..." -ForegroundColor DarkGray
$QdrantAvailable = Test-NetConnection -ComputerName 127.0.0.1 -Port 6333 -InformationLevel Quiet -WarningAction SilentlyContinue

if (-not $QdrantAvailable) {
    Write-Host ""
    Write-Warning "Qdrant vector database is not reachable on http://localhost:6333."
    Write-Host "Please install and start Qdrant to use the AI RAG documentation knowledge base."
    Write-Host "Official website & installation guide: https://qdrant.tech" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "NOTE: Qdrant is ONLY needed for RAG vector search. You can run the emulator via 'cargo run -p gui'." -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

Write-Host "[OK] Qdrant vector database is active on http://localhost:6333." -ForegroundColor Green

if ($CheckOnly) {
    exit 0
}

Write-Host "Indexing technical documentation into local collection ('amiga', source: '$Source')..." -ForegroundColor Cyan
& $AmigaRagScript $Path --source $Source

if ($LASTEXITCODE -eq 0) {
    Write-Host "Documentation bootstrap completed successfully." -ForegroundColor Green
} else {
    Write-Warning "Documentation indexing exited with code $LASTEXITCODE."
}
