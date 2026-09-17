<#
.SYNOPSIS
    External test sources, hardware vectors, and reference emulator bootstrapper.

.DESCRIPTION
    Verifies, decompresses, and provisions physical silicon SingleStepTests 68000
    test vectors, AmigaTestKit diagnostic floppy images, vAmiga C++ reference sources,
    and vAmigaTS chipset regression suites in ref_src/ and tools/.

    Actions performed:
    1. Expands any SingleStepTests .zip archives into ref_src/SingleStepTests-680x0.
    2. Decompresses .gz test suite archives into .json format.
    3. Migrates test suites from 68000/ into the canonical 68000/v1/ directory.
    4. Validates AmigaTestKit ADF presence in tools/AmigaTestKit/.
    5. Validates vAmiga and vAmigaTS reference repositories in ref_src/.
    6. Executes a single-step test runner smoke check (test_nop) unless -NoSmoke is passed.

.PARAMETER NoSmoke
    Skips the SingleStep test runner smoke check ('cargo test -p test_runner --test test_singlestep test_nop').

.PARAMETER SmokeOnly
    Only executes the SingleStep test runner smoke check without archive inspection or decompression.

.EXAMPLE
    .\tools\bootstrap_sources.ps1
    Verifies and provisions all test sources and executes the smoke check.

.EXAMPLE
    .\tools\bootstrap_sources.ps1 -NoSmoke
    Decompresses archives and migrates suites without running cargo test.

.EXAMPLE
    .\tools\bootstrap_sources.ps1 -SmokeOnly
    Quickly verifies the single-step test runner harness.
#>

[CmdletBinding()]
param(
    [switch]$NoSmoke,
    [switch]$SmokeOnly
)

$RepoRoot = Split-Path -Parent $PSScriptRoot

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 External Sources & Test Suites Bootstrapper" -ForegroundColor Cyan
    Write-Host "=====================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap_sources.ps1                    : Verify & provision test sources, then run smoke check"
    Write-Host "  .\tools\bootstrap_sources.ps1 -NoSmoke           : Decompress & verify suites without running cargo test"
    Write-Host "  .\tools\bootstrap_sources.ps1 -SmokeOnly         : Only run single-step smoke check"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -NoSmoke                 : Skip cargo test single-step smoke check"
    Write-Host "  -SmokeOnly               : Only run cargo test smoke check"
    Write-Host ""
}

Write-Host ""
Write-Host "Bootstrapping Verification & Hardware Test Suites..." -ForegroundColor Green
Write-Host "---------------------------------------------------------" -ForegroundColor DarkGray

$SingleStepBaseDir = Join-Path $RepoRoot "ref_src\SingleStepTests-680x0"
$SingleStepDir = Join-Path $SingleStepBaseDir "68000\v1"

if (-not $SmokeOnly) {
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
}

if (-not $NoSmoke) {
    Write-Host ""
    Write-Host "Running quick SingleStep test runner smoke check (test_nop)..." -ForegroundColor Cyan
    cargo test -p test_runner --test test_singlestep test_nop
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Test suite bootstrap verified. Single-step test runner is operational!" -ForegroundColor Green
    } else {
        Write-Warning "Single-step test runner smoke check failed. Run 'cargo test -p test_runner --test test_singlestep' for details."
    }
}
