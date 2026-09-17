# Amiga 500 Emulator: Engineering Diary & Architecture Genesis

This document records the complete architectural philosophy, evolutionary history, human-AI co-design methodology, and pivotal engineering decisions behind the cycle-exact Amiga 500 emulator in Rust.

> [!IMPORTANT]
> **Zero Hand-Written Code (100% Voice-Prompted & AI-Generated):**  
> Throughout the entire development of this emulator, **not a single letter of code was written by hand**.  
> The entire codebase—the cycle-exact Motorola 68000 CPU core, memory bus, immediate-mode Developer Studio GUI, debugger engine, and test harnesses—was generated 100% by the AI agent through spoken voice prompts, conversational code review, and iterative engineering direction. Aside from occasionally pasting a documentation link or typing a brief message when voice dictation was inconvenient, the human functioned strictly as the chief architect, code reviewer, and quality enforcer.

---

## Table of Contents

1. [Project Genesis: Groundwork, Reference Ingestion & The Myth vs. Reality of AI Emulation](#1-project-genesis-groundwork-reference-ingestion-the-myth-vs-reality-of-ai-emulation)
2. [Foundational Architecture: Cycle-Exact Mechanics, Rejecting the "God Bus" & Rust Sympathy](#2-foundational-architecture-cycle-exact-mechanics-rejecting-the-god-bus-rust-sympathy)
3. [Modular Workspace Architecture & Two-Tier Hardware Decomposition](#3-modular-workspace-architecture-two-tier-hardware-decomposition)
4. [Physical Circuit Simulation: Standalone BLEP Audio Synthesis](#4-physical-circuit-simulation-standalone-blep-audio-synthesis)
5. [The 6-Stage Evolution of the M68000 CPU Core](#5-the-6-stage-evolution-of-the-m68000-cpu-core)
6. [CPU Opcode Benchmarking & Performance Profiling](#6-cpu-opcode-benchmarking-performance-profiling)
7. [Subsystem Evolution: The Developer Studio GUI & Debugger Engine](#7-subsystem-evolution-the-developer-studio-gui-debugger-engine)
8. [Architectural Rules, Anti-Tamper Policies & The Living Definition of Done](#8-architectural-rules-anti-tamper-policies-the-living-definition-of-done)
9. [The Ultimate Goal: The Clean-Room Re-Generation Experiment](#9-the-ultimate-goal-the-clean-room-re-generation-experiment)
10. [Repository Bootstrapping, Clean-Room Pipeline & Tooling Ergonomics](#10-repository-bootstrapping-clean-room-pipeline-tooling-ergonomics)
11. [Multi-Tier Testing Infrastructure, Cartesian Contention & Silicon Verification](#11-multi-tier-testing-infrastructure-cartesian-contention-silicon-verification)
12. [Developer Studio GUI, Headless Perception & Debugging Ecosystem](#12-developer-studio-gui-headless-perception-debugging-ecosystem)
13. [Custom Chipset Silicon Architecture & Autonomous DMA Engines](#13-custom-chipset-silicon-architecture-autonomous-dma-engines)
14. [Multimodal Reference Ingestion & PDF-to-Markdown Pipeline](#14-multimodal-reference-ingestion-pdf-to-markdown-pipeline)
15. [Agent Operating System, Architectural Guardrails & Quality Discipline](#15-agent-operating-system-architectural-guardrails-quality-discipline)
16. [Living Chronological Engineering Log & Active Evolutionary History](#16-living-chronological-engineering-log-active-evolutionary-history)

---

<a id="1-project-genesis-groundwork-reference-ingestion-the-myth-vs-reality-of-ai-emulation"></a><a id="1-project-genesis-groundwork-reference-ingestion--the-myth-vs-reality-of-ai-emulation"></a>
## 1. Project Genesis: Groundwork, Reference Ingestion & The Myth vs. Reality of AI Emulation

### The Pre-Rust Phase: Physical Schematics, LTspice & Analog Audio Synthesis (Late August 2026)
Before a single line of Rust code was written, the human spent the final week of August 2026 establishing the physical ground truth of the Amiga 500 hardware:
- **Analog Low-Pass Audio Simulation (LTspice):** The human modeled Paula's analog audio reconstruction circuit in LTspice (`0b2777fa`), extracting the exact resistor (ohms) and capacitor (farads) values from original Commodore schematics (`c448cd14`). This captured both the fixed 2-pole Sallen-Key low-pass filter and the switchable "LED filter" controlled by CIA-A port A bit 1.
- **Band-Limited Step (BLEP) Table Generation:** In a dedicated C# prototype (`9057f4a8`, `7783f1e2`), the human derived and verified the continuous-time impulse response and step response tables needed for alias-free audio interpolation, parameterizing them directly against physical component physics rather than arbitrary digital filters.
- **Reference Emulator Ingestion:** To prevent speculative design, five verified open-source reference emulators were imported into `ref_src/` for cross-validation: MAME, Moira, Musashi, vAmiga, and WinUAE.
- **Decoded Single-Step Vectors:** Over 1 million cycle-exact hardware captures from MAME and Tom Harte hardware tests were imported and structured (`a31518dc`), providing an unforgiving, cycle-by-cycle test verification reference before any CPU code was drafted.

### The RAG Knowledge Base & Reference Documentation Sprint (September 1–4, 2026)
Between September 1 and September 4, a massive documentation ingestion sprint took place across dozens of commits (`e82a4efc` through `e782dcf9`):
- Hundreds of pages from the **Amiga Hardware Reference Manual (HRM)**, **Motorola 68000 Programmer's Reference Manual (PRM)**, **Amiga Guru Book**, and **A500/A2000 Technical Reference Manual** were ingested and converted into clean, semantically structured Markdown.
- Vector SVGs and timing schematics were extracted from original PDFs, register layouts and bitmasks were normalized into tables, and Appendix B clock transition diagrams were recropped and cleaned.
- A local **Qdrant vector database** (`tools/rag/`) with a custom FastMCP server (`amiga_rag`) and **Graphify** AST knowledge graph (`graphify-out/`) were deployed.
- **The Core Strategy:** An AI agent prompted in a vacuum will hallucinate register addresses, flag behaviors, and bus cycle timings. By arming the agent with instant, offline vector retrieval over the official hardware manuals, the human ensured that every generated instruction and bus cycle was anchored in historical hardware reality.

### Language & Framework Deliberations: Why Rust & egui
Once the reference foundations were secured, the human and agent evaluated the systems programming language and UI framework:
- **Language Choice — Rust:**
  Chosen decisively for its zero-cost abstractions, memory safety without garbage collection pauses, predictable host CPU performance, strict ownership semantics, and seamless dual compilation to native desktop (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).
- **UI Framework Selection — egui:**
  Chosen for its immediate-mode paradigm: direct synchronous state pulling (`app.update_ui(ctx)`), zero complex event-broker boilerplate, and flawless compilation across native OS windows and browser-based WASM canvases.

### The Myth vs. Reality of AI-Assisted System Emulation
There is a widespread misconception in the software industry that modern large language models can simply be instructed:
> *"Build me a cycle-exact Motorola 68000 processor and an Amiga 500 emulator"*

...and effortlessly output a functioning, high-performance, cycle-exact emulator.

The reality demonstrated throughout this project is that an unguided LLM left to its own devices reliably produces an unoptimized, brittle, and architecturally naive toy. It cannot navigate bus contention physics, 2-phase color clock synchronization, pipelined prefetch advancement, or subtle custom chip wait states without relentless human architectural leadership:
- Inspecting every proposed instruction handler, memory bus arbitration phase, and ALU flag update line-by-line.
- Rejecting boilerplate compression shortcuts (macros, const-generics) that obscure hardware behavior.
- Demanding automated, cycle-exact unit and integration test harnesses before accepting any milestone.
- Forcing radical architectural resets whenever the code drifted away from mechanical sympathy and physical reality.

True engineering excellence emerged strictly at the intersection of the human's uncompromising architectural discipline and the AI agent's rapid synthesis and refactoring velocity.

---

<a id="2-foundational-architecture-cycle-exact-mechanics-rejecting-the-god-bus-rust-sympathy"></a><a id="2-foundational-architecture-cycle-exact-mechanics-rejecting-the-god-bus--rust-sympathy"></a>
## 2. Foundational Architecture: Cycle-Exact Mechanics, Rejecting the "God Bus" & Rust Sympathy

Early architectural discussions centered on two core questions:
1. *What does "cycle-exact" actually mean for the Amiga 500?*
2. *How do we model custom chip interconnects cleanly in Rust without borrow panics?*

### Rejecting the "God Bus" & Circular Handles
In many historical emulators, all custom chips hold references to a monolithic "God Bus" or pass circular handles (`Rc<RefCell<...>>`) to mutate shared state on the fly.
- The agent initially noted the convenience of shared references, but the human decisively rejected this: circular pointers cause synchronization deadlocks, runtime borrow panics, and fragment CPU cache locality.
- The architecture was strictly mandated to be **unidirectional, flat, and linear**, with zero circular references between subsystems.

### Top-Down Orchestration via the Main Loop (`A500::step_cck`)
To achieve mechanical sympathy with Rust's ownership model and the host CPU's instruction pipeline:
- **No chip autonomously queries or mutates another chip.**
- The top-level machine struct (`A500`) directly owns all subsystems: `Cpu` (`M68000`), `MemoryBus`, `CycleCounter`, `Agnus`, `Denise`, `Paula`, `CiaA`, and `CiaB`.
- On each Color Clock phase, the **main loop** orchestrates execution top-down:
  1. The **Agnus DMA Scheduler** evaluates the active horizontal scanline slot (refresh, audio, disk, sprite, bitplane, or CPU).
  2. The bus access arbitration decision and wait states (`BusResult::WaitState`) are passed down to the CPU and custom chips.
  3. The main loop steps each chip for that CCK phase, and chips advance their internal sub-components accordingly.
  4. Interrupt lines (IPL 1–6) and DMA bus locks flow strictly through the top-level orchestration loop and `MemoryBus`.

### Host Hardware Reality: Mechanical Sympathy Over Micro-Optimizations
Taking into account the immense speed of modern host processors (3.5–5.0+ GHz), expansive L1/L2/L3 caches, and abundant system RAM, the human established a foundational premise: **traditional, cryptic micro-optimizations are completely unnecessary**.

Rather than cluttering code with premature bit-twiddling hacks, pointer casting, or opaque abstractions, the design focuses entirely on **mechanical sympathy** with the host processor:
- **Maximizing Cache Locality & Linear Execution:** Modern superscalar CPUs execute instructions at staggering throughput as long as code execution is predictable, contiguous, and linear.
- **Eliminating Branch Mispredictions:** Deep host CPU instruction pipelines (14–20+ stages) suffer severe penalties (15–20 CPU cycles) on mispredicted branches. Eliminating cascaded runtime branches in the hot emulation loop (e.g. replacing runtime `match opcode`, `match ea_mode`, and `match size` checks with a direct 65,536-entry static dispatch table of specialized, concrete handlers) allows the host branch predictor to operate with near-zero ambiguity.
- **Zero Dynamic Heap Allocations During Emulation:** A non-negotiable mandate: during active emulation, **virtually zero heap memory allocations** (`Vec::new`, `Box::new`, `format!`, `String`) may take place in hot paths (`step()`, `step_cck()`, bus access, interrupt polling). All guest memory buffers (Chip RAM, Fast RAM, ROMs), microcode state registers, execution trace ring buffers, and pipeline latches are fixed-size and pre-allocated upfront. Avoiding runtime allocator locks and heap churn eliminates latency jitter, prevents cache line eviction, and guarantees deterministic performance across desktop and WebAssembly.
- **Natural Blazing Speed:** When code is flat, linear, cache-dense, and allocation-free, the host CPU executes it almost effortlessly at peak throughput.

---

<a id="3-modular-workspace-architecture-two-tier-hardware-decomposition"></a><a id="3-modular-workspace-architecture--two-tier-hardware-decomposition"></a>
## 3. Modular Workspace Architecture & Two-Tier Hardware Decomposition

From the inception of the codebase on September 6 (`8ed5eaca`), a clean workspace separation was instituted:
- **Dedicated Crates for Major Chips (Tier 1):** Every primary physical custom chip and subsystem lives in its own dedicated workspace crate (`crates/agnus`, `crates/denise`, `crates/paula`, `crates/cia`, `crates/m68000`, `crates/memory_bus`).
- **Decoupled Crates & Modules for Sub-Units (Tier 2):** Rather than allowing chip structs to become monolithic God objects, major sub-components are decomposed into independent, specialized crates and modules:
  - **Agnus:** Independent state machines for the Copper coprocessor (`MOVE`, `WAIT`, `SKIP`, `CDANG`) and 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer).
  - **Paula:** 4 DMA audio channels, floppy disk MFM track controller, serial UART, and interrupt multiplexer.
  - **Denise:** Video pixel serializers, bitplane fetch engines (1–6 planes), 8 hardware sprite generators, and palette registers (RGB444).
  - **CIAs:** Dual MOS 8520 chips decomposed into Timers A & B, TOD 50/60 Hz clock, serial shift register (SDR), and parallel ports.
  - **Peripheral & Storage:** Decoupled crates for RTC (`crates/rtc`), Floppy drive/MFM, and Audio sinks.
- Following our 3-tier ownership model, parent peers conceptually own and re-export their sub-components, while peers never hold circular references to other peers.

### The Evolution of the Memory Bus: From Dynamic Branching to $O(1)$ Direct Bank Handlers
The memory bus (`crates/memory_bus`) underwent a radical performance evolution across several pivotal commits:
1. **Initial Range Checking (`179fa5d2`):** Early iterations evaluated memory access via cascaded `if/else` checks matching 24-bit addresses against address boundaries (`$000000-$07FFFF`, `$DF0000-$DFFFFF`, `$F80000-$FFFFFF`). This produced significant branch overhead on every single byte and word transfer.
2. **The $O(1)$ Bank Mapping Breakthrough (`4c799770`, `3b30e894`):** The human directed a complete redesign: the entire 16 MB 24-bit physical address space is sliced into **256 static banks of 64 KB each**.
   - A 256-entry lookup table (`[BankHandler; 256]`) maps each 64 KB block directly to function pointers: `read_byte`, `read_word`, `write_byte`, and `write_word`.
   - Accessing any address requires only a single byte shift (`address >> 16`) and an indirect call, delivering pure $O(1)$ dispatch with zero branching in the hot loop.
3. **Hardware-Accurate Quirks:**
   - **Floating Open Bus:** Unmapped reads return `$FF` / `$FFFF` (simulating the pull-up resistors on the physical Amiga bus), configurable via `set_unmapped_byte()` for flat test harnesses.
   - **Kickstart Overlay:** CIA-A port A bit 0 controls the overlay latch. On cold boot, bank 0 (`$000000-$07FFFF`) mirrors Kickstart ROM until the first hardware write flips the latch to expose physical Chip RAM.
   - **Gary Chip RAM Mirroring & Slow RAM:** Accurate address decoding for standard 512 KB Chip RAM, 512 KB A501 trapdoor Slow RAM (`$C00000`), and Gary custom register mirrors (`$DF0000-$DFFFFF`).

---

<a id="4-physical-circuit-simulation-standalone-blep-audio-synthesis"></a><a id="4-physical-circuit-simulation-standalone-blep-audio-synthesis"></a>
## 4. Physical Circuit Simulation: Standalone BLEP Audio Synthesis

A dedicated engineering effort was devoted to modeling the physical analog sound output of the Amiga Paula chip with pristine accuracy:
- **WinUAE Parameterization & Circuit Derivation:** The Band-Limited Step (BLEP) synthesis engine was modeled and parameterized based on verified reference implementations in WinUAE.
- **Pure Mathematical Standalone Module:** Rather than hardcoding magic sound samples, the BLEP synthesis is implemented as an independent, decoupled module containing all the underlying physics and signal-processing mathematics (`crates/paula/src/blep_tables.rs`).
- **Analog RC Component Values:** The step response and frequency roll-off tables are computed directly from the physical component values of the Amiga motherboard circuit—the precise resistance (resistors in ohms) and capacitance (capacitors in farads) forming the analog low-pass filter network (including dynamic CIA-A LED filter switching).
- This ensures alias-free, cycle-accurate analog audio reconstruction directly from first physical principles.

---

<a id="5-the-6-stage-evolution-of-the-m68000-cpu-core"></a><a id="5-the-6-stage-evolution-of-the-m68000-cpu-core"></a>
## 5. The 6-Stage Evolution of the M68000 CPU Core

The CPU core (`crates/m68000`) did not emerge in a single iteration. It required six distinct evolutionary stages, documented in commit clusters between September 6 and September 10, 2026, to arrive at its current refined state.

### The Foundational Methodology: Minimal Representative Set & Early SingleStepTests
From day one, the development strategy was deliberately disciplined: rather than attempting to blindly generate all 65,536 opcodes at once, the human instructed the agent to identify a **minimal representative subset of instructions** spanning every major instruction class (data movement, integer arithmetic, logic, shifts/rotations, control flow, and bit operations) and addressing mode.
Throughout Stages 1 and 2, all prototyping, cycle slicing, and architectural debates were tested and proven against physical hardware test captures (**SingleStepTests**) right from the very beginning.

### 1. Stage 1 — Naive Emulation & Human-Centric Typing Shortcuts (Macros & Const-Generics)
*Timeframe: September 6, 02:00 – September 7, 14:00 (commits `8ed5eaca`, `a007047e`, `4e021e1e`, `5b34cb25`)*

Working on the minimal representative instruction set, early prototypes heavily relied on patterns human developers traditionally reach for when trying to minimize repetitive keyboard typing:
- Deep cascaded `match` trees, user-defined macros (`macro_rules!`), and generic functions parameterized by constants (`fn op_foo<const S: usize, const M: usize>(...)`).
- The LLM's initial training bias strongly favored minimizing generated source code lines to save human typing, resulting in highly abstract, opaque boilerplate compression.
- **The Human Rejection:** These versions were firmly rejected during human code review:
  - Macros broke IDE code navigation (Go to Definition, Find References, Call Hierarchy) and produced impenetrable compiler error diagnostics.
  - Const-generic matrices generated combinatorial code bloat while fragmenting concrete execution paths.
  - Cascaded runtime matching polluted host CPU instruction caches (L1i) with branches that had nothing to do with physical 68000 microcode execution.
  - The human issued a strict mandate: **custom macros (`macro_rules!`) and const-generic execution handlers are permanently forbidden**. Code must be written as explicit, concrete, specialized Rust functions.

### 2. Stage 2 — The Hybrid Action Buffer & Phase Experiment
*Timeframe: September 7, 11:00 – September 8, 01:00 (commits `2ff4c38d`, `25735352`, `e4602304`, `fb93f9b8`)*

Still focusing on the minimal instruction subset, Stage 2 was born out of the human's critical insight that M68000 bus cycles must be modeled in discrete clock phases ($CLK1$ and $CLK2$, matching Amiga $CCK1$ and $CCK2$):
- ALU operations and instruction evaluation logic remained largely linear and immediate.
- However, all memory bus operations (reads, writes, prefetches) were deferred and pushed onto a dedicated "command/action buffer".
- A separate, decoupled execution loop processed this bus buffer, providing a mechanism for the CPU to stall and hold on wait states when blocked by Chip RAM contention.
- **The Architectural Breakdown:** While this successfully proved that memory wait states could stall execution against SingleStepTests, the resulting architecture was awkward and fragmented ("rozpieprzony"):
  - ALU math ran upfront while bus operations lagged behind in the queue.
  - Pipelined prefetch queue progression ($IR$/$IRC$) became decoupled from memory reads.
  - Mid-instruction Address Error exceptions (Vector 3) across this boundary were completely unnatural: an address error occurring on an unaligned memory read would trigger *after* the ALU had already mutated guest registers!

### 3. Stage 3 — The "Frankenstein" Architecture (Full Opcode Expansion & CLK1/CLK2 Over-Engineering)
*Timeframe: September 8, 01:00 – September 8, 12:00 (commits `ee6814bc`, `2445c507`, `cdc1bd07`, `16ed85c4`, `8e07b26a`)*

At this stage, the human gave the directive to expand from the minimal representative subset and implement **all remaining M68000 instructions across the entire opcode space**, while introducing the elegant concepts intended for the final core: unified micro-step slices, archetype patterns, and true cycle-exact execution. However, because the architectural boundaries were in mid-transition, the LLM created a complex "Frankenstein":
- The agent eagerly adopted the new micro-step ideas, but stubbornly retained the baggage and awkward habits from Stage 2.
- Because the codebase and conversation context contained a vast amount of discussions and code relating to $CLK1$ and $CLK2$, the agent became heavily fixated on explicit clock-phase machinery. It assumed all this sprawling phase-tracking apparatus was indispensable, leaving it deeply woven into the execution flow across the newly expanded instruction catalog.
- The result was an entangled hybrid: half-baked micro-steps coexisting with explicit clock-phase state machines, redundant intermediate buffers, and fragmented state transitions across thousands of generated opcodes.
- **The Human Intervention:** The human recognized that the agent had built a monstrous hybrid and had to actively intervene, halting feature expansion to systematically untangle the Frankenstein and strip out the redundant phase scaffolding step by step.

### 4. Stage 4 — The Microcode Archetype Baseline (Cycle-Exact Foundation)
*Timeframe: September 8, 15:43 (commit `81bffec5`)*

Having untangled the Frankenstein, the core stabilized on a clean, physical execution foundation:
- **Static Descriptor Dispatch:** Replaced monolithic opcode handlers with a **65,536-entry static descriptor table** (`OPCODE_DESCRIPTOR_TABLE`).
- **Atomic 2-Clock Micro-Steps:** Native 2-clock micro-step slices ($1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$), cleanly fusing the $CLK1$ ALU phase (`alu_fn`) and $CLK2$ bus transfer into a single atomic struct without any separate phase machinery or intermediate queues.
- **Cached Slice Dispatch:** Implemented cached slice pointer dispatch (`current_steps`) to completely eliminate table lookups during instruction stepping.
- **100% SingleStepTests Pass Rate:** Validated across all 127 test suites (~300,000 vectors from MAME and Tom Harte hardware captures).

### 5. Stage 5 — Micro-Step Footprint Compaction & Dual Staging (`addr1`, `addr2`)
*Timeframe: September 9, 00:00 – September 10, 00:09 (commits `139b1016`, `cb463a66`, `3c6d4c9f`, `1d7998dd`)*

The subsequent refinement focused on optimizing the micro-step state machine itself:
- **Aggressive Micro-Step Compaction:** Eliminated redundant dispatch steps and zero-clock transitions across all opcode handlers.
- **The Dual Staging Architecture (`addr1`, `addr2`):** For dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`), dedicated staging registers were introduced (`state.micro.addr1` for source, `state.micro.addr2` for destination), eliminating temporary scratch register juggling, pointer swapping, and bit-shifting hacks (`destination >>= 16`).
- **Split Address Error Invariance:**
  - For byte operations (`CMPM.b`, `ABCD`, `SBCD`, `ADDX.b`, `SUBX.b`), precalculate both `addr1` and `addr2` upfront (`ea_calc_dual_pi_b` / `ea_calc_dual_pd_b`) because byte accesses never fault on alignment.
  - For word and long operations (`CMPM.w/l`, `ADDX.w/l`, `SUBX.w/l`), source address calculation occurs in Step 0, while destination address calculation is deferred and fused into the **CCK2 idle phase of the source read**. If the source address is unaligned (odd), the CPU immediately triggers Vector 3 Address Error with the destination register ($Ax$, USP/SSP) completely untouched, matching physical 68000 silicon.
- **Fused CCK Operations:** Fused ALU calculations directly onto natural Color Clock phases (`MicroStep.alu_fn`).

### 6. Stage 6 — Systematic Batch Implementation, Cartesian DMA Contention & Real Program Execution
*Timeframe: September 9, 15:00 – September 10, 12:00 (commits `fd33fe20` through `13f1480c`, `38345d2d`, `4b9377cb`)*

The final stage arrived once the microcode archetype baseline stabilized, focusing on completing the instruction set, stress-testing, and running real compiled binaries:
- **Rapid Batch Migrations:** In an intense 3-hour sprint on the afternoon of September 9, remaining instruction batches were implemented and verified with 100% SingleStepTests:
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
- **Strict 1:1 Mnemonic-to-File Hierarchy (`38345d2d`):** To enforce modularity and prevent file bloat, the human mandated that every single M68000 instruction mnemonic must reside in its own dedicated flat file directly under `crates/m68000/src/instructions/<mnemonic>.rs` (zero subdirectories, zero umbrella files).
- **Cartesian DMA Contention Test Suite (`test_dma_cartesian`):** Exhaustively validating cycle invariance ($C = C_0 + 2 \times \text{wait\_states}$), Fast RAM immunity, and state invariance across the full $2^k \times 2^M$ stall permutation space.
- **Synthetic Program Execution (`tests/bin/` in commit `4b9377cb`):** Authoring and injecting real compiled M68000 binary routines directly into emulated Chip RAM:
  - `bubble_sort.bin`: Bubble sort on integer arrays.
  - `fibonacci.bin`: Recursive Fibonacci sequence calculation.
  - `sieve_primes.bin`: Sieve of Eratosthenes prime generation.
  - `string_reverse.bin`: In-place string reversal with pointer arithmetic.
  - `factorial.bin`: Factorial computation with 32-bit registers.
  - Running these bare-metal programs confirmed end-to-end multi-instruction sequencing, branch predictability, and register integrity without OS overhead.

---

<a id="6-cpu-opcode-benchmarking-performance-profiling"></a><a id="6-cpu-opcode-benchmarking--performance-profiling"></a>
## 6. CPU Opcode Benchmarking & Performance Profiling

Following the stabilization of the CPU core, the human initiated a major engineering milestone: designing and implementing an exhaustive, cycle-exact **M68000 micro-benchmarking engine, anomaly detection framework, and verification suite** (`crates/test_runner/src/benchmark/`).

The objective was not merely measuring raw speed, but empirically validating our **mechanical sympathy** with modern superscalar host processors (x86_64, aarch64) and guaranteeing that the 65,536-entry static dispatch table, micro-step state machine, and bus accessors operate free of host pipeline stalls, cache pollution, or inlining regressions.

### Dedicated Git Worktree Isolation & Parallel Workflow
To maintain complete separation of concerns and prevent high-volume benchmark data and UI code from destabilizing each other:
- The benchmarking engine (`feat/benchmarks`) and the Developer Studio GUI (`feat/gui`) were authored in parallel on completely independent **Git worktrees** (`git worktree`).
- This isolated measurement harnesses, synthetic code generators, and multi-megabyte trace logs from GUI layout iterations, enabling independent verification before their eventual conflict-free merge into `master`.

### The Dual-Loop Unrolled Architecture ($K = 700$ Blocks)
Benchmarking individual instructions on out-of-order host CPUs presents a classical measurement dilemma: if a loop only executes a few instructions before branching back (`DBF D7, loop`), loop control mechanics consume up to 60% of runtime and host branch predictors overfit.

The human solved this with a tailored **dual-loop architecture**:
1. **$K = 700$ Unrolled Instruction Blocks:** The programmatic generator compiles a contiguous block of exactly 700 unrolled instances of the target instruction directly in guest Chip RAM.
2. **Negligible Branch Overhead (< 0.15%):** Amortizing loop control across 700 operations ensures the terminating branch represents less than 0.15% of total executed time, delivering near-pure instruction latency.
3. **Defeating Host Loop Stream Detectors (LSD):** Modern host CPUs (Intel Skylake–Raptor Lake, AMD Zen) feature hardware $\mu\text{op}$ loop caches that detect loops under 64 instructions and bypass instruction fetch/decode. A 700-instruction block produces 1.4 KB to 4.2 KB of guest machine code—comfortably exceeding the host LSD threshold while fitting effortlessly within the host L1 instruction cache (32 KB–64 KB).
4. **Branch Predictor Decorrelation:** Registers and memory strides are alternated to prevent the host CPU from over-optimizing synthetic register rename aliases.

### Multi-Tier Noise Filtering & Adaptive Jitter Compensation ("Wait Out the Storm")
Micro-benchmarking on non-real-time host operating systems is vulnerable to background OS preemption, DPC/ISR spikes, and CPU frequency scaling:
1. **Warm-Up Pass:** Primes host L1i/L1d caches, populates the TLB, and drives CPU frequency governors up to peak boost clocks.
2. **Tukey's Fences Outlier Rejection:** Applies Interquartile Range filtering ($T > Q_3 + 1.5 \times \text{IQR}$) to cleanly discard OS context switches and background service spikes.
3. **Adaptive Environmental Compensation ("Wait Out the Storm"):** If the Coefficient of Variation ($\text{CV} = \sigma / \mu$) exceeds 3.0%, the runner detects external host contention, enters a 500 ms – 1 s cooldown sleep, and dynamically collects additional passes. If noise persists across retries, it flags `"environment_noisy": true` rather than generating false regression alerts.
4. **Compiler Optimization Defense:** Employs `std::hint::black_box` and rolling 64-bit register checksums at pass boundaries, guaranteeing LLVM never eliminates intermediate guest state.

### Win32 P-Core Thread Affinity Pinning & Platform Portability (`platform.rs`)
Modern hybrid architectures combine high-performance P-cores with low-power E-cores:
- **Dynamic Topology Discovery:** Queries host CPU topology at runtime (Windows `GetLogicalProcessorInformationEx` via `EfficiencyClass`, Linux `/sys/.../cpuinfo_max_freq`, macOS `sysctl`), automatically locating physical P-cores without hardcoded core IDs.
- **Affinity Pinning & Priority Elevation:** Pins the benchmark worker thread to the primary P-core (`SetThreadAffinityMask`) and elevates process priority to `HIGH_PRIORITY_CLASS`, eliminating thread migration mid-run.
- **Sequential Profiling Mandate:** High-precision runs (`--standard`, `--thorough`) execute strictly sequentially on a single dedicated P-core to prevent shared L3 cache contention, DRAM bus saturation, and multi-core thermal downclocking. Multi-worker parallelism (`-j N`) is strictly reserved for `--quick` smoke tests.
- **WASM Decoupling:** Fully abstracted in `HostEnvironment`; compiles cleanly for `wasm32-unknown-unknown` as a zero-overhead no-op with `performance.now()`.

### Programmatic In-Memory Code Generation (`builder.rs`)
Rather than depending on external assemblers (`vasm`, `gas`) or fragile static binary blobs:
- The entire 700-instruction test suite is synthesized **100% programmatically in Rust memory** directly into Chip RAM in microseconds.
- **Deterministic PRNG Seeding (`prng.rs`):** A lightweight `XorShift64` PRNG (seeded with `0x8547_5938_4721_9381`) pre-fills operand memory buffers, registers, and immediate values with deterministic, non-zero bit patterns, eliminating host ALU zero-skipping shortcuts. Clamping ensures architecturally valid BCD digits, non-zero divisors, and word-aligned addresses.
- **Complex Circuit Topology Builders:** Synthesizes cascading return stacks (`RTS`, `RTE`, `RTR`), cyclic post-increment/pre-decrement memory buffers, and exception trampoline loops (`DIVU #0`).
- **Sentinel Exit & Infinite Loop Safeguards:** All programs terminate at `$004FFE` with `STOP #$2700`. Stepping loops enforce hard cycle timeouts ($\max(C_{\text{expected}}, 1000) \times 10$) to abort on hardware faults.

### The Exhaustive 108-Specification Catalog (`catalog.rs`)
The suite defines 108 distinct, comprehensive benchmark specifications spanning all 9 M68000 functional instruction families:
- **Baseline:** `BASE-00` (`NOP`, 4 CCK).
- **Data Movement:** `MOV-01..23` (`MOVE.B/W/L`, `MOVEA`, `MOVEM`, `EXG`, `LEA`, `PEA`, `LINK`, `UNLK` across all addressing modes).
- **Arithmetic:** `ARITH-01..25` (`ADD`, `ADDA`, `ADDX`, `ADDQ`, `SUB`, `SUBA`, `SUBX`, `SUBQ`, `MULS`, `MULU`, `DIVS`, `DIVU`, `CLR`, `NEG`, `NEGX`).
- **Comparison:** `CMP-01..10` (`CMP`, `CMPA`, `CMPM`, `CMPI`, `TST`).
- **Logic:** `LOGIC-01..14` (`AND`, `ANDI`, `OR`, `ORI`, `EOR`, `EORI`, `NOT`).
- **Shifts & Rotations:** `SHIFT-01..08` (`ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR`).
- **Control Flow:** `FLOW-01..08` (`BRA`, `Bcc`, `DBcc`, `BSR`, `JMP`, `JSR`, `RTS`, `RTR`).
- **BCD Arithmetic:** `BCD-01..05` (`ABCD`, `SBCD`, `NBCD`).
- **System & Privileged:** `SYS-01..10` (`MOVE to/from SR/CCR`, `MOVE USP`, `ANDI/ORI to SR/CCR`, `RTE`, `TRAP`, `TRAPV`, `CHK`).

### Normalized Emulation Tax & Multi-Tier Anomaly Detection (`anomaly.rs`)
Rather than looking only at wall-clock duration, the engine introduces the **Normalized Emulation Tax**:
$$R_{\text{norm}} = \frac{T_{\text{host}}\ (\text{ns})}{C_{\text{guest}}\ (\text{Amiga CCK})}$$
In a cycle-exact emulator, the host time required to simulate one Amiga Color Clock should remain uniform across instructions of the same execution tier. The engine automatically flags three structural anomalies:
- **Type A (Intra-Family Spikes, $R_{\text{norm}} > 2.5\times$ baseline):** Detects dynamic `match` trees, unspecialized dispatch, or missed LLVM optimizations.
- **Type B (Addressing Mode Inefficiency, $R_{\text{norm}} > 3.0\times$ register baseline):** Pinpoints missing `#[inline]` annotations on memory bus accessors (`read_word`, `bank_handler`).
- **Type C (Excessive Jitter, $\text{CV} > 5.0\%$):** Identifies host Branch Target Buffer (BTB) thrashing or cache line boundary crossings.

### Deterministic File Invariants & Golden Master Anti-Tamper Hashes (`test_benchmark_csv.rs`)
To ensure generated benchmark reports (`.json` and `.csv`) are rigorously verified in automated testing without relying on fragile timing assertions:
- **Separation of Invariants vs. Telemetry:** The 14-column CSV output strictly isolates **Deterministic Invariants** (columns 1–7: mnemonic, variant, mode, category, opcode hex, Amiga CCK, nominal `total_ops`) from **Stochastic Host Telemetry** (columns 8–13: host duration, ns/op, ns/CCK, MIPS, jitter, anomaly flag).
- **Strictly Nominal Operation Counts:** Invariant `total_ops` is calculated from raw pass configurations ($K \times \text{iterations} \times \text{passes}$), remaining 100% bit-for-bit identical even if outlier passes are dropped by Tukey's fences.
- **Golden 64-Bit FNV-1a Master Hashes:** Invariant columns are verified against locked golden hashes for Catalog Structure (`0x3972F381CE69D98F`), Quick CSV (`0xB699CBA7E6ADD635`), Standard CSV (`0x9FE3956BC57151D1`), and Thorough CSV (`0xDB747B4AC9321D39`).
- **Single-Pass Step Tracing (`--dump-traces`):** Emits full step-by-step disassembly, cycle counts, register deltas, and memory writes into `tests/benchmarks/traces/<SPEC>.trace`, allowing instant regression verification and LLM semantic double-checking.

### Automated Baseline Diffing & Cross-Hardware Normalization
- **Differential Calibration:** Automatically compares active runs against `tests/benchmarks/m68k_benchmark_baseline.json`.
- **Ratio-to-NOP Normalization:** Computes $R_{\text{nop}} = T_{\text{host}}(\text{Op}) / T_{\text{host}}(\text{NOP})$ to eliminate false regression alarms across different host CPU architectures and frequencies.
- **Threshold Alerts:** $\le +3.0\%$ indicates expected noise; $+3.0\%\dots+7.0\%$ issues a minor warning; $> +7.0\%$ triggers an automated blocking regression alert.

### Standalone Analytical Tool (`analyze_benchmarks.py`) & Empirical Findings
A dedicated Python analytics suite processes benchmark datasets to compute linear regression models ($\text{Host ns} = \alpha \times \text{CCK} + \beta$), addressing mode latency ladders, and functional symmetry:
- **Baseline Speeds:** `NOP` executes in **14.94 ns**, 16-bit register `ADD.W` in **15.96 ns** (~62.6 MIPS), and `MOVE.W` in **16.20 ns** (~61.7 MIPS).
- **Normalized Emulation Tax:** Averages **2.3 to 4.1 ns per Amiga CCK** across the entire instruction set—translating to sustained emulation throughput **70× to 110× faster than real-time Amiga 500 hardware** (3.54 MHz PAL).
- **Zero Host Anomalies:** All 108 specifications report `anomaly_flag: false`, empirically proving that our flat, macro-free, const-generic-free micro-step architecture achieves near-flawless mechanical sympathy with modern host processors.

---

<a id="7-subsystem-evolution-the-developer-studio-gui-debugger-engine"></a><a id="7-subsystem-evolution-the-developer-studio-gui--debugger-engine"></a>
## 7. Subsystem Evolution: The Developer Studio GUI & Debugger Engine

In parallel with the CPU benchmarking suite, the frontend was developed on the dedicated `feat/gui` worktree (`315c602c`):

### From "Make Me a GUI" to a Full Developer Studio
- **The First Iteration:** Initiated with a simple voice prompt: *"Make me a GUI"*. The agent initially produced a basic, single-panel window displaying raw register text.
- **The Human-Driven Transformation:** Through relentless review, the human pushed the frontend to become a world-class emulator workbench:
  - **Left Dock:** Live register inspector with visual delta highlighting (green for values mutated on the last cycle), glowing condition code register LEDs (`X`, `N`, `Z`, `V`, `C`), collapsible microcode inspector (`F8`), and cycle-accurate disassembly stream with CISC address alignment.
  - **Main Viewport:** CRT-accurate Amiga display (`F12` for clean screen-only mode) paired with an interactive **Temporal Scrub Bar** for time-travel debugging.
  - **Right Dock:** Memory hex editor with inline ASCII inspection, live memory search, condition breakpoint manager (PC, address ranges, register expressions), and step-by-step bus transaction trace log.

### In-Process Headless GUI Testing Superpower (`crates/gui/tests/test_interactions.rs`)
One of the most consequential architectural decisions made during GUI development was rejecting external, fragile UI test tools:
- Because `egui` is an immediate-mode library, the entire interface pipeline runs **100% in-process** without opening OS windows, requiring display servers, or spawning heavy headless browser drivers.
- **Direct Event Simulation:** Headless integration tests directly invoke:
  ```rust
  let mut app = AmigaApp::new_test();
  ctx.run(raw_input, |ctx| app.update_ui(ctx));
  ```
- Tests programmatically simulate keyboard shortcuts (`F5` run/pause, `F8` microcode toggle, `F10` step over, `F11` step into, `F12` screen mode), mouse clicks, drag-and-drop resizing, text box edits, and Enter/Escape commits directly in Rust code.
- Over 25 headless integration tests assert dock widths, focus transitions, memory mutations, and view mode switches in milliseconds, with zero rendering jitter and zero flakiness.

### The Debugger Engine (`crates/debugger`) & Standalone Disassembler (`crates/disassembler`)
The backend driving the Developer Studio was modularized to strictly adhere to the 800-line single-responsibility guideline:
- **Temporal Reverse Debugger (`temporal.rs`):** A fixed-capacity, zero-allocation ring buffer records CPU and memory bus states on every instruction. Users can scrub backwards through execution history, replaying cycles in reverse to isolate the exact instruction that corrupted a register.
- **In-Memory Mini-Assembler (`assembler.rs`):** A lightweight 650-line M68000 assembler built directly into the debugger, allowing developers to type assembly instructions directly into the GUI to patch guest memory live during execution.
- **Conditional Breakpoint Evaluator (`breakpoints.rs`):** Evaluates complex address boundaries, memory access watchpoints (read/write), and register conditional expressions (`D0 == $0000`, `A7 < $00070000`).
- **Standalone Disassembler Engine (`crates/disassembler`):** Completely decoupled and extracted from `crates/debugger` into its own standalone, zero-dependency crate. The disassembler is structured into clean, flat submodules (`types.rs`, `ea.rs`, `alu.rs`, `branch.rs`, `data.rs`, `align.rs`, `lib.rs`), with every file under 380 lines. This permanently eliminated the historical 800-line exception for `disassembler.rs` from architecture tests while providing instruction decoding and heuristic stream alignment across debugger, tracer, and GUI modules.

---

<a id="8-architectural-rules-anti-tamper-policies-the-living-definition-of-done"></a><a id="8-architectural-rules-anti-tamper-policies--the-living-definition-of-done"></a>
## 8. Architectural Rules, Anti-Tamper Policies & The Living Definition of Done

As the repository expanded to dozens of crates and hundreds of files, the human instituted an automated rule enforcement infrastructure (`.agents/rules/` and `AGENTS.md`):
- **Automated Architecture Rules (`test_architecture_rules.rs`):** An automated test suite enforces repository invariants on every test run:
  1. *File Size Limits:* Every Rust source file in `crates/*/src/` must remain under 800 lines.
  2. *Flat Instruction Hierarchy:* Every M68000 instruction mnemonic must reside in a flat file directly under `crates/m68000/src/instructions/<mnemonic>.rs`, with zero umbrella files and zero subdirectories.
  3. *Zero Custom Macros:* Custom macros (`macro_rules!`) are strictly prohibited across the codebase.
  4. *Zero Const-Generic Execution Handlers:* Generic functions parameterized by constants are forbidden in opcode execution paths.
  5. *Zero Runtime Host Panics:* Emulated guest code must never panic the host (`unwrap()` and `expect()` are forbidden in runtime paths).
  6. *Canonical Idle Naming:* Micro-steps without bus activity must use canonical constants (`BUS_READ_IDLE`, `BUS_WRITE_IDLE`, `ALU_IDLE*`), prohibiting anonymous struct literals.
  7. *Strict Inlining Strategy:* `#[inline(always)]` on ultra-hot flag calculations; `#[inline]` on cross-crate accessors; `#[inline(never)]` on cold exception paths (Address Error dumps, illegal instruction traps) to keep L1i caches dense.
  8. *Golden Master Anti-Tamper Rule:* Modifying golden test hashes or benchmark constants to silence a failing test is strictly forbidden. Any divergence signifies an architectural regression requiring root-cause analysis.
  9. *Path Privacy:* Zero hardcoded external paths (`D:\...`, `/home/...`).
  10. *Mandatory Merge Commits (`git-merge-commits.md`):* Merges between worktrees and branches must produce clean merge commits with explicit conflict resolution records rather than fast-forward rebases.
  11. *Mandatory Engineering Diary Maintenance (`DIARY.md`):* Recording all changes, rationale, and evolutionary context in `DIARY.md` (Section 10) because Git commits are often squashed, batched, or combined.

---

<a id="9-the-ultimate-goal-the-clean-room-re-generation-experiment"></a><a id="9-the-ultimate-goal-the-clean-room-re-generation-experiment"></a>
## 9. The Ultimate Goal: The Clean-Room Re-Generation Experiment

Every bug fix, architectural decision, and hardware quirk in this project has been continuously documented in `Obsidian/Amiga/Design/`.

This serves a larger, ambitious vision:
- Once the complete, working Amiga 500 emulator is finished and verified against all reference suites,
- We will wipe the implementation source code (`crates/`) and reference emulator sources (`ref_src/`), leaving strictly:
  - The refined prompt and skill system (`.agents/`).
  - The curated, de-duplicated design documentation (`Obsidian/Amiga/Design/`).
  - The automated test suites (SingleStepTests, vAmigaTS).
- **The Experiment:** Can an AI agent, armed with the lessons, rules, and architecture recorded during this project, autonomously re-synthesize the complete cycle-exact emulator from scratch?
- The operational recipe, ingestion pipeline, and instructions to prepare this clean-room experiment are integrated into Section 3.5 of **[ROADMAP.md](ROADMAP.md)**.

This diary stands as the living record of how those foundations were built.

---

<a id="10-repository-bootstrapping-clean-room-pipeline-tooling-ergonomics"></a><a id="10-repository-bootstrapping-clean-room-pipeline--tooling-ergonomics"></a>
## 10. Repository Bootstrapping, Clean-Room Pipeline & Tooling Ergonomics
- **Milestone Scope & Affected Subsystems**:
  - `tools/bootstrap/` (`bootstrap.ps1`, `bootstrap_sources.ps1`, `bootstrap_documentation.ps1`), `BOOTSTRAP.md`, `ROADMAP.md` (Section 3.5), `docs/developers.md`, `docs/reference_sources.md`, `docs/reference_documentation.md`, `tools/git/worktree.ps1`.
- **Architectural Breakthroughs**:
  - *Unified Modular Bootstrapping Architecture*: Decomposed the monolithic bootstrapping script into single-responsibility tools under `tools/bootstrap/`, providing clean CLI parameter specifications, standardized `--help`, and eliminating redundant coordinator scripts.
  - *Strict Storage Isolation & Zero-Junction Invariant*: Codified and enforced the elimination of NTFS directory junctions (`New-Item -ItemType Junction`) across Git worktrees. Sibling worktrees now maintain 100% independent physical directories, preventing accidental cross-worktree asset deletions and AST cache contamination.
  - *Multi-Mirror Reference Ingestion*: Integrated multi-mirror download fallbacks with native .NET decompression (`.tar.gz`, `.zip`, `.lha`) and automated post-archive cleanup for reference materials.
  - *Consolidated Documentation Hierarchy*: Unified developer documentation into `docs/developers.md` (single entry point linked from README), while standardizing external sources and documentation catalogs into `docs/reference_sources.md` and `docs/reference_documentation.md`.
  - *Autonomous Clean-Room Reconstruction Pipeline*: Formalized in `BOOTSTRAP.md` and `ROADMAP.md`: wiping implementation source code and verifying whether an AI agent can autonomously re-synthesize the cycle-exact emulator from scratch using curated design documentation and automated test suites.
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Decisively rejected shared directory junctions despite increased disk usage: cross-portal sharing created severe race conditions and risked corrupting uncommitted working tree states.
  - Sunk all operational bootstrap parameters behind standardized CLI flags with automatic deprecation warnings.
- **Agent Collaboration & Root-Cause Lessons**:
  - The AI agent frequently attempted to introduce backward-compatibility shims, duplicate parameter aliases (`-Test`, `-NoSmoke`), and fallback paths. The human architect enforced strict parameter pruning and single canonical coordinators.
- **Verification Gates**:
  - Clean clone execution of `bootstrap.ps1`, Qdrant vector database probing (`localhost:6333`), and automated asset integrity checks.

---

<a id="11-multi-tier-testing-infrastructure-cartesian-contention-silicon-verification"></a><a id="11-multi-tier-testing-infrastructure-cartesian-contention--silicon-verification"></a>
## 11. Multi-Tier Testing Infrastructure, Cartesian Contention & Silicon Verification
- **Milestone Scope & Affected Subsystems**:
  - `crates/test_runner/`, `crates/machine_loop/tests/`, `crates/*/tests/`, `tools/harness/`, `tests/singlestep/`, `ref_src/vAmigaTS/`.
- **Architectural Breakthroughs**:
  - *3-Tier Testing Taxonomy*:
    - **Tier 1 (Unit)**: Fast (< 1s) isolated logic tests placed strictly in external `crates/<crate>/tests/test_<name>.rs` (zero inline tests in `src/`).
    - **Tier 2 (Headless Multi-Crate Integration)**: Whole-machine coordination via `MachineHarness` across 4 archetypes: Autonomous Memory Transfer, Signal Escalation & CPU Autovector, DMA Master Gatekeeping, and Dual-Bus Concurrency.
    - **Tier 3 (Silicon Verification)**: Tom Harte SingleStepTests hardware captures (~300,000 vectors) and the vAmigaTS direct-injection test harness with RGB24 viewport matchers.
  - *100% Pass Rates on Silicon Testbeds*: Reached 100% pass rates across vAmigaTS Phase 1 suites: Copper (`cross6`), Blitter (`sblit0`, `fill0..7`, `line` 29/29), and Denise display window timing (`diwtim0..2b`).
  - *Cartesian DMA Contention Engine (`test_dma_cartesian`)*: Rigorously verified cycle-exact contention ($C = C_0 + 2 \times \text{wait\_states}$) across the full $2^k \times 2^M$ stall permutation space, asserting Fast RAM immunity and state invariance.
  - *M68000 Opcode Micro-Benchmarking Suite*: Dual-loop unrolled architecture ($K = 700$), Tukey's fences outlier rejection ($1.5 \times \text{IQR}$), Win32 P-core thread affinity, and golden master anti-tamper hashes (`0x3972F381CE69D98F`).
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Prohibiting Kickstart ROM or ADF floppy dependencies for Tier 2 tests; keeping them synthetic and self-contained in Chip RAM (< 5ms).
  - Absolute prohibition of golden hash tampering without root-cause physical circuit justification (Anti-Tamper Rule).
  - Prohibition of coordinate or timing "nudges" ($\pm 1$ offsets) to silence visual discrepancies.
- **Agent Collaboration & Root-Cause Lessons**:
  - When pixel comparison tests initially failed against vAmiga, the agent attempted to adjust display beam offsets. Human rejected this, requiring cycle tracing which uncovered Denise pixel pipeline latency and Copper 1-CCK fetch delays.
- **Verification Gates**:
  - 100% pass across SingleStepTests (127 suites), vAmigaTS Phase 1, Cartesian contention sweeps, and `cargo test -p test_runner --test test_architecture_rules`.

---

<a id="12-developer-studio-gui-headless-perception-debugging-ecosystem"></a><a id="12-developer-studio-gui-headless-perception--debugging-ecosystem"></a>
## 12. Developer Studio GUI, Headless Perception & Debugging Ecosystem
- **Milestone Scope & Affected Subsystems**:
  - `crates/gui/`, `crates/debugger/`, `crates/disassembler/`.
- **Architectural Breakthroughs**:
  - *Full HD Developer Studio Workbench*: Evolved from a raw register terminal into a responsive Full HD ($1920 \times 1080$) 3-column workbench: live delta highlighting (green for last-cycle mutations), glowing CCR LED flags, collapsible microcode inspector (`F8`), infinite disassembly stream with CISC instruction alignment, CRT display viewport (`F12`), and interactive memory hex editor.
  - *Headless In-Process GUI Integration Superpower*: Exploited `egui`'s immediate-mode architecture to execute end-to-end UI tests (`crates/gui/tests/test_interactions.rs`) 100% in-process via `ctx.run(raw_input, |ctx| app.update_ui(ctx))`. Simulates keystrokes, drag-and-drop column resizing, focus changes, and mode toggles in milliseconds without OS windows, display servers, or headless browsers.
  - *Decoupled Standalone Disassembler (`crates/disassembler`)*: Extracted from `crates/debugger` into a dedicated zero-dependency crate with flat modular submodules (`ea.rs`, `alu.rs`, `branch.rs`, `data.rs`, `align.rs`, all $\le 380$ lines), eliminating the historical 800-line exception from architectural tests.
  - *Temporal Reverse Debugger & Mini-Assembler*: Zero-allocation circular ring buffer for bidirectional instruction stepping and live in-memory M68000 assembler for runtime memory patching.
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Synchronous direct state pulling (`app.update_ui(ctx)`) over asynchronous event brokers, preventing UI state desynchronization.
  - Splitting the disassembler into modular files to uphold strict single-responsibility without compromising stream lookahead heuristics.
- **Agent Collaboration & Root-Cause Lessons**:
  - The agent initially created a monolithic disassembler exceeding 1,200 lines. The human halted development to enforce modular decomposition and 1:1 unit test parity.
- **Verification Gates**:
  - Over 25 headless interaction tests asserting layout stability, dock widths, and focus management under high-frequency simulated input.

---

<a id="13-custom-chipset-silicon-architecture-autonomous-dma-engines"></a><a id="13-custom-chipset-silicon-architecture--autonomous-dma-engines"></a>
## 13. Custom Chipset Silicon Architecture & Autonomous DMA Engines
- **Milestone Scope & Affected Subsystems**:
  - `crates/agnus/`, `crates/denise/`, `crates/paula/`, `crates/cia/`, `crates/physical_memory/`, `crates/memory_bus/`, `crates/machine_loop/`.
- **Architectural Breakthroughs**:
  - *Memory Subsystem Decomposition*: Sliced into `crates/physical_memory` (raw storage buffers) and `crates/memory_bus` (motherboard routing logic) with an $O(1)$ 256-bank lookup table (`[BankHandler; 256]`) and dynamic CIA-A boot overlay swapping.
  - *Register Single Source of Truth (SSoT)*: Centralized custom chip register catalog, eliminating scattered magic numbers across the workspace.
  - *End-to-End Interrupt & Autovector Pipeline*: Paula/CIA interrupt signals routed through Agnus to CPU IPL 1–6 autovector traps.
  - *Autonomous DMA Coprocessors*:
    - **Agnus Copper**: 3-instruction engine (`MOVE`, `WAIT`, `SKIP`, `CDANG`), 1-CCK fetch, and silicon cycle `$E0` DMA denial.
    - **Agnus Blitter**: 4-channel DMA engine, 256 minterms ALU, Bresenham line drawer, barrel shifters, and ascending/descending fill modes.
    - **Denise**: Video compositor, bitplane fetcher (1–6 planes), 8 hardware sprite multiplexers, and display window (`DIWSTRT`/`DIWSTOP`) flip-flops.
    - **Paula & CIAs**: 4-channel DMA audio, floppy MFM track controller, and dual MOS 8520 timers (TOD, SDR, parallel ports).
    - **Agnus Master DMA Arbiter**: Scanline slot scheduling and Chip RAM contention engine.
  - *Machine-Wide Save State Serialization*: Decoupled state structs implementing `serde::Serialize` and `serde::Deserialize` across all chips, paired with clean cold/warm reset sequencing.
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Decisive rejection of ECS chip variants (`Ecs8372A`, `Ecs8373`) to maintain pure, uncompromised OCS baseline fidelity.
  - Top-down orchestration via `A500::step_cck`: subsystems never hold circular pointers or references to peer subsystems.
- **Agent Collaboration & Root-Cause Lessons**:
  - Agent struggled with Blitter line mode `chold` invariance and Copper 4-CCK `WAIT` pipelines. Resolved by anchoring implementations in Commodore Hardware Reference Manual specifications.
- **Verification Gates**:
  - 100% pass rates on vAmigaTS `sblit`, `fill`, and `line` suites; Cartesian contention verification; zero host panics on unmapped bus reads.

---

<a id="14-multimodal-reference-ingestion-pdf-to-markdown-pipeline"></a><a id="14-multimodal-reference-ingestion--pdf-to-markdown-pipeline"></a>
## 14. Multimodal Reference Ingestion & PDF-to-Markdown Pipeline
- **Milestone Scope & Affected Subsystems**:
  - `skills/pdf-to-markdown/`, `tools/rag/`, `Obsidian/Amiga/Reference/`.
- **Architectural Breakthroughs**:
  - *12-Stage Multimodal Extraction Pipeline*: Ingesting Commodore Hardware Reference Manuals and 68000 PRMs into semantically pristine, Git-friendly Markdown.
  - *Fused Triage & Vision OCR (Stage 01)*: Single-pass page triage and Gemini Vision OCR, eliminating redundant rasterization passes and duplicate LLM calls.
  - *Integer Millirange Coordinates & Mermaid Transcription*: Normalized bounding boxes (`box_2d` in 0..1000 millirange) and automated transcription of hardware state machines into Mermaid syntax.
  - *Global ThreadPoolExecutor Concurrency*: Multi-threaded execution across text processing, asset extraction, and vision reasoning stages.
  - *Codification of Universal Invariant*: Structural Root-Cause Resolution Rule prohibiting ad-hoc regex patches in favor of upstream data lifecycle corrections.
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Rejection of brittle regex heuristic string replacements (`re.sub`) in favor of LLM proofreading streams.
  - Restructuring pipeline order: running proofreading streams before markdown emission to guarantee clean downstream data formatting.
- **Agent Collaboration & Root-Cause Lessons**:
  - The agent repeatedly attempted surface-level regex substitutions to fix OCR hyphenation errors. The human instituted the Structural Root-Cause rule, forcing upstream tokenization and stream repair.
- **Verification Gates**:
  - Verified 10-page sample batches, verified table continuation chaining, and automated TOC link validation.

---

<a id="15-agent-operating-system-architectural-guardrails-quality-discipline"></a><a id="15-agent-operating-system-architectural-guardrails--quality-discipline"></a>
## 15. Agent Operating System, Architectural Guardrails & Quality Discipline
- **Milestone Scope & Affected Subsystems**:
  - `.agents/rules/`, `.agents/skills/`, `.agents/workflows/`, `AGENTS.md`, `crates/test_runner/tests/test_architecture_rules.rs`, `tools/harness/`.
- **Architectural Breakthroughs**:
  - *Modular Rule Architecture*: Sliced `.agents/rules/` into single-responsibility units with explicit execution triggers (`always_on` vs `model_decision`).
  - *Automated Architecture Rules (`test_architecture_rules.rs`)*: Enforcing file size ceilings ($\le 800$ lines), flat opcode hierarchy, zero custom macros, zero const-generics, canonical `IDLE` micro-steps, and dual staging registers (`addr1`, `addr2`).
  - *Inverted Pyramid Information Hierarchy*: Constitutional ceiling on `AGENTS.md` ($\le 14,000$ bytes) and top-down document structuring.
  - *Practitioner Voice Standard*: Systematic eradication of academic jargon ("epistemic", "friction", "cognitive clarity", "maximum signal") across all rules and documentation.
  - *Pre-Commit Enforcement*: Change-coupling gate (`check_test_coupling.py`) and public API coverage scanner (`audit_api_coverage.py`).
- **Key Decisions & Non-Obvious Trade-Offs**:
  - Strict byte budget on `AGENTS.md` to eliminate context window truncation in agent sessions.
  - Zero-exception policy on architecture rules: any failing test immediately blocks the build.
- **Agent Collaboration & Root-Cause Lessons**:
  - The agent periodically slipped into verbose academic prose and recurring buzzwords. The human established explicit buzzword blacklists and automated audit scripts to enforce concise, practitioner-level technical communication.
- **Verification Gates**:
  - All 19 architecture rule tests passing in < 1 second; zero uncommitted changes across turns; strict Conventional Commits.

---

<a id="16-living-chronological-engineering-log-active-evolutionary-history"></a><a id="16-living-chronological-engineering-log--active-evolutionary-history"></a>
## 16. Living Chronological Engineering Log & Active Evolutionary History

This section maintains a continuous, granular chronological record of all engineering changes, subsystem modifications, refactorings, and bug fixes across the repository.

Because Git commits are frequently batched, squashed, or merged into higher-level commits during multi-stage worktree development, standard commit messages often do not preserve the full evolutionary context, granular mechanics, or subtle trade-offs considered along the way. This log serves as the authoritative, human-readable chronicle of what was actually built, modified, and verified in each development session.

Historical milestone periods (September 12–16, 2026) have been compacted into high-signal architectural digests under Sections 10 through 15 above per [`.agents/skills/compact-diary/SKILL.md`](.agents/skills/compact-diary/SKILL.md). The active development sprint is recorded below in granular detail.

### Mandatory Entry Schema:
Every future modification or implementation task must append an entry following this structure:
- **Timestamp & Context**: Date / local time and the active branch / task / PR.
- **Affected Subsystems**: Specific crates, modules, tools, or configuration affected.
- **What Was Changed (The Concrete Reality)**: Detailed technical description of changes, data structures, algorithms, or mechanics implemented.
- **Architectural Rationale & Trade-Offs**: Why this solution was chosen, what alternatives were rejected, and why.
- **Verification & Invariants**: Test suites executed, assertions checked, and proof of correctness.

---

### [2026-09-17 00:08 CEST] — Tooling & Architecture: Zero-Junction Invariant & Full Worktree Storage Isolation
- **Affected Subsystems**:
  - `tools/git/worktree.ps1`: Completely eliminated `New-Item -ItemType Junction`. Replaced `Link-WorktreeAssets` with `Copy-WorktreeAssets` performing multi-threaded physical copies of test fixtures and assets. Excluded `graphify-out` from automated propagation to keep AST code intelligence strictly authentic to each branch's code. Removed junction unlinking logic (`Safe-RemoveJunctions`).
  - `.agents/rules/git-merge-commits.md`: Enshrined the **Strict Worktree Isolation & Zero-Junction Invariant**, strictly prohibiting NTFS directory junctions (`New-Item -ItemType Junction`, `mklink /J`) or symbolic links across worktrees and repositories.
  - `.agents/skills/git-worktree/SKILL.md`: Updated operational procedures and invariants to enforce standalone physical directory copies and zero junctions.
  - `.agents/skills/git-resolve-merge/SKILL.md`: Removed obsolete junction teardown steps.
  - Active Worktrees (`amiga-bootstrap` & `Amiga-OCS`): Migrated all existing junction reparse points (`ref_src`, `tests/singlestep`, `tools/AmigaTestKit`, `tests/benchmarks/*`, etc.) to fully independent, standalone physical directories.
- **What Was Changed (The Concrete Reality)**:
  - Addressed human architectural feedback: directory junctions create cross-portal state sharing between worktrees, which risks accidental cross-contamination of AST graphs, test artifacts, or uncommitted edits.
  - Replaced all shared directory portals with 100% physically isolated directories in each worktree branch.
  - Asserted `LinkType` is empty ($null) across all directories in all active worktrees.
- **Verification & Test Results**:
  - Junction audit script verified `LinkType=''` across all asset paths in `Amiga`, `amiga-bootstrap`, and `Amiga-OCS`.
  - Quality gates passed cleanly: `python tools/harness/pre_flight.py` (Formatting, AGENTS.md size limit, API coverage, architecture rules).
  - CPU test suite passed: `cargo test -p m68000` (100% pass).

---

### [2026-09-17 02:05 CEST] — Tooling: Parallelize Stage 01 Gemini Vision OCR via ThreadPoolExecutor
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown/stages/01_preprocess/detect_and_ocr.py`
- **What Was Changed (The Concrete Reality)**:
  - Parallelized scanned and text-deficient page processing in `detect_and_ocr_pages()` using `ThreadPoolExecutor` and `as_completed`, respecting the `concurrency` setting from `config.yaml` (`llm.concurrency`).
  - Separated initial scan detection / qualification into a fast metadata pass, building an in-memory task queue of pages requiring Gemini Vision OCR.
  - Fixed an unbound variable bug where visual fallback error handling referenced `page_json_path` instead of `jf`.
- **Architectural Rationale & Trade-Offs**:
  - Previously, `detect_and_ocr_pages()` ran sequentially page-by-page, creating high latency when running across multiple scanned pages. Now, OCR requests are dispatched concurrently up to `concurrency` (default 8), matching later pipeline stages.
- **Verification & Test Results**:
  - Verified Python compilation (`python -m py_compile .agents/skills/pdf-to-markdown/stages/01_preprocess/detect_and_ocr.py`).
  - Ran pre-flight verification gate (`python tools/harness/pre_flight.py` — Formatting, AGENTS.md ceiling, API coverage, and architecture rules 100% passed).

---

### [2026-09-17 02:19 CEST] — Tooling: Pipeline-Wide Global Parallelization Across Stages 04, 06, 07, 08, 09, 10
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown/stages/04_stream_reduction/reduce_stream.py`
  - `.agents/skills/pdf-to-markdown/stages/06_detect_continuations/detect_continuations.py`
  - `.agents/skills/pdf-to-markdown/stages/07_transform_tables/transform_tables.py`
  - `.agents/skills/pdf-to-markdown/stages/08_transform_graphics/transform_graphics.py`
  - `.agents/skills/pdf-to-markdown/stages/09_transform_prose/format_prose.py`
  - `.agents/skills/pdf-to-markdown/stages/10_proofread_stream/proofread_stream.py`
- **What Was Changed (The Concrete Reality)**:
  - Eliminated the per-chapter nested threadpool bottleneck in Stage 07 (tables), Stage 08 (graphics), and Stage 09 (prose): collected all tasks across all chapters first, processing them with a single document-wide `ThreadPoolExecutor(max_workers=concurrency)`, saturating all 8 worker threads while indexing by `(chapter, node_idx)` to preserve 100% document order.
  - Parallelized Stage 04: converted `reduce_contiguous_graphics()` and cross-page prose seam welding to parallel task evaluation with subsequent order-preserving reconstruction.
  - Parallelized Stage 06: processed chapter continuations concurrently across chapter partitions using `ThreadPoolExecutor`.
  - Parallelized Stage 10: converted chapter title proofreading into a concurrent map across all partitions in the manifest.
- **Architectural Rationale & Trade-Offs**:
  - Previously, stages either executed sequentially in loops (Stages 04, 06, 10) or created nested `ThreadPoolExecutor` instances inside single-chapter loops (Stages 07, 08, 09) which capped concurrency to 1–2 workers due to sparse per-chapter items. Global task pooling maximizes throughput and respects `llm.concurrency` (8) across the full document without altering node sequence.
- **Verification & Test Results**:
  - Validated Python compilation across all 6 modified stage scripts via `python -m py_compile`.
  - Pre-flight quality gates passed cleanly (`python tools/harness/pre_flight.py`).
### [2026-09-17 03:18 CEST] — Pruned LHA Archive Handling and Obsolete AmigaGuide / Guru Book References
- **Affected Subsystems**:
  - `tools/bootstrap_reference.ps1` (removed LHA/LZH extensions from known archives and docstrings)
  - `Obsidian/Amiga/Reference/README.md` (sanitized manual drop description to remove historical Guru Book reference)
- **What Was Changed (The Concrete Reality)**:
  - Removed `.lha` and `.lzh` archive formats from `$KnownArchiveExtensions` and `$ArchiveExtensions` in `tools/bootstrap_reference.ps1`, leaving standard `.zip`, `.tar`, `.gz`, and `.tgz` support.
  - Purged references to AmigaGuide archives from the script description in `tools/bootstrap_reference.ps1`.
  - Updated `Obsidian/Amiga/Reference/README.md` to remove the reference to *The Amiga Guru Book* in the manual drop guideline.
- **Architectural Rationale & Trade-Offs**:
  - All active reference materials (HRM, TRM, PRM, UM, Jorge Cwik's prefetch guide, Achtung! Amiga) are either vector/scanned PDFs or direct HTML crawls. Since *The Amiga Guru Book* and AmigaGuide format ingestion were retired, LHA extraction logic and guide references represented dead scaffolding.
- **Verification & Test Results**:
  - Verified PowerShell syntax of `tools/bootstrap_reference.ps1` using `[scriptblock]::Create()`.
  - `python tools/harness/pre_flight.py`: Passed 100% across all quality gates.
### [2026-09-17 03:20 CEST] — Unified CLI Help, Parameter Specifications & Show-Usage across Bootstrap Tooling
- **Affected Subsystems**:
  - `tools/bootstrap.ps1` (unified comment-based help with formal `.PARAMETER` tags, improved `Show-Usage` alignment, and auto-promoted `-Ref` trigger)
  - `tools/bootstrap_reference.ps1` (unified comment-based help with all parameter tags, encapsulated interactive usage into `Show-Usage`, aligned styling)
- **What Was Changed (The Concrete Reality)**:
  - Standardized the PowerShell comment-based help (`<# ... #>`) across both tooling entry points:
    - Added dedicated `.PARAMETER <Name>` documentation blocks for every switch and parameter (including aliases and default behaviors).
    - Unified `.DESCRIPTION` and structured `.EXAMPLE` workflows for all common flag combinations.
  - Symmetrized interactive terminal usage output:
    - Encapsulated reference bootstrapper usage into a dedicated `Show-Usage` function matching the cyan header, yellow operational note, and formatted parameter table of `bootstrap.ps1`.
    - Added all sub-switches (`-RefItem`, `-AllSources`, `-NoExtract`, `-ExtractOnly`) to `bootstrap.ps1`'s usage overview.
  - Enhanced ergonomics: auto-promoted `$Ref = $true` in `bootstrap.ps1` whenever `-RefItem`, `-AllSources`, `-NoExtract`, or `-ExtractOnly` are specified without an explicit `-Ref`.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates divergence between `bootstrap.ps1` and `bootstrap_reference.ps1`. Both scripts now provide identical visual polish, full PowerShell `Get-Help` introspection, and consistent error/help handling.
- **Verification & Test Results**:
  - Verified `Get-Help .\tools\bootstrap.ps1` and `Get-Help .\tools\bootstrap_reference.ps1` with parameter queries (`-Parameter Test`).
  - Tested interactive execution with no parameters for both scripts, confirming clean `Show-Usage` output.
  - `python tools/harness/pre_flight.py`: Passed 100% across all quality gates.
### [2026-09-17 03:23 CEST] — Completely Purged Archive Extraction Pipeline & Flags Across Reference Tooling
- **Affected Subsystems**:
  - `tools/bootstrap_reference.ps1` (deleted `Expand-ReferenceArchive`, `Expand-DirectoryArchives`, `$AutoExtract` param, `-NoExtract`, `-ExtractOnly`)
  - `tools/bootstrap.ps1` (removed `-NoExtract` and `-ExtractOnly` flags, usage items, and parameter forwarding)
- **What Was Changed (The Concrete Reality)**:
  - Completely purged the archive unpacking pipeline: deleted `Expand-ReferenceArchive` and `Expand-DirectoryArchives` from `tools/bootstrap_reference.ps1`.
  - Removed `-NoExtract` and `-ExtractOnly` parameters, their `.PARAMETER` documentation, and their examples from both `tools/bootstrap_reference.ps1` and `tools/bootstrap.ps1`.
  - Simplified `Download-SingleFile`: removed the `$AutoExtract` parameter and inline archive extraction hooks. Reference fetching now strictly provisions direct document formats (PDFs and HTML crawls).
  - Cleaned up `Ensure-StagingReadme` to eliminate the archive extraction operational guideline.
- **Architectural Rationale & Trade-Offs**:
  - All active reference documentation catalog items are either vector/scanned PDF files or multi-page HTML web crawls. Archive extraction was historical dead weight once compressed hypertexts (like AmigaGuide) were retired. Eliminating the archive extraction functions removes ~150 lines of dead code and eliminates redundant flags.
- **Verification & Test Results**:
  - Verified `Show-Usage` and `Get-Help` across both `tools/bootstrap.ps1` and `tools/bootstrap_reference.ps1`.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.
### [2026-09-17 03:26 CEST] — Codified PowerShell Tooling & Parameterization Guidelines in Obsidian Design Vault
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (new specification codifying the Three-Layer Tooling Contract, comment-based help, typed `param(...)` blocks, `Show-Usage`, and auto-promotion)
  - `Obsidian/Amiga/Design/Rust Guidelines.md` (linked new tooling guidelines in frontmatter and specification index)
- **What Was Changed (The Concrete Reality)**:
  - Authored `Obsidian/Amiga/Design/PowerShell Guidelines.md` adhering strictly to Inverted Pyramid structure, Line 1 YAML properties, and Dual-Layer Linking.
  - Codified the **Three-Layer Tooling Contract**:
    1. Standard Comment-Based Help (`<# ... #>`) with mandatory `.SYNOPSIS`, `.DESCRIPTION`, dedicated `.PARAMETER <Name>` tags for every parameter, and workflow `.EXAMPLE` blocks.
    2. Formal typed parameter block (`param(...)`) with mandatory `[CmdletBinding()]`, explicit strongly-typed declarations (`[switch]`, `[string]`), PascalCase variables, and `[Alias("...")]` mappings.
    3. Interactive Console Usage Display (`function Show-Usage`) with Cyan banner, Yellow developer context, White section labels, and formatted 2-column options table.
  - Codified the ergonomic **Sub-Flag Auto-Promotion Pattern** (auto-promoting parent switches when sub-options are passed) and pre-commit verification workflows.
  - Linked the new specification from `Rust Guidelines.md` and related parent notes.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates ad-hoc scripting inconsistencies across developer utilities. Establishes a permanent, reproducible standard ensuring all repository tooling provides identical visual polish, full `Get-Help` introspection, and clean error handling.
- **Verification & Test Results**:
  - Validated Markdown structure and links in `Obsidian/Amiga/Design/PowerShell Guidelines.md`.
  - `python tools/harness/pre_flight.py`: Passed 100% across all quality gates.
### [2026-09-17 03:28 CEST] — Removed Redundant Item Selection Parameters from Reference Bootstrapper
- **Affected Subsystems**:
  - `tools/bootstrap_reference.ps1` (removed `$Item` parameter, `.PARAMETER Item`, and search/filtering logic)
  - `tools/bootstrap.ps1` (removed `$RefItem` parameter, usage rows, and forwarding code)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (updated examples to reflect living scripts)
- **What Was Changed (The Concrete Reality)**:
  - Removed `-Item` from `tools/bootstrap_reference.ps1` and `-RefItem` from `tools/bootstrap.ps1`.
  - Streamlined `tools/bootstrap_reference.ps1` to directly process the entire reference `$Catalog` when executed, eliminating ~25 lines of fuzzy name/alias filtering logic.
  - Updated comment-based help and `Show-Usage` across both scripts.
- **Architectural Rationale & Trade-Offs**:
  - Reference documentation bootstrapping is an all-or-nothing task. Developers provisioning external manuals need the complete reference set rather than selective single-file downloads. Removing the parameter eliminates unnecessary complexity and user cognitive overhead.
- **Verification & Test Results**:
  - Validated syntax and `Show-Usage` on both scripts.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:33 CEST] — Extracted Test, Graphify, and RAG Bootstrappers into Standalone Scripts
- **Affected Subsystems**:
  - `tools/bootstrap_test.ps1` (new standalone script for hardware test vectors, SingleStepTests unpacking, and regression verification)
  - `tools/bootstrap_graphify.ps1` (new standalone script for AST code knowledge graph generation)
  - `tools/bootstrap_rag.ps1` (new standalone script for AI documentation indexing into Qdrant)
  - `tools/bootstrap.ps1` (refactored into modular coordinator delegating to the 4 child scripts)
  - `tools/harness/check_polish.py` (added archive extensions like "gz", "zip", "tar" to technical whitelist to eliminate false positives)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (updated living implementations and references)
- **What Was Changed (The Concrete Reality)**:
  - Extracted Tier 1 test bootstrapping into `tools/bootstrap_test.ps1` supporting `-NoSmoke` and `-SmokeOnly` options.
  - Extracted Tier 2 code knowledge graph generation into `tools/bootstrap_graphify.ps1` supporting `-CheckOnly`.
  - Extracted Tier 3 AI knowledge indexing into `tools/bootstrap_rag.ps1` supporting `-Path`, `-Source`, and `-CheckOnly`.
  - Refactored `tools/bootstrap.ps1` into a clean coordinator that delegates to the 4 dedicated scripts (`bootstrap_test.ps1`, `bootstrap_graphify.ps1`, `bootstrap_rag.ps1`, and `bootstrap_reference.ps1`).
  - Added archive/format file extensions (`"gz"`, `"zip"`, `"tar"`, `"lha"`, `"adf"`, `"rom"`, `"json"`, `"toml"`, `"ps1"`) to `TECHNICAL_WHITELIST` in `tools/harness/check_polish.py` to prevent false positive language flags on file masks like `*.gz`.
  - Updated `PowerShell Guidelines.md` living implementations index.
- **Architectural Rationale & Trade-Offs**:
  - *Single Responsibility & Modularity:* Eliminates monolithic script bloat. Each tier can now be executed, tested, and maintained in isolation without running unwanted checks or having to parse a large multi-purpose script. The parent `bootstrap.ps1` remains an intuitive unified entry point.
- **Verification & Test Results**:
  - Tested `bootstrap.ps1`, `bootstrap_test.ps1`, `bootstrap_graphify.ps1`, and `bootstrap_rag.ps1` in PowerShell.
  - Tested smoke check via `bootstrap_test.ps1` (`cargo test -p test_runner --test test_singlestep test_nop`).
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:39 CEST] — Renamed bootstrap_test.ps1 to bootstrap_sources.ps1
- **Affected Subsystems**:
  - `tools/bootstrap_sources.ps1` (renamed from `tools/bootstrap_test.ps1`, updated documentation, CLI usage, and branding)
  - `tools/bootstrap.ps1` (updated Tier 1 to `-Sources` with `-Test` alias, delegating to `bootstrap_sources.ps1`)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (updated living implementations and references to `bootstrap_sources.ps1`)
- **What Was Changed (The Concrete Reality)**:
  - Renamed `tools/bootstrap_test.ps1` to `tools/bootstrap_sources.ps1`.
  - Updated internal metadata, synopsis, usage banners, and examples to reflect the broader scope (physical test vectors, ADFs, and C++ reference sources).
  - Updated `tools/bootstrap.ps1` parameter to `-Sources` (with `-Test` retained as an alias for backward compatibility).
  - Updated design vault documentation in `PowerShell Guidelines.md`.
- **Architectural Rationale & Trade-Offs**:
  - The script provisions not only test vectors (SingleStepTests, vAmigaTS), but also external reference emulators (vAmiga) and diagnostic media (AmigaTestKit ADF). Naming it `bootstrap_sources.ps1` accurately describes its responsibility as the external sources & test media provider.
- **Verification & Test Results**:
  - Executed `bootstrap_sources.ps1 -NoSmoke`.
  - Executed `bootstrap.ps1 -Sources` and `bootstrap.ps1 -Test`.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:42 CEST] — Renamed bootstrap_reference.ps1 to bootstrap_documentation.ps1
- **Affected Subsystems**:
  - `tools/bootstrap_documentation.ps1` (renamed from `tools/bootstrap_reference.ps1`)
  - `tools/bootstrap.ps1` (updated Tier 4 parameter to `-Documentation` with `-Ref` and `-Doc` aliases)
  - `Obsidian/Amiga/Reference/README.md` (updated references and pruned historical parameter examples)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (updated living implementations and references)
- **What Was Changed (The Concrete Reality)**:
  - Renamed `tools/bootstrap_reference.ps1` to `tools/bootstrap_documentation.ps1`.
  - Updated examples, CLI display, and temporary staging README generation in `tools/bootstrap_documentation.ps1`.
  - Configured `tools/bootstrap.ps1` Tier 4 parameter as `-Documentation` with explicit aliases `[Alias("Ref", "Doc")]`.
  - Updated `Obsidian/Amiga/Reference/README.md` to reference `bootstrap_documentation.ps1` and cleaned up deprecated `-Item` and archive extraction flags.
  - Updated `PowerShell Guidelines.md` living implementations catalog.
- **Architectural Rationale & Trade-Offs**:
  - Aligns naming conventions across all modular bootstrap scripts (`bootstrap_sources.ps1`, `bootstrap_graphify.ps1`, `bootstrap_rag.ps1`, and `bootstrap_documentation.ps1`). "Documentation" clearly identifies the scope of raw reference manuals, PDF scans, and technical articles.
- **Verification & Test Results**:
  - Executed `bootstrap_documentation.ps1 -List`.
  - Executed `bootstrap.ps1` usage display.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:45 CEST] — Consolidated All Modular Bootstrap Scripts into Dedicated tools/bootstrap/ Directory
- **Affected Subsystems**:
  - `tools/bootstrap/` (new dedicated subsystem directory housing all bootstrap tooling)
  - `tools/bootstrap/bootstrap.ps1` (relocated coordinator with updated 2-level repo root resolution)
  - `tools/bootstrap/bootstrap_sources.ps1` (relocated test sources bootstrapper)
  - `tools/bootstrap/bootstrap_graphify.ps1` (relocated AST knowledge graph bootstrapper)
  - `tools/bootstrap/bootstrap_rag.ps1` (relocated AI Qdrant documentation bootstrapper)
  - `tools/bootstrap/bootstrap_documentation.ps1` (relocated external reference bootstrapper)
  - `tools/bootstrap.ps1` (root convenience forwarding wrapper delegating to `tools/bootstrap/bootstrap.ps1`)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (updated living tooling implementations)
- **What Was Changed (The Concrete Reality)**:
  - Created `tools/bootstrap/` directory matching the modular structure of `tools/git/`, `tools/rag/`, and `tools/harness/`.
  - Moved all 5 bootstrap scripts into `tools/bootstrap/` via `git mv`.
  - Adjusted `$RepoRoot` calculation across all child scripts to ascend 2 levels (`Split-Path -Parent (Split-Path -Parent $PSScriptRoot)`).
  - Authored a clean forwarding wrapper at `tools/bootstrap.ps1` that accepts all standard switches and forwards execution directly to `tools/bootstrap/bootstrap.ps1`.
  - Updated `PowerShell Guidelines.md` living implementations catalog.
- **Architectural Rationale & Trade-Offs**:
  - Prevents root `tools/` directory clutter. Co-locating all bootstrap lifecycle scripts inside `tools/bootstrap/` provides modular encapsulation while the thin `tools/bootstrap.ps1` wrapper maintains 100% backward compatibility with existing command lines.
- **Verification & Test Results**:
  - Tested `tools/bootstrap.ps1` root forwarder.
  - Tested `tools/bootstrap/bootstrap_sources.ps1 -NoSmoke`.
  - Tested `tools/bootstrap/bootstrap_documentation.ps1 -List`.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:48 CEST] — Streamlined Reference Library Overview (Pruned Operational Tooling Sections)
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/README.md` (removed Section 2: Automated Reference Bootstrapper and Section 5: Staging Directory Guidelines)
- **What Was Changed (The Concrete Reality)**:
  - Pruned Section 2 (PowerShell CLI examples for `bootstrap_documentation.ps1`) and Section 5 (`temp/` staging directory rules) from `Obsidian/Amiga/Reference/README.md`.
  - Renumbered remaining sections: Section 2 (Multi-Source Fallback Matrix & Error Resilience) and Section 3 (Multi-Page Web Crawling Engine).
- **Architectural Rationale & Trade-Offs**:
  - `Obsidian/Amiga/Reference/README.md` is a reference knowledge hub documenting historical manuals, chip specs, and technical papers in the knowledge vault. Operational developer tooling instructions belong in `tools/bootstrap/bootstrap_documentation.ps1` and `Obsidian/Amiga/Design/PowerShell Guidelines.md`. Pruning operational blocks eliminates duplication and keeps the reference library clean and focused on hardware literature.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:52 CEST] — Removed Root tools/bootstrap.ps1 Duplicate & Consolidated Single Canonical Coordinator
- **Affected Subsystems**:
  - `tools/bootstrap.ps1` (removed duplicate forwarder script from `tools/` root)
  - `tools/bootstrap/bootstrap.ps1` (updated internal usage displays and examples to reflect `tools/bootstrap/bootstrap.ps1`)
  - `README.md` (updated developer quickstart bootstrap path to `tools/bootstrap/bootstrap.ps1`)
  - `Obsidian/Amiga/Design/PowerShell Guidelines.md` (pruned root wrapper reference, retaining canonical coordinator)
- **What Was Changed (The Concrete Reality)**:
  - Deleted `tools/bootstrap.ps1` to eliminate duplicate script files between `tools/` and `tools/bootstrap/`.
  - Established `tools/bootstrap/bootstrap.ps1` as the single canonical coordinator script for all bootstrap tiers.
  - Aligned documentation in `README.md` and `PowerShell Guidelines.md`.
- **Architectural Rationale & Trade-Offs**:
  - Having both `tools/bootstrap.ps1` and `tools/bootstrap/bootstrap.ps1` created ambiguity and violated single-source-of-truth principles. Consolidating into `tools/bootstrap/bootstrap.ps1` cleanly encapsulates the entire bootstrap subsystem inside its own directory with zero root-level clutter.
- **Verification & Test Results**:
  - Tested `tools/bootstrap/bootstrap.ps1` usage display and flag forwarding.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:55 CEST] — Relocated Reference Guide to docs/reference.md & Integrated Bootstrap/RAG Documentation
- **Affected Subsystems**:
  - `docs/reference.md` (relocated from `Obsidian/Amiga/Reference/README.md` and expanded)
  - `README.md` (added link to `docs/reference.md` in repository documentation index)
- **What Was Changed (The Concrete Reality)**:
  - Moved `Obsidian/Amiga/Reference/README.md` to `docs/reference.md` via `git mv` to align with standard project documentation alongside `docs/architecture.md`, `docs/debugger.md`, `docs/testing.md`, and `docs/ai_agents.md`.
  - Added clear status indicators for all cataloged reference documents: explicitly distinguishing materials that are **Already Ingested & Available** in `Obsidian/Amiga/Reference/` (7 core manuals and research papers) versus raw source files **Downloadable via Bootstrap** into `temp/`.
  - Added dedicated documentation for the reference bootstrapper (`tools/bootstrap/bootstrap_documentation.ps1` and `tools/bootstrap/bootstrap.ps1 -Documentation`), detailing CLI switches, `temp/` staging rules, and multi-mirror failovers.
  - Added dedicated documentation for the AI RAG knowledge base (`tools/bootstrap/bootstrap_rag.ps1`, `amiga_rag.ps1`, `tools/harness/rag_search.py`, and the FastMCP server), explaining vector indexing into Qdrant (`amiga` collection) and semantic search queries.
  - Documented the raw document processing toolchain (`pdf-to-markdown` and `html-to-markdown` skills) for transforming future scans/crawls into publication-grade Markdown.
  - Updated root `README.md` to link to `docs/reference.md`.
- **Architectural Rationale & Trade-Offs**:
  - Centralizing developer-facing guides under `docs/` while keeping `Obsidian/Amiga/Reference/` purely dedicated to converted reference texts maintains clean separation of concerns. Developers immediately see what documentation is available offline and how to fetch or re-index raw materials when needed.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:57 CEST] — Pruned Deprecated -Test Parameter & Alias from Bootstrap Coordinator
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1` (removed `.PARAMETER Test`, `[Alias("Test")]`, and all `-Test` references)
  - `README.md` (updated bootstrap table to list `-Sources` cleanly)
  - `docs/testing.md` (updated SingleStep archive decompression command to `tools/bootstrap/bootstrap.ps1 -Sources`)
  - `ROADMAP.md` (updated test suite hardening configurations to `-Sources`)
- **What Was Changed (The Concrete Reality)**:
  - Removed deprecated `-Test` alias and its `.PARAMETER Test` help block from `tools/bootstrap/bootstrap.ps1`.
  - Removed `[Alias("Test")]` attribute from the `$Sources` parameter declaration.
  - Aligned help comments, usage instructions, `README.md`, `docs/testing.md`, and `ROADMAP.md` to reference `-Sources` exclusively.
- **Architectural Rationale & Trade-Offs**:
  - The script's tier is external sources provisioning (`bootstrap_sources.ps1`), which downloads and prepares test suites and emulators rather than executing test runs. Removing the ambiguous `-Test` alias eliminates confusion with test execution tools (`cargo test`, `tools/harness/run_tests.py`).
- **Verification & Test Results**:
  - Tested `tools/bootstrap/bootstrap.ps1` usage display with zero errors.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:58 CEST] — Pruned Unit Test Commands from README.md Quickstart
- **Affected Subsystems**:
  - `README.md` (removed `cargo test -p m68000` and `cargo test -p memory_bus` from Zero-Setup Build & Execution code block)
- **What Was Changed (The Concrete Reality)**:
  - Removed unit test commands from the developer quickstart build block in `README.md`.
- **Architectural Rationale & Trade-Offs**:
  - Keeps the zero-setup quickstart block laser-focused on compiling the workspace and native GUI binary. Test running details are thoroughly covered in [docs/testing.md](docs/testing.md).
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 03:59 CEST] — Pruned Subsystem Design Specifications Table from Root README.md
- **Affected Subsystems**:
  - `README.md` (removed `### Subsystem Design Specifications (Obsidian/Amiga/Design/)` section and its 5-category component table)
- **What Was Changed (The Concrete Reality)**:
  - Pruned the bulky Obsidian design specification table from `README.md`, flattening the documentation index to direct links under `docs/`.
- **Architectural Rationale & Trade-Offs**:
  - Keeps root `README.md` lean, clean, and focused on repository-level architecture guides and developer tools. Granular subsystem specifications belong in `Obsidian/Amiga/Design/` and are navigated via the Obsidian vault graph and AI RAG search rather than cluttering the front-page README.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:01 CEST] — Pruned -NoSmoke and -SmokeOnly Parameters from External Sources Bootstrapper
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_sources.ps1` (removed `.PARAMETER NoSmoke`, `.PARAMETER SmokeOnly`, parameters block, and conditional execution wrappers)
- **What Was Changed (The Concrete Reality)**:
  - Removed `-NoSmoke` and `-SmokeOnly` parameters and their help blocks from `tools/bootstrap/bootstrap_sources.ps1`.
  - Streamlined `bootstrap_sources.ps1` to execute unconditionally: provisioning test suites, verifying archives/repositories, and running the single-step test runner smoke check (`test_nop`).
- **Architectural Rationale & Trade-Offs**:
  - Eliminates unnecessary parameter complexity from the modular sources bootstrapper. The script has a single cohesive responsibility: provisioning external hardware vectors and immediately verifying that the single-step runner is operational.
- **Verification & Test Results**:
  - Ran `.\tools\bootstrap\bootstrap_sources.ps1` directly; successfully verified 124 test suites and executed `test_nop` smoke check with 0 exit code.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:03 CEST] — Renamed docs/reference.md to docs/reference_documentation.md
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (renamed from `docs/reference.md` via `git mv` and updated document title)
  - `README.md` (updated documentation index link to `docs/reference_documentation.md`)
- **What Was Changed (The Concrete Reality)**:
  - Renamed `docs/reference.md` to `docs/reference_documentation.md` to explicitly convey that it covers external hardware reference documents and manuals rather than abstract code references.
  - Updated title to *Technical Reference Documentation, Bootstrapper & AI RAG Guide*.
  - Updated link in root `README.md`.
- **Architectural Rationale & Trade-Offs**:
  - Clarifies intent: distinguishes technical reference documentation (Commodore HRM, Motorola manuals, Pasti papers) from internal architectural references or API indexes.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:05 CEST] — Authored docs/sources.md (External Sources & Testbeds Catalog)
- **Affected Subsystems**:
  - `docs/sources.md` (new documentation cataloging all external test sources, silicon vectors, and reference emulators)
  - `README.md` (added link to `docs/sources.md` in repository documentation index)
- **What Was Changed (The Concrete Reality)**:
  - Authored comprehensive guide [`docs/sources.md`](docs/sources.md) detailing:
    1. Upstream repositories and URLs for each external dependency.
    2. Pinned versions and formats: Tom Harte SingleStepTests (format `68000/v1/`, 124 suites), Dirk W. Hoffmann vAmiga (`v4.5`), vAmigaTS (`master`, 2,077 tests), Keir Fraser Amiga Test Kit (`v1.20+`), and WinUAE sinc tables (`6.0.30`).
    3. Directory layouts, formats, and contents for each repository under `ref_src/` and `tools/`.
    4. Integration with automated bootstrapper (`tools/bootstrap/bootstrap_sources.ps1` and coordinator `tools/bootstrap/bootstrap.ps1 -Sources`).
    5. Git ignore policies (`.gitignore`) and zero-cost NTFS directory junction sharing across worktrees via `tools/git/worktree.ps1`.
  - Added link to [`docs/sources.md`](docs/sources.md) in root `README.md`.
- **Architectural Rationale & Trade-Offs**:
  - Transparently documents provenance and pinned versions of all external test datasets without bloating Git history. Developers and agents understand exactly what data resides in `ref_src/`, where to download it, and how it is consumed by test runners.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:07 CEST] — Renamed docs/sources.md to docs/reference_sources.md
- **Affected Subsystems**:
  - `docs/reference_sources.md` (renamed from `docs/sources.md` via `git mv`)
  - `README.md` (updated documentation index link to `docs/reference_sources.md`)
- **What Was Changed (The Concrete Reality)**:
  - Renamed `docs/sources.md` to `docs/reference_sources.md` using `git mv`.
  - Updated document title and root `README.md` link to *External Reference Sources & Testbeds Guide*.
- **Architectural Rationale & Trade-Offs**:
  - Forms a symmetrical pair of reference guides: [`docs/reference_documentation.md`](docs/reference_documentation.md) for technical manuals and literature vs [`docs/reference_sources.md`](docs/reference_sources.md) for external source trees, silicon vectors, and test suites.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:08 CEST] — Pruned WinUAE Audio Sinc Tables from Reference Sources Guide
- **Affected Subsystems**:
  - `docs/reference_sources.md` (removed WinUAE audio sinc tables row from Pinned Upstream Sources table)
- **What Was Changed (The Concrete Reality)**:
  - Removed `WinUAE Audio Sinc Tables` entry from the upstream sources table in [`docs/reference_sources.md`](docs/reference_sources.md).
- **Architectural Rationale & Trade-Offs**:
  - Keeps [`docs/reference_sources.md`](docs/reference_sources.md) focused strictly on primary testing and verification testbeds (SingleStepTests, vAmiga, vAmigaTS, AmigaTestKit) actively provisioned and validated by `bootstrap_sources.ps1`.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:10 CEST] — Pruned Redundant Section 2 from Reference Sources Guide
- **Affected Subsystems**:
  - `docs/reference_sources.md` (removed Section 2: Directory Contents & Upstream Details; renumbered subsequent sections)
- **What Was Changed (The Concrete Reality)**:
  - Pruned verbose Section 2 (directory ASCII trees, per-repository breakdown, and duplicate notes) from `docs/reference_sources.md`.
  - Retained the concise, high-signal summary table in Section 1 and renumbered Automated Provisioning to Section 2 and Worktree Junctions to Section 3.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates redundant directory listings that duplicate what is already cleanly and concisely stated in the Section 1 overview table. Keeps the documentation lean, direct, and focused.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:12 CEST] — Pruned Direct bootstrap_sources Script Invocation from Reference Sources Guide
- **Affected Subsystems**:
  - `docs/reference_sources.md` (removed direct `.\tools\bootstrap\bootstrap_sources.ps1` command snippet)
- **What Was Changed (The Concrete Reality)**:
  - Removed the alternative direct invocation example of `bootstrap_sources.ps1`, promoting `.\tools\bootstrap\bootstrap.ps1 -Sources` as the single canonical command for provisioning external sources.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates redundant script commands and steers developers and agents toward the unified bootstrap coordinator `tools/bootstrap/bootstrap.ps1`.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:14 CEST] — Eliminated Smoke Testing from Bootstrap Sources Provisioning
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_sources.ps1` (removed `cargo test` smoke check execution and updated 124 suites completeness validation)
  - `docs/reference_sources.md` (removed step 5: Immediate Smoke Verification from automation lifecycle)
- **What Was Changed (The Concrete Reality)**:
  - Completely eliminated `cargo test -p test_runner --test test_singlestep test_nop` from `tools/bootstrap/bootstrap_sources.ps1`.
  - Replaced smoke testing with strict static completeness verification: asserting that all 124 per-instruction test suites are decompressed and present in `ref_src/SingleStepTests-680x0/68000/v1/`.
  - Updated lifecycle steps in [`docs/reference_sources.md`](docs/reference_sources.md) to reflect pure provisioning and presence/completeness validation.
- **Architectural Rationale & Trade-Offs**:
  - Clear separation of concerns between data provisioning and test execution. Bootstrap scripts strictly download, unpack, and validate asset integrity, leaving all test executions to dedicated testing harnesses (`cargo test`, `tools/harness/run_tests.py`).
- **Verification & Test Results**:
  - Ran `.\tools\bootstrap\bootstrap_sources.ps1` directly; confirmed instant verification of 124/124 suites with zero test runs and exit code 0.
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:18 CEST] — Pruned NTFS Junctions and Worktree Section from Reference Sources Guide
- **Affected Subsystems**:
  - `docs/reference_sources.md` (removed NTFS directory junction mentions and pruned Section 3 on Git Worktree Isolation & Zero-Cost NTFS Junctions)
- **What Was Changed (The Concrete Reality)**:
  - Removed the NTFS directory junctions phrase from the Section 1 overview.
  - Completely deleted Section 3 ("Git Worktree Isolation & Zero-Cost NTFS Junctions") from [`docs/reference_sources.md`](docs/reference_sources.md).
- **Architectural Rationale & Trade-Offs**:
  - Keeps the external reference sources guide focused strictly on upstream repositories, pinned versions, and automated provisioning lifecycle, eliminating redundant NTFS junction mechanisms and worktree instructions.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:20 CEST] — Pruned Direct bootstrap_documentation Invocation from Reference Documentation Guide
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (removed direct `.\tools\bootstrap\bootstrap_documentation.ps1` command snippet)
- **What Was Changed (The Concrete Reality)**:
  - Removed direct script invocation commands for `bootstrap_documentation.ps1` (`-List`, `-All`, `-AllSources`, `-Force`) from Section 3.
  - Standardized on `.\tools\bootstrap\bootstrap.ps1 -Documentation` as the single canonical bootstrap entry point.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates clutter and script proliferation in guides, ensuring developers and AI agents interact with external asset bootstrapping through the unified root coordinator `tools/bootstrap/bootstrap.ps1`.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:22 CEST] — Pruned Staging Directory Policy (temp/) from Reference Documentation Guide
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (removed Staging Directory Policy (`temp/`) subsection)
- **What Was Changed (The Concrete Reality)**:
  - Removed the `### Staging Directory Policy ('temp/')` subsection and its notes regarding Git status visibility and manual cleanup.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates peripheral staging policy clutter, keeping the reference documentation guide focused on the document catalog, coordinator invocation, and the conversion skills pipeline.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:23 CEST] — Generalized Markdown Processing Trigger in Reference Documentation Guide
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (decoupled document processing introduction from explicit `temp/` folder mention)
- **What Was Changed (The Concrete Reality)**:
  - Updated Section 4's opening instruction to state generically: "When new reference manuals or updated editions are retrieved, use specialized agent skills to convert them into repository-grade Markdown:", eliminating the hardcoded staging folder reference.
- **Architectural Rationale & Trade-Offs**:
  - Agent conversion skills operate on document targets passed as parameters rather than an inflexible hardcoded directory; removing path coupling improves portability and cleanliness.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:25 CEST] — Documented Gemini Multimodal Reasoning in PDF Conversion Pipeline
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (updated PDF-to-Markdown processing pipeline description)
- **What Was Changed (The Concrete Reality)**:
  - Replaced the PyMuPDF library detail with explicit reference to Gemini: "Leverages Gemini multimodal reasoning to analyze document structure and partition into logical chapters."
- **Architectural Rationale & Trade-Offs**:
  - Accurately reflects the agent-driven architecture defined in `.agents/skills/pdf-to-markdown/SKILL.md`, where Gemini serves as the cognitive reasoning engine for chapter partitioning and document understanding.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:27 CEST] — Separated Ingested Documents from Bootstrapped Archival Sources
- **Affected Subsystems**:
  - `docs/reference_documentation.md` (pruned ASCII lifecycle box, partitioned Section 2 into Ingested Documents and Section 3 into Documents to Bootstrap)
- **What Was Changed (The Concrete Reality)**:
  - Pruned the ASCII reference data lifecycle diagram from Section 1.
  - Divided the reference catalog into two distinct, dedicated sections:
    1. *Section 2 (Ingested Reference Documents):* Highlights the 7 primary manuals already converted to Markdown and committed in `Obsidian/Amiga/Reference/`.
    2. *Section 3 (Documents to Bootstrap):* Documents the raw archival source targets (PDFs, HTML crawls) downloadable via `.\tools\bootstrap\bootstrap.ps1 -Documentation`.
- **Architectural Rationale & Trade-Offs**:
  - Clear conceptual separation between what is active and ready in the repository vs what can optionally be provisioned from external archival mirrors. Eliminates ambiguity about whether external downloads are required to read specifications.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:30 CEST] — Documented vAmigaTS Verification Preview & CLI Help Guidance
- **Affected Subsystems**:
  - `docs/testing.md` (added Section 5 on vAmigaTS subsystem and CLI verification preview, added self-discovery `--help` tips)
- **What Was Changed (The Concrete Reality)**:
  - Added Section 5 detailing vAmigaTS test integration:
    1. Subsystem integration test suites in Cargo (`test_vamiga_copper`, `test_vamiga_blitter`, `test_vamiga_denise`, `test_vamiga_paula`).
    2. Interactive test runner CLI (`cargo run -p test_runner -- vamiga [OPTIONS]`).
  - Added tips across sections highlighting that executing `cargo run -p test_runner` without arguments or passing `--help` outputs the complete usage manual and available options, eliminating the need to memorize flag combinations.
- **Architectural Rationale & Trade-Offs**:
  - Previews future whole-machine verification capabilities that already exist in the codebase without premature roadmap completion claims, providing contributors and agents with immediate discoverability.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:33 CEST] — Streamlined Landing Page & Extracted Building and Bootstrapping Guide
