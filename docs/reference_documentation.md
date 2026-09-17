# Technical Reference Documentation, Bootstrapper & AI RAG Guide

This guide details the technical reference manuals, microarchitectural research papers, automated documentation bootstrapping, and AI RAG (Retrieval-Augmented Generation) vector indexing system for the Amiga 500 emulator.

---

## 1. Reference Library Overview & Status

The repository maintains an authoritative, high-fidelity reference library under `Obsidian/Amiga/Reference/`. 

All primary reference materials are **already converted into structured Markdown specifications and committed directly to the repository**. Developers and AI agents can read, cross-reference, and semantically search these documents immediately without requiring any external downloads.

For developers wishing to inspect the original archival scans, verify raw circuit schematics, or ingest new technical documentation, an **automated bootstrapper** and **re-processing toolchain** are provided:

```text
┌────────────────────────────────────────────────────────────────────────────────┐
│                           REFERENCE DATA LIFECYCLE                             │
├────────────────────────────────────────────────────────────────────────────────┤
│ 1. INGESTED REPOSITORY MARKDOWN (Active & Ready)                               │
│    Location: Obsidian/Amiga/Reference/                                         │
│    Status: ✅ Fully converted, structured chapters, searchable, offline        │
├────────────────────────────────────────────────────────────────────────────────┤
│ 2. RAW ARCHIVAL SOURCES (Optional Download via Bootstrapper)                   │
│    Location: Obsidian/Amiga/Reference/temp/<Document_Name>/                    │
│    Status: 📥 Downloaded via tools/bootstrap/bootstrap_documentation.ps1       │
├────────────────────────────────────────────────────────────────────────────────┤
│ 3. CONVERSION & RE-PROCESSING TOOLCHAIN (Skills)                               │
│    PDF Scans -> Markdown: .agents/skills/pdf-to-markdown/SKILL.md              │
│    Web Crawls -> Markdown: .agents/skills/html-to-markdown/SKILL.md             │
├────────────────────────────────────────────────────────────────────────────────┤
│ 4. SEMANTIC AI VECTOR RAG (Local Qdrant)                                       │
│    Indexer: tools/bootstrap/bootstrap_rag.ps1 (amiga_rag.ps1)                  │
│    Search:  python tools/harness/rag_search.py "<query>"                       │
└────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Reference Documentation Catalog

Every document in the library is cataloged below with its **current repository status**, hardware focus, and raw archival source:

### 1. Hardware Reference Manual (Addison-Wesley 2nd Edition, 1989)
- **Status:** ✅ **Already Ingested & Available** in `Obsidian/Amiga/Reference/Hardware Reference Manual/`
- **Scope:** Primary reference for Amiga custom chipsets (OCS): Agnus (Copper, Blitter), Denise (Bitplanes, Sprites, Color palette), Paula (Audio DMA, Floppy disk controller), interrupt priority routing, and memory map.
- **Provenance:** Based strictly on the Addison-Wesley 2nd Edition (1989) typeset on Commodore Amiga 2500/UX (AMIX), covering pure A500 OCS hardware without ECS contamination.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`hrm`): 405-page, 600 DPI PDF scan (~30 MB) from Internet Archive.

### 2. A500 A2000 Technical Reference Manual (Commodore-Amiga OEM, 1987)
- **Status:** ✅ **Already Ingested & Available** in `Obsidian/Amiga/Reference/A500 A2000 Technical Reference Manual/`
- **Scope:** Official Commodore OEM engineering manual covering bus timing, Gary gate array logic, system motherboard schematics, expansion bus (Zorro), and bridgeboard signals.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`trm`): 308-page, 200 DPI PDF scan (~30 MB) from Internet Archive.

### 3. M68000 Programmer's Reference Manual (Motorola Rev 1, 1992)
- **Status:** ✅ **Already Ingested & Available** in `Obsidian/Amiga/Reference/68000 Programmer's Reference Manual/`
- **Scope:** Authoritative instruction set definitions (68000 core), addressing modes, Condition Code Register (CCR) flag calculations, and execution cycle charts.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`prm`): 646-page vector PDF (~4.5 MB) from Internet Archive and Bitsavers.

### 4. M68000 User's Manual (Motorola Rev 8, 1993)
- **Status:** ✅ **Already Ingested & Available** in `Obsidian/Amiga/Reference/68000 User's Manual/`
- **Scope:** Cycle-by-cycle bus timing diagrams, bus state phases ($S0$–$S7$), read/write cycles, wait states, bus arbitration signals (`BR`, `BG`, `BGACK`), pinouts, and electrical characteristics.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`um`): 216-page, 601 DPI PDF scan (~10 MB) from Internet Archive and Bitsavers.

