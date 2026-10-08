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

### [2026-09-16 23:59 CEST] — Historical Digest: Reference Conversion and Searchable Hardware Knowledge
- **Timestamp & Context**: 2026-09-15 to 2026-09-16. Conversion infrastructure was followed by diagram descriptions and local retrieval.
- **Affected Subsystems**: Historical PDF converter, reference vault, Qdrant/RAG tooling and agent rules.
- **What Was Changed**:
  - Built the original 14-stage multimodal pipeline with Gemini OCR, normalized bounds, graphic triage, HTML table spans and TOC extraction. Stage workspaces supported bounded context, fail-fast execution and recovery.
  - Added searchable diagram sidecars and hardware/design indexing. Schematics retained textual descriptions; register layouts used ASCII/GFM representations for retrieval.
  - Established root-cause resolution and practitioner documentation rules alongside the ingestion workflow.
- **Architectural Rationale & Trade-Offs**: Prepare technical sources once, preserve their relationships in text and retrieve focused hardware context. Staged conversion separates model requests and recovery; sidecars expose visual circuit information to text search. Later converter and RAG changes are recorded at their own dates.
- **Verification & Invariants**: The historical log reported validation on 20 reference pages with 100% table/diagram fidelity, offline CLI retrieval checks and sidecar validation. This preserves the original sample claim, not acceptance evidence for later converters or all books.

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

### [2026-09-21 23:59 CEST] — Historical Digest: Executable Quality Rules and CPU Specification Alignment
- **Timestamp & Context**: 2026-09-19 to 2026-09-21. Quality/compiler foundations were followed by CPU cleanup and bidirectional specification review.
- **Affected Subsystems**: Workspace configuration, CPU, harness audits, design specifications, agent rules and roadmap.
- **What Was Changed**:
  - Renamed m68000 to cpu, standardized crate roots/re-exports and established code, hardware and documentation audits. Workspace warnings became errors; selected Clippy checks, boolean naming and short-circuit conventions formed the compiler baseline.
  - Consolidated prefetch storage to one u16 with explicit two-word pipeline dynamics. Replaced mechanical OOP accessors with register/slice operations and clarified restore/stack handling through a_long(7), usp and ssp.
  - Reviewed CPU specifications against code, removed ghost features, clarified Fast RAM TAS behavior and consolidated platform quirks. Scope discipline and a pending-only roadmap made implementation boundaries explicit.
- **Architectural Rationale & Trade-Offs**: Compile-time and executable checks catch structural violations; hardware registers remain transparent data rather than acquiring accessor boilerplate. Semantic review complements mechanical checks by explaining actual CPU behavior and boundaries.
- **Verification & Invariants**: Historical records reported all 23 crates warning/Clippy-clean; documentation/code/hardware audit pillars passed 10/10, 7/7 and 5/5. The later review reported architecture 21/21 and full SingleStepTests passing. These are dated results; subsequent regressions and verification limits remain in later entries.

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

### [2026-09-29 01:50 CEST] — Historical Diary Compaction
- **Affected Subsystems**: DIARY.md and diary/roadmap maintenance guidance.
- **What Was Changed**: Consolidated more than 340 September 12-22 entries into ten digests at the time, reducing roughly 8,995 lines/930 KB to 620 lines/75 KB. Retained the then-active MachineLoop review and established milestone compaction guidance.
- **Architectural Rationale & Trade-Offs**: Preserve decisions and historical verification while leaving transient micro-diffs in Git. Later compaction may combine related digests without changing event order.
- **Verification & Invariants**: Historical quick pre-flight and architecture 20/20 passed.

---

### [2026-10-05 21:57 CEST] — Historical Digest: Agent Guidance and Procedure Consolidation
- **Timestamp & Context**: 2026-10-05, 21:33 to 21:57. One sequence of instruction and tooling cleanup.
- **Affected Subsystems**: Agent rules/skills, AGENTS.md, RAG adapter, harness checks and developer documentation.
- **What Was Changed**:
  - Shortened shared guidance, clarified per-commit logging and milestone compaction, aligned checkpoints/links and removed forced source/test coupling and assertion quotas.
  - Bounded RAG retrieval through source selection, read-only metadata and the canonical CLI/MCP adapter; delegated indexing to its existing skill and removed the retired search stub.
  - Condensed clean-break refactoring around complete internal migrations and explicit compatibility decisions, then separated concise policy rules from skill-owned procedures/examples. Retained 27 rules and mandatory Graphify navigation while removing its redundant redirect.
- **Architectural Rationale & Trade-Offs**: Reduce duplicated instruction context while preserving hardware/golden-reference invariants, scope control and external/saved-data contracts. Keep obligations in rules, procedures in skills and mechanical coverage bounded to what checks observe.
- **Verification & Invariants**: Historical quick gates passed throughout; architecture passed 20 tests initially and 19 after fixture changes. RAG tests passed 9 and the native MCP launch regression passed; harness fixtures grew from 32 to 41 passing tests. Selected governance/link/size audits passed, including 66 new links, TOML parsing and Python compilation. Earlier full-documentation audits still reported 9 then 8 findings; selected audit success did not close them. No emulator runtime source changed or full domain verification ran.

