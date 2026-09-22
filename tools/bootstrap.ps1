<#
.SYNOPSIS
    Repository bootstrap runner for the Amiga 500 emulator.

.DESCRIPTION
    Provisions external test assets and AI knowledge bases:
    -Test     : Verifies and provisions physical silicon SingleStepTests test vectors and regression media.
    -Graphify : Generates and updates AST-level code knowledge graph (graphify-out/) for structural queries (alias: -Graph).
    -Rag      : Indexes Amiga documentation into the local RAG database (aliases: -Doc, -Qdrant).
    -All      : Executes test suites, Graphify AST, and RAG documentation bootstrapping (tests -> Graphify -> RAG).

.EXAMPLE
    .\tools\bootstrap.ps1 -Test
    .\tools\bootstrap.ps1 -Graphify
    .\tools\bootstrap.ps1 -Rag
    .\tools\bootstrap.ps1 -All
#>

[CmdletBinding()]
param(
    [switch]$Test,
    [Alias("Graph")]
    [switch]$Graphify,
    [Alias("Doc", "Qdrant")]
    [switch]$Rag,
    [switch]$Ref,
    [string]$RefItem,
    [Alias("AllMirrors")]
    [switch]$AllSources,
    [switch]$NoExtract,
    [switch]$ExtractOnly,
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
    Write-Host "  .\tools\bootstrap.ps1 -Test     : Provision hardware test vectors (SingleStepTests, vAmiga, AmigaTestKit)"
    Write-Host "  .\tools\bootstrap.ps1 -Graphify : Provision code knowledge graph (Graphify AST extraction)"
    Write-Host "  .\tools\bootstrap.ps1 -Rag      : Index Amiga docs, design notes, and references into the RAG database"
    Write-Host "  .\tools\bootstrap.ps1 -Ref      : Provision external reference materials into temp/ (PDFs, HTML crawls)"
    Write-Host "  .\tools\bootstrap.ps1 -All      : Provision all components (tests -> Graphify AST -> RAG docs)"
    Write-Host ""
}

if ($ExtractOnly) { $Ref = $true }

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
    Write-Host "[$CurrentStep/$TotalSteps] Bootstrapping Amiga RAG Documentation..." -ForegroundColor Green
    Write-Host "--------------------------------------------------------------" -ForegroundColor DarkGray
    $CurrentStep++

    $RagQdrantCommand = Get-Command rag_qdrant -ErrorAction SilentlyContinue
    if (-not $RagQdrantCommand) {
        Write-Warning "rag_qdrant CLI is not found on PATH. Install it before running -Rag."
    } else {
        $RagIndexJson = $env:RAG_INDEX_JSON
        if ([string]::IsNullOrWhiteSpace($RagIndexJson)) {
            Write-Warning "RAG_INDEX_JSON must name the shared rag_qdrant state file before running -Rag."
            return
        }

        $RepositoryDocumentationRoot = Join-Path $RepoRoot "docs"
        $AmigaDocumentationRoot = Join-Path $RepoRoot "Obsidian\Amiga"
        $IndexCommands = @(
            @($RepositoryDocumentationRoot, "--source", "amiga", "--index-json", $RagIndexJson),
            @($AmigaDocumentationRoot, "--source", "amiga", "--index-json", $RagIndexJson)
        )
        $IndexingSucceeded = $true

        foreach ($IndexArguments in $IndexCommands) {
            & $RagQdrantCommand.Path @IndexArguments
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "rag_qdrant indexing exited with code $LASTEXITCODE."
                $IndexingSucceeded = $false
                break
            }
        }

        if ($IndexingSucceeded) {
            Write-Host "Amiga RAG documentation bootstrap completed successfully." -ForegroundColor Green
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
        $RefParams = @{}
        if ($RefItem) {
            $RefParams["Item"] = $RefItem
        } else {
            $RefParams["All"] = $true
        }
        if ($AllSources) {
            $RefParams["AllSources"] = $true
        }
        if ($NoExtract) {
            $RefParams["NoExtract"] = $true
        }
        if ($ExtractOnly) {
            $RefParams["ExtractOnly"] = $true
        }
        & $RefScript @RefParams
    }
}

Write-Host ""
Write-Host "Bootstrap procedure finished." -ForegroundColor Cyan

