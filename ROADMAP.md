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
  - Headless backend engine with stepping (`step_cck`, `step_instruction`), cycle-exact M68000 disassembler (with LEA, CLR, DBRA, TST, NOT, NEG support), breakpoints, memory watchpoints, and 1024-entry execution trace ring buffer (`crates/debugger`).
  - Cross-platform immediate-mode Developer Studio GUI (`crates/gui` via egui/eframe) with synchronous direct-state pull, bounded execution time-slicing, 1:1 layout mapping, live CPU register diffs, self-documenting contextual tooltips, CCR LED toggles, collapsible microcode inspector with live clock counters (`F8`), fixed-width cell slot memory hex editor, clean game screen mode (`F12`), binary injection loader, and lean circular temporal time-travel scrubber (25,000 frames default, paused by default, with disassembly historical loop iteration rewind).
  - 100% covered by 23 automated integration and UI interaction tests (`crates/gui/tests/`).
- **Per-Opcode Micro-Benchmark Harness & Verification Framework (Completed Baseline Deliverable):**
  - Exhaustive cycle-exact benchmarking framework in `crates/test_runner` measuring instruction execution time, cycles per instruction, and throughput across all 65,536 dispatch entries and operand modes.
  - Comprehensive golden reference datasets (`m68k_benchmark_baseline.csv`, `m68k_benchmark_baseline.json`) with cryptographic SHA256 anti-tamper contracts (`golden_row_hashes.rs`).
  - Automated execution trace audit logger (`--dump-traces`), statistical variance tracker, and cycle anomaly detector.
- **Repository Sanitization & History Scrubbing (Completed Baseline Deliverable):**
  - Full Git history audit, Conventional Commits normalization, pruning of 146 empty/redundant commits (preserving author/committer timestamps), and deep asset purge (decoupling `ref_src`, PDFs, schematics, and caches), shrinking repository packfile from ~2.8 GB to 4.43 MB (99.8% reduction).
- **Custom Chipset Integration & Autonomous Execution Engines (Completed Baseline Deliverable):**
  - Scaffolded 16 flat workspace crates (`copper`, `blitter`, `dma`, `agnus`, `sprites`, `frame_builder`, `mouse`, `joystick`, `denise`, `audio`, `floppy`, `interrupts`, `paula`, `keyboard`, `cia`, `machine_loop`) with zero circular references and parameter-based borrow splitting.
  - Complete custom chip hardware registers with calibrated CCK electronic propagation delay pipelines ($K$-phase latency) and zero-cost stack-allocated `MemoryBus<'a>` router decoding Custom Chips (`$DF`), CIAs (`$BF`), and RTC (`$DC`).
  - Action dispatch pipeline bridging register writes to strongly-typed subsystem methods (`FloppyDrive`, `Blitter`, `Audio`, `Copper`, `Denise`).
  - Decoupled `A500State` serialization supporting referenced/self-contained modes, JSON and gzip compression (<50 KB), and 5-slot in-memory quick-save ring buffer with Developer Studio UI integration (`F6`/`F9`).
  - Machine-wide cold (`reset_cold`) and warm (`reset_warm`) reset sequencing, privileged M68000 `RESET` line propagation, and hardware `Ctrl-Amiga-Amiga` reset trigger.
  - End-to-end multi-chip interrupt priority arbitration pipeline (Levels 1–6) across Agnus, Paula, and CIAs with autovector exception processing.
  - Autonomous chipset execution engines: Agnus Copper coprocessor (`MOVE`, `WAIT`, `SKIP`, `CDANG`), Agnus 4-channel DMA Blitter (256-minterm Boolean ALU, barrel shifters, line drawer), Denise pixel pipeline (bitplane serializer, 32-color palette, EHB, HAM6, dual playfield) and 8 hardware sprites, Paula 4-channel audio sample streaming, Floppy MFM track encoder/decoder with ADF injection, and dual MOS 8520 CIAs (timers, TOD, keyboard serial shift register).
  - Agnus master DMA bus arbiter with 227/226 CCK horizontal scanline scheduling, 8-tier bus priority, Blitter Nasty / starvation yield, and direct CPU Chip RAM wait-state stalling with Fast RAM immunity.
  - 100% verified across 114 unit and integration tests throughout the workspace.
- **Whole-Machine Integration & Cross-Chip Pipeline Verification (Completed Baseline Deliverable):**
  - Full `A500Machine` cycle-exact integration test suite (`crates/machine_loop/tests/`) running unified CPU and chipset color clock loops (`step_cck`, `step_instruction`).
  - Synthetic end-to-end verification covering Copper beam synchronization, `WAIT`/`MOVE`, `SKIP` condition branching, and `CDANG` danger mode; Blitter 2D memory copies, area fill operations, and Blitter Nasty contention vs Fast RAM immunity; Denise palette batch updates, sprite arming/windowing, 1-bitplane fetch, and display window clipping; Paula audio sample streaming, buffer refill IRQ, and multi-interrupt priority arbitration (IPL 1–6); CIA-A/B timer underflows, 50 Hz VBlank TOD ticking, and DMACON master/channel arbitration with 1-cycle electronic signal propagation.
  - Standardized 4-question integration checklist and 4 canonical test archetypes codified in `.agents/rules/unit-testing-policy.md` and design specifications.
  - 100% verified across 27 integration test cases in `crates/machine_loop/tests/` without relying on heavy full-frame reference captures.
- **Automated vAmigaTS Test Runner & Verification Infrastructure (Completed Baseline Deliverable):**
  - Fully recursive catalog discovery and categorization engine in `crates/test_runner/src/vamiga/` (`catalog.rs`, `script.rs`, `runner.rs`, `matcher.rs`, `injector.rs`).
  - Indexes all 2,077 test directories in `ref_src/vAmigaTS`, classifying each test into 1,468 active Phase 1 Baseline OCS tests and 609 formally deferred tests.
  - Direct-injection payload extractor slicing Sector 2 (`$000400`) directly into Chip RAM at `$00070000`, installing zero-allocation ExecBase/GfxBase stub tables and setting Supervisor Stack Pointer ($SSP = \$0007FF00$).
  - Full `.retrosh` script directive parser extracting target machine profiles, CPU revisions, and calibrated frame execution budgets.
  - Golden raw reference frame differencer matching rendered $716 \times 285$ 24-bit RGB viewports against 2,815 physical silicon frame captures (`.raw` / `_ocs.raw`).
  - Unified CLI test runner interface: `cargo run -p test_runner -- vamiga [--category <CAT>] [--test <NAME>] [--frames <N>] [--list-deferred] [--summary] [-v]`.
  - 100% verified across 12 automated unit and integration tests (`crates/test_runner/tests/test_vamiga_runner.rs`, `test_vamiga_harness.rs`, `test_vamiga_copper.rs`, `test_vamiga_blitter.rs`, `test_vamiga_denise.rs`, `test_vamiga_paula.rs`).

### Phase 2: Enhanced Chipset (ECS) & Later Models
- **A500 Rev 6A (1 MB Chip):** Fat Agnus 8372A with 1 MB Chip RAM jumper configuration.
- **A500 Plus:** Full ECS chipset (Agnus 8372A 1MB, Denise 8373 with Productivity modes), Kickstart 2.04 (512 KB), onboard battery-backed RTC.
- **Deferred ECS vAmigaTS Test Suite Execution Gate:**
  - Unfilter and execute all vAmigaTS test suites deferred during Phase 1 that require ECS silicon features (`_ecs.raw`, `_plus.raw`, `BPLCON3`, SuperHires, 1 MB / 2 MB Agnus registers, and Productivity scan modes) per the *Test Suite Categorization, Filtering & Deferred Execution Matrix* in Step 3.1.

### Phase 3: Advanced Graphics Architecture (AGA / A1200)
- **A1200 (AGA):** Motorola 68EC020 (32-bit), 2 MB Chip RAM, Alice, Lisa, 24-bit color palette.
- **Deferred AGA vAmigaTS Test Suite Execution Gate:**
  - Unfilter and execute all vAmigaTS test suites requiring AGA silicon features (`_A1200.raw`, 68EC020 CPU, 24-bit color palette, 8 bitplanes) deferred during Phase 1 & 2.

### Peripheral Extensions (Post-Baseline)
- **4-Player Joystick Adapter:** Parallel port 4-joystick adapter for multiplayer games (e.g. *Super Skidmarks*, *Dynablaster*).
- **Analog Joysticks:** Proportional analog potentiometer sampling via POT0DAT/POT1DAT.
- **Light Pen / Gun:** Video beam position latching via VPOSR/VHPOSR registers and BPLCON0 bit 3 (LPEN).

---

## 2. Core Implementation Strategy (Remaining Milestones)

### Agent Execution & Step Completion Protocol (Mandatory Contract)

Every actionable item in this roadmap represents an achievable milestone with an explicit **Objective**, **Actionable Scope**, and **Verification Gate**.
When an agent finishes any numbered item:
1. **Pass Verification Gate**: Run the specified commands and ensure all tests, linters, and pre-flight gates pass.
2. **Log in DIARY.md**: Append the milestone narrative, design rationale, and modified files to Section 10 using `python tools/harness/log_diary.py`.
3. **Prune Active Item from ROADMAP.md**: Completely delete the finished item or sub-step text from Section 2 per `.agents/rules/roadmap-maintenance.md` (zero retention of completed items in the active backlog).
4. **Update Section 1 Baseline**: If an entire Step or major architectural capability was delivered, summarize it concisely in Section 1 ("Completed Baseline Deliverables").
5. **Renumber Contiguously**: Ensure remaining steps and sub-steps remain contiguously indexed ($1, 2, 3\dots$) to eliminate gaps.

---

### Step 1: Clean-Slate Custom Chipset Spec Reset & Self-Bootstrapped Verification (Immediate Primary Focus)

- **0.1: MachineLoop Code Review & Architecture Orientation**
  - **Objective:** Read and understand the current `crates/machine_loop` implementation end-to-end before making any changes to the chipset. Establish a mental model of the actual execution path, bus arbitration sequence, and poll-based signal routing as it exists today vs. the HRM spec.
  - **Actionable Scope:**
    - Trace the main step loop: chip step order, CCK phase progression, poll-and-route signal flow.
    - Identify structural gaps, stale scaffolding, or deviations from the hardware-bus-topology rule.
    - Produce a short written summary of findings (inline comments or Obsidian note); no production code changes.
  - **Verification Gate:**
    - Agent written orientation summary committed to `Obsidian/Amiga/Design/MachineLoop Architecture Review.md`.

