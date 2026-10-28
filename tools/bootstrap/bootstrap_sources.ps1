<#
.SYNOPSIS
    External test sources, hardware vectors, and reference emulator bootstrapper.

.DESCRIPTION
    Downloads, provisions, decompresses, and validates external reference sources:
    1. Tom Harte SingleStepTests 68000 software-generated test vectors (ref_src/SingleStepTests-680x0).
    2. Keir Fraser Amiga Test Kit diagnostic floppy disk ADF (tools/AmigaTestKit/AmigaTestKit.adf).
    3. Dirk W. Hoffmann vAmiga C++ reference emulator sources (ref_src/vAmiga).
    4. Dirk W. Hoffmann vAmigaTS chipset regression test suites (ref_src/vAmigaTS).
    5. MiniMig FPGA Amiga implementation (ref_src/minimig).
    6. Amiga Replacement Project FPGA chip reconstructions (ref_src/amiga_replacement_project).

    Decompresses .gz and .zip archives, relocates files to canonical directories, and
    validates dataset completeness.

.PARAMETER Force
    Forces re-download and re-extraction even if target datasets are already provisioned.

.PARAMETER List
    Displays catalog of reference test sources, pinned versions, URLs, and local status.

.PARAMETER SingleStep
    Provisions only Tom Harte SingleStepTests 68000 reference vectors.

.PARAMETER AmigaTestKit
    Provisions only Amiga Test Kit diagnostic floppy disk (ADF).

.PARAMETER VAmiga
    Provisions only vAmiga C++ reference emulator sources.

.PARAMETER VAmigaTS
    Provisions only vAmigaTS chipset regression test suites.

.PARAMETER MiniMig
    Provisions only the pinned MiniMig FPGA source archive.

.PARAMETER AmigaReplacementProject
    Provisions only the pinned Amiga Replacement Project source archive.

.PARAMETER All
    Provisions all external test and research sources.

.PARAMETER Help
    Displays usage instructions and parameter descriptions.

.EXAMPLE
    .\tools\bootstrap\bootstrap_sources.ps1
    Displays available source selections without downloading.

.EXAMPLE
    .\tools\bootstrap\bootstrap_sources.ps1 -MiniMig -AmigaReplacementProject
    Provisions both pinned FPGA reference repositories.

.EXAMPLE
    .\tools\bootstrap\bootstrap_sources.ps1 -List
    Displays catalog and current local presence of test sources.

.EXAMPLE
    .\tools\bootstrap\bootstrap_sources.ps1 -All -Force
    Forces clean re-download and re-provisioning of all sources.

.EXAMPLE
    .\tools\bootstrap\bootstrap_sources.ps1 -Help
    Displays available parameters, flags, and usage examples.
#>

[CmdletBinding()]
param(
    [switch]$Force,
    [switch]$List,
    [switch]$SingleStep,
    [switch]$AmigaTestKit,
    [switch]$VAmiga,
    [switch]$VAmigaTS,
    [switch]$MiniMig,
    [switch]$AmigaReplacementProject,
    [switch]$All,
    [switch]$Help
)

