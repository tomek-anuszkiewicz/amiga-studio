# Amiga 500 Emulator: Engineering Diary & Architecture Genesis

This document records the architectural evolution, human-AI co-design methodology, and pivotal engineering decisions behind the Amiga 500 emulator in Rust. Dated results describe the source and scope checked at the time; they do not certify the current emulator. The design specifications and roadmap own present behavior and pending work.

> [!IMPORTANT]
> **Zero Hand-Written Code (100% Voice-Prompted & AI-Generated):**
> Throughout the entire development of this emulator, **not a single letter of code was written by hand**.
> The entire codebase—the cycle-exact Motorola 68000 CPU core, memory bus, immediate-mode Developer Studio GUI, debugger engine, and test harnesses—was generated 100% by the AI agent through spoken voice prompts, conversational code review, and iterative engineering direction. Aside from occasionally pasting a documentation link or typing a brief message when voice dictation was inconvenient, the human functioned strictly as the chief architect, code reviewer, and quality enforcer.

---

## 1. Project Genesis: Groundwork, Reference Ingestion & The Myth vs. Reality of AI Emulation

### The Pre-Rust Phase: Physical Schematics, LTspice & Analog Audio Synthesis (Late August 2026)
Before a single line of Rust code was written, the human spent the final week of August 2026 establishing the physical ground truth of the Amiga 500 hardware:
- **Analog Low-Pass Audio Simulation (LTspice):** The human modeled Paula's analog audio reconstruction circuit in LTspice (`0b2777fa`), extracting the exact resistor (ohms) and capacitor (farads) values from original Commodore schematics (`c448cd14`). This captured both the fixed 2-pole Sallen-Key low-pass filter and the switchable "LED filter" controlled by CIA-A port A bit 1.
- **Band-Limited Step (BLEP) Table Generation:** In a dedicated C# prototype (`9057f4a8`, `7783f1e2`), the human derived and verified the continuous-time impulse response and step response tables needed for alias-free audio interpolation, parameterizing them directly against physical component physics rather than arbitrary digital filters.
- **Reference Emulator Ingestion:** To prevent speculative design, five verified open-source reference emulators were imported into `ref_src/` for cross-validation: MAME, Moira, Musashi, vAmiga, and WinUAE.
- **Decoded Single-Step Vectors:** Over one million software-generated SingleStep cases were imported and structured (`a31518dc`), providing reference state and bus sequences before CPU implementation. Later work clarified that agreement with these vectors did not constitute a physical hardware capture.

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
3. **Early Address-Space Model:**
   - **Unmapped Reads:** The initial model returned configurable `$FF` / `$FFFF` values. October consolidation replaced the configuration with shared fixed placeholders; neither version established physical bus retention or decay.
   - **Kickstart Overlay:** CIA-A port A bit 0 controls the overlay latch. On cold boot, bank 0 (`$000000-$07FFFF`) mirrors Kickstart ROM until the first hardware write flips the latch to expose physical Chip RAM.
   - **Gary Chip RAM Mirroring & Slow RAM:** Accurate address decoding for standard 512 KB Chip RAM, 512 KB A501 trapdoor Slow RAM (`$C00000`), and Gary custom register mirrors (`$DF0000-$DFFFFF`).

### September Integration and October Ownership Corrections

The September 12 foundation separated the disassembler, physical memory, bus and machine coordinator. On September 18, bank handlers took direct responsibility for `BusResult`, Agnus retained DMA address mastership, and Denise/Paula consumed delivered words rather than fetching Chip RAM themselves. Floppy control moved out of MemoryBus into motherboard routing through CIA and Paula. Serializable chip state remained separate from runtime handles. The then-current Cartesian contention test checked the model's wait-state relation; its later replacement is recorded in Chapter 5.

Early September 14-15 integration work covered Copper fetch/comparator behavior, CDANG and the PAL `$E0` slot restriction; Blitter minterms, shifts, modulo, unconnected latches and fill; and Denise window/serializer behavior. The earlier diary described these as silicon calibration and passing vAmigaTS suites. Those historical claims do not establish measured timing or native external-suite execution. The capture infrastructure was later retired, and book-derived reconstruction remained necessary; Chapter 14 records that harness transition.

On September 29, `62c9f4c` closed the then-numbered Step 1.4 architecture review, mapping CCK progression, bus arbitration and inter-chip coordination. Historical quick preflight and twenty architecture checks passed; this was an architecture review, not functional completion of the chipset.

October 12-14 work centralized OCS register names and CIA addresses, removed unsupported BPLCON3/DIWHIGH state, and routed shared BPLCON0, display-window and sprite writes explicitly. Agnus gated bitplane fetches by the vertical window and DDF while Denise clipped horizontal output. DMA pointers were bounded to the OCS 512 KB range and Paula's direct Chip RAM fetch path was retired. Serialized fields became explicit rather than silently defaulted. Focused routing/pointer tests and applicable commit checks passed; later register, Gary and signal stages in Chapters 11-13 refined these foundations without certifying the receiving engines.

The CPU crate was renamed from `m68000` to `cpu` on September 19 (`9ec915c`). Earlier paths in the CPU history describe their original snapshots.

---

## 4. Physical Circuit Simulation: Standalone BLEP Audio Synthesis

A dedicated engineering effort was devoted to modeling the physical analog sound output of the Amiga Paula chip with pristine accuracy:
- **WinUAE Parameterization & Circuit Derivation:** The Band-Limited Step (BLEP) synthesis engine was modeled and parameterized based on verified reference implementations in WinUAE.
- **Pure Mathematical Standalone Module:** Rather than hardcoding magic sound samples, the BLEP synthesis is implemented as an independent, decoupled module containing all the underlying physics and signal-processing mathematics (`crates/paula/src/blep_tables.rs`).
- **Analog RC Component Values:** The step response and frequency roll-off tables are computed directly from the physical component values of the Amiga motherboard circuit—the precise resistance (resistors in ohms) and capacitance (capacitors in farads) forming the analog low-pass filter network (including dynamic CIA-A LED filter switching).
- This ensures alias-free, cycle-accurate analog audio reconstruction directly from first physical principles.