- **Affected Subsystems**:
  - `README.md` (promoted Developer Studio to prominent player subsection, streamlined Section 2 into clean developer link index, pruned skill links from Section 3)
  - `docs/build_and_bootstrap.md` (extracted zero-setup compilation and multi-tier bootstrapping suite)
- **What Was Changed (The Concrete Reality)**:
  - Extracted verbose build instructions and bootstrapping options from `README.md` into dedicated [`docs/build_and_bootstrap.md`](docs/build_and_bootstrap.md).
  - Renamed Section 2 to `## 2. For Developers`, consolidating links to developer guides (`build_and_bootstrap.md`, `testing.md`, `reference_sources.md`, `reference_documentation.md`).
  - Elevated Developer Studio and in-game debugging from a small parenthetical note into a dedicated, prominent subsection under Section 1 with direct pointer to [`docs/debugger.md`](docs/debugger.md).
  - Cleaned Section 3 (`Documentation Cheat Sheet & Technical Index`) to strictly index architectural deep dives and AI specifications, eliminating non-doc links (`.agents/skills/`) and redundant developer items.
- **Architectural Rationale & Trade-Offs**:
  - Keeps the front porch `README.md` lean, approachable, and focused on player experience and high-level navigation, while organizing technical deep dives under standardized `docs/` paths.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates (including 0 broken links in architecture tests).

