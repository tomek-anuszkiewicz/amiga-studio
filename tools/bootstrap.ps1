<#
.SYNOPSIS
    Repository bootstrap runner for the Amiga 500 emulator.

.DESCRIPTION
    Provisions external knowledge bases and test assets:
    -Doc  : Indexes Commodore reference manuals and Obsidian design notes into local RAG vector database.
    -Test : Verifies and provisions physical silicon SingleStepTests test vectors and regression media.
    -All  : Executes both documentation and test suite bootstrapping.

.EXAMPLE
    .\tools\bootstrap.ps1 -Doc
    .\tools\bootstrap.ps1 -Test
    .\tools\bootstrap.ps1 -All
#>

[CmdletBinding()]
param(
    [switch]$Doc,
    [switch]$Test,
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
    Write-Host "Usage:"
    Write-Host "  .\tools\bootstrap.ps1 -Doc   : Provision AI knowledge & RAG (for asking questions / design work)"
    Write-Host "  .\tools\bootstrap.ps1 -Test  : Provision hardware test vectors (for running single-step tests)"
    Write-Host "  .\tools\bootstrap.ps1 -All   : Provision both documentation and test suites"
    Write-Host ""
}

if (-not $Doc -and -not $Test -and -not $All) {
    Show-Usage
    exit 0
}

# -----------------------------------------------------------------------------
# Tier 1: Knowledge & Documentation Bootstrap (-Doc / -All)
# -----------------------------------------------------------------------------
if ($Doc -or $All) {
    Write-Host ""
    Write-Host "[1/2] Bootstrapping Documentation & AI Knowledge Base..." -ForegroundColor Green
    Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray
    
    $AmigaRagScript = Join-Path $RepoRoot "tools\rag\bin\amiga_rag.ps1"
    $ObsidianPath = Join-Path $RepoRoot "Obsidian\Amiga"

    if (-not (Test-Path $AmigaRagScript)) {
        Write-Error "amiga_rag runner not found at: $AmigaRagScript"
    } else {
        Write-Host "Probing local Qdrant vector database on http://localhost:6333..." -ForegroundColor DarkGray
        $QdrantAvailable = Test-NetConnection -ComputerName 127.0.0.1 -Port 6333 -InformationLevel Quiet -WarningAction SilentlyContinue

        # If not responding, check if Docker is installed and can start an existing container
        if (-not $QdrantAvailable) {
            $DockerCmd = Get-Command docker -ErrorAction SilentlyContinue
            if ($DockerCmd) {
                $ExistingContainer = docker ps -a --filter "name=amiga-qdrant" --format "{{.Names}}"
                if ($ExistingContainer -eq "amiga-qdrant") {
                    Write-Host "Detected stopped 'amiga-qdrant' container. Attempting to start..." -ForegroundColor Cyan
                    docker start amiga-qdrant | Out-Null
                    Start-Sleep -Seconds 2
                    $QdrantAvailable = Test-NetConnection -ComputerName 127.0.0.1 -Port 6333 -InformationLevel Quiet -WarningAction SilentlyContinue
                }
            }
        }

        if (-not $QdrantAvailable) {
            Write-Host ""
            Write-Warning "Qdrant vector database is not reachable on http://localhost:6333."
            Write-Host ""
            Write-Host "What is Qdrant?" -ForegroundColor Cyan
            Write-Host "  Qdrant is an open-source vector search engine. In this repository, it powers the"
            Write-Host "  local AI RAG knowledge base, storing embeddings of Commodore Hardware Reference"
            Write-Host "  Manuals, M68000 PRMs, and design notes for semantic search by AI agents."
            Write-Host ""
            Write-Host "How to start Qdrant:" -ForegroundColor Cyan

            $DockerCmd = Get-Command docker -ErrorAction SilentlyContinue
            if ($DockerCmd) {
                $ExistingContainer = docker ps -a --filter "name=amiga-qdrant" --format "{{.Names}}"
                if ($ExistingContainer -eq "amiga-qdrant") {
                    Write-Host "  Start existing Docker container:" -ForegroundColor Yellow
                    Write-Host "    docker start amiga-qdrant" -ForegroundColor White
                } else {
                    Write-Host "  Option A (Docker - Recommended):" -ForegroundColor Yellow
                    Write-Host "    docker run -d --name amiga-qdrant -p 6333:6333 -p 6334:6334 -v qdrant_storage:/qdrant/storage:z qdrant/qdrant:latest" -ForegroundColor White
                }
            } else {
                Write-Host "  Option A (Docker):" -ForegroundColor Yellow
                Write-Host "    docker run -d --name amiga-qdrant -p 6333:6333 -p 6334:6334 -v qdrant_storage:/qdrant/storage:z qdrant/qdrant:latest" -ForegroundColor White
            }

            Write-Host ""
            Write-Host "  Option B (Standalone Binary - No Docker):" -ForegroundColor Yellow
            Write-Host "    1. Download the prebuilt binary from: https://github.com/qdrant/qdrant/releases" -ForegroundColor White
            Write-Host "    2. Extract and launch: .\qdrant.exe" -ForegroundColor White
            Write-Host ""
            Write-Host "NOTE: Qdrant is ONLY needed for AI agent RAG knowledge retrieval (-Doc)." -ForegroundColor DarkGray
            Write-Host "      You can build and play the emulator without Qdrant: cargo run -p gui" -ForegroundColor DarkGray
            Write-Host ""
        } else {
            Write-Host "[OK] Qdrant vector database is active on http://localhost:6333." -ForegroundColor Green
            Write-Host "Indexing Obsidian technical documentation into local collection ('amiga')..." -ForegroundColor Cyan
            & $AmigaRagScript $ObsidianPath --source amiga
            if ($LASTEXITCODE -eq 0) {
                Write-Host "Documentation bootstrap completed successfully." -ForegroundColor Green
            } else {
                Write-Warning "Documentation indexing exited with code $LASTEXITCODE."
            }
        }
    }
}

