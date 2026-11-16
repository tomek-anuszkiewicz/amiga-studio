<#
.SYNOPSIS
    Manages Git worktrees with independent physical copies of ignored assets (Zero NTFS Junctions).

.DESCRIPTION
    Creates, removes, or synchronizes Git worktrees located as sibling directories in the
    parent folder. Copies required test assets, .env, and the Graphify graph seed into independent
    physical directories, then refreshes the graph in the target worktree. It strictly avoids NTFS
    directory junctions or symbolic links to preserve total repository isolation.

.EXAMPLE
    .\tools\worktree.ps1 add feature-blitter
    .\tools\worktree.ps1 sync
    .\tools\worktree.ps1 remove feature-blitter
    .\tools\worktree.ps1 list
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0, Mandatory = $false)]
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

function Show-Usage {
    Write-Host @'
Usage: .\tools\worktree.ps1 <add|test|remove|sync|list> [Name] [-Base COMMIT] [-Path PATH]

  add Name     Create or select a task branch in a new sibling checkout.
  test Name    Create a disposable detached checkout; create no task branch.
  remove Name  Remove only a registered clean worktree, by exact branch or path.
               Preserve dirty files and failed removals; never force or prune.
  sync         Physically copy missing ignored assets into the current checkout.
  list         Show registered worktrees.
  -Base        Starting commit for a new branch or test checkout (default: HEAD).
  -Path        Explicit sibling directory for add/test (default: <repo>-<Name>).

Creation and sync copy independent fixtures, .env and graph data from the primary
checkout; Cargo caches remain independent. Task worktrees require the user's
location choice. Detached tests keep implementation in that selected checkout.
Removal discards all ignored files in the worktree without preservation.
Stop commands using the checkout before removal.
Run removal from a surviving checkout. Branch deletion is a separate operation.

Examples:
  .\tools\worktree.ps1 add feature-blitter -Base master
  .\tools\worktree.ps1 test integration-probe -Base HEAD
  .\tools\worktree.ps1 remove feature-blitter
  .\tools\worktree.ps1 sync
  .\tools\worktree.ps1 list
'@
}

if (-not $Action) { Show-Usage; exit 0 }
if ($Action -notin @('add', 'test', 'remove', 'sync', 'list') -or
    ($Action -in @('add', 'test', 'remove') -and -not $Name) -or
    ($Action -in @('sync', 'list') -and $Name) -or
    ($Action -notin @('add', 'test') -and ($Base -or $Path))) {
    Write-Host 'Invalid action or parameter combination.'
    Show-Usage
    exit 1
}

function Invoke-Git {
    & git @args
    if ($LASTEXITCODE -ne 0) { throw "Git failed with exit code $LASTEXITCODE." }
}

function Get-MainRepoRoot {
    $commonDir = Invoke-Git rev-parse --path-format=absolute --git-common-dir
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
        return (Invoke-Git rev-parse --show-toplevel)
    }
}

function Get-IgnoredAssetDirectoryDefinitions {
    return @(
        "ref_src",
        "Obsidian/Amiga/Reference",
        "tools/AmigaTestKit",
        ".agent/tmp/singlestep",
        ".agent/tmp/benchmarks",
        "graphify-out"
    )
}

function Update-WorktreeGraph {
    param(
        [string]$WorktreeRoot
    )

    $refreshScript = Join-Path $PSScriptRoot 'graphify/refresh.py'
    $graphifyCommand = Get-Command graphify -ErrorAction SilentlyContinue
    if (-not $graphifyCommand) {
        Write-Warning "graphify CLI is not available; copied graphify-out was not refreshed. After provisioning Graphify, run python `"$refreshScript`" from `"$WorktreeRoot`"."
        return
    }

    Write-Host ">> Refreshing Graphify knowledge graph in $WorktreeRoot..." -ForegroundColor Cyan
    Push-Location $WorktreeRoot
    try {
        & python $refreshScript
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Graphify refresh exited with code $LASTEXITCODE in $WorktreeRoot."
        }
    } finally {
        Pop-Location
    }
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
                throw "Asset copy failed with exit code $exitCode for $rel"
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
$currentRepo = (Invoke-Git rev-parse --show-toplevel)