- **0.2: Test Suite Review & Structural Orientation**
  - **Objective:** Understand the current state of all test crates (`test_runner`, per-crate `tests/`) — what is covered, what is missing, what is structurally sound vs. accidental.
  - **Actionable Scope:**
    - Catalogue existing L1/L2/L3 tests per subsystem.
    - Identify zombie tests, coverage gaps, and coupling violations.
    - Decide if any structural rework (file layout, harness helpers) is needed before the HRM-aligned test sprint.
  - **Verification Gate:**
    - `python tools/harness/audit_code_quality.py --dead-code`
    - Written gap matrix committed to `Obsidian/Amiga/Design/Test Coverage Matrix.md`.

- **0.3: Profiling Infrastructure — `cargo flamegraph` Feasibility & Manual Workflow**
  - **Objective:** Establish whether `cargo flamegraph` (based on `perf` / `dtrace` / `samply`) is viable on this host; document the manual workflow if so.
  - **Actionable Scope:**
    - Attempt `cargo flamegraph --bin <emulator_binary>` on a headless run; capture a sample SVG.
    - If blocked by OS/permission constraints, fall back to `samply` (see `profile-external` skill).
    - Document the working invocation, post-processing steps, and baseline throughput in `Obsidian/Amiga/Design/CPU Instruction Benchmarking.md`.
  - **Verification Gate:**
    - At least one flamegraph SVG committed to `Obsidian/Amiga/Design/assets/`.

- **0.4: Profiling Infrastructure — `cargo-profiler` / `perf` Feasibility & Manual Workflow**
  - **Objective:** Evaluate `cargo-profiler` (callgrind / cachegrind backend) as a complementary profiling tool to flamegraph.
  - **Actionable Scope:**
    - Attempt `cargo profiler callgrind --bin <emulator_binary>` and capture annotated output.
    - Compare hot-symbol lists with flamegraph findings.
    - Document limitations (WASM incompatibility, Windows availability) and recommended host platform.
  - **Verification Gate:**
    - Callgrind output or documented limitation note committed to benchmarking doc.

- **1.1: Strategic Clean-Slate Reset of Custom Chips & Machine Loop (Baseline HRM Spec Alignment)**
  - **Objective:** Cleanse and reset `crates/machine_loop` and all specialized custom chip subsystems (`agnus`, `denise`, `paula`, `cia`, `copper`, `blitter`, `floppy`) outside the CPU and physical memory to pure HRM architectural specifications, purging accumulated legacy scaffolding.
  - **Actionable Scope:**
    - Cleanse register definitions and reset states to align strictly with the official Commodore Amiga Hardware Reference Manual (HRM).
    - Establish an "idealistic", pure architectural baseline with zero ad-hoc hacks.
    - Defer real-world hardware quirks, timing latencies, and edge cases until demanded by failing tests and empirical verification.
  - **Verification Gate:**
    - `cargo check --workspace`
    - `cargo test -p test_runner --test test_architecture_rules`

- **1.1a: Signal Propagation Documentation Audit & Hardening**
  - **Objective:** Ensure that all agent governance files (rules, skills, workflows, design docs) accurately and completely describe the two signal propagation mechanisms so any future agent has zero ambiguity about how to implement them correctly.
  - **Actionable Scope:**
    - **Register-write propagation:** Verify that `hardware-bus-topology.md`, relevant design specs, and `MachineLoop` documentation clearly describe what happens clock-phase by clock-phase when a CPU or DMA write lands in a custom chip register (e.g. `COLOR00`, `BPLCON0`, `INTENA`). Update or author missing sections.
    - **Special inter-chip signal lines:** Verify documentation for DMA request/grant (`RGA` bus), interrupt lines (`_IPL0`–`_IPL2`, `_INTREQ`/`_INTENA` propagation through Paula → CIA → CPU), and any other active signal paths (e.g. blitter busy, disk DMA, copper WAIT/SKIP). Ensure polling methods and timing are documented with cycle-phase precision.
    - Sync any divergence between documentation and actual `MachineLoop` poll-and-route implementation found in step 0.1.
  - **Verification Gate:**
    - `python tools/harness/pre_flight.py --milestone` (Docs Quality pillar).
    - All updated design docs have `last_verified_commit` checkpoint bumped.

- **1.1b: HRM-Aligned Subsystem Test Suite Authoring**
  - **Objective:** Write a clean, authoritative test suite for each custom chip subsystem derived strictly from the Amiga Hardware Reference Manual — not from source code inference. Tests must be simple, register-behavioral, and free from cycle-exact timing complexity.
  - **Actionable Scope:**
    - One test file per subsystem: `test_agnus.rs`, `test_denise.rs`, `test_paula.rs`, `test_cia.rs`, `test_copper.rs`, `test_blitter.rs`.
    - Each test: write a register value → advance minimum required clocks → assert observable output or state. No multi-chip timing chains in L1 tests.
    - Simple inter-subsystem interaction tests (L2): focus on the cleanest signal paths — e.g. CIA timer overflow → Paula `_INT` → CPU IPL change. Prefer to write inter-chip tests and any required implementation changes during horizontal blanking (hblank return) when bus is idle, to minimize contention complexity.
    - Do **not** target vAmigaTS or silicon-level cycle accuracy in this step — that is Step 1.3.
  - **Verification Gate:**
    - `python tools/harness/pre_flight.py --quick`
    - `cargo test -p test_runner --test test_architecture_rules`
    - All new test files pass with ≥ 2 `#[test]` functions and ≥ 10 assertions per subsystem per unit-testing-policy.

