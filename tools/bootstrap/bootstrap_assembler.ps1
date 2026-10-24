<#
.SYNOPSIS
    Provision the pinned M68000 assembler used by hand-written CPU benchmarks.

.DESCRIPTION
    Downloads the Attackwave m68k-asm release pinned in assembler-manifest.json,
    verifies the release archive and executable SHA-256 hashes, and installs
    the executable and MIT license under tools/m68k-asm/<release>/<platform>/.
    Running the script again verifies
    the existing executable and downloads only if it is missing or differs.
    Downloaded archives and extraction files are disposable under .agent/tmp/.

.EXAMPLE
    .\tools\bootstrap\bootstrap_assembler.ps1
    Provision the assembler for the current x86-64 host.
#>

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ManifestPath = Join-Path $PSScriptRoot 'assembler-manifest.json'
$Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$Release = $Manifest.release
$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($Architecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "Unsupported assembler host architecture: $Architecture (x86-64 required)"
}

if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    $Platform = 'x86_64-pc-windows-msvc'
} elseif ($IsLinux) {
    $Platform = 'x86_64-unknown-linux-gnu'
} elseif ($IsMacOS) {
    $Platform = 'x86_64-apple-darwin'
} else {
    throw 'Unsupported assembler host operating system'
}

$PlatformManifest = $Manifest.platforms.PSObject.Properties[$Platform].Value
if (-not $Release -or -not $PlatformManifest) {
    throw "Assembler manifest is missing a release or platform: $Platform"
}
$AssetStem = "m68k-$Release-"
$ArchiveExtension = $PlatformManifest.archive_extension
$ExecutableName = $PlatformManifest.executable_name
$ArchiveHash = $PlatformManifest.archive_sha256
$ExecutableHash = $PlatformManifest.executable_sha256
if (-not $ArchiveExtension -or -not $ExecutableName -or
    -not $ArchiveHash -or -not $ExecutableHash -or
    $ArchiveHash.Length -ne 64 -or $ExecutableHash.Length -ne 64) {
    throw "Assembler manifest contains invalid platform data: $Platform"
}

$InstallDir = Join-Path $RepoRoot "tools/m68k-asm/$Release/$Platform"
$InstalledExecutable = Join-Path $InstallDir $ExecutableName
$InstalledLicense = Join-Path $InstallDir 'LICENSE'
if ((Test-Path -LiteralPath $InstalledExecutable) -and (Test-Path -LiteralPath $InstalledLicense)) {
    $CurrentHash = (Get-FileHash -LiteralPath $InstalledExecutable -Algorithm SHA256).Hash
    if ($CurrentHash -ieq $ExecutableHash) {
        Write-Host "Assembler ready: $InstalledExecutable"
        exit 0
    }
}

$ArchiveName = "$AssetStem$Platform$ArchiveExtension"
$Url = "https://github.com/Attackwave/m68k/releases/download/$Release/$ArchiveName"
$WorkDir = Join-Path $RepoRoot ".agent/tmp/bootstrap_assembler/$([guid]::NewGuid().ToString('N'))"
$ArchivePath = Join-Path $WorkDir $ArchiveName
$ExtractDir = Join-Path $WorkDir 'extract'
New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
try {
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath
    $DownloadedHash = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash
    if ($DownloadedHash -ine $ArchiveHash) {
        throw "Assembler archive SHA-256 mismatch: $ArchiveName"
    }

    if ($ArchiveExtension -eq '.zip') {
        Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDir
    } else {
        & tar -xzf $ArchivePath -C $ExtractDir
        if ($LASTEXITCODE -ne 0) { throw "Assembler archive extraction failed: $ArchiveName" }
    }

    $ExtractedDir = Join-Path $ExtractDir "$AssetStem$Platform"
    $ExtractedExecutable = Join-Path $ExtractedDir $ExecutableName
    $ExtractedLicense = Join-Path $ExtractedDir 'LICENSE'
    if (-not (Test-Path -LiteralPath $ExtractedExecutable) -or -not (Test-Path -LiteralPath $ExtractedLicense)) {
        throw "Assembler archive is incomplete: $ArchiveName"
    }
    $ExtractedHash = (Get-FileHash -LiteralPath $ExtractedExecutable -Algorithm SHA256).Hash
    if ($ExtractedHash -ine $ExecutableHash) {
        throw "Assembler executable SHA-256 mismatch: $ArchiveName"
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -LiteralPath $ExtractedExecutable -Destination $InstalledExecutable -Force
    Copy-Item -LiteralPath $ExtractedLicense -Destination $InstalledLicense -Force
    if ($ArchiveExtension -ne '.zip') {
        & chmod +x $InstalledExecutable
        if ($LASTEXITCODE -ne 0) { throw "Could not mark assembler executable: $InstalledExecutable" }
    }
    Write-Host "Assembler ready: $InstalledExecutable"
} finally {
    $ResolvedRepo = [IO.Path]::GetFullPath($RepoRoot).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $ResolvedWork = [IO.Path]::GetFullPath($WorkDir)
    $ExpectedPrefix = $ResolvedRepo + [IO.Path]::DirectorySeparatorChar + '.agent' + [IO.Path]::DirectorySeparatorChar + 'tmp' + [IO.Path]::DirectorySeparatorChar
    if ($ResolvedWork.StartsWith($ExpectedPrefix, [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $WorkDir)) {
        Remove-Item -LiteralPath $WorkDir -Recurse -Force
    }
}