[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12 -bor [System.Net.SecurityProtocolType]::Tls13

$RepoRoot = if ($PSScriptRoot) {
    Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
} else {
    (Get-Location).Path
}
if (-not (Test-Path (Join-Path $RepoRoot "Cargo.toml"))) {
    $RepoRoot = (Get-Location).Path
}

# -----------------------------------------------------------------------------
# Upstream Sources Catalog
# -----------------------------------------------------------------------------
$SourcesCatalog = @(
    @{
        Id          = "singlestep"
        Name        = "Tom Harte SingleStepTests (68000 Reference Vectors)"
        TargetDir   = Join-Path $RepoRoot "ref_src\SingleStepTests-680x0"
        Version     = "Format v1 (68000/v1/)"
        Url         = "https://github.com/SingleStepTests/680x0/archive/refs/heads/main.zip"
        Description = "124 per-instruction test suites captured directly from physical 68000 silicon pins"
    },
    @{
        Id          = "amigatestkit"
        Name        = "Amiga Test Kit Diagnostic Floppy Disk (Keir Fraser)"
        TargetDir   = Join-Path $RepoRoot "tools\AmigaTestKit"
        Version     = "Release v1.21"
        Url         = "https://github.com/keirf/amiga-stuff/releases/download/testkit-v1.21/AmigaTestKit-1.21.zip"
        Description = "Diagnostic floppy disk (AmigaTestKit.adf) for CIA, custom chips, and memory testing"
    },
    @{
        Id          = "vamiga"
        Name        = "vAmiga C++ Reference Emulator (Dirk W. Hoffmann)"
        TargetDir   = Join-Path $RepoRoot "ref_src\vAmiga"
        Version     = "Release v4.5"
        Url         = "https://github.com/dirkwhoffmann/vAmiga/archive/refs/tags/v4.5.zip"
        Description = "Clean-room C++ Amiga 500 emulator sources for cross-verification & Graphify indexing"
    },
    @{
        Id          = "vamigats"
        Name        = "vAmigaTS Chipset Regression Test Suite (Dirk W. Hoffmann)"
        TargetDir   = Join-Path $RepoRoot "ref_src\vAmigaTS"
        Version     = "master"
        Url         = "https://github.com/dirkwhoffmann/vAmigaTS/archive/refs/heads/master.zip"
        Description = "2,000+ custom chipset test cases (Copper, Blitter, Denise, Paula) with golden viewports"
    },
    @{
        Id          = "minimig"
        Name        = "MiniMig FPGA Amiga Implementation"
        TargetDir   = Join-Path $RepoRoot "ref_src\minimig"
        Version     = "89c2643101860be4caa6ffb275e9bdd7c60d61d2"
        Url         = "https://codeload.github.com/cristianoag/minimig/zip/89c2643101860be4caa6ffb275e9bdd7c60d61d2"
        ArchiveSha256 = "6EAFBB93AA5D448026DC2A285DFB0AD4012AF865B1863CBD54BEA08ADFBCED03"
        FileCount   = 142
        RequiredFiles = @("README.md", "software and firmware/old/source/fpga core/Agnus.v", "software and firmware/old/source/fpga core/Paula.v")
        Description = "FPGA chipset model and supporting MiniMig material for comparison with hardware evidence"
    },
    @{
        Id          = "amiga_replacement_project"
        Name        = "Amiga Replacement Project"
        TargetDir   = Join-Path $RepoRoot "ref_src\amiga_replacement_project"
        Version     = "143ad5f1b5acae093ea66186e832a8ab4fb1bc0f"
        Url         = "https://codeload.github.com/nonarkitten/amiga_replacement_project/zip/143ad5f1b5acae093ea66186e832a8ab4fb1bc0f"
        ArchiveSha256 = "FA2670EB023202B965E72F542F26C0CAA88E35D857FBDDEB7AE0671D87744FE3"
        FileCount   = 171
        RequiredFiles = @("README.md", "LICENSE.md", "paula/Paula.v", "agnus/ice/Agnus.v")
        Description = "FPGA chip reconstructions for inspecting proposed signal and register behavior"
    }
)

# -----------------------------------------------------------------------------
# Status & Verification Helpers
# -----------------------------------------------------------------------------
function Test-SourceProvisioned {
    param([hashtable]$Source)

    if ($Source.ArchiveSha256) {
        return Test-PinnedSourceProvisioned -Source $Source
    }

    if ($Source.Id -eq "singlestep") {
        $v1Dir = Join-Path $Source.TargetDir "68000\v1"
        if (Test-Path $v1Dir) {
            $count = (Get-ChildItem -Path $v1Dir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count
            return [bool]($count -ge 124)
        }
        return $false
    }
    elseif ($Source.Id -eq "amigatestkit") {
        $adf = Join-Path $Source.TargetDir "AmigaTestKit.adf"
        return [bool]((Test-Path $adf) -and (Get-Item $adf).Length -gt 0)
    }
    elseif ($Source.Id -eq "vamiga") {
        if (Test-Path $Source.TargetDir) {
            $hasFiles = Get-ChildItem -Path $Source.TargetDir -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
            return ($null -ne $hasFiles)
        }
        return $false
    }
    elseif ($Source.Id -eq "vamigats") {
        if (Test-Path $Source.TargetDir) {
            $hasFiles = Get-ChildItem -Path $Source.TargetDir -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
            return ($null -ne $hasFiles)
        }
        return $false
    }
    return $false
}

function Test-PinnedSourceProvisioned {
    param([hashtable]$Source)

    $manifestPath = Join-Path $Source.TargetDir "_bootstrap_source.json"
    if (-not (Test-Path -LiteralPath $manifestPath)) { return $false }
    try {
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ($manifest.revision -ne $Source.Version -or
            $manifest.archive_sha256 -ne $Source.ArchiveSha256 -or
            $manifest.files.Count -ne $Source.FileCount) { return $false }
        foreach ($file in $manifest.files) {
            $path = Join-Path $Source.TargetDir $file.path
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $false }
            if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $file.sha256) { return $false }
        }
        foreach ($relativePath in $Source.RequiredFiles) {
            if (-not (Test-Path -LiteralPath (Join-Path $Source.TargetDir $relativePath) -PathType Leaf)) { return $false }
        }
        return $true
    } catch {
        return $false
    }
}

