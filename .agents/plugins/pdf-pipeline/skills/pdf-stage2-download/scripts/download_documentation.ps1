<#
.SYNOPSIS
    Automated downloader for external Amiga and 68000 reference PDF manuals.

.DESCRIPTION
    Fetches raw source technical PDF manuals directly into target project manual directories:
      1. Hardware Reference Manual (Addison-Wesley 2nd Edition)
      2. A500 A2000 Technical Reference Manual (Commodore 1987)
      3. 68000 Programmer's Reference Manual (Motorola M68000PM/AD Rev 1)
      4. 68000 User's Manual (Motorola M68000UM/AD Rev 8)

    Features:
    - Multi-source resilience: 2-3 verified mirrors per document with automated failover.
    - All-sources mode (-AllSources / -AllMirrors): downloads from all available mirrors
      for comprehensive testing or archival redundancy.
    - Idempotent: Skips downloading if target PDF is already provisioned and matches minimum size.
    - Clear error reporting if all mirror sources for an item are unavailable.

.PARAMETER All
    Downloads all configured reference PDF manuals in the catalog.

.PARAMETER Destination
    Custom destination directory (defaults to repository root).

.PARAMETER AllSources
    Downloads from ALL mirrors and sources for each document, rather than stopping after
    the first successful mirror. Alias: -AllMirrors.

.PARAMETER Force
    Forces re-download even if target file already exists and byte size matches.

.PARAMETER List
    Displays catalog of reference documents and mirror sources.

.PARAMETER Hrm
    Processes only the Commodore Amiga Hardware Reference Manual.

.PARAMETER Trm
    Processes only the A500 A2000 Technical Reference Manual.

.PARAMETER Prm
    Processes only the 68000 Programmer's Reference Manual.

.PARAMETER Um
    Processes only the 68000 User's Manual.

.PARAMETER Help
    Displays usage instructions and parameter descriptions. Aliases: -h, -?, --help.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -List
    Displays catalog of reference documents and mirror sources.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -All
    Downloads all configured reference materials in failover mode.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Hrm
    Downloads only the Hardware Reference Manual.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Help
    Displays available parameters, flags, and usage examples.
#>

[CmdletBinding()]
param(
    [switch]$All,
    [string]$Destination,
    [Alias("AllMirrors")]
    [switch]$AllSources,
    [switch]$Force,
    [switch]$List,
    [switch]$Hrm,
    [switch]$Trm,
    [switch]$Prm,
    [switch]$Um,
    [string]$Manual,
    [Alias("h", "?")]
    [switch]$Help,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingArgs
)

# -----------------------------------------------------------------------------
# Configuration & Security Protocols
# -----------------------------------------------------------------------------
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12 -bor [System.Net.SecurityProtocolType]::Tls13
# Allow fallback for archival servers with legacy or self-signed certificates
[System.Net.ServicePointManager]::ServerCertificateValidationCallback = { $true }

$dir = $PSScriptRoot
while ($dir -and -not (Test-Path (Join-Path $dir ".git"))) {
    $parent = Split-Path -Parent $dir
    if ($parent -eq $dir) { break }
    $dir = $parent
}
$RepoRoot = if ($dir -and (Test-Path (Join-Path $dir ".git"))) { $dir } else { Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))) }

if (-not $Destination) {
    $Destination = $RepoRoot
}

$DefaultUserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"

