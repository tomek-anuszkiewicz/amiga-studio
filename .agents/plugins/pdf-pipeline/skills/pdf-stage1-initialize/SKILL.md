---
name: pdf-stage1-initialize
description: >-
  Stage 1: Initializes the execution environment, verifies Python and dependencies (PyMuPDF, Pillow, pytesseract, etc.), and installs Tesseract OCR on Windows.
---

# Stage 1: Initialize Environment & Dependencies

This skill configures and verifies the execution environment, ensuring that all required Python dependencies and the Tesseract OCR engine are installed and operational before running subsequent pipeline stages.

## Purpose & Scope
- **Python Dependencies:** Installs and validates packages specified in `requirements.txt` (`pymupdf`, `pytesseract`, `Pillow`, `pypdf`, `tqdm`, `ocrmypdf`).
- **Tesseract OCR Engine:** Verifies installation of Tesseract OCR on Windows. If missing, automatically installs it via winget or official binary installer and adds it to PATH.
- **Environment Verification:** Executes a self-check verifying that PyMuPDF, pytesseract, Pillow, and Tesseract CLI are active and functional.
- **Pipeline Gateway:** Prepares the system so Stage 2 (`pdf-stage2-download`) can immediately execute without environment errors.

## Input & Output
- **Input:**
  - Python 3.10+ execution environment and PowerShell CLI.
  - `.agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/requirements.txt` (dependency manifest).
- **Output:**
  - Configured environment with verified Python packages (`fitz`/PyMuPDF, `pytesseract`, `PIL`/Pillow, etc.).
  - Operational Tesseract OCR engine configured on Windows `PATH`.
  - Verified environment readiness for downstream pipeline stages.

---

## How to Execute

### 1. Complete Environment Setup & Verification
Run the master setup script from PowerShell:
```powershell
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/setup_all.ps1
```

### 2. Standalone Tesseract Installation (if needed)
```powershell
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/install_tesseract.ps1
```

### 3. Install Python Packages Directly
```powershell
pip install -r .agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/requirements.txt
```