The November 8 regression work protected retained numerical curves without establishing physical audio output. Its offline reference checks and optional WinUAE comparison belong to the verification history in Chapter 14.

---

## 5. The 6-Stage Evolution of the M68000 CPU Core

The CPU core (`crates/m68000` at the time) did not emerge in a single iteration. Six distinct evolutionary stages between September 6 and September 10, 2026 established the early baseline; October reconciliation and its verification limits follow those stages below.

### The Foundational Methodology: Minimal Representative Set & Early SingleStepTests
From day one, the development strategy was deliberately disciplined: rather than attempting to blindly generate all 65,536 opcodes at once, the human instructed the agent to identify a **minimal representative subset of instructions** spanning every major instruction class (data movement, integer arithmetic, logic, shifts/rotations, control flow, and bit operations) and addressing mode.
Throughout Stages 1 and 2, prototyping, cycle slicing and architectural alternatives were checked against software-generated **SingleStepTests** reference cases. Those comparisons tested agreement with the reference model, not physical silicon.

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
- **The Architectural Breakdown:** While this successfully proved that memory wait states could stall execution against SingleStepTests, the resulting architecture was awkward and fragmented:
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
- **Historical SingleStepTests Result:** The early baseline recorded passing all 127 selected suites, approximately 300,000 software-generated vectors; this was not physical hardware validation.

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

### October 11: Timed Transfers, Exceptions and Continuation

The later CPU-CYCLE-01, CPU-PC-01 and CPU-DOCS-01 work reconciled timed reads, writes and internal CCK phases across arithmetic, logic, compare, transfer, branch, frame, return and system instructions. Predecrement calculation was separated from access, long read-modify-write sequences retained their supported word order, and Address Error began at the rejected transfer with phase-correct frame PCs. MOVE/MOVEM completion, CHK/TRAPV timing, STOP suspension, RESET ownership and TAS writes were corrected. Illegal and Line A/F exceptions used the rejected opcode PC; Trace sampled T at instruction entry before an accepted IRQ. External level-seven NMI was excluded by user direction.

The Cartesian DMA permutations were replaced with retry checks across 52 public CPU transfers (`6df6c3e`), keeping the hardware-derived slot schedule separate from runtime arbitration. The final-source comparison passed 1,000,060 vectors across 124 original files, including 178,087 Address Error entries; all original hashes remained unchanged. Commit and milestone profiles passed, with 128/128 SingleStep groups, 1,002,737 reported executions and two override controls. Existing reviewed ASL.b corrections remained explicit. These software-generated vectors and synthetic bus fixtures did not prove electrical pins, device IACK, external BERR or arbitrary in-flight restore.

CPU-CONTINUE-01 (`c53eb76`) then added fifteen continuation cases for reset, exceptions, returns, waits, snapshots and following guest effects. SINGLESTEP-MUTATION-01 (`d5a3708`) required exact formal next PC, both prefetch words, timed callbacks and valid vector structure. Its eleven-seed pilot used 1,003 raw mutations: comparison rejected 748 observable changes, format validation rejected 165 invalid copies and 33 equivalent controls passed. Ten affected groups passed 80,650 executions. At user direction, SINGLESTEP-RAM-01 (`6663ab8`) removed final RAM comparison and its Address Error allowances because synthetic writes already followed the timed callbacks; initial RAM ranges stayed validated. Follow-up commit profiles and focused continuation tests passed, but full SingleStep and milestone profiles were not rerun for that follow-up. Physical keyboard timing, external BERR, level-seven NMI and arbitrary active-slice restore remained outside scope.

### October 23-25: DBcc and Opcode-Legality Boundaries

DBcc expiry was corrected to fourteen CPU clocks with three timed reads and wait-state regressions (`2505924`). Motorola PRM review removed six unsupported PC-relative CMPI primary words and aligned disassembly and the local opcode inventory (`9e7efe6`). The software-generated map agreed with dispatch and disassembly across 65,536 words; that agreement was not an independent Motorola-derived inventory. The user closed opcode-legality work without the complete primary-source inventory. Focused CPU, disassembler and architecture checks passed, while the then-current commit profile failed Book Review UI/assets; physical captures and milestone verification were not run for that closure.

---

## 6. CPU Benchmarking: From Generated Cases to Handwritten Workloads

The initial 108-case benchmark catalog generated repeated guest instructions in memory and measured host time with warm-up, noise filtering and protected CSV hashes. It supported early CPU profiling, but its catalog, trace regressions, hashes and analyzer were later retired. The measurements and anomaly claims from that runner are historical, not a current performance baseline.

A decoder-derived CPU-BENCH-01 pilot on October 12-14 enumerated 45,821 legal primary opcodes and measured 4,571 representative workloads, with 27 targeted retries and 47 NOP controls. All workload replays matched state and CPU clocks; the recorded full SingleStep run passed 1,002,739 executions. Its commit profile still failed then-existing Book Review errors and an RAG adapter timeout. Its fixture and NOP-control protocol differed from the later program-level measurements, so those numbers cannot be compared directly. The decoder-derived runner was subsequently removed from the active benchmark path.

October 24-26 work replaced generated candidates with handwritten, assembler-backed programs in `benchmarks/m68k`, including branch, counter and conditional-store paths (`7412408`). The reviewed full run completed 2,257 cases and 6,771 timed series without execution failures or persistent load disturbance. Per-series timing, load checks and bounded retries made the measured path reviewable. The user approved replacing the reference; tracked hashes and source-run identities matched that run, while raw host-specific reports became local. A later approved snapshot refreshed 86 cases (`98da1ad`) without repeating the complete measurement. Focused Python/Rust checks passed; the then-current commit profile failed Book Review contracts, and full SingleStep was not rerun for the later snapshot. One catalog commit explicitly waived tests and was not remeasured.

