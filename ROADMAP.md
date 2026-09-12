# Amiga 500 Emulator: Development Roadmap & Strategy

This document outlines the phased development plan, hardware milestones, verification methodologies, and AI agent testing strategies for the Amiga 500 emulator project.

---

## 1. Hardware Roadmap & Milestones

### Phase 1: Baseline Amiga 500 (Rev 5 / Rev 6a OCS) — Immediate Focus
- **CPU:** Motorola 68000 cycle-exact core based on the **Microcode Archetype Baseline** (Native 2-clock micro-step slices: $1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$, 100% complete opcode coverage with 45,565 active valid opcodes out of 65,536 across Batches 1.1–1.11; 100% SingleStepTests pass across all test suites against physical silicon hardware captures [Tom Harte SingleStepTests-680x0]; 100% Cartesian DMA contention verification asserting cycle and state invariance across the full $2^k \times 2^M$ permutation space).
- **Memory Configuration:**
  - 512 KB Chip RAM ($000000-).
  - Optional 512 KB Trapdoor Slow RAM ($C00000-).
  - Optional 4 MB Auto-Config Fast RAM ($200000-).
- **Custom Chipset (OCS):**
  - Agnus 8370 (NTSC) / 8371 (PAL): Beam counters, Copper coprocessor, 4-channel Blitter, DMA arbitration.
  - Denise 8362: Bitplane serialization, 8 hardware sprites, 32-color palette, dual playfield.
  - Paula: 4-channel 8-bit audio DACs, floppy MFM controller, UART, interrupt priority encoder.
  - CIAs (2x MOS 8520): Timers A & B, I/O ports, TOD clock, keyboard shift register, overlay _OVL.
- **Peripherals & I/O:**
  - Dual game ports: Port 1 (Mouse), Port 2 (Joystick).
  - Floppy Drive: Internal DF0 with ADF byte slice injection.
  - Kickstart ROM: 256 KB (1.2 / 1.3) with low-memory overlay boot sequence.
- **Integrated Developer Debugger & Developer Studio GUI (Completed Baseline Deliverable):**
  - Headless backend engine with stepping (step_cck, step_instruction), cycle-exact M68000 disassembler (with LEA, CLR, DBRA, TST, NOT, NEG support), breakpoints, memory watchpoints, and 1024-entry execution trace ring buffer (crates/debugger).
  - Cross-platform immediate-mode Developer Studio GUI (crates/gui via egui/eframe) with synchronous direct-state pull, bounded execution time-slicing, 1:1 layout mapping, live CPU register diffs, self-documenting contextual tooltips, CCR LED toggles, collapsible microcode inspector with live clock counters (F8), fixed-width cell slot memory hex editor, clean game screen mode (F12), binary injection loader, and lean circular temporal time-travel scrubber (25,000 frames default, paused by default, with disassembly historical loop iteration rewind).
  - 100% covered by 23 automated integration and UI interaction tests (crates/gui/tests/).
- **Per-Opcode Micro-Benchmark Harness & Verification Framework (Completed Baseline Deliverable):**
  - Exhaustive cycle-exact benchmarking framework in crates/test_runner measuring instruction execution time, cycles per instruction, and throughput across all 65,536 dispatch entries and operand modes.
  - Comprehensive golden reference datasets (m68k_benchmark_baseline.csv, m68k_benchmark_baseline.json) with cryptographic SHA256 anti-tamper contracts (golden_row_hashes.rs).
  - Automated execution trace audit logger (--dump-traces), statistical variance tracker, and cycle anomaly detector.

### Phase 2: Enhanced Chipset (ECS) & Later Models
- **A500 Rev 6A (1 MB Chip):** Fat Agnus 8372A with 1 MB Chip RAM jumper configuration.
- **A500 Plus:** Full ECS chipset (Agnus 8372A 1MB, Denise 8373 with Productivity modes), Kickstart 2.04 (512 KB), onboard battery-backed RTC.
- **A1200 (AGA):** Motorola 68EC020 (32-bit), 2 MB Chip RAM, Alice, Lisa, 24-bit color palette.

