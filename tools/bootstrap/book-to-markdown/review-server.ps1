param(
    [ValidateSet('start', 'status', 'restart', 'stop')]
    [string]$Action,
    [ValidateRange(1, 65535)]
    [int]$Port
)

$ErrorActionPreference = 'Stop'
if (-not $Action) {
    Write-Output @'
Usage: ./tools/bootstrap/book-to-markdown/review-server.ps1 <action> [-Port <number>]

Action  start    Start the Compose service and wait for its local HTTP endpoint.
        status   Report availability, pending rendering and render errors.
        restart  Rebuild and replace the owned service using current source.
        stop     Stop and remove the owned service, including its children.
Port             Local HTTP port, 1-65535. Overrides BOOK_REVIEW_PORT (default 8765).
                 Port 8765 uses Compose project amiga. Other ports isolate each checkout.

Environment:
  BOOK_REVIEW_LIBRARY      Imported library directory (default tools/bootstrap/books).
  BOOK_REVIEW_CONFIG       Service configuration file (default tool config.yaml).
  BOOK_REVIEW_DIAGNOSTICS  Host diagnostics directory (default .agent/tmp/book-review-diagnostics in the primary checkout).
  BOOK_REVIEW_AUTH         Optional private runtime authentication file.

Relative environment paths are resolved from the calling directory. Configuration
and authentication must be outside the writable mounts. Choose a dedicated
diagnostics directory separate from workflow evidence and controller files.
Controller logs and settings stay under the primary checkout's .agent/tmp/book-review. Start
opens an existing library. Rendering begins when a book is opened.

Examples:
  ./tools/bootstrap/book-to-markdown/review-server.ps1 start
  ./tools/bootstrap/book-to-markdown/review-server.ps1 status -Port 8765
  ./tools/bootstrap/book-to-markdown/review-server.ps1 restart
  ./tools/bootstrap/book-to-markdown/review-server.ps1 stop
'@
    exit 0
}

$defaultPort = 8765
if (-not $PSBoundParameters.ContainsKey('Port')) {
    $Port = $defaultPort
    if ($env:BOOK_REVIEW_PORT -and
        (-not [int]::TryParse($env:BOOK_REVIEW_PORT, [ref]$Port) -or $Port -lt 1 -or $Port -gt 65535)) {
        throw 'BOOK_REVIEW_PORT must be an integer from 1 through 65535.'
    }
}

function Get-SelectedPath([string]$Setting, [string]$Default) {
    $path = [Environment]::GetEnvironmentVariable($Setting)
    if (-not $path) { $path = $Default }
    if (-not $path) { return }
    if (-not [IO.Path]::IsPathRooted($path)) {
        $path = Join-Path (Get-Location).ProviderPath $path
    }
    return [IO.Path]::GetFullPath($path)
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$gitDirectory = & git -C $repoRoot rev-parse --path-format=absolute --git-common-dir
if ($LASTEXITCODE -ne 0 -or -not $gitDirectory) { throw 'Cannot locate the primary Git checkout.' }
$primaryRoot = Split-Path -Parent $gitDirectory
$primaryTemp = Join-Path $primaryRoot '.agent/tmp'
$library = Get-SelectedPath BOOK_REVIEW_LIBRARY (Join-Path $PSScriptRoot '../books')
$config = Get-SelectedPath BOOK_REVIEW_CONFIG (Join-Path $PSScriptRoot 'config.yaml')
$defaultDiagnostics = Join-Path $primaryTemp 'book-review-diagnostics'
$diagnostics = Get-SelectedPath BOOK_REVIEW_DIAGNOSTICS $defaultDiagnostics
$auth = Get-SelectedPath BOOK_REVIEW_AUTH ''

function Test-AtOrBelow([string]$Path, [string]$Root) {
    $rootPrefix = $Root.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    return $Path.TrimEnd('\', '/') -eq $Root.TrimEnd('\', '/') -or
        $Path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)
}

foreach ($root in @($primaryRoot, $repoRoot) | Select-Object -Unique) {
    $sharedTemp = Join-Path $root '.agent/tmp'
    if (Test-AtOrBelow $sharedTemp $diagnostics) {
        throw 'BOOK_REVIEW_DIAGNOSTICS must not contain the shared .agent/tmp directory.'
    }
    foreach ($protected in @('.agent/tmp/book-review', '.agent/tmp/workflow-TA',
            '.agent/tmp/integration', '.agent/task-archives')) {
        $protectedPath = Join-Path $root $protected
        if ((Test-AtOrBelow $protectedPath $diagnostics) -or (Test-AtOrBelow $diagnostics $protectedPath)) {
            throw "BOOK_REVIEW_DIAGNOSTICS must be separate from $protected."
        }
    }
}

# The default service shares a project with capture and RAG. Alternate ports
# keep the per-checkout project for disposable services.
$project = 'amiga'
if ($Port -ne $defaultPort) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $checkoutKey = [BitConverter]::ToString(
            $sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($repoRoot.ToUpperInvariant()))
        ).Replace('-', '').Substring(0, 16).ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
    $project = "book-review-$checkoutKey-$Port"
}
$runtime = Join-Path $primaryTemp "book-review/$Port/$project"
$override = Join-Path $runtime 'override.json'
$composeFile = Join-Path $PSScriptRoot 'compose.yaml'
$compose = @('compose', '--project-name', $project, '--project-directory', $PSScriptRoot,
    '--file', $composeFile, '--file', $override)
