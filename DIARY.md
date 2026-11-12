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

## 6. CPU Benchmarking: From Generated Cases to Handwritten Workloads

The initial 108-case benchmark catalog generated repeated guest instructions in memory and measured host time with warm-up, noise filtering and protected CSV hashes. It supported early CPU profiling, but its catalog, trace regressions, hashes and analyzer were later retired. The measurements and anomaly claims from that runner are historical, not a current performance baseline.

A broader decoder-derived pilot then measured 4,571 representative workloads. Its fixture and NOP-control protocol differed from the later program-level measurements, so those numbers cannot be compared directly. The Rolling Changelog records the pilot's results and verification limits.

The active benchmark path uses handwritten, assembler-backed M68000 programs in `benchmarks/m68k`. The reviewed full run contained 2,257 cases and 6,771 timed series. Its approved baseline is tracked with provenance and hashes; raw host-specific runs remain local. The current snapshot includes 86 refreshed cases from filtered runs. Compare measurements only when the program source, configuration and timing protocol match. See the dated October 2026 digests and the living performance specification for the exact workflow.

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
  11. *Mandatory Engineering Diary Maintenance (`DIARY.md`):* Recording all changes, rationale, and evolutionary context in `DIARY.md`'s Rolling Changelog because Git commits are often squashed, batched, or combined.

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

## 10. Accepted-Recipe Reconstruction and Reference Publication

On October 29, 2026, `ebc99c8` delivered Book Review reconstruction from an accepted `book.json` recipe and its matching source PDF, plus publication of reviewed Markdown with stable chapter links and image descriptions. This completed a tooling capability; it did not certify the Test Book or close the four-manual conversion stage. Earlier source preparation, authoring decisions and failed acceptance checks remain in the dated historical digests below.

The accepted recipe became the portable handoff between clones. Reconstruction rebuilds the editor's local page and asset workspace from the identified source while retaining the authored recipe. Working remains the edit surface, and acceptance remains an explicit author decision. Bulk description tasks were limited to visible Working elements rather than hidden content. Publication and RAG indexing remained a downstream user workflow. The same delivery added a structural frontmatter gate for local reference Markdown; that gate checks metadata, not source fidelity.

The recorded Test Book replay completed PDF stages 01-08 with 545 cache hits and zero new model calls, then imported 993 elements. Source validation still reported 56 invalid fields in that raw import. The user retained the fresh import without treating successful execution as source acceptance. The closing commit records a passing commit profile, 35 design specifications with 691 resolving links, diary/roadmap checks and valid frontmatter in 104 local Markdown files. These are historical results for that delivery, not checks rerun during diary compaction.

The roadmap pilot was removed by user direction, rather than verified as complete. The remaining repository deliverable was four accepted manual recipes with matching source identities; full-book quality, downstream publication and RAG ingestion were not established by this tooling delivery.

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

Device-engine reconstruction remained in 2.4 and functional guest verification in 2.5. Serial and parallel ports remained disconnected; deterministic pin tests did not prove communication with attached devices. Firmware, media execution, vAmigaTS and measured silicon timing remained outside this closure. The separately closed timing research was not reopened. The November 12 workflow entry below records a separate planned register-propagation audit; this transport closure does not establish that audit's result or its current roadmap placement.

---

## Rolling Changelog

This final, unnumbered section records changes in chronological order. Completed stages with a full narrative are recorded in the numbered chapters above; settled supporting work remains in dated digests. Detailed entries for unfinished work stay here until its scope is settled. Historical verification applies to the source and scope recorded at the time, not automatically to the current emulator.

### Entry Format

Every authorized repository commit adds its own entry here, including routine code, tests, rules, documentation and merges. Logging and compaction follow [diary-maintenance.md](.agents/rules/diary-maintenance.md) and [compact-diary](.agents/skills/compact-diary/SKILL.md). Record:

- **Timestamp & Context**: Date/time and the task or stage where needed.
- **Affected Subsystems**: Components touched.
- **What Was Changed**: Concrete behavior or mechanisms delivered.
- **Architectural Rationale & Trade-Offs**: Decisions and rejected alternatives.
- **Verification & Invariants**: Actual results, failures and unrun checks.

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
  - Architecture at implementation: 18/19, with 62 pre-existing broken Design-to-Reference links in untouched documents. Native-copy, mixed, blank/graphic-only and real rotated/cropped fragment acceptance remained unverified. No full-book conversion or RAG indexing occurred. The following digest records the later closure checks.
---

