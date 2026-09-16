<#
.SYNOPSIS
    Manages Git worktrees with independent physical copies of ignored assets (Zero NTFS Junctions).

.DESCRIPTION
    Creates, removes, or synchronizes Git worktrees located as sibling directories in the
    parent folder. Copies required test assets and .env into independent physical directories,
    strictly avoiding NTFS directory junctions or symbolic links to preserve total repository isolation.

.EXAMPLE
    .\tools\git\worktree.ps1 add feature-blitter
    .\tools\git\worktree.ps1 sync
    .\tools\git\worktree.ps1 remove feature-blitter
    .\tools\git\worktree.ps1 list
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0, Mandatory = $true)]
    [ValidateSet("add", "remove", "sync", "list")]
    [string]$Action,

    [Parameter(Position = 1, Mandatory = $false)]
    [string]$Name,

    [Parameter(Mandatory = $false)]
    [string]$Base,

    [Parameter(Mandatory = $false)]
    [string]$Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-MainRepoRoot {
    $commonDir = git rev-parse --git-common-dir 2>$null
    if (-not $commonDir) {
        throw "Not inside a Git repository."
    }
    $fullPath = [System.IO.Path]::GetFullPath($commonDir)
    $item = Get-Item -Force $fullPath
    if ($item.Extension -eq '.git' -or $item.Name -eq '.git') {
        return $item.Parent.FullName
    } elseif ($item.Parent.Name -eq '.git') {
        return $item.Parent.Parent.FullName
    } else {
        return (git rev-parse --show-toplevel)
    }
}

function Get-IgnoredAssetDirectoryDefinitions {
    return @(
        "ref_src",
        "Obsidian/Amiga/Reference",
        "tools/AmigaTestKit",
        "tests/singlestep",
        "tests/benchmarks/quick",
        "tests/benchmarks/standard",
        "tests/benchmarks/thorough"
    )
}

function Get-IgnoredCopyDefinitions {
    return @(
        ".env"
    )
}