The living performance specification and benchmark README own the executable workflow. Compare measurements only when program source, configuration and timing protocol match; retired traces, hashes and profiling tools are historical evidence, not available commands or a current baseline.

---

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

## 8. Architectural Rules, Anti-Tamper Policies & The Living Definition of Done

As the workspace grew, architectural decisions became explicit rules and executable checks: cohesive source files, flat CPU instruction ownership, no custom macros or const-generic instruction handlers, no runtime host panics, named hardware operations and allocation-free hot paths. Protected vectors, timing references and benchmark baselines could not be changed merely to obtain a passing test. The September 19-21 compiler work made warnings fatal and removed artificial encapsulation requirements that obscured plain hardware state.

The enforcement mechanisms changed with the project. Early Rust architecture tests, automatic coupling rules and merge-only guidance were later replaced or narrowed; their old lists are not the present contract. [Chapter 14](#14-sdlc-agent-workflows-and-development-harness) records that evolution and its evidence. [AGENTS.md](AGENTS.md), the owning rules and the verification README define the current obligations.

---

## 9. The Ultimate Goal: The Clean-Room Re-Generation Experiment

Every bug fix, architectural decision, and hardware quirk in this project has been continuously documented in `Obsidian/Amiga/Design/`.

This serves a larger, ambitious vision:
- Once the complete, working Amiga 500 emulator is finished and verified against all reference suites,
- We will wipe the implementation source code (`crates/`) and reference emulator sources (`ref_src/`), leaving strictly:
  - The refined prompt and skill system (`.agents/`).
  - The curated, de-duplicated design documentation (`Obsidian/Amiga/Design/`).
  - The automated test suites (SingleStepTests, vAmigaTS).
- **The Experiment:** Can an AI agent, armed with the lessons, rules, and architecture recorded during this project, autonomously re-synthesize the complete cycle-exact emulator from scratch?
- The separately scheduled scope is recorded in [Roadmap Step 9](ROADMAP.md#step-9-documentation-to-code-regeneration-experiment).

This diary stands as the living record of how those foundations were built.

---

## 10. Accepted-Recipe Reconstruction and Reference Publication

On October 29, 2026, `ebc99c8` delivered Book Review reconstruction from an accepted `book.json` recipe and its matching source PDF, plus publication of reviewed Markdown with stable chapter links and image descriptions. This completed a tooling capability; it did not certify the Test Book or close the four-manual conversion stage. The September-October path to that delivery separated source preparation, machine execution and author acceptance after several earlier mechanisms were replaced.

### Source Preparation and the Rejected Pipeline Assumptions

The September 15-16 converter began as a fourteen-stage multimodal pipeline with Gemini OCR, normalized page boxes, table reconstruction and text descriptions for diagrams. A selected twenty-page trial was recorded; it did not certify full-book quality. October 5-6 work replaced transport with subscription-authenticated Codex requests and explicit model/effort settings. Cache replay avoided new requests for identical inputs, but successful execution remained separate from fidelity acceptance.

PDF-TEXT-1.1 (`5a9add8`, October 6) made Stage 01 read a reopened native/OCR PDF rather than treating cached OCR JSON as the text-layer artifact. It bound physical page IDs, text geometry and PNG transforms. PyMuPDF's q/Q wrappers were permitted only around unchanged source streams, with identical selected-page renders. The user closed the task using page-64 evidence corresponding to source-manual page 5: the source hash and all 160 page geometries were preserved, and Stage 01 extracted twenty blocks without inference. The OCR result came from one live request before cache reuse. Focused PDF/shared suites passed nineteen tests each, but the milestone profile failed 62 existing reference links and 22 unchanged Rust conditions. Native-copy, mixed, blank and real rotated/cropped categories remained unverified. This was user-directed scope closure, not a passing repository milestone.

Later preparation moved to local Tesseract through PyMuPDF, retaining source geometry over graphics. PDF-STATE-1 replaced manifests, lineage validators, snapshots and manual handoffs with stage-owned artifacts and atomic execution status; those earlier safeguards were removed by user direction rather than retained as a compatibility layer. Named attempts preserved their configuration, restart cleared the chosen stage and successors, and downstream selection retained physical page identity. Historical closure checks passed PDF/shared/publication suites at 25/21/10 tests and architecture at 19/19; migration retained 645 result files without OCR or model calls. Publication was explicit, protected working sources and accepted only absent or empty destinations. None of this proved whole-book reconstruction or bulk publication.

### Page Objects, Ordering and Cache Identity

October 7-10 work established source-backed page objects, graphic/prose separation, table spans, code roles, optional rotation and review against original-page geometry. Filtering deliberately removed page furniture, selected front matter and standalone margin logos while preserving source tables, character diagrams and drawing sheets. Independent descriptions remained separate from coordinated diagram labels. HTML tables received saved RAG text; both table formats retained source-image companions. Final book assembly moved out of PDF conversion into Book-to-Markdown.

PDF-ORDER-2.46 was closed by user request without certifying all reading-order observations. A subsequent deterministic row-grouping and left-to-right ordering stage was also removed at user request because geometric sorting could override intended source order. The retired stages and their tests are unavailable; their historical passes do not certify current ordering.

Compact JSON omitted defaults, but equivalent field values alone did not guarantee a cache hit: prompt bytes, serialization order, schema, model, effort and images all contributed to identity. Replacement fields therefore retained contract order. Temporary aliases were limited to verified equivalent requests and then removed; changed page-54 geometry was retained as a legitimate miss. A historical audit found 138 Stage 02.42 hits with no misses, followed by 81 Stage 02.44 hits and that one legitimate miss. Source-hash variants kept source identity separate from response reuse. The October 28 relocation (`e35239e`) preserved 6,694 files, including 4,775 cache entries, without changing response identities or running a new conversion/import.

November 10-11 changes `47580a8` and `d8c9e33` extended reuse to completed PDF preparation, per-page PNG/text bundles and graphic crops. Cache entries bound source bytes, effective settings and runtime identity to output hashes; file existence alone was insufficient. Crop identity included the source PNG and bounding box. Stages 04, 05, 07 and 08 stored only changed pages, while readers selected the latest replacement over Stage 03. An explicit empty filtered page prevented excluded objects from returning. These changes reduced repeated preparation without certifying transcription quality.

Temporary capacity failures moved to fresh transports with exponential backoff that could be interrupted; budget failures remained terminal. Coordinator cancellation interrupted worker requests, delays and retry waits after a regression reproduced an unwanted extra request. On Windows, a system execution request prevented idle sleep during selected pipeline stages and restored the previous state on exit. PDF conversion and table defaults used high reasoning effort. Historical checks passed 48 shared, two HTML, 48 PDF and 192 Book Review tests; the initial commit profile failed only on five vocabulary terms, then passed after explicit user approval. Sparse-page follow-up checks covered 31 PDF execution/cache, six proofreading, two parallel-table and four import tests. No new book conversion or import was run for those deliveries.

### Working, Accepted and the Test Book Decisions

The editor made Working the edit surface and Accepted an explicit author decision, with guarded Codex edits, source selection, merge controls and chapter organization. On October 23-25 the selected source became the 308-page A500/A2000 OCR PDF; other manuals remained local references. The initial one-chapter recipe was later replaced by a fourteen-chapter, 975-element Accepted model. The user closed then-Roadmap 1.1 while description, proofreading and standalone reconstruction remained separate. Its 102 passing editor contracts and matching recipe hashes did not certify source attribution: a later audit found 319 invalid fields. Commit and milestone checks still had English/PDF failures.

October 26-27 work recorded the rebuilt PDF identity `78e52f075e17`, a 999-element recipe and source-backed substitutions, accepted literals and visible source differences. The tracked recipe then differed from ignored Accepted in 74 elements. Stage 08 introduced text-first proofreading with optional page-image resolution and validated `md_text` patches before Book import, replacing chapter proofreading. At those commits, no live Stage 08 conversion/import or chapter-quality review ran, and commit profiles still failed English, with an earlier PDF `KeyError: 'blocks'`.

On October 27, `570d578` replaced import matching with page-local source alignment and published the current Accepted recipe. The user accepted chapter formatting and short edited fragments while retaining authored source references, closing then-Roadmap 1.3/1.4 by direction. The 160 passing Book Review tests and matching root/Accepted and PDF hashes did not establish semantic attribution or standalone reconstruction: source audit still found 56 fields requiring attention among 940 fields/26,360 references; commit/milestone English checks and five semantic checks failed. Standalone extraction and the four manual recipes remained pending. Chapter/contents renaming, scoped image/table descriptions and active-request cancellation followed in `ac53f5e`, with 182 Book Review and 44 shared conversion tests passing; no new conversion/import ran for that delivery.

On November 12, `783bb8a` added a read-only inventory of unmatched text with source context and navigation. Source alignment retained code and numbered content, joined local hyphenated continuations without absorbing a fused OCR glyph, and preserved offsets through multiline math and HTML. Resolved text was cached by recipe and source-page version with replacement-race checks; incremental viewer updates retained unchanged chapters, resources and reading geometry. Working revision checks still guarded acceptance and discard. Initial English findings were resolved through approved API vocabulary and file-specific test/regex exceptions before the full commit profile and independent review passed. Focused cache, browser, alignment and recipe suites passed 7, 6, 31 and 37 tests respectively; server availability was checked separately from content quality.

Integration in `45f9094` exposed a further source-mapping defect: raw backtick matching confused escaped delimiters, joined markers across separate inline containers and selected repeated text from the wrong table cell. The repair used parser-owned code spans, mapped every inline container in source order and decoded recipe escapes while retaining cleanup offsets. Alignment, cleanup and unmatched-text inventory shared the corrected boundaries. Historical final checks passed 33 alignment and 39 recipe tests, all 222 Book Review contracts and the full commit profile, with independent extended approval. Those results did not run a new conversion or reconstruction, full SingleStep or a milestone profile.

### Portable Reconstruction and the Remaining Manual Recipes

The accepted recipe became the portable handoff between clones. Reconstruction rebuilds the editor's local page and asset workspace from the identified source while retaining the authored recipe. Working remains the edit surface, and acceptance remains an explicit author decision. Bulk description tasks were limited to visible Working elements rather than hidden content. Publication and RAG indexing remained a downstream user workflow. The same delivery added a structural frontmatter gate for local reference Markdown; that gate checks metadata, not source fidelity.

The recorded Test Book replay completed PDF stages 01-08 with 545 cache hits and zero new model calls, then imported 993 elements. Source validation still reported 56 invalid fields in that raw import. The user retained the fresh import without treating successful execution as source acceptance. The closing commit records a passing commit profile, 35 design specifications with 691 resolving links, diary/roadmap checks and valid frontmatter in 104 local Markdown files. These are historical results for that delivery, not checks rerun during diary compaction.

November 11-12 artifact commit `3d54d55` recorded the A500/A2000 accepted recipe and Hardware Reference Manual OCR provenance without tracking local PDFs or generated review bundles. The 2,000-element recipe matched Working and Accepted, and source/prepared hashes matched their identity records. An initial audit resolved 60,164 references but reported 112 literal-content or unsupported-math fields. Thirty edited fields raised the resolved-reference count to 60,546, with 107 such fields remaining. The user explicitly accepted known source-occurrence mismatches on pages 149, 150, 183 and 242 for this artifact; the general attribution requirement was unchanged. The commit profile and independent review passed with that exception. Valid addresses and matching hashes did not establish correct printed occurrences, transcription quality or clean reconstruction.

The roadmap pilot was removed by user direction, rather than verified as complete: the fresh Test Book had no Accepted bundle or demonstrated clean reconstruction gate. The remaining repository deliverable was four accepted manual recipes with matching source identities, now owned by [Roadmap 11.1](ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion). Full-book quality, downstream publication and RAG ingestion were not established by the tooling delivery. These unresolved limits remain attached to the narrative; they do not make every settled converter change an active rolling entry.

---

## 11. Register Access and Delivery to Owning Chips

On November 11, 2026, `c83f046` closed Roadmap 2.1 under the explicitly approved documentation-and-delivery boundary (`REGISTER-ACCESS-CLOSE-20261111`). It followed direct CIA dispatch (`CIA-DIRECT-DISPATCH-20261111`), direct custom dispatch (`CUSTOM-DIRECT-DISPATCH-20261111`) and separation of private register application from engine execution. The completed contract identifies every modeled register, its access semantics, recipients, queue and logical owner; it does not certify every receiving engine's eventual effect.

Generic motherboard-address mutations were replaced with named readers and queued setters for thirty canonical CIA addresses and 180 custom write offsets. Shared writes preflight their recipients, and Agnus owns the queue for Copper/Blitter children. Register-module separation preserved all 221 existing method bodies and signatures. CPU CIA synchronization and acknowledgment remain distinct from falling-E capture and same-phase overlay routing. Pending bus, TOD and output state survives continuation; reset cancels pending work. Restore-time polling that consumed unrouted events was removed.

Effective Port-B output now respects DDRB and pull-ups after a regression reproduced unintended drive selection. Related corrections covered LOAD/unused-bit readback, TOD middle-write halt, pending ICR mask enable and one-shot timer-high loading. Palette/sync requests gained a positive provisional CCK delay; due events retain issue order, full queues reject writes without early application, rejected source outputs are retained and Paula replays all due writes rather than truncating the ninth. A merged interrupt-input destination remains separate from CPU INTREQ. The obsolete audio location setter/fixture and unused Denise inspection decoder were retired; Agnus retains audio pointer coverage. The user restored snapshot header version 1 for the evolving development format without an old-format decoder.

Motorola's E-clock description, vAmiga's provisional waits and the replacement FPGA project's CIA capture/counting behavior informed the model. Queue delays, mutation modes and issue order remain model settings rather than measured silicon timing. Nine inert custom offsets, unresolved CIA PRA/PRB pin reads and missing engine effects were explicitly handed to later work; Gary decoding and board signal transport were separate stages.

Historical delivery checks covered all 256 custom slots, all 180 writable offsets, word/both byte lanes, live/debug readers and PAL/NTSC. The final MemoryBus wiring suite passed 16 tests including 1,536 write transfers; preceding focused suites passed 168 tests before the last read-selection addition. CIA coverage included canonical addresses, five E phases, CPU retries, continuation and queue boundaries. The closing commit records passing commit/milestone profiles, 128/128 SingleStep groups with existing reviewed ASL.b corrections unchanged, strict Clippy, native/WASM checks, vault audits and independent approval of the exact staged snapshot. Static register/map checks covered no signal/quirk sentinels. Unknown hardware vocabulary was added only after explicit user approval. RAG retrieval was insufficient at that time, so targeted local references supplied evidence. No firmware, operating system, vAmigaTS execution or silicon calibration was claimed.

---

## 12. A500 Gary Address Decoding

On November 11, 2026, `e7d870e` closed Roadmap 2.2 (`MEM-GARY-01`). Original-code failures first reproduced missing CIA aliases, dual selection, opposite-parity writes, custom DE routing and physical bank classification. The repair addressed the decoder and production bus paths while retaining installed memory boundaries and the existing CCK wait model.

Banks A0-BF route through independent A12/A13 CIA selection using full canonical register identities. CIA-B supplies the high read lane and CIA-A the low lane; byte writes broadcast to selected chips regardless of parity, while word writes retain each chip's lane. DE and DF share custom live/debug/queued paths. Selected CIA receivers synchronize together, and CIA-A PA0 changes affect overlay only after commitment. Inactive read lanes retain FF and unmapped reads FFFF; Slow/Fast RAM, RTC precedence and physical DMA storage remain separate from device aliases.

By explicit user decision, the redundant CIA bus capacity preflight was removed while sixteen mutation slots and direct-setter overflow rejection remained. Named CCK constants express the existing wait schedule, an optional countdown was simplified and an obsolete CIA debug-byte helper was replaced by the production word-debug API. The source review distinguished canonical HRM addresses, A500 Rev. 6A/7 wiring and differing reference-emulator equations. Extra mirrors and measured timing calibration were deferred. Full CIA CPU-cycle timing, including the hardware 10-19-clock range, remained unverified in this CCK model and was handed to later signal/guest verification work.

Historical focused checks passed 47 CIA/MemoryBus tests and twelve production CIA/Gary integration tests across PAL/NTSC and all three memory presets, plus strict Clippy and machine WASM compilation. The user specifically authorized nine existing production-machine ADF/action regressions with a stop on errors; they passed, including disk DMA, while the floppy crate itself contained no tests. This exception did not authorize new media or firmware work. Static checks covered 196 offsets, twelve map boundaries and 26 crate relationships, with no signal/quirk sentinel coverage. The closing commit records passing commit/milestone profiles, 128/128 SingleStep groups and independent extended approval. Complete hardware timing and attached-device behavior were not established by decoder closure.

---

## 13. Receiver-Owned Signal Transport and Continuation

On November 11, 2026, `e3ca26c` closed Roadmap 2.3 (`SIGNAL-THREE-REFERENCE-20261111`). The stage established motherboard transport through captured receiver-owned inputs at explicit CCK phases. It preserved the boundary between delivering a signal, reconstructing the receiving engine and proving a complete guest-visible exchange.

Agnus retains beam-counter ownership and sends RGA synchronization events into receiver-owned state. Register contracts distinguish immediate address strobes from queued data writes; pending delivery remains serializable rather than depending on restore-time polling. Pure machine snapshots were separated from the higher-level save-file and preview format. CIA-B PA0/SP and PA1/CNT wiring was completed in both directions after a regression reproduced the missing reverse route. Obsolete direct SDR-byte injection and unused pin helpers were retired with their callers.

Twenty focused machine transport and pin-propagation tests passed across PAL and NTSC. The recorded checks also passed workspace all-target Clippy with denied warnings, machine_loop/save_state WASM builds, configured semantic checks and a 35-note vault audit with 999 links before closure edits. Thirteen baseline RAG contracts passed in isolation from a separately staged adapter repair. The closing commit records passing commit/milestone profiles, 1,002,737 SingleStep executions with two existing reviewed overrides and independent extended approval of the exact committed snapshot. Those CPU vectors do not certify electrical pins or complete peripheral exchanges.

Device-engine reconstruction remained in 2.4 and functional guest verification in 2.5. Serial and parallel ports remained disconnected; deterministic pin tests did not prove communication with attached devices. Firmware, media execution, vAmigaTS and measured silicon timing remained outside this closure. The separately closed timing research was not reopened. Chapter 14 records the later decision to remove a separate pre-reconstruction propagation gate; transport closure did not establish that proposed audit's result.

---

## 14. SDLC, Agent Workflows and Development Harness

Between September 12 and November 12, 2026, the software development life cycle (SDLC) became an engineering track of its own: rules and skills, agent configuration, reference retrieval, isolated worktrees, test orchestration and protected evidence evolved alongside the emulator. The lasting result was explicit ownership and review of both implementation and its verification criteria. These tooling deliveries did not close chipset behavior milestones. This chapter brings their settled history together; linked rules and tool documentation own the current procedures.

### From Implicit Guidance to Owned Rules and Skills

September work split shared instructions into focused rules (`8ea3676`), introduced reproducible-defect investigation and the Platform Quirks Catalog (`3781b8f`), and added code, hardware and documentation audits. Compiler warnings became fatal, while rules that forced artificial encapsulation around hardware state were removed. These mechanisms made some invariants executable; passing their static checks never established all hardware behavior. Human-readable design and semantic review remained necessary.

On October 5, `da87753` migrated agent roles and MCP registration to native Codex TOML. It removed hook wrappers, installation guidance and obsolete catalogs while retaining explicit verification commands. `66f642d` then clarified ownership: AGENTS.md routes shared obligations, rules state requirements and exceptions, skills own procedures, and scripts/tests own mechanical checks. Standalone HTML/PDF conversion programs replaced duplicated agent procedures. Scope control distinguished an inspection from permission to edit and a useful suggestion from an authorized feature. Closed execution plans became disposable after their durable closure record was committed, without deleting unrelated or unfinished work.

On November 8, `99c8e39` consolidated scoped specification compliance and independent review for every authorized commit, including documentation and tooling. The intended source snapshot, verification evidence and review verdict had to agree. Accurate specifications stayed unchanged; compatible implementation detail could be enriched, while conflicting requirements and protected references required user decisions. Obsolete instruction/CPU/test-runner skills were retired rather than retained as alternative command paths. HTML attachment tests moved to their owning tool and verification routing. Approved frontmatter repairs restored metadata in 66 ignored reference chapters per checkout without changing chapter text; metadata presence did not establish source fidelity.

### A Single Verification Harness and Explicit Evidence Boundaries

The first preflight script (`b334e13`, September 12) and the later tools/harness layout (`3a85b76`, September 15) combined compilation, architecture and quality checks. October 5 work removed mechanical source/test coupling and assertion quotas: a changed file did not by itself justify mirrored tests. October 9 verification work introduced unconditional standard checks, distinct commit/milestone selections, independent diagnostic sets and execution before staging. On October 10, `8280791` centralized the implementation in tools/verification, with shared utilities under tools and architecture checks in Python rather than the old Rust architecture suite.

The resulting commit profile covered ordinary crate tests, coverage and standard quality/tool contracts; milestone verification selected full SingleStep rather than treating every routine commit as a full CPU run. Focused tests served development. The old vAmigaTS capture/execution infrastructure, chipset throughput workload and obsolete benchmark traces/hashes were removed; their recorded results were historical, and native external-suite execution required later firmware/media bring-up. Source comparison, software-generated vectors, synthetic bus tests and measured hardware each had different evidentiary limits. Chapter 5 records the CPU comparison and continuation decisions; Chapter 6 records the replacement performance protocol.

The October 10 final commit profile passed its selected Rust, quality, language and five Python contract suites; full SingleStep, new performance measurements and the milestone profile were not selected. The October 27 divergent bootstrap merge initially failed English, then passed the full selected profile after explicit vocabulary/literal decisions and a prompt correction. Those later passes did not retroactively turn earlier failed converter or milestone checks into passes. The November 8 workflow delivery records a passing full commit profile and independent extended approval, with no roadmap milestone closed.

### Environment Isolation, References and Retrieval

Worktree automation initially linked ignored assets through NTFS junctions (`acb25d7`, September 14). Shared mutable paths risked cross-checkout contamination; `f4eb29e` replaced them with physical directories/copies on September 17. Graphify state was later seeded into worktrees, and November work included pinned FPGA references. Fast-forward reintegration became the direct path in `5cbbc02` on October 14; divergent branches still needed conflict reconciliation and verification. Routine commit evidence subsequently favored native Git and original logs over extra helper scripts and copied manifests.

September 16-22 retrieval work combined local documentation search with searchable diagram sidecars, then made the standalone CLI the indexing authority. Design and Reference had explicit scopes, .obsidian was excluded, and external DevNotes material stayed retrieval-only. The MCP adapter delegated rather than implementing another indexer; its reindex entry point was removed. October work separated service-owned read-only retrieval configuration from indexing state. Image descriptions belonged to conversion/publication rather than being regenerated for each query.

Pinned MiniMig and Amiga Replacement Project sources and measured Techtravels evidence were added in `8ba6def` on October 28, replacing the duplicate ignored vAmiga tree with one canonical source. The research distinction was deliberate: measured signals, reconstructed FPGA circuitry and emulator models could inform one another but were not interchangeable proof. On November 11, `3a88406` repaired shared-reference retrieval across checkouts and explicitly decoded CLI output as UTF-8. Fifteen adapter/launch contracts passed, followed by recorded commit verification and independent review. A shared index hit still needed source/path filtering; process success or MCP configuration alone did not prove retrieval quality or session activation.

### Protected Numerical References and Named Hardware Meaning

The BLEP part of `99c8e39` on November 8 made numerical regression independent of optional WinUAE chart generation. Offline CSV tests protected the retained curves; moving eight numerical tests out of production source preserved their bodies and four production implementations. Four CSV hashes and the committed SVG stayed unchanged, and a one-time WinUAE comparison regenerated identical plotted paths using an explicit source input. Focused BLEP checks and all-target Clippy passed. This protected numerical evidence without establishing physical speaker output or measured board audio.

On November 12, `d2a9f6d` required named REGISTER_ identities and BIT_/MASK_ encodings, and explicit approval for architecture-check changes or weakened verification gates. Implementation `2957128`, split from policy at user request, migrated unnamed encodings and their callers without changing runtime behavior. Shared disk/interrupt definitions used two approved configuration dependencies; four semantic-parser/source-selection changes and identifier corrections were also explicitly approved. The static audit covered 196 register rows, twelve memory boundaries and 27 crate names, with no signal/quirk sentinels. The first implementation profile failed English, followed by a passing focused scan after approved corrections; full SingleStep was not run for this refactor. Naming clarity and static agreement did not substitute for behavior tests or permit silent baseline changes.

### Simplifying Guidance Without Claiming Proven Equivalence

November 10 changes `c1952f5` and `65da673` shortened approved Rust, API, documentation, testing and PowerShell guidance and simplified commit/review reporting. Git identified the source snapshot; existing tools and original logs supplied evidence. Verification and review could overlap only where their results still applied to the same final source. Mandatory gates, independent approval and limits of CPU observations remained.

Three screening rounds supported individual instruction replacements, not overall behavioral equivalence. The third round recorded nineteen correct results and one preserved host-fixture arithmetic failure; six approved replacements matched, but combined behavioral validation was not performed. Local links and whitespace passed. These were bounded screening results, not proof that shorter instructions reduced cost or preserved every possible agent behavior.

### Roadmap Ownership and the Reconstruction Verification Order

Roadmap consolidation `9206fae` on November 11 moved diagnostics, remaining implementation and acceptance evidence into their owning stages. Specifications retained hardware contracts and concise caveats rather than duplicate task lists. The review passed 35 design notes, 832 links and changed-document roadmap anchors without executable changes or milestone closure. The resulting scope separated 2.4 engine reconstruction, 2.5 guest/propagation acceptance, later firmware/media/native suites, and deferred peripheral/bus work.

On November 12, `0dc9126` scheduled REGISTER-PROPAGATION-VERIFY-20261111 as a separate gate before reconstruction, while BOOK-GUEST-PROGRAM-TESTS-20261111 remained after it. The same change excluded local .agent/tasks plans from language scans, including explicit-file scans; 87 verification-tool tests and the repository English scan passed. The scanner change did not execute either chipset task. The user then approved removing that separate propagation prerequisite in `0969545`: component unit tests accompany reconstruction in 2.4, and functional propagation/guest acceptance stays in 2.5. Access/Gary/signal foundations remained established, but no removed gate was declared passed and no new hardware milestone was closed.

### Logging, Compaction and the Permanent Record

The diary appender arrived in `12eb591` on September 12. Policy briefly limited entries to verified roadmap/milestone closures (`a1e3fc3`, September 25); `4a986fc` restored concise per-commit entries on October 5, with compaction at verified roadmap closures. The November 12 change `0969545` added completed-stage chapters and a stable, final unnumbered Rolling Changelog. Its commit profile and focused diary/roadmap/skill checks passed, with independent approval; it closed no implementation stage.

The later editorial decision was to use that changelog as a buffer for work awaiting resolution or narrative integration. Durable decisions and evidence move into the owning chapter, including this SDLC history; routine mechanical changes, merges and compaction statistics remain in Git. Existing digests have no automatic right to permanent retention, and mentioning an open roadmap stage does not make a settled tooling change active. Granular investigation stays only while needed to continue unfinished work. Historical failures, user-directed scope closures and unresolved limits remain visible in the relevant narrative even after their original entries are removed.

---

## 15. Book-Derived Custom Chipset Reconstruction

On November 12-13, 2026, CHIPSET-BOOK-REBUILD-20261111 replaced the chipset engines under the established register, signal and CCK interfaces. The work retained the CPU, storage boundary, delayed register queues and captured connector transport. Agnus owns Copper, Blitter and DMA as internal modules; Denise owns sprites; Paula owns audio and interrupts; game_ports owns mouse and joystick logic. Eight separate child crates were removed together with their old imports and verification selections. Explicitly approved checker changes moved protection to the owning crates without adding dependency permissions.

The reconstruction made arbitration drive actual transfers. Agnus retains requests and granted data until their recipients accept them, advances the owning pointers, and reports shared-bus refusal without advancing the CPU transaction. Copper fetches and Blitter area/line operations progress under those grants. Audio separates programmed reload values, active sample state and memory addresses; disk words pass through Paula's input decoder, FIFO and serializer instead of a decoded drive-status bridge. The approved Paula model receives all four RGA synchronization addresses, preserves STRLONG phase, and derives VERTB from entry into STREQU. It replaces the old decoded vertical event while preserving receiver-owned state.

Denise reconstructs bitplane, palette, sprite and collision behavior and assembles PAL/NTSC fields, including alternating interlaced rows. The existing GUI uploads that frame buffer. The directly loaded live_stripes ASM/BIN fixture produced recognizable red/green stripes in a fresh headless GUI capture after two completed fields. CIA timers, TOD, serial and port handshakes, the keyboard protocol, digital game ports, JOYTEST and disconnected UART behavior have component coverage and current-format continuation checks. Analog inputs, attached external devices and host audio/input routing retain later owners.

Snapshot format 4 removes duplicate outer Agnus DMA registers and preserves active engine state, pending delivery and rendered pixels. The user approved replacing the old format without migration or deleting saved files. Completion cleanup removed unused helpers, corrected Clippy visibility findings and stale JOYTEST documentation, and added six approved dictionary terms. Three approved quality exceptions retain the sector encoder used to prepare DMA test input and read-only disk FIFO/UART output observations used by behavior and continuation tests. The isolated closure check initially found two unused GUI parameters on WASM. Target-specific parameter use restored the build without altering native file dialogs.

The closure review ran the commit and milestone profiles, native/WASM compilation, the live GUI capture and an independent extended review. The current SingleStep run completed 128 groups and 1,002,737 vectors. Static semantic checks covered 196 register offsets, twelve memory boundaries and nineteen crate mappings; they did not establish functional signal coverage. Component and production-path regressions establish the modeled transitions, not complete guest acceptance or measured silicon edge timing. BOOK-GUEST-PROGRAM-TESTS-20261111 retains the full register/engine inventory and independent guest/output checks in Roadmap 2.5. Firmware, media, external suites and physical timing calibration remain separate work.

---

## Rolling Changelog

This final, unnumbered section holds work awaiting resolution or integration into the numbered chapters. Historical results belong to their recorded source and scope. Settled history through November 12, 2026 has been integrated above; no unfinished investigation entries remain from that set.

### Entry Format

Every authorized repository commit adds a concise entry here. Logging and compaction follow [diary-maintenance.md](.agents/rules/diary-maintenance.md) and [compact-diary](.agents/skills/compact-diary/SKILL.md). Record the timestamp, affected components, delivered change, rationale and actual verification, including failures and unrun checks. At compaction, retain lasting decisions and evidence in the owning chapter and leave routine details in Git.
---

### [2026-11-13 00:08 CET] — Complete book-derived chipset reconstruction
- **Affected Subsystems**:
  - `chipset`
  - `peripherals`
  - `machine loop`
  - `GUI`
  - `save state`
  - `verification`
  - `design specifications`
- **What Was Changed (The Concrete Reality)**:
  - Close CHIPSET-BOOK-REBUILD-20261111 and Roadmap 2.4
  - Consolidate the completed stage in Chapter 15 and retain guest acceptance in 2.5
  - Repair the isolated WASM build
- **Architectural Rationale & Trade-Offs**:
  - Preserve register transport
  - hardware ownership and independent verification while replacing engines
- **Verification & Test Results**:
  - Previous final-source commit and milestone profiles passed, including 128 SingleStep groups and 1002737 current vectors
  - The isolated WASM check initially failed on two unused GUI parameters and passed after the repair
  - Prior live GUI inspection and extended source review passed
  - Renewed final-source profile and review outcomes are recorded in the closing commit
---

### [2026-11-13 00:19 CET] — Defer English audits to authorized commit workflows
- **Affected Subsystems**:
  - `agent rules`
  - `verification guidance`
- **What Was Changed (The Concrete Reality)**:
  - Run language and vocabulary audits only during authorized commits or explicit language work
  - Keep ordinary implementation and validation free of language scans and vocabulary decisions
- **Architectural Rationale & Trade-Offs**:
  - Keep language review at the user-approved workflow boundary
- **Verification & Test Results**:
  - The user reported testing and review complete
  - No tests or audits were rerun under the explicit commit-only instruction
---

### [2026-11-13 00:20 CET] — Align Amiga RAG commands with repository-relative indexing
- **Affected Subsystems**:
  - `RAG adapter`
  - `bootstrap`
  - `indexing guidance`
- **What Was Changed (The Concrete Reality)**:
  - Pass an explicit repository root for all three Amiga indexing scopes
  - Update bootstrap contract expectations and command examples
  - Describe actual Qdrant counts and shared worktree document identity
- **Architectural Rationale & Trade-Offs**:
  - Keep indexing paths stable across checkouts and distinguish stored points from cached state
- **Verification & Test Results**:
  - The user reported testing and review complete
  - No tests or audits were rerun under the explicit commit-only instruction
---

### [2026-11-13 00:20 CET] — Require fixed 256 KiB Kickstart images
- **Affected Subsystems**:
  - `physical memory`
  - `debugger`
  - `GUI`
  - `save state`
  - `design specifications`
- **What Was Changed (The Concrete Reality)**:
  - Keep the ROM buffer private and fixed at 256 KiB with mirrored addressing
  - Reject invalid ROM loads and snapshot buffers while preserving the active machine
  - Propagate load failures and keep visible GUI feedback
  - Update callers and regression fixtures
  - Record that snapshot backward compatibility is not required before the first release
- **Architectural Rationale & Trade-Offs**:
  - Enforce the selected A500 ROM capacity at loading and restoration boundaries
- **Verification & Test Results**:
  - The user reported testing and review complete
  - No tests or audits were rerun under the explicit commit-only instruction
---

### [2026-11-13 13:55 CET] — Normalize RGB444 output to the full host color range
- **Affected Subsystems**:
  - `frame_builder`
  - `Denise`
  - `machine and GUI video tests`
- **What Was Changed (The Concrete Reality)**:
  - Expand each four-bit channel by multiplication by 17 with opaque alpha
  - Keep independent expected colors in component, guest and texture tests
- **Architectural Rationale & Trade-Offs**:
  - Apply the approved full-range host normalization without adding gamma or CRT correction
- **Verification & Test Results**:
  - Focused frame_builder tests passed
  - Historical Kickstart color comparison and user visual acceptance are recorded in session 01a1d47e-011b-7980-8771-ca3c01af51f5
  - Broader display calibration remains pending
