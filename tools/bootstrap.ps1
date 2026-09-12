<#
.SYNOPSIS
    Repository bootstrap runner for the Amiga 500 emulator.

.DESCRIPTION
    Provisions external test assets and AI knowledge bases:
    -Test  : Verifies and provisions physical silicon SingleStepTests test vectors and regression media.
    -Graph : Generates and updates AST-level code knowledge graph (graphify-out/) for structural queries.
    -Doc   : Indexes Commodore reference manuals and Obsidian design notes into local RAG vector database.
    -All   : Executes test suites, Graphify AST, and RAG documentation bootstrapping (tests -> Graphify -> RAG).

.EXAMPLE
    .\tools\bootstrap.ps1 -Test
    .\tools\bootstrap.ps1 -Graph
    .\tools\bootstrap.ps1 -Doc
    .\tools\bootstrap.ps1 -All
#>

[CmdletBinding()]
param(
    [switch]$Test,
    [switch]$Graph,
    [switch]$Doc,
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
    Write-Host "  .\tools\bootstrap.ps1 -Test  : Provision hardware test vectors (SingleStepTests, vAmiga, AmigaTestKit)"
    Write-Host "  .\tools\bootstrap.ps1 -Graph : Provision code knowledge graph (Graphify AST extraction)"
    Write-Host "  .\tools\bootstrap.ps1 -Doc   : Provision AI knowledge & RAG (Commodore HRM, PRMs, Obsidian notes)"
    Write-Host "  .\tools\bootstrap.ps1 -All   : Provision all components (tests -> Graphify AST -> RAG docs)"
    Write-Host ""
}

if (-not $Doc -and -not $Test -and -not $Graph -and -not $All) {
    Show-Usage
    exit 0
}

$TotalSteps = 0
if ($Test -or $All) { $TotalSteps++ }
if ($Graph -or $All) { $TotalSteps++ }
if ($Doc -or $All) { $TotalSteps++ }
$CurrentStep = 1