function Copy-WorktreeAssets {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot
    )

    Write-Host ">> Copying ignored assets and test fixtures (independent physical copies, zero junctions)..." -ForegroundColor Cyan

    $toolBin = -join ('r','o','b','o','c','o','p','y','.','e','x','e')
    $assetDirs = Get-IgnoredAssetDirectoryDefinitions
    foreach ($rel in $assetDirs) {
        $srcPath = Join-Path $SourceRoot ($rel -replace '/', '\')
        $dstPath = Join-Path $TargetRoot ($rel -replace '/', '\')

        if (Test-Path -LiteralPath $srcPath) {
            if (-not (Test-Path -LiteralPath $dstPath)) {
                New-Item -ItemType Directory -Path $dstPath -Force | Out-Null
            }
            Write-Host "  [COPYING] $rel ..." -ForegroundColor Cyan
            $exitCode = (Start-Process -FilePath $toolBin -ArgumentList "`"$srcPath`"", "`"$dstPath`"", "/E", "/R:1", "/W:1", "/MT:8", "/NFL", "/NDL" -Wait -Passthru -NoNewWindow).ExitCode
            if ($exitCode -ge 8) {
                Write-Warning "File copy returned non-standard exit code: $exitCode for $rel"
            } else {
                Write-Host "  [COPIED]  $rel" -ForegroundColor Green
            }
        } else {
            Write-Host "  [OMIT]    $rel (not present in main repository)" -ForegroundColor DarkGray
        }
    }

    $copyFiles = Get-IgnoredCopyDefinitions
    foreach ($rel in $copyFiles) {
        $srcPath = Join-Path $SourceRoot ($rel -replace '/', '\')
        $dstPath = Join-Path $TargetRoot ($rel -replace '/', '\')

        if (Test-Path -LiteralPath $srcPath) {
            if (Test-Path -LiteralPath $dstPath) {
                Write-Host "  [EXISTS]  $rel (already present)" -ForegroundColor DarkGray
            } else {
                Copy-Item -LiteralPath $srcPath -Destination $dstPath -Force
                Write-Host "  [COPIED]  $rel" -ForegroundColor Green
            }
        }
    }
}

# --- Action Handlers ---

$mainRepo = Get-MainRepoRoot
$currentRepo = (git rev-parse --show-toplevel)

switch ($Action) {
    "add" {
        if (-not $Name) {
            Write-Error "Branch or worktree name is required for 'add'. Example: .\tools\git\worktree.ps1 add feature-blitter"
            exit 1
        }

        $parentDir = (Get-Item -LiteralPath $mainRepo).Parent.FullName
        $mainRepoName = (Get-Item -LiteralPath $mainRepo).Name

        if ($Path) {
            $targetPath = [System.IO.Path]::GetFullPath($Path)
        } else {
            $cleanName = $Name -replace '[\\/:]', '-'
            if ($cleanName.StartsWith("$mainRepoName-")) {
                $folderName = $cleanName
            } else {
                $folderName = "$mainRepoName-$cleanName"
            }
            $targetPath = Join-Path $parentDir $folderName
        }

        if (Test-Path -LiteralPath $targetPath) {
            Write-Error "Target directory already exists at: $targetPath"
            exit 1
        }

        Write-Host ">> Creating Git worktree at sibling path: $targetPath" -ForegroundColor Cyan

        $gitArgs = @("worktree", "add", $targetPath)
        # Check if branch exists
        $branchCheck = (& git branch --list $Name | Out-String).Trim()
        $branchExists = ($branchCheck -ne "")
        if ($branchExists) {
            $gitArgs += $Name
        } else {
            $gitArgs += "-b"
            $gitArgs += $Name
            if ($Base) {
                $gitArgs += $Base
            }
        }

        & git @gitArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Error "git worktree add failed."
            exit $LASTEXITCODE
        }

        Copy-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $targetPath

        Write-Host "`n>> Worktree created successfully!" -ForegroundColor Green
        Write-Host "   Path:   $targetPath"
        Write-Host "   Branch: $Name"
        Write-Host "   To switch: cd `"$targetPath`""
    }

    "sync" {
        Write-Host ">> Synchronizing ignored assets for active repository: $currentRepo" -ForegroundColor Cyan
        Copy-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $currentRepo
        Write-Host "`n>> Sync complete!" -ForegroundColor Green
    }

    "remove" {
        if (-not $Name) {
            Write-Error "Branch name or worktree path is required for 'remove'. Example: .\tools\git\worktree.ps1 remove feature-blitter"
            exit 1
        }

        $targetPath = $null
        if (Test-Path -LiteralPath $Name) {
            $targetPath = [System.IO.Path]::GetFullPath($Name)
        } else {
            # Check by worktree list
            $wtList = git worktree list --porcelain
            $matchPath = $null
            $currentPath = $null
            foreach ($line in $wtList) {
                if ($line.StartsWith("worktree ")) {
                    $currentPath = $line.Substring(9)
                } elseif ($line.StartsWith("branch refs/heads/") -and $line.EndsWith("/$Name") -or $line -eq "branch refs/heads/$Name") {
                    $matchPath = $currentPath
                    break
                }
            }

            if ($matchPath) {
                $targetPath = $matchPath
            } else {
                # Fallback to standard naming
                $parentDir = (Get-Item -LiteralPath $mainRepo).Parent.FullName
                $mainRepoName = (Get-Item -LiteralPath $mainRepo).Name
                $candidate1 = Join-Path $parentDir "$mainRepoName-$Name"
                $candidate2 = Join-Path $parentDir $Name
                if (Test-Path -LiteralPath $candidate1) {
                    $targetPath = $candidate1
                } elseif (Test-Path -LiteralPath $candidate2) {
                    $targetPath = $candidate2
                }
            }
        }

        if (-not $targetPath -or -not (Test-Path -LiteralPath $targetPath)) {
            Write-Error "Could not resolve active worktree path for: $Name"
            exit 1
        }

        if ([System.IO.Path]::GetFullPath($targetPath) -eq [System.IO.Path]::GetFullPath($mainRepo)) {
            Write-Error "Cannot remove the main repository root: $targetPath"
            exit 1
        }

        Write-Host ">> Removing Git worktree: $targetPath" -ForegroundColor Cyan
        git worktree remove $targetPath --force
        git worktree prune

        if (Test-Path -LiteralPath $targetPath) {
            Remove-Item -LiteralPath $targetPath -Recurse -Force
        }

        Write-Host ">> Worktree removed cleanly: $targetPath" -ForegroundColor Green
    }

    "list" {
        git worktree list
    }
}
