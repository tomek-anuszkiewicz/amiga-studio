<#
.SYNOPSIS
    AST-level code knowledge graph bootstrapper for the Amiga 500 emulator.

.DESCRIPTION
    Generates and updates the AST-level code knowledge graph in graphify-out/ using
    the Graphify CLI tool. Maps modules, functions, structs, and dependencies across
    the codebase for semantic code queries, call hierarchies, and architectural navigation.

    Graphify is optional and only used for AI code structure navigation.

.PARAMETER CheckOnly
    Only verifies if the graphify CLI tool is installed on PATH without running an update.

.EXAMPLE
    .\tools\bootstrap_graphify.ps1
    Updates the AST code knowledge graph in graphify-out/.

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
    Write-Host "  .\tools\bootstrap_graphify.ps1                   : Update AST code knowledge graph in graphify-out/"
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

Write-Host "Updating code AST knowledge graph in graphify-out/ (graphify update .)..." -ForegroundColor Cyan
Push-Location $RepoRoot
try {
    graphify update .
} finally {
    Pop-Location
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "Code knowledge graph updated successfully in graphify-out/." -ForegroundColor Green
} else {
    Write-Warning "Graphify update exited with code $LASTEXITCODE."
}
