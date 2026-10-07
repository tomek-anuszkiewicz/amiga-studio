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
  - Staged English scan reported the Python standard-library identifier copytree as Polish; manually verified this false positive without changing the scanner
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

### [2026-10-06 19:34 CET] — Prioritize RunPod document-model comparison
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Add RUNPOD-PDF-MODELS as Step 1.1 for Qwen3-VL-32B-Instruct, Chandra OCR 2 and olmOCR-2-7B-1025 on the test PDF
  - record source-quality, correction-effort, GPU runtime and cost acceptance criteria
  - renumber remaining Step 1 tasks
- **Architectural Rationale & Trade-Offs**:
  - Select the conversion model from measured source fidelity and actual GPU cost before bulk conversion
  - keeping subscription quota and token counts separate
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture suite passed 19/19
  - git diff --check passed
  - planning only, no GPU rental or conversion run
---

### [2026-10-06 23:33 CEST] — Complete Achtung Amiga subpage download catalog
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Add More_register_changes_in_a_scanline.html to the download catalog
  - Order index and 17 subpages according to the requested publication hierarchy
  - Correct catalog and developer documentation page counts
- **Architectural Rationale & Trade-Offs**:
  - The fixed download list omitted the available Copper subchapter, leaving its content out of conversion
- **Verification & Test Results**:
  - PowerShell AST parsing passed and catalog matched index plus all 17 requested subpages in order
  - Quick pre-flight passed
  - Architecture rules passed 19/19
  - git diff --check passed
  - No download or conversion run
---

### [2026-10-06 23:43 CET] — Embed linked assembly sources in HTML reference conversion
- **Affected Subsystems**:
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/html-to-markdown`
  - `tests/test_html_assembly_sources.py`
- **What Was Changed (The Concrete Reality)**:
  - Add readclock.s to the undocumented-chipset download catalog
  - Resolve local linked .s files into filename-captioned m68k listings before single-page and crawl transcription
  - Require complete assembly source preservation and replace download references with embedded listings in the prompt
  - Document attachment handling and retain focused regression protection
- **Architectural Rationale & Trade-Offs**:
  - Downloading a source attachment alone did not expose it to the HTML-only transcription input
  - leaving the battery-clock listing absent and its relative link unresolved.
- **Verification & Test Results**:
  - Regression failed on original code for both conversion routes and passes after the fix (2 tests)
  - Shared Codex conversion suite passes (19 tests)
  - Bootstrap -Undocumented downloaded readclock.s from the live source (1174 bytes, 77 lines), and local preparation preserves its full text
  - Quick pre-flight passes and architecture suite passes (19 tests)
  - Publication suite passes 9 of 10 tests, with the isolated unmodified PDF resume case failing its validation-depth assertion (1 versus 16)
  - Graphify code update completed
  - No LLM fragment conversion or full crawl transcription was run
---

### [2026-10-06 23:46 CET] — Move PDF-only conversion modules into the PDF common package
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Relocate six PDF-only modules to pdf-to-markdown/common without compatibility aliases
  - Update worker and orchestrator imports, existing test imports and package documentation
  - Fingerprint both shared conversion and PDF common modules for stage reuse
- **Architectural Rationale & Trade-Offs**:
  - Keep conversion limited to infrastructure shared by HTML and PDF while placing PDF stage contracts and helpers with their converter
  - Existing stage identities change and saved completions require regeneration
- **Verification & Test Results**:
  - PDF technical tests passed 30/30
  - Shared Codex tests passed 19/19
  - CLI import smoke checks passed for 14 scripts
  - Publication tests passed 9/10 with the same pre-existing resume assertion failure before and after relocation (1 versus 16)
  - Quick pre-flight passed
  - Architecture suite passed 19/19
  - Graphify code update completed
  - No source conversion run
---

### [2026-10-07 16:00 CET] — PDF-FRAGMENT-00: prepare only selected source pages
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Stage 00 writes only selected pages and applies OCR only to selected textless pages
  - validate only selected pages
  - retain source numbering with explicit prepared_index mapping through Stage 01
  - document restart requirements
- **Architectural Rationale & Trade-Offs**:
  - Fragment preparation must not save or validate all source pages
  - compact output must preserve original page identities for downstream conversion
- **Verification & Test Results**:
  - Reproduction failed before fix: five-page output for two-page selection
  - 32 PDF technical tests passed, including native noncontiguous mapping and mixed OCR selection
  - quick preflight passed
  - architecture suite 19/19 passed
  - graphify update completed
  - no real-source fragment conversion or performance benchmark run
---

### [2026-10-07 16:03 CET] — PDF-FRAGMENT-00: remove preparation output validation
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove candidate render, geometry, stream, native-text and source-immutability comparisons
  - remove output reopening and text digests
  - remove Stage 00 completion content-validation contract
  - remove separate OCR recovery integrity checks
  - retain source/prepared page mapping
- **Architectural Rationale & Trade-Offs**:
  - User explicitly requires Stage 00 to select pages
  - OCR textless pages and save without validation
- **Verification & Test Results**:
  - Focused native-page test failed before removal because validation rendered the page
  - 31 PDF technical tests passed after removal
  - obsolete candidate-validation test removed
  - quick preflight passed
  - architecture tests 19/19 passed
  - Graphify refreshed
  - no live fragment conversion or timing measurement
---

### [2026-10-07 16:07 CET] — Remove prepared PDF hashes from conversion metadata
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove pdf_sha256 from preparation manifests, per-page metadata and OCR recovery
  - remove dedicated PDF hash validation
  - update stage documentation and existing regression assertions
- **Architectural Rationale & Trade-Offs**:
  - Document-wide prepared PDF hashes changed otherwise identical page prompts and prevented response cache reuse across attempts
  - retain general stage artifact lineage
- **Verification & Test Results**:
  - Focused existing regression failed before implementation and passed after removal
  - PDF suite 31 passed
  - conversion transport/cache suite 19 passed
  - quick pre-flight passed
  - architecture suite 19 passed
  - graphify update completed
  - no live fragment conversion run
---

### [2026-10-07 16:00 CEST] — PDF-OCR-SHARED: reuse a full-document sibling prepared PDF
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Stage 00 prepares every source page into a sibling -ocr.pdf and skips page processing and PDF writing when it already exists
  - Stage 01 applies the attempt page selection with stable full-source page indices
  - Register and hash the shared external PDF in local completion records while preserving it through workspace cleanup
  - Record prepared or absent text provenance without inventing native versus OCR origin for an existing PDF
  - Adapt fragment regressions and verify cross-workspace byte-identical inputs and CLI restart
- **Architectural Rationale & Trade-Offs**:
  - The compact prepared-page index changed Stage 02 prompts across overlapping fragments and prevented exact response-cache reuse
  - Shared full-source preparation gives stable page inputs and avoids repeated OCR
  - File existence deliberately controls preparation reuse and explicit shared-file removal requests regeneration
- **Verification & Test Results**:
  - Original focused regressions failed for omitted textless pages and compact output
  - PDF suite 32/32 passed including deterministic CLI execution and restart
  - Shared conversion suite 19/19 passed
  - Quick pre-flight passed
  - Architecture rules 19/19 passed
  - Graphify incremental update passed
  - No real-book conversion or live model call was run
---

### [2026-10-07 17:26 CEST] — Plan Stage 02.9 Markdown and crop asset export (PDF-MARKDOWN-02.9)
- **Affected Subsystems**:
  - `.agent/tasks/pdf-stage-02.9-markdown-assets.md`
  - `tools/bootstrap/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Add an active English execution plan for deterministic Stage 02.9 assembly of one document.md and assets directory from validated Stage 02 objects and original Stage 01 PNGs
  - specify exact pixel cropping, ordered text/image emission, predecessor validation and independent restart behavior
  - retain implementation as pending
- **Architectural Rationale & Trade-Offs**:
  - Provide a readable intermediate Markdown bundle from the existing page conversion without new model requests or redesigning the downstream stream pipeline
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture rules passed 19/19
  - plan language scan passed
  - plan relative links passed
  - no implementation, live inference or fragment conversion ran
---

### [2026-10-07 17:30 CEST] — Track Test Book source PDFs and conversion artifacts
- **Affected Subsystems**:
  - `.gitignore`
  - `Obsidian/Amiga/Reference/Test Book example-4567`