---

### [2026-10-05 22:14 CEST] — Historical Digest: Roadmap Scope and Priority Consolidation
- **Timestamp & Context**: 2026-10-05, 22:11 to 22:14. Backlog restructuring followed by narrowing the regeneration experiment.
- **Affected Subsystems**: ROADMAP.md, roadmap rules/skill and review procedure.
- **What Was Changed**: Prioritized manual conversion/ingestion, test orientation/profiling and chipset reconstruction; removed historical summaries and duplicate methodology while retaining deferred harness/eval capabilities. Limited Step 10 to the selected isolated documentation-to-code regeneration experiment, preserving its unit/integration gate.
- **Architectural Rationale & Trade-Offs**: Keep pending work readable without losing source-fidelity requirements or acceptance gates. Surfaced the proposed chipset reset's conflict with timing/quirks invariants; restructuring neither authorizes that deviation nor completes implementation milestones.
- **Verification & Invariants**: Historical quick and architecture checks passed (19/19); governance/MCP/discovery regressions passed 19 tests and documentation audit reported zero issues. Skill, language, numbering and local-link checks passed; the final backlog held 11 steps and 40 tasks. No regeneration experiment, subsystem semantic-parity review or milestone completion was performed.

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

### [2026-10-06 01:12 CEST] — Historical Digest: Codex Conversion and PDF Text Preparation
- **Timestamp & Context**: 2026-10-05 to 2026-10-06. Early migration and PDF-TEXT-1.1 closure; later digests record the replacement of experimental mechanisms.
- **Affected Subsystems**: HTML/PDF converters, shared Codex transport/cache, bootstrap documentation and conversion plans.
- **What Was Changed**: Replaced Gemini calls with shared subscription-authenticated Codex transport and explicit stage model/effort settings. Established positioned text JSON and matching PNG inputs, isolated attempts, restart handling and response reuse. Removed redundant configuration and temporary test trackers.
- **Architectural Rationale & Trade-Offs**: Converter development follows a requested change, conversion of a user-selected fragment, then user assessment. Existing technical tests protect transport and execution; they do not certify reconstruction quality. Independent full-book pilots, quality matrices and routine crop-test expansion were excluded.
- **Verification & Invariants**: Historical page-19 pilot completed the then-current 14-stage pipeline; replay used cache with zero new requests. PDF-TEXT-1.1 (`5a9add8`) was closed by user decision using page-64 OCR/render/geometry evidence. Native-copy, mixed, blank and real rotated/cropped categories remained unverified. Closure checks passed PDF/shared-client suites at 19/19 each; the milestone gate failed on 62 existing reference links and 22 unchanged Rust compound conditions, with architecture 18/19. This was scope closure, not certification of a repository milestone or later pipeline versions.

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

### [2026-10-07 18:43 CEST] — Historical Digest: Bootstrap Publication and PDF Execution Model
- **Timestamp & Context**: 2026-10-06 to 2026-10-07. Settled execution decisions from the continuing book-reconstruction work.
- **Affected Subsystems**: Bootstrap dispatcher, shared publication, HTML/PDF converters, Stage 00/01 preparation and attempt orchestration.
- **What Was Changed**: Downloads use per-book temporary directories; publication is explicit, accepts only absent/empty destinations and preserves working sources. HTML transcription includes linked assembly sources. PDF preparation moved from model OCR and coordinate reconstruction to local Tesseract through PyMuPDF, retaining its original text geometry over source graphics. Stage 00 prepares/reuses a full-document sibling OCR PDF; downstream selection keeps physical source page identities.
- **Architectural Rationale & Trade-Offs**: Named attempts live beside the source and preserve an existing attempt configuration. PDF-STATE-1 replaced manifests, snapshots, inventories, local validators and manual handoffs with stage-owned artifacts and atomic execution status. Restart explicitly clears the selected stage and all successors; successful execution is not artifact or fidelity certification. HTML retained its separate checks. Retired segmentation branches, duplicate dumps and one-time migration tools were removed rather than maintained as compatibility layers.
- **Verification & Invariants**: At PDF-STATE-1 closure, historical offline PDF/shared/publication suites passed 25/25, 21/21 and 10/10, with quick pre-flight and architecture 19/19. The selected attempt migration preserved 645 result files without OCR or model calls. Publication safeguards passed offline tests; this did not demonstrate bulk publication, a complete book reconstruction or RAG ingestion. Earlier preparation validation and lineage mechanisms were subsequently removed by user direction; their historical test results do not describe current safeguards.

---