- **1.1c: Lockstep Differential Tracer vs. vAmiga (Contingency)**
  - **Trigger:** Only if HRM-aligned tests (1.1b) fail to isolate a regression — i.e. tests pass but behavior diverges from reference in ways not yet covered by the test suite.
  - **Objective:** Build a cycle-exact co-simulation harness that runs the same ROM/ADF through both this emulator and vAmiga in lockstep, dumps a full machine state snapshot at every CCK boundary, and reports the first divergence point with a structured diff.
  - **Actionable Scope:**
    - Add a `StepTracer` trait (or feature-gated callback) to `MachineLoop`: at each `step_cck()`, serialize `CpuState` + all chip register banks into a compact binary or JSON snapshot. All state structs already implement `serde::Serialize` — snapshot is essentially `serde_json::to_string(&machine.snapshot())`.
    - Wire vAmiga's existing headless state-dump mode (`ref_src/vAmiga`) to emit equivalent snapshots at the same CCK boundaries.
    - Harness (`tools/harness/lockstep_diff.py`): load both snapshot streams, walk them in parallel, and stop at the first CCK where any field diverges — output: `CCK #N | field | expected (vAmiga) | actual (ours)`.
    - Keep tracer behind a feature flag (`--features tracer`) so zero overhead in production builds.
  - **Verification Gate:**
    - Tracer successfully identifies the CCK and field of a known injected regression (synthetic test).
    - `cargo test -p test_runner --test test_lockstep_tracer` (smoke test against a trivial ROM loop).

- **1.2: Kickstart ROM & Floppy Subsystem Bring-Up for Native Program Execution**

  - **Objective:** Operationalize the authentic floppy disk subsystem and Kickstart ROM overlay bootloader sequence to load and execute genuine Amiga programs from disk images (`.adf`).
  - **Actionable Scope:**
    - Implement `DF0:` drive mechanics, MFM track deserializer, and DMA track streaming.
    - Wire Kickstart ROM overlay boot sequence ($000000 / $FC0000).
    - Establish end-to-end capability to boot ADF disk images as the foundational prerequisite for native test harnesses.
  - **Verification Gate:**
    - Unit tests in `crates/floppy/tests/`
    - Machine loop boot integration tests in `crates/machine_loop/tests/`

- **1.3: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** Execute native Amiga disk-based test suites validating drive control, floppy DMA, and CPU coordination.
  - **Actionable Scope:**
    - Execute disk-based vAmigaTS tests verifying that the emulator can run native Amiga software to test itself.
    - Leverage unthrottled headless execution (running as fast as the host CPU permits without wall-clock rate limiting) for maximum throughput.
  - **Verification Gate:**
    - Automated test run reports for disk-based test suites passing with zero unexpected halts.

- **1.4: Principled, Rule-Compliant Custom Chip & Register Verification**
  - **Objective:** Advance custom chips and registers systematically from clean baseline specifications to fully validated cycle-exact implementations.
  - **Actionable Scope:**
    - Implement and calibrate custom chip features following the substrate-first causality chain (Layer 0 $\to$ Layer 4).
    - Strictly adhere to repro-first defect resolution (`.agents/rules/repro-first.md`) and 4-tier integration archetypes (`.agents/rules/unit-testing-policy.md`).
  - **Verification Gate:**
    - `cargo test -p machine_loop`
    - `python tools/harness/run_tests.py --integration`

---

### Step 2: Developer Studio GUI & Diagnostic Tooling Hardening (Ongoing Companion Track)

- **2.1: Continuous Testing & Polish of Developer Studio (Debugger View)**
  - **Objective:** Actively test, harden, and refine the Developer Studio GUI (`crates/gui`) under live emulation across all display tiers and interactive workflows.
  - **Actionable Scope:**
    - Test interactive panels under live stepping: Disassembly infinite stream browsing, in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
    - Test upcoming Save State management (`State` menu, in-memory quick slots 1–5, `F6`/`F9` shortcuts, and native file dialogs).
  - **Verification Gate:**
    - `cargo test -p gui --test test_interactions` (all headless egui interaction tests passing).

- **2.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability**
  - **Objective:** Complete audit, documentation, and user-facing surfacing of all implemented keyboard shortcuts across Developer Studio and Game View modes, ensuring zero hidden keys.
  - **Actionable Scope:**
    - Implement a dedicated in-app Keyboard Shortcuts Reference Modal (`?` / `F1` / Top Menu Help > Shortcuts).
    - Enrich button hover tooltips with contextual shortcut cues.
    - Surface all shortcuts: View Modes (`F12`/`F2`, `Escape`), Stepping Controls (`F5`/`Space`, `F10`, `Shift + F10`, `F11`), Tooling & Machine Controls (`F8`, `Alt + T`, `Ctrl + R`, `Ctrl + O`), Save States (`F6`, `F9`, `Ctrl + S`, `Ctrl + L`), Display Zoom (`Ctrl + +/=`, `Ctrl + -`, `Ctrl + 0`), and Hex/Disassembly editing navigation.
  - **Verification Gate:**
    - Headless integration test in `crates/gui/tests/` verifying modal dialog trigger and shortcut handling.

