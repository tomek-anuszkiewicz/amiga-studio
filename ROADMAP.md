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
> - **M68000 Baseline Core & Debugger:** All 12 addressing modes (with Vector 3 Address Error), 52 instruction suites verified (100% green against SingleStepTests including PEA), and headless debugger backend (disassembler, trace ring buffer, breakpoints/watchpoints).
> - **CCK Sub-Cycle Bus Interface & 2-Phase State Machine:** 2-phase Color Clock protocol (CCK1 address/arbitration, CCK2 data commit/_DTACK), structured `BusCycle` with UDS/LDS strobes, `CpuMicroState` machine, and cycle-exact Chip RAM DMA wait-state stalling in `MemoryBus` and `Cpu`.
> - **Micro-Step Instruction Decomposition (6 Archetypes PoC):** Cycle-exact sub-cycle micro-step engine decomposing all 6 representative instruction archetypes: Internal ALU (`NOP`, `MOVE.w Dx, Dy` - 4 clocks / 2 CCKs), Memory Read (`MOVE.w (Ax), Dy` - 8 clocks / 4 CCKs), Memory Write (`MOVE.w Dx, (Ay)` - 8 clocks / 4 CCKs), Read-Modify-Write Class 0 (`ADD.w Dx, (Ay)` - 12 clocks / 6 CCKs), Branching (`Bcc.s` / `BRA.s` - untaken 8 clocks / taken 10 clocks), and Stack Push/Call (`PEA (An)` - 12 clocks / 6 CCKs, `JSR (An)` - 16 clocks / 8 CCKs). Validated with 100% green unit tests and synthetic CCK1/CCK2 DMA contention stalls.
> - **Cycle Length, Bus Transaction & DMA Contention Harness:** Cycle length verification (`VerifyMode::StateAndCycles` / `Full`), zero-allocation bus transaction recording in `m68000::Cpu` and matching in `crates/test_runner` (`transactions.rs`) against Tom Harte silicon and MAME logs (read/write/TAS, 24-bit address, size, data, FC lines, strobes). Synthetic Agnus DMA contention runner (`dma_harness.rs`) verifying State Invariance and Cycle Invariance under single-cycle and burst stalls. Validated 100% green on `NOP` and `PEA (An)`.
> - **Linearized 13 ALU Instructions (Step 1 Batch 1):** Full cycle-exact Color Clock migration of 13 arithmetic/comparison instructions (`ADD`, `ADDA`, `ADDI`, `ADDQ`, `SUB`, `SUBA`, `SUBI`, `SUBQ`, `CMP`, `CMPA`, `CMPI`, `CMPM`, `TST`) with compile-time flattened dispatch (`linear_arithmetic.rs`, `linear_compare.rs`, `linear_ea.rs`), passing 100% green across MAME and Tom Harte SingleStepTests.
> - **Linearized Logic & Bit Instructions (Step 1 Batch 2):** Full cycle-exact Color Clock migration of bitwise logic (`AND`, `OR`, `EOR`, `NOT`, `ANDI`, `ORI`, `EORI`) and bit manipulation (`BTST`, `BSET`, `BCLR`, `BCHG`) instructions with compile-time flattened dispatch (`linear_logic.rs`, `linear_bits.rs`, `linear_ea.rs`), passing 100% green across MAME and Tom Harte SingleStepTests.
> - **Linearized Shift & Rotate Instructions (Step 1 Batch 3):** Full cycle-exact Color Clock migration of all Group 0xE shift and rotate instructions (`ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR`) across register ($6/8 + 2n$ clocks) and memory (Word size, count = 1, Class 0 RMW writeback, 12–16 clocks) forms with compile-time flattened dispatch (`linear_shifts.rs`), passing 100% green across all 24 suites in SingleStepTests.
> - **Linearized All 52 Baseline Instructions (Step 1 Complete - Batches 1 to 5):** Full cycle-exact Color Clock migration of all 52 baseline instructions: Batch 1 (ALU & Immediate: 13 instructions), Batch 2 (Bitwise Logic & Bit Ops: 11 instructions), Batch 3 (Shifts & Rotates: 8 instructions), Batch 4 (Data Movement: 12,288 opcodes across 5 instructions), and Batch 5 (Extended Arithmetic & Control Flow: 9 instructions: `ADDX`, `SUBX`, `BRA`, `Bcc`, `JMP`, `JSR`, `RTS`, `TRAP`, `NOP`). Complete compile-time static dispatch tables (`ADDX_SUBX_REG_TABLE`, `ADDX_SUBX_MEM_TABLE`, `BCC_TABLE`, `JMP_TABLE`, `JSR_TABLE`, `MOVE_REG_TABLE`, `MOVE_MEM_TABLE`, etc.), total elimination of legacy monolithic interpreters and cascading branches in hot paths, and 100% green pass rate across SingleStepTests (all 77 suites in `test_singlestep`).
> - **Modular Per-Mnemonic Architecture & Single-Mnemonic Dispatch:** Reorganized `crates/m68000/src/instructions` into individual per-mnemonic modules (`add.rs`, `sub.rs`, `move.rs`, `movea.rs`, `bra.rs`, `bsr.rs`, `bcc.rs`, `asl.rs`, `asr.rs`, `lsl.rs`, `lsr.rs`, `roxl.rs`, `roxr.rs`, `rol.rs`, `ror.rs`, etc.), moving all arithmetic/CCR logic and cycle-exact execution handlers inside. All 65,536 entries in `dispatch_table.rs` map directly to single-mnemonic static handlers (eliminating all dynamic branching for `ADD`/`SUB`, `AND`/`OR`, `MOVE`/`MOVEA`, `BRA`/`BSR`/`Bcc`, shifts & rotates), deleted all obsolete `linear_*.rs` files, and verified 100% green across all unit, architectural, and single-step tests.

