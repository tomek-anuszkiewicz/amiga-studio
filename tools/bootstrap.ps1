<#
.SYNOPSIS
    Repository bootstrap runner for the Amiga 500 emulator.

.DESCRIPTION
    Automates the provisioning of external hardware test suites, AI code knowledge
    graphs, local RAG vector documentation, and external reference documentation.

    Bootstrapping is organized across modular tiers:
    - Tier 1: Hardware Verification & Test Vectors (-Test)
    - Tier 2: AST-Level Code Knowledge Graph (-Graphify)
    - Tier 3: AI Knowledge & Qdrant RAG Vector Index (-Rag)
    - Tier 4: External Reference Documentation & Scans (-Ref)

    NOTE: Bootstrapping is NOT required to build, test, or run the emulator.
    A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'.

.PARAMETER Test
    Verifies and provisions physical silicon SingleStepTests 68000 test vectors
    and regression media in ref_src/. Decompresses .gz and .zip suites and migrates
    test archives into canonical v1 directories.

.PARAMETER Graphify
    Generates and updates the AST-level code knowledge graph in graphify-out/ for
    structural queries, call hierarchies, and architectural navigation.
    Alias: -Graph.

.PARAMETER Rag
    Indexes Commodore hardware reference manuals, PRMs, and Obsidian architecture
    notes into the local Qdrant vector database (http://localhost:6333, collection: amiga).
    Aliases: -Doc, -Qdrant.

.PARAMETER Ref
    Provisions raw external reference documentation (PDF scans, microarchitectural guides,
    and multi-page HTML crawls) into Obsidian/Amiga/Reference/temp/. Delegates execution
    to tools/bootstrap_reference.ps1.

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

    $VAmigaDir = Join-Path $RepoRoot "ref_src\vAmiga"
    if (-not (Test-Path $VAmigaDir)) {
        $ExistingVAmiga = Get-ChildItem -Path (Join-Path $RepoRoot "ref_src") -Directory -Filter "vAmiga*" -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne "vAmigaTS" } | Select-Object -First 1
        if ($ExistingVAmiga) {
            $VAmigaDir = $ExistingVAmiga.FullName
        }
    }
    if (Test-Path $VAmigaDir) {
        Write-Host "[OK] vAmiga C++ reference emulator found at: ref_src\$((Get-Item $VAmigaDir).Name)" -ForegroundColor Green
    } else {
        Write-Warning "vAmiga C++ reference emulator not found in ref_src/."
        Write-Host "To populate the clean-room C++ reference emulator, clone https://github.com/dirkwhoffmann/vAmiga into ref_src/vAmiga." -ForegroundColor Yellow
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
# Tier 2: Code Knowledge Graph Bootstrap (-Graphify / -All)
# -----------------------------------------------------------------------------
if ($Graphify -or $All) {
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
        Push-Location $RepoRoot
        try {
            graphify update .
        } finally {
            Pop-Location
        }
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Code knowledge graph updated successfully in graphify-out/." -ForegroundColor Green
        } else {
            Write-Warning "Graphify update exited with code $LASTEXITCODE."
        }
    }
}

# -----------------------------------------------------------------------------
# Tier 3: Knowledge & Documentation Bootstrap (-Rag / -All)
# -----------------------------------------------------------------------------
if ($Rag -or $All) {
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

        if (-not $QdrantAvailable) {
            Write-Host ""
            Write-Warning "Qdrant vector database is not reachable on http://localhost:6333."
            Write-Host "Please install and start Qdrant to use the AI RAG documentation knowledge base (-Rag)."
            Write-Host "Official website & installation guide: https://qdrant.tech" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "NOTE: Qdrant is ONLY needed for -Rag. You can run the emulator via 'cargo run -p gui'." -ForegroundColor DarkGray
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
# Tier 4: External Reference Documentation Bootstrap (-Ref)
# -----------------------------------------------------------------------------
if ($Ref) {
    Write-Host ""
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping External Reference Documentation..." -ForegroundColor Green
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