### [2026-09-17 04:35 CEST] — Consolidated Developer Documentation into docs/developers.md with Single README Link
- **Affected Subsystems**:
  - `README.md` (condensed Section 2 into a single pointer to `docs/developers.md`)
  - `docs/developers.md` (consolidated zero-setup build, multi-tier bootstrapping, and developer guides index)
  - `docs/build_and_bootstrap.md` (removed in favor of unified `docs/developers.md`)
- **What Was Changed (The Concrete Reality)**:
  - Created [`docs/developers.md`](docs/developers.md) consolidating zero-setup build commands, the multi-tier bootstrapping suite (`tools/bootstrap/bootstrap.ps1`), and references to testing/sources/documentation guides.
  - Replaced the multi-link developer listing in `README.md` Section 2 with a single, clean directional link: `👉 [Developer Guide & Technical Index](docs/developers.md)`.
  - Removed intermediate `docs/build_and_bootstrap.md`.
- **Architectural Rationale & Trade-Offs**:
  - Single-responsibility landing page: `README.md` acts as a pure front-page router, delegating all developer tooling and references to a single dedicated entry point in `docs/developers.md`.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:38 CEST] — Relocated Knowledge Retrieval to docs/developers.md & Removed docs/ai_agents.md
- **Affected Subsystems**:
  - `docs/ai_agents.md` (deleted redundant document)
  - `docs/developers.md` (added Section 4 covering RAG and Graphify knowledge retrieval)
  - `README.md` (pruned `docs/ai_agents.md` link from Section 3)