### Step 1: In-Memory Mutations & Dynamic DMA Contention Stress Testing
- **Address Space Remapping Mutations:**
  - Execute SingleStepTest suites with programmatic address remapping mutations on the linearized 52 instructions (per [CPU SingleStepTests.md](Obsidian/Amiga/Design/CPU%20SingleStepTests.md#8-in-code-test-mutation-strategy-chipfast-ram--dma-contention)):
    - `ForceChipRam`: Offset code, operands, and stack into Chip RAM (`$000000-$07FFFF`) to test contention and Gary bus limits.
    - `ForceFastRam`: Offset addresses into Auto-Config Fast RAM (`$200000-$27FFFF`) to verify zero-wait-state full-speed execution.
    - `ForceSlowRam`: Remap addresses into A501 Slow / Trapdoor RAM (`$C00000-$C7FFFF`).
    - `MixedChipFast`: Map instruction opcodes in Fast RAM while placing data operands in Chip RAM (and vice versa).
- **Simulated Agnus DMA Bus Contention (`DmaSchedule`):**
  - Inject parameterized DMA bus contention schedules into the CPU Color Clock phases (CCK1/CCK2):
    - Alternating cycle stalls (simulating display bitplane and Copper DMA).
    - Burst stalls (simulating Blitter nastiness blocking the CPU for $N$ consecutive CCK cycles).
  - Verify bus arbitration invariants: CPU properly pauses instruction phase on `MemoryBusResult::Blocked`, accumulates wait states, and matches final register/memory state with exact cycle count increases.
- **Milestone Gate:** Linearized 52 instructions maintain 100% state invariance and cycle invariance across Chip RAM, Fast RAM, Slow RAM, and under single-cycle and burst DMA contention.

### Step 2: Implementation of Remaining Complex & Multi-Cycle Instructions
- **Implement Directly as Linear Handlers in CCK Engine:**
  - *Batch 6 (Multi-Register Moves):* `MOVEM` (looping bus cycles, predecrement/postincrement register ordering, interrupt sensitivity).
  - *Batch 7 (Multiplication & Division):* `MULU` / `MULS` (38–70 clocks data-dependent), `DIVU` / `DIVS` (38–158 clocks data-dependent, divide-by-zero trap vector 5).
  - *Batch 8 (BCD & Math Extensions):* `ABCD`, `SBCD`, `NBCD`, `NEG`, `NEGX`, `CLR`, `EXT`. (Note: `NOT` migrated in Batch 2).
  - *Batch 9 (Looping & Conditional Setting):* `DBcc`, `Scc`.
  - *Batch 10 (Stack & Frame Control):* `LINK`, `UNLK`, `PEA`, `LEA`, `EXG`, `SWAP`, `CHK`.
  - *Batch 11 (Privileged & Atomic Hardware Ops):* `MOVE to/from SR`, `MOVE USP`, `STOP`, `RESET`, `TAS` (indivisible RMW bus cycle with Amiga write-drop quirk).
- **100% SingleStepTest Pass Rate Target:** Complete all 127 MAME suites and 125 Tom Harte suites with both register/memory match and cycle/bus-exact match.

### Step 3: Custom Chipsets (Agnus, Denise, Paula, CIAs)
- **Step 3.1: Minimal Machine Main Loop (`A500::step_cck`) & Subsystem Orchestration:**
  - Create the top-level machine struct (`A500`) owning all primary subsystems without circular references: `cpu`, `memory_bus`, `cycle_counter`, `agnus`, `denise`, `paula`, `cia_a`, `cia_b`.
  - Multi-level stepping interfaces: `step_cck(cck: u64)`, `step_instruction()`, `step_cycles(n)`, `step_frame()`.
  - Strict lockstep Color Clock stepping: clock beam counters, advance DMA slots, clock CIAs, drive CPU CCK1/CCK2 bus phases against `MemoryBus`.
  - Central interrupt priority arbitration pipeline: sample Paula (Levels 1, 3, 4, 5), CIA-A (Level 2), and CIA-B (Level 6), calculate highest unmasked level, and drive `cpu.set_ipl()`.
- **Step 3.2: Machine-Wide Reset Sequencing (`reset_cold` & `reset_warm`):**
  - Physical `_RESET` line propagation across all chips.
  - Boot overlay engagement (`map_kickstart_to_low_memory` in `MemoryBus`).
  - *Cold Reset:* Zero physical RAM buffers (`$00`), reset chip registers to power-on defaults (`DMACON = $0000`, `INTENA/INTREQ = $0000`, CIA latches cleared), initialize CPU `SR = $2700`, load initial `SSP`/`PC` from `$000000`/`$000004` (Kickstart ROM), prime prefetch queue (`IR`, `IRC`).
  - *Warm Reset:* Preserve RAM contents intact (ensuring Kickstart memory checksum and resident module discovery pass), re-engage `_OVL`, assert chip reset lines, reload initial vectors.
  - Hardware keyboard reset line: wire `Ctrl-Amiga-Amiga` reset trigger line to main machine reset flow.
- **Step 3.3: Delayed Signal & Register Mutation Propagation Pipeline:**
  - *Physical Circuit Simulation:* Register reads return the currently latched active state **immediately** ("Read is NOW"). Register writes, strobes, and register mutations (e.g. `DMACON`, `BPLCON0`, `COLORxx`, `INTENA`, `COPJMP1`, `BLTSIZE`, CIA timer latches) do not take instantaneous cross-chip effect; they are staged and propagate after $K$ Color Clock phases / CCK cycles before altering the active execution path.
  - *Zero-Allocation Hot Path Design:* Model staged mutations using fixed-size inline pipeline latches / ring buffers (e.g. `[Option<DelayedWrite>; 4]` or fixed-capacity shift latches) embedded directly within chip structs. Zero dynamic heap allocation (`Vec`, `Box`) during CCK stepping.
  - *Save State Persistence:* The delayed mutation pipeline, staged values, and remaining cycle countdowns are fully serializable in save states (`AgnusState`, `DeniseState`, etc.), guaranteeing deterministic round-trip snapshot capture and rewind/restore even mid-propagation.
- **Step 3.4: Agnus DMA Bus Arbiter (Baseline Model & Contention Exposure):**
  - Implement the baseline Agnus horizontal scanline DMA slot schedule (CCK 0..3 DRAM refresh, CCK 4 disk, CCK 5..8 audio, CCK 12..27 sprites, bitplanes, and even/odd slots).
  - CPU and Blitter contention arbitration (`BLTPRI` Blitter Nasty mode).
  - Direct bus lock exposure: drive `MemoryBus::set_chip_ram_blocked(bool)` so the CPU and all custom chips observe bus contention and stall with wait states (`MemoryBusResult::Blocked`), establishing correct bus contention physics even before individual channel internal DSP/rendering logic is fully completed.
- **Step 3.5: Decomposed Subsystem Deep Implementations:**
  - *Agnus:* Copper coprocessor state machine (`MOVE`, `WAIT`, `SKIP`, `CDANG` danger mode), 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer, ascending/descending modes).
  - *Paula Audio Engine with Native BLEP Synthesis:* Precomputed alias-free BLEP tables (`blep_tables.rs`) across Paula's 4 DMA audio channels (dynamic CIA-A LED filter switching), floppy MFM track controller, serial UART, interrupt multiplexer.
  - *Denise:* Video pixel serializer, bitplanes (1–6), 8 hardware sprites, 32-color palette (RGB444), dual playfield, collision detection registers (`CLXDAT`, `CLXCON`).
  - *CIAs (Dual MOS 8520):* Timers A & B, TOD clock, serial shift register (SDR), parallel/control ports, E-clock synchronization.

### Step 4: Presentation, Host Integration & Full Interactive Debugger GUI
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
