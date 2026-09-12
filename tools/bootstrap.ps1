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

    if (Test-Path $AmigaRagScript) {
        Write-Host "Indexing Obsidian technical documentation into local Qdrant collection ('amiga')..." -ForegroundColor Cyan
        & $AmigaRagScript $ObsidianPath --source amiga
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Documentation bootstrap completed successfully." -ForegroundColor Green
        } else {
            Write-Warning "Documentation indexing exited with code $LASTEXITCODE. Ensure Qdrant is running on http://localhost:6333."
        }
    } else {
        Write-Error "amiga_rag runner not found at: $AmigaRagScript"
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
    }

    $VAmigaTsDir = Join-Path $RepoRoot "ref_src\vAmigaTS"
    if (Test-Path $VAmigaTsDir) {
        Write-Host "[OK] vAmigaTS test suite found at: ref_src\vAmigaTS" -ForegroundColor Green
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