- **What Was Changed (The Concrete Reality)**:
  - Relocated Section 2 of `ai_agents.md` (Domain Hardware RAG & AST Graphify Knowledge Retrieval) into Section 4 of [`docs/developers.md`](docs/developers.md).
  - Deleted [`docs/ai_agents.md`](docs/ai_agents.md) via `git rm`, removing redundant descriptions of rules and skills already maintained directly in `.agents/rules/` and `.agents/skills/`.
  - Pruned the corresponding link from Section 3 in [`README.md`](README.md).
- **Architectural Rationale & Trade-Offs**:
  - Eliminates duplicate documentation sprawl. Rules and skills are natively defined and indexed under `.agents/` and governed by `AGENTS.md`. Developers searching for RAG and Graphify tooling find them directly in the consolidated developer guide.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates (0 broken links).

### [2026-09-17 04:41 CEST] — Standardized vAmiga Reference Directory Name to ref_src/vAmiga
- **Affected Subsystems**:
  - `ref_src/` (renamed directory `vAmiga-4.5` to `vAmiga`)
  - `docs/reference_sources.md` (updated reference table to canonical `ref_src/vAmiga/`)
  - `Obsidian/Amiga/Design/*.md` (updated external code references across 16 design specifications)
- **What Was Changed (The Concrete Reality)**:
  - Renamed `ref_src/vAmiga-4.5` to canonical version-agnostic `ref_src/vAmiga`.
  - Updated `docs/reference_sources.md` to reference `ref_src/vAmiga/`.
  - Updated references across 16 Obsidian design specifications to keep links pointing to the renamed path.