# -----------------------------------------------------------------------------
# Catalog Definition (2-3 Verified Mirrors per Item)
# -----------------------------------------------------------------------------
$Catalog = @(
    @{
        Id          = "hrm"
        Name        = "Hardware Reference Manual"
        Folder      = "Hardware Reference Manual"
        Type        = "SingleFile"
        TargetFile  = "Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf"
        MinSize     = 30000000
        Description = "Addison-Wesley 2nd Edition (1989) covering pure A500 OCS (405 pages, 600 DPI)"
        Mirrors     = @(
            @{
                Name    = "Internet Archive (1989 2nd Ed OCS PDF, Primary A500 OCS 405p 600 DPI)"
                Url     = "https://archive.org/download/commodore-amiga-hardware-reference-manual-2nd/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf"
                File    = "Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf"
                MinSize = 30000000
            },
            @{
                Name    = "Internet Archive (1985 1st Ed PDF, Failover 1st Ed)"
                Url     = "https://archive.org/download/Amiga_Hardware_Reference_Manual_1985_Commodore/Amiga_Hardware_Reference_Manual_1985_Commodore.pdf"
                File    = "Amiga_Hardware_Reference_Manual_1985_Commodore.pdf"
                MinSize = 2000000
            }
        )
    },
    @{
        Id          = "trm"
        Name        = "A500 A2000 Technical Reference Manual"
        Folder      = "A500 A2000 Technical Reference Manual"
        Type        = "SingleFile"
        TargetFile  = "Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf"
        MinSize     = 10000000
        Description = "Commodore-Amiga OEM Manual (1987) with schematics and expansion architecture"
        Mirrors     = @(
            @{
                Name    = "Internet Archive (1987 OEM Clean Scan PDF, Primary 308p 200 DPI)"
                Url     = "https://archive.org/download/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf"
                File    = "Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf"
                MinSize = 10000000
            },
            @{
                Name    = "Internet Archive (1987 OEM Alternate Scan PDF, Failover 309p)"
                Url     = "https://archive.org/download/CommodoreAmigaA500A2000TechnicalReferenceManual/Commodore%20Amiga%20A500-A2000%20Technical%20Reference%20Manual.pdf"
                File    = "Commodore_Amiga_A500-A2000_Technical_Reference_Manual.pdf"
                MinSize = 15000000
            }
        )
    },
    @{
        Id          = "prm"
        Name        = "68000 Programmer's Reference Manual"
        Folder      = "68000 Programmer's Reference Manual"
        Type        = "SingleFile"
        TargetFile  = "M68000PRM.pdf"
        MinSize     = 2000000
        Description = "Motorola M68000PM/AD Rev 1 (1992) born-digital vector manual (646 pages)"
        Mirrors     = @(
            @{
                Name    = "Internet Archive (M68000PM/AD Rev 1 1992 Born-Digital Vector PDF, Primary 646p)"
                Url     = "https://archive.org/download/M68000PRM/M68000PRM.pdf"
                File    = "M68000PRM.pdf"
                MinSize = 4000000
            },
            @{
                Name    = "Internet Archive / Bitsavers (M68000PM/AD Rev 1 1992 PDF, Failover)"
                Url     = "https://archive.org/download/bitsavers_motorola68ogrammersReferenceManual1992_2394181/M68000PM_AD_Rev_1_Programmers_Reference_Manual_1992.pdf"
                File    = "M68000PM_AD_Rev_1_Programmers_Reference_Manual_1992.pdf"
                MinSize = 2000000
            }
        )
    },
    @{
        Id          = "um"
        Name        = "68000 User's Manual"
        Folder      = "68000 User's Manual"
        Type        = "SingleFile"
        TargetFile  = "M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf"
        MinSize     = 8000000
        Description = "Motorola M68000UM/AD Rev 8 (1993) with bus cycle timing and electrical tables"
        Mirrors     = @(
            @{
                Name    = "Internet Archive / Bitsavers (Rev 8 1993 PDF, Primary 601 DPI 216p)"
                Url     = "https://archive.org/download/bitsavers_motorola68MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf"
                File    = "M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf"
                MinSize = 8000000
            },
            @{
                Name    = "Internet Archive (Rev 8 Alternate Archive Item, Failover)"
                Url     = "https://archive.org/download/bitsavers_motorola6868000MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf"
                File    = "M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf"
                MinSize = 8000000
            },
            @{
                Name    = "Internet Archive / Bitsavers (Motorola 68000 Family Reference 1988 PDF, Companion 608p)"
                Url     = "https://archive.org/download/bitsavers_motorola68rence1988_23248083/M68000_Family_Reference_1988.pdf"
                File    = "M68000_Family_Reference_1988.pdf"
                MinSize = 15000000
            }
        )
    }
)

# -----------------------------------------------------------------------------
# Helper Functions
# -----------------------------------------------------------------------------

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga & 68000 Reference Manual Downloader" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Downloads source technical PDF manuals directly into target project manual directories." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -List     : Show all documents and configured mirrors"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -All      : Download all reference manuals (failover mode)"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Hrm      : Download only Hardware Reference Manual"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Trm      : Download only A500 A2000 Technical Reference Manual"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Prm      : Download only 68000 Programmer's Reference Manual"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Um       : Download only 68000 User's Manual"
    Write-Host "  powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Help     : Display this usage and parameter list"
    Write-Host ""
    Write-Host "Available Parameters:" -ForegroundColor White
    Write-Host "  -List                    : Display catalog of reference documents and mirrors"
    Write-Host "  -All                     : Process all reference materials in the catalog"
    Write-Host "  -Hrm                     : Process only Hardware Reference Manual"
    Write-Host "  -Trm                     : Process only A500 A2000 Technical Reference Manual"
    Write-Host "  -Prm                     : Process only 68000 Programmer's Reference Manual"
    Write-Host "  -Um                      : Process only 68000 User's Manual"
    Write-Host "  -Destination <path>      : Custom destination directory (defaults to repository root)"
    Write-Host "  -AllSources              : Download from all mirrors for redundancy (alias: -AllMirrors)"
    Write-Host "  -Force                   : Re-download even if target file already exists"
    Write-Host "  -Help                    : Display this usage and parameter list (aliases: -h, -?, --help)"
    Write-Host ""
}

