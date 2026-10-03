<#
.SYNOPSIS
    Automated installer for Tesseract OCR on Windows.
.DESCRIPTION
    Checks if Tesseract-OCR is installed. If not, installs UB-Mannheim Tesseract-OCR
    via winget (with silent fallback to direct GitHub installer download), updates PATH,
    and validates installation.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/install_tesseract.ps1
#>

$ErrorActionPreference = "Stop"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " Tesseract OCR Windows Installer" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Check if already in PATH
$tesseractInPath = Get-Command "tesseract.exe" -ErrorAction SilentlyContinue
if ($tesseractInPath) {
    Write-Host "[OK] Tesseract is already installed in PATH: $($tesseractInPath.Source)" -ForegroundColor Green
    & tesseract --version
    exit 0
}

# 2. Check standard installation folders
$defaultPaths = @(
    "C:\Program Files\Tesseract-OCR\tesseract.exe",
    "C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
    "$env:LOCALAPPDATA\Programs\Tesseract-OCR\tesseract.exe"
)

$foundExe = $null
foreach ($path in $defaultPaths) {
    if (Test-Path $path) {
        $foundExe = $path
        break
    }
}

if ($foundExe) {
    Write-Host "[OK] Tesseract found at: $foundExe" -ForegroundColor Green
    $installDir = Split-Path -Parent $foundExe
    
    # Add to current session PATH
    if ($env:PATH -notlike "*$installDir*") {
        $env:PATH = "$installDir;$env:PATH"
        Write-Host "[INFO] Added $installDir to current session PATH." -ForegroundColor Yellow
    }
    
    & "$foundExe" --version
    exit 0
}

Write-Host "[INFO] Tesseract not found. Starting installation..." -ForegroundColor Yellow

# 3. Attempt install via winget
$installed = $false
$wingetCmd = Get-Command "winget.exe" -ErrorAction SilentlyContinue

if ($wingetCmd) {
    Write-Host "[1/2] Attempting install via winget (UB-Mannheim.TesseractOCR)..." -ForegroundColor Cyan
    try {
        winget install --id UB-Mannheim.TesseractOCR --accept-source-agreements --accept-package-agreements
        $installed = $true
    } catch {
        Write-Host "[WARN] winget install encountered an issue: $_" -ForegroundColor DarkYellow
    }
}

# 4. Fallback: Direct download from GitHub releases if winget didn't succeed
if (-not $installed -or -not (Test-Path "C:\Program Files\Tesseract-OCR\tesseract.exe")) {
    Write-Host "[2/2] Downloading official UB-Mannheim installer from GitHub..." -ForegroundColor Cyan
    $installerUrl = "https://github.com/UB-Mannheim/tesseract/releases/download/v5.4.0.20240606/tesseract-ocr-w64-setup-5.4.0.20240606.exe"
    $tempInstaller = Join-Path $env:TEMP "tesseract-ocr-setup.exe"

    Write-Host "Downloading $installerUrl -> $tempInstaller..."
    Invoke-WebRequest -Uri $installerUrl -OutFile $tempInstaller

    Write-Host "Running installer (please accept UAC if prompted)..." -ForegroundColor Yellow
    Start-Process -FilePath $tempInstaller -ArgumentList "/S" -Wait
    
    if (Test-Path $tempInstaller) {
        Remove-Item $tempInstaller -Force -ErrorAction SilentlyContinue
    }
}

# 5. Verify final installation
$finalPath = "C:\Program Files\Tesseract-OCR\tesseract.exe"
if (Test-Path $finalPath) {
    $installDir = Split-Path -Parent $finalPath
    
    # Update current session PATH
    if ($env:PATH -notlike "*$installDir*") {
        $env:PATH = "$installDir;$env:PATH"
    }

    # Persist in User PATH if not already present
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        try {
            [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
            Write-Host "[OK] Added $installDir to persistent User PATH environment variable." -ForegroundColor Green
        } catch {
            Write-Host "[WARN] Could not update persistent User PATH: $_" -ForegroundColor DarkYellow
        }
    }

    Write-Host "`n[SUCCESS] Tesseract OCR installed successfully!" -ForegroundColor Green
    & "$finalPath" --version
} else {
    Write-Host "`n[ERROR] Tesseract installation could not be verified automatically." -ForegroundColor Red
    Write-Host "Please download and run the installer manually from:" -ForegroundColor Yellow
    Write-Host "https://github.com/UB-Mannheim/tesseract/wiki" -ForegroundColor Cyan
    exit 1
}