$url = "http://127.0.0.1:$Port"

function Invoke-LoggedCommand {
    param([string]$Executable, [string[]]$Arguments, [string]$Operation, [switch]$ExcludeOutput)
    $ErrorActionPreference = 'Continue'
    $output = @(& $Executable @Arguments 2>&1)
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $log = Join-Path $runtime 'controller.log'
    Add-Content -LiteralPath $log -Encoding UTF8 -Value "$Operation (exit $code)"
    if (-not $ExcludeOutput -and $output) {
        Add-Content -LiteralPath $log -Encoding UTF8 -Value $output
    }
    if ($code -ne 0) { throw "$Operation failed (exit $code). See $log." }
    return $output | Where-Object { $_ -is [string] }
}

function Invoke-Docker([string[]]$Arguments, [string]$Operation, [switch]$ExcludeOutput) {
    Invoke-LoggedCommand -Executable $docker -Arguments $Arguments -Operation $Operation -ExcludeOutput:$ExcludeOutput
}

function Confirm-NoNativeServer {
    foreach ($root in @($primaryRoot, $repoRoot) | Select-Object -Unique) {
        $legacy = Join-Path $root '.agent/tmp/book-review'
        foreach ($directory in Get-ChildItem -LiteralPath $legacy -Directory -ErrorAction SilentlyContinue) {
            $recordPath = Join-Path $directory.FullName 'process.json'
            if (-not (Test-Path -LiteralPath $recordPath -PathType Leaf)) { continue }
            $record = Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json
            $legacyId = 0
            $legacyStarted = 0L
            if (-not [int]::TryParse([string]$record.pid, [ref]$legacyId) -or $legacyId -lt 1 -or
                -not [long]::TryParse([string]$record.started, [ref]$legacyStarted)) {
                throw 'The old native process record is invalid. Resolve it before using Compose.'
            }
            $process = Get-Process -Id $legacyId -ErrorAction SilentlyContinue
            if ($process -and $process.StartTime.ToUniversalTime().Ticks -eq $legacyStarted) {
                throw 'A recorded native Book Review process is still active. Stop it with its original launcher.'
            }
        }
    }

    $nativeServers = Get-CimInstance Win32_Process -Filter "Name LIKE 'python%.exe' OR Name = 'py.exe'" |
        Where-Object { $_.CommandLine -match 'book_review\.py(?:["'']|\s)' -and
            $_.CommandLine -match '(?:^|\s)serve(?:\s|$)' }
    if ($nativeServers) {
        throw 'A native Book Review server is still active. Stop it before using Compose.'
    }
}

function Convert-DockerPath([string]$Path) {
    $path = $Path.Replace('\', '/')
    if ($path -match '^/(?:run/desktop/mnt/host|host_mnt)/([a-z])(?:/|$)(.*)$') {
        $path = "$($Matches[1]):/$($Matches[2])"
    }
    return $path.TrimEnd('/')
}

function Get-OwnedContainer {
    $arguments = @('ps', '--all', '--quiet', '--filter', "label=com.docker.compose.project=$project",
        '--filter', 'label=com.docker.compose.service=book-review')
    $ids = @(Invoke-Docker $arguments 'Docker project inspection')
    if (-not $ids) { return }
    if ($ids.Count -ne 1) { throw 'Found multiple Book Review containers. Refusing to control them.' }

    $inspected = @(Invoke-Docker @('inspect', '--type', 'container', $ids[0]) 'Container inspection' -ExcludeOutput)
    $container = ($inspected -join [Environment]::NewLine | ConvertFrom-Json)
    if (@($container).Count -ne 1) { throw 'Expected one inspected Book Review container.' }
    $labels = $container.Config.Labels
    $expectedFiles = "$(Convert-DockerPath $composeFile),$(Convert-DockerPath $override)"
    if ($labels.'com.docker.compose.project' -ne $project -or
        $labels.'com.docker.compose.service' -ne 'book-review' -or
        $labels.'com.docker.compose.oneoff' -ne 'False' -or
        (Convert-DockerPath $labels.'com.docker.compose.project.config_files') -ne $expectedFiles) {
        throw 'Container does not belong to this Book Review Compose definition. Refusing to control it.'
    }

    $bindings = @($container.HostConfig.PortBindings.'8765/tcp')
    if (@($container.HostConfig.PortBindings.PSObject.Properties).Count -ne 1 -or
        $bindings.Count -ne 1 -or $bindings[0].HostIp -ne '127.0.0.1' -or $bindings[0].HostPort -ne "$Port") {
        throw 'Container has an unexpected port binding. Refusing to control it.'
    }

    $mounts = @{
        '/library' = @{ Source = $library; Writable = $false }
        '/app/tools/bootstrap/book-to-markdown/config.yaml' = @{ Source = $config; Writable = $false }
        '/app/.agent/tmp' = @{ Source = $diagnostics; Writable = $true }
    }
    if ($auth) { $mounts['/run/secrets/book-review-auth'] = @{ Source = $auth; Writable = $false } }
    $binds = @($container.Mounts | Where-Object { $_.Type -eq 'bind' })
    foreach ($mount in $binds) {
        $expected = $mounts[$mount.Destination]
        if ($expected) {
            if ((Convert-DockerPath $mount.Source) -ne (Convert-DockerPath $expected.Source) -or
                $mount.RW -ne $expected.Writable) {
                throw 'Container uses different Book Review mounts. Refusing to control it.'
            }
            $mounts.Remove($mount.Destination)
            continue
        }
        # Imported variants are writable, with read-only input mounts inside each.
        $relative = $mount.Destination -creplace '^/library/', ''
        $variant = $relative -cmatch '^[^/]+/[^/]+$'
        $protectedInput = $relative -cmatch '^[^/]+/[^/]+/(source\.pdf|source\.json|source-ocr\.pdf|source-ocr\.json|pdf-to-markdown|pdf)$'
        if ($relative -ceq $mount.Destination -or
            -not ($variant -or $protectedInput) -or $mount.RW -ne $variant -or
            (Convert-DockerPath $mount.Source) -ne "$(Convert-DockerPath $library)/$relative") {
            throw 'Container has an unexpected bind mount. Refusing to control it.'
        }
    }
    if ($mounts.Count -gt 0) { throw 'Container is missing required Book Review mounts. Refusing to control it.' }
    return $container
}

function Confirm-PortFree {
    $listeners = [Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners()
    if ($listeners.Port -contains $Port) {
        throw "Port $Port is already occupied by an unmanaged listener. Stop that listener first."
    }
}

function Get-HttpStatus {
    try {
        $state = Invoke-RestMethod -Uri "$url/api/status" -TimeoutSec 2
    } catch {
        return
    }
    if ($null -ne $state.books -and $null -ne $state.reviews) { return $state }
}

function Write-Status($Container, $State) {
    Write-Output "Running: $url, container=$($Container.Id), books=$($State.books.Count)"
    foreach ($review in $State.reviews) {
        Write-Output "  $($review.id): pending=$($review.pending)"
        if ($review.error) { Write-Output '  Render failed. View details in Book Review.' }
    }
    Write-Output "Logs: $runtime"
}

[IO.Directory]::CreateDirectory($runtime) | Out-Null
$lock = [IO.File]::Open((Join-Path $runtime 'controller.lock'), 'OpenOrCreate', 'Write', 'None')
$previousSettings = @{}
try {
    Confirm-NoNativeServer
    $docker = (Get-Command docker -CommandType Application | Select-Object -First 1).Source
    $container = Get-OwnedContainer
    if (-not $container.State.Running) { Confirm-PortFree }
    if ($Action -eq 'status' -or ($Action -eq 'start' -and $container.State.Running)) {
        if (-not $container.State.Running) {
            Write-Output "Stopped: no running Book Review service on port $Port."
            exit 0
        }
        $state = Get-HttpStatus
        if (-not $state) { throw "Book Review container is running but HTTP status is unavailable. See $runtime." }
        Write-Status $container $state
        exit 0
    }

    # Compose loads only Book Review's definitions. Preserve services from the
    # other files even when the caller enables orphan removal.
    $settings = @{
        COMPOSE_IGNORE_ORPHANS = 'true'
        COMPOSE_REMOVE_ORPHANS = 'false'
        BOOK_REVIEW_LIBRARY = $library
        BOOK_REVIEW_CONFIG = $config
        BOOK_REVIEW_DIAGNOSTICS = $diagnostics
        BOOK_REVIEW_AUTH = $auth
        BOOK_REVIEW_PORT = "$Port"
    }
    foreach ($setting in $settings.Keys) {
        $previousSettings[$setting] = [Environment]::GetEnvironmentVariable($setting)
        [Environment]::SetEnvironmentVariable($setting, $settings[$setting])
    }

    if ($Action -eq 'stop') {
        if ($container) {
            $null = Invoke-Docker ($compose + @('rm', '--stop', '--force', 'book-review')) 'Compose stop'
        }
        Write-Output "Stopped: Book Review on port $Port."
        exit 0
    }

    if (-not $previousSettings['BOOK_REVIEW_DIAGNOSTICS']) {
        [IO.Directory]::CreateDirectory($defaultDiagnostics) | Out-Null
    }
    $python = (Get-Command python -CommandType Application | Select-Object -First 1).Source
    $pending = Join-Path $runtime 'override.pending.json'
    try {
        $null = Invoke-LoggedCommand -Executable $python -Operation 'Book Review mount configuration' -Arguments @(
            (Join-Path $PSScriptRoot 'compose-mounts.py'), '--library', $library, '--output', $pending
        )
        Move-Item -LiteralPath $pending -Destination $override -Force
    } finally {
        if (Test-Path -LiteralPath $pending) { Remove-Item -LiteralPath $pending }
    }

    $up = @('up', '--detach', '--build', '--no-deps')
    if ($Action -eq 'restart') { $up += '--force-recreate' }
    $null = Invoke-Docker ($compose + $up + 'book-review') 'Compose start'
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        $container = Get-OwnedContainer
        if ($container.State.Running) {
            $state = Get-HttpStatus
            if ($state) {
                Write-Status $container $state
                exit 0
            }
        } elseif ($container -and $container.State.Status -eq 'exited') {
            throw "Book Review exited with code $($container.State.ExitCode). Inspect the Compose service logs."
        }
        Start-Sleep -Milliseconds 500
    }
    throw "Book Review did not become ready. Inspect the Compose service logs and $runtime."
} finally {
    foreach ($setting in $previousSettings.Keys) {
        [Environment]::SetEnvironmentVariable($setting, $previousSettings[$setting])
    }
    $lock.Dispose()
}
