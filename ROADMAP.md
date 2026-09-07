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

## 2. Core Implementation Strategy

### Step 1: BlepGenerator — Rust Port & Audio Antialiasing Validation
- **Convert [tools/BlebGenerator](tools/BlebGenerator) to Rust:**
  - Port the C# table generation implementation ([`tools/BlebGenerator/genblepsharp.cs`](tools/BlebGenerator/genblepsharp.cs)) into a native Rust tool (`tools/bleb-generator` or cargo workspace utility crate).
  - Port the analytical audio filter transfer function, windowed sinc integration, and BLEP table calculation pipeline.
  - Generate band-limited step (BLEP) interpolation tables for Paula's audio channels across A500 PAL (~3.54 MHz) and NTSC (~3.58 MHz) clock domains.
- **Validate Against WinUAE Ground Truth:**
  - Validate the generated tables directly against WinUAE's reference `winsinc_integral` tables ([`ref_src/WinUAE-6030/sinctable.cpp`](ref_src/WinUAE-6030/sinctable.cpp)) to ensure clean, alias-free sound rendering for Paula's variable-rate audio channels.
- **Embed Static Tables in Paula Core:**
  - Embed the precalculated BLEP sinc tables as static arrays in Paula's audio rendering pipeline, adhering strictly to the zero-allocation hot-path guideline.

### Step 2: MemoryBus & 2-Phase CCK Bus Arbitration
- **24-Bit Physical Address Decoding:**
  - Implement full 24-bit physical decoding: Chip RAM (512 KB/1 MB), Slow RAM (`$C00000`), Fast RAM (`$200000`), Kickstart ROM (`$F80000`), and hardware register spaces (`$DFF000`, `$BFE001`, `$BFD000`).
  - Implement low-memory boot overlay (`_OVL`): route `$000000-$07FFFF` to Kickstart ROM on cold/warm reset until cleared.
  - Model open bus behavior: floating address lines return `$FF` / `$FFFF` on unmapped spaces without bus errors.
  - Implement Amiga hardware quirks: `TAS` write-drop in Chip/Slow RAM, CIA byte lane mapping (even on CIA-B, odd on CIA-A).
- **Sub-Cycle 2-Phase CCK Bus Arbitration:**
  - Implement two-phase CCK latching (`read_phase1`/`read_phase2`, `write_phase1`/`write_phase2`) returning `MemoryBusResult` (`Phase1Ready`, `Ready(u16)`, `Blocked`).
  - Transparent read latch buffer (`read_latch`) isolating CPU during bus wait states.
  - Simulate Agnus cycle stealing and DMA wait states on Chip RAM via `lock_chip_ram()` / `unlock_chip_ram()`.
  - Provide a test-loading mode to populate arbitrary RAM bytes for `SingleStepTests` execution.

### Step 3a: M68000 CPU Foundations, All Addressing Modes & Early Debugger Backend
> [!IMPORTANT]
> **Debugger is an immediate prerequisite:** Building and validating a cycle-exact CPU without an inspection backend is nearly impossible. The core headless debugger engine must be implemented concurrently with the CPU.

- **Early Debugger Primitives (Required Immediately):**
  - Built-in zero-dependency M68000 opcode disassembler (decodes instructions to human-readable strings).
  - Stepping primitives (`step_instruction`, `step_cck`).
  - PC execution breakpoints and memory read/write watchpoints.
  - Read-only state inspection (`CpuState`, registers $D_0-D_7$, $A_0-A_7$, $SR$, CCR flags, prefetch queue).
  - Fixed-size trace ring buffer (last 1024 instructions) for instant post-mortem diagnosis of test failures or crashes.