function Show-CatalogList {
    Write-Host ""
    Write-Host "Reference Documentation Catalog & Mirror Matrix" -ForegroundColor Cyan
    Write-Host "==============================================" -ForegroundColor Cyan
    Write-Host ""
    foreach ($item in $Catalog) {
        Write-Host "[$($item.Id)] $($item.Name)" -ForegroundColor Green
        Write-Host "    Description : $($item.Description)" -ForegroundColor White
        Write-Host "    Directory   : $($item.Folder)\build\" -ForegroundColor DarkGray
        Write-Host "    Target PDF  : $($item.TargetFile)" -ForegroundColor DarkGray
        Write-Host "    Mirrors ($($item.Mirrors.Count) configured):" -ForegroundColor Yellow
        $idx = 1
        foreach ($m in $item.Mirrors) {
            $mFile = if ($m.File) { " (File: $($m.File))" } else { "" }
            Write-Host "      $idx. $($m.Name)$mFile" -ForegroundColor White
            Write-Host "         $($m.Url)" -ForegroundColor DarkGray
            $idx++
        }
        Write-Host ""
    }
}

function Download-SingleFile {
    param(
        [hashtable]$Item,
        [string]$TargetDir,
        [bool]$ForceDownload,
        [bool]$AllSourcesMode
    )

    $FinalTargetPath = Join-Path $TargetDir $Item.TargetFile
    if (-not $ForceDownload -and (Test-Path $FinalTargetPath)) {
        $FinalSize = (Get-Item $FinalTargetPath).Length
        if ($FinalSize -ge $Item.MinSize) {
            Write-Host "  [SKIP] Target already provisioned: $($Item.TargetFile) ($([math]::Round($FinalSize / 1MB, 2)) MB)" -ForegroundColor DarkGray
            if (-not $AllSourcesMode) {
                return $true
            }
        }
    }

    $MirrorIndex = 1
    $TotalMirrors = $Item.Mirrors.Count
    $AnySuccess = $false

    foreach ($mirror in $Item.Mirrors) {
        $SourceUrl = $mirror.Url
        $ActualFileName = if ($mirror.File) { $mirror.File } else { $Item.TargetFile }
        $ActualDestPath = Join-Path $TargetDir $ActualFileName

        $MinExpected = if ($mirror.MinSize) { $mirror.MinSize } elseif ($mirror.File) { 1000 } else { $Item.MinSize }

        if (-not $ForceDownload -and (Test-Path $ActualDestPath)) {
            $CurrentSize = (Get-Item $ActualDestPath).Length
            if ($CurrentSize -ge $MinExpected) {
                Write-Host "  [SKIP] Already present ($($mirror.Name)): $ActualFileName ($([math]::Round($CurrentSize / 1MB, 2)) MB)" -ForegroundColor DarkGray
                $AnySuccess = $true
                if (-not $AllSourcesMode) {
                    return $true
                }
                $MirrorIndex++
                continue
            }
        }

        Write-Host "  Downloading from mirror [$MirrorIndex/$TotalMirrors]: $($mirror.Name)..." -ForegroundColor Cyan
        Write-Host "    $SourceUrl" -ForegroundColor DarkGray

        try {
            $webRequest = [System.Net.HttpWebRequest]::Create($SourceUrl)
            $webRequest.Method = "GET"
            $webRequest.Timeout = 30000
            $webRequest.UserAgent = $DefaultUserAgent
            $webRequest.AllowAutoRedirect = $true

            $webResponse = $webRequest.GetResponse()
            $responseStream = $webResponse.GetResponseStream()
            $fileStream = [System.IO.File]::Create($ActualDestPath)

            $buffer = New-Object byte[] 65536
            $bytesRead = 0
            $totalBytes = 0

            while (($bytesRead = $responseStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                $fileStream.Write($buffer, 0, $bytesRead)
                $totalBytes += $bytesRead
            }

            $fileStream.Flush()
            $fileStream.Close()
            $fileStream.Dispose()
            $responseStream.Close()
            $responseStream.Dispose()
            $webResponse.Close()
            $webResponse.Dispose()

            if ($totalBytes -ge $MinExpected) {
                Write-Host "  [OK] Downloaded successfully: $ActualFileName ($([math]::Round($totalBytes / 1MB, 2)) MB)" -ForegroundColor Green
                $AnySuccess = $true
                if (-not $AllSourcesMode) {
                    return $true
                }
            } else {
                Write-Warning "  Downloaded file is smaller than expected ($totalBytes bytes < $MinExpected bytes)."
                Remove-Item -Path $ActualDestPath -Force -ErrorAction SilentlyContinue
            }
        }
        catch {
            Write-Warning "  Mirror failed: $($_.Exception.Message)"
            if (Test-Path $ActualDestPath) {
                Remove-Item -Path $ActualDestPath -Force -ErrorAction SilentlyContinue
            }
        }

        $MirrorIndex++
    }

    if ($AnySuccess) {
        return $true
    }

    Write-Error "ERROR: All $TotalMirrors configured mirror sources for '$($Item.Name)' failed. Please verify internet connection or manually place the file in: $TargetDir"
    return $false
}

# -----------------------------------------------------------------------------
# Main Execution Logic
# -----------------------------------------------------------------------------

# Handle help request explicitly (-Help, -h, -?, --help)
$IsHelpRequested = $Help -or ($RemainingArgs -contains "--help") -or ($RemainingArgs -contains "-help") -or ($RemainingArgs -contains "help") -or ($RemainingArgs -contains "-h") -or ($RemainingArgs -contains "-?")
if ($IsHelpRequested) {
    Show-Usage
    exit 0
}

if ($List) {
    Show-CatalogList
    exit 0
}

# Auto-promote -AllSources or -Force to -All
if ($AllSources -or $Force) {
    $All = $true
}

# Determine which documents to process
$SelectedIds = @()
if ($Hrm) { $SelectedIds += "hrm" }
if ($Trm) { $SelectedIds += "trm" }
if ($Prm) { $SelectedIds += "prm" }
if ($Um)  { $SelectedIds += "um" }
if ($Manual) {
    $matched = $Catalog | Where-Object { $_.Id -ieq $Manual -or $_.Folder -ieq $Manual -or $_.Name -ieq $Manual }
    foreach ($m in $matched) {
        if (-not ($SelectedIds -contains $m.Id)) {
            $SelectedIds += $m.Id
        }
    }
}

$HasExplicitAction = $All -or ($SelectedIds.Count -gt 0)
if (-not $HasExplicitAction) {
    Show-Usage
    exit 0
}

$ItemsToProcess = @(
    if ($SelectedIds.Count -gt 0) {
        $Catalog | Where-Object { $SelectedIds -contains $_.Id }
    } else {
        $Catalog
    }
)

Write-Host ""
Write-Host "Provisioning Reference PDF Manuals" -ForegroundColor Cyan
Write-Host "Destination : $Destination" -ForegroundColor DarkGray
Write-Host "Items count : $($ItemsToProcess.Count)" -ForegroundColor DarkGray
Write-Host "Mode        : $(if ($AllSources) { 'All Mirrors & Sources (Redundancy Mode)' } else { 'Failover Mode (First Success)' })" -ForegroundColor Yellow
Write-Host ""

$HasErrors = $false
$ProcessedCount = 0

foreach ($entry in $ItemsToProcess) {
    $ProcessedCount++
    Write-Host "[$ProcessedCount/$($ItemsToProcess.Count)] Checking '$($entry.Name)'..." -ForegroundColor Yellow
    $ItemTargetDir = Join-Path (Join-Path $Destination $entry.Folder) "build"
    if (-not (Test-Path $ItemTargetDir)) {
        New-Item -ItemType Directory -Path $ItemTargetDir -Force | Out-Null
    }

    $DownloadSuccess = Download-SingleFile -Item $entry -TargetDir $ItemTargetDir -ForceDownload $Force -AllSourcesMode $AllSources

    if (-not $DownloadSuccess) {
        $HasErrors = $true
    }
    Write-Host ""
}

if ($HasErrors) {
    Write-Error "One or more reference downloads failed. Review warnings and errors above."
    exit 1
} else {
    Write-Host "All requested reference PDF manuals provisioned successfully." -ForegroundColor Green
    Write-Host ""
    Write-Host "Target Manual Directories:" -ForegroundColor Yellow
    foreach ($entry in $ItemsToProcess) {
        Write-Host "  - $($entry.Folder)/build/$($entry.TargetFile)" -ForegroundColor White
    }
    exit 0
}