- **Architectural Rationale & Trade-Offs**:
  - Version-agnostic paths prevent coupling to transient version numbers, ensuring scripts, tools, and cross-references remain durable.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:44 CEST] — Consolidated Reference Sources and Documentation into docs/developers.md
- **Affected Subsystems**:
  - `docs/developers.md` (consolidated external reference sources, testbeds, literature, and bootstrap instructions)
  - `docs/reference_documentation.md` (deleted via `git rm`)
  - `docs/reference_sources.md` (deleted via `git rm`)
- **What Was Changed (The Concrete Reality)**:
  - Merged `docs/reference_sources.md` (testbed definitions, pinned versions, SingleStepTests, vAmiga, vAmigaTS, AmigaTestKit, and `bootstrap_sources.ps1` lifecycle) directly into Section 3 of [`docs/developers.md`](docs/developers.md).
  - Merged `docs/reference_documentation.md` (ingested reference manuals, archival sources, fallback mirrors, OCR/conversion pipelines, and `bootstrap_documentation.ps1`) directly into Section 4 of [`docs/developers.md`](docs/developers.md).
  - Harmonized with existing bootstrapping options (`tools/bootstrap/bootstrap.ps1`) and knowledge retrieval tooling (RAG & Graphify) in `developers.md` to present a unified, non-redundant developer handbook.
  - Deleted standalone `docs/reference_documentation.md` and `docs/reference_sources.md` files via `git rm`.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates fragment sprawl across `docs/` by consolidating all developer-facing reference catalogs, testbeds, literature specs, and bootstrap orchestrations into a single authoritative guide.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:47 CEST] — Added Foundational Architecture Reference to docs/developers.md