function Install-PinnedSource {
    param([hashtable]$Source)

    $referenceRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "ref_src"))
    $target = [System.IO.Path]::GetFullPath($Source.TargetDir)
    if (-not $target.StartsWith($referenceRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Pinned source target is outside ref_src/: $target"
    }
    $staging = Join-Path $referenceRoot ("_bootstrap_{0}_{1}" -f $Source.Id, [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $staging -Force | Out-Null
    try {
        $archive = Join-Path $staging "source.zip"
        if (-not (Download-Archive -Url $Source.Url -DestinationPath $archive -Name $Source.Name)) {
            throw "Download failed for $($Source.Name)"
        }
        $actualHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        if ($actualHash -ne $Source.ArchiveSha256) {
            throw "Archive hash mismatch for $($Source.Name): $actualHash"
        }
        $extracted = Join-Path $staging "extracted"
        Expand-Archive -LiteralPath $archive -DestinationPath $extracted
        $roots = @(Get-ChildItem -LiteralPath $extracted -Directory)
        if ($roots.Count -ne 1) { throw "Expected one archive root for $($Source.Name)" }
        $sourceRoot = $roots[0].FullName
        $files = @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Force)
        if ($files.Count -ne $Source.FileCount) {
            throw "Incomplete archive for $($Source.Name): $($files.Count) files, expected $($Source.FileCount)"
        }
        foreach ($relativePath in $Source.RequiredFiles) {
            if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot $relativePath) -PathType Leaf)) {
                throw "Missing required file for $($Source.Name): $relativePath"
            }
        }
        $manifestFiles = @($files | ForEach-Object {
            [ordered]@{
                path = $_.FullName.Substring($sourceRoot.Length + 1).Replace('\', '/')
                sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
            }
        })
        [ordered]@{
            upstream_url = $Source.Url
            revision = $Source.Version
            archive_sha256 = $Source.ArchiveSha256
            files = $manifestFiles
        } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $sourceRoot "_bootstrap_source.json") -Encoding UTF8

        if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
        Move-Item -LiteralPath $sourceRoot -Destination $target
        Write-Host "  [OK] Installed pinned revision $($Source.Version)" -ForegroundColor Green
    } finally {
        if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    }
}

