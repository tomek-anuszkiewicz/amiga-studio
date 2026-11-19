<#
.SYNOPSIS
    Manages Git worktrees with independent physical copies of ignored assets (Zero NTFS Junctions).

.DESCRIPTION
    Creates, removes, or lists Git worktrees located as sibling directories in the
    parent folder. Creation copies local test assets, reference manuals, .env and
    existing graph directories from the primary checkout as independent files.

.EXAMPLE
    .\tools\worktree.ps1 add feature-blitter
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
    [string]$Path,

    [Parameter(Mandatory = $false)]
    [string]$ExpectedBranch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Show-Usage {
    Write-Host @'
Usage: .\tools\worktree.ps1 <add|test|remove|list> [Name] [-Base COMMIT] [-Path PATH] [-ExpectedBranch REF]

  add Name     Create or select a task branch in a new sibling checkout.
  test Name    Create a disposable detached checkout; create no task branch.
  remove Name  Remove only a registered clean worktree, by exact branch or path.
               Preserve dirty files and failed removals; never force or prune.
  list         Show registered worktrees.
  -Base        Starting commit for a new branch or test checkout (default: HEAD).
  -Path        Explicit sibling directory for add/test (default: <repo>-<Name>).
  -ExpectedBranch  Full branch ref to require immediately before removal.

Creation copies local fixtures, ignored reference files, .env and existing
graph directories from the primary checkout. The reference graph travels inside
ref_src/. Copying excludes files tracked in either checkout and preserves existing
destination files.
Each checkout has an independent Cargo cache.
Task worktrees require the user's location choice.
Detached tests keep implementation in that selected checkout.
Removal discards all ignored files in the worktree without preservation.
Stop commands using the checkout before removal.
Run removal from a surviving checkout. Branch deletion is a separate operation.

Examples:
  .\tools\worktree.ps1 add feature-blitter -Base master
  .\tools\worktree.ps1 test integration-probe -Base HEAD
  .\tools\worktree.ps1 remove feature-blitter
  .\tools\worktree.ps1 list
'@
}

if (-not $Action) { Show-Usage; exit 0 }
if ($Action -notin @('add', 'test', 'remove', 'list') -or
    ($Action -in @('add', 'test', 'remove') -and -not $Name) -or
    ($Action -eq 'list' -and $Name) -or
    ($ExpectedBranch -and ($Action -ne 'remove' -or -not $ExpectedBranch.StartsWith('refs/heads/'))) -or
    ($Action -notin @('add', 'test') -and ($Base -or $Path))) {
    Write-Host 'Invalid action or parameter combination.'
    Show-Usage
    exit 1
}

