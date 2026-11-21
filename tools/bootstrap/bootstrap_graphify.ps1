<#
.SYNOPSIS
    Build or refresh the independent project and reference Graphify datasets.
.PARAMETER CheckOnly
    Check Graphify availability without changing either graph.
#>
[CmdletBinding()]
param([switch]$CheckOnly)

Write-Host @'
Usage: .\tools\bootstrap\bootstrap_graphify.ps1 [-CheckOnly]

Default: create or refresh both project and reference code graphs.
         Project document semantics use TA-Graphify-extract-semantics separately.
-CheckOnly: check Graphify availability; change neither graph.

Examples:
  .\tools\bootstrap\bootstrap_graphify.ps1
  .\tools\bootstrap\bootstrap_graphify.ps1 -CheckOnly
'@

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$GraphifyCmd = Get-Command graphify -ErrorAction SilentlyContinue
if (-not $GraphifyCmd) {
    Write-Error "Graphify is missing. Install with: uv tool install graphifyy==0.9.74. Preserve the repository-owned TA graph skills."
    exit 1
}
Write-Host "[OK] Graphify is available."
if ($CheckOnly) { exit 0 }

$Result = 0
foreach ($Graph in @('project', 'reference')) {
    Write-Host "Building/updating $Graph graph..."
    & python (Join-Path $RepoRoot 'tools/graphify/refresh.py') --graph $Graph --root $RepoRoot
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "$Graph graph failed with exit code $LASTEXITCODE."
        $Result = 1
    } else {
        Write-Host "[OK] $Graph code refreshed. Project semantics require TA-Graphify-extract-semantics."
    }
}
exit $Result