### [2026-10-08 17:36 CEST] — Progress Digest: Page Reconstruction, Tables and Visual Review
- **Timestamp & Context**: 2026-10-07 to 2026-10-08, including earlier page-conversion groundwork. Ongoing converter refinement; repeated prompts, stage renames and TestBook reruns remain in Git rather than the diary.
- **Affected Subsystems**: PDF page conversion/refinement, source filtering, table transcription/review, Markdown export and stream assembly.
- **What Was Changed**: Added page-object reconstruction and optional refinements for callouts, table splits/classification, code classification/formatting, image rotation, reading order and graphic/prose separation. Review uses original-page geometry and resolved objects. Invocation-scoped physical page selection applies to page workers. Fractional stage IDs use two digits; obsolete later stages were removed.
- **Architectural Rationale & Trade-Offs**: Preserve source order, technical content and meaningful graphic groupings; extract only independent descriptions. PDF-ORDER-2.46 closed at user request without certifying all reading-order observations. Deliberate filtering removes headers/footers, pre-TOC content, list/index pages and matching standalone NXP logos. Table transcription uses exact source crops, HTML spans for merged cells, deterministic empty-row cleanup and separate source/converted review. Both table formats retain source-image companions; only HTML receives saved RAG text. Review rendering and execution success remain separate from user fidelity acceptance.
- **Verification & Invariants**: Historical user-selected fragments exercised these stages and produced retained review/export artifacts; prompt changes do not establish universal quality. Recent technical checks reached PDF execution 35 tests, review/selection 9 tests and architecture 19 tests, with quick pre-flight passing. Graphic pages 21 and 35 demonstrated retained coordinated descriptions versus independent prose extraction, still subject to user assessment. Bulk conversion, full downstream assembly acceptance and RAG ingestion remain pending; current implementation contracts live in converter documentation. Temporary reading-order diagnostics were removed; applied page overrides remain.

---

### [2026-10-08 17:55 CEST] — Historical Digest: Compact Artifacts and Response-Cache Identity
- **Timestamp & Context**: 2026-10-08. Settled JSON/cache changes and user-approved TestBook snapshots.
- **Affected Subsystems**: PDF artifact IO, shared request cache, replacement handling and tracked TestBook outputs.
- **What Was Changed**: Adopted compact objects that omit defaults and retired replacement/lineage/image-only metadata without injecting defaults into model prompts. Preserved Stage 01 OCR input and downstream saved code Markdown. Temporary dual-key migration scaffolding was removed after cache preparation. Returning meaningful fields are inserted at response-contract positions so equivalent replacements retain request serialization order.
- **Architectural Rationale & Trade-Offs**: Cache identity depends on exact prompt bytes as well as model, effort, schema and images. Equal JSON values with different field order caused avoidable misses; changed page geometry creates a genuinely different request. One-time aliases were limited to verified equivalent requests. The user chose to retain new page-54 geometry and its legitimate miss. Artifact snapshots record approved changes and review-directory migrations, not a complete pipeline checkpoint.
- **Verification & Invariants**: Historical PDF/shared/review/callout suites passed 35/21/8/1 tests; compact cache replay made zero model calls. Stage 02.42 reconstruction found 138 hits and zero misses after equivalent-key repairs. Later Stage 02.44 audit found 81 hits and one legitimate page-54 miss, blocking that page's downstream cache audit at the time. User-approved snapshots verified equivalent JSON changes and unchanged PNG bytes where claimed; the later snapshot included reviewed differences. Quick pre-flight and architecture 19/19 passed for these changes. No bulk reconstruction or source-fidelity certification follows from cache hits or committed artifacts.

---

### [2026-10-08 18:15 CEST] — PDF-ORDER-2.48: Plan deterministic ordering within page rows
- **Affected Subsystems**:
  - `.agent/tasks/PDF-ORDER-2.48.md`
- **What Was Changed (The Concrete Reality)**:
  - Record a planned Stage 02.48 after 02.47 and before 02.50
  - define anchored inclusive 7-pixel top-coordinate groups of arbitrary size
  - preserve objects and unrelated array positions
  - record the pending X-versus-Y clarification with literal Y as the current request
- **Architectural Rationale & Trade-Offs**:
  - Use deterministic geometry for the requested row ordering
  - keep planning separate from implementation and user-requested conversion
  - page 81 remains an unresolved user quality observation
- **Verification & Test Results**:
  - Read-only grouping measurement completed for physical pages 81 and 90 and 91
  - quick pre-flight passed
  - architecture suite passed 19 tests
  - no converter implementation or model calls or conversion runs
---

### [2026-10-08 18:19 CEST] — Book-Reconstruction Diary Compaction
- **Affected Subsystems**: DIARY.md.
- **What Was Changed**: Replaced 176 converter/TestBook entries with four historical/progress digests, preserving unrelated history and active PDF-ORDER-2.48.
- **Architectural Rationale & Trade-Offs**: Keep durable decisions and evidence limits while omitting repeated experiments; conversion work remains ongoing.
- **Verification & Invariants**: Quick pre-flight, architecture 19/19, preservation and whitespace checks passed. Full-file language scan retained two existing findings; staged additions passed. No conversion or milestone acceptance ran.

---
---

### [2026-10-08 18:22 CEST] — Consolidate related historical diary entries
- **Affected Subsystems**:
  - `DIARY.md`