---

### Step 3: vAmigaTS Silicon Verification Sub-Suites & Contention Track (Active Focus)

- **3.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)**
  - **Objective:** Enforce strict gating and categorization for tests requiring hardware beyond Phase 1 Baseline OCS.
  - **Actionable Scope:**
    - Maintain and audit formal gate categories: FPU Coprocessors (206 tests deferred to Phase 3), ECS & AGA Silicon (112 tests deferred to Phase 2/3), 68010 CPU Architecture (91 tests deferred to 68010 Milestone), AmigaOS Floppy MFM Bootblock (7 tests deferred to Step 7.1), and Non-Visual Register Assertions (193 tests deferred to Step 3.2.6).
    - Ensure test runner automatically filters deferred tests when executing Phase 1 baseline suites.
  - **Verification Gate:**
    - `cargo test -p test_runner --test test_vamiga_runner -- test_vamiga_deferred_reasons_audit`

- **3.2: Targeted Subsystem Verification Sub-Suites Execution Track (1,468 Active Baseline Tests)**
  - **Objective:** Progressively execute and pass the 1,468 active baseline vAmigaTS tests in strict substrate-first causal order (Layer 0 $\to$ Layer 4), tracking progress in [vAmigaTS Verification Scorecard](Obsidian/Amiga/Design/vAmigaTS%20Verification%20Scorecard.md).
  - **Actionable Scope:**
    - **Sub-Suite 3.2.1: Agnus Master DMA Contention & Bus Arbitration (Layer 0 Substrate, 225 tests):** `Agnus/` (`DMACON/`, `BplDma/`, `DIW/`, `DDF/`, `bususage/`), verifying Color Clock phase allocation (odd/even), refresh cycles, CPU wait-state generation, and Blitter Nasty (`BLTPRI`).
    - **Sub-Suite 3.2.2: Agnus Blitter & Copper Coprocessor Engines (Layer 1 Coprocessors, 364 tests):** `Agnus/Blitter/` (250 tests: `line/`, `fill/`, `sblit/`, `timing/`, `bbusy/`, `bltint/`) and `Agnus/Copper/` (114 tests: `Wait/`, `Skip/`, `coptim/`, `coprace/`, `copvbl/`), verifying autonomous bus master operations, 256-minterm ALU, barrel shifts, modulos, and Copper 4-CCK timing.
    - **Sub-Suite 3.2.3: Denise Video, Bitplanes & Sprites (Layer 2 Video Serializer, 210 tests):** `Denise/` (`Registers/`, `Modes/`, `DIW/`, `Sprites/`), verifying pixel serialization, palette translation, display window clipping, and sprite multiplexing.
    - **Sub-Suite 3.2.4: Paula Audio & Floppy Subsystem (Layer 3 Peripherals, 107 tests):** `Paula/` (`Audio/`, `Interrupts/basicint/`), verifying PCM sample streaming, period clock division, and Level 1–4 interrupt requests.
    - **Sub-Suite 3.2.5: Complex CIA-A / CIA-B & Timers (Layer 3 Peripherals):** Timers A & B, TOD clock 50/60 Hz synchronization, serial shift register (SDR), and port handshake lines.
    - **Sub-Suite 3.2.6: Mainboard, Memory & Peripheral Assertions (Layer 4 System, 58 visual + 193 non-visual tests):** `Mainboard/`, `Memory/`, `Misc/`, verifying address decoding, port registers, and RAM expansion configurations.
    - **Sub-Suite 3.2.7: M68000 CPU Silicon Pipeline (Layer 4 System Integration, 503 tests):** `CPU/` (ALU, bitwise, shifts, exceptions, traps, IPL autovectors).
  - **Verification Gate:**
    - Scorecard synchronization in `Obsidian/Amiga/Design/vAmigaTS Verification Scorecard.md`.
    - `cargo run -p test_runner -- vamiga --category <CAT> --summary`

- **3.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)**
  - **Objective:** Optimize the dynamic per-CCK DMA slot priority evaluation into a clean, pre-computed scanline slot lookup table (`[DmaSlot; 227]`).
  - **Actionable Scope:**
    - Localize optimization strictly within the Agnus DMA scheduler (`crates/dma/` / `crates/agnus/src/dma/`).
    - Preserve code simplicity and readability, with zero complex asynchronous event wheels or timing skips.
    - Re-evaluate with `--profile` across vAmigaTS suites to measure empirical throughput gains.
  - **Verification Gate:**
    - Zero regression across all active Phase 1 vAmigaTS suites.
    - `cargo test -p test_runner --test test_dma_cartesian`

---

### Step 4: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)

- **4.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector**
  - **Objective:** Implement dedicated register inspection docks/tabs in Developer Studio (`crates/gui`) with live delta highlighting and mutation delay observability.
  - **Actionable Scope:**
    - Dedicated docks for Agnus, Denise, Paula, CIAs (A & B), and RTC.
    - Format values in hex/binary with electric cyan change/delta highlighting.
    - Bitfield breakdown widgets and interactive tooltips for control/status registers (`DMACON`, `INTENA`, `INTREQ`, `BPLCON0`-`BPLCON2`, `ADKCON`, `CRA`/`CRB`).
    - Visualize staged pipeline register latches taking effect across Color Clock phases ($CCK1 \to CCK2$).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying register dock rendering and delta tracking.

