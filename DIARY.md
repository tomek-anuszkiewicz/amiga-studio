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
- The operational recipe, ingestion pipeline, and instructions to prepare this clean-room experiment are detailed in **[BOOTSTRAP.md](BOOTSTRAP.md)**.

This diary stands as the living record of how those foundations were built.

---

## 10. Living Chronological Engineering Log & Evolutionary Change History

This section maintains a continuous, granular chronological record of all engineering changes, subsystem modifications, refactorings, and bug fixes across the repository.

Because Git commits are frequently batched, squashed, or merged into higher-level commits during multi-stage worktree development, standard commit messages often do not preserve the full evolutionary context, granular mechanics, or subtle trade-offs considered along the way. This log serves as the authoritative, human-readable chronicle of what was actually built, modified, and verified in each development session.

### Mandatory Entry Schema:
Every future modification or implementation task must append an entry following this structure:
- **Timestamp & Context**: Date / local time and the active branch / task / PR.
- **Affected Subsystems**: Specific crates, modules, tools, or configuration affected.
- **What Was Changed (The Concrete Reality)**: Detailed technical description of changes, data structures, algorithms, or mechanics implemented.
- **Architectural Rationale & Trade-Offs**: Why this solution was chosen, what alternatives were rejected, and why.
- **Verification & Invariants**: Test suites executed, assertions checked, and proof of correctness.

---

### [2026-09-12 10:15 CEST] — Codification of Mandatory Engineering Diary Maintenance Rule
- **Affected Subsystems**:
  - `AGENTS.md` (Section 1 Operating Rules & Section 4 Definition of Done)
  - `.agents/rules/docs-maintenance.md` (Engineering Diary Maintenance mandate)
  - `.agents/workflows/code-review.md` (Documentation & Diary Review checklist)
  - `.agents/skills/code-review/SKILL.md` (Step 3 & Step 4 Living Docs & Diary audit)
  - `DIARY.md` (Section 8 rules expansion & Section 10 genesis)
- **What Was Changed (The Concrete Reality)**:
  - Formally codified a repository-wide engineering mandate: every code modification, refactoring, bug fix, or milestone implementation must append a narrative entry to `DIARY.md` (Section 10).
  - Integrated this requirement into the automated Definition of Done checklist in `AGENTS.md`, `docs-maintenance.md`, `code-review` workflow, and `code-review` skill.
  - Initialized Section 10 in `DIARY.md` with an explicit entry schema and this inaugural entry.
- **Architectural Rationale & Trade-Offs**:
  - *The "Squashed Commit" Information Loss:* During active emulator development, commits are frequently batched or squashed when merging worktrees (e.g. `feat/benchmarks`, `feat/gui` into `master`). While git commit messages summarize high-level features, the granular problem-solving narrative, rejected prototypes, and subtle edge-case discoveries risk being lost.
  - *Self-Documenting Evolution:* Keeping `DIARY.md` synchronized in lockstep with the code guarantees that the engineering journey remains completely transparent, human-readable, and aligned with the long-term clean-room re-generation experiment ([BOOTSTRAP.md](BOOTSTRAP.md)).
- **Verification & Invariants**:
  - Executed `cargo test -p test_runner --test test_architecture_rules` verifying that `AGENTS.md` (22,348 bytes) and `.agents/rules/docs-maintenance.md` (4,524 bytes) remain strictly under the 23,000-byte prompt-injection safety ceiling (`test_rule_files_size_limit_and_truncation_safety`).
  - Ran `cargo fmt --all -- --check` across the entire workspace.

---

### [2026-09-12 10:25 CEST] — Extraction of Standalone Disassembler Crate & Elimination of 800-Line Architecture Exception
- **Affected Subsystems**:
  - `Cargo.toml`, `Cargo.lock` (added `crates/disassembler` workspace member and dependency)
  - `crates/disassembler/` (new standalone zero-dependency crate with flat layout: `lib.rs`, `types.rs`, `ea.rs`, `alu.rs`, `branch.rs`, `data.rs`, `align.rs`)
  - `crates/debugger/` (removed internal `disassembler.rs`, `disassembler_alu.rs`, `ea_format.rs`; re-exported `disassembler` crate for full backward compatibility)
  - `crates/test_runner/` (updated `tracer.rs` to import `disassembler`; removed `disassembler.rs` from `LINE_COUNT_EXCEPTIONS`; added `disassembler` to `CORE_EMULATION_CRATES`)
  - `AGENTS.md`, `.agents/rules/unit-testing-policy.md`, `Obsidian/Amiga/Design/Debugger.md`, `Obsidian/Amiga/Design/General Architecture.md` (updated crate taxonomy, rules, and design documents)
- **What Was Changed (The Concrete Reality)**:
  - Extracted the M68000 disassembler out of `crates/debugger` into an independent, zero-dependency workspace crate (`crates/disassembler`).
  - Decomposed the disassembler into a strictly flat hierarchy of cohesive modules, each under 380 lines of code:
    - `types.rs` (35 lines): `Disassembly` representation and `format_line()` with fixed-column spacing (`$%08X:  %-24s %s`).
    - `ea.rs` (210 lines): Addressing mode decoding, 16/32-bit immediate formatting, MOVEM register mask formatting, and branch condition code naming.
    - `alu.rs` (380 lines): Arithmetic, logic, comparisons, immediate operations, bit manipulations, shifts/rotates, and multiply/divide formatting.
    - `branch.rs` (75 lines): Control flow formatting (`Bcc`, `DBcc`, `Scc`, `BSR`, `JMP`, `JSR`, `TRAP`, `LINK`, `UNLK`, `STOP`, `RTS`, `RTE`, `RTR`, `RESET`).
    - `data.rs` (165 lines): Data movement (`MOVE`, `MOVEA`, `MOVEQ`, `MOVEM`, `MOVE to/from SR/CCR/USP`, `LEA`, `PEA`, `CLR`, `NEG`, `NOT`, `TST`, `ADDQ`, `SUBQ`, `EXT`, `SWAP`).
    - `align.rs` (145 lines): Heuristic backward stream alignment (`find_aligned_disassembly_start`) enabling smooth scrolling in memory and disassembly GUI views.
    - `lib.rs` (75 lines): Clean crate root facade coordinating instruction classification, extension word reads, and top-level re-exports.
  - Sliced and migrated integration tests to `crates/disassembler/tests/test_disassembler.rs` (10 tests covering arithmetic, logic, data movement, control flow, loops, shifts, stack, and stream alignment).
  - Maintained complete backward compatibility in `crates/debugger` by re-exporting all disassembler types and preserving module aliases (`ea_format`, `disassembler`).
  - Removed `"disassembler.rs"` from `LINE_COUNT_EXCEPTIONS` in `test_architecture_rules.rs` and added `"disassembler"` to `CORE_EMULATION_CRATES` (enforcing zero runtime unwraps/panics).
- **Architectural Rationale & Trade-Offs**:
  - *Decoupling from the Debugger:* The disassembler is an algorithmic text formatter that has zero dependency on debugger state (breakpoints, temporal trace buffers, session controllers) or CPU execution logic. Downstream tools like benchmark tracers or CLI utilities that only need instruction disassembly no longer pull in the heavier debugger crate.
  - *Elimination of Architecture Rule Exception:* Previously, `disassembler.rs` was grandfathered into `LINE_COUNT_EXCEPTIONS` as an exception to the 800-line limit. Refactoring it into domain-specific submodules brought every single file under 400 lines (healthy baseline), permanently removing the exception and improving maintainability.
  - *Zero Runtime Allocations & Safe Error Handling:* Maintained zero runtime unwraps and safe handling of truncated instruction streams (gracefully formatting partial or invalid words as `DATA.W $%04X`).
- **Verification & Invariants**:
  - `cargo test -p disassembler`: 10/10 tests passed.
  - `cargo test -p debugger`: 35/35 tests passed.
  - `cargo test -p gui --test test_interactions`: 25/25 headless integration tests passed.
  - `cargo test -p test_runner --test test_architecture_rules`: 12/12 passed (formatting, file sizes, zero unwraps in `disassembler`, zero custom macros, path privacy, rule file size limits).
  - `cargo fmt --all -- --check`: passed cleanly.

---

### [2026-09-12 10:35 CEST] — 64 KB Memory Bank Precalculation & Dynamic CIA Boot Overlay Swapping
- **Affected Subsystems**:
  - `crates/memory_bus/src/map.rs` (added `BOOT_OVERLAY_HANDLER`, removed `low_memory_overlay` branching from Chip RAM handlers, simplified `write_tas_byte`)
  - `crates/memory_bus/src/lib.rs` (dynamic bank swapping in `map_kickstart_to_low_memory` and `map_chip_ram_to_low_memory`, fused single-lookup bank dispatch in `read_byte`, `read_word`, `write_byte`, `write_word`)
  - `crates/memory_bus/src/arbitration.rs` (eliminated range checks in `is_chip_ram_target`, querying `bank_map[idx].is_contended` directly)
  - `crates/memory_bus/src/test_bus.rs` (added empty classification fast-path in `is_chip_ram_target_internal`)
  - `crates/memory_bus/tests/test_config.rs` (updated `test_256_entry_bank_map` to verify precalculated boot overlay bank mappings)
  - `Obsidian/Amiga/Design/MemoryBus.md` (updated low-memory boot overlay and contention documentation)
- **What Was Changed (The Concrete Reality)**:
  - Transitioned the memory bus from runtime conditional checks (`if low_memory_overlay && addr < 0x080000`) on every memory cycle to a precalculated hardware-exact bank swapping model.
  - Unified Kickstart ROM handling: eliminated `BOOT_OVERLAY_HANDLER` and `read_overlay_rom` completely in favor of `KICKSTART_ROM_HANDLER`. Because Kickstart ROM address decoding masks with `rom_len - 1` (where `0xF80000 & mask == 0`), the exact same handler and byte/word reader serves both the `$F80000..$FFFFFF` base mapping and the `$000000..$07FFFF` low-memory boot overlay.
  - When the OS writes to CIA-A Port A bit 0 (`_OVL = 1`), `map_chip_ram_to_low_memory()` dynamically swaps banks `0x00..=0x07` to `CHIP_RAM_HANDLER` (`is_contended = true`). When overlay is active, banks `0x00..=0x07` directly reference `KICKSTART_ROM_HANDLER` (`is_contended = false`).
  - Completely eliminated all `if bus.low_memory_overlay` checks from `read_chip_ram`, `read_chip_ram_word`, `write_chip_ram`, and `write_chip_ram_word`.
  - Eliminated redundant `if (addr as usize) < bus.chip_ram.len()` bounds checks in `read_chip_ram`, `read_chip_ram_word`, `write_chip_ram`, and `write_chip_ram_word`: since banks 0..7 map strictly to `$000000..$07FFFF` (< 512 KB), any address reaching these handlers is guaranteed within bounds.
  - Streamlined Fast RAM (`read_fast_ram`, `write_fast_ram`), Slow RAM (`read_slow_ram`, `write_slow_ram`), and Kickstart ROM (`read_kickstart_byte`) by eliminating redundant `offset < len` and `idx < rom_len` branches.
  - Fused bank table indexing in `read_byte`, `read_word`, `write_byte`, and `write_word`: the bank descriptor is now retrieved once per cycle, evaluating contention and dispatching the function pointer in a single pass without redundant table lookups.
  - Simplified `write_tas_byte` to use `bank.is_contended` directly instead of complex address range checks.
  - Added an `if classification.is_empty()` fast-path in `TestMemoryBus::is_chip_ram_target_internal`, avoiding double `HashMap` queries during standard single-step tests.
- **Architectural Rationale & Trade-Offs**:
  - *Host CPU Mechanical Sympathy:* Amiga code generates up to 1.77 million memory transfers per second. Eliminating 2-3 dynamic branches and 1 table lookup per transfer saves 5-10 million CPU operations per second on the host, preventing instruction cache eviction and branch misprediction stalls in superscalar host pipelines.
  - *Hardware Accuracy:* This aligns with physical Amiga motherboard circuitry, where Gary PLD lines `A23..A16` select bank decoders directly. The CIA overlay bit simply toggles the address decoding lines rather than invoking dynamic software switches on every cycle.
