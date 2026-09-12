# Amiga 500 Emulator: Engineering Diary & Architecture Genesis

This document records the architectural philosophy, evolutionary history, and human-AI co-design methodology behind the cycle-exact Amiga 500 emulator in Rust.

> [!IMPORTANT]
> **Zero Hand-Written Code (100% Voice-Prompted & AI-Generated):**  
> Throughout the entire development of this emulator, **not a single letter of code was written by hand**.  
> The entire codebase—the cycle-exact Motorola 68000 CPU core, memory bus, immediate-mode Developer Studio GUI, debugger engine, and test harnesses—was generated 100% by the AI agent through voice prompts, conversational review, and iterative instruction. Aside from occasionally pasting a link or typing a brief message when voice dictation was inconvenient, the human functioned strictly as the architect, code reviewer, and quality enforcer.

---

## 1. Project Genesis: Foundations & The Myth vs. Reality of AI Emulation

### Language & Framework Deliberations: Why Rust & egui
At the inception of the project, the human and agent engaged in focused discussions on the core technology stack:
- **Language Choice — Rust:**
  We evaluated different systems programming languages. The choice decisively landed on **Rust** for its memory safety without garbage collection overhead, predictable performance, strict ownership semantics, and seamless compilation to both native desktop (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).
- **UI Framework Selection — egui:**
  We debated what kind of GUI to build and which framework to adopt. We chose **egui / eframe**: an immediate-mode GUI framework that eliminated complex asynchronous message-passing boilerplate, enabled direct synchronous state pulling, and supported both native windowing and browser-based WASM canvases from a single codebase.

### The Myth vs. Reality of AI-Assisted System Emulation
There is a widespread misconception that modern AI can simply be prompted with:
> *"Build me a cycle-exact Motorola 68000 processor and an Amiga 500 emulator"*

...and immediately output a clean, working, high-performance emulator.

The reality is that an unguided prompt produces an unoptimized, brittle, and architecturally naive toy. It cannot navigate the intricate hardware quirks, bus contention physics, and cycle-accurate phase transitions of custom chips (Agnus, Denise, Paula, CIAs) without deep human engineering leadership.

Virtually every module, data structure, and optimization in this codebase was heavily influenced, reviewed, and directed by human engineering:
- Conducting thorough code reviews on every proposed subsystem.
- Inspecting instruction handlers, memory bus arbitration, and ALU flag calculations line-by-line.
- Demanding rigorous, automated test suites before accepting any component.
- Rejecting suboptimal, bloated, or overly abstract implementations and forcing architectural resets when code drifted from mechanical sympathy.

True engineering excellence emerged when the human's architectural vision intersected with the agent's rapid synthesis, refactoring, and code generation capabilities.

---

## 2. Foundational Architecture: Cycle-Exact Mechanics, Rejecting the "God Bus" & Rust Sympathy

Early architectural discussions centered on two core foundational questions:
1. *What does "cycle-exact" actually mean for the Amiga?*
2. *How do we model chip interconnects cleanly in Rust?*

### Rejecting the "God Bus"
In many emulators, all chips hold references to a monolithic "God Bus" or pass circular handles (`Rc<RefCell<...>>`) to query each other dynamically.
- The agent strongly advised against a God Bus, highlighting that shared mutable references lead to synchronization deadlocks, race conditions, and runtime borrow panics.
- The human strongly reinforced this: the architecture must be simple, linear, and contain minimal references between components.

### Top-Down Orchestration via the Main Loop (`A500::step_cck`)
To achieve mechanical sympathy with Rust's ownership model and the host CPU's instruction pipeline:
- **No chip autonomously queries or mutates another chip.**
- The top-level machine struct (`A500`) owns all subsystems directly in a flat hierarchy.
- On each Color Clock phase, the **main loop** orchestrates execution top-down:
  1. The **Agnus DMA Scheduler** determines the active horizontal scanline slot (refresh, audio, disk, sprites, bitplanes, or CPU).
  2. The bus access decision and wait states (`BusResult::WaitState`) are passed down to the CPU and custom chips.
  3. The main loop steps each chip for that CCK slice, and chips advance their internal sub-components accordingly.
  4. Interrupt lines (IPL 1–6) and DMA locks flow strictly through the main loop and `MemoryBus`.

Data flows in one clear, predictable direction with zero circular handles and zero runtime heap allocations in hot paths.

### Host Hardware Reality: Mechanical Sympathy Over Micro-Optimizations
Taking into account the immense speed of modern host processors (3.5–5.0+ GHz), expansive L1/L2/L3 caches, and abundant system RAM, the human established a core architectural premise: **traditional, complex micro-optimizations are completely unnecessary**.

Rather than cluttering the code with premature optimizations, bit-twiddling hacks, or obscure abstractions, the design focuses entirely on **mechanical sympathy** with the host CPU:
- **Maximizing Cache Locality & Linear Execution:** Modern superscalar CPUs are extraordinarily capable of executing code at staggering speeds as long as execution is reasonably linear and predictable.
- **Eliminating Branch Mispredictions:** Deep host CPU instruction pipelines (14–20+ stages) suffer severe penalties (15–20 CPU cycles) on mispredicted branches. If we avoid cascaded dynamic branches in the hot emulation loop (e.g. eliminating runtime `match opcode`, `match ea_mode`, and `match size` checks in favor of a direct 65,536-entry static dispatch table and concrete, specialized functions), the host branch predictor has near-zero ambiguity.
- **Zero Dynamic Heap Allocations During Emulation:** A fundamental foundational mandate was that during active emulator execution, **virtually zero heap memory allocations** (`Vec`, `Box`, `String`, `format!`) may take place. All guest memory buffers (Chip RAM, Fast RAM, ROMs), microcode state registers, execution trace ring buffers, and pipeline latches are fixed-size and pre-allocated upfront. Avoiding runtime allocator locks and heap churn eliminates latency jitter, prevents cache line eviction, and guarantees rock-solid deterministic performance across both native desktop and WebAssembly.
- **Natural Blazing Speed:** When code is flat, linear, cache-dense, and allocation-free, the host processor executes it almost effortlessly at peak throughput. This guiding principle shaped the M68000 CPU core and directly dictates how custom chips (Agnus, Denise, Paula) are designed: minimal code branching, linear data flows, and zero dynamic heap allocations in hot paths.

---

## 3. The 6-Stage Evolution of the M68000 CPU Core

The CPU core (`crates/m68000`) did not emerge in a single iteration. It required six distinct evolutionary stages to arrive at its current refined state.

### The Foundational Methodology: Minimal Representative Set & Early SingleStepTests
From day one, the development strategy was deliberately disciplined: rather than attempting to blindly generate all 65,536 opcodes at once, the human instructed the agent to identify a **minimal representative subset of instructions** spanning every major instruction class (data movement, integer arithmetic, logic, shifts/rotations, control flow, and bit operations) and addressing mode.
Throughout Stages 1 and 2, all prototyping, cycle slicing, and architectural debates were tested and proven against physical hardware test captures (**SingleStepTests**) right from the very beginning.

1. **Stage 1 — Naive Emulation & Human-Centric Typing Shortcuts (Macros & Const-Generics):**
   Working on the minimal representative instruction set, early prototypes heavily relied on patterns human developers traditionally reach for when trying to minimize repetitive keyboard typing:
   - Deep cascaded `match` trees, user-defined macros (`macro_rules!`), and generic functions parameterized by constants (`fn op_foo<const S: usize, const M: usize>(...)`).
   - The LLM's initial training bias strongly favored minimizing generated source code lines to save human typing, resulting in highly abstract, opaque boilerplate compression.
   - These versions were rejected during human code review: macros and const-generic matrices destroyed IDE navigation, generated confusing compiler errors, polluted host instruction caches, and completely decoupled the implementation from physical 68000 microcode timing.
2. **Stage 2 — The Hybrid Action Buffer & Phase Experiment:**
   Still focusing on the minimal instruction subset, Stage 2 was born out of the human's critical insight that M68000 bus cycles must be modeled in discrete clock phases ($CLK1$ and $CLK2$, matching Amiga $CCK1$ and $CCK2$):
   - ALU operations and instruction evaluation logic remained largely linear and immediate.
   - However, all memory bus operations (reads, writes, prefetches) were deferred and pushed onto a dedicated "command/action buffer".
   - A separate, decoupled execution loop processed this bus buffer, providing a mechanism for the CPU to stall and hold on wait states when blocked by Chip RAM contention.
   - While this successfully proved that memory wait states could stall execution against SingleStepTests, the resulting architecture was awkward and fragmented ("rozpieprzony"): ALU math ran upfront while bus operations lagged behind in the queue. Coordinating prefetch pipeline progression ($IR$/$IRC$) and mid-instruction Address Error traps across this split boundary proved convoluted.
3. **Stage 3 — The "Frankenstein" Architecture (Full Opcode Expansion & CLK1/CLK2 Over-Engineering):**
   At this stage, the human gave the directive to expand from the minimal representative subset and implement **all remaining M68000 instructions across the entire opcode space**, while introducing the elegant concepts intended for the final core: unified micro-step slices, archetype patterns, and true cycle-exact execution. However, because the architectural boundaries were in mid-transition, the LLM created a complex "Frankenstein":
   - The agent eagerly adopted the new micro-step ideas, but stubbornly retained the baggage and awkward habits from Stage 2.
   - Because the codebase and conversation context contained a vast amount of discussions and code relating to $CLK1$ and $CLK2$, the agent became heavily fixated on explicit clock-phase machinery. It assumed all this sprawling phase-tracking apparatus was indispensable, leaving it deeply woven into the execution flow across the newly expanded instruction catalog.
   - The result was an entangled hybrid: half-baked micro-steps coexisting with explicit clock-phase state machines, redundant intermediate buffers, and fragmented state transitions across thousands of generated opcodes.
   - The human recognized that the agent had built a monstrous hybrid and had to actively unravel and untangle this Frankenstein, stripping out the redundant phase scaffolding step by step.
4. **Stage 4 — The Microcode Archetype Baseline (Cycle-Exact Foundation):**
   Having untangled the Frankenstein, the core stabilized on a clean, physical execution foundation:
   - Native 2-clock micro-step slices ($1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$), cleanly fusing the $CLK1$ ALU phase (`alu_fn`) and $CLK2$ bus transfer into a single atomic struct without any separate phase machinery or intermediate queues.
   - 65,536-entry static dispatch table with zero dynamic runtime branches, fully embracing expansive, self-documenting code.
   - 100% SingleStepTests pass rate across all 127 test suites (~300,000 vectors from MAME and Tom Harte hardware captures).
5. **Stage 5 — Micro-Step Footprint Compaction & Dual Staging (`addr1`, `addr2`):**
   The subsequent refinement focused on optimizing the micro-step state machine itself:
   - Aggressive minimization of micro-step counts and internal operations per instruction, eliminating redundant dispatch steps.
   - Introduction of the **dual staging architecture** (`state.micro.addr1` and `state.micro.addr2`), moving away from temporary scratch register juggling and pointer shifting.
   - Deferring destination address calculation to the CCK2 idle phase of the source read on word/long dual-memory operations, guaranteeing hardware-accurate Address Error invariance (odd source address traps before the destination register is ever touched).
   - Fusing ALU calculations directly onto natural Color Clock phases (`alu_fn`).
6. **Stage 6 — Exhaustive Verification, Cartesian DMA Contention & Real Program Execution:**
   The final stage arrived once the microcode archetype baseline and micro-steps stabilized, focusing on stress-testing and end-to-end verification:
   - **Cartesian DMA Contention Test Suite (`test_dma_cartesian`):** Exhaustively validating cycle invariance ($C = C_0 + 2 \times \text{wait\_states}$), Fast RAM immunity, and state invariance across the full $2^k \times 2^M$ stall permutation space.
   - **Synthetic Program Execution:** Authoring and injecting real compiled M68000 binary routines (arithmetic loops, memory block transfers, control-flow jumps) directly into emulated RAM without OS overhead to observe multi-instruction execution and confirm that everything functions deterministically end-to-end.

---

## 4. CPU Opcode Benchmarking & Performance Profiling

Following the stabilization of the CPU core, a dedicated milestone was initiated to measure host performance, execution latency, and cache behavior across the opcode matrix:
- **Dedicated Git Worktree Isolation:** The Benchmarking harness and the Developer Studio GUI were developed in parallel on separate Git worktrees (`git worktree`) across isolated branches to keep measurement code and UI work cleanly decoupled.
- **Harness Scope:** Building automated per-opcode micro-benchmarks measuring host execution time (nanoseconds per instruction) and throughput (MIPS) across all 65,536 dispatch table entries.
- *Note:* Deep profiling results, hardware PMU counter analysis (L1i/L1d cache misses, branch mispredictions), and compaction findings will be detailed here once merged from the benchmarking branch.

---

## 5. Subsystem Evolution & The Developer Studio GUI

The same human-directed iterative refinement drove all other subsystems:

- **The Developer Studio GUI (`crates/gui`):**
  - The first iteration was initiated with a simple instruction: *"Make me a GUI"*.
  - The initial output was basic, but established the immediate-mode foundation (`egui`/`eframe`).
  - Through continuous review, the human transformed it into a full Developer Studio: live register diffs, glowing CCR LEDs, memory hex editor, temporal reverse debugger, collapsible microcode inspector (`F8`), and clean game display mode (`F12`).
  - **In-Process Headless GUI Testing Superpower:**
    One of the greatest advantages of selecting `egui` was how beautifully and cleanly testing came together. Because `egui` is an immediate-mode library, the entire UI pipeline can run completely **in-process** without opening OS windows, requiring display servers, or spawning heavy browser drivers.
    - Tests directly invoke `egui::Context::run(RawInput { ... }, |ctx| app.update_ui(ctx))` in headless unit tests.
    - We can programmatically synthesize keyboard presses (e.g. `F5`, `F8`, `F10`, `F12`), mouse clicks, text inputs, and drag gestures directly in Rust code.
    - We can inspect and assert widget states, layout stability, focus changes, and memory mutations instantly and deterministically.
    - Zero external UI automation agents, zero fragile visual diffing tools—the entire verification happens directly in fast, reliable Rust code (`crates/gui/tests/`).
  - To guide ongoing development without breaking layouts or freezing frames, dedicated rules (`egui-best-practices.md`) and a suite of 23 headless integration tests were instituted.

---

## 6. Two-Tier Hardware Decomposition

A critical architectural decision was how to structure the custom chips:

1. **Tier 1: Macro Chip Isolation (The Obvious Layer):**
   - High-level, dedicated crates for each physical chip: `agnus`, `denise`, `paula`, `cia`, `m68000`, `memory_bus`.
2. **Tier 2: Granular Internal Sub-Components (The Human Mandate):**
   - Rather than letting chip structs become monolithic God objects, the human mandated deep internal decomposition for every sub-unit:
     - **Agnus:** Independent state machines for the Copper coprocessor (`MOVE`, `WAIT`, `SKIP`, `CDANG`) and 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer).
     - **Paula:** Native BLEP synthesis audio engine (4 independent channels with dynamic CIA-A LED filter), floppy MFM track controller, UART, and interrupt multiplexer.
     - **Denise:** Video pixel serializers, bitplane fetch engines (1–6 planes), 8 hardware sprite generators, and palette registers (RGB444).
     - **CIAs:** Dual MOS 8520 chips decomposed into Timers A & B, TOD 50/60 Hz clock, serial shift register (SDR), and parallel ports.

---

## 7. The Ultimate Goal: The Clean-Room Re-Generation Experiment

Every bug fix, architectural decision, and hardware quirk in this project has been continuously documented in `Obsidian/Amiga/Design/`.

This serves a larger, ambitious vision:
- Once the complete, working Amiga 500 emulator is finished and verified against all reference suites,
- We will wipe the implementation source code (`crates/`) and reference emulator sources (`ref_src/`), leaving strictly:
  - The refined prompt and skill system (`.agents/`).
  - The curated, de-duplicated design documentation (`Obsidian/Amiga/Design/`).
  - The automated test suites (SingleStepTests, vAmigaTS).
- **The Experiment:** Can an AI agent, armed with the lessons, rules, and architecture recorded during this project, autonomously re-synthesize the complete cycle-exact emulator from scratch?

This diary stands as the living record of how those foundations were built.