- **What Was Changed (The Concrete Reality)**:
  - Remove the disabled Test Book ignore rule
  - Add both source/OCR PDFs, the page tracker, and the pages-all conversion workspace (653 files)
- **Architectural Rationale & Trade-Offs**:
  - Preserve the complete existing Test Book fixture and conversion snapshot in Git at the user's explicit request
- **Verification & Test Results**:
  - pre_flight.py --quick passed
  - test_architecture_rules passed 19/19
  - parsed all 325 JSON files successfully and scanned text/configuration for private host paths and common secret patterns with zero findings
  - git diff --cached --check passed; optional staged language scan failed with 29 false positives in OCR noise, proper names and technical labels (including Kokomo, ADKCON and todmid); preserved the existing conversion artifacts
  - conversion output was not regenerated or semantically reviewed
---

### [2026-10-07 17:31 CET] — Plan final Step 1 Test Book branch cleanup
- **Affected Subsystems**:
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Add Step 1.6 (TEST-BOOK-CLEANUP) after conversion and RAG ingestion verification
  - Plan removal of tracked Test Book files, retention of the local fixture, and restoration of its ignore rule
- **Architectural Rationale & Trade-Offs**:
  - Record the requested final Step 1 cleanup with explicit Git verification criteria
- **Verification & Test Results**:
  - pre_flight.py --quick passed
  - architecture tests passed 19/19
  - ROADMAP.md English language scan and git diff --check passed
  - Step 1 numbering remains contiguous
  - cleanup is planned and was not executed
---

### [2026-10-07 17:39 CEST] — PDF-MARKDOWN-02.9: emit selected-page Markdown and exact crop assets
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tests/test_pdf_conversion_codex.py`
  - `tests/test_conversion_publish.py`
  - `docs/developers.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Add deterministic Stage 02.9 after review and before raw-stream assembly, with explicit Stage 01/02 dependencies and standalone CLI execution
  - Preserve decoded md_text and physical-page/object order in one document.md with minimal YAML and a source-stem heading
  - Emit exact original-PNG table/graphic/cover crops with upstream segment filenames and relative image links, retaining captions and continuation text without merging
  - Validate every bundle before replacement and completion, remove stale assets and retain stream independence
  - Add page/object context to existing Stage 02 validation errors without changing its schema or prompt
  - Update converter documentation and retain user fragment assessment in roadmap 1.2
  - Adapt the existing restart expectation and repair a publication fixture that declared complete resume while supplying only Stage 00
- **Architectural Rationale & Trade-Offs**:
  - Reuse the existing page-object and geometry contracts with Pillow PNG tooling instead of requesting another conversion or introducing another schema
  - Bind completion to both predecessor records and existing procedure/configuration identity, with 02.5 and 02.9 restarts independent of the stream
  - Shared validation changes intentionally invalidate older procedure fingerprints: regenerate incompatible completion records rather than relabeling artifacts
  - Tables and illustrations remain raster crops and Stage 14 publication remains separate
  - Close execution plan PDF-MARKDOWN-02.9 after verified implementation, retaining source-quality assessment as pending user-selected fragment work rather than claiming acceptance
- **Verification & Test Results**:
  - Existing PDF suite passed 32/32, shared conversion suite 19/19 and publication suite 10/10
  - Publication fixture first failed 1 != 17 and was reproduced on original HEAD as 1 != 16 before repair, then passed with complete stage fixture records
  - Disposable synthetic orchestration check passed standalone 02.9 without review artifacts, disjoint page selection, validated output, byte-identical rerun, stale-asset removal and two-input lineage with zero model calls
  - python tools/harness/pre_flight.py --quick passed and architecture rules passed 19/19
  - graphify update . completed AST refresh, with optional PDF extraction skipped on the first refresh because pypdf was unavailable
  - No real-source fragment conversion, quality assessment, full-book run, milestone gate or publication was performed
---

### [2026-10-07 17:54 CEST] — Simplify PDF resume to completed stage status
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tests/test_pdf_conversion_codex.py`
  - `tests/test_conversion_publish.py`
- **What Was Changed (The Concrete Reality)**:
  - Task pdf-status-resume: remove stage, source, artifact and procedure digests from completion tracking
  - retain required file paths and manifest snapshots
  - normalize existing saved records and simplify the selected Test Book state
  - persist invalidation before cleanup
  - preserve runtime validators and request caches
- **Architectural Rationale & Trade-Offs**:
  - Resume should skip completed stages with existing files
  - Git and explicit restarts control applying procedure changes to saved results
- **Verification & Test Results**:
  - Cleanup interruption regression failed before repair and passes afterward
  - PDF tests 34/34, shared conversion/cache tests 19/19, publication tests 10/10
  - quick pre-flight passed
  - architecture tests 19/19
  - Graphify incremental code refresh completed
  - no user-book conversion run
---

### [2026-10-07 18:01 CEST] — Require explicit PDF restart stages and clear later results
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tests/test_pdf_conversion_codex.py`
  - `tests/test_conversion_publish.py`
- **What Was Changed (The Concrete Reality)**:
  - Remove the resume option and automatic stage selection
  - Clear the selected stage and every later stage in execution order regardless of artifact dependencies or the execution end stage
  - Require an explicit starting stage for deterministic execution and adapt restart/publication tests and documentation
- **Architectural Rationale & Trade-Offs**:
  - The operator chooses each subsequent start and advances it to retain completed results
  - Preserve predecessor artifacts and reusable response cache while recording invalidation before cleanup
- **Verification & Test Results**:
  - PDF converter tests: 35 passed
  - Publication tests: 10 passed
  - Quick pre-flight: all six gates passed
  - Architecture rules: 19 passed
  - CLI help and git diff whitespace checks passed
  - Graphify code update completed
  - No user-source conversion or quality evaluation was run
---

### [2026-10-07 18:05 CEST] — PDF-STATE-1: Remove conversion completion file inventories
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `Test Book example-4567 conversion state`
- **What Was Changed (The Concrete Reality)**:
  - Remove completion file inventories, recursive file collection and predecessor file-existence checks
  - Remove Stage 00 completion inventory membership checks and unused path/output arguments
  - Discard legacy file lists on state reads and remove 643 inventory entries from the selected saved workspace
  - Adapt existing completion/restart tests and converter documentation
- **Architectural Rationale & Trade-Offs**:
  - Use completed status for predecessor bookkeeping while preserving manifest snapshots for restart and runtime artifact validation in executing workers
  - Missing artifacts no longer invalidate saved completion status and may fail when consumed
- **Verification & Test Results**:
  - PDF converter tests: 35 passed
  - Publication tests: 10 passed
  - Quick pre-flight: passed
  - Architecture rules: 19 passed
  - Graphify incremental code refresh: passed
  - No user-source conversion or milestone verification run
---

### [2026-10-07 18:35 CEST] — HTML-AUDIT-REMOVE: remove unused conversion sanity auditor
- **Affected Subsystems**:
  - `tools/bootstrap/html-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Delete scripts/audit_conversion.py
  - Remove its entry from the converter README
- **Architectural Rationale & Trade-Offs**:
  - Remove the standalone heuristic quality checker at the user's request
  - Repository search found no callers or imports
- **Verification & Test Results**:
  - Quick pre-flight: all six gates passed
  - Architecture rules: 19 passed
  - Repository search: no remaining active references
  - Graphify incremental code refresh: passed
  - No source conversion or quality evaluation run
---

### [2026-10-07 18:37 CEST] — Remove unused Markdown reference comparator
- **Affected Subsystems**:
  - `tools/bootstrap/html-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Delete the standalone reference comparison script
  - Remove its file listing from the HTML converter README
- **Architectural Rationale & Trade-Offs**:
  - The user requested removal after repository searches found no external callers or pipeline integration.
- **Verification & Test Results**:
  - python tools/harness/pre_flight.py --quick: passed
  - cargo test -p test_runner --test test_architecture_rules -- --quiet: 19 passed
  - Repository search after deletion found no remaining comparator references before this diary entry
  - graphify update .: passed
  - Full-worktree git diff --check reports an existing blank line at EOF in the unrelated PDF selection module.
---

### [2026-10-07 18:39 CEST] — HTML-RENDER-REMOVE: remove unused visual comparison renderer
- **Affected Subsystems**:
  - `tools/bootstrap/html-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Delete the standalone visual comparison renderer
  - Remove its README listing and usage section
  - Update optional dependency guidance and renumber link validation
- **Architectural Rationale & Trade-Offs**:
  - The user requested removal after inspection found only manual README usage and no pipeline callers.
- **Verification & Test Results**:
  - Quick pre-flight: all six gates passed
  - Architecture rules: 19 passed
  - Repository search: no remaining active renderer or comparison-image references
  - Scoped git diff --check: passed
  - Graphify incremental code refresh: passed
  - No source conversion or quality evaluation run
---

### [2026-10-07 18:40 CET] — PDF-STATE-1: Use stage-owned PDF artifacts and execution status
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `tests`
  - `converter contract`
  - `Test Book pages-all attempt`
- **What Was Changed (The Concrete Reality)**:
  - Remove conversion state, PDF manifests, snapshots, local PDF configuration/artifact/response validation and manual handoff modes
  - persist requested inputs in attempt config
  - carry chapter metadata with nodes through automatic stages
  - record atomic status after worker and asset operations
  - retain HTML checks and authenticated transport completion
  - migrate only pages-all and preserve 645 result files plus status bytes
  - close PDF-STATE-1 after verification
- **Architectural Rationale & Trade-Offs**:
  - Implement the user-approved execution plan: success records completed execution rather than artifact certification
  - keep direct predecessor ownership without replacement registries, validators or older-stage fallbacks
- **Verification & Test Results**:
  - PDF offline suite 25/25
  - shared conversion suite 21/21
  - publication suite 10/10
  - pre_flight --quick passed
  - architecture suite 19/19
  - Graphify incremental refresh passed
  - CLI help and removed-mechanism searches passed
  - Python compilation and global-reference check passed
  - synthetic migration transferred 12 chapters and missing-metadata refusal preserved all files
  - pages-all migration retained existing outputs and metrics with zero OCR/model calls
  - no real fragment conversion or quality campaign ran
  - roadmap milestones remain pending
---

### [2026-10-07 18:43 CEST] — chore(pdf): remove the one-time legacy attempt migrator
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Delete migrate_attempt.py
  - Remove legacy transfer instructions and the obsolete utility link
- **Architectural Rationale & Trade-Offs**:
  - The user requested removal after inspection established that the utility was manually invoked and had no pipeline callers
- **Verification & Test Results**:
  - python tools/harness/pre_flight.py --quick: all six gates passed
  - cargo test -p test_runner --test test_architecture_rules -- --quiet: 19 passed
  - Repository search found no remaining migrator references before this entry
  - git diff --check: passed
  - graphify update .: passed
  - No fragment conversion or conversion-quality evaluation run
---

### [2026-10-07 18:51 CEST] — PDF-MARKDOWN-02.9: record Test Book page Markdown export
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all`
- **What Was Changed (The Concrete Reality)**:
  - Track the generated Stage 02.9 document.md and 203 PNG assets
  - Record successful Stage 02.9 execution in stage_status.json with 7.4 seconds duration and zero model calls
- **Architectural Rationale & Trade-Offs**:
  - Preserve the requested Stage 02.9 execution results in Git without rewriting the generated Markdown or claiming conversion-quality acceptance
- **Verification & Test Results**:
  - python tools/harness/pre_flight.py --quick: all six gates passed
  - cargo test -p test_runner --test test_architecture_rules -- --quiet: 19 passed
  - Inspected staged scope: document.md, 203 PNG assets and Stage 02.9 status
  - git diff --cached --check reported nine two-space Markdown hard breaks, preserved under the unchanged-text contract
  - No conversion rerun or visual fidelity assessment performed in this commit task
  - Roadmap quality assessment remains pending
---

### [2026-10-07 19:00 CEST] — PDF-FILTER-02.8: filter source objects before Markdown and stream assembly
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Add deterministic Stage 02.8 with ordered page JSON, exact type exclusions and page/object statistics
  - Remove headers and footers, pre-TOC objects and whole list/index pages while preserving surviving fields and IDs
  - Route 02.9 and 03 exclusively through 02.8 with Stage 01 geometry and original PNGs
  - Register dependencies, inventory and execution-order cleanup, and remove header/footer filtering from Stage 04
  - Update workflow documentation and the two existing test expectations superseded by routing
  - Close the execution plan after this commit
- **Architectural Rationale & Trade-Offs**:
  - PDF-FILTER-02.8 makes source removal explicit before either assets or downstream nodes exist
  - Determine the TOC boundary from original objects even on an excluded page, and skip pre-TOC removal when no selected-input heading exists
  - Preserve complete Stage 02.5 review and original predecessor artifacts, add no inference configuration or runtime validation, and keep generated YAML metadata
  - Existing downstream successes need explicit regeneration starting at 02.8 and were not changed during implementation
- **Verification & Test Results**:
  - Python compilation and diff whitespace check passed
  - Existing offline PDF suite passed all 25 tests
  - Bounded synthetic pipeline check passed filtered routing, field preservation, predecessor preservation and single-stage downstream cleanup
  - graphify update . completed without model calls
  - python tools/harness/pre_flight.py --quick passed
  - cargo test -p test_runner --test test_architecture_rules -- --quiet passed all 19 tests
  - No source-fragment conversion or conversion-quality assessment ran, and no milestone completion is claimed
---

### [2026-10-07 19:13 CEST] — PDF-FILTER-02.8 / PDF-MARKDOWN-02.9: record filtered Test Book export
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all`
- **What Was Changed (The Concrete Reality)**:
  - Track 147 retained Stage 02.8 page JSON files
  - Update Stage 02.9 document.md and remove eight obsolete crop assets leaving 195 PNG assets
  - Record successful Stage 02.8 and refreshed Stage 02.9 execution status with zero model calls
- **Architectural Rationale & Trade-Offs**:
  - Preserve the user-requested generated results after source-content filtering without rewriting the conversion output
  - Keep conversion-quality assessment pending
- **Verification & Test Results**:
  - python tools/harness/pre_flight.py --quick: all six gates passed
  - cargo test -p test_runner --test test_architecture_rules -- --quiet: 19 passed
  - Parsed all 147 Stage 02.8 JSON files and checked all 195 Markdown asset links resolve
  - Existing status records report 0.26 seconds for Stage 02.8 and 8.3 seconds for Stage 02.9
  - No conversion rerun or visual fidelity assessment performed in this commit task
---

### [2026-10-07 19:18 CEST] — PDF-GRAPHIC-EXCLUSION-1: Exclude the recurring standalone NXP logo
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Bundle the exact supplied page-49 NXP PNG
  - filter surviving standalone upper-left graphics with fixed relative geometry, foreground aspect and RGB comparison
  - require Stage 01/02 completion
  - document restart and export preservation
- **Architectural Rationale & Trade-Offs**:
  - Implement the authorized source-fidelity exception upstream of both Stage 02.9 and Stage 03 without configuration or generic exclusion infrastructure. Preserve all survivor fields, IDs, geometry and order
  - retain merged or nonmatching objects. Fixed constants: reference page 2550x3300, region [0,0,0.13,0.055], size tolerance 0.20, aspect tolerance 0.03, RGB tolerance 0.02, foreground channel below 245. Source and existing attempts remain untouched.
- **Verification & Test Results**:
  - PASS: 25 existing PDF technical tests
  - existing offline filtered routing/upstream-preservation/single-stage-cleanup diagnostic
  - quick preflight
  - architecture suite 19/19
  - git diff --check
  - explicit graphify update. Read-only comparison of supplied page-49/50 crops passes geometry and image gates: page 49 aspect/RGB 0/0, page 50 aspect 0.01052632 and RGB 0.00569827. Bundled 257x94 template is pixel-identical to original page-49 crop. No fragment pipeline conversion or downstream regeneration requested or run
  - pre-existing edited export preserved. Book-wide adequacy remains unverified. Implementation task closed
  - remove its active plan after committing this record.
  - Staged language scan reported two false positives for the proper algorithm name Lanczos in English documentation; reviewed and retained the correct technical name. No language-checker changes made.
---

### [2026-10-07 19:23 CEST] — PDF-GRAPHIC-EXCLUSION-2: Match upright NXP logos on landscape pages
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.8_filter_page_content`
  - `tests/test_pdf_conversion_codex.py`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Orient the fixed reference page dimensions to the candidate canvas while keeping the logo upright
  - retain all existing geometry and image thresholds
  - document landscape behavior
  - add a focused geometry regression for page 63
- **Architectural Rationale & Trade-Offs**:
  - The reported page-63 logo is visually identical to the template but its 3300x2550 canvas caused width/height deviations of -22.4%/+29.4% against portrait proportions. Correct the reference orientation instead of loosening image thresholds or special-casing page IDs.
- **Verification & Test Results**:
  - Focused regression failed before repair and passed afterward
  - all 26 PDF technical tests passed
  - quick preflight passed
  - architecture suite 19/19 passed
  - graphify update completed
  - git diff --check passed. Pipeline stages 02.8-02.9 succeeded for only page 63 in workspace/page-63-nxp-fix-attempt-01 using copied successful Stage 01/02 predecessors, without model requests. Removed page_0063_seg_001 with aspect and RGB differences 0
  - filtered JSON and new Markdown/assets omit the logo, retaining two table crops. Existing pages-all artifacts and user edits untouched. Book-wide behavior and visual content assessment remain unverified.
---

### [2026-10-07 19:36 CEST] — PDF-GRAPHIC-EXCLUSION-3: Record regenerated Stage 02.8 and 02.9 outputs
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all/02.8_filter_page_content`
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all/02.9_emit_page_markdown`
  - `stage_status.json`
- **What Was Changed (The Concrete Reality)**:
  - Commit existing regenerated filtered objects and Markdown bundle
  - remove 26 standalone NXP logo segments, PNG assets and image links on physical pages 38-63
  - retain associated execution status changes
- **Architectural Rationale & Trade-Offs**:
  - User explicitly requested committing Stage 02.8 and 02.9 results after the logo filter and landscape correction. Review and preserve the existing working-tree outputs without another conversion run.
- **Verification & Test Results**:
  - Diff verification passed: 26 page JSONs differ only by one removed graphic each
  - all survivor fields and order unchanged
  - Markdown differs only by the 26 corresponding image links
  - exactly 26 logo PNGs deleted
  - all 169 retained image links resolve. Existing status records report success, 0.60 seconds for Stage 02.8 and 7.74 seconds for Stage 02.9, with no model calls. These are recorded run metrics, not a new agent-run conversion. Quick preflight and architecture suite 19/19 passed. Visual content quality and book-wide matching are not certified.
---

### [2026-10-07 19:50 CEST] — PDF-TABLE-2.81: Specify collapsible Markdown companions for HTML tables
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Extend the active local plan and proposed response contract with table_markdown for HTML results
  - Require exactly one details section with a Markdown transcription inside a fenced text block
  - Repeat associated caption, footnote and table_legend content in source order while retaining their visible source positions
  - Place details after the last associated block, allowing captions above or below and footnotes before or after legends
  - Carry companion text and table ownership through page and stream renderers without duplicate emission
- **Architectural Rationale & Trade-Offs**:
  - Record the requested behavior before implementation
  - Assemble associated text from source segments rather than inventing content outside the table crop
  - Keep the active execution plan local under the existing Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Documentation planning only: implementation and inference have not started
  - Active plan remains open and is excluded from Git by repository policy
---

### [2026-10-07 19:51 CEST] — PDF-TABLE-2.81: Specify warnings for simple HTML table results
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Extend the active local plan with a console warning before Stage 02.81 completion
  - Identify regular rectangular HTML tables with no effective colspan or rowspan and Markdown-compatible cell contents
  - Include physical page and segment ID in each warning and inspect cached results too
  - Preserve HTML output, Markdown companion and successful stage status without retries or automatic conversion
- **Architectural Rationale & Trade-Offs**:
  - Expose potentially unnecessary HTML choices for user review using generated table structure
  - Treat spans of 1 as unmerged and avoid classifying complex cells from span absence alone
  - Keep the active plan local under the existing Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Planning only: no converter implementation or inference run
---

### [2026-10-07 19:54 CEST] — PDF-TABLE-2.82: Plan side-by-side visual table reviews
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Add planned review-only Stage 02.82 between conversion 02.81 and export 02.9
  - Specify one comparison PNG per table with original Stage 01 raster at left and rendered Markdown or HTML at right
  - Include associated captions, footnotes and table legends with source order and visible page, segment and format labels
  - Expand the existing HTML details companion beneath the converted group with literal monospaced Markdown text
  - Require full-content capture, unchanged predecessor artifacts and explicit handling of unconverted tables
  - Reuse shared export group assembly and keep 02.9 and 03 dependent on 02.81 data rather than review images
- **Architectural Rationale & Trade-Offs**:
  - Support quick user visual assessment without new inference or automatic content changes
  - Reconstruct original crops from saved bounds rather than retaining inference temporary files
  - Select and verify a local rendering backend during implementation
  - Keep the active execution plan local under its existing Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Planning only: no implementation, inference, browser backend checks or comparison PNG generation performed
---

### [2026-10-07 19:55 CEST] — PDF-TABLE-2.81: Simplify the planned transcription prompt
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace the detailed prompt draft with a minimal faithful Markdown-first transcription instruction and HTML fallback
  - Retain only the JSON response contract, closest Markdown companion for HTML and unconverted outcome
  - Remove suggested merged-cell patterns and detailed source-layout guidance
- **Architectural Rationale & Trade-Offs**:
  - Follow the user request for a minimal prompt without suggested table structure
  - Keep caption association, companion assembly, warnings and visual review in the pipeline rather than the transcription prompt
  - Preserve the active local plan under its Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Planning only: no implementation or inference performed
---

### [2026-10-07 19:57 CEST] — PDF-TABLE-2.81: Plan inferred complete RAG companions for HTML tables
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace deterministic companion concatenation with a separate inference for each HTML table group
  - Supply original table crop, HTML transcription and full associated caption, footnote and table_legend blocks in source order
  - Add a dedicated prompt explaining collapsed details and literal text storage for RAG with no summarization, omissions or added content
  - Replace the table-only companion contract with table_rag_text for the complete group
  - Keep the primary Markdown or HTML transcription prompt minimal and wrap saved companion text deterministically
  - Reuse saved text for export and expanded Stage 02.82 review without another inference
- **Architectural Rationale & Trade-Offs**:
  - Follow the user clarification that an agent prepares the complete textual companion
  - Preserve table relationships and complete associated source information in order
  - Cache the full group inputs and fail normally on companion transport or parsing errors instead of silently substituting concatenation
  - Keep the active plan local under its Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Inspected affected planned contracts and consumers for obsolete companion references
  - Planning only: implementation and inference have not started
---

### [2026-10-07 19:58 CEST] — PDF-TABLE-2.81: Plan original-image details companions
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Add a second collapsed details section immediately after each HTML textual RAG companion with the original table image
  - Preserve the exact Stage 01 table crop as a published asset and link it relatively from the section
  - Update page and stream export plans to retain HTML source-image assets while converted Markdown tables remain crop-free
  - Separate published source crops from temporary inference crops
  - Include the second collapsed section in Stage 02.82 while its source image is already shown at left
  - Clarify that only the textual companion uses inference and both sections must survive export without duplication
- **Architectural Rationale & Trade-Offs**:
  - Provide the original raster for checking transcription fidelity
  - Correct the earlier no-assets rule for converted HTML tables so source-image links remain valid
  - Keep the active plan local under its existing Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Checked planned export and asset lifecycle statements for consistency
  - Planning only: no implementation, inference or images generated
---

### [2026-10-07 20:09 CEST] — PDF-TABLE-2.81: Resolve table grouping and reusable crop decisions
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.81-table-transcription.md`
- **What Was Changed (The Concrete Reality)**:
  - Use existing Stage 02 table classifications and collect associated blocks upward/downward in source order
  - Retain repeated caption, footnote and table_legend blocks with console warnings including captions above and below
  - Bound initial collection to adjacent associated blocks on the same page and warn on ambiguous ownership
  - Record ordered table_group_segment_ids and persistent table_source_asset crops for all table outcomes
  - Have Stage 02.82 reuse crops and Stage 02.9/03 copy required table assets while still cropping graphics/covers
  - Allow RAG inference to express HTML relationships freely and faithfully in plain text rather than require a Markdown grid
  - Limit later companion edits to typo corrections or source-grounded additions and defer RAG ingestion verification to the user TODO
- **Architectural Rationale & Trade-Offs**:
  - Record user decisions resolving the reviewed plan gaps
  - Preserve anomalous source content instead of dropping extra blocks
  - Remove recropping and the dependency on a later export for original table images
  - Keep the primary conversion prompt minimal and the active plan local under its Git exclusion
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - Architecture suite passed 19 of 19 tests
  - Reviewed crop lifecycle, companion prompt and downstream field propagation in the plan
  - Planning only: no implementation, inference or review image generation performed
---

### [2026-10-07 20:16 CET] — PDF-TABLE-2.81: transcribe page tables and render source comparisons
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `docs/developers.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Add Stage 02.81 table transcription and required separate HTML group inference
  - save persistent original crops and ordered group metadata
  - add Stage 02.82 Playwright/Edge comparisons
  - share companion assembly across page export and final stream
  - preserve converted fragments through reduction and continuation workers
  - update routing, settings and documentation