- **Verification & Invariants**:
  - `cargo test -p memory_bus`: All 22 tests passed (boot overlay, CIA control, contention, RTC, configs).
  - `cargo test -p test_runner --test test_dma_cartesian`: All 19 tests passed (full $2^k \times 2^M$ DMA contention permutations verified).
  - `cargo test -p m68000`: All 42 tests passed.
  - `cargo test -p test_runner --test test_singlestep test_nop` / `test_add_w`: passed.
  - `cargo test -p gui --test test_interactions`: All 25 tests passed.
  - `cargo test -p test_runner --test test_architecture_rules`: All 12 architecture rules passed (file size <= 800 lines, zero macros, zero panics, strict English).
  - `cargo fmt --all -- --check`: Clean formatting.

---

### [2026-09-12 10:45 CEST] — Obsidian Vault RAG Indexer Script & One-Way Privacy Membrane Ingestion
- **Affected Subsystems**:
  - `<PATH_TO_VAULT>/index_to_rag.ps1` (new PowerShell launcher in Obsidian vault for automated Qdrant ingestion)
  - `<PATH_TO_VAULT>/.ragignore` (new exclusion manifest enforcing strict exclusion of `_Private/` and editor dot-folders)
  - `tools/rag/rag_qdrant/indexer.py` (added `.ragignore` parsing, `--exclude` and `--include-dirs` filtering, scoped deleted-file pruning)
  - `tools/rag/rag_qdrant/cli.py` (added `--exclude`, `--include-dirs`, and `--no-root-notes` options to CLI)
  - `tools/rag/README.md` (updated CLI documentation with exclusion and filtering examples)
- **What Was Changed (The Concrete Reality)**:
  - Created `index_to_rag.ps1` in the user's Obsidian vault (`D:\GoogleDrive\AI\Obsidian\Default`).
  - Implemented dual-mode layer discovery:
    - **Universal Mode (Default):** Dynamically discovers all directories starting with two digits (`^[0-9]{2}`), seamlessly ingesting existing layers (`01 Substrate & Mechanical Sympathy`, `02 Harness...`, ..., `05 Operator Psychology...`) and automatically adapting when future layers (`06...`, `07...`) are added.
    - **Strict Mode (`-Strict`):** Explicitly restricts ingestion to the 5 canonical layers defined in the vault's information hierarchy.
  - Enforced the **One-Way Privacy Membrane**:
    - Multi-layered defense guarantees that `_Private/` and any folder/file matching `*private*` (case-insensitive) are strictly excluded from RAG ingestion.
    - Active runtime guard in `index_to_rag.ps1` immediately aborts execution if any private directory is targeted.
    - Added default ignore rules in `indexer.py` for `.obsidian`, `.smart-env`, `.trash`, and case-insensitive `_private` / `private`.
    - Integrated `.ragignore` file parsing directly into `KnowledgeIndexer.index_directory`.
  - Fixed cache cleanup scoping in `indexer.py`:
    - Previously, `delete_file_points` purged all points in `source_cache[source_name]` not present in `active_paths`. When indexing directories in succession under the same source tag, subsequent folders wiped out previously indexed vectors.
    - Scoped deletion strictly to files within the directory being indexed (`old_p.is_relative_to(dir_path)`), preventing inter-directory cache invalidation.
- **Architectural Rationale & Trade-Offs**:
  - *Unified Vault Ingestion:* Indexing the vault root with `--include-dirs` ensures that chunk paths stored in Qdrant retain their knowledge layer prefix (e.g. `01 Substrate & Mechanical Sympathy/...`), enriching vector retrieval context with the structural layer.
  - *Automatic Stale Vector Cleanup:* Reorganizing the vault from older flat categories (`Architecture Guidelines/`, `Future/`) to numbered layers meant 84 stale vectors lingered in Qdrant. Root-scoped pruning automatically removes obsolete vectors from disk while adding the new layer hierarchy in an atomic pass.
- **Verification & Invariants**:
  - `powershell -File tools\rag\bin\amiga_rag.ps1 --help`: verified CLI argument parsing and help banner.
  - `powershell -File <PATH_TO_VAULT>\index_to_rag.ps1 -Status`: verified Qdrant connectivity and status report.
  - Executed full vault indexing: 100 public notes (1,996 chunks) chunked and incrementally ingested with zero private leaks.

---

### [2026-09-12 10:50 CEST] — Removal of Premature KICKSTART_SIZE_512K Constant & 256 KB ROM Scope Alignment
- **Affected Subsystems**:
  - `crates/memory_bus/src/lib.rs` (removed unused `KICKSTART_SIZE_512K` constant; refined `kickstart_rom` doc comment to 256 KB)
  - `crates/memory_bus/src/map.rs` (aligned `read_kickstart_word` and `read_kickstart_byte` doc comments to 256 KB mirroring)
- **What Was Changed (The Concrete Reality)**:
  - Removed `pub const KICKSTART_SIZE_512K: usize = 512 * 1024;` from `crates/memory_bus/src/lib.rs`.
  - Updated `MemoryBus.kickstart_rom` field documentation from `(256 KB or 512 KB)` to `(256 KB)` to accurately reflect current Amiga 500 Kickstart 1.2 / 1.3 physical ROM scope.
  - Refined doc comments on `read_kickstart_word` and `read_kickstart_byte` in `crates/memory_bus/src/map.rs` to explicitly state 256 KB ROM mirroring across Gary's decoded `$F80000..$FFFFFF` space.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of Dead / Premature Constants:* The Amiga 500 baseline uses 256 KB ROMs (Kickstart 1.2 / 1.3). 512 KB ROMs (Kickstart 2.04+, A500+, A600, A3000, A1200) require different address decoding and memory bank configurations. Exposing a 512 KB constant without active configuration or architecture support creates ambiguity.

---

### [2026-09-12 10:52 CEST] — Removal of Premature CHIP_RAM_SIZE_1MB Constant & 512 KB Chip RAM Baseline Alignment
- **Affected Subsystems**:
  - `crates/memory_bus/src/lib.rs` (removed unused `CHIP_RAM_SIZE_1MB` constant; updated `MemoryBank::ChipRam` and `MemoryBus.chip_ram` doc comments to 512 KB baseline)
  - `crates/memory_bus/src/map.rs` (aligned `read_chip_ram`, `read_chip_ram_word`, `write_chip_ram`, `write_chip_ram_word` doc comments to standard 512 KB `$000000-$07FFFF` range)
- **What Was Changed (The Concrete Reality)**:
  - Removed `pub const CHIP_RAM_SIZE_1MB: usize = 1024 * 1024;` from `crates/memory_bus/src/lib.rs`.
  - Updated `MemoryBank::ChipRam` enum documentation from `(Base $000000-$07FFFF, optionally extended to $000000-$0FFFFF)` to `($000000-$07FFFF)`.
  - Updated `MemoryBus.chip_ram` field doc comment to `Physical Chip RAM buffer (512 KB)`.
  - Refined Chip RAM read/write doc comments in `crates/memory_bus/src/map.rs` to remove speculative `optionally extended` notes.
- **Architectural Rationale & Trade-Offs**:
  - *Baseline Alignment with A500 OCS Hardware:* The standard Amiga 500 OCS baseline has 512 KB of onboard Chip RAM (`ChipRamSize::Kb512`), with additional RAM at `$C00000` mapped as Slow/Trapdoor pseudo-fast RAM rather than true Chip RAM. 1 MB Chip RAM requires ECS Agnus (8372A) or motherboard jumper modifications not active in current presets. Retaining unused 1 MB Chip RAM constants created dead code and false expectations.
- **Verification & Invariants**:
  - `cargo test -p memory_bus`: All 22 tests passed.
  - `cargo test -p test_runner --test test_architecture_rules`: All 12 architectural checks passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 11:35 CEST] — Vault-Wide Dual-Layer Linking Standard & Automated Architecture Integrity CI
- **Affected Subsystems**:
  - `.agents/rules/vault-linking-and-graph-integrity.md` (new mandatory rule codifying Dual-Layer Linking Standard, Inverted Pyramid hierarchy, and zero broken links policy)
  - `.agents/rules/docs-maintenance.md` (updated with the Obsidian Vault Linking & Graph Integrity Contract)
  - `AGENTS.md` (updated Section 1 and Section 4 with rule registry and Definition of Done; tightened phrasing to 22,505 bytes, strictly below the 23,000-byte ceiling)
  - `Obsidian/Amiga/Design/*.md` (all 28 architecture specifications comprehensively upgraded with Dual-Layer links; 548 total links verified with 0 broken links)
  - `crates/test_runner/tests/test_architecture_rules.rs` (implemented `test_obsidian_design_docs_links_integrity()`, validating all 28 design docs and >=300 links with zero broken paths)
