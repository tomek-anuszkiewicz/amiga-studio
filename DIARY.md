# Amiga 500 Emulator: Engineering Diary & Architecture Genesis

This document records the complete architectural philosophy, evolutionary history, human-AI co-design methodology, and pivotal engineering decisions behind the cycle-exact Amiga 500 emulator in Rust.

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
3. **Hardware-Accurate Quirks:**
   - **Floating Open Bus:** Unmapped reads return `$FF` / `$FFFF` (simulating the pull-up resistors on the physical Amiga bus), configurable via `set_unmapped_byte()` for flat test harnesses.
   - **Kickstart Overlay:** CIA-A port A bit 0 controls the overlay latch. On cold boot, bank 0 (`$000000-$07FFFF`) mirrors Kickstart ROM until the first hardware write flips the latch to expose physical Chip RAM.
   - **Gary Chip RAM Mirroring & Slow RAM:** Accurate address decoding for standard 512 KB Chip RAM, 512 KB A501 trapdoor Slow RAM (`$C00000`), and Gary custom register mirrors (`$DF0000-$DFFFFF`).

---

## 4. Physical Circuit Simulation: Standalone BLEP Audio Synthesis

A dedicated engineering effort was devoted to modeling the physical analog sound output of the Amiga Paula chip with pristine accuracy:
- **WinUAE Parameterization & Circuit Derivation:** The Band-Limited Step (BLEP) synthesis engine was modeled and parameterized based on verified reference implementations in WinUAE.
- **Pure Mathematical Standalone Module:** Rather than hardcoding magic sound samples, the BLEP synthesis is implemented as an independent, decoupled module containing all the underlying physics and signal-processing mathematics (`crates/paula/src/blep_tables.rs`).
- **Analog RC Component Values:** The step response and frequency roll-off tables are computed directly from the physical component values of the Amiga motherboard circuit—the precise resistance (resistors in ohms) and capacitance (capacitors in farads) forming the analog low-pass filter network (including dynamic CIA-A LED filter switching).
- This ensures alias-free, cycle-accurate analog audio reconstruction directly from first physical principles.

---

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

## 10. Living Chronological Engineering Log & Evolutionary Change History

This section maintains a continuous, granular chronological record of all engineering changes, subsystem modifications, refactorings, and bug fixes across the repository.

Because Git commits are frequently batched, squashed, or merged into higher-level commits during multi-stage worktree development, standard commit messages often do not preserve the full evolutionary context, granular mechanics, or subtle trade-offs considered along the way. This log serves as the authoritative, human-readable chronicle of what was actually built, modified, and verified in each development session.

### Mandatory Entry Schema:
Every repository commit must include a new Section 10 entry for the changes it records, including routine code, test, rule, documentation, and merge commits. At minor roadmap points and milestones, compact settled entries into separate milestone digests while retaining unfinished work. Use the following entry structure:
- **Timestamp & Context**: Date / local time and the active branch / task / PR.
- **Affected Subsystems**: Specific crates, modules, tools, or configuration affected.
- **What Was Changed (The Concrete Reality)**: Detailed technical description of changes, data structures, algorithms, or mechanics implemented.
- **Architectural Rationale & Trade-Offs**: Why this solution was chosen, what alternatives were rejected, and why.
- **Verification & Invariants**: Test suites executed, assertions checked, and proof of correctness.

---

---

### [2026-09-12 23:59 CEST] — Milestone Digest: Disassembler Extraction, Memory Banking & Core Machine Setup
- **Timestamp & Context**: 2026-09-12 — Foundation of the decoupled cycle-exact Amiga 500 emulator architecture.
- **Affected Subsystems**: `crates/disassembler`, `crates/memory_bus`, `crates/physical_memory`, `crates/cpu` (formerly `m68000`), `crates/gui`, `crates/machine_loop`.
- **What Was Changed (The Concrete Reality)**:
  - Extracted M68000 disassembler into a standalone, pure crate with zero runtime allocations in hot paths.
  - Implemented decoupled physical memory banking (`PhysicalMemory`) modeling Chip RAM, Kickstart ROM, Fast RAM, and open-bus unmapped space.
  - Established initial `MachineLoop` with color clock phase synchronization (`CCK1`/`CCK2`) and CPU bus cycle arbitration.
  - Built egui-based Developer Studio GUI skeleton with synchronous state pulling, memory hex dump views, and disassembly inspection windows.
- **Architectural Rationale & Trade-Offs**:
  - *Decoupled Crates:* Separating disassembler and memory banking from CPU and UI guarantees that the emulation core remains system-agnostic and compiles to WASM without dependencies on host graphics or threading.
  - *CCK Execution Model:* Standardizing on 2-phase color clock synchronization provides the foundation for cycle-exact Agnus DMA cycle stealing without inter-chip pointer smuggling.
- **Verification & Invariants**:
  - Automated unit test suites in `tests/` across disassembler and memory banks.
  - Verification of big-endian memory accessors (`read_byte`, `read_word`, `read_long`).

---

### [2026-09-14 23:59 CEST] — Milestone Digest: Language Guardrails, HRM Alignment & CPU Verification
- **Timestamp & Context**: 2026-09-14 — Quality assurance infrastructure, verification suites, and hardware manual alignment.
- **Affected Subsystems**: `crates/test_runner`, `.agents/rules/`, `tools/harness`, `Obsidian/Amiga/Reference/`.
- **What Was Changed (The Concrete Reality)**:
  - Codified the strict English language policy across code, documentation, and agent reasoning.
  - Ingested Commodore Hardware Reference Manuals (HRM) and Amiga Hardware Reference Manual into local reference stores.
  - Built headless verification infrastructure in `crates/test_runner` for Tom Harte single-step CPU tests and Cartesian DMA contention suites.
  - Implemented pre-flight gate scripts (`pre_flight.py`) to enforce formatting, architectural rules, and test coupling.
- **Architectural Rationale & Trade-Offs**:
  - *Headless Test Harness:* Driving test cases via synthetic headless runners avoids dependency on host UI rendering, enabling high-speed verification (< 5s for full suites).
  - *Silicon-Exact Ground Truth:* Grounding execution against established single-step test vectors prevents subtle flag calculation divergences or bus cycle phase misalignments.
- **Verification & Invariants**:
  - `cargo test -p test_runner`: Architecture rule suite and single-step CPU validation tests.
  - `pre_flight.py`: Pre-commit validation of formatting, API coverage, and rule compliance.

---

### [2026-09-16 12:00 CEST] — Milestone Digest: Multimodal Reference Ingestion & 14-Stage PDF-to-Markdown Pipeline
- **Timestamp & Context**: 2026-09-15 to 2026-09-16 — Building the publication-grade technical manual ingestion and conversion pipeline.
- **Affected Subsystems**: `.agents/skills/pdf-to-markdown/`, `tools/pdf/`, `Obsidian/Amiga/Reference/`.
- **What Was Changed (The Concrete Reality)**:
  - Architected and built the complete 14-stage stream-based technical PDF conversion pipeline driven by LLM multimodal capabilities.
  - Integrated Gemini Vision OCR with integer millirange normalized bounding boxes (`box_2d`), producing clean layout geometry.
  - Designed automated 3-way graphic triage: distinguishing technical schematics/block diagrams (processed into sidecar text descriptions), photos/illustrations, and register bitfield diagrams (transcribed into pure ASCII art / GFM tables).
  - Built prioritized HTML table reconstruction with multi-cell row/colspan merging and automated multi-block table of contents (TOC) extraction.
  - Established strict fail-fast error handling, single-pass page triage, and stage-workspace directory isolation.
- **Architectural Rationale & Trade-Offs**:
  - *Stream-Based 14-Stage Architecture:* Breaking document conversion into discrete stages (extract -> triage -> OCR -> tables -> proofread -> Markdown) prevents context window exhaustion and enables resume/recovery at stage boundaries.
  - *ASCII Art for Registers over Bitmaps:* Converting hardware register layouts to text ASCII art makes them directly queryable by RAG and readable in terminal/IDE editors.
- **Verification & Invariants**:
  - Validated on 20 representative HRM and Amiga reference pages with 100% table and diagram fidelity.

---

### [2026-09-16 23:59 CEST] — Milestone Digest: RAG Knowledge Base, Diagram Sidecars & Attractor Elimination
- **Timestamp & Context**: 2026-09-16 — Offline knowledge retrieval, technical diagram processing, and codebase cleanup.
- **Affected Subsystems**: `tools/amiga-rag-mcp-server/`, `tools/harness/rag_qdrant.py`, `.agents/rules/amiga-rag.md`, `.agents/rules/asset-descriptions.md`, `.agents/rules/structural-root-cause.md`.
- **What Was Changed (The Concrete Reality)**:
  - Integrated local vector-based RAG knowledge base indexing hardware documentation, chip schematics, and Obsidian design notes into a local Qdrant collection (`amiga`).
  - Implemented offline diagram sidecar pipeline (`<image>.txt`), producing Git-tracked technical descriptions for custom chip schematics and block diagrams.
  - Codified the Structural Root-Cause Resolution rule, strictly prohibiting surface-level coordinate nudges or ad-hoc regex patches in favor of upstream data lifecycle and timing fixes.
  - Codified Practitioner Voice & Tone guidelines, purging academic buzzwords in favor of concrete systems engineering terminology.
- **Architectural Rationale & Trade-Offs**:
  - *Local Vector Search:* Eliminates reliance on cloud APIs and enables rapid, targeted retrieval of complex Amiga hardware register timings without manual page scanning.
  - *Sidecar Markdown Representation:* Storing visual circuit schematics as searchable text sidecars allows text-based agents and search indexes to reason about physical pinouts and bus topology directly.
- **Verification & Invariants**:
  - Offline CLI retrieval tests via `rag_qdrant`.
  - Sidecar validation in `audit_docs_quality.py`.

---

### [2026-09-17 23:59 CEST] — Milestone Digest: Tooling Ergonomics, Zero-Junction Invariant & Full Worktree Storage Isolation
- **Timestamp & Context**: 2026-09-16 to 2026-09-17 — Git worktree engineering, zero-junction filesystem safety, and script standardization.
- **Affected Subsystems**: `tools/bootstrap/`, `tools/harness/`, `.agents/skills/git-worktree/`, `docs/`.
- **What Was Changed (The Concrete Reality)**:
  - Eliminated NTFS directory junctions in Git worktree management, enforcing physical copying of untracked test assets, ROM slices, and `.env` to guarantee absolute filesystem isolation.
  - Reorganized repository tooling into distinct directories: `tools/bootstrap/` (environment setup, dependency provisioning) and `tools/harness/` (testing, quality gates, benchmarking).
  - Consolidated scattered bootstrap scripts into unified PowerShell entry points with standardized parameter validation and help texts.
  - Unified developer-facing documentation into `docs/sources.md` and `docs/reference_documentation.md`, pruning redundant documentation fragments.
- **Architectural Rationale & Trade-Offs**:
  - *Zero NTFS Junctions:* Symlinks and junctions on Windows cause cross-worktree contamination when concurrent agent branches modify untracked scratch assets. Full physical isolation completely eliminates shared-state bugs.
  - *Standardized Tooling Hierarchy:* Cleanly separating harness scripts from bootstrap scripts prevents accidental CI dependencies on setup tooling.
- **Verification & Invariants**:
  - Validated concurrent Git worktree creation and teardown with zero shared filesystem leakage.
  - PowerShell syntax and pre-flight quality checks passing cleanly.

---

### [2026-09-18 23:59 CEST] — Milestone Digest: MemoryBus Streamlining, Agnus DMA Mastership & Decoupled State Serialization
- **Timestamp & Context**: 2026-09-18 — Hardware bus topology enforcement, memory banking refactoring, and state serialization.
- **Affected Subsystems**: `crates/memory_bus`, `crates/physical_memory`, `crates/agnus`, `crates/paula`, `crates/denise`, `crates/machine_loop`.
- **What Was Changed (The Concrete Reality)**:
  - Enforced Agnus DMA Address Mastership: Agnus exclusively drives Chip RAM DMA addresses and RGA bus lines; Denise and Paula act as passive data latchers with zero direct memory reads.
  - Refactored `MemoryBus` to push `BusResult` directly into memory bank handlers, eliminating nested dynamic conditionals and outer contention branches.
  - Decoupled `FloppyController` from `MemoryBus`, restoring native motherboard signal routing via CIA and Paula latches.
  - Implemented complete `serde::Serialize` and `serde::Deserialize` support across custom chip state snapshots with zero runtime heap allocation in hot paths.
- **Architectural Rationale & Trade-Offs**:
  - *Passive Data Latching:* Accurately reflects physical Amiga custom chip silicon, where Agnus generates DMA addresses on the internal bus, preventing inter-chip pointer coupling.
  - *Clean Bank Delegation:* Moving wait-state evaluation into bank dispatch functions flattens the CPU bus loop and optimizes branch prediction on modern host processors.
- **Verification & Invariants**:
  - Cartesian DMA contention suite (`test_dma_cartesian.rs`): Validated CPU wait-state generation (C = C_0 + 2 * wait_states).
  - Hardware bus topology automated checks in `audit_hardware_quality.py`.

---

### [2026-09-19 23:59 CEST] — Milestone Digest: Tripartite Quality Audits, Crate Renaming (m68000 -> cpu), and Compiler Lints Baseline
- **Timestamp & Context**: 2026-09-19 — Rigorous quality assurance framework, naming alignment, and zero-warning compilation baseline.
- **Affected Subsystems**: Workspace `Cargo.toml`, `crates/cpu` (formerly `m68000`), `tools/harness`, `.agents/rules/`, `.agents/skills/`.
- **What Was Changed (The Concrete Reality)**:
  - Renamed crate `m68000` to `cpu` and unified named crate root (`cpu.rs`), standardizing 3-tier re-export hierarchy across all workspace crates.
  - Established Tripartite Quality Auditing framework (`audit-code-quality`, `audit-hardware-quality`, `audit-docs-quality`), covering 27 distinct architectural and hardware pillars.
  - Institutionalized self-documenting boolean logic, short-circuit preservation, and method naming conventions (`get_` omission, `is_`/`has_` boolean prefixes).
  - Consolidated CPU prefetch queue into single `u16` (`CpuState.prefetch`) with explicit 2-word pipeline dynamics.
  - Configured workspace-wide strict compiler baseline (`warnings = "deny"` in `Cargo.toml`) and curated Clippy lints balancing Rust safety with Amiga silicon requirements.
- **Architectural Rationale & Trade-Offs**:
  - *Automated Architecture Verification:* Turning architectural guidelines into executable Python and Rust tests guarantees long-term maintainability and prevents architectural regression across agent sessions.
  - *Zero-Warning Compilation:* Catching dead code, unused mutability, and unhandled branches at compiler time ensures absolute cleanliness in the emulation codebase.
- **Verification & Invariants**:
  - All 23 workspace crates compile cleanly with zero compiler warnings and zero Clippy errors.
  - 10/10 pillars in `audit_docs_quality.py`, 7/7 pillars in `audit_code_quality.py`, and 5/5 pillars in `audit_hardware_quality.py` passing.

---

### [2026-09-21 23:59 CEST] — Milestone Digest: Strict Compilation Policy, CPU Semantic Parity & Scope Discipline
- **Timestamp & Context**: 2026-09-20 to 2026-09-21 — Compiler hardening, M68000 CPU architectural cleanup, semantic parity audit, and governance rule codification.
- **Affected Subsystems**: `Cargo.toml`, `crates/cpu`, `Obsidian/Amiga/Design/`, `.agents/rules/`, `ROADMAP.md`.
- **What Was Changed (The Concrete Reality)**:
  - Enforced strict zero-warning compilation policy (`warnings = "deny"` under `[workspace.lints.rust]`) across all workspace targets.
  - Eliminated artificial OOP getter/setter encapsulation and raw public field restrictions on plain-old-data (POD) structs, standardizing on idiomatic Rust accessors (`d_long`/`set_d_long`, slice views, `is_`/`has_` boolean prefixes).
  - Executed bidirectional semantic parity audit for M68000 CPU specifications, eliminating ghost features, clarifying Fast RAM `TAS` erratum, and consolidating platform quirks into `Platform Quirks and Invariants Catalog.md`.
  - Streamlined `Cpu::restore_state` and stack pointer mechanics (canonical `a_long(7)` dispatch, invariant `usp`/`ssp` fields).
  - Codified the Strict Scope Discipline rule (`strict-scope-discipline.md`) to enforce task containment, prevent unsolicited refactoring, and require structured delivered vs suggested reporting.
  - Restructured `ROADMAP.md` into an executable nested hierarchy with explicit verification gates and zero retention of completed items.
- **Architectural Rationale & Trade-Offs**:
  - *Zero-Warning Guarantee:* Denying warnings at the compiler level ensures dead code, unused mutability, or open-bus anomalies fail the build immediately rather than accumulating as technical debt.
  - *Idiomatic Systems Rust over OOP Boilerplate:* Amiga hardware registers, coordinates, and ALU states are transparent hardware structures; mechanical OOP accessors added syntactic noise without providing domain invariants.
  - *Bidirectional Parity:* Ensuring living design specs perfectly match active code prevents agent hallucinations and spec drift.
- **Verification & Invariants**:
  - 100% clean build with 0 warnings and 0 Clippy lints across all workspace crates.
  - 21/21 architecture rules passed in `test_architecture_rules`.
  - Full SingleStepTests test suite passing across all 68000 instructions.

---

### [2026-09-22 23:59 CEST] — Milestone Digest: RAG Tooling Reform, CLI Single Source of Truth & Documentation Scoping
- **Timestamp & Context**: 2026-09-22 — Decoupling and hardening the Amiga RAG indexing infrastructure, CLI unification, and documentation scoping.
- **Affected Subsystems**: `tools/amiga-rag-mcp-server/`, `tools/harness/rag_qdrant.py`, `.agents/rules/amiga-rag.md`, `.agents/skills/index-amiga-rag/`.
- **What Was Changed (The Concrete Reality)**:
  - Established `rag_qdrant` CLI as the sole runtime authority for the unified `projects_docs` Qdrant collection, enforcing the local index state file contract (`--index-json $env:RAG_INDEX_JSON`).
  - Decoupled the project FastMCP server (`tools/amiga-rag-mcp-server/`) into a lightweight delegation adapter that calls the canonical CLI rather than maintaining duplicate indexing or retrieval logic.
  - Enforced explicit directory scopes for indexing (`Obsidian/Amiga/Design` and `Obsidian/Amiga/Reference`) while strictly excluding the `.obsidian` configuration directory.
  - Re-established the reference image description bootstrap contract: visual asset descriptions are generated once during bootstrap rather than re-evaluated per query.
  - Isolated the external `devnotes` knowledge source as retrieval-only, safeguarding it against unilateral Amiga index mutations.
- **Architectural Rationale & Trade-Offs**:
  - *CLI-First Architecture:* Relying on the standalone Python CLI as the single source of truth ensures vector database operations function identically in interactive shell sessions, background CI scripts, and IDE subagents.
  - *Explicit Index Boundaries:* Pointing indexing directly to design and reference subtrees prevents indexing non-document assets or configuration artifacts.
- **Verification & Invariants**:
  - 14 focused RAG MCP and bootstrap contract tests passed.
  - Pre-flight quality gates passed with 100% compliance.

---

### [2026-09-29 01:37 CEST] — Step 1.4 Completed: MachineLoop Architecture Review and Subsystem Coordination Audit
- **Affected Subsystems**:
  - `crates/machine_loop`
  - `Obsidian/Amiga/Design/`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Conducted end-to-end code review and architectural orientation of crates/machine_loop
  - Authored Obsidian/Amiga/Design/MachineLoop Architecture Review.md documenting CCK phase progression, Agnus DMA mastership, passive chip latching, and bus arbitration
  - Pruned Step 1.4 from ROADMAP.md and renumbered remaining sub-steps contiguously
- **Architectural Rationale & Trade-Offs**:
  - Establish an authoritative, cycle-exact mental model of machine loop execution flow
  - Map inter-chip signal propagation and physical bus routing prior to executing remaining chipset verification and HRM alignment
- **Verification & Test Results**:
  - pre_flight.py --quick passed (100% compliance across formatting, AGENTS.md ceiling, API coverage, Clippy, Code Quality 1-2, Hardware Quality 1-2)
  - test_architecture_rules passed (20/20)

---

### [2026-09-29 01:50 CEST] — Historical Section 10 Compaction into Architectural Milestone Digests
- **Affected Subsystems**:
  - `DIARY.md` (Section 10 synthesized per `.agents/skills/compact-diary/SKILL.md`)
  - `.agents/rules/diary-maintenance.md`, `.agents/rules/roadmap-maintenance.md`, `.agents/skills/compact-diary/SKILL.md`
- **What Was Changed (The Concrete Reality)**:
  - Audited and compacted 340+ historical chronological entries in Section 10 (September 12–22, 2026) spanning over 8,500 lines into high-signal Architectural Milestone Digests covering all major project phases:
    1. Disassembler Extraction, Memory Banking & Core Machine Setup (2026-09-12)
    2. Language Guardrails, HRM Alignment & CPU Verification (2026-09-14)
    3. Multimodal Reference Ingestion & 14-Stage PDF-to-Markdown Pipeline (2026-09-16)
    4. RAG Knowledge Base, Diagram Sidecars & Attractor Elimination (2026-09-16)
    5. Tooling Ergonomics, Zero-Junction Invariant & Full Worktree Storage Isolation (2026-09-17)
    6. MemoryBus Streamlining, Agnus DMA Mastership & Decoupled State Serialization (2026-09-18)
    7. Tripartite Quality Audits, Crate Renaming (m68000 -> cpu), and Compiler Lints Baseline (2026-09-19)
    8. Strict Compilation Policy, CPU Semantic Parity & Scope Discipline (2026-09-21)
    9. RAG Tooling Reform, CLI Single Source of Truth & Documentation Scoping (2026-09-22)
  - Purged obsolete raw micro-diffs, transient command traces, and uncompacted historical duplication while permanently preserving every major subsystem accomplishment, architectural dilemma, and verification result.
  - Retained the active September 29 roadmap milestone entry (Step 1.4: MachineLoop Architecture Review) in full granular detail.
  - Updated diary maintenance rules and skills to formally allow compaction upon completing minor roadmap points or when Section 10 exceeds operational size thresholds.
  - Reduced `DIARY.md` from 8,995 lines (930 KB) down to ~620 lines (~75 KB) with zero loss of architectural wisdom or hardware rationale.
- **Architectural Rationale & Trade-Offs**:
  - Eliminates prompt bloat and token exhaustion while permanently safeguarding core design dilemmas, hardware silicon discoveries, and human-AI collaboration dynamics.
- **Verification & Invariants**:
  - `python tools/harness/pre_flight.py --quick` passed cleanly.
  - `cargo test -p test_runner --test test_architecture_rules` passed (20/20).
---

### [2026-10-05 21:33 CEST] — Simplify agent guidance and clarify per-commit diary logging
- **Affected Subsystems**:
  - `.agents/rules`
  - `.agents/skills`
  - `.codex/agents`
  - `tools/harness`
  - `tools/bootstrap/pdf-to-markdown`
  - `crates/test_runner/tests`
  - `documentation`
- **What Was Changed (The Concrete Reality)**:
  - Consolidated scope and root-cause rules while preserving the isolated special-case prohibition
  - clarified logging on every commit and compaction at milestone completion
  - reduced common instructions and made domain guidance selective
  - aligned checkpoint names, Markdown links, inlining examples, verification tiers, and converter stage commands
  - removed forced source/test file coupling and assertion quotas
- **Architectural Rationale & Trade-Offs**:
  - Preserve emulator invariants while reducing duplicated instructions and resolving contradictory procedures
  - keep the previously reported documentation-audit remediation outside this task
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 20 tests
  - Python harness suite passed 32 tests
  - agent TOML syntax and Python compilation passed
  - Graphify code graph refreshed
  - full documentation audit still reports 9 deferred findings after the merged prime-directives rule was removed
  - no emulator production source changed
---

### [2026-10-05 21:34 CEST] — Clarify RAG context retrieval and MCP search guidance
- **Affected Subsystems**:
  - `.agents/rules/amiga-rag.md`
  - `tools/amiga-rag-mcp-server`
- **What Was Changed (The Concrete Reality)**:
  - Shortened retrieval guidance and delegated indexing to the existing skill
  - documented source selection, bounded results, and read-only behavior in MCP metadata
- **Architectural Rationale & Trade-Offs**:
  - Use a concise rule and tool metadata for focused context retrieval without adding a redundant search skill
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 20 tests
  - RAG adapter suite passed 9 tests
  - Codex MCP launch regression passed
  - published search metadata verified
  - Graphify update found no topology changes
---

### [2026-10-05 21:37 CEST] — Shorten clean-break refactoring guidance
- **Affected Subsystems**:
  - `.agents/rules/clean-break-refactoring.md`
- **What Was Changed (The Concrete Reality)**:
  - Condensed the rule from 66 lines to 8
  - retained complete internal API migration and explicit opt-in for compatibility
  - clarified external contracts and saved-data impact
- **Architectural Rationale & Trade-Offs**:
  - Make the refactoring policy concise while preventing unsolicited legacy interfaces
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 20 tests
---

### [2026-10-05 21:48 CEST] — Remove retired project-side RAG search stub
- **Affected Subsystems**:
  - `tools/harness`
- **What Was Changed (The Concrete Reality)**:
  - Deleted the retired rag_search.py command that only printed MCP migration guidance and exited with failure
- **Architectural Rationale & Trade-Offs**:
  - No tracked callers reference the retired script. Supported retrieval remains available through the Amiga RAG MCP adapter and canonical CLI
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite passed 20 tests
  - Full documentation audit reports 8 remaining findings: 7 direct-reference findings and 1 unregistered diary rule
---

### [2026-10-05 21:57 CET] — Separate agent policy from execution procedures
- **Affected Subsystems**:
  - `.agents/rules`
  - `.agents/skills`
  - `AGENTS.md`
  - `docs`
  - `tools/harness`
  - `tests`
  - `crates/test_runner/tests`
- **What Was Changed (The Concrete Reality)**:
  - Retain 27 concise rules and remove only the Graphify redirect
  - Move naming, layout, cohesion and inlining examples into five references owned by existing skills
  - Keep mandatory Graphify navigation in AGENTS.md while removing redundant skill routing
  - Preserve the parallel auditor simplification and reconcile companion classifications, workflow commands and regression fixtures
- **Architectural Rationale & Trade-Offs**:
  - Keep obligations and exceptions in rules
  - execution in skills
  - and deterministic checks bounded to their observed coverage. Preserve existing hardware and golden-reference policy without emulator runtime changes.
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite passed 19 tests
  - Python fixture suite passed 41 tests
  - Selected size, skill, locality, companion and workflow audits reported zero issues
  - Verified 66 new local links and the unchanged Graphify skill
  - Refreshed the AST graph. Full emulator domain suites were not run because runtime code was unchanged.
---

### [2026-10-05 22:11 CEST] — Consolidate the active roadmap and maintenance procedure
- **Affected Subsystems**:
  - `ROADMAP.md`
  - `.agents/rules/roadmap-maintenance.md`
  - `.agents/skills/roadmap-maintenance`
  - `.agents/skills/code-review`
- **What Was Changed (The Concrete Reality)**:
  - Group manual conversion and ingestion first, test orientation and profiling second, and chipset reconstruction third
  - remove historical baseline and duplicated methodology while preserving pending extensions
  - consolidate conversion requirements and remap remaining task references
  - retain missing harness architecture and eval work after checking native Codex and MCP evidence
  - surface the reset proposal's timing and quirks conflict before implementation
  - align the roadmap rule and skills with a backlog-only document
- **Architectural Rationale & Trade-Offs**:
  - Make immediate work readable without discarding artifact validation, source fidelity, manual handoffs, or deferred capabilities
  - keep policy in rules and execution procedure in skills
  - this reorganization does not certify or complete any planned subsystem milestone
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19 tests
  - governance, MCP contract, and live Codex discovery regressions passed 19 tests total
  - documentation audit passed with zero issues
  - both changed skills validated, using Python UTF-8 mode after a Windows decoding failure on the existing code-review file
  - language and diff checks passed
  - numbering and local-link check passed for 11 steps, 44 tasks, 7 sub-suites, and 20 links
  - full subsystem milestone and semantic-parity gates were not run because no implementation milestone was completed
---

### [2026-10-05 22:14 CEST] — Limit roadmap Step 10 to subsystem regeneration
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove the four unselected Step 10 tasks
  - retain the isolated documentation-to-code regeneration experiment as 10.1 and rename the workstream
- **Architectural Rationale & Trade-Offs**:
  - Keep only the experiment selected by the user
  - preserving its isolated-checkout scope and unit/integration acceptance gate
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture tests passed 19 of 19
  - roadmap verification passed for 11 steps and 40 tasks with valid numbering, references and 20 local links
  - language and diff checks passed
  - no regeneration experiment or subsystem milestone was executed
---

### [2026-10-05 22:15 CEST] — Select Codex Vision for manual conversion
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace the olmOCR and enriched Docling comparison with planned Codex Vision conversion
  - align the workflow audit with replacing existing Gemini calls
  - retain representative-sample validation and the asset contract
- **Architectural Rationale & Trade-Offs**:
  - Record the user's selected conversion approach and remove the unnecessary pipeline-selection experiment
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture tests passed 19 of 19
  - roadmap numbering and local-link verification passed
  - language and diff checks passed
  - no olmOCR, Docling or RunPod comparison remains in ROADMAP.md
  - converter implementation and conversion runs were not performed
---

### [2026-10-05 22:23 CEST] — Separate OCR extraction from PNG conversion in the roadmap
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Merge workflow review and Codex Vision adaptation into 1.1
  - renumber text-layer preparation and positioned JSON extraction to 1.2
  - add explicit PNG-to-Markdown and crop conversion as 1.3 with mandatory matching OCR text JSON in each model request
- **Architectural Rationale & Trade-Offs**:
  - Make the artifact handoff from OCR extraction to visual conversion explicit and independently verifiable while removing overlapping preparation tasks
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture tests passed 19 of 19
  - roadmap numbering and local-link checks passed for 11 steps and 40 tasks
  - language and diff checks passed
  - converter code and model requests were not executed
---

### [2026-10-05 22:27 CEST] — Remove redundant document-processing agent roles
- **Affected Subsystems**:
  - `.codex/agents`
  - `docs/ai_agents.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove doc_ingestor, tech_writer and vision_analyst definitions
  - update doc_curator routing to standalone conversion programs and validated CLI indexing
  - document four remaining native roles and parent-owned narrative and GUI workflows
  - clarify model-role configuration and per-stage thinking budgets without changing converter configuration
- **Architectural Rationale & Trade-Offs**:
  - Let scripted conversion stages own model calls and persisted artifacts instead of adding overlapping native agent roles
  - preserve existing authoring and GUI skills for the parent session
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19 tests
  - governance suite passed 9 tests
  - documentation audit reported zero issues
  - all four remaining TOML definitions parse and match the documented catalog
  - 56 local links resolve
  - active references to removed roles were eliminated
  - language and diff checks passed
  - converter configuration and code were unchanged, no conversion or fresh-client agent activation test was run
---

### [2026-10-05 22:33 CEST] — Remove unused PDF converter configuration sections
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/config.yaml`
- **What Was Changed (The Concrete Reality)**:
  - Remove unused table_heuristics and segmentation sections
- **Architectural Rationale & Trade-Offs**:
  - Remove ineffective controls because the converter does not read these settings.
- **Verification & Test Results**:
  - YAML parsing and comparison against HEAD passed: only the two unused sections were removed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no document conversion was run
---

### [2026-10-05 22:34 CEST] — Use provider sampling defaults in reference converters
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/html-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Remove temperature from both configurations and Gemini clients
  - remove temperature from cache key construction and the HTML fallback configuration
  - update HTML configuration documentation
- **Architectural Rationale & Trade-Offs**:
  - Let Gemini use its default sampling and prevent responses generated with explicit temperature settings from being reused for new requests
- **Verification & Test Results**:
  - Offline checks passed for both real SDK request configurations and cache APIs
  - all other YAML settings preserved
  - Python syntax checks passed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - Graphify update passed
  - no live API call or document conversion was run
---

### [2026-10-05 22:35 CEST] — Remove unused PDF converter path settings
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/config.yaml`
- **What Was Changed (The Concrete Reality)**:
  - Remove the unused paths section
- **Architectural Rationale & Trade-Offs**:
  - Remove ineffective YAML controls because the CLI and fixed stage directory layout already determine paths.
- **Verification & Test Results**:
  - YAML parsing and comparison against HEAD passed: only paths was removed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no document conversion was run
---

### [2026-10-05 22:43 CEST] — Plan roadmap 1.1 Codex conversion and per-stage configuration
- **Affected Subsystems**:
  - `ROADMAP.md`
  - `.agent/tasks/codex-document-conversion.md`
- **What Was Changed (The Concrete Reality)**:
  - Extract roadmap 1.1 execution detail into a linked plan
  - specify shared Codex integration with ChatGPT authentication and explicit model/effort entries for both converters
  - audit inference stages and plan cache/resume/manual handoff validation
- **Architectural Rationale & Trade-Offs**:
  - Make the next conversion task executable while retaining roadmap 1.2-1.6 as pending work and keeping active Gemini clients/configuration unchanged
- **Verification & Test Results**:
  - YAML examples cover 10 PDF inference stages and one HTML inference stage
  - 15 local links resolve
  - English checks passed
  - initial portability diagnostic falsely matched an HTTPS URL and the corrected path-only scan passed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no SDK installation, live inference, production migration, conversion or indexing was performed
---

### [2026-10-05 22:47 CEST] — Bound roadmap 1.1 SDK pilots to individual test-book pages
- **Affected Subsystems**:
  - `.agent/tasks/codex-document-conversion.md`
- **What Was Changed (The Concrete Reality)**:
  - Confirm Python SDK as the selected Codex transport
  - record the supplied test-book PDF and tracker using repository-relative paths
  - require incremental single-page pilots with concurrency 1 and usage review before further calls
- **Architectural Rationale & Trade-Offs**:
  - Keep migration validation source-grounded and limit subscription token usage through focused live samples
- **Verification & Test Results**:
  - Source PDF exists and metadata confirms 160 pages
  - plan links and YAML examples validated
  - English and portable-path checks passed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no inference, SDK installation or book conversion was run
---

### [2026-10-05 22:49 CEST] — Point roadmap 1.1 pilots at the Codex test-book directory
- **Affected Subsystems**:
  - `.agent/tasks/codex-document-conversion.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace the test dataset directory with Test Book example-4567-codex
- **Architectural Rationale & Trade-Offs**:
  - Use the user-selected Codex test dataset for the existing single-page pilot plan
- **Verification & Test Results**:
  - PDF and tracker exist in the new directory
  - metadata confirms 160 PDF pages
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no inference or conversion was run
---

### [2026-10-05 22:52 CEST] — Track the Codex test-page matrix and ignore local book artifacts
- **Affected Subsystems**:
  - `.gitignore`
  - `Obsidian/Amiga/Reference/Test Book example-4567-codex/TEST_PAGES_TRACKER.md`
  - `.agent/tasks/codex-document-conversion.md`
- **What Was Changed (The Concrete Reality)**:
  - Add only the Codex test-page tracker to Git
  - ignore all other files and nested directories in its dataset and the entire AGY dataset
  - align tracker and plan with the current PDF location
  - preserve seven Markdown hard breaks without trailing whitespace
- **Architectural Rationale & Trade-Offs**:
  - Keep test expectations versioned while leaving local PDF sources and conversion outputs outside Git history
- **Verification & Test Results**:
  - Eight ignore cases passed including nested paths and the tracker exception
  - current PDF and tracker paths resolve
  - tracker language check passed
  - quick pre-flight passed
  - architecture suite passed all 19 tests
  - no source PDF or generated artifacts were staged
---

### [2026-10-05 22:57 CET] — Document local reconstruction and replace source quotations in the test tracker
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567-codex/TEST_PAGES_TRACKER.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace three manual paragraphs with alert placeholders and independent verification criteria
  - add missing source PDF page 28 to the selection list
  - record four source SHA-256 hashes and local reconstruction instructions
- **Architectural Rationale & Trade-Offs**:
  - Keep the versioned tracker focused on test expectations and reproducible page selection while source book content remains local
  - amend the tracker-import commit as requested
- **Verification & Test Results**:
  - All 160 selected pages match source decoded content streams and media boxes
  - all four selection lists and counts match the matrix
  - all four source hashes and page counts verified
  - three source paragraphs removed
  - language and diff checks passed
  - quick pre-flight passed
  - all 19 architecture tests passed
  - no inference or conversion run
---

### [2026-10-05 23:09 CET] — Migrate HTML reference transcription to Codex and start a one-page PDF pilot
- **Affected Subsystems**:
  - `tools/bootstrap/conversion`
  - `tools/bootstrap/html-to-markdown`
  - `conversion task plan`
- **What Was Changed (The Concrete Reality)**:
  - Add pinned subscription-authenticated stage-bound Codex transport, strict YAML configuration and isolated completed-response cache
  - remove HTML Gemini clients and silent DOM/configuration substitutions
  - verify source transcription on one HTML page and physical PDF page 16
  - retain PDF production migration and artifact lineage as pending
- **Architectural Rationale & Trade-Offs**:
  - Roadmap task 1.1.A-B and HTML part of 1.1.C: establish the Codex boundary before migrating PDF workers
  - keep pilot page and live-call counts small
- **Verification & Test Results**:
  - 15 conversion contract tests and 4 bootstrap contract tests passed
  - quick pre-flight and 19 architecture tests passed
  - live text, JSON and original-detail image/schema pilots completed with ChatGPT auth at gpt-6.1-sol/medium
  - one HTML cache replay avoided inference
  - PDF pilot used one physical page and one request
  - full PDF pipeline, resume/manual lineage and milestone gates not completed
---

### [2026-10-05 23:35 CEST] — Migrate PDF stages to shared Codex conversion client
- **Affected Subsystems**:
  - `tools/bootstrap/conversion`
  - `tools/bootstrap/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Bind every PDF inference worker to explicit stage model and effort with structured response schemas
  - Remove duplicated Gemini client and cache
  - Validate predecessor completion identities and record worker metrics
  - Update conversion usage and task evidence
- **Architectural Rationale & Trade-Offs**:
  - Task 1.1: pass one physical PDF page through the existing 14-stage workflow using ChatGPT authentication
  - Conversion-quality repairs and title-call optimization were reverted as requested
  - Broader recovery and source-fidelity acceptance remain pending
- **Verification & Test Results**:
  - Physical page 19 passed all 14 stages with 8 live requests across the pilot
  - Full replay passed using 9 cache hits and zero new requests
  - Completed-run resume validated all stages without inference
  - 22 conversion tests and 4 bootstrap tests passed
  - Python compilation and PDF CLI help passed
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed because Design links target 62 missing Reference materials
  - Staged English scan reported the Python standard-library identifier copytree as an English vocabulary violation; manually verified this false positive without changing the scanner
  - Graphify code refresh completed without inference
---

### [2026-10-05 23:48 CEST] — Reset PDF conversion dependents within independent workspaces
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/lineage.py`
  - `tools/bootstrap/pdf-to-markdown/pipeline.py`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Validate retained predecessors and rebuild missing working manifests from snapshots
  - Clear all stage outputs and manual tasks from the requested restart stage onward
  - Resume from the earliest missing stage artifact
  - Preserve upstream assets and current manual edits during apply
  - Default final output to the selected workspace
- **Architectural Rationale & Trade-Offs**:
  - Task 1.1 follow-up: one workspace owns one source/page experiment
  - Whole downstream cleanup avoids ambiguous selective page invalidation
  - Missing artifacts are supported while modified retained artifacts remain incompatible
  - Conversion-quality changes remain deferred
- **Verification & Test Results**:
  - Confirmed four restart regressions failed before the repair and passed afterward
  - 32 conversion tests and 4 bootstrap tests passed
  - Page 19 passed stages 01-14 using only cached responses in a separate workspace
  - Stage 05 reset passed with missing manifests and chapter output then resume completed stages 06-14
  - Preserved stages and original workspace remained unchanged with zero live inference requests
  - Python compilation and quick pre-flight passed
  - Architecture suite remains 18 passed and 1 failed due to 62 missing Reference links
  - Graphify code refresh passed
  - Staged English scanner false-positive on standard-library copytree was verified manually
---

### [2026-10-05 23:52 CEST] — Remove redundant PDF raw text assets
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Stop writing raw asset_node text dumps and remove raw_text_path from reduction and continuation handling and table task metadata
  - Keep raw_text in node JSON and retain image description sidecars
  - Update converter stage documentation
  - Remove 379 empty legacy dumps and their references from the selected manual workspace without changing raw_text or remaining non-JSON artifacts
- **Architectural Rationale & Trade-Offs**:
  - Automatic conversion consumes raw_text from JSON and visual crops
  - Separate raw PDF text dumps have no downstream reader
- **Verification & Test Results**:
  - Focused regression failed on both scanned and native-text PDF fixtures before the change and passed afterward
  - All 33 offline conversion tests passed
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on 62 unrelated broken design-document Markdown links
  - Workspace cleanup verified raw_text equality and unchanged hashes for all remaining non-JSON artifacts
  - git diff --cached --check passed
  - Graphify AST refresh completed
---

### [2026-10-05 23:57 CEST] — Use PNG and JSON as PDF stage handoff
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Stop generating single-page PDFs and remove pdf_file from the page manifest
  - Replace PDF clipping and rendering with direct crops from Stage 01 PNGs using JSON page dimensions
  - Preserve typographic and vertical neighbor padding limits with outward pixel rounding
  - Fail missing PNG or geometry inputs and invalid or empty crops
  - Pass configured DPI through Stage 04 recropping and remove the unused percentage-padding option
  - Update stage documentation
  - Remove 50 legacy single-page PDFs from the selected workspace and retain all other artifacts
- **Architectural Rationale & Trade-Offs**:
  - PNG and JSON are the downstream inputs
  - Source PDF access is confined to preprocessing and native vector extraction is intentionally removed
- **Verification & Test Results**:
  - Focused regression failed before the change and passed afterward
  - All 36 offline conversion tests passed including native and scanned sources and exact crop pixels and merged table bounds and missing PNG rejection
  - All 295 existing raw-stream visual crops were exercised in a disposable output directory
  - Workspace cleanup verified unchanged hashes for retained files except removal of pdf_file from the manifest
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on the same 62 unrelated broken design-document Markdown links
  - All PDF converter Python modules parsed and staged diff check passed
  - Graphify AST refresh completed
---

### [2026-10-06 00:04 CEST] — Plan a uniform OCR PDF input before preprocessing
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Clarify Step 1.2 text-layer preparation before preprocessing
  - Always create a separate source-stem - OCR.pdf by adding OCR where needed or copying native-text input
  - Require the validated output as the sole downstream PDF input and verify it in the acceptance gate
- **Architectural Rationale & Trade-Offs**:
  - Give scanned and native-text PDFs one artifact handoff and one downstream conversion path
  - This is pending roadmap scope and does not implement OCR
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on 62 existing broken design-document links outside this change
  - Roadmap diff and whitespace check passed
  - No converter execution was required for this documentation-only change
---

### [2026-10-06 00:08 CEST] — Limit roadmap 1.1 validation to flow and input/output contracts
- **Affected Subsystems**:
  - `.agent/tasks/codex-document-conversion.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Narrow remaining acceptance to stage flow, request inputs, output schemas, artifact and asset handoffs, cache and resume/manual recovery
  - Prefer offline fixtures and compatible cache replays
  - Exclude conversion-quality assessment and tuning
  - Refresh recorded test counts and PNG-only handoff inventory
- **Architectural Rationale & Trade-Offs**:
  - Task 1.1: apply the user's flow and input/output validation scope while retaining separate roadmap 1.2-1.6 work and required milestone closure
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on 62 existing broken Design-to-Reference links outside this change
  - Local Markdown links, scope boundaries and remaining roadmap numbering verified
  - Plan and roadmap English scans and diff whitespace check passed
  - Documentation-only change, no conversion or live inference ran
---

### [2026-10-06 00:11 CEST] — Move active Codex conversion validation out of the roadmap
- **Affected Subsystems**:
  - `.agent/tasks/codex-document-conversion.md`
  - `ROADMAP.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove implemented migration steps, configuration examples and execution history from the plan
  - Keep only remaining flow and input/output verification and milestone closure
  - Move the former roadmap 1.1 scope and acceptance gate entirely into the active plan
  - Remove the duplicated roadmap entry and renumber remaining conversion work to 1.1-1.5
  - Update internal dependencies and conversion contract links
- **Architectural Rationale & Trade-Offs**:
  - Task codex-document-conversion: keep one remaining-work plan and remove duplicate roadmap ownership without claiming completion
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on 62 existing broken Design-to-Reference links
  - Verified 18 local Markdown links, remaining-only plan, exclusive task ownership and contiguous numbering
  - English and whitespace checks passed
  - Documentation-only reorganization, no conversion, live inference or milestone completion checks ran
---

### [2026-10-06 00:17 CEST] — Drop additional validation of the migrated conversion workflow
- **Affected Subsystems**:
  - `.agent/tasks/codex-document-conversion.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove the plan whose remaining scope was additional conversion validation
  - Remove the coverage matrix, additional conversion regressions and pilot gate
  - Use existing migration evidence as the basis for follow-on roadmap work
  - Keep conversion-quality evaluation outside the current scope
- **Architectural Rationale & Trade-Offs**:
  - Task codex-document-conversion: apply the user's decision to proceed without expanding tests of a workflow due for substantial redesign
  - Do not claim full coverage, source-fidelity acceptance or milestone completion
- **Verification & Test Results**:
  - Required per-commit quick pre-flight passed
  - Required architecture suite: 18 passed and 1 failed on the same 62 existing broken Design-to-Reference links
  - English scan and whitespace check passed
  - No active references to the deleted plan remain in searched task, governance, tooling and developer documentation
  - No conversion tests, coverage audit, live inference, book conversion or milestone checks ran
---

### [2026-10-06 00:21 CEST] — Remove the four PDF asset and crop tests
- **Affected Subsystems**:
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Delete PdfAssetTests and its class-local stage-loading helper
  - Remove the four raw-stream asset, PNG crop, merged-table crop and missing-PNG tests
  - Preserve all remaining PDF tests and converter implementation
- **Architectural Rationale & Trade-Offs**:
  - Apply the user's explicit request to remove these four tests from the converter suite
- **Verification & Test Results**:
  - AST comparison confirmed exactly the requested class was removed and all 17 remaining PDF tests are unchanged
  - Required quick pre-flight passed
  - Required architecture suite: 18 passed and 1 failed on 62 existing broken Design-to-Reference links
  - Graphify AST refresh passed without inference
  - Whitespace check passed
  - Conversion tests and live inference were not run
---

### [2026-10-06 00:23 CEST] — Document developer-led converter iterations and test scope
- **Affected Subsystems**:
  - `AGENTS.md`
  - `.agents/rules/unit-testing-policy.md`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/html-to-markdown/README.md`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Define change then user-requested fragment conversion then user assessment as the converter development workflow
  - Retain existing configuration, transport, cache and recovery tests without default test expansion
  - Exclude independent pilots, quality scoring and reinstatement of removed crop tests
  - Route agent and converter entrypoints to the shared workflow and testing policy
- **Architectural Rationale & Trade-Offs**:
  - Persist the user's agreed workflow for subsequent sessions while keeping repository commit checks and runtime schema validation distinct from conversion-quality assessment
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Documentation governance audit reported zero issues
  - All 9 existing skill-governance tests passed
  - All 14 added Markdown links and anchors resolved
  - Architecture suite: 18 passed and 1 failed on 62 existing broken Design-to-Reference links
  - No converter tests, fragment conversion or live inference ran
---

### [2026-10-06 00:28 CEST] — Ignore local test-book materials and remove the old page tracker
- **Affected Subsystems**:
  - `.gitignore`
  - `Obsidian/Amiga/Reference/Test Book example-4567-codex/TEST_PAGES_TRACKER.md`
- **What Was Changed (The Concrete Reality)**:
  - Commit the pending ignore patterns for local test-book directories and timing-analysis variants
  - Record removal of the old tracked Codex test-page matrix
  - Remove the obsolete tracker exception and redundant test-book ignore entries
- **Architectural Rationale & Trade-Offs**:
  - Apply the requested commit while keeping local reference inputs and conversion artifacts outside Git
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on the same 62 existing broken Design-to-Reference links
  - git check-ignore verified all four representative local reference paths are ignored
  - No active tracker references found in searched agent, roadmap, tooling and developer documentation
  - No converter tests or inference ran
---

### [2026-10-06 00:31 CEST] — Verify converter response-cache reuse and invalidation
- **Affected Subsystems**:
  - `tests/test_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Extend cache replay checks to text, image and JSON calls through fresh client/cache instances
  - Add four behavioral tests for request changes, rejected cache entries, cached JSON validation and cache-directory isolation
  - Extend identity checks for instructions and runtime/contract versions
  - Keep converter implementation unchanged
- **Architectural Rationale & Trade-Offs**:
  - Add the user's explicitly requested technical cache tests and verify actual transport-call avoidance rather than only comparing request hashes
- **Verification & Test Results**:
  - Existing shared/HTML baseline: 15 passed
  - Updated shared/HTML suite: 19 passed
  - Both converter suites: 36 passed in 1.702 seconds
  - Deliberately disabling cache load or save was detected by the persistence test
  - Quick pre-flight passed
  - Architecture suite: 18 passed and 1 failed on 62 existing broken Design-to-Reference links
  - Graphify AST refresh passed
  - All conversion checks used fake transport and temporary directories with no live inference
---

### [2026-10-06 01:03 CEST] — Milestone Digest: PDF Text-Layer Implementation (PDF-TEXT-1.1)
- **Timestamp & Context**: Implementation commit `5a9add8` on 2026-10-06 delivered PDF-TEXT-1.1-A/B/C. The user subsequently closed the task using page-64 evidence and the known verification limits; this digest records settled implementation, not a fully passing repository milestone.
- **Affected Subsystems**: `tools/bootstrap/conversion/`, PDF stages 00/01 and Stage 02 input validation, converter configuration/orchestration, both converter test suites, converter documentation and roadmap.
- **What Was Changed**:
  - Stage 00 publishes a separate validated OCR PDF and native/OCR/no-text provenance. Deterministic Stage 01 exclusively reads that reopened PDF and persists physical page IDs, displayed text geometry and matching PNG transforms.
  - Shared lineage checks artifact contents and exact page coverage before reuse/cleanup. Registry position lookup replaces numeric asset-propagation offsets; restart retains only validated predecessors and compatible OCR recovery.
  - Two focused regressions protect missing Stage 00 rejection and original stream preservation with q/Q wrappers. The existing invalid-cache test also protects schema-valid inverted OCR boxes; each defect was reproduced before repair.
- **Architectural Rationale & Trade-Offs**:
  - Preserve original/native content and certify only selected preparation coverage. Reopened PDF text, geometry and identical selected-page renders establish publication safety; cached OCR JSON is not the final text-layer artifact.
  - PyMuPDF adds q/Q wrappers to unwrapped streams. Verification permits those wrappers while requiring byte-identical source streams in order, unchanged native/unselected content and exact selected-page renders.
  - Validate OCR semantics before cache publication and on hits. Ship dependent stage/configuration/restart changes atomically; keep the Stage 02 model-request redesign and additional fragment selection separate.
- **Verification & Invariants**:
  - At implementation: PDF suite 19/19, shared-client suite 19/19, quick preflight and CLI help passed. Semantic parity and Graphify refresh completed; all 26 tracked emulator specifications were within audit tolerance.
  - Authorized TestBook page 64 matched source-manual page 5 by render and exercised actual OCR. Source hash and all 160 page geometries/native or unselected content survived; Stage 01 extracted 20 blocks from the reopened PDF, with identical selected-page render/PNG and zero inference.
  - One live OCR request produced the response; successful reruns reused cache. Restart at 01 retained preparation and physical selection; modified retained JSON was rejected. A disposable Unicode probe verified serializer positions through all four rotations and a nonzero crop offset.
  - Architecture at implementation: 18/19, with 62 pre-existing broken Design-to-Reference links in untouched documents. Native-copy, mixed, blank/graphic-only and real rotated/cropped fragment acceptance remained unverified. No full-book conversion or RAG indexing occurred. Final closure checks are reported in the separate closure entry.
---

### [2026-10-06 01:12 CEST] — Close PDF-TEXT-1.1 with recorded evidence limits
- **Affected Subsystems**:
  - `ROADMAP.md`
  - `DIARY.md`
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_segmentation/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Close PDF-TEXT-1.1 by the explicit user decision after outstanding acceptance limits were stated
  - Remove its acceptance-only roadmap point and renumber remaining conversion work 1.1-1.4 with matching active references
  - Compact the settled implementation entry for 5a9add8 into its own evidence-preserving digest and retain unrelated active diary entries
  - Record administrative scope closure separately from source-category verification and repository milestone health
- **Architectural Rationale & Trade-Offs**:
  - The user accepts closing this scope using existing page-64 evidence without requesting additional native mixed blank or rotated/cropped fragments
  - Preserve the distinction between verified converter behavior and untested categories or failing repository gates
  - Keep source fidelity and publication safeguards unchanged
- **Verification & Test Results**:
  - Quick preflight passed
  - Tier 1 unit and Tier 2 integration harnesses passed
  - PDF tests 19/19 and shared client tests 19/19 passed
  - Milestone preflight executed and failed Architecture Rules plus Docs Quality on 62 existing broken links and Code Quality on 22 compound conditions in unchanged Rust code
  - Separate architecture run 18/19
  - Design-sync review passed with all 26 tracked emulator specifications within tolerance and no affected checkpoints
  - Converter semantic parity closure review preserved implemented behavior and bounded claims with no Python-code diff from 5a9add8
  - Settled implementation compaction reduced diary line count from 1253 to 1236 before adding this closure entry while preserving source-stream wrappers Unicode geometry restart cache regressions and sample evidence
  - No additional conversion model calls RAG ingestion unrelated Rust repair or golden-reference change performed
---

### [2026-10-06 01:16 CEST] — Remove closed execution plans as part of task closure
- **Affected Subsystems**:
  - `AGENTS.md`
  - `.agents/rules/structural-root-cause.md`
  - `.agent/tasks/`
- **What Was Changed (The Concrete Reality)**:
  - Add a shared execution-plan lifecycle rule and route it from root agent instructions
  - Require agents to delete a closed task plan after its durable closure record is committed without separate cleanup confirmation
  - Preserve active paused blocked and unrelated plans and keep verification limits in permanent history
  - Remove the ignored roadmap-1.1-pdf-text-layer.md plan after confirming its closure is already recorded in DIARY.md and commits 5a9add8 and 98eaa9f
- **Architectural Rationale & Trade-Offs**:
  - Execution plans guide active work and should not accumulate as duplicate completed-task history
  - Plan cleanup belongs to the agent closing workflow with no hooks or background services
  - Explicit user closure may end scope while failed or unrun checks remain accurately recorded
- **Verification & Test Results**:
  - Closed plan absence verified
  - Skill-governance tests 9/9 and documentation-auditor tests 7/7 passed
  - Quick preflight passed with AGENTS.md at 7895 bytes within its 14000-byte ceiling
  - Architecture suite 18/19 with the same 62 pre-existing missing Design links
  - No implementation or unrelated plan changed
---

### [2026-10-06 01:27 CEST] — Stage reference downloads in per-book temporary directories
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/bootstrap.ps1`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Store downloaded PDF and HTML sources in per-book -tmp directories
  - keep PDF workspaces with sources and route finished Markdown/assets to sibling directories without the suffix
  - update staging README and developer usage documentation
  - retain existing HTML parent mirroring and existing source directories
- **Architectural Rationale & Trade-Offs**:
  - Separate raw sources and resumable conversion artifacts from finished reference output as requested
  - preserve existing downloads without implicit migration
- **Verification & Test Results**:
  - PowerShell parsing and isolated routing smoke check passed for all six catalog items plus crawl fallback without network or inference
  - existing bootstrap contract tests passed 4/4
  - quick preflight passed all six checks
  - architecture rules passed 18/19 with the link-integrity test reporting 18 missing Reference targets outside this change
  - Graphify AST update completed
  - live downloads and conversion not run
---

### [2026-10-06 01:35 CEST] — Publish converted books only to empty destinations
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/bootstrap.ps1`
  - `tools/bootstrap/conversion/publication.py`
  - `HTML and PDF pipeline CLIs`
  - `converter documentation`
  - `tests/test_conversion_publish.py`
- **What Was Changed (The Concrete Reality)**:
  - Remove output-dir from both public pipeline CLIs
  - retain PDF results in workspace/14_link_toc and HTML results in workspace/html_to_markdown
  - add explicit Publish switches and boolean converter forwarding
  - reject all occupied book destinations before download or inference and recheck before atomic publication
  - copy Markdown and assets without overwriting book files
  - remove automatic HTML parent mirrors
  - retain working sources and metrics
- **Architectural Rationale & Trade-Offs**:
  - Make publication explicit and protect existing books from cleanup and overwrite
  - share destination validation and prepared-copy publication across both converters
- **Verification & Test Results**:
  - Publication safeguards and CLI tests passed 10/10
  - shared Codex tests passed 19/19
  - PDF orchestration tests passed 19/19
  - bootstrap contract tests passed 4/4
  - PowerShell parsing and local routing checks passed all catalog cases plus crawl fallback
  - coordinator Publish forwarding and both pipeline help commands passed
  - quick preflight passed all six checks
  - architecture rules passed 18/19 with 15 pre-existing broken Reference links
  - Graphify AST update completed
  - no live downloads or model requests run
---

### [2026-10-06 01:38 CET] — Remove bootstrap switch aliases
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1`
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/bootstrap_sources.ps1`
- **What Was Changed (The Concrete Reality)**:
  - Remove all explicit switch aliases and their help text
  - remove remaining-argument help compatibility handling
  - retain canonical switches and native PowerShell parameter binding
- **Architectural Rationale & Trade-Offs**:
  - Use one documented switch name consistently across the bootstrap scripts
- **Verification & Test Results**:
  - PowerShell parsing and canonical help checks passed for all three scripts
  - removed Qdrant Convert Process AllMirrors aliases and positional help rejected
  - native parameter abbreviations and double-dash Help binding remain available
  - bootstrap contract tests passed 4/4
  - quick preflight passed all six gates
  - architecture suite passed 18/19 with 15 pre-existing broken Reference links
  - Graphify AST update completed
  - git diff check passed
---

### [2026-10-06 01:40 CET] — Clarify the bootstrap Sources help scope
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap.ps1`
- **What Was Changed (The Concrete Reality)**:
  - Describe Sources as external sources encompassing reference emulators
  - test suites and vectors in both usage and options
- **Architectural Rationale & Trade-Offs**:
  - The delegated sources catalog provisions more than SingleStepTests
- **Verification & Test Results**:
  - Bootstrap help executed successfully
  - quick preflight passed all six gates
  - architecture suite passed 18/19 with 15 existing broken Reference links
  - no provisioning logic changed
---

### [2026-10-06 01:48 CEST] — Clarify compound branch predicates
- **Affected Subsystems**:
  - `CPU`
  - `debugger and disassembler`
  - `DMA`
  - `floppy`
  - `frame builder`
  - `GUI`
  - `test runner`
- **What Was Changed (The Concrete Reality)**:
  - Replace opcode membership chains with matches patterns
  - Name interrupt eligibility and branch decisions
  - Reuse the existing fixed DMA slot map
  - Express MFM sync and cutout membership directly
  - Preserve lazy GUI event evaluation and existing silicon verification tolerances
- **Architectural Rationale & Trade-Offs**:
  - Make branch intent explicit without changing hardware behavior or golden references
  - The milestone condition checker is a regex heuristic rather than model inference
- **Verification & Test Results**:
  - Milestone gate 11/11 PASS
  - Quick gate PASS
  - Architecture tests 19/19 PASS
  - Affected crate tests 224/224 PASS including 38 GUI interactions
  - Cartesian DMA tests 20/20 PASS
  - Full SingleStepTests with SINGLESTEP_FULL=1: 126/127 PASS with DIVU vector 5744 failing on RAM byte 0x0007FE
  - The same DIVU failure was reproduced with all 13 refactored files restored to HEAD and the refactored sources then restored
  - Graphify incremental AST update completed
  - No new baseline regressions demonstrated and no reference vectors modified
---

### [2026-10-06 01:53 CET] — Rename the PDF TOC heading segment type
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_schemas.py`
  - `tools/bootstrap/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Rename the TOC title type to toc_heading in the segmentation prompt and response schema
  - Update chapter partitioning, prose transformation, Markdown emission and pipeline documentation
- **Architectural Rationale & Trade-Offs**:
  - Distinguish a body TOC heading from running page headers
  - Existing segmentation artifacts require explicit regeneration from Stage 02
- **Verification & Test Results**:
  - Modified Python files compile successfully
  - No obsolete identifier remains in bootstrap sources
  - Quick per-commit gate PASS
  - Architecture tests 19/19 PASS
  - Graphify incremental update completed
  - No fragment conversion requested or run
---

### [2026-10-06 02:03 CEST] — Clip extracted PDF text bounds to the displayed page
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_geometry.py`
  - `PDF stages 00-01`
- **What Was Changed (The Concrete Reality)**:
  - Intersect extracted text bounding boxes with the displayed page before validation and normalization
  - Preserve complete extracted text and original PDF content
  - Retain strict OCR geometry validation and reject malformed or fully outside extraction boxes
  - Document the approved clipping behavior
- **Architectural Rationale & Trade-Offs**:
  - PDF-BOUNDS-1: user approved clipping existing native text bounds after physical TestBook page 2 exceeded the page width
- **Verification & Test Results**:
  - Focused regression failed before the fix and passed afterward
  - PDF conversion tests 21/21 PASS
  - Quick per-commit gate PASS
  - Architecture tests 19/19 PASS
  - All-page stages 00-01 rerun passed page 2 and reached page 65 where invalid OCR text stopped Stage 00
  - Stage 01 did not run
  - Stage 00 recorded 2 live calls and 1 cache hit
  - Graphify refresh attempted but unavailable because the graphify Python module is not installed
---

### [2026-10-06 02:19 CEST] — Persist immediate Stage 00 page reads for inspection
- **Affected Subsystems**:
  - `PDF Stage 00`
  - `bootstrap conversion diagnostics`
- **What Was Changed (The Concrete Reality)**:
  - PDF-PAGE-READS-1: write per-page JSON under 00_text_layer/page_reads for native extraction and schema-shaped OCR responses
  - Preserve rejected OCR text and its validation error before aborting
  - Rebuild read records for cached and compatible recovery responses
  - Document partial-read and restart semantics
- **Architectural Rationale & Trade-Offs**:
  - Make already-read text inspectable when full-document PDF preparation fails without promoting rejected OCR into validated recovery or downstream inputs
- **Verification & Test Results**:
  - Focused rejected-OCR regression failed before the fix and passed afterward
  - PDF tests 22/22 PASS
  - Quick per-commit gate PASS
  - Architecture tests 19/19 PASS
  - Requested all-page stages 00-01 rerun wrote pages 1-65 including rejected page 65
  - Stage 00 still halted on unreadable OCR and Stage 01 did not run
  - One live request and two cache hits
  - Graphify refresh attempted but graphify module unavailable
---

### [2026-10-06 02:26 CEST] — Omit unreadable OCR markers from invisible PDF text
- **Affected Subsystems**:
  - `PDF Stage 00`
  - `OCR inspection records`
- **What Was Changed (The Concrete Reality)**:
  - PDF-OCR-OMIT-1: replace OCR U+FFFD markers with spaces and omit marker-only blocks before insertion
  - Keep raw OCR and the derived insertion response with omitted-marker counts in per-page JSON
  - Preserve strict native text and OCR geometry/control-character validation
  - Align the OCR prompt and conversion contract with the user-approved policy
- **Architectural Rationale & Trade-Offs**:
  - Unreadable markers indicate missing transcription and should not become searchable PDF text
  - Spaces preserve separation between readable fragments without inventing missing content
- **Verification & Test Results**:
  - Focused regression failed before implementation
  - PDF tests 23/23 PASS including reopening a generated PDF with readable text and no U+FFFD
  - Quick per-commit gate PASS
  - Architecture tests 19/19 PASS
  - All-page stages 00-01 rerun passed page 65 with one marker omitted and reached page 72
  - Stage 00 halted on an inverted OCR bounding box on page 72 and Stage 01 did not run
  - Page reads 1-72 retained
  - Ten live calls and no cache hits
  - Graphify refresh attempted but graphify module unavailable
---

### [2026-10-06 09:32 CEST] — PDF-OCR-TESSERACT: replace Stage 00 model OCR with local Tesseract
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tools/bootstrap/conversion`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Use PyMuPDF Tesseract OCR on selected textless page renders and publish <source stem>-ocr.pdf
  - remove Stage 00 model selection and obsolete prompt
  - preserve native text, invisible insertion, publication validation and compatible per-page recovery
  - retain existing uncommitted reversed-box normalization changes
  - update artifact consumers, deterministic dispatch and documentation
  - normalize fractional Tesseract bounds into the integer 0-1000 schema
- **Architectural Rationale & Trade-Offs**:
  - The user requested the local Tesseract text-layer mechanism used by the reference OCR script instead of Codex. Keep the validated Stage 00 to Stage 01 handoff and source fidelity without copying margin filters or schematic omission heuristics.
- **Verification & Test Results**:
  - PASS 25 PDF conversion tests
  - 19 shared Codex tests
  - 10 publication tests
  - quick pre-flight
  - 19 architecture checks and git diff --check. Real Tesseract synthetic-image recognition
  - blank-image result and reopened invisible-text geometry passed after fixing the initially observed fractional-coordinate schema rejection. Graphify incremental update completed. No user source fragment
  - full-book conversion or milestone gate run.
---

### [2026-10-06 12:18 CEST] — PDF-OCR-CLEANUP: remove Stage 00 OCR corrections
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove OCR box clipping, reversed-corner correction, unreadable-marker substitution and marker-only block omission
  - remove the derived response and correction counters
  - validate OCR directly and retain raw inspection records
  - remove obsolete correction tests and update documentation
- **Architectural Rationale & Trade-Offs**:
  - The user requested removing model-era OCR corrections after switching Stage 00 to Tesseract. Preserve integer coordinate conversion
  - validation and PDF publication checks.
- **Verification & Test Results**:
  - PASS 23 PDF conversion tests
  - quick pre-flight
  - 19 architecture checks and git diff --check. Graphify incremental update completed. No source fragment conversion or milestone gate run.
---

### [2026-10-06 12:19 CEST] — PDF-STAGE00-READS: remove duplicate preparation text JSON
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove page_reads directory generation and immediate native/OCR inspection JSON introduced by 2077f3d
  - validate OCR directly before insertion or recovery publication
  - keep failure-publication protection in the adapted test
  - document Stage 01 ownership of per-page positioned text JSON
- **Architectural Rationale & Trade-Offs**:
  - The user requested removing the Stage 00 inspection feature because Stage 01 extracts the prepared PDF into JSON. Retain the validation manifest and restart recovery independently of content extraction.
- **Verification & Test Results**:
  - PASS 23 PDF conversion tests
  - quick pre-flight
  - 19 architecture checks and git diff --check. Graphify incremental update completed. No source fragment conversion or milestone gate run.
---

### [2026-10-06 14:24 CET] — Document PDF workspace directory convention
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Keep PDF workspaces under the book staging directory's workspace subdirectory
  - document named children for independent conversion attempts and explicit --workspace selection
- **Architectural Rationale & Trade-Offs**:
  - Record the user-selected directory layout so future conversion runs keep working state in a predictable per-book location.
- **Verification & Test Results**:
  - PASS quick pre-flight, 19 architecture checks and git diff --check. Documentation only
  - no conversion run.
---

### [2026-10-06 14:27 CET] — Place PDF workspaces beside their source PDF
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace the staging-only convention with a workspace subdirectory of the source PDF directory
  - align the quick-start path and explain the existing CLI default and optional experiment children
- **Architectural Rationale & Trade-Offs**:
  - The user corrected the directory convention: conversion state belongs under the directory containing the PDF
  - including locally supplied sources outside bootstrap staging.
- **Verification & Test Results**:
  - PASS quick pre-flight, 19 architecture checks and git diff --check. Documentation only
  - no conversion run or artifact relocation.
---

### [2026-10-06 14:38 CET] — Accept zero extents in normalized OCR boxes
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_geometry.py`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Allow zero width and height only in normalized OCR validation
  - retain reversed-edge and range checks and strict native geometry validation
  - add the reproduced quantization regression and document downstream insertion limits
- **Architectural Rationale & Trade-Offs**:
  - The user requested keeping integer 0-1000 coordinates while accepting extents collapsed by rounding.
- **Verification & Test Results**:
  - Regression failed before repair and passed after repair. PASS 24 PDF conversion tests, quick pre-flight, 19 architecture checks and git diff --check. Graphify updated. Synthetic zero-width, zero-height and point-box insertion each failed reopened text preservation
  - full source conversion not rerun and remains unresolved.
---

### [2026-10-06 14:44 CET] — Clamp OCR coordinates to displayed page bounds
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_geometry.py`
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Clamp all OCR line coordinates before integer 0-1000 normalization
  - retain zero extents and reject reversed corners before clipping
  - update the conversion contract and stage documentation
- **Architectural Rationale & Trade-Offs**:
  - The user requested clipping OCR bounds after the selected PDF failed on page 64 with a line extending beyond page height.
- **Verification & Test Results**:
  - Regression failed before repair and passed afterward. PASS 25 PDF conversion tests, quick pre-flight, 19 architecture checks and git diff --check. Graphify updated. Requested all-page 00-01 conversion processed 160 pages but Stage 00 failed reopened text preservation with missing or extra lines
  - Stage 01 did not run. Repeat using compatible OCR recovery confirmed the same failure.
---

### [2026-10-06 15:00 CET] — Trust PyMuPDF OCR text insertion in Stage 00
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_geometry.py`
  - `tools/bootstrap/conversion/pdf_artifacts.py`
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove OCR line count, text and position comparisons after PDF insertion and from predecessor validation
  - remove unused expected-line metadata and validator
  - retain source, native-content, geometry, visible-render and artifact identity checks
  - document the user-selected trusted-writer contract
- **Architectural Rationale & Trade-Offs**:
  - The user approved the review PDF and requested trusting PyMuPDF text insertion rather than testing its line segmentation and round-trip placement.
- **Verification & Test Results**:
  - Regression failed before repair and passed afterward. PASS 26 PDF conversion tests, quick pre-flight, 19 architecture checks and git diff --check. Graphify updated. All 160 requested pages completed stages 00 and 01, producing prepared OCR PDF and 160 PNG/JSON pairs with zero Codex calls. Independent completed-state and artifact validation passed. PowerShell capture returned exit 1 after Tesseract stderr warnings despite pipeline completion
  - completion was verified from validated state and artifacts.
---

### [2026-10-06 15:12 CET] — Preserve the original Tesseract PDF text layer
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/00_text_layer`
  - `tools/bootstrap/conversion/pdf_geometry.py`
  - `tools/bootstrap/conversion/pdf_schemas.py`
  - `tests/test_pdf_conversion_codex.py`
  - `tests/test_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Overlay original OCR PDF text operators and fonts without rebuilding strings
  - remove normalized coordinate conversion, rounding, clipping and bounding-box font fitting
  - replace only the duplicate OCR raster with transparency
  - preserve native pages and original page rotation
  - retain original per-page OCR PDFs with recovery identity/hash metadata
  - remove obsolete normalized OCR schemas, validators and tests
- **Architectural Rationale & Trade-Offs**:
  - The user requested passing Tesseract output directly to PyMuPDF without modifying text positions or spacing.
- **Verification & Test Results**:
  - Raw-overlay regression failed before repair and passed afterward. PASS 23 PDF conversion tests, 19 shared Codex tests, quick pre-flight, 19 architecture checks and git diff --check. Rotated overlay diagnostic preserved displayed coordinates within 0.000008 points. Graphify updated. All 160 requested pages completed stages 00 and 01 with 160 PNG/JSON pairs and zero Codex calls
  - completed-record digests and prepared PDF hash verified. PowerShell log capture returned exit 1 after Tesseract stderr warnings despite confirmed pipeline completion. User assessment of text selection remains separate.
---

### [2026-10-06 15:32 CET] — PDF-SEGMENTATION-REVIEW-02K: add graphical segmentation review
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/pipeline.py`
  - `tools/bootstrap/pdf-to-markdown/stages/02k_segmentation_review`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Add deterministic Stage 02k between segmentation and raw-stream construction
  - retain original page resolution and add an equal-width right panel
  - draw containing 2-pixel frames with numbered type and optional heading-level labels in JSON order
  - distinguish coincident frames through separate connector anchors
  - use registry order for stage resolution and restart retention
  - update documentation
- **Architectural Rationale & Trade-Offs**:
  - The user requested a quick visual assessment of bounding-box coverage and classification without printing coordinate values or changing segmentation. Preserve completed 00-02 artifacts and avoid new model requests.
- **Verification & Test Results**:
  - PASS 24 PDF conversion tests, quick pre-flight, 19 architecture checks and git diff --check. Graphify updated. User-selected page 20 completed Stage 02k with 14 frames and a visually inspected 4200x2704 PNG from the original 2100x2704 image, with zero model calls. The initial raster-key typo and outdated numeric-stage test expectation were corrected before completion. Earlier source-stage records remain retained. Close PDF-SEGMENTATION-REVIEW-02K
  - no roadmap milestone or broader conversion run.
---

### [2026-10-06 15:35 CET] — PDF-JSON-STREAM-ORDER: preserve reviewed segment order
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02k_segmentation_review`
  - `tools/bootstrap/pdf-to-markdown/stages/03_build_raw_stream`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Keep review labels in JSON array order
  - replace independent elbow routes with shared-boundary straight panel leaders so vertical-order inversions remain visible
  - use frame-center ports without artificial ranking of coincident boxes
  - remove Stage 03 coordinate sorting and preserve each page array when assigning sequential nodes
  - document stream-order semantics
- **Architectural Rationale & Trade-Offs**:
  - The user clarified that segmentation JSON represents the intended Markdown reading stream and crossed leaders should expose an ordering problem rather than be hidden by label rearrangement.
- **Verification & Test Results**:
  - Focused Stage 03 order regression failed before repair and passed afterward. PASS 25 PDF conversion tests
  - quick pre-flight
  - 19 architecture checks and git diff --check. Graphify updated. Page-20 Stage 02k regenerated and visually inspected: 14 JSON-ordered labels
  - coincident graphic boxes sharing a port
  - and zero model calls. No full Markdown conversion run. Close PDF-JSON-STREAM-ORDER.
---

### [2026-10-06 16:01 CET] — PDF-INDEPENDENT-02D-02M: plan the separate page-conversion branch
- **Affected Subsystems**:
  - `ROADMAP.md`
  - `.agent/tasks/pdf-independent-02d-02m.md`
- **What Was Changed (The Concrete Reality)**:
  - Record pending Stage 02d independent object grouping, new classifications and raw_text/md_text output
  - define deterministic Stage 02m review and dependency-aware restart
  - preserve the existing 01-02-02k branch unchanged
  - retain the active local execution plan
- **Architectural Rationale & Trade-Offs**:
  - The user requested planning only the new 01-02d-02m branch
  - with OCR JSON as assistance and no Markdown formatting hints. Later stream integration is deferred.
- **Verification & Test Results**:
  - PASS quick pre-flight, 19 architecture checks and git diff --check. Planning documentation only
  - no converter implementation, model calls or fragment conversion. PDF-INDEPENDENT-02D-02M remains planned and open.
---

### [2026-10-06 16:13 CET] — PDF-INDEPENDENT-02D-02M: implement independent page conversion and review
- **Affected Subsystems**:
  - `tools/bootstrap/conversion`
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Add explicit execution order 01-02-02d-02k-02m with artifact-specific dependencies and selective restart
  - add strict independent logical objects and ordered md_text with full PNG/text JSON
  - add deterministic crop-only graphical review
  - preserve existing 02/02k workers and compatible old completion identities through exact fingerprint bridge
  - document the Stage 02d formatting exception
  - retain the active plan for user review
- **Architectural Rationale & Trade-Offs**:
  - The new page conversion must group logical source objects independently of OCR blocks while retaining the old downstream stream and authenticated original-detail Codex transport. Validate every selected interval external input before cleanup.
- **Verification & Test Results**:
  - PASS 26 PDF technical tests, 19 shared Codex tests, quick pre-flight, 19 architecture checks, compileall, CLI help and git diff --check. Graphify updated. Offline fake-transport diagnostic confirmed complete inputs, decoded Markdown and model order, cache reuse, deterministic review PNG dimensions/bytes and selective 02d/02m restart with unchanged old records. The missing-external-input regression failed before repair and passed afterward. No live inference or real fragment conversion
  - user-selected conversion/review and downstream 03-14 integration remain pending. No roadmap milestone completion.
---

### [2026-10-06 16:46 CET] — PDF-INDEPENDENT-02D-02M: narrow table descriptions to source legends
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_page_conversion.py`
  - `tools/bootstrap/conversion/pdf_lineage_compatibility.json`
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Rename table_description to table_legend in the separate Stage 02d schema and prompt
  - require source text below a table defining its symbols, notation or abbreviations
  - classify general explanations as prose while retaining captions and individually referenced footnotes
  - update documentation and active plan
  - preserve old-branch lineage compatibility
- **Architectural Rationale & Trade-Offs**:
  - The user narrowed the role to a table notation legend. Classification belongs to Stage 02d
  - Stage 02k only renders the older branch. Existing JSON is not relabeled in place and the new schema/prompt invalidates previous 02d responses.
- **Verification & Test Results**:
  - PASS 26 PDF tests
  - quick pre-flight
  - 19 architecture checks
  - compileall and git diff --check. Graphify updated. Runtime diagnostic accepted table_legend and rejected the former enum
  - validated existing 00/01/02/02k records and confirmed earlier 02d identity requires regeneration. No new conversion or model calls. Previous eight-page outputs remain available for review with their former classification.
---

### [2026-10-06 16:52 CET] — PDF-INDEPENDENT-02D-02M: require bounds for every logical object
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_page_conversion.py`
  - `tools/bootstrap/conversion/pdf_lineage_compatibility.json`
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion`
  - `tools/bootstrap/pdf-to-markdown/stages/02m_page_conversion_review`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Require a nonempty original-PNG integer bbox for every Stage 02d object
  - request complete logical bounds independently of OCR blocks
  - render frames and straight leaders for every Stage 02m object by reusing the unchanged Stage 02k renderer with an identity pixel transform
  - update documentation and the active plan
  - preserve compatible old-branch identities
- **Architectural Rationale & Trade-Offs**:
  - The user requested textual object geometry to support graphical review, even where downstream Markdown assembly does not need it. Keep model array order and Markdown unchanged
  - reject null or invalid bounds before caching and handoff without synthesizing OCR rectangles.
- **Verification & Test Results**:
  - PASS 26 PDF tests, quick pre-flight, 19 architecture checks, compileall and git diff --check. Graphify updated. Technical diagnostic verified caption/table/legend/footer boxes, rejection of null/outside/empty/noninteger rectangles, four ordered frames and leaders, review dimensions, and retained 00/01/02/02k lineage. No live model calls or real fragment conversion
  - prior 02d/02m artifacts require regeneration for this contract.
---

### [2026-10-06 17:05 CEST] — PDF-02D-TABLE-CLASSIFICATION: distinguish decorative frames from graphics
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion/prompt.md`
- **What Was Changed (The Concrete Reality)**:
  - Classify objects by internal content and structure
  - recognize regular text or number grids as tables despite decorative frames or Figure captions
  - preserve cell boundaries and meaningful diagram lines
- **Architectural Rationale & Trade-Offs**:
  - The reviewed page-126 grid was classified as graphic. Clarify the classification criterion without changing Markdown formatting recipes.
- **Verification & Test Results**:
  - PASS quick pre-flight and 19 architecture checks
  - prompt diff reviewed and git diff --check passed
  - no fragment conversion or model calls
  - existing outputs were not regenerated
---

### [2026-10-06 17:07 CET] — PDF-02D-NOTE-LABELS: classify table and figure note introductions by role
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion/prompt.md`
- **What Was Changed (The Concrete Reality)**:
  - Reserve headings for document sections and subsections
  - keep table and figure note labels with their content and classify by semantic role
  - preserve source emphasis and include the approved prose example
- **Architectural Rationale & Trade-Offs**:
  - The reviewed page-130 note introduction was classified as a level-two heading. The user approved an explicit rule and Markdown example to distinguish attached explanations from document hierarchy.
- **Verification & Test Results**:
  - PASS quick pre-flight and 19 architecture checks
  - prompt diff reviewed and git diff --check passed
  - no fragment conversion or model calls
  - existing outputs were not regenerated
---

### [2026-10-06 17:15 CET] — PDF-02D-TYPE-DESCRIPTIONS: enumerate logical object classifications
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion/prompt.md`
- **What Was Changed (The Concrete Reality)**:
  - Describe all 23 schema types in individual bullet points
  - clarify callouts, code listings, instruction banners, continuation captions and navigation entries
  - preserve independent object grouping, attached-note rules and pixel bounds
- **Architectural Rationale & Trade-Offs**:
  - Make the Stage 02d classification vocabulary explicit like Stage 02 while retaining the independent branch contract and dedicated index/list headings.
- **Verification & Test Results**:
  - PASS quick pre-flight, 19 architecture checks, exact 23-type schema/list comparison and git diff --check. No model calls or fragment conversion
  - existing outputs were not regenerated.
---

### [2026-10-06 17:24 CET] — PDF-02D-CONTINUATION: represent named page continuations as a boolean
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_page_conversion.py`
  - `tools/bootstrap/conversion/pdf_lineage_compatibility.json`
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion/prompt.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove caption_continuation from Stage 02d types
  - require continuation on every response and stored object
  - recognize explicit top-of-page continuation names including Concluded and later sheets
  - retain normal caption and dedicated heading types
  - preserve compatible old-branch procedure identities
- **Architectural Rationale & Trade-Offs**:
  - Continuation is an independent property of a source-visible name
  - rather than a separate caption classification. Reject old 02d artifacts and regenerate 02d/02m instead of migrating saved classifications.
- **Verification & Test Results**:
  - PASS 26 PDF unittest tests, quick pre-flight, 19 architecture checks, response/stored boolean diagnostics, rejection of absent or invalid values and former type, exact 22-type prompt comparison, compatibility fingerprint verification and git diff --check. Graphify updated. Initial pytest invocation could not run because pytest is not installed
  - the suite passed through its native unittest entry point. No model calls or fragment conversion
  - existing outputs were not regenerated.
---

### [2026-10-06 17:28 CET] — PDF-02D-COVER: classify complete book covers as one object
- **Affected Subsystems**:
  - `tools/bootstrap/conversion/pdf_page_conversion.py`
  - `tools/bootstrap/conversion/pdf_lineage_compatibility.json`
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Add cover to response and stored schemas
  - define one complete cover object including artwork, branding and source-visible publication text
  - document regeneration and preserve compatible old-branch fingerprints
- **Architectural Rationale & Trade-Offs**:
  - The reviewed cover was split into graphic
  - heading and prose because the independent stage vocabulary lacked cover. Stage 02m already renders arbitrary validated object labels.
- **Verification & Test Results**:
  - Reproduced schema rejection with the retained cover regression before repair. PASS 27 PDF unittest tests, quick pre-flight, 19 architecture checks, exact 23-type prompt/schema comparison, compatibility fingerprint verification, generic cover rendering diagnostic and git diff --check. Graphify updated. No model calls or fragment conversion
  - existing outputs were not regenerated.
---

### [2026-10-06 17:30 CET] — PDF-02M-CONTINUATION-LABELS: display continuation in review labels
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02m_page_conversion_review`
  - `tools/bootstrap/pdf-to-markdown/stages/02k_segmentation_review/render_review.py`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Show explicit continuation true/false on every Stage 02m object label
  - opt in through the shared renderer while preserving Stage 02k labels
  - document the review output
- **Architectural Rationale & Trade-Offs**:
  - Stage 02d already stores the boolean but Stage 02m omitted it from graphical review
- **Verification & Test Results**:
  - Reproduced missing labels with the focused regression before repair
  - PASS 28 PDF unittest tests, quick pre-flight and 19 architecture checks
  - Graphify updated
  - no model calls or fragment conversion, existing review PNGs were not regenerated
---

### [2026-10-06 17:35 CET] — Group diagram-connected explanations into graphics
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02d_page_conversion`
- **What Was Changed (The Concrete Reality)**:
  - Clarify prose exclusions and complete graphic boundaries for visually connected explanatory text
  - Apply graphic grouping before separate note classification
  - Document regeneration requirement
- **Architectural Rationale & Trade-Offs**:
  - Complete explanatory sentences connected by meaningful diagram geometry belong to the figure rather than independent prose objects
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture rules passed 19/19
  - git diff --check passed
  - selected physical page 96 converted in an independent workspace through 00, 01, 02d and 02m with validated lineage
  - 02d made one fresh Codex call and emitted heading, graphic and footer
  - both former prose objects and the connected note are inside the graphic
  - original and new page PNG hashes match
  - no other fragment or downstream conversion run
---

### [2026-10-06 17:37 CET] — PDF-WORKSPACE-DIRS: create final output only when Stage 14 executes
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/pipeline.py`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/pdf-to-markdown/README.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove unconditional final output directory creation during pipeline initialization
  - retain a regression for partial execution through 02m
  - document lazy stage directory creation
  - verify the reported Stage 14 directory is absent at completion
- **Architectural Rationale & Trade-Offs**:
  - Independent page-conversion restarts retain the old stream and therefore did not clean the prematurely created final output directory
- **Verification & Test Results**:
  - Regression failed on original code for premature Stage 14 directory creation
  - all 29 PDF tests passed after repair
  - quick pre-flight passed
  - architecture rules passed 19/19
  - Graphify updated
  - no model calls or fragment conversion
---

### [2026-10-06 17:40 CET] — Remove obsolete PDF segmentation and review stages
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove stages 02 and 02k, their configuration and response schemas
  - move review drawing primitives into 02m
  - remove obsolete lineage compatibility bridge
  - adapt restart and lineage tests and documentation
  - reject legacy Stage 03 assembly until its input contract is redesigned
- **Architectural Rationale & Trade-Offs**:
  - Keep the user-selected 02d/02m page-conversion workflow without dependencies on deleted stages or silently adapting incompatible Markdown/pixel objects into legacy raw-text/PDF-point nodes
- **Verification & Test Results**:
  - PDF conversion tests: 29 passed
  - shared conversion tests: 19 passed
  - quick pre-flight passed
  - architecture tests: 19 passed
  - CLI help and git diff --check passed
  - graphify update completed
  - no fragment conversion run. Current fragment runs must stop at 02m
  - Stage 03-14 end-to-end conversion remains unavailable. Shared procedure changes require explicit regeneration of incompatible workspace records.
---

### [2026-10-06 17:46 CET] — Renumber PDF page stages and connect raw-stream assembly
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Rename page conversion 02d to 02 and review 02m to 02.5 across stage directories, registry, configuration, validation, tests and documentation
  - make Stage 03 depend directly on Stage 02
  - assemble validated objects preserving Markdown, classifications, IDs, continuation flags and pixel boxes
  - derive legacy text and geometry fields for downstream workers
- **Architectural Rationale & Trade-Offs**:
  - Make page conversion the canonical stream input while keeping graphical review independent of stream lineage and restart invalidation
- **Verification & Test Results**:
  - 30 PDF converter tests and 19 shared converter tests passed
  - quick pre-flight passed
  - 19 architecture tests passed
  - CLI help and git diff --check passed
  - Graphify refreshed. No fragment conversion requested or run. Old stage/configuration identities require regeneration
  - later stream formatting and merging behavior remains unchanged.
---

### [2026-10-06 18:17 CET] — PDF-REVIEW-TRUE: omit false continuation labels
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Render continuation labels only when true
  - adapt the existing label test and documentation
  - regenerate the 37 selected workspace review PNGs through Stage 02.5
- **Architectural Rationale & Trade-Offs**:
  - Reduce review label clutter while preserving required boolean validation and the Stage 02 data contract
- **Verification & Test Results**:
  - PDF converter tests: 30 passed
  - quick pre-flight passed
  - architecture rules: 19 passed
  - Stage 02.5 completed with validated lineage and zero model calls
  - Graphify refreshed with no LLM calls
  - git diff --check passed
---

### [2026-10-06 18:31 CET] — PDF-FOOTNOTE-GROUP: preserve complete referenced notes
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Establish complete object boundaries before classification
  - retain continuation lines and abbreviation definitions within marker-referenced footnotes
  - restrict table legends to independent source blocks
  - synchronize converter documentation
- **Architectural Rationale & Trade-Offs**:
  - Prevent sentence-level role classification from splitting one source footnote into footnote and table_legend objects
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture rules passed 19/19
  - physical page 67 completed stages 00-02.5 in an independent footnote-page-67 workspace with one live Codex request and no cache hits
  - JSON and review PNG contain one footnote encompassing both source lines
  - git diff --check passed
  - no full-book conversion or new tests
---

### [2026-10-06 CEST] — PDF-02-DEFERRED-TEXT: defer graphic table and cover content
- **Affected Subsystems**:
  - `PDF Stage 02`
  - `page conversion validation`
- **What Was Changed (The Concrete Reality)**:
  - Require empty md_text for graphic/table/cover in the prompt and response/artifact validation
  - Preserve complete classification and pixel bounds
  - Update the conversion contract and existing contract test
- **Architectural Rationale & Trade-Offs**:
  - Defer internal transcription and content reconstruction until later image-crop processing
  - Reject incompatible saved content rather than silently clearing it
- **Verification & Test Results**:
  - PDF conversion suite: 30 tests passed
  - Quick pre-flight passed
  - Architecture suite: 19 tests passed
  - Graphify incremental update completed
  - No source fragment conversion requested or run
---

### [2026-10-06 18:42 CET] — Classify aligned parameter assignments as code blocks
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
- **What Was Changed (The Concrete Reality)**:
  - Distinguish aligned input/output and register assignment blocks from tables regardless of font
  - preserve shared labels and wrapped descriptions
  - document the classification boundary
- **Architectural Rationale & Trade-Offs**:
  - Column alignment alone led to parameter blocks being treated as tables and invented column headers
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion requested or run
---

### [2026-10-06 18:48 CEST] — Clarify contextual page reading order
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
- **What Was Changed (The Concrete Reality)**:
  - Use top-to-bottom and left-to-right as the default reading order
  - allow clear layout and semantic relationships to place headings before associated content despite vertical offsets
- **Architectural Rationale & Trade-Offs**:
  - Geometric sorting alone can place section content before its offset heading
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion requested or run
---

### [2026-10-06 18:51 CEST] — Exclude incidental adjacent-page fragments from page conversion
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Identify intended-page content before extraction
  - exclude adjacent-page fragments and associated text
  - preserve incomplete intended-page objects and uncertain ownership
- **Architectural Rationale & Trade-Offs**:
  - Preserving every visible mark can classify clipped neighboring-page artwork as intended-page content
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion requested or run
---

### [2026-10-06 18:52 CEST] — Group complete technical drawing sheets as graphics
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Group integrated drawing descriptions titles views dimensions notes and connection tables into one graphic
  - apply sheet grouping before table and caption classification
  - retain independent document content and page furniture separately
- **Architectural Rationale & Trade-Offs**:
  - Technical drawing composition can establish object membership without explicit leader lines
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion requested or run
---

### [2026-10-06 18:56 CEST] — Classify source ASCII art as code blocks
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Include visibly character-based diagrams and timing waveforms in code_block
  - preserve characters spaces line breaks and alignment in fenced text blocks
  - distinguish drawn graphics from source ASCII art
- **Architectural Rationale & Trade-Offs**:
  - Character-built geometry requires faithful text transcription rather than graphic reconstruction
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion requested or run
---

### [2026-10-06 19:07 CET] — Keep register-map and bit-assignment tables out of graphic grouping
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Classify repeated address or offset rows with bit positions and field descriptions as table
  - retain connecting brackets and leader lines inside table bounds
  - give this rule precedence over graphic grouping and document regeneration
- **Architectural Rationale & Trade-Offs**:
  - Local bit-to-description connectors must not turn a register table into a drawing sheet
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - no fragment conversion run after prompt change
---

### [2026-10-06 19:10 CEST] — Preserve visible order around offset headings
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace semantic heading promotion with visible source-row ordering
  - Keep short blocks before adjacent multiline headings when visually above most of the heading
  - Document Stage 02 and review regeneration
- **Architectural Rationale & Trade-Offs**:
  - Page 81 review showed Equipment Flags after its adjacent heading because the previous prompt explicitly allowed heading promotion
  - Subject association must not override the requested visible order
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite passed 19/19
  - Fragment conversion not run after prompt change and model adherence remains unverified
---

### [2026-10-06 19:14 CEST] — Prioritize character-built layouts over table and graphic classification
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Recognize character-built timing and bit-field structures before register tables and drawing-sheet grouping
  - Add explicit table exclusions for character geometry despite column headings offsets and captions
  - Preserve integrated aligned data and explanations inside code blocks
- **Architectural Rationale & Trade-Offs**:
  - Reviewed page 153 was grouped as graphic and page 156 bit-field layout as table
  - Earlier classification precedence overrode the ASCII-art rule
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture suite passed 19/19
  - Source review images and stored classifications inspected
  - No new conversion run and model adherence remains unverified
---

### [2026-10-06 19:16 CET] — Require named PDF attempt workspaces from the first run
- **Affected Subsystems**:
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tools/bootstrap/pdf-to-markdown/README.md`
- **What Was Changed (The Concrete Reality)**:
  - Require an explicit named workspace child even when the workspace container is empty
  - distinguish independent attempts from continuation and restart
  - update the quick-start example
- **Architectural Rationale & Trade-Offs**:
  - Keep the first and subsequent agent-run attempts organized consistently without changing the CLI default or migrating existing artifacts
- **Verification & Test Results**:
  - Quick preflight passed
  - architecture rules passed (19 tests)
  - documentation-only change, no conversion requested or run
---

### [2026-10-09 18:34 CEST] — VERIFY-01: Centralize change-aware completion verification
- **Affected Subsystems**:
  - `tools/harness`
  - `tests`
  - `agent governance`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Replace pre_flight.py with verify.py --commit and --milestone
  - Select real-component crate tests through reverse workspace dependencies and route synthetic CPU suites separately
  - Route Python contracts by their tool environment and reject incomplete staging or changing snapshots
  - Centralize completion instructions while retaining focused diagnostic commands
- **Architectural Rationale & Trade-Offs**:
  - Completion commands must be selected mechanically from the actual test environment rather than agent judgement
  - Milestone includes all test suites plus chipset throughput and runnable vAmiga capture comparison without rewriting goldens
- **Verification & Test Results**:
  - Nine verifier regression tests passed
  - Vault links and script governance passed
  - Existing PDF publication test fails in unchanged code with expected 16 versus actual 1
  - Commit verification passed including 19 architecture tests and five affected Python fixture suites
  - Milestone selection plan inspected; full milestone not run
---

### [2026-10-09 19:01 CEST] — VERIFY-02: Keep standard verification unconditional and expose CPU suite overrides
- **Affected Subsystems**:
  - `tools/harness`
  - `verification tests`
  - `agent instructions`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Always run ordinary workspace tests, standard test_runner suites and Python contracts for commit verification
  - Retain staged-path triggers only for Cartesian DMA and full SingleStepTests
  - Add independent additive --verify-dma and --verify-singlestep switches
  - Keep milestone as the complete profile including both CPU suites
  - Remove reverse dependency and Python fixture selection machinery
- **Architectural Rationale & Trade-Offs**:
  - Measured runtimes support a fixed standard profile with only the two expensive synthetic CPU suites selected conditionally
  - Explicit overrides allow verification without altering trigger paths or disabling normal tests
- **Verification & Test Results**:
  - Nine routing and CLI regressions passed
  - Commit verification ran all standard tests and omitted both expensive suites in 36.86 seconds
  - Quality gates including 19 architecture tests and all ordinary crate tests passed
  - Existing golden trace, Copper, matcher and PDF publication failures remain and produce a failed gate
  - Generated trace outputs were preserved under .agent/tmp and restored
  - Full milestone not run
---

### [2026-10-09 19:15 CEST] — VERIFY-03: Allow independent DMA and SingleStep test sets
- **Affected Subsystems**:
  - `tools/harness`
  - `verifier regressions`
  - `agent instructions`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Allow --verify-dma and --verify-singlestep without --commit or --milestone
  - Standalone selectors run only their explicit sets on the current working tree without staging or quality gates
  - Preserve automatic commit triggers, complete milestone coverage and additive deduplicated combinations
  - Reject empty and conflicting completion selections
- **Architectural Rationale & Trade-Offs**:
  - Targeted CPU verification must be callable independently from completion profiles without implicitly executing the standard commit suite
- **Verification & Test Results**:
  - Eleven CLI and selection regressions passed
  - Combined standalone command ran only the two requested sets in 58.08 seconds
  - Cartesian DMA passed and full SingleStepTests retained the existing DIVU failure
  - Link and governance checks passed
  - Commit profile retains known golden trace, Copper, matcher and PDF publication failures
  - No goldens or expectations changed
  - Full milestone not run
---

### [2026-10-09 19:22 CEST] — VERIFY-04: Separate milestone checks from commit verification
- **Affected Subsystems**:
  - `tools/harness`
  - `verifier regressions`
  - `agent rules and skills`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Milestone runs only its four additional audits, chipset throughput and full vAmiga capture checks
  - Remove implicit commit tests and DMA/SingleStep selection from milestone
  - Retain explicit CPU-set selectors alongside either profile
  - Update every active milestone workflow to run commit once for the same snapshot before milestone
- **Architectural Rationale & Trade-Offs**:
  - Verification profiles must have independent execution scopes and avoid repeating standard commit checks during stage closure
- **Verification & Test Results**:
  - Eleven verifier regression tests passed including disjoint gate and command selection
  - Vault links and script governance passed
  - Commit baseline retains previously recorded golden trace, Copper, matcher and PDF publication failures
  - Milestone selection inspected without running the complete capture catalog
  - No golden expectations changed
---

### [2026-10-09 20:15 CEST] — VERIFY-05: Allow verification without staging
- **Affected Subsystems**:
  - `Verification harness`
  - `agent workflow`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Allow commit and milestone sets on clean or dirty working trees
  - Select expensive CPU suites from staged, unstaged and nonignored new paths
  - Fingerprint untracked file contents and retain mutation detection
  - Update regression tests and remove staging prerequisites from guidance
- **Architectural Rationale & Trade-Offs**:
  - Verification profiles describe test sets and must be runnable on demand
  - independently of Git staging.
- **Verification & Test Results**:
  - 12 verifier regression tests passed
  - documentation scripts, vault links and governance passed
  - Graphify updated
  - full commit verification ran without staging: quality gates and ordinary crate tests passed, existing test_benchmark_trace, test_vamiga_copper, test_vamiga_harness and PDF publication contract failures remain. Trace tests rewrote tracked reference outputs, causing snapshot rejection
  - generated traces preserved under ignored diagnostics and original references restored. Milestone execution not run.
---

### [2026-10-09 20:17 CEST] — VERIFY-06: Show verification instructions without arguments
- **Affected Subsystems**:
  - `Verification CLI`
  - `testing documentation`
- **What Was Changed (The Concrete Reality)**:
  - Print detailed help and exit successfully when no arguments are provided
  - Explain commit and milestone sets, conditional CPU-suite selection, explicit selectors and plan-only behavior
  - Preserve invalid argument errors and add a regression proving help performs no verification
- **Architectural Rationale & Trade-Offs**:
  - Expose the available verification sets and their selection rules directly at the command entry point.
- **Verification & Test Results**:
  - 13 verifier regression tests passed
  - manual no-argument and help output checked
  - documentation links and governance passed
  - Graphify updated
  - full commit verification retains existing trace, Copper, capture matcher and PDF publication failures plus snapshot rejection from generated traces. Generated reference traces restored without changing goldens. Milestone execution not run.
---

### [2026-10-09 20:22 CEST] — VERIFY-07: Limit crate-root architecture scanning to Cargo source trees
- **Affected Subsystems**:
  - `Architecture verification`
- **What Was Changed (The Concrete Reality)**:
  - Scan crates and immediate Cargo helper-tool directories for prohibited lib.rs files instead of traversing the repository
  - Retain crate root and Cargo.toml entry-point validation
- **Architectural Rationale & Trade-Offs**:
  - Repository artifacts and external reference sources are outside the workspace naming policy and dominated traversal time. Helper Cargo tools remain covered by the existing policy.
- **Verification & Test Results**:
  - All 19 architecture tests passed in 0.13-0.14s versus previous approximately 4.55s
  - verifier Architecture Rules total 0.38s versus previous 4.78s
  - temporary scope probes confirmed detection in crates and Cargo helpers and exclusion of diagnostics
  - Graphify updated. Full commit verification retains existing benchmark trace, Copper, capture matcher and PDF publication failures. Generated tracked traces restored
  - no golden references changed. Milestone execution not run.
---

### [2026-10-09 20:26 CEST] — VERIFY-08: Remove redundant quality preview from execution output
- **Affected Subsystems**:
  - `Verification console output`
- **What Was Changed (The Concrete Reality)**:
  - Print the Quality preview only in plan mode
  - Preserve quality gate execution and RUN/PASS/FAIL progress messages
- **Architectural Rationale & Trade-Offs**:
  - Normal execution already reports each quality check and its result
  - making the initial list redundant.
- **Verification & Test Results**:
  - 13 verifier regression tests passed
  - inspected normal output without Quality preview and plan output retaining it
  - Graphify updated
  - full commit verification retains known benchmark trace, Copper, capture matcher and PDF publication failures. Generated tracked traces restored
  - no golden changes. Milestone execution not run.
---

### [2026-10-10 01:42 CEST] — Refresh SHIFT-04 trace after canonical mnemonic correction
- **Affected Subsystems**:
  - `Benchmark execution traces`
- **What Was Changed (The Concrete Reality)**:
  - Refresh SHIFT-04 instruction labels from ROXL/ROXR to ASL/ASR
  - Preserve opcodes, register deltas, CCK timings and existing golden hashes
- **Architectural Rationale & Trade-Offs**:
  - Align the tracked trace artifact with the canonical disassembler output and the golden hash approved in commit c621cdc. Scoped design review selected Performance Profiling and Optimization Strategy
  - no documentation change is required for this artifact refresh.
- **Verification & Test Results**:
  - SHIFT-04 normalized FNV-1a matches the existing golden hash 0xA7B1BA76B22AC2C7
  - python tests/verify.py --commit ran: ordinary crate tests, coverage reporting, formatting, Clippy, architecture, hardware quality and code quality passed
  - Standard harness suite failed on BCD-02 and BCD-04 trace hash mismatches
  - Python tool contracts failed in test_validated_pdf_resume_publishes_workspace_output
  - No golden constants changed
  - Milestone verification not run
---

### [2026-10-10 02:02 CEST] — Reorganize verification tooling and scope automated test coverage
- **Affected Subsystems**:
  - `Verification harness`
  - `quality checks`
  - `agent governance`
  - `test_runner`
  - `frame_builder tests`
  - `design documentation`
- **What Was Changed (The Concrete Reality)**:
  - Move verification and coverage reporting to tests and concrete audits to tests/checks
  - Move skill-owned documentation, semantic, diary and roadmap checks into their skill script directories
  - Move benchmark analysis into test_runner and worktree utility into tools/harness
  - Remove standalone harness utility tests, vAmigaTS regression suites, viewport extraction tests and selected Python audit regression tests while retaining emulator execution and golden-result checks
  - Automatically report ordinary-crate LLVM coverage and preserve diagnostic tier selectors
  - Consolidate quality audit scope and make milestone verification the staged English audit
  - Require scoped design review before authorized commits, remove obsolete synchronization checkpoints and align instructions with the new layout
  - Refresh eight remaining canonical-mnemonic trace artifacts without changing golden constants
  - Add tracked-path ownership for benchmark analysis and verification
- **Architectural Rationale & Trade-Offs**:
  - Record all user-authorized pending changes as one verification and agent-workflow reorganization. Documentation review covered changed design metadata and the benchmark, testing, frame-buffer and profiling contracts
  - production emulator behavior was not changed. Removed suites reduce automated regression coverage
  - retained manual tools do not replace that coverage.
- **Verification & Test Results**:
  - python tests/verify.py --commit ran: ordinary workspace crate tests and formatting, Clippy, architecture, hardware and code quality gates passed
  - Ordinary-crate line coverage 52.41 percent, functions 58.62 percent and regions 49.80 percent, without a threshold
  - Standard harness verification failed on existing BCD-02 and BCD-04 golden trace mismatches
  - Python contracts failed in test_validated_pdf_resume_publishes_workspace_output
  - All eight changed trace artifacts match existing golden hashes
  - Vault links and frontmatter checks passed for 42 documents and 1054 links
  - Roadmap retention check and git diff --check passed
  - Graphify code graph updated
  - Milestone closure, full SingleStepTests, Cartesian DMA and manual capture/performance runs not performed
---

### [2026-10-10 03:02 CET] — Initialize opcode PC in silicon and DMA test fixtures
- **Affected Subsystems**:
  - `test_runner`
  - `CPU SingleStepTests specification`
- **What Was Changed (The Concrete Reality)**:
  - Initialize instruction_pc from the primed PC in SingleStep and DMA fixtures
  - Document opcode-address initialization and add scoped tracked-path ownership
  - Retain existing silicon-vector coverage without the user-rejected additional regression test
- **Architectural Rationale & Trade-Offs**:
  - The first instruction previously inherited a zero opcode address
  - corrupting the divide-by-zero exception frame for DIVU vector 5744. Existing full SingleStepTests cover the demonstrated failure.
- **Verification & Test Results**:
  - python tests/verify.py --commit executed: full SingleStepTests 127/127 and 962412/962412 vectors passed
  - Cartesian DMA 20/20 passed
  - Ordinary crate tests and formatting, Clippy, architecture, hardware and code quality gates passed
  - Standard harness retains existing BCD-02 and BCD-04 trace hash failures
  - Python contracts retain the existing PDF publication failure
  - Vault frontmatter and all 1054 links passed
  - Milestone verification not run
  - Golden references unchanged
---

### [2026-10-10 03:03 CET] — Use service-owned configuration for read-only RAG tools
- **Affected Subsystems**:
  - `Amiga RAG MCP adapter`
  - `bootstrap RAG commands`
  - `developer documentation`
- **What Was Changed (The Concrete Reality)**:
  - Remove index-state configuration and arguments from MCP search, status and source listing
  - Keep explicit source and state-file selection for indexing
  - Update bootstrap status invocations, documentation and command contract tests
  - Integrate with Qdrant read-contract commit e8b9b14
- **Architectural Rationale & Trade-Offs**:
  - Read-only clients should not choose an indexing ledger. The persistent service owns the read configuration
  - so worktree-local state paths no longer block retrieval. Scoped tooling and documentation review required no additional emulator specification changes.
- **Verification & Test Results**:
  - python tests/verify.py --commit executed: ordinary crate tests, formatting, Clippy, architecture, hardware and code quality passed
  - All 10 MCP and 4 bootstrap contract tests passed
  - Python contracts retained only the existing PDF publication failure
  - Standard harness retained existing BCD-02 and BCD-04 trace hash failures
  - Qdrant preflight --full passed with 35 tests
  - After service restart live CLI status, source listing and search passed, and fresh stdio MCP status and search passed
  - Already connected MCP process still requires client-side reload
  - Graphify updated
  - Milestone verification not run
  - No indexing or golden changes
---

### [2026-10-10 13:15 CEST] — Remove benchmark trace regressions and hash fixtures
- **Affected Subsystems**:
  - `test_runner`
  - `benchmark documentation`
  - `verification policy`
- **What Was Changed (The Concrete Reality)**:
  - Removed benchmark tracing APIs and CLI option plus all 108 reference traces
  - Removed golden hash tables and checks in favor of direct CSV reference comparisons
  - Moved generated benchmark reports into ignored .agent/tmp directories while preserving tracked performance baseline lookup
  - Updated selected specifications and reference-integrity rules
- **Architectural Rationale & Trade-Offs**:
  - Microbenchmarks retain program generation and execution without comparing historical diagnostic output
  - Authoritative SingleStep and DMA suites remain the CPU and contention correctness checks
- **Verification & Test Results**:
  - Final-source verification passed ordinary Rust tests and quality gates plus benchmark tests
  - Full SingleStepTests passed 127/127 and Cartesian DMA passed 20/20
  - Vault link and frontmatter audit passed
  - Commit verification remains failing solely on the pre-existing test_validated_pdf_resume_publishes_workspace_output Python contract
---

### [2026-10-10 13:23 CEST] — Align PDF publication resume fixture with completed stages
- **Affected Subsystems**:
  - `tests/test_conversion_publish.py`
- **What Was Changed (The Concrete Reality)**:
  - Populate the simulated resume state with completion records for every registered PDF stage
  - Preserve the full-prefix validation assertion and add a merge note to discard this local adjustment in favor of the converter development branch
- **Architectural Rationale & Trade-Offs**:
  - The fixture mocked all stages as complete while retaining only Stage 00 in state
  - Keep this correction limited to the test because the separate branch owns the evolving lineage and resume contracts
- **Verification & Test Results**:
  - Publication tests passed 10/10
  - python tests/verify.py --commit passed all selected checks including ordinary Rust and benchmark tests plus Python tool contracts
  - No CPU or DMA implementation changed and dedicated silicon suites were not selected for this fixture-only change
---

### [2026-10-10 15:16 CEST] — Check English vocabulary across the repository and clarify documentation entry points
- **Affected Subsystems**:
  - `tests/checks`
  - `tests/verify.py`
  - `docs`
  - `.agents`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Replace the former language detector with pyspellchecker and explicit approved vocabulary
  - Scan unchanged tracked and nonignored new text files on every commit and milestone using the maintained check_english.json scope
  - Use half the available logical CPUs with one to six processes and deterministic reports
  - Require a user decision before correcting unknown words or expanding vocabulary exceptions
  - Organize docs as reader orientation and move operational procedures to agent references and tests/README.md
  - Update affected links and approved vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Check English words directly while keeping large generated and external collections outside the audit
  - Keep English findings visible until the user decides how to resolve them
  - Separate project orientation from operational details
- **Verification & Test Results**:
  - PASS python tests/verify.py --commit on final code and documentation
  - PASS one-process and six-process comparison across 574 files including unknown words and read errors
  - PASS process count bounds and full scans of 571 files at about one second
  - PASS 42 design documents and 1048 vault links
  - PASS 267 local documentation links and git diff --check
  - Graphify code graph refreshed
  - Full SingleStep and Cartesian DMA were not selected because no emulator source changed
---

### [2026-10-10 15:48 CEST] — Organize verification assets and tool contract tests
- **Affected Subsystems**:
  - `test_runner`
  - `debugger tests`
  - `verification tooling`
  - `bootstrap and RAG MCP contracts`
  - `agent skills`
- **What Was Changed (The Concrete Reality)**:
  - Move tracked benchmark baselines to baselines/benchmarks and debugger examples to examples/m68k
  - Move local benchmark reports and SingleStep snapshots into ignored .agent/tmp directories and update worktree copying
  - Move six Python contract files beside their owning tools and discover four suites independently from verify.py
  - Remove the redundant silicon vector prerequisite and the worktree source assertion test
  - Move the frontmatter parser into its sole consumer audit_vault.py
  - Update documentation links and tracked path ownership without changing baseline content
- **Architectural Rationale & Trade-Offs**:
  - Separate executable tests from examples and reference data while keeping local results outside authored source
  - Keep contract tests beside their tools and preserve the existing verification entry point
  - Retain existing benchmark files without overwriting newer local results
- **Verification & Test Results**:
  - PASS commit verification before the final documentation review including all four Python contract suites and full SingleStepTests
  - PASS discovery of all 74 retained Python test methods
  - PASS final vault audit across 42 specifications and 1048 links
  - PASS byte preservation during relocation of 266 files and unchanged tracked baseline content
  - PASS worktree script syntax validation
  - Milestone verification and Cartesian DMA were not run for this organization change
---

### [2026-10-10 17:30 CEST] — VERIFY-CLI-01: centralize verification tools and commands
- **Affected Subsystems**:
  - `tools/verification`
  - `tools/worktree.ps1`
  - `tools/log_diary.py`
  - `crates/cpu/tests`
  - `agent rules and skills`
  - `design and developer documentation`
- **What Was Changed (The Concrete Reality)**:
  - Move verification orchestration and checks from tests to tools/verification and shared worktree and diary utilities to tools
  - Replace switch-based selection with commit, milestone, test, check and runner commands and route skill diagnostics through Verify
  - Port 17 architecture inspection checks to Python and retain the compiled opcode coverage test unchanged in CPU tests
  - Remove the unused HTML conversion audit and reference comparison scripts
  - Update active callers, command references, ownership mappings and approved programming vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Verification describes repository checks and diagnostics rather than emulator fixtures
  - Architecture source inspection requires no Rust compilation
  - Preserve quality gates, ordinary crate coverage, change-aware silicon selection, logging and source/index stability
  - Keep bootstrap, conversion, indexing, worktree operations and reference recording outside verification
- **Verification & Test Results**:
  - Commit verification passed ordinary crate coverage, all quality gates, standard harness suites, full SingleStepTests, Cartesian DMA and all five Python contract directories
  - Verification contracts passed 35 tests
  - Rust and Python architecture implementations matched on 47 fixture outcomes across 17 controls
  - Scoped design review covered eight specifications with no unmapped crate paths
  - Vault audit passed 42 specifications and 1049 links
  - No roadmap milestone closure or reference baseline changes
---

### [2026-10-10 17:54 CEST] — DESIGN-ORGANIZATION: organize design specifications and authoring guidance
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design`
  - `agent rules and skills`
  - `verification tooling`
  - `test_runner documentation references`
- **What Was Changed (The Concrete Reality)**:
  - Keep system hardware and peripheral specifications plus Frame Buffer in the Design root
  - Move 11 application verification and performance documents into three subdirectories and update incoming links
  - Combine machine-loop specifications under Machine Loop.md with crate ownership and execution details
  - Replace PowerShell Guidelines with the write-powershell skill and remove the vAmigaTS scorecard and its maintenance requirements
  - Make link and metadata audits recursive and resolve links from each source directory with regression coverage
  - Apply the user-approved wording correction in the PowerShell skill
- **Architectural Rationale & Trade-Offs**:
  - Group specifications by responsibility while preserving existing hardware content
  - Keep authoring procedures with their owning skill and preserve simple no-argument PowerShell execution
  - Retain the unresolved DMA slot-description discrepancy without changing hardware expectations or protected references
- **Verification & Test Results**:
  - Commit verification passed ordinary crate tests with coverage and all quality gates including the full English audit
  - Standard harness suites and all five Python contract directories passed
  - Vault audit passed 39 specifications and 1026 links
  - Scoped pre-commit design review found no unmapped changed crate paths and confirmed 37 documents changed only in their links
  - All 19 architecture contract tests passed including nested-note regressions
  - Skill validation passed and the code graph was refreshed
  - Milestone verification and full SingleStepTests and DMA sets were not required or run for this documentation change
---

### [2026-10-10 18:16 CEST] — TEST-RUNNER-CLI-01: adopt sentence-style diagnostic commands
- **Affected Subsystems**:
  - `crates/test_runner`
  - `tools/verification`
  - `agent skills`
  - `design specifications`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace diagnostic command switches with summary diff suite bench chipset and vamiga sentences
  - Parse selector values into execution configurations before running workloads
  - Forward the same sentence selectors through verification and external profiling
  - Update command examples and specification ownership for the new CLI module
- **Architectural Rationale & Trade-Offs**:
  - Keep diagnostic invocation consistent with the verification grammar
  - Reject unknown or repeated selectors and invalid counts with exit code 2
  - Apply benchmark profile defaults before explicit overrides independently of argument order
  - Preserve standalone reference recording and block it through verification
- **Verification & Test Results**:
  - Commit verification passed ordinary crate coverage all quality gates standard harness suites and all five Python contract directories
  - Three Rust CLI tests passed including real binary help and argument error checks
  - Scoped documentation review covered three specifications and resolved the new CLI module ownership
  - Vault audit passed 39 specifications and 1026 links before the added parser link
  - Initial Clippy visibility findings were corrected and the final Clippy check passed
  - Graphify code graph refreshed
  - Full SingleStepTests Cartesian DMA milestone checks hardware captures and performance measurements were not selected or run for this CLI change
---

### [2026-10-10 18:40 CEST] — VAMIGA-HARNESS-REMOVE-01: remove the previous capture harness
- **Affected Subsystems**:
  - `crates/test_runner`
  - `crates/frame_builder`
  - `crates/denise`
  - `tools/verification`
  - `baselines/benchmarks`
  - `DIARY.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove the vAmigaTS execution module and its library exports command routing and CLI test cases
  - Remove the dependent coptim1 chipset benchmark and its stored baseline
  - Remove capture viewport extraction and the orphaned Denise display window setter while retaining equivalent test input state
  - Classify step_frame as the documented host stepping API in the code quality audit
  - Remove the earlier chipset calibration digest and its summary references
- **Architectural Rationale & Trade-Offs**:
  - Retire the previous test-driven chipset integration before future implementation work
  - Keep CPU benchmarks SingleStepTests Cartesian DMA and host frame stepping
  - Preserve bootstrap documentation roadmap external sources and older local artifacts at the user request
  - Retained documentation still contains commands for the removed integration
- **Verification & Test Results**:
  - Final implementation verification passed ordinary crate coverage all quality gates standard harness suites full SingleStepTests Cartesian DMA and all five Python contract directories
  - Focused CLI and Denise pixel pipeline tests passed
  - Pre-commit design review covered five mapped specifications and the Machine Loop stepping contract
  - Library export and removed harness paths lacked tracked path ownership and mapping edits were deferred because the user retained documentation unchanged
  - Initial test-only API findings and formatting failure were resolved
  - Graphify code graph refreshed and git diff --check passed
  - Milestone verification vault audits hardware captures and performance measurements were not run
---

### [2026-10-10 23:28 CET] — TEST-RUNNER-TARGETS-01: separate Cargo test and benchmark targets
- **Affected Subsystems**:
  - `crates/test_runner`
  - `crates/physical_memory`
  - `crates/machine_loop/tests`
  - `tools/verification`
  - `agent rules and skills`
  - `design specifications`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Move benchmark implementation into the Cargo benchmark target and shared silicon helpers into tests/common
  - Remove the test_runner application and benchmark validation suites while retaining measurement reports and reference baselines
  - Generate SingleStep summary and diff reports automatically
  - Route benchmark analysis and profiling through verification tooling
  - Separate commit checks from milestone CPU suites and select Python contracts by affected inputs during development
  - Prevent empty standard harness selection from running every Cargo test target
  - Remove the unused BusResult::ok helper and its dedicated assertions with user approval and preserve machine integration checks through explicit Ready matching
  - Retain set_unmapped_byte as documented host configuration API
  - Add user-approved tomllib vocabulary and correct moved-module visibility and formatting
  - Synchronize specification paths command examples dependencies and completion policy
- **Architectural Rationale & Trade-Offs**:
  - Keep the verification package as Cargo test and benchmark targets with no runtime application
  - Preserve existing benchmark measurements and protected reference artifacts
  - Keep full synthetic CPU suites at milestone or explicit diagnosis boundaries
  - Resolve audit findings against actual symbol users and intentional host APIs
- **Verification & Test Results**:
  - Focused CLI contracts passed after an empty-standard-selection regression reproduced the original failure
  - Workspace compilation passed
  - Vault audit passed 39 specifications and 1020 links
  - Scoped design review covered benchmark silicon memory and verification specifications including both sides of moved paths
  - Initial commit checks exposed vocabulary visibility and unused API findings and the obsolete helper assertions
  - Final python tools/verification/verify.py commit passed ordinary crate tests with coverage all quality gates the full English audit and all five Python contract directories
  - Final commit profile selected no standard harness command because only the explicitly excluded CPU targets remain
  - Full SingleStepTests Cartesian DMA performance measurements and milestone checks were not selected for the final source snapshot
  - Graphify code graph refreshed
  - No roadmap milestone closure performance recording or baseline update
---

### [2026-10-11 01:26 CEST] — SINGLESTEP-FULL-01 and CPU-CYCLE-01: enforce timed comparison and repair register ADD.l
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/test_runner`
  - `agent rules and skills`
  - `design specifications`
  - `docs/testing.md`
  - `tools/verification/README.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove weaker SingleStep comparison modes and require state clocks and timed bus callbacks for every executed vector
  - Preserve internal intervals in the reference clock sequence and record observed CCK callback clocks
  - Add missing MOVEP suites remove the redundant PEA sample and execute formerly skipped ASL cases
  - Add a comparator regression that rejects a shifted read with unchanged data and total clocks
  - Move the four internal clocks after prefetch in ADD.l Dn,Dn and ADD.l An,Dn
  - Correct retired capture and benchmark commands and document synthetic fixture and comparison limits
- **Architectural Rationale & Trade-Offs**:
  - A correct result and total clock count do not prove the correct bus phase
  - Keep silicon references unchanged and sampling limited to input count
  - Repair CPU behavior in small batches with review evidence under CPU-CYCLE-01
  - Earlier state-focused results are not a current complete-comparison baseline
- **Verification & Test Results**:
  - The pre-repair exhaustive run selected 124 files and 978542 vectors with 519902 failures under stronger comparison
  - Batch 01 now passes all 787 register ADD.l vectors and all 10042 selected vectors including NOP and register ADD.b and ADD.w controls
  - Extra batch diagnostics checked final prefetch and formal PC
  - The full containing ADD.l file gains exactly 787 passes with no regressions and still fails 6757 of 8065 cases outside batch 01
  - Focused ADD and ADDX Cartesian tests full NOP comparator regression formatting architecture Clippy and vault checks passed
  - Scoped design review selected six specifications with no unmapped crate paths and left the CPU programmer model unchanged
  - python tools/verification/verify.py commit passed ordinary crate tests with coverage all quality gates the full English audit the transaction checker and all five Python contract directories
  - No milestone closure full final CPU run hardware capture performance measurement reference change or human acceptance is claimed
---

### [2026-10-11 01:49 CEST] — Centralize the hardware-derived DMA schedule
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design`
  - `tools/verification/checks/english_words`
- **What Was Changed (The Concrete Reality)**:
  - Remove the duplicate DMA timetable and arbitration narrative from Machine Loop and the cross-chip catalog
  - Rebuild DMA.md from HRM allocation diagrams and Fat Agnus signal descriptions with explicit coordinate and phase limits
  - Record the observed fifth refresh allocation on long NTSC lines without changing runtime behavior
  - Add user-approved RAS CAS and unrequested vocabulary
- **Architectural Rationale & Trade-Offs**:
  - Keep scheduling in the DMA specification and signal routing in the coordinator and connection catalog
  - Treat the existing scheduler as implementation evidence rather than hardware authority
  - Preserve the unresolved refresh boundary and whole-machine verification limits
- **Verification & Test Results**:
  - Scoped design review selected the three changed design notes and Testing Strategy by tracked_paths with no missing ownership and no further corrections
  - Vault frontmatter and link audit passed 39 specifications and 1048 links
  - Focused DMA and Agnus tests passed 10 and 24 tests
  - Production Agnus observation covered eight complete PAL and NTSC lines with forced disk requests and the existing audio request path and observed five refresh selections on long NTSC lines
  - python tools/verification/verify.py commit passed ordinary crate tests with coverage standard transaction tests all quality gates the full English audit and all five Python contract directories
  - Full SingleStepTests Cartesian DMA hardware captures and milestone closure were not run or claimed
---

### [2026-10-11 02:34 CEST] — SINGLESTEP-FORMAT-01: use one silicon-vector contract
- **Affected Subsystems**:
  - `crates/test_runner`
  - `CPU verification documentation`
  - `silicon test skill`
  - `docs/testing.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove both name-based is_harte detectors and always prime the internal PC from the formal input PC plus four
  - Remove the comparator format argument and legacy byte-lane conversion and optional UDS/LDS handling
  - Update existing comparator calls and describe the silicon-only schema without unsupported strict-validation claims
  - Close SINGLESTEP-FORMAT-01 after commit verification and retire its execution plan
- **Architectural Rationale & Trade-Offs**:
  - Test names are human descriptions with no execution semantics
  - Only the current physical-silicon vectors are supported and future external datasets require their own code changes
  - The user requested removal of both newly added regression tests and the additional tag and record-length validator
  - RAM comparison exceptions remain assigned to SINGLESTEP-RAM-01 and CPU repairs remain assigned to CPU-CYCLE-01
- **Verification & Test Results**:
  - The existing transaction timing test passed after the final parser simplification
  - Earlier focused NOP and TST.b checks each passed 8065 silicon vectors and sampled Cartesian TST passed
  - Structural inspection covered all 1000060 input vectors and found only the expected silicon tags and field counts
  - Scoped design review selected two specifications with no missing mappings and left the testing strategy unchanged
  - Vault audit passed 39 documents and 1049 links
  - python tools/verification/verify.py commit passed ordinary crate tests with coverage the existing transaction timing test all quality gates the full English audit and all five Python contract directories
  - No full final CPU run or milestone closure is claimed and no CPU implementation or reference vectors changed
---

### [2026-10-11 03:07 CEST] — CPU-CYCLE-01 batch 02: repair ADD predecrement timing and reconcile instruction classes
- **Affected Subsystems**:
  - `crates/cpu`
  - `CPU Micro-Step State Machine`
  - `CPU Motorola M68000`
- **What Was Changed (The Concrete Reality)**:
  - Separate predecrement address calculation from the operand read in four ADD byte/word source and read-modify-write sequences
  - Validate word alignment during the initial internal CCK through step_check_ea_read_word_idle so previously correct Address Error timing remains intact
  - Remove the register ADD.l comment requested by the user
  - Reconcile the existing Class 1, Class 3 and Class 5 descriptions and tables with the current ADD sequences
  - Correct the main CPU timing list to distinguish four-clock postincrement reads from six-clock predecrement reads
  - Require instruction-class specification updates for every microcode change in the active CPU-CYCLE-01 execution plan
- **Architectural Rationale & Trade-Offs**:
  - Normal predecrement ADD lacked two internal clocks before reading the operand
  - Delaying the original fused read would regress the 322 passing word predecrement fault vectors
  - Keep the specification as the current class model and retain diagnostic history in the batch review packet
  - Other exception entry paths and full CPU agreement remain separate active work
- **Verification & Test Results**:
  - All 6,372 selected normal memory vectors pass state/RAM/clocks/timed callbacks and extra exact final prefetch/formal-PC checks
  - Gain 1,257 passing vectors: 825 byte and 432 word predecrement cases, with no lost passes in either containing file
  - All 322 previously passing predecrement Address Error cases retain their passing result
  - All 10,042 register ADD/NOP controls pass including exact final prefetch/formal-PC checks
  - Focused synthetic ADD/ADDX Cartesian DMA and CPU crate tests pass
  - Full ADD.b remains 7,604/8,065 passing with 461 failures outside this batch, and full ADD.w remains 5,001/8,065 with 3,064 outside failures
  - Formatting/architecture/Clippy and specification links/frontmatter/English checks pass for the recorded batch source
  - Final commit profile command: python tools/verification/verify.py commit, with its result retained in .agent/tmp/CPU-CYCLE-01/batch-02/commit-profile.log and the commit message
  - Synthetic DMA covers CPU plus TestMemoryBus waiting invariance rather than production Agnus arbitration
  - Existing RAM masks and unobserved signal limits remain, and no silicon vectors or expected timing references changed
  - CPU-CYCLE-01 remains active for later batches, with no milestone closure or final full CPU pass claimed
---

### [2026-10-11 13:05 CEST] — CPU-CYCLE-01 batch 03: Address Error entry and predecrement phases
- **Affected Subsystems**:
  - `crates/cpu`
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md`
- **What Was Changed (The Concrete Reality)**:
  - Start Address Error processing in the rejected-access CCK without an operand transfer
  - Latch data-access frame PC from the progressed stream before exception initialization and retain the existing program-space policy
  - Separate 44 fused word/long predecrement reads into internal address calculation and common operand reads
  - Remove the temporary early-alignment helper and align the direct-trigger fixture with the primed stream PC
  - Update the execution flowchart and instruction classes 3, 5 and 12
- **Architectural Rationale & Trade-Offs**:
  - Rejected transfers consume no separate bus phase. Predecrement must finish its internal address phase before alignment validation in the operand read
  - Scoped design review selected both CPU specifications through valid tracked paths. The main CPU specification already describes progressed PC and predecrement timing and remains unchanged
- **Verification & Test Results**:
  - Fresh python tools/verification/verify.py commit passed: ordinary workspace tests with coverage, full repository English audit, formatting, Clippy, architecture, hardware and code quality checks, standard transaction harness and all Python contracts
  - Fresh vault audit passed for 39 specifications and 1050 links with valid frontmatter
  - Rechecked archived batch-03 evidence: all 87 CPU source hashes, 124 reference hashes and 20 evidence hashes match current inputs. Diagnostics were not rerun during this commit workflow
  - Source-matched diagnostics passed all 1772 selected ADD.w operand faults and 6372 normal ADD.b/w vectors. Scoped predecrement faults passed 15053/15053 and normal vectors passed 10112/19520, with no lost prior passes
  - The broad archived Address Error diagnostic passed 145346/178088 and retains 32742 failures in other instruction families. Containing ADD.b/w/l suites still fail outside this batch
  - RAM comparison masks, the final-PC allowance and unobserved electrical signals remain limits. Full SingleStepTests, full Cartesian DMA and milestone verification were not run. CPU-CYCLE-01, CPU-PC-01 and SINGLESTEP-RAM-01 remain active
---

### [2026-10-11 14:57 CEST] — CPU-CYCLE-01: repair selected arithmetic and shift sequences with reviewed reference overrides
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/test_runner`
  - `Obsidian/Amiga/Design`
  - `.agents/rules/docs-maintenance.md`
  - `.agents/skills`
  - `tools/verification/README.md`
- **What Was Changed (The Concrete Reality)**:
  - Repair static sequences for batches 06.01, 06.02, 06.03, 06.07 and 06.10: separate byte predecrement phases, place size-dependent internal delays after prefetch and write long RMW low word before high word
  - Apply two approved ASL.b overrides through the shared SingleStep/DMA loader, checking original SHA-256 identities and reporting corrected executed cases
  - Preserve original references, the complete comparator and all leaf ALU/decoder/engine implementations
  - Consolidate instruction-class descriptions and require shared mechanisms to be described once rather than repeated for each mnemonic
- **Architectural Rationale & Trade-Offs**:
  - SingleStep expectations and approved overrides determine observable CPU behavior. The user explicitly authorized correcting conflicting code-derived descriptions throughout CPU-CYCLE-01 without repeated specification-approval questions
  - The ASL byte records incorrectly modify upper register bytes. Upstream issue 4 and analogous MAME records support the three approved final-field replacements, without establishing identical full-state or physical-capture evidence
  - Missing, ambiguous or changed override targets fail loading. No vector is counted as passed without execution
- **Verification & Test Results**:
  - Selected complete comparisons pass: 06.01 1065/1065, 06.02 40325/40325, 06.03 48390/48390, 06.07 96780/96780 and 06.10 177314/177314 with two overrides. Exact final prefetch and formal PC pass in each selected scope
  - Standard full-input ASL.b passes 8065/8065 with both override names recorded. Earlier full SUB, ADDX, CLR, NEG and NOT filters also pass. Controls retain 10042/10042 passes
  - CPU crate tests, focused Cartesian arithmetic/unary/shift/CMP checks and five override-loader contract tests pass. DMA uses a synthetic TestMemoryBus and sampled bounded schedules, not production bus or chipset arbitration
  - All 124 reference hashes are unchanged. CPU source hashes match the validated batch packet. Vault links/frontmatter and scoped English checks pass, with existing vocabulary findings retained only in the ignored task plan
  - The required per-commit profile is executed separately for this snapshot. No full whole-CPU SingleStep, full Cartesian or milestone closure is claimed. CPU-CYCLE-01, CPU-PC-01 and SINGLESTEP-RAM-01 remain active
---

### [2026-10-11 15:25 CEST] — CPU-CYCLE-01: correct BCD phases and verify batches 06.11 through 06.14
- **Affected Subsystems**:
  - `crates/cpu`
  - `Obsidian/Amiga/Design`
  - `CPU-BENCH-01`
  - `SINGLESTEP-MUTATION-01`
- **What Was Changed (The Concrete Reality)**:
  - Move the two-clock register ABCD/SBCD/NBCD idle after prefetch; retain six-clock totals and existing ALU/flag behavior
  - Separate NBCD byte predecrement calculation from the operand read; use a seven-step sequence with the required two-clock address phase
  - Correct the MULU range comment and existing CPU class/timing descriptions; multiplication and division execution otherwise remain unchanged
  - Record local CPU-BENCH-01 and SINGLESTEP-MUTATION-01 plans for benchmark measurement validity and verifier sensitivity; implementation remains pending
- **Architectural Rationale & Trade-Offs**:
  - Authoritative vectors require prefetch before the BCD register tail and an independent NBCD predecrement phase
  - Scoped design review selected the two CPU specifications; shared phase descriptions remain concise and no reference files or comparison masks changed
  - Benchmark planning identified historical CPU-clock values labeled CCK and estimated work counts; mutation planning targets observable fields plus current PC/prefetch/RAM comparison gaps
- **Verification & Test Results**:
  - Source-matched diagnostics pass 06.11 16246/16246, 06.12 16130/16130, 06.13 16130/16130 and 06.14 24195/24195; exact final prefetch and formal PC pass all 72701 selected vectors
  - BCD improves from 13596 to 24195 complete passes with no lost prior passes; NOP/register ADD controls retain 10042/10042
  - Standard full-input BCD, focused Cartesian BCD/multiply-divide/shift fixtures, CPU crate tests, CPU Clippy, formatting, architecture, English and vault checks passed during implementation
  - All 124 original reference hashes and the validated packet match current inputs; selected scopes execute no overrides
  - DIVU includes one zero-divisor vector; DIVS contains none. DMA uses the existing 5% sample and bounded synthetic contention rather than production arbitration
  - Benchmark and mutation plans passed English/link checks; fresh benchmark measurements and mutation execution have not started. Historical benchmark warnings do not establish a current CPU regression
  - The required commit profile runs separately for this frozen snapshot. Full CPU SingleStep, full Cartesian and milestone closure are not claimed; all related plans remain active
---

### [2026-10-11 15:56 CEST] — CPU-CYCLE-01: repair logic, bit operation and EXG timing
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/test_runner`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Repair batches 06.06, 06.08 and 06.09: separate byte logic predecrement phases, complete prefetch before internal register delays and write long logic RMW low word before high word
  - Use the existing countdown for register bit modifications, preserving original bit numbers for aliases and accounting for the selected register word
  - Move the EXG delay after prefetch and add its missing Cartesian register operation fixture
  - Correct two stale local BSET bit-4 assertions to six clocks, supported by all 40 matching original silicon records
  - At the user request, shorten the documentation to corrected prefetch semantics and the register-bit timing dependency, removing repeated timing tables and instruction lists
- **Architectural Rationale & Trade-Offs**:
  - Silicon vectors constrain bus phases and clock totals independently, so correct final state alone is insufficient
  - Shared execution, leaf ALU formulas, decoders, comparator masks and original reference files remain unchanged
  - Scoped design review selected both CPU specifications and both verification specifications, with valid tracked paths and no missing mappings
  - Keep shared delay and word-order descriptions general, and leave already accurate verification specifications unchanged
- **Verification & Test Results**:
  - Source-matched diagnostics pass 06.06 72585/72585, 06.08 32260/32260 and 06.09 32260/32260, including exact final prefetch and formal PC
  - Gain 31540 with no lost prior passes, and controls retain 10042/10042 passes
  - All 17 selected standard SingleStep suites, focused Cartesian logic/bit/register fixtures and CPU crate tests pass
  - DMA uses the existing 5 percent sample and bounded synthetic contention, not production bus arbitration
  - All 124 original references retain their hashes, and no override executes in the selected scopes
  - Formatting, architecture, Clippy, English and vault checks passed during implementation, with explicit Graphify AST refresh
  - The per-commit profile is run separately for the final snapshot, with its outcome recorded in the commit message and local batch log
  - No whole-CPU SingleStep, exhaustive Cartesian or milestone closure is claimed, and CPU-CYCLE-01 remains active
---

### [2026-10-11 16:32 CEST] — CPU-CYCLE-01: correct comparison timing and verify batches 06.04 and 06.05
- **Affected Subsystems**:
  - `crates/cpu`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Separate the CMP.b predecrement address CCK from its operand read
  - Prefetch before the two-clock register CMP.l and CMPA tail, correcting the excessive CMP.l Dn delay
  - Add the missing two-clock tail after long CMP memory and immediate reads
  - Execute the CMPI.l register comparison with prefetch and move its internal CCK afterward
  - Preserve already correct CMPM and TST microprograms
  - Add the explicitly approved term microprograms to the software vocabulary
  - Update instruction classes with comparison tails and the distinct dual-read CMPM layout
- **Architectural Rationale & Trade-Offs**:
  - Silicon bus phases and clock totals constrain comparison timing independently from flags and final registers
  - Reuse existing shared microsteps without changing the common engine, decoders, addressing helpers or flag formulas
  - Scoped design review selected the parent CPU specification and explicit microstep specification, with valid tracked paths and no missing code mappings
  - The parent CPU specification remains accurate and unchanged
- **Verification & Test Results**:
  - Source-matched diagnostics pass CMPM 2677/2677 and CMP/CMPI/CMPA/TST 61843/61843, including exact final prefetch and formal PC
  - Repair 9702 timing divergences with no lost prior passes, while NOP/register ADD controls retain 10042/10042 passes
  - All eight containing SingleStep suites and three filtered CMPM tests pass, along with CPU crate tests and focused comparison/TST Cartesian tests
  - Cartesian validation retains the existing 5 percent sample and bounded synthetic contention rather than production arbitration
  - All 124 original reference files retain their hashes and no override executes in the selected scopes
  - Formatting, architecture, Clippy, English and vault checks passed, with explicit Graphify AST refresh
  - The initial commit profile passed all tests and other checks but rejected microprograms as unknown vocabulary, which the user subsequently approved
  - The required per-commit profile runs separately on the isolated commit snapshot, with its outcome recorded in commit-profile.log and the commit message
  - Existing RAM masks and unobserved electrical signals remain verification limits, and no full whole-CPU suite or milestone closure is claimed
  - CPU-CYCLE-01 remains active
---

### [2026-10-11 17:04 CEST] — CPU-CYCLE-01: complete data-transfer timing batches 06.15 through 06.20
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/test_runner/tests`
  - `Obsidian/Amiga/Design`
  - `tools/verification/checks/english_words`
- **What Was Changed (The Concrete Reality)**:
  - Check MOVE word/long destination alignment in CCK1 setup
  - retain executing IR while predecrement prefetch advances PC and retire only after the successful store
  - write memory-source absolute-long destinations before the remaining stream fetch
  - complete MOVEM stores in the final write CCK and prefetch directly for empty store masks
  - implement MOVEP word/long transfers through spaced bytes with wait-safe staged addresses and data
  - split LEA/PEA indexed calculation around the extension pair and push absolute PEA operands before prefetch
  - add MOVEP and LEA/PEA fixtures to the existing Cartesian harness
  - correct Class 4/11 descriptions and the predecrement quirks catalog under the recorded SingleStep authority decision
  - preserve already correct MOVE/MOVEA register/immediate and MOVEQ sequences
  - add the explicitly approved busfn vocabulary term
- **Architectural Rationale & Trade-Offs**:
  - Repair demonstrated bus phases and retirement mechanisms without changing silicon references or comparison strength
  - preserve source acquisition before destination addressing and retain state while waiting
  - keep instruction semantics in the parent specification and shared phases in the micro-step specification
  - scoped sync-design-docs review selected the CPU and micro-step specifications plus the quirks catalog and both verification specifications
  - existing verification descriptions remain accurate and unchanged
- **Verification & Test Results**:
  - All 96780 selected vectors pass complete state, clocks and timed callbacks plus independent exact final prefetch/formal PC checks
  - improve from 66502 passes with no lost prior pass and retain 10042 passing NOP/register ADD controls
  - all 124 original reference files are unchanged and no override executes
  - full-input test_move, test_lea, test_pea and adjacent BTST/BCHG/BCLR/BSET tests pass
  - focused synthetic DMA fixtures and CPU crate tests pass
  - formatting, architecture, Clippy, English and affected Verification CLI Python contracts pass
  - vault audit passes for 39 specifications and 1063 links
  - explicit Graphify AST refresh completes
  - python tools/verification/verify.py commit passes for this snapshot, including quality gates, architecture, all ordinary crate and standard harness tests, all Python contracts and the full English audit
  - evidence is retained in .agent/tmp/CPU-CYCLE-01/batch-06-15-20
  - DMA uses a 5 percent seeded sample and at most six contention CCKs against the same uncontended CPU on TestMemoryBus
  - existing RAM masks, unobserved electrical signals and unavailable partial-transfer bus errors remain limits
  - no whole-CPU or milestone closure is claimed
---

### [2026-10-11 18:52 CEST] — SINGLESTEP-PERF-01: finalize aggregate execution and reporting
- **Affected Subsystems**:
  - `test_runner`
  - `verification tooling`
  - `design specifications`
- **What Was Changed (The Concrete Reality)**:
  - Run selected SingleStep groups in parallel inside one aggregate Rust test
  - finalize reports once after all groups
  - preserve failure exits and distinct CMP/CMPM selection identities
  - build optimized verification artifacts with runtime checks
  - add launcher completion and ownership contracts
  - rename launcher tests to test_run_singlestep.py
  - add user-approved API vocabulary
  - close SINGLESTEP-PERF-01 and retire its execution plan after this commit
- **Architectural Rationale & Trade-Offs**:
  - Remove repeated global aggregation while retaining vector comparisons, diagnostics, history and explicit incomplete-run handling. Keep commands selection, generic process execution and SingleStep launch procedure separate. No vector-loading optimization was adopted
  - CPU correctness remains tracked by CPU-CYCLE-01.
- **Verification & Test Results**:
  - Six renamed launcher tests passed. Initial commit verification passed ordinary crate coverage/tests, formatting, Clippy, architecture, hardware/code quality, standard harness and all Python contracts
  - its language gate identified API names subsequently approved by the user. Vault audit passed 39 documents and 1071 links
  - Graphify refresh completed. Full CPU and DMA suites were not rerun during commit preparation. Prior task evidence recorded 128 completed groups, 924967 passing and 77770 failing JSON vectors, with unchanged diagnostic checksums and aggregate runs of 3.273 and 3.184 seconds
  - these measurements are retained task evidence, not a new passing CPU result.
  - Final `python tools/verification/verify.py commit` passed all gates, ordinary crate and standard harness tests, and all Python contracts after the approved vocabulary update. No milestone closure is claimed.
---

### [2026-10-11 19:38 CEST] — CPU-WAIT-01: replace Cartesian contention with ordinary CPU wait tests
- **Affected Subsystems**:
  - `crates/cpu/tests`
  - `crates/test_runner/tests`
  - `tools/verification`
  - `agent instructions and design specifications`
- **What Was Changed (The Concrete Reality)**:
  - Remove the complete Cartesian memory and DMA schedule permutation suite and its exclusive helper state
  - Add a PhysicalMemory-backed recording wrapper and extend the existing CPU suites
  - Exercise all 52 distinct public transfer handlers with zero, one and three refusals and real instruction retry sequences
  - Remove DMA and harness selectors and reconcile milestone selection, policies, skills and specifications
  - Close CPU-WAIT-01 and retire its execution plan after this commit
- **Architectural Rationale & Trade-Offs**:
  - Count actual bus refusals independently and require stable retry identity, retained state, one successful transfer and exact waiting cost
  - Cover CCK phases, long halves, address updates, RMW, prefetch, stack, exceptions, MOVEM and MOVEP without arbitrary schedule permutations
  - Keep production DMA arbitration separate and preserve CPU runtime behavior, silicon references and comparison strength
  - Scoped sync-design-docs review selects the parent CPU specification and six changed design documents with no missing or invalid mappings
  - The parent CPU specification remains accurate and unchanged
- **Verification & Test Results**:
  - CPU crate tests, standard harness and verification Python contracts pass
  - CPU and test_runner Clippy with warnings denied, formatting and architecture pass
  - Vault audit passes 39 specifications and 1073 links and the roadmap check passes
  - MOVEP SingleStep sample passes 100 of 100 vectors across both selected groups with state, clocks and timed callbacks
  - Five cached timing runs measure the selected wait cases at a median 0.065 seconds including test-process startup
  - Changed files pass scoped English checks and Graphify code refresh completes
  - Required final commit-profile verification is recorded in the commit message
  - Full SingleStep and milestone profiles are not run and no roadmap milestone closure is claimed
---

### [2026-10-11 19:50 CET] — CPU-CYCLE-01: repair frame, jump and return batches
- **Affected Subsystems**:
  - `crates/cpu`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Repair batches 06.21, 06.22, 06.27 and 06.29 through 06.31
  - Restore the missing UNLK low-word read completion CCK
  - Read PC high before status in RTR and RTE and retain staged status until its restore phase
  - Capture target minus four and instruction-not for odd JMP/JSR/RTS/RTR/RTE target frames
  - Preserve already correct LINK and JSR return-address calculations
  - Extend existing CPU retry fixtures for both new reads, CCR restoration, banked RTE return and privilege entry
  - Reconcile the existing instruction classes and parent fault-PC description
- **Architectural Rationale & Trade-Offs**:
  - Repair demonstrated silicon phases and frame capture without changing original references, comparator masks or the generic branch target path
  - Scoped sync-design-docs review selects both CPU specifications with no missing or invalid tracked mappings
  - CPU-CYCLE-01 remains active and this commit does not close a milestone
- **Verification & Test Results**:
  - All 56455 selected vectors pass complete comparison plus independent exact final PC and both prefetch words, improving by 35940 cases with no lost prior pass
  - NOP remains 8065 of 8065
  - Eight full-input official SingleStep groups, CPU crate tests, formatting, architecture, Clippy and the full repository English audit pass during implementation
  - All 124 original reference hashes match and zero overrides execute
  - Vault links and frontmatter pass and Graphify AST refresh completes
  - Initial synthetic privilege fixture failed because it lacked opcode-PC initialization, corrected before the final passing CPU crate run
  - Supplied stack addresses are aligned and initial execution is supervisor, so odd-stack and partial bus-error behavior lack selected silicon inputs
  - Privilege and retry controls are synthetic CPU fixtures rather than production arbitration
  - Existing RAM masks and electrical signal limits remain
  - python tools/verification/verify.py commit passes all quality gates, ordinary crate and standard harness tests, Python contracts and the full repository English audit; output remains in the local commit-profile.log
  - Full CPU SingleStep and milestone profiles are not run
---

### [2026-10-11 19:59 CEST] — CPU-CYCLE-01: repair branch frames and Scc predecrement timing
- **Affected Subsystems**:
  - `crates/cpu`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Repair batches 06.23 through 06.26 and 06.28
  - Reuse control-target reads for odd BRA, taken Bcc/DBcc and BSR targets, capturing target minus four and information bit 3
  - Separate Scc predecrement address calculation from its byte read, restoring the missing internal CCK
  - Preserve BSR return-address writes before target faults and leave untaken paths unchanged
  - Reconcile both affected CPU specifications and retain CPU-CYCLE-01 as an active plan
- **Architectural Rationale & Trade-Offs**:
  - Fix mechanisms demonstrated by original silicon inputs without changing references, comparison strength or masks. Reuse shared read phases rather than adding handlers.
  - Scoped sync-design-docs review selects the parent CPU specification and the explicitly changed micro-step specification, with no missing or invalid ownership paths. This commit does not close a roadmap point or milestone.
- **Verification & Test Results**:
  - All 32260 selected vectors pass complete comparison plus independent exact final PC and both prefetch words, improving by 9446 cases without losing a prior pass
  - Five official full-input SingleStep groups pass, including 8065 NOP controls
  - CPU crate tests, formatting, architecture, Clippy and full repository English audit pass
  - All 124 original reference hashes match and zero overrides execute
  - Vault audit passes 39 specifications and 1083 links and Graphify AST refresh completes
  - Supplied DBcc vectors contain no expired counters, so independent expiry timing is not established
  - Initial mode is supervisor and BSR stacks are aligned
  - Existing RAM masks, partial bus-error and electrical signal limits remain
  - Retry controls use synthetic CPU buses rather than production chipset arbitration
  - python tools/verification/verify.py commit passes all quality gates, ordinary crate and standard harness tests, all Python contracts and the full repository English audit. Output remains in the local commit-profile.log
  - Full CPU SingleStep and milestone profiles are not run
---

### [2026-10-11 20:14 CEST] — CPU-CYCLE-01: repair system instruction timing and RESET output ownership
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/machine_loop`
  - `Obsidian/Amiga/Design`
  - `opcode implementation skill`
- **What Was Changed (The Concrete Reality)**:
  - Deliver batches 06.32 through 06.36 plus 06.38 and 06.39
  - Prefetch before CHK and taken TRAPV exception entry and restore CHK bound-dependent internal timing and indexed address cost
  - Complete STOP at four clocks before suspension and preserve the PC after its immediate word
  - Hold RESET output for 124 clocks and release before prefetch while the machine observes its assertion edge without clearing CPU state
  - Preserve already correct SR/CCR/USP and TRAP behavior and add focused privilege and wait controls
  - Reconcile CPU and machine specifications and replace the obsolete RESET idle constant in the opcode procedure
- **Architectural Rationale & Trade-Offs**:
  - Repair mechanisms demonstrated by unchanged silicon vectors and focused failing STOP/RESET tests without weakening comparisons or masks
  - Scoped sync-design-docs review selects both CPU specifications and Machine Loop with no missing or invalid ownership paths
  - CPU-CYCLE-01 remains active and no roadmap point or milestone is closed
- **Verification & Test Results**:
  - All 120975 selected silicon vectors pass complete comparison plus independent exact formal PC and both final prefetch words with 9734 gained passes and no lost prior pass
  - All 16 official full-input groups pass including 8065 NOP controls and all 124 original reference hashes match with zero overrides
  - CPU crate tests and production machine reset integration pass together with formatting architecture Clippy English and vault checks
  - STOP has no supplied silicon file and RESET vectors do not capture output pin duration independently so these timing controls use synthetic CPU fixtures and production machine observation
  - Cold warm and keyboard reset tests currently stop before verified instruction continuation and no explicit test rules out repeated peripheral reset after the pulse
  - Existing RAM masks and unobserved electrical signals remain limits and full CPU SingleStep and milestone profiles are not run
  - python tools/verification/verify.py commit passes all quality gates ordinary crate and standard harness tests all Python contracts and the full repository English audit. Output remains in the local commit-profile.log
---

### [2026-10-11 20:28 CEST] — CPU-CYCLE-01: repair remaining SingleStep timing and expose TAS platform writes
- **Affected Subsystems**:
  - `crates/cpu`
  - `crates/test_runner`
  - `crates/machine_loop`
  - `Obsidian/Amiga/Design`
- **What Was Changed (The Concrete Reality)**:
  - Add missing ADD long memory and ADDA internal tails
  - Move ADDI and ADDQ internal delays after prefetch and separate ADDQ byte predecrement
  - Write low word before high word for ADD/ADDI/ADDQ long memory operands
  - Expand TAS reference intervals into exact byte read and write phases with a comparator regression
  - Add production machine TAS tests for Chip Slow and Fast RAM across 36 constructed cases
  - Reconcile affected micro-step and SingleStep specifications
- **Architectural Rationale & Trade-Offs**:
  - Preserve original silicon references and the two approved ASL byte corrections without new masks or overrides
  - Keep TAS Chip/Slow write suppression as an exposed platform defect rather than changing expected behavior
  - User authorized committing after the two failing platform tests were reported
  - Scoped sync-design-docs review selects five specifications with no missing ownership and leaves already accurate documents untouched
  - CPU-CYCLE-01 remains active and no roadmap point or milestone closes
- **Verification & Test Results**:
  - Full SingleStep passes all 128 selected groups across 124 original files containing 1000060 vectors
  - Independent diagnostic passes 48390 selected vectors with exact formal PC and both final prefetch words
  - Required python tools/verification/verify.py commit fails solely on the two TAS Chip/Slow RAM tests and all other ordinary tests pass
  - Fast RAM TAS and flags across all 36 constructed cases pass while Chip/Slow wrongly set memory bit seven
  - Standard harness tests all Python contracts formatting architecture Clippy hardware and code quality and full English audit pass
  - Vault audit passes 39 specifications and 1094 links and Graphify code refresh completes
  - Original reference hashes remain unchanged and no milestone verification is run
  - Commit profile log is retained in .agent/tmp/CPU-CYCLE-01/remainder/commit-profile.log