### 5. Instruction Prefetch on the Motorola 68000 Processor (Jorge Cwik / Pasti, 2005)
- **Status:** ✅ **Already Ingested & Available** as `Obsidian/Amiga/Reference/Instruction Prefetch on the Motorola 68000 Processor.md`
- **Scope:** Authoritative microarchitectural prefetch queue study (Version 1.3). Establishes the 68000 two-stage `IR` (Instruction Register) and `IRC` (Instruction Register Capture) pipeline model and timing interactions.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`prefetch`): Original web article from Pasti Project and Wayback Machine snapshots.

### 6. Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis (Jorge Cwik / Pasti)
- **Status:** ✅ **Already Ingested & Available** as `Obsidian/Amiga/Reference/Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis.md`
- **Scope:** Cycle-exact algorithmic analysis of 68000 non-restoring integer division. Formulates precise cycle calculations verified against physical silicon test vectors.
- **Raw Source Archive:** Synthesized directly from Pasti C verification algorithms.

### 7. Undocumented Features of OCS, ECS and AGA Chipsets (Kuba Winnicki / Achtung! Amiga, 2002)
- **Status:** ✅ **Already Ingested & Available** as `Obsidian/Amiga/Reference/Undocumented features of OCS, ECS and AGA chipsets.md`
- **Scope:** 16-chapter investigation into silicon quirks: Copper hazards, sprite demultiplexing, DMA slot arbitration, UHRES display modes, and video beam timing anomalies.
- **Raw Source Archive:** 📥 **Downloadable via Bootstrap** (`achtung`): 16-page multi-page HTML crawl from `winnicki.net/amiga/achtung/` and Wayback Machine.

---

## 3. Automated Reference Bootstrapper (`tools/bootstrap/bootstrap_documentation.ps1`)

The repository includes an automated bootstrapper for downloading raw archival materials into a dedicated staging directory: `Obsidian/Amiga/Reference/temp/<Document_Name>/`.

### Running the Bootstrapper

Invoke via the central bootstrap coordinator:
```powershell
# Provision raw reference manuals and scans:
.\tools\bootstrap\bootstrap.ps1 -Documentation
```

---

## 4. Processing Raw Documents into Markdown

When new reference manuals or updated editions are retrieved, use specialized agent skills to convert them into repository-grade Markdown:

1. **PDF Scans to Markdown ([`pdf-to-markdown`](../.agents/skills/pdf-to-markdown/SKILL.md)):**
   - Uses PyMuPDF (`fitz`) to split documents into logical chapters.
   - Extracts and crops circuit diagrams, register maps, and waveforms into high-resolution PNG/SVG assets.
   - Stitches multi-page register tables into GitHub-flavored Markdown tables.
   - Generates Git-tracked multimodal sidecar text files (`<image>.txt`) describing timing diagrams for offline AI inspection.