### Peripheral Extensions (Post-Baseline)
- **4-Player Joystick Adapter:** Parallel port 4-joystick adapter for multiplayer games (e.g. *Super Skidmarks*, *Dynablaster*).
- **Analog Joysticks:** Proportional analog potentiometer sampling via POT0DAT/POT1DAT.
- **Light Pen / Gun:** Video beam position latching via VPOSR/VHPOSR registers and BPLCON0 bit 3 (LPEN).

---
## 2. Core Implementation Strategy (Remaining Milestones)

### Step 1: Standalone CPU Program Execution, Synthetic Workloads & Performance Projection (Active)
- **Direct Memory Program Injection & Execution (Leveraging Developer GUI & Harness):**
  - Inject compiled M68000 binary routines (raw machine code binaries, assembled routines) into emulated RAM without requiring Kickstart ROM or OS overhead.
  - Set initial execution context ($, $, $) and execute self-contained test programs to completion, designated stop addresses, or trap returns.
  - Validate multi-instruction correctness, register state, and memory side effects on concrete routines (e.g. arithmetic loops, block memory transfers, array sorting).
- **Complex Workloads, Standard Synthetic Benchmarks & Real-World Cache Analysis:**
  - Load and execute established M68000 algorithmic benchmarks (e.g. Dhrystone, Sieve of Eratosthenes, math/ALU stress kernels) directly in the CPU core harness.
  - Assert functional correctness and numerical determinism across long-running execution sequences.
  - Measure baseline emulation throughput (effective MIPS, instruction throughput, host CPU cycle cost per emulated CCK).
  - **Real-Workload Host Cache Miss & Footprint Impact:** Profile host L1i/L1d cache misses and branch mispredictions during continuous execution of real test programs. Determine the empirical performance impact of CPU core memory footprint on real-world workloads, validating whether memory reduction efforts yield meaningful speedups.
- **Cross-Architecture & Mobile Performance Projections:**
  - Extrapolate measured desktop throughput (x86_64 / desktop ARM) to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile devices).
  - Model CPU overhead margins to ensure headroom for sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation once custom chipset DMA contention and rendering are integrated.

### Step 2: CPU Core Memory Footprint Audit, Cache Profiling & Host Pipeline Optimization
- **CPU Memory Footprint Audit & Host Cache Miss Profiling:**
  - **Memory Footprint Audit (in Kilobytes):** Measure and document the exact memory footprint of the CPU emulator core:
    - Host .rodata footprint: 65,536-entry static dispatch table (sizeof(OpcodeDescriptor) * 65,536), static [MicroStep; N] array slices, and decoding metadata.
    - Host runtime state footprint: Cpu, CpuState, CpuMicroState sizes in bytes, auditing memory layout, alignment, and cache-line compactness.
  - **Host Cache Miss Profiling:** Measure host CPU performance counters (L1i instruction cache misses, L1d data cache misses, Last Level Cache / LLC misses, superscalar IPC, branch mispredictions) across opcode execution runs (via perf stat, cachegrind, or host PMU tooling).
  - Automatically rank handlers by host latency and flag operations exhibiting disproportionate execution overhead relative to emulated M68000 cycle counts.
- **Footprint Compaction Assessment & Host Pipeline Optimization:**
  - **Footprint Reduction Feasibility:** Investigate whether compacting the CPU footprint (e.g., bit-packing OpcodeDescriptor, microcode array deduplication, index packing) yields measurable L1i/L1d miss reductions and throughput gains, or whether the current flat layout already maximizes host branch-predictor and cache throughput.
  - Refactor identified slow handlers using host hardware efficiency principles (direct specialized flattening, branchless bit operations, eliminated redundant register banking, and cross-crate MIR inlining).
  - Strictly enforce architectural constraints: zero custom macros (macro_rules!), zero const-generic function matrices, zero dynamic heap allocations, and zero compromise on code readability.
- **Fidelity & Regression Validation Gate:**
  - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity and passes the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") with zero regressions.

