---
title: "Amiga Reference Documentation Library & Bootstrapper"
aliases: ["Reference Library", "Amiga References", "Reference Bootstrapper"]
tags: ["amiga", "reference", "documentation", "bootstrap", "mirrors"]
category: "Reference"
subsystem: "reference"
status: "active"
created: 2026-09-14
updated: 2026-09-14
related: ["[General Architecture.md](../Design/General%20Architecture.md)", "[Platform Quirks and Invariants Catalog.md](../Design/Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Amiga Reference Documentation Library & Bootstrapper

> [!NOTE]
> This directory contains technical reference specifications, instruction set manuals, hardware chipset documentation, and research articles that serve as technical references for the Amiga 500 emulator.

---

## 1. Reference Documentation Catalog

The repository maintains converted, structured markdown documents for rapid navigation and AI retrieval, backed by external primary sources:

1. **`Hardware Reference Manual`**:
   - Primary reference for Amiga custom chipsets (OCS): Agnus, Denise, Paula, Blitter, Copper, and audio channels.
   - Based strictly on the Addison-Wesley 2nd Edition (1989) typeset on Commodore Amiga 2500/UX (AMIX), covering pure A500 OCS hardware without ECS contamination.
2. **`A500 A2000 Technical Reference Manual`**:
   - Official Commodore-Amiga OEM engineering manual (1987).
   - Covers bus timing, system schematics, Zorro expansion architecture, and bridgeboard signals.
3. **`68000 Programmer's Reference Manual`**:
   - Motorola M68000PM/AD Rev 1 (1992).
   - Authoritative instruction set definitions, effective addressing modes, and condition code calculations.
4. **`68000 User's Manual`**:
   - Motorola M68000UM/AD Rev 8 (1993).
   - Cycle-by-cycle bus timing diagrams, read/write bus state phases, pinouts, and electrical characteristics.
5. **`Instruction Prefetch on the Motorola 68000 Processor`**:
   - Jorge Cwik's authoritative microarchitectural prefetch queue study (Version 1.3, 2005, Pasti Project).
   - Establishes the `IR` (Instruction Register) and `IRC` (Instruction Register Capture) pipeline model.
6. **`Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis`**:
   - Cycle-exact algorithmic analysis of 68000 non-restoring integer division.
   - Based on Jorge Cwik's Pasti C algorithms and verified against physical silicon test vectors.
7. **`Undocumented features of OCS, ECS and AGA chipsets`**:
   - Kuba Winnicki's 16-page publication (*Achtung! Amiga*, 2002).
   - Covers undocumented Copper hazards, sprite demultiplexing, DMA arbitration quirks, and UHRES display modes.

---

## 2. Automated Reference Bootstrapper (`tools/bootstrap_documentation.ps1`)

To download raw, unprocessed source materials (PDF scans, HTML crawls, and archives) without storing large binary files in Git, use the automated bootstrapper:

```powershell
# List available reference items and configured mirrors
.\tools\bootstrap_documentation.ps1 -List

# Download all reference materials (default failover mode: stops after 1st successful mirror)
.\tools\bootstrap_documentation.ps1 -All

# Download from ALL mirrors & sources simultaneously (redundancy & full testing mode)
.\tools\bootstrap_documentation.ps1 -All -AllSources

# Or via the main repository bootstrapper
.\tools\bootstrap.ps1 -Documentation
.\tools\bootstrap.ps1 -Documentation -AllSources
```

---

## 3. Multi-Source Fallback Matrix & Error Resilience

To protect against dead links, server downtime, and rate limits, every reference item is mapped to **2–3 independent, verified online sources** across archival databases, established Amiga documentation portals (`amigadev.elowar.com`), the worldwide Aminet network, original author websites, and the Wayback Machine:

| Document | Primary Mirror (Verified 200 OK) | Secondary Mirror (Verified 200 OK) | Tertiary / Fallback Mirror |
| :--- | :--- | :--- | :--- |
| **`Hardware Reference Manual`** | [Internet Archive (1989 2nd Ed OCS PDF, 405p 600 DPI)](https://archive.org/download/commodore-amiga-hardware-reference-manual-2nd/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf) | [Internet Archive (1985 1st Ed PDF)](https://archive.org/download/Amiga_Hardware_Reference_Manual_1985_Commodore/Amiga_Hardware_Reference_Manual_1985_Commodore.pdf) | Manual local file drop |
| **`A500 A2000 Technical Reference Manual`** | [Internet Archive (1987 OEM Clean Scan PDF, 308p 200 DPI)](https://archive.org/download/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf) | [Internet Archive (1987 OEM Alternate Scan PDF, 309p)](https://archive.org/download/CommodoreAmigaA500A2000TechnicalReferenceManual/Commodore%20Amiga%20A500-A2000%20Technical%20Reference%20Manual.pdf) | Manual local file drop |
| **`68000 Programmer's Reference Manual`** | [Internet Archive (M68000PM/AD Rev 1 Vector PDF, 646p)](https://archive.org/download/M68000PRM/M68000PRM.pdf) | [Bitsavers (M68000PM/AD Rev 1 1992 PDF)](https://archive.org/download/bitsavers_motorola68ogrammersReferenceManual1992_2394181/M68000PM_AD_Rev_1_Programmers_Reference_Manual_1992.pdf) | Manual local file drop |
| **`68000 User's Manual`** | [Internet Archive / Bitsavers (Rev 8 PDF, 601 DPI 216p)](https://archive.org/download/bitsavers_motorola68MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf) | [Internet Archive (Rev 8 Alternate Item)](https://archive.org/download/bitsavers_motorola6868000MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf) | [Internet Archive / Bitsavers (Family Reference 1988, 608p)](https://archive.org/download/bitsavers_motorola68rence1988_23248083/M68000_Family_Reference_1988.pdf) |
| **`Instruction Prefetch`** | [Pasti Project (Original Live Web)](http://pasti.fxatari.com/68kdocs/68kPrefetch.html) | [Wayback Machine (2021 Snapshot)](https://web.archive.org/web/20210211153835id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html) | [Wayback Machine (2019 Snapshot)](https://web.archive.org/web/20190317072535id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html) |
| **`Undocumented features`** | [Achtung! Amiga (Original Live Web)](https://www.winnicki.net/amiga/achtung/) | [Wayback Machine (2022 Snapshot)](https://web.archive.org/web/20220330190533id_/https://www.winnicki.net/amiga/achtung/) | [Wayback Machine (2016 Snapshot)](https://web.archive.org/web/20160410052327id_/http://www.winnicki.net/amiga/achtung/) |

### Automated Failover & Error Reporting Policy
- **Sequential Mirror Traversal:** The bootstrapper attempts Mirror 1 first. If a transient error, HTTP 404/503, connection timeout, or byte-size mismatch occurs, it logs a warning and immediately falls back to Mirror 2, then Mirror 3.
- **Fail-Fast Error Reporting:** If all configured mirrors for an item fail, the bootstrapper writes an explicit error message to standard error identifying the failed item and exits with code `1`. Silent failures are strictly prohibited.

---

## 4. Multi-Page Web Crawling Engine

For multi-page web publications, the bootstrapper incorporates a recursive crawling engine:
- **Kuba Winnicki's *Achtung! Amiga*:** Downloads the root index and all 16 technical subpages (`Copper.html`, `Sprite_Hardware.html`, `Freeing_the_DMA.html`, `More_sprites_in_one_line.html`, `Disappearing_sprites.html`, `UHRES_Display.html`, `Speed_Up_Tricks.html`, `Faster_Chipmem_bus_in_PAL_mode.html`, `Other_Amiga_Native_Hardware.html`, `CD32_Controller.html`, `Battery_Backed_Clock.html`, `Desaturation_Control_Bit.html`, `Video_timings.html`, `Links.html`, `Last_Words.html`, `What_is_this_all_about.html`).
- If the primary live server at `winnicki.net` is unreachable or blocks requests, the crawler automatically switches to the permanent Wayback Machine snapshot mirror.

---

## 5. Staging Directory (`temp/`) Guidelines & Git Visibility

Downloaded raw materials are placed in:
```
Obsidian/Amiga/Reference/temp/<Document_Name>/
```

- **Intentional Git Visibility:** `temp/` is intentionally **NOT** added to `.gitignore`. When files are downloaded, `temp/` appears in `git status` as untracked files. This gives developers immediate visual awareness that temporary raw assets exist locally.
- **Safe to Delete:** Any file or directory inside `temp/` can be deleted at any time without impacting emulator compilation, unit tests, or CI checks.
- **Offline & Manual Drop:** If external network access is restricted or a user possesses a physical copy of a document, raw files can be placed directly into `temp/<Document_Name>/` for manual inspection.