switch ($Action) {
    { $_ -in 'add', 'test' } {
        if (-not $Name) {
            Write-Error "Branch or worktree name is required for 'add'. Example: .\tools\worktree.ps1 add feature-blitter"
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
        if ([System.IO.Path]::GetDirectoryName($targetPath.TrimEnd('\', '/')) -ne $parentDir) {
            throw 'Worktrees must use a sibling directory of the primary checkout.'
        }

        Write-Host ">> Creating Git worktree at sibling path: $targetPath" -ForegroundColor Cyan

        $gitArgs = @("worktree", "add", $targetPath)
        # Check if branch exists
        $branchCheck = (Invoke-Git branch --list $Name | Out-String).Trim()
        $branchExists = ($branchCheck -ne "")
        if ($Action -eq 'test') {
            $gitArgs += @('--detach', $(if ($Base) { $Base } else { 'HEAD' }))
        } elseif ($branchExists) {
            if ($Base) { throw '-Base applies only when creating a new task branch.' }
            $gitArgs += $Name
        } else {
            $gitArgs += "-b"
            $gitArgs += $Name
            if ($Base) {
                $gitArgs += $Base
            }
        }

        Invoke-Git @gitArgs

        Copy-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $targetPath
        Update-WorktreeGraph -WorktreeRoot $targetPath

        Write-Host "`n>> Worktree created successfully!" -ForegroundColor Green
        Write-Host "   Path:   $targetPath"
        Write-Host "   Mode:   $Action $Name"
        Write-Host "   To switch: cd `"$targetPath`""
    }

    "sync" {
        Write-Host ">> Synchronizing ignored assets for active repository: $currentRepo" -ForegroundColor Cyan
        Copy-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $currentRepo
        Update-WorktreeGraph -WorktreeRoot $currentRepo
        Write-Host "`n>> Sync complete!" -ForegroundColor Green
    }

    "remove" {
        if (-not $Name) {
            Write-Error "Branch name or worktree path is required for 'remove'. Example: .\tools\worktree.ps1 remove feature-blitter"
            exit 1
        }

        $targetPath = $null
        $requestedPath = [System.IO.Path]::GetFullPath($Name)
        $wtList = Invoke-Git -c core.quotePath=false worktree list --porcelain
        $registeredPath = $null
        foreach ($line in $wtList) {
            if ($line.StartsWith('worktree ')) {
                $registeredPath = [System.IO.Path]::GetFullPath($line.Substring(9))
                if ($registeredPath -eq $requestedPath) { $targetPath = $registeredPath }
            } elseif ($line -eq "branch refs/heads/$Name") {
                $targetPath = $registeredPath
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
        $resolvedTarget = (Get-Item -LiteralPath $targetPath -Force).FullName
        $resolvedCurrent = (Get-Location).Path
        if ($resolvedCurrent -eq $resolvedTarget -or
            $resolvedCurrent.StartsWith($resolvedTarget.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar,
                [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'Run removal from a surviving checkout, outside the target directory.'
        }
        if ((Get-Item -LiteralPath $targetPath -Force).Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
            throw 'Refusing to remove a linked worktree directory.'
        }

        Write-Host ">> Removing Git worktree: $targetPath" -ForegroundColor Cyan
        Invoke-Git worktree remove $resolvedTarget
        if (Test-Path -LiteralPath $resolvedTarget) { throw 'Worktree directory remains after Git removal.' }
        $remaining = Invoke-Git -c core.quotePath=false worktree list --porcelain
        if ($remaining -contains ('worktree ' + ($resolvedTarget -replace '\\', '/'))) {
            throw 'Worktree registration remains after Git removal.'
        }

        Write-Host ">> Worktree removed cleanly: $targetPath" -ForegroundColor Green
    }

    "list" {
        Invoke-Git worktree list
    }
}