- **CPU Core & Addressing Modes Engine:**
  - Build cycle-exact instruction execution state machine mapped to CCK phases (CCK1/CCK2), directly interfaced with `MemoryBus`.
  - Implement and thoroughly test **all M68000 addressing modes** upfront:
    - Data & Address Register Direct (`Dn`, `An`)
    - Address Register Indirect (`(An)`)
    - Address Register Indirect with Postincrement (`(An)+`) and Predecrement (`-(An)`) [with `A7` 2-byte alignment quirk on byte ops]
    - Address Register Indirect with Displacement (`(d16, An)`)
    - Address Register Indirect with Index (`(d8, An, Xn)`) [parsing brief extension word]
    - Absolute Short & Long (`(xxx).W`, `(xxx).L`)
    - Program Counter with Displacement & Index (`(d16, PC)`, `(d8, PC, Xn)`)
    - Immediate data & Status Register (`#<data>`, `SR`, `CCR`)
- **Initial Representative Instructions (One from Each Category):**
  - Implement a representative instruction from every major instruction category to exercise the execution pipeline:
    - *Data Movement:* `MOVE` / `MOVEA`
    - *Integer Arithmetic:* `ADD` / `SUB`
    - *Logic:* `AND` / `OR`
    - *Shift & Rotate:* `LSL` / `LSR` (or `ASL` / `ASR`)
    - *Bit Manipulation:* `BTST` / `BSET`
    - *Control Flow:* `BRA` / `Bcc` / `JMP` / `RTS`
    - *System & Exceptions:* `NOP`, `TRAP`, address error / unaligned access exception handling (7-word stack frame).
  - Initial SingleStepTest validation run using `m68k-singlestep-test` to ensure bus cycle timing, prefetch queue (`IR`/`IRC`), and CCR calculations are exact across all addressing modes.

### Step 3b: Full M68000 Instruction Set Completion & Cycle-Exact Validation
- **Implement All Remaining Instructions:**
  - Complete the full M68000 opcode matrix across all categories:
    - Block & specialized moves: `MOVEM`, `MOVEP`, `EXG`, `LEA`, `PEA`
    - Extended/BCD arithmetic: `ADDX`, `SUBX`, `NEGX`, `ABCD`, `SBCD`, `NBCD`, `MULS`, `MULU`, `DIVS`, `DIVU`, `EXT`
    - Bit/comparison operations: `BCHG`, `BCLR`, `TST`, `CMP`, `CMPA`, `CMPI`, `CMPM`, `CLR`, `NEG`, `NOT`
    - Specialized control & loops: `DBcc`, `Scc`, `JSR`, `RTE`, `RTR`, `LINK`, `UNLK`, `SWAP`, `CHK`
    - Privileged & atomic instructions: `STOP`, `RESET`, `TAS`, `TRAPV`, `MOVE to SR/CCR`, `MOVE from SR`, `MOVE USP`
- **Comprehensive SingleStepTest Suite Coverage:**
  - Autonomous agentic loop using `add-m68k-instruction` and `m68k-singlestep-test` skills.
  - 100% pass rate against all 127 per-instruction test suites in [`ref_src/SingleStepTests-m68000/v1/`](ref_src/SingleStepTests-m68000/v1) (MAME).
  - 100% pass rate against all 125 compressed suites in [`ref_src/SingleStepTests-680x0/68000/v1/`](ref_src/SingleStepTests-680x0/68000/v1) (Tom Harte).
  - Rigorous verification of edge cases: unaligned word/long address errors, prefetch queue reload delays, bus cycle states, and condition code quirks.
  - Cycle-Exact Diagnostics: log the exact execution cycle/CCK phase and bus transaction on test failure during cycle-stepped execution.

### Step 4: Custom Chipsets (Agnus, Denise, Paula, CIAs)
- Decompose monolithic chip logic into focused subcomponents (Copper, Blitter, DMA, Audio, Floppy, Timers, Ports).
- Implement interrupt priority line (IPL 1–6) aggregation and main loop arbitration.
- Wire multi-chip peripherals (Floppy drive, Game ports, Keyboard reset line).

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
