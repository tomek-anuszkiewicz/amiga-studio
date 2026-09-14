# Amiga 500 Emulator: Repository Bootstrap & Documentation Strategy

This document outlines the architecture, execution model, and asset provisioning pipeline of the Amiga 500 emulator repository bootstrapping system (`tools/bootstrap.ps1`), along with the reference documentation lifecycle and clean-room regeneration protocol.

---

## 1. Core Bootstrap Philosophy: Zero-Setup Compilation vs. Optional Provisioning

A freshly cloned repository is **100% self-contained for standard compilation and execution** using the stable Rust toolchain:

```powershell
# Compile and run the emulator immediately
cargo run -p gui -- --game
```

No external downloads, test archives, or database services are needed to run games or the Developer Studio. Bootstrapping via `tools/bootstrap.ps1` is strictly an on-demand developer facility divided into three independent tiers.

---

## 2. The Three Bootstrap Tiers (`tools/bootstrap.ps1`)

| Tier | Flag | Purpose | Provisioned Assets & Services |
| :--- | :--- | :--- | :--- |
| **Tier 1: Verification Testbed** | `-Test` | Exhaustive single-step CPU validation and DMA contention stress testing | Automatically detects and decompresses Tom Harte physical silicon test vectors (`ref_src/SingleStepTests-680x0/`, 124 `.json` suites), verifies vAmiga / vAmigaTS reference test harness paths, and checks diagnostic disks (`tools/AmigaTestKit`). |
| **Tier 2: Code Knowledge Graph** | `-Graph` | AST-level structural code navigation and symbol dependency analysis | Runs `graphify update .` to extract an AST relationship graph (`graphify-out/`), mapping crates, structs, traits, functions, and cross-crate call hierarchies for AI agent pairing. |
| **Tier 3: Documentation & RAG** | `-Doc` | AI agent domain knowledge and architecture retrieval | Connects to the local Qdrant vector database (`http://localhost:6333`, collection `amiga`), indexing Commodore reference manuals, hardware documentation, and Obsidian design notes via `tools/rag/bin/amiga_rag.ps1`. |
| **Complete Setup** | `-All` | Complete initial development environment setup | Executes Tiers 1, 2, and 3 sequentially (tests -> Graphify AST -> RAG docs). |

---

## 3. Reference Documentation Analysis (`Obsidian/Amiga/Reference/`)

The reference directory currently contains 409 files totaling ~23.8 MB across 6 subdirectories and 6 standalone files. Based on development history and codebase inspection, these assets fall into three distinct utility tiers:

### Tier 1: Core Upstream Ground Truth (Actively Used & Essential)
These documents provided mathematical algorithms, micro-step bus cycle sequences, and chip register definitions directly implemented in Rust:
- **`Hardware Reference Manual` (HRM)** (77 files, 0.84 MB): The authoritative primary reference for all Amiga custom chips (Agnus Copper/Blitter, Denise video/sprites, Paula audio/interrupts, CIAs, and register memory maps).
- **`Instruction Prefetch on the Motorola 68000 Processor.md`** (23 KB): The foundational basis for the emulator's 2-phase Color Clock (`CCK1`/`CCK2`) micro-step execution state machine, `IR`/`IRC` pipeline progression, and instruction classification (Class 0, 1, 2).
- **`Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis.md`** (8.5 KB): Contains the exact 15-iteration non-restoring division cycle calculation algorithms implemented verbatim in `crates/m68000/src/instructions/divu.rs` and `divs.rs`.
- **`Undocumented features of OCS, ECS and AGA chipsets.md`** (35.6 KB): Real silicon behaviors, Copper wake-up quirks, and open-bus floating values ($FFFF).
- **`68000 User's Manual`** (Core Sections: 2, 5, 6, 8): Bus cycle state transitions ($S_0..S_7$), handshaking (`/AS`, `/UDS`, `/LDS`, `/DTACK`), standard instruction timings, and 7-word Address Error exception stack frames.

### Tier 2: Subsystem-Specific Reference (Partially Used, Contains Unused Sections)
- **`A500 A2000 Technical Reference Manual`** (69 files, 4.33 MB):
  - *Used:* Section 1 (Differences), Section 2 (Block Diagrams), Section 6 (Fat Agnus pinouts), Section 7.1 (OKI MSM6242B RTC registers).
  - *Unused:* Section 4 (PC XT Bridgeboard, Janus library, BIOS entry points), Section 5 (SCSI controller), and Zorro expansion bus specifications.
- **`68000 Programmer's Reference Manual` (PRM)** (64 files, 4.95 MB):
  - *Used:* Integer instructions, addressing modes, and exception processing.
  - *Unused:* Section 5 (68881/68882 Floating Point Coprocessors), Section 7 (CPU32 microcontroller instructions), and Appendix C (S-Record format).
- **`Amiga Guru Book`** (28 files, 1.43 MB): Primarily high-level AmigaOS system software, C compiler libraries (SAS/C, Aztec C), and DOS filesystem packets. Mostly redundant for bare-metal hardware and Kickstart emulation.

### Tier 3: Zero Historical Use (Candidates for Pruning)
- **`68000 Resident Structured Assembler Reference Manual`** (13 files, 0.18 MB): Describes Motorola's 1980s proprietary `ASM68K` macro assembler and structured directives (`IF_..THEN_..ENDI_`). Never used; test suites and debugger assemblers use standard Motorola mnemonics and raw hex opcodes.
- **`Guide to the Amiga Kickstart.md`** (26 KB): An end-user hobbyist guide describing physical ROM replacement, buying Kickstart chips, and Early Startup menus. Contains zero hardware circuit or register specifications.
- **`68000 FAQ 1.md` & `68000 FAQ 2.md`** (34 KB): Motorola BBS Q&A focused on board-level hardware engineering (PCB traces, SIMM wiring, physical DRAM refresh).

---

## 4. End-State Clean-Room Regeneration & Pruning Protocol

The long-term goal for the emulator's architectural knowledge base is full autonomy from external reference manuals:

1. **Design Documentation as Authoritative Ground Truth:**
   All verified hardware behaviors, silicon quirks, micro-step sequences, and timing formulas are continuously codified into `Obsidian/Amiga/Design/`. Once this specification suite is fully complete, the entire emulator can be regenerated directly from the design notes.
2. **Post-Regeneration Reference Audit:**
   When the project reaches full autonomous regeneration maturity:
   - Audit AI agent transcripts and RAG query logs to identify which reference files under `Obsidian/Amiga/Reference/` were ever consulted.
   - Permanently remove all Tier 3 documents and unreferenced Tier 2 subdirectories.
   - Retain only the minimal, canonical primary references in `Obsidian/Amiga/Reference/` to minimize repository size and eliminate noise in vector database embeddings.
