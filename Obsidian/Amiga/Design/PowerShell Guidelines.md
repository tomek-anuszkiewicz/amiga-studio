---
title: "PowerShell Tooling & Parameterization Guidelines"
aliases: ["PowerShell Guidelines", "CLI Scripting Standards", "PowerShell Parameterization Guidelines"]
tags: ["amiga", "design", "powershell", "tooling", "cli", "guidelines", "standards"]
category: "Design"
subsystem: "tooling"
status: "active"
created: 2026-09-17
updated: 2026-09-17
related: ["[Rust Guidelines.md](Rust%20Guidelines.md)", "[Testing Strategy and Quality Assurance.md](Testing%20Strategy%20and%20Quality%20Assurance.md)", "[General Architecture.md](General%20Architecture.md)"]
---

# PowerShell Tooling & Parameterization Guidelines

- **Parent Specifications:** [General Architecture.md](General%20Architecture.md) | [Testing Strategy and Quality Assurance.md](Testing%20Strategy%20and%20Quality%20Assurance.md)
- **Living Tooling Implementations:** [`tools/bootstrap.ps1`](../../../tools/bootstrap.ps1) | [`tools/bootstrap_sources.ps1`](../../../tools/bootstrap_sources.ps1) | [`tools/bootstrap_graphify.ps1`](../../../tools/bootstrap_graphify.ps1) | [`tools/bootstrap_rag.ps1`](../../../tools/bootstrap_rag.ps1) | [`tools/bootstrap_reference.ps1`](../../../tools/bootstrap_reference.ps1) | [`tools/rag/bin/amiga_rag.ps1`](../../../tools/rag/bin/amiga_rag.ps1)
- **Operating Rules:** [`performance-and-readability.md`](../../../.agents/rules/performance-and-readability.md) | [`vault-linking-and-graph-integrity.md`](../../../.agents/rules/vault-linking-and-graph-integrity.md)

> [!NOTE]
> This document defines the engineering standards, conventions, and parameterization guidelines for authoring PowerShell scripts and developer tooling across this repository.

---

## 1. Core Principles: The Three-Layer Tooling Contract