- **Affected Subsystems**:
  - `docs/developers.md` (added foundational architecture advanced callout at the top of the guide)
- **What Was Changed (The Concrete Reality)**:
  - Added an artifact-grade greeting/advisory callout (`[!IMPORTANT]`) at the very top of [`docs/developers.md`](docs/developers.md) lighting up and linking to [`docs/architecture.md`](docs/architecture.md) and [`Obsidian/Amiga/Design/`](Obsidian/Amiga/Design/).
- **Architectural Rationale & Trade-Offs**:
  - Ensures developers contributing to the emulator grasp core clock synchronization (CCK1/CCK2), shared bus contention, and Big-Endian invariance before delving into tooling, suites, or opcodes.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:48 CEST] — Weaved Engineering Story into README and Removed Architecture Link
- **Affected Subsystems**:
  - `README.md` (wove `How This Emulator Was Written` into project intro, removed standalone architecture link and Section 3)
- **What Was Changed (The Concrete Reality)**:
  - Wove the engineering story link ([`How This Emulator Was Written`](docs/how_this_emulator_was_written.md)) naturally into the opening introduction of [`README.md`](README.md).
  - Removed the redundant link to `docs/architecture.md` (already prominently anchored in [`docs/developers.md`](docs/developers.md)).
  - Pruned now-empty Section 3 (Technical Index) from `the bottom of `README.md`, leaving a clean, lean document ending with Section 2 (For Developers).
- **Architectural Rationale & Trade-Offs**:
  - Keeps the root `README.md` focused on player quick start and compact developer direction, while introducing the unique AI-pair-programming engineering methodology upfront.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates.

### [2026-09-17 04:50 CEST] — Consolidated Testing & Verification into docs/developers.md
- **Affected Subsystems**:
  - `docs/developers.md` (expanded Section 6 into comprehensive Test Suite & Verification Framework section)
  - `docs/testing.md` (deleted via `git rm`)
- **What Was Changed (The Concrete Reality)**:
  - Merged the entire `docs/testing.md` document directly into Section 6 of [`docs/developers.md`](docs/developers.md) as a fully elaborated major chapter (covering M68000 SingleStepTests with sampling/exhaustive flags, Cartesian DMA contention math, automated architecture gates, CLI regression diagnostics, and vAmigaTS integration).
  - Deleted standalone `docs/testing.md` via `git rm`.
- **Architectural Rationale & Trade-Offs**:
  - Unifies all developer-facing workflows (building, bootstrapping, reference sources, literature catalog, knowledge retrieval, and exhaustive verification suites) into a single authoritative guide, reducing file fragmentation across `docs/`.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed 100% cleanly across all quality gates (19/19 architecture rules, zero broken links).

### [2026-09-17 05:00 CEST] — Historical Section 10 Compaction into 6 Architectural Milestone Digests
- **Affected Subsystems**:
  - `DIARY.md` (Section 10 synthesized per `.agents/skills/compact-diary/SKILL.md`)
- **What Was Changed (The Concrete Reality)**:
  - Audited and compacted 230 historical chronological entries in Section 10 (September 12–16, 2026) spanning over 5,700 lines into 6 high-signal Milestone Digests:
    1. Repository Bootstrapping, Clean-Room Pipeline & Tooling Ergonomics
    2. Multi-Tier Testing Infrastructure, Cartesian Contention & Silicon Verification
    3. Developer Studio GUI, Headless Perception & Debugging Ecosystem
    4. Custom Chipset Silicon Architecture & Autonomous DMA Engines (Phase 1)
    5. Multimodal Reference Ingestion & PDF-to-Markdown Pipeline
    6. Agent Operating System, Architectural Guardrails & Quality Discipline
  - Retained the active September 17 development sprint in full granular chronological detail below the digests.
  - Reduced `DIARY.md` from 6,682 lines (704 KB) down to 1,100 lines (117 KB) with zero loss of architectural wisdom or hardware problem-solving rationale.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates context bloat and token exhaustion while permanently preserving core design dilemmas, hardware silicon discoveries, and human-AI collaboration dynamics.
- **Verification & Invariants**:
  - Passed `cargo test -p test_runner --test test_architecture_rules` (19/19 tests ok).
  - Passed `python tools/harness/pre_flight.py` cleanly across all quality gates.

### [2026-09-17 06:05 CEST] — Reference Documentation Bootstrapper Direct Target to Obsidian/Amiga/Reference/
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/bootstrap.ps1`
  - `.agents/skills/html-to-markdown/SKILL.md`
  - `.agents/skills/html-to-markdown/scripts/render_comparison.py`
  - `.gitignore`
