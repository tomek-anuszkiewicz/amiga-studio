# Amiga 500 Emulator: Development Roadmap & Strategy

This document outlines the phased development plan, hardware milestones, verification methodologies, and AI agent testing strategies for the Amiga 500 emulator project.

---

## 1. Hardware Roadmap & Milestones

### Phase 1: Baseline Amiga 500 (Rev 5 / Rev 6a OCS) — Immediate Focus
- **CPU:** Motorola 68000 cycle-exact core based on the **Microcode Archetype Baseline** (Native 2-clock micro-step slices: $1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$, 16 orthogonal archetype modules covering 100% of bus cycle types, addressing modes, wait-state arbitration, and prefetch refills, plus completed Batch 1.1 isomorphic arithmetic `SUB`, `SUBA`, `SUBI`, `SUBQ`, `SUBX`; verified under exhaustive Cartesian $2^k \times 2^M$ DMA contention and Chip/Fast RAM permutations with 100% state and cycle invariance; 100% SingleStepTests pass across all active instructions; remaining derived/isomorphic instructions scheduled in Step 1).
- **Memory Configuration:**
  - 512 KB Chip RAM (`$000000-$07FFFF`).
  - Optional 512 KB Trapdoor Slow RAM (`$C00000-$C7FFFF`).
  - Optional 4 MB Auto-Config Fast RAM (`$200000-$5FFFFF`).
- **Custom Chipset (OCS):**
  - Agnus 8370 (NTSC) / 8371 (PAL): Beam counters, Copper coprocessor, 4-channel Blitter, DMA arbitration.
  - Denise 8362: Bitplane serialization, 8 hardware sprites, 32-color palette, dual playfield.
  - Paula: 4-channel 8-bit audio DACs, floppy MFM controller, UART, interrupt priority encoder.
  - CIAs (2x MOS 8520): Timers A & B, I/O ports, TOD clock, keyboard shift register, overlay `_OVL`.
- **Peripherals & I/O:**
  - Dual game ports: Port 1 (Mouse), Port 2 (Joystick).
  - Floppy Drive: Internal DF0 with ADF byte slice injection.
  - Kickstart ROM: 256 KB (1.2 / 1.3) with low-memory overlay boot sequence.
- **Integrated Developer Debugger (Mandatory Phase 1 Deliverable):**
  - Headless backend engine with stepping (`step_cck`, `step_instruction`), M68000 disassembler, breakpoints, memory watchpoints, and 1024-entry execution trace ring buffer.
  - Interactive `egui` developer GUI panels (disassembly, registers, memory hex, Copper/Blitter inspector, and DMA logic analyzer).

### Phase 2: Enhanced Chipset (ECS) & Later Models
- **A500 Rev 6A (1 MB Chip):** Fat Agnus 8372A with 1 MB Chip RAM jumper configuration.
- **A500 Plus:** Full ECS chipset (Agnus 8372A 1MB, Denise 8373 with Productivity modes), Kickstart 2.04 (512 KB), onboard battery-backed RTC.
- **A1200 (AGA):** Motorola 68EC020 (32-bit), 2 MB Chip RAM, Alice, Lisa, 24-bit color palette.

### Peripheral Extensions (Post-Baseline)
- **4-Player Joystick Adapter:** Parallel port 4-joystick adapter for multiplayer games (e.g. *Super Skidmarks*, *Dynablaster*).
- **Analog Joysticks:** Proportional analog potentiometer sampling via `POT0DAT`/`POT1DAT`.
- **Light Pen / Gun:** Video beam position latching via `VPOSR`/`VHPOSR` registers and `BPLCON0` bit 3 (`LPEN`).

---
## 2. Core Implementation Strategy (Remaining Milestones)