function Show-CatalogList {
    Write-Host ""
    Write-Host "External Hardware Test Sources & Reference Catalog" -ForegroundColor Cyan
    Write-Host "=================================================" -ForegroundColor Cyan
    Write-Host ""

    foreach ($s in $SourcesCatalog) {
        $provisioned = Test-SourceProvisioned -Source $s
        $statusTag = if ($provisioned) { "[PRESENT]" } else { "[MISSING]" }
        $statusColor = if ($provisioned) { "Green" } else { "Yellow" }
        $relPath = $s.TargetDir.Substring($RepoRoot.Length).TrimStart('\', '/')

        Write-Host "$statusTag $($s.Name)" -ForegroundColor $statusColor
        Write-Host "    Version   : $($s.Version)" -ForegroundColor White
        Write-Host "    Directory : $relPath" -ForegroundColor DarkGray
        Write-Host "    Source URL: $($s.Url)" -ForegroundColor DarkGray
        Write-Host ""
    }
}

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 External Sources & Test Suites Bootstrapper" -ForegroundColor Cyan
    Write-Host "=====================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1              : Show usage without downloading"
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1 -All         : Download, verify & provision all external sources"
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1 -MiniMig -AmigaReplacementProject : Provision both FPGA references"
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1 -List        : Show catalog and local status of upstream sources"
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1 -All -Force  : Re-download and replace all sources"
    Write-Host "  .\tools\bootstrap\bootstrap_sources.ps1 -Help        : Display this help message"
    Write-Host ""
    Write-Host "Available Parameters:" -ForegroundColor White
    Write-Host "  -List                    : Display catalog of upstream sources and local presence"
    Write-Host "  -Force                   : With selected sources or -All, re-download and replace their local copies"
    Write-Host "  -SingleStep              : Provision only Tom Harte SingleStepTests 68000 vectors"
    Write-Host "  -AmigaTestKit            : Provision only Amiga Test Kit diagnostic disk (ADF)"
    Write-Host "  -VAmiga                  : Provision only vAmiga C++ reference emulator sources"
    Write-Host "  -VAmigaTS                : Provision only vAmigaTS chipset regression suites"
    Write-Host "  -MiniMig                 : Provision only pinned MiniMig FPGA reference in ref_src/minimig"
    Write-Host "  -AmigaReplacementProject : Provision only pinned Amiga Replacement Project in ref_src/amiga_replacement_project"
    Write-Host "  -All                     : Provision all external test and research sources"
    Write-Host "  -Help                    : Display this usage and parameter list"
    Write-Host ""
}

# -----------------------------------------------------------------------------
# Download & Extraction Helpers
# -----------------------------------------------------------------------------
function Download-Archive {
    param(
        [string]$Url,
        [string]$DestinationPath,
        [string]$Name
    )

    Write-Host "  Downloading $Name..." -ForegroundColor Cyan
    Write-Host "  URL: $Url" -ForegroundColor DarkGray

    $curlCmd = Get-Command "curl.exe" -ErrorAction SilentlyContinue
    if ($curlCmd) {
        & curl.exe -f -L --retry 3 --retry-delay 2 -o $DestinationPath $Url
        if ($LASTEXITCODE -eq 0 -and (Test-Path $DestinationPath) -and (Get-Item $DestinationPath).Length -gt 0) {
            $sz = (Get-Item $DestinationPath).Length
            Write-Host "  [OK] Downloaded: $(Split-Path -Leaf $DestinationPath) ($([math]::Round($sz / 1MB, 2)) MB)" -ForegroundColor Green
            return $true
        }
    }

    # Fallback to .NET WebClient with TLS 1.2
    try {
        $wc = New-Object System.Net.WebClient
        $wc.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        $wc.DownloadFile($Url, $DestinationPath)
        if ((Test-Path $DestinationPath) -and (Get-Item $DestinationPath).Length -gt 0) {
            $sz = (Get-Item $DestinationPath).Length
            Write-Host "  [OK] Downloaded: $(Split-Path -Leaf $DestinationPath) ($([math]::Round($sz / 1MB, 2)) MB)" -ForegroundColor Green
            return $true
        }
    } catch {
        Write-Error "  Failed to download $Name from $Url : $($_.Exception.Message)"
        return $false
    }

    return $false
}