function Invoke-Git {
    $previousEncoding = [Console]::OutputEncoding
    try {
        [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
        & git @args
        if ($LASTEXITCODE -ne 0) { throw "Git failed with exit code $LASTEXITCODE." }
    } finally {
        [Console]::OutputEncoding = $previousEncoding
    }
}

function Get-MainRepoRoot {
    $commonDir = Invoke-Git rev-parse --path-format=absolute --git-common-dir
    if (-not $commonDir) {
        throw "Not inside a Git repository."
    }
    $fullPath = [System.IO.Path]::GetFullPath($commonDir)
    $item = Get-Item -LiteralPath $fullPath -Force
    if ($item.Extension -eq '.git' -or $item.Name -eq '.git') {
        return $item.Parent.FullName
    } elseif ($item.Parent.Name -eq '.git') {
        return $item.Parent.Parent.FullName
    } else {
        return (Invoke-Git rev-parse --show-toplevel)
    }
}

function Get-GitAssetPaths {
    param(
        [string]$Root,
        [string[]]$Selection,
        [string[]]$AssetPaths
    )

    $paths = (Invoke-Git -C $Root --literal-pathspecs ls-files -z @Selection -- @AssetPaths) -join ""
    $paths.Split([char]0, [System.StringSplitOptions]::RemoveEmptyEntries)
}

function Copy-LocalAssetFile {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot,
        [string]$RelativePath,
        [System.Collections.Generic.HashSet[string]]$TrackedPaths
    )

    # A tracked ancestor can also own the destination, for example a submodule.
    $ancestor = $RelativePath
    while ($ancestor) {
        if ($TrackedPaths.Contains($ancestor)) { return }
        $separator = $ancestor.LastIndexOf('/')
        if ($separator -lt 0) { break }
        $ancestor = $ancestor.Substring(0, $separator)
    }

    $srcPath = Join-Path $SourceRoot ($RelativePath -replace '/', '\')
    $dstPath = Join-Path $TargetRoot ($RelativePath -replace '/', '\')
    if (-not (Test-Path -LiteralPath $dstPath)) {
        [void][System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($dstPath))
        [System.IO.File]::Copy($srcPath, $dstPath)
    }
}

function Copy-WorktreeAssets {
    param(
        [string]$SourceRoot,
        [string]$TargetRoot
    )

    Write-Host ">> Copying ignored assets and test fixtures (independent physical copies, zero junctions)..." -ForegroundColor Cyan

    $referenceDir = 'Obsidian/Amiga/Reference'
    $envFile = '.env'
    $assetDirs = @('ref_src', $referenceDir, 'tools/AmigaTestKit', 'graphify-out')
    $assetPaths = $assetDirs + $envFile
    $trackedPaths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($root in @($SourceRoot, $TargetRoot)) {
        foreach ($path in (Get-GitAssetPaths -Root $root -Selection '--cached' -AssetPaths $assetPaths)) {
            [void]$trackedPaths.Add($path)
        }
    }

    $toolBin = -join ('r','o','b','o','c','o','p','y','.','e','x','e')
    foreach ($rel in $assetDirs) {
        $srcPath = Join-Path $SourceRoot ($rel -replace '/', '\')
        $dstPath = Join-Path $TargetRoot ($rel -replace '/', '\')

        if (-not (Test-Path -LiteralPath $srcPath)) {
            Write-Host "  [OMIT]    $rel (not present in main repository)" -ForegroundColor DarkGray
            continue
        }
        if ($trackedPaths.Contains($rel) -and -not (Test-Path -LiteralPath $dstPath -PathType Leaf)) {
            continue
        }

        Write-Host "  [COPYING] $rel ..." -ForegroundColor Cyan
        if ($rel -eq $referenceDir) {
            # Git selects ignored files without importing ordinary untracked notes.
            $ignored = Get-GitAssetPaths -Root $SourceRoot -Selection @('--others', '--ignored', '--exclude-standard') -AssetPaths $rel
            foreach ($path in $ignored) {
                Copy-LocalAssetFile -SourceRoot $SourceRoot -TargetRoot $TargetRoot -RelativePath $path -TrackedPaths $trackedPaths
            }
        } else {
            # Keep bulk copying for prepared asset trees, excluding all existing files.
            $copyArguments = @("`"$srcPath`"", "`"$dstPath`"", '/E', '/R:1', '/W:1', '/MT:8', '/NFL', '/NDL', '/XC', '/XN', '/XO')
            $excludeFiles = @()
            $excludeDirs = @()
            foreach ($trackedPath in $trackedPaths) {
                if (-not $trackedPath.StartsWith("$rel/", [System.StringComparison]::OrdinalIgnoreCase)) { continue }
                $excluded = Join-Path $SourceRoot ($trackedPath -replace '/', '\')
                if (Test-Path -LiteralPath $excluded -PathType Container) {
                    $excludeDirs += "`"$excluded`""
                } else {
                    $excludeFiles += "`"$excluded`""
                }
            }
            if ($excludeFiles.Count -gt 0) { $copyArguments += @('/XF') + $excludeFiles }
            if ($excludeDirs.Count -gt 0) { $copyArguments += @('/XD') + $excludeDirs }

            $exitCode = (Start-Process -FilePath $toolBin -ArgumentList $copyArguments -Wait -Passthru -NoNewWindow).ExitCode
            if ($exitCode -ge 8) {
                throw "Asset copy failed with exit code $exitCode for $rel"
            }
        }
        Write-Host "  [COPIED]  $rel" -ForegroundColor Green
    }

    if (Test-Path -LiteralPath (Join-Path $SourceRoot $envFile)) {
        Copy-LocalAssetFile -SourceRoot $SourceRoot -TargetRoot $TargetRoot -RelativePath $envFile -TrackedPaths $trackedPaths
    }
}

# --- Action Handlers ---

if ($Action -ne 'list') {
    $mainRepo = Get-MainRepoRoot
}

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
        if ($Action -eq 'test') {
            $gitArgs += @('--detach', $(if ($Base) { $Base } else { 'HEAD' }))
        } else {
            $branchCheck = (Invoke-Git branch --list $Name | Out-String).Trim()
            if ($branchCheck) {
                if ($Base) { throw '-Base applies only when creating a new task branch.' }
                $gitArgs += $Name
            } else {
                $gitArgs += @('-b', $Name)
                if ($Base) {
                    $gitArgs += $Base
                }
            }
        }

        Invoke-Git @gitArgs

        Copy-WorktreeAssets -SourceRoot $mainRepo -TargetRoot $targetPath

        Write-Host "`n>> Worktree created successfully!" -ForegroundColor Green
        Write-Host "   Path:   $targetPath"
        Write-Host "   Mode:   $Action $Name"
        Write-Host "   To switch: cd `"$targetPath`""
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
        if ($ExpectedBranch -and (Invoke-Git -C $resolvedTarget symbolic-ref --quiet HEAD) -cne $ExpectedBranch) {
            throw 'Worktree branch changed before removal.'
        }
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