### Step 3: Custom Chipsets (Agnus, Denise, Paula, CIAs)
- **Step 3.1: Minimal Machine Main Loop (A500::step_cck) & Subsystem Orchestration:**
  - Create the top-level machine struct (A500) owning all primary subsystems without circular references: cpu, memory_bus, cycle_counter, gnus, denise, paula, cia_a, cia_b.
  - Multi-level stepping interfaces: step_cck(cck: u64), step_instruction(), step_cycles(n), step_frame().
  - Strict lockstep Color Clock stepping: clock beam counters, advance DMA slots, clock CIAs, drive CPU CCK1/CCK2 bus phases against MemoryBus.
  - Central interrupt priority arbitration pipeline: sample Paula (Levels 1, 3, 4, 5), CIA-A (Level 2), and CIA-B (Level 6), calculate highest unmasked level, and drive cpu.set_ipl().
- **Step 3.2: Machine-Wide Reset Sequencing (eset_cold & eset_warm):**
  - Physical _RESET line propagation across all chips.
  - Boot overlay engagement (map_kickstart_to_low_memory in MemoryBus).
  - *Cold Reset:* Zero physical RAM buffers ($00), reset chip registers to power-on defaults (DMACON = , INTENA/INTREQ = , CIA latches cleared), initialize CPU SR = , load initial SSP/PC from $000000/$000004 (Kickstart ROM), prime prefetch queue (IR, IRC).
  - *Warm Reset:* Preserve RAM contents intact (ensuring Kickstart memory checksum and resident module discovery pass), re-engage _OVL, assert chip reset lines, reload initial vectors.
  - Hardware keyboard reset line: wire Ctrl-Amiga-Amiga reset trigger line to main machine reset flow.
- **Step 3.3: Delayed Signal & Register Mutation Propagation Pipeline:**
  - *Physical Circuit Simulation:* Register reads return the currently latched active state **immediately** ("Read is NOW"). Register writes, strobes, and register mutations (e.g. DMACON, BPLCON0, COLORxx, INTENA, COPJMP1, BLTSIZE, CIA timer latches) do not take instantaneous cross-chip effect; they are staged and propagate after $ Color Clock phases / CCK cycles before altering the active execution path.
  - *Zero-Allocation Hot Path Design:* Model staged mutations using fixed-size inline pipeline latches / ring buffers (e.g. [Option<DelayedWrite>; 4] or fixed-capacity shift latches) embedded directly within chip structs. Zero dynamic heap allocation (Vec, Box) during CCK stepping.
  - *Save State Persistence:* The delayed mutation pipeline, staged values, and remaining cycle countdowns are fully serializable in save states (AgnusState, DeniseState, etc.), guaranteeing deterministic round-trip snapshot capture and rewind/restore even mid-propagation.
- **Step 3.4: Agnus DMA Bus Arbiter (Baseline Model & Contention Exposure):**
  - Implement the baseline Agnus horizontal scanline DMA slot schedule (CCK 0..3 DRAM refresh, CCK 4 disk, CCK 5..8 audio, CCK 12..27 sprites, bitplanes, and even/odd slots).
  - CPU and Blitter contention arbitration (BLTPRI Blitter Nasty mode).
  - Direct bus lock exposure: drive bus lock methods (lock_chip_ram / unlock_chip_ram) so the CPU and all custom chips observe bus contention and stall with wait states (BusResult::WaitState), establishing correct bus contention physics even before individual channel internal DSP/rendering logic is fully completed.
- **Step 3.5: Decomposed Subsystem Deep Implementations:**
  - *Agnus:* Copper coprocessor state machine (MOVE, WAIT, SKIP, CDANG danger mode), 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer, ascending/descending modes).
  - *Paula Audio Engine with Native BLEP Synthesis:* Precomputed alias-free BLEP tables (blep_tables.rs) across Paula's 4 DMA audio channels (dynamic CIA-A LED filter switching), floppy MFM track controller, serial UART, interrupt multiplexer.
  - *Denise:* Video pixel serializer, bitplanes (1–6), 8 hardware sprites, 32-color palette (RGB444), dual playfield, collision detection registers (CLXDAT, CLXCON).
  - *CIAs (Dual MOS 8520):* Timers A & B, TOD clock, serial shift register (SDR), parallel/control ports, E-clock synchronization.
- **Step 3.6: Host Audio, CRT Shaders & Copper/DMA Logic Analyzer:**
  - Audio sink: Ring buffer decoupled from host audio playback (cpal / Web Audio).
  - GPU post-processing shaders for authentic CRT TV look and feel (scanlines, shadow mask, curvature, phosphor bloom).
  - Copper list visualizer with live beam position cursor and DMA slot logic analyzer timeline.