- **What Was Changed (The Concrete Reality)**:
  - Merge eleven adjacent entries into four chronological digests
  - shorten two compaction records
- **Architectural Rationale & Trade-Offs**:
  - Group related engineering decisions rather than minor task numbers while retaining date ranges and historical verification limits
- **Verification & Test Results**:
  - Quick pre-flight passed
  - architecture 19/19 passed
  - untouched-entry preservation and chronological-order assertions passed
  - scoped whitespace check passed
---

### [2026-10-08 18:22 CET] — PDF-ORDER-2.48: Implement deterministic ordering within page rows
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Add JSON-only Stage 02.48 after 02.47 with anchored inclusive 7-pixel top-coordinate grouping for every object type
  - sort each group by ascending Y with stable ties in its original occupied slots, preserving IDs, original objects and resolved page metadata
  - integrate sparse resolution, registry, page selection, inspection and restart cleanup without inference settings
  - update stage and consumer documentation and retained restart expectations
  - close PDF-ORDER-2.48 and remove its execution plan in the closing workflow
- **Architectural Rationale & Trade-Offs**:
  - Implement the plan's literal Y axis because no X clarification was received
  - preserve sparse predecessor resolution and downstream array order
  - shared artifact IO keeps existing compact-object semantics
  - page 81 remains an unresolved user quality observation and left-to-right ordering is not implied by Y
- **Verification & Test Results**:
  - Eight disposable geometry examples passed: arbitrary group size, inclusive 7-pixel boundary, 8-pixel exclusion, fixed anchors without chaining, stable ties, unrelated slots, object preservation and idempotence
  - synthetic orchestrator checks passed for JSON-only selected-page processing, resolved 02.47 input, metadata preservation, zero model calls, incomplete-status blocking and restart removing an obsolete override on a no-change rerun
  - the first disposable check invocation used incorrect config-loader arguments and was corrected before passing
  - PDF conversion Codex 35 tests passed
  - PDF review selection 9 tests passed
  - quick pre-flight passed
  - architecture rules 19 tests passed
  - Graphify incremental AST refresh completed
  - new documentation links resolve
  - no book conversion, live inference or emulator domain changes
---

