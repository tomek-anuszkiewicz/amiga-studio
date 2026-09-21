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
- The operational recipe, ingestion pipeline, and instructions to prepare this clean-room experiment are integrated into Section 3.5 of **[ROADMAP.md](ROADMAP.md)**.

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

### [2026-09-14 23:59 CEST] — Milestone Digest: Language Guardrails, HRM Alignment & vAmigaTS Verification Harness
- **Timestamp & Context**: 2026-09-14 — Quality assurance infrastructure, verification suites, and hardware manual alignment.
- **Affected Subsystems**: `crates/test_runner`, `.agents/rules/`, `tools/harness`, `Obsidian/Amiga/Reference/`.
- **What Was Changed (The Concrete Reality)**:
  - Codified the strict English language policy across code, documentation, and agent reasoning.
  - Ingested Commodore Hardware Reference Manuals (HRM) and Amiga Hardware Reference Manual into local reference stores.
  - Built the `vAmigaTS` headless test runner infrastructure in `crates/test_runner` supporting Tom Harte single-step CPU tests, Cartesian DMA contention suites, and RGB24 viewport golden master matchers.
  - Implemented pre-flight gate scripts (`pre_flight.py`) to enforce formatting, architectural rules, and test coupling.
- **Architectural Rationale & Trade-Offs**:
  - *Headless Test Harness:* Driving test cases via synthetic headless runners avoids dependency on host UI rendering, enabling high-speed verification (< 5s for full suites).
  - *Silicon-Exact Ground Truth:* Grounding execution against established single-step test vectors prevents subtle flag calculation divergences or bus cycle phase misalignments.
- **Verification & Invariants**:
  - `cargo test -p test_runner`: Architecture rule suite and single-step CPU validation tests.
  - `pre_flight.py`: Pre-commit validation of formatting, API coverage, and rule compliance.

---

### [2026-09-16 23:59 CEST] — Milestone Digest: RAG Knowledge Base, Diagram Sidecars & Attractor Elimination
- **Timestamp & Context**: 2026-09-16 — Offline knowledge retrieval, technical diagram processing, and codebase cleanup.
- **Affected Subsystems**: `tools/rag`, `.agents/rules/amiga-rag.md`, `.agents/rules/asset-descriptions.md`, `.agents/rules/structural-root-cause.md`.
- **What Was Changed (The Concrete Reality)**:
  - Integrated local vector-based RAG knowledge base (`tools/rag/`) indexing hardware documentation, chip schematics, and Obsidian design notes into a local Qdrant collection (`amiga`).
  - Implemented offline diagram sidecar pipeline (`<image>.txt`), producing Git-tracked technical descriptions for custom chip schematics and block diagrams.
  - Codified the Structural Root-Cause Resolution rule, strictly prohibiting surface-level coordinate nudges or ad-hoc regex patches in favor of upstream data lifecycle and timing fixes.
- **Architectural Rationale & Trade-Offs**:
  - *Local Vector Search:* Eliminates reliance on cloud APIs and enables rapid, targeted retrieval of complex Amiga hardware register timings without manual page scanning.
  - *Sidecar Markdown Representation:* Storing visual circuit schematics as searchable text sidecars allows text-based agents and search indexes to reason about physical pinouts and bus topology directly.
- **Verification & Invariants**:
  - Offline CLI retrieval tests via `tools/harness/rag_search.py`.
  - Sidecar validation in `audit_docs_quality.py`.

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

### [2026-09-20 00:03 CEST] — Enforced Strict Zero-Warning Compilation Policy Across Rust Workspace

- **Files Modified**:
  - `Cargo.toml`: Configured `warnings = "deny"` under `[workspace.lints.rust]`, with `all = { level = "allow", priority = -1 }` and `correctness = { level = "deny", priority = -1 }` under `[workspace.lints.clippy]` to preserve explicitly curated clippy gates while preventing unconfigured stylistic clippy lints from blocking compilation.
  - `.agents/rules/rust-best-practices.md`: Documented the strict zero-warning compilation policy in Section 1 (Safety & Error Discipline).
- **Architectural Rationale & Trade-Offs**:
  - *Zero-Warning Guarantee:* Denying `warnings` at compiler level (`rustc`) guarantees that any unhandled compiler warning (unused variables, unused mut, unused imports, dead code, open-bus anomalies, unformatted debug types) triggers an immediate compilation failure (`error: ... implied by -D warnings`) rather than being passively ignored.
  - *Separation of Compiler vs. Clippy Scopes:* Using `priority = -1` on clippy's default group isolates rustc's global `-D warnings` flag so that our explicitly curated clippy deny/allow list continues to govern clippy invariant checks in `pre_flight.py` without noise.
- **Verification & Test Results**:
  - `cargo check --workspace --all-targets`: Passed with 0 warnings/errors.
  - Verified active denial: Intentionally introduced unused mutable variable in `crates/config/src/config.rs` and confirmed build failure with `-D unused-mut implied by -D warnings`.
  - `cargo clippy --workspace --all-targets`: Passed with 0 errors across all 27 workspace members.
  - `cargo fmt --all -- --check`: 100% compliant.
  - `python tools/harness/pre_flight.py`: All 6 pre-flight quality gates passed cleanly (Formatting, AGENTS.md ceiling, Test Coupling, API Coverage, Clippy Invariants, Architecture Rules).
  - `python tools/harness/run_tests.py --unit`: All 23 workspace crates + 7 test_runner unit suites passed cleanly (100%).

---

### [2026-09-20 12:06 CEST] — Removed Category A & B Struct Encapsulation Rules from Rules, Skills, and Workflows

- **Files Modified**:
  - `.agents/rules/rust-best-practices.md`: Removed Category A (POD / Value Objects) and Category B (Complex Structs) boilerplate encapsulation rules and the absolute prohibition on raw public fields. Renamed Section 6 to `Method Naming & Accessor Conventions`, preserving standard getter naming (`<field>(&self)` without `get_`), boolean prefixes (`is_`/`has_`/`can_`), setter prefixes (`set_<field>`), and collection slice views (`&[T]`, `&mut [T]`).
  - `.agents/skills/audit-code-quality/SKILL.md`: Updated Pillar 5 to `Pillar 5: Method Naming & Accessor Conventions (--accessors)`, adjusted pillar count to Seven, and removed Section 5 (`Struct Encapsulation & Accessor Remediation Playbook`).
  - `.agents/workflows/audit-code-quality.md`: Updated Step 7 in Section 3 and Conscience Question 6 in Section 4 to eliminate Category A/B encapsulation and raw public field checks while maintaining method naming and accessor conventions.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of OOP Getter/Setter Boilerplate on POD Structs:* In systems programming and cycle-exact emulator design, pure data transfer objects, coordinates, audio samples, and bus states legitimately expose public fields. Forcing private fields and mechanical boilerplate getters/setters adds syntactic noise without domain benefits.
  - *Preservation of API Consistency:* Retained method naming conventions (prohibition of `get_` prefix, boolean naming, `set_` setters, and slice views over concrete containers) to ensure idiomatic Rust API ergonomics across public and crate interfaces.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: All 6 pre-flight quality gates passed cleanly (Formatting, AGENTS.md ceiling, Test Coupling, API Coverage, Clippy Invariants, Architecture Rules: 21/21 passed).
  - `cargo check --workspace --all-targets`: Passed with 0 errors/warnings.