- **4.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer**
  - **Objective:** Build a horizontal scanline timeline visualizer displaying the 227 Color Clock slots per scanline (PAL) / 226 slots (NTSC).
  - **Actionable Scope:**
    - Fixed allocations: DRAM refresh (CCK 0..3), Disk DMA (CCK 4), Audio DMA 0–3 (CCK 5..8), Sprite pairs 0–7 (CCK 12..27).
    - Dynamic allocations: Bitplane DMA (BPL 1–6), Blitter DMA (channels A, B, C, D), and CPU bus slots.
    - Live beam position cursor tracking horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates.
    - Bus contention & wait-state indicator visually flagging cycles where CPU or Copper are stalled (BLTPRI / Blitter Nasty).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying timeline visualizer rendering.

- **4.3: Copper Coprocessor Inspector & Real-Time Execution Tracker**
  - **Objective:** Implement a dedicated Copper list disassembler panel decoding instruction streams directly from Chip RAM with real-time execution tracking.
  - **Actionable Scope:**
    - Decode instruction streams (`MOVE`, `WAIT`, `SKIP`) from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
    - Real-time execution pointer tracking: highlight executing Copper instruction, pending wait condition ($VPOS$/$HPOS$ comparison), and CDANG danger mode status.
    - Visual correlation with raster beam: highlight beam position where Copper interrupts or register modifications trigger palette swaps or Blitter dispatches.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying Copper disassembler panel.

- **4.4: Internal Chipset State Machines & Deep Diagnostics**
  - **Objective:** Surface live diagnostic panels for internal chipset state machines in `crates/gui`.
  - **Actionable Scope:**
    - *Blitter Engine Diagnostics:* Visual representation of active channels (A, B, C, D), 256-minterm truth table visualization ($LF$ code decomposition), shift/mask register stages, and Bresenham line-drawing state counters.
    - *Denise Video & Sprite Pipeline:* Live inspection of bitplane serializers, dual-playfield priority layers, 8 hardware sprite registers, and collision latches (`CLXDAT`/`CLXCON`).
    - *Paula Multi-Engine Status:* Audio channel frequency, length, volume, BLEP table synthesis status, floppy MFM bit-stream buffers/sync-word detector (`$4489`), and serial UART FIFO.
    - *CIA Timers & Port Observability:* Live countdown of Timers A & B, TOD clock sub-second counters, serial shift register (SDR) status, and I/O port pin states.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying state machine diagnostic views.

---

### Step 5: Host I/O Peripherals, Audio Playback & Controller Hub

- **5.1: Host Audio Playback & CRT Presentation Shaders**
  - **Objective:** Operationalize host real-time audio output streaming and authentic CRT display presentation.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/audio/tests/`.
    - Headless shader initialization and rendering verification.

- **5.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/mouse`, `crates/joystick`, `crates/game_ports`, `crates/gui`).
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Dual Game Port Hub:** Slot assignment for Port 1 & Port 2 (Mouse, Joystick, CD32 Pad, Keyboard-Joystick, Disconnected), simultaneous dual-mouse and dual-joystick modes, hardware button routing (CIA-A `PRA`, Paula `POTGOR`), and CD32 7-button shift register protocol.
    - **Physical Host Mouse Capture:** Viewport mouse grab/lock mode, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), and sensitivity tuning.
    - **Physical Gamepad & Joystick Ingestion:** Host gamepad enumeration via `gilrs` (native desktop) and HTML5 Gamepad API (WASM), analog deadzones, and button mapping.
    - **Developer Studio Input UI:** Interactive "Game Ports & Input" tab/dock displaying port status, device dropdowns, and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/`, `crates/mouse/tests/`, `crates/joystick/tests/`, and `crates/game_ports/tests/`.
    - Headless UI input tests in `crates/gui/tests/`.

---

### Step 6: Dedicated Player GUI & Frontend Experience

- **6.1: Hardware Configuration & Kickstart ROM Selector**
  - **Objective:** Deliver user-friendly Amiga hardware profile selection and Kickstart ROM management.
  - **Actionable Scope:**
    - Profile selector: Basic A500 512 KB, Classic A500 1 MB (Recommended), Expanded A500 4 MB.
    - Reset warning confirmation modal informing user that changing hardware configuration requires a cold machine reset.
    - Kickstart ROM manager: file picker for ROM images (1.2, 1.3, custom) with automatic checksum validation (CRC32/SHA-256).
  - **Verification Gate:**
    - Headless UI tests in `crates/gui/tests/` for profile selection and ROM validation modals.

- **6.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`)**
  - **Objective:** Visual floppy drive slot manager with live drive activity indicators and disk insertion controls.
  - **Actionable Scope:**
    - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
    - Per-drive enable/disable toggles to mount or disconnect external floppy drives on the fly.
    - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
    - Visual drive activity indicators and drive stepping audio feedback.
  - **Verification Gate:**
    - Unit tests for multi-drive management and headless UI interaction tests.