### Step 4: Dedicated Player GUI & Frontend Experience
- **Step 4.1: Hardware Configuration & Kickstart ROM Selector:**
  - Amiga hardware profile selector (Basic A500 512 KB, Classic A500 1 MB [Recommended], Expanded A500 4 MB).
  - Explicit notification and confirmation modal informing the user that changing hardware parameters requires a cold machine reset.
  - Kickstart ROM manager: file picker for Kickstart ROM images (1.2, 1.3, custom ROMs) with automatic checksum validation (CRC32/SHA-256).
- **Step 4.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`):**
  - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
  - Individual drive enable/active toggle checkboxes to mount or disconnect external floppy drives on the fly.
  - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
  - Visual drive activity indicators and floppy motor/stepping audio feedback.
- **Step 4.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels):**
  - Interactive save/load state overlay and slot manager.
  - Visual snapshot cards containing:
    - **Automatic Screen Capture:** Embedded thumbnail screenshot of the active Amiga display captured at the exact moment of saving.
    - **Timestamp:** Formatted creation date and time.
    - **Custom Label:** Optional user-defined state name / description for memorable checkpoints and game phases.
    - **Configuration Integrity Guard:** Verifies matching hardware profiles (RAM sizes, chipset mode) before restoring state to prevent emulator panics or guest crashes.

---

## 3. AI Agent Testing & Differential Execution Strategy

To achieve cycle-exact accuracy and debug complex game/demo edge cases, the project leverages automated agent workflows and differential emulation:

### 3.1 Differential Testing Against Reference Emulators
- **Cross-Emulator Execution Harness:**
  - Run target test cases, test ROMs, or problematic games in verified reference emulators ([vAmiga](ref_src/vAmiga) and [WinUAE](ref_src/WinUAE)).
  - Step emulation to a designated frame number or instruction milestone.
  - Dump execution traces, CPU registers, DMA channel allocations, and memory state.
  - Run the same binary on our emulator core and compare state dumps to detect cycle deviations instantly.

### 3.2 Automated vAmigaTS Test Suite Extraction
- Unpack and catalog the test disks in ref_src/vAmigaTS/.
- Implement headless runners that load each test ADF, execute until test completion, and verify screen buffers against reference PNG frame renders.

### 3.3 Visual & Audio Multimodal Validation
- **Screenshot Frame Dumps:**
  - Export rendered video frames at specific VBlank intervals.
  - Use visual comparison (pixel diffs or multimodal LLM inspection) to verify Copper color gradients, sprite multiplexing, and raster splits against WinUAE/vAmiga output.
- **Audio Sample Dumps:**
  - Capture raw PCM audio buffers from Paula channels at fixed cycle intervals and compare waveform phase and amplitude against hardware recordings.

### 3.4 Headless Debugger Interface for Agents
- Provide a machine-readable REST / IPC interface to the [Debugger](Obsidian/Amiga/Design/Debugger.md) engine:
  - Set PC breakpoints and memory watchpoints programmatically.
  - Query register state, disassembly, and execution trace history ring buffers.
  - Enable autonomous debugging agents to diagnose CPU hangs or crash dumps.

### 3.5 Autonomous Bootstrapping, Documentation Ingestion & Knowledge Pipeline Refinement (Active)

To guarantee that autonomous AI agents can reconstruct and verify emulator subsystems cleanly, the repository bootstrapping and knowledge ingestion pipeline must undergo end-to-end verification and calibration:

- **Verification of Document Bootstrapping (`tools/bootstrap.ps1 -Doc` / Qdrant RAG):**
  - Verify end-to-end documentation ingestion against the local Qdrant vector database (`http://localhost:6333`).
  - Validate SHA-256 incremental hashing cache, chunking fidelity, multi-threaded fast embeddings, and offline sidecar vision descriptions (`<image>.txt`).
  - Ensure zero regressions or stalls during fresh database initialization and incremental reindexing.

- **Documentation Conversion Skills Audit & AmigaGuide Evaluation:**
  - Audit and refine agent documentation skills (e.g. `pdf-to-markdown`) against primary hardware manuals (Commodore HRM, Motorola M68000 PRMs) to ensure structured, accurate Markdown extraction.
  - Evaluate whether a dedicated `amigaguide-to-markdown` skill is genuinely necessary or whether standard conversion tools and LLM comprehension suffice for AmigaGuide hypertexts.