- **Architectural Rationale & Trade-Offs**:
  - Replace table rasters with faithful markup while retaining source context and reusable crops
  - review is deterministic and independent of export
  - preserve existing attempt settings and explicit restart boundaries
- **Verification & Test Results**:
  - Quick preflight PASS
  - architecture 19/19
  - existing PDF 26/26, Codex transport/cache 21/21 and publication 10/10
  - Python compilation and CLI help PASS
  - synthetic offline group/export/stream/restart acceptance PASS
  - Edge rendered three formats and HTML fixture visually inspected
  - Graphify incremental update PASS. No live inference or existing attempt regeneration. First user-selected fragment and fidelity review pending
  - RAG details ingestion deferred. Active plan retained.
---

### [2026-10-07 20:51 CEST] — PDF-CALLOUT-2.83: Plan text-only callout recovery
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.83-callout-reclassification.md`
  - `tools/bootstrap/pdf-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Save an active local execution plan while keeping table review at Stage 02.82 as confirmed by the user
  - plan Stage 02.83 keyword-triggered text-only model classification with adjacent context
  - inventory 307 existing page JSONs across Stage 02 and 02.8 yielding 18 distinct callout labels and NOTE CAUTION WARNING vocabulary
  - identify 18 distinct non-callout candidates including eight prose three code blocks four footnotes and three table legends
  - plan complete page overrides only for changed pages with per-page Stage 02.81 fallback in 02.9 and 03
  - preserve predecessor data table assets group metadata and source text
- **Architectural Rationale & Trade-Offs**:
  - Recover advisory roles missed by initial classification while leaving ordinary keyword mentions and attached notes to model review
  - preserve existing table review numbering and use sparse page replacements
  - retain the active plan under its existing Git exclusion until implementation and user-selected review are resolved
- **Verification & Test Results**:
  - Planning inspection and inventory completed without inference or regeneration
  - quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - implementation fragment conversion and user fidelity assessment remain pending
---

### [2026-10-07 20:56 CEST] — PDF-CALLOUT-2.83: Require source-page vision in the plan
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.83-callout-reclassification.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace the text-only proposal with a vision request for every keyword candidate as requested by the user
  - supply the complete original Stage 01 PNG with original detail plus target text identity and pixel bounds
  - use full-page layout to distinguish independent callouts from table or figure notes
  - preserve sparse page overrides and Stage 02.81 fallback
- **Architectural Rationale & Trade-Offs**:
  - Text alone cannot establish visual attachment to a table or figure
  - the full original page preserves the surrounding layout without a narrow candidate crop
  - recorded roles and table groups remain context rather than authoritative classification
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - checked the plan for superseded text-only requirements
  - planning only with no implementation inference or artifact regeneration
---

### [2026-10-07 20:59 CEST] — PDF-CALLOUT-2.83: Plan framed vision and explicit block replacements
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.83-callout-reclassification.md`
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
- **What Was Changed (The Concrete Reality)**:
  - Replace classification-only edits with explicit source-frame ranges and faithful replacement text
  - plan numbered colored outlines mapped to stable segment IDs using the existing review renderer
  - provide original and annotated full-page images in one joint request per candidate-bearing page
  - handle label and body merging splitting and retained ordinary text
  - splice disjoint ranges once against frozen input and preserve unselected objects
  - propagate replacement lineage and affected table-group IDs and regenerate only affected HTML companions when ownership changes
- **Architectural Rationale & Trade-Offs**:
  - Give the model visible stable block references and responsibility for advisory boundaries while the program owns exact range replacement
  - joint page proposals avoid overlapping decisions from independent candidate calls
  - preserve source content and ordering rather than treating reduction as summarization
  - frame-assisted accuracy remains to be assessed on user-selected examples
- **Verification & Test Results**:
  - Inspected existing Stage 02.5 frame renderer and table-group consumers
  - quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - planning only with no implementation inference or artifact regeneration
---

### [2026-10-07 21:00 CEST] — PDF-CALLOUT-2.83: Move planned callout recovery to Stage 02.4
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.4-callout-reclassification.md`
- **What Was Changed (The Concrete Reality)**:
  - Rename the active local plan and planned stage directory from 02.83 to 02.4 at the user's request while retaining the stable task reference
  - place recovery after Stage 02 and before Stage 02.5
  - change planned sparse overrides to use Stage 02 as predecessor
  - route effective pages into review filtering and explicit direct table input
  - remove post-table companion repair because grouping now occurs after recovery
- **Architectural Rationale & Trade-Offs**:
  - Keep stage numbering execution order and artifact dependencies consistent
  - later table grouping and companions consume corrected objects through existing downstream stages
  - preserve historical diary references and existing Stage 02.82 table review
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - reviewed planned routing and checked active task and converter documentation for stale stage-directory references
  - planning only with no implementation inference or regeneration
---

### [2026-10-07 21:02 CEST] — PDF-CALLOUT-2.83: Plan Stage 02.5 annotation changes
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.4-callout-reclassification.md`
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
- **What Was Changed (The Concrete Reality)**:
  - Extend the active local plan with stable colors per displayed object type
  - consolidate uninterrupted adjacent prose into one review frame and type label before annotation
  - require the union to avoid intervening or overlapping objects and preserve source mappings
  - omit header footer and thumb-index bookmark-tab frames leaders and labels
- **Architectural Rationale & Trade-Offs**:
  - Make Stage 02.5 page reviews easier to read while retaining source pixels and predecessor objects
  - keep Stage 02.4 inference frames mapped to original objects
  - preserve the existing Git exclusion for the active plan
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - reviewed the added requirements against the existing Stage 02.5 README
  - planning only with implementation inference and artifact regeneration pending
---

### [2026-10-07 21:04 CEST] — PDF-REVIEW-2.5: Separate the first-priority review plan
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.5-review-presentation.md`
  - `.agent/tasks/pdf-2.4-callout-reclassification.md`
- **What Was Changed (The Concrete Reality)**:
  - Extract type colors prose annotation consolidation and hidden header footer thumb-index annotations into the independent PDF-REVIEW-2.5 plan
  - schedule this implementation first before callout recovery
  - remove duplicated requirements from the callout plan and link the standalone task
  - base review work on existing Stage 01 and Stage 02 inputs without requiring Stage 02.4 or its resolver
- **Architectural Rationale & Trade-Offs**:
  - Allow the requested review changes to be implemented and verified separately first
  - keep both unfinished plans under their existing Git exclusion and preserve original-object mappings for future inference
- **Verification & Test Results**:
  - Quick preflight passed all six gates
  - architecture suite passed 19 of 19
  - both plans passed the explicit language scan
  - checked requirement placement priority and reciprocal plan references
  - planning only with no implementation inference or regeneration
---

### [2026-10-07 21:07 CEST] — PDF-REVIEW-2.5: type colors and consolidated prose review annotations
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Use fixed distinct colors keyed by object type for frames, leaders and labels
  - Consolidate consecutive nearby prose in a common column only when the complete-page union is unobstructed
  - Suppress header/footer/thumb-index annotations and restore their original pixels under crossing leaders
  - Save review-only source ordinal/ID mappings and combined continuation flags
  - Update presentation documentation while preserving Stage 02 inputs, downstream content and raw inference-object drawing
- **Architectural Rationale & Trade-Offs**:
  - PDF-REVIEW-2.5 is an independent presentation change before planned Stage 02.4
  - Conservative shared-column and relative-gap rules can leave narrower or distant prose separate
  - Keep the active plan until user-selected fragment review execution and assessment
- **Verification & Test Results**:
  - Existing PDF technical suite: 26 tests passed
  - Focused disposable presentation diagnostic passed grouping/blocking, columns, source identity/immutability, type colors, continuation and hidden-pixel checks
  - Quick preflight passed
  - Architecture suite: 19 tests passed
  - graphify update completed
  - No real fragment regenerated: user selection remains pending
  - No inference calls or quality certification
---

### [2026-10-07 21:22 CEST] — PDF-REVIEW-2.5: fix short-paragraph grouping and complete page-31 review
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Recognize a shared prose column by stable left alignment and overlap relative to its narrowest member
  - Bound vertical gaps by column width instead of paragraph height so short aligned paragraphs remain in one run
  - Retain complete-page obstacle checks and source-order boundaries
  - Preserve actual segment_id values in review mappings with id fallback
  - Add a focused regression from observed page-31 geometry and regenerate only its Stage 02.5 review through the orchestrator
- **Architectural Rationale & Trade-Offs**:
  - The original widest-width overlap and shortest-height gap conditions split page-31 prose into five groups despite a common unobstructed column
  - Imported completed Stage 01/02 page-31 artifacts into workspace/page-31-prose-review-01 and retained source attempt settings apart from page selection
  - PDF-REVIEW-2.5 implementation and requested fragment review execution are complete, allowing removal of its active plan
  - Stage 02.4 remains independent and user assessment does not certify other pages
- **Verification & Test Results**:
  - Focused regression failed before repair with five annotations instead of one, then passed
  - PDF technical suite: 27 tests passed
  - Existing focused presentation diagnostic passed
  - Quick preflight passed
  - Architecture suite: 19 tests passed
  - graphify update completed
  - Stage 02.5 completed for physical page 31 with zero model calls and three annotations: graphic 1, caption 3 and prose 4-11
  - Confirmed IDs, unchanged source-attempt hashes and retained settings
  - Visually inspected regenerated page_0031_review.png at Obsidian/Amiga/Reference/Test Book example-4567/workspace/page-31-prose-review-01/02.5_page_conversion_review/
  - No broader conversion or quality claim
---

### [2026-10-07 21:28 CEST] — PDF-REVIEW-2.5: use only source adjacency and union obstacles
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Remove prose alignment, column, overlap-width and vertical-gap thresholds
  - Extend consecutive source-order prose by its smallest enclosing rectangle only when no other page object overlaps it
  - Keep hidden objects as obstacles and exclude all contributing source members from the check
  - Preserve run restart behavior, source mappings, continuation and original artifacts
  - Update the stage README and shared conversion contract to the user-approved union-only rule
- **Architectural Rationale & Trade-Offs**:
  - The user explicitly replaced spatial-flow heuristics with an enclosing-rectangle obstacle check
  - Shifted, distant or overlapping contributing prose may join if the union contains no other object
  - Any non-prose source-order interruption ends the run
- **Verification & Test Results**:
  - Focused union-only regression failed before repair with three separate annotations, then passed including blocked-run restart
  - PDF technical suite: 28 tests passed
  - Quick preflight passed
  - Architecture suite: 19 tests passed
  - graphify update completed
  - Regenerated only physical page 31 through orchestrator Stage 02.5 in workspace/page-31-prose-review-01 with zero model calls
  - Prose ordinals 4-11 remain one annotation
  - Predecessor artifact and config hashes unchanged
---

### [2026-10-07 21:38 CEST] — PDF-REVIEW-2.5: commit refreshed Test Book reviews
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all`
- **What Was Changed (The Concrete Reality)**:
  - Commit 159 changed review PNGs and 160 new review mapping JSONs from the existing 160-page attempt
  - Record removal of the transient Stage 02.5 metrics file
  - Preserve the attempt configuration default and execution status changes accompanying the refresh
- **Architectural Rationale & Trade-Offs**:
  - The user requested committing the existing Stage 02.5 refresh after the union-only presentation change
  - Stage status records successful 02.5/02.8 refreshes and invalidation of the older 02.9 status
  - No source changes or additional conversion execution are part of this commit
- **Verification & Test Results**:
  - Inventoried 160 review PNGs and 160 mapping JSONs
  - Page-31 mapping retains prose source ordinals 4-11 as one annotation with segment IDs
  - Existing Stage 02.5 status reports success and zero model calls
  - Quick preflight passed
  - Architecture suite: 19 tests passed
  - User visual quality assessment remains separate
---

### [2026-10-07 21:59 CEST] — PDF-CALLOUT-2.83: implement Stage 02.4 advisory range recovery
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion/config.py`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Add candidate keyword discovery and one frozen-page two-image vision request per candidate-bearing page using original detail
  - Share page outline rendering with exact frame-ID legends and read-only structural anchors
  - Interpret explicit contiguous replacement ranges with deterministic identities, source geometry, continuation and lineage while retaining conflicts
  - Write complete-page sparse overrides only for changed pages and route review/filter/direct table inputs through completed optional-layer resolution
  - Preserve existing attempt model selections and add restart invalidation and stage registration
  - Carry lineage into Stage 03 metadata and normalize decorated advisory labels in Stage 09
  - Fix demonstrated duplicate callout-body rendering after Stage 09 batching and retain its regression test
  - Update converter and stage contracts while retaining the active task for user-selected fragment review
- **Architectural Rationale & Trade-Offs**:
  - Keyword hits initiate review rather than establish advisory roles
  - Source-page layout and explicit replacement mappings avoid deleting unrelated prose, listings or table/figure notes
  - Sparse overrides preserve Stage 02 evidence and filtering remains authoritative for omitted pages
  - The recovered roles need assembly that does not reread already batched body tails
- **Verification & Test Results**:
  - Quick preflight passed
  - Architecture suite passed 19/19
  - PDF execution/configuration/restart suite passed 29/29, including the duplicate-body regression that failed 2 != 1 before repair
  - Shared Codex transport/cache suite passed 21/21
  - Python compileall and both CLI help checks passed
  - Disposable synthetic fixture verified exact range splicing, retained conflicts/anchors, untouched predecessors, sparse completed/absent/failed routing, two-image original-detail cache identity and callout text/lineage through actual Stage 03-09 workers with formatting stub
  - Graphify incremental code update completed and git diff --check passed
  - No source-book inference or conversion run: user-selected fragment and quality review remain pending
  - No milestone completion or conversion accuracy claim
---

### [2026-10-07 22:11 CEST] — PDF-CALLOUT-2.83: remove aggregate decision report file
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.4_reclassify_callouts`
- **What Was Changed (The Concrete Reality)**:
  - Stop writing decision_report.json and print aggregate counters to the console instead
  - Update the stage README and remove the user-designated existing aggregate report from the pages-all attempt
- **Architectural Rationale & Trade-Offs**:
  - The user requested removal of this aggregate file
  - Keep sparse changed-page JSON output and per-page diagnostic behavior
- **Verification & Test Results**:
  - Python compilation passed
  - Existing disposable synthetic replacement/routing/03-09 check passed
  - Quick preflight passed
  - Architecture suite passed 19/19
  - No model inference or source conversion rerun
---

### [2026-10-07 22:24 CEST] — PDF-CALLOUT-2.83: close the previous Stage 02.4 execution plan
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.4-callout-reclassification.md`
  - `tools/bootstrap/pdf-to-markdown/stages/02.4_reclassify_callouts`
- **What Was Changed (The Concrete Reality)**:
  - Close and remove the previous Stage 02.4 plan at the user request
  - Retain the implemented converter and source attempt artifacts
  - Record the discussed simpler direction: original full-page image, candidate block IDs and pixel boxes, a short callout question and explicit replacement JSON without a framed companion or separate decision protocol
- **Architectural Rationale & Trade-Offs**:
  - The user withdrew the previous plan after assessing its prompt and method as too complicated
  - Closing the old plan waives its outstanding review scope rather than certifying callout recovery accuracy
  - The simpler method is a discussed direction and is not implemented by this documentation-only closure
- **Verification & Test Results**:
  - Current quick preflight passed
  - Current architecture suite passed 19/19
  - Prior implementation commit 33c5140 records 29 passing PDF tests and 21 passing transport/cache tests, not rerun for this closure
  - Previously inspected user-run pages-all artifacts showed successful execution on 160 input pages with 18 candidate objects on 14 requested pages and zero changed pages
  - No inference or conversion run in this closure
  - No milestone or classification-quality certification
---