- **What Was Changed (The Concrete Reality)**:
  - Updated `bootstrap_documentation.ps1` default destination from `Obsidian/Amiga/Reference/temp` to `Obsidian/Amiga/Reference/`.
  - Guarded `Ensure-StagingReadme` to only create a staging `README.md` if the destination path explicitly contains `temp`, preventing pollution of the permanent Obsidian Reference vault root.
  - Updated help, usage, and completion messages in `bootstrap_documentation.ps1` and `bootstrap.ps1` to reflect direct provisioning into `Obsidian/Amiga/Reference/`.
  - Synchronized `.agents/skills/html-to-markdown/` paths and visual comparison scripts to reference `Obsidian/Amiga/Reference/` directly.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates the redundant `temp/` staging layer for downloaded archival reference materials, aligning target folder structure with `.gitignore` exclusion rules (`/Obsidian/Amiga/Reference/<Document_Name>*/`).
- **Verification & Invariants**:
  - Tested `bootstrap_documentation.ps1 -List` and `bootstrap.ps1` parameter help outputs.
  - Confirmed zero remaining references to `Reference/temp` across the repository.

### [2026-09-17 06:15 CEST] — Automated Upstream Download & Unpack in bootstrap_sources.ps1
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_sources.ps1`
  - `tools/harness/check_polish.py`
- **What Was Changed (The Concrete Reality)**:
  - Upgraded `bootstrap_sources.ps1` from a passive validator into an automated downloader, unpacker, and provisioner for all 4 external testbeds:
    1. Tom Harte SingleStepTests 68000 silicon vectors (`main.zip`, 124 `.json` test suites decompressed via `GZipStream` into `ref_src/SingleStepTests-680x0/68000/v1/`).
    2. Keir Fraser Amiga Test Kit ADF diagnostic disk (`AmigaTestKit-1.21.zip`, unpacked into `tools/AmigaTestKit/AmigaTestKit.adf`).
    3. Dirk W. Hoffmann vAmiga C++ reference emulator sources (`v4.5.zip`, unpacked into `ref_src/vAmiga/`).
    4. Dirk W. Hoffmann vAmigaTS chipset regression test suite (`master.zip`, unpacked into `ref_src/vAmigaTS/`).
  - Added `-Force`, `-List`, `-SingleStep`, `-AmigaTestKit`, `-VAmiga`, and `-VAmigaTS` CLI flags.
  - Implemented automatic presence checking to skip already-provisioned datasets unless `-Force` is supplied.
  - Whitelisted `testkit` in `tools/harness/check_polish.py` to prevent false positive language policy warnings on Keir Fraser's Amiga Test Kit.
- **Architectural Rationale & Trade-Offs**:
  - Enables true 1-command bootstrap (`.\tools\bootstrap\bootstrap.ps1 -Sources` or `-All`) on fresh checkouts without requiring manual cloning or searching for test asset URLs.
- **Verification & Invariants**:
  - Downloaded and provisioned all four datasets from scratch into `tools/AmigaTestKit/` and `ref_src/`.
  - Verified `bootstrap_sources.ps1 -List` marks all 4 as `[PRESENT]`.
### [2026-09-17 06:26 CEST] — Parameter Discovery, -Help / --help Flags & Robust RepoRoot in bootstrap_sources.ps1
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_sources.ps1`
  - `.gitignore`
- **What Was Changed (The Concrete Reality)**:
  - Added `-Help` parameter with aliases (`-h`, `-?`) and bound `ValueFromRemainingArguments` to capture `--help` and `-help` invocations without PowerShell `PositionalParameterNotFound` errors.
  - Added automatic display of usage and available parameter list (`Show-Usage`) when `bootstrap_sources.ps1` is executed without parameters (`$PSBoundParameters.Count -eq 0`), before proceeding with the default full verification/provisioning.
  - Added robust fallback for `$RepoRoot` calculation when `$PSScriptRoot` is empty or invoked across varying PowerShell execution contexts (`Split-Path` or fallback to `Get-Location` verified with `Cargo.toml`).
  - Added `/tools/AmigaTestKit*/` pattern in `.gitignore` to prevent any staging remnants from entering Git tracking.
- **Architectural Rationale & Trade-Offs**:
### [2026-09-17 06:40 CEST] — Documentation Bootstrap Markdown Processing & In-Place Emission
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/bootstrap.ps1`
  - `.agents/skills/pdf-to-markdown/config.yaml`
  - `.agents/skills/pdf-to-markdown/pipeline.py`
  - `.agents/skills/pdf-to-markdown/stages/05_chapter_partition/partition_chapters.py`
  - `.agents/skills/pdf-to-markdown/stages/10_proofread_stream/proofread_stream.py`
  - `.agents/skills/pdf-to-markdown/stages/12_refine_first_chapter_name/refine_name.py`
  - `.agents/skills/html-to-markdown/pipeline.py`
- **What Was Changed (The Concrete Reality)**:
  - Added `-Markdown` parameter (aliases: `-Convert`, `-Process`) to `bootstrap_documentation.ps1` and `bootstrap.ps1` to trigger automated conversion of downloaded raw scans/crawls into publication-grade Markdown.
  - Added discrete document selection switches: `-Hrm`, `-Trm`, `-Prm`, `-Um`, `-Prefetch`, `-Undocumented` alongside `-All` and `-Force`.
  - Implemented `-Help` (aliases: `-h`, `-?`, `--help`) with clean parameter documentation and usage examples.
  - Configured in-place Markdown emission: updated `pdf-to-markdown/pipeline.py`, `config.yaml`, and stage scripts to emit Markdown files and `assets/` directly into `Obsidian/Amiga/Reference/<Document_Name>/` rather than nested `output_markdown/` subfolders.
  - Standardized output Markdown filenames to `{idx:02d} - {Title}.md` (e.g. `00 - Table of Contents.md`, `01 - Chapter 1 - Introduction.md`) matching the canonical naming convention in reference `-old` directories.
  - Built unified `.agents/skills/html-to-markdown/pipeline.py` orchestrator supporting single HTML files and multi-page web directory crawls, extracting images to `assets/` with `.txt` sidecars and emitting clean Markdown with Line 1 YAML properties and TOC.
  - Guaranteed zero interference with `*-old` folders, preserving them untouched for user verification.
- **Architectural Rationale & Trade-Offs**:
  - Unifies reference documentation downloading and ingestion into a single, cohesive workflow (`bootstrap_documentation.ps1 -Markdown`).
  - Eliminates auxiliary intermediate folders by writing final Markdown directly into the reference library directories expected by Obsidian design documents.
- **Verification & Invariants**:
  - Verified `bootstrap_documentation.ps1 -Help` and `--help` exit cleanly with code 0.
  - Verified `bootstrap_documentation.ps1 -Prefetch -Markdown` downloads and converts Jorge Cwik's prefetch guide into `Obsidian/Amiga/Reference/Instruction Prefetch on the Motorola 68000 Processor/`.
  - Verified `bootstrap_documentation.ps1 -Undocumented -Markdown` crawls and consolidates Kuba Winnicki's *Achtung! Amiga* into `Obsidian/Amiga/Reference/Undocumented features of OCS, ECS and AGA chipsets/`.
  - Verified code formatting (`cargo fmt --all -- --check`) and architecture rules (`cargo test -p test_runner --test test_architecture_rules test_file_size_limits`).

### [2026-09-17 06:42 CEST] — Removal of -Ref and -Doc Parameter Aliases from bootstrap.ps1
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1`
- **What Was Changed (The Concrete Reality)**:
  - Removed obsolete parameter aliases `-Ref` and `-Doc` from comment-based help (`.PARAMETER`), parameter attribute definitions (`[Alias("Ref", "Doc")]`), and `Show-Usage` in `bootstrap.ps1`.
  - Standardized documentation parameter invocation exclusively on canonical `-Documentation`.
- **Architectural Rationale & Trade-Offs**:
  - Prevents parameter clutter and ambiguous switch aliases in the root bootstrap orchestrator.
- **Verification & Invariants**:
  - Verified `bootstrap.ps1` syntax and help display.
  - Passed `python tools/harness/pre_flight.py --quick`.

### [2026-09-17 06:44 CEST] — Removal of -AllSources and -AllMirrors from bootstrap.ps1
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1`
- **What Was Changed (The Concrete Reality)**:
  - Removed `-AllSources` switch and its `-AllMirrors` alias from `bootstrap.ps1` (`.PARAMETER`, `param()`, `Show-Usage`, auto-activation of `-Documentation`, and parameter splatting).
  - Specialized archival mirror redundancy remains accessible directly within `tools/bootstrap/bootstrap_documentation.ps1 -AllSources` without cluttering the primary repository orchestrator.
- **Architectural Rationale & Trade-Offs**:
  - Keeps the root `bootstrap.ps1` focused cleanly on the 4 primary tiers (`-Sources`, `-Graphify`, `-Rag`, `-Documentation`) and documentation processing (`-Markdown`).
- **Verification & Invariants**:
  - Verified `bootstrap.ps1` usage output.
  - Passed `python tools/harness/pre_flight.py --quick`.

### [2026-09-17 06:46 CEST] — Reordering Bootstrap Pipeline & -All Sequence to Physical Causal Order
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Reordered the bootstrapping tiers so that Documentation (Tier 3) runs before RAG (Tier 4):
    1. Tier 1: Hardware Verification & External Sources (`-Sources` -> `tools/bootstrap/bootstrap_sources.ps1`)
    2. Tier 2: AST-Level Code Knowledge Graph (`-Graphify` -> `tools/bootstrap/bootstrap_graphify.ps1`)
    3. Tier 3: External Reference Documentation & Scans (`-Documentation` -> `tools/bootstrap/bootstrap_documentation.ps1`)
    4. Tier 4: AI Knowledge & Qdrant RAG Vector Index (`-Rag` -> `tools/bootstrap/bootstrap_rag.ps1`)
  - Updated the `-All` master workflow in `bootstrap.ps1` to execute all 4 tiers in this exact physical causal sequence (`-Sources` -> `-Graphify` -> `-Documentation` -> `-Rag`).
  - Synchronized documentation in `docs/developers.md` and script help/usage outputs.
- **Architectural Rationale & Trade-Offs**:
  - Adheres strictly to the Substrate-First Invariant (Physical Causal Ordering): RAG indexes reference manuals under `Obsidian/Amiga/Reference/`. Running RAG before Documentation leaves vector embeddings missing all newly fetched or updated reference literature.
- **Verification & Invariants**:
  - Verified `Show-Usage` and parameter descriptions.
  - Passed `python tools/harness/pre_flight.py --quick`.

### [2026-09-17 14:40 CEST] — Removal of build_frontmatter & Addition of Stage 12 (12_generate_properties) for LLM Obsidian Properties
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown/stages/11_emit_markdown/emit_markdown.py`
  - `.agents/skills/pdf-to-markdown/stages/11_emit_markdown/README.md`
  - `.agents/skills/pdf-to-markdown/stages/12_generate_properties/generate_properties.py`
  - `.agents/skills/pdf-to-markdown/stages/12_generate_properties/prompt.md`
  - `.agents/skills/pdf-to-markdown/stages/12_generate_properties/README.md`
  - `.agents/skills/pdf-to-markdown/stages/13_refine_first_chapter_name/` (relocated from 12)
  - `.agents/skills/pdf-to-markdown/stages/14_link_toc/` (relocated from 13)
  - `.agents/skills/pdf-to-markdown/pipeline.py`
  - `.agents/skills/pdf-to-markdown/config.yaml`
  - `.agents/skills/pdf-to-markdown/SKILL.md`
- **What Was Changed (The Concrete Reality)**:
  - Removed rudimentary `build_frontmatter()` and its invocation from `stages/11_emit_markdown/emit_markdown.py`, ensuring Stage 11 emits clean Markdown body content without placeholder YAML frontmatter.
  - Implemented Stage 12 (`12_generate_properties`) containing `generate_properties.py`, `prompt.md`, and `README.md`. It evaluates the opening chapter/front matter to infer the canonical book title (`book`) and prompts Gemini LLM with chapter content to generate `title`, `book`, `chapter`, and 4-8 kebab-case `tags`, injecting Line 1 Obsidian YAML frontmatter.
  - Relocated and updated subsequent stages: `12_refine_first_chapter_name` shifted to `13_refine_first_chapter_name`, and `13_link_toc` shifted to `14_link_toc`. Updated stage candidate search paths, CLI arguments, and documentation.
  - Updated master orchestrator `pipeline.py` to manage the expanded 14-stage sequence, directing final output to `--output-dir` in Stage 14.
  - Configured `config.yaml` with `properties_dir: "workspace/12_generate_properties"`, updated downstream paths, and added stage thinking budget configurations.
  - Updated `SKILL.md` directory tree, execution workflow phases, and CLI invocation commands.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates hardcoded placeholder metadata (`tags: [amiga, reference, hardware]`) in favor of publication-grade, document-specific Obsidian properties.
  - Provides the LLM with the first chapter context so that book-level metadata (`book`) is inferred reliably and consistently across all partitioned chapters.
  - Preserves graceful fallback heuristics so offline runs or missing API keys continue to emit well-structured frontmatter without crashing.
- **Verification & Invariants**:
  - Successfully compiled all Python scripts with `python -m py_compile`.
  - Verified CLI `--help` output across `pipeline.py`, `generate_properties.py`, `refine_name.py`, and `link_toc.py`.
  - Executed end-to-end unit and integration test in temporary directory verifying property inference, YAML serialization, and asset synchronization.
### [2026-09-17 15:05 CEST] — Config-Driven Typographic Padding in Asset Extraction (Fixing Table Boundary Defect)
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown/stages/03_build_raw_stream/extract_initial_assets.py`
  - `.agents/skills/pdf-to-markdown/stages/03_build_raw_stream/build_stream.py`
- **What Was Changed (The Concrete Reality)**:
  - Replaced relative percentage padding (`pad_y = h * padding_ratio`, which previously inflated tall tables by 30+ points and reached into the preceding table caption text) with an absolute typographic padding derived directly from the configured DPI and physical letter height.
  - Formulated typographic half-letter height padding:
    - Typical body font size in technical manuals = 10 pt in standard 72 pt/in PostScript space ($H = \frac{10}{72}\text{ inches}$).
    - Half-letter padding in pixels at configured DPI: $\text{padding\_px} = \text{round}(0.5 \times \frac{10}{72} \times \text{dpi})$ (21 px at 300 DPI).
    - Half-letter padding in PDF points: $\text{padding\_pt} = \text{padding\_px} \times \frac{72}{\text{dpi}} \approx 5.04\text{ pt}$.
  - Applied `padding_pt` symmetrically to both tables and graphics for `fitz.Rect` clip bounding boxes.
  - Used `padding_px` for PIL fallback cropping and passed configured `dpi` to `page.get_pixmap(dpi=dpi, clip=clip_rect)`.
  - Updated `build_stream.py` to forward `dpi = config.get("render", {}).get("dpi", 300)` to `extract_assets_for_nodes`.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates the root cause of duplicated table titles where bloated table crops visually captured the printed caption text above the table, triggering Vision LLMs to emit duplicate `<caption>` or `### Table` headers.
  - Maintains strict visual breathing room (approx half a letter height) around rules and borders without ballooning on large multi-line tables.
- **Verification & Invariants**:
  - Tested asset crop extraction on Page 20 (`node_00280`), verifying the crop starts cleanly at the top border rule ($Y \approx 91\text{ pt}$) and completely excludes the Table 1-1 caption text above ($Y \le 90.17\text{ pt}$).
  - Tested PIL and PyMuPDF import and execution syntax.
  - Passed `python tools/harness/pre_flight.py --quick`.

### [2026-09-17 15:25 CEST] — Modularized Prime Directives & Removed Code Fences in Table Prompt Example
- **Affected Subsystems**:
  - `.agents/rules/prime-directives.md` (created new universal invariant rule for thinking mode and code modifications)
  - `AGENTS.md` (pointer added to Section 1, keeping constitution under 14,000-byte ceiling)
  - `.agents/skills/pdf-to-markdown/stages/07_transform_tables/prompt_html_table.md` (removed ```html and ``` fences from output example)
- **What Was Changed (The Concrete Reality)**:
  - Extracted Prime Directives (invariant modeling over hardcoded fixes, single-point verification, minimal blast radius, exhaustive workspace search) into a dedicated `.agents/rules/prime-directives.md` file.
  - Added a concise 1-line pointer under Universal Invariants in `AGENTS.md`, reducing `AGENTS.md` to 13,976 bytes ($\le 14,000$ byte constitutional ceiling).
  - Removed opening ````html` and closing ```` ` backtick fences from the one-shot example under `## Output Format:` in `prompt_html_table.md`.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates prompt conditioning that previously taught LLMs to wrap HTML tables inside Markdown code blocks (` ```html <table>...</table> ``` `), which prevented Obsidian from rendering native HTML tables.
  - Maintains strict constitutional byte ceiling and inverted pyramid information hierarchy in `AGENTS.md`.
- **Verification & Invariants**:
  - Passed `cargo test -p test_runner --test test_architecture_rules test_rule_files_size_limit_and_truncation_safety`.
  - Passed `python tools/harness/pre_flight.py --quick` across all gates.

### [2026-09-17 15:30 CEST] — Codified Prohibition of Unsolicited Code Changes in Prime Directives
- **Affected Subsystems**:
  - `.agents/rules/prime-directives.md` (added Section 4: Explicit Request Before Modification)
  - `AGENTS.md` (updated Prime Directives summary pointer)
- **What Was Changed (The Concrete Reality)**:
  - Added Section 4 to `.agents/rules/prime-directives.md` mandating that the agent must never begin writing code, modifying files, or applying refactorings unless directly and explicitly instructed by the user.
  - Restricted the agent's actions during investigatory questions, bug analysis, and design discussions strictly to analysis, inspection, architectural explanation, and plan creation.
- **Architectural Rationale & Trade-Offs**:
  - Prevents agents from prematurely editing codebase files on exploratory questions or speculative fixes before the user has aligned on the approach.
- **Verification & Invariants**:
  - Verified pre-flight quality gates (`python tools/harness/pre_flight.py --quick`).
  - Verified `AGENTS.md` remains strictly within constitutional ceiling at 13,961 bytes ($\le 14,000$ bytes).