---

### [2026-09-20 12:37 CEST] — Pruned Inference-Based Duplicates from Code Quality Workflow and Harmonized Rules & Skills

- **Files Modified**:
  - `.agents/workflows/audit-code-quality.md`: Pruned duplicate Questions 6 (Method Naming & Accessor Review) and 7 (Compiler AST & Trait Discipline Review) from Section 4 ("The Verbal Double-Check"), refocusing the conscience review strictly on 5 non-scriptable qualitative architectural heuristics. Standardized report line in Section 5 to `Method Naming & Accessor Conventions` and updated conscience review count to `5/5`.
  - `.agents/rules/file-size-and-cohesion.md`: Pruned stale reference to `tools/harness/audit_code_quality.py` from Section 4 (Line 50), confirming `crates/test_runner/tests/test_architecture_rules.rs` as the sole authority for `LINE_COUNT_EXCEPTIONS`.
  - `.agents/skills/audit-code-quality/SKILL.md`: Standardized CLI comment to `Audit method naming and accessor conventions`.
  - `tools/harness/audit_code_quality.py`: Harmonized argument parser description and summary output string to consistently use canonical title `method naming & accessor conventions`.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of Script vs. Inference Redundancy:* Method naming/accessors and compiler trait lints are already deterministically verified via mechanical tooling (`tools/harness/audit_code_quality.py --accessors`, `cargo clippy`, `pre_flight.py`). Duplicating them in the cognitive-inference verbal review added cognitive noise without providing additional defect detection.
  - *Unified Heuristic Scoping:* Aligning `/audit-code-quality` to 5 genuine qualitative conscience questions creates symmetrical parity with `/audit-docs-quality` (5/5).
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: Passed cleanly across all 6 gates (100% compliant, 21/21 architecture rules passed).
  - `python tools/harness/audit_code_quality.py --help`: Verified standardized parser description.

---

### [2026-09-20 13:02 CEST] — Remediation of Pillar 5 Method Naming and Accessor Conventions (Zero Forbidden `get_` Getters)

- **Files Modified**:
  - `crates/cpu/src/state.rs`: Renamed `get_ccr(&self)` to `ccr(&self)` (no `get_` prefix on standard getters per Rust API guidelines) and renamed condition code getters `get_x`, `get_n`, `get_z`, `get_v`, `get_c` to canonical boolean predicates `is_x`, `is_n`, `is_z`, `is_v`, `is_c`. Updated internal callers in `eval_condition(&self)`.
  - `crates/cpu/src/instructions/logic_sr_ccr.rs`: Updated `state.get_ccr()` to `state.ccr()` across ORI/ANDI/EORI to CCR handlers.
  - `crates/cpu/src/instructions/abcd.rs`, `crates/cpu/src/instructions/addx.rs`, `crates/cpu/src/instructions/chk.rs`, `crates/cpu/src/instructions/negx.rs`, `crates/cpu/src/instructions/roxl.rs`, `crates/cpu/src/instructions/roxr.rs`, `crates/cpu/src/instructions/sbcd.rs`, `crates/cpu/src/instructions/subx.rs`, `crates/cpu/src/instructions/trapv.rs`: Clean-break workspace cutover from `get_x/n/z/v` to `is_x/n/z/v`.
  - `crates/cpu/tests/test_addressing.rs`: Updated unit assertions to use `is_n/z/v/c/x` and added explicit verification for `cpu.state.ccr()`.
  - `crates/floppy/src/floppy.rs`: Renamed `get_current_track_data(&self)` to `current_track_data(&self)` and updated caller in `load_current_track_mfm()`.
  - `crates/floppy/tests/test_floppy.rs`: Added `test_floppy_track_data_and_dma` verifying track encoding and DMA stepping, maintaining test change coupling compliance.
- **Architectural Rationale & Trade-Offs**:
  - *Idiomatic Rust Accessor Alignment:* In standard Rust and `rust-best-practices.md`, simple getters do not carry the `get_` prefix (which is reserved for fallible slice/map indexing). Standard getters match the property name (`ccr()`, `current_track_data()`), and boolean getters carry `is_*` (`is_x()`, `is_z()`), while setters retain `set_*`.
  - *Clean-Break Workspace Cutover:* In accordance with `clean-break-refactoring.md`, zero legacy aliases were retained, cutting over all instruction handlers and test suites atomically.
- **Verification & Test Results**:
  - `python tools/harness/audit_code_quality.py --accessors`: Verified 0 accessor issues (Status: `[PASS]`).
  - `cargo clippy --workspace --all-targets`: Clean compilation with 0 warnings or errors.
  - `cargo test -p test_runner --test test_architecture_rules`: All 21 architecture tests passed.
  - `python tools/harness/run_tests.py --unit`: 100% pass across all 23 crates + 7 runner suites.
  - `python tools/harness/pre_flight.py`: 100% pass on formatting, change coupling, API coverage, and ceiling gates.

---

### [2026-09-20 13:35 CEST] — Clean-Break Retirement of Single Responsibility (<= 12 Public Fields) Rule Across Audit Harness & Docs

- **Files Modified**:
  - `tools/harness/audit_code_quality.py`: Removed `RE_PUB_STRUCT`, deleted `scan_srp_and_cohesion()`, removed `--srp` CLI flag, and consolidated active audit reporting into 4 canonical pillars (Dead Code & Zombies, Minimum Visibility, Condition Soup, Accessor Conventions). Cleaned up JSON export schema and summary reporting.
  - `tools/harness/audit_docs_quality.py`: Updated `REGISTERED_RULE_AUDITS` for `file-size-and-cohesion.md` to map strictly to `test_architecture_rules.rs (test_file_size_limits)` without stale auditor pillar references. Synchronized pillar numbers for `performance-and-readability.md` and `rust-best-practices.md`.
  - `.agents/skills/audit-code-quality/SKILL.md`: Removed unencapsulated "God Structs" (> 12 public fields) heuristic from Pillar 3, renumbered remaining quality pillars to 6, and removed `--srp` from CLI examples.
  - `.agents/workflows/audit-code-quality.md`: Removed `--srp` command from targeted audit section.
- **Architectural Rationale & Trade-Offs**:
  - *Silicon Hardware Modeling Reality:* In a cycle-exact emulator, custom chip structs (`Agnus`, `Blitter`, `Denise`, `Paula`, `Cia`, `AudioChannel`, `CpuMicroState`) directly model physical silicon registers, internal latches, and serializable save-state snapshots (`serde::Serialize`/`Deserialize`). Enforcing an arbitrary OOP ceiling of $\le 12$ public fields forced artificial nesting indirection, degraded hot color-clock loop performance, and triggered 16 false-alarm warnings.
  - *Module-Level Aspect Boundaries:* Single Responsibility Principle in this codebase is strictly anchored at the module and aspect level by the 800-line source file ceiling per `.agents/rules/file-size-and-cohesion.md`, mechanically verified via `cargo test -p test_runner --test test_architecture_rules`.
  - *Clean-Break Refactoring:* In accordance with `clean-break-refactoring.md` and explicit user approval, the rule and `--srp` flag were completely retired without leaving backward-compatibility shims, allowlists, or legacy residue.
- **Verification & Test Results**:
  - `python tools/harness/audit_code_quality.py --all`: 0 dead symbols, 0 zombies, 0 visibility leaks, 0 false-alarm struct cohesion issues.
  - `python tools/harness/audit_code_quality.py --all --json`: Valid JSON output confirming clean schema.
  - `cargo clippy --workspace --all-targets`: Passed with 0 warnings/errors.
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed.
  - `python tools/harness/audit_docs_quality.py --all`: 100% doc governance pass rate (0 issues).
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly.

---

### [2026-09-20 13:40 CEST] — Forward Parity Remediation: M68000 CPU Design Specifications Synchronization

- **Files Modified**:
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`:
    - Section 1.1: Added `reset_line_asserted: bool` (external bidirectional `_RESET` pin latch) and `cycle_counter: u64` (master CPU cycle accumulator) to the `CpuState` fields table. Cleaned up `a[0..=7]` description to reference full 32-bit sign-extended commitment via `write_a` / `set_a_long` (pruned ghost method `set_a_word`). Removed obsolete ghost field `step` (which resides in `state.micro.micro_step`).
    - Section 2: Replaced obsolete draft type `MemoryBusResult::Blocked` with canonical `BusResult::WaitState`.
    - Section 3.3: Enriched `CpuMicroState` field inventory with dual staging registers `addr1` and `addr2`, pre-decoded register indices `reg_src` and `reg_dst`, cached slice pointer `current_steps`, and Group 0 exception framing fields `fault_addr`, `info_word`, and `ssp_base`. Pruned ghost method alias `step_opcode` and aligned specialized bus handler catalog with canonical `step_bus_read_src_*`, `step_bus_read_addr1_*`, and `step_bus_write_addr2_*` naming.
    - Section 6: Expanded into Section 6.1, 6.2, and 6.3 documenting bidirectional `_RESET` pin dynamics, `RESET` instruction assertion (`reset_line_asserted`) signaling external device reset without resetting CPU registers or RAM, and added the System Reset Comparison Matrix.
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md`:
    - Section 2.3: Added specialized Dual Staged Operands category table documenting `step_bus_read_addr1_*`, `step_bus_read_addr2_*`, and `step_bus_write_addr2_*` bus cycle primitives.
    - Section 2.4: Synchronized `CpuMicroState` field specification by adding `fault_addr`, `info_word`, `ssp_base`, `target_refill`, and `prefetch_retired`, while pruning obsolete draft field `read_to_dest`.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of Forward Parity Blind Spots:* As revealed during `/audit-semantic-parity`, our living design specifications omitted several crucial hardware latches (`reset_line_asserted`) and execution primitives (`addr1`/`addr2` dual staging registers) that had been implemented and verified in the Rust core. Aligning the documentation ensures that future AI pair-programming agents and human architects have complete visibility into the cycle-exact execution model.
  - *Elimination of Ghost Residue:* Pruning non-existent accessor methods (`set_a_word`), obsolete bus types (`MemoryBusResult::Blocked`), and ghost aliases (`step_opcode`) prevents speculative coding and maintains absolute fidelity between specifications and code.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py --all`: 100% pass across all 10 pillars (0 issues detected).
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed cleanly.
  - `python tools/harness/pre_flight.py`: 100% compliant across formatting, AGENTS.md ceiling, API coverage, Clippy invariants, and architecture tests.

---

### [2026-09-20 13:45 CEST] — Reverse Parity Remediation: Pruning Ghost Features in M68000 CPU Specifications

- **Files Modified**:
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`:
    - Section 1.1: Pruned non-existent CCR setter methods `set_ccr_raw` and `set_ccr_nzc_clear_v`; aligned list to active methods in `crates/cpu/src/state.rs` (`set_ccr`, `set_ccr_xnzvc`, `set_ccr_nzvc`, `set_ccr_nz_clear_vc`, `set_ccr_z_only`, `set_ccr_v_clear_c`).
    - Section 3.3: Pruned speculative struct `TargetRefill { target, new_ir }` and field `scratch_prefetch`; aligned to live architecture where target opcodes are captured directly into `state.micro.irc` with flag `state.micro.target_refill = true`, committed to `cpu.state.ir` on retirement via `retire_current_instruction()`.
    - Section 7.5: Pruned ghost functions `cpu.initiate_prefetch()` and `mark_standard_prefetch_retire()`; replaced with `common::PREFETCH_NEXT_READ`, `common::BUS_READ_IDLE`, and `retire_current_instruction()`. Replaced legacy "internal scratch" and "scratch prefetch latch" with canonical `state.micro.destination` and `state.micro.irc`.
    - Section 7.12 & 7.13: Replaced obsolete `internal scratch register` and `cpu.scratch` references with canonical micro-state registers `state.micro.destination` and `state.micro.source`.
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md`:
    - Section 1:
      - Invariant 4: Corrected `prefetch: [u16; 2]` to scalar `prefetch: u16` (`CpuState`) and `irc: u16` (`CpuMicroState`), matching real hardware registers `IR`, `IRD`, and `IRC`.
      - Invariant 5: Removed non-existent field `last_read`; clarified operand arrival into `CpuMicroState` registers (`source`, `destination`, `addr1`, `addr2`).
      - Invariant 7: Replaced non-existent `state.micro.write_buffer: u32` with canonical `state.micro.destination: u32`.
      - Invariant 12: Replaced `scratch[0]` with canonical `state.micro.movem_mask`.
      - Invariant 13: Replaced obsolete names `EXCEPTION_GROUP0_STEPS` / `EXCEPTION_GROUP1_STEPS` with actual static slice arrays `STEPS_ADDRESS_ERROR`, `STEPS_INTERRUPT`, `STEPS_PRIVILEGE_VIOLATION`, `STEPS_ZERO_DIVIDE`.
    - Section 2.3: Replaced non-existent pseudo-functions `step_write_word_at`, `step_write_byte_at`, `step_read_word_at`, `step_read_byte_at` with actual execution primitives: `ALU callbacks (AluFn)`, `ALU_IDLE_*`, `step_bus_write_trap_*`, `step_bus_write_aerr_*`, and `step_bus_read_vector_*`.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of Ghost Residue & Speculative Pseudo-Code:* Speculative abstractions and draft function signatures that never materialized in the implementation create confusion, mislead agents into inventing non-existent APIs, and degrade code navigation. Bringing specifications into 100% bidirectional parity with the underlying Rust implementation establishes a single authoritative source of truth.
  - *Direct Latch & Dual Staging Fidelity:* Documenting canonical micro-state registers (`source`, `destination`, `irc`, `movem_mask`) instead of vague "scratch buffers" reinforces the physical electronic causality modeled by the emulator core.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py --all`: 100% pass rate across all 10 documentation quality pillars (0 issues).
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, API coverage, Clippy, architecture rules).
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed.

---

### [2026-09-20 13:51 CEST] — Semantic Parity Audit & Remediation: M68000 CPU Specifications

- **Files Modified**:
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`:
    - Frontmatter & Header: Corrected `Module Location` from legacy `m68000/` to canonical `crates/cpu/`.
    - Section 3.3: Corrected ghost type signature `StepFn = fn(&mut Cpu, &mut MemoryBus) -> BusResult<()>` to canonical `BusFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>`, documenting architectural decoupling from concrete `MemoryBus`. Updated method signatures (`step_cck`, `step_cck_internal`, `step_instruction`) to accept `&mut dyn AddressBus`.
    - Section 3.3: Documented post-deserialization lifecycle method `rehydrate_micro_steps(&mut self)` and debugger/test harness helper `set_pc_and_prime_prefetch(&mut self, target_pc: u32, bus: &mut dyn AddressBus)`.
    - Section 5: Added Section 5.2 documenting Privilege Violation (`STEPS_PRIVILEGE_VIOLATION`, Vector 8, 34 CPU clocks / 17 CCKs) and Section 5.3 documenting Divide-by-Zero (`STEPS_DIV_ZERO`, Vector 5, 38 CPU clocks). Added Section 5.4 documenting MC68000 silicon Double Bus Fault on odd initial Program Counter (`pc & 1 != 0`).
    - Section 6.1: Aligned cold/warm reset sequence Step 4 (documenting Double Bus Fault on odd PC) and Step 5 (documenting lookahead prefetch register latching into `state.prefetch` rather than `irc`).
    - Frontmatter: Bumped `last_synced_commit` to current verified checkpoint.
- **Architectural Rationale & Trade-Offs**:
  - *Zero Ghost Residue & Trait Decoupling:* Replacing obsolete `StepFn` and concrete `MemoryBus` references with `BusFn` and `dyn AddressBus` reinforces the repository's strict bus topology and decoupling invariants.
  - *Silicon Exception Fidelity:* Documenting the exact microcode pipelines for Privilege Violation (34 clocks), Divide-by-Zero (38 clocks), and reset-phase Double Bus Faults ensures that the CPU specification matches Tom Harte silicon ground truth and live Rust execution models.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py --all`: 100% pass across all 10 documentation quality pillars (0 issues, 26/26 specs in sync).
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, API coverage, Clippy, architecture rules).
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed.

---

### [2026-09-20 14:02 CEST] — Semantic Parity Audit & Remediation: CPU Micro-Step State Machine Specifications

- **Files Modified**:
  - `Obsidian/Amiga/Design/CPU Micro-Step State Machine.md`:
    - Section 1–8: Eliminated all ghost prototype identifiers (`write_buffer` -> `destination`, `last_read` -> `source`, `scratch_prefetch` -> `irc`, `scratch[0]` -> `movem_mask`, `PrefetchNextOpcodeAndRetire` -> `PREFETCH_NEXT_READ & retire_current_instruction()`).
    - Section 1.1: Replaced references to obsolete `step_read_word_at`/`step_write_word_at` with direct 2-phase CCK read/write transactions (`step_bus_read_src_word`, `step_bus_write_dst_word`).
    - Section 2.6: Removed non-existent predecrement long read constants (`READ_ADDR1_PD_LONG_*`, `READ_ADDR2_PD_LONG_*`).
    - Section 3: Replaced ghost type signature `StepFn` with `BusFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>`, decoupling the state machine from concrete `MemoryBus`.
    - Section 6.1: Removed non-existent `step_opcode` alias; specified canonical `step_instruction(&mut self, bus: &mut dyn AddressBus) -> u32`.
    - Section 7: Completely overhauled archetypal micro-step traces across Classes 1 to 11 (`ADD`, `ORI`, `MOVE.W`, `MOVE.L`, `ADD` RMW, `ADDI` RMW, `JSR`, `RTS`, `JMP`, `Bcc`, `DIVU`, `MOVEM`), mapping every operation to canonical 2-clock CCK micro-steps (`MicroStep` / `common::*`) with fused `alu_fn` and exact register names.
    - Frontmatter: Bumped `last_synced_commit` to current verified commit checkpoint (`cff10c82a3`).
- **Architectural Rationale & Trade-Offs**:
  - *Full Microcode Parity:* Synchronizing the microcode specification with the live Rust execution engine in `crates/cpu/src/micro/` guarantees that architects and AI pair programmers work from an exact 1:1 reflection of physical 68000 micro-operations without ghost artifacts from early prototypes.
  - *Fused CCK Model Cohesion:* Aligning Classes 1 through 11 with the 2-clock Color Clock phase model (`BUS_READ_IDLE`, `BUS_WRITE_IDLE`, `FETCH_EXT_READ`, `PREFETCH_IRC_READ`) eliminates contradictions between the high-level description and low-level step tables.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py --all`: 100% pass across all 10 documentation quality pillars (0 issues, 26/26 specs in sync).
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, API coverage, Clippy, architecture rules).
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed.

---

### [2026-09-20 14:15 CEST] — Consolidated M68000 Silicon Quirks Catalog & Streamlined CPU Specifications

- **Files Modified**:
  - `Obsidian/Amiga/Design/Platform Quirks and Invariants Catalog.md`:
    - Section 2: Added `Provenance & Source` classification column with 3 explicit tiers: Category A (Motorola PRM/UM), Category B (Micro-Bus Specs), Category C (Silicon Reality).
    - Section 2: Corrected line 51 erratum erroneously categorizing `CMPA` as CCR-immune (removed `CMPA` and clarified that address modifications `MOVEA`/`ADDA`/`SUBA`/`ADDQ`/`SUBQ` leave CCR flags untouched while `CMPA` compares and updates $N, Z, V, C$).
    - Section 2: Enriched the table with 3 additional silicon quirks from the live CPU implementation:
      1. Post-Increment `(An)+` Address Error AGU Register Commitment asymmetry (reads commit $An \leftarrow An + \text{inc}$; writes do not).
      2. `ASR` Count $\ge$ Width Silicon Exhaustion ($C = 0, X = 0$).
      3. `MOVE` to Predecrement `-(An)` Prefetch Inversion & Bus Ordering (prefetch precedes write on Byte/Word stores).
    - Frontmatter: Bumped `updated` to `2026-09-20`.
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`:
    - Frontmatter: Added `Platform Quirks and Invariants Catalog.md` to `related:` and bumped `updated` to `2026-09-20`.
    - Section 7: Added a prominent note callout linking to the centralized Platform Quirks Catalog.
    - Section 7.5: Replaced verbose silicon exhaustion text with a concise pointer to Platform Quirks Catalog.
    - Section 7.6: Streamlined Address Error specification to focus on architectural Program/Data Space selection ($FC = 2/6$ vs $1/5$), delegating physical AGU register commitment and predecrement prefetch inversion to Platform Quirks Catalog.
    - Section 7: Removed redundant sub-sections 7.7 (`ASR`), 7.8 (`MOVE -(An)`), and 7.11 (`Post-Increment (An)+`), eliminating content duplication across specifications per Information Hierarchy guidelines. Renumbered remaining sections cleanly to 7.7–7.12.
- **Architectural Rationale & Trade-Offs**:
  - *Single Source of Truth for Silicon Quirks:* Housing detailed silicon anomalies in [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md) while keeping subsystem design documents focused on their primary architectural responsibilities enforces strict conceptual boundaries and eliminates out-of-sync duplicate sprawl.
  - *Provenance Transparency:* Explicitly tagging each quirk with its provenance (official PRM vs. bus cycle sheets vs. reverse-engineered silicon) helps architects and AI pair programmers distinguish intentional architectural invariants from physical silicon errata.
- **Verification & Test Results**:
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, API coverage, Clippy, architecture rules).
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architecture tests passed (including Obsidian design docs link integrity check).

---

### [2026-09-20 14:18 CEST] — Remediated Ghost Identifiers in Class 0 RMW & Clarified Fast RAM TAS Erratum

- **Files Modified**:
  - `Obsidian/Amiga/Design/Platform Quirks and Invariants Catalog.md`:
    - Section 2: Updated `Class 0 RMW Prefetch Order` implementation invariant to state that the microcode pipeline initiates `PREFETCH_IRC_READ` & `PREFETCH_IRC_FINISH` into `state.micro.irc` before executing `WRITE_DST_*`, completely eliminating legacy prototype names `BusPrefetchToScratch` and `BusWrite`.
    - Section 3: Clarified `TAS Read-Modify-Write Silicon Erratum` to specify that `TAS` functions correctly and updates bit 7 only in Fast RAM (`$200000-$9FFFFF`); on Chip RAM (`$000000-$07FFFF`) and Slow RAM (`$C00000-$C7FFFF`), the write phase is dropped by Gary/Agnus, updating CCR while leaving memory bit 7 unmodified.
  - `tools/harness/audit_docs_quality.py`:
    - Updated `quirk_signatures` to accept `PREFETCH_IRC` for `Class 0 RMW Prefetch Order`.
    - Synchronized quirk signature validators for renamed `Address Register Direct CCR Immunity & Sign Extension` and 3 newly consolidated silicon quirks (`Post-Increment (An)+`, `ASR Count >= Width`, `MOVE to Predecrement -(An)`), achieving 18/18 (100%) regression test sentinel coverage.
- **Architectural Rationale & Trade-Offs**:
  - *Unified Microcode Vocabulary:* Eliminating outdated prototype names (`BusPrefetchToScratch`) ensures that the quirks catalog uses the identical canonical vocabulary (`MicroStep`, `common::*`) established across the Rust CPU core and design specs.
  - *A500 Memory Map Disambiguation:* Explicitly differentiating Fast RAM (where unbroken RMW succeeds) from Chip/Slow RAM (where Gary/Agnus suppresses the write) ensures accurate physical Amiga 500 circuit understanding.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py --all`: 100% pass across all 10 pillars (18/18 silicon quirks verified with test sentinels).
  - `python tools/harness/pre_flight.py`: 100% compliant across formatting, AGENTS.md limits, Clippy, and architecture tests (21/21 passed).

---

### [2026-09-20 14:50 CEST] — Consolidated CPU State Restoration: Introduced `Cpu::restore_state` & Purged `rehydrate_micro_steps`

- **Files Modified**:
  - `crates/cpu/src/cpu.rs`: Added `restore_state(&mut self, state: CpuState)` unifying state assignment with opcode dispatch table micro-step slice rehydration; permanently removed standalone `rehydrate_micro_steps(&mut self)`.
  - `crates/cpu/tests/test_visibility.rs`: Updated rehydration integration test to verify `cpu.restore_state(state)` and micro-step slice repopulation.
  - `crates/machine_loop/src/machine_loop.rs`: Consolidated two-step `cpu.state = ...; cpu.rehydrate_micro_steps();` into single atomic call `self.cpu.restore_state(state.cpu.clone())` in `A500Machine::load_state()`.
  - `crates/machine_loop/tests/test_save_state.rs`: Added regression assertion verifying micro-step slice rehydration post-restore.
  - `crates/debugger/src/session.rs`: Migrated `scrub_to_frame` and `jump_to_live_head` to use `cpu.restore_state()`.
  - `crates/debugger/tests/test_stepping_and_session.rs`: Added assertions verifying CPU micro-step rehydration on temporal scrubbing and return to live head.
  - `crates/gui/src/layout/left_dock/disassembly.rs`: Migrated historical pass rewind points to use `cpu.restore_state()`.
  - `crates/gui/tests/test_interactions.rs`: Added assertions verifying micro-step slice rehydration during time-travel navigation.
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`: Synchronized Section 3.3 public API listing to document `restore_state(&mut self, state: CpuState)`.
- **Architectural Rationale & Trade-Offs**:
  - *Atomic State Restoration:* State restoration previously required two decoupled calls (`self.cpu.state = state.cpu.clone(); self.cpu.rehydrate_micro_steps();`). Failing to call `rehydrate_micro_steps` left `current_steps` pointing to empty execution slices, creating subtle runtime bugs. Encapsulating state restoration and microcode pointer rehydration into `Cpu::restore_state()` provides complete atomic safety.
  - *Clean-Break Refactoring:* In accordance with [.agents/rules/clean-break-refactoring.md](.agents/rules/clean-break-refactoring.md), `rehydrate_micro_steps` was removed without backwards-compatibility shims or legacy aliases.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: Passed.
  - `cargo test -p cpu --test test_visibility`: 3/3 passed.
  - `cargo test -p machine_loop --test test_save_state`: 8/8 passed.
  - `cargo test -p debugger --test test_debugger_save_state`: 4/4 passed.
  - `cargo test -p gui --test test_interactions`: 38/38 passed.
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 passed.
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, test coupling, API coverage, Clippy, architecture rules).

---

### [2026-09-20 15:05 CEST] — Pruned Bulk Register Mutators (`set_d_regs`, `set_a_regs`) in Favor of Canonical Register-by-Register Operations

- **Files Modified**:
  - `crates/cpu/src/state.rs`: Pruned `set_d_regs(&mut self, [u32; 8])` and `set_a_regs(&mut self, [u32; 8])` bulk mutator methods; updated header to `Full Array Accessors (Test Harness & State Snapshots)`.
  - `crates/cpu/tests/test_visibility.rs`: Added `test_register_by_register_initialization` verifying register-by-register mutation of $D_0-D_7$ and $A_0-A_7$ via `set_d_long` and `set_a_long` across all indices $0..=7$.
  - `crates/test_runner/src/runner.rs`: Replaced bulk setter calls with unrolled `set_d_long(0..=7, ...)` and `set_a_long(0..=7, ...)`, eliminating intermediate stack array construction.
  - `crates/test_runner/src/dma_harness.rs`: Replaced bulk setter calls with unrolled `set_d_long(0..=7, ...)` and `set_a_long(0..=7, ...)`.
  - `crates/test_runner/src/benchmark/builder.rs`: Replaced bulk setter calls with register-by-register iteration over $D_0-D_7$ and $A_0-A_7$.
  - `crates/test_runner/tests/test_builder.rs`: Added assertions verifying injected register states for $D_0-D_7$ and $A_0-A_7$.
- **Architectural Rationale & Trade-Offs**:
  - *Elimination of Redundant API Surface:* `set_d_regs` and `set_a_regs` were only used in test harnesses and required allocating temporary arrays `[test.initial.d0, ...]` on the stack before copying them into `state.d` and `state.a`. Mutating registers directly via canonical `set_d_long` and `set_a_long` methods is more direct and eliminates redundant methods.
  - *Clean-Break Refactoring:* In accordance with [.agents/rules/clean-break-refactoring.md](.agents/rules/clean-break-refactoring.md), the bulk setters were pruned workspace-wide with zero deprecated aliases or shims.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: Passed.
  - `cargo test -p cpu`: 53/53 tests passed.
  - `cargo test -p test_runner --test test_builder`: 6/6 passed.
  - `cargo test -p test_runner --test test_singlestep`: 127/127 passed.
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates passed cleanly (formatting, AGENTS.md ceiling, test coupling, API coverage, Clippy, architecture rules).


---

### [2026-09-20 15:06 CEST] — Reconciliation of Audit Workflows, Compiler/Clippy Gates, and Governance Rules
- **Affected Subsystems**:
  - `tools/harness/audit_hardware_quality.py`
  - `.agents/workflows/audit-*.md`
  - `.agents/rules/docs-maintenance.md`
  - `.agents/rules/unit-testing-policy.md`
- **What Was Changed (The Concrete Reality)**:
  - Pruned redundant unwrap and transmute regex checks from audit_hardware_quality.py
  - Removed orphaned Struct Cohesion (> 12 fields) metric from audit-code-quality.md
  - Removed duplicate test_architecture_rules runs from workflows
  - Codified Two-Way Script Locality and Skills Catalog sync in docs-maintenance.md
  - Codified Test-Only Zombie triage in unit-testing-policy.md
- **Architectural Rationale & Trade-Offs**:
  - Establishes a clean division of labor between compiler-grade gates (Tier 0)
  - automated architecture tests (Tier 1)
  - and specialized audits (Tier 2)
  - eliminating duplicate passes and aligning rules with audit checks.
- **Verification & Test Results**:
  - pre_flight.py passed
  - audit_hardware_quality.py passed with 0 issues
  - audit_code_quality.py passed with 0 issues
  - audit_docs_quality.py passed with 0 issues
  - test_architecture_rules passed 21/21 tests.

---

### [2026-09-20 15:25 CEST] — Refactored `CpuState` Register Accessors to Canonical `d_long`/`set_d_long` and `a_long`/`set_a_long`

- **Files Modified**:
  - `crates/cpu/src/state.rs`: Permanently pruned bulk slice getters `d_regs()` and `a_regs()`, and legacy helpers `read_a()` and `write_a()`. Implemented direct canonical single-register accessors `a_long(&self, reg: usize) -> u32` and `set_a_long(&mut self, reg: usize, val: u32)`. Updated `a_word` to delegate to `self.a_long(reg) as u16`.
  - `crates/cpu/src/instructions/*` (`add.rs`, `adda.rs`, `addq.rs`, `cmp.rs`, `cmpa.rs`, `jsr.rs`, `lea.rs`, `move_b.rs`, `move_l.rs`, `move_usp.rs`, `move_w.rs`, `movea.rs`, `movem.rs`, `sub.rs`, `suba.rs`, `subq.rs`): Migrated all internal address register accesses to canonical `a_long()` and `set_a_long()`.
  - `crates/cpu/src/micro/*` (`common.rs`, `ea.rs`, `step_control.rs`, `step_execution.rs`): Replaced all `read_a` and `write_a` invocations with `a_long` and `set_a_long`.
  - `crates/cpu/tests/*`: Updated `test_programs.rs`, `test_visibility.rs`, `test_addressing.rs`, `test_interrupts.rs`, and `test_micro_archetypes.rs` to use canonical single-register accessors.
  - `crates/debugger/src/loader.rs` & `crates/debugger/tests/*`: Migrated SP zero-checks and assertions from `a_regs()[7]` to `a_long(7)` and trace reads to `a_long()`.
  - `crates/gui/src/layout/left_dock/registers.rs` & `crates/gui/tests/test_interactions.rs`: Updated data and address register grids to read values and evaluate diff highlights via `state.d_long(i)` and `state.a_long(i)`.
  - `crates/machine_loop/tests/*`: Updated `test_reset.rs` zero-register checks to loop over `d_long(i)` and `a_long(i)`. Corrected `test_save_state.rs` deterministic stepping roundtrip to map chip RAM and execute active NOP sequence.
  - `crates/test_runner/src/*`: Migrated `runner.rs`, `dma_harness.rs`, and benchmark `runner.rs`/`tracer.rs` to use `d_long` and `a_long`. Updated `test_builder.rs` to verify canonical accessors.
  - `Obsidian/Amiga/Design/CPU Motorola M68000.md`, `GUI Specification.md`, and `.agents/skills/add-m68k-instruction/SKILL.md`: Updated documentation and instruction implementation recipes to reflect canonical `d_long`/`set_d_long` and `a_long`/`set_a_long` conventions.
- **Architectural Rationale & Trade-Offs**:
  - *Clean-Break Refactoring & API Orthogonality:* Symmetrical naming across data and address registers (`d_long`/`set_d_long`, `a_long`/`set_a_long`) eliminates confusing aliases (`read_a` vs `a_long`, `write_a` vs `set_a_long`). Removing bulk array slice views (`d_regs`, `a_regs`) prevents internal representation leaks and forces all consumers to access registers through encapsulated, single-register methods with bounded index access.
  - *Zero Legacy Shims:* All callers across the workspace cut over in a single atomic pass, adhering strictly to the Clean-Break Refactoring rule.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: 100% compliant.
  - `python tools/harness/pre_flight.py`: All pre-flight quality gates PASSED (formatting, AGENTS.md ceiling <= 14KB, test coupling, API coverage 100%, Clippy, architecture rules 21/21).
  - `cargo test -p cpu`: 57/57 unit tests passed.
  - `cargo test -p debugger`: 43/43 unit tests passed.
  - `cargo test -p gui`: 50/50 headless integration tests passed.
  - `cargo test -p machine_loop`: 70/70 integration tests passed.
  - `cargo test -p test_runner --test test_builder`: 7/7 tests passed.
  - `python tools/harness/run_tests.py --unit`: Tier 1 passed (23 crates + 7 test_runner unit suites).
  - `python tools/harness/run_tests.py --integration`: Tier 2 passed (memory_bus, machine_loop, debugger, gui).
  - `python tools/harness/audit_code_quality.py --all`: 0 dead symbols, 0 zombies, 0 visibility leaks, 0 method naming issues.
  - `python tools/harness/audit_docs_quality.py`: 10/10 pillars PASSED (0 issues).

### 2026-09-20: Encapsulation of `CpuState` Stack Pointers (`usp`, `ssp`), Status Register (`sr`), and Bidirectional $A_7$ Synchronization
- **Scope & Affected Subsystems**: `crates/cpu`, `crates/debugger`, `crates/gui`, `crates/machine_loop`, `crates/test_runner`.
- **Key Modifications**:
  - `crates/cpu/src/state.rs`: Encapsulated `usp`, `ssp`, and `sr` as private fields. Made active $A_7$ (`a[7]`) the single source of truth for the currently selected mode ($S=1 \implies SSP$, $S=0 \implies USP$).
  - `crates/cpu/src/state.rs`: Updated `set_a_long(7, val)` to automatically synchronize the corresponding inactive bank (`ssp` if $S=1$ or `usp` if $S=0$), guaranteeing that modifying $A_7$ never leaves stack pointer mirrors stale. Added `a7(&self) -> u32` and `set_a7(&mut self, val: u32)`.
  - `crates/cpu/src/state.rs`: Added encapsulated accessors `usp(&self) -> u32`, `set_usp(&mut self, val: u32)`, `ssp(&self) -> u32`, `set_ssp(&mut self, val: u32)`. When modifying the currently active mode's stack pointer via `set_ssp` or `set_usp`, active $A_7$ is updated synchronously.
  - `crates/cpu/src/state.rs`: Added `sr(&self) -> u16`, `set_sr(&mut self, val: u16)`, `set_supervisor(&mut self, supervisor: bool)`, and trace bit helpers (`is_trace(&self) -> bool`, `set_trace(&mut self, trace: bool)`, `clear_trace(&mut self)`).
  - `crates/cpu/src/state.rs`: Unified supervisor mode transitions in `update_supervisor_mode(&mut self, new_s: bool)`. When the $S$ bit transitions, active $A_7$ and the stored inactive bank are atomically exchanged, eliminating desynchronization.
  - `crates/cpu/src/state.rs`: Updated `clear_registers()` to clear `d`, `a`, `usp`, and `ssp`. Purged obsolete `sync_stack_pointers()` across the entire repository.
  - `crates/cpu/src/instructions/*` & `crates/cpu/src/micro/common.rs`: Updated `move_usp.rs`, `logic_sr_ccr.rs`, `move_sr_ccr.rs`, `chk.rs`, `trap.rs`, `trapv.rs`, and micro-operation exception helpers to use domain methods (`set_usp`, `usp`, `sr`, `clear_trace`).
  - `crates/cpu/tests/test_state.rs`: Created dedicated unit test suite covering stack pointer synchronization in supervisor and user modes, atomic mode swaps, `SR_MASK` enforcement, trace bit helpers, `clear_registers`, and Serde roundtrip.
  - `crates/debugger/*`, `crates/gui/*`, `crates/machine_loop/tests/*`, and `crates/test_runner/*`: Migrated all direct field accesses on `.sr`, `.ssp`, `.usp` to encapsulated methods.
- **Architectural Rationale & Trade-Offs**:
  - *Structural Root Cause Resolution:* Eliminated ad-hoc calls to `sync_stack_pointers()`. Making fields private and unifying transitions under `update_supervisor_mode` makes it impossible for external callers, instructions, or test harnesses to desynchronize $A_7$, $USP$, and $SSP$.
  - *Clean-Break Refactoring:* Removed `sync_stack_pointers()` without legacy shims, executing an immediate full-workspace cutover.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: 100% compliant.
  - `python tools/harness/pre_flight.py`: All Pre-Flight Quality Gates PASSED (formatting, AGENTS.md ceiling <= 14KB, test coupling, API coverage 100%, Clippy, architecture rules 21/21).
  - `python tools/harness/run_tests.py --all`: Tier 1 (23 crates + 7 test_runner suites) and Tier 2 (memory_bus, machine_loop, debugger, gui) PASSED.
  - `python tools/harness/run_tests.py --harness`: Tier 3 (Cartesian DMA contention 20/20, architecture rules 21/21, benchmark smoke) PASSED.
  - `cargo test -p test_runner --test test_singlestep`: 127/127 SingleStep silicon test suites PASSED.

---

### [2026-09-20 16:20 CEST] — Streamlined `CpuState` Stack Pointer API: Canonical `a_long(7)`, Invariant Field Returns (`usp`/`ssp`), and Unified `set_supervisor`

- **Files Modified**:
  - `crates/cpu/src/state.rs`: Purged redundant `a7()` and `set_a7()` methods in favor of canonical `a_long(7)` and `set_a_long(7, val)`. Simplified `usp(&self) -> u32` and `ssp(&self) -> u32` to return `self.usp` and `self.ssp` directly with zero runtime branching, leveraging the invariant that `set_a_long` keeps active and inactive stack pointers synchronized at all times. Consolidated mode transition logic into `set_supervisor(&mut self, supervisor: bool)` and eliminated `update_supervisor_mode`, pruning redundant stack pointer copies during privilege level swaps.
  - `crates/cpu/tests/test_state.rs`: Migrated all test assertions and mutations from `a7()` / `set_a7(...)` to `a_long(7)` and `set_a_long(7, val)`.
  - `crates/debugger/src/loader.rs` & `crates/debugger/tests/test_loader.rs`: Updated binary loader SP zero-check and initialization from `cpu.state.a7()` / `set_a7()` to canonical `a_long(7)` and `set_a_long(7, val)`, adding test verification for non-zero pre-existing SP preservation.
- **Architectural Rationale & Trade-Offs**:
  - *Orthogonal API & Minimal Surface:* Eliminates redundant convenience aliases (`a7`/`set_a7`) in favor of uniform single-register accessors (`a_long`/`set_a_long`).
  - *Invariant-Driven Simplification:* Because `set_a_long(7, val)` updates the respective `usp` or `ssp` backing field on every write and `set_supervisor` swaps active `a[7]`, `self.usp` and `self.ssp` are permanently authoritative, allowing the getters to return stored values unconditionally without evaluating `SR_S`.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: 100% compliant.
  - `cargo test -p cpu`: 7/7 unit tests in `test_state.rs` and 64/64 total CPU tests passed.
  - `cargo test -p debugger`: 43/43 unit tests passed.
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architectural tests passed.

---

### [2026-09-21 10:20 CEST] — Codified Strict Scope Discipline & Harmonized Root-Cause Resolution

- **Files Modified / Added**:
  - `.agents/rules/strict-scope-discipline.md`: [NEW] Codified universal operational invariant enforcing strict task containment, the minimal necessary diff principle, prohibition of unsolicited drive-by refactoring or sibling defect fixes, and mandatory turn completion reporting (Delivered Changes vs Observed Opportunities & Future Recommendations).
  - `.agents/rules/structural-root-cause.md`: Clarified Section 3 ("Assume Systematic Scope") to explicitly decouple causal mechanism fidelity (forbidding coordinate nudges, ad-hoc regexes, and symptom masking) from spatial task scope expansion (forbidding unsolicited rewrites of adjacent working opcodes/modules).
  - `AGENTS.md`: Registered `strict-scope-discipline.md` in Section 1.A Universal Invariants, with tight description maintenance to remain safely under the constitutional 14,000-byte ceiling (13,832 bytes).
  - `docs/ai_agents.md`: Registered `strict-scope-discipline.md` in Section 1 ("Architectural Guardrails & Invariants").
  - `crates/test_runner/tests/test_architecture_rules.rs`: Registered `strict-scope-discipline.md` in `registered_rules` in `test_all_rules_audited_in_quality_harness`.
  - `tools/harness/audit_docs_quality.py`: Registered `strict-scope-discipline.md` in `PASSIVE_INVARIANT_RULES` and `REGISTERED_RULE_AUDITS`.
- **Architectural Rationale & Trade-Offs**:
  - *Anti-Scope Creep Guardrail:* Eliminates LLM agent tendencies to expand tasks, perform drive-by cleanups, or bundle unrequested refactorings into discrete prompt requests.
  - *Decoupled Causal Fidelity from Task Perimeter:* Guarantees hardware fidelity without allowing the agent to unilaterally rewrite sibling instructions or modules.
  - *Transparent Governance:* Mandates structured end-of-turn reporting distinguishing delivered tasks from suggested future opportunities.
- **Verification & Test Results**:
  - `cargo fmt --all -- --check`: 100% compliant.
  - `cargo test -p test_runner --test test_architecture_rules`: 21/21 architectural tests passed.
  - `python tools/harness/audit_docs_quality.py`: 10/10 pillars PASSED (30/30 rules audited, 0 issues).
  - `python tools/harness/pre_flight.py`: All Pre-Flight Quality Gates PASSED (formatting, AGENTS.md 13,832 B <= 14KB ceiling, test coupling, API coverage 100%, Clippy, architecture rules 21/21).

---

### [2026-09-21 10:35 CEST] — Restructured ROADMAP.md into a Standardized Executable Nested Hierarchy

- **Files Modified**:
  - `ROADMAP.md`: Restructured the entire roadmap into a standardized, hierarchical nested list format. Standardized Section 1 by incorporating the delivered vAmigaTS Test Runner & Verification Infrastructure (v1.0) into Completed Baseline Deliverables. Formulated the explicit *Agent Execution & Step Completion Protocol (Mandatory Contract)* at the top of Section 2. Standardized every executable sub-step across Steps 1–7, Section 3.5, and Section 4.1 into a 3-tier schema (`Objective`, `Actionable Scope`, `Verification Gate`). Corrected all out-of-sync numbering (`Step 3.x` under Step 4, `Step 4.x` under Step 5, etc.) into contiguous indices.
- **Architectural Rationale & Trade-Offs**:
  - *Unambiguous Agent Actionability:* Eliminates mixed formatting styles (ad-hoc bullet lists, numbered paragraphs, mismatched prefixes) so autonomous AI agents clearly recognize the achievement of each milestone and understand the exact 5-step post-completion protocol (pass gates, log in DIARY.md, prune ROADMAP.md, update baseline, renumber contiguously).
  - *Context Hygiene & Zero Retention:* Enforces `.agents/rules/roadmap-maintenance.md` by moving completed infrastructure to Section 1 baseline deliverables and keeping Section 2 strictly as a forward-looking active backlog.
- **Verification & Test Results**:
  - `python tools/harness/audit_docs_quality.py`: 10/10 pillars PASSED (Pillar 10 Roadmap Zero Retention confirmed, 0 issues).
  - `python tools/harness/pre_flight.py`: All Pre-Flight Quality Gates PASSED (formatting, AGENTS.md 13,832 B <= 14,000 ceiling, test coupling, API coverage 100%, Clippy, architecture rules 21/21).




