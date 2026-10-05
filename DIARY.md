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

### [2026-09-15 23:59 CEST] — Milestone Digest: Custom Chipset Silicon Calibration & Phase 1 vAmigaTS Verification
- **Timestamp & Context**: 2026-09-14 to 2026-09-15 — Hardware circuit timing calibration across Agnus (Copper/Blitter), Denise display pipeline, and whole-machine integration testing.
- **Affected Subsystems**: `crates/copper`, `crates/blitter`, `crates/denise`, `crates/machine_loop`, `crates/test_runner/src/vamiga`.
- **What Was Changed (The Concrete Reality)**:
  - Calibrated Agnus Copper execution timing: 1-CCK instruction fetch pipeline, comparator race conditions against beam counters, `CDANG` danger mode handling, and cycle `$E0` DMA denial in PAL display scanlines.
  - Fully implemented Blitter HRM Table 6.2 decomposition: 256-minterm Boolean ALU, barrel shifters, modulo pointer advancement, unconnected channel latch calibration, and area fill logic with 100% pass on Blitter regression suites.
  - Calibrated Denise pixel serializer and display window flip-flops (`DIWSTRT`/`DIWSTOP`), aligning pipeline latency and bitplane shifter arming.
  - Implemented the 4-iteration whole-machine cascading integration verification framework in `crates/machine_loop/tests/`, testing Copper, Blitter, Denise, and interrupt cascades without external GUI overhead.
- **Architectural Rationale & Trade-Offs**:
  - *Silicon-Exact Comparator Matching:* Rather than using abstract programmatic coordinates, Copper beam comparisons evaluate on exact physical clock phases, mirroring Agnus comparator silicon.
  - *Table 6.2 Truth Table Decomposition:* Modeling Blitter logic as a pure 256-entry lookup table avoids branch cascades and executes at peak throughput on modern host CPU architectures.
- **Verification & Invariants**:
  - 100% pass on vAmigaTS Blitter area fill and line drawer tests.
  - Full suite of Tier 2 whole-machine integration tests passing in `crates/machine_loop/tests/`.

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

### [2026-09-29 01:50 CEST] — Historical Section 10 Compaction into 10 Architectural Milestone Digests
- **Affected Subsystems**:
  - `DIARY.md` (Section 10 synthesized per `.agents/skills/compact-diary/SKILL.md`)
  - `.agents/rules/diary-maintenance.md`, `.agents/rules/roadmap-maintenance.md`, `.agents/skills/compact-diary/SKILL.md`
- **What Was Changed (The Concrete Reality)**:
  - Audited and compacted 340+ historical chronological entries in Section 10 (September 12–22, 2026) spanning over 8,500 lines into 10 high-signal Architectural Milestone Digests covering all major project phases:
    1. Disassembler Extraction, Memory Banking & Core Machine Setup (2026-09-12)
    2. Language Guardrails, HRM Alignment & vAmigaTS Verification Harness (2026-09-14)
    3. Custom Chipset Silicon Calibration & Phase 1 vAmigaTS Verification (2026-09-15)
    4. Multimodal Reference Ingestion & 14-Stage PDF-to-Markdown Pipeline (2026-09-16)
    5. RAG Knowledge Base, Diagram Sidecars & Attractor Elimination (2026-09-16)
    6. Tooling Ergonomics, Zero-Junction Invariant & Full Worktree Storage Isolation (2026-09-17)
    7. MemoryBus Streamlining, Agnus DMA Mastership & Decoupled State Serialization (2026-09-18)
    8. Tripartite Quality Audits, Crate Renaming (m68000 -> cpu), and Compiler Lints Baseline (2026-09-19)
    9. Strict Compilation Policy, CPU Semantic Parity & Scope Discipline (2026-09-21)
    10. RAG Tooling Reform, CLI Single Source of Truth & Documentation Scoping (2026-09-22)
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
