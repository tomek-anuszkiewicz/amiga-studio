<#
.SYNOPSIS
    Repository bootstrap coordinator for the Amiga 500 emulator.

.DESCRIPTION
    Coordinates the provisioning of external hardware test suites, AI code knowledge
    graphs, shared RAG access, and external reference documentation.

    Delegates execution to modular standalone bootstrap scripts:
    - Tier 1: Hardware Verification & External Sources (-Sources -> tools/bootstrap/bootstrap_sources.ps1)
    - Tier 2: AST-Level Code Knowledge Graph (-Graphify -> tools/bootstrap/bootstrap_graphify.ps1)
    - Tier 3: External Reference Documentation & Scans (-Documentation -> tools/bootstrap/bootstrap_documentation.ps1)
    - Tier 4: Shared RAG Service and Explicit Indexing (-Rag -> tools/bootstrap/bootstrap_rag.ps1)
    - Tier 5: M68000 Assembler (-Assembler -> tools/bootstrap/bootstrap_assembler.ps1)

    NOTE: Bootstrapping is NOT required to build, test, or run the emulator.
    A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'.

.PARAMETER Sources
    Verifies and provisions software-generated SingleStepTests 68000 test vectors,
    AmigaTestKit ADF, reference emulators, and pinned FPGA references in ref_src/.
    Delegates to tools/bootstrap_sources.ps1 -All.

.PARAMETER Graphify
    Creates or refreshes project code in graphify-out/ and reference code in
    ref_src/graphify-out/. Project document semantics use TA-extract-graph-semantics.
    Delegates to tools/bootstrap_graphify.ps1.

.PARAMETER Rag
    Starts the shared Compose RAG service, indexes Amiga documentation and
    reports status. Delegates to tools/bootstrap_rag.ps1 -Index.

.PARAMETER Documentation
    Provisions raw external reference documentation (PDF scans, microarchitectural guides,
    and multi-page HTML crawls) into Obsidian/Amiga/Reference/<Document_Name>-tmp/. Delegates to
    tools/bootstrap_documentation.ps1.

.PARAMETER Assembler
    Provisions the pinned M68000 assembler used by CPU performance tests.

.PARAMETER Markdown
    When bootstrapping documentation (-Documentation), converts downloaded
    reference scans and crawls into Markdown and assets inside staging workspaces.

.PARAMETER Publish
    With -Markdown, copies finished output into an empty sibling book directory without -tmp.

.PARAMETER All
    Executes all 5 bootstrap tiers in physical causal order:
    Sources, Graphify, Documentation, RAG service and Assembler.
    The RAG tier starts the shared service and reports status. Add -Rag to index.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Sources
    Provision generated SingleStep vectors, vAmiga/vAmigaTS, MiniMig, and Amiga Replacement Project.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Graphify
    Create or refresh project and reference code graphs.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Rag
    Index Amiga documentation through the shared RAG service.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Documentation
    Download all external reference documentation into Obsidian/Amiga/Reference/.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Documentation -Markdown
    Download and convert external reference documentation into publication-grade Markdown.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -All
    Run all bootstrap tiers, including the M68000 assembler.

.EXAMPLE
    .\tools\bootstrap\bootstrap.ps1 -Assembler
    Provision only the pinned M68000 assembler for CPU performance tests.
#>

[CmdletBinding()]
param(
    [switch]$Sources,
    [switch]$Graphify,
    [switch]$Rag,
    [switch]$Documentation,
    [switch]$Assembler,
    [switch]$Markdown,
    [switch]$Publish,
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
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Sources         : Provision external sources (reference emulators, test suites & vectors)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Graphify        : Create/refresh both independent Graphify code graphs"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Documentation   : Provision external reference materials (PDFs, HTML crawls)"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Rag             : Start the shared RAG service and index Amiga documentation"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -Assembler       : Provision the M68000 assembler"
    Write-Host "  .\tools\bootstrap\bootstrap.ps1 -All             : Provision all 5 tiers in causal order (sources -> graph -> docs -> RAG -> assembler)"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Sources                 : Verify & provision external sources (reference emulators, test suites & vectors)"
    Write-Host "  -Graphify                : Create/refresh project and reference code graphs"
    Write-Host "  -Documentation           : Download external reference materials into Reference/"
    Write-Host "  -Markdown                : Convert downloaded materials into publication-grade Markdown"
    Write-Host "  -Publish                 : With -Markdown, copy output into an empty sibling book directory"
    Write-Host "  -Rag                     : Index Amiga documentation through the shared service"
    Write-Host "  -Assembler               : Download and verify the pinned M68000 assembler"
    Write-Host "  -All                     : Provision all tiers and check RAG status; add -Rag to index"
    Write-Host ""
}

if ($Publish -and -not $Markdown) { throw "-Publish requires -Markdown" }
if ($Markdown) { $Documentation = $true }

if (-not $Rag -and -not $Sources -and -not $Graphify -and -not $Documentation -and -not $Assembler -and -not $All) {
    Show-Usage
    exit 0
}

$TotalSteps = 0
if ($Sources -or $All) { $TotalSteps++ }
if ($Graphify -or $All) { $TotalSteps++ }
if ($Documentation -or $All) { $TotalSteps++ }
if ($Rag -or $All) { $TotalSteps++ }
if ($Assembler -or $All) { $TotalSteps++ }
$CurrentStep = 1

# -----------------------------------------------------------------------------
# Tier 1: Verification & External Sources Bootstrap (-Sources / -All)
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
        & $SourcesScript -All
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
        if ($LASTEXITCODE -ne 0) { throw "Graphify bootstrap failed with exit code $LASTEXITCODE" }
    }
}

# -----------------------------------------------------------------------------
# Tier 3: External Reference Documentation Bootstrap (-Documentation / -All)
# -----------------------------------------------------------------------------
if ($Documentation -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to External Documentation Bootstrapper..." -ForegroundColor Green
    Write-Host "-------------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $DocScript = Join-Path $PSScriptRoot "bootstrap_documentation.ps1"
    if (-not (Test-Path $DocScript)) {
        Write-Error "bootstrap_documentation.ps1 not found at: $DocScript"
    } else {
        $DocParams = @{ All = $true }
        if ($Markdown) {
            $DocParams["Markdown"] = $true
        }
        if ($Publish) {
            $DocParams["Publish"] = $true
        }
        & $DocScript @DocParams
    }
}

# -----------------------------------------------------------------------------
# Tier 4: Shared RAG Service (-Rag / -All)
# -----------------------------------------------------------------------------
if ($Rag -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping the shared RAG service..." -ForegroundColor Green
    Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $RagScript = Join-Path $PSScriptRoot "bootstrap_rag.ps1"
    if (-not (Test-Path -LiteralPath $RagScript -PathType Leaf)) {
        throw "bootstrap_rag.ps1 not found at: $RagScript"
    }
    if ($Rag) {
        & $RagScript -Index
    } else {
        & $RagScript -CheckOnly
    }
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

# -----------------------------------------------------------------------------
# Tier 5: M68000 Assembler Bootstrap (-Assembler / -All)
# -----------------------------------------------------------------------------
if ($Assembler -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Delegating to M68000 Assembler Bootstrapper..." -ForegroundColor Green
    Write-Host "------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $AssemblerScript = Join-Path $PSScriptRoot 'bootstrap_assembler.ps1'
    if (-not (Test-Path -LiteralPath $AssemblerScript)) {
        throw "bootstrap_assembler.ps1 not found at: $AssemblerScript"
    }
    & $AssemblerScript
}

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan
