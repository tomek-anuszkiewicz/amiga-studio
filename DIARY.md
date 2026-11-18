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
- **Reference Emulator Ingestion:** To prevent speculative design, five open-source reference emulators were imported into `ref_src/` for cross-validation: MAME, Moira, Musashi, vAmiga, and WinUAE.
- **Decoded Single-Step Vectors:** Over one million software-generated SingleStep cases were imported and structured (`a31518dc`), providing reference state and bus sequences before CPU implementation. Later work clarified that agreement with these vectors did not constitute a physical hardware capture.

### The RAG Knowledge Base & Reference Documentation Sprint (September 1–4, 2026)
Between September 1 and September 4, a massive documentation ingestion sprint took place across dozens of commits (`e82a4efc` through `e782dcf9`):
- Hundreds of pages from the **Amiga Hardware Reference Manual (HRM)**, **Motorola 68000 Programmer's Reference Manual (PRM)**, **Amiga Guru Book**, and **A500/A2000 Technical Reference Manual** were ingested and converted into clean, semantically structured Markdown.
- Vector SVGs and timing schematics were extracted from original PDFs, register layouts and bitmasks were normalized into tables, and Appendix B clock transition diagrams were recropped and cleaned.
- A local **Qdrant vector database** (`tools/rag/`) with a custom FastMCP server (`amiga_rag`) and **Graphify** AST knowledge graph (`graphify-out/`) were deployed.
- **The Core Strategy:** An AI agent prompted in a vacuum will hallucinate register addresses, flag behaviors, and bus cycle timings. By arming the agent with instant, offline vector retrieval over the official hardware manuals, the human made source evidence available for checking generated behavior. Retrieval alone did not establish that every instruction or bus cycle matched hardware.

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

### Host Performance: Locality, Allocation and Measurement

The early design preferred straightforward, predictable execution and preallocated
storage over opaque arithmetic tricks. Direct opcode dispatch and allocation-free
hot paths were chosen to keep host overhead visible and avoid allocator work during
emulation. Those choices are design intentions, not throughput measurements or a
guarantee of branch prediction, cache behavior or deterministic host latency.
The [optimization examples](.agents/skills/optimize-emulation/references/project-examples.md)
record later measurements and cases where active-work costs changed the conclusion.

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
1. **Initial Range Checking (`179fa5d2`):** Early iterations evaluated memory access via cascaded `if/else` checks matching 24-bit addresses against address boundaries (`$000000-$07FFFF`, `$DF0000-$DFFFFF`, `$F80000-$FFFFFF`). This placed range selection on every byte and word transfer.
2. **The $O(1)$ Bank Mapping Breakthrough (`4c799770`, `3b30e894`):** The human directed a complete redesign: the entire 16 MB 24-bit physical address space is sliced into **256 static banks of 64 KB each**.
   - A 256-entry lookup table (`[BankHandler; 256]`) maps each 64 KB block directly to function pointers: `read_byte`, `read_word`, `write_byte`, and `write_word`.
   - Accessing any address requires only a single byte shift (`address >> 16`) and an indirect call, providing direct bank selection. The selected handler still performs its own access checks; this description is not a measured host-latency result.
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

A dedicated engineering effort was devoted to modeling the physical analog sound output of the Amiga Paula chip using circuit-derived response curves:
- **WinUAE Parameterization & Circuit Derivation:** The Band-Limited Step (BLEP) synthesis engine was modeled and parameterized based on verified reference implementations in WinUAE.
- **Pure Mathematical Standalone Module:** Rather than hardcoding magic sound samples, the BLEP synthesis is implemented as an independent, decoupled module containing all the underlying physics and signal-processing mathematics (`crates/paula/src/blep_tables.rs`).
- **Analog RC Component Values:** The step response and frequency roll-off tables are computed directly from the physical component values of the Amiga motherboard circuit—the precise resistance (resistors in ohms) and capacitance (capacitors in farads) forming the analog low-pass filter network (including dynamic CIA-A LED filter switching).
- The resulting curves model the selected circuit assumptions; they do not by themselves establish physical output accuracy or eliminate every aliasing source.

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

### From "Make Me a GUI" to a Developer Studio

The first voice request produced a basic register window. Iterative review expanded
it into register and microcode inspection, disassembly, an Amiga image, memory
editing, breakpoints and an execution-history control. The important choice was to
make machine activity inspectable while retaining a separate presentation view;
current shortcuts and layouts belong to the living GUI documentation.

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

As the workspace grew, architectural decisions became explicit rules and executable checks: cohesive source files, flat CPU instruction ownership, no custom macros or const-generic instruction handlers, no runtime host panics, named hardware operations and allocation-free hot paths. Protected vectors, timing references and benchmark baselines could not be changed merely to obtain a passing test. The September 19-21 compiler work made warnings fatal and removed artificial encapsulation requirements that obscured plain hardware state.

The enforcement mechanisms changed with the project. Early Rust architecture tests, automatic coupling rules and merge-only guidance were later replaced or narrowed; their old lists are not the present contract. [Chapter 14](#14-sdlc-agent-workflows-and-development-harness) records that evolution and its evidence. [AGENTS.md](AGENTS.md), the owning rules and the verification README define the current obligations.

---

## 9. The Ultimate Goal: The Clean-Room Re-Generation Experiment

The design notes retain hardware contracts, consequential decisions and known limits; Git and this diary retain the development history.

This serves a larger, ambitious vision:
- Once the complete, working Amiga 500 emulator is finished and verified against all reference suites,
- We will wipe the implementation source code (`crates/`) and reference emulator sources (`ref_src/`), leaving strictly:
  - The refined prompt and skill system (`.agents/`).
  - The curated, de-duplicated design documentation (`Obsidian/Amiga/Design/`).
  - The automated test suites (SingleStepTests, vAmigaTS).
- **The Experiment:** Can an AI agent, armed with the lessons, rules, and architecture recorded during this project, autonomously re-synthesize the complete cycle-exact emulator from scratch?
- That was the original ambition, not an instruction to delete the working repository. The currently scheduled experiment is an isolated, single-subsystem trial in [Roadmap Step 9](ROADMAP.md#step-9-documentation-to-code-regeneration-experiment).

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

## 16. Native Kickstart and Disk Program Execution

On November 15, 2026, NATIVE-DISK-BRING-UP-20261115 supplied the missing autonomous
ADF read stream. The baseline Kickstart reached disk DMA but never the original
cross1 body within 30 emulated seconds. Focused reproductions also showed that
reset ejected mounted disks and that the sector helper lacked a leading preamble.
The repair prepares complete circular tracks at insertion, masks checksum data
positions and carries MFM clocks between longwords and sectors. An ideal 300 RPM
spindle emits 500,000 raw cells per second through the existing captured connector;
PAL/NTSC rational phase accumulation prevents rounding drift. Selection and side
changes preserve angular phase. The sampled INDEX and data pulses are CCK events,
not measured analog pulse widths or spindle acceleration.

The drive owns media and mechanics, Paula separates raw words and frames DMAL,
Agnus arbitrates and supplies addresses, and the machine commits words to Chip RAM.
No sector decoder or host OS substitute enters that execution path. Electronic
reset stops motors and cancels pending signals while retaining physical media,
protection and head position. Snapshot version 9 embeds usable raw tracks and
rotation state; serialized mid-transfer execution resumes with the same words,
completion CCK and future machine state. No old-format parser was added.

The original cross1 bootblock reads its payload to $70000. Kickstart reset,
overlay removal, production disk DMA, bootblock execution, original graphics.library
startup and the actual body were observed in order. Both clean runs reached the
loop at CCK 14,752,752. The independently identified Copper list at $70214 produced
its complete 83-color sequence repeatedly, including actual palette commitment,
over multiple full fields. A 30-second no-disk control lacked the program signature.
The machine test guide records exact input hashes, startup bytes, commands and
milestones. Focused drive and machine tests verify raw words on both surfaces,
track wrap, CPU contention, reset and serialized continuation.

The separately supplied Workbench 1.3 OFS disk executed its bootblock and DOS
startup. Its original scripts use StartupII, resident commands and LoadWB delay.
The named Workbench task appeared within the separate 60-second budget, but the
saved frame was visibly corrupted. Desktop visual acceptance and broader original
vAmigaTS image comparison remain with native verification; this single-case boot
neither closes that track nor establishes universal AmigaDOS support. The read
workloads needed no media writes, host input or invented companion disk.
Canonical closure gates and independent review are recorded by the closing commit;
the focused evidence above does not substitute for those gates.

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

### [2026-11-13 09:48 CET] — Distinguish math formatting from missing source references
- **Affected Subsystems**:
  - `Book Review source validation`
- **What Was Changed (The Concrete Reality)**:
  - Allow italic text and spacing commands without reporting added content
  - Report other math commands as missing source references
  - Add regression coverage and clarify the source audit documentation
  - Add the user-approved textit command to the English vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Separate source provenance from renderer syntax support while retaining checks on visible text and symbols
- **Verification & Test Results**:
  - The regression failed on the original validator
  - All 108 focused tests passed after the correction
  - The hardware manual audit removed 18 formatting findings and retained seven math commands plus 158 literal content findings
  - Review server HTTP checks passed with no pending render or error
---

### [2026-11-13 11:16 CET] — Save reference manual preparation artifacts
- **Affected Subsystems**:
  - `Book Review recipes`
  - `PDF preparation metadata`
- **What Was Changed (The Concrete Reality)**:
  - Save the current Hardware Reference Manual editor recipe with 2508 elements from 405 PDF pages
  - Record OCR preparation provenance for the M68000 programmer reference manual
- **Architectural Rationale & Trade-Offs**:
  - Preserve editable book preparation and matching source identity while source review remains unfinished
- **Verification & Test Results**:
  - Original and prepared PDF hashes match both manuals metadata
  - OCR profiles match their workspace configuration and page selections are within the 405-page and 646-page sources
  - The recipe matches Working and Accepted and loads all 80 graphics
  - Source audit resolves 60752 references but exits with code 1 for 158 literal content findings and seven math commands without source references
  - No new conversion or reconstruction run and no whole-book quality approval
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
---

### [2026-11-13 13:58 CET] — Define shared guest-program integration work for stage 2.5
- **Affected Subsystems**:
  - `roadmap`
- **What Was Changed (The Concrete Reality)**:
  - Plan a shared guest-program catalog with equivalent GUI and automated runs
  - Allow selected standalone vAmigaTS adaptations and bounded frame-count execution
  - Record the user-authorized correction of faulty tests and later documentation reconciliation
- **Architectural Rationale & Trade-Offs**:
  - Keep small integration programs independent of firmware startup while preserving evidence and native-suite boundaries
- **Verification & Test Results**:
  - Roadmap retention check passed
  - Reviewed original user decisions in session 01a1d211-1581-7222-955b-226b5213cc0c
  - Stage 2.5 remains pending
---

### [2026-11-13 14:03 CET] — Store the complete visible image without blanking samples
- **Affected Subsystems**:
  - `frame_builder`
  - `Denise`
  - `GUI`
  - `machine and save-state tests`
  - `GUI guidance`
- **What Was Changed (The Concrete Reality)**:
  - Retain visible border and playfield on a high-resolution spatial grid
  - Exclude horizontal and vertical blanking and incomplete edge lines
  - Preserve row tails, field parity and pending assembly in snapshots
  - Fit the complete image uniformly in the GUI
  - Add the user-approved framebuffer vocabulary term
- **Architectural Rationale & Trade-Offs**:
  - Map receiver coordinates to visible image rows without shifting or stretching the display
  - Retain current modeled blanking timing without claiming silicon calibration
- **Verification & Test Results**:
  - Focused frame_builder, Denise, machine_loop, save_state and GUI tests passed
  - Vault metadata and 1047 links passed
  - Fresh live_display GUI capture was rendered and visually inspected: complete centered image and proportional scaling
  - Historical ROM captures remain in the display-centering diagnostics
---

### [2026-11-13 14:10 CET] — Drive playback by video fields and expose shared Game controls
- **Affected Subsystems**:
  - `debugger session`
  - `GUI playback and file loading`
  - `browser host integration`
- **What Was Changed (The Concrete Reality)**:
  - Run bounded CCK slices to completed fields with boundary pause and frame stepping
  - Pace PAL and NTSC playback with speed controls and measured field FPS
  - Disconnect Game inspection and expose File, State and playback controls
  - Load ROM buffers through a shared desktop and browser path with visible source labels and errors
  - Restart instruction tracking after GUI execution edits and route native loading through the tested file entry points
  - Add user-approved API vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Keep host timing and file operations in the frontend while both views execute the same machine and share explicit loading controls
- **Verification & Test Results**:
  - Focused debugger and GUI tests passed
  - WASM package check passed
  - Fresh Studio and Game captures were visually inspected
  - Vault metadata and 1048 links passed
  - The execution-edit regression failed before the repair and passed for both PC editing and instruction patching after it
  - Native dialogs and browser picker interaction were not exercised in this commit workflow
---

### [2026-11-13 14:26 CET] — Keep the machine running while the CPU waits in STOP
- **Affected Subsystems**:
  - `debugger session`
  - `GUI playback`
- **What Was Changed (The Concrete Reality)**:
  - Continue CCK execution during CPU STOP in Developer Studio
  - Preserve field stepping, pause and HALT behavior
  - Resume breakpoint checks after interrupt entry
  - Explain the stopped CPU status
- **Architectural Rationale & Trade-Offs**:
  - CPU STOP suspends instruction execution while video
  - timers and interrupt delivery must continue
- **Verification & Test Results**:
  - STOP regression failed on the previous implementation
  - Six frame execution tests passed including CIA wakeup and breakpoints
  - GUI test confirmed advancing video and working Pause during STOP
  - Vault metadata and 1048 links passed
---

### [2026-11-13 14:31 CET] — Add independent Amiga image scaling
- **Affected Subsystems**:
  - `GUI screen rendering`
  - `GUI preferences`
  - `display specifications`
- **What Was Changed (The Concrete Reality)**:
  - Select Fit or fixed 0.5x, 1x, 2x and 4x image scales in both views
  - Use bilinear filtering for Fit and nearest sampling for fixed scales
  - Preserve physical pixel scale across DPI and interface zoom
  - Center and clip oversized images
  - Persist selection without exposing unfinished fields during filter changes
  - Apply approved texel vocabulary and pixels_per_point spelling
- **Architectural Rationale & Trade-Offs**:
  - Keep image presentation independent of emulated samples and interface scaling
- **Verification & Test Results**:
  - Scaling, interaction, persistence and execution-edit tests passed
  - WASM package check passed
  - Fresh Studio capture visually inspected with selector visible and proportional Fit image
  - Vault metadata and 1049 links passed
---

### [2026-11-13 14:43 CET] — Retain prepared debugger inspectors during continuous Run
- **Affected Subsystems**:
  - `debugger session`
  - `GUI inspection panels and controls`
- **What Was Changed (The Concrete Reality)**:
  - Keep prepared values and visible rows during Run and pending Pause
  - Separate live breakpoint checks from trace and history collection
  - Preserve diagnostics for explicit instruction, CCK and frame steps
  - Refresh on real stop, reset, loading and paused edits
  - Keep Game inspection disconnected through its return pause
  - Preserve execution-edit invalidation and apply approved program_counters spelling
- **Architectural Rationale & Trade-Offs**:
  - Remove repeated diagnostic work while retaining live execution controls and the Amiga image without snapshots or cached drawings
- **Verification & Test Results**:
  - 132 focused debugger and GUI tests passed
  - WASM check and vault audit passed
  - Fresh paused, running and stopped captures visually inspected
  - Same 60-field live_stripes release comparison measured Developer execution at 36.772 ms before and 25.042 ms after, excluding pacing waits
  - Fresh instrumentation recorded zero inspector refresh, disassembly, trace and history calls during steady Run and zero inspector drawing in Game
  - Measurements establish this local scenario, not a general FPS guarantee
---

### [2026-11-13 14:50 CET] — Plan display mode, color and border comparisons
- **Affected Subsystems**:
  - `ROADMAP display verification backlog`
- **What Was Changed (The Concrete Reality)**:
  - Add deferred resolution and interlace transition coverage
  - Plan palette, dual playfield, Extra Half-Brite and HAM6 comparisons
  - Include overscan and dynamic border decisions with WinUAE and physical Amiga references
- **Architectural Rationale & Trade-Offs**:
  - Record pending visual acceptance work after its guest execution prerequisites without treating model agreement as hardware evidence
- **Verification & Test Results**:
  - Roadmap structure check passed
  - Reviewed numbering and prerequisites against existing 2.5, 4.2.3 and 4.3 tasks
  - No implementation or milestone closure is claimed
---

### [2026-11-13 16:40 CET] — Reduce register queue work per Color Clock
- **Affected Subsystems**:
  - `config mutation queue`
  - `Agnus and its register writers`
  - `Denise`
  - `Paula`
  - `CIA`
  - `bus and snapshot tests`
  - `design specifications`
- **What Was Changed (The Concrete Reality)**:
  - Complete stage one of CORE-PERFORMANCE-PREP-20261113 with private occupied counts and reusable due buffers
  - Preserve capacities, register delays, issue ordering and receiver phases while migrating all consumers
  - Serialize pending entries only and discard due observations on clone or reset, with no old snapshot parser
  - Prepare the second-stage sprite, input, frame storage and audio candidates without implementing them
- **Architectural Rationale & Trade-Offs**:
  - Remove repeated scans and the initialization and copying of mostly empty fixed arrays without changing guest timing or allocating in the execution loop
- **Verification & Test Results**:
  - 317 affected component and integration tests passed
  - 28 complete PAL and NTSC machine-state checkpoints matched the starting implementation after normalizing only empty queue slots
  - Native workspace all-target and WASM checks, Clippy, formatting and architecture checks passed
  - Matched release measurements over 5000 frames per version reduced core frame time from 23.889839 to 10.017356 ms, a 58.07 percent reduction with pixel generation enabled and GUI work excluded
  - Separate profiles reduced queue tick share from 22.75 to 0.59 percent
  - Vault metadata and 1051 links passed
  - These measurements cover the recorded scenarios and do not establish a general FPS guarantee or hardware timing acceptance
---

### [2026-11-13 20:07 CET] — Reduce sprite evaluation work per pixel
- **Affected Subsystems**:
  - `Denise sprite engine`
  - `register application`
  - `machine and bus tests`
  - `sprite specification`
- **What Was Changed (The Concrete Reality)**:
  - Continue CORE-PERFORMANCE-PREP-20261113 with cached horizontal starts, attachment and collision enables
  - Skip inactive sprite pairs while completing disarmed serializers
  - Replace repeated color and pair collision decoding with small lookup tables
  - Rebuild derived state from canonical serialized values and migrate channel inspection callers
  - Retain the line buffer proposal and other performance candidates as future work
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated work in the pixel loop while preserving current register commitment
  - color priority
  - hidden collisions and snapshot continuation
- **Verification & Test Results**:
  - 228 focused Denise, machine, memory bus and save state tests passed
  - 138 full PAL and NTSC machine checkpoints matched the baseline byte for byte without masks, including framebuffer and serialized continuation
  - Native all-target and WASM workspace builds, warning-free Clippy, formatting and architecture checks passed
  - Matched release ABBA runs measured 3000 frames per version per scene after 300 warmup frames with rendering enabled and GUI excluded
  - Compared with the intermediate idle-sprite guard, throughput rose 4.70 percent for Kickstart, 3.63 percent for separated sprites, 4.12 percent for overlapping sprites and 4.89 percent for attached sprites with mid-line Copper writes
  - Active synthetic scenes used eight DMA channels over 200 lines, two bitplanes and a stopped CPU
  - Results preserve the existing model and do not establish new silicon timing or milestone acceptance
---

### [2026-11-13 20:41 CET] — Cache sprite line output and playfield collision conditions
- **Affected Subsystems**:
  - `crates/denise`
  - `crates/machine_loop/tests`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - CORE-PERFORMANCE-PREP-20261113: replace per-pixel sprite serialization with a lazily built 2 KiB line buffer shared by palette selection and both collision paths
  - Rebuild future output after applied sprite changes and retain consumption-time CLXDAT updates plus current shifter inspection and snapshot continuation
  - Precompute all 64 playfield collision masks when CLXCON changes and retain current palette and display priority
  - Add the explicitly approved computation and Python API terms to the English vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Move repeated sprite work to occupied word spans while preserving register delivery and observable hardware state
- **Verification & Test Results**:
  - Matched release ABBA runs against bbcbcdf9ad87e4105ffee26b33e5cea824a6be9d: 300 warmup frames and 3000 measured frames per version per workload
  - Whole-machine FPS: ROM 109.417 to 116.571 (+6.538%)
  - separated DMA sprites 121.745 to 125.830 (+3.355%)
  - overlapping 124.798 to 128.540 (+2.998%)
  - attached with mid-line writes 123.323 to 126.865 (+2.872%)
  - GUI excluded and active synthetic scenes use a stopped CPU
  - 138 full PAL/NTSC machine states and 67840 pixel samples across 4220 deterministic events match baseline bytes without exclusions
  - Independent review found an H=510 word wrapping into earlier cache entries; clipping to the anchored window fixes early collisions while preserving continuation, with a regression test failing before and passing after
  - All 231 focused Denise, machine-loop, memory-bus and save-state tests pass on the corrected source
  - Native release build and Wasm workspace check pass
  - vault audit passes for 35 specifications and 1054 links
  - Compatibility evidence preserves the existing timing model rather than measuring silicon
  - Raw measurements and chart: .agent/tmp/sprite-line-buffer/report.html
  - Other core performance candidates remain pending and this is not milestone closure
---

### [2026-11-13 21:25 CET] — Report local artifact storage after commits
- **Affected Subsystems**:
  - `tools/artifacts`
  - `verification tool contracts`
  - `commit and roadmap procedures`
- **What Was Changed (The Concrete Reality)**:
  - Add a read-only report of target and temporary artifact sizes with creation and modification date groups
  - Show cleanup commands bound to the measured target directory
  - Require a fresh report after commits and scoped temporary-directory recommendations at roadmap closure
  - Register artifact report tests in ordinary Python selection and commit verification
- **Architectural Rationale & Trade-Offs**:
  - Expose accumulated data across work periods while keeping cleanup under user control
  - Distinguish surviving file sizes from historical growth and actual use
- **Verification & Test Results**:
  - Artifact tests completed with four passes and one platform-dependent symbolic link skip
  - All 91 verification tool contracts passed
  - Full local artifact measurement and command help completed successfully
  - Vault audit passed for 35 specifications and 1055 links
  - Whitespace checks passed
---

### [2026-11-13 22:15 CET] — Select verification tests from changed packages
- **Affected Subsystems**:
  - `tools/verification`
  - `testing specifications and procedures`
- **What Was Changed (The Concrete Reality)**:
  - VERIFICATION-SELECTION-20261113: select changed Cargo packages and their transitive consumers
  - Skip Rust tests and coverage for independent documentation and known Python tool changes
  - Retain full Rust verification for shared build inputs, verification changes, specifications and unknown ownership
  - Keep all quality gates and the existing Python contract policy
  - Identify coverage scope and remove retired source files from report totals without deleting build caches
- **Architectural Rationale & Trade-Offs**:
  - The measured cached commit profile spent 78.59 of 88.08 seconds in coverage and standard harness tests
  - Reduce that work when it cannot be affected while preserving full Clippy and architecture enforcement
- **Verification & Test Results**:
  - All 103 verification tool contracts passed
  - All 18 architecture checks passed
  - Nine BLEP generator Rust tests and partial coverage passed in a 4.24 second pipeline
  - Vault audit passed for 35 specifications and 1056 links
  - Partial coverage exposed and now excludes 13 retired source files retained in cached objects
  - No full SingleStep or performance benchmarks were run
  - This is ordinary tooling work and does not close a roadmap milestone
---

### [2026-11-13 22:33 CET] — Use coverage gaps to guide behavioral tests
- **Affected Subsystems**:
  - `tools/verification`
  - `testing policy`
  - `code review`
- **What Was Changed (The Concrete Reality)**:
  - Add focused package and workspace coverage commands
  - export detailed LLVM data and source gap reports
  - require coverage analysis during emulator changes and review
  - stop report publication after failed coverage steps
- **Architectural Rationale & Trade-Offs**:
  - Make missing execution paths visible while requiring meaningful behavioral assertions and preserving measurement scope
- **Verification & Test Results**:
  - Verification tool tests and a focused blep_generator coverage run passed during development
  - skill and vault checks passed. Full emulator coverage audit was not performed.
---

### [2026-11-13 22:48 CET] — Automate language review during change rounds
- **Affected Subsystems**:
  - `.agents`
  - `tools/verification`
  - `AGENTS.md`
  - `Obsidian/Amiga/Design/Verification`
- **What Was Changed (The Concrete Reality)**:
  - Resolve clear language findings and maintain reviewed vocabulary during each change round
  - Block completion on unresolved findings and finish language corrections before final tests
  - Keep language data changes from expanding Rust test scope while retaining all quality gates
  - Cover standalone and mixed language-data changes with selection regressions
- **Architectural Rationale & Trade-Offs**:
  - The user delegated routine language decisions and approved the narrow test-selection exception
  - Keep genuine language problems blocking without unnecessary emulator test runs
- **Verification & Test Results**:
  - The new regression reproduced the original full-scope selection
  - 111 verification tool tests passed during development
  - Scoped language checks and the Design vault audit passed
  - Full SingleStep and milestone verification were not run
---

### [2026-11-13 23:12 CET] — Batch accelerated playback independently of presentation
- **Affected Subsystems**:
  - `GUI playback`
  - `presentation timing`
- **What Was Changed (The Concrete Reality)**:
  - Execute accelerated playback in complete-field batches with 50 Hz PAL and 60 Hz NTSC presentation
  - count every completed emulated field in FPS
  - preserve bounded slices and debugger stops
- **Architectural Rationale & Trade-Offs**:
  - Allow faster emulation without requiring a texture upload for every emulated field
- **Verification & Test Results**:
  - Scoped specification review agrees with the requested pacing and FPS behavior
  - Design vault audit passed for 35 specifications and 1057 links
  - whitespace checks passed
  - commit verification and independent review are recorded in the commit message
---

### [2026-11-13 23:14 CET] — Remember binary and ROM file locations
- **Affected Subsystems**:
  - `GUI file actions`
  - `desktop persistence`
  - `browser file references`
- **What Was Changed (The Concrete Reality)**:
  - Keep separate last-used folders and five recent files per category
  - reload native files with saved binary options
  - persist browser names and handles in IndexedDB and open recent folders through explicit selection
- **Architectural Rationale & Trade-Offs**:
  - Keep host file references outside machine snapshots and avoid caching file contents
- **Verification & Test Results**:
  - Reviewed successful-load history and failure paths against the approved GUI contract
  - native integration and browser persistence fixtures are included
  - commit verification and independent review are recorded in the commit message
---

### [2026-11-13 23:14 CET] — Add an optimization workflow and mixed guest workload
- **Affected Subsystems**:
  - `Agent performance workflow`
  - `machine integration fixtures`
  - `GUI inspector`
- **What Was Changed (The Concrete Reality)**:
  - Add optimize-emulation guidance and evidence-backed examples
  - deliver a deterministic standalone guest exercising CPU and chipset activity
  - add separate audit and benchmark modes with deterministic input and audio consumption
  - expose the same guest through the GUI inspector
  - Accept established profiling and audio terms in the reviewed programming vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Measure repeated host work while preserving guest timing and compare fixed workloads without imposing a timing threshold
- **Verification & Test Results**:
  - Skill validation passed
  - source review confirms the runner uses ordinary machine stepping and keeps observation outside timed runs
  - MIXED-GAME-SCENE-20261113 delivers the scene requested with this skill
  - visual fidelity and hardware comparison remain unverified
  - commit verification and independent review are recorded in the commit message
---

### [2026-11-14 00:34 CET] — Skip unchanged game-port counter work
- **Affected Subsystems**:
  - `Denise`
  - `game ports`
  - `device regression tests`
- **What Was Changed (The Concrete Reality)**:
  - Decode direction pins through a 16-entry phase table and skip unchanged JOY counter updates
  - add ordinary inline hints to four small device and pin paths
  - cover held inputs after JOYTEST, wrapping, reversal, cloning and reset
  - document the unchanged CCK sampling contract
- **Architectural Rationale & Trade-Offs**:
  - Reuse counter low bits as the retained phase without adding snapshot state
  - CORE-PERFORMANCE-PREP-20261113 device optimization preserves input timing, while host batching and keyboard scheduling remain future work
- **Verification & Test Results**:
  - The six source and test files match the frozen measured version after line-ending normalization
  - earlier validation passed 238 focused tests and native, Wasm, Clippy, formatting and architecture checks
  - 112 complete checkpoints and 300000 per-CCK samples matched the previous model
  - native ABBA throughput improved by 2.378 percent for ROM, 6.686 percent for idle devices and 7.141 percent for active input batches
  - these measurements exclude GUI and host latency
  - current commit verification and independent review are recorded in the commit message
---

### [2026-11-14 02:24 CET] — Write display pixels directly to fixed-stride rows
- **Affected Subsystems**:
  - `crates/denise`
  - `crates/frame_builder`
  - `crates/gui`
  - `crates/save_state`
  - `mixed_scene`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - CORE-PERFORMANCE-PREP-20261113: finish row-address mapping with a fixed 840-pixel stride and paired Denise output
  - Use a compile-time RGB444 lookup and clear only invalid row tails
  - Update texture upload and PNG previews to consume visible row prefixes and advance the internal save format to version 5
  - Extend the mixed guest scene and regressions for mixed resolution and row boundaries
  - Preserve border color through horizontal blanking without injecting a black pair into visible storage
  - Retain optional image-filter evaluation in Roadmap 6.1
- **Architectural Rationale & Trade-Offs**:
  - Preserve visible geometry and pipeline order while avoiding row relocation and repeated pixel work
  - Shared-memory presentation and host input delivery remain separate planned tasks
- **Verification & Test Results**:
  - Reviewed the prior frame-direct-apply verification record: component coverage and machine/display/GUI checks succeeded
  - git diff --check passes for the combined candidate
  - The initial combined profile found a snapshot assertion retaining the former black edge; the user approved red pixels at x=0 and x=1 while preserving continuation and geometry assertions
  - This batch will receive one combined commit profile and independent review before three commits as requested
  - Intermediate revisions are not separately tested and no milestone or measured silicon timing is claimed
---

### [2026-11-14 02:24 CET] — Generate a deterministic stereo BLEP stream
- **Affected Subsystems**:
  - `crates/paula`
  - `crates/machine_loop`
  - `tools/blep_generator`
  - `audio callers and snapshots`
- **What Was Changed (The Concrete Reality)**:
  - BLEP-AUDIO-STREAM-20261114: replace per-CCK output with 44100 Hz stereo PCM using timed steps and PAL/NTSC correction tables
  - Preserve channel DMA and interrupt timing while routing CIA-A filter selection and exposing buffer draining
  - Retain unread PCM and synthesis history across snapshots with a 1024-pair overwrite ring
  - Move generated runtime tables into Paula while preserving accepted PAL CSV references
  - Register read_audio_samples as an intentional host I/O boundary in the existing code-quality symbol set with explicit user approval
  - Close the superseded audio-output performance investigation without retaining its exploratory variants or claiming a speedup
  - Finish the original CORE-PERFORMANCE-PREP-20261113 candidates with the approved replacement audio contract
- **Architectural Rationale & Trade-Offs**:
  - Provide deterministic buffered output for future host playback without host audio I/O in the core
  - The old per-CCK preservation experiment was superseded by the approved output contract
  - Further queue overflow investigation remains unscheduled and host input work retains its separate plan
- **Verification & Test Results**:
  - Reviewed the prior BLEP coverage log and focused verification record with successful component and integration checks
  - The initial combined code-quality gate rejected the host drain API as test-only; the approved single-symbol exception retains this API without adding a playback backend
  - Final combined verification and independent review are recorded in the commit messages
  - Host playback and analog filter-switch transient calibration remain deferred and no milestone is closed
---

### [2026-11-14 02:24 CET] — Verify DMA audio spectra and attachment output
- **Affected Subsystems**:
  - `crates/machine_loop/tests/audio_spectrum`
  - `crates/machine_loop/tests/test_audio_spectrum.rs`
  - `reviewed English vocabulary`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - AUDIO-SPECTRUM-20261114: capture PAL/NTSC DMA output as WAV and compare tones plus DAC images against independent frequency-domain expectations
  - AUDIO-ATTACHMENT-20261114: verify looping volume and period modulation with a silent source side and return to the initial plateau
  - Retain negative controls and original amplitude frequency residual alias and stereo tolerances
  - Add reviewed Hann DFT RMS WAV SciPy and powi vocabulary
  - Keep enhanced reconstruction experiments deferred until games and host music playback are available
- **Architectural Rationale & Trade-Offs**:
  - Exercise Chip RAM DMA and reconstructed output together without using runtime tables as the spectral oracle
  - Settled modulation checks complement existing transition tests and do not calibrate hardware edges
- **Verification & Test Results**:
  - The task records report four PAL/NTSC filter combinations and both attachment scenarios passing with WAV decoding and negative controls
  - Fresh combined commit verification and review are required before creating this batch
  - This closes the two output-test tasks without claiming host-device playback or Roadmap 2.5 completion
---

### [2026-11-14 03:56 CET] — Prepare and verify changes as one commit batch
- **Affected Subsystems**:
  - `Agent workflow`
  - `verification tooling`
  - `testing specifications`
- **What Was Changed (The Concrete Reality)**:
  - FAST-TURNS-BATCH-COMMITS-20261114: defer recurring audits and maintenance to batch preparation
  - run preliminary gates before expensive suites
  - retain matching successful verification groups for pending changes against HEAD
  - remove manual base, resume and editorial options
  - exclude only the changing agent tool-call identifier from execution identity while retaining build and runtime settings
  - prepare diary entries and commit partitions before freezing the verified content
- **Architectural Rationale & Trade-Offs**:
  - Avoid repeated work after independent corrections while preserving required gates and artifact integrity
  - document changes conservatively invalidate execution results
  - one prepared batch can produce several atomic commits without repeating verification between them
- **Verification & Test Results**:
  - 130 focused Python tests passed
  - a focused regression preserved execution identity across agent calls while RUSTFLAGS changes still invalidated it
  - separate real Cargo probe invocations reused English and coverage results with zero Cargo commands on the second invocation
  - a real Cargo LLVM and English probe reused coverage after unchanged retries and vocabulary corrections, and reran it after source or document changes, report corruption and a new HEAD
  - these fixtures do not establish emulator performance or milestone completion
  - independent task review approved the implementation
  - preparation resolved one vocabulary finding and interrupted an incomplete profile to correct the agent-call identity issue
  - final canonical verification and commit review are recorded in the commit message
---

### [2026-11-14 14:40 CET] — reuse immutable channel phase sequences
- **Affected Subsystems**:
  - `agnus`
- **What Was Changed (The Concrete Reality)**:
  - Use immutable channel/fill phase tables
  - reuse the selected execution phase and retain startup, waits and destination drain tests
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated phase construction while preserving existing bus phases
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — skip timer work between relevant clock events
- **Affected Subsystems**:
  - `cia`
- **What Was Changed (The Concrete Reality)**:
  - Skip timer work without a clock edge or pending pulse
  - avoid resolving Port B when timer outputs are hidden
  - retain pulse and edge regression tests
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated idle work without stopping hidden timer waveforms or changing clock sources
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — simplify fixed DMA slots and sprite requests
- **Affected Subsystems**:
  - `agnus`
- **What Was Changed (The Concrete Reality)**:
  - Use a fixed slot table and direct enable checks
  - share sprite output availability and simplify start/stop comparisons
  - retain arbitration and sprite boundary tests
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated scheduling arithmetic while preserving grant order and pending outputs
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — use constant disk clock divisors
- **Affected Subsystems**:
  - `paula`
- **What Was Changed (The Concrete Reality)**:
  - Select constant fast/slow disk cell divisors
  - verify phase continuity when FAST changes
- **Architectural Rationale & Trade-Offs**:
  - Retain the local arithmetic simplification
  - whole-machine gain remains inconclusive in the earlier audit
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — skip stable idle UART work
- **Affected Subsystems**:
  - `paula`
- **What Was Changed (The Concrete Reality)**:
  - Return early for stable idle transmit/receive state
  - retain tests for status and transition boundaries
- **Architectural Rationale & Trade-Offs**:
  - Avoid work when counters and status cannot change
  - whole-machine gain remains inconclusive in the earlier audit
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — reuse display decoding and skip blanked composition
- **Affected Subsystems**:
  - `denise`
  - `machine_loop`
  - `save_state`
- **What Was Changed (The Concrete Reality)**:
  - Derive display mode fields from BPLCON0
  - use dual-playfield lookup tables
  - preserve serializers and collisions while bypassing blanked color composition
  - update callers and snapshot tests
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated pixel decoding with one authoritative raw register value
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — give Denise explicit framebuffer coordinates
- **Affected Subsystems**:
  - `denise`
  - `frame_builder`
  - `machine_loop`
  - `gui`
  - `save_state`
- **What Was Changed (The Concrete Reality)**:
  - Move timing-to-image coordinates into the Denise frame cursor
  - pass final positions to framebuffer storage
  - relocate rendering tests and adapt GUI fixtures
  - persist the cursor in snapshot format 6
- **Architectural Rationale & Trade-Offs**:
  - User explicitly retains this simplification even without a demonstrated speed gain
  - host image dimensions and scaling stay unchanged
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:40 CET] — retain boundary checks after rejected experiments
- **Affected Subsystems**:
  - `machine_loop`
  - `paula`
  - `rtc`
- **What Was Changed (The Concrete Reality)**:
  - Keep audio output, signal transport and RTC boundary tests using the restored interfaces
  - remove the pending signal suppression, audio mixer cache and deferred CCK RTC experiments before publication
- **Architectural Rationale & Trade-Offs**:
  - The captured audit found repeatable regressions for all three experiments
  - host UTC plus an independent persisted offset outside save states remains a local second-round proposal and is not implemented
- **Verification & Test Results**:
  - The combined candidate passed ordinary Rust tests, the standard harness and canonical commit verification before this diary partition
  - changed executable paths were reviewed against coverage
  - individual commit boundaries were not tested separately
  - no new timing measurement of the final combination
---

### [2026-11-14 14:55 CET] — Commit omitted optimization skill updates
- **Affected Subsystems**:
  - `.agents/skills/optimize-emulation`
- **What Was Changed (The Concrete Reality)**:
  - Include the pending optimization skill and project example updates omitted from earlier commits
- **Architectural Rationale & Trade-Offs**:
  - Complete the previously reviewed work with the remaining documentation files
- **Verification & Test Results**:
  - The user reports that verification was completed in another session. Tests
  - verification profiles and review were not repeated at the explicit user request.
---

### [2026-11-14 15:12 CET] — Reuse verification after diary edits and interrupted runs
- **Affected Subsystems**:
  - `tools/verification`
  - `verification specifications`
- **What Was Changed (The Concrete Reality)**:
  - Keep successful independent checks after root diary edits while refreshing the English gate and final candidate verdict
  - Historically resumed interrupted runs in the same results directory after descendant ownership checks; those checks are unavailable after VERIFICATION-LOCK-SIMPLIFICATION-20261116
  - Add reviewed API names and valid terminology reported by the language gate, including terms in the previously committed optimization guide
- **Architectural Rationale & Trade-Offs**:
  - A diary-only commit partition repeated emulator tests despite unchanged code
  - abandoned lock files also discarded useful verification history
- **Verification & Test Results**:
  - 137 verification tool tests passed
  - a real Cargo and LLVM probe executed three Cargo commands initially, zero after a diary edit and three after a source edit
  - this bounded probe is not a full emulator benchmark
---

### [2026-11-14 15:12 CET] — Partition authorized commits automatically
- **Affected Subsystems**:
  - `.agents/rules/git-commits.md`
  - `docs/ai_agents.md`
- **What Was Changed (The Concrete Reality)**:
  - Delegate coherent commit grouping to the agent for ordinary commit requests and roadmap closure
  - inspect session provenance and changes, preserve explicit scope and report commit counts and IDs
  - finish diary entries and cheap partition checks before expensive verification
- **Architectural Rationale & Trade-Offs**:
  - Users should not need a separate request to split independent concerns or avoid repeated verification caused by late preparation
- **Verification & Test Results**:
  - Reviewed ordinary single-change, multiple-session, shared-file and explicit-partition cases against the batch lifecycle
  - commit timing and push authorization remain user controlled
---

### [2026-11-14 19:16 CET] — Avoid mutation queue self-copies
- **Affected Subsystems**:
  - `config`
  - `machine_loop`
- **What Was Changed (The Concrete Reality)**:
  - Update only remaining delay while an entry retains its original queue position
  - copy complete entries only after a compaction hole
- **Architectural Rationale & Trade-Offs**:
  - Preserve due order, capacity, positive delays and snapshot behavior while reducing repeated data movement
- **Verification & Test Results**:
  - Existing config and machine tests passed: 182 tests across 41 targets
  - occupied focused workloads improved about 17-21 percent
  - full-state and framebuffer comparisons passed
---

### [2026-11-14 19:16 CET] — Skip idle mouse-axis arithmetic
- **Affected Subsystems**:
  - `game_ports`
  - `machine_loop`
- **What Was Changed (The Concrete Reality)**:
  - Advance pending mouse motion only on active axes
  - retain unconditional phase normalization and add motion, saturation and restored-phase regressions
- **Architectural Rationale & Trade-Offs**:
  - Keep connector sampling and active motion on every CCK without calculating zero movement
- **Verification & Test Results**:
  - 169 tests across 38 targets passed
  - focused idle, one-axis and two-axis cases improved
  - whole-machine impact was mostly below measurement spread
---

### [2026-11-14 19:16 CET] — Retain derived bitplane masks and pixel decoder selection
- **Affected Subsystems**:
  - `denise`
  - `machine_loop`
- **What Was Changed (The Concrete Reality)**:
  - Derive the plane mask and pixel decoder from effective BPLCON0 writes and deserialization
  - keep raw-word snapshot storage and cover zero planes, BPU=7 and mode combinations
  - remove the obsolete plane-count getter and cache, with register and pixel-output assertions protecting the same behavior
- **Architectural Rationale & Trade-Offs**:
  - Remove repeated mode decoding while preserving HAM history, palette selection and both pixel halves
- **Verification & Test Results**:
  - 211 tests across 44 targets passed
  - the original matched trial reduced mixed PAL time 2.47 percent and NTSC 1.61 percent
  - complete states and framebuffers matched
---

### [2026-11-14 19:16 CET] — Retain collision decoding on identical CLXCON writes
- **Affected Subsystems**:
  - `denise`
  - `machine_loop`
- **What Was Changed (The Concrete Reality)**:
  - Skip table reconstruction for identical effective CLXCON values
  - preserve register commitment, accumulated collisions and live-read clearing
  - add repeated and changed write coverage
- **Architectural Rationale & Trade-Offs**:
  - Avoid repeated construction of the same derived table without moving collision side effects
- **Verification & Test Results**:
  - 212 tests across 44 targets passed
  - repeated-write focused time improved about 83 percent
  - complete machine states and framebuffers matched
---

### [2026-11-14 19:16 CET] — Follow host UTC independently of machine snapshots
- **Affected Subsystems**:
  - `rtc`
  - `gui`
  - `machine_loop`
  - `save_state`
  - `optimization documentation`
- **What Was Changed (The Concrete Reality)**:
  - Use supplied host UTC plus an independently persisted battery offset
  - exclude live RTC state from snapshot format 7
  - preserve HOLD, STOP, calendar writes and reset behavior
  - correct the calendar year estimate at February and leap-day boundaries
  - isolate host timestamp conversion and correct negative fractional seconds to match the Wasm floor convention
  - cover 12-hour guest calendar writes, battery restart and native epoch-boundary conversion
  - close EMULATION-OPTIMIZATION-ROUND-02-20261114 implementation and the requested rejected-candidate repeat
  - mark retired RTC and audio examples as historical
- **Architectural Rationale & Trade-Offs**:
  - The user explicitly retained RTC as a functional change despite the measured standalone NTSC cost
  - PAL is the primary standard for the requested repeat
  - no rejected optimization was reintroduced and additional general profiling was cancelled
- **Verification & Test Results**:
  - 315 RTC, bus, machine, save-state and GUI tests passed in 58 targets after reproducing and repairing the calendar underflow
  - native and Wasm production builds passed
  - final combined core time fell 4.09 percent PAL, 4.41 percent NTSC and 3.66 percent ROM versus preserved B0
  - native GUI time fell 2.83 percent PAL and 3.99 percent NTSC
  - every validated GUI field matched its paired core
  - 408 repeated machine series and 160 focused series preserved state/output
  - canonical batch verification and final independent review remain pending
  - these RTC2 measurements precede a Wasm constant rename and removal of the obsolete Denise plane-count cache and getter
  - focused RTC calendar and native host-conversion tests passed after coverage review, including reproduction and repair of pre-epoch rounding
---

### [2026-11-14 20:50 CET] — Keep verifier orchestration changes within Python contracts
- **Affected Subsystems**:
  - `tools/verification`
  - `Verification design specification`
- **What Was Changed (The Concrete Reality)**:
  - Classify five known execution and evidence files explicitly as orchestration inputs
  - preserve package selection for concurrent Rust changes and the full Rust fallback for selection logic and unknown files
  - add selection contract tests and describe the ownership boundary
- **Architectural Rationale & Trade-Offs**:
  - Avoid starting emulator tests solely because execution or evidence handling changed while retaining Python contracts and quality gates
- **Verification & Test Results**:
  - Tests
  - verifiers
  - graph refresh
  - language scans and independent review were not run for this batch at the user's explicit request.
---

### [2026-11-14 20:50 CET] — Use segment patterns for Graphify exclusions
- **Affected Subsystems**:
  - `.graphifyignore`
  - `Graphify skill`
- **What Was Changed (The Concrete Reality)**:
  - Express generated-directory and binary-format exclusions with recursive segment patterns and ASCII case classes
  - document root anchoring and preservation of the selected reference corpus
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated whole-path matching while preserving case-insensitive exclusions and reference relationships
- **Verification & Test Results**:
  - Tests
  - verifiers
  - graph refresh
  - language scans and independent review were not run for this batch at the user's explicit request.
---

### [2026-11-14 20:50 CET] — Overlap independent preparation work on a stable candidate
- **Affected Subsystems**:
  - `Commit policy`
  - `Graphify skill`
- **What Was Changed (The Concrete Reality)**:
  - Allow independent review, Graphify refresh and canonical verification to overlap on a stable candidate
  - require isolated outputs and completion before freeze
  - serialize work sharing Cargo or coverage outputs and bound competing Graphify workers
- **Architectural Rationale & Trade-Offs**:
  - Reduce sequential preparation overhead without treating background launch as completion or changing verifier group scheduling
- **Verification & Test Results**:
  - Tests
  - verifiers
  - graph refresh
  - language scans and independent review were not run for this batch at the user's explicit request.
---

### [2026-11-14 22:34 CET] — Reuse sprite DMA comparisons without changing transfer timing
- **Affected Subsystems**:
  - `agnus`
  - `memory_bus register tests`
  - `save_state`
  - `DMA and save-state specifications`
- **What Was Changed (The Concrete Reality)**:
  - Complete the bounded PERIPHERAL-EVENT-OPTIMIZATION-20261114 pass with compact sprite active/request masks and comparisons reused until their inputs change
  - Preserve mid-line register writes and vertical latches with DMA disabled or output blocked
  - Group sprite state in save format 8 and test continuation from an active latch
  - Reject and restore three disk variants, the UART variant and two audio variants after whole-machine measurements
- **Architectural Rationale & Trade-Offs**:
  - Reduce repeated per-CCK work while preserving fixed DMA slots and snapshot continuation
  - Keep only the candidate supported by the requested PAL measurements
- **Verification & Test Results**:
  - Mixed PAL 8.51569 to 7.87536 ms per field, 7.52 percent lower time
  - Mixed NTSC 7.19934 to 6.69548 ms, 7.00 percent lower time
  - Kickstart PAL 6.72357 to 6.74259 ms, 0.28 percent higher within overlapping ranges
  - Four 1000-field samples per build and workload in two ABBA blocks, with matching full state values and frames after a lossless sprite-field relocation
  - 275 focused tests passed with no failures or ignored tests
  - Core Wasm compilation and architecture, code, hardware, Clippy and formatting checks passed
  - Commit-profile verification and independent batch review are pending
  - Game-port work remains in Host Input Delivery
---

### [2026-11-14 23:37 CET] — Record pending roadmap updates
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Commit the pending roadmap edits at the user's explicit request
- **Architectural Rationale & Trade-Offs**:
  - Preserve the roadmap changes separately from the sprite DMA optimization
- **Verification & Test Results**:
  - Tests
  - verification and content inspection were not performed
  - as explicitly requested
---

### [2026-11-15 00:16 CET] — Store completed Denise samples once per color clock
- **Affected Subsystems**:
  - `Denise`
  - `FrameBuilder`
- **What Was Changed (The Concrete Reality)**:
  - Complete DENISE-RENDER-EXPERIMENTS-20261114 with one four-sample framebuffer write per CCK
  - Preserve pixel composition order and two-sample output latency
  - Add clipping and row-boundary coverage
  - Reject grouped serializers, table gathering, shared sprite checks and stream reconstruction after comparison
- **Architectural Rationale & Trade-Offs**:
  - Resolve output row coordinates once per CCK while retaining current rendering
  - collision and snapshot behavior
- **Verification & Test Results**:
  - Mixed PAL median 8.476499 to 8.156421 ms per field, 3.78 percent less time
  - Mixed NTSC median 7.113554 to 6.851335 ms, 3.69 percent less time
  - Four samples per build from interleaved runs after 64 warmup fields, with 600 fields per PAL series and 240 per NTSC series
  - Complete serialized states and frame images match, excluding GUI and pacing
  - Idle PAL remains within overlapping ranges
  - Invalid final measurements caused by preserved source timestamps were discarded and repeated after rebuilding
  - 68 focused tests, core Wasm compilation, Clippy, formatting and diff checks passed
  - Commit-profile verification and independent review are pending
---

### [2026-11-15 00:49 CET] — Integrate Bootstrap Codex into master
- **Affected Subsystems**:
  - `Book Review`
  - `reference manual recipes`
  - `branch integration`
- **What Was Changed (The Concrete Reality)**:
  - Merge bootstrap-codex-api into master while preserving both histories
  - Retain the Book Review math provenance correction and reference manual preparation artifacts
  - Reconcile both diary append sequences chronologically and retain both vocabulary additions
  - Include all 56 changed Markdown paths in the integration review
  - Correct retained documentation about accelerated field presentation, test selection and SingleStep comparison boundaries
- **Architectural Rationale & Trade-Offs**:
  - Combine the bootstrap work with current emulator and workflow development, then advance the bootstrap branch to the merge point
  - Retain unfinished source review and roadmap work
- **Verification & Test Results**:
  - Both initial worktrees were clean
  - Fast-forward into master was unavailable because the histories diverged
  - Both imported JSON artifacts parse and exactly match the source branch
  - Git diff checks passed
  - Initial language, frontmatter, formatting, architecture, hardware, code quality and Clippy checks passed
  - The initial profile was stopped during coverage to reconcile documentation findings
  - Final canonical verification and independent review are pending
---

### [2026-11-15 09:34 CET] — Prioritize native disk execution and plan shared player workflows
- **Affected Subsystems**:
  - `ROADMAP.md`
  - `Design documentation`
- **What Was Changed (The Concrete Reality)**:
  - Add linked contents and reorder remaining Step 2 work as native disk bring-up then native vAmigaTS verification then optional lockstep diagnosis
  - Withdraw BOOK-GUEST-PROGRAM-TESTS-20261111 and delete its obsolete ignored plan by explicit user direction without claiming guest acceptance
  - Retain missing media-stream and DMA checks in disk bring-up and move examples/m68k retirement into native verification
  - Plan shared configuration and recognized ROM selection plus persistent unnamed save catalogs and portable game profiles
  - Plan WASM bundles and desktop game libraries plus video capture and both detailed debugger rewind and checkpoint/event replay
  - Update incoming roadmap references while retaining timing and coverage limits
- **Architectural Rationale & Trade-Offs**:
  - Prioritize running original ADF software over a standalone guest-program campaign
  - Record future player capabilities without implementing them or settling open packaging and layout choices
- **Verification & Test Results**:
  - Roadmap retention and navigation checks passed
  - Vault audit passed for 35 Design notes and 1083 links
  - Source inspection confirmed existing quick slots and preview support plus the zero timestamp and missing runtime media stream
  - Initial English scan reported the valid term gameplay as unknown and the reviewed software vocabulary now includes it
  - Independent review identified three plain-text references to the withdrawn stage and these now point to native bring-up or verification
  - Documentation-only change with no production coverage measurement required
  - Canonical commit verification and independent review are pending
---

### [2026-11-15 09:48 CET] — Normalize math angle entities during book import
- **Affected Subsystems**:
  - `Book-to-Markdown importer`
  - `PDF source recipes`
- **What Was Changed (The Concrete Reality)**:
  - Decode named and numeric HTML angle entities inside parsed Markdown math before source alignment
  - Preserve code and preformatted text plus entities outside math
  - Add import-to-render regression coverage without changing PDF conversion prompts
  - Locate math by parser source spans and apply heading prefixes once
- **Architectural Rationale & Trade-Offs**:
  - Prevent HTML angle entities from reaching KaTeX as invalid math while keeping PDF references and exports consistent
- **Verification & Test Results**:
  - The new regression failed before the fix and passed afterward
  - All 41 text recipe tests and 33 source alignment tests passed
  - Reimported 5334 elements from 646 pages of the M68000 Programmer reference manual
  - Rendering completed and Working matches Accepted
  - Editor and document endpoints returned HTTP 200 with no pending render or error
---

### [2026-11-15 10:07 CET] — Store prepared Motorola manual artifacts
- **Affected Subsystems**:
  - `Reference manual recipes`
  - `PDF preparation metadata`
- **What Was Changed (The Concrete Reality)**:
  - Store the M68000 Programmer reference manual book recipe
  - Store the M68000 User manual revision 8 OCR provenance
- **Architectural Rationale & Trade-Offs**:
  - Record the remaining local book artifacts at the user-requested commit point
- **Verification & Test Results**:
  - No tests or verification were run for this commit at the explicit user request
  - Earlier importer verification does not certify whole-book transcription quality
---

### [2026-11-15 10:53 CET] — Run the machine on an independent native and browser owner
- **Affected Subsystems**:
  - `gui`
  - `debugger`
  - `frame_builder`
  - `save_state`
  - `host runtime policy`
- **What Was Changed (The Concrete Reality)**:
  - Move the complete session to one host worker
  - exchange seeded reusable pixel allocations through synchronized ownership
  - migrate commands and stopped inspection
  - retire direct-session file and ROM wrappers
  - recognize production JavaScript callers in code-quality reference discovery
  - preserve debugger ROM patching and 24-bit address wrapping through the owner command boundary
  - bound the absent-image playback test failure path
  - add grouped regressions for owner history and stack/reset commands, unfinished-field continuation, GUI ROM/RAM byte edits and breakpoint submission, and actual snapshot file save/error completion
- **Architectural Rationale & Trade-Offs**:
  - HOST-SHARED-MEMORY-PRESENTATION-20261113 keeps deterministic CCK execution independent of GUI repaint
  - six exact Clippy permissions and the JavaScript corpus change have explicit user approval
  - host audio and guest input endpoints remain open
- **Verification & Test Results**:
  - The initial five-concern candidate passed the canonical commit profile with CARGO_BUILD_JOBS=1
  - subsequent coverage review identified reachable migration gaps and added grouped behavioral regressions
  - focused instruction/hex/breakpoint, persistence, history, continuation and corrected warm-reset tests pass
  - final combined verification and review remain pending
---

### [2026-11-15 10:53 CET] — Recover missing recent files and picker folders
- **Affected Subsystems**:
  - `gui file actions`
  - `native history`
  - `browser history`
- **What Was Changed (The Concrete Reality)**:
  - Remove unavailable recent binaries and ROMs after selection with an explanatory message
  - choose the nearest existing native parent folder
  - use the browser default folder when unavailable
  - preserve entries after access denial or ordinary cancellation
- **Architectural Rationale & Trade-Offs**:
  - Keep load options and category histories independent
  - record successful owner replies before updating file labels or history
- **Verification & Test Results**:
  - The originating session reported seven Rust tests and a Chromium history regression passing
  - final combined verification remains pending
---

### [2026-11-15 10:53 CET] — Require optimized browser builds without Cargo lint warnings
- **Affected Subsystems**:
  - `tools/web.ps1`
  - `verification`
  - `frontend setup`
- **What Was Changed (The Concrete Reality)**:
  - Build and serve Release WebAssembly with the existing Nightly shared-memory configuration
  - require the real browser build in commit verification
  - reject Cargo lint warnings and restore the caller environment
  - include browser build tools in execution conditions
- **Architectural Rationale & Trade-Offs**:
  - Catch browser build failures and manifest warnings during development and before a commit
  - keep the global default compiler independent of the browser launcher
- **Verification & Test Results**:
  - The originating session reported the actual browser build and architecture checks passing
  - final combined verification remains pending
---

### [2026-11-15 10:53 CET] — Reserve the visible pixel count during screen conversion
- **Affected Subsystems**:
  - `gui screen texture`
  - `screen scaling tests`
- **What Was Changed (The Concrete Reality)**:
  - Allocate the visible pixel count once and convert each visible row without stride padding
  - preserve ARGB alpha and image dimensions
- **Architectural Rationale & Trade-Offs**:
  - The retained WP01 experiment reduced local conversion time by about half
  - it did not establish a large whole-application FPS improvement
- **Verification & Test Results**:
  - The retained performance report records measured A/B comparisons and behavioral checks
  - final combined verification remains pending
---

### [2026-11-15 10:53 CET] — Report Stable release updates during commit verification
- **Affected Subsystems**:
  - `tools/verification`
  - `Rust toolchain`
- **What Was Changed (The Concrete Reality)**:
  - Check official Stable metadata once per actual commit invocation including evidence reuse
  - show the active compiler and log the advisory result
  - retain a five-second timeout and offline plan previews
  - do not automatically update or gate tests on release availability
- **Architectural Rationale & Trade-Offs**:
  - The account default Stable changed from Rust and Cargo 1.98.1 with LLVM 22.1.8 to Rust and Cargo 1.100.0 with LLVM 23.1.5
  - the experimental named toolchain was removed
  - Nightly and installed targets/components remain available without a repository pin
- **Verification & Test Results**:
  - Eight update contracts and the final 164-test tooling run passed before preparation
  - workspace compilation and 165 machine-loop tests passed on the updated Stable
  - required compiler identity still invalidates test evidence
  - final combined verification remains pending
---

### [2026-11-15 10:53 CET] — Track verifier command trees with Windows Job Objects
> Historical: This mechanism is unavailable after VERIFICATION-LOCK-SIMPLIFICATION-20261116. Checkout-level locking replaces command-tree ownership and telemetry cleanup.
- **Affected Subsystems**:
  - `tools/verification`
  - `Windows process ownership`
- **What Was Changed (The Concrete Reality)**:
  - Assign commands to a named Job Object during process creation
  - retain the job with a guardian until descendants drain
  - preserve nested verifier containment, result-directory locking and fail-closed recovery
  - retain the conservative POSIX marker scan
- **Architectural Rationale & Trade-Offs**:
  - Replace unstable Windows PID-list comparisons without killing orphan commands
  - port the reviewed repair from the separate verification-job-tracking worktree without changing that source checkout
  - archive only completed legacy lock metadata under its kernel lock after establishing that no tagged descendants remain
- **Verification & Test Results**:
  - Six integrated real Windows tests pass
  - prior completed batch evidence stays in its original directory
  - full combined canonical verification and independent review remain pending
---

### [2026-11-15 11:26 CET] — Integrate bootstrap and shared-memory host work into master
- **Affected Subsystems**:
  - `Bootstrap importer and manual recipes`
  - `GUI host runtime`
  - `Frame buffer and save state`
  - `Verification tooling`
- **What Was Changed (The Concrete Reality)**:
  - Combine both source histories with master in one explicit merge
  - Preserve both diary histories and all reviewed vocabulary additions
  - Inventory 24 changed Markdown paths across the three branch histories and the affected general architecture note
  - Retain master roadmap priorities alongside independent host execution and current book import behavior
  - Correct browser toolchain instructions and stale file-access ownership descriptions in the integrated documentation
- **Architectural Rationale & Trade-Offs**:
  - Verify the complete integrated candidate in one batch before synchronizing and publishing branches
- **Verification & Test Results**:
  - All worktrees were clean before integration
  - Fast-forward attempts confirmed divergent histories
  - Native Git merge preparation and diff whitespace checks completed
  - Vault metadata and link audit passed for 35 Design notes and 1096 links
  - Canonical verification and independent review are pending
---

### [2026-11-15 12:27 CET] — LOCAL-INTEGRATION-20261115: prepare local integration on the task branch
- **Affected Subsystems**:
  - `Agent rules`
  - `Verification tooling`
  - `Worktree lifecycle`
- **What Was Changed (The Concrete Reality)**:
  - Require an explicit task checkout choice while retaining bounded detached diagnostics
  - Compare complete integration candidates with immutable primary-branch commits
  - Freeze approved trees and partitions and seal exact mechanical commits
  - Publish through prepared native Git reference locks with recoverable checkout transitions
  - Preserve unique local material before safe worktree removal and conditional branch deletion
- **Architectural Rationale & Trade-Offs**:
  - Keep the combined reviewed candidate as the acceptance target
  - Refuse concurrent reference changes and preserve unexpected file edits
  - Keep commit-only and complete integration authorization distinct
- **Verification & Test Results**:
  - Passed 16 real-Git integration contracts including competing processes and termination at five publication boundaries
  - Passed 3 PowerShell worktree contracts and script parsing
  - Passed 34 existing continuation contracts and 43 existing CLI contracts
  - Vault audit passed for 35 specifications and 1097 links
  - Canonical commit verification and independent final review remain pending
  - Independent review identified a staged ignored-file omission and a recovery reference race; candidate trees now retain the real index and recovery holds native verification locks
  - Sealing also rejects a same-tree ancestor rewind with an empty partition; worktree test ownership preserves the script's full Rust verification scope
  - The first complete profile passed static gates and the browser build but coverage compilation exhausted virtual memory; the next attempt limits Cargo to one build job without reducing test scope
  - Automatic cleanup conservatively refuses inaccessible Windows process ownership; successful teardown contracts isolate process discovery while exercising real Git and PowerShell removal
---

### [2026-11-15 17:31 CET] — Reduce repeated idle input work
- **Affected Subsystems**:
  - `crates/keyboard`
  - `crates/game_ports`
- **What Was Changed (The Concrete Reality)**:
  - Encode joystick direction contacts when switches change
  - retain an explicit empty keyboard idle phase and wake it on queued input
  - preserve wire timing and serialized continuation
- **Architectural Rationale & Trade-Offs**:
  - Avoid repeated per-CCK reconstruction and empty selection without changing device phases
- **Verification & Test Results**:
  - Existing focused keyboard and game-port checks passed before preparation
  - retained measurement report records direct original-to-candidate throughput gains and the isolated active-keyboard tradeoff.
---

### [2026-11-15 17:31 CET] — Deliver host input at three field boundaries
- **Affected Subsystems**:
  - `crates/gui`
  - `crates/agnus`
  - `crates/denise`
  - `crates/debugger`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Collect native OS and controller events on blocking listener threads
  - publish browser DOM and sampled gamepad events through bounded shared queues
  - admit ordered events three times per field, aligned with the receiver origin after startup and reset
  - release controls and reject obsolete generations across lifecycle changes
  - document host transport
- **Architectural Rationale & Trade-Offs**:
  - Separate immediate host publication from deterministic device delivery without adding host synchronization per CCK
- **Verification & Test Results**:
  - Focused GUI, debugger, machine-loop and Agnus tests and native/Wasm quality checks passed before preparation. Existing Windows and Chrome probes exercised keyboard/mouse delivery
  - physical gamepad freshness and Linux/macOS execution remain unverified. Vault audit passed for 35 documents and 1102 links.
---

### [2026-11-15 17:31 CET] — Plan reusable input profiles and gamepad frontend control
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Plan reusable keyboard and controller mappings with named profiles
  - defer full gamepad-only frontend navigation until player controls exist
  - specify physical input diagnostics after basic host input and Workbench boot
- **Architectural Rationale & Trade-Offs**:
  - Keep advanced input configuration and interactive acceptance evidence explicit in their owning roadmap stages; user approved the basic adapter before the separate, still-open A500 combination fidelity investigation
- **Verification & Test Results**:
  - Reviewed future prerequisites and verification scope
  - no roadmap stage closed or advanced profile feature implemented.
---

### [2026-11-15 19:13 CET] — TASK-CLOSURE-20261115: approve and complete local task integrations
- **Affected Subsystems**:
  - `Agent rules`
  - `Task closure skill`
  - `Verification tooling`
- **What Was Changed (The Concrete Reality)**:
  - Replace the merge procedure with git-close-task and require approval of an exact integration and cleanup plan
  - Allow an explicitly unverified working commit before one combined integration verification
  - Accept an unchanged clean fast-forward after confirmation of the exact tested tip without fabricating canonical evidence
  - Preserve original divergent verification scope through explicitly requested rebase and freeze revalidation
- **Architectural Rationale & Trade-Offs**:
  - Keep commit-only requests scoped and avoid duplicate preparation while retaining exact conditional publication and safe cleanup
- **Verification & Test Results**:
  - 202 verification tool tests passed before preparation
  - Skill validation passed and 179 local Markdown targets resolved
  - Vault audit passed for 35 Design notes and 1098 links
  - Canonical commit verification and final independent review are pending
---

### [2026-11-15 19:14 CET] — Remove the module splitting skill
- **Affected Subsystems**:
  - `Agent skills`
  - `Rust module guidance`
- **What Was Changed (The Concrete Reality)**:
  - Remove refactor-split-module and its two supporting references
  - Remove incoming links and retain module cohesion and workspace boundary rules
- **Architectural Rationale & Trade-Offs**:
  - Retire the requested procedure while preserving the existing architecture constraints and neighboring-code guidance
- **Verification & Test Results**:
  - No Rust production code or architecture checker was changed
  - No retired skill references remained in active instructions or documentation
  - Combined batch verification and independent review are pending
---

### [2026-11-15 19:40 CET] — HOST-INPUT-DELIVERY-20261113: close host input delivery after local integration
- **Affected Subsystems**:
  - `GUI host input`
  - `Keyboard and game ports`
  - `Integration and task closure`
- **What Was Changed (The Concrete Reality)**:
  - Rebase the three committed task concerns onto master decdd50 and preserve both diary histories and vocabulary sets
  - Deliver ordered bounded host input at three field boundaries with native blocking listeners and browser shared queues
  - Retain keyboard idle and joystick contact optimizations without changing wire timing or adding mouse smoothing
  - Keep mapping profiles and physical input acceptance work in the roadmap
  - Synchronize the toolbar layout fixture with the existing asynchronous host settling helper while retaining all geometry assertions
  - Name raw key mappings locally and reuse the keyboard modifier constants through the explicitly approved GUI dependency
  - Protect Developer image-click capture and window-focus release through real headless pointer events and guest button-pin assertions
  - Extend native mouse translation assertions to all three buttons
- **Architectural Rationale & Trade-Offs**:
  - Close the delivered task through one combined integration verification while retaining deterministic ownership and recorded platform limits
- **Verification & Test Results**:
  - Rebase completed with conflicts limited to the diary and programming vocabulary
  - Range comparison preserves all production code and tests from the original task tip ef138f0
  - Twenty-three changed Markdown paths were reconciled and the detailed task report will survive in the integration archive
  - Existing Windows and Chrome probes remain historical evidence with no new timing measurements
  - Physical gamepad freshness, Linux and macOS execution and A500 combination fidelity remain unverified or deferred
  - The first integration coverage run exposed a toolbar fixture that exhausted four immediate GUI passes while awaiting the machine worker
  - The user approved the direct GUI-to-keyboard dependency and its single architecture permission for authoritative keycode reuse
  - The user approved stopping the remaining MSVC telemetry process from the first verifier job without changing the process ownership guard
  - Coverage review identified missing retained GUI-focus interaction protection; the added focused regression passes and honors repaint retries
  - Canonical integration verification and final independent review are pending
---

### [2026-11-15 22:08 CET] — Separate independent review into four bounded lanes
- **Affected Subsystems**:
  - `Agent governance`
  - `Review skills and role routing`
- **What Was Changed (The Concrete Reality)**:
  - Assign fresh reviewers to behavior, architecture, tests and evidence, and documentation
  - Synchronize affected reviewer criteria with instruction changes
  - Carry unchanged scoped approvals after exact comparison
- **Architectural Rationale & Trade-Offs**:
  - Keep each reviewer focused and avoid repeating unaffected reviews after descriptive corrections
- **Verification & Test Results**:
  - Reviewed the pending instructions and their routing
  - No emulator behavior changes in this concern
  - Canonical verification and independent review are pending
---

### [2026-11-15 22:08 CET] — Discard ignored worktree files during integration cleanup
- **Affected Subsystems**:
  - `Worktree tooling`
  - `Integration cleanup and documentation`
- **What Was Changed (The Concrete Reality)**:
  - Remove ignored-file inventories and preservation archives during worktree deletion
  - Retain conditional branch deletion and native removal failure handling
  - Ignore inaccessible process directories while blocking confirmed source commands
  - Recommend bootstrap steps from changed tracked procedures without automatically running them
- **Architectural Rationale & Trade-Offs**:
  - Apply the user-approved cleanup contract without comparing or copying disposable local material
- **Verification & Test Results**:
  - Reviewed cleanup data flow and updated behavioral assertions
  - git diff --check passed before diary preparation
  - Canonical verification and independent review are pending
---

### [2026-11-15 22:08 CET] — Stop verification-owned MSVC telemetry at job completion
> Historical: This mechanism is unavailable after VERIFICATION-LOCK-SIMPLIFICATION-20261116. Checkout-level locking replaces command-tree ownership and telemetry cleanup.
- **Affected Subsystems**:
  - `Windows verification process ownership`
- **What Was Changed (The Concrete Reality)**:
  - Identify vctip.exe by its MSVC toolchain path and verify job membership through retained process handles
  - Stop telemetry only when no other kind of job member remains
  - Cover isolated telemetry completion and mixed-member refusal
- **Architectural Rationale & Trade-Offs**:
  - Allow verification completion without repeated manual telemetry termination while protecting real commands and unrelated processes
- **Verification & Test Results**:
  - Reviewed process-handle ownership and telemetry identity checks
  - Updated the testing specification to describe the approved exception
  - Canonical verification and independent review are pending
---

### [2026-11-15 23:15 CET] — NATIVE-DISK-BRING-UP-20261115: native disk execution and closure
- **Affected Subsystems**:
  - `Floppy`
  - `Machine loop`
  - `Save state`
  - `Roadmap and specifications`
- **What Was Changed (The Concrete Reality)**:
  - Generate circular timed ADF tracks through production pins and DMA
  - preserve media on electronic reset and serialize usable tracks
  - prove original cross1 startup and repeated Copper results
  - record the separate Workbench attempt
  - prepare roadmap pruning and integrate settled chipset closure into Chapters 15 and 16
- **Architectural Rationale & Trade-Offs**:
  - Complete native guest startup without injected payloads or substituted system services
  - retain independent hardware ownership and evidence boundaries
- **Verification & Test Results**:
  - Focused floppy and autonomous DMA suites pass
  - cross1 boots twice at CCK 14752752 and no-disk control lacks its signature
  - Workbench bootblock DOS and named task observed within 60 seconds with corrupted frame
  - full image agreement and media writes remain unverified
  - canonical closure profiles and independent review pending
---

### [2026-11-15 23:32 CET] — Review imported books through one catalog server
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Discover imported book variants in a browser catalog
  - Bind editor requests and render queues to the selected book
  - Keep PDF page and pane width per book
  - Replace per-book server arguments with browser selection
- **Architectural Rationale & Trade-Offs**:
  - Allow separate browser tabs to review books without redirecting edits to another book
- **Verification & Test Results**:
  - Book library contracts passed
  - PowerShell parser and usage checks passed
  - Status reports five books and no pending renders or errors for the two opened books
  - Invalid actions return a nonzero exit code
---

### [2026-11-15 23:32 CET] — Record the accepted M68000 user manual recipe
- **Affected Subsystems**:
  - `tools/bootstrap/books/m68000-users-manual-rev8/e41cbe7e14dc`
- **What Was Changed (The Concrete Reality)**:
  - Add the accepted source-backed reconstruction recipe
- **Architectural Rationale & Trade-Offs**:
  - Preserve the current accepted recipe alongside its existing source identity and OCR data
- **Verification & Test Results**:
  - Recipe matches local Accepted
  - Accepted input resolves one chapter with 1343 elements and 100 assets
  - Source PDF SHA-256 matches source.json
  - Whole-book transcription and layout quality were not assessed
---

### [2026-11-16 00:49 CET] — BOOTSTRAP-NATIVE-INTEGRATION-20261116: integrate book catalog with native disk execution
- **Affected Subsystems**:
  - `Local integration`
  - `Book review catalog`
  - `Native disk documentation`
- **What Was Changed (The Concrete Reality)**:
  - Combine Bootstrap commits fb5a48b and bccaee1 with native tip c7563f4 without rewriting their history
  - Preserve native Chapter 16 and roadmap boundaries together with both book entries
  - Retain vocabulary from both branches
- **Architectural Rationale & Trade-Offs**:
  - Keep independent book review sessions and the accepted manual recipe alongside the unchanged native disk implementation
  - Reconcile all changed Markdown against the common base and both branches
- **Verification & Test Results**:
  - Native c7563f4 was published unchanged using its confirmed final-source verification
  - Bootstrap catalog contracts and recipe checks are historical results recorded by the original commits
  - No recipe was regenerated and whole-book quality was not reassessed
  - Integrated canonical verification and four independent review lanes are pending
---

### [2026-11-16 01:03 CET] — MEMORY-ACCESS-INVARIANTS-20261115 / MUTATION-QUEUE-OVERFLOW-20261115: enforce fatal runtime invariants
- **Affected Subsystems**:
  - `CPU`
  - `Physical memory`
  - `Custom chips and CIA`
  - `Machine loop`
  - `Native and browser GUI`
  - `Save states`
- **What Was Changed (The Concrete Reality)**:
  - Make Chip RAM private and count CPU and DMA transfers against one serialized budget per CCK
  - Guard CPU attempts and read/write phases in both execution engines, including waits and TAS
  - Make mutation queue overflow and earlier shared-register/CIA preflights fatal before partial delivery
  - Report the original diagnostic at the native owner or browser worker boundary, reject later work and finish shutdown without waiting for the failed owner
  - Migrate fixtures and the benchmark bus to explicit cycles and require admission state in save format 9
  - Apply the approved function-scoped panic permission, CPU dependency on config and removal of the obsolete empty raw-buffer test while retaining open-bus address behavior
  - Reject instruction retirement before read completion, preserve rejected disk requests, and retain the first native diagnostic across flag/completion and poisoned-mailbox races
  - Remove the unused block-injection helper, reuse physical byte inspection in the debug bank paths and reconcile active retry/dependency descriptions
- **Architectural Rationale & Trade-Offs**:
  - Stop internal violations before rejected side effects while preserving guest exceptions, ordinary waits, queue capacities and propagation timing
  - Keep both checks enabled in optimized builds
  - User approved the narrow panic and dependency permissions and the one obsolete test removal
  - Older snapshot formats are unsupported under the existing pre-release policy
- **Verification & Test Results**:
  - Focused owning suites passed, including 200 component tests, 124 GUI tests and 20 persistence tests
  - Seven CPU admission tests and physical admission tests passed in debug and release
  - Sampled SingleStep verification passed 6350 vectors with state, clocks and bus comparisons retained
  - Current architecture, Clippy, formatting, real browser build and actual Rust/Wasm fatal delivery passed
  - All 11 TAS benchmark cases executed after fixture migration
  - Final interleaved original/candidate comparison measured 123.442 to 119.584 FPS mixed and 211.130 to 198.452 FPS CPU/Fast RAM, reductions of 3.13 and 6.00 percent
  - Native worker timing spread overlaps and no browser throughput or full hardware capture result is claimed
  - Final optimized profiles and matching source/binary identities remain in task artifacts
  - New CPU retirement and disk-admission regressions failed before their fixes and pass afterward; deterministic native failure classification passes in debug and release
  - Reviewed changed coverage spans after extending repeated-clock, pending-retirement, replacement-preflight and unavailable-disk-data tests
  - Native coverage omits source-owning test modules and Wasm, which have separate direct and browser evidence; poisoned-mailbox forwarding branches were not exercised
---

### [2026-11-16 08:26 CET] — Publish Windows verification fixture readiness atomically
> Historical: The Windows Job Object fixture and its process-ownership protection are unavailable after VERIFICATION-LOCK-SIMPLIFICATION-20261116.
- **Affected Subsystems**:
  - `Verification CLI test fixtures`
- **What Was Changed (The Concrete Reality)**:
  - Write the detached test child PID to a temporary file and atomically rename it to the ready marker after closing the complete payload
  - Preserve all process ownership assertions, timeouts, telemetry identity checks and required gates
- **Architectural Rationale & Trade-Offs**:
  - Two canonical attempts reproduced reading an empty ready file before write_text finished, followed by cleanup sharing errors
  - Readiness must mean the complete PID is available rather than only that a file was opened
- **Verification & Test Results**:
  - Existing Windows Job fixture suite passed all nine tests after the change
  - Canonical attempts before the fixture correction failed with empty-PID ValueError and cleanup sharing errors
  - Emulator production code and reference values are unchanged by this prerequisite fix
### [2026-11-16 09:28 CET] — VERIFICATION-LOCK-SIMPLIFICATION-20261116: simplify verifier ownership
- **Affected Subsystems**:
  - `tools/verification`
  - `Verification requirements and commit continuation`
- **What Was Changed (The Concrete Reality)**:
  - Use one nonblocking OS lock per checkout across executing verification entry points
  - Remove command-tree supervision and its obsolete tests while retaining evidence reuse and all acceptance gates
  - Isolate executing CLI fixtures and document the replacement contract
- **Architectural Rationale & Trade-Offs**:
  - Authorize verifier mutual exclusion without process registries or recovery machinery
  - Surviving children may overlap a later invocation and are no longer detected or terminated
  - Keep administrative integration transactions and psutil consumers unchanged
- **Verification & Test Results**:
  - Focused verification passed 200 Python tests on Windows
  - Real subprocess tests cover contention and unchanged evidence plus release and reuse after interruption
  - git diff --check passed
  - POSIX execution is unrun
  - Canonical commit verification and four independent reviews are pending
### [2026-11-16 09:41 CET] — GUI-MONITOR-IMAGE-FAILURE-20261116: wait for the monitor image before checking its upload
- **Affected Subsystems**:
  - `GUI integration tests`
- **What Was Changed (The Concrete Reality)**:
  - Capture the expected stopped CCK before transferring the session to the host
  - Wait for the presented image to match the current generation and expected CCK and dimensions
  - Retain every full-image, pixel, geometry, centering, UV and unchanged-framebuffer assertion
- **Architectural Rationale & Trade-Offs**:
  - Command replies and video use independent paths, so idle commands do not prove that the GUI has received an image
  - Keep the command-idle predicate and existing timeout unchanged
- **Verification & Test Results**:
  - The unchanged test passed once and then reproduced the texture lookup failure naturally
  - Controlled mailbox contention reproduced the same failure in all eight PAL/NTSC, interlace and view combinations
  - All eight paired cases using the repaired waiting block passed the original assertions
  - The complete GUI package passed 120 tests without failures or ignored tests
  - The browser build and focused formatting checks passed
  - Canonical commit verification is pending
  - The historical failure logs remain unavailable, so the exact historical thread schedule is unverified
### [2026-11-16 09:42 CET] — Publish Windows Job Object fixture readiness atomically
> Historical: The Windows Job Object fixture and its process-ownership protection are unavailable after VERIFICATION-LOCK-SIMPLIFICATION-20261116.
- **Affected Subsystems**:
  - `tools/verification/tests/test_windows_job.py`
- **What Was Changed (The Concrete Reality)**:
  - Write the complete PID to a temporary file and atomically replace the ready signal before the parent reads it
- **Architectural Rationale & Trade-Offs**:
  - The full tooling run demonstrated an empty-PID read and cleanup failure when file existence became visible before its contents
  - Preserve every containment and cleanup assertion
- **Verification & Test Results**:
  - The initial 216-case run failed in this fixture
  - All nine Windows Job Object cases and the corrected 218-case tooling suite passed
### [2026-11-16 09:42 CET] — TEST-RUNTIME-AND-VALUE-20261116: report verification timings without invalidating inputs
- **Affected Subsystems**:
  - `tools/verification`
  - `tests/test-runtime-report.md`
  - `verification instructions and specifications`
- **What Was Changed (The Concrete Reality)**:
  - Generate a versioned timing report and grouped console summary from existing command timers
  - Exclude this exact output from selection source stability input identities and language scans while sealing it in the candidate tree
  - Select verification-tool contracts only for their owning code tests policies and consumed shared tooling
- **Architectural Rationale & Trade-Offs**:
  - Expose executed reused failed missing and skipped work without repeating tests to measure them
  - Preserve required repository gates and detect real source edits
- **Verification & Test Results**:
  - All 218 focused tooling contracts passed after the separate PID fixture correction
  - Six alternating reused fixture pairs measured median generation 24.608 ms and writing 0.528 ms with a noisy 87 ms end-to-end difference
  - Input fixture profiling found 5.671 s of machine advance and 13 roughly 26 MB snapshots with 6.313 s acquisition plus 3.311 s decoding
  - Emulator production code is unchanged and changed-code coverage review is inapplicable
  - Canonical batch verification and independent review are recorded in the commit message
---

### [2026-11-16 09:42 CET] — TEST-RUNTIME-AND-VALUE-20261116: shorten audio captures and retire presentation campaigns
- **Affected Subsystems**:
  - `crates/gui/tests`
  - `crates/machine_loop/tests`
- **What Was Changed (The Concrete Reality)**:
  - Remove thirteen pacing and graphical-program assertion cases while preserving guest assets and manual measurement examples
  - Shorten PAL and NTSC audio WAV captures to about 120 ms without widening acceptance bounds
- **Architectural Rationale & Trade-Offs**:
  - Apply the recorded user decision to evaluate presentation and FPS manually while retaining CPU chip bus input and host command correctness checks
- **Verification & Test Results**:
  - Three audio cases passed before and after with execution falling from 6.88 s to 1.76 s outside compilation
  - Twenty-seven focused GUI input and host cases passed
  - Browser build and formatting passed
  - No whole-commit speedup or hardware agreement is established
---

### [2026-11-16 09:47 CET] — GRAPHIFY-REFRESH-PERFORMANCE-20261116: cache source paths within each refresh
- **Affected Subsystems**:
  - `tools/graphify`
  - `tools/worktree.ps1`
  - `tools/bootstrap/bootstrap_graphify.ps1`
  - `agent refresh procedures`
- **What Was Changed (The Concrete Reality)**:
  - Use a local Graphify 0.9.74 refresh command with per-instance path memoization
  - Route bootstrap and worktree refresh callers through the command
  - Retain full corpus extraction and resolver context
  - Preserve deterministic default hash seeds and UTF-8 output across interpreter restarts
  - Keep immutable reference freezing and unchanged-project preflight as active unfinished work
- **Architectural Rationale & Trade-Offs**:
  - Avoid repeated lexical path calculations without changing persisted graph semantics or the installed tool
  - The reference-layer prototype still needs complete dependency invalidation before production use
- **Verification & Test Results**:
  - Ten refresh contracts and four worktree contracts pass
  - PowerShell parsing and usage checks pass
  - Comparable four-worker unchanged pair: 42.155 to 34.367 seconds
  - Comparable four-worker one-function Rust pair: 54.128 to 46.910 seconds
  - The original baseline uses the unmodified Graphify module CLI to retain importable worker startup
  - Earlier console-launcher timings mixed sequential and parallel extraction and cannot attribute savings to memoization
  - Both final pairs preserve the entire graph including attributes and communities
  - These shared-host observations do not establish typical speedup
  - The 24 existing partial-extraction warnings remain visible
### [2026-11-16 11:04 CET] — Batch task integration with recoverable source cleanup
- **Affected Subsystems**:
  - `tools/verification`
  - `.agents/rules`
  - `.agents/skills/git-close-task`
  - `Obsidian/Amiga/Design/Verification`
- **What Was Changed (The Concrete Reality)**:
  - Completed task git-close-task-batch-integration-20261116
  - Extend the existing integration record with approved immutable sources and all-source publication locks
  - Assemble real intermediate merges before one combined verification and bind their history through the recorded HEAD
  - Retain operation-specific control records in the surviving checkout and report independently retryable source cleanup
  - Update the owning rules and procedures plus affected reviewer criteria
- **Architectural Rationale & Trade-Offs**:
  - Preserve original task commits and single-source behavior while accepting only one combined target transition
  - Keep cleanup recoverable after assembly removal and distinguish operations reaching the same commit
  - This implements the workflow without integrating any live task branches
- **Verification & Test Results**:
  - Focused integration suite passed all 36 tests before the record-isolation correction
  - The additional record-isolation regression passed and independently cleaned both source selections
  - Skill validation passed
  - Vault audit passed for 35 specifications and 1113 links
  - Canonical commit verification and four final review verdicts follow in batch preparation
  - Git fixtures establish tool behavior and process protections rather than hardware behavior
---

### [2026-11-16 11:35 CET] — Wait for the expected monitor image in the upload test
- **Affected Subsystems**:
  - `crates/gui/tests/test_interactions.rs`
- **What Was Changed (The Concrete Reality)**:
  - Apply the exact test correction from a2f604b with explicit user approval
  - Wait for a presented image at the current generation and original stopped CCK with the expected dimensions before examining the texture
  - Preserve every pixel and geometry assertion plus the existing timeout and command-idle checks
- **Architectural Rationale & Trade-Offs**:
  - The initial canonical profile failed the unchanged upload test because command replies may arrive before the independent video handoff
  - Scope expansion is limited to this existing synchronization correction and does not change GUI runtime behavior
- **Verification & Test Results**:
  - The original canonical failure is retained in the batch evidence
  - The corrected exact upload test passed all eight PAL and NTSC combinations with interlace and view selection
  - The previous canonical attempt passed all 216 verification-tool contracts including 37 integration tests
  - A complete canonical rerun and renewed four-lane review will cover the final two-commit batch
### [2026-11-16 12:03 CET] — PARALLEL-TEST-EXECUTION-PILOT-20261116: bound parallel verification without changing selected protection
- **Affected Subsystems**:
  - `Verification coordinator`
  - `Rust coverage runner`
  - `Persistence and GUI test fixtures`
- **What Was Changed (The Concrete Reality)**:
  - Use nextest 0.9.152 for ordinary coverage with sixteen slots and four GUI cases
  - Serialize the native-listener RTC case and the shared sample-binary writer and reader
  - Use PID-scoped persistence files and the existing monitor-image readiness repair from a2f604b with all original assertions
  - Add deterministic obsolete-generation rejection protection
  - Overlap audited Python directories with coverage and independent preliminary gates with two coordinator workers
  - Preserve serial builds and harness writers and coordinator-owned evidence with ordered coverage failure handling
- **Architectural Rationale & Trade-Offs**:
  - Preserve selected identities and assertions without retries or changed timeouts
  - Retain focused Cargo diagnostics and doctests
  - Include runner version and configuration in evidence identity
  - Keep bootstrap setup and full SingleStep separate
  - The user stopped additional timing campaigns after the measured benefit was clear
- **Verification & Test Results**:
  - Base bb68537 with identical fixture repairs in both arms selected 790 cases with 788 active and two asset-dependent ignored cases
  - Warm ordinary coverage reset plus tests plus export took 280.403 seconds with Cargo and 134.418 seconds with nextest which is 52.06 percent less elapsed time and 2.09 times as fast
  - A second nextest arm passed in 135.350 seconds
  - Sampled peak process-tree working sets were 523.6 MiB baseline and 392.8 MiB nextest with mean host CPU 28.9 and 26.9 percent
  - Four executing GUI cases were observed and ten RTC samples had no competing test
  - Source coverage compared across 207 files with identical function and line totals and one changed stop-loop break span versus one changed toast-expiration span reflecting asynchronous timing
  - No coverage exclusion or acceptance threshold changed
  - Earlier failed or interrupted trials remain diagnostics under .agent/tmp/parallel-tests and are not passing evidence
  - The full verifier speedup and Linux or macOS execution were not measured
  - Existing continuation contracts passed 34 cases and the focused new parallel and selection contracts passed
  - The browser build and the 35-specification vault audit with 1114 links passed
  - The first canonical profile passed twelve groups but failed Windows process contracts when a ready file existed before its PID had been written
  - Publish the complete PID through an owned pending file and atomic replacement without changing process-lifetime assertions or deadlines
  - All nine focused Windows process contracts passed after that fixture correction
  - Final canonical commit verification and four independent review verdicts are pending
---

### [2026-11-16 12:12 CET] — Plan debugger register and chip-state screen layout
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Add planned item 5.5 linking custom-chip and CIA register inspection with internal chip-state views and screen layout
  - Consider tabs and grouped panels while leaving the final layout undecided
- **Architectural Rationale & Trade-Offs**:
  - Make hardware activity inspectable within the debugger's available screen space
- **Verification & Test Results**:
  - Tests and automated verification and independent review were not run at the user's explicit request

### [2026-11-16 12:49 CET] — INTEGRATION-ALL-20261116: integrate seven source branches and close completed tasks
- **Affected Subsystems**:
  - `Emulator memory and host failure handling`
  - `GUI regression protection`
  - `Verification and local integration tooling`
  - `Graphify refresh`
- **What Was Changed (The Concrete Reality)**:
  - Close MEMORY-ACCESS-INVARIANTS-20261115 and MUTATION-QUEUE-OVERFLOW-20261115 with their committed CPU bus guards shared Chip RAM budget fatal mutation preflights and native/browser failure reporting
  - Close GUI-MONITOR-IMAGE-FAILURE-20261116 while preserving both source histories and one effective upload synchronization fix
  - Integrate the delivered GRAPHIFY-REFRESH-PERFORMANCE-20261116 path memoization and caller updates while retaining its active plan and source checkout for unfinished reference-layer and preflight work
  - Close TEST-RUNTIME-AND-VALUE-20261116 with the approved presentation-test retirement shorter audio captures scoped tooling contracts and generated timing data
  - Close VERIFICATION-LOCK-SIMPLIFICATION-20261116 and PARALLEL-TEST-EXECUTION-PILOT-20261116 with one checkout lock bounded concurrent groups and nextest resource limits
  - Close git-close-task-batch-integration-20261116 with immutable source selections conditional all-source publication and independently retryable cleanup
  - Preserve f38576a and all eleven original task commits through local assembly merges 1152b01 3abe385 50cf735 8077e0c 2d86aef 5f6467a and cddb03b awaiting this combined verification
  - Reconcile overlapping timing and lock changes and retain native disk snapshot requirements alongside CPU admission state; migrate the native disk DMA fixture to explicit producer labels and observational debug reads without changing assertions
  - Remove completed plan copies and integrated source checkouts only after accepted local publication while retaining DOCUMENTATION-KNOWLEDGE-PRUNING-20261116 and unfinished Graphify work
- **Architectural Rationale & Trade-Offs**:
  - Verify one complete candidate against immutable target f38576a9d0cdad720b731f11c00531cd78ea16a0 rather than treating source-only results as acceptance
  - Preserve approved test retirements without restoring obsolete Windows process-ownership code or tests
  - Distinguish concurrent command durations from invocation wall time and preserve failed missing executed and reused report rows
  - No roadmap stage completion or remote operation is implied
- **Verification & Test Results**:
  - Pinned FPGA manifests and hashes verified in primary and integration checkouts
  - Focused continuation contracts passed 46 tests after correcting timer propagation across the entry-point lock wrapper
  - Initial focused execution exposed that combined wrapper defect and failed before correction
  - Added report assertions for concurrent failure continuation reuse and skipped coverage export; publish the rewritten lock fixture PID atomically and keep diagnostic report Git fixtures and platform-version mocks isolated
  - Canonical combined commit verification coverage-span review vault audit graph refresh and four independent approvals are pending and their final outcome belongs in the integration commit
---

### [2026-11-16 15:23 CET] — DOCUMENTATION-KNOWLEDGE-PRUNING-20261116: retain valuable knowledge and remove code narration
- **Affected Subsystems**:
  - `Agent governance`
  - `Design specifications`
  - `Reader and tool documentation`
  - `Rust source documentation`
  - `Roadmap and engineering history`
- **What Was Changed (The Concrete Reality)**:
  - Adopt one documentation-value criterion and reconcile its review and enrichment consumers
  - Remove duplicate field and module catalogs plus copied procedures while retaining hardware contracts and rationale
  - Correct CPU-history and evidence claims and preserve the known in-flight restoration gap
  - Replace Roadmap 5.2 timing assumptions with the owning Agnus and DMA specifications under explicit user approval
  - Preserve all 89 historical changelog records and restore chronological order
- **Architectural Rationale & Trade-Offs**:
  - Retain independent hardware evidence and costly knowledge without transcribing implementation
  - Keep source books conversion prompts executable code test bodies and protected references unchanged
- **Verification & Test Results**:
  - Focused vault checks passed for 35 specifications and 1069 links
  - Roadmap structure formatting and configured semantic-sync checks passed
  - Exact comparison preserves seven hardware columns across 256 register rows and executable text plus fenced examples in 21 Rust files
  - Local link and anchor checks passed for 1208 links after the approved roadmap correction
  - Canonical commit verification and four independent review lanes are pending
  - No runtime repair hardware certification or milestone closure is claimed
  - The first commit preflight found two dictionary entries and the AGENTS byte ceiling; shortened routing and resolved language findings under the standing language policy
  - A later preparation run passed preliminary checks and Clippy but its browser build was interrupted to correct structural-test ownership wording before final verification
---

### [2026-11-16 17:03 CET] — Update KaTeX to patched version 0.18.2
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Pin KaTeX 0.18.2 in the npm manifest and lockfile
  - Update the renderer cache version to match the installed math library
- **Architectural Rationale & Trade-Offs**:
  - Address the [KaTeX security advisory](https://github.com/advisories/GHSA-238p-pmpm-9mq7) and Dependabot alert 1
- **Verification & Test Results**:
  - cargo build --workspace --locked passed with unused dependency warnings for bitflags and serde_json
  - tools/web.ps1 build passed
  - Python byte compilation and Node syntax checks for the installed KaTeX JavaScript and module passed
  - Automated tests independent reviews and the canonical commit profile were skipped at the explicit user request
---

### [2026-11-16 17:05 CET] — Clarify game library, input and player roadmap
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Unify reusable input mappings with named emulator configurations
  - define desktop directory games and player controls
  - separate current manual WASM setup from future bundles and persistent save catalogs
  - Update related roadmap terminology and recognize the Dependabot name in the English vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Record the selected frontend scope and verification gates without claiming implementation or milestone completion
- **Verification & Test Results**:
  - Roadmap structure check and git diff --check passed
  - no production code changed
  - Initial canonical preflight found vocabulary in the preceding diary entry; link its advisory identifier and register the established service name before retrying
---

### [2026-11-16 17:16 CET] — Update Pillow to patched version 13.0.0
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
  - `tools/bootstrap/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Pin Pillow 13.0.0 in both converter requirement files
- **Architectural Rationale & Trade-Offs**:
  - Address the MPO and DCX decompression-bomb protection bypass reported in Dependabot alert 39
  - Keep the shared Python installation unchanged because installed Docling and FastEmbed packages restrict Pillow to versions below 13
- **Verification & Test Results**:
  - cargo build --workspace --locked passed with existing unused dependency warnings for bitflags and serde_json
  - tools/web.ps1 build passed
  - Python compilation passed for 93 tracked bootstrap source files
  - Book image smoke checks could not run because this checkout contains no local book PNG assets
  - Automated test suites and independent reviews and the canonical commit profile were skipped at the explicit user request
---

### [2026-11-16 18:51 CET] — Separate project and reference Graphify datasets
- **Affected Subsystems**:
  - `tools/graphify`
  - `tools/graphify_query.py`
  - `tools/bootstrap`
  - `tools/worktree.ps1`
  - `agent instructions`
- **What Was Changed (The Concrete Reality)**:
  - Complete GRAPHIFY-TWO-GRAPHS-20261116 with independent project and reference roots and outputs
  - make bootstrap create or refresh both datasets while preparation and worktree refresh only project code
  - require explicit query selection and source provenance
  - retain project documentation and host-agent semantic extraction with attributed caching
  - exclude vAmigaTS by user decision and retire the copied general skill pipelines
  - Run native traversal through a process-local adapter that suppresses the pinned CLI query stamp and query log without changing the installed package
- **Architectural Rationale & Trade-Offs**:
  - Keep project relationships separate from imported models without duplicating the installed Graphify package or adding an external LLM service
  - preserve physical worktree isolation and tracked reference configuration
  - unchanged-project preflight remains separate future work
- **Verification & Test Results**:
  - Fresh project and reference structural builds succeeded
  - focused real Graphify corpus tests passed for Git-ignore settings and semantic preservation and source removal and output isolation
  - bootstrap contract tests passed
  - reference parser reported 24 files with syntax errors and partial extraction
  - canonical verification and independent review remain preparation gates
  - no performance or hardware correctness claim
---

### [2026-11-16 19:53 CET] — GAME-LIBRARY-INPUT-PLAYER-20261116: desktop library and player task closure
- **Affected Subsystems**:
  - `crates/gui`
  - `crates/cpu`
  - `crates/save_state`
  - `crates/debugger`
  - `crates/floppy`
  - `crates/game_ports`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Deliver desktop game discovery and Add game / Edit game with copied ADF media and JPEG/PNG covers
  - Add named hardware and input configurations with explicit independent Amiga port routing
  - Provide a compact player panel with four drives and one session shared with Debug
  - Preserve active CPU execution-sequence identity in version 10 snapshots with game/media associations and durable desktop progress
  - Retain manual browser setup and explicit snapshot import/export with persistent settings but no media cache
  - Prepare the user-selected external library with Amiga Test Kit, Workbench, Cadaver and a vAmigaTS diagnostic, preserving original source media
- **Architectural Rationale & Trade-Offs**:
  - The user requested a usable setup-to-disk path and library/player prototype, then authorized task closure with a rebase onto the current local master
  - Static microcode arrays and host pointers are reconstructed from a serialized execution identity because opcode decoding alone loses redirected branch and exception sequences
  - Host controls remain outside deterministic CCK execution and both game ports release on loss of capture or device availability
  - Record only the delivered task scope, retaining broader roadmap acceptance and deferred physical-controller trials
- **Verification & Test Results**:
  - Before integration, the combined ordinary Cargo run passed 551 tests, and the later library/player GUI regressions passed 53 cases
  - Native cross1 booted twice with identical milestones and a no-disk negative control, and Amiga Test Kit 1.21 reached its main menu
  - Real-guest snapshots during floppy DMA resumed identically for 20000 CCK and the browser export/import retained exact CPU and CCK values
  - Native and actual WASM builds, focused quality gates and wide/narrow visual checks passed before the rebase
  - Source checkpoints ca665ac9e5a011db9392d0df2dd5ffd23542954f and 853f7aba7b9ae3ee54e11ab7f2c324c8e650ab6f were explicitly unverified integration working commits and became 1f9976389b6fa6c750896398da60323b4cb4d95a and aa3ab645a400fd7ec84a7414bb0cf88d0dd0a786 after the user-approved rebase onto de52e8efe3062e818cc1ec422e984e057eb7b72b
  - Final combined acceptance is gated by the canonical commit profile, changed-code coverage review and four independent lanes, with final verdicts recorded in the closing commit
  - Physical controller and real permission-dialog trials remain deferred, Workbench rendering artifacts also occur on the baseline, and complete Cadaver startup is unverified
  - User approved the complete-state CPU architecture-test fixture and exactly Arc/Mutex permissions for the existing browser worker bootstrap
  - No milestone or full SingleStep claim is made, and no remote publication was authorized

  - Integration review reproduced and corrected stale host bindings after controller reassignment, durable game-slot menu availability, missing replacement confirmation, stale ROM selection and shortcuts escaping focused host text.
  - Focused correction checks passed 55 GUI cases and three CPU continuation cases, including interrupt entry with wait states, short untaken branches and condition-true DBcc. Mouse port/sensitivity and keyboard-alias release assertions were added. The original slot-menu implementation was separately confirmed to report a persisted slot as empty.
  - Direct and recent-ROM file operations now use the same reset confirmation, while invalid ROMs retain their original non-mutating error path. Tests preserve the existing load/history assertions after explicit confirmation.
  - Confirmation rendering was checked across repeated GUI layout passes and visually inspected at 1024x600. The ROM/file regression group passed all 13 cases after the correction.
  - The combined run exposed an older F5 fixture conflicting with focused-text priority. The test now verifies that F5 leaves the editor paused, then starts through Run and retains every frozen-editor assertion; all five frozen-inspection cases passed.
  - Coverage review added behavioral protection for malformed library metadata and duplicate IDs, concurrent edit preservation, configuration conflicts and creation controls, selected-game Continue, disk switch/ejection acknowledgments, manual launch readiness and atomic rejection of incompatible player restores. A new assertion reproduced missing arrow-key frontend conflict detection; explicit physical-name normalization fixes all four directions.
---

### [2026-11-16 21:46 CET] — Correct final bitplane DMA group for Workbench rows
- **Affected Subsystems**:
  - `agnus`
  - `DMA specification`
- **What Was Changed (The Concrete Reality)**:
  - Request the final complete fetch group at the boundary after DDFSTOP and apply modulo only on its last word
  - Add ROM-free component regressions for Kickstart and Workbench across consecutive rows with zero and signed modulos
  - Correct the existing mid-group stop expectation with explicit user approval
- **Architectural Rationale & Trade-Offs**:
  - Workbench used 76 rather than 80 bytes per row
  - The selected final-group rule follows vAmiga and retains the hardware evidence limits in the DMA specification
- **Verification & Test Results**:
  - On the original code the Kickstart regression passed and the Workbench regression failed
  - cargo test -p agnus passed after the repair
  - Native Workbench text and window borders are straight after the repair
  - Native Kickstart and Cross1 captures are byte-identical before and after the repair
---

### [2026-11-16 22:10 CET] — Compare rejected-ROM snapshots independently of save creation time
- **Affected Subsystems**:
  - `gui tests`
- **What Was Changed (The Concrete Reality)**:
  - Deserialize both snapshots and normalize only their creation timestamp before comparing their complete values
  - Preserve machine state and all other snapshot metadata assertions
- **Architectural Rationale & Trade-Offs**:
  - Separate saves can cross a UTC second while the machine remains unchanged
  - The user explicitly approved this single-field comparison exception after the canonical test exposed the mismatch
- **Verification & Test Results**:
  - The previous canonical failure differed only in header timestamp
  - The corrected focused host runtime test passed
---

### [2026-11-16 22:56 CET] — RTC-WB13-01: restore Workbench clock detection and close the task
- **Affected Subsystems**:
  - `crates/rtc`
  - `crates/machine_loop/tests/test_rtc.rs`
  - `Obsidian/Amiga/Design/RTC.md`
- **What Was Changed (The Concrete Reality)**:
  - Start the installed MSM6242B with HOLD clear so readable seconds advance before guest initialization
  - Add a machine-bus startup regression and explicitly engage HOLD in the existing calendar-write fixture
  - Document seconds-based detection and the SetClock 34.3 update for original Workbench dates after 1999
  - Record working commit 38f8fd624d36260af86edf9bb84f8486e389af4a as awaiting combined integration verification against 34f04bf1585c329d886f895650a5e4ed28c84ec4
- **Architectural Rationale & Trade-Offs**:
  - The clock was present but its startup HOLD latch kept the seconds register fixed and caused SetClock to report no battery clock
  - Preserve two BCD year digits and use the guest SetClock update for its independent year 2000 defect
  - The user approved local integration into master and removal of rtc-workbench-detection after verification, without remote publication
- **Verification & Test Results**:
  - The new startup regression failed on the original code and passed after the fix
  - cargo test -p rtc -p machine_loop -p memory_bus passed, including 20 RTC tests
  - cargo check -p rtc --target wasm32-unknown-unknown passed
  - Paced headless boots with local Kickstart 1.3 and Workbench 34.20 reproduced the original missing-clock message and read the date after the fix
  - A diagnostic disk copy with SetClock 34.3 displayed 2026 in this emulator, and WinUAE 6.0.3 displayed 2026 with both SetClock versions
  - Canonical integration verification and four independent review lanes remain pending, and no roadmap milestone is claimed
---

### [2026-11-16 23:15 CET] — Make input capture explicit and reserve F12
- **Affected Subsystems**:
  - `crates/gui`
  - `docs/debugger.md`
  - `docs/game-library.md`
  - `Obsidian/Amiga/Design/Application/GUI.md`
- **What Was Changed (The Concrete Reality)**:
  - Reserve F12 for Game/Debug
  - replace the default Escape release with fixed middle mouse and an optional unassigned keyboard key
  - remove Ctrl+R
  - separate gamepad gameplay from pointer capture
  - retain controller contacts on capture release and release all inputs on focus loss
  - show shared capture guidance
- **Architectural Rationale & Trade-Offs**:
  - Keep Amiga keys available and allow controllers to play without capturing the desktop pointer
- **Verification & Test Results**:
  - GUI suite passed before final review corrections
  - focused capture and view suites passed after text-focus and shared-guidance corrections
  - browser adapter contention regression passed
  - actual WASM build and vault audit passed
  - configuration screenshot rendered and inspected. Canonical verification and final independent approvals pending. Physical controllers and Linux/macOS were not exercised.
---

### [2026-11-16 22:56 CET] — Show disk activity in the player panel
- **Affected Subsystems**:
  - `crates/config`
  - `crates/floppy`
  - `crates/paula`
  - `crates/machine_loop`
  - `crates/gui`
  - `Obsidian/Amiga/Design/Application/GUI.md`
- **What Was Changed (The Concrete Reality)**:
  - Replaced motor-based Active text with a fixed-position green activity light.
  - Retained head steps and consumed disk data between host publications.
  - Held the light for 120 ms and cleared its baseline on restore or any drive reset, including guest RESET and stepped keyboard reset.
  - Shared the existing drive-count limit through config without adding dependencies.
  - Added behavioral protection for head steps, byte reads, DMA reads, serialized writes, timing and reset/restore.
  - Added the valid egui API namespace epaint to the programming vocabulary.
- **Architectural Rationale & Trade-Offs**:
  - Confirm real drive work without a track display or a light held on by the spindle motor.
  - Activity and reset counters and the pending controller observation are required serialized fields, so older snapshots without them are unsupported.
- **Verification & Test Results**:
  - Focused floppy, Paula and GUI crate tests passed, as did machine-loop activity and autonomous ADF DMA suites, architecture checks and the browser build.
  - The revised GUI activity suite passed guest RESET and stepped keyboard reset checks.
  - Headless wide and compact game panels were visually inspected.
  - Canonical batch coverage and independent review are pending.
---

### [2026-11-17 00:28 CET] — Integrate input capture and floppy activity
- **Affected Subsystems**:
  - `crates/gui`
  - `crates/floppy`
  - `crates/paula`
  - `crates/machine_loop`
  - `GUI specification`
- **What Was Changed (The Concrete Reality)**:
  - Integrate task input-capture at bbdd32cab335d5a17317a8ead7e54ee66adee133 and task floppy-activity at 0cfab14e55db4f7bbab1ab408fed66b0d33353aa against primary baseline 1cc643c1833acbda53d0bfc78507f7a46ad60ab0
  - Preserve both features and the primary RTC fix while resolving overlapping GUI requirements and diary entries
  - Record intermediate merges 4612d23 and ca0d45c as awaiting combined verification
  - Retain both task branches, worktrees and the input-capture working plan under the approved merge-only scope
  - Invalidate cached controller contacts on every runtime generation change and preserve fresh gamepad edges admitted after a pending full release while discarding keyboard joystick presses on capture loss
  - Retry a contended fatal-test command submission without weakening its failure assertions
  - Explain the physical motor-controlled drive LED separately from GUI event feedback in the floppy specification
- **Architectural Rationale & Trade-Offs**:
  - Assemble both source histories before one combined acceptance pass
  - preserve pending primary roadmap edits separately during publication
  - no remote publication or milestone closure
- **Verification & Test Results**:
  - Source commit messages record passing source-only canonical checks and four review approvals for each feature
  - merge conflict resolutions and git diff --check passed
  - Both controller defects failed focused regressions before the runtime correction, including separate keyboard-reset and running-restore reproductions
  - All 45 focused input, runtime, player capture and mapping tests passed after correction; the Node adapter regression also passed
  - A further regression reproduced a keyboard joystick press retained by the initial queue correction; all 11 input tests passed after distinguishing gamepad ports
  - The vault audit passed all 35 specifications and 1078 links
  - Add the HRM-confirmed DSKMOTOR signal name to reviewed hardware vocabulary after the preliminary language gate identified it
  - The first combined profile failed at a contended fatal-test command submission; final combined canonical verification and four independent review lanes are pending
  - physical controller and live browser interaction remain outside this verification
---

### [2026-11-17 00:26 CET] — Recover the DIW edge and store a fixed video rectangle
- **Affected Subsystems**:
  - `Denise`
  - `frame_builder`
  - `machine_loop`
  - `save_state`
  - `GUI`
- **What Was Changed (The Concrete Reality)**:
  - VIDEO-FRAME-RECT-20261116: compare DIW against the delayed output pair and preserve H0
  - store PAL 720 x 568 and selected NTSC 720 x 480 directly at a 720-sample stride, without a second cropped image
  - retain parity, clear missing tails and absent rows including blank lines between painted lines, select geometry at construction and retain it on reset
  - advance save schema to 11 with no compatibility parser
  - adapt component, machine, preview, exchange and texture regressions to the fixed rectangle
- **Architectural Rationale & Trade-Offs**:
  - The original renderer discards the first pair of an ordinary bitplane word stream before composition. The independent reproduction fails on the original renderer and passes after the phase correction. Presentation dimensions are a project choice
  - they do not claim analog PAL/NTSC raster dimensions or silicon edge calibration.
- **Verification & Test Results**:
  - Focused Denise, frame-builder, Agnus fetch, machine display/state, save persistence and GUI exchange/scaling checks passed during development
  - the original-renderer word-stream regression fails at its first expected sample
  - Workbench Clock width is 170 and its directory remains 428 samples, with exactly 420 recovered samples in the old saved scene
  - final PAL equals the corrected old image cropped at (52, 4), and Cross1 matches the previous corrected-DMA crop across three identical fields
  - PAL and NTSC Kickstart captures are 720 x 568 and 720 x 480
  - vault audit passed 35 Design notes and 1079 links
  - a headless Game view shows the complete rectangle under uniform Fit scaling. WinUAE 6.0.3 displayed the selected NTSC configuration at 59.8 Hz
  - the first canonical attempt exposed one GUI fixture still computing the retired raster size; its expectation now uses the independently specified fixed dimensions
  - independent review reproduced old pixels on a blank middle line; the corrected mapping clears those rows after completing the previous row, preserving opposite interlaced parity
  - reset-profile and blank-row regressions pass, and coverage review restored explicit black queries outside the rectangle
  - NTSC origin is confirmed against WinUAE 6.0.3 before-filtering output: the selected 720 x 480 rectangle at (16, 0) in its 752 x 484 native capture matches all 345600 RGB samples exactly, without scaling, tolerance or masks
  - canonical code verification passed 837 ordinary tests (3 existing skips), 9 standard-harness tests, native Clippy and the actual WASM build; changed-runtime coverage has no unresolved in-scope gap
  - the selected presentation origins are receiver CCK 49, PAL line 28 and NTSC line 21; analog timing calibration and broader native-display fidelity remain outside this task
  - no milestone or broader roadmap closure is claimed.
---

### [2026-11-17 00:58 CET] — Integrate fixed framebuffer geometry with the current frontend
- **Affected Subsystems**:
  - `Denise`
  - `frame_builder`
  - `machine_loop`
  - `save_state`
  - `GUI`
- **What Was Changed (The Concrete Reality)**:
  - Integrate VIDEO-FRAME-RECT-20261116 at 5f01bfcdb15180494c51c09b6c25ef70c287507f with primary baseline 51379e3d44ed8fb9cfa4892c2984434c62a2d24a
  - preserve fixed PAL and NTSC geometry together with explicit input capture, F12 and floppy activity
  - retain both parent diary histories and regenerate the timing report
  - reconcile every tracked Markdown path changed from the common base
  - retain schema 11 and the existing task closure record
  - compare the DIW receiver restart value before the delayed output phase, preserving a programmed horizontal start of 2
  - observe gamepad state directly at an idle owner boundary and retry inactive GUI frames until focus-loss release is visible, without a focused redraw or a fixed sleep
- **Architectural Rationale & Trade-Offs**:
  - Preserve the source histories and accept the combined result before conditional local publication and approved worktree removal
  - unrelated primary roadmap edits remain outside this integration
- **Verification & Test Results**:
  - Source commit records 837 passing ordinary tests, 3 existing skips, 9 harness tests, four review approvals and the exact NTSC WinUAE comparison
  - merge code paths combined without conflicts and git diff --check passed
  - the first combined profile passed 852 ordinary tests, 3 existing skips and 9 standard-harness tests
  - independent review identified the skipped DIW origin comparison; a new start-2 regression failed on the candidate and all four display-window tests passed after the correction, including both resolutions and consecutive lines
  - a later combined attempt failed the gamepad focus-loss test after its fixed 30 ms sleep; direct snapshot observation and inactive-frame retries preserve the release assertion, and all six focused capture tests passed during diagnosis
  - final canonical integration verification and independent integration reviews remain pending
  - no milestone or remote publication is claimed
---

### [2026-11-17 01:25 CET] — Plan optional floppy loading and disk controls
- **Affected Subsystems**:
  - `Roadmap`
- **What Was Changed (The Concrete Reality)**:
  - Add turbo floppy mode and deferred automatic disk swapping exploration
  - Include disk selection and swapping in configurable gamepad actions
- **Architectural Rationale & Trade-Offs**:
  - Record future frontend work without selecting an acceleration or recognition mechanism or claiming implementation
- **Verification & Test Results**:
  - Reviewed the roadmap diff and section anchors
  - git diff --check passed before this entry
  - No runtime code or acceptance references changed
---

### [2026-11-17 02:07 CET] — Observe drive activity at chipset source events
- **Affected Subsystems**:
  - `CIA-B board routing`
  - `Paula disk observations`
  - `Floppy controls`
  - `GUI drive indicator`
  - `SaveState`
- **What Was Changed (The Concrete Reality)**:
  - DISK-ACTIVITY-20261117: record effective CIA-B control events and accepted Paula transfers in wrapping per-drive observations
  - remove both per-CCK activity collectors and presentation bookkeeping from Floppy
  - share connector control decoding with drive mechanics and establish next-CCK transfer recipients after the CPU phase
  - retain selected/running sustained indication and the 120 ms hold with reset/load baselines
  - use save schema 12 without compatibility parsing
  - update owning specifications and add production-path regressions and an inspector scenario
  - retain the documented standalone subsystem stepping API under the user-approved single-symbol host/spec exemption in the code-quality checker
- **Architectural Rationale & Trade-Offs**:
  - Preserve short events and correct drive attribution across publication gaps without per-CCK activity collection, duplicated writable mechanics, chip callbacks or new dependencies
  - observation metadata does not alter guest registers, physical signal phases or media-write support
- **Verification & Test Results**:
  - Original machine and GUI activity suites passed before implementation
  - final full CIA, Floppy, Paula, SaveState, Debugger, GUI and Machine Loop suites passed, as did the actual WASM build and git diff --check
  - GUI visual capture and integration assertions verify sustained indication, hold timing, fixed bounds, restore and reset
  - six samples per version and workload in A-B-B-A order gave idle median 178.967 to 180.621 FPS and active byte-read median 171.221 to 172.104 FPS, within observed variation
  - no whole-machine speedup or regression is established, and browser performance and physical hardware fidelity remain unverified
  - canonical batch gates, changed-code coverage review and four independent review lanes are pending
---

### [2026-11-17 02:58 CET] — Commit configuration editing and current controls on progress restore
- **Affected Subsystems**:
  - `GUI`
  - `debugger`
  - `keyboard`
  - `save_state`
- **What Was Changed (The Concrete Reality)**:
  - Complete CONFIG-INPUT-STATE-20261116: isolate configuration and shared-mapping drafts behind Save and Cancel, including unsaved navigation and committed-only persistence
  - Separate launch choices, port attachment and host input values while retaining the current flat RON settings contract and stable configuration IDs
  - Apply changed endpoints at a stopped owner boundary and restore progress with current session controls across library, file and quick-slot loads
  - Preserve exact core restoration and observational capture, receiver-owned registers, mouse phase, embedded media and live RTC independence
  - Retain deferred keyboard releases behind a saturated ten-code ring and preserve retry and release-before-press ordering, recording this state in snapshot schema 12
- **Architectural Rationale & Trade-Offs**:
  - Changing controls must allow the same game to continue without reset or replaying launch hardware
  - A reproduced full-ring release lost key-up and emitted only overflow, requiring serializable pending releases rather than protocol flushing
  - Earlier snapshots remain unsupported under the pre-release contract, while existing user configuration data stays readable without migration
- **Verification & Test Results**:
  - Original port-rejection regression passed before the approved acceptance update, and the full-ring keyboard reproduction failed on the original controller
  - Full GUI crate tests and subsequent affected editor, import, capture, restore, runtime input and library suites passed
  - Keyboard, save_state and debugger ordinary tests and machine keyboard/CIA integration passed, including synthetic CPU observation of key-up from partial and full-ring snapshots
  - Architecture, Clippy and the actual WASM application build passed, with existing unused-dependency manifest warnings retained outside scope
  - Chromium 157.0.8103.51 verified Cancel persistence, stopped port changes and importing older endpoint types with current controls, preserving CCK 2549684 and reporting no page or worker failures
  - Inspected 1280x800 and 480x640 editor captures and the narrow unsaved dialog
  - Independent review reproduced missing controller validation on Debug and dirty Discard entry, then shared validated session navigation and retained editor origin fixed both regressions
  - Retired the unused LoadSnapshot owner command and implicit attachment cache, migrating existing tests to explicit restore requests without weakening assertions
  - Added rejected Save coverage for referenced and unused blank-name mappings, catalog identity and mapping-field validation, unchanged machine/control state and Cancel recovery
  - A parallel focused runtime run exceeded its existing GUI wait limit under host pressure, while the unchanged checks passed when serialized
  - Initial canonical coverage compilation failed with memory allocation and Windows paging-file errors before test execution; the full suite is being retried with one Cargo build job and unchanged acceptance criteria
  - Canonical commit verification, changed-code coverage review and final independent review approvals remain pending at entry preparation
  - Physical controllers, private firmware boot and milestone verification were not run
---

### [2026-11-17 10:47 CET] — Integrate current controls with chipset drive activity
- **Affected Subsystems**:
  - `GUI`
  - `keyboard`
  - `SaveState`
  - `disk activity`
- **What Was Changed (The Concrete Reality)**:
  - Close CONFIG-INPUT-STATE-20261116 by integrating configuration-gameports-save-state into local master
  - preserve the configuration and disk-activity commits through a real merge
  - retain both restore and mount regressions and both inspector scenarios
  - use schema 13 for the combined pending-keyboard-release and chipset-observation layout
- **Architectural Rationale & Trade-Offs**:
  - The two independent branches used schema 12 for incompatible state layouts
  - current-control reconciliation must preserve captured disk observations and physical delivery phases
  - retain existing settings data while unsupported snapshots receive an explicit version rejection
  - User explicitly approved the single test-file std::thread::spawn permission needed to verify bounded input contention and deferred draft publication
- **Verification & Test Results**:
  - Source commit 902fc62 passed canonical attempt 2e60da62d2f64a47854c888126948086 with 875 ordinary tests passed and three skipped plus four independent approvals before integration
  - merge resolutions retain both parent behaviors and migrate the additional activity test to explicit RestorePlayer
  - combined canonical verification and independent review are pending at preparation
  - physical controllers and private firmware remain unverified and this integration does not close a roadmap milestone

---

### [2026-11-17 14:39 CET] — Close window keyboard routing and integrate current configuration controls
- **Affected Subsystems**:
  - `GUI host input`
  - `native window and browser adapters`
  - `frontend configuration`
  - `roadmap`
- **What Was Changed (The Concrete Reality)**:
  - KEYBOARD-DIAGNOSTICS-20261117: replace global rdev keyboard and mouse listeners with application-window input and one native/browser keyboard routing policy
  - use upstream eframe/egui 0.32.3 through the public external-event-loop API without vendored patches
  - remove the temporary keyboard terminal and all added tracing
  - retain physical modifier sides, keypad identities, command edges, session generations, bounded queues and gilrs gamepads
  - reconcile the current controls.host configuration structure, draft editor and schema 13 user-load release policy from master
  - preserve accepted frontend bindings without guest counterparts and retain queued keys across host activation/publication delays
  - cover obsolete generations, physical held/released keys, focus-loss ordering, inbox and cross-frame retry overflow through the GUI-to-owner path
  - verify full frontend modifier/command routing, GUI zoom and stopped Developer recording without starting the machine
  - expose the existing native key enqueue boundary for typed metadata and bounded-buffer regressions
  - record deferred profiler-guided CPU reduction in roadmap 8.5
  - working commits e162dae and 4c3eb88 await this combined verification
- **Architectural Rationale & Trade-Offs**:
  - Preserve master history and current configuration/save behavior while limiting keyboard observation to the application window
  - host adapters stay outside the CCK-driven hardware models
  - the user approved local integration, closure and removal of both keyboard experiment worktrees, including the discarded older rdev experiment
- **Verification & Test Results**:
  - Pre-integration source: 120 selected Rust tests, browser adapter Node test and native/WASM builds passed
  - the user confirmed Workbench Notepad and Cadaver on the preceding window-input experiment
  - merged GUI and all tests compile after configuration and egui API reconciliation
  - independent review reproduced lost Home/End/F11/PageDown actions; its new regression failed before the frontend-position correction and passed afterward
  - the new ingress regression reproduced first-key loss before host activation; the corrected activation/owner-readiness retry and both ingress regressions passed
  - the default cargo-llvm-cov 0.9.1 wrapper omitted GUI instrumentation despite running its tests; an isolated documented non-wrapper full-GUI measurement supplied actual GUI coverage without changing enforcement
  - GUI settlement now includes pending host mapping/capture activation; synthetic gamepad transitions after text focus resume wait for that acknowledgment while preserving all original gameplay assertions
  - final upstream-adapter physical typing, physical gamepad freshness and Linux/macOS execution remain unverified
  - canonical combined results and the four scoped review verdicts are required before publication and recorded by the final integration commit
  - no roadmap implementation milestone or remote publication is claimed

---

### [2026-11-17 15:00 CET] — Simplify memory admission with CCK timestamps
- **Affected Subsystems**:
  - `CPU`
  - `physical memory`
  - `Agnus DMA`
  - `machine loop`
  - `save state`
- **What Was Changed (The Concrete Reality)**:
  - MEMORY-INVARIANT-SIMPLIFICATION-20261117: replace CPU admission countdowns and Chip RAM transfer descriptions with access timestamps
  - normalize write callbacks by one CCK
  - preserve duplicate attempts, wait identity and read completion
  - remove owner and PC plumbing
  - preserve access age across reset and snapshots
  - advance save schema to 14
  - apply explicitly approved reduced diagnostic assertions
- **Architectural Rationale & Trade-Offs**:
  - Keep shared CPU/DMA storage admission and two-CCK CPU cycles with less successful-path bookkeeping
  - pending transfer identity remains functional
  - mutation queue diagnostics remain unchanged
- **Verification & Test Results**:
  - Focused CPU, physical_memory, agnus and machine_loop tests pass
  - architecture and vault checks pass
  - twelve interleaved release samples per version show median 8.379795 to 8.184074 ms/field, 119.335 to 122.189 fields/s, throughput +2.39%
  - CCK counts, guest progress, activity and frame bytes agree
  - measurement excludes GUI and pacing but retains pixel/audio generation
  - final canonical verification and independent review pending
  - browser speed, isolated Fast RAM and real-game compatibility unmeasured
---

### [2026-11-17 15:13 CET] — Preserve GUI reply ownership in the host-input fixture
- **Affected Subsystems**:
  - `GUI host-runtime tests`
- **What Was Changed (The Concrete Reality)**:
  - Before direct snapshot polling, wait for the GUI to process its outstanding replies while retaining the requested window focus
  - keep all input assertions and the existing deadline
- **Architectural Rationale & Trade-Offs**:
  - The direct runtime helper drains all replies and discards unrelated IDs
  - entering it while AppHost owns a pending UTC command can strand bookkeeping and prevent later settling
- **Verification & Test Results**:
  - Two canonical coverage attempts timed out in the unchanged input case
  - normal and instrumented focused cases passed
  - a controlled temporary reproduction demonstrated a GUI reply consumed by direct snapshot polling and a permanently non-idle AppHost
  - the diagnostic was removed
  - final combined batch validation pending
---

### [2026-11-17 16:23 CET] — Close memory invariant simplification and integrate current window input
- **Affected Subsystems**:
  - `CPU`
  - `Chip RAM`
  - `machine loop`
  - `snapshots`
  - `GUI input fixtures`
- **What Was Changed (The Concrete Reality)**:
  - Close MEMORY-INVARIANT-SIMPLIFICATION-20261117 by integrating task commits 68d908e and fb6c76f with master b17f9b3 through a real merge
  - retain normalized CPU cycle-start spacing and shared Chip RAM timestamps
  - preserve the GUI reply-ownership fixture repair, current window keyboard routing and deferred profiler roadmap work
  - combine both histories in the rolling changelog and regenerate runtime evidence
  - migrate the browser workflow's snapshot assertion to schema 14 without changing its other checks
- **Architectural Rationale & Trade-Offs**:
  - Source-only verification does not establish the combined result after master advances
  - preserve all original commits and both sets of behavior, then conditionally publish the exact approved target transition
- **Verification & Test Results**:
  - Task source fb6c76f passed canonical verification with 890 ordinary tests and three existing private-fixture skips, WebAssembly, Clippy and four independent reviews
  - original native PAL measurements showed median throughput +2.39 percent at matching guest progress with GUI and pacing excluded
  - integration verification and four fresh reviews are pending
  - the initial integrated run passed 900 ordinary tests; stale coverage executables exceeded the Windows export command limit, so workspace coverage artifacts were refreshed before repeating verification
  - source-only performance is historical evidence and was not remeasured for the integrated frontend
  - full SingleStep, hardware captures, private firmware and browser performance are not established
---

### [2026-11-17 20:27 CET] — Separate the manual TA workflow into bounded operations
- **Affected Subsystems**:
  - `.agents`
  - `.codex/agents`
  - `tools/workflow`
  - `verification and Graphify consumers`
- **What Was Changed (The Concrete Reality)**:
  - Implement DETERMINISTIC-WORKFLOW-V1-20261116 with seven named roles and 43 atomic TA skills
  - reduce root instructions to shared obligations and selective domain routing
  - move detailed result criteria to review owners while preserving exact exceptions
  - add typed text and JSON handoffs, bounded rounds and normal result validation
  - update moved checker imports and active documentation consumers
- **Architectural Rationale & Trade-Offs**:
  - Keep operation selection in the interactive session and make missing prerequisites return control
  - preserve existing verification and conditional Git publication guards without an automatic lifecycle engine
  - inherited role permissions remain visible behavioral limits, not a claimed sandbox
  - the migration uses a separate candidate worktree and the unchanged source harness under explicit task-scoped authorization
- **Verification & Test Results**:
  - The first actual fresh TA-planner completed one round and its receiver validated both results
  - existing focused suites passed 23 architecture contracts, five timing/selection contracts and 15 Graphify contracts
  - PowerShell parsing and safe invocation checks passed
  - no emulator behavior, hardware/golden references or conversion prompts changed
  - practical coverage is bounded and does not establish a speedup or exhaustive skill coverage
---

### [2026-11-17 22:57 CET] — Explain TA results and offer explicit next choices
- **Affected Subsystems**:
  - `.agents`
  - `tools/workflow`
  - `docs/ai_agents.md`
- **What Was Changed (The Concrete Reality)**:
  - Implement WORKFLOW-NEXT-STEP-GUIDANCE-20261117 on the preserved pending V1 candidate
  - add one receiving-session procedure with outcome, consequence, recommendation and source-bound options
  - preserve worker reports and migrate their recommendation into result schema version 2
  - render exact role, skill, inputs, findings, scope and round budget for selected operations
  - update affected routing and four independent review criteria
- **Architectural Rationale & Trade-Offs**:
  - Interpretation is automatic while the next operation waits for selection
  - current guidance retains exact source and candidate identities without an execution engine
  - stale options and missing inputs remain explicit and criterion approval remains separate from repair authority
  - task-specific trial delegation does not become a future default
- **Verification & Test Results**:
  - Initial tracked diff and all 84 V1 new-file hashes matched the recorded prior candidate
  - Python compilation and both selected schema description commands passed
  - canonical profile preview selected the combined V1 and extension scope
  - practical handoff trials and final combined verification are recorded separately when executed
  - no emulator, golden reference or conversion prompt changes

---

### [2026-11-18 00:08 CET] — Constrain official TA suggestions with a skill matrix
- **Affected Subsystems**:
  - `.agents`
  - `tools/workflow`
  - `docs/ai_agents.md`
- **What Was Changed (The Concrete Reality)**:
  - Implement WORKFLOW-SKILL-MATRIX-20261117 on the preserved pending workflow candidate
  - define all 43 source and destination skills with shared conditions in one authoritative matrix
  - generate a readable matrix and expose source-row lookup through the existing result tool
  - require official guidance to identify its matrix, transition condition and actual evidence basis
  - retain ordinary direct skill requests outside the official recommendation matrix
- **Architectural Rationale & Trade-Offs**:
  - Marked cells identify conditional suggestions while the receiver interprets evidence and waits for selection
  - reuse the existing role mapping and keep worker result validation independent of recommendation membership
  - preserve historical results and distinguish structural validation from meaningful evidence assessment
  - no execution engine, automatic workflow test suite or default permission expansion
- **Verification & Test Results**:
  - Python compilation, guidance schema description, matrix rendering and coverage-row lookup passed
  - the matrix covers 43 skills with 282 populated cells and 39 shared conditions
  - actual read-only trial accepted a directly selected skill outside the matrix and rejected the same pair as an official suggestion
  - the canonical commit profile passed all 19 groups; exact snapshots, independent reviews and evidence reuse are recorded in the task report
  - no emulator, golden reference or conversion prompt changes

---

### [2026-11-18 02:36 CET] — Remove obsolete browser build output
- **Affected Subsystems**:
  - `Browser build`
- **What Was Changed (The Concrete Reality)**:
  - Remove the generated JavaScript, WebAssembly and HTML files from `crates/gui/dist`.
- **Architectural Rationale & Trade-Offs**:
  - The current Trunk configurations and documented launcher build into `.agent/tmp/web-dist`.
- **Verification & Test Results**:
  - Confirmed the configured output paths and checked that the tracked launcher and source files do not reference the obsolete output. Commit verification is pending.
---

### [2026-11-18 09:50 CET] — Close the manual TA workflow tasks
- **Affected Subsystems**:
  - `.agents`
  - `.codex/agents`
  - `tools/workflow`
  - `verification and Graphify consumers`
- **What Was Changed (The Concrete Reality)**:
  - Close DETERMINISTIC-WORKFLOW-V1-20261116, including AGENT-INSTRUCTIONS-SIMPLIFICATION-20261116, WORKFLOW-NEXT-STEP-GUIDANCE-20261117 and WORKFLOW-SKILL-MATRIX-20261117
  - Adopt seven named roles and 45 atomic skills, including agent and skill creation, with structured results, next-step guidance and the official recommendation matrix
  - Record working commit 3f70adc4261910152e6b2318f631902b1a35ba48 as awaiting integration verification
  - Preserve both diary histories and regenerate the timing report for this integration
- **Architectural Rationale & Trade-Offs**:
  - The interactive session selects operations and uses structured results to evaluate recommendations
  - The matrix constrains official recommendations while direct skill selection remains available; Keep the role and skill model defaults already present in the selected worktree
  - Verification and conditional Git publication retain their existing gates
- **Verification & Test Results**:
  - The earlier pending candidate passed 19 required verification groups and exercised bounded role and handoff trials, including rejection of an off-matrix official suggestion
  - The integrated diff passed the Git whitespace check before this entry
  - Previous trial limits include named role dispatch availability, technical permission isolation and untested interruption and exhaustion paths; The present model defaults have no effective runtime evidence from those trials
---

### [2026-11-18 12:05 CET] — Record the execution capture inputs
- **Affected Subsystems**:
  - `Workflow`
- **What Was Changed (The Concrete Reality)**:
  - Recorded the approved execution capture plan, practical test scenarios, environment inventory and delivery decisions.
- **Architectural Rationale & Trade-Offs**:
  - Keep the implementation starting point independently recoverable from Git.
- **Verification & Test Results**:
  - In a limited pilot, one dedicated CLI turn produced fourteen telemetry logs with the expected conversation identifier. The collector retained the allowed control and excluded both rejected controls.
  - The planned implementation scenarios S1 through S5 remain unrun end to end.
---

### [2026-11-18 12:49 CET] — Capture selected workflow executions
- **Affected Subsystems**:
  - `Workflow`
- **What Was Changed (The Concrete Reality)**:
  - Register selected threads and UTC boundaries with a project receiver. Record portable tool inventories and compare observed environments. Preserve scoped session records in a private archive and assess each revision against independent observations.
- **Architectural Rationale & Trade-Offs**:
  - Keep approved inputs in Git and retain enough execution evidence to investigate individual task runs.
- **Verification & Test Results**:
  - The dedicated CLI emitted an explicit skill invocation. Checked root and child spans matched the collector output. The independent fixture retained its archive and both commits after its task worktree was removed.
