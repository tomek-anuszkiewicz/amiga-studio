<#
.SYNOPSIS
    Automated bootstrapper for external Amiga reference documentation.

.DESCRIPTION
    Fetches raw, unprocessed external reference materials (PDFs, multi-page HTML
    crawls, and AmigaGuide archives) into:
        Obsidian/Amiga/Reference/temp/<Document_Name>/

    Features:
    - Multi-source resilience: 2-3 verified mirrors per document with automated failover.
    - Full web crawling for multi-page articles (e.g. Kuba Winnicki's 16-page 'Achtung! Amiga').
    - Clear error reporting if all mirror sources for an item are unavailable.
    - Intentional Git visibility: temp/ is not hidden by .gitignore so temporary raw assets
      remain explicitly visible in git status until inspected, processed, or deleted.

.PARAMETER All
    Downloads all configured reference materials.

.PARAMETER Item
    Downloads a specific document by name or alias (e.g. "Hardware Reference Manual", "Prefetch").

.PARAMETER Destination
    Custom destination directory (defaults to Obsidian/Amiga/Reference/temp).

.PARAMETER Force
    Forces re-download even if target file already exists and byte size matches.

.PARAMETER List
    Displays the catalog of reference documents and their configured mirrors.

.EXAMPLE
    .\tools\bootstrap_reference.ps1 -List
    .\tools\bootstrap_reference.ps1 -Item "Hardware Reference Manual"
    .\tools\bootstrap_reference.ps1 -Item "Undocumented features"
    .\tools\bootstrap_reference.ps1 -All
#>

[CmdletBinding()]
param(
    [switch]$All,
    [string]$Item,
    [string]$Destination,
    [switch]$Force,
    [switch]$List
)

# -----------------------------------------------------------------------------
# Configuration & Security Protocols
# -----------------------------------------------------------------------------
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12 -bor [System.Net.SecurityProtocolType]::Tls13
# Allow fallback for archival servers with legacy or self-signed certificates
[System.Net.ServicePointManager]::ServerCertificateValidationCallback = { $true }

$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not $Destination) {
    $Destination = Join-Path $RepoRoot "Obsidian\Amiga\Reference\temp"
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
        Description = "Addison-Wesley 2nd Edition (1989) covering OCS/ECS, A1000/A500/A2000"
        Mirrors     = @(
            @{
                Name = "Internet Archive (1989 2nd Ed PDF)"
                Url  = "https://archive.org/download/commodore-amiga-hardware-reference-manual-2nd/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf"
            },
            @{
                Name = "Internet Archive (1991 3rd Ed ECS PDF)"
                Url  = "https://archive.org/download/amiga-hardware-reference-manual-3rd-edition/Amiga_Hardware_Reference_Manual_3rd_edition.pdf"
            },
            @{
                Name = "AmigaDev Elowar ADCD 2.1 (Online HTML Guide)"
                Url  = "http://amigadev.elowar.com/read/ADCD_2.1/Hardware_Manual_guide/node0000.html"
                File = "Hardware_Manual_guide_node0000.html"
            }
        )
    },
    @{
        Id          = "trm"
        Name        = "A500 A2000 Technical Reference Manual"
        Folder      = "A500 A2000 Technical Reference Manual"
        Type        = "SingleFile"
        TargetFile  = "Commodore_Amiga_A500-A2000_Technical_Reference_Manual.pdf"
        MinSize     = 10000000
        Description = "Commodore-Amiga OEM Manual (1987) with schematics and expansion architecture"
        Mirrors     = @(
            @{
                Name = "Internet Archive (1987 OEM PDF, Primary Scan)"
                Url  = "https://archive.org/download/CommodoreAmigaA500A2000TechnicalReferenceManual/Commodore%20Amiga%20A500-A2000%20Technical%20Reference%20Manual.pdf"
            },
            @{
                Name = "Internet Archive (Alternate Clean Scan PDF)"
                Url  = "https://archive.org/download/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf"
            },
            @{
                Name = "Internet Archive (Searchable OCR Text PDF)"
                Url  = "https://archive.org/download/CommodoreAmigaA500A2000TechnicalReferenceManual/Commodore%20Amiga%20A500-A2000%20Technical%20Reference%20Manual_text.pdf"
            }
        )
    },
    @{
        Id          = "prm"
        Name        = "68000 Programmer's Reference Manual"
        Folder      = "68000 Programmer's Reference Manual"
        Type        = "SingleFile"
        TargetFile  = "M68000_Programmers_Reference_Manual_4th_Edition.pdf"
        MinSize     = 15000000
        Description = "Motorola M68000PM/AD Rev 1 (1992) covering 68000 and CPU32 instruction sets"
        Mirrors     = @(
            @{
                Name = "Internet Archive (M68000PM/AD Rev 1 1992 PDF, Complete)"
                Url  = "https://archive.org/download/M68000_16_32-Bit_Microprocessor_Programmers_Reference_Manual_4th_Edition/M68000_16_32-Bit_Microprocessor_Programmers_Reference_Manual_4th_Edition.pdf"
            },
            @{
                Name = "Internet Archive (Motorola Family PRM Alternate Scan)"
                Url  = "https://archive.org/download/m68000familyprog0000unse/m68000familyprog0000unse.pdf"
            },
            @{
                Name = "Internet Archive (Searchable OCR Text PDF)"
                Url  = "https://archive.org/download/M68000_16_32-Bit_Microprocessor_Programmers_Reference_Manual_4th_Edition/M68000_16_32-Bit_Microprocessor_Programmers_Reference_Manual_4th_Edition_text.pdf"
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
                Name = "Internet Archive / Bitsavers (Rev 8 1993 PDF)"
                Url  = "https://archive.org/download/bitsavers_motorola68MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf"
            },
            @{
                Name = "Internet Archive (Motorola Microprocessors UM Alternate Scan)"
                Url  = "https://archive.org/download/m6800081632bitmi0000unse_l8l8/m6800081632bitmi0000unse_l8l8.pdf"
            },
            @{
                Name = "Internet Archive (Searchable OCR Text PDF)"
                Url  = "https://archive.org/download/bitsavers_motorola68MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993_text.pdf"
            }
        )
    },
    @{
        Id          = "prefetch"
        Name        = "Instruction Prefetch on the Motorola 68000 Processor"
        Folder      = "Instruction Prefetch on the Motorola 68000 Processor"
        Type        = "SingleFile"
        TargetFile  = "68kPrefetch.html"
        MinSize     = 20000
        Description = "Jorge Cwik's authoritative microarchitectural prefetch queue study (v1.3, 2005)"
        Mirrors     = @(
            @{
                Name = "Pasti Project (Original Live Web)"
                Url  = "http://pasti.fxatari.com/68kdocs/68kPrefetch.html"
            },
            @{
                Name = "Wayback Machine (2021 Snapshot)"
                Url  = "https://web.archive.org/web/20210211153835id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html"
            },
            @{
                Name = "Wayback Machine (2019 Snapshot)"
                Url  = "https://web.archive.org/web/20190317072535id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html"
            }
        )
    },
    @{
        Id          = "undocumented"
        Name        = "Undocumented features of OCS, ECS and AGA chipsets"
        Folder      = "Undocumented features of OCS, ECS and AGA chipsets"
        Type        = "Crawl"
        Description = "Kuba Winnicki's 16-page publication on Copper, Sprites, DMA, UHRES (2002)"
        SubPages    = @(
            "index.html",
            "Copper.html",
            "Sprite_Hardware.html",
            "Freeing_the_DMA.html",
            "More_sprites_in_one_line.html",
            "Disappearing_sprites.html",
            "UHRES_Display.html",
            "Speed_Up_Tricks.html",
            "Faster_Chipmem_bus_in_PAL_mode.html",
            "Other_Amiga_Native_Hardware.html",
            "CD32_Controller.html",
            "Battery_Backed_Clock.html",
            "Desaturation_Control_Bit.html",
            "Video_timings.html",
            "Links.html",
            "Last_Words.html",
            "What_is_this_all_about.html"
        )
        Mirrors     = @(
            @{
                Name    = "Achtung! Amiga (Original Live Web)"
                BaseUrl = "https://www.winnicki.net/amiga/achtung/"
            },
            @{
                Name    = "Wayback Machine (2022 Snapshot)"
                BaseUrl = "https://web.archive.org/web/20220330190533id_/https://www.winnicki.net/amiga/achtung/"
            },
            @{
                Name    = "Wayback Machine (2016 Snapshot)"
                BaseUrl = "https://web.archive.org/web/20160410052327id_/http://www.winnicki.net/amiga/achtung/"
            }
        )
    },
    @{
        Id          = "gurubook"
        Name        = "Amiga Guru Book"
        Folder      = "Amiga Guru Book"
        Type        = "SingleFile"
        TargetFile  = "The_Amiga_Guru_Book.pdf"
        MinSize     = 100000000
        Description = "Ralph Babel's 2nd Edition Abridged (1993) Amiga system reference"
        Mirrors     = @(
            @{
                Name = "vdoc.pub Community Scan (740-Page PDF)"
                Url  = "https://vdoc.pub/documents/the-amiga-guru-book-3g783k1i91t0"
            },
            @{
                Name = "Aminet Documentation & Errata Index (LHA)"
                Url  = "http://aminet.net/docs/misc/gurubook-info.lha"
                File = "gurubook-info.lha"
            }
        )
    }
)