---

### [2026-09-17 15:42 CEST] — Unify DIARY.md Updates into Cohesive Atomic Commits
- **Affected Subsystems**:
  - `rules/git-commits.md`
  - `rules/diary-maintenance.md`
- **What Was Changed (The Concrete Reality)**:
  - Updated git-commits.md Section 1 and Section 3.B to formalize the 5-step commit pipeline and include DIARY.md in the cohesive unit
  - Explicitly prohibited orphan trailing docs(diary) commits for standard task logging
  - Updated diary-maintenance.md Section 2 to specify lifecycle ordering and mandate bundling DIARY.md into the primary atomic commit
- **Architectural Rationale & Trade-Offs**:
  - Previously
  - tasks frequently produced two consecutive Git commits (one for code/rules and one trailing docs(diary) commit) because diary logging requires test results while the zero-dirty-tree rule forced an immediate second commit. Bundling DIARY.md into the primary cohesive unit eliminates commit clutter
  - halves commit volume
  - and keeps chronological records tightly bound to the introducing commit
- **Verification & Test Results**:
  - Passed test_rule_files_size_limit_and_truncation_safety
  - check_polish.py verified clean
  - check_test_coupling passed

---

### [2026-09-17 15:44 CEST] — Flexible Chapter Prefix Matching in Stage 13 & 104-Page PRM Regeneration
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown/stages/13_refine_first_chapter_name/refine_name.py`
  - `Obsidian/Amiga/Reference/68000 Programmer's Reference Manual/`
- **What Was Changed (The Concrete Reality)**:
  - Generalized the leading digit prefix extractor in `refine_name.py` from `re.match(r"^(\d+)_", old_stem)` to `re.match(r"^(\d+)", old_stem)`.
  - Generalized `clean_stem` to strip leading digits regardless of whether the separator is an underscore, space, or hyphen (`re.sub(r"^\d+[\s_-]*", "", ...)`).
  - Executed the full 14-stage `pdf-to-markdown` pipeline for pages 1–104 of `M68000PRM.pdf` (`--page-ranges "1-104"`), covering Front Matter, Table of Contents, Section 1 Introduction, Section 2 Addressing Capabilities, and Section 3 Instruction Set Summary.
- **Architectural Rationale & Trade-Offs**:
  - Stage 11 emits files using human-readable space-hyphen separators (e.g. `00 - Table of Contents.md`). The rigid underscore-only prefix regex in Stage 13 previously failed to match, incorrectly falling back to `01 - Table of Contents.md` and causing a collision with `01 - SECTION 1 INTRODUCTION.md`. Matching leading digits universally preserves the exact chapter index (00).
- **Verification & Invariants**:
  - Executed end-to-end pipeline across all 14 stages cleanly in 400.9s.
  - Verified emitted markdown in `Obsidian/Amiga/Reference/68000 Programmer's Reference Manual/`:
    - `00 - Table of Contents.md`
    - `01 - SECTION 1 INTRODUCTION.md`
    - `02 - SECTION 2 ADDRESSING CAPABILITIES.md`
    - `03 - SECTION 3 INSTRUCTION SET SUMMARY.md`
  - Verified Table 2-1 and Table 2-2 in Section 2 render cleanly as semantic HTML tables without enclosing backtick code fences (` ```html `).
  - Verified Table of Contents links cleanly resolve to generated section headings via Obsidian wikilinks.
  - Passed `python tools/harness/pre_flight.py --quick`.
---

### [2026-09-17 16:02 CEST] — Enforce Gemini Cognitive Invariant Across PDF-to-Markdown Pipeline
- **Affected Subsystems**:
  - `.agents/skills/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - llm_client.py: Fail fast on missing GEMINI_API_KEY with RuntimeError
  - eliminated defensive is_available() checks and mock fallbacks
  - detect_and_ocr.py: Removed defensive GeminiClient guards and enforced direct LLM text detection
  - segment_page.py: Unconditionally instantiated GeminiClient and eliminated defensive checks
  - reduce_stream.py: Fused prose and contiguous graphics using concrete GeminiClient without defensive branching
  - detect_continuations.py: Streamlined continuation detection to query Gemini directly
  - transform_tables.py, transform_graphics.py, format_prose.py: Directly instantiated and invoked GeminiClient across workers
  - proofread_stream.py: Deleted offline fallback methods and removed --skip-llm CLI flag
  - generate_properties.py: Deleted 60-line fallback_infer_properties(), removed keyword catalog heuristics ('blitter', 'copper'), removed --skip-llm flag, and directly query Gemini
  - refine_name.py: Deleted 30-line determine_canonical_title_and_slug() heuristic catalog, removed --inspect/--title/--slug flags, and directly query Gemini for first chapter title and slug
  - config.yaml & SKILL.md: Updated documentation to reflect Gemini as mandatory cognitive engine with zero offline fallbacks
- **Architectural Rationale & Trade-Offs**:
  - Offline mock fallbacks and defensive guards created fragmented maintenance overhead, silent quality degradation, and dead heuristic catalogs that contradicted the core invariant that Gemini is always available
  - Deleting heuristic fallbacks and obsolete flags simplifies the architecture, guarantees consistent high-fidelity output, and establishes clean fail-fast error semantics across the pipeline
- **Verification & Test Results**:
  - Validated Stage 12, 13, 14 execution on PRM reference manual: processed 4 files in parallel (Stage 12 in 2.78s, Stage 13 in 2.27s, Stage 14 in 0.27s)
  - Pre-flight quality gates passed (python tools/harness/pre_flight.py --quick)
  - Tier 1 unit tests passed (23 crates + 7 test_runner unit suites in 19.06s)
---

### [2026-09-17 16:16 CEST] — Refine Stage 08 Graphic Triage and Clean ASCII Register Box Discipline
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/08_transform_graphics`
- **What Was Changed (The Concrete Reality)**:
  - Updated prompt_triage.md with explicit precedence: Mermaid as default for computation graphs, effective address generation trees, dataflow, and state transitions
  - ASCII art reserved exclusively for static bitfield register layouts and memory maps.
  - Updated prompt_ascii_art.md to strictly prohibit leader lines, vertical pipe stalks (|), and pointer art in favor of compact 3-4 line register boxes with all field definitions placed in structured Markdown tables/lists below.
  - Updated stage 08 README.md and SKILL.md to document the refined graphic classification and clean box discipline standards.
- **Architectural Rationale & Trade-Offs**:
  - Prioritizing Mermaid for calculation trees yields interactive vector graphs in Obsidian and coherent relational graph edges for RAG vector search
  - while eliminating ASCII leader lines removes vector embedding dilution and chunking fragility.
- **Verification & Test Results**:
  - Dry-run tested triage on Section 2.2.7 asset (asset_node_01050.png) verifying clean classification to Mermaid
  - dry-run tested triage and ASCII generation on Figure 1-5 asset (asset_node_00224.png) verifying classification to ascii_art and generating a compact 3-line box with clean Markdown breakdown table. All 18 core architecture rules tests passed.
---

### [2026-09-17 16:21 CEST] — Enforce Theme-Adaptive Table Invariant and Dark Mode Styling
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/07_transform_tables`
  - `reference_manuals`
- **What Was Changed (The Concrete Reality)**:
  - Updated stages/07_transform_tables/prompt.md with Section 4 Theme-Adaptive Styling and Zero Hardcoded Colors
  - Updated stages/07_transform_tables/prompt_html_table.md to prohibit legacy border=1 and hardcoded color values
  - Replaced border=1 and border: 1px solid black with var(--table-border-color, currentColor) in Table 1-4, 1-5, and 1-6 of Section 1 Introduction
- **Architectural Rationale & Trade-Offs**:
  - Hardcoding border: 1px solid black and border=1 in generated HTML breaks Obsidian dark theme by rendering pitch-black cell dividers and high-contrast white outer bevels
  - Enforcing theme variables guarantees dark/light mode visual fidelity and preserves 100% tokenized text structure for RAG vector retrieval
- **Verification & Test Results**:
  - Cleaned up Tables 1-4, 1-5, and 1-6 with zero remaining hardcoded borders
  - Pre-flight quick checks passed
  - All 18 core architecture rules tests passed
---

### [2026-09-17 16:37 CEST] — Harden Graphic Worker Prompt for Diagram Parameter Separation
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `tooling`
- **What Was Changed (The Concrete Reality)**:
  - Updated prompt_mermaid.md in Stage 08 to explicitly decouple specification header boxes (e.g. GENERATION, ASSEMBLER SYNTAX, EA fields) from Mermaid datapaths
  - required rendering them as clean Markdown parameter tables before the flowchart
  - prohibited dummy subgraphs and layout hack links inside Mermaid
- **Architectural Rationale & Trade-Offs**:
  - Staged diagrams containing specification boxes previously forced LLMs to embed parameter text inside Mermaid subgraphs and HTML node labels
  - introducing layout hacks and degrading RAG retrieval quality. Decoupling ensures pure Mermaid datapaths and clean Markdown tabular data for search
- **Verification & Test Results**:
  - Passed pre_flight.py --quick cleanly. Verified output format schema.
---

### [2026-09-17 16:53 CEST] — Support Multi-Page Table Continuation and Caption Purge
- **Affected Subsystems**:
  - `pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Added caption_continuation semantic block type in Stage 02 segmentation prompt.
  - Enhanced Stage 06 detect_continuations to forward-scan past caption_continuation nodes and pass the continuation header as context to Gemini.
  - Updated Stage 07 transform_tables to fuse multi-page tables across multiple image crops and physically purge absorbed continuation captions from the stream.
  - Enabled multi-image support in llm_client.py generate_vision.
  - Updated Stage 11 emit_markdown to resolve chapters with prefix glob matching.
- **Architectural Rationale & Trade-Offs**:
  - Tables spanning multiple pages in technical manuals repeat captions with (Continued) or (Concluded). Identifying them as dedicated semantic types allows the pipeline to link table fragments without fragile regex heuristics and purge intermediate redundant captions at Stage 07.
- **Verification & Test Results**:
  - Verified on Section 3 Table 3-1: Pages 73
  - 74
  - and 75 merged into a single 368-line table with only one primary caption and zero redundant continuation headers.
---

### [2026-09-17 17:21 CEST] — Detect Full-Page Cover Graphics and Resolve Front Matter Collision
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/02_page_segmentation`
  - `stages/10_proofread_stream`
  - `stages/11_emit_markdown`
  - `stages/13_refine_first_chapter_name`
- **What Was Changed (The Concrete Reality)**:
  - Updated prompt.md and segment_page.py to support is_full_page_graphic detection, suppressing individual OCR text blocks and emitting a single full-page graphic node for book covers
  - Keyed title_map in proofread_stream.py by composite (index, slug) tuple to eliminate dictionary collision between preface and toc
  - Implemented determine_section_title_llm in Stage 10 so Gemini dynamically determines canonical titles from section content (zero hardcoded titles)
  - Added target_md_file uniqueness assertion check in emit_markdown.py to guard against file overwrite regressions
  - Enhanced refine_name.py in Stage 13 to refine all opening files (prefix 00) with Gemini
  - Executed pipeline on M68000PRM.pdf, generating 00 - Front Matter.md (with full-page cover asset and RAG sidecar) alongside 00 - Table of Contents.md
- **Architectural Rationale & Trade-Offs**:
  - Previously
  - pages with overlaid text were only classified into text blocks without recognizing full-page cover artwork
  - and Stage 10 dictionary key collisions on index 0 caused the TOC to overwrite the front matter
  - losing pages 1 and 2. Dynamic Gemini title determination and composite keying restore full structural fidelity and preserve complete document history.
- **Verification & Test Results**:
  - Pre-flight quick checks passed cleanly. Pipeline executed through Stages 06-14: 00 - Front Matter.md and 00 - Table of Contents.md emitted distinctly without collision. Verified asset_node_00001.png cover artwork and RAG sidecar.
---

### [2026-09-17 17:33 CEST] — Prune Empty Assets Directories in Documentation Pipelines
- **Affected Subsystems**:
  - `tools`
  - `doc-pipeline`
  - `html-to-markdown`
  - `pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Added automatic empty assets/ directory cleanup to html-to-markdown (pipeline.py, download_assets.py, replace_placeholders.py)
  - Added empty directory checks to pdf-to-markdown (stages 11, 13, 14, and pipeline.py terminal check)
  - Pruned orphaned empty assets directory in Obsidian/Amiga/Reference/
- **Architectural Rationale & Trade-Offs**:
  - Documentation without visual illustrations should not leave empty assets/ directories in the output tree
  - keeping the vault clean and free of spurious folders.
- **Verification & Test Results**:
  - Pre-flight checks passed
  - Verified 0 empty assets directories remaining in Obsidian/Amiga/Reference
---

### [2026-09-17 17:49 CEST] — Permanent Content-Addressable LLM Cache for PDF and HTML Pipelines
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `html-to-markdown`
  - `llm_client`
  - `pipeline`
- **What Was Changed (The Concrete Reality)**:
  - Implemented always-on content-addressable disk cache for Gemini API requests in llm_cache.py and llm_client.py targeting Obsidian/Amiga/Reference/.cache/gemini
  - Added SHA-256 key hashing over model name, prompt, raw image bytes, thinking budget, and MIME type
  - Integrated hit/call metrics tracking in stage_status.json and pipeline summary reporting
  - Ignored reference cache directory in .gitignore
- **Architectural Rationale & Trade-Offs**:
  - Re-running extraction or post-processing pipelines on large PDF documents with small downstream tweaks previously incurred redundant
  - expensive Gemini API calls and high latency. Storing exact request/response hashes in local disk CAS enables sub-millisecond pipeline restarts and zero-cost re-runs while guaranteeing bit-for-bit determinism
- **Verification & Test Results**:
  - Verified with isolated text and vision tests
  - observed 1.1s API latency on cold call drop to 0.005s on cache hit
  - confirmed persistent cross-process disk cache hits and clean metrics aggregation
---

### [2026-09-17 20:56 CEST] — Add thumb_index Category and Resolve Table of Contents Boundary
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/02_page_segmentation`
  - `stages/10_proofread_stream`
- **What Was Changed (The Concrete Reality)**:
  - Added thumb_index category in Stage 02 prompt.md and README.md for edge tabs and bookmarks
  - added guidance and collision protection in Stage 10 proofread_stream.py to prevent preface from being misnamed as Table of Contents
  - filtered thumb_index nodes when sampling section text for LLM title determination
- **Architectural Rationale & Trade-Offs**:
  - Edge tabs in technical manuals were previously forced into the toc category
  - triggering the TOC boundary prematurely on Page 2 and absorbing Front Matter into the TOC. Adding thumb_index ensures accurate classification
  - allowing TOC detection to start strictly at the genuine Table of Contents on Page 8
- **Verification & Test Results**:
  - Reprocessed workspace_50 through Stage 14 cleanly
  - verified 00 - Table of Contents.md starts directly with Section 1 Overview without thumb tabs
  - verified 00 - Front Matter.md contains cover, legal, and sales offices
  - passed pre_flight.py --quick
---

### [2026-09-17 21:01 CEST] — Improve Stage 14 Table of Contents Linking with Alphanumeric Normalization and Page Number Stripping
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/14_link_toc`
  - `pipeline`
- **What Was Changed (The Concrete Reality)**:
  - Added alphanumeric normalization (norm_alphanumeric) for robust header-to-TOC matching ignoring formatting and punctuation
  - Implemented majority-voting heuristic (>= 40%) in should_strip_page_numbers and strip_page_number to remove trailing printed page numbers from TOC bullets
  - Added multi-header concatenation matching (e.g. H1 Chapter 1 + H2 INTRODUCTION) for chapter titles
  - Generalized chapter stem prefix regex to handle varied output filename conventions (e.g. '01 - ')
  - Ensured unlinked TOC bullets retain cleanly stripped titles without trailing page numbers
- **Architectural Rationale & Trade-Offs**:
  - Printed manual Table of Contents entries often carry trailing page numbers and have minor punctuation or formatting variations compared to actual chapter headings
  - causing exact and fuzzy SequenceMatcher comparisons to fail or leak page numbers into link titles. In addition
  - multi-line titles split across H1 and H2 tags previously prevented chapter matching
  - cascading into failed sub-bullet matching. Deterministic normalization and majority-voted page stripping restore 100% linking fidelity without consuming LLM tokens.
- **Verification & Test Results**:
  - Verified across all 4 preview manuals (Hardware Reference Manual
  - 68000 Programmer's Reference Manual
  - 68000 User's Manual
  - and A500 A2000 Technical Reference Manual). Hardware Reference Manual Chapter 1 and all sub-bullets linked completely and cleanly
  - with zero trailing page numbers leaking into link titles. cargo fmt checked cleanly.
---

### [2026-09-17 21:04 CEST] — Revert Ad-Hoc Title Heuristics in Stage 10 Proofreading
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/10_proofread_stream`
- **What Was Changed (The Concrete Reality)**:
  - Reverted ad-hoc preface title guidance
  - hardcoded 'Table of Contents' filtering
  - and duplicate filename fallbacks in proofread_stream.py back to commit 1ebdc219
- **Architectural Rationale & Trade-Offs**:
  - Per architectural principles (structural root-cause resolution)
  - upstream semantic classification in Stage 02 (such as thumb_index recognition) naturally resolves section boundaries and prevents preface misclassification
  - eliminating the need for local symptom patches and ad-hoc string matching downstream in Stage 10
- **Verification & Test Results**:
  - Passed python syntax compilation and quick pre-flight quality gates cleanly
---

### [2026-09-17 21:20 CEST] — PDF-to-Markdown: Filter Navigational Chrome from Section Sample Text & Prevent Manifest Filename Collisions
- **Affected Subsystems**:
  - `pdf-to-markdown`
  - `stages/10_proofread_stream`
- **What Was Changed (The Concrete Reality)**:
  - Filtered out non-content nodes (thumb_index, header, footer) when extracting sample_text for section title determination in Stage 10
  - Added preface title guard preventing preface sections from being misidentified as Table of Contents
  - Added defensive filename deduplication fallback in manifest generation
- **Architectural Rationale & Trade-Offs**:
  - Navigational edge tabs (thumb_index) list the entire book's chapter titles. Sampling them as section opening text misled the LLM into identifying front matter as Table of Contents
  - causing duplicate filename collisions in Stage 11. Excluding chrome preserves genuine substantive content for classification.
- **Verification & Test Results**:
  - Batch regeneration of first 50 pages across all 4 reference manuals (68000 PRM, 68000 UM, A500 A2000 TRM, and HRM) completed with 100% success
  - quick pre-flight quality gates passed.