Every user-facing PowerShell tool in `tools/` must adhere to the **Three-Layer Tooling Contract**:

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. COMMENT-BASED HELP (<# ... #>)                           │
│    PowerShell engine introspection (Get-Help, -?, IDE hover)│
│    Mandatory: .SYNOPSIS, .DESCRIPTION, .PARAMETER, .EXAMPLE │
├─────────────────────────────────────────────────────────────┤
│ 2. FORMAL PARAMETER BLOCK (param(...))                      │
│    Engine validation, type safety, aliases, CmdletBinding   │
│    Mandatory: [CmdletBinding()], explicit types, PascalCase │
├─────────────────────────────────────────────────────────────┤
│ 3. INTERACTIVE CONSOLE USAGE (Show-Usage)                   │
│    Direct developer UX when invoked interactively / empty   │
│    Mandatory: Cyan banner, Yellow advisory, aligned options │
└─────────────────────────────────────────────────────────────┘
```

### 1.1 Zero Divergence Between Layers
Every single switch, parameter, and alias must be represented **identically and symmetrically** across all three layers:
- If a parameter exists in `param(...)`, it **must** have a dedicated `.PARAMETER <Name>` block in the comment header.
- If a parameter is documented in `.PARAMETER`, it **must** appear in the console `Show-Usage` table.
- Silently accepting unlisted parameters or documenting non-existent flags is strictly prohibited.

---

## 2. Comment-Based Help Standard (`<# ... #>`)

Every script must start on Line 1 with a complete PowerShell Comment-Based Help block.

### 2.1 Required Tags & Sequence
```powershell
<#
.SYNOPSIS
    [Concise 1-line imperative summary of what the script accomplishes]

.DESCRIPTION
    [Detailed architectural explanation of what the script does, what subsystems]
    [it provisions or validates, prerequisites, and operational guidelines.]

.PARAMETER <ParameterName>
    [Concrete explanation of what this parameter does, its format, defaults,]
    [and any aliases (e.g. "Alias: -AllMirrors"). Repeat for EVERY parameter.]

.EXAMPLE
    .\tools\script.ps1 -Flag
    [Concise 1-line explanation of the workflow executed by this example]

.EXAMPLE
    .\tools\script.ps1 -Param "Value"
    [Explanation of targeted scenario]
#>
```

### 2.2 Dedicated `.PARAMETER` Mandate
- **No Free-Form Omission:** Never bundle parameter descriptions exclusively into a bullet list inside `.DESCRIPTION`. PowerShell's native `Get-Help .\tools\script.ps1 -Parameter <Name>` requires dedicated `.PARAMETER <Name>` tags to function.
- **Documenting Aliases & Defaults:** Always note aliases and default values explicitly:
  ```powershell
  .PARAMETER AllSources
      Downloads from ALL configured mirrors for each document rather than
      stopping after the first successful mirror. Useful for archival redundancy.
      Alias: -AllMirrors.
  ```

---

## 3. Formal Parameter Block (`param(...)`) Standard

Directly below the comment-based help block, define the parameter block using standard PowerShell cmdlet attributes.

### 3.1 Syntax Schema
```powershell
[CmdletBinding()]
param(
    [switch]$PrimaryAction,
    [Alias("AltAction")]
    [switch]$SecondaryAction,
    [string]$TargetName,
    [string]$DestinationPath,
    [Alias("AllMirrors")]
    [switch]$AllSources,
    [switch]$Force,
    [switch]$All
)
```

### 3.2 Engineering Invariants for Parameters
1. **`[CmdletBinding()]` Required:** Ensures standard cmdlet behaviors, common parameters (`-Verbose`, `-Debug`, `-ErrorAction`), and strict parameter binding.
2. **Explicit Strongly-Typed Declarations:** Never use bare untyped parameters (`$Name`). Always declare explicit types:
   - Boolean toggles: `[switch]$Flag`
   - Text arguments: `[string]$Item`
   - Numeric inputs: `[int]$Count`
   - Structured inputs: `[hashtable]$Config`
3. **PascalCase Naming:** Variable names must use standard PascalCase (`$RepoRoot`, `$TargetDir`, `$AllSources`).
4. **Ergonomic Aliases (`[Alias("...")]`):** Provide intuitive shorthand or legacy migration aliases for long options (`[Alias("Graph")][switch]$Graphify`, `[Alias("Doc", "Qdrant")][switch]$Rag`).

---

## 4. Interactive Console Usage Display (`Show-Usage`)

When a developer runs a script without arguments or with invalid input, they must receive an immediate, aesthetically formatted guide.

### 4.1 Standard Visual Hierarchy & Color Coding
The output must use standard PowerShell `Write-Host` color tokens matching repository aesthetic conventions:
- **Title Banner (`Cyan`):** Script title and underlined separator (`Cyan`).
- **Operational Advisory (`Yellow`):** Crucial developer context (e.g., explaining that bootstrapping is optional and not required to compile or run the emulator).
- **Section Headers (`White`):** `Usage:` and `Options:`.
- **Option Table Columns:**
  - Left column: Cyan switch names with fixed padding (25 characters).
  - Right column: White descriptions with embedded DarkGray alias/default notes.

### 4.2 Canonical `Show-Usage` Implementation Pattern
```powershell
function Show-Usage {
    Write-Host ""
    Write-Host "Amiga 500 Emulator Repository Bootstrapper" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "NOTE: Bootstrapping is NOT required to build or run the emulator!" -ForegroundColor Yellow
    Write-Host "      A bare clone compiles and runs the GUI immediately via 'cargo run -p gui'."
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor White
    Write-Host "  .\tools\bootstrap.ps1 -Test                      : Provision hardware test vectors (SingleStepTests, vAmiga)"
    Write-Host "  .\tools\bootstrap.ps1 -Ref                       : Provision external reference materials (PDFs, HTML crawls)"
    Write-Host "  .\tools\bootstrap.ps1 -All                       : Provision all primary tiers (tests -> Graphify -> RAG)"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Test                    : Verify & unpack SingleStepTests 68000 test vectors"
    Write-Host "  -Graphify                : Update AST code knowledge graph (alias: -Graph)"
    Write-Host "  -Rag                     : Index Obsidian docs into Qdrant (aliases: -Doc, -Qdrant)"
    Write-Host "  -Ref                     : Download external reference materials into temp/"
    Write-Host "  -AllSources              : Download from all mirrors for -Ref (alias: -AllMirrors)"
    Write-Host "  -All                     : Run all primary tiers (-Test, -Graphify, -Rag)"
    Write-Host ""
}
```

---

## 5. Ergonomic Sub-Flag Auto-Promotion Pattern

CLI scripts frequently have tiered parameters (e.g. parent switch `-Ref` and child modifier `-AllSources`, or `-Force` implying `-All`).

### 5.1 The Anti-Pattern: Failing on Missing Parent Switches
Forcing a developer to type `.\tools\bootstrap.ps1 -Ref -AllSources` when they already explicitly specified `-AllSources` is brittle and frustrating.

### 5.2 The Solution: Automatic Tier Promotion
Evaluate child parameter presence and auto-promote parent tier flags before the validation check:

```powershell
# Auto-promote parent switch if specific child modifier is passed
if ($AllSources) { 
    $Ref = $true 
}

# Auto-promote -AllSources or -Force to -All in reference bootstrapper
if ($AllSources -or $Force) {
    $All = $true
}

# Validation guard: display usage if no active actions were triggered
if (-not $Rag -and -not $Test -and -not $Graphify -and -not $Ref -and -not $All) {
    Show-Usage
    exit 0
}
```

---

## 6. Verification & Quality Gates for PowerShell Scripts

Before committing any modifications to PowerShell scripts:

1. **Syntax & ScriptBlock Parsing Check:**
   ```powershell
   powershell -Command "[scriptblock]::Create((Get-Content 'tools/script.ps1' -Raw)) | Out-Null; Write-Host 'Syntax OK'"
   ```
2. **Comment-Based Help Parsing:**
   ```powershell
   powershell -Command "Get-Help .\tools\script.ps1 -Full"
   powershell -Command "Get-Help .\tools\script.ps1 -Parameter <ParamName>"
   ```
3. **Interactive Console Usage Output:**
   Run the script with no arguments and verify column alignment and color formatting:
   ```powershell
   powershell -File tools/script.ps1
   ```
4. **Pre-Flight Quality Gate:**
   ```powershell
   python tools/harness/pre_flight.py
   ```

---

## 7. Reference Documentation & Upstream Ground Truth

- **Tooling Implementations:**
  - [`tools/bootstrap.ps1`](../../../tools/bootstrap.ps1): Multi-tier repository bootstrap coordinator.
  - [`tools/bootstrap_sources.ps1`](../../../tools/bootstrap_sources.ps1): External test sources, hardware vectors, and reference emulator bootstrapper.
  - [`tools/bootstrap_graphify.ps1`](../../../tools/bootstrap_graphify.ps1): AST code knowledge graph bootstrapper.
  - [`tools/bootstrap_rag.ps1`](../../../tools/bootstrap_rag.ps1): AI documentation knowledge base and Qdrant vector index bootstrapper.
  - [`tools/bootstrap_reference.ps1`](../../../tools/bootstrap_reference.ps1): Multi-mirror reference documentation fetcher and crawler.
  - [`tools/rag/bin/amiga_rag.ps1`](../../../tools/rag/bin/amiga_rag.ps1): Environment and runner wrapper for the Python RAG vector engine.
- **Upstream Standards:**
  - Microsoft PowerShell Documentation: [About Comment-Based Help](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_comment_based_help).
  - Microsoft PowerShell Documentation: [About Functions Advanced Parameters](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_functions_advanced_parameters).