function Expand-And-Flatten {
    param(
        [string]$ZipPath,
        [string]$DestinationDir
    )

    if (-not (Test-Path $DestinationDir)) {
        New-Item -ItemType Directory -Path $DestinationDir -Force | Out-Null
    }

    $TempExtract = Join-Path $DestinationDir "_extract_temp"
    if (Test-Path $TempExtract) {
        Remove-Item $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host "  Expanding archive $(Split-Path -Leaf $ZipPath)..." -ForegroundColor Cyan
    Expand-Archive -Path $ZipPath -DestinationPath $TempExtract -Force

    # If the zip created a single top-level directory (e.g. repo-main, vAmiga-4.5, AmigaTestKit-1.21)
    $TopItems = Get-ChildItem -Path $TempExtract
    if ($TopItems.Count -eq 1 -and $TopItems[0].PSIsContainer) {
        $InnerDir = $TopItems[0].FullName
        Get-ChildItem -Path $InnerDir -Force | ForEach-Object {
            $TargetItem = Join-Path $DestinationDir $_.Name
            if (Test-Path $TargetItem) {
                Remove-Item -Path $TargetItem -Recurse -Force -ErrorAction SilentlyContinue
            }
            Move-Item -Path $_.FullName -Destination $DestinationDir -Force
        }
    } else {
        Get-ChildItem -Path $TempExtract -Force | ForEach-Object {
            $TargetItem = Join-Path $DestinationDir $_.Name
            if (Test-Path $TargetItem) {
                Remove-Item -Path $TargetItem -Recurse -Force -ErrorAction SilentlyContinue
            }
            Move-Item -Path $_.FullName -Destination $DestinationDir -Force
        }
    }

    Remove-Item $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $ZipPath -Force -ErrorAction SilentlyContinue
}

function Decompress-GzJsonSuites {
    param([string]$BaseDir)

    $GzFiles = Get-ChildItem -Path $BaseDir -Filter "*.gz" -Recurse -ErrorAction SilentlyContinue
    if ($GzFiles.Count -eq 0) {
        return
    }

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
        Write-Host "  Decompressed $DecompressedCount SingleStep test suite(s) from .gz archives." -ForegroundColor Green
    }
}

