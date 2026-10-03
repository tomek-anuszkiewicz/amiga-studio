---
name: pdf-stage2-download
description: >-
  Stage 2: Downloads and provisions source technical PDF manuals and reference documentation from verified multi-mirror archival sources with automated failover and integrity validation.
---

# Stage 2: Download Reference Manuals

This skill automates the acquisition and provisioning of primary source PDF manuals and external reference documents into their respective project directories from verified multi-mirror archival repositories.

## Purpose & Scope
- **Multi-Source Resilience:** Implements automated failover across 2–3 verified mirrors per document (Internet Archive, Bitsavers, Wayback Machine snapshots).
- **Target Manual Directories:** Directly provisions primary source files into their standard project `build/` locations to keep manual roots clean:
  - `Hardware Reference Manual/build/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf`
  - `A500 A2000 Technical Reference Manual/build/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf`
  - `68000 Programmer's Reference Manual/build/M68000PRM.pdf`
  - `68000 User's Manual/build/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf`
- **Integrity & Idempotency:** Validates downloaded files against minimum byte-size thresholds. Existing files matching size expectations are skipped automatically unless `-Force` is specified.
- **Redundancy Mode (`-AllSources`):** Optional mode to download from all configured mirrors for archival verification.

## Input & Output
- **Input (Read-Only):**
  - Remote archival mirror endpoints (Internet Archive, Bitsavers, Wayback Machine) configured in `scripts/download_documentation.ps1`.
- **Output:**
  - Primary source PDF manuals provisioned in target project `build/` directories:
    - `Hardware Reference Manual/build/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf`
    - `A500 A2000 Technical Reference Manual/build/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf`
    - `68000 Programmer's Reference Manual/build/M68000PRM.pdf`
    - `68000 User's Manual/build/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8.pdf`

---

## How to Execute

### 1. Provision All Reference Manuals (Failover Mode)
```powershell
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -All
```

### 2. Display Catalog and Mirror Matrix
```powershell
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -List
```

### 3. Provision Specific Target Manuals
```powershell
# Hardware Reference Manual only
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Hrm

# A500/A2000 Technical Reference Manual only
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Trm

# 68000 Programmer's Reference Manual only
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Prm

# 68000 User's Manual only
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -Um
```

### 4. Force Re-download
```powershell
powershell -ExecutionPolicy Bypass -File .agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1 -All -Force
```