### [2026-10-07 22:27 CEST] — PDF-CALLOUT-SIMPLE-2.4: plan simplified advisory recovery
- **Affected Subsystems**:
  - `.agent/tasks/pdf-2.4-simple-callout-recovery.md`
- **What Was Changed (The Concrete Reality)**:
  - Create a new active plan replacing the closed PDF-CALLOUT-2.83 approach
  - Specify one original page image with candidate segment IDs and containing pixel boxes, a concise callout question and replacement-only JSON
  - Remove planned frame companions and persistent diagnostics while preserving changed-page-only output, successful sparse fallback, configuration and source lineage
  - Define bounded technical verification and user-selected fragment review without changing current code
- **Architectural Rationale & Trade-Offs**:
  - Align the next implementation with the user corrections and reduce model-facing protocol complexity
  - Nearby tables or figures must not automatically exclude visibly distinct advisories
  - Preserve explicit source mapping so unrelated content stays intact
- **Verification & Test Results**:
  - Planning only: no converter implementation or source conversion run
  - Quick preflight passed
  - Architecture suite passed 19/19
  - Plan uses portable paths and English repository content
---

### [2026-10-07 22:32 CEST] — PDF-CALLOUT-SIMPLE-2.4: Simplify advisory recovery to one original image
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace frame and attachment decisions with stable segment IDs and replacement-only responses
  - Send one unmodified original-detail page image with source objects and containing pixel boxes
  - Remove inference-only frame rendering options and stage diagnostics while preserving sparse routing and review presentation
  - Regenerate only physical pages 64/130/144 through 02.4 in workspace/callouts-simple-64-130-144
- **Architectural Rationale & Trade-Offs**:
  - A nearby table or figure must not automatically suppress a visibly distinct advisory
  - Keep content geometry and lineage derived from contributors and leave unresolved or overlapping ranges unchanged
  - Preserve existing attempts and configuration and retain the plan pending user content assessment
- **Verification & Test Results**:
  - Compilation and worker CLI checks passed
  - Bounded synthetic one-image/mixed-block/no-change/sparse-routing check passed
  - 21 shared conversion technical tests and 29 retained PDF technical tests passed
  - Quick preflight and 19 architecture tests passed
  - Graphify incremental AST update completed
  - Three live requests completed in 16.31 seconds and wrote page_0064_segments.json / page_0130_segments.json / page_0144_segments.json with callout labels and bodies
  - No other source pages or later conversion stages ran and content quality remains for user assessment
---

### [2026-10-07 22:39 CEST] — PDF-CALLOUT-SIMPLE-2.4: Exclude table and image notes from callout recovery
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.4_reclassify_callouts`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Tell Stage 02.4 to preserve table/image/figure footnotes and attached explanations in their source roles even when labeled NOTE
  - Align stage documentation and the active plan with the user refinement
- **Architectural Rationale & Trade-Offs**:
  - The user explicitly limited recovery to independent advisories and excluded notes belonging to tables or images
- **Verification & Test Results**:
  - Quick preflight passed
  - Architecture suite passed 19/19
  - No source conversion or model requests ran
  - Earlier pages 64/130/144 outputs predate this prompt refinement
---

### [2026-10-07 22:47 CEST] — PDF-CALLOUT-SIMPLE-2.4: Distinguish footnotes from separate NOTE advisories
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.4_reclassify_callouts`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Narrow the prompt exclusion to numbered or symbol-marked footnotes and table/figure legends or explanatory lists
  - Recover visually distinct NOTE labels with advisory paragraphs even when their subject concerns a table or figure
  - Regenerate only pages 64/130/144 through 02.4 in workspace/callouts-distinct-note-64-130-144 and update the active plan
- **Architectural Rationale & Trade-Offs**:
  - The previous broad exclusion treated the advisory subject as disqualifying and the user reported missing NOTE objects
  - Classification follows the label and presentation instead of subject association
- **Verification & Test Results**:
  - Three fresh model requests and zero cache hits completed in 24.32 seconds
  - All three page overrides contain NOTE callout labels and callout_text bodies
  - Page 130 Notes for the above Table remains prose
  - Quick preflight and 19 architecture tests passed
  - No other source pages or later stages ran and user content assessment remains pending
---

### [2026-10-07 22:56 CEST] — PDF-CALLOUT-SIMPLE-2.4: Record user-generated callout and downstream artifacts
- **Affected Subsystems**:
  - `Obsidian/Amiga/Reference/Test Book example-4567/workspace/pages-all`
- **What Was Changed (The Concrete Reality)**:
  - Commit six sparse Stage 02.4 overrides for pages 64/73/130/144/154/156 from the user run
  - Record updated Stage 02.5 review JSONs and PNGs plus Stage 02.8 filtered objects
  - Preserve the actual attempt configuration and stage status
- **Architectural Rationale & Trade-Offs**:
  - The user requested committing the remaining artifacts after Stage 02.4
  - Source Stage 01/02 artifacts and converter implementation are unchanged
- **Verification & Test Results**:
  - Saved stage status reports success for 02.4/02.5/02.8 and 11 fresh plus 3 cached requests in the user run
  - Quick preflight passed
  - Architecture suite passed 19/19
  - Git diff whitespace check passed
  - No new source conversion or additional page quality assessment ran for this artifact commit
---

### [2026-10-07 23:02 CET] — PDF-CALLOUT-BOX-2.4: preserve distinct replacement geometry
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/common/pdf_callouts.py`
  - `Stage 02.4`
  - `reference conversion contract`
  - `tests/test_pdf_callout_geometry.py`
- **What Was Changed (The Concrete Reality)**:
  - Add the shared BBOX schema to every replacement segment
  - request tight image-derived boxes for labels, bodies and residual text
  - save model coordinates directly instead of contributor unions
  - document explicit restart for existing overrides
- **Architectural Rationale & Trade-Offs**:
  - User approved replacing shared split geometry with independent model-provided boxes so a standalone NOTE label no longer inherits its entire source paragraph box. Source IDs and lineage remain contributor-derived.
- **Verification & Test Results**:
  - Focused regression failed before repair with three identical contributor boxes and passed after repair (1 test)
  - existing PDF conversion suite passed (29 tests)
  - quick pre-flight passed
  - architecture rules passed (19 tests). No live fragment conversion or visual quality assessment performed.
---

### [2026-10-07 23:07 CET] — PDF-PAGE-SELECTION-2.4-2.5: honor configured pages on restart
- **Affected Subsystems**:
  - `PDF artifact enumeration`
  - `sparse page resolver`
  - `Stage 02.4`
  - `Stage 02.5`
  - `tests/test_pdf_review_selection.py`
- **What Was Changed (The Concrete Reality)**:
  - Filter predecessor filenames by configured physical input.pages before callout keyword collection and requests
  - pass review configuration selection through the sparse override resolver
  - preserve all-page behavior for absent or null selections
  - document selected-page restarts
- **Architectural Rationale & Trade-Offs**:
  - An all-page workspace retained every Stage 02 artifact
  - so later-stage restarts ignored --page-ranges despite the pipeline persisting the selection. Both stages now share optional selection in page-file enumeration. Existing workspace artifacts and global downstream cleanup behavior are unchanged.
- **Verification & Test Results**:
  - Both targeted regressions failed before repair by processing 63 and 64 despite selecting only 64, then passed (2 tests)
  - geometry regression passed (1 test)
  - existing PDF suite passed (29 tests)
  - quick pre-flight passed
  - architecture rules passed (19 tests)
  - graphify update completed. No live conversion or user workspace regeneration performed.
---

### [2026-10-07 23:10 CET] — PDF-PAGE-SELECTION-THROUGH-2.82: enforce page-worker selection
- **Affected Subsystems**:
  - `Stage 02`
  - `Stage 02.8`
  - `Stage 02.81`
  - `Stage 02.82`
  - `PDF pipeline README`
  - `reference conversion contract`
  - `selection regressions`
- **What Was Changed (The Concrete Reality)**:
  - Pass configured physical pages into predecessor enumeration in the four remaining page workers
  - apply Stage 02.8 selection before TOC-boundary discovery
  - preserve selected input for either table predecessor
  - record the per-stage selection contract and maintenance rule through 02.82
- **Architectural Rationale & Trade-Offs**:
  - User requested an audit through 02.82 and a durable reminder. Inspection confirmed Stage 01 already honors configured selection and Stage 00 deliberately prepares or reuses the full-source PDF. Stages 02.4 and 02.5 were repaired previously. Stage 03 creates a stream
  - stream stages do not refilter physical pages. Stage 02.9 is a separate page export outside this repair scope. Restart cleanup remains global.
- **Verification & Test Results**:
  - Four new focused regressions reproduced unwanted all-page requests/rendering and an out-of-selection TOC boundary before repair
  - all six selection tests passed after repair. Existing PDF suite passed (29 tests), including full-source OCR and selected preprocessing checks
  - callout geometry regression passed (1 test)
  - quick pre-flight passed
  - architecture rules passed (19 tests)
  - graphify update completed. No live fragment conversion, quality assessment or user workspace regeneration performed.
---

### [2026-10-07 23:14 CET] — PDF-PAGE-CLI: make physical selection invocation-scoped
- **Affected Subsystems**:
  - `PDF pipeline`
  - `page workers 01 through 02.82`
  - `selection helper`
  - `configuration template`
  - `converter documentation`
  - `PDF regression suites`
- **What Was Changed (The Concrete Reality)**:
  - Use --page-ranges as the sole selection source
  - omission selects all available predecessor pages
  - forward the parameter directly to page-worker CLIs
  - remove input.pages from the template and retire legacy selection on the next pipeline config write
  - retain source-path persistence, full-source OCR and the stream boundary
- **Architectural Rationale & Trade-Offs**:
  - User explicitly rejected controlling pages through configuration. Page ranges now apply only to the current invocation
  - for both orchestration and direct worker execution. Existing workspace data was not regenerated or modified during implementation. Earlier diary records describe the superseded persisted-selection behavior.
- **Verification & Test Results**:
  - Two restart tests reproduced persisted-selection and omitted-range defects before repair
  - updated PDF suite passed (30 tests), including worker command forwarding and real temporary-PDF restart now extracting all pages. Six page selection tests passed with conflicting legacy config and explicit page arguments, including direct review CLI omission
  - geometry regression passed (1 test). Quick pre-flight passed
  - architecture suite passed (19 tests)
  - graphify update completed. No live book fragment conversion performed.
---

### [2026-10-07 23:20 CET] — PDF-FIXED-EXECUTION: remove source and runtime configuration knobs
- **Affected Subsystems**:
  - `PDF pipeline and workers`
  - `shared Codex client`
  - `converter documentation`
  - `existing conversion tests`
- **What Was Changed (The Concrete Reality)**:
  - Require invocation-scoped --pdf for conversion and restart and pass it to Stage 02.9
  - remove persisted source configuration and source resolver
  - fix rendering at 300 DPI PNG, request timeout at 180 seconds and inference concurrency at 1
  - require completed Stage 02.8 as the table predecessor and remove the bypass switch
  - retire former fields on the next attempt config write while retaining model settings
  - update existing tests and CLI examples
- **Architectural Rationale & Trade-Offs**:
  - Source identity is supplied by the invocation. Execution constants and stage dependencies belong to code rather than attempt configuration. HTML timeout configuration remains supported. Existing book workspaces were not regenerated or edited.
- **Verification & Test Results**:
  - PDF suite passed (30 tests)
  - page selection passed (6)
  - callout geometry passed (1)
  - publication passed (10)
  - transport/configuration passed (21)
  - quick pre-flight passed
  - architecture passed (19)
  - changed Python files parsed and diff check passed
  - graphify update completed. No live fragment conversion or quality assessment performed.
---

### [2026-10-07 23:21 CET] — PDF-FIXED-OCR: remove OCR configuration knobs
- **Affected Subsystems**:
  - `PDF Stage 00`
  - `pipeline configuration`
  - `converter documentation`
- **What Was Changed (The Concrete Reality)**:
  - Remove the OCR section from the template and retire it from existing attempt configs on the next pipeline write
  - keep eng in a private Stage 00 constant
  - discover Tesseract data through PyMuPDF and TESSDATA_PREFIX
  - fingerprint fixed rendering and language settings for OCR recovery
  - update documentation
- **Architectural Rationale & Trade-Offs**:
  - OCR language and data discovery are execution policy rather than YAML options. The existing prepared PDF reuse rule remains intact.
- **Verification & Test Results**:
  - Existing PDF suite passed (30 tests)
  - quick pre-flight passed
  - architecture rules passed (19 tests)
  - diff check passed
  - graphify update completed. Verified installed PyMuPDF discovery implementation. No live fragment conversion or existing book workspace modification.
---

### [2026-10-07 23:25 CET] — PDF-2.4-REFINE: record regenerated callout artifacts and close the plan
- **Affected Subsystems**:
  - `Test Book attempt pages-all`
  - `Stage 02.4 overrides`
  - `Stage 02.5 reviews`
  - `Stage 02.8 filtered objects`
  - `execution plan`
- **What Was Changed (The Concrete Reality)**:
  - Record refined callout geometry for pages 64, 73, 130, 144, 154 and 156 with corresponding review and filtered artifacts
  - retain updated attempt configuration and execution status
  - remove the completed pdf-2.4-simple-callout-recovery plan
- **Architectural Rationale & Trade-Offs**:
  - User requested committing the remaining 2.4 refinement work. Converter implementation and CLI-only configuration changes are already committed. Preserve the regenerated attempt artifacts as the concrete refinement record and close the active plan
  - content fidelity remains subject to user assessment.
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture rules passed (19 tests). Existing attempt status records success through 02.8 and 14 cached Stage 02.4 requests with zero fresh model calls. This commit turn did not rerun conversion or certify output fidelity. Existing technical test evidence is retained in earlier diary entries.
---

### [2026-10-07 23:31 CET] — PDF-TABLE-2.41: Split grouped table objects with original-page vision
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion/config.py`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Add per-table Stage 02.41 vision requests using complete unmodified source pages and target JSON
  - preserve singleton responses and assign deterministic split IDs with lineage
  - share ordered 02.4/02.41 sparse override resolution and failed-status blocking
  - register selection, configuration, metrics and restart cleanup
  - update stage contracts and existing restart expectations
- **Architectural Rationale & Trade-Offs**:
  - Separate independent tables before crop transcription while preserving source page objects and the developer-led conversion workflow
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture 19/19
  - existing PDF execution/restart 30/30, page selection 6/6, callout geometry 1/1 and transport/cache 21/21
  - Python compilation and diff checks passed
  - Graphify incremental update passed
  - own-layer exclusion and failed retained-layer blocking verified. Stage 02.41 completed on physical pages 50 and 54 in workspace/table-split-50-54 with 4 live calls in 28.74 seconds: page 50 tables 1 to 2, page 54 tables 3 to 5. Selected predecessor hashes and non-table objects unchanged. Initial pages-all restart failed before inference on a locked review directory
  - all changes to that attempt were restored from its clean Git baseline. Later conversion stages were not run
  - split quality awaits user assessment. Closes execution plan PDF-TABLE-2.41
  - source copies and generated results remain local and are not committed.
---

### [2026-10-07 23:36 CET] — PDF-TABLE-2.41: Prohibit model-inferred table continuation
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.41_split_tables`
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Require continuation false in the Stage 02.41 prompt and response schema
  - force false when writing split tables
  - document unchanged singleton predecessor flags and explicit regeneration of existing artifacts
- **Architectural Rationale & Trade-Offs**:
  - Table boundary splitting must not invent continuation relationships
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture 19/19
  - Python compilation and focused schema/application assertions passed, including a mocked true response forced to false for split children and unchanged singleton source
  - Graphify update passed
  - diff check passed. Existing page artifacts were not rewritten and no conversion was requested or run.
---

### [2026-10-07 23:39 CET] — PDF-TABLE-2.41: Allow model-assessed continuation for split tables
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.41_split_tables`
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/reference-conversion-contract.md`
- **What Was Changed (The Concrete Reality)**:
  - Restore boolean continuation responses and remove forced false from split outputs
  - explicitly ask vision to mark any split-out table as a continuation when it judges a relationship, including side-by-side table parts
  - update stage contracts and cache regeneration guidance
- **Architectural Rationale & Trade-Offs**:
  - Apply the user decision to assess continuation for each split table using full-page context
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture 19/19
  - Python compilation and focused assertions passed: boolean schema accepts both values and split outputs preserve independent false/true model decisions without mutating the source
  - Graphify update and diff checks passed. Singleton responses retain predecessor objects. Existing attempt artifacts were not regenerated
  - no fragment conversion was requested.