2. **Web Crawls to Markdown ([`html-to-markdown`](../.agents/skills/html-to-markdown/SKILL.md)):**
   - Crawls multi-page HTML hierarchies (e.g. Kuba Winnicki's *Achtung! Amiga*).
   - Strips legacy table formatting, inline styling, and obsolete navigational chrome.
   - Normalizes cross-chapter hyperlinks into Obsidian internal vault links (`[[Chapter#Section]]`).

---

## 5. AI RAG Knowledge Base & Vector Indexing

All ingested reference specifications and architecture design documents are indexed into a high-speed local vector database (Qdrant) to power autonomous AI agent pair-programming and developer CLI queries.

### Architecture
- **Vector Database:** Local Qdrant instance running on `http://localhost:6333`.
- **Collection:** `amiga` (unified collection with source tags: `amiga` for hardware reference manuals, `obsidian` for architectural design notes).
- **Embedding Model:** Local FastEmbed (`BAAI/bge-small-en-v1.5`), 100% offline with zero cloud API keys required.
- **Incremental Cache:** `amiga_rag_cache.json` tracks SHA-256 hashes of individual files, reindexing modified documents in $< 1$ second while skipping unchanged files.

### Provisioning & Reindexing RAG

Provision via the coordinator:
```powershell
# Set up Qdrant Docker container and index all documentation:
.\tools\bootstrap\bootstrap.ps1 -Rag
```

Or trigger incremental reindexing directly:
```powershell
# Index technical reference documentation:
.\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga/Reference" --source amiga

# Index architectural design notes:
.\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga/Design" --source obsidian
```

### Querying the Knowledge Base

Developers and AI agents can query the RAG database instantly via CLI or FastMCP:

```powershell
# Fast CLI search across hardware manuals:
python tools/harness/rag_search.py "Agnus blitter line mode minterm" --source amiga

# Fast CLI search across architectural design specs:
python tools/harness/rag_search.py "Color Clock CCK phases memory bus wait states" --source obsidian

# Check database status and document count:
python tools/rag/rag_qdrant/cli.py status
```

In the IDE, the FastMCP server (`tools/rag/rag_mcp_server.py`) exposes `rag_search`, `rag_status`, and `rag_list_sources` directly to AI agents.

---

## 6. Multi-Source Fallback Matrix & Error Resilience

To guarantee download resilience against link rot, server downtime, and rate limits, every reference item in `bootstrap_documentation.ps1` is backed by **2–3 independent, verified online mirrors**:

| Document | Primary Mirror (Verified 200 OK) | Secondary Mirror (Verified 200 OK) | Tertiary / Fallback Mirror |
| :--- | :--- | :--- | :--- |
| **`Hardware Reference Manual`** | [Internet Archive (1989 2nd Ed OCS PDF, 405p 600 DPI)](https://archive.org/download/commodore-amiga-hardware-reference-manual-2nd/Commodore_Amiga_Hardware_Reference_Manual_2nd.pdf) | [Internet Archive (1985 1st Ed PDF)](https://archive.org/download/Amiga_Hardware_Reference_Manual_1985_Commodore/Amiga_Hardware_Reference_Manual_1985_Commodore.pdf) | Manual local file drop in `temp/` |
| **`A500 A2000 Technical Reference Manual`** | [Internet Archive (1987 OEM Clean Scan PDF, 308p 200 DPI)](https://archive.org/download/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore/Commodore_Amiga_A500_A2000_Technical_Reference_Manual_1987_Commodore.pdf) | [Internet Archive (1987 OEM Alternate Scan PDF, 309p)](https://archive.org/download/CommodoreAmigaA500A2000TechnicalReferenceManual/Commodore%20Amiga%20A500-A2000%20Technical%20Reference%20Manual.pdf) | Manual local file drop in `temp/` |
| **`68000 Programmer's Reference Manual`** | [Internet Archive (M68000PM/AD Rev 1 Vector PDF, 646p)](https://archive.org/download/M68000PRM/M68000PRM.pdf) | [Bitsavers (M68000PM/AD Rev 1 1992 PDF)](https://archive.org/download/bitsavers_motorola68ogrammersReferenceManual1992_2394181/M68000PM_AD_Rev_1_Programmers_Reference_Manual_1992.pdf) | Manual local file drop in `temp/` |
| **`68000 User's Manual`** | [Internet Archive / Bitsavers (Rev 8 PDF, 601 DPI 216p)](https://archive.org/download/bitsavers_motorola68MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf) | [Internet Archive (Rev 8 Alternate Item)](https://archive.org/download/bitsavers_motorola6868000MicroprocessorUsersManualRev81993_11152468/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf) | [Internet Archive / Bitsavers (Family Reference 1988, 608p)](https://archive.org/download/bitsavers_motorola68rence1988_23248083/M68000_Family_Reference_1988.pdf) |
| **`Instruction Prefetch`** | [Pasti Project (Original Live Web)](http://pasti.fxatari.com/68kdocs/68kPrefetch.html) | [Wayback Machine (2021 Snapshot)](https://web.archive.org/web/20210211153835id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html) | [Wayback Machine (2019 Snapshot)](https://web.archive.org/web/20190317072535id_/http://pasti.fxatari.com/68kdocs/68kPrefetch.html) |
| **`Undocumented features`** | [Achtung! Amiga (Original Live Web)](https://www.winnicki.net/amiga/achtung/) | [Wayback Machine (2022 Snapshot)](https://web.archive.org/web/20220330190533id_/https://www.winnicki.net/amiga/achtung/) | [Wayback Machine (2016 Snapshot)](https://web.archive.org/web/20160410052327id_/http://www.winnicki.net/amiga/achtung/) |

### Multi-Page Web Crawling Engine
For multi-page web publications, the bootstrapper incorporates an autonomous crawling engine:
- **Kuba Winnicki's *Achtung! Amiga*:** Downloads the root index and all 16 technical subpages (`Copper.html`, `Sprite_Hardware.html`, `Freeing_the_DMA.html`, `More_sprites_in_one_line.html`, `Disappearing_sprites.html`, `UHRES_Display.html`, `Speed_Up_Tricks.html`, `Faster_Chipmem_bus_in_PAL_mode.html`, `Other_Amiga_Native_Hardware.html`, `CD32_Controller.html`, `Battery_Backed_Clock.html`, `Desaturation_Control_Bit.html`, `Video_timings.html`, `Links.html`, `Last_Words.html`, `What_is_this_all_about.html`).
- If the primary live server at `winnicki.net` is unreachable or blocks requests, the crawler automatically switches to the permanent Wayback Machine snapshot mirror.
