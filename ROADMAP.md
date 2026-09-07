# Amiga 500 Emulator: Development Roadmap & Strategy

This document outlines the phased development plan, hardware milestones, verification methodologies, and AI agent testing strategies for the Amiga 500 emulator project.

---

## 1. Hardware Roadmap & Milestones

### Phase 1: Baseline Amiga 500 (Rev 5 / Rev 6a OCS) — Immediate Focus
- **CPU:** Motorola 68000 cycle-exact core (Color Clock phase execution CCK1/CCK2).
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

> [!NOTE]
> **Completed Baseline Foundation:**
> - **Audio Antialiasing:** `tools/blep_generator` ported to Rust, BLEP tables validated against WinUAE `sinctable.cpp`.
> - **MemoryBus Architecture:** 24-bit physical decoding, 256-bank direct dispatch table, `_OVL` boot overlay, open bus floating physics (`$FF`/`$FFFF`), hardware quirks (`TAS` write-drop, CIA lane mapping), and OKI MSM6242B RTC integration.
> - **M68000 Baseline Core & Debugger:** All 12 addressing modes (with Vector 3 Address Error), 51 instruction suites verified (100% green against SingleStepTests), and headless debugger backend (disassembler, trace ring buffer, breakpoints/watchpoints).

### Step 1: Color Clock (CCK) Sub-Cycle Bus Interface & 2-Phase State Machine
- **Clocking & Synchronization Model:**
  - $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks (S0-S7)} = 2\ \text{Color Clocks (CCK1 + CCK2)}$.
  - CCK1 (Clocks S0–S3): Address output, `_AS` assertion, bus arbitration check against Agnus DMA. If blocked, Gary withholds `_DTACK` -> CPU pauses without advancing micro-step.
  - CCK2 (Clocks S4–S7): Data transfer, latch read data (`read_latch`) or commit write data, acknowledge `_DTACK`.
- **Bus Protocol Refinement in `crates/memory_bus`:**
  - Define `CckPhase` (`Cck1`, `Cck2`) and `BusCycle` representation (`addr`, `data`, `size`, `fc`, `is_read`, `uds`, `lds`).
  - Provide 2-phase API: `begin_cycle(addr, size, fc, is_read) -> MemoryBusResult` and `end_cycle(data) -> MemoryBusResult`.
- **CPU Micro-State Machine in `crates/m68000`:**
  - Introduce `CpuMicroState` / `CckPhase` / `active_bus_cycle: Option<BusCycle>` into CPU execution core.
  - Implement `step_cck(&mut bus) -> StepResult (InFlight | InstructionComplete | BusWait)`.
  - Seamlessly handle Chip RAM DMA contention: CPU holds intermediate states and wait cycles when `MemoryBusResult::Blocked` is returned.

### Step 2: Micro-Step Instruction Decomposition on 6 Representative Archetypes (Proof of Concept)
- **Decompose 6 Archetypes into Cycle-Exact Micro-Operations:**
  1. *Internal Register ALU (4 clocks / 2 CCKs):* `NOP`, `MOVE.w D0, D1` (Zero external memory cycles beyond opcode prefetch).
  2. *Memory Read (8 clocks / 4 CCKs):* `MOVE.w (A0), D0` (Opcode fetch + prefetch + memory read).
  3. *Memory Write (8 clocks / 4 CCKs):* `MOVE.w D0, (A0)` (Opcode fetch + memory write + prefetch).
  4. *Read-Modify-Write (12 clocks / 6 CCKs):* `ADD.w D0, (A0)` (Opcode fetch + memory read + ALU execution + memory write + prefetch).
  5. *Conditional Branching (10 vs 8 clocks):* `BRA.s` / `Bcc` (Taken branch: 10 clocks with pipeline flush/refetch; Untaken branch: 8 clocks with sequential prefetch).
  6. *Stack Push/Pop (Multi-Word):* `PEA (A0)` / `JSR (A0)` (Sequential stack memory writes across multiple CCK cycles).
- **Validation of Proof of Concept:**
  - Verify that each archetype matches exact cycle length (`test.length`) under unblocked memory execution.
  - Verify that injecting synthetic DMA stalls during CCK1 pauses the micro-step state machine without state corruption.

### Step 3: Cycle Length & Bus Transaction Verification Harness
- **Harness Extensions in `crates/test_runner`:**
  - Add cycle length verification asserting `cpu.cycles == test.length` against MAME and Tom Harte test vectors.
  - Implement bus cycle transaction recording (`[cycle, address, value, type, uds, lds]`) and match against Tom Harte's silicon bus transaction logs.
  - Add synthetic Agnus DMA contention test runner: inject DMA stalls at every possible CCK phase of each instruction to verify invariant preservation (final registers and memory identical, total cycles increased by wait states).