# -----------------------------------------------------------------------------
# Helper Functions
# -----------------------------------------------------------------------------

function Show-CatalogList {
    Write-Host ""
    Write-Host "Amiga Reference Documentation Catalog & Mirror Matrix" -ForegroundColor Cyan
    Write-Host "=====================================================" -ForegroundColor Cyan
    Write-Host ""
    foreach ($item in $Catalog) {
        Write-Host "[$($item.Id)] $($item.Name)" -ForegroundColor Green
        Write-Host "    Description : $($item.Description)" -ForegroundColor White
        Write-Host "    Directory   : temp\$($item.Folder)\" -ForegroundColor DarkGray
        Write-Host "    Mirrors ($($item.Mirrors.Count) configured):" -ForegroundColor Yellow
        $idx = 1
        foreach ($m in $item.Mirrors) {
            $mUrl = if ($m.Url) { $m.Url } else { $m.BaseUrl }
            Write-Host "      $idx. $($m.Name)" -ForegroundColor White
            Write-Host "         $mUrl" -ForegroundColor DarkGray
            $idx++
        }
        Write-Host ""
    }
}

function Ensure-StagingReadme {
    param([string]$TempDir)
    if (-not (Test-Path $TempDir)) {
        New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
    }
    $ReadmePath = Join-Path $TempDir "README.md"
    if (-not (Test-Path $ReadmePath)) {
        $Content = @'
# Temporary External Reference Staging Directory

This directory contains raw, unprocessed external reference materials (PDF scans, HTML crawls, and archives) downloaded by `tools/bootstrap_reference.ps1` (or `tools/bootstrap.ps1 -Ref`).

## Operational Guidelines
- **Safe to Delete:** You can safely delete this directory or any subfolder at any time. It has zero impact on compiling, testing, or running the emulator.
- **Git Visibility:** This directory is intentionally **NOT** listed in `.gitignore`. When files are downloaded, it appears in `git status` as untracked files to ensure developers have visual confirmation of temporary downloaded materials.
- **Processing:** Converted markdown specifications live in the parent `Obsidian/Amiga/Reference/` directory and are tracked in Git.
'@
        Set-Content -Path $ReadmePath -Value $Content -Encoding UTF8
    }
}