function Canonicalize-SingleStepDirs {
    param(
        [string]$BaseDir,
        [string]$TargetDir
    )

    # Relocate any root json files into 68000/v1/
    $Parent68kDir = Join-Path $BaseDir "68000"
    if (Test-Path $Parent68kDir) {
        if (-not (Test-Path $TargetDir)) {
            New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
        }
        $Root68kJsons = Get-ChildItem -Path $Parent68kDir -Filter "*.json" -File -ErrorAction SilentlyContinue
        if ($Root68kJsons.Count -gt 0 -and (Get-ChildItem -Path $TargetDir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
            Write-Host "  Relocating $($Root68kJsons.Count) test suites into canonical v1 directory ($TargetDir)..." -ForegroundColor Cyan
            foreach ($F in $Root68kJsons) {
                Move-Item -Path $F.FullName -Destination $TargetDir -Force
            }
        }
    }
}

# -----------------------------------------------------------------------------
# Main Execution Logic
# -----------------------------------------------------------------------------

# Handle the canonical help switch.
if ($Help) {
    Show-Usage
    exit 0
}

if ($List) {
    Show-CatalogList
    exit 0
}

# A parameterized source tool needs an explicit selection.
if ($PSBoundParameters.Count -eq 0) {
    Show-Usage
    exit 0
}

# Determine which sources to process
$SelectedIds = @()
if ($SingleStep)   { $SelectedIds += "singlestep" }
if ($AmigaTestKit) { $SelectedIds += "amigatestkit" }
if ($VAmiga)       { $SelectedIds += "vamiga" }
if ($VAmigaTS)     { $SelectedIds += "vamigats" }
if ($MiniMig)      { $SelectedIds += "minimig" }
if ($AmigaReplacementProject) { $SelectedIds += "amiga_replacement_project" }

if ($All) {
    $SelectedIds = @("singlestep", "amigatestkit", "vamiga", "vamigats", "minimig", "amiga_replacement_project")
} elseif ($SelectedIds.Count -eq 0) {
    Write-Error "Select a source or pass -All. -Force alone does not select sources."
    Show-Usage
    exit 1
}

Write-Host ""
Write-Host "Bootstrapping Verification & Hardware Test Suites..." -ForegroundColor Green
Write-Host "---------------------------------------------------------" -ForegroundColor DarkGray

$HasErrors = $false

foreach ($source in $SourcesCatalog) {
    if ($SelectedIds -notcontains $source.Id) {
        continue
    }

    Write-Host ""
    Write-Host "[$($source.Id.ToUpper())] $($source.Name)" -ForegroundColor Yellow

    $isProvisioned = Test-SourceProvisioned -Source $source

    if ($isProvisioned -and -not $Force) {
        $relPath = $source.TargetDir.Substring($RepoRoot.Length).TrimStart('\', '/')
        Write-Host "  [SKIP] Already provisioned: $relPath" -ForegroundColor DarkGray
    } else {
        if ($source.ArchiveSha256) {
            try {
                Install-PinnedSource -Source $source
            } catch {
                Write-Error $_.Exception.Message
                $HasErrors = $true
            }
            continue
        }
        if (-not (Test-Path $source.TargetDir)) {
            New-Item -ItemType Directory -Path $source.TargetDir -Force | Out-Null
        }

        $TempZip = Join-Path $source.TargetDir "_download.zip"
        $downloadOk = Download-Archive -Url $source.Url -DestinationPath $TempZip -Name $source.Name

        if ($downloadOk) {
            Expand-And-Flatten -ZipPath $TempZip -DestinationDir $source.TargetDir
        } else {
            $HasErrors = $true
            Write-Warning "Failed to download and provision $($source.Name)."
        }
    }

    # Post-processing for SingleStepTests
    if ($source.Id -eq "singlestep") {
        $v1Dir = Join-Path $source.TargetDir "68000\v1"

        # Expand any loose .zip archives inside ref_src/SingleStepTests-680x0
        $NestedZips = Get-ChildItem -Path $source.TargetDir -Filter "*.zip" -Recurse -ErrorAction SilentlyContinue
        foreach ($z in $NestedZips) {
            if ($z.Name -ne "_download.zip") {
                Write-Host "  Expanding nested archive: $($z.Name)..." -ForegroundColor Cyan
                Expand-Archive -Path $z.FullName -DestinationPath $z.DirectoryName -Force
            }
        }

        # Decompress any .json.gz or .gz archives
        Decompress-GzJsonSuites -BaseDir $source.TargetDir

        # Canonicalize 68000/ to 68000/v1/
        Canonicalize-SingleStepDirs -BaseDir $source.TargetDir -TargetDir $v1Dir
    }
}

foreach ($source in $SourcesCatalog | Where-Object { $_.ArchiveSha256 }) {
    if (Test-PinnedSourceProvisioned -Source $source) {
        Write-Host "  [OK] $($source.Name): ref_src/$($source.Id) @ $($source.Version)" -ForegroundColor Green
    } elseif ($SelectedIds -contains $source.Id) {
        Write-Warning "  [MISSING] $($source.Name) pinned source is absent or incomplete."
        $HasErrors = $true
    }
}

# -----------------------------------------------------------------------------
# Final Validation Summary
# -----------------------------------------------------------------------------
Write-Host ""
Write-Host "Verification & Provisioning Summary" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan

# 1. SingleStepTests
$SingleStepBaseDir = Join-Path $RepoRoot "ref_src\SingleStepTests-680x0"
$SingleStepDir = Join-Path $SingleStepBaseDir "68000\v1"
if (Test-Path $SingleStepDir) {
    $JsonCount = (Get-ChildItem -Path $SingleStepDir -Filter "*.json" -ErrorAction SilentlyContinue | Measure-Object).Count
    if ($JsonCount -ge 124) {
        Write-Host "  [OK] SingleStepTests-680x0 : $JsonCount / 124 test suites in ref_src\SingleStepTests-680x0\68000\v1" -ForegroundColor Green
    } elseif ($JsonCount -gt 0) {
        Write-Warning "  [PARTIAL] SingleStepTests contains only $JsonCount / 124 test suites."
        if ($SelectedIds -contains "singlestep") { $HasErrors = $true }
    } else {
        Write-Warning "  [FAIL] SingleStepTests contains zero .json test suites."
        if ($SelectedIds -contains "singlestep") { $HasErrors = $true }
    }
} else {
    if ($SelectedIds -contains "singlestep") {
        Write-Warning "  [MISSING] SingleStepTests directory not found: ref_src\SingleStepTests-680x0\68000\v1"
        $HasErrors = $true
    } else {
        Write-Host "  [NOT REQUESTED] SingleStepTests directory not found (pass -SingleStep to provision)" -ForegroundColor DarkGray
    }
}

# 2. AmigaTestKit
$AdfTestKit = Join-Path $RepoRoot "tools\AmigaTestKit\AmigaTestKit.adf"
if (Test-Path $AdfTestKit) {
    Write-Host "  [OK] AmigaTestKit         : tools\AmigaTestKit\AmigaTestKit.adf found ($([math]::Round((Get-Item $AdfTestKit).Length / 1KB, 1)) KB)" -ForegroundColor Green
} else {
    if ($SelectedIds -contains "amigatestkit") {
        Write-Warning "  [MISSING] AmigaTestKit diagnostic disk not found: tools\AmigaTestKit\AmigaTestKit.adf"
        $HasErrors = $true
    } else {
        Write-Host "  [NOT REQUESTED] AmigaTestKit diagnostic disk not found (pass -AmigaTestKit to provision)" -ForegroundColor DarkGray
    }
}

# 3. vAmiga
$VAmigaDir = Join-Path $RepoRoot "ref_src\vAmiga"
if (Test-Path $VAmigaDir) {
    $fileCount = (Get-ChildItem -Path $VAmigaDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
    if ($fileCount -gt 0) {
        Write-Host "  [OK] vAmiga C++ Core      : ref_src\vAmiga found ($fileCount source files)" -ForegroundColor Green
    } else {
        Write-Warning "  [EMPTY] vAmiga directory exists but contains zero files."
        if ($SelectedIds -contains "vamiga") { $HasErrors = $true }
    }
} else {
    if ($SelectedIds -contains "vamiga") {
        Write-Warning "  [MISSING] vAmiga C++ reference emulator not found: ref_src\vAmiga"
        $HasErrors = $true
    } else {
        Write-Host "  [NOT REQUESTED] vAmiga C++ reference emulator not found (pass -VAmiga to provision)" -ForegroundColor DarkGray
    }
}

# 4. vAmigaTS
$VAmigaTsDir = Join-Path $RepoRoot "ref_src\vAmigaTS"
if (Test-Path $VAmigaTsDir) {
    $testDirsCount = (Get-ChildItem -Path $VAmigaTsDir -Directory -ErrorAction SilentlyContinue | Measure-Object).Count
    if ($testDirsCount -gt 0) {
        Write-Host "  [OK] vAmigaTS Test Suite  : ref_src\vAmigaTS found ($testDirsCount test categories)" -ForegroundColor Green
    } else {
        Write-Warning "  [EMPTY] vAmigaTS directory exists but contains zero test categories."
        if ($SelectedIds -contains "vamigats") { $HasErrors = $true }
    }
} else {
    if ($SelectedIds -contains "vamigats") {
        Write-Warning "  [MISSING] vAmigaTS regression test suite not found: ref_src\vAmigaTS"
        $HasErrors = $true
    } else {
        Write-Host "  [NOT REQUESTED] vAmigaTS regression test suite not found (pass -VAmigaTS to provision)" -ForegroundColor DarkGray
    }
}

Write-Host ""
if ($HasErrors) {
    Write-Error "One or more external test sources could not be provisioned. Review messages above."
    exit 1
} else {
    Write-Host "All requested external verification & test sources provisioned successfully." -ForegroundColor Green
    exit 0
}