- **Iterative Design Documentation Calibration Loop (`Obsidian/Amiga/Design/`):**
  - Establish a closed-loop calibration process for design specifications:
    1. Bootstrap design specifications into the RAG vector collection (`amiga`).
    2. Test semantic retrieval queries against core architectural concepts (e.g. CCK phases, bus contention, wait states, delayed register mutations).
    3. Inspect retrieved chunks, identifying gaps, ambiguity, or missing circuit invariants.
    4. Refine and calibrate the design Markdown files per [`.agents/rules/vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md) (inverted pyramid structure, dual-layer linking).
    5. Re-bootstrap into RAG and verify improved agent comprehension and code generation accuracy.

- **End-to-End Hardening of `tools/bootstrap.ps1`:**
  - Exhaustively test the complete PowerShell bootstrapper across all flag configurations (`-Test`, `-Graph`, `-Doc`, `-All`).
  - Verify clean-room resilience on fresh environments: archive decompression (`.gz`/`.zip`), directory creation, missing dependency warnings, and non-zero exit code reporting.

---

## 4. Post-Phase 1 Extensions: Reverse Engineering & Extraction

### 4.1 Resource Extractor & Reverse Engineering Assistant
- Extract graphics (bitplanes, sprites, palettes) and audio samples directly from memory buffers.
- Annotate assets, memory addresses, and game phases using LLM assistance.
- Map active assets to the visual 24-bit memory map.

---

## 5. Repository Sanitization & Public Release Preparation

To prepare the repository for open-source publication and public release, the codebase and Git history underwent a thorough sanitization and scrubbing pass. This eliminated bulky external assets, copyright-encumbered materials, accidental historical leaks, and commit noise while consolidating fragmented work into a clean, professional, and logical development history (reduced `.git` packfile from ~2.8 GB to 4.43 MB).

### 5.1 Historical Commit Message Normalization & Atomic Squashing
- [x] **Rewrite & Standardize Commit Messages:**
  - Audited all historical commits across the repository to replace auto-generated, low-signal, or casual commit messages.
  - Standardized messages into clean, professional, descriptive entries following Conventional Commits, closely aligned with the narrative in `DIARY.md`.
- [x] **Commit History Reordering & Empty Commit Pruning:**
  - Audited commit history and pruned 146 empty/redundant commits using `git-filter-repo` with `--prune-empty=always`.
  - Retained exact author identities, email addresses, and original author/committer timestamps across all 212 clean commits.

### 5.2 Deep Git History Scrubbing, Privacy Audit & Asset Purge
- [x] **Historical Content Hygiene & Privacy Audit:**
  - Thoroughly inspected all past commits and tree snapshots for leaks, sensitive personal information, private absolute paths, temporary debugging dumps, or unintended scratch files.
- [x] **Preserve Commit Metadata & Branch Topology:**
  - Executed comprehensive history rewrite using `git-filter-repo` ensuring commit author dates, committer timestamps, and overall branch topology remain intact.
- [x] **Purge Bulky, Copyrighted & Third-Party Assets from All Commits:**
  - **Reference Emulator Sources (`ref_src/`):** Completely purged from past commits; decoupled via `.gitignore` and retained 100% locally on disk for SingleStepTests.
  - **Books & Reference Documents:** Removed historical OCR caches and reference PDFs (`Obsidian/Amiga/Reference/`) from Git history; retained 100% locally on disk for RAG.
  - **Hardware Schematics (`schematics/`):** Purged all schematic scans and PDFs from Git history; retained 100% locally on disk.
  - **Third-Party Tools & Standalone Executables:** Purged third-party diagnostic executables (`AmigaTestKit`, `WinGuide.exe`) from Git history.
  - **Generated Knowledge Graphs & Analysis Artifacts:** Purged `graphify-out/` and local AST/vector caches from Git history.
  - **Repository Footprint Result:** Successfully reduced `.git` packfile database from **~2.8 GB down to 4.43 MB** (a 99.8% reduction), while preserving 100% of local disk assets.