function Download-SingleFile {
    param(
        [hashtable]$Item,
        [string]$TargetDir,
        [bool]$ForceDownload
    )

    $TargetFileName = $Item.TargetFile
    $DestinationPath = Join-Path $TargetDir $TargetFileName

    if (-not $ForceDownload -and (Test-Path $DestinationPath)) {
        $CurrentSize = (Get-Item $DestinationPath).Length
        if ($CurrentSize -ge $Item.MinSize) {
            Write-Host "  [SKIP] Already present: $TargetFileName ($([math]::Round($CurrentSize / 1MB, 2)) MB)" -ForegroundColor DarkGray
            return $true
        }
    }

    $MirrorIndex = 1
    $TotalMirrors = $Item.Mirrors.Count

    foreach ($mirror in $Item.Mirrors) {
        $SourceUrl = $mirror.Url
        $ActualFileName = if ($mirror.File) { $mirror.File } else { $TargetFileName }
        $ActualDestPath = Join-Path $TargetDir $ActualFileName

        Write-Host "  Trying mirror [$MirrorIndex/$TotalMirrors]: $($mirror.Name)..." -ForegroundColor Cyan
        Write-Host "    $SourceUrl" -ForegroundColor DarkGray

        try {
            $webRequest = [System.Net.HttpWebRequest]::Create($SourceUrl)
            $webRequest.Method = "GET"
            $webRequest.Timeout = 25000
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

            # Verify downloaded size if minimum expected size is defined
            $MinExpected = if ($mirror.MinSize) { $mirror.MinSize } elseif ($mirror.File) { 1000 } else { $Item.MinSize }
            if ($totalBytes -ge $MinExpected) {
                Write-Host "  [OK] Downloaded successfully: $ActualFileName ($([math]::Round($totalBytes / 1MB, 2)) MB)" -ForegroundColor Green
                return $true
            } else {
                Write-Warning "  Downloaded file is smaller than expected ($totalBytes bytes < $MinExpected bytes). Trying next mirror..."
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

    # All mirrors failed
    Write-Error "ERROR: All $TotalMirrors configured mirror sources for '$($Item.Name)' failed. Please verify internet connection or manually place the file in: $TargetDir"
    return $false
}

function Download-CrawlItem {
    param(
        [hashtable]$Item,
        [string]$TargetDir,
        [bool]$ForceDownload
    )

    $TotalPages = $Item.SubPages.Count
    $MirrorIndex = 1
    $TotalMirrors = $Item.Mirrors.Count

    foreach ($mirror in $Item.Mirrors) {
        $BaseUrl = $mirror.BaseUrl
        Write-Host "  Attempting crawl from mirror [$MirrorIndex/$TotalMirrors]: $($mirror.Name)..." -ForegroundColor Cyan
        Write-Host "    Base URL: $BaseUrl" -ForegroundColor DarkGray

        $SuccessCount = 0
        $FailedPages = @()

        foreach ($subpage in $Item.SubPages) {
            $PageDest = Join-Path $TargetDir $subpage
            $PageDir = Split-Path -Parent $PageDest
            if (-not (Test-Path $PageDir)) {
                New-Item -ItemType Directory -Path $PageDir -Force | Out-Null
            }

            if (-not $ForceDownload -and (Test-Path $PageDest) -and ((Get-Item $PageDest).Length -gt 200)) {
                $SuccessCount++
                continue
            }

            $PageUrl = $BaseUrl.TrimEnd('/') + '/' + $subpage

            try {
                $webRequest = [System.Net.HttpWebRequest]::Create($PageUrl)
                $webRequest.Method = "GET"
                $webRequest.Timeout = 15000
                $webRequest.UserAgent = $DefaultUserAgent
                $webRequest.AllowAutoRedirect = $true

                $webResponse = $webRequest.GetResponse()
                $responseStream = $webResponse.GetResponseStream()
                $fileStream = [System.IO.File]::Create($PageDest)

                $buffer = New-Object byte[] 16384
                $bytesRead = 0

                while (($bytesRead = $responseStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                    $fileStream.Write($buffer, 0, $bytesRead)
                }

                $fileStream.Flush()
                $fileStream.Close()
                $fileStream.Dispose()
                $responseStream.Close()
                $responseStream.Dispose()
                $webResponse.Close()
                $webResponse.Dispose()

                if ((Get-Item $PageDest).Length -gt 100) {
                    $SuccessCount++
                } else {
                    $FailedPages += $subpage
                }
            }
            catch {
                $FailedPages += $subpage
            }
        }

        if ($SuccessCount -eq $TotalPages) {
            Write-Host "  [OK] Successfully crawled all $TotalPages pages from $($mirror.Name)." -ForegroundColor Green
            return $true
        } else {
            Write-Warning "  Mirror crawled $SuccessCount/$TotalPages pages. $($FailedPages.Count) pages failed. Trying next mirror..."
        }

        $MirrorIndex++
    }

    # All mirrors failed
    Write-Error "ERROR: All $TotalMirrors crawl mirror sources for '$($Item.Name)' failed. Please verify internet connection or check base URLs."
    return $false
}

# -----------------------------------------------------------------------------
# Main Execution Logic
# -----------------------------------------------------------------------------

if ($List) {
    Show-CatalogList
    exit 0
}

if (-not $All -and -not $Item) {
    Write-Host ""
    Write-Host "Amiga Reference Bootstrapper" -ForegroundColor Cyan
    Write-Host "============================" -ForegroundColor Cyan
    Write-Host "Usage:"
    Write-Host "  .\tools\bootstrap_reference.ps1 -List        : Show all documents and configured mirrors"
    Write-Host "  .\tools\bootstrap_reference.ps1 -Item <name> : Download specific document"
    Write-Host "  .\tools\bootstrap_reference.ps1 -All         : Download all reference materials"
    Write-Host ""
    exit 0
}

Ensure-StagingReadme -TempDir $Destination

$ItemsToProcess = @()
if ($All) {
    $ItemsToProcess = $Catalog
} elseif ($Item) {
    $SearchTerm = $Item.ToLower()
    $Matched = $Catalog | Where-Object {
        $_.Id.ToLower() -eq $SearchTerm -or
        $_.Name.ToLower().Contains($SearchTerm) -or
        $_.Folder.ToLower().Contains($SearchTerm)
    }
    if (-not $Matched) {
        Write-Error "No catalog entry matching '$Item'. Run with -List to inspect available items."
        exit 1
    }
    $ItemsToProcess = @($Matched)
}

Write-Host ""
Write-Host "Bootstrapping External Amiga Reference Materials" -ForegroundColor Cyan
Write-Host "Destination: $Destination" -ForegroundColor DarkGray
Write-Host "Items to process: $($ItemsToProcess.Count)" -ForegroundColor DarkGray
Write-Host ""

$HasErrors = $false
$ProcessedCount = 0

foreach ($entry in $ItemsToProcess) {
    $ProcessedCount++
    Write-Host "[$ProcessedCount/$($ItemsToProcess.Count)] Processing '$($entry.Name)'..." -ForegroundColor Yellow
    $ItemTargetDir = Join-Path $Destination $entry.Folder
    if (-not (Test-Path $ItemTargetDir)) {
        New-Item -ItemType Directory -Path $ItemTargetDir -Force | Out-Null
    }

    $Success = $false
    if ($entry.Type -eq "Crawl") {
        $Success = Download-CrawlItem -Item $entry -TargetDir $ItemTargetDir -ForceDownload $Force
    } else {
        $Success = Download-SingleFile -Item $entry -TargetDir $ItemTargetDir -ForceDownload $Force
    }

    if (-not $Success) {
        $HasErrors = $true
    }
    Write-Host ""
}

if ($HasErrors) {
    Write-Error "One or more reference downloads failed. Review warnings and errors above."
    exit 1
} else {
    Write-Host "All requested reference documentation items provisioned successfully." -ForegroundColor Green
    Write-Host ""
    Write-Host "NOTE: Check 'git status' to inspect untracked downloaded assets in:" -ForegroundColor Yellow
    Write-Host "      Obsidian/Amiga/Reference/temp/" -ForegroundColor White
    exit 0
}