### [2026-10-06 01:12 CEST] — Historical Digest: Codex Conversion and PDF Text Preparation
- **Timestamp & Context**: 2026-10-05 to 2026-10-06. Early migration and PDF-TEXT-1.1 closure; later digests record the replacement of experimental mechanisms.
- **Affected Subsystems**: HTML/PDF converters, shared Codex transport/cache, bootstrap documentation and conversion plans.
- **What Was Changed**: Replaced Gemini calls with shared subscription-authenticated Codex transport and explicit stage model/effort settings. Established positioned text JSON and matching PNG inputs, isolated attempts, restart handling and response reuse. Removed redundant configuration and temporary test trackers.
- **Architectural Rationale & Trade-Offs**: Converter development follows a requested change, conversion of a user-selected fragment, then user assessment. Existing technical tests protect transport and execution; they do not certify reconstruction quality. Independent full-book pilots, quality matrices and routine crop-test expansion were excluded.
- **Verification & Invariants**: Historical page-19 pilot completed the then-current 14-stage pipeline; replay used cache with zero new requests. PDF-TEXT-1.1 (`5a9add8`) was closed by user decision using page-64 OCR/render/geometry evidence. Native-copy, mixed, blank and real rotated/cropped categories remained unverified. Closure checks passed PDF/shared-client suites at 19/19 each; the milestone gate failed on 62 existing reference links and 22 unchanged Rust compound conditions, with architecture 18/19. This was scope closure, not certification of a repository milestone or later pipeline versions.

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

### [2026-10-08 18:40 CEST] — Historical Digest: Removed deterministic PDF row ordering
- **Timestamp & Context**: 2026-10-08; a short Stage 02.48 experiment was removed at the user's request.
- **Affected Subsystems**: PDF page ordering, review and converter documentation.
- **What Was Changed**: Tested anchored row grouping and left-to-right sorting, then removed the stage, its tests and documentation. Page conversion retains its reviewed source order.
- **Architectural Rationale & Trade-Offs**: A fixed geometric ordering pass could override intended reading order; no compatibility stage was retained.
- **Verification & Invariants**: Historical ordering tests passed before removal. The removal passed focused PDF review and conversion tests; those earlier tests do not certify the current page order.

---

### [2026-10-10 14:17 CEST] — Historical Digest: PDF preparation and Book Review foundations
- **Timestamp & Context**: 2026-10-08 to 2026-10-10; settled converter and editor groundwork before later source-alignment work.
- **Affected Subsystems**: `tools/bootstrap/pdf-to-markdown`, `tools/bootstrap/book-to-markdown`, shared conversion transport and book catalog.
- **What Was Changed**:
  - Reduced the PDF pipeline to preparation and page-object output. Stage 02 handles source objects; table transcription replaces table markup, and final book assembly moved to Book-to-Markdown. The temporary page Markdown export and chapter proofreading instruction were retired by later editor and Stage 08 work.
  - Settled the book element vocabulary: heading levels, generic notes and legends, distinct code listings and preformatted text. Filtering is limited to page furniture and margin-logo classification; source tables, character-built diagrams and drawing sheets retain their intended roles.
  - Added source-hash variants, local source PDFs and shared response-cache identity independent of stage names. The catalog and cache were later relocated to `tools/bootstrap/books` and `tools/bootstrap/.cache`.
  - Added deterministic graphic extraction and editable Working/Accepted review with a three-column bitmap UI. Later entries record source-backed recipes, import requirements and authoring controls.
- **Architectural Rationale & Trade-Offs**: Keep conversion artifacts close to source pages, make author choices explicit in the editor and identify cache responses by their actual request content. Local source PDFs and generated review bundles stay outside Git.
- **Verification & Invariants**: Historical focused converter, cache, graphic and review tests passed. A one-page pilot and cache replay did not establish whole-book reconstruction or source fidelity. Later Test Book and Stage 08 entries supersede the original editor and chapter-proofreading workflow.

---

### [2026-10-10 23:28 CET] — Historical Digest: Verification and specification tooling
- **Timestamp & Context**: 2026-10-09 to 2026-10-10; verification CLI, policy and Rust target consolidation.
- **Affected Subsystems**: `tools/verification`, `crates/test_runner`, design specifications, agent rules and tests.
- **What Was Changed**:
  - Replaced scattered completion scripts with `verify.py` commit and milestone profiles. Standard checks remain unconditional; CPU suites, Python contracts and development selections use explicit routing. Verification can run before staging, and the CLI exposes its plan and help.
  - Moved test-runner benchmarks to Cargo benchmark targets, retained silicon helpers in tests, generated SingleStep reports and removed the previous vAmiga capture harness and obsolete benchmark trace/hash fixtures. Protected reference vectors and baselines were not silently changed.
  - Organized design ownership, documentation rules, English scanning, test contracts and service-owned read-only RAG configuration. Prompt-only converter contracts replaced duplicate schema instructions.
- **Architectural Rationale & Trade-Offs**: One verifier selects the required gate while focused tests serve development; living specifications own behavior and protected references require explicit evidence and approval.
- **Verification & Invariants**: The final 2026-10-10 commit profile passed ordinary crate tests, coverage, formatting, Clippy, architecture, quality, English and five Python contract directories. Full SingleStep, performance measurement and milestone gates were not selected. Earlier focused checks and renamed fixtures are historical results, not evidence for retired harnesses.

---