# -----------------------------------------------------------------------------
# Tier 2: Verification & Test Suite Bootstrap (-Test / -All)
# -----------------------------------------------------------------------------
if ($Test -or $All) {
    Write-Host ""
    Write-Host "[2/2] Bootstrapping Verification & Hardware Test Suites..." -ForegroundColor Green
    Write-Host "---------------------------------------------------------" -ForegroundColor DarkGray

    $SingleStepDir = Join-Path $RepoRoot "ref_src\SingleStepTests-680x0\68000\v1"
    if (Test-Path $SingleStepDir) {
        $JsonCount = (Get-ChildItem -Path $SingleStepDir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count
        Write-Host "[OK] SingleStepTests-680x0 found: $JsonCount test suites in $SingleStepDir." -ForegroundColor Green
    } else {
        Write-Warning "SingleStepTests directory not found: $SingleStepDir"
        Write-Host "To populate SingleStep hardware vectors, clone or download https://github.com/SingleStepTests/680x0 into ref_src/SingleStepTests-680x0." -ForegroundColor Yellow
    }

    $AdfTestKit = Join-Path $RepoRoot "tools\AmigaTestKit\AmigaTestKit.adf"
    if (Test-Path $AdfTestKit) {
        Write-Host "[OK] AmigaTestKit ADF diagnostic disk found at: tools\AmigaTestKit\AmigaTestKit.adf" -ForegroundColor Green
    } else {
        Write-Warning "AmigaTestKit diagnostic disk not found: $AdfTestKit"
        Write-Host "Download AmigaTestKit ADF from https://github.com/keirf/amiga-stuff into tools/AmigaTestKit/." -ForegroundColor Yellow
    }

    $VAmigaDir = Join-Path $RepoRoot "ref_src\vAmiga-4.5"
    if (-not (Test-Path $VAmigaDir)) {
        $VAmigaDir = Join-Path $RepoRoot "ref_src\vAmiga"
    }
    if (Test-Path $VAmigaDir) {
        Write-Host "[OK] vAmiga C++ reference emulator found at: ref_src\$((Get-Item $VAmigaDir).Name)" -ForegroundColor Green
    } else {
        Write-Warning "vAmiga C++ reference emulator not found in ref_src/."
        Write-Host "To populate the clean-room C++ reference emulator, clone https://github.com/dirkwhoffmann/vAmiga into ref_src/vAmiga-4.5." -ForegroundColor Yellow
    }

    $VAmigaTsDir = Join-Path $RepoRoot "ref_src\vAmigaTS"
    if (Test-Path $VAmigaTsDir) {
        Write-Host "[OK] vAmigaTS test suite found at: ref_src\vAmigaTS" -ForegroundColor Green
    } else {
        Write-Warning "vAmigaTS regression test suite not found: $VAmigaTsDir"
        Write-Host "To populate the custom chipset regression test suite, clone https://github.com/dirkwhoffmann/vAmigaTS into ref_src/vAmigaTS." -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Running quick SingleStep test runner smoke check (test_nop)..." -ForegroundColor Cyan
    cargo test -p test_runner --test test_singlestep test_nop
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Test suite bootstrap verified. Single-step test runner is operational!" -ForegroundColor Green
    } else {
        Write-Warning "Single-step test runner smoke check failed. Run 'cargo test -p test_runner --test test_singlestep' for details."
    }
}

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan
