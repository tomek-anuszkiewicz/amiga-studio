<#
.SYNOPSIS
    Complete environment setup for PDF OCR and Markdown extraction.
.DESCRIPTION
    1. Installs all required Python packages from requirements.txt.
    2. Installs and configures Tesseract OCR engine on Windows.
    3. Verifies that PyMuPDF, pytesseract, Pillow, and tesseract.exe are functional.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/setup_all.ps1
#>

$ErrorActionPreference = "Continue"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " PDF to Markdown - Full Environment Setup" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# Locate python command
$pythonCmd = "python"
if (-not (Get-Command "python" -ErrorAction SilentlyContinue)) {
    if (Get-Command "py" -ErrorAction SilentlyContinue) {
        $pythonCmd = "py"
    } else {
        $userPy = "$env:LOCALAPPDATA\Programs\Python\Python313\python.exe"
        if (Test-Path $userPy) {
            $pythonCmd = $userPy
        }
    }
}

# 1. Install Python packages from requirements.txt
Write-Host "`n[1/3] Installing Python dependencies from requirements.txt..." -ForegroundColor Cyan
$reqPath = Join-Path $PSScriptRoot "requirements.txt"
if (Test-Path $reqPath) {
    Write-Host "[INFO] Using requirements file: $reqPath" -ForegroundColor DarkGray
    & $pythonCmd -m pip install -r $reqPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[OK] Python packages installed successfully." -ForegroundColor Green
    } else {
        Write-Host "[WARN] pip install returned non-zero exit code: $LASTEXITCODE" -ForegroundColor Yellow
    }
} else {
    Write-Host "[ERROR] requirements.txt not found at $reqPath" -ForegroundColor Red
}

# 2. Install Tesseract OCR
Write-Host "`n[2/3] Checking and installing Tesseract OCR engine..." -ForegroundColor Cyan
$installerScript = Join-Path $PSScriptRoot "install_tesseract.ps1"
if (Test-Path $installerScript) {
    & powershell -ExecutionPolicy Bypass -File $installerScript
} else {
    Write-Host "[ERROR] $installerScript not found!" -ForegroundColor Red
}

$checkEnvScript = Join-Path $PSScriptRoot "check_env.py"
if (Test-Path $checkEnvScript) {
    & $pythonCmd $checkEnvScript
} else {
    Write-Host "[ERROR] check_env.py not found at $checkEnvScript" -ForegroundColor Red
}

Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host " Setup complete! You can now run Stage 2 Download:" -ForegroundColor Green
Write-Host '   powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -All' -ForegroundColor White
Write-Host "============================================================" -ForegroundColor Cyan