### Step 1: Reintroduction of Isomorphic Instructions & Complex Operations
- **Phase A: Reintroduction of Isomorphic & Derived Instructions (from Archetype Blueprints):**
  - *Completed:* **Batch 1.1 (Isomorphic Arithmetic):** `SUB`, `SUBA`, `SUBI`, `SUBQ`, `SUBX` (derived from `ADD*` archetypes; 100% SingleStepTests & Cartesian DMA verified).
  - *Batch 1.2 (Isomorphic Logic):* `AND`, `ANDI`, `OR`, `ORI`, `EOR`, `EORI` (derived from `NOT`/`ADD` bitwise ALU blueprints).
  - *Batch 1.3 (Comparisons & Tests):* `CMP`, `CMPA`, `CMPI`, `TST` (derived from `CMPM` and subtractive flags).
    - *Test Harness Update Note:* MAME and Tom Harte bundle `CMP` and `CMPM` into `CMP.<size>.json` without separate `CMPM.<size>.json` files. When implementing `CMP`/`CMPA`/`CMPI`, update the `load_mame_tests` guard in [`test_dma_cartesian.rs`](crates/test_runner/tests/test_dma_cartesian.rs) and add full `CMP` tests alongside the filtered `CMPM` tests in [`test_singlestep.rs`](crates/test_runner/tests/test_singlestep.rs) so the entire `CMP` vector file is tested.
  - *Batch 1.4 (Isomorphic Shifts & Rotates):* `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR` (derived from `ASL` archetype).
  - *Batch 1.5 (Isomorphic Bit Operations):* `BTST`, `BCLR`, `BCHG` (derived from `BSET` archetype).
  - *Batch 1.6 (Remaining Move Sizes):* `MOVE.B`, `MOVE.L` (derived from `MOVE.W` archetype).
- **Phase B: Implementation of Remaining Complex & Multi-Cycle Instructions:**
  - *Batch 1.7 (Multiplication & Division):* `MULU` / `MULS` (38–70 clocks data-dependent), `DIVU` / `DIVS` (38–158 clocks data-dependent, divide-by-zero trap vector 5).
  - *Batch 1.8 (BCD & Math Extensions):* `ABCD`, `SBCD`, `NBCD`, `NEG`, `NEGX`, `CLR`, `EXT`.
  - *Batch 1.9 (Looping & Conditional Setting):* `DBcc`, `Scc`.
  - *Batch 1.10 (Stack & Frame Control):* `LINK`, `UNLK`, `LEA`, `EXG`, `SWAP`, `CHK`.
  - *Batch 1.11 (Privileged & Atomic Hardware Ops):* `MOVE to/from SR`, `MOVE USP`, `STOP`, `RESET`, `TAS` (indivisible RMW bus cycle with Amiga write-drop quirk).
- **Dual-Tier Verification Gate for Every Instruction Batch:**
  As each batch of instructions is reintroduced or implemented, it must pass a mandatory two-tier verification gate before the batch is declared complete:
  1. **Tier 1 — SingleStepTest Vector Verification:** 100% pass rate against MAME and Tom Harte hardware vectors in `crates/test_runner/tests/test_singlestep.rs`, validating architectural registers ($D_0-D_7, A_0-A_7$), CCR flags ($X, N, Z, V, C$), bus transaction counts, and prefetch behavior.
  2. **Tier 2 — Mandatory Cartesian DMA Contention Verification:** Every implemented or reintroduced instruction must be integrated into the Cartesian stress suite in `crates/test_runner/tests/test_dma_cartesian.rs`, asserting all hardware invariants across the full combinatorial product of $2^k$ memory classifications (Chip vs Fast RAM) and $2^M$ DMA cycle schedules:
     - *Cycle Invariance:* $C = C_0 + 2 \times \text{wait\_states}$.
     - *Fast RAM Immunity:* strictly 0 wait states under 100% Fast RAM ($C = C_0$).
     - *State Invariance:* bit-for-bit register and memory match with uncontended golden execution across the entire $2^k \times 2^M$ permutation space.
     - *Chip RAM Contention Poisoning:* memory inversion poisoning verifying that any illegal read/write during Agnus DMA stalls triggers assertion failures.


### Step 2: Comprehensive Opcode Benchmarking & Performance Profiling
- **Automated Per-Opcode Micro-Benchmark Harness:**
  - Develop an exhaustive automated micro-benchmark harness (e.g. using `criterion` and dedicated throughput harnesses in `crates/test_runner`) measuring host execution time, nanoseconds per instruction, and throughput (MIPS) across all 65,536 dispatch entries and instruction variants.
  - Test diverse operand combinations: data register direct, address register indirect with displacement/indexing, and immediate/memory forms under both cached and unblocked bus scenarios.
- **CPU Memory Footprint Audit & Host Cache Miss Profiling:**
  - **Memory Footprint Audit (in Kilobytes):** Measure and document the exact memory footprint of the CPU emulator core:
    - Host `.rodata` footprint: 65,536-entry static dispatch table (`sizeof(OpcodeDescriptor) * 65,536`), static `[MicroStep; N]` array slices, and decoding metadata.
    - Host runtime state footprint: `Cpu`, `CpuState`, `CpuMicroState` sizes in bytes, auditing L1d cache line alignment and residency.
  - **Host Cache Miss Profiling:** Measure host CPU performance counters (L1i instruction cache misses, L1d data cache misses, Last Level Cache / LLC misses, superscalar IPC, branch mispredictions) across opcode execution runs (via `perf stat`, cachegrind, or host PMU tooling).
  - Automatically rank handlers by host latency and flag operations exhibiting disproportionate execution overhead relative to emulated M68000 cycle counts.
- **Footprint Compaction Assessment & Mechanical Sympathy Optimization:**
  - **Footprint Reduction Feasibility:** Investigate whether compacting the CPU footprint (e.g., bit-packing `OpcodeDescriptor`, microcode array deduplication, index packing) yields measurable L1i/L1d miss reductions and throughput gains, or whether the current flat layout already maximizes host branch-predictor and cache throughput.
  - Refactor identified slow handlers using host CPU mechanical sympathy principles (direct specialized flattening, branchless bit operations, eliminated redundant register banking, and cross-crate MIR inlining).
  - Strictly enforce architectural constraints: zero custom macros (`macro_rules!`), zero const-generic function matrices, zero dynamic heap allocations, and zero compromise on code readability.
- **Fidelity & Regression Validation Gate:**
  - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity and passes the full exhaustive SingleStepTests suite (`$env:SINGLESTEP_FULL = "1"`) with zero regressions.

### Step 3: Standalone CPU Program Execution, Synthetic Workloads & Performance Projection
- **Direct Memory Program Injection & Execution:**
  - Build a headless execution harness that directly injects compiled M68000 binary routines (raw machine code binaries, assembled routines) into emulated RAM without requiring Kickstart ROM or OS overhead.
  - Set initial execution context ($PC$, $SSP$, $SR$) and execute self-contained test programs to completion, designated stop addresses, or trap returns.
  - Validate multi-instruction correctness, register state, and memory side effects on concrete routines (e.g. arithmetic loops, block memory transfers, array sorting).
- **Complex Workloads, Standard Synthetic Benchmarks & Real-World Cache Analysis:**
  - Load and execute established M68000 algorithmic benchmarks (e.g. Dhrystone, Sieve of Eratosthenes, math/ALU stress kernels) directly in the CPU core harness.
  - Assert functional correctness and numerical determinism across long-running execution sequences.
  - Measure baseline emulation throughput (effective MIPS, instruction throughput, host CPU cycle cost per emulated CCK).
  - **Real-Workload Host Cache Miss & Footprint Impact:** Profile host L1i/L1d cache misses and branch mispredictions during continuous execution of real test programs. Determine the empirical performance impact of CPU core memory footprint on real-world workloads, validating whether memory reduction efforts yield meaningful speedups.
- **Cross-Architecture & Mobile Performance Projections:**
  - Extrapolate measured desktop throughput (x86_64 / desktop ARM) to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile devices).
  - Model CPU overhead margins to ensure headroom for sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation once custom chipset DMA contention and rendering are integrated.
  - *(Deep research on binary formats, M68k assembly toolchains, and benchmark sources will be performed upon reaching this step).*

### Step 4: Custom Chipsets (Agnus, Denise, Paula, CIAs)
- **Step 4.1: Minimal Machine Main Loop (`A500::step_cck`) & Subsystem Orchestration:**
  - Create the top-level machine struct (`A500`) owning all primary subsystems without circular references: `cpu`, `memory_bus`, `cycle_counter`, `agnus`, `denise`, `paula`, `cia_a`, `cia_b`.
  - Multi-level stepping interfaces: `step_cck(cck: u64)`, `step_instruction()`, `step_cycles(n)`, `step_frame()`.
  - Strict lockstep Color Clock stepping: clock beam counters, advance DMA slots, clock CIAs, drive CPU CCK1/CCK2 bus phases against `MemoryBus`.
  - Central interrupt priority arbitration pipeline: sample Paula (Levels 1, 3, 4, 5), CIA-A (Level 2), and CIA-B (Level 6), calculate highest unmasked level, and drive `cpu.set_ipl()`.
- **Step 4.2: Machine-Wide Reset Sequencing (`reset_cold` & `reset_warm`):**
  - Physical `_RESET` line propagation across all chips.
  - Boot overlay engagement (`map_kickstart_to_low_memory` in `MemoryBus`).
  - *Cold Reset:* Zero physical RAM buffers (`$00`), reset chip registers to power-on defaults (`DMACON = $0000`, `INTENA/INTREQ = $0000`, CIA latches cleared), initialize CPU `SR = $2700`, load initial `SSP`/`PC` from `$000000`/`$000004` (Kickstart ROM), prime prefetch queue (`IR`, `IRC`).
  - *Warm Reset:* Preserve RAM contents intact (ensuring Kickstart memory checksum and resident module discovery pass), re-engage `_OVL`, assert chip reset lines, reload initial vectors.
  - Hardware keyboard reset line: wire `Ctrl-Amiga-Amiga` reset trigger line to main machine reset flow.
- **Step 4.3: Delayed Signal & Register Mutation Propagation Pipeline:**
  - *Physical Circuit Simulation:* Register reads return the currently latched active state **immediately** ("Read is NOW"). Register writes, strobes, and register mutations (e.g. `DMACON`, `BPLCON0`, `COLORxx`, `INTENA`, `COPJMP1`, `BLTSIZE`, CIA timer latches) do not take instantaneous cross-chip effect; they are staged and propagate after $K$ Color Clock phases / CCK cycles before altering the active execution path.
  - *Zero-Allocation Hot Path Design:* Model staged mutations using fixed-size inline pipeline latches / ring buffers (e.g. `[Option<DelayedWrite>; 4]` or fixed-capacity shift latches) embedded directly within chip structs. Zero dynamic heap allocation (`Vec`, `Box`) during CCK stepping.
  - *Save State Persistence:* The delayed mutation pipeline, staged values, and remaining cycle countdowns are fully serializable in save states (`AgnusState`, `DeniseState`, etc.), guaranteeing deterministic round-trip snapshot capture and rewind/restore even mid-propagation.
- **Step 4.4: Agnus DMA Bus Arbiter (Baseline Model & Contention Exposure):**
  - Implement the baseline Agnus horizontal scanline DMA slot schedule (CCK 0..3 DRAM refresh, CCK 4 disk, CCK 5..8 audio, CCK 12..27 sprites, bitplanes, and even/odd slots).
  - CPU and Blitter contention arbitration (`BLTPRI` Blitter Nasty mode).
  - Direct bus lock exposure: drive bus lock methods (`lock_chip_ram` / `unlock_chip_ram`) so the CPU and all custom chips observe bus contention and stall with wait states (`BusResult::WaitState`), establishing correct bus contention physics even before individual channel internal DSP/rendering logic is fully completed.
- **Step 4.5: Decomposed Subsystem Deep Implementations:**
  - *Agnus:* Copper coprocessor state machine (`MOVE`, `WAIT`, `SKIP`, `CDANG` danger mode), 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer, ascending/descending modes).
  - *Paula Audio Engine with Native BLEP Synthesis:* Precomputed alias-free BLEP tables (`blep_tables.rs`) across Paula's 4 DMA audio channels (dynamic CIA-A LED filter switching), floppy MFM track controller, serial UART, interrupt multiplexer.
  - *Denise:* Video pixel serializer, bitplanes (1–6), 8 hardware sprites, 32-color palette (RGB444), dual playfield, collision detection registers (`CLXDAT`, `CLXCON`).
  - *CIAs (Dual MOS 8520):* Timers A & B, TOD clock, serial shift register (SDR), parallel/control ports, E-clock synchronization.

### Step 5: Presentation, Host Integration & Full Interactive Debugger GUI
- Video rendering: Decoupled ARGB8888 frame buffer with 4:3 aspect ratio scaling.
- Audio sink: Ring buffer decoupled from host audio playback (`cpal` / Web Audio).
- GUI: Native and WebAssembly UI using `egui` + `wgpu`.
- GPU post-processing shaders for authentic CRT TV look and feel (scanlines, shadow mask, curvature, phosphor bloom).
- **Full Interactive Debugger Tool Windows:**
  - Live disassembly view with execution pointer and double-click breakpoints.
  - Interactive CPU register editor and CCR bit toggles.
  - 24-bit memory hex dump viewer with ASCII pane and live search.
  - Copper list visualizer with live beam position cursor.
  - DMA slot logic analyzer timeline.

---

## 3. AI Agent Testing & Differential Execution Strategy

To achieve cycle-exact accuracy and debug complex game/demo edge cases, the project leverages automated agent workflows and differential emulation:

### 3.1 Differential Testing Against Reference Emulators
- **Cross-Emulator Execution Harness:**
  - Run target test cases, test ROMs, or problematic games in verified reference emulators ([vAmiga](ref_src/vAmiga-4.5) and [WinUAE](ref_src/WinUAE-6030)).
  - Step emulation to a designated frame number or instruction milestone.
  - Dump execution traces, CPU registers, DMA channel allocations, and memory state.
  - Run the same binary on our emulator core and compare state dumps to detect cycle deviations instantly.

### 3.2 Automated vAmigaTS Test Suite Extraction
- Unpack and catalog the test disks in `ref_src/vAmigaTS/`.
- Implement headless runners that load each test ADF, execute until test completion, and verify screen buffers against reference PNG frame renders.

### 3.3 Visual & Audio Multimodal Validation
- **Screenshot Frame Dumps:**
  - Export rendered video frames at specific VBlank intervals.
  - Use visual comparison (pixel diffs or multimodal LLM inspection) to verify Copper color gradients, sprite multiplexing, and raster splits against WinUAE/vAmiga output.
- **Audio Sample Dumps:**
  - Capture raw PCM audio buffers from Paula channels at fixed cycle intervals and compare waveform phase and amplitude against hardware recordings.

### 3.4 Headless Debugger Interface for Agents
- Provide a machine-readable REST / IPC interface to the [Debugger](Obsidian/Amiga/Design/Debugger.md) engine:
  - Set PC breakpoints and memory watchpoints programmatically.
  - Query register state, disassembly, and execution trace history ring buffers.
  - Enable autonomous debugging agents to diagnose CPU hangs or crash dumps.

---

## 4. Post-Phase 1 Extensions: Reverse Engineering & Extraction

### 4.1 Resource Extractor & Reverse Engineering Assistant
- Extract graphics (bitplanes, sprites, palettes) and audio samples directly from memory buffers.
- Annotate assets, memory addresses, and game phases using LLM assistance.
- Map active assets to the visual 24-bit memory map.

### 4.2 LLM-Assisted Decompiler
- Integrate resource extraction to infer meaningful variable and function names.
- Provide synchronized side-by-side debugging of raw M68k disassembly alongside decompiled high-level logic.