### [2026-10-11 22:08 CEST] — Milestone Digest: M68000 timing, exception and bus reconciliation
- **Timestamp & Context**: 2026-10-11; CPU-CYCLE-01, CPU-PC-01 and CPU-DOCS-01 closure.
- **Affected Subsystems**: `crates/cpu`, `crates/memory_bus`, `crates/physical_memory`, machine integration tests and CPU design specifications.
- **What Was Changed**:
  - Reconciled timed reads, writes and internal CCK phases across arithmetic, logic, compare, transfer, branch, frame, return and system instructions. Predecrement calculation is separate from operand access; long read-modify-write sequences use the supported word order. Address Error begins at the rejected access and retains phase-correct frame PCs.
  - Fixed MOVE/MOVEM completion, CHK and TRAPV timing, STOP suspension, RESET output ownership and TAS write context. Illegal and Line A/F exceptions use rejected opcode PC; Trace samples T at instruction entry before an accepted IRQ. External level-seven NMI was excluded at user direction.
  - Replaced Cartesian DMA contention permutations with CPU retry tests across 52 public transfers. Documented the hardware-derived DMA schedule separately from runtime arbitration. Kept approved ASL.b reference overrides with original vector hashes intact.
- **Architectural Rationale & Trade-Offs**: Model phase-specific PC and bus behavior directly, without a global frame-PC shortcut. Synthetic CPU and bus fixtures verify observed callbacks but do not prove physical pins, device IACK, BERR or arbitrary in-flight snapshot recovery.
- **Verification & Invariants**: Final-source comparison passed 1,000,060 vectors across 124 original files, including 178,087 Address Error entries; all 124 original hashes stayed unchanged. Commit and milestone profiles passed; the full SingleStep report completed 128/128 groups with zero failures and 1,002,737 reported executions plus two override controls. These are historical results for the 2026-10-11 source.

---

### [2026-10-11 23:50 CEST] — Milestone Digest: CPU continuation and SingleStep sensitivity
- **Timestamp & Context**: 2026-10-11; CPU-CONTINUE-01, SINGLESTEP-MUTATION-01 and SINGLESTEP-RAM-01 closure.
- **Affected Subsystems**: CPU and machine continuation fixtures, SingleStep loader/comparator, testing specifications.
- **What Was Changed**: Added 15 continuation tests for reset, exceptions, returns, waits, snapshots and following guest effects. Mutation tests require exact formal next PC, both prefetch words, timed callbacks and valid vector structure. At user direction, final RAM comparison and its Address Error allowances were removed; initial RAM ranges remain validated.
- **Architectural Rationale & Trade-Offs**: A final queue state alone does not prove subsequent execution. Synthetic writes already follow timed callbacks, so final RAM comparison duplicated fixture consistency rather than independent CPU evidence.
- **Verification & Invariants**: The mutation pilot used 11 original seeds and 1,003 raw JSON mutations: 748 observable changes were rejected by comparison, 165 invalid copies by format and 33 equivalent controls passed. Ten affected groups passed 80,650 executions. Commit profiles and focused continuation checks passed; full SingleStep and milestone profiles were not rerun after these follow-up changes. Physical keyboard timing, BERR, level-seven NMI and arbitrary active-slice restore remained outside scope.

---

### [2026-10-14 02:23 CEST] — Historical Digest: OCS register routing and DMA address ownership
- **Timestamp & Context**: 2026-10-12 to 2026-10-14; register map and chipset routing work.
- **Affected Subsystems**: MemoryBus, PhysicalMemory, Agnus, Denise, Paula, CIA, custom-register map and design specifications.
- **What Was Changed**:
  - Centralized OCS register names and CIA addresses; removed unsupported BPLCON3/DIWHIGH state and flat aliases. Explicitly routed shared BPLCON0, DIWSTRT/DIWSTOP and sprite position/control writes to their recipients; reserved Agnus offsets are ignored.
  - Gated bitplane DMA by vertical display window and DDF while Denise clips horizontal pixels. Bounded Agnus, Copper, Blitter and Paula DMA pointers to the OCS 512 KB address range; retired Paula's direct Chip RAM fetch path.
  - Replaced configurable open-bus values with shared FF/FFFF placeholders, removed implicit defaults for serialized fields and documented board signal paths with source uncertainty. CPU state internals gained accessors while bus and machine callers were updated.
- **Architectural Rationale & Trade-Offs**: Named physical routes and shared constants keep bus ownership explicit. Register and signal descriptions distinguish manual evidence from modeled behavior; the chipset reset and Gary decoding remain roadmap work.
- **Verification & Invariants**: Historical focused CIA, MemoryBus, Denise, DMA-pointer and architecture regressions passed, as did applicable commit profiles and vault checks. No physical capture or full native chipset verification was claimed.

---

### [2026-10-14 05:06 CEST] — Historical Digest: Design cleanup and branch reintegration
- **Timestamp & Context**: 2026-10-12 to 2026-10-14; specification consolidation, policy updates and two branch merges.
- **Affected Subsystems**: Design vault, agent rules, `ROADMAP.md`, verification and Git merge procedure.
- **What Was Changed**: Consolidated CPU, GUI, physical-memory and signal-map specifications and removed stale examples and unused peripheral suites. Merged the emulator and bootstrap branches with reviewed Markdown, conflicts, verification scope and explicit English vocabulary decisions. A clean fast-forward later became the direct reintegration path; divergent merges retain reconciliation and verification. Removed a completed OCS pointer roadmap item at user direction.
- **Architectural Rationale & Trade-Offs**: Keep one living design description for each mechanism, retain source evidence limits and make branch integration review correspond to actual changed files. Roadmap removal records a user decision, not proof that every broader chipset task is complete.
- **Verification & Invariants**: Historical merge profiles, vault and focused contracts passed after reviewed corrections. The merged branches' earlier failed or unrun checks are not retroactively passing results.

---

### [2026-10-14 12:42 CEST] — Historical Digest: Decoder-derived CPU benchmark pilot
- **Timestamp & Context**: 2026-10-12 to 2026-10-14; CPU-BENCH-01 before the later handwritten benchmark replacement.
- **Affected Subsystems**: CPU benchmark runner, analyzer and performance documentation.
- **What Was Changed**: Enumerated 45,821 legal primary opcodes and 4,571 representative workloads, with 27 targeted retries and 47 NOP controls. The method and its baseline were later replaced by complete handwritten workloads; no decoder-derived runner remains in the active benchmark path.
- **Architectural Rationale & Trade-Offs**: Decoder coverage was useful for a broad pilot but its fixture and control protocol could not be compared directly with the later program-level baseline.
- **Verification & Invariants**: At the pilot, all 4,571 workload replays matched state and CPU clocks and final timings passed analysis; full SingleStep executed 1,002,739 vectors with zero failures. The commit profile failed on then-existing Book Review errors and an RAG MCP timeout. No protected baseline was silently changed.

---

### [2026-10-25 13:54 CET] — Milestone Digest: Test Book editor review and source choices
- **Timestamp & Context**: 2026-10-23 to 2026-10-25; user-directed closure of Roadmap 1.1 and continuing authoring work.
- **Affected Subsystems**: Book Review editor, PDF table conversion, Test Book recipe and roadmap.
- **What Was Changed**: Added guarded Codex editing, table/graphic merge controls, source recipe resolution and reviewer actions. Selected the 308-page A500/A2000 OCR PDF as the active manual source, retaining other manuals locally for reference. Versioned the reviewed Test Book recipe, later replacing its first one-chapter snapshot with the current 14-chapter, 975-element Accepted model. Book Actions and direct-text cleanup followed; later source-alignment work replaced the early cleanup mechanism.
- **Architectural Rationale & Trade-Offs**: Working remains the edit surface; Accepted and the root recipe represent explicit author decisions. Source PDF identity uses hashes, while local PDFs and generated bundles stay outside Git. Description, proofreading and standalone reconstruction were separate follow-up work.
- **Verification & Invariants**: Editor closure reported 102 passing Book Review contracts and matching source/Working/Accepted hashes. Its commit profile still failed on 36 English findings and a PDF `KeyError: 'blocks'`; the milestone profile passed full SingleStep but failed English. A later source audit found 319 unrelated invalid fields, so the early recipe was not certified for standalone reconstruction.

---

### [2026-10-25 21:00 CET] — Historical Digest: DBcc timing and opcode legality scope
- **Timestamp & Context**: 2026-10-23 to 2026-10-25; disassembler and CPU corrections with user-directed roadmap closure.
- **Affected Subsystems**: CPU, disassembler, SingleStep evidence notes and verification rules.
- **What Was Changed**: Corrected DBcc expiry to 14 clocks with three timed reads and added wait-state regressions. Removed six unsupported PC-relative CMPI primary words after Motorola PRM review, aligned disassembly and the local opcode inventory, and documented limits of software-generated vectors. The user closed opcode-legality work without a complete Motorola-derived 65,536-word inventory.
- **Architectural Rationale & Trade-Offs**: A software-generated legal-opcode map and SingleStep vectors are useful checks but do not replace primary hardware evidence or prove every path.
- **Verification & Invariants**: Focused CPU/disassembler/architecture checks passed; dispatch, disassembler and local CSV agreed with the software-generated map for 65,536 words. The commit profile failed then-existing Book Review UI/asset errors; full Motorola inventory, physical captures and milestone verification were not run for that closure.

---

### [2026-10-26 11:03 CET] — Milestone Digest: Handwritten M68000 performance baseline
- **Timestamp & Context**: 2026-10-24 to 2026-10-26; replacement of the decoder-derived candidate workflow.
- **Affected Subsystems**: `benchmarks/m68k`, test-runner benchmark target, performance analyzer, approved baseline and design notes.
- **What Was Changed**: Added assembler-backed whole-program cases, including branch, counter and conditional-store paths. Replaced decoder-derived candidates with 2,257 handwritten cases, per-series timing, load checks, bounded retries and compatible-run analysis. Removed tracked host-specific raw runs, retained a reviewed baseline and published a current snapshot with 86 refreshed cases; unused external profiling was retired.
- **Architectural Rationale & Trade-Offs**: Complete programs make target path and host timing provenance reviewable. The user approved replacing the old reference; differing fixture and NOP protocols make old and new numbers incomparable. Raw local runs remain outside Git, while approved baseline hashes and source-run identities remain tracked.
- **Verification & Invariants**: The full run completed 2,257 cases and 6,771 timed series with no execution failure or persistent load disturbance. Baseline hashes and rows matched the reviewed run; focused Python and Rust checks passed. The commit profile failed then-existing Book Review UI/asset contracts; full SingleStep and a new timing measurement were not run for the later snapshot. One catalog commit explicitly waived tests and was not remeasured in that turn.

---

### [2026-10-26 15:27 CET] — Historical Digest: Named physical memory addresses
- **Timestamp & Context**: 2026-10-26; behavior-preserving memory constant cleanup.
- **Affected Subsystems**: MemoryBus and PhysicalMemory.
- **What Was Changed**: Named the 24-bit address mask and Fast/Slow RAM bases; derived the synthetic boot SSP from the configured 512 KB Chip RAM size.
- **Architectural Rationale & Trade-Offs**: Tie repeated addresses and fallback stack values to their owning definitions without changing routing.
- **Verification & Invariants**: Historical physical_memory/memory_bus tests passed 55 integration cases; Graphify found no topology change and whitespace checks passed. The entry recorded the commit profile as pending.

---

### [2026-10-27 18:08 CET] — Milestone Digest: Rebuilt Test Book source and page proofreading
- **Timestamp & Context**: 2026-10-26 to 2026-10-27; rebuilt Test Book source provenance and PDF Stage 08 preparation before import.
- **Affected Subsystems**: `tools/bootstrap/book-to-markdown`, `tools/bootstrap/pdf-to-markdown`, shared conversion transport, verification selection, Book documentation and the testing specification.
- **What Was Changed**:
  - Record the rebuilt `78e52f075e17` PDF identity and a tracked 999-element recipe snapshot. At that time the root recipe differed from ignored Accepted in 74 elements; Working and Accepted stayed outside Git.
  - Add source-backed substitutions, accepted literals, cleanup navigation and source-difference highlighting. Share browser sessions in Book Review tests, add request pacing and content-free tracing, and select Bootstrap contracts for Bootstrap changes.
  - Add Stage 08 text-first proofreading with optional page-image resolution and validated `md_text` patches for all retained pages. Require Stage 08 before Book import, retire chapter proofreading, and improve source binding for split words, compact tables, spaced text and inline math.
- **Architectural Rationale & Trade-Offs**: Keep prepared page text and graphics as the import handoff, source identity and OCR provenance separate from editor bundles, and ambiguous OCR visible for later review. The source PDF and local backup bundles were not committed.
- **Verification & Invariants**:
  - The 2026-10-26 commit profile failed with 117 unknown English words and a PDF conversion `KeyError: 'blocks'`; its other selected suites, formatting and architecture checks passed. Source SHA-256, vault links and frontmatter passed.
  - The 2026-10-27 commit profile failed with 145 unknown English words; its other selected suites, formatting, quality and vault checks passed. No live Stage 08 conversion, Test Book import or chapter-quality review was run in that commit. These historical results do not establish current closure gates.

---

### [2026-10-27 23:29 CET] — Historical Digest: Published Test Book authoring decisions
- **Timestamp & Context**: 2026-10-27; user-directed closure of then-Roadmap 1.3/1.4 in `570d578`.
- **Affected Subsystems**: Book Review, Test Book recipe and roadmap.
- **What Was Changed**: Replaced import matching with page-local source alignment, removed the obsolete compiler tests and adjacent-correction helper, and published the current Accepted recipe. The user accepted chapter formatting, retained authored source references and explicitly accepted short edited fragments. Standalone extraction remained in 1.5 and four manual recipes in 1.7.
- **Architectural Rationale & Trade-Offs**: An accepted edit and publication do not establish semantic source attribution or standalone reconstruction.
- **Verification & Invariants**: Historical Book Review passed 160 tests. Commit/milestone profiles failed English with 101 unknown-word occurrences; their other selected suites passed. Semantic sync failed five emulator checks; source audit reported 56 fields requiring attention among 940 fields/26,360 references. Root/Accepted hashes and PDF/source identity matched; roadmap/whitespace passed. No standalone reconstruction or RAG ingestion ran.

---

### [2026-10-27 23:39 CET] — Historical Digest: Bootstrap branch integration
- **Timestamp & Context**: 2026-10-27; divergent merge `0f68632`.
- **Affected Subsystems**: Agent rules, Bootstrap, verifier, testing specification, roadmap and diary.
- **What Was Changed**: Integrated reviewed authoring/Stage 08 work with CPU benchmarks and memory changes. Reconciled seven text conflicts, interleaved 25 diary entries and reviewed 63 changed Markdown paths and links.
- **Architectural Rationale & Trade-Offs**: Retain shared browser/dependency setup, master CPU-vector coverage requirements and conditional Bootstrap contracts. Fast-forward was unavailable because the branches diverged.
- **Verification & Invariants**: Historical focused CLI/Book Review/setup, formatting, architecture, vault and whitespace checks passed. The first full profile failed only 101 English occurrences; after explicit user vocabulary/literal decisions and one reworded prompt, the full profile passed Rust coverage, Clippy, architecture, quality, standard harness, English and six Python contract suites.

---

### [2026-10-28 15:41 CET] — Historical Digest: FPGA references and measured hardware research
- **Timestamp & Context**: 2026-10-28; `8ba6def` closed then-Roadmap 1.9/1.10.
- **Affected Subsystems**: Source bootstrap, Graphify queries, hardware researcher and design links.
- **What Was Changed**: Pinned MiniMig and Amiga Replacement Project sources with individual/general source selection; unified Graphify scopes and source filters, routed measured research to Techtravels and removed the duplicate ignored vAmiga tree. Updated the English hash scanner and semantic memory-map audit.
- **Architectural Rationale & Trade-Offs**: Keep one canonical vAmiga source and separate measured signals from reconstructed circuitry and emulator models.
- **Verification & Invariants**: Historical repeated source bootstrap retained pinned manifests. A Paula DMAL query distinguished measured Techtravels evidence from reconstruction; Graphify query tests, English, semantic/vault audits and roadmap checks passed.

---

### [2026-10-28 16:50 CET] — Historical Digest: Book catalog and response-cache relocation
- **Timestamp & Context**: 2026-10-28; `e35239e`.
- **Affected Subsystems**: Bootstrap books, shared conversion cache, Book Review, verification and exclusions.
- **What Was Changed**: Moved catalog/source/review data to `tools/bootstrap/books` and the completed-response cache to `tools/bootstrap/.cache`; updated consumers, server defaults, links and Git/Graphify/verification exclusions.
- **Architectural Rationale & Trade-Offs**: Keep source and shared cache at bootstrap scope without changing response identities or local review workspaces. The testing specification already described this correctly.
- **Verification & Invariants**: Historical SHA-256 comparison preserved 6,694 files including 4,775 cache entries. A cache hit, PDF status, review HTTP/API, focused conversion/PDF/Book Review/bootstrap/verifier tests, Graphify and whitespace passed. No PDF conversion or Book import ran.

---

### [2026-10-28 17:05 CET] — Historical Digest: Book Review documentation
- **Timestamp & Context**: 2026-10-28; `67743aa`.
- **Affected Subsystems**: Bootstrap instructions and Book Review README.
- **What Was Changed**: Organized commands, review workflow, source identity and recipe contract; removed duplicate implementation/UI prose and shortened nested guidance.
- **Architectural Rationale & Trade-Offs**: Keep operational procedures and author decisions discoverable; selected no emulator specification change.
- **Verification & Invariants**: Historical focused English, thirty local links and whitespace passed. An unrelated tracked `crates/test_runner/src` path remained unresolved. Application tests were not run for this documentation-only change.

---

### [2026-10-28 23:22 CET] — Historical Digest: Chapter names and scoped descriptions
- **Timestamp & Context**: 2026-10-28; `ac53f5e`.
- **Affected Subsystems**: Book Review, shared conversion, Test Book recipe and commit guidance.
- **What Was Changed**: Added chapter/contents renaming with source-aligned titles and safe Markdown exports, selected/bulk image and table descriptions, Working rendering and active Codex-request cancellation. Updated the Test Book chapter structure and one image description; kept an authorized commit active during language decisions.
- **Architectural Rationale & Trade-Offs**: Keep rendered/exported content consistent with its authored recipe and restrict description writes to selected source context.
- **Verification & Invariants**: Historical Book Review passed 182 tests and conversion contracts 44; whitespace passed. Clearer test names and a JavaScript term followed explicit user approval. The testing specification remained accurate; no PDF conversion or Book import ran.

---

### [2026-10-28 23:52 CET] — Historical Digest: Earlier diary compaction
- **Timestamp & Context**: 2026-10-28; `4dbfa5e`.
- **Affected Subsystems**: Diary history and benchmark narrative.
- **What Was Changed**: Consolidated settled September/October history from 5,740 to 859 lines before the entry, preserving then-unfinished Book Review work and the transition to handwritten CPU workloads.
- **Architectural Rationale & Trade-Offs**: Keep decisions and dated verification limits in the diary; Git retains mechanical changes.
- **Verification & Invariants**: Historical diary, English, whitespace and commit checks passed before the entry. No milestone profile ran because the compaction closed no roadmap point.

---

### [2026-10-29 17:42 CET] — Historical Digest: Four-manual recipe scope
- **Timestamp & Context**: 2026-10-29; explicit roadmap scope removal preceding the tooling delivery in Chapter 10.
- **Affected Subsystems**: Roadmap and Book-to-Markdown workflow boundary.
- **What Was Changed**: Removed the former Step 11.1 Test Book pilot by user decision. Kept four manual PDF conversions, recipe adjustment and committed `book.json` files; downstream Markdown/descriptions/publication/RAG remained a documented user workflow rather than roadmap deliverables.
- **Architectural Rationale & Trade-Offs**: The pilot was removed, not verified complete: the fresh Test Book had no descriptions or Accepted bundle and no demonstrated clean reconstruction gate. The four-manual stage remained pending.
- **Verification & Invariants**: Historical roadmap retention, focused English and whitespace passed. Commit/milestone profiles and semantic parity were not run for this scope-only edit.

---

### [2026-11-08 23:41 CET] — Historical Digest: Agent workflow ownership
- **Timestamp & Context**: 2026-11-08; workflow portion of `99c8e39`.
- **Affected Subsystems**: Agent rules/skills/roles, verification, HTML contracts, worktrees and documentation.
- **What Was Changed**: Consolidated instruction ownership, replaced obsolete skills with scoped compliance/canonical verification and required independent review for every authorized commit. Included pinned FPGA references in worktrees, moved HTML attachment tests to their owning tool and verification selection, and restored approved metadata in 66 ignored reference chapters in each of two checkouts without changing chapter text.
- **Architectural Rationale & Trade-Offs**: Keep requirements, procedures and tests with their owners while preserving reference artifacts.
- **Verification & Invariants**: Historical CLI/HTML contracts, vault metadata/links and skill checks passed. All 104 local frontmatter files passed in each active checkout; Graphify refreshed all three checkouts. No roadmap stage closed or milestone profile ran.

---

### [2026-11-08 23:41 CET] — Historical Digest: BLEP reference regression protection
- **Timestamp & Context**: 2026-11-08; audio portion of `99c8e39`.
- **Affected Subsystems**: BLEP generator, numerical tests and optional WinUAE comparison chart.
- **What Was Changed**: Added offline CSV regression protection, moved eight numerical tests outside production source and made external chart generation optional with an explicit source path.
- **Architectural Rationale & Trade-Offs**: Separate stable numerical regression from optional reference-implementation comparison. Historical FIR conventions reproduced all retained reference samples exactly.
- **Verification & Invariants**: Historical BLEP tests and all-target Clippy passed. Eight moved test bodies, four production implementations, four CSV hashes and the committed SVG were preserved. A one-time WinUAE comparison and chart regeneration produced identical plotted paths; these numerical results do not establish physical audio output.

---

### [2026-11-10 21:50 CET] — Historical Digest: Approved instruction and commit-report simplification
- **Timestamp & Context**: 2026-11-10 21:29-21:50; `c1952f5` and `65da673`.
- **Affected Subsystems**: Rules, PowerShell guidance, CPU verifier role, code review and specification-compliance skills.
- **What Was Changed**: Shortened approved Rust/API/documentation/test guidance and PowerShell invocation/error guidance while preserving accurate-document dates and CPU observation limits. Preferred native Git/existing tools/original logs and concise reports, identified snapshots through Git and allowed review reading during verification.
- **Architectural Rationale & Trade-Offs**: Retain mandatory gates and independent approval without extra evidence copies. Three screening rounds supported individual replacements, not overall equivalence; combined behavioral validation was not performed.
- **Verification & Invariants**: Historical round-three screening produced nineteen correct results and one preserved host-fixture arithmetic failure. Six approved replacements matched; twenty first-change and 58 later local link targets resolved. Whitespace passed, original index/experiment artifacts were preserved and scoped review found no emulator-specification update. No milestone profile ran.

---

### [2026-11-11 15:11 CET] — Historical Digest: Pending implementation and verification ownership
- **Timestamp & Context**: 2026-11-11; roadmap consolidation `9206fae`.
- **Affected Subsystems**: Roadmap, design specifications, developer docs and documentation rules.
- **What Was Changed**: Moved diagnostics and acceptance work to owning roadmap stages; retained the shared beam interface and clarified DMA review, production CPU/Gary tests, guest programs and live-frame prerequisites. Tracked debugger/deferred bus/interrupt/peripheral gaps and aligned audio/host-input prerequisites; refreshed recipe guidance and two comments without executable changes.
- **Architectural Rationale & Trade-Offs**: Specifications retain hardware contracts and concise caveats; the roadmap owns pending work and evidence. Reorganization does not close implementation stages.
- **Verification & Invariants**: Historical audit passed 35 design notes, 832 links, 21 changed-document roadmap anchors, roadmap and whitespace checks. Source inspection confirmed documented limitations and preserved both Rust files' executable content. No milestone profile ran.

---

### [2026-11-11 23:40 CET] — Historical Digest: Shared-reference RAG retrieval
- **Timestamp & Context**: 2026-11-11; adapter repair `3a88406`.
- **Affected Subsystems**: Amiga RAG adapter, subprocess boundary, contracts and README.
- **What Was Changed**: Accepted manual hits from other checkouts while preserving Amiga/reference-directory filters; decoded CLI output explicitly as UTF-8. Added shared-checkout and actual UTF-8 subprocess regressions.
- **Architectural Rationale & Trade-Offs**: The shared index can refer outside the active checkout; command output must not depend on the Windows locale.
- **Verification & Invariants**: Historical fifteen adapter/launch contracts passed; full commit verification and independent review were recorded in the closing commit.

---

### [2026-11-12 01:18 CET] — Historical Digest: Named encodings and protected verification decisions
- **Timestamp & Context**: 2026-11-12 01:11-01:18; policy `d2a9f6d` followed by implementation `2957128`, split at user request.
- **Affected Subsystems**: Emulator constants, configuration, chipset, memory, debugger/GUI, semantic audit and agent rules.
- **What Was Changed**: Required REGISTER_ names and BIT_/MASK_ flags, and explicit approval for architecture checks or weakened gates. Migrated unnamed domain encodings and callers, reused shared disk/interrupt flags through two explicitly approved config dependencies and updated architecture docs/permissions. Applied four approved semantic-parser/source-selection changes and approved identifier word splits without vocabulary expansion.
- **Architectural Rationale & Trade-Offs**: Make hardware meaning and ownership traceable without runtime behavior changes; keep architecture and acceptance criteria under user control.
- **Verification & Invariants**: Historical restored implementation matched the isolated pre-split snapshot before identifier corrections. Static audit covered 196 register rows, twelve memory boundaries and 27 crate names, but no signal/quirk sentinels; vault passed 35 notes/998 links. The initial implementation commit profile failed only English; focused English passed after approved corrections. Policy English and whitespace passed. Full SingleStep was not run and no roadmap stage closed.

---

### [2026-11-12 01:43 CET] — Verify register propagation before reconstruction and exempt task plans
- **Affected Subsystems**:
  - `roadmap`
  - `agent rules`
  - `language verification`
  - `verification tests`
- **What Was Changed (The Concrete Reality)**:
  - Schedule REGISTER-PROPAGATION-VERIFY-20261111 before chipset reconstruction and retain BOOK-GUEST-PROGRAM-TESTS-20261111 functional acceptance after reconstruction
  - exempt .agent/tasks from language requirements and scans even with explicit file arguments
  - preserve ordinary explicit-file exclusions elsewhere
  - add path-boundary regressions and the user-approved chdir vocabulary entry
  - keep execution plans local and reference stable task IDs in the roadmap
- **Architectural Rationale & Trade-Offs**:
  - Separate propagation evidence from engine completion without dropping later functional tests
  - avoid language findings and vocabulary approvals for temporary task plans
  - keep other repository language protection unchanged
- **Verification & Test Results**:
  - New scanner regressions failed on the original implementation and pass after the change
  - PASS all 87 verification tool tests
  - PASS full repository English scan
  - PASS task-only scan selects zero files
  - PASS roadmap check and diff whitespace
  - scoped specification review found the testing strategy already delegates language scope to its owning policy
  - chipset tasks were not executed and no roadmap milestone is closed
---

### [2026-11-12 09:11 CET] — Compact diary chapters and align reconstruction verification
- **Affected Subsystems**:
  - `Diary`
  - `agent rules and skills`
  - `diary tools`
  - `roadmap`
- **What Was Changed (The Concrete Reality)**:
  - Promote settled book reconstruction and register Gary and signal-transport stages into chapters 10-13
  - replace the numbered log with the final unnumbered Rolling Changelog
  - compact settled entries while retaining historical evidence limits and unfinished work
  - align the appender and chronology checker with the stable heading
  - record the user-approved removal of the separate pre-reconstruction propagation gate and move component unit tests into 2.4 while preserving functional propagation and guest acceptance in 2.5
- **Architectural Rationale & Trade-Offs**:
  - Keep completed-stage narratives separate from rolling changes without a fixed section number
  - preserve the explicitly approved reconstruction and verification order without claiming a new hardware milestone
- **Verification & Test Results**:
  - PASS diary structure and chronological order with 47 entries before this entry
  - PASS seven checker boundary cases and appender dry run without diary mutation
  - PASS skill validation and focused English scan of six changed files
  - PASS local-link and whitespace checks
  - PASS roadmap retention check
  - scoped specification selection found no mapped design changes or invalid tracked paths and the testing strategy remains accurate
  - diary compaction reduced 1211 lines and 121583 bytes to 909 lines and 113367 bytes before this entry
  - no implementation stage is closed by this documentation change