- **What Was Changed (The Concrete Reality)**:
  - Formulated and adopted the **Dual-Layer Linking Standard** and **Inverted Pyramid Model** adapted from external knowledge systems while strictly excluding non-applicable rules (`one-way-privacy-membrane.md` and `language-agnostic-architecture.md`):
    - **Layer 1 (Contextual Cross-Links):** Embedded in metadata headers and in-text prose linking related design documents and parent specifications.
    - **Layer 2 (Structural Ground Truth):** A standardized bottom reference section (`## Reference Documentation & Upstream Ground Truth`) in every design doc linking directly to:
      1. Upstream official hardware documentation under `Obsidian/Amiga/Reference/` (Motorola 68000 User's Manual, Amiga Hardware Reference Manual, Amiga Guru Book).
      2. Verified reference emulator implementations under `ref_src/` (MAME, Moira, Musashi, vAmiga, WinUAE).
      3. Living Rust implementation source files under `crates/*/src/` and automated test suites.
  - Systematically audited and refactored all 28 design documents across 4 logical groups:
    - *Group 1 (Custom Chipset & Peripherals):* `Agnus.md`, `Denise.md`, `Paula.md`, `CIA.md`, `Floppy.md`, `Keyboard.md`, `Mouse.md`, `Joystick.md`, `RTC.md`.
    - *Group 2 (System Architecture, Bus & Coordination):* `General Architecture.md`, `MemoryBus.md`, `CycleCounter.md`, `Main loop A500.md`, `Configuration.md`, `SaveState.md`.
    - *Group 3 (CPU Micro-Architecture & Verification):* `CPU Motorola M68000.md`, `CPU Micro-Step State Machine.md`, `CPU SingleStepTests.md`, `CPU Instruction Benchmarking.md`, `CPU Instruction Benchmark Catalog.md`, `CPU Instruction Benchmark Strategies.md`, `CPU Benchmark Analysis Guide.md`.
    - *Group 4 (Developer Tools, GUI & Workflows):* `Debugger.md`, `GUI.md`, `GUI Specification.md`, `Git Worktree Workflow.md`, `Rust Guidelines.md`, `egui Guidelines.md`.
  - Solved the markdown URL parenthesis truncation issue by percent-encoding parentheses in reference document paths (`%28Address%20Order%29.md`), preventing markdown parsers and regexes from truncating targets at the first closing parenthesis.
  - Fixed legacy references across specs (e.g. `loader.rs` in `crates/gui` updated to `crates/debugger/src/loader.rs`, `table.rs` updated to `dispatch_table.rs`, `alu.rs` updated to `crates/m68000/src/instructions/`).
  - Added automated architectural test `test_obsidian_design_docs_links_integrity()` to `test_architecture_rules.rs`, which parses all markdown links across `Obsidian/Amiga/Design/*.md`, performs URL decoding, and asserts zero broken links on every `cargo test`.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of the Bottom-Heavy Accumulation Trap:* Design documents frequently suffered from historical drift where superseded proposals and broken paths accumulated unchecked. The Dual-Layer standard guarantees that every document forms an explicit bridge between high-level architectural rationale, upstream hardware ground truth, and living Rust code.
  - *Automated CI Enforcement:* Manual link audits inevitably decay as code is refactored. Integrating markdown link validation into `test_architecture_rules.rs` turns documentation integrity into a hard build invariant.
- **Verification & Invariants**:
  - Python full-vault scan: 28 documents, 548 total links, **0 broken links**.
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.62s.
  - `cargo test -p m68000 -p memory_bus -p config -p rtc -p disassembler -p debugger -p gui`: All unit and integration tests green.
  - `cargo fmt --all -- --check`: Formatting 100% compliant.

---

### [2026-09-12 11:46 CEST] — Roadmap Milestone: Repository Sanitization & Public Release Preparation
- **Affected Subsystems**:
  - `ROADMAP.md` (added Section 5: "Repository Sanitization & Public Release Preparation")
- **What Was Changed (The Concrete Reality)**:
  - Added milestone Section 5 outlining the pre-release strategy for open-source publication:
    - *Section 5.1 (Historical Commit Message Normalization):* Systematic audit and rewriting of historical commit messages across the entire Git history to replace casual or auto-generated messages (e.g. IDE "Generate" commits) with standardized, descriptive messages aligned with `DIARY.md`.
    - *Section 5.2 (Deep Git History Scrubbing & Asset Purge):* Comprehensive Git history rewrite preserving all commit dates, author/committer timestamps, and branch/merge topologies, while completely purging large binaries, copyrighted reference materials, and proprietary blobs:
      - Purging reference emulator sources (`ref_src/` — vAmiga, WinUAE, MAME, Musashi, Moira, single-step tests).
      - Purging third-party books, hardware reference manuals, and copyrighted documentation PDFs (`Obsidian/Amiga/Reference/`).
      - Purging hardware schematics and board scan archives (`schematics/`).
      - Purging third-party tools and standalone executables (`AmigaTestKit`, `WinGuide.exe`).
      - Purging generated knowledge graph outputs and vector caches (`graphify-out/`).
      - Conducting repository footprint and licensing audit prior to remote publication.
- **Architectural Rationale & Trade-Offs**:
  - *Clean Open-Source Release:* Reference emulators, scans, third-party diagnostic executables, and proprietary books are vital during development for cycle-exact differential validation, but must not be distributed in the final public repository.
  - *History Metadata Invariance:* Scrubbing file contents from commits while preserving timestamps and topology ensures the evolutionary narrative of the emulator's development remains accurate without retaining multi-gigabyte or copyright-encumbered binary blobs.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 11:47 CEST] — Autonomous Clean-Room Pipeline: Documentation Skill Calibration Loop in BOOTSTRAP.md
- **Affected Subsystems**:
  - `BOOTSTRAP.md` (updated Section 2: "Prompt & Skill System Refinement")
- **What Was Changed (The Concrete Reality)**:
  - Codified the iterative documentation conversion skill refinement loop into Section 2 of `BOOTSTRAP.md`:
    - Designated primary hardware PDFs (Commodore *Amiga Hardware Reference Manual* and Motorola *M68000 User's Manual / Programmer's Reference Manual*) as the ground-truth benchmark suite for the `.agents/skills/pdf-to-markdown/` skill.
    - Defined an iterative execution and calibration cycle: repeatedly running the skill against these PDFs and tuning prompt instructions, table split merging, figure extraction, and layout heuristics until agent-generated documentation achieves structural and technical parity with our curated reference documentation (`Obsidian/Amiga/Reference/`).
- **Architectural Rationale & Trade-Offs**:
  - *Clean-Room Re-generation Pre-requisite:* For an autonomous agent to re-create the emulator without human intervention or copyrighted repository bloating, it requires a battle-tested skill that reliably transforms raw technical PDFs into clean, high-fidelity Markdown notes indistinguishable from human-curated specifications.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 11:48 CEST] — Clean-Room Pipeline: Closed-Loop Design Documentation vs. Generated Code Parity
- **Affected Subsystems**:
  - `BOOTSTRAP.md` (updated Section 4: "Reference Code Clean-Room Wipe & Regeneration Experiment")
- **What Was Changed (The Concrete Reality)**:
  - Formulated and added the **Iterative Closed-Loop Calibration (Design Documentation vs. Generated Code Parity)** methodology to Section 4 of `BOOTSTRAP.md`:
    - Discarded the naive assumption of a one-shot "wipe-and-pray" code generation attempt.
    - Established an iterative feedback loop where an agent generates modules and subsystems strictly from curated design documentation (`Obsidian/Amiga/Design/`).
    - Benchmarked generated code against proven test suites (SingleStepTests, DMA contention, architecture rules) and numerical/cycle baselines (`m68k_benchmark_baseline.csv`, `golden_row_hashes.rs`).
    - Defined continuous documentation refinement: sharpening state invariants, cycle phase models, and edge cases until the design documentation reliably yields code 100% equivalent to our proven implementation.
- **Architectural Rationale & Trade-Offs**:
  - *Disciplined Scientific Approach:* Autonomous software regeneration cannot rely on stochastic luck. By treating documentation as a formal specification compiler target and validating generated code against cryptographic golden baselines, we systematically eliminate specification ambiguities before conducting the final code wipe.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 11:49 CEST] — Clean-Room Pipeline: BLEP Synthesis & Analog Audio Filter Generator Experiment
- **Affected Subsystems**:
  - `BOOTSTRAP.md` (updated Section 1 and Section 4: added "Pilot Subsystem Testbed: BLEP Synthesis & Analog Audio Filter Generator Experiment")
- **What Was Changed (The Concrete Reality)**:
  - Added a concrete subsystem pilot testbed to `BOOTSTRAP.md` Section 4 for validating the documentation-to-code generation loop:
    - Target: Physical Amiga 500 audio output circuitry (op-amp amplifier stages, passive RC and active Sallen-Key low-pass filter networks, dynamic CIA-A LED filter switching, and raw component values for resistors and capacitors).
    - Protocol: Supply mathematical requirements for band-limited step (BLEP) anti-aliasing audio synthesis without providing precomputed lookup tables or reference DSP source code.
    - Calibration Loop: Instruct the agent to independently author the BLEP synthesis script, derive filter transfer functions, and generate the complete Paula audio DSP/filter pipeline.
    - Iteratively refine the design documentation until the synthesized BLEP tables and audio filter implementation match our verified ground truth (`blep_tables.rs` and Paula audio engine) bit-for-bit.
- **Architectural Rationale & Trade-Offs**:
  - *Hard Subsystem Testbed:* The Paula audio engine combines continuous-time analog circuit modeling (op-amps, Sallen-Key filter equations) with discrete digital DSP (BLEP residual convolution). It serves as an ideal stress test for proving whether design documentation alone is sufficient for an autonomous agent to reproduce mathematically complex emulator subsystems.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 11:51 CEST] — Roadmap Refinement: Commit History Squashing & Privacy Hygiene Audit
- **Affected Subsystems**:
  - `ROADMAP.md` (updated Section 5: "Repository Sanitization & Public Release Preparation")
- **What Was Changed (The Concrete Reality)**:
  - Enhanced Section 5 of `ROADMAP.md` with two core release hygiene mandates:
    - *Historical Content Hygiene & Privacy Audit:* Added an explicit mandate in Section 5.2 to thoroughly audit past Git commit snapshots and file trees for accidental disclosures: private host paths, credentials, unintended temporary debug dumps, scratch experiments, or sensitive personal data.
    - *Commit History Reordering & Selective Squashing:* Added a mandate in Section 5.1 to analyze the commit graph for fragmented, disjointed, or trial-and-error commits (e.g. micro-fixups, typo adjustments, multi-commit implementation fragments) and reorder/squash them into clean, atomic milestones.
- **Architectural Rationale & Trade-Offs**:
  - *Professional Open-Source Presentation:* An open-source release should present a clean, coherent architectural narrative rather than exposing messy intermediate trial-and-error commits or accidental debug dumps. Selective squashing consolidates related changes while preserving milestone-level historical fidelity and author timestamps.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:03 CEST] — Automated Operating System: Rules, Skills & Definition of Done Overhaul
- **Affected Subsystems**:
  - `.agents/rules/docs-maintenance.md` (added major milestone gates for diary compaction and dead code pruning, non-redundant constitution rule)
  - `.agents/rules/amiga-rag.md` (automated incremental reindexing on `Obsidian/Amiga/Reference/` changes, path privacy fix)
  - `.agents/rules/graphify.md` (scoped incremental reindexing for `crates/` and `ref_src/`, prohibited full-workspace crawls)
  - `.agents/rules/vault-linking-and-graph-integrity.md` (mandatory Obsidian Properties YAML frontmatter at line 1 evaluated on every edit, renumbered sections)
  - `AGENTS.md` (updated Section 1 rule index with `amiga-rag` and scoped graphify, updated Section 4 Definition of Done, strictly verified size <= 23 KB)
  - `.agents/skills/compact-diary/SKILL.md` (new skill: milestone-driven diary compaction and synthesis)
  - `.agents/skills/prune-dead-code/SKILL.md` (new skill: systematic dead code and scaffolding elimination)
  - `.agents/skills/obsidian-vault-linking/SKILL.md` (new skill: operationalizing Obsidian Properties and Dual-Layer Linking Standard)
- **What Was Changed (The Concrete Reality)**:
  - Codified the complete automated matrix of rules, triggers, and skills:
    1. *Major Milestone Gate:* On completing major roadmap steps, trigger `compact-diary` to synthesize older historical entries into milestone digests while keeping recent work granular, and trigger `prune-dead-code` to purge unreferenced symbols.
    2. *Incremental RAG Trigger:* Changes in `Obsidian/Amiga/Reference/` automatically trigger incremental vector reindexing in Qdrant via `amiga_rag`.
    3. *Scoped Graphify Subtree Isolation:* Mandated that graphify updates during development must strictly be scoped to `crates/` (`graphify update crates/`) or `ref_src/` (`graphify update ref_src/`), eliminating unconstrained full-workspace scans.
    4. *Mandatory Obsidian Properties:* Standardized YAML frontmatter (`title`, `aliases`, `tags`, `category`, `subsystem`, `status`, `created`, `updated`, `related`) at line 1 for all notes in `Obsidian/Amiga/Design/`, with continuous evaluation on every document edit.
    5. *Constitutional Non-Redundancy:* Explicitly codified that `AGENTS.md` must not duplicate detailed rules or full DoD workflows already modularized under `.agents/rules/`, guaranteeing `AGENTS.md` remains a concise index strictly under the 23,000-byte truncation safety ceiling.
- **Architectural Rationale & Trade-Offs**:
  - *Autonomous Predictability:* High-reliability agentic pair programming requires deterministic rules with explicit triggers rather than vague suggestions. By decoupling high-level constitutional constraints (`AGENTS.md`) from modular operational rules (`.agents/rules/`) and procedural runbooks (`.agents/skills/`), the agent operates within optimal context window token bounds while consistently triggering the right automation.
- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.53s.
  - All rule files and `AGENTS.md` validated strictly <= 23,000 bytes.
  - Zero external hardcoded paths verified across all rules and codebase.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:12 CEST] — Skills Expansion: Diary Compaction Philosophy & RAG Ingestion Skill
- **Affected Subsystems**:
  - `.agents/skills/compact-diary/SKILL.md` (codified core guiding philosophy: high-signal architectural & collaboration lessons over low-level code details)
  - `.agents/skills/index-amiga-rag/SKILL.md` (new skill: incremental Qdrant vector database ingestion triggered by `Obsidian/Amiga/Reference/` modifications)
  - `.agents/skills/graphify/SKILL.md` (documented scoped subtree re-indexing policy for `crates/` and `ref_src/`)
  - `.agents/skills/prune-dead-code/SKILL.md` (refined goal statement, removing unnecessary L1 cache mention in favor of cognitive clutter elimination)
- **What Was Changed (The Concrete Reality)**:
  - Formulated the exact philosophical contract for diary compaction:
    - *Omission of Low-Level Noise:* Eliminate line-by-line diffs, variable renames, and mechanical changes already recorded in Git.
    - *Architectural Dilemmas & Breakthroughs:* Preserve structural design trade-offs, circuit race conditions, and cycle-exact timing models.
    - *Agent Collaboration Records:* Document hard problems solved with the agent, traps encountered, and institutional safeguards devised.
    - *Future Insights:* Record non-obvious hardware quirks and lessons that cannot be deduced from source code alone.
  - Implemented the `index-amiga-rag` skill operationalizing automated incremental reindexing of `Obsidian/Amiga/Reference/` via `amiga_rag.ps1` / `indexer.py`.
  - Added scoped subtree reindexing documentation to `graphify` skill.
  - Removed artificial L1 cache references from `prune-dead-code/SKILL.md`.
- **Architectural Rationale & Trade-Offs**:
  - *Signal-to-Noise Maximization:* The engineering diary serves as institutional memory. By capturing high-signal decisions and agent-collaboration patterns while discarding low-level diff clutter, the diary remains a permanently readable and actionable knowledge base across long-running project phases.

- **Verification & Invariants**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:24 CEST] — Eradication of Artificial L1/L1i Cache Semantic Attractor
- **Affected Subsystems**:
  - `AGENTS.md` (lines 16, 72-74, 100: purged rhetorical "L1i cache density" phrases in favor of lean, compact, and contiguous execution paths)
  - `.agents/rules/performance-and-readability.md` (eliminated pseudo-intellectual L1i/L1d intro; focused on branch predictability, flat execution, and compact instruction paths)
  - `.agents/rules/method-inlining.md` (removed 32 KB L1i thrashing claims; grounded in compiler cross-crate MIR inlining and code bloat prevention)
  - `.agents/workflows/code-review.md` (pruned L1i checklist attractor)
  - `.agents/skills/add-m68k-instruction/SKILL.md` (pruned L1i cache phrasing)
  - `.agents/skills/compact-diary/SKILL.md` (replaced L1 cache dynamics example with branch predictor dynamics)
  - `Obsidian/Amiga/Design/Rust Guidelines.md` (lines 73, 101: replaced L1i cache references with compact and out-of-line execution paths)
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md` (line 881: replaced "host L1d cache residency" with "compact contiguous runtime state layout")
  - `ROADMAP.md` (line 64: simplified state footprint audit to focus on memory layout, alignment, and cache-line compactness)
- **What Was Changed (The Concrete Reality)**:
  - Systematically audited and eradicated the artificial "L1 cache attractor"—a repetitive, hallucinatory buzzword pattern that previous LLM iterations injected as an all-purpose justification across unrelated rules (e.g. dead code pruning, method inlining, code review checklists).
  - Clarified and preserved legitimate hardware profiling specifications in `ROADMAP.md` (Step 2 PMU counter benchmarks: `L1-icache-load-misses`, `L1-dcache-load-misses`, LLC misses) and `Obsidian/Amiga/Design/CPU Instruction Benchmarking.md` (testing 700-instruction unrolled loops against CPU Loop Stream Detectors and single P-core pinned benchmarking).
- **Architectural Rationale & Trade-Offs**:
  - *Preventing Model Semantic Drifting:* LLMs develop strong attractor basins around specific technical jargon. Once "L1i cache density" was cited in early benchmarking discussions, subsequent model turns parroted the term into completely inappropriate contexts (e.g. justifying deleting unused Rust structs to "keep L1 instruction cache compact"). Purging this pseudo-justification restores clear, precise, and hardware-accurate engineering language across repository guidelines.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.53s.
  - `AGENTS.md` and all rule files verified strictly <= 23,000 bytes (`AGENTS.md` is 22,908 bytes).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:37 CEST] — Audit & Elimination of Additional Semantic Attractors (Invariants, Oracles, Friction, Epistemic)
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design/CPU Benchmark Analysis Guide.md` (line 48: eradicated the tautological attractor `"The Invariance Invariant"` $\rightarrow$ `"Addressing Mode Consistency"`)
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md` (line 160: purged `"zero cognitive friction"` in favor of clean consistency)
  - `DIARY.md` (line 19: replaced theatrical `"testing oracle"` with concrete `"test verification reference"`)
  - `.agents/skills/index-amiga-rag/SKILL.md` (line 48: replaced `"Incremental Skip Invariant"` with `"Content-Hash Skip Mechanism"`)
  - `.agents/skills/prune-dead-code/SKILL.md` (line 47: replaced `"Invariant Verification"` with `"Correctness Verification"`)
  - `.agents/skills/compact-diary/SKILL.md` (lines 4, 57: replaced `"verified invariants"` / `"Verified Invariants"` with `"verified test results"` / `"Verification Gates"`)
  - `.agents/rules/docs-maintenance.md` (lines 12, 24: replaced `"Verification & Invariants"` boilerplate with `"Verification & Test Results"`)
  - `.agents/rules/unit-testing-policy.md` (line 37: replaced `"Layout & Invariant Assertions"` with `"Layout & State Assertions"`)
  - `.agents/rules/egui-best-practices.md` (lines 41, 44, 45: pruned inflated invariant references in favor of layout state and geometry stability)
  - `AGENTS.md` (line 153: replaced `"invariant tests"` with `"unit, and integration tests"`)
- **What Was Audited & Clarified (The Concrete Reality)**:
  - Conducted a comprehensive audit of 4 prevalent AI semantic attractor categories:
    1. *Invariants as a universal answer to everything:* Stripped ornamental usage of "invariant" from section headers, file-caching skips, and UI tests, strictly confining "invariance" to genuine mathematical transformations (DMA formula $C = C_0 + 2 \times \text{wait\_states}$, address error register immutability, FNV-1a hash column determinism).
    2. *Harness + Oracles:* Replaced theatrical "oracle" terminology in documentation with direct, literal testing references. Confirmed "harness" is used strictly in concrete technical contexts (Cargo `harness = false`, `dma_harness.rs`, hardware wire harness).
    3. *Zero friction warnings / claims:* Eradicated "zero cognitive friction" phrasing; verified zero instances of "zero friction trap".
    4. *Epistemic philosophical jargon:* Verified zero occurrences of "epistemic" across the entire repository.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.54s.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:44 CEST] — Obsidian Properties (YAML Frontmatter) Rollout Across All 28 Design Documents
- **Affected Subsystems**:
  - All 28 notes in `Obsidian/Amiga/Design/*.md` (`Agnus.md`, `CIA.md`, `CPU Benchmark Analysis Guide.md`, `CPU Instruction Benchmark Catalog.md`, `CPU Instruction Benchmark Strategies.md`, `CPU Instruction Benchmarking.md`, `CPU Micro-Step State Machine.md`, `CPU Motorola M68000.md`, `CPU SingleStepTests.md`, `Configuration.md`, `CycleCounter.md`, `Debugger.md`, `Denise.md`, `Floppy.md`, `GUI Specification.md`, `GUI.md`, `General Architecture.md`, `Git Worktree Workflow.md`, `Joystick.md`, `Keyboard.md`, `Main loop A500.md`, `MemoryBus.md`, `Mouse.md`, `Paula.md`, `RTC.md`, `Rust Guidelines.md`, `SaveState.md`, `egui Guidelines.md`).
- **What Was Changed (The Concrete Reality)**:
  - Systematically evaluated and generated active **Obsidian Properties** blocks (YAML frontmatter bounded by `---` lines at Line 1) across all 28 design specifications in `Obsidian/Amiga/Design/`.
  - Implemented the full schema per `.agents/rules/vault-linking-and-graph-integrity.md`:
    - `title`: Canonical full document or subsystem name.
    - `aliases`: Recognized chip codes (e.g. `MOS 8370`, `MOS 8520`), abbreviations, and alternate titles.
    - `tags`: Subsystem classification tags (`["amiga", "design", "<subsystem>"]`).
    - `category`: Strictly `"Design"`.
    - `subsystem`: Concrete architectural subsystem mapping (`agnus`, `denise`, `paula`, `m68000`, `memory_bus`, `cia`, `gui`, `debugger`, `config`, `cycle_counter`, `rtc`, `general`).
    - `status`: Set to `"active"`.
    - `created`: Accurate historical creation dates derived from Git commit history (August–September 2026).
    - `updated`: Set to current evaluation date `2026-09-12`.
    - `related`: High-signal relative markdown links to sibling design specifications.
- **Architectural Rationale & Trade-Offs**:
  - *Obsidian Knowledge Graph Integrity:* YAML frontmatter at line 1 allows Obsidian, graph view, search filters, and external indexing tools to index metadata, aliases, and reciprocal relations without polluting prose.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed, including `test_obsidian_design_docs_links_integrity` verifying 0 broken links across all newly linked `related` properties.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:47 CEST] — Pre-Task Conceptual Retrieval Rule (`source = "obsidian"`)
- **Affected Subsystems**:
  - `.agents/rules/amiga-rag.md` (added dedicated section: `Mandatory Pre-Task Conceptual Retrieval (source = "obsidian")`)
  - `AGENTS.md` (line 19: updated summary to reflect pre-task conceptual retrieval mandate)
- **What Was Changed (The Concrete Reality)**:
  - Formulated and enforced a mandatory operating rule for task inception and planning:
    - Whenever starting a new feature, refactoring, architectural plan, or non-trivial task (before writing code):
      - Query the local RAG knowledge base targeting the user's architectural knowledge vault:
        `rag_search(query="<task-topic-or-architecture-concept>", sources=["obsidian"])` (and `sources=["amiga"]` when hardware specifics are required).
      - Evaluate retrieved context snippets for relevant architectural principles, operator heuristics, systems design guidance, or ergonomics.
      - Weave applicable insights directly into the reasoning, implementation plan (`implementation_plan.md`), or design approach.
- **Architectural Rationale & Trade-Offs**:
  - *Contextual Alignment & Mental Model Synchronization:* The user's Obsidian knowledge vault contains distilled architectural wisdom, ergonomics preferences, and design principles. By mandating a semantic search against `source = "obsidian"` at the inception of each task, the agent automatically aligns with the user's established engineering philosophy before proposing plans or authoring code.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.55s.
  - All rule files and `AGENTS.md` (22,921 B) strictly $\le 23,000$ bytes.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 12:52 CEST] — AGENTS.md Deduplication & Constitutional Size Ceiling Guardrail
- **Affected Subsystems**:
  - `AGENTS.md`: Pruned redundant sections (Sections 3.5–3.10 and verbose Definition of Done prose) duplicating `.agents/rules/*.md`.
  - `crates/test_runner/tests/test_architecture_rules.rs`: Added dedicated `MAX_AGENTS_MD_BYTES = 14_000` ceiling to `test_rule_files_size_limit_and_truncation_safety`.
- **What Was Changed (The Concrete Reality)**:
  - Eliminated extensive duplicated code blocks and guidelines in `AGENTS.md` that were already modularized across:
    - `.agents/rules/performance-and-readability.md` (mechanical sympathy, flat execution, zero macros, zero const generics, dual staging registers).
    - `.agents/rules/file-size-and-cohesion.md` (file size $\le 800$ lines, 1:1 opcode files).
    - `.agents/rules/method-inlining.md` (inlining strategy).
    - `.agents/rules/workspace-structure-and-reexports.md` (flat crates, 3-tier re-exports).
    - `.agents/rules/opcode-naming.md` (canonical `IDLE` micro-steps).
    - `.agents/rules/docs-maintenance.md` & `.agents/rules/spec-compliance.md` (Definition of Done verbose prose).
  - Preserved `AGENTS.md` as the high-level architectural constitution and indexing hub:
    - Section 1: Complete, 1-line index of all 20 `.agents/rules/*.md` files.
    - Section 2: Core machine principles (portability, CCK1/CCK2 clock model, decoupled ownership, circuit simulation).
    - Section 3: Machine-level systems invariants (endianness, zero panics on guest code, wrapping math, zero heap allocation in hot loop).
    - Section 4: Lean Definition of Done checklist pointing to rules and workflows.
    - Section 5: Knowledge base and reference navigation.
  - Dropped `AGENTS.md` file size from 22,921 bytes down to 12,578 bytes (~45% token footprint reduction).
  - Tightened automated architecture test `test_rule_files_size_limit_and_truncation_safety` with `MAX_AGENTS_MD_BYTES = 14_000`, ensuring any accidental copy-pasting of rule chapters fails CI immediately.
- **Architectural Rationale & Trade-Offs**:
  - *Context Budget Optimization & Truncation Safety:* `AGENTS.md` is injected into every agent prompt. Redundant copies of modularized rules waste context window budget and pushed the file dangerously close to the 23,000-byte silent prompt truncation cliff. Constitutional indexing keeps prompts lean, focused, and well within limits.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.60s.
  - `AGENTS.md` size confirmed on disk: 12,578 bytes (within $\le 14,000$ byte threshold).
  - `cargo fmt --all -- --check`: 100% compliant.

---

### [2026-09-12 12:55 CEST] — Git Commits & Atomic History Protocol Rule
- **Affected Subsystems**:
  - `.agents/rules/git-commits.md`: Created dedicated operating rule for commit decomposition, Conventional Commits, context reconstruction, and pre-commit verification gates.
  - `AGENTS.md`: Indexed `git-commits.md` in Section 1.
- **What Was Changed (The Concrete Reality)**:
  - Codified the full protocol governing standard git commit operations (`zrób commita`, `commit`):
    - **Context Reconstruction:** Running `git status` / `git diff --stat` and cross-referencing recent entries in `DIARY.md` (Section 10) and session transcripts to reconstruct multi-phase work.
    - **Atomic Decomposition:** Mandating that accumulated changes across distinct domains (docs, rules/skills, subsystem logic, architecture tests) be separated into dedicated atomic commits using targeted `git add` rather than blind bulk commits (`git add -A`).
    - **Cohesive Unit Exception:** Preserving code + unit test + design doc in a single commit when they belong to the exact same feature.
    - **Conventional Commits:** Standardizing `<type>(<scope>): <summary>` format in strict English with structured bullet points.
    - **Pre-Commit Quality Gate:** Mandating `cargo fmt --all -- --check` and architecture test suite runs before committing.
- **Architectural Rationale & Trade-Offs**:
  - *Clean, Auditable Git History:* Large multi-task development sessions frequently accumulate diverse modifications across documentation, rules, tests, and code. Decomposing these into atomic commits with context informed by `DIARY.md` ensures the Git log remains bisectable, clean, and meaningful.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.53s.
  - `AGENTS.md` size: 12,767 bytes (strictly $\le 14,000$).
  - `.agents/rules/git-commits.md` size: 4,041 bytes (strictly $\le 23,000$).
  - `cargo fmt --all -- --check`: 100% compliant.

---

### [2026-09-12 13:10 CEST] — Rules Decomposition into Single-Responsibility Units & Trigger Architecture
- **Affected Subsystems**:
  - `.agents/rules/agents-md-limits.md`: Created dedicated rule for AGENTS.md constitutional role, non-redundancy, and size ceiling (`trigger: always_on`).
  - `.agents/rules/diary-maintenance.md`: Created dedicated rule for DIARY.md Section 10 living engineering narrative logging (`trigger: model_decision`).
  - `.agents/rules/roadmap-maintenance.md`: Created dedicated rule for ROADMAP.md active list pruning, baseline updating, and milestone gates (`trigger: model_decision`).
  - `.agents/rules/docs-maintenance.md`: Refactored to focus strictly on single responsibility: synchronizing Obsidian design documentation with code and pruning draft proposals (`trigger: model_decision`).
  - `.agents/rules/*.md`: Standardized YAML frontmatter across all 24 rule files with explicit triggers:
    - 8 Universal Invariants: `trigger: always_on` (`language-policy.md`, `no-external-paths.md`, `audio-transcription.md`, `model-reasoning-advisory.md`, `spec-compliance.md`, `performance-and-readability.md`, `amiga-rag.md`, `agents-md-limits.md`).
    - 16 Domain-Specific Rules: `trigger: model_decision` with concise `description:` metadata.
  - `AGENTS.md`: Updated Section 1 to index all 24 rules categorized into Universal Invariants vs Domain-Specific Rules.
- **What Was Changed (The Concrete Reality)**:
  - Eliminated composite "hodgepodges" ("zbitki") where `docs-maintenance.md` had accumulated 8 disparate mandates (Obsidian linking, CI test running, formatting, AGENTS.md limits, DIARY logging, ROADMAP pruning).
  - Decomposed these into clean, cohesive, single-responsibility rule files.
  - Configured progressive disclosure via `trigger: model_decision` for all domain-specific rules, ensuring that only the 8 true constitutional invariants are injected unconditionally into every prompt turn.
  - Maintained `AGENTS.md` at 13,321 bytes (strictly $\le 14,000$ B limit).
- **Architectural Rationale & Trade-Offs**:
  - *Context Window Optimization & Instruction Dilution Prevention:* Injecting 20+ rules unconditionally on every turn burns ~60 KB of prompt context and causes cognitive diffusion, distracting the model from core task logic. Shifting domain-specific rules to `model_decision` cuts baseline prompt injection by ~70% while keeping every rule readily available via progressive disclosure.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 13 tests passed in 0.56s.
  - `AGENTS.md` size: 13,321 bytes (strictly $\le 14,000$).
  - All 24 rule files in `.agents/rules/*.md` verified $\le 23,000$ bytes.
  - `cargo fmt --all -- --check`: 100% compliant.

---

### [2026-09-12 13:25 CEST] — Automated Attractor Detection, Vocabulary Discipline & Immune Guardrails Against LLM Cognitive Drifts
- **Affected Subsystems**:
  - `scripts/lint_attractors.py`: Created standalone, zero-dependency Python 3 linter scanning repository files for synthetic linguistic attractors, theatrical testing phrasing, catchphrases, and leaked hardware buzzwords.
  - `.agents/rules/attractor-discipline.md`: Formulated and ratified dedicated operating rule (`trigger: always_on`) codifying prohibited attractor patterns, direct unpretentious replacements, and domain containment principles.
  - `crates/test_runner/tests/test_architecture_rules.rs`: Implemented `test_zero_synthetic_attractors()` native Rust architecture test running on every CI pass.
  - `.agents/rules/git-commits.md`: Integrated `python scripts/lint_attractors.py` into the mandatory pre-commit verification gate.
  - `AGENTS.md`: Indexed `attractor-discipline.md` in Section 1.A under Universal Invariants ($\le 14,000$ byte constitutional ceiling maintained).
- **What Was Changed (The Concrete Reality)**:
  - Deployed an automated, dual-layer immune defense system (Python CLI linter + native Rust architecture test) targeting the primary categories of linguistic attractors (*gravity wells*):
    1. *Academic/Philosophical Jargon:* Banning "epistemic", "teleological" across `.agents/`, `Obsidian/Amiga/Design/`, and `crates/`.
    2. *Theatrical Testing Phrasing:* Replacing inflated phrases like "testing oracle" or "oracle verification" with grounded engineering terms ("test verification reference", "ground truth vector").
    3. *Formulaic Attractor Catchphrases:* Prohibiting "zero-friction trap", "zero cognitive friction", and "The Invariance Invariant".
    4. *Hardware Microarchitecture Term Leaks:* Prohibiting the misapplication of physical CPU cache terminology ("L1i cache density", "L1 cache footprint thrashing") to high-level markdown documentation and agent rules.
  - Built-in whitelisting for historical retrospectives (`DIARY.md`), test harnesses (`test_architecture_rules.rs`), and the rule/linter definition files themselves (`attractor-discipline.md`, `lint_attractors.py`).
  - Executed negative regression testing: introduced a temporary markdown violation to prove that both `python scripts/lint_attractors.py` and `cargo test -p test_runner --test test_architecture_rules` fail loudly and block CI execution until sanitized.
- **Architectural Rationale & Human-AI Co-Design Insight**:
  - *The Inevitable Cognitive Biases of LLM Agentic Systems:*
    Every frontier AI model family develops its own characteristic cognitive slants, rhetorical deformations, and stylistic idiosyncrasies:
    - **ChatGPT:** Frequently drifts into corporate verbosity, sycophancy, excessive defensive hedging, and over-engineered enterprise boilerplate.
    - **Claude:** Frequently falls into hyper-intellectualized ethical reframing, polite conversational hedging, and self-referential analytical pedantry.
    - **Gemini:** Demonstrates a distinct tendency toward high-register pontification ("mądrkowanie"), adopting theatrical academic rhetoric ("epistemic drift", "teleological intentionality"), grandiose metaphors ("the testing oracle"), and cross-domain term contamination (such as projecting host CPU cache hardware terms like "L1i density" into simple rule documentation).
  - *The Autoregressive RAG Gravity Well (The Feedback Loop):*
    When an agent introduces high-register jargon into documentation or rule files, an insidious feedback loop begins:
    1. The model invents or adopts an inflated high-concept catchphrase.
    2. Subsequent user prompts trigger semantic RAG retrieval (`amiga-rag`) or keyword grep searches.
    3. The agent retrieves its own past synthetic prose, treats it as authoritative repository idiom, and re-injects the terms with even higher frequency into newly generated code and documentation.
    4. Over time, the vocabulary of the repository collapses into a sterile, pretentious monoculture that alienates human readers and obscures practical systems engineering reality.
  - *The Necessity of Automated Immune Defense:*
    Relying purely on polite prompt instructions or occasional conversational corrections is futile against the statistical gravitational pull of large language models over long development trajectories. In any serious, long-lived agentic project—whether powered by Gemini, Claude, or ChatGPT—architects must inevitably construct deterministic, automated guardrails (linters and architecture unit tests) that function as an immune system, mechanically enforcing linguistic discipline, domain containment, and unpretentious engineering clarity.
- **Verification & Test Results**:
  - `python scripts/lint_attractors.py`: Clean pass across 263 files (exit code 0).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed (0.55s).
  - Deliberate negative regression verified and passed.
  - `AGENTS.md` size confirmed on disk: 13,473 bytes (strictly within $\le 14,000$ byte threshold).
  - `cargo fmt --all -- --check`: 100% compliant.

---

### [2026-09-12 13:40 CEST] — De-clustering "Mechanical Sympathy" Heading & Slogan Attractor
- **Affected Subsystems**:
  - `scripts/lint_attractors.py`: Added heading sloganization detection (`^#+\s+.*mechanical sympathy`) and inflated slogan filters (`guardian of mechanical sympathy`, `mechanical sympathy invariant`, `outlawing mechanical sympathy`), with automated case-preserving `--fix` substitutions.
  - `crates/test_runner/tests/test_architecture_rules.rs`: Integrated heading sloganization assertions into `test_zero_synthetic_attractors()`.
  - `.agents/rules/attractor-discipline.md`: Added heading slogan prohibition and grounded replacements table entry.
  - `.agents/rules/performance-and-readability.md`: De-sloganized rule title and Section 1 heading to `High Performance & Host Hardware Efficiency` and `Hardware-Aligned Execution`.
  - `AGENTS.md`: Updated Section 1.A and Section 3.5 pointers to `Hardware Efficiency & Readability` ($\le 14,000$ B limit preserved).
  - `.agents/workflows/code-review.md`: Updated Section B heading and checklist item to `Host Hardware Efficiency`.
  - `ROADMAP.md`: De-sloganized Step 2 heading and task bullets to `Host Pipeline Optimization` and `host hardware efficiency principles`.
  - `Obsidian/Amiga/Design/`: Refactored headings, tables, and bullets across `CPU Motorola M68000.md`, `CPU Instruction Benchmarking.md`, `CPU Micro-Step State Machine.md`, `Rust Guidelines.md`, and `Git Worktree Workflow.md`.
  - `crates/m68000/src/state.rs`: Refactored line 249 comment to `Host Hardware Efficiency`.
- **What Was Changed (The Concrete Reality)**:
  - Pruned repetitive sloganized occurrences of `mechanical sympathy` across Markdown section headings, checklist summaries, and documentation bullets.
  - Replaced inflated catchphrases with precise, varied systems engineering terms (`host hardware efficiency`, `host pipeline optimization`, `hardware-aligned execution`, `physical execution reality`).
  - Confined legitimate references strictly to grounded historical context (Martin Thompson's original systems engineering definition in early diary logs and substrate architecture notes).
  - Enforced automated CI gates ensuring that future agent passes cannot re-introduce `mechanical sympathy` into Markdown headings or slogans.
- **Architectural Rationale & Trade-Offs**:
  - *Preventing Slogan Mode Collapse:* Even valuable systems engineering concepts suffer from rhetorical mode collapse when an LLM begins slapping them indiscriminately into every title, heading, and rule checklist. De-clustering catchphrases preserves professional variety, clarity, and precision.
- **Verification & Test Results**:
  - `python scripts/lint_attractors.py`: Clean pass across 263 files (exit code 0).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passing.
  - `AGENTS.md` size confirmed on disk: 13,501 bytes (strictly within $\le 14,000$ byte threshold).
  - `cargo fmt --all -- --check`: 100% compliant.

---

### [2026-09-12 14:26 CEST] — Repository Sanitization, Git History Normalization & 99.8% Packfile Reduction
- **Affected Subsystems**:
  - `.gitignore`: Decoupled heavy and external assets (`ref_src/`, `Obsidian/Amiga/Reference/`, `schematics/`, `archive/`, `graphify-out/`, `tools/winguide/`, `tools/AmigaTestKit-*/`, `adfs/`, `progs/`).
  - Git Object Database (`.git`): Rewrote history using `git-filter-repo` in an isolated scratch sandbox, stripping all historical blobs from past commits while preserving 100% of files on local disk.
  - Commit History: Normalized 29 generic, auto-generated, or low-signal commit messages into clean Conventional Commits; pruned 146 empty commits via `--prune-empty=always`; retained original author names, emails, and commit timestamps.
  - `ROADMAP.md`: Marked Milestone 5 (Repository Sanitization & Public Release Preparation) completed across all sub-tasks.
- **What Was Changed (The Concrete Reality)**:
  - Audited all 358 historical commits across the repository, identifying ~11.2 GB of uncompressed historical bloat from external reference emulators (`ref_src/`), bulky manual scans (`docs-org/`, `docs-todo/`), schematics, and generated AST graphs.
  - Established the core safety invariant: all files currently on the local workstation remain 100% intact on disk (23,177 files in `ref_src`, 409 reference files, 162 schematics, 117 archive files).
  - Updated `.gitignore` to ignore external reference directories and committed as a prep commit on `master`.
  - Executed memory-stream filtering via `git-filter-repo` in `scratch/amiga_clean`, purging all bulky paths across the entire commit graph.
  - Pruned stale worktree metadata (`.git/worktrees/Amiga-gui`), transient IDE diff refs (`refs/codex/...`), and rebuilt the Git commit graph cache (`git commit-graph write --reachable`).
  - Successfully reduced the active repository `.git` packfile database from **2,841.74 MB (~2.84 GB) down to 4.43 MB** (a **99.8% size reduction**).
- **Architectural Rationale & Trade-Offs**:
  - *Public Release Cleanliness vs. Local Development Continuity:* A public cycle-exact emulator repository cannot redistribute gigabytes of third-party GPL emulators, scanned corporate service manuals, or IDE state files. Decoupling them via `.gitignore` allows the public repository to remain pristine and lightweight (~4.4 MB packfile), while local CPU test runners (`test_singlestep.rs`, `test_dma_cartesian.rs`) and RAG indexing scripts continue accessing the existing files locally on disk with zero friction.
- **Verification & Test Results**:
  - `git count-objects -vH`: `size-pack: 4.43 MiB`, `in-pack: 4008`, zero loose objects, zero garbage.
  - `git fsck --full`: 100% clean verification, 0 errors, 0 dangling references.
  - `git status`: Clean working tree on `master`.
  - Local asset persistence: Confirmed all 23,177 files in `ref_src/`, 409 files in `Obsidian/Amiga/Reference/`, 162 in `schematics/`, 117 in `archive/` intact on disk.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passing (0.54s).
  - `cargo test -p test_runner --test test_singlestep -- nop`: Passed.
  - `python scripts/lint_attractors.py`: Clean pass across 263 files (exit code 0).

---

### [2026-09-12 14:36 CEST] — Deletion of Obsolete WinGuide Utility & Tools Cleanup
- **Affected Subsystems**:
  - `tools/winguide/`: Deleted unused third-party viewer directory (`WinGuide.exe`, `Winguide.rea`).
  - `README.md`: Removed `tools/winguide/` from repository layout tree; updated `tools/blep_generator` entry.
  - `.gitignore`: Pruned obsolete `/tools/winguide/` ignore pattern.
- **What Was Changed (The Concrete Reality)**:
  - Permanently removed the obsolete third-party viewer directory `tools/winguide/` containing `WinGuide.exe` and `Winguide.rea`.
  - Synchronized `README.md` repository directory tree structure to reflect active developer tools.
  - Removed dangling `/tools/winguide/` pattern from `.gitignore`.
- **Architectural Rationale & Trade-Offs**:
  - *Tooling Hygiene:* AmigaGuide documentation has been fully converted into structured Markdown under `Obsidian/Amiga/Reference/` and indexed into the local RAG knowledge base. The standalone Windows viewer utility was unused and redundant.
- **Verification & Test Results**:
  - Verified directory removal on disk (`Test-Path "tools\winguide"` returned `False`).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed (0.52s).
  - `python scripts/lint_attractors.py`: Clean pass across all files.

---

### [2026-09-12 14:50 CEST] — Hardware Schematics Catalog in README & Blep Generator Audio Filter Schematic
- **Affected Subsystems**:
  - `README.md`: Added Section 5.6 (*Hardware Schematics & Circuit Archives*) with direct links to Amiga PCB Explorer, Toni Wilen's Amiga Technical Resource archive, Retro-Commodore scans, and BBOAH; annotated `schematics/` in repository tree.
  - `tools/blep_generator/`: Integrated full high-resolution circuit schematic [`a500_audio_filter_schematic.png`](file:///d:/Programowanie/Amiga/tools/blep_generator/a500_audio_filter_schematic.png) from Commodore Rev 6A/7 Sheet 4 (covering Paula 8364 audio outputs, the complete Audio Filters down to ground, Gary floppy logic, and Paula/U14 power decoupling); fixed Markdown rendering in `tools/blep_generator/README.md`.
- **What Was Changed (The Concrete Reality)**:
  - Documented online public mirrors for hardware schematics in `README.md`, fulfilling external link centralization without bloating the Git repository.
  - Extracted the complete Commodore Sheet 4 schematic into `tools/blep_generator/a500_audio_filter_schematic.png`, preserving all ground lines, power supply rails, and IC pinouts without bottom cutoff.
  - Corrected image embedding syntax in `tools/blep_generator/README.md` (`./a500_audio_filter_schematic.png`, unindented) to ensure immediate rendering across VS Code and GitHub markdown previews.
- **Architectural Rationale & Trade-Offs**:
  - *Contextual Proximity & Completeness:* Having the entire circuit schematic (Paula audio pins -> filter network -> op-amp power rails) side-by-side with the BLEP synthesis mathematical equations and IIR biquad models provides instant visual clarity on physical component designations ($R_{331}, C_{331}, R_{332}, R_{333}, C_{332}, C_{333}$, power decoupling) for Paula DSP development.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed (0.53s).
  - `cargo fmt --all -- --check`: Clean formatting across the workspace.
  - `python scripts/lint_attractors.py`: Clean pass across 263 files.

---

### [2026-09-12 15:02 CEST] — Extraction of Foundational Hardware Circuit Realities into MemoryBus Design Spec
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design/MemoryBus.md`: Formalized four hardware architecture ground-truth realities extracted from motherboard circuit schematics and Gary pinouts.
- **What Was Changed (The Concrete Reality)**:
  - **CIA Partial Address Decoding & Gary Chip Select:** Documented that MOS 8520 CIAs only connect `RS0..RS3` to 68000 address lines `A8..A11` (with `A1..A7` unconnected), causing every register to mirror across 256-byte boundaries; Gary decodes `_CS` via `A12 = 0` (CIA-A, odd bytes) and `A13 = 0` (CIA-B, even bytes). Contrasted with custom chips (`A1..A8` connected, `A9..A15` ignored, 128-fold mirroring across `$DFF000-$DFFFFE`).
  - **Slow RAM Contention & OCS Agnus Invisibility:** Documented that trapdoor RAM at `$C00000-$C7FFFF` is decoded by Gary (`_RAMEN`), physically resides on the shared Chip RAM bus, and suffers full wait states via `_DTACK` withholding whenever Agnus DMA is active. Clarified the OCS Agnus 19-bit DRAM address limit (`DRA0..DRA8` = 512 KB), proving that custom chip DMA physically cannot address or see Slow RAM.
  - **Boot Overlay Asymmetry (CPU vs Custom Chipset):** Established that Gary's `_OVL` interception applies strictly to 68000 CPU bus transactions (`A23..A19`), whereas Agnus DRAM address lines (`DRA0..DRA8`) drive Chip RAM directly. Even while `_OVL = 0`, custom chip DMA accesses to `$000000` always hit physical Chip RAM, never Kickstart ROM.
  - **Paula & Chipset DMA Bus Signaling (`DMAL` & `RGA`):** Documented Agnus master DMA scheduling and physical signaling to Paula via pin 12 (`DMAL`) and register address lines `RGA(8:1)` for `AUDxDAT` and `DSKDAT`.
  - Updated YAML frontmatter `related` array linking `Paula.md` and `CIA.md`.
- **Architectural Rationale & Trade-Offs**:
  - *Extracting Hardware Realities from Schematics:* As physical schematics are decoupled and untracked, vital hardware circuit facts (pin wirings, address line skips, and cross-chip bus arbitrations) are permanently documented in the authoritative design specification and indexed into RAG.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed (0.53s), including link integrity check.
  - `python scripts/lint_attractors.py`: Clean pass across 263 files.

---

### [2026-09-12 15:05 CEST] — Deletion of Decoupled Local Hardware Schematics Directory
- **Affected Subsystems**:
  - `schematics/`: Removed the local directory (162 files, ~127 MB) from disk following the centralization of curated online schematic mirrors in `README.md` (Section 5.6) and extraction of the audio filter schematic to `tools/blep_generator/`.
  - `README.md`: Pruned `schematics/` entry from the directory layout tree.
- **What Was Changed (The Concrete Reality)**:
  - Deleted the untracked `schematics/` directory via `Remove-Item -Recurse -Force "schematics"`.
  - Updated the repository layout tree in `README.md` to reflect the removal.
  - Preserved `.gitignore` entries (`/schematics/`, `/schematics-unused/`) to prevent accidental commits of local schematics.
- **Architectural Rationale & Trade-Offs**:
  - *Clean-Room Hygiene & Lean Disk Footprint:* With essential hardware circuit realities transcribed into `MemoryBus.md`, the audio filter circuit schematic preserved directly alongside the BLEP tool in `tools/blep_generator/`, and verified public download mirrors indexed in `README.md`, retaining ~127 MB of redundant PDFs and scans locally on disk is unnecessary.
- **Verification & Test Results**:
  - Verified directory removal on disk (`Test-Path "schematics"` returned `False`).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed.
  - `python scripts/lint_attractors.py`: Clean pass.

---

### [2026-09-12 15:10 CEST] — Condensation of README Hardware Schematics Section (Second-Order Reduction)
- **Affected Subsystems**:
  - `README.md`: Replaced the 25-line Section 5.6 external PDF link-farm with a concise 2-sentence note pointing to online archives on demand and anchoring developers to in-repo circuit specifications (`Obsidian/Amiga/Design/` and `tools/blep_generator/`).
- **What Was Changed (The Concrete Reality)**:
  - Eliminated 15 fragile, deep links to individual PDFs hosted on a dynamic DNS home server (`amiga.serveftp.net`) for non-A500 hardware (A3000, A4000, CD32, expansions).
  - Preserved direct pointer to Amiga PCB Explorer and repository-internal circuit documentation.
- **Architectural Rationale & Trade-Offs**:
  - *Mitigating Bit-Rot & Redundant Link-Farms:* Deep links to external PDF archives degrade rapidly and create maintenance drag. Developers and LLMs can query search engines or generative tools on demand, while core hardware design invariants remain formally documented and verified within the repository.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed.
  - `python scripts/lint_attractors.py`: Clean pass across 263 files.

---

### [2026-09-12 15:18 CEST] — Packaging of Attractor Discipline Skill & Linter Relocation
- **Affected Subsystems**:
  - `.agents/skills/attractor-discipline/`: Created dedicated agent skill (`SKILL.md`) and packaged linter script under `scripts/lint_attractors.py`.
  - `scripts/lint_attractors.py`: Converted into a clean forwarding trampoline to ensure complete backward compatibility with existing workflows and commands.
  - `.agents/rules/attractor-discipline.md`: Updated Section 5 to reference the packaged skill and linter location.
  - `.agents/skills/code-review/SKILL.md`: Added linguistic attractor and vocabulary discipline audit to Step 2 and the final review checklist.
- **What Was Changed (The Concrete Reality)**:
  - Formulated `.agents/skills/attractor-discipline/SKILL.md` documenting validation commands, automated `--fix` cleaning workflows, targeted path scanning, and whitelisting.
  - Relocated full linter implementation into `.agents/skills/attractor-discipline/scripts/lint_attractors.py` with enhanced repository root discovery (`ROADMAP.md` parent traversal).
  - Maintained `scripts/lint_attractors.py` as a lightweight trampoline using standard library `runpy.run_path`.
- **Architectural Rationale & Trade-Offs**:
  - *Separation of Policy and Tooling:* Aligning with repository customization principles, declarative constraints live in rules (`.agents/rules/`), while actionable execution scripts and runbooks belong within structured skills (`.agents/skills/`). The root trampoline prevents any breakage in established habit or CI scripts.
- **Verification & Test Results**:
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).
  - `python scripts/lint_attractors.py`: Trampoline executed successfully with exit code 0.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed (0.55s).

---

### [2026-09-12 15:35 CEST] — Reference Code Clean-Room Wipe & Pure Hardware Single-Step Test Architecture
- **Affected Subsystems**:
  - `ref_src/`: Purged 14 obsolete reference cores, simulators, and testbenches (~1.2 GB), retaining strictly `SingleStepTests-680x0` (Tom Harte physical silicon vectors), `vAmiga-4.5` (clean C++ reference), and `vAmigaTS` (ADF test suite).
  - `crates/test_runner/src/runner.rs`: Purged all 8 MAME simulator divergence workarounds (`!is_harte`), enforcing 100% physically faithful hardware cycle testing.
  - `crates/test_runner/tests/test_singlestep.rs`: Streamlined test runner to execute exclusively against Tom Harte hardware vectors; rewrote `test_stop` to verify `STOP` opcode directly via Rust CPU state stepping.
  - `crates/test_runner/tests/test_dma_cartesian.rs`: Updated `load_hardware_tests` to load test vectors directly from `ref_src/SingleStepTests-680x0/68000/v1/`.
  - `README.md`: Updated directory tree, Section 5 catalog, and Section 6 test commands to reflect the 3 clean-room reference assets and hardware single-step testing.
  - `AGENTS.md`, `ROADMAP.md`, `BOOTSTRAP.md`: Updated references to SingleStepTests and checked off the clean-room reference code wipe milestone.
  - `Obsidian/Amiga/Design/*.md`: Fixed broken markdown links pointing to pruned `ref_src` emulators across `Agnus.md`, `CIA.md`, `CPU Micro-Step State Machine.md`, `CPU Motorola M68000.md`, `CPU SingleStepTests.md`, `Denise.md`, `Floppy.md`, `Main loop A500.md`, `Paula.md`, `RTC.md`, and `SaveState.md`.
- **What Was Changed (The Concrete Reality)**:
  - Deleted ~1.12 GB MAME SingleStepTests archive and purged all tolerance workarounds from the test runner.
  - Re-routed all test suites (SingleStepTests, Cartesian DMA contention) to physical silicon captures.
  - Execution speed: SingleStep test execution time halved from ~12s down to 5.43s.
  - Cartesian DMA contention tests ran all 19 permutation suites in 37.49s on Tom Harte vectors with 0 failures.
- **Architectural Rationale & Trade-Offs**:
  - *Silicon Truth over Simulator Artifacts:* MAME's 68000 core had microcode bugs and simulator quirks (such as faulty `TAS` bus cycles, `TRAPV` status bits, and pre-fault AGU handling) that required artificial tolerance branches (`if !is_harte`) in our test runner. Pruning MAME and aligning 100% with Tom Harte vectors guarantees that our emulator is verified against physical Motorola 68000 silicon.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_singlestep`: All 127 tests passed in 5.43s (0 failures).
  - `cargo test -p test_runner --test test_dma_cartesian`: All 19 tests passed in 37.49s (0 failures).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.53s.
  - `cargo fmt --all -- --check`: Passed cleanly.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).

---

### [2026-09-12 15:45 CEST] — Compiling, Running & Two-Tier Bootstrapping Architecture
- **Affected Subsystems**:
  - `README.md`: Added Section 3 ("Compiling & Running the Emulator") and Section 4 ("Bootstrapping the Environment (-Doc vs -Test)"). Clarified that a fresh clone compiles and runs the GUI out of the box without any external downloads or bootstrapping.
  - `tools/bootstrap.ps1`: Implemented repository bootstrapper supporting `-Doc` (AI documentation & local RAG vector indexing) and `-Test` (Tom Harte physical silicon single-step vectors & test media validation).
  - `scripts/`: Deleted `scripts/lint_attractors.py` and removed root `scripts/` directory, centralizing the linter cleanly inside `.agents/skills/attractor-discipline/scripts/lint_attractors.py`.
  - `.agents/rules/` & `.agents/skills/`: Updated `attractor-discipline.md`, `git-commits.md`, `attractor-discipline/SKILL.md`, and `code-review/SKILL.md` to point directly to the packaged skill script.
- **What Was Changed (The Concrete Reality)**:
  - Formulated a clear two-tier bootstrapping architecture addressing the user's workflow requirements:
    1. *Knowledge & AI Documentation Bootstrap (`-Doc`):* Provisions Qdrant and indexes `Obsidian/Amiga/` (Commodore manuals and design notes) into local vector search for AI agent interaction and technical research.
    2. *Verification & Hardware Test Suite Bootstrap (`-Test`):* Verifies and provisions Tom Harte `SingleStepTests-680x0` hardware test vectors and system test disks (`vAmigaTS`, `AmigaTestKit`) for compiling and running test suites.
  - Provided complete compilation instructions (`cargo build`, `cargo build --release -p gui`, `cargo check --target wasm32-unknown-unknown`) and desktop/WASM runtime commands (`cargo run -p gui`, `--game`, `--load`, `trunk serve`).
- **Architectural Rationale & Trade-Offs**:
  - *Separation of Runtime vs Research/Test Dependencies:* A developer or user should never be forced to download ~1 GB of test vectors or run vector databases just to build and enjoy the emulator. Decoupling the workflow into self-contained compilation, optional RAG documentation indexing (`-Doc`), and optional exhaustive test vector provisioning (`-Test`) maintains a lightweight developer experience.
- **Verification & Test Results**:
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Test`: Passed cleanly, verifying 124 hardware test suites and executing smoke test (`test_nop`).
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1`: Usage menu verified.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.53s.
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 15:48 CEST] — Player-First README Landing Page & Technical Documentation Extraction to `docs/`
- **Affected Subsystems**:
  - `README.md`: Restructured the landing page into a punchy, 4-tier landing page and cheat sheet ("ściąga") tailored for players and developers.
    1. *Scope & Hardware Targets:* Amiga 500 (OCS) gaming emulator dedicated to authentic floppy gaming (`DF0:`, `.adf`, direct binary injection) without hard drives or bulky expansion clutter; configurations: Basic (512 KB Chip), Classic (512 KB Chip + 512 KB Slow RAM at `$C00000`), Expanded (Fast RAM); Native desktop + WebAssembly.
    2. *For Players (Quick Start):* Clean game launch (`--game`), WebAssembly browser launch (`trunk serve`), instant clean-screen pause/resume toggle (`F12`), loading floppy disks (`.adf`) and machine code binaries (`--load`), complete global keybindings reference table, and Developer Studio features (time-travel rewind, live register diffs, inline memory hex editing, disassembly patching, and breakpoints).
    3. *For Developers (Build & Bootstrap):* Zero-setup build instructions (`cargo build`, `cargo run -p gui`), explicit notice that bootstrapping is strictly optional and not required to compile or play, and clean two-tier bootstrapping summary (`tools/bootstrap.ps1 -Doc` vs `-Test`).
    4. *Documentation Cheat Sheet & Technical Index:* Standardized links to deep technical docs in `docs/` and structured index of all 28 subsystem architecture specifications under `Obsidian/Amiga/Design/`.
  - `docs/`: Created GitHub-standard documentation repository containing extracted deep technical guides:
    - `docs/architecture.md`: Formal CCK1/CCK2 Color Clock phases, Gary bus arbitration, Agnus DMA contention, circuit simulation, Big-Endian invariance, and decoupled ownership.
    - `docs/testing.md`: SingleStep test options (`SINGLESTEP_FULL`, `SINGLESTEP_LIMIT`), Cartesian DMA contention math ($2^k \times 2^M$), and CLI regression diagnostics.
    - `docs/ai_agents.md`: AI agent pair-programming guide, rules adherence, RAG knowledge base, Graphify AST, and specialized skills.
    - `docs/worktrees.md`: Multi-branch parallel workflows and isolated build contexts with Git worktrees.
- **What Was Changed (The Concrete Reality)**:
  - Pruned dense, verbose architectural and testing blocks from `README.md`, reducing it from 386 lines down to an attractive, easily scannable 130-line landing page.
  - Placed player experience and quick start front and center while retaining an organized technical index for contributors and AI agents.
- **Architectural Rationale & Trade-Offs**:
  - *Separation of Landing Page vs Technical Deep Dives:* A repository `README.md` serves as a front porch for users and players trying to launch games or developers wanting to compile. Burying keybindings and launch commands beneath pages of microarchitectural bus timing diagrams hurt readability. Extracting deep specifications into GitHub-standard `docs/` maintains rigorous technical documentation while presenting a welcoming, intuitive landing page.
- **Verification & Test Results**:
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.54s (verifying zero broken markdown links in `README.md` and `docs/`).
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 268 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 15:52 CEST] — Complete Purge of MAME Simulator Quirks & Dual-Suite References from Documentation, Runner & Skills
- **Affected Subsystems**:
  - `Obsidian/Amiga/Design/CPU SingleStepTests.md`: Streamlined the entire specification to focus 100% on Tom Harte physical silicon vectors (`SingleStepTests-680x0`). Removed the obsolete "Dual Test Suite Overview" and MAME transaction format sections; updated Section 9 to detail physical hardware invariants (`TAS` indivisible RMW, `ASR` sign exhaustion, Address Error AGU commit) and failure diagnosis protocols.
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`: Removed all historical MAME simulator divergence notes from sections 7.6 through 7.9. Documented pure physical silicon MC68000 behavior for postincrement AGU commitment on unaligned access, shift count > width pipeline exhaustion ($C=0, X=0$), and 32-bit `MOVE.l` condition code evaluation.
  - `Obsidian/Amiga/Design/General Architecture.md`: Updated table entries and links to reference Tom Harte physical silicon vectors exclusively.
  - `Obsidian/Amiga/Design/CPU Instruction Benchmarking.md`: Updated `--suite` CLI documentation to target Tom Harte vectors.
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md`: Replaced MAME reference in Address Error stacking order with Motorola PRM Figure B-9 and physical captures.
  - `crates/test_runner/src/main.rs`: Removed dead `mame_path` execution branch in `run_specific_suite`, executing purely against Tom Harte hardware captures without file-not-found errors.
  - `crates/test_runner/src/runner.rs`: Cleaned up program counter verification comments and standardized suite names directly to `Real68k::<stem>`.
  - `crates/test_runner/src/schema.rs`, `crates/test_runner/src/transactions.rs`, `crates/test_runner/tests/test_dma_cartesian.rs`: Cleaned up header doc comments and inline notes.
  - `.agents/skills/add-m68k-instruction/SKILL.md`: Updated Tier 1 verification gate example from `run_dual_test` to `run_test`.
  - `.agents/skills/m68k-singlestep-test/SKILL.md`: Updated regression detection sample alert to `Real68k::ADD.b`.
- **What Was Changed (The Concrete Reality)**:
  - Following the earlier deletion of the MAME test suite from `ref_src/` and removal of `!is_harte` tolerance branches in `runner.rs`, conducted a full repository-wide audit for all lingering references to MAME and simulator divergences.
  - Every active design document, skill, test script, and runner file was brought into 100% alignment with our hardware ground-truth policy.
- **Architectural Rationale & Trade-Offs**:
  - *Unified Single-Source Ground Truth:* Retaining simulator divergence documentation and dual-suite tooling after decommissioning the flawed simulator core created cognitive friction and confusion. Emulation behavior is now anchored entirely in physical Motorola 68000 silicon pin captures.
- **Verification & Test Results**:
  - `cargo run -p test_runner -- --suite ADD.b`: Executed cleanly against Tom Harte vectors with 50/50 tests passed in 0.12s.
  - `cargo test -p test_runner --test test_singlestep test_nop`: Passed in 1.28s.
  - `cargo test -p test_runner --test test_singlestep test_add_b`: Passed in 0.40s.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.52s (0 broken links across all 28 Obsidian documents).
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 15:58 CEST] — Enhanced Bootstrapper Qdrant Port Probing & Plain-Language Diagnostics
- **Affected Subsystems**:
  - `tools/bootstrap.ps1`: Upgraded Tier 1 (`-Doc` / `-All`) bootstrap workflow with active pre-flight connectivity verification.
- **What Was Changed (The Concrete Reality)**:
  - Replaced blind execution of `amiga_rag.ps1` with active TCP port probing on `http://localhost:6333` using `Test-NetConnection -InformationLevel Quiet -WarningAction SilentlyContinue`.
  - Added smart auto-recovery: if port 6333 is closed but Docker is present and an existing `amiga-qdrant` container exists (stopped), the script attempts to start it automatically (`docker start amiga-qdrant`), re-checking port connectivity before giving up.
  - Formulated clear, comprehensive diagnostic instructions ("kawa na ławę") if Qdrant remains unreachable:
    - Explains what Qdrant is (open-source vector search engine hosting embeddings for Commodore Hardware Reference Manuals, M68000 PRMs, and architecture notes).
    - Details exact startup options: 1-line Docker command (`docker run -d --name amiga-qdrant -p 6333:6333 -p 6334:6334 -v qdrant_storage:/qdrant/storage:z qdrant/qdrant:latest`) and standalone executable download instructions from GitHub releases.
    - Emphasizes that Qdrant is strictly optional and not required to compile or play the emulator (`cargo run -p gui`).
- **Architectural Rationale & Trade-Offs**:
  - *Preventing Silent Connection Crashes:* Rather than allowing Python to crash with a raw stack trace when Qdrant is absent, probing the port upfront gives users instant, self-explanatory instructions on how to provision the database.
- **Verification & Test Results**:
  - Verified port probing returns `$true` when Qdrant is active and `$false` on closed ports without warning noise.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.52s.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 16:05 CEST] — Added vAmiga Reference Emulator Verification to Bootstrapper
- **Affected Subsystems**:
  - `tools/bootstrap.ps1`: Added verification and fallback acquisition guidance for the clean-room C++ reference emulator (`ref_src/vAmiga-4.5` / `ref_src/vAmiga`) in Tier 2 (`-Test` / `-All`).
- **What Was Changed (The Concrete Reality)**:
  - Added existence check for `ref_src\vAmiga-4.5` (falling back to `ref_src\vAmiga`).
  - Added warning banner and direct GitHub clone URL (`https://github.com/dirkwhoffmann/vAmiga`) if absent.
  - Added corresponding warnings and download instructions for `AmigaTestKit.adf` and `vAmigaTS`.
- **Verification & Test Results**:
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Test`: Successfully detected `vAmiga-4.5`, `SingleStepTests-680x0`, `AmigaTestKit.adf`, and `vAmigaTS`, executing the smoke check with 0 failures.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.96s.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 16:08 CEST] — Native .NET Decompression of SingleStepTests Archives (.gz / .zip) in Bootstrapper
- **Affected Subsystems**:
  - `tools/bootstrap.ps1`: Added recursive archive extraction (`.zip`) and streaming decompression (`*.json.gz`, `*.gz`) for `SingleStepTests-680x0` in Tier 2 (`-Test` / `-All`).
  - `README.md`: Updated Tier 2 provisioning summary in the bootstrapping table.
  - `docs/testing.md`: Documented the automated archive decompression workflow under SingleStepTests verification options.
- **What Was Changed (The Concrete Reality)**:
  - Implemented automatic archive discovery and native decompression in `tools/bootstrap.ps1`:
    1. *ZIP Extraction:* Recursively identifies any `.zip` archives within `ref_src\SingleStepTests-680x0` and extracts them in-place via `Expand-Archive -Force`.
    2. *Streaming GZip Decompression:* Recursively identifies all `*.json.gz` or `*.gz` files under `ref_src\SingleStepTests-680x0`. Decompresses each file to its corresponding `.json` file using native .NET `System.IO.Compression.GZipStream` without requiring external binaries (`gzip`, `7z`, or Python packages).
    3. *Idempotency:* Checks whether the target `.json` file already exists with non-zero length; if present, skips decompression instantly (0 ms overhead on warm runs).
    4. *Directory Normalization:* Automatically checks if `.json` files are situated in `68000/` rather than the canonical `68000/v1/` subfolder expected by `test_singlestep.rs`, creating `v1/` and migrating the suites if needed.
    5. *Status Reporting:* Emits clear console progress displaying the number of decompressed suites and active test suite count.
- **Architectural Rationale & Trade-Offs**:
  - *Developer Onboarding & Storage Footprint:* Upstream SingleStepTests contain ~124 per-instruction files expanding to ~1.1 GB of uncompressed JSON. Clones or release downloads often bundle them in compressed format (`.gz` / `.zip`, ~50–80 MB). Requiring developers to manually decompress 124 separate GZ files creates unnecessary friction.
  - *Zero External Dependencies:* Implementing the decompression stream with .NET's built-in `GZipStream` ensures 100% platform portability across Windows PowerShell 5.1 and PowerShell 7+ on any developer machine without requiring third-party tools.
- **Verification & Test Results**:
  - Verified full round-trip decompression by injecting a compressed `.json.gz` payload into `ref_src\SingleStepTests-680x0\68000\v1\`, executing `.\tools\bootstrap.ps1 -Test`, verifying extraction into valid `.json`, and safely cleaning up.
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Test`: Passed smoke check `test_nop` and verified 124 suites.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.52s.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 16:11 CEST] — Bootstrapper Reordering: Test Vectors & Verification First, RAG Ingestion Last
- **Affected Subsystems**:
  - `tools/bootstrap.ps1`: Inverted tier execution order so Tier 1 executes test suite provisioning and verification (`-Test`), while Tier 2 executes documentation and RAG vector ingestion (`-Doc`). Added dynamic step numbering (`[$CurrentStep/$TotalSteps]`).
  - `README.md`: Aligned bootstrap reference table and CLI examples to present `-Test` first and `-Doc` second.
- **What Was Changed (The Concrete Reality)**:
  - Reorganized execution logic in `tools/bootstrap.ps1`:
    1. *Verification First:* Hardware test suite extraction, `.gz` streaming decompression, reference emulator verification (`vAmiga-4.5`, `vAmigaTS`), and `test_nop` smoke execution now run first (`Tier 1`).
    2. *RAG Ingestion Last:* Documentation knowledge base connectivity checks and `amiga_rag.ps1` indexing now run last (`Tier 2`).
    3. *Dynamic Progress Counter:* Implemented `$CurrentStep = 1; $TotalSteps = if ($All) { 2 } else { 1 }` so standalone runs display `[1/1]` while combined `-All` runs display `[1/2]` and `[2/2]`.
    4. *CLI Help & Documentation:* Updated `Show-Usage`, parameter comments, and `README.md` to consistently guide developers on the new order.
- **Architectural Rationale & Trade-Offs**:
  - *Immediate Feedback vs Heavy Ingestion:* Test suite verification and decompression complete in ~1 second, immediately confirming that local testbeds and reference emulators are fully operational. RAG vector ingestion across hundreds of documentation chunks requires several minutes of compute. Running the fast, critical testbed verification first ensures that running `-All` gives immediate confidence before entering the long-running vectorization stage.
- **Verification & Test Results**:
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1`: Verified updated `Show-Usage` menu.
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Test`: Verified dynamic `[1/1]` header and clean smoke test execution.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.61s.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.

---

### [2026-09-12 16:14 CEST] — Added Code Knowledge Graph (Graphify AST) Provisioning Tier to Bootstrapper
- **Affected Subsystems**:
  - `tools/bootstrap.ps1`: Added `-Graph` switch, Tier 2 Graphify code AST extraction (`graphify update .`), dynamic 3-tier step counter (`$Test`, `$Graph`, `$Doc`), and missing CLI diagnostic guidance.
  - `README.md`: Documented the Code Knowledge Graph tier in the bootstrapping table and quickstart examples.
- **What Was Changed (The Concrete Reality)**:
  - Integrated Graphify code knowledge graph generation into `tools/bootstrap.ps1` as a first-class AI knowledge component alongside Qdrant documentation RAG.
  - Formulated a clear 3-tier execution sequence when running `-All`:
    1. *Tier 1: Verification & Hardware Test Suites (`-Test`):* Hardware test vectors, `.gz` stream decompression, reference emulators (`vAmiga-4.5`, `vAmigaTS`), and `test_nop` smoke execution (~1s).
    2. *Tier 2: Code Knowledge Graph (`-Graph`):* Fast AST-level symbol extraction and call hierarchy analysis into `graphify-out/` without LLM calls (~4s on incremental, ~30s on full re-index).
    3. *Tier 3: Documentation & AI Knowledge Base (`-Doc`):* Local vector database ingestion of Commodore HRM, M68000 PRMs, and Obsidian design specs into Qdrant (`amiga_rag.ps1`).
  - Added dynamic multi-switch progress calculation (`$TotalSteps = ($Test ? 1 : 0) + ($Graph ? 1 : 0) + ($Doc ? 1 : 0)`), ensuring arbitrary combinations (`-Test -Graph`, `-Graph -Doc`, `-All`) display exact sequential step counts (`[1/2]`, `[2/2]`, `[1/3]`).
  - Added helpful diagnostic explanations and installation guidance (`pip install graphify`) if `graphify` is absent from PATH.
- **Architectural Rationale & Trade-Offs**:
  - *Dual-Engine AI Knowledge Base:* As codified in `.agents/rules/amiga-rag.md`, AI agent pair-programming relies on two complementary knowledge systems: RAG provides natural language hardware domain specifications, while Graphify provides structural code intelligence (functions, types, call graphs, AST relationships). Incorporating Graphify into the bootstrapper ensures both knowledge engines can be provisioned with a single command.
  - *Ascending Complexity Pipeline:* Organizing `-All` into Tests (~1s) $\rightarrow$ Graphify AST (~15s) $\rightarrow$ Documentation RAG (heaviest) ensures that fast, deterministic steps execute first and give immediate feedback before entering multi-minute vectorization.
- **Verification & Test Results**:
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1`: Verified updated 4-switch usage help.
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Graph`: Verified standalone execution and incremental AST extraction in 4s.
  - `powershell -ExecutionPolicy Bypass -File .\tools\bootstrap.ps1 -Test -Graph`: Verified dynamic `[1/2]` and `[2/2]` step sequence.
  - `cargo test -p test_runner --test test_architecture_rules`: All 14 tests passed in 0.55s.
  - `python .agents/skills/attractor-discipline/scripts/lint_attractors.py`: Passed cleanly across 264 files (0 violations).
  - `cargo fmt --all -- --check`: Clean formatting across workspace.
