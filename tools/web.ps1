[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Action
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Show-Usage {
    Write-Output @'
Build or serve the shared-memory browser application.

Usage: ./tools/web.ps1 <build|serve>

  build  Compile the optimized GUI and worker into .agent/tmp/web-dist.
  serve  Compile optimized code, watch sources and serve with isolation headers.

Requires Trunk and Rust nightly with rust-src and wasm32-unknown-unknown.
Uses nightly only for this invocation; the default toolchain is unchanged.
The shared linear memory uses the wasm32 maximum of 4 GiB; it grows on demand.
Cargo lint warnings fail the build, including workspace manifest warnings.

Examples:
  ./tools/web.ps1 build
  ./tools/web.ps1 serve
'@
}

if (-not $Action) {
    Show-Usage
    exit 0
}
if ($Action -notin @('build', 'serve')) {
    Write-Output "Unknown action: $Action"
    Show-Usage
    exit 1
}

$WebEnvironmentNames = @('RUSTUP_TOOLCHAIN', 'CARGO_UNSTABLE_BUILD_STD', 'CARGO_BUILD_WARNINGS', 'RUSTFLAGS', 'NO_COLOR')
$PreviousWebEnvironment = @{}
foreach ($WebEnvironmentName in $WebEnvironmentNames) {
    $PreviousWebEnvironment[$WebEnvironmentName] = [Environment]::GetEnvironmentVariable($WebEnvironmentName, 'Process')
}
$WebExitCode = 1
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $env:RUSTUP_TOOLCHAIN = 'nightly'
    $env:NO_COLOR = 'true'
    $env:CARGO_UNSTABLE_BUILD_STD = 'std,panic_abort'
    $env:CARGO_BUILD_WARNINGS = 'deny'
    $env:RUSTFLAGS = '-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--max-memory=4294967296 -C link-arg=--import-memory -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base'
    & trunk $Action --release --locked --no-sri
    $WebExitCode = $LASTEXITCODE
}
finally {
    foreach ($WebEnvironmentName in $WebEnvironmentNames) {
        [Environment]::SetEnvironmentVariable($WebEnvironmentName, $PreviousWebEnvironment[$WebEnvironmentName], 'Process')
    }
    Pop-Location
}
exit $WebExitCode
