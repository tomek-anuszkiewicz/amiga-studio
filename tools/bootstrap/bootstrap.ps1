<#
.SYNOPSIS
    Repository bootstrap coordinator for the Amiga 500 emulator.

.DESCRIPTION
    Coordinates the provisioning of external hardware test suites, AI code knowledge
    graphs, local RAG vector documentation, and external reference documentation.

    Delegates execution to modular standalone bootstrap scripts:
    - Tier 1: Hardware Verification & External Sources (-Sources / -Test -> tools/bootstrap/bootstrap_sources.ps1)
    - Tier 2: AST-Level Code Knowledge Graph (-Graphify -> tools/bootstrap/bootstrap_graphify.ps1)
    - Tier 3: AI Knowledge & Qdrant RAG Vector Index (-Rag -> tools/bootstrap/bootstrap_rag.ps1)
    - Tier 4: External Reference Documentation & Scans (-Documentation / -Ref -> tools/bootstrap/bootstrap_documentation.ps1)

    NOTE: Bootstrapping is NOT required to build, test, or run the emulator.
    A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'.

.PARAMETER Sources
    Verifies and provisions physical silicon SingleStepTests 68000 test vectors,
    AmigaTestKit ADF, and reference emulators in ref_src/. Delegates to
    tools/bootstrap_sources.ps1. Alias: -Test.

.PARAMETER Test
    Alias for -Sources. Verifies and provisions physical silicon SingleStepTests
    68000 test vectors and regression media in ref_src/.

.PARAMETER Graphify
    Generates and updates the AST-level code knowledge graph in graphify-out/ for
    structural queries, call hierarchies, and architectural navigation.
    Delegates to tools/bootstrap_graphify.ps1. Alias: -Graph.

.PARAMETER Rag
    Indexes Commodore hardware reference manuals, PRMs, and Obsidian architecture
    notes into the local Qdrant vector database (http://localhost:6333, collection: amiga).
    Delegates to tools/bootstrap_rag.ps1. Alias: -Qdrant.

.PARAMETER Documentation
    Provisions raw external reference documentation (PDF scans, microarchitectural guides,
    and multi-page HTML crawls) into Obsidian/Amiga/Reference/temp/. Delegates to
    tools/bootstrap_documentation.ps1. Aliases: -Doc, -Ref.

.PARAMETER Ref
    Alias for -Documentation. Provisions raw external reference documentation into
    Obsidian/Amiga/Reference/temp/.

.PARAMETER Doc
    Alias for -Documentation. Provisions raw external reference documentation into
    Obsidian/Amiga/Reference/temp/.

.PARAMETER AllSources
    When using -Documentation, downloads from ALL configured mirrors for each document
    rather than stopping after the first successful mirror. Useful for archival redundancy.
    Alias: -AllMirrors.

.PARAMETER All
    Executes all primary bootstrap tiers sequentially (-Sources -> -Graphify -> -Rag).

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Sources
    Provision external hardware test vectors and sources (SingleStepTests, vAmiga).

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Graphify
    Update AST code knowledge graph in graphify-out/.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Rag
    Index Obsidian technical documentation into the local Qdrant vector database.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Documentation
    Download all external reference documentation into Obsidian/Amiga/Reference/temp/.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -All
    Run all primary bootstrap tiers (sources -> Graphify AST -> RAG documentation).
#>

[CmdletBinding()]
param(
    [Alias("Test")]
    [switch]$Sources,
    [Alias("Graph")]
    [switch]$Graphify,
    [Alias("Qdrant")]
    [switch]$Rag,
    [Alias("Ref", "Doc")]
    [switch]$Documentation,
    [Alias("AllMirrors")]
    [switch]$AllSources,
    [switch]$All
)

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 Emulator Repository Bootstrapper" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "NOTE: Bootstrapping is NOT required to build or run the emulator!" -ForegroundColor Yellow
    Write-Host "      A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'."
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Sources         : Provision external test sources & vectors (SingleStepTests, vAmiga)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Graphify        : Provision code knowledge graph (Graphify AST extraction)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Rag             : Provision AI knowledge & RAG (Commodore HRM, PRMs, Obsidian)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Documentation   : Provision external reference materials (PDFs, HTML crawls)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -All             : Provision all primary tiers (sources -> Graphify -> RAG)"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Sources                 : Verify & unpack SingleStepTests 68000 test vectors (alias: -Test)"
    Write-Host "  -Graphify                : Update AST code knowledge graph (alias: -Graph)"
    Write-Host "  -Rag                     : Index Obsidian docs into Qdrant (alias: -Qdrant)"
    Write-Host "  -Documentation           : Download external reference materials into temp/ (aliases: -Doc, -Ref)"
    Write-Host "  -AllSources              : Download from all mirrors for -Documentation (alias: -AllMirrors)"
    Write-Host "  -All                     : Run all primary tiers (-Sources, -Graphify, -Rag)"
    Write-Host ""
}