### Step 4: Systematic Migration of Existing 51 Instructions to CCK Engine
- **Structured Migration Batches:**
  - *Batch 1 (Simple ALU & Immediate):* `ADD`, `ADDA`, `ADDI`, `ADDQ`, `SUB`, `SUBA`, `SUBI`, `SUBQ`, `CMP`, `CMPA`, `CMPI`, `CMPM`, `TST`.
  - *Batch 2 (Bitwise Logic & Bit Ops):* `AND`, `OR`, `BTST`, `BSET`, `BCLR`, `BCHG`.
  - *Batch 3 (Shifts & Rotates):* `ASL`, `ASR`, `LSL`, `LSR` (Dynamic micro-step loops: 6/8 base clocks + 2 clocks per bit shifted).
  - *Batch 4 (Data Movement):* `MOVE.b/w/l`, `MOVEA.w/l`.
  - *Batch 5 (Extended Arithmetic & Control Flow):* `ADDX`, `SUBX`, `BRA`, `Bcc`, `JMP`, `JSR`, `RTS`, `TRAP`, `NOP`.
- **Milestone Gate:** All 51 migrated instructions pass 100% green in SingleStepTests with both state match AND exact cycle count (`test.length`).

### Step 5: Implementation of Remaining Complex & Multi-Cycle Instructions
- **Implement Directly in CCK Engine:**
  - *Batch 6 (Multi-Register Moves):* `MOVEM` (looping bus cycles, predecrement/postincrement register ordering, interrupt sensitivity).
  - *Batch 7 (Multiplication & Division):* `MULU` / `MULS` (38–70 clocks data-dependent), `DIVU` / `DIVS` (38–158 clocks data-dependent, divide-by-zero trap vector 5).
  - *Batch 8 (BCD & Math Extensions):* `ABCD`, `SBCD`, `NBCD`, `NEG`, `NEGX`, `CLR`, `NOT`, `EXT`.
  - *Batch 9 (Looping & Conditional Setting):* `DBcc`, `Scc`.
  - *Batch 10 (Stack & Frame Control):* `LINK`, `UNLK`, `PEA`, `LEA`, `EXG`, `SWAP`, `CHK`.
  - *Batch 11 (Privileged & Atomic Hardware Ops):* `MOVE to/from SR`, `MOVE USP`, `STOP`, `RESET`, `TAS` (indivisible RMW bus cycle with Amiga write-drop quirk).
- **100% SingleStepTest Pass Rate Target:** Complete all 127 MAME suites and 125 Tom Harte suites with both register/memory match and cycle/bus-exact match.

### Step 6: In-Memory Mutations & Dynamic DMA Contention Stress Testing
- **Address Space Remapping Mutations:**
  - Execute SingleStepTest suites with programmatic address remapping mutations (per [CPU SingleStepTests.md](Obsidian/Amiga/Design/CPU%20SingleStepTests.md#8-in-code-test-mutation-strategy-chipfast-ram--dma-contention)):
    - `ForceChipRam`: Offset code, operands, and stack into Chip RAM (`$000000-$07FFFF`) to test contention and Gary bus limits.
    - `ForceFastRam`: Offset addresses into Auto-Config Fast RAM (`$200000-$27FFFF`) to verify zero-wait-state full-speed execution.
    - `ForceSlowRam`: Remap addresses into A501 Slow / Trapdoor RAM (`$C00000-$C7FFFF`).
    - `MixedChipFast`: Map instruction opcodes in Fast RAM while placing data operands in Chip RAM (and vice versa).
- **Simulated Agnus DMA Bus Contention (`DmaSchedule`):**
  - Inject parameterized DMA bus contention schedules into the CPU Color Clock phases (CCK1/CCK2):
    - Alternating cycle stalls (simulating display bitplane and Copper DMA).
    - Burst stalls (simulating Blitter nastiness blocking the CPU for $N$ consecutive CCK cycles).
  - Verify bus arbitration invariants: CPU properly pauses instruction phase on `MemoryBusResult::Blocked`, accumulates wait states, and matches final register/memory state with exact cycle count increases.

### Step 7: Custom Chipsets (Agnus, Denise, Paula, CIAs)
- Decompose monolithic chip logic into focused subcomponents (Copper, Blitter, DMA, Audio, Floppy, Timers, Ports).
- **Paula Audio Engine with Native BLEP Synthesis:** Integrate the precomputed BLEP tables (`blep_tables.rs`) generated by `tools/blep_generator` into Paula's 4 DMA audio channels, enabling alias-free variable-rate PCM playback across Amiga 500 and Amiga 1200 models (supporting dynamic CIA-A LED low-pass filter switching).
- Implement interrupt priority line (IPL 1–6) aggregation and main loop arbitration.
- Wire multi-chip peripherals (Floppy drive, Game ports, Keyboard reset line).

### Step 8: Presentation, Host Integration & Full Interactive Debugger GUI
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