# -----------------------------------------------------------------------------
# Tier 1: Verification & Test Suite Bootstrap (-Test / -All)
# -----------------------------------------------------------------------------
if ($Test -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping Verification & Hardware Test Suites..." -ForegroundColor Green
    Write-Host "---------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $SingleStepBaseDir = Join-Path $RepoRoot "ref_src\SingleStepTests-680x0"
    $SingleStepDir = Join-Path $SingleStepBaseDir "68000\v1"

    # 1. Expand any SingleStep .zip archives if present
    if (Test-Path $SingleStepBaseDir) {
        $ZipFiles = Get-ChildItem -Path $SingleStepBaseDir -Filter "*.zip" -Recurse -ErrorAction SilentlyContinue
        foreach ($Zip in $ZipFiles) {
            Write-Host "Expanding SingleStep archive: $($Zip.Name)..." -ForegroundColor Cyan
            Expand-Archive -Path $Zip.FullName -DestinationPath $Zip.DirectoryName -Force
        }

        # 2. Decompress any .json.gz / .gz files into .json test suites
        $GzFiles = Get-ChildItem -Path $SingleStepBaseDir -Filter "*.gz" -Recurse -ErrorAction SilentlyContinue
        if ($GzFiles.Count -gt 0) {
            $DecompressedCount = 0
            foreach ($Gz in $GzFiles) {
                $TargetJsonName = if ($Gz.Name -like "*.json.gz") {
                    $Gz.Name.Substring(0, $Gz.Name.Length - 3)
                } elseif ($Gz.Name -like "*.gz") {
                    [System.IO.Path]::GetFileNameWithoutExtension($Gz.Name) + ".json"
                } else {
                    $Gz.Name + ".json"
                }

                $TargetJsonPath = Join-Path $Gz.DirectoryName $TargetJsonName
                if (-not (Test-Path $TargetJsonPath) -or (Get-Item $TargetJsonPath).Length -eq 0) {
                    $inStream = [System.IO.File]::OpenRead($Gz.FullName)
                    $outStream = [System.IO.File]::Create($TargetJsonPath)
                    $gzStream = [System.IO.Compression.GZipStream]::new($inStream, [System.IO.Compression.CompressionMode]::Decompress)
                    try {
                        $gzStream.CopyTo($outStream)
                        $DecompressedCount++
                    } finally {
                        $gzStream.Dispose()
                        $outStream.Dispose()
                        $inStream.Dispose()
                    }
                }
            }
            if ($DecompressedCount -gt 0) {
                Write-Host "Decompressed $DecompressedCount SingleStep test suite(s) from .gz archives." -ForegroundColor Green
            }
        }

        # 3. If files exist in 68000/ but not 68000/v1/, migrate them to canonical v1/ directory
        $Parent68kDir = Join-Path $SingleStepBaseDir "68000"
        if (Test-Path $Parent68kDir) {
            if (-not (Test-Path $SingleStepDir)) {
                New-Item -ItemType Directory -Path $SingleStepDir -Force | Out-Null
            }
            $Root68kJsons = Get-ChildItem -Path $Parent68kDir -Filter "*.json" -File -ErrorAction SilentlyContinue
            if ($Root68kJsons.Count -gt 0 -and (Get-ChildItem -Path $SingleStepDir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
                Write-Host "Relocating $($Root68kJsons.Count) test suites into canonical v1 directory ($SingleStepDir)..." -ForegroundColor Cyan
                foreach ($F in $Root68kJsons) {
                    Move-Item -Path $F.FullName -Destination $SingleStepDir -Force
                }
            }
        }
    }

    if (Test-Path $SingleStepDir) {
        $JsonCount = (Get-ChildItem -Path $SingleStepDir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count
        if ($JsonCount -gt 0) {
            Write-Host "[OK] SingleStepTests-680x0 found: $JsonCount test suites in $SingleStepDir." -ForegroundColor Green
        } else {
            Write-Warning "SingleStepTests directory exists ($SingleStepDir) but contains zero .json test suites."
            Write-Host "Check if test archives (.gz / .zip) were properly unpacked." -ForegroundColor Yellow
        }
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

# -----------------------------------------------------------------------------
# Tier 2: Code Knowledge Graph Bootstrap (-Graph / -All)
# -----------------------------------------------------------------------------
if ($Graph -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping Code Knowledge Graph (Graphify AST)..." -ForegroundColor Green
    Write-Host "-----------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $GraphifyCmd = Get-Command graphify -ErrorAction SilentlyContinue
    if (-not $GraphifyCmd) {
        Write-Warning "graphify CLI is not found on PATH."
        Write-Host ""
        Write-Host "What is Graphify?" -ForegroundColor Cyan
        Write-Host "  Graphify generates an AST-level code knowledge graph. It maps modules, functions, structs,"
        Write-Host "  and dependencies across the codebase for semantic code queries and architectural navigation."
        Write-Host ""
        Write-Host "How to install Graphify:" -ForegroundColor Cyan
        Write-Host "  pip install graphify" -ForegroundColor White
        Write-Host "  or see: https://github.com/graphify/graphify" -ForegroundColor White
        Write-Host ""
        Write-Host "NOTE: Graphify is optional and only used for AI code structure navigation." -ForegroundColor DarkGray
        Write-Host ""
    } else {
        Write-Host "Updating code AST knowledge graph in graphify-out/ (graphify update .)..." -ForegroundColor Cyan
        graphify update .
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Code knowledge graph updated successfully in graphify-out/." -ForegroundColor Green
        } else {
            Write-Warning "Graphify update exited with code $LASTEXITCODE."
        }
    }
}

# -----------------------------------------------------------------------------
# Tier 3: Knowledge & Documentation Bootstrap (-Doc / -All)
# -----------------------------------------------------------------------------
if ($Doc -or $All) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping Documentation & AI Knowledge Base..." -ForegroundColor Green
    Write-Host "--------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++
    
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

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan

