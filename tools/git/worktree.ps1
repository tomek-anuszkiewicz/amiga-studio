<#
.SYNOPSIS
    Manages Git worktrees with automatic NTFS junction linking of large ignored assets.

.DESCRIPTION
    Creates, removes, or synchronizes Git worktrees located as sibling directories in the
    parent folder. Automatically establishes zero-cost NTFS directory junctions for heavy
    ignored test vectors (ref_src, SingleStepTests, AmigaTestKit, reference docs) and copies .env,
    preventing multi-gigabyte disk duplication and broken test suites.

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

function Get-IgnoredJunctionDefinitions {
    return @(
        "ref_src",
        "Obsidian/Amiga/Reference",
        "tools/AmigaTestKit",
        "tests/singlestep",
        "tests/benchmarks/quick",
        "tests/benchmarks/standard",
        "tests/benchmarks/thorough",
        "graphify-out"
    )
}

function Get-IgnoredCopyDefinitions {
    return @(
        ".env"
    )
}

function Link-WorktreeAssets {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot
    )

    Write-Host ">> Linking ignored assets and test fixtures..." -ForegroundColor Cyan

    $junctions = Get-IgnoredJunctionDefinitions
    foreach ($rel in $junctions) {
        $srcPath = Join-Path $SourceRoot ($rel -replace '/', '\')
        $dstPath = Join-Path $TargetRoot ($rel -replace '/', '\')

        if (Test-Path -LiteralPath $srcPath) {
            if (Test-Path -LiteralPath $dstPath) {
                $item = Get-Item -LiteralPath $dstPath -Force
                if ($item.LinkType -eq 'Junction') {
                    Write-Host "  [EXISTS] $rel (already a junction)" -ForegroundColor DarkGray
                    continue
                } else {
                    Write-Host "  [SCAN]   $rel (standard directory, linking children...)" -ForegroundColor Cyan
                    $childItems = Get-ChildItem -LiteralPath $srcPath -Force
                    foreach ($child in $childItems) {
                        $childDst = Join-Path $dstPath $child.Name
                        if (-not (Test-Path -LiteralPath $childDst)) {
                            if ($child.PSIsContainer) {
                                New-Item -ItemType Junction -Path $childDst -Target $child.FullName | Out-Null
                                Write-Host "    [LINKED] $rel/$($child.Name) -> NTFS Junction" -ForegroundColor Green
                            } else {
                                Copy-Item -LiteralPath $child.FullName -Destination $childDst -Force
                                Write-Host "    [COPIED] $rel/$($child.Name)" -ForegroundColor Green
                            }
                        }
                    }
                    continue
                }
            }

            $parentDir = Split-Path -Path $dstPath -Parent
            if (-not (Test-Path -LiteralPath $parentDir)) {
                New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
            }

            New-Item -ItemType Junction -Path $dstPath -Target $srcPath | Out-Null
            Write-Host "  [LINKED] $rel -> NTFS Junction (0 ms, 0 bytes)" -ForegroundColor Green
        } else {
            Write-Host "  [OMIT]   $rel (not present in main repository)" -ForegroundColor DarkGray
        }
    }

    $copyFiles = Get-IgnoredCopyDefinitions
    foreach ($rel in $copyFiles) {
        $srcPath = Join-Path $SourceRoot ($rel -replace '/', '\')
        $dstPath = Join-Path $TargetRoot ($rel -replace '/', '\')

        if (Test-Path -LiteralPath $srcPath) {
            if (Test-Path -LiteralPath $dstPath) {
                Write-Host "  [EXISTS] $rel (already present)" -ForegroundColor DarkGray
            } else {
                Copy-Item -LiteralPath $srcPath -Destination $dstPath -Force
                Write-Host "  [COPIED] $rel" -ForegroundColor Green
            }
        }
    }
}

function Safe-RemoveJunctions {
    param([string]$TargetRoot)

    if (-not (Test-Path -LiteralPath $TargetRoot)) {
        return
    }

    Write-Host ">> Unlinking NTFS junctions before teardown..." -ForegroundColor Cyan
    $items = Get-ChildItem -Path $TargetRoot -Recurse -Force -ErrorAction SilentlyContinue | Where-Object { $_.LinkType -eq 'Junction' }
    foreach ($item in $items) {
        try {
            (Get-Item -LiteralPath $item.FullName -Force).Delete()
            Write-Host "  [UNLINKED] $($item.FullName)" -ForegroundColor DarkGray
        } catch {
            Write-Host "  [WARN] Failed to delete junction: $($item.FullName)" -ForegroundColor Yellow
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

        Link-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $targetPath

        Write-Host "`n>> Worktree created successfully!" -ForegroundColor Green
        Write-Host "   Path:   $targetPath"
        Write-Host "   Branch: $Name"
        Write-Host "   To switch: cd `"$targetPath`""
    }

    "sync" {
        Write-Host ">> Synchronizing ignored assets for active repository: $currentRepo" -ForegroundColor Cyan
        Link-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $currentRepo
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

        Safe-RemoveJunctions -TargetRoot $targetPath

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