### [2026-10-08 18:28 CEST] — PDF-ORDER-2.48: Sort row objects left to right by X
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.48_order_page_rows`
  - `PDF conversion documentation`
  - `tests/test_pdf_review_selection.py`
- **What Was Changed (The Concrete Reality)**:
  - Change the within-row sort key to bbox[0] with predecessor-position ties
  - retain anchored inclusive 7-pixel Y grouping and original occupied JSON slots
  - update active documentation to X ordering
  - retain one regression covering page-64 geometry, equal-X stability, unrelated slots and JSON field preservation
- **Architectural Rationale & Trade-Offs**:
  - The user explicitly corrected the requested axis from Y to X after page 64 placed the right-hand prose before its left heading
  - group by Y but order each group's JSON objects from left to right
- **Verification & Test Results**:
  - Focused regression failed with Y ordering before the change and passed with X
  - PDF review selection 10 tests passed
  - eight disposable geometry examples and synthetic orchestrator/restart/status/selection checks passed
  - read-only page-64 predecessor check confirms left heading precedes right prose
  - quick pre-flight passed
  - architecture rules 19 tests passed
  - Graphify AST refresh completed
  - existing book conversion and review artifacts were not regenerated or staged
  - no live model calls
---

### [2026-10-08 18:40 CEST] — PDF-ORDER-2.48: Remove deterministic row ordering at user request
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `PDF converter tests`
  - `Test Book pages-all workspace`
- **What Was Changed (The Concrete Reality)**:
  - Remove Stage 02.48 worker and README, registry entry, sparse resolver layer and page-range routing
  - remove its regression and update retained restart expectations
  - remove active documentation and temporary verification script
  - delete the two generated row-order overrides on pages 63 and 144 and remove stage status
  - restore page-144 review and its recorded checkpoint to the committed pre-stage versions
- **Architectural Rationale & Trade-Offs**:
  - The user requested removal after strict coordinate ordering inverted NOTE and its body on page 144
  - resume predecessor order through 02.47 before review/filtering
  - retain historical diary records
- **Verification & Test Results**:
  - PDF review selection 9 tests passed
  - PDF conversion Codex 35 tests passed
  - quick pre-flight passed
  - architecture rules 19 tests passed
  - active code/test/documentation search found no 02.48 references
  - removed CLI stage lookup and predecessor resolution for pages 63 and 144 verified
  - page-144 current and committed review JSON and PNG pixels matched the corresponding ordered and predecessor inputs before restoration
  - Graphify AST refresh completed
  - no book conversion or live inference
---

### [2026-10-08 20:04 CEST] — BOOK-RECIPE-PROOFREAD-1: Add chapter proofreading instructions
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Create the tool directory with a starter README
  - Adapt Stage 09 and Stage 10 prompts and worker behavior into a manual chapter proofreading instruction
  - Preserve recipe-owned structure and require evidence for technical OCR substitutions
- **Architectural Rationale & Trade-Offs**:
  - Prepare the optional agent editing pass before implementing deterministic chapter generation
  - Keep inferred naming and legacy stage orchestration outside the chapter prompt
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture rules 19 tests passed
  - English-language scans passed
  - All new relative documentation links resolve
  - No live proofreading or book conversion was run
---

### [2026-10-08 20:14 CEST] — BOOK-RECIPE-PROOFREAD-2: Refine chapter inputs and HTML formatting
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Use Markdown-linked images only when proofreading requires inspection
  - Remove supplied-source and frontmatter instructions plus crop and rotation metadata references
  - Allow inline-code emphasis for technical terms
  - Remove run-in heading formatting and add HTML code math and escaping rules
  - Align README usage with the prompt
- **Architectural Rationale & Trade-Offs**:
  - Follow the user-defined chapter-only agent workflow and preserve the generator-owned layout
  - Respect different inline syntax in raw HTML
- **Verification & Test Results**:
  - Quick pre-flight passed
  - Architecture rules 19 tests passed
  - English-language scans and relative links passed
  - Whitespace check passed
  - No live proofreading run
---

### [2026-10-08 20:47 CET] — Remove obsolete PDF conversion stages
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/conversion`
  - `tools/bootstrap/book-to-markdown`
  - `tests`
  - `docs/developers.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove stages 02.40-02.50, 02.82, 02.83, 03, 04, 05, 09, 10, 11 and 14 with their prompts, helpers and obsolete tests
  - use explicit 02 to 02.80 to 02.81 to 02.90 handoffs
  - publish the final page export
  - update configuration, dependencies and documentation
- **Architectural Rationale & Trade-Offs**:
  - User requested pipeline simplification and explicitly included dependent stage 04
  - retain the initial preparation, filtering, table conversion and page export path
- **Verification & Test Results**:
  - 58 converter and publication tests passed
  - 3 page selection tests passed
  - synthetic Stage 02.81 to 02.90 export copied the saved table asset and retained its source companion
  - quick preflight passed
  - architecture suite passed 19 tests
  - Graphify incremental refresh completed
  - no source fragment or full-book conversion requested or run
---

### [2026-10-08 20:52 CET] — Add draft table and image description prompts
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Add separate prompts for describing prepared table and image groups
  - document supplied details sections with names, notes, legends and Markdown or HTML tables
  - return descriptions for insertion without altering source content
- **Architectural Rationale & Trade-Offs**:
  - Use user-defined grouping as context for preliminary description prompts rather than rediscovering associations
- **Verification & Test Results**:
  - Quick preflight passed
  - architecture suite passed 19 tests
  - prompt and README references reviewed
  - no model requests, fragment conversions or automatic insertion implemented
---

### [2026-10-08 20:55 CET] — Limit Stage 02.81 to table markup replacement
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_review_selection.py`
- **What Was Changed (The Concrete Reality)**:
  - Use one transcription request per table and temporary crops
  - remove table group inference, RAG text, persistent source companions and their prompt
  - emit converted table markup directly in Stage 02.90
  - crop only raster fallbacks
  - update documentation and the existing HTML selection regression
- **Architectural Rationale & Trade-Offs**:
  - User restricted Stage 02.81 to replacing a table with Markdown or HTML
  - grouping and descriptions belong to later book processing
- **Verification & Test Results**:
  - 58 converter/publication tests and 3 selection tests passed
  - HTML regression confirmed one request, preserved markup, no companion fields or persistent request assets and no details in page export
  - quick preflight passed
  - 19 architecture tests passed
  - Graphify refreshed
  - no source fragment conversion requested or run
---

### [2026-10-08 20:58 CET] — Define draft book element vocabulary
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Add an element list based on the Stage 02 source vocabulary
  - define image_note and image_legend as separate image-associated source blocks
  - link the catalog from the book tool README
- **Architectural Rationale & Trade-Offs**:
  - User requested an initial element catalog for book authoring with explicit image note and legend roles
- **Verification & Test Results**:
  - Verified all 23 Stage 02 types plus the two requested image roles and resolved local links
  - quick preflight passed
  - architecture suite passed 19 tests
  - PDF schema and runtime unchanged
  - no conversion requested or run
---

### [2026-10-08 21:03 CET] — Use generic book note and legend types
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Replace table_legend, image_note and image_legend in the book vocabulary with note and legend
  - define authored group membership independently of type
  - allow standalone notes in chapter text
  - update the README
- **Architectural Rationale & Trade-Offs**:
  - User requested shared note and legend roles without table or image specialization
- **Verification & Test Results**:
  - Reviewed the catalog and README for consistent terminology
  - quick preflight passed
  - architecture suite passed 19 tests
  - PDF response schema and runtime unchanged
---

### [2026-10-08 21:04 CET] — Represent book headings with explicit hierarchy levels
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Use heading for chapter, appendix and section titles
  - define level as hierarchy depth and optional number as source numbering
  - document numbered and unnumbered headings in the catalog and README
- **Architectural Rationale & Trade-Offs**:
  - User accepted heading plus level instead of chapter types encoding depth
- **Verification & Test Results**:
  - Documentation diff reviewed
  - quick preflight passed
  - architecture suite passed 19 tests
  - PDF schema and runtime unchanged
---

### [2026-10-08 21:05 CET] — Treat dedicated book heading roles as level one
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Assign level-one treatment to toc_heading, index_heading, list_of_tables_heading and list_of_figures_heading
  - retain semantic types while sharing heading hierarchy and Markdown rendering behavior
  - update the catalog and README
- **Architectural Rationale & Trade-Offs**:
  - User specified that these dedicated headings should be treated like heading at level one
- **Verification & Test Results**:
  - Documentation diff reviewed
  - quick preflight passed
  - architecture suite passed 19 tests
  - no runtime or PDF schema change
---

### [2026-10-08 21:06 CET] — Separate book code listings from preformatted text
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown`
- **What Was Changed (The Concrete Reality)**:
  - Restrict code_block to program and command listings
  - add preformatted_text for ASCII art, dumps and aligned source layouts
  - specify text fences and preservation of original character spacing and line breaks
  - update the README
- **Architectural Rationale & Trade-Offs**:
  - User distinguished source code from fixed-width source material that must retain its original layout
- **Verification & Test Results**:
  - Catalog and proofreading instructions reviewed for consistency
  - quick preflight passed
  - architecture suite passed 19 tests
  - no runtime or PDF schema change
---

### [2026-10-08 21:07 CET] — Align book caption terminology with graphic elements
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/elements.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace figure with graphic in the caption definition and its explanatory paragraph
- **Architectural Rationale & Trade-Offs**:
  - User requested terminology consistent with the graphic element type
- **Verification & Test Results**:
  - Diff reviewed
  - quick preflight passed
  - architecture suite passed 19 tests
---

### [2026-10-08 21:09 CEST] — Align page conversion prompt with the current book element vocabulary
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion/prompt.md`
- **What Was Changed (The Concrete Reality)**:
  - Replace chapter with heading plus level and optional number
  - distinguish code_block from preformatted_text
  - replace table_legend with generic note and legend roles
  - set dedicated listing headings to level one
- **Architectural Rationale & Trade-Offs**:
  - User explicitly selected prompt-only alignment
  - the unchanged PDF response schema still uses older types and heading_level and remains incompatible with the revised prompt
  - schema and downstream migration are outside this task
- **Verification & Test Results**:
  - Prompt vocabulary matches all 24 catalog types
  - quick preflight passed
  - architecture suite passed 19 tests
  - no fragment conversion requested or run
  - vocabulary-check command corrected for shell quoting and to exclude the separate heading-example table before passing
---

### [2026-10-08 21:14 CEST] — Simplify page classification for manual editor review
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02_page_conversion/prompt.md`
- **What Was Changed (The Concrete Reality)**:
  - Reduce prompt from 284 to 84 lines
  - remove cover and contents/index/table-list/figure-list classification
  - define ordered checks starting with headers and footers then callouts and separate annotations
  - identify tables before graphics and leave prose as the final fallback
- **Architectural Rationale & Trade-Offs**:
  - User will assign document-specific roles and correct approximate classification in a manual editor
  - classification priority is explicit and separate from output reading order
  - preserve the previously selected prompt-only scope and unchanged legacy response schema
- **Verification & Test Results**:
  - Classification order and excluded roles reviewed
  - git diff --check passed
  - quick preflight passed
  - architecture suite passed 19 tests
  - no conversion fragment requested or run
---

### [2026-10-08 21:16 CEST] — Limit automatic PDF filtering to page furniture and the fixed NXP logo
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.80_filter_page_content`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_review_selection.py`
- **What Was Changed (The Concrete Reality)**:
  - Remove pre-TOC boundary deletion and whole-page index/list exclusions
  - retain editor-assigned roles even in older artifacts
  - preserve header/footer and fixed NXP matching removals
  - update the stage README and shared documentation
  - adapt the existing selection regression to verify retained roles and removed page furniture
- **Architectural Rationale & Trade-Offs**:
  - The simplified Stage 02 prompt leaves cover and contents/index/list classification to later manual editing
  - filtering must preserve that content rather than delete it based on legacy roles
- **Verification & Test Results**:
  - Three PDF selection and stage-consumer tests passed
  - quick preflight passed
  - architecture suite passed 19 tests
  - git diff --check passed
  - Graphify incremental code update completed
  - no source conversion requested or run
---

### [2026-10-08 21:17 CEST] — Replace fixed NXP image exclusion with margin-logo classification
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/reference-conversion-contract.md`
  - `docs/developers.md`
  - `tests/test_pdf_conversion_codex.py`
- **What Was Changed (The Concrete Reality)**:
  - Delete the NXP template and all image-comparison code
  - filter only header and footer types without reading images
  - classify recurring margin logos and publisher branding as header/footer in the page prompt
  - remove the obsolete logo geometry test and update documentation and CLI stage description
- **Architectural Rationale & Trade-Offs**:
  - User removed the fixed logo-elimination mechanism
  - page furniture classification handles margin branding while graphics remain for manual editing
- **Verification & Test Results**:
  - Three PDF selection tests and 26 PDF conversion tests passed
  - quick preflight passed
  - architecture suite passed 19 tests
  - Graphify incremental code refresh completed
  - no source conversion run and prompt effectiveness remains unevaluated
---

### [2026-10-08 22:33 CET] — Save only changed PDF table pages
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tests/test_pdf_review_selection.py`
  - `tests/test_pdf_conversion_codex.py`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Stage 02.81 saves full page JSON only when table processing changes its data
  - Stage 02.90 overlays changed pages on selected filtered input
  - persist the table-stage invocation range in stage_status.json
  - retain historical full-dataset export behavior
- **Architectural Rationale & Trade-Offs**:
  - Avoid copying pages without table changes while preserving unchanged selected pages and fragment boundaries in final export
- **Verification & Test Results**:
  - Reproduced redundant page writes and missing sparse export pages before repair
  - 31 PDF selection/orchestration tests pass
  - pre_flight.py --quick passes
  - all 19 architecture tests pass
  - git diff --check passes
  - no live book rerun requested or performed
---

### [2026-10-08 22:34 CET] — Show live PDF table transcription progress
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/stages/02.81_transform_page_tables`
  - `tools/bootstrap/pdf-to-markdown/pipeline.py`
- **What Was Changed (The Concrete Reality)**:
  - Flush table request start and completion messages immediately with segment identity, page-local position, format and elapsed time
  - log changed, unchanged and table-free pages
  - flush stage banners before starting workers
- **Architectural Rationale & Trade-Offs**:
  - Make long table inference visible in the console instead of waiting for the final stage summary
- **Verification & Test Results**:
  - 31 existing PDF tests pass and exercise progress output
  - quick preflight passes
  - 19 architecture tests pass
  - Graphify update completes
  - no live book rerun performed
---

### [2026-10-08 22:38 CET] — Remove PDF page Markdown export stage
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/bootstrap_documentation.ps1`
  - `tools/bootstrap/book-to-markdown`
  - `tests`
  - `docs/developers.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Remove Stage 02.90 and unused page Markdown/crop emitter and overlay loader
  - end PDF preparation at 02.81
  - remove PDF publication option and reject bootstrap PDF publication before downloads
  - update tests, documentation and book recipe plan for direct JSON/image handoff
- **Architectural Rationale & Trade-Offs**:
  - Prepare source objects and transcribed tables for separate chapter generation rather than emitting an intermediate page-ordered Markdown document
- **Verification & Test Results**:
  - 42 relevant PDF, publication and bootstrap tests pass
  - quick preflight passes
  - all 19 architecture tests pass
  - PDF CLI help confirms publication removal
  - bootstrap PDF publication rejects before download as expected
  - Graphify refresh completes
  - existing workspace exports are preserved
  - book generator remains unimplemented
---

### [2026-10-08 22:42 CET] — Separate HTML-only conversion elements from PDF infrastructure
- **Affected Subsystems**:
  - `tools/bootstrap/conversion`
  - `tools/bootstrap/pdf-to-markdown/common`
  - `tools/bootstrap/html-to-markdown/support`
  - `converter entry points and requirements`
  - `tests`
  - `docs/developers.md`
- **What Was Changed (The Concrete Reality)**:
  - Move HTML YAML loader, config validation, model catalog checks, text requests and publication into HTML support
  - keep generic Codex execution, completion, cache identity and metrics shared
  - move direct PDF config parsing into PDF common
  - remove unused vision method, JSON schema enforcement branch, stage catalog and PDF constants
  - split document dependencies by converter and remove unused jsonschema
- **Architectural Rationale & Trade-Offs**:
  - Determine ownership by individual functions, methods and constants rather than moving mixed modules wholesale
  - preserve strict HTML configuration/Markdown checks and unvalidated PDF schema guidance
- **Verification & Test Results**:
  - Audited 50 top-level definitions and reviewed client/transport methods
  - 63 existing conversion, PDF, publication and bootstrap tests pass
  - quick preflight and all 19 architecture tests pass
  - both CLI help commands load
  - Graphify refresh completes
  - no live conversion or dependency installation performed
---

### [2026-10-08 22:46 CET] — Renumber PDF preparation stages consecutively
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown`
  - `tools/bootstrap/book-to-markdown`
  - `tests`
  - `docs/developers.md`
  - `ROADMAP.md`
- **What Was Changed (The Concrete Reality)**:
  - Renumber OCR, preprocessing, page conversion, filtering and table transcription as 01 through 05
  - rename stage directories and propagate IDs through predecessor routing, configuration, status, consumers, CLI examples, tests and book handoff documentation
  - reject prior numbering before cleanup
- **Architectural Rationale & Trade-Offs**:
  - Keep stage order and directory names consistent without fractional identifiers or reinterpreting older attempt status records
- **Verification & Test Results**:
  - 64 conversion, PDF, publication and bootstrap tests pass, including legacy-attempt preservation
  - quick preflight and 19 architecture tests pass
  - local converter Markdown links resolve
  - Graphify refresh completes
  - existing workspaces and caches are not migrated and newly numbered inference stages use new cache keys
  - no live conversion performed
---

### [2026-10-08 22:48 CET] — Simplify PDF preparation README documentation
- **Affected Subsystems**:
  - `tools/bootstrap/pdf-to-markdown/README.md`
  - `tools/bootstrap/pdf-to-markdown/stages/*/README.md`
- **What Was Changed (The Concrete Reality)**:
  - Reduce six READMEs from 378 to 156 lines
  - keep current five-stage inputs, outputs, invocation, selection and restart behavior
  - replace duplicated classification and historical implementation details with prompt/contract links
  - correct outdated claims about persisted source paths and source-free restarts
- **Architectural Rationale & Trade-Offs**:
  - Match documentation to the reduced preparation pipeline and keep operational instructions easy to find
- **Verification & Test Results**:
  - All local README links resolve
  - CLI help matches documented options
  - quick preflight and all 19 architecture tests pass
  - git diff --check passes
  - no code changes or conversion runs
---

### [2026-10-08 22:49 CET] — Initialize book source hash catalog
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/books`
- **What Was Changed (The Concrete Reality)**:
  - Create four named book directories and 14 source variant directories using 12-character SHA-256 prefixes
  - store the full source SHA-256 in each source.json
  - leave PDFs in the existing local audit workspace
- **Architectural Rationale & Trade-Offs**:
  - Establish the requested minimal book and variant structure without copying source content or implementing recipe generation
- **Verification & Test Results**:
  - All 14 source hashes match existing local PDFs
  - 14 JSON files and directory prefixes validated
  - quick preflight passes
  - all 19 architecture tests pass
  - no downloads or PDF copies performed
---

### [2026-10-08 22:50 CET] — Populate local book variant PDFs
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/books`
- **What Was Changed (The Concrete Reality)**:
  - Copy each of the 14 audited PDF variants into its catalog directory as source.pdf
  - add a catalog-local ignore rule to keep reference PDFs outside Git history
- **Architectural Rationale & Trade-Offs**:
  - Keep each local source beside its SHA-256 metadata while committing only catalog configuration and the acquisition record
- **Verification & Test Results**:
  - All 14 original and copied PDFs match their source.json SHA-256
  - PDFs remain ignored by Git
  - quick preflight passes
  - all 19 architecture tests pass
  - no downloads performed
---

### [2026-10-08 22:54 CET] — Copy Test Book into the book catalog and verify reconstruction
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/books/test-book-example-4567`
- **What Was Changed (The Concrete Reality)**:
  - Copy the complete local Test Book directory including its conversion workspace
  - name the catalog directory and PDFs with lowercase hyphen-separated names matching existing books
  - adapt the copied tracker source paths and matrix book identifiers to the books catalog
  - reconstruct a separate 160-page PDF using the tracker mapping
  - ignore local PDFs and workspace
  - omit source.json
- **Architectural Rationale & Trade-Offs**:
  - Use catalog book identifiers and exact source hashes to reproduce the existing test selection while retaining the original directory and conversion provenance
- **Verification & Test Results**:
  - All 160 reconstructed pages match source pages and the original PDF in decoded streams, media boxes, rotation and RGB renders at 72 DPI
  - inspected six-page visual preview
  - quick preflight passes
  - all 19 architecture tests pass
  - original directory preserved
  - no inference or downloads performed
---

### [2026-10-08 22:56 CET] — Document author-managed book PDF acquisition
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/README.md`
- **What Was Changed (The Concrete Reality)**:
  - Document manual titles and catalog directory names
  - describe online acquisition, full SHA-256 verification and placement as source.pdf in the hash variant directory
  - distinguish author preparation from ordinary chapter generation
  - link the Test Book reconstruction tracker
- **Architectural Rationale & Trade-Offs**:
  - Make local source preparation reproducible using accepted source identities and the existing catalog convention
- **Verification & Test Results**:
  - All local README links resolve
  - git diff --check passes
  - quick preflight passes
  - all 19 architecture tests pass
  - documentation-only change with no downloads or conversion runs
---

### [2026-10-08 22:57 CET] — Centralize book catalog ignore rules
- **Affected Subsystems**:
  - `tools/bootstrap/book-to-markdown/books`
- **What Was Changed (The Concrete Reality)**:
  - Move Test Book PDF and workspace ignore patterns to books/.gitignore
  - remove the redundant Test Book .gitignore
- **Architectural Rationale & Trade-Offs**:
  - Apply one shared ignore policy to local book source files and conversion workspaces
- **Verification & Test Results**:
  - git check-ignore confirms Test Book PDF, workspace artifact and reference source PDF remain ignored
  - git diff --check passes
  - quick preflight passes
  - all 19 architecture tests pass