if ($AllSources) { $Documentation = $true }

if (-not $Rag -and -not $Sources -and -not $Graphify -and -not $Documentation -and -not $All) {
    Show-Usage
    exit 0
}

$TotalSteps = 0
if ($Sources -or $All) { $TotalSteps++ }
if ($Graphify -or $All) { $TotalSteps++ }
if ($Rag -or $All) { $TotalSteps++ }
if ($Documentation) { $TotalSteps++ }
$CurrentStep = 1

# -----------------------------------------------------------------------------
# Tier 1: Verification & External Sources Bootstrap (-Sources / -Test / -All)
# -----------------------------------------------------------------------------
if ($Sources -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to External Sources Bootstrapper..." -ForegroundColor Green
    Write-Host "-------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $SourcesScript = Join-Path $PSScriptRoot "bootstrap_sources.ps1"
    if (-not (Test-Path $SourcesScript)) {
        Write-Error "bootstrap_sources.ps1 not found at: $SourcesScript"
    } else {
        & $SourcesScript
    }
}

# -----------------------------------------------------------------------------
# Tier 2: Code Knowledge Graph Bootstrap (-Graphify / -All)
# -----------------------------------------------------------------------------
if ($Graphify -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to Code Knowledge Graph Bootstrapper..." -ForegroundColor Green
    Write-Host "-----------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $GraphifyScript = Join-Path $PSScriptRoot "bootstrap_graphify.ps1"
    if (-not (Test-Path $GraphifyScript)) {
        Write-Error "bootstrap_graphify.ps1 not found at: $GraphifyScript"
    } else {
        & $GraphifyScript
    }
}

# -----------------------------------------------------------------------------
# Tier 3: Knowledge & Documentation Bootstrap (-Rag / -All)
# -----------------------------------------------------------------------------
if ($Rag -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to AI Documentation Bootstrapper..." -ForegroundColor Green
    Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $RagScript = Join-Path $PSScriptRoot "bootstrap_rag.ps1"
    if (-not (Test-Path $RagScript)) {
        Write-Error "bootstrap_rag.ps1 not found at: $RagScript"
    } else {
        & $RagScript
    }
}

# -----------------------------------------------------------------------------
# Tier 4: External Reference Documentation Bootstrap (-Documentation / -Ref)
# -----------------------------------------------------------------------------
if ($Documentation) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to External Documentation Bootstrapper..." -ForegroundColor Green
    Write-Host "-------------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $DocScript = Join-Path $PSScriptRoot "bootstrap_documentation.ps1"
    if (-not (Test-Path $DocScript)) {
        Write-Error "bootstrap_documentation.ps1 not found at: $DocScript"
    } else {
        $DocParams = @{ All = $true }
        if ($AllSources) {
            $DocParams["AllSources"] = $true
        }
        & $DocScript @DocParams
    }
}

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan
