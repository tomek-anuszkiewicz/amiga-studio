param(
    [ValidateSet('start', 'stop', 'restart', 'status')]
    [string]$Action = 'status',
    [string]$Book = 'tools/bootstrap/book-to-markdown/books/test-book-example-4567/1cea0b35ee62',
    [ValidateRange(1, 65535)]
    [int]$Port = 8765
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$runtime = Join-Path $repoRoot ".agent/tmp/book-review/$Port"
$recordPath = Join-Path $runtime 'process.json'
$url = "http://127.0.0.1:$Port"

function Get-ManagedProcess {
    if (-not (Test-Path -LiteralPath $recordPath)) { return $null }
    $record = Get-Content -LiteralPath $recordPath -Raw | ConvertFrom-Json
    $candidate = Get-Process -Id $record.pid -ErrorAction SilentlyContinue
    if (-not $candidate -or $candidate.StartTime.ToUniversalTime().Ticks.ToString() -ne $record.started) {
        return $null
    }
    $command = (Get-CimInstance Win32_Process -Filter "ProcessId = $($candidate.Id)").CommandLine
    if (-not $command -or -not $command.Contains($record.script)) {
        throw 'Recorded process does not match the review server; refusing to control it.'
    }
    return $candidate
}

function Show-Status {
    $managed = Get-ManagedProcess
    if (-not $managed) {
        Write-Output "Stopped (no managed server on port $Port)."
        return
    }
    try {
        $state = Invoke-RestMethod "$url/api/status" -TimeoutSec 3
        Write-Output "Running: PID $($managed.Id), $url, pending=$($state.pending)"
        if ($state.error) { Write-Output "Review error: $($state.error)" }
    } catch {
        Write-Output "Process $($managed.Id) exists, but HTTP status is unavailable: $($_.Exception.Message)"
    }
    Write-Output "Logs: $runtime"
}

if ($Action -eq 'status') {
    Show-Status
    exit 0
}

$managed = Get-ManagedProcess
if ($Action -in @('stop', 'restart')) {
    if ($managed) {
        # Include the renderer's Playwright/Edge children, owned by this server.
        & taskkill.exe /PID $managed.Id /T /F | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not stop the managed server process tree.' }
        $managed.WaitForExit()
        Write-Output "Stopped PID $($managed.Id)."
    }
    if (Test-Path -LiteralPath $recordPath) { Remove-Item -LiteralPath $recordPath }
    if ($Action -eq 'stop') { exit 0 }
} elseif ($managed) {
    Show-Status
    exit 0
}

if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) {
    throw "Port $Port is already occupied by an unmanaged process; stop that server first."
}
$bookPath = if ([IO.Path]::IsPathRooted($Book)) { $Book } else { Join-Path $repoRoot $Book }
$bookPath = (Resolve-Path -LiteralPath $bookPath).Path
$scriptPath = Join-Path $PSScriptRoot 'book_review.py'
$python = (Get-Command python -CommandType Application | Select-Object -First 1).Source
New-Item -ItemType Directory -Path $runtime -Force | Out-Null
$launched = Start-Process -FilePath $python -ArgumentList @('-u', "`"$scriptPath`"", 'serve', '--book', "`"$bookPath`"", '--port', $Port) -WorkingDirectory $repoRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $runtime 'stdout.log') -RedirectStandardError (Join-Path $runtime 'stderr.log') -PassThru
@{ pid = $launched.Id; started = $launched.StartTime.ToUniversalTime().Ticks.ToString(); script = $scriptPath; book = $bookPath } | ConvertTo-Json | Set-Content -LiteralPath $recordPath -Encoding UTF8
for ($attempt = 0; $attempt -lt 30; $attempt++) {
    if ($launched.HasExited) {
        throw "Server exited with code $($launched.ExitCode). Inspect $runtime/stderr.log."
    }
    try {
        $null = Invoke-RestMethod "$url/api/status" -TimeoutSec 1
        Show-Status
        exit 0
    } catch {
        Start-Sleep -Milliseconds 500
    }
}
throw "Server did not become ready. Inspect logs in $runtime; use status or stop to control the process."
