<#
.SYNOPSIS
    Root repository bootstrap runner forwarding to tools/bootstrap/bootstrap.ps1.

.DESCRIPTION
    Convenience forwarding wrapper for the modular bootstrap suite located under
    tools/bootstrap/. Forwards all switches and arguments to the central coordinator
    at tools/bootstrap/bootstrap.ps1.

    Modular bootstrap suite components:
    - Tier 1: Hardware Verification & External Sources (tools/bootstrap/bootstrap_sources.ps1)
    - Tier 2: AST-Level Code Knowledge Graph (tools/bootstrap/bootstrap_graphify.ps1)
    - Tier 3: AI Knowledge & Qdrant RAG Vector Index (tools/bootstrap/bootstrap_rag.ps1)
    - Tier 4: External Reference Documentation & Scans (tools/bootstrap/bootstrap_documentation.ps1)

.PARAMETER Sources
    Verifies and provisions physical silicon SingleStepTests 68000 test vectors,
    AmigaTestKit ADF, and reference emulators. Alias: -Test.

.PARAMETER Test
    Alias for -Sources. Verifies and provisions hardware test vectors.

.PARAMETER Graphify
    Generates and updates the AST-level code knowledge graph in graphify-out/. Alias: -Graph.

.PARAMETER Rag
    Indexes Commodore hardware reference manuals, PRMs, and Obsidian architecture
    notes into the local Qdrant vector database. Alias: -Qdrant.

.PARAMETER Documentation
    Provisions raw external reference documentation (PDF scans, HTML crawls) into
    Obsidian/Amiga/Reference/temp/. Aliases: -Doc, -Ref.

.PARAMETER Ref
    Alias for -Documentation.

.PARAMETER Doc
    Alias for -Documentation.

.PARAMETER AllSources
    When using -Documentation, downloads from ALL configured mirrors for each document.
    Alias: -AllMirrors.

.PARAMETER All
    Executes all primary bootstrap tiers sequentially (-Sources -> -Graphify -> -Rag).

.EXAMPLE
    .\tools\bootstrap.ps1 -Sources
    Provision external hardware test vectors and sources.

.EXAMPLE
    .\tools\bootstrap.ps1 -Documentation
    Download all external reference documentation.

.EXAMPLE
    .\tools\bootstrap.ps1 -All
    Run all primary bootstrap tiers.
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

$CoordinatorScript = Join-Path $PSScriptRoot "bootstrap\bootstrap.ps1"
if (-not (Test-Path $CoordinatorScript)) {
    Write-Error "Bootstrap coordinator script not found at: $CoordinatorScript"
    exit 1
}

$ForwardParams = @{}
if ($Sources) { $ForwardParams["Sources"] = $true }
if ($Graphify) { $ForwardParams["Graphify"] = $true }
if ($Rag) { $ForwardParams["Rag"] = $true }
if ($Documentation) { $ForwardParams["Documentation"] = $true }
if ($AllSources) { $ForwardParams["AllSources"] = $true }
if ($All) { $ForwardParams["All"] = $true }

& $CoordinatorScript @ForwardParams
