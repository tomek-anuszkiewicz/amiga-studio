# Amiga 500 Emulator: Engineering Diary & Architecture Genesis

This diary records how the Rust Amiga 500 emulator took shape, how the developer and AI agent worked together, and why they made key engineering decisions. Each dated result applies to the version and scope checked at the time. Consult the design specifications for current behavior and the roadmap for pending work.

## Contents

- [1. Project Genesis: Groundwork, Reference Ingestion & The Myth vs. Reality of AI Emulation](#1-project-genesis-groundwork-reference-ingestion--the-myth-vs-reality-of-ai-emulation)
- [2. Foundational Architecture: Cycle-Exact Mechanics, Rejecting the "God Bus" & Rust Sympathy](#2-foundational-architecture-cycle-exact-mechanics-rejecting-the-god-bus--rust-sympathy)
- [3. Modular Workspace Architecture & Two-Tier Hardware Decomposition](#3-modular-workspace-architecture--two-tier-hardware-decomposition)
- [4. Physical Circuit Simulation: Standalone BLEP Audio Synthesis](#4-physical-circuit-simulation-standalone-blep-audio-synthesis)
- [5. The 6-Stage Evolution of the M68000 CPU Core](#5-the-6-stage-evolution-of-the-m68000-cpu-core)
- [6. CPU Benchmarking: From Generated Cases to Handwritten Workloads](#6-cpu-benchmarking-from-generated-cases-to-handwritten-workloads)
- [7. Subsystem Evolution: The Developer Studio GUI & Debugger Engine](#7-subsystem-evolution-the-developer-studio-gui--debugger-engine)
- [8. Architectural Rules, Anti-Tamper Policies & The Living Definition of Done](#8-architectural-rules-anti-tamper-policies--the-living-definition-of-done)
- [9. The Ultimate Goal: The Clean-Room Re-Generation Experiment](#9-the-ultimate-goal-the-clean-room-re-generation-experiment)
- [10. Accepted-Recipe Reconstruction and Reference Publication](#10-accepted-recipe-reconstruction-and-reference-publication)
- [11. Register Access and Delivery to Owning Chips](#11-register-access-and-delivery-to-owning-chips)
- [12. A500 Gary Address Decoding](#12-a500-gary-address-decoding)
- [13. Receiver-Owned Signal Transport and Continuation](#13-receiver-owned-signal-transport-and-continuation)
- [14. SDLC, Agent Workflows and Development Harness](#14-sdlc-agent-workflows-and-development-harness)
- [15. Book-Derived Custom Chipset Reconstruction](#15-book-derived-custom-chipset-reconstruction)
- [16. Native Kickstart and Disk Program Execution](#16-native-kickstart-and-disk-program-execution)
- [17. Core Performance: Removing Repeated Host Work](#17-core-performance-removing-repeated-host-work)
- [18. Video: From Visible Raster Storage to a Stable Image](#18-video-from-visible-raster-storage-to-a-stable-image)
- [19. Host Execution, Player Workflow and Input Ownership](#19-host-execution-player-workflow-and-input-ownership)
- [20. Runtime Invariants and Floppy Activity Observations](#20-runtime-invariants-and-floppy-activity-observations)
- [Rolling Changelog](#rolling-changelog)

> [!IMPORTANT]
> **Zero Hand-Written Code (100% Voice-Prompted & AI-Generated):**
> Throughout development, **not a single letter of code was written by hand**.
> The AI agent generated 100% of the codebase, including the cycle-exact Motorola 68000 CPU core, the memory bus, the immediate-mode Developer Studio GUI, the debugger engine and the test harnesses. The developer set the architecture, reviewed the code in conversation and enforced quality requirements through spoken instructions and repeated rounds of engineering direction. They occasionally pasted a documentation link or typed a brief message when voice dictation was inconvenient.

---

## 1. Project Genesis: Groundwork, Reference Ingestion & The Myth vs. Reality of AI Emulation

### The Pre-Rust Phase: Physical Schematics, LTspice & Analog Audio Synthesis (Late August 2026)
The developer spent the final week of August 2026 gathering hardware references and modeling the Amiga 500's analog audio circuit. This work preceded the first generated Rust code.

- **Analog Low-Pass Audio Simulation (LTspice).** The developer took the exact resistor (ohms) and capacitor (farads) values from original Commodore schematics (`c448cd14`) and used them to model Paula's analog audio reconstruction circuit in LTspice (`0b2777fa`). The model covered the fixed 2-pole Sallen-Key low-pass filter and the switchable "LED filter" that CIA-A port A bit 1 controls.
- **Band-Limited Step (BLEP) Table Generation:** In a dedicated C# prototype (`9057f4a8`, `7783f1e2`), the human derived and verified the continuous-time impulse response and step response tables needed for alias-free audio interpolation, parameterizing them directly against physical component physics rather than arbitrary digital filters.
- **Reference Emulator Ingestion.** To ground design choices in existing implementations, the project imported five open-source reference emulators into `ref_src/` for cross-validation. These were MAME, Moira, Musashi, vAmiga and WinUAE.
- **Decoded Single-Step Vectors.** The project imported and organized over one million software-generated SingleStep cases before implementing the CPU (`a31518dc`). These cases supplied reference state and bus sequences. Later work clarified that agreement with these vectors did not constitute a physical hardware capture.

### The RAG Knowledge Base & Reference Documentation Sprint (September 1–4, 2026)
Between September 1 and September 4, the project prepared reference documentation across dozens of commits (`e82a4efc` through `e782dcf9`).

- The work converted hundreds of pages from the **Amiga Hardware Reference Manual (HRM)**, **Motorola 68000 Programmer's Reference Manual (PRM)**, **Amiga Guru Book** and **A500/A2000 Technical Reference Manual** into Markdown organized by the meaning of the source material.
- Vector SVGs and timing schematics were extracted from original PDFs, register layouts and bitmasks were normalized into tables, and Appendix B clock transition diagrams were recropped and cleaned.
- The project deployed a local **Qdrant vector database** (`tools/rag/`) with a custom FastMCP server (`amiga_rag`), along with a **Graphify** knowledge graph built from abstract syntax trees (ASTs) (`graphify-out/`).
- **The Core Strategy:** An AI agent prompted in a vacuum will hallucinate register addresses, flag behaviors, and bus cycle timings. By arming the agent with instant, offline vector retrieval over the official hardware manuals, the human made source evidence available for checking generated behavior. Retrieval alone did not establish that every instruction or bus cycle matched hardware.

### Language & Framework Deliberations: Why Rust & egui
Once the reference material was available, the developer and agent evaluated the systems programming language and UI framework.

- **Language Choice — Rust.** The project chose Rust for zero-cost abstractions, memory safety without garbage collection pauses, predictable host CPU performance and strict ownership. Rust could also compile the project for native desktop (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).
- **UI Framework Selection — egui.** The project chose egui because its immediate-mode interface reads state directly and synchronously (`app.update_ui(ctx)`), avoiding complex event-broker boilerplate. The framework compiled for native OS windows and browser-based WASM canvases.

### The Myth vs. Reality of AI-Assisted System Emulation
The project challenged a widespread idea in software development. Could a large language model produce a working, high-performance, cycle-exact emulator from one request?
> *"Build me a cycle-exact Motorola 68000 processor and an Amiga 500 emulator"*

The project found unguided generation prone to brittle, unoptimized implementations with unsuitable architecture. Bus contention, 2-phase color clock synchronization, pipelined prefetch advancement and subtle custom chip wait states required continuing architectural direction from the developer.

- The developer inspected each proposed instruction handler, memory bus arbitration phase and ALU flag update line by line.
- They rejected shortcuts for reducing repetitive code, including macros and const-generics, where those shortcuts obscured hardware behavior.
- They required automated, cycle-exact unit and integration test harnesses before accepting a milestone.
- They directed major architectural resets when the code stopped reflecting the hardware or became a poor fit for the host.

The developer set and enforced the architecture, and the agent rapidly generated and refactored the implementation in response.

---

## 2. Foundational Architecture: Cycle-Exact Mechanics, Rejecting the "God Bus" & Rust Sympathy

Two questions guided the early architecture discussions.
1. *What does "cycle-exact" actually mean for the Amiga 500?*
2. *How do we model custom chip interconnects cleanly in Rust without borrow panics?*

### Rejecting the "God Bus" & Circular Handles
In many historical emulators, all custom chips hold references to one central "God Bus" or pass circular handles (`Rc<RefCell<...>>`) so they can change shared state directly. The agent initially pointed out the convenience of shared references. The developer rejected circular pointers because of synchronization deadlocks, runtime borrow panics and poorer CPU cache locality. The design required **unidirectional, flat, and linear** coordination, with zero circular references between subsystems.

### Top-Down Orchestration via the Main Loop (`A500::step_cck`)
To achieve mechanical sympathy with Rust's ownership model and the host CPU's instruction pipeline:
- **No chip autonomously queries or mutates another chip.**
- The top-level machine struct (`A500`) directly owns all subsystems: `Cpu` (`M68000`), `MemoryBus`, `CycleCounter`, `Agnus`, `Denise`, `Paula`, `CiaA`, and `CiaB`.
- On each Color Clock phase, the **main loop** orchestrates execution top-down:
  1. The **Agnus DMA Scheduler** evaluates the active horizontal scanline slot (refresh, audio, disk, sprite, bitplane, or CPU).
  2. The bus access arbitration decision and wait states (`BusResult::WaitState`) are passed down to the CPU and custom chips.
  3. The main loop steps each chip for that CCK phase, and chips advance their internal sub-components accordingly.
  4. Interrupt lines (IPL 1–6) and DMA bus locks flow strictly through the top-level orchestration loop and `MemoryBus`.

### Host Performance: Locality, Allocation and Measurement

The early design preferred straightforward, predictable execution and preallocated
storage over opaque arithmetic tricks. Direct opcode dispatch and allocation-free
hot paths were chosen to keep host overhead visible and avoid allocator work during
emulation. Those choices are design intentions, not throughput measurements or a
guarantee of branch prediction, cache behavior or deterministic host latency.
The [optimization examples](.agents/skills/TA-profile-emulation/references/project-examples.md)
record later measurements and cases where active-work costs changed the conclusion.

---

## 3. Modular Workspace Architecture & Two-Tier Hardware Decomposition

From the inception of the codebase on September 6 (`8ed5eaca`), a clean workspace separation was instituted:

- **Dedicated Crates for Major Chips (Tier 1).** Each primary physical custom chip and subsystem had a dedicated workspace crate (`crates/agnus`, `crates/denise`, `crates/paula`, `crates/cia`, `crates/m68000`, `crates/memory_bus`).
- **Decoupled Crates & Modules for Sub-Units (Tier 2):** Rather than allowing chip structs to become monolithic God objects, major sub-components are decomposed into independent, specialized crates and modules:
  - **Agnus:** Independent state machines for the Copper coprocessor (`MOVE`, `WAIT`, `SKIP`, `CDANG`) and 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer).
  - **Paula:** 4 DMA audio channels, floppy disk MFM track controller, serial UART, and interrupt multiplexer.
  - **Denise:** Video pixel serializers, bitplane fetch engines (1–6 planes), 8 hardware sprite generators, and palette registers (RGB444).
  - **CIAs:** Dual MOS 8520 chips decomposed into Timers A & B, TOD 50/60 Hz clock, serial shift register (SDR), and parallel ports.
  - **Peripheral & Storage:** Decoupled crates for RTC (`crates/rtc`), Floppy drive/MFM, and Audio sinks.
- In the project's 3-tier ownership model, parents conceptually own and re-export their subcomponents. References between peers must remain free of cycles.

### The Evolution of the Memory Bus: From Dynamic Branching to $O(1)$ Direct Bank Handlers
The memory bus (`crates/memory_bus`) moved from repeated range checks to direct bank selection.

1. **Initial Range Checking (`179fa5d2`).** Early versions used cascaded `if/else` checks to match 24-bit addresses against address boundaries (`$000000-$07FFFF`, `$DF0000-$DFFFFF`, `$F80000-$FFFFFF`). That put range selection on every byte and word transfer.
2. **The $O(1)$ Bank Mapping Breakthrough (`4c799770`, `3b30e894`).** The developer directed a redesign that split the entire 16 MB 24-bit physical address space into **256 static banks of 64 KB each**.
   - A 256-entry lookup table (`[BankHandler; 256]`) maps each 64 KB block directly to the function pointers `read_byte`, `read_word`, `write_byte` and `write_word`.
   - A single byte shift (`address >> 16`) selects the bank, and an indirect call runs the appropriate handler. The selected handler still performs its own access checks. Host latency would require a separate measurement.
3. **Early Address-Space Model:**
   - **Unmapped Reads:** The initial model returned configurable `$FF` / `$FFFF` values. In October, the project replaced those settings with shared fixed placeholders. Physical bus retention and decay remained unverified in both versions.
   - **Kickstart Overlay:** CIA-A port A bit 0 controls the overlay latch. On cold boot, bank 0 (`$000000-$07FFFF`) mirrors Kickstart ROM until the first hardware write flips the latch to expose physical Chip RAM.
   - **Gary Chip RAM Mirroring & Slow RAM:** Accurate address decoding for standard 512 KB Chip RAM, 512 KB A501 trapdoor Slow RAM (`$C00000`), and Gary custom register mirrors (`$DF0000-$DFFFFF`).

### September Integration and October Ownership Corrections

On September 12, the project separated the disassembler, physical memory, bus and machine coordinator. On September 18, bank handlers took direct responsibility for `BusResult`, Agnus retained DMA address mastership, and Denise/Paula consumed delivered words rather than fetching Chip RAM themselves. Floppy control moved out of MemoryBus into motherboard routing through CIA and Paula. Serializable chip state stayed separate from runtime handles. The Cartesian contention test checked the model's wait-state relation at the time. Chapter 5 records its replacement.

The early integration work on September 14-15 covered Copper fetch and comparator behavior, CDANG and the PAL `$E0` slot restriction. It also covered Blitter minterms, shifts, modulo, unconnected latches and fill, along with Denise window and serializer behavior. The earlier diary described these as silicon calibration and passing vAmigaTS suites. Those historical claims do not establish measured timing or native external-suite execution. The capture infrastructure was later retired, and book-derived reconstruction remained necessary; Chapter 14 records that harness transition.

On September 29, `62c9f4c` closed the architecture review then numbered Step 1.4. It mapped CCK progression, bus arbitration and coordination between chips. The quick preflight and twenty architecture checks passed at that time. Functional completion of the chipset required separate work.

October 12-14 work centralized OCS register names and CIA addresses, removed unsupported BPLCON3/DIWHIGH state, and routed shared BPLCON0, display-window and sprite writes explicitly. Agnus gated bitplane fetches by the vertical window and DDF while Denise clipped horizontal output. DMA pointers were bounded to the OCS 512 KB range and Paula's direct Chip RAM fetch path was retired. Serialized fields became explicit rather than silently defaulted. Focused routing/pointer tests and applicable commit checks passed; later register, Gary and signal stages in Chapters 11-13 refined these foundations without certifying the receiving engines.

The CPU crate was renamed from `m68000` to `cpu` on September 19 (`9ec915c`). Earlier paths in the CPU history describe their original snapshots.

---

## 4. Physical Circuit Simulation: Standalone BLEP Audio Synthesis

The audio work used circuit-derived response curves to model the analog output of the Amiga's Paula chip.

- **WinUAE Parameterization & Circuit Derivation:** The Band-Limited Step (BLEP) synthesis engine was modeled and parameterized based on verified reference implementations in WinUAE.
- **Pure Mathematical Standalone Module:** Rather than hardcoding magic sound samples, the BLEP synthesis is implemented as an independent, decoupled module containing all the underlying physics and signal-processing mathematics (`crates/paula/src/blep_tables.rs`).
- **Analog RC Component Values.** The calculations use the exact resistance (resistors in ohms) and capacitance (capacitors in farads) of the Amiga motherboard's analog low-pass filter network to produce step response and frequency roll-off tables. They account for dynamic CIA-A LED filter switching.
- The resulting curves model the selected circuit assumptions; they do not by themselves establish physical output accuracy or eliminate every aliasing source.

The November 8 regression work protected the retained numerical curves. Physical audio output still required separate evidence. Chapter 14 records the offline reference checks and the optional WinUAE comparison.

### November 14-16: Buffered Stereo Output and Independent Spectral Checks

BLEP-AUDIO-STREAM-20261114 (`fb1d439`) replaced per-CCK observations with 44,100 Hz
stereo PCM. Timed DAC steps use PAL/NTSC correction tables and CIA-A filter selection
without moving channel DMA or interrupt events. A 1,024-pair overwrite ring retains
unread PCM and synthesis history across continuation. Runtime tables moved into
Paula; accepted PAL CSV references stayed protected. The host drain API received an
explicit single-symbol quality exception. This approved output contract superseded
the earlier audio-preservation experiment; host-device playback remained pending.

AUDIO-SPECTRUM-20261114 and AUDIO-ATTACHMENT-20261114 (`5ed7c28`) exercised Chip RAM
DMA, WAV decoding, independently predicted tones and DAC images, and looping volume
and period modulation. Four PAL/NTSC filter combinations and both attachment
scenarios passed. Checks included a silent source side, return to the initial
plateau and negative controls. The expected results were independent of the runtime
synthesis tables. The combined three-commit candidate passed 688 ordinary tests,
nine harness tests, 131 Python contracts and the commit profile. Intermediate
commits had no separate checks. Host playback and physical calibration of
filter-switch transients still required evidence.

TEST-RUNTIME-AND-VALUE-20261116 later shortened captures to about 120 ms without
widening amplitude, frequency, residual-alias or stereo tolerances. Three cases
passed before and after, reducing execution from 6.88 to 1.76 seconds outside
compilation. Enhanced reconstruction experiments remained deferred until games and
host music playback were available.

---

## 5. The 6-Stage Evolution of the M68000 CPU Core

Six stages between September 6 and September 10, 2026 established the early CPU core baseline (`crates/m68000` at the time). The October revisions and the limits of their checks follow the early stages below.

### The Foundational Methodology: Minimal Representative Set & Early SingleStepTests
From day one, the development strategy was deliberately disciplined: rather than attempting to blindly generate all 65,536 opcodes at once, the human instructed the agent to identify a **minimal representative subset of instructions** spanning every major instruction class (data movement, integer arithmetic, logic, shifts/rotations, control flow, and bit operations) and addressing mode.
During Stages 1 and 2, the project used software-generated **SingleStepTests** cases to check prototypes, the division of execution into clock slices and alternative designs against the reference model. These were software comparisons. Physical silicon would have required separate observations.

### 1. Stage 1 — Naive Emulation & Human-Centric Typing Shortcuts (Macros & Const-Generics)
*Timeframe: September 6, 02:00 – September 7, 14:00 (commits `8ed5eaca`, `a007047e`, `4e021e1e`, `5b34cb25`)*

For the minimal representative instruction set, the early prototypes favored constructs commonly used to reduce repetitive typing. The agent used deep cascaded `match` trees, user-defined macros (`macro_rules!`) and generic functions parameterized by constants (`fn op_foo<const S: usize, const M: usize>(...)`). The diary attributed this preference for fewer lines to the model's training bias toward saving human typing. The resulting abstractions made the generated code hard to follow.

The developer rejected those versions in code review for several reasons.

- Macros broke IDE navigation (Go to Definition, Find References, Call Hierarchy) and made compiler errors hard to interpret.
- Matrices of const-generic parameters produced many code variants and fragmented the concrete execution paths.
- Cascaded runtime matching put branches unrelated to physical 68000 microcode execution into the host CPU instruction cache (L1i).
- The developer permanently prohibited **custom macros (`macro_rules!`) and const-generic execution handlers**. The required form was explicit, concrete, specialized Rust functions.

### 2. Stage 2 — The Hybrid Action Buffer & Phase Experiment
*Timeframe: September 7, 11:00 – September 8, 01:00 (commits `2ff4c38d`, `25735352`, `e4602304`, `fb93f9b8`)*

The developer kept the minimal instruction subset in Stage 2 and required the model to represent M68000 bus cycles in discrete clock phases ($CLK1$ and $CLK2$, matching Amiga $CCK1$ and $CCK2$). ALU operations and instruction evaluation remained largely linear and immediate. The implementation deferred all memory bus operations (reads, writes, prefetches) to a dedicated "command/action buffer". A separate loop processed the buffer, so the CPU could stall and remain in wait states when Chip RAM contention blocked a transfer.

The SingleStepTests checks established that memory wait states could stall execution in this implementation. Separating instruction evaluation from memory access also created three problems.

- The ALU ran upfront while bus operations waited in the queue.
- The pipelined prefetch queue ($IR$/$IRC$) advanced separately from memory reads.
- Mid-instruction Address Error exceptions (Vector 3) became hard to handle correctly. By the time an unaligned memory read triggered an address error, the ALU had already changed guest registers.

### 3. Stage 3 — The "Frankenstein" Architecture (Full Opcode Expansion & CLK1/CLK2 Over-Engineering)
*Timeframe: September 8, 01:00 – September 8, 12:00 (commits `ee6814bc`, `2445c507`, `cdc1bd07`, `16ed85c4`, `8e07b26a`)*

The developer asked the agent to implement **all remaining M68000 instructions across the entire opcode space** and introduce the intended execution model. That model used unified micro-step slices and recurring instruction patterns, called archetypes, to represent cycle-exact execution.

The agent added micro-steps during the architecture transition and carried forward machinery from Stage 2. The diary attributes this to the extensive earlier discussion and code about $CLK1$ and $CLK2$. The agent treated explicit phase tracking as essential and kept it throughout the expanded instruction catalog. Thousands of generated opcodes then used incomplete micro-steps alongside explicit phase state machines, redundant buffers and fragmented state transitions. The developer halted feature expansion and directed the gradual removal of redundant phase tracking.

### 4. Stage 4 — The Microcode Archetype Baseline (Cycle-Exact Foundation)
*Timeframe: September 8, 15:43 (commit `81bffec5`)*

After the project removed redundant phase tracking, the CPU core settled on the following execution model.

- **Static Descriptor Dispatch.** A **65,536-entry static descriptor table** (`OPCODE_DESCRIPTOR_TABLE`) replaced the monolithic opcode handlers.
- **Atomic 2-Clock Micro-Steps.** Each native 2-clock slice ($1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$) combines the $CLK1$ ALU phase (`alu_fn`) and $CLK2$ bus transfer in a single atomic struct. This removed separate phase machinery and intermediate queues.
- **Cached Slice Dispatch.** A cached pointer to the active slice (`current_steps`) eliminated table lookups during instruction stepping.
- **Historical SingleStepTests Result.** The early baseline recorded passing all 127 selected suites, covering approximately 300,000 software-generated vectors. Physical hardware validation was outside the scope of those suites.

### 5. Stage 5 — Micro-Step Footprint Compaction & Dual Staging (`addr1`, `addr2`)
*Timeframe: September 9, 00:00 – September 10, 00:09 (commits `139b1016`, `cb463a66`, `3c6d4c9f`, `1d7998dd`)*

The next stage reduced work in the micro-step state machine and made the timing of dual-memory addresses explicit.

- **Aggressive Micro-Step Compaction:** Eliminated redundant dispatch steps and zero-clock transitions across all opcode handlers.
- **The Dual Staging Architecture (`addr1`, `addr2`).** For dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`), the CPU began holding each address in a dedicated staging register (`state.micro.addr1` for source, `state.micro.addr2` for destination). These registers replaced temporary scratch register handling, pointer swapping and bit shifting (`destination >>= 16`).
- **Split Address Error Invariance:**
  - For byte operations (`CMPM.b`, `ABCD`, `SBCD`, `ADDX.b`, `SUBX.b`), the CPU calculates both `addr1` and `addr2` up front (`ea_calc_dual_pi_b` / `ea_calc_dual_pd_b`). Byte accesses never fault on alignment.
  - For word and long operations (`CMPM.w/l`, `ADDX.w/l`, `SUBX.w/l`), the CPU calculates the source address in Step 0 and the destination address during the **CCK2 idle phase of the source read**. If the source address is unaligned (odd), the CPU immediately triggers Vector 3 Address Error and leaves the destination register ($Ax$, USP/SSP) untouched. This behavior matches physical 68000 silicon.
- **Fused CCK Operations:** Fused ALU calculations directly onto natural Color Clock phases (`MicroStep.alu_fn`).

### 6. Stage 6 — Systematic Batch Implementation, Cartesian DMA Contention & Real Program Execution
*Timeframe: September 9, 15:00 – September 10, 12:00 (commits `fd33fe20` through `13f1480c`, `38345d2d`, `4b9377cb`)*

The final stage arrived once the microcode archetype baseline stabilized, focusing on completing the instruction set, stress-testing, and running real compiled binaries:

- **Rapid Batch Migrations.** The project implemented the remaining instruction batches in a 3-hour sprint on the afternoon of September 9. The diary records a 100% SingleStepTests result for this work.
  - *Batch 1.2 (`fd33fe20`):* `AND`, `ANDI`, `OR`, `ORI`, `EOR`, `EORI`.
  - *Batch 1.3 (`3161e834`):* `CMP`, `CMPA`, `CMPI`, `TST`.
  - *Batch 1.4 (`4f6f968e`):* `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR`.
  - *Batch 1.5 (`9079d0be`):* `BTST`, `BCLR`, `BCHG`.
  - *Batch 1.6 (`ef16dda9`):* `MOVE.B`, `MOVE.L`.
  - *Batch 1.7 (`017897b6`):* `MULU`, `MULS`, `DIVU`, `DIVS`.
  - *Batch 1.8 (`6de83cc6`):* `ABCD`, `SBCD`, `NBCD`, `NEG`, `NEGX`, `CLR`, `EXT`.
  - *Batch 1.9 (`2cadb550`):* `DBcc`, `Scc`.
  - *Batch 1.10 (`66325f0f`):* `LINK`, `UNLK`, `LEA`, `EXG`, `SWAP`, `CHK`.
  - *Batch 1.11 (`13f1480c`):* Privileged, `SR`/`CCR`, `USP`, `STOP`, `RESET`, `TAS`, `RTE`, `RTR`, `TRAPV`.
- **Strict 1:1 Mnemonic-to-File Hierarchy (`38345d2d`).** To keep modules focused and individual files small, the developer required a dedicated file per M68000 instruction mnemonic at `crates/m68000/src/instructions/<mnemonic>.rs`, with zero subdirectories and zero umbrella files.
- **Cartesian DMA Contention Test Suite (`test_dma_cartesian`).** This suite checked cycle invariance ($C = C_0 + 2 \times \text{wait\_states}$), Fast RAM immunity and state invariance across the full $2^k \times 2^M$ space of stall permutations.
- **Synthetic Program Execution (`tests/bin/` in commit `4b9377cb`):** Authoring and injecting real compiled M68000 binary routines directly into emulated Chip RAM:
  - `bubble_sort.bin`: Bubble sort on integer arrays.
  - `fibonacci.bin`: Recursive Fibonacci sequence calculation.
  - `sieve_primes.bin`: Sieve of Eratosthenes prime generation.
  - `string_reverse.bin`: In-place string reversal with pointer arithmetic.
  - `factorial.bin`: Factorial computation with 32-bit registers.
  - Running these bare-metal programs confirmed end-to-end multi-instruction sequencing, branch predictability, and register integrity without OS overhead.

### October 11: Timed Transfers, Exceptions and Continuation

The later CPU-CYCLE-01, CPU-PC-01 and CPU-DOCS-01 work reconciled timed reads, writes and internal CCK phases across arithmetic, logic, compare, transfer, branch, frame, return and system instructions. Predecrement calculation was separated from access, long read-modify-write sequences retained their supported word order, and Address Error began at the rejected transfer with phase-correct frame PCs. MOVE/MOVEM completion, CHK/TRAPV timing, STOP suspension, RESET ownership and TAS writes were corrected. Illegal and Line A/F exceptions used the rejected opcode PC; Trace sampled T at instruction entry before an accepted IRQ. External level-seven NMI was excluded by user direction.

The project replaced the Cartesian DMA permutations with retry checks across 52 public CPU transfers (`6df6c3e`). The hardware-derived slot schedule stayed separate from runtime arbitration. The final-source comparison passed 1,000,060 vectors across 124 original files, including 178,087 Address Error entries. All original hashes remained unchanged. Commit and milestone profiles passed, with 128/128 SingleStep groups, 1,002,737 reported executions and two override controls. Existing reviewed ASL.b corrections remained explicit. Electrical pins, device IACK, external BERR and arbitrary in-flight restore required evidence beyond these software-generated vectors and synthetic bus fixtures.

CPU-CONTINUE-01 (`c53eb76`) then added fifteen continuation cases for reset, exceptions, returns, waits, snapshots and following guest effects. SINGLESTEP-MUTATION-01 (`d5a3708`) required exact formal next PC, both prefetch words, timed callbacks and valid vector structure. Its eleven-seed pilot used 1,003 raw mutations: comparison rejected 748 observable changes, format validation rejected 165 invalid copies and 33 equivalent controls passed. Ten affected groups passed 80,650 executions. At user direction, SINGLESTEP-RAM-01 (`6663ab8`) removed final RAM comparison and its Address Error allowances because synthetic writes already followed the timed callbacks; initial RAM ranges stayed validated. Follow-up commit profiles and focused continuation tests passed, but full SingleStep and milestone profiles were not rerun for that follow-up. Physical keyboard timing, external BERR, level-seven NMI and arbitrary active-slice restore remained outside scope.

### October 23-25: DBcc and Opcode-Legality Boundaries

DBcc expiry was corrected to fourteen CPU clocks with three timed reads and wait-state regressions (`2505924`). Motorola PRM review removed six unsupported PC-relative CMPI primary words and aligned disassembly and the local opcode inventory (`9e7efe6`). The software-generated map agreed with dispatch and disassembly across 65,536 words; that agreement was not an independent Motorola-derived inventory. The user closed opcode-legality work without the complete primary-source inventory. Focused CPU, disassembler and architecture checks passed, while the then-current commit profile failed Book Review UI/assets; physical captures and milestone verification were not run for that closure.

---

## 6. CPU Benchmarking: From Generated Cases to Handwritten Workloads

The initial 108-case benchmark catalog generated repeated guest instructions in memory. The runner warmed up the workloads, filtered noise and measured host time, with protected CSV hashes. It supported early CPU profiling. The project later retired the catalog, trace regressions, hashes and analyzer. Its measurements and reported anomalies describe that historical runner. Current comparisons use the later protocol.

A decoder-derived CPU-BENCH-01 pilot on October 12-14 enumerated 45,821 legal primary opcodes and measured 4,571 representative workloads, with 27 targeted retries and 47 NOP controls. All workload replays matched state and CPU clocks; the recorded full SingleStep run passed 1,002,739 executions. Its commit profile still failed then-existing Book Review errors and an RAG adapter timeout. Its fixture and NOP-control protocol differed from the later program-level measurements, so those numbers cannot be compared directly. The decoder-derived runner was subsequently removed from the active benchmark path.

October 24-26 work replaced generated candidates with handwritten, assembler-backed programs in `benchmarks/m68k`, including branch, counter and conditional-store paths (`7412408`). The reviewed full run completed 2,257 cases and 6,771 timed series without execution failures or persistent load disturbance. Per-series timing, load checks and bounded retries made the measured path reviewable. The user approved replacing the reference; tracked hashes and source-run identities matched that run, while raw host-specific reports became local. A later approved snapshot refreshed 86 cases (`98da1ad`) without repeating the complete measurement. Focused Python/Rust checks passed; the then-current commit profile failed Book Review contracts, and full SingleStep was not rerun for the later snapshot. One catalog commit explicitly waived tests and was not remeasured.

The performance specification and benchmark README govern the current workflow. Compare measurements only when program source, configuration and timing protocol match. Retired traces, hashes and profiling tools document earlier work. The project retired those commands and uses the later protocol for current baselines.

---

## 7. Subsystem Evolution: The Developer Studio GUI & Debugger Engine

While CPU benchmarking was under development, the project built the frontend in the dedicated `feat/gui` worktree (`315c602c`).

### From "Make Me a GUI" to a Developer Studio

A first voice request produced a basic register window. Through repeated reviews,
the developer and agent expanded it to include register and microcode inspection,
disassembly, an Amiga image, memory editing, breakpoints and an execution-history
control. The GUI kept a separate presentation view while making machine activity
visible to the developer. The GUI documentation describes current shortcuts and
layouts.

### Headless Interaction Testing

Because egui accepts explicit input and context state, interaction tests could run
in process without an OS window. This supported repeatable checks of focus,
keyboard commands, edits and panel geometry without external UI automation. Such
checks do not prove native event delivery, GPU output or browser behavior. Later
worker ownership also introduced asynchronous image delivery: command completion
alone did not establish that a new frame had reached the GUI.

### Debugger and Disassembler Boundaries

CPU history uses bounded storage and restores CPU snapshots; it does not
restore guest memory or the complete machine. The original time-travel description
was broader than the implemented capability. Full rewind and replay remain separate
roadmap work.

The embedded mini-assembler permits guest-memory patches. Conditional breakpoints
inspect execution and memory activity. Extracting disassembly into its own crate
made decoding reusable by the debugger, tracer and GUI and retired the former
large-file exception. The separation and its rationale matter beyond the current
file names or line counts.

---

## 8. Architectural Rules, Anti-Tamper Policies & The Living Definition of Done

As the workspace grew, the project turned architecture decisions into written rules and executable checks. These required cohesive source files, flat CPU instruction ownership, named hardware operations and allocation-free hot paths. They prohibited custom macros, const-generic instruction handlers and runtime host panics. The rules protected vectors, timing references and benchmark baselines against changes made solely to pass a test. The September 19-21 compiler work made warnings fatal and removed artificial encapsulation requirements that obscured straightforward hardware state.

The enforcement mechanisms changed with the project. Early Rust architecture tests, automatic coupling rules and merge-only guidance were later replaced or narrowed; their old lists are not the present contract. [Chapter 14](#14-sdlc-agent-workflows-and-development-harness) records that evolution and its evidence. [AGENTS.md](AGENTS.md), the owning rules and the verification README define the current obligations.

---

## 9. The Ultimate Goal: The Clean-Room Re-Generation Experiment

The design notes retain hardware contracts, consequential decisions and known limits. Git and this diary retain the development history.

This serves a larger, ambitious vision:
- Once the complete, working Amiga 500 emulator is finished and verified against all reference suites,
- We will wipe the implementation source code (`crates/`) and reference emulator sources (`ref_src/`), leaving strictly:
  - The refined prompt and skill system (`.agents/`).
  - The curated, de-duplicated design documentation (`Obsidian/Amiga/Design/`).
  - The automated test suites (SingleStepTests, vAmigaTS).
- **The Experiment:** Can an AI agent, armed with the lessons, rules, and architecture recorded during this project, autonomously re-synthesize the complete cycle-exact emulator from scratch?
- That was the original ambition, not an instruction to delete the working repository. The currently scheduled experiment is an isolated, single-subsystem trial in [Roadmap Step 9](ROADMAP.md#step-9-documentation-to-code-regeneration-experiment).

This diary records how the project built those foundations.

---

## 10. Accepted-Recipe Reconstruction and Reference Publication

On October 29, 2026, `ebc99c8` enabled Book Review to reconstruct a book from an accepted `book.json` recipe and its matching source PDF. It also enabled publication of reviewed Markdown with stable chapter links and image descriptions. Test Book certification and closure of the four-manual conversion stage required separate acceptance. In September and October the project replaced several earlier mechanisms so that preparing sources, running the pipeline and accepting the result each had its own boundary.

### Source Preparation and the Rejected Pipeline Assumptions

The September 15-16 converter began as a fourteen-stage multimodal pipeline with Gemini OCR, normalized page boxes, table reconstruction and text descriptions for diagrams. The project recorded a trial on twenty selected pages. Full-book quality still required its own assessment. On October 5-6, the project changed how the converter sent model requests. It used subscription-authenticated Codex requests with explicit model and reasoning effort settings. Cache replay avoided new requests for identical inputs. Acceptance still required checking fidelity to the source.

PDF-TEXT-1.1 (`5a9add8`, October 6) made the reopened native/OCR PDF the source for Stage 01. It replaced cached OCR JSON as the text-layer artifact. The change tied physical page IDs to text geometry and PNG transforms. PyMuPDF's q/Q wrappers were permitted only around unchanged source streams, with identical selected-page renders. The user closed the task using page-64 evidence corresponding to source-manual page 5: the source hash and all 160 page geometries were preserved, and Stage 01 extracted twenty blocks without inference. The OCR result came from one live request before cache reuse. Focused PDF/shared suites passed nineteen tests each, but the milestone profile failed 62 existing reference links and 22 unchanged Rust conditions. Native-copy, mixed, blank and real rotated/cropped categories remained unverified. This was user-directed scope closure, not a passing repository milestone.

Later preparation moved to local Tesseract through PyMuPDF, retaining source geometry over graphics. PDF-STATE-1 replaced manifests, lineage validators, snapshots and manual handoffs with stage-owned artifacts and atomic execution status; those earlier safeguards were removed by user direction rather than retained as a compatibility layer. Named attempts preserved their configuration, restart cleared the chosen stage and successors, and downstream selection retained physical page identity. Historical closure checks passed PDF/shared/publication suites at 25/21/10 tests and architecture at 19/19; migration retained 645 result files without OCR or model calls. Publication was explicit, protected working sources and accepted only absent or empty destinations. None of this proved whole-book reconstruction or bulk publication.

### Page Objects, Ordering and Cache Identity

October 7-10 work established source-backed page objects, graphic/prose separation, table spans, code roles, optional rotation and review against original-page geometry. Filtering deliberately removed page furniture, selected front matter and standalone margin logos while preserving source tables, character diagrams and drawing sheets. Independent descriptions remained separate from coordinated diagram labels. HTML tables received saved RAG text; both table formats retained source-image companions. Final book assembly moved out of PDF conversion into Book-to-Markdown.

PDF-ORDER-2.46 was closed by user request without certifying all reading-order observations. A subsequent deterministic row-grouping and left-to-right ordering stage was also removed at user request because geometric sorting could override intended source order. The retired stages and their tests are unavailable; their historical passes do not certify current ordering.

Compact JSON omitted defaults. Cache identity depended on prompt bytes, serialization order, schema, model, effort and images in addition to field values. Replacement fields therefore retained contract order. Temporary aliases were limited to verified equivalent requests and then removed; changed page-54 geometry was retained as a legitimate miss. A historical audit found 138 Stage 02.42 hits with no misses, followed by 81 Stage 02.44 hits and that one legitimate miss. Source-hash variants kept source identity separate from response reuse. The October 28 relocation (`e35239e`) preserved 6,694 files, including 4,775 cache entries, without changing response identities or running a new conversion/import.

November 10-11 changes `47580a8` and `d8c9e33` extended reuse to completed PDF preparation, per-page PNG/text bundles and graphic crops. Cache entries linked source bytes, effective settings and runtime identity to output hashes. An existing file alone was insufficient for reuse. Crop identity included the source PNG and bounding box. Stages 04, 05, 07 and 08 stored only changed pages, while readers selected the latest replacement over Stage 03. An explicit empty filtered page prevented excluded objects from returning. These changes reduced repeated preparation without certifying transcription quality.

Temporary capacity failures moved to fresh transports with exponential backoff that could be interrupted; budget failures remained terminal. Coordinator cancellation interrupted worker requests, delays and retry waits after a regression reproduced an unwanted extra request. On Windows, a system execution request prevented idle sleep during selected pipeline stages and restored the previous state on exit. PDF conversion and table defaults used high reasoning effort. Historical checks passed 48 shared, two HTML, 48 PDF and 192 Book Review tests; the initial commit profile failed only on five vocabulary terms, then passed after explicit user approval. Sparse-page follow-up checks covered 31 PDF execution/cache, six proofreading, two parallel-table and four import tests. No new book conversion or import was run for those deliveries.

### Working, Accepted and the Test Book Decisions

Authors edited in Working and explicitly chose when to move a result to Accepted. The editor supported guarded Codex edits, source selection, merge controls and chapter organization. On October 23-25 the project selected the 308-page A500/A2000 OCR PDF as the source and kept the other manuals as local references. A fourteen-chapter, 975-element Accepted model later replaced the initial one-chapter recipe. The user closed the stage then numbered Roadmap 1.1 while description, proofreading and standalone reconstruction stayed separate. Although 102 editor contracts passed and the recipe hashes matched, a later source attribution audit found 319 invalid fields. The commit and milestone checks still reported English/PDF failures.

October 26-27 work recorded the rebuilt PDF identity `78e52f075e17`, a 999-element recipe and source-backed substitutions, accepted literals and visible source differences. The tracked recipe then differed from ignored Accepted in 74 elements. Stage 08 introduced text-first proofreading with optional page-image resolution and validated `md_text` patches before Book import, replacing chapter proofreading. At those commits, no live Stage 08 conversion/import or chapter-quality review ran, and commit profiles still failed English, with an earlier PDF `KeyError: 'blocks'`.

On October 27, `570d578` replaced import matching with page-local source alignment and published the current Accepted recipe. The user accepted chapter formatting and short edited fragments while retaining authored source references, closing then-Roadmap 1.3/1.4 by direction. Book Review passed 160 tests, and the root/Accepted and PDF hashes matched. Semantic attribution and standalone reconstruction were still unverified. The source audit found 56 fields requiring attention among 940 fields/26,360 references. Commit/milestone English checks and five semantic checks failed. Standalone extraction and the four manual recipes remained pending. Chapter/contents renaming, scoped image/table descriptions and active-request cancellation followed in `ac53f5e`, with 182 Book Review and 44 shared conversion tests passing; no new conversion/import ran for that delivery.

On November 12, `783bb8a` added a read-only inventory of unmatched text with source context and navigation. Source alignment retained code and numbered content, joined local hyphenated continuations without absorbing a fused OCR glyph, and preserved offsets through multiline math and HTML. Resolved text was cached by recipe and source-page version with replacement-race checks; incremental viewer updates retained unchanged chapters, resources and reading geometry. Working revision checks still guarded acceptance and discard. Initial English findings were resolved through approved API vocabulary and file-specific test/regex exceptions before the full commit profile and independent review passed. Focused cache, browser, alignment and recipe suites passed 7, 6, 31 and 37 tests respectively; server availability was checked separately from content quality.

Integration in `45f9094` exposed another source-mapping defect. Matching raw backticks misread escaped delimiters, joined markers across separate inline containers and chose an occurrence of repeated text from the wrong table cell. The repair used the parser's code spans, mapped every inline container in source order and decoded recipe escapes while preserving cleanup offsets. Alignment, cleanup and the unmatched-text inventory then used the same corrected boundaries. At the time, 33 alignment tests, 39 recipe tests, all 222 Book Review contracts and the full commit profile passed. The delivery also received independent extended approval. A new conversion or reconstruction, full SingleStep and the milestone profile were outside those checks.

### Portable Reconstruction and the Remaining Manual Recipes

The accepted recipe became the portable handoff between clones. Reconstruction uses the identified source to rebuild the editor's local pages and assets while retaining the authored recipe. Authors still edit in Working and decide explicitly when to accept changes. Bulk description tasks covered only visible Working elements. Publication and RAG indexing remained a downstream user workflow. The same delivery added a structural frontmatter gate for local reference Markdown to the commit profile. The gate checks metadata. Source fidelity still required its own assessment.

The recorded Test Book replay completed PDF stages 01-08 with 545 cache hits and zero new model calls, then imported 993 elements. Source validation still reported 56 invalid fields in that raw import. The user retained the fresh import. Source acceptance remained open. The closing commit records a passing commit profile, 35 design specifications with 691 resolving links, diary/roadmap checks and valid frontmatter in 104 local Markdown files. These are historical results for that delivery, not checks rerun during diary compaction.

November 11-12 artifact commit `3d54d55` recorded the A500/A2000 accepted recipe and Hardware Reference Manual OCR provenance without tracking local PDFs or generated review bundles. The 2,000-element recipe matched Working and Accepted, and source/prepared hashes matched their identity records. An initial audit resolved 60,164 references but reported 112 literal-content or unsupported-math fields. Thirty edited fields raised the resolved-reference count to 60,546, with 107 such fields remaining. The user explicitly accepted known source-occurrence mismatches on pages 149, 150, 183 and 242 for this artifact; the general attribution requirement was unchanged. The commit profile and independent review passed with that exception. Valid addresses and matching hashes did not establish correct printed occurrences, transcription quality or clean reconstruction.

The user directed removal of the roadmap pilot. The fresh Test Book lacked an Accepted bundle and had yet to demonstrate a clean reconstruction gate. The remaining repository deliverable was four accepted manual recipes with matching source identities. It is now tracked in [Roadmap 11.1](ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion). Full-book quality, downstream publication and RAG ingestion were not established by the tooling delivery. These unresolved limits remain attached to the narrative; they do not make every settled converter change an active rolling entry.

### November 13-16: Manual Artifacts, Math Provenance and a Shared Catalog

The Hardware Reference Manual recipe recorded 2,508 elements from 405 PDF pages.
Working and Accepted matched, all eighty graphics loaded, and source/prepared
hashes matched metadata. Its audit resolved 60,752 references but still reported
158 literal-content findings and seven math commands lacking source references.
A validator repair separated italic/spacing syntax from visible content: the
regression failed first and 108 focused tests passed, removing eighteen formatting
findings without waiving the remaining source findings.

Motorola programmer-manual preparation covered 646 pages. Importer correction
`45a1302` decoded named/numeric angle entities only inside parser-owned math spans,
retaining code, preformatted text, non-math entities and source offsets. It fixed
KaTeX rendering without changing PDF-to-Markdown prompts. Forty-one recipe and
33 alignment tests passed after a failing regression; the reimport contained
5,334 elements, Working matched Accepted and rendering completed. Programmer and
revision-8 user-manual artifacts then joined the tracked recipes. The accepted
user manual contained 1,343 elements and 100 assets with a matching PDF identity.
Artifact matches did not certify whole-book transcription or reconstruction.

The catalog server (`fb5a48b`, November 15-16) replaced per-book launch arguments
with browser selection. Requests, render queues, PDF page and pane width belonged
to the selected book so separate tabs could not redirect edits to another book.
Catalog contracts and PowerShell checks passed; five books were discovered and the
two opened books had no pending render/error. BOOTSTRAP-NATIVE-INTEGRATION-20261116
preserved this delivery alongside native disk execution. Earlier integration
`0e6438e` passed seventeen commit groups and 224 Book Review tests; these were
integration checks, without a fresh whole-book quality assessment.

The November 16 updates pinned KaTeX 0.18.2 and Pillow 13.0.0 for the reported
security advisories. Native/browser builds and syntax/compilation checks ran, but
automated suites and review were explicitly skipped for those commits. Image smoke
tests lacked local PNG assets, and the shared Python environment kept its existing
Pillow because other installed tools required a version below 13. These limits
remain separate from the recipe and importer results.

---

## 11. Register Access and Delivery to Owning Chips

On November 11, 2026, `c83f046` closed Roadmap 2.1 within the scope the user explicitly approved for documentation and delivery (`REGISTER-ACCESS-CLOSE-20261111`). The work followed direct CIA dispatch (`CIA-DIRECT-DISPATCH-20261111`), direct custom dispatch (`CUSTOM-DIRECT-DISPATCH-20261111`) and separation of private register updates from engine execution. The resulting contract identifies every modeled register, how it can be accessed, where writes go, which queue applies and which component owns it. The eventual effects in each receiving engine required separate verification.

Named readers and queued setters replaced generic changes by motherboard address for thirty canonical CIA addresses and 180 custom write offsets. A shared write checks its recipients before proceeding. Agnus owns the queue for its Copper and Blitter children. Separating the register modules preserved all 221 existing method bodies and signatures. CPU access to a CIA still separates synchronization and acknowledgment from the falling edge of E. Capture occurs on that edge, and overlay routing follows in the same phase. Pending bus, TOD and output state survives continuation. Reset cancels pending work. The project removed polling at restore time because it consumed events before they were routed.

A regression reproduced unintended drive selection. The repair made effective Port-B output respect DDRB and pull-ups. Related corrections covered LOAD/unused-bit readback, TOD middle-write halt, pending ICR mask enable and one-shot timer-high loading. Palette/sync requests gained a positive provisional CCK delay. Due events retain issue order. A full queue rejects writes without applying them early, and rejected source outputs are retained. Paula now replays all due writes, including the ninth that the previous implementation truncated. A merged interrupt-input destination remains separate from CPU INTREQ. The obsolete audio location setter/fixture and unused Denise inspection decoder were retired; Agnus retains audio pointer coverage. The user restored snapshot header version 1 for the evolving development format without an old-format decoder.

Motorola's E-clock description, vAmiga's provisional waits and the replacement FPGA project's CIA capture/counting behavior informed the model. Queue delays, mutation modes and issue order remain model settings rather than measured silicon timing. Nine inert custom offsets, unresolved CIA PRA/PRB pin reads and missing engine effects were explicitly handed to later work; Gary decoding and board signal transport were separate stages.

Historical delivery checks covered all 256 custom slots, all 180 writable offsets, word/both byte lanes, live/debug readers and PAL/NTSC. The final MemoryBus wiring suite passed 16 tests including 1,536 write transfers; preceding focused suites passed 168 tests before the last read-selection addition. CIA coverage included canonical addresses, five E phases, CPU retries, continuation and queue boundaries. The closing commit records passing commit/milestone profiles, 128/128 SingleStep groups with existing reviewed ASL.b corrections unchanged, strict Clippy, native/WASM checks, vault audits and independent approval of the exact staged snapshot. Static register/map checks covered no signal/quirk sentinels. Unknown hardware vocabulary was added only after explicit user approval. RAG retrieval was insufficient at that time, so targeted local references supplied evidence. No firmware, operating system, vAmigaTS execution or silicon calibration was claimed.

---

## 12. A500 Gary Address Decoding

On November 11, 2026, `e7d870e` closed Roadmap 2.2 (`MEM-GARY-01`). Original-code failures first reproduced missing CIA aliases, dual selection, opposite-parity writes, custom DE routing and physical bank classification. The repair changed the decoder and the production bus paths within the existing installed memory boundaries and CCK wait model.

Banks A0-BF route through independent A12/A13 CIA selection using full canonical register identities. CIA-B supplies the high read lane and CIA-A the low lane; byte writes broadcast to selected chips regardless of parity, while word writes retain each chip's lane. DE and DF share custom live/debug/queued paths. Selected CIA receivers synchronize together, and CIA-A PA0 changes affect overlay only after commitment. Inactive read lanes retain FF and unmapped reads FFFF; Slow/Fast RAM, RTC precedence and physical DMA storage remain separate from device aliases.

By explicit user decision, the redundant CIA bus capacity preflight was removed while sixteen mutation slots and direct-setter overflow rejection remained. Named CCK constants express the existing wait schedule, an optional countdown was simplified and an obsolete CIA debug-byte helper was replaced by the production word-debug API. The source review distinguished canonical HRM addresses, A500 Rev. 6A/7 wiring and differing reference-emulator equations. Extra mirrors and measured timing calibration were deferred. Full CIA CPU-cycle timing in this CCK model, including the hardware 10-19-clock range, remained unverified. The project left it for later signal and guest verification.

Historical focused checks passed 47 CIA/MemoryBus tests and twelve production CIA/Gary integration tests across PAL/NTSC and all three memory presets, plus strict Clippy and machine WASM compilation. The user specifically authorized nine existing production-machine ADF/action regressions with a stop on errors; they passed, including disk DMA, while the floppy crate itself contained no tests. This exception did not authorize new media or firmware work. Static checks covered 196 offsets, twelve map boundaries and 26 crate relationships, with no signal/quirk sentinel coverage. The closing commit records passing commit/milestone profiles, 128/128 SingleStep groups and independent extended approval. Complete hardware timing and attached-device behavior were not established by decoder closure.

---

## 13. Receiver-Owned Signal Transport and Continuation

On November 11, 2026, `e3ca26c` closed Roadmap 2.3 (`SIGNAL-THREE-REFERENCE-20261111`). The work established how motherboard signals reach their destinations. At defined CCK phases, each receiver captures inputs in state it owns. Reconstructing the engine that acts on those inputs and checking the complete exchange as seen by a guest program were separate stages.

Agnus retains beam-counter ownership and sends RGA synchronization events into receiver-owned state. Register contracts distinguish immediate address strobes from queued data writes. Pending delivery can be serialized and restored without polling. Pure machine snapshots were separated from the higher-level save-file and preview format. CIA-B PA0/SP and PA1/CNT wiring was completed in both directions after a regression reproduced the missing reverse route. Obsolete direct SDR-byte injection and unused pin helpers were retired with their callers.

Twenty focused machine transport and pin-propagation tests passed across PAL and NTSC. The recorded checks also passed workspace all-target Clippy with denied warnings, machine_loop/save_state WASM builds, configured semantic checks and a 35-note vault audit with 999 links before closure edits. Thirteen baseline RAG contracts passed in isolation from a separately staged adapter repair. The closing commit records passing commit/milestone profiles, 1,002,737 SingleStep executions with two existing reviewed overrides and independent extended approval of the exact committed snapshot. Those CPU vectors do not certify electrical pins or complete peripheral exchanges.

Device-engine reconstruction remained in 2.4 and functional guest verification in 2.5. Serial and parallel ports remained disconnected; deterministic pin tests did not prove communication with attached devices. Firmware, media execution, vAmigaTS and measured silicon timing remained outside this closure. The separately closed timing research was not reopened. Chapter 14 records the later decision to remove a separate pre-reconstruction propagation gate; transport closure did not establish that proposed audit's result.

---

## 14. SDLC, Agent Workflows and Development Harness

Between September 12 and November 21, 2026, the software development life cycle (SDLC) became a project in its own right. Rules and skills, agent configuration, reference retrieval, isolated worktrees, test orchestration and protection for verification evidence evolved alongside the emulator. The project assigned clear ownership and review requirements to the implementation and its verification criteria. Chipset behavior milestones required separate closure. This chapter records the settled tooling decisions. The linked rules and tool documentation govern current procedures.

### From Implicit Guidance to Owned Rules and Skills

September work split shared instructions into focused rules (`8ea3676`), introduced reproducible-defect investigation and the Platform Quirks Catalog (`3781b8f`), and added code, hardware and documentation audits. Compiler warnings became fatal, while rules that forced artificial encapsulation around hardware state were removed. These mechanisms made some invariants executable; passing their static checks never established all hardware behavior. Human-readable design and semantic review remained necessary.

On October 5, `da87753` migrated agent roles and MCP registration to native Codex TOML. It removed hook wrappers, installation guidance and obsolete catalogs while retaining explicit verification commands. `66f642d` then clarified ownership. AGENTS.md routes shared obligations, rules state requirements and exceptions, skills own procedures, and scripts and tests own mechanical checks. Standalone HTML/PDF conversion programs replaced duplicated agent procedures. Scope control distinguished an inspection from permission to edit and a useful suggestion from an authorized feature. Closed execution plans became disposable after their durable closure record was committed, without deleting unrelated or unfinished work.

On November 8, `99c8e39` consolidated scoped specification compliance and independent review for every authorized commit, including documentation and tooling. The intended source snapshot, verification evidence and review verdict had to agree. Accurate specifications stayed unchanged; compatible implementation detail could be enriched, while conflicting requirements and protected references required user decisions. Obsolete instruction/CPU/test-runner skills were retired rather than retained as alternative command paths. HTML attachment tests moved to their owning tool and verification routing. Approved frontmatter repairs restored metadata in 66 ignored reference chapters per checkout without changing chapter text; metadata presence did not establish source fidelity.

### A Single Verification Harness and Explicit Evidence Boundaries

The first preflight script (`b334e13`, September 12) and the later tools/harness layout (`3a85b76`, September 15) combined compilation, architecture and quality checks. On October 5, the project removed rules that mechanically paired source files with tests and imposed assertion quotas. Changing a source file alone was insufficient reason to add a corresponding test. October 9 verification work introduced unconditional standard checks, separate selections for commits and milestones, independent diagnostic sets and execution before staging. On October 10, `8280791` centralized verification in tools/verification, with shared utilities under tools. Python architecture checks replaced the old Rust architecture suite.

The resulting commit profile covered ordinary crate tests, coverage and standard quality/tool contracts; milestone verification selected full SingleStep rather than treating every routine commit as a full CPU run. Focused tests served development. The old vAmigaTS capture/execution infrastructure, chipset throughput workload and obsolete benchmark traces/hashes were removed; their recorded results were historical, and native external-suite execution required later firmware/media bring-up. Source comparison, software-generated vectors, synthetic bus tests and measured hardware each had different evidentiary limits. Chapter 5 records the CPU comparison and continuation decisions; Chapter 6 records the replacement performance protocol.

The October 10 final commit profile passed its selected Rust, quality, language and five Python contract suites; full SingleStep, new performance measurements and the milestone profile were not selected. The October 27 divergent bootstrap merge initially failed English, then passed the full selected profile after explicit vocabulary/literal decisions and a prompt correction. Those later passes did not retroactively turn earlier failed converter or milestone checks into passes. The November 8 workflow delivery records a passing full commit profile and independent extended approval, with no roadmap milestone closed.

### Environment Isolation, References and Retrieval

Worktree automation initially linked ignored assets through NTFS junctions (`acb25d7`, September 14). A change in one checkout could therefore affect another. On September 17, `f4eb29e` replaced the links with physical directories and copies. Later provisioning seeded worktrees with Graphify state, and November work included pinned FPGA references. On October 14, `5cbbc02` made fast-forward reintegration the direct path. Divergent branches still needed conflict reconciliation and verification. The project later favored native Git and original logs for routine commit evidence, reducing reliance on helper scripts and copied manifests.

September 16-22 retrieval work combined local documentation search with searchable diagram sidecars, then made the standalone CLI the indexing authority. Design and Reference had explicit scopes, .obsidian was excluded, and external DevNotes material stayed retrieval-only. The MCP adapter delegated rather than implementing another indexer; its reindex entry point was removed. October work separated service-owned read-only retrieval configuration from indexing state. Image descriptions belonged to conversion/publication rather than being regenerated for each query.

Pinned MiniMig and Amiga Replacement Project sources and measured Techtravels evidence were added in `8ba6def` on October 28, replacing the duplicate ignored vAmiga tree with one canonical source. The research distinction was deliberate: measured signals, reconstructed FPGA circuitry and emulator models could inform one another but were not interchangeable proof. On November 11, `3a88406` repaired shared-reference retrieval across checkouts and explicitly decoded CLI output as UTF-8. Fifteen adapter/launch contracts passed, followed by recorded commit verification and independent review. A hit in the shared index still needed filtering by source and path. Retrieval quality and session activation required evidence beyond process success or MCP configuration.

### Protected Numerical References and Named Hardware Meaning

The BLEP part of `99c8e39` on November 8 made numerical regression independent of optional WinUAE chart generation. Offline CSV tests protected the retained curves; moving eight numerical tests out of production source preserved their bodies and four production implementations. Four CSV hashes and the committed SVG stayed unchanged, and a one-time WinUAE comparison regenerated identical plotted paths using an explicit source input. Focused BLEP checks and all-target Clippy passed. This protected numerical evidence without establishing physical speaker output or measured board audio.

On November 12, `d2a9f6d` required named REGISTER_ identities and BIT_/MASK_ encodings, and explicit approval for architecture-check changes or weakened verification gates. Implementation `2957128`, split from policy at user request, migrated unnamed encodings and their callers without changing runtime behavior. Shared disk/interrupt definitions used two approved configuration dependencies; four semantic-parser/source-selection changes and identifier corrections were also explicitly approved. The static audit covered 196 register rows, twelve memory boundaries and 27 crate names, with no signal/quirk sentinels. The first implementation profile failed English, followed by a passing focused scan after approved corrections; full SingleStep was not run for this refactor. Naming clarity and static agreement did not substitute for behavior tests or permit silent baseline changes.

### Simplifying Guidance Without Claiming Proven Equivalence

November 10 changes `c1952f5` and `65da673` shortened approved Rust, API, documentation, testing and PowerShell guidance and simplified commit/review reporting. Git identified the source snapshot; existing tools and original logs supplied evidence. Verification and review could overlap only where their results still applied to the same final source. Mandatory gates, independent approval and limits of CPU observations remained.

Three screening rounds supported individual instruction replacements. The third recorded nineteen correct results, one preserved arithmetic failure in a host fixture and matches for six approved replacements. Local link and whitespace checks passed. The project had yet to check the combined behavior. Establishing that shorter instructions reduced cost or preserved all possible agent behavior would have required broader evidence.

### Roadmap Ownership and the Reconstruction Verification Order

Roadmap consolidation `9206fae` on November 11 moved diagnostics, remaining implementation and acceptance evidence into their owning stages. Specifications retained hardware contracts and concise caveats rather than duplicate task lists. The review passed 35 design notes, 832 links and changed-document roadmap anchors without executable changes or milestone closure. The resulting scope separated 2.4 engine reconstruction, 2.5 guest/propagation acceptance, later firmware/media/native suites, and deferred peripheral/bus work.

On November 12, `0dc9126` scheduled REGISTER-PROPAGATION-VERIFY-20261111 as a separate gate before reconstruction, while BOOK-GUEST-PROGRAM-TESTS-20261111 remained after it. The same change excluded local .agent/tasks plans from language scans, including explicit-file scans; 87 verification-tool tests and the repository English scan passed. The scanner change did not execute either chipset task. The user then approved removing that separate propagation prerequisite in `0969545`: component unit tests accompany reconstruction in 2.4, and functional propagation/guest acceptance stays in 2.5. Access/Gary/signal foundations remained established, but no removed gate was declared passed and no new hardware milestone was closed.

On November 15, `aaa12c1` gave priority to original disk execution. The user withdrew
BOOK-GUEST-PROGRAM-TESTS-20261111 and its obsolete plan without declaring guest
acceptance. Missing media-stream/DMA behavior moved into native bring-up; native
verification retained the retirement of `examples/m68k/`. Lockstep tracing became
a diagnostic contingency after native tests rather than a prerequisite. Chapters
15-16 describe the earlier stage boundaries and the subsequent delivered path;
Roadmap 2.6 owns the remaining native-suite acceptance.

### Logging, Compaction and the Permanent Record

The diary appender arrived in `12eb591` on September 12. Policy briefly limited entries to verified roadmap/milestone closures (`a1e3fc3`, September 25); `4a986fc` restored concise per-commit entries on October 5, with compaction at verified roadmap closures. The November 12 change `0969545` added completed-stage chapters and a stable, final unnumbered Rolling Changelog. Its commit profile and focused diary/roadmap/skill checks passed, with independent approval; it closed no implementation stage.

The later editorial decision was to use that changelog as a buffer for work awaiting resolution or narrative integration. Durable decisions and evidence move into the owning chapter, including this SDLC history; routine mechanical changes, merges and compaction statistics remain in Git. Existing digests have no automatic right to permanent retention, and mentioning an open roadmap stage does not make a settled tooling change active. Granular investigation stays only while needed to continue unfinished work. Historical failures, user-directed scope closures and unresolved limits remain visible in the relevant narrative even after their original entries are removed.

### November 13-16: Selecting Work, Reusing Evidence and Simplifying Ownership

VERIFICATION-SELECTION-20261113 selected changed Cargo packages and their transitive
consumers, with conservative full-workspace fallback for unknown/shared inputs.
Detailed LLVM gap reports identified their measured scope and excluded retired
objects; coverage guided meaningful missing behavior, without assertion quotas.
The motivating cached profile spent 78.59 of 88.08 seconds in coverage and standard
tests. A focused BLEP selection passed in 4.24 seconds; this demonstrated selection,
without measuring a whole-emulator speedup.

FAST-TURNS-BATCH-COMMITS-20261114 moved preparation ahead of expensive verification
and reused successful independent groups only while their inputs matched. A real
Cargo/LLVM probe ran three commands initially, zero after a diary edit and three
after a source edit; corruption, build settings and HEAD changes still invalidated
evidence. Independent review, graph refresh and verification could overlap on a
stable candidate, while shared Cargo/coverage outputs stayed serialized. Three
November 14 orchestration/guidance concerns were committed without tests, scans or
review at explicit user direction. Language review moved between commit-only and
per-round boundaries during this period; these superseded procedures are history.

LOCAL-INTEGRATION-20261115, TASK-CLOSURE-20261115 and
git-close-task-batch-integration-20261116 introduced exact source/target selections,
combined verification, conditional Git publication and retryable cleanup. An
explicitly unverified working commit could preserve work before the combined check.
Native reference locks protected against competing publication; source-only approval
did not establish an integrated result. Cleanup policy also changed: ignored files
briefly became disposable by user decision before selected input/output preservation
returned through explicit archive operations. These transaction tools document the historical implementation. A simpler revision
was in progress.

Windows command-tree supervision briefly used Job Objects, a guardian and a narrow
MSVC telemetry exception. Empty readiness files exposed a race in the fixture.
Publishing the complete PID atomically preserved the lifetime assertions.
VERIFICATION-LOCK-SIMPLIFICATION-20261116 then removed command-tree tracking and its
tests in favor of one nonblocking OS lock per checkout. It retained mutual exclusion
and evidence reuse; surviving children were no longer detected or killed. Two hundred
focused Windows contracts passed; POSIX execution was unrun at that boundary.
INTEGRATION-ALL-20261116 (`e3093cd`) subsequently passed the combined commit profile
and four independent reviews for seven source branches, closing the selected memory,
fixture, timing, locking and parallel-runner tasks.

### November 16-21: Parallel Verification and Test Value

PARALLEL-TEST-EXECUTION-PILOT-20261116 used nextest 0.9.152 with sixteen slots, four
GUI cases and explicit serialization for conflicting fixtures. With identical
fixture repairs and 790 selected identities (788 active, two asset-dependent skips),
warm coverage reset/test/export fell from 280.403 seconds under Cargo to 134.418
under nextest; a second nextest run took 135.350. Coverage compared 207 files with
identical function/line totals and two differing asynchronous spans. No threshold
or exclusion changed. This measured ordinary coverage, not the entire verifier or
other operating systems.

TEST-RUNTIME-AND-VALUE-20261116 also retired thirteen presentation/pacing campaign
cases by explicit user decision, retaining guest assets, manual measurement examples
and CPU/chip/bus/input/host correctness checks. Reports distinguished executed,
reused, failed, missing and skipped work, and concurrent command durations from
invocation elapsed time. WORKFLOW-ARTIFACTS-20261118 (`db4336c`) moved timing reports
out of tracked candidate content into per-attempt outputs with locators/hashes and
retained selected original bytes before teardown. Reports described their inputs;
they did not define new acceptance criteria.

WORKFLOW-CORRECTIONS-20261118 moved Python cases to isolated processes/directories,
restored independent Git fixture copies, ran marked cases exclusively and rejected
missing outcomes after abrupt worker exit. Affinity-aware concurrency left host
capacity available; interruption stopped peer workers and retained collected
results. Final 267-case trials took 241.3 seconds with one worker and 41.2, 39.0 and
40.3 seconds with sixteen. Focused checks passed on Windows and Ubuntu under WSL;
fixture archives rebuilt byte-identically on both. Generic POSIX cleanup under WSL
left a child alive during shutdown interruption. The owner accepted that limitation
and waived four final approvals for `716fd90`; native macOS/BSD remained unrun.

WORKFLOW-TEST-PERFORMANCE-20261121 (`9606c62`) extended the runner to capture/routing,
preserving live Git operations on isolated copies. Shared-module changes selected
all three Python suites. The first focused run passed 386/387: a verifier case
exceeded its ten-second startup deadline before testing lock recovery, then passed
alone and in the 306-case retry. Capture/routing passed 39/42 cases on their first
attempts. The closing commit records all 387 cases and fifteen commit groups passing.
Original durations were retained rather than rerunning tests solely to time them.

### November 16-18: Independent Graphs and Local Worktree Inputs

GRAPHIFY-REFRESH-PERFORMANCE-20261116 memoized paths within Graphify 0.9.74 refreshes
without reducing the corpus/resolver context or modifying the installed package.
Comparable four-worker pairs fell from 42.155 to 34.367 seconds unchanged and
54.128 to 46.910 after one Rust-function edit; complete graphs, attributes and
communities matched. Earlier sequential/parallel mixtures were invalid comparisons.
Twenty-four parser omissions remained, and shared-host timings did not establish
a typical speedup.

GRAPHIFY-TWO-GRAPHS-20261116 (`de52e8e`) delivered independent project/reference
roots and outputs, explicit query selection, provenance and host-agent semantic
extraction. Bootstrap could build both; ordinary preparation refreshed project code.
The user excluded vAmigaTS. Real-corpus separation/read-only traversal checks, the
commit profile and four reviews passed. This superseded the frozen-reference
prototype; unchanged-project preflight remained future work, without an acceptance
result.

Later provisioning copied available local sources and both graphs, preserving
existing destinations and paths tracked by either checkout. Only source-ignored
Reference assets were copied; visible drafts stayed with their owner. Provisioning
stopped rebuilding/synchronizing graphs automatically and required no network
download. Portable fixtures reduced repeated Git reads.
DOCUMENTATION-KNOWLEDGE-PRUNING-20261116 (`7c34cd7`) established one value criterion:
retain hardware contracts, rationale and costly findings, while removing copied
procedures and code inventories. Its approved roadmap timing correction preserved
hardware evidence limits and did not repair runtime behavior.

### November 17-21: Manual Workflow, Findings and Owner Decisions

DETERMINISTIC-WORKFLOW-V1-20261116 (including
AGENT-INSTRUCTIONS-SIMPLIFICATION-20261116), WORKFLOW-NEXT-STEP-GUIDANCE-20261117 and
WORKFLOW-SKILL-MATRIX-20261117 separated operation selection from execution and
result interpretation. Seven roles and eventually 45 atomic skills supplied
structured handoffs, bounded rounds and an official recommendation matrix; direct
user skill selection stayed available. A trial accepted a directly selected
off-matrix skill and rejected it as an official suggestion. Integration `ba27373`
passed nineteen groups. Bounded trials did not prove runtime model selection,
technical permission isolation, every skill or interruption/exhaustion behavior.

Four fresh review lanes separated behavior, architecture, tests/evidence and
documentation. Later guidance required a supported sequence to a reported state
and a concrete consequence: a forced probe established conditional behavior only.
Findings did not authorize unrelated repairs or waived requirements. Planning bound
approvals to selected versions and acceptance scenarios; ordinary drafts could stay
in one working file while retaining selected approved/handed-off versions. Fewer
routine copies did not make stale evidence reusable.

WORKFLOW-CORRECTIONS-20261118 integrated as `ccc20ab` after twenty canonical groups
passed, while reviewers requested changes and fresh VS Code capture proof remained
outstanding. The user authorized a one-time local publication exception for that
tree. PLAN-DESIGN-APPROVALS-INTEGRATION-20261119 (`d6426fc`, acceptance record
`3057547`) also delivered under acceptance of all nine round-five observations:
fifteen groups passed while four primary reports requested changes. These were
accepted delivery limits, not repaired findings.

WF-VERSION-CHECKS-20261120 (`954ac9f`) simplified draft handoffs, exposed model/effort
choices and restored overview/recovery maps. Eighty-three focused cases passed;
maps were rendered at 1,180 pixels and 107 destinations checked. MR-02/MR-03 were
procedure walkthroughs, without actual alternate launches. Four first-round reports
requested changes and seven challenged findings were upheld. The owner accepted
all seven without repair under D-OWNER-ACCEPT-REVIEW-01. Four fresh final reviews
approved within that exception; fourteen integration groups passed. Vocabulary
follow-up `b99c2af` preserved the nineteen original content files and passed fifteen
groups. Those records do not establish a later full-English/publication outcome.

### November 18-21: Execution Capture and Retained Artifacts

Selected-thread capture (`699c018`) registered identities/UTC boundaries with a
project receiver, inventoried portable environments and retained scoped private
session records. A dedicated CLI emitted an explicit skill invocation; independently
checked root/child spans matched the Collector, and fixture archives/commits
survived worktree removal. Four original task inputs were compared with their
starting Git blobs before retirement. The initial pilot alone did not execute every
planned end-to-end scenario.

WORKFLOW-ARTIFACTS-20261118 preserved approved inputs and selected reports apart
from authored Git content. Later corrections supplied durable services, narrow
Collector output isolation, authenticated local registration and upgrade/rollback.
CLI capture survived Docker restart and isolated installation recovery passed.
Book Review retained disposable editor data across recreation with separate writable
diagnostics; shared RAG retained its database/index and limited MCP reader scope.
Service recovery and a CLI event left capture from the initiating VS Code client
unverified. A later planning task recorded such an event for its own execution,
and WF-VERSION-CHECKS continuation obtained fresh active-client OTLP confirmation.
The earlier task still lacked that proof.

A read-only artifact report introduced on November 13 exposed sizes/date groups
and suggested scoped cleanup commands without deleting data. The November 21
workflow simplification (`ac3963d`) separated copying task outputs from the former
publication process. Per-invocation reports stayed in the primary checkout, and
task closure preserved additional local outputs before removing a worktree.
The capture results above retain their recorded execution boundaries.

---

## 15. Book-Derived Custom Chipset Reconstruction

On November 12-13, 2026, CHIPSET-BOOK-REBUILD-20261111 replaced the chipset engines under the established register, signal and CCK interfaces. The work retained the CPU, storage boundary, delayed register queues and captured connector transport. Agnus owns Copper, Blitter and DMA as internal modules; Denise owns sprites; Paula owns audio and interrupts; game_ports owns mouse and joystick logic. Eight separate child crates were removed together with their old imports and verification selections. Explicitly approved checker changes moved protection to the owning crates without adding dependency permissions.

The reconstruction made arbitration govern the transfers themselves. Agnus keeps each request and its granted data until the recipient accepts them. Agnus advances the owning pointers and reports when the shared bus refuses a CPU transfer, leaving the CPU transaction at the same point. Copper fetches and Blitter area/line operations progress under those grants. Audio separates programmed reload values, active sample state and memory addresses; disk words pass through Paula's input decoder, FIFO and serializer instead of a decoded drive-status bridge. The approved Paula model receives all four RGA synchronization addresses, preserves STRLONG phase, and derives VERTB from entry into STREQU. It replaces the old decoded vertical event while preserving receiver-owned state.

Denise reconstructs bitplane, palette, sprite and collision behavior and assembles PAL/NTSC fields, including alternating interlaced rows. The existing GUI uploads that frame buffer. The former directly loaded live_stripes ASM/BIN fixture produced recognizable red/green stripes in a fresh headless GUI capture after two completed fields. CIA timers, TOD, serial and port handshakes, the keyboard protocol, digital game ports, JOYTEST and disconnected UART behavior have component coverage and current-format continuation checks. Analog inputs, attached external devices and host audio/input routing retain later owners.

Snapshot format 4 removes duplicate outer Agnus DMA registers and preserves active engine state, pending delivery and rendered pixels. The user approved replacing the old format without migration or deleting saved files. Completion cleanup removed unused helpers, corrected Clippy visibility findings and stale JOYTEST documentation, and added six approved dictionary terms. Three approved quality exceptions retain the sector encoder used to prepare DMA test input and read-only disk FIFO/UART output observations used by behavior and continuation tests. The isolated closure check initially found two unused GUI parameters on WASM. Target-specific parameter use restored the build without altering native file dialogs.

To close the work, the project ran the commit and milestone profiles, compiled for native and WASM, captured the live GUI and obtained an independent extended review. At that point SingleStep completed 128 groups and 1,002,737 vectors. Static semantic checks covered 196 register offsets, twelve memory boundaries and nineteen crate mappings. Those checks left functional signal coverage to separate verification. Component and production-path regressions established the modeled transitions. Complete guest acceptance and measured silicon edge timing required their own evidence. At reconstruction closure, BOOK-GUEST-PROGRAM-TESTS-20261111 held the full register/engine inventory and independent guest/output checks in Roadmap 2.5. Firmware, media, external suites and physical timing calibration remained separate work.

---

## 16. Native Kickstart and Disk Program Execution

On November 15-16, 2026, NATIVE-DISK-BRING-UP-20261115 (`c7563f4`) supplied the missing autonomous
ADF read stream. The baseline Kickstart reached disk DMA but never the original
cross1 body within 30 emulated seconds. Focused reproductions also showed that
reset ejected mounted disks and that the sector helper lacked a leading preamble.
The repair prepares complete circular tracks at insertion, masks checksum data
positions and carries MFM clocks between longwords and sectors. An ideal 300 RPM
spindle emits 500,000 raw cells per second through the existing captured connector;
PAL/NTSC rational phase accumulation prevents rounding drift. Selection and side
changes preserve angular phase. The sampled INDEX and data pulses represent CCK events. Analog pulse widths and
spindle acceleration remained unmeasured.

The drive owns the media and mechanics. Paula separates raw words and frames
DMAL. Agnus arbitrates for the bus and supplies the addresses, and the machine
commits the words to Chip RAM. This path runs directly through the modeled devices,
with no sector decoder or host OS substitute. Electronic reset stops the motors
and cancels pending signals while retaining the physical media, protection and head
position. Snapshot version 9 stores usable raw tracks and rotation state. A transfer
serialized midway resumes with the same words, completion CCK and future machine
state. The delivery excluded an old-format parser.

The original cross1 bootblock reads its payload to $70000. The recorded execution
showed Kickstart reset, overlay removal, production disk DMA, bootblock execution,
original graphics.library startup and execution of the cross1 body in that order. Both clean runs reached the
loop at CCK 14,752,752. The independently identified Copper list at $70214 produced
its complete 83-color sequence repeatedly, including actual palette commitment,
over multiple full fields. A 30-second no-disk control lacked the program signature.
The recorded inputs were used with a PAL A500 OCS, 512 KiB Chip RAM and 512 KiB
Slow RAM, starting from zeroed RAM and the ROM reset vectors:

| Input | Size in bytes | SHA-256 |
| --- | --- | --- |
| Kickstart 1.3 rev 34.5 | 262,144 | `ee05862d8102a08436ac4056da7d549db31625c7d47b24dfb7b3c9a5c113ca53` |
| Original cross1 ADF | 901,120 | `133a0a489ccb241fe4011e5e1f6d9d5641dd31c4e8c05620290ee57a0769838a` |
| Supplied Workbench 1.3 ADF | 901,120 | `097d332a6443be8339ef686aea49be578988e6f0131eb6e6d36a47f246c9cae8` |

Focused drive and machine tests verify raw words on both surfaces,
track wrap, CPU contention, reset and serialized continuation.

The separately supplied Workbench 1.3 OFS disk executed its bootblock and DOS
startup. Its original scripts use StartupII, resident commands and LoadWB delay.
The named Workbench task appeared within the separate 60-second budget, but the
saved frame was visibly corrupted. Desktop visual acceptance and broader original
vAmigaTS image comparison remain with native verification; this single-case boot
neither closes that track nor establishes universal AmigaDOS support. The read
workloads needed no media writes, host input or invented companion disk.
The closing commit records passing commit/milestone profiles, 1,002,737 SingleStep
vectors with two existing ASL.b overrides, two native checks and four independent
review approvals. Ordinary tests used one thread after parallel GUI attempts
exposed settlement/texture races; concurrent harness stability was not established.

### November 16: Workbench Rows and Battery Clock Detection

Workbench rows were corrupted because DMA consumed 76 of the expected 80 bytes
per row. Commit `8313853` made DMA request the final complete fetch group after
DDFSTOP and apply modulo only on its last word. The Workbench
regression failed on the old code, while Kickstart passed. Agnus tests passed after
the repair, including consecutive rows with zero or signed modulo. Native Workbench
text and window borders became straight. Kickstart and cross1 captures stayed
byte-identical. The selected rule followed vAmiga and still lacked measured hardware
timing evidence. Chapter 18 records the separate delayed-DIW and image-rectangle
correction.

RTC-WB13-01 (`38f8fd6`, closed by `1cc643c`) reproduced Workbench's missing-clock
message despite an installed RTC: startup HOLD kept seconds fixed. Starting the
MSM6242B with HOLD clear restored seconds-based detection, while calendar-write
tests engaged HOLD explicitly. The startup regression failed first; RTC/machine/bus
tests and the RTC WASM check passed afterward. Paced Kickstart 1.3 / Workbench 34.20
boots read the date after repair. Original SetClock had a separate post-1999 defect:
a diagnostic disk copy with SetClock 34.3 showed 2026 here, while WinUAE 6.0.3 showed
2026 with both versions. Two-digit RTC years stayed unchanged. The closing
integration passed the canonical profile and four reviews.

The repairs established specific desktop paths. Broad AmigaOS and native-suite
acceptance remained open. Roadmap 2.6 still tracks readable-desktop confirmation,
original vAmigaTS disk execution and reference comparisons. Native Amiga software
ran in the emulator. Execution on a physical Amiga remained unverified.

---

## 17. Core Performance: Removing Repeated Host Work

November 13-17, 2026 established measured local optimizations while retaining
CCK-driven timing. CORE-PERFORMANCE-PREP-20261113 began with mutation queues:
occupied counts and reusable due buffers removed scans/copies of empty slots;
serialization retained pending values, discarding derived observations on clone or
reset. All 317 affected tests passed; 28 PAL/NTSC checkpoints matched after
normalizing only empty slots. Later compaction copied full entries only across a
hole, preserving due order, capacity and positive delays.

Sprite work moved from every pixel to register-derived state and occupied spans.
Cached starts, attachment and collision enables skipped inactive pairs without
stopping disarmed serializers. A lazy 2 KiB line buffer served palette selection
and both collision paths, rebuilding future output after applied writes while
retaining consumption-time CLXDAT effects. Review found an H=510 word wrapping into
earlier entries; anchored-window clipping fixed early collisions with a regression
that failed before repair. Derived state rebuilt from canonical snapshot values.
Precomputed playfield masks and unchanged-CLXCON guards retained accumulation and
live-read clearing. Tests compared 138 full checkpoints and 67,840 pixel samples
over 4,220 deterministic events without exclusions.

Other accepted changes reused immutable Blitter phases, fixed DMA slots, sprite
request masks, BPLCON0 decoding and dual-playfield tables. Blank composition could
be skipped while serializers/collisions progressed. CIA idle guards retained hidden
timer waveforms and pending pulses. JOY lookup, idle mouse-axis guards, encoded
joystick contacts and empty keyboard phases avoided repeated work without changing
connector sampling or wire timing. Constant disk divisors retained FAST phase
continuity; UART guards retained status/transition tests. Several simplifications
had no demonstrated whole-machine gain.

Each measurement below has its own baseline. Do not add together gains from
different baselines. Core timings include pixel generation and exclude GUI work
and pacing unless stated otherwise.

| Date / task | Retained result | Measurement and evidence limit |
| --- | --- | --- |
| Nov 13, queue pass | 23.889839 to 10.017356 ms/frame, 58.07% less time | 5,000 release frames/version; queue tick profile share 22.75% to 0.59%; no general FPS guarantee |
| Nov 13, sprite line cache | ROM 109.417 to 116.571 FPS; three active scenes +2.87% to +3.36% | ABBA, 300 warmup and 3,000 measured frames/version/scene, baseline `bbcbcdf9`; synthetic active scenes used a stopped CPU |
| Nov 14, JOY guard | ROM +2.378%, idle +6.686%, input batches +7.141% throughput | 112 checkpoints and 300,000 per-CCK samples matched; host latency excluded |
| Nov 14, queue self-copy guard | Occupied focused cases about 17-21% faster | 182 tests, equal full states/frames; focused result |
| Nov 14, pixel decoder | Mixed PAL time -2.47%, NTSC -1.61% | 211 tests, equal complete states/frames |
| Nov 14, repeated CLXCON | Focused repeated-write time about -83% | 212 tests, equal states/frames; not a whole-machine result |
| Nov 14, final round two | Core PAL -2.13%, warmed ROM -4.67%, NTSC -3.23%; GUI PAL -2.82% time | Final B0/PREP results in `fdd77f6` supersede intermediate RTC2 figures; GUI NTSC mean -2.02% with overlapping ranges |
| Nov 14, sprite DMA reuse | PAL 8.51569 to 7.87536 ms/field (-7.52%); NTSC -7.00% | PERIPHERAL-EVENT-OPTIMIZATION-20261114 (`6ba827d`); four 1,000-field samples/version/workload, equal relocated state/frames; ROM within spread |
| Nov 15, framebuffer write/CCK | Mixed PAL -3.78%, NTSC -3.69% time | DENISE-RENDER-EXPERIMENTS-20261114 (`ccceed1`); interleaved samples, equal serialized state/images; idle PAL ranges overlap |
| Nov 17, timestamp admission | 119.335 to 122.189 fields/s (+2.39%) | MEMORY-INVARIANT-SIMPLIFICATION-20261117; twelve interleaved samples/version, equal progress/frames; integrated frontend not remeasured |

The sprite-DMA pass passed 275 focused tests; final Denise storage passed 68 focused
tests and its commit profile with 733 ordinary tests. Final round two passed 730
ordinary tests, nine harness tests, twelve commit groups and independent review;
intermediate commit partitions were not independently checked.

Signal suppression, an audio mixer cache and deferred-CCK RTC experiments showed
regressions. Three disk, one UART and two audio variants
failed later whole-machine comparison. Grouped serializers, table gathering,
shared sprite checks and stream reconstruction were rejected too. Preserved source
timestamps left an old binary in one final measurement set; those results were
discarded and repeated after rebuilding. No rejected variant became an accepted
speedup.

EMULATION-OPTIMIZATION-ROUND-02-20261114 retained a functional RTC change despite
its isolated NTSC cost: supplied host UTC plus a separately persisted battery
offset replaced CCK progression, and live RTC left snapshots in format 7.
HOLD/STOP, guest calendar writes and battery restart remained supported; February,
leap-day and negative fractional epoch conversion were repaired. Additional general
profiling was cancelled by user direction. State/output comparisons used the
approved exclusion of live RTC from snapshots. MIXED-GAME-SCENE-20261113 (`492818f`)
exercised ordinary machine stepping with separate audit/benchmark modes and
observation outside timed runs; visual fidelity and hardware agreement were not
independently accepted.

---

## 18. Video: From Visible Raster Storage to a Stable Image

November 13-17, 2026 corrected presentation and receiver phase separately from
chipset reconstruction. Four-bit RGB channels expanded by multiplication by 17 into
opaque host pixels under explicit full-range normalization, without gamma/CRT
correction. Visible border/playfield initially used a high-resolution spatial grid
excluding modeled blanking. Fixed-stride storage then removed row relocation, used
RGB444 lookup and exposed visible row prefixes to GUI/PNG consumers. The user
approved border color at the first two samples instead of the former black edge.
Explicit Denise coordinates and a persisted frame cursor followed; the coordinate
simplification was retained without a demonstrated speed gain.

VIDEO-FRAME-RECT-20261116 (`5f01bfc`, integrated by `83fe3dc`) found that DIW compared
against the wrong phase, dropping the first pair of an ordinary bitplane word
stream. Comparing against the delayed output pair recovered the edge and preserved
H0. An integration regression also showed that horizontal start 2 lacked a comparison
against the receiver origin. After the repair, checks passed for starts 2/3, both
resolutions and consecutive lines. Missing tails and blank middle rows were cleared
while preserving opposite interlaced parity, reset geometry and snapshot continuation.

Presentation became one 720-stride PAL 720 x 568 or NTSC 720 x 480 rectangle, with
receiver CCK origin 49 and line origins 28/21. These are project choices, without
analog raster calibration. The saved Workbench Clock remained 170 samples wide and
its directory 428; 420 missing samples returned in the old scene. PAL matched the
corrected prior image cropped at (52, 4), and cross1 matched across three identical
fields. The NTSC result matched all 345,600 RGB samples in the selected rectangle at
(16, 0) in a 752 x 484 capture of WinUAE 6.0.3 before filtering. The comparison
used no scaling, tolerance or masks. It established agreement with the reference
model for one configuration. Physical hardware still required measurement.

The source profile passed 837 ordinary tests (three existing skips), nine harness
tests, native Clippy and the actual WASM build; final integration passed its
canonical profile and four reviews. Wider display-mode/color/interlace/border
acceptance remained in Roadmap 4.4 and native verification. GUI Fit preserved the
whole framebuffer's width-to-height ratio with uniform scaling; fixed scales used
nearest sampling and Fit used bilinear filtering, independently of interface DPI.

---

## 19. Host Execution, Player Workflow and Input Ownership

### November 13-15: Field Playback and One Machine Owner

Playback moved to bounded CCK slices ending at PAL/NTSC fields, with boundary pause,
field stepping, speed controls and measured field FPS. Accelerated execution batched
fields independently of 50/60 Hz presentation. CPU STOP continued video, timers and
interrupts rather than freezing the machine; HALT and Pause stayed distinct. Six
focused cases covered CIA wakeup/breakpoints; GUI coverage checked advancing video
during STOP. Developer Run retained prepared inspectors instead of rebuilding
disassembly, trace and history, refreshing on stops/edits/reset/load; Game inspection
was disconnected. A matched sixty-field trial with the former live_stripes fixture fell from 36.772 to
25.042 ms of Developer execution, with zero steady-run inspector/trace/history calls.
Texture conversion reserved space for visible pixels once and skipped stride
padding. The gain was local to that conversion. Whole-application FPS still
required a separate measurement.

HOST-SHARED-MEMORY-PRESENTATION-20261113 (`f0fa90c`, integrated by `a416e45`) moved
the complete session to one native/browser worker, exchanged reusable pixel
allocations and routed commands/stopped inspection through that owner. Deterministic
CCK execution no longer depended on repaint. ROM patching, wrapping, stepping,
breakpoints, history, persistence and partial-field continuation retained checks.
Combined integration passed eighteen groups, real browser startup/delayed startup/
idle notification checks and independent review. Coverage used current objects for
150 binaries; stale mappings and 482 LLVM mismatches remained measurement limits.
Optimized shared-memory WASM became an actual build check, separate from advisory
Stable toolchain update reporting.

Native and browser loading shared validation and showed errors to the user.
Kickstart moved to a private, fixed 256 KiB buffer with mirrored addressing. Invalid
loads and restores preserved the machine. Separate histories for binaries and ROMs
kept five recent entries and the options from successful loads. Host file contents
stayed outside snapshots. Selecting a missing file removed its history entry and
showed feedback. Permission denial or cancellation retained the entry. Native
pickers used an existing ancestor. Browser history stored handles and names in
IndexedDB and relied on explicit selection. Actual picker and permission trials
remained separate from the fixture evidence.

### November 15-17: Deterministic Input and Window Routing

HOST-INPUT-DELIVERY-20261113 (`387d079`) published ordered native/browser events to
bounded queues and admitted them at three receiver-aligned boundaries per field,
without per-CCK host synchronization. Generations rejected obsolete events;
reset/load/focus transitions released controls. Windows/Chrome probes and focused
GUI/device tests supplied bounded evidence. Physical gamepad freshness, Linux/macOS
execution and A500 key-combination fidelity were not established.

Explicit capture (`bbdd32c`) reserved F12 for Game/Debug and the middle mouse button
for pointer release. The optional keyboard release key started unassigned. Gamepads could
continue without pointer capture; focus loss released all controls. Integration
`51379e3` corrected stale contacts across generations, preserving fresh gamepad
edges after pending release while discarding keyboard-joystick presses on capture
loss. Separate reset/restore/queued-key regressions failed before repair. Fixed
sleeps gave way to owner observation and inactive-frame retries within the same bound.

KEYBOARD-DIAGNOSTICS-20261117 (`e162dae`, closed by `b17f9b3`) replaced global rdev
listeners with application-window events and one native/browser routing policy on
upstream eframe/egui 0.32.3. Physical modifier sides, keypad identity, command edges,
generations, queues and gilrs gamepads survived; temporary terminals/traces were
removed. Review reproduced lost Home/End/F11/PageDown and first-key loss before
activation; position mapping and readiness retries fixed both. Combined acceptance
passed 896 ordinary tests (three skips), harness tests, native/WASM checks and four
reviews. An isolated non-wrapper run instrumented 198 GUI tests after cargo-llvm-cov
0.9.1's default wrapper omitted GUI coverage. Final-adapter physical typing and
other-platform trials remained unrun.

### November 16-17: Library, Configurations and Continued Games

GAME-LIBRARY-INPUT-PLAYER-20261116 (`58c2070`) delivered desktop discovery, Add/Edit
game, copied ADF media and JPEG/PNG covers, named hardware/input configurations,
independent Amiga ports, four-drive controls and a session shared with Debug. Browser
setup retained manual media selection, persistent settings and snapshot import/export
without a media cache. A selected library included Amiga Test Kit, Workbench,
Cadaver and a vAmigaTS diagnostic while preserving source media. Amiga Test Kit 1.21
reached its menu; complete Cadaver startup was unverified.

Snapshot format 10 retained the CPU's active execution-sequence identity: decoding
the opcode again lost redirected branch/exception sequences. Static arrays/pointers
rebuilt from that identity. Real-guest floppy-DMA snapshots continued identically
for 20,000 CCK; browser import/export retained exact CPU/CCK values. Review repaired
replacement confirmations, stale ROM/bindings, durable slots, text focus and arrow-key
conflicts. Unverified checkpoints became `1f99763`/`aa3ab64` after the approved rebase;
combined acceptance passed 833 ordinary tests (three private-asset skips), browser
workflow and four reviews. A rejected-ROM test normalized only creation time after
reproducing a UTC-second mismatch; machine state and all other metadata stayed
compared. Coverage also protected malformed/duplicate library records, concurrent
edits, configuration conflicts, disk acknowledgments and atomic restore rejection.

CONFIG-INPUT-STATE-20261116 (`902fc62`, closed by `101f86c`) isolated configuration/
mapping drafts behind Save/Cancel, unsaved-navigation choices and committed-only
persistence. Endpoint changes occurred at a stopped owner boundary. Loading progress retained exact core state, media, receiver registers and mouse
phase. The load reconciled current controls without replaying launch hardware. Flat settings stayed
readable. A saturated ten-code keyboard ring had lost key-up; serialized deferred
releases preserved retry and release-before-press order, including CPU observation
after restore. Coverage protected rejected saves, dirty navigation and current-control
restoration. Chromium verified Cancel, port changes and older endpoint imports with
current controls at CCK 2,549,684. Combined schema 13 replaced the two parents'
incompatible schema-12 layouts. Integration, coverage review and four lanes passed;
physical controllers/private firmware were unrun. Broader player/save-catalog/profile
acceptance remains scheduled.

### Asynchronous Test Boundaries

The command queue could become idle before the GUI received the image because the
image traveled through a separate exchange. GUI-MONITOR-IMAGE-FAILURE-20261116 waited for the expected
generation, stopped CCK and dimensions before running every original pixel, UV and
geometry assertion. Eight controlled mode/view combinations failed before the
change and passed afterward. The package passed 120 tests. Direct snapshot polling
could also drain replies owned by AppHost, leaving a UTC request pending. The test
fixture began waiting for the GUI to settle before observing the snapshot directly
(`68d908e`), with the original input assertions and deadlines. Host overload and
compilation memory failures caused failed or interrupted attempts. Later passes
apply only to the final candidates.

---

## 20. Runtime Invariants and Floppy Activity Observations

MEMORY-ACCESS-INVARIANTS-20261115 and MUTATION-QUEUE-OVERFLOW-20261115 (`3711224`,
integrated by `e3093cd`, November 16) made Chip RAM private with one shared CPU/DMA
transfer budget per CCK. Attempts/read-write phases, including waits/TAS, were
checked in both CPU execution engines. Mutation overflow and shared register/CIA
preflights failed before partial delivery. Native/browser boundaries kept the first
diagnostic, refused later work and shut down without waiting for a dead owner.
Narrow panic/dependency/checker permissions were explicitly approved; guest
exceptions, ordinary waits, capacities and delays stayed distinct.

Debug/release admission and 6,350 sampled SingleStep vectors passed with state,
clock and bus comparisons, along with actual native/WASM fatal delivery and eleven
TAS benchmarks. The final matched comparison cost 3.13% mixed and 6.00% CPU/Fast-RAM
throughput; native worker timing ranges overlapped. Coverage did not exercise
poisoned-mailbox forwarding. This accepted diagnostic cost was recorded plainly.

MEMORY-INVARIANT-SIMPLIFICATION-20261117 (`fb6c76f`, closed by `72adaf0`) replaced
countdowns/verbose descriptions with CCK timestamps. Normalizing writes by one CCK
preserved two-CCK CPU spacing, pending wait identity, read completion and shared
storage admission. Access age survived reset/restore; owner/PC plumbing and approved
diagnostic assertions were reduced, while mutation diagnostics stayed intact.
Schema 14 replaced development snapshots without compatibility parsing or deleting
saves. Final integration and four reviews passed. Chapter 17 retains the source-only
measurement; full SingleStep, physical captures, private firmware and browser speed
were unrun for this simplification.

Drive feedback changed from a motor-based label to a fixed green light (`0cfab14`)
with a 120 ms hold and reset/load baselines. DISK-ACTIVITY-20261117 (`8f52021`) moved
observations to effective CIA-B controls and accepted Paula transfers, removing both
per-CCK collectors and presentation state from Floppy. Wrapping
counters for each drive preserved short events between publications. The machine established
the recipients for the next CCK after the CPU phase. Shared connector decoding kept
the reported activity consistent with drive mechanics without callbacks or
dependencies. The GUI kept the bounds fixed and continued to indicate a drive
while it was selected and running. The physical motor-controlled LED remained a
separate behavior.
The results from six-sample A-B-B-A idle/read workloads stayed within variation
(178.967 to 180.621 and 171.221 to 172.104 FPS). The measurements left any speedup unverified. Source suites, WASM and
visual checks passed, followed by combined `101f86c` acceptance. Guest media writes,
motor dynamics and physical floppy fidelity remained future work; observation
metadata did not implement them.

---

## Rolling Changelog

This final, unnumbered section retains unresolved work needed for continuation.
Settled history through November 21, 2026 is integrated into the thematic chapters.
Historical tests apply only to their recorded inputs. ROADMAP.md and active plans
own future behavior; compaction does not close their acceptance gates.

### Entry Format

Ordinary commits do not require an entry. When the user explicitly requests an entry, explain what the selected committed work completed, why it was done and which checks actually ran, including failures and limits of the evidence. Keep pending edits and instructions for continuing work in task plans. The [history criteria](.agents/skills/TA-WF-Step-compact-diary/SKILL.md#history-criteria) govern how entries are written and condensed. During [milestone closure](.agents/skills/TA-WF-Main-close-milestone/SKILL.md), [diary compaction](.agents/skills/TA-WF-Step-compact-diary/SKILL.md) moves lasting decisions and evidence into the chapter they belong to and leaves routine details in Git.

### [2026-11-16 09:47 CET] — Graphify work outside the delivered optimizations

- GRAPHIFY-REFRESH-PERFORMANCE-20261116 delivered memoization; the subsequent
  GRAPHIFY-TWO-GRAPHS-20261116 delivered separate corpora (`de52e8e`).
- Unchanged-project preflight remained unfinished. The frozen-reference prototype
  lacked complete dependency invalidation; measurements did not establish a
  production-ready incremental skip mechanism.
- No later acceptance evidence for preflight was found during compaction.
  Chapter 14 retains delivered mechanisms and their verification limits.

### [2026-11-19 22:29 CET] — Workflow correction delivery retained an exception

- WORKFLOW-CORRECTIONS-20261118: `002f75c` was the working commit; `ccc20ab` records
  twenty passing integration groups and a one-time local-publication exception
  despite changes-requested reviews and missing fresh VS Code capture proof.
- Isolated recovery, upgrade/rollback and a real CLI archive event passed. The later
  client proof applies to the task that obtained it. The original reports and
  pending observations still apply here, with no retrospective repair claimed.

### [2026-11-20 00:20 CET] — Planning and delivery findings accepted without repair

- PLAN-DESIGN-APPROVALS-INTEGRATION-20261119: the owner accepted all nine round-five
  observations and selected local delivery (`d6426fc`, `3057547`). Fifteen groups
  passed; four primary reports requested changes.
- Offline checks covered Python tests, delegated Book Review and the mixed benchmark.
  Other launch routes had no recorded offline execution. A shortened benchmark ended
  before guest progress; an enclosing snapshot check failed while files changed.
  Those attempts do not supply the missing route evidence.

<a id="2026-11-21-cet--version-findings-and-the-workflow-revision-in-progress"></a>

### [2026-11-21 CET] — Version findings and the delivered workflow simplification

- WF-VERSION-CHECKS-20261120: D-OWNER-ACCEPT-REVIEW-01 accepted ARCH-01, BEH-01,
  BEH-02, DOC-WV-01, DOC-WV-02, DOC-WV-03 and TEST-EV-01 without repair. Original
  reports/challenge verdicts remain authoritative for those findings.
- `954ac9f` records fourteen integration groups and four final approvals within that
  exception. `b99c2af` records fifteen passing groups after vocabulary corrections,
  preserving original content files. Neither repairs the seven findings or proves
  a later full-English/publication outcome.
- Continuation delivery-20261121-02 obtained fresh active-client OTLP confirmation;
  MR-02/MR-03 remain walkthroughs, without alternate-launch evidence.
- `ac3963d` delivered user-selected Main/Step operations, a simpler skill result
  contract and ordinary Git integration. It retired the skill matrix and the
  routing/publication transaction tools. `7da7fc3` consolidated the engineering
  history into thematic chapters. These commits identify the delivered changes;
  they do not establish that the earlier accepted findings were repaired or that
  any unrecorded verification was performed.
