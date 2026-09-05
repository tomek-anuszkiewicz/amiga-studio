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

---

## 2. Core Implementation Strategy

### Step 1: BlepGenerator (Audio Antialiasing)
- Port the BlepGenerator logic to Rust.
- Validate band-limited step (BLEP) curves against reference audio output to ensure clean, alias-free sound rendering for Paula's variable-rate audio channels.

### Step 2: M68000 CPU Subsystem & Early Debugger Backend
> [!IMPORTANT]
> **Debugger is an immediate prerequisite:** Building and validating a cycle-exact CPU without an inspection backend is nearly impossible. The core headless debugger engine must be implemented concurrently with the CPU.

- **Early Debugger Primitives (Required Immediately):**
  - M68000 opcode disassembler (decodes instructions to human-readable strings).
  - Stepping primitives (`step_instruction`, `step_cck`).
  - PC execution breakpoints and memory read/write watchpoints.
  - Read-only state inspection (`CpuState`, registers $D_0-D_7$, $A_0-A_7$, $SR$, CCR flags, prefetch queue).
  - Fixed-size trace ring buffer (last 1024 instructions) for instant post-mortem diagnosis of test failures or crashes.
- **CPU Core Implementation:**
  - Build cycle-exact instruction execution state machine mapped to CCK phases.
  - Implement one instruction from each instruction group (Data Movement, Arithmetic, Logic, Shifts, Bit Manipulation, Control Flow, System/Exceptions).
  - Validate against the 127 per-instruction test suites in `ref_src/SingleStepTests-m68000/v1/`.
  - Employ an autonomous agentic development loop using the `add-m68k-instruction` and `m68k-singlestep-test` skills.
  - Test address errors, unaligned accesses, prefetch queue pipeline (`IR`/`IRC`), and status register condition code quirks (`TAS`, `TRAPV`).

### Step 3: MemoryBus & Bus Contention
- Implement 24-bit physical decoding and two-phase CCK bus latching (`read_phase1`/`read_phase2`, `write_phase1`/`write_phase2`).
- Simulate Agnus cycle stealing and DMA wait states on Chip RAM via `MemoryBusResult::Blocked`.

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
  - Run target test cases, test ROMs, or problematic games in verified reference emulators ([vAmiga](file:///d:/Programowanie/Amiga/ref_src/vAmiga-4.5) and [WinUAE](file:///d:/Programowanie/Amiga/ref_src/WinUAE-6030)).
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
- Provide a machine-readable REST / IPC interface to the [Debugger](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/Debugger.md) engine:
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