- **6.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels)**
  - **Objective:** Provide an interactive visual save/load state overlay and slot manager with screenshot previews and configuration guards.
  - **Actionable Scope:**
    - Visual snapshot cards containing embedded thumbnail screenshots captured at the moment of saving.
    - Formatted creation date and time timestamps and user-defined state labels/descriptions.
    - Configuration integrity guard verifying matching hardware profiles before restoring state to prevent guest crashes.
  - **Verification Gate:**
    - Save state serialization unit tests and UI state manager tests in `crates/gui/tests/`.

---

### Step 7: Real-World Amiga Workloads, Host Cache Profiling & Pipeline Optimization (Post-Boot)

- **7.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate**
  - **Objective:** Verify physical floppy track MFM streaming and full Kickstart 1.3 bootstrap test suites.
  - **Actionable Scope:**
    - Execute the 9 non-standard test ADFs using physical floppy MFM track streaming via `crates/floppy`.
    - Unfilter and pass all vAmigaTS tests requiring full Kickstart 1.3 bootstrap (`dos.library`, `intuition.library`, and filesystem calls).
  - **Verification Gate:**
    - vAmigaTS test runner passing all MFM and OS-level tests.

- **7.2: End-to-End Bootable ADF Integration Testing**
  - **Objective:** Boot and execute genuine Amiga benchmarks and diagnostic suites directly from floppy disk images (`cargo test -p test_runner --test test_boot_adf`).
  - **Actionable Scope:**
    - Load and execute `AmigaTestKit.adf`, `SysInfo.adf`, and Dhrystone on the authentic Kickstart / Amiga chipset stack.
    - Leverage address guards, breakpoint traps, and instruction bounds for parameterized termination.
    - Assert functional correctness, numerical determinism, and measure baseline throughput (MIPS, host CPU cycle cost per emulated CCK).
  - **Verification Gate:**
    - `cargo test -p test_runner --test test_boot_adf` passing cleanly.

- **7.3: Real-Workload Host Cache Miss & Footprint Impact**
  - **Objective:** Profile host L1i/L1d cache misses, Last Level Cache (LLC) misses, and branch mispredictions during continuous execution of genuine Amiga software.
  - **Actionable Scope:**
    - Profile host PMU counters across the 65,536-entry static dispatch table during real workloads.
    - Quantify the empirical performance impact of CPU core memory footprint on non-repetitive workloads.
  - **Verification Gate:**
    - Documented PMU profiling metrics and analysis in `Obsidian/Amiga/Design/`.

- **7.4: CPU Core Memory Footprint Audit & Compaction Assessment**
  - **Objective:** Audit the exact memory footprint of the CPU core and assess compaction strategies.
  - **Actionable Scope:**
    - Audit host `.rodata` footprint (sizeof(OpcodeDescriptor) * 65,536, microcode arrays) and runtime state footprint (`Cpu`, `CpuState`, `CpuMicroState`).
    - Investigate whether bit-packing or deduplication yields measurable cache-miss reductions, or whether flat layout maximizes throughput.
    - Refactor slow handlers using host efficiency principles without custom macros or const-generics.
  - **Verification Gate:**
    - Memory footprint audit report and SingleStepTests passing with zero regressions.

- **7.5: Cross-Architecture & Mobile Performance Projections**
  - **Objective:** Extrapolate measured throughput to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile).
  - **Actionable Scope:**
    - Model CPU overhead margins to ensure sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation with full chipset contention and rendering.
  - **Verification Gate:**
    - Documented performance analysis projections in `Obsidian/Amiga/Design/`.

- **7.6: Fidelity & Regression Validation Gate**
  - **Objective:** Final regression certification across all optimization and refactoring phases.
  - **Actionable Scope:**
    - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity.
    - Pass the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") and full workspace unit/integration test suites.
  - **Verification Gate:**
    - `$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep`
    - `cargo test --workspace`

---

## 3. AI Agent Testing & Autonomous Infrastructure

To achieve cycle-exact accuracy and debug complex game/demo edge cases, the project leverages automated agent workflows and differential emulation:

### 3.1: Differential Testing Against Reference Emulators (Methodology)
- Run target test cases or problematic games in verified reference emulators ([vAmiga](ref_src/vAmiga) and [WinUAE](ref_src/WinUAE)).
- Step emulation to designated frame numbers or instruction milestones.
- Dump execution traces, CPU registers, DMA channel allocations, and memory state, comparing dumps to detect cycle deviations instantly.

### 3.2: Automated vAmigaTS Direct-Injection Architecture (Methodology)
- Slices pre-assembled test payloads at Sector 2 offset `$000400` directly into Chip RAM at `$00070000`.
- Executes bare-metal hardware tests with zero dependency on floppy drive stepping mechanics or Kickstart ROM bootstrap.
- Renders full frames in `FrameBuilder` and matches pixel buffers against 2,815 reference frame dumps ($716 \times 285 \times 3$ RGB).

### 3.3: Visual & Audio Multimodal Validation (Methodology)
- Render offscreen viewports in `FrameBuilder` and compare pixel buffers against golden reference captures with per-channel tolerance ($\pm 1$ ADC noise).
- Leverage AI agent vision tools to inspect rendered CRT frames for raster beam splits, Copper gradients, display window clipping (`DIW`), and sprite multiplexing anomalies.
- Capture raw PCM audio buffers from Paula's 4 DMA channels and verify frequency, period division, and volume modulation envelopes.

### 3.4: Headless Debugger Interface for Agents (Methodology)
- Machine-readable interface to the [Debugger](Obsidian/Amiga/Design/Debugger.md) engine:
  - Programmatically set PC breakpoints and memory watchpoints.
  - Query register state, disassembly, and execution trace history ring buffers.
  - Enable autonomous debugging agents to diagnose CPU hangs or crash dumps.

### 3.5: Autonomous Bootstrapping, Documentation Ingestion & Knowledge Pipeline Refinement (Active Milestone)

- **3.5.1: Verification of Document Bootstrapping (`tools/bootstrap.ps1 -Rag` / Qdrant RAG)**
  - **Objective:** Verify end-to-end documentation ingestion against local Qdrant vector database (`http://localhost:6333`).
  - **Actionable Scope:**
    - Validate SHA-256 incremental hashing cache, chunking fidelity, multi-threaded fast embeddings, and offline sidecar vision descriptions (`<image>.txt`).
    - Ensure zero regressions or stalls during fresh database initialization and incremental reindexing.
  - **Verification Gate:**
    - `.\tools\bootstrap.ps1 -Rag` completes with exit code 0.

- **3.5.2: Iterative Design Documentation Calibration Loop (`Obsidian/Amiga/Design/`)**
  - **Objective:** Establish a closed-loop calibration process for design specifications to ensure agent comprehension.
  - **Actionable Scope:**
    - Bootstrap design specifications into the RAG vector collection (`amiga`).
    - Test semantic retrieval queries against core architectural concepts (CCK phases, bus contention, wait states, delayed register mutations).
    - Inspect retrieved chunks, identifying gaps, ambiguity, or missing circuit invariants.
    - Refine and calibrate Markdown files per `.agents/rules/vault-linking-and-graph-integrity.md` (inverted pyramid, dual-layer linking).
    - Re-bootstrap into RAG and verify improved agent comprehension and code generation accuracy.
  - **Verification Gate:**
    - Verified retrieval precision and accuracy across test query sets via `python tools/harness/rag_search.py`.

- **3.5.3: Staged Clean-Room Reconstruction & Autonomous Documentation-to-Code Regeneration Testing**
  - **Objective:** Test and validate the end-to-end capability of an autonomous AI agent to reconstruct emulator subsystems directly from design documentation and rules.
  - **Actionable Scope:**
    - *Targeted Subsystem Source Deletion:* Delete the source code of an isolated subsystem or module (e.g. an auxiliary crate, a peripheral device, or a coprocessor block).
    - *Autonomous Re-generation:* Instruct the agent to re-synthesize the deleted implementation solely from curated design specifications (`Obsidian/Amiga/Design/`), architectural invariants, and test vectors.
    - *Parity & Diff Evaluation:* Compare the re-generated source code against the original implementation and verify behavior against test suites.
    - *Prompt & Documentation Refinement:* If generated code diverges or fails tests, enhance prompts, skill instructions, or design documentation with explicit invariants.
  - **Verification Gate:**
    - Re-generated subsystem passes all unit and integration test suites without regressions.

- **3.5.4: Reference Documentation Audit & Final Pruning Protocol (`Obsidian/Amiga/Reference/`)**
  - **Objective:** Prune unused reference documentation following mature clean-room autonomous regeneration.
  - **Actionable Scope:**
    - *Agent History Audit:* Inspect historical agent transcripts and vector database retrieval logs to determine which reference materials were ever consulted.
    - *Prune Hardware Irrelevancies:* Strip unused subsystem sections from multi-volume manuals (e.g. FPU 68881/2 in PRM, PC XT Bridgeboard / SCSI in A500/A2000 Technical Reference Manual).
    - *Minimal Ground Truth Retainment:* Preserve solely lean, canonical primary sources (`Hardware Reference Manual`, `Instruction Prefetch`, `Undocumented Features`, core `68000 User's Manual`), minimizing repository footprint.
  - **Verification Gate:**
    - `python tools/harness/audit_docs_quality.py` passing with zero broken references.

- **3.5.5: End-to-End Hardening of `tools/bootstrap.ps1`**
  - **Objective:** Exhaustively test the complete PowerShell bootstrapper across all flag configurations.
  - **Actionable Scope:**
    - Test `-Test`, `-Graphify`, `-Rag`, and `-All` flags.
    - Verify clean-room resilience on fresh environments: archive decompression (`.gz`/`.zip`), directory creation, missing dependency warnings, and non-zero exit code reporting.
  - **Verification Gate:**
    - `powershell -ExecutionPolicy Bypass -File tools/bootstrap.ps1 -Test` completes successfully with exit code 0.

---

## 4. Post-Phase 1 Extensions: Reverse Engineering & Extraction

- **4.1: Resource Extractor & Reverse Engineering Assistant**
  - **Objective:** Extract and annotate Amiga assets (bitplanes, sprites, palettes, audio samples) directly from emulator memory buffers.
  - **Actionable Scope:**
    - Extract graphics and audio samples directly from memory buffers.
    - Annotate assets, memory addresses, and game phases using LLM assistance.
    - Map active assets to the visual 24-bit memory map.
  - **Verification Gate:**
    - Unit tests for asset extraction algorithms in `crates/debugger/tests/`.
