<#
.SYNOPSIS
    Repository bootstrap coordinator for the Amiga 500 emulator.

.DESCRIPTION
    Coordinates the provisioning of external hardware test suites, AI code knowledge
    graphs, local RAG vector documentation, and external reference documentation.

    Delegates execution to modular standalone bootstrap scripts:
    - Tier 1: Hardware Verification & Test Vectors (-Test -> tools/bootstrap_test.ps1)
    - Tier 2: AST-Level Code Knowledge Graph (-Graphify -> tools/bootstrap_graphify.ps1)
    - Tier 3: AI Knowledge & Qdrant RAG Vector Index (-Rag -> tools/bootstrap_rag.ps1)
    - Tier 4: External Reference Documentation & Scans (-Ref -> tools/bootstrap_reference.ps1)

    NOTE: Bootstrapping is NOT required to build, test, or run the emulator.
    A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'.

.PARAMETER Test
    Verifies and provisions physical silicon SingleStepTests 68000 test vectors
    and regression media in ref_src/. Delegates to tools/bootstrap_test.ps1.

.PARAMETER Graphify
    Generates and updates the AST-level code knowledge graph in graphify-out/ for
    structural queries, call hierarchies, and architectural navigation.
    Delegates to tools/bootstrap_graphify.ps1. Alias: -Graph.

.PARAMETER Rag
    Indexes Commodore hardware reference manuals, PRMs, and Obsidian architecture
    notes into the local Qdrant vector database (http://localhost:6333, collection: amiga).
    Delegates to tools/bootstrap_rag.ps1. Aliases: -Doc, -Qdrant.

.PARAMETER Ref
    Provisions raw external reference documentation (PDF scans, microarchitectural guides,
    and multi-page HTML crawls) into Obsidian/Amiga/Reference/temp/. Delegates to
    tools/bootstrap_reference.ps1.

.PARAMETER AllSources
    When using -Ref, downloads from ALL configured mirrors for each document rather than
    stopping after the first successful mirror. Useful for archival redundancy.
    Alias: -AllMirrors.

.PARAMETER All
    Executes all primary bootstrap tiers sequentially (-Test -> -Graphify -> -Rag).

.EXAMPLE
    .\tools\bootstrap.ps1 -Test
    Provision hardware test vectors (SingleStepTests, vAmiga, AmigaTestKit).

.EXAMPLE
    .\tools\bootstrap.ps1 -Graphify
    Update AST code knowledge graph in graphify-out/.

.EXAMPLE
    .\tools\bootstrap.ps1 -Rag
    Index Obsidian technical documentation into the local Qdrant vector database.

.EXAMPLE
    .\tools\bootstrap.ps1 -Ref
    Download all external reference documentation into Obsidian/Amiga/Reference/temp/.

.EXAMPLE
    .\tools\bootstrap.ps1 -All
    Run all primary bootstrap tiers (tests -> Graphify AST -> RAG documentation).
#>

[CmdletBinding()]
param(
    [switch]$Test,
    [Alias("Graph")]
    [switch]$Graphify,
    [Alias("Doc", "Qdrant")]
    [switch]$Rag,
    [switch]$Ref,
    [Alias("AllMirrors")]
    [switch]$AllSources,
    [switch]$All
)

$RepoRoot = Split-Path -Parent $PSScriptRoot

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 Emulator Repository Bootstrapper" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "NOTE: Bootstrapping is NOT required to build or run the emulator!" -ForegroundColor Yellow
    Write-Host "      A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'."
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap.ps1 -Test                      : Provision hardware test vectors (SingleStepTests, vAmiga)"
    Write-Host "  .\tools\bootstrap.ps1 -Graphify                  : Provision code knowledge graph (Graphify AST extraction)"
    Write-Host "  .\tools\bootstrap.ps1 -Rag                       : Provision AI knowledge & RAG (Commodore HRM, PRMs, Obsidian)"
    Write-Host "  .\tools\bootstrap.ps1 -Ref                       : Provision external reference materials (PDFs, HTML crawls)"
    Write-Host "  .\tools\bootstrap.ps1 -All                       : Provision all primary tiers (tests -> Graphify -> RAG)"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Test                    : Verify & unpack SingleStepTests 68000 test vectors"
    Write-Host "  -Graphify                : Update AST code knowledge graph (alias: -Graph)"
    Write-Host "  -Rag                     : Index Obsidian docs into Qdrant (aliases: -Doc, -Qdrant)"
    Write-Host "  -Ref                     : Download external reference materials into temp/"
    Write-Host "  -AllSources              : Download from all mirrors for -Ref (alias: -AllMirrors)"
    Write-Host "  -All                     : Run all primary tiers (-Test, -Graphify, -Rag)"
    Write-Host ""
}

if ($AllSources) { $Ref = $true }

if (-not $Rag -and -not $Test -and -not $Graphify -and -not $Ref -and -not $All) {
    Show-Usage
    exit 0
}

$TotalSteps = 0
if ($Test -or $All) { $TotalSteps++ }
if ($Graphify -or $All) { $TotalSteps++ }
if ($Rag -or $All) { $TotalSteps++ }
if ($Ref) { $TotalSteps++ }
$CurrentStep = 1

# -----------------------------------------------------------------------------
# Tier 1: Verification & Test Suite Bootstrap (-Test / -All)
# -----------------------------------------------------------------------------
if ($Test -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to Test Suite Bootstrapper..." -ForegroundColor Green
    Write-Host "---------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $TestScript = Join-Path $PSScriptRoot "bootstrap_test.ps1"
    if (-not (Test-Path $TestScript)) {
        Write-Error "bootstrap_test.ps1 not found at: $TestScript"
    } else {
        & $TestScript
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
# Tier 4: External Reference Documentation Bootstrap (-Ref)
# -----------------------------------------------------------------------------
if ($Ref) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to External Reference Bootstrapper..." -ForegroundColor Green
    Write-Host "------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $RefScript = Join-Path $PSScriptRoot "bootstrap_reference.ps1"
    if (-not (Test-Path $RefScript)) {
        Write-Error "bootstrap_reference.ps1 not found at: $RefScript"
    } else {
        $RefParams = @{ All = $true }
        if ($AllSources) {
            $RefParams["AllSources"] = $true
        }
        & $RefScript @RefParams
    }
}

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan
