<#
.SYNOPSIS
    AST-level code knowledge graph bootstrapper for the Amiga 500 emulator.

.DESCRIPTION
    Generates one AST-level code graph in graphify-out/ for project code and the
    selected reference sources under ref_src/. The first run extracts Git-ignored
    references according to .graphifyignore; later runs update that same graph.

    Graphify is optional and only used for AI code structure navigation.

.PARAMETER CheckOnly
    Only verifies if the graphify CLI tool is installed on PATH without running an update.

.EXAMPLE
    .\tools\bootstrap_graphify.ps1
    Builds or updates the project and selected reference code graph in graphify-out/.

.EXAMPLE
    .\tools\bootstrap_graphify.ps1 -CheckOnly
    Verifies graphify tool installation and status.
#>

[CmdletBinding()]
param(
    [switch]$CheckOnly
)

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 Code Knowledge Graph Bootstrapper (Graphify)" -ForegroundColor Cyan
    Write-Host "=====================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap_graphify.ps1                   : Build/update one code graph for project sources and selected ref_src/ repositories"
    Write-Host "  .\tools\bootstrap_graphify.ps1 -CheckOnly        : Verify graphify CLI installation"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -CheckOnly               : Only check graphify availability without updating"
    Write-Host ""
}

Write-Host ""
Write-Host "Bootstrapping Code Knowledge Graph (Graphify AST)..." -ForegroundColor Green
Write-Host "-----------------------------------------------------------------" -ForegroundColor DarkGray

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
    exit 1
}

Write-Host "[OK] graphify CLI found at: $($GraphifyCmd.Source)" -ForegroundColor Green

if ($CheckOnly) {
    exit 0
}

Write-Host "Building/updating the unified code graph in graphify-out/..." -ForegroundColor Cyan
Push-Location $RepoRoot
try {
    $BuildConfigPath = Join-Path $RepoRoot "graphify-out\.graphify_build.json"
    $IncludesIgnoredSources = $false
    if (Test-Path -LiteralPath $BuildConfigPath) {
        try {
            $BuildConfig = Get-Content -LiteralPath $BuildConfigPath -Raw | ConvertFrom-Json
            $IncludesIgnoredSources = $BuildConfig.gitignore -eq $false
        } catch {
            Write-Warning "Graphify build configuration is unreadable; rebuilding the selected source graph."
        }
    }

    if ($IncludesIgnoredSources) {
        graphify update .
    } else {
        graphify extract . --no-gitignore --code-only --force
    }
    $GraphifyExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($GraphifyExitCode -eq 0) {
    Write-Host "Code knowledge graph updated successfully in graphify-out/." -ForegroundColor Green
} else {
    Write-Error "Graphify exited with code $GraphifyExitCode."
    exit $GraphifyExitCode
}
