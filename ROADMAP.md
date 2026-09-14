# Amiga 500 Emulator: Development Roadmap & Strategy

This document outlines the phased development plan, hardware milestones, verification methodologies, and AI agent testing strategies for the Amiga 500 emulator project.

---

## 1. Hardware Roadmap & Milestones

### Phase 1: Baseline Amiga 500 (Rev 5 / Rev 6a OCS) — Immediate Focus
- **CPU:** Motorola 68000 cycle-exact core based on the **Microcode Archetype Baseline** (Native 2-clock micro-step slices: $1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$, 100% complete opcode coverage with 45,565 active valid opcodes out of 65,536 across Batches 1.1–1.11; 100% SingleStepTests pass across all test suites against physical silicon hardware captures [Tom Harte SingleStepTests-680x0]; 100% Cartesian DMA contention verification asserting cycle and state invariance across the full $2^k \times 2^M$ permutation space).
- **Memory Configuration:**
  - 512 KB Chip RAM ($000000-).
  - Optional 512 KB Trapdoor Slow RAM ($C00000-).
  - Optional 4 MB Auto-Config Fast RAM ($200000-).
- **Custom Chipset (OCS):**
  - Agnus 8370 (NTSC) / 8371 (PAL): Beam counters, Copper coprocessor, 4-channel Blitter, DMA arbitration.
  - Denise 8362: Bitplane serialization, 8 hardware sprites, 32-color palette, dual playfield.
  - Paula: 4-channel 8-bit audio DACs, floppy MFM controller, UART, interrupt priority encoder.
  - CIAs (2x MOS 8520): Timers A & B, I/O ports, TOD clock, keyboard shift register, overlay _OVL.
- **Peripherals & I/O:**
  - Dual game ports: Port 1 (Mouse), Port 2 (Joystick).
  - Floppy Drive: Internal DF0 with ADF byte slice injection.
  - Kickstart ROM: 256 KB (1.2 / 1.3) with low-memory overlay boot sequence.
- **Integrated Developer Debugger & Developer Studio GUI (Completed Baseline Deliverable):**
  - Headless backend engine with stepping (step_cck, step_instruction), cycle-exact M68000 disassembler (with LEA, CLR, DBRA, TST, NOT, NEG support), breakpoints, memory watchpoints, and 1024-entry execution trace ring buffer (crates/debugger).
  - Cross-platform immediate-mode Developer Studio GUI (crates/gui via egui/eframe) with synchronous direct-state pull, bounded execution time-slicing, 1:1 layout mapping, live CPU register diffs, self-documenting contextual tooltips, CCR LED toggles, collapsible microcode inspector with live clock counters (F8), fixed-width cell slot memory hex editor, clean game screen mode (F12), binary injection loader, and lean circular temporal time-travel scrubber (25,000 frames default, paused by default, with disassembly historical loop iteration rewind).
  - 100% covered by 23 automated integration and UI interaction tests (crates/gui/tests/).
- **Per-Opcode Micro-Benchmark Harness & Verification Framework (Completed Baseline Deliverable):**
  - Exhaustive cycle-exact benchmarking framework in crates/test_runner measuring instruction execution time, cycles per instruction, and throughput across all 65,536 dispatch entries and operand modes.
  - Comprehensive golden reference datasets (m68k_benchmark_baseline.csv, m68k_benchmark_baseline.json) with cryptographic SHA256 anti-tamper contracts (golden_row_hashes.rs).
  - Automated execution trace audit logger (--dump-traces), statistical variance tracker, and cycle anomaly detector.
- **Repository Sanitization & History Scrubbing (Completed Baseline Deliverable):**
  - Full Git history audit, Conventional Commits normalization, pruning of 146 empty/redundant commits (preserving author/committer timestamps), and deep asset purge (decoupling `ref_src`, PDFs, schematics, and caches), shrinking repository packfile from ~2.8 GB to 4.43 MB (99.8% reduction).
- **Custom Chipset Integration & Autonomous Execution Engines (Completed Baseline Deliverable):**
  - Scaffolded 17 flat workspace crates (`copper`, `blitter`, `dma`, `agnus`, `sprites`, `frame_builder`, `mouse`, `joystick`, `denise`, `audio`, `floppy`, `serial_port`, `paula`, `keyboard`, `parallel_port`, `cia`, `machine_loop`) with zero circular references and parameter-based borrow splitting.
  - Complete custom chip hardware registers with calibrated CCK electronic propagation delay pipelines ($K$-phase latency) and zero-cost stack-allocated `MemoryBus<'a>` router decoding Custom Chips (`$DF`), CIAs (`$BF`), and RTC (`$DC`).
  - Action dispatch pipeline bridging register writes to strongly-typed subsystem methods (`FloppyDrive`, `Blitter`, `Audio`, `Copper`, `Denise`).
  - Decoupled `A500State` serialization supporting referenced/self-contained modes, JSON and gzip compression (<50 KB), and 5-slot in-memory quick-save ring buffer with Developer Studio UI integration (`F6`/`F9`).
  - Machine-wide cold (`reset_cold`) and warm (`reset_warm`) reset sequencing, privileged M68000 `RESET` line propagation, and hardware `Ctrl-Amiga-Amiga` reset trigger.
  - End-to-end multi-chip interrupt priority arbitration pipeline (Levels 1–6) across Agnus, Paula, and CIAs with autovector exception processing.
  - Autonomous chipset execution engines: Agnus Copper coprocessor (`MOVE`, `WAIT`, `SKIP`, `CDANG`), Agnus 4-channel DMA Blitter (256-minterm Boolean ALU, barrel shifters, line drawer), Denise pixel pipeline (bitplane serializer, 32-color palette, EHB, HAM6, dual playfield) and 8 hardware sprites, Paula 4-channel audio sample streaming, Floppy MFM track encoder/decoder with ADF injection, and dual MOS 8520 CIAs (timers, TOD, keyboard serial shift register).
  - Agnus master DMA bus arbiter with 227/226 CCK horizontal scanline scheduling, 8-tier bus priority, Blitter Nasty / starvation yield, and direct CPU Chip RAM wait-state stalling with Fast RAM immunity.
  - 100% verified across 114 unit and integration tests throughout the workspace.

### Phase 2: Enhanced Chipset (ECS) & Later Models
- **A500 Rev 6A (1 MB Chip):** Fat Agnus 8372A with 1 MB Chip RAM jumper configuration.
- **A500 Plus:** Full ECS chipset (Agnus 8372A 1MB, Denise 8373 with Productivity modes), Kickstart 2.04 (512 KB), onboard battery-backed RTC.
- **Deferred ECS vAmigaTS Test Suite Execution Gate:**
  - Unfilter and execute all vAmigaTS test suites deferred during Phase 1 that require ECS silicon features (`_ecs.raw`, `_plus.raw`, `BPLCON3`, SuperHires, 1 MB / 2 MB Agnus registers, and Productivity scan modes) per the *Test Suite Categorization, Filtering & Deferred Execution Matrix* in Step 2.

### Phase 3: Advanced Graphics Architecture (AGA / A1200)
- **A1200 (AGA):** Motorola 68EC020 (32-bit), 2 MB Chip RAM, Alice, Lisa, 24-bit color palette.
- **Deferred AGA vAmigaTS Test Suite Execution Gate:**
  - Unfilter and execute all vAmigaTS test suites requiring AGA silicon features (`_A1200.raw`, 68EC020 CPU, 24-bit color palette, 8 bitplanes) deferred during Phase 1 & 2.

### Peripheral Extensions (Post-Baseline)
- **4-Player Joystick Adapter:** Parallel port 4-joystick adapter for multiplayer games (e.g. *Super Skidmarks*, *Dynablaster*).
- **Analog Joysticks:** Proportional analog potentiometer sampling via POT0DAT/POT1DAT.
- **Light Pen / Gun:** Video beam position latching via VPOSR/VHPOSR registers and BPLCON0 bit 3 (LPEN).

---
## 2. Core Implementation Strategy (Remaining Milestones)

### Step 1: Developer Studio GUI & Diagnostic Tooling Hardening (Ongoing Companion Track)
- **Continuous Testing & Polish of Developer Studio (Debugger View):**
  - Actively test, harden, and refine the Developer Studio GUI (Debugger View) alongside core hardware emulation workloads.
  - Continuously test all interactive panels under live emulation: Disassembly infinite stream browsing and in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
  - Verify layout stability, symmetric margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`) across diverse window sizes and display resolutions.
  - Test and refine upcoming Save State management (`State` menu, in-memory quick slots 1–5, `F6`/`F9` shortcuts, native file dialogs, and State Manager modal dialog).

### Step 2: vAmigaTS Automated Test Suite Execution Harness & Silicon Verification Gate (Active Focus)
- **Direct-Injection Payload Extraction Architecture (`$000400` Sector 2 Slicing):**
  - **Zero-Floppy Bare-Metal Bootstrap:** Exploit the standard vAmigaTS micro-bootblock layout present across 2,068 out of 2,077 test ADFs (99.6%).
  - **Direct Chip RAM Injection:** 2,068 out of 2,077 test ADFs (99.6%) use the standard micro-bootblock that loads Sector 2 (`$000400`) directly to `$00070000`. By injecting the payload and providing a minimal zero-allocation stub ExecBase/GfxBase table, tests run in milliseconds without Kickstart bootstrap delays.
  - **Machine State & Prefetch Initialization:** Set Supervisor Stack Pointer ($SSP = \$0007FF00$), zero CPU data/address registers, and prime the CPU instruction prefetch pipeline directly at target entry point (`set_pc_and_prime_prefetch(0x070000)`).
- **Verification of Remaining Non-Standard Test ADFs (9 Tests):**
  - **Full Suite Completeness Mandate:** While 2,068 ADFs (99.6%) leverage Sector 2 direct injection, the remaining 9 non-standard test ADFs must also be supported and verified to reach 100% test coverage across all 2,077 test ADFs and 2,815 physical silicon reference captures.
  - **Execution Path:** Execute these remaining tests via floppy MFM track decoding and bootblock streaming (using the existing `crates/floppy` controller and CIA index pulse handling) or specialized custom entry point loaders.
- **Dual Runtime Test Execution Categories:**
  - **Pure Bare-Metal Custom Chip Tests (2,118 test files):** Autonomous test execution touching exclusively custom chip registers ($DFF000–$DFF1FE) and dual MOS 8520 CIAs ($BFE001 / $BFD000) with zero OS, trap, or Kickstart library dependencies.
  - **Scarab Mini-Startup Tests (~874 test files):** Provide a lightweight, zero-allocation stub `ExecBase` jump table at address `$000004` handling standard `graphics.library` cleanup calls (`OpenLibrary`, `LoadView(NULL)`, `WaitTOF`, `CloseLibrary`), or execute against initialized Kickstart 1.3 memory image.
- **Cycle-Exact Output Verification & Golden Reference Differencer:**
  - **Full Frame Buffer Golden Matchers:** Automated headless comparison of `Denise` / `FrameBuilder` rendered output against the 2,815 verified $716 \times 285$ 24-bit RGB (`.raw`, 612,180 bytes) reference frame captures in `ref_src/vAmigaTS`.
  - **Micro-Timing Anomaly Detection:** Any discrepancy in CIA timer underflow, Copper beam wait wake-up, or interrupt assertion manifests immediately as shifted colored raster bars (`COLOR00`) or displaced bitplane/sprite pixels.
  - **Non-Visual Register & State Assertions:** For pure arithmetic, flag, and timing tests, assert register states (`DMACONR`, `INTENAR`, `VPOSR`, `ICR`, CIA counters) upon reaching test completion breakpoints.
- **Targeted Subsystem Verification Sub-Suites (`cargo test -p test_runner --test test_vamiga_*`):**
  - *Suite 2.1: CIA Timers, TOD & ICR Interrupts:* `ref_src/vAmigaTS/CIA/CIA/Timer/` (`timer1`–`timer5`, `cont1`–`cont4`), validating cascaded timers, one-shot reloads, and Level 2 / Level 6 IRQ timing.
  - *Suite 2.2: Agnus Copper Coprocessor Engine:* `ref_src/vAmigaTS/Agnus/Copper/` (`Wait/`, `Skip/`, `coptim/`, `coprace/`, `copvbl/`), verifying 4-CCK instruction cycle timing, beam wake-up latency, and `CDANG` danger mode.
  - *Suite 2.3: Agnus 4-Channel DMA Blitter Engine:* `ref_src/vAmigaTS/Agnus/Blitter/` (`line/`, `fill/`, `timing/`, `bbusy/`, `bltint/`), verifying 256-minterm Boolean ALU, barrel shifts, modulos, and `_BLITINT` generation.
  - *Suite 2.4: Denise Video, Bitplanes & Sprites:* `ref_src/vAmigaTS/Denise/` (`Registers/`, `Modes/`, `DIW/`, `Sprites/`), verifying pixel serialization, palette translation, display window clipping, and sprite multiplexing.
  - *Suite 2.5: Paula Audio & Interrupts:* `ref_src/vAmigaTS/Paula/` (`Audio/`, `Interrupts/basicint/`), verifying PCM sample streaming, period clock division, and Level 1–4 interrupt requests.
  - *Suite 2.6: Agnus Master DMA Contention & CPU Stealing:* `ref_src/vAmigaTS/Agnus/Blitter/bususage`, `cputim`, `Denise/Sprites/spritedma`, verifying cycle-exact CPU wait-state stalling under heavy DMA and Blitter Nasty.
- **Test Suite Categorization, Filtering & Deferred Execution Matrix:**
  - *Chipset Model Scope Filter (Phase 1 OCS Baseline vs Phase 2 ECS / Phase 3 AGA):*
    - The active test execution harness strictly targets the Phase 1 Baseline A500 OCS machine model (Fat Agnus 8371 PAL / 8370 NTSC, OCS Denise 8362, 512 KB Chip RAM).
    - Tests in `ref_src/vAmigaTS` that specifically exercise ECS features (Agnus 8372A 1MB/2MB registers, Denise 8373 Productivity modes, `BPLCON3`, SuperHires) or AGA hardware (68EC020, 24-bit palette, 8 bitplanes) and provide only `_ecs.raw`, `_plus.raw`, or `_A1200.raw` reference captures are formally filtered and deferred to Phase 2 (ECS) and Phase 3 (AGA).
  - *Output Verification Modality Filter (Visual RGB24 Viewport vs Non-Visual Register Assertions):*
    - Tests producing $716 \times 285$ RGB24 frame buffers are matched pixel-for-pixel against verified `.raw` / `_ocs.raw` captures.
    - CIA tests, serial/parallel communication tests, and register-level test cases that only have hardware CRT camera photographs (`.jpeg`) or lack visual frame buffers are filtered into non-visual verification harnesses asserting register and memory states (`DMACONR`, `INTENAR`, `VPOSR`, `ICR`, CIA counters) upon reaching breakpoint milestones.
  - *Bootstrap Modality Filter (Sector 2 Direct Injection vs Floppy MFM Boot):*
    - 2,068 ADFs (99.6%) execute via Sector 2 direct Chip RAM injection with zero-allocation Exec/Gfx stubs.
    - The 9 non-standard ADFs are scheduled for MFM track streaming / custom bootblock loader verification.
  - *OS Library Dependency Filter (Scarab Ministartup vs Full Kickstart ROM):*
    - Tests running under bare-metal or Scarab ministartup execute immediately via our zero-allocation stub jump tables.
    - Tests with dependencies on full Kickstart 1.3 libraries (`dos.library`, `intuition.library`, filesystem) are deferred to the Kickstart 1.3 low-memory overlay bootstrap milestone.

### Step 3: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)
- **Step 3.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector:**
  - Dedicated custom chipset register docks/tabs in Developer Studio (`crates/gui`): Agnus, Denise, Paula, CIAs (A & B), and RTC.
  - Live values formatted in hex/binary with visual change/delta highlighting (electric cyan diffs).
  - Bitfield breakdown widgets and interactive tooltips ("Zero-External-Lookup Principle") for complex control/status registers (e.g. `DMACON`/`DMACONR`, `INTENA`/`INTENAR`, `INTREQ`/`INTREQR`, `BPLCON0`-`BPLCON2`, `ADKCON`/`ADKCONR`, CIA `CRA`/`CRB`).
  - Observability of hardware mutation delays: visualize staged pipeline register latches taking effect across subsequent Color Clock phases ($CCK1 \to CCK2$) rather than instantaneous propagation.
- **Step 3.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer:**
  - Horizontal scanline timeline visualizer displaying the 227 Color Clock (CCK) slots per scanline (PAL) / 226 slots (NTSC).
  - Color-coded channel mapping across fixed and dynamic DMA allocations:
    - Fixed allocations: DRAM refresh (CCK 0..3), Disk DMA (CCK 4), Audio DMA channels 0–3 (CCK 5..8), Sprite DMA pairs 0–7 (CCK 12..27).
    - Dynamic allocations: Bitplane DMA (BPL 1–6) across display window, Blitter DMA (channels A, B, C, D), and available CPU bus slots.
  - Live beam position cursor tracking current horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates.
  - Bus contention & wait-state indicator: visually identify cycles where CPU or Copper are stalled waiting for Chip RAM access (BLTPRI / Blitter Nasty mode).
- **Step 3.3: Copper Coprocessor Inspector & Real-Time Execution Tracker:**
  - Dedicated Copper list disassembler panel decoding instruction streams (`MOVE`, `WAIT`, `SKIP`) directly from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
  - Real-time execution pointer tracking: highlight currently executing Copper instruction, pending wait condition (beam comparison against $VPOS$/$HPOS$ and mask), and CDANG danger mode status.
  - Visual correlation with raster beam: highlight beam position where Copper interrupts or register modifications trigger palette swaps, display window splits, or Blitter dispatches.
- **Step 3.4: Internal Chipset State Machines & Deep Diagnostics:**
  - *Blitter Engine Diagnostics:* Visual representation of active channels (A, B, C, D), 256-minterm truth table visualization ($LF$ code decomposition), shift/mask register stages, Bresenham line-drawing state counters, and Blitter busy/idle flags.
  - *Denise Video & Sprite Pipeline:* Live inspection of bitplane serializers, dual-playfield priority layers, 8 hardware sprite position registers/active states, and hardware collision latches (`CLXDAT`/`CLXCON`).
  - *Paula Multi-Engine Status:* Audio channel frequency, length, volume, BLEP table synthesis status, floppy MFM bit-stream decoding buffers/sync-word detector (`$4489`), and serial UART FIFO/baud counters.
  - *CIA Timers & Port Observability:* Live countdown display of Timers A & B, TOD clock sub-second counters, serial shift register (SDR) status, and I/O port pin states.

### Step 4: Host I/O Peripherals, Audio Playback & Controller Hub
- **Step 4.1: Host Audio Playback & CRT Presentation Shaders:**
  - Audio sink: Ring buffer decoupled from host audio playback (cpal / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
  - GPU post-processing shaders for authentic CRT TV look and feel (scanlines, shadow mask, curvature, phosphor bloom).
- **Step 4.2: Host Input Subsystem, Game Controller Mapping & Port Hub (`crates/keyboard`, `crates/mouse`, `crates/joystick`, `crates/game_ports`, `crates/gui`):**
  - **Host Keyboard to Amiga Matrix & Scancode Mapping:**
    - Full mapping table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes.
    - Amiga-specific qualifiers: Left/Right Amiga keys (mapped to host Windows/Command or Alt), Left/Right Alt, Ctrl, CapsLock, Help, and numeric keypad.
    - Hardware reset combo: `Ctrl + Left Amiga + Right Amiga` assertion wired into `keyboard.reset_line_asserted` triggering machine warm reset (`reset_warm`).
    - **Keyboard-as-Joystick Emulation:** Configurable key bindings (e.g. Numpad `8/4/6/2` + `0/5/Enter`, or `WASD` + `Space`, or Arrow keys + `RCtrl`) mapped to Amiga Port 1 or Port 2 digital joystick switches for players without physical gamepads.
  - **Dual Game Port Hub & Amiga Port Hot-Swapping (`crates/game_ports`):**
    - Independent device slot assignment:
      - *Port 1:* Mouse (default), Digital Joystick, CD32 Pad, Keyboard-Joystick 1, or Disconnected.
      - *Port 2:* Digital Joystick (default), Mouse (for 2-player dual-mouse games like *The Settlers* and *Lemmings*), CD32 Pad, Keyboard-Joystick 2, or Disconnected.
    - Multi-device configurations:
      - Simultaneous dual-mouse mode decoding independent quadrature into `JOY0DAT` and `JOY1DAT`.
      - Simultaneous dual-joystick mode decoding independent directional switches into `JOY0DAT` and `JOY1DAT`.
    - Hardware button routing: Primary Fire / Left Click to CIA-A `PRA` bits 6 & 7 (`/FIR0`, `/FIR1`), Secondary Fire / Right Click to Paula `POTGOR` bits 10 & 14, and Middle Button / Fire 3 to Paula `POTGOR` bits 8 & 12.
    - CD32 7-button shift register protocol serialization over pin 5 clocked by Paula `POTGO`.
  - **Physical Host Mouse Capture & Motion Scaling (`crates/mouse`, `crates/gui`):**
    - Viewport mouse grab/lock mode capturing relative mouse deltas (`dx`, `dy`), hiding the host cursor, with clean toggle/release shortcuts (`Esc` or Middle-Click).
    - Relative motion delta accumulation and scaling into Amiga 8-bit wrap-around quadrature counters ($X, Y \in 0..255$).
    - Configurable sensitivity, acceleration, and axis inversion parameters.
  - **Physical Gamepad & Joystick Ingestion (`crates/joystick`, `crates/gui`):**
    - Host gamepad enumeration and event streaming (via `gilrs` on native desktop and HTML5 Gamepad API in WebAssembly).
    - Mapping analog sticks / D-pads with configurable deadzones to Amiga 4-direction digital switches.
    - Multi-button mapping (Fire 1, Fire 2, CD32 buttons) and dynamic assignment of connected physical controllers to Amiga Port 1 / Port 2.
  - **Interactive Developer Studio Input Configuration UI (`crates/gui`):**
    - Dedicated "Game Ports & Input" tab/dock: visual status of Port 1 and Port 2, connected device dropdowns, real-time input indicators (directional switch arrows and fire button LEDs), and keyboard-joystick toggles.
  - **Dedicated Unit & Integration Tests:**
    - Test suites verifying host scancode translation, keyboard-as-joystick key mapping, dual-mouse and dual-joystick port arbitration, POTGOR button sensing, and headless UI input interaction.

### Step 5: Dedicated Player GUI & Frontend Experience
- **Step 5.1: Hardware Configuration & Kickstart ROM Selector:**
  - Amiga hardware profile selector (Basic A500 512 KB, Classic A500 1 MB [Recommended], Expanded A500 4 MB).
  - Explicit notification and confirmation modal informing the user that changing hardware parameters requires a cold machine reset.
  - Kickstart ROM manager: file picker for Kickstart ROM images (1.2, 1.3, custom ROMs) with automatic checksum validation (CRC32/SHA-256).
- **Step 5.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`):**
  - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
  - Individual drive enable/active toggle checkboxes to mount or disconnect external floppy drives on the fly.
  - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
  - Visual drive activity indicators and floppy motor/stepping audio feedback.
- **Step 5.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels):**
  - Interactive save/load state overlay and slot manager.
  - Visual snapshot cards containing:
    - **Automatic Screen Capture:** Embedded thumbnail screenshot of the active Amiga display captured at the exact moment of saving.
    - **Timestamp:** Formatted creation date and time.
    - **Custom Label:** Optional user-defined state name / description for memorable checkpoints and game phases.
    - **Configuration Integrity Guard:** Verifies matching hardware profiles (RAM sizes, chipset mode) before restoring state to prevent emulator panics or guest crashes.

### Step 6: Real-World Amiga Workloads, Host Cache Profiling & Pipeline Optimization (Post-Boot)
- **Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate:**
  - Execute the 9 non-standard test ADFs using physical floppy MFM track streaming via the `crates/floppy` controller.
  - Unfilter and execute all vAmigaTS tests requiring full Kickstart 1.3 bootstrap (`dos.library`, `intuition.library`, and filesystem calls) deferred from Phase 1 bare-metal execution.
- **End-to-End Bootable ADF Integration Testing (`cargo test -p test_runner --test test_boot_adf`):**
  - Load and execute established Amiga benchmarks and diagnostic suites directly from floppy disk images (e.g. `AmigaTestKit.adf`, `SysInfo.adf`, Dhrystone) on the authentic Kickstart / Amiga chipset stack.
  - Leverage existing address guards, breakpoint traps, and instruction bounds for parameterized termination.
  - Assert functional correctness, numerical determinism, and hardware register states across long-running real-world execution sequences.
  - Measure baseline emulation throughput (effective MIPS, instruction throughput, host CPU cycle cost per emulated CCK).
- **Real-Workload Host Cache Miss & Footprint Impact:**
  - Profile host L1i/L1d cache misses, Last Level Cache (LLC) misses, and branch mispredictions during continuous execution of genuine Amiga software across the 65,536-entry static dispatch table.
  - Determine the empirical performance impact of CPU core memory footprint on real-world, non-repetitive workloads.
- **CPU Core Memory Footprint Audit & Compaction Assessment:**
  - **Memory Footprint Audit (in Kilobytes):** Measure and document the exact memory footprint of the CPU emulator core:
    - Host .rodata footprint: 65,536-entry static dispatch table (sizeof(OpcodeDescriptor) * 65,536), static [MicroStep; N] array slices, and decoding metadata.
    - Host runtime state footprint: Cpu, CpuState, CpuMicroState sizes in bytes, auditing memory layout, alignment, and cache-line compactness.
  - **Footprint Compaction Assessment:** Investigate whether compacting the CPU footprint (e.g., bit-packing OpcodeDescriptor, microcode array deduplication, index packing) yields measurable L1i/L1d miss reductions and throughput gains, or whether the current flat layout already maximizes host branch-predictor and cache throughput.
  - Refactor identified slow handlers using host hardware efficiency principles (direct specialized flattening, branchless bit operations, eliminated redundant register banking, and cross-crate MIR inlining).
  - Strictly enforce architectural constraints: zero custom macros (macro_rules!), zero const-generic function matrices, zero dynamic heap allocations, and zero compromise on code readability.
- **Cross-Architecture & Mobile Performance Projections:**
  - Extrapolate measured desktop throughput (x86_64 / desktop ARM) to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile devices).
  - Model CPU overhead margins to ensure headroom for sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation once custom chipset DMA contention and rendering are integrated.
- **Fidelity & Regression Validation Gate:**
  - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity and passes the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") with zero regressions.

---

## 3. AI Agent Testing & Differential Execution Strategy

To achieve cycle-exact accuracy and debug complex game/demo edge cases, the project leverages automated agent workflows and differential emulation:

### 3.1 Differential Testing Against Reference Emulators
- **Cross-Emulator Execution Harness:**
  - Run target test cases, test ROMs, or problematic games in verified reference emulators ([vAmiga](ref_src/vAmiga) and [WinUAE](ref_src/WinUAE)).
  - Step emulation to a designated frame number or instruction milestone.
  - Dump execution traces, CPU registers, DMA channel allocations, and memory state.
  - Run the same binary on our emulator core and compare state dumps to detect cycle deviations instantly.

### 3.2 Automated vAmigaTS Test Suite Direct-Injection Runner (Active Step 3 Architecture)
- **Direct-Injection Execution Engine:** Fully detailed in [**Step 3: vAmigaTS Automated Test Suite Execution Harness & Silicon Verification Gate**](#step-3-vamigats-automated-test-suite-execution-harness--silicon-verification-gate).
- **Direct Sector 2 Payload Slicing:** Extracts pre-assembled test payloads at offset `$000400` directly into Chip RAM at `$00070000`, executing bare-metal hardware tests with zero dependency on floppy mechanics or Kickstart bootstrap.
- **Headless Frame Differencer:** Renders full frames in `FrameBuilder` and compares pixel buffers directly against the 2,815 reference `.raw` frame dumps ($716 \times 285 \times 3$ RGB).

### 3.3 Visual & Audio Multimodal Validation
- **Full HD ($1920 \times 1080$) Standardization & Responsive Multi-Tier Studio Workbench (`LayoutTier`):**
  - Standardized Full HD ($1920 \times 1080$) as the primary Developer Studio baseline, eliminating empty panel voids and black margins around the CRT display.
  - Implemented responsive multi-tier layout architecture (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`):
    - **Full HD Wide ($\ge 1680\text{px}$):** 4-pane Studio Workbench featuring full-height Disassembly stream, prominent 4:3 Amiga CRT monitor ($512 \times 384$), dedicated Execution Trace Log, new Emulation Engine Status card, and expanded Memory Hex editor (up to 36 rows = 576 bytes).
    - **Standard Desktop & Compact ($< 1680\text{px}$):** Adaptive 3-column layout with vertical scrollbars and wrapped transport controls, ensuring usability down to $1024 \times 600$.
  - Introduced centralized semantic design tokens (`ColorTokens` in `crates/gui/src/theme/tokens.rs`), replacing harsh neon cyan with soft sky blue (`#38BDF8`), electric cyan diffs (`#67E8F9`), and high-contrast dark slate condition code badges.
  - Stabilized register layout with fixed-width `{:>11}` decimals, eliminating column jitter across signed 32-bit values.
  - Guarded by 36 automated tests in `crates/gui` and verified visually across $1920 \times 1080$, $1280 \times 720$, and $1024 \times 600$ via `gui-inspector`.
- **Native `eframe::Storage` Persistence & UI Settings Retention:**
  - Integrated native `eframe::Storage` persistence (`persistence` feature with RON serialization) across desktop and WebAssembly (`localStorage`).
  - Automatically preserves window geometry (position and size), panel splitter widths (`SidePanel` Left Dock and Right Dock), and all `CollapsingHeader` states (open vs closed) via `egui::Memory`.
  - Serializes high-level user preferences (`UserPreferences`: active theme, Developer Studio vs ScreenOnly mode, microcode inspector visibility, and temporal history ring buffer capacity) across sessions under `eframe::APP_KEY`.
  - Enforced strict machine state transience: guest execution state (`DebuggerSession`, CPU registers, RAM contents, execution counter) is never saved to disk and always starts clean on app launch.
  - Added `default-run = "amiga-studio"` to `crates/gui/Cargo.toml`, enabling single-command launch via `cargo run -p gui`.
  - Guarded by 3 automated integration tests in `crates/gui/tests/test_persistence.rs`.
- **Vision-Driven Headless GUI Inspector (`gui-inspector`) & Autonomous Self-Healing (`egui-vision-debugger`):**
  - Built headless offscreen capture harness (`crates/gui/src/bin/gui_inspector.rs`) utilizing `egui_kittest` + `wgpu` strictly isolated under `cfg(not(target_arch = "wasm32"))`.
  - Authored specialized agent skill (`.agents/skills/egui-vision-debugger/`) for automated scenario execution, visual layout audits via `view_file`, and self-healing iterations.
  - Formalized the **Self-Documenting UI Standard ("Zero-External-Lookup Principle")** across `egui-best-practices.md` and Obsidian design specifications: every inspectable register, flag, and memory region provides contextual documentation on hover (`.on_hover_ui`/`.on_hover_text`) using zero-allocation static string slices (`&'static str`).
  - Added dedicated hover inspection presets (`hover_register`, `hover_ccr`, `hover_memory`, `game_mode`, `workbench_theme`) with `tooltip_delay = 0.0` for immediate headless capture.
  - Corrected immediate-mode focus lifecycles and keyboard Enter activation across register and disassembly inline editors.
  - Guarded by 29 automated headless integration tests in `crates/gui/tests/test_interactions.rs`.
- **Interactive Developer Ergonomics, Draggable Splitters & Continuous Memory Browsing:**
  - **Full-Height Disassembly & True Infinite Scroll:** Eliminated all outer and nested `ScrollArea` wrappers around Disassembly; visible instruction row count is computed directly from available height (`(ui.available_height() / 19.0).floor() as usize`); mouse-wheel streams instructions forward/backward across 24-bit memory space with Ctrl (10x) and Shift (5x) acceleration and event consumption; includes integrated 24-bit vertical scrollbar ($000000..=$00FFFFFE) on the right edge; stepping (`F10`, `Shift+F10`, `F11`) or running (`F5`) automatically snaps view back to live $PC$.
  - **Vertical Column Splitter (Full HD Mode):** Inline draggable vertical divider (`ResizeHorizontal` cursor with hover stroke) between Column 2 (Disassembly) and Column 3 (CRT Screen & Trace Log), dynamically adjusting `disasm_pane_width` (default 460px, clamped 280px..=total_w - 380px) and persisted in `UserPreferences`.
  - **CRT Screen / Trace Log Horizontal Splitter (Full HD Mode):** Draggable horizontal divider between top Amiga CRT display/temporal bar and bottom Execution Trace Log, persisted in `crt_pane_height`.
  - **Dynamic Right Dock Vertical Fill & Bottom Tools Docking:** Removed the horizontal splitter in Right Dock; adopted natural `Layout::bottom_up` layout where bottom tools (`Memory Search`, `Breakpoints & Watchpoints`) dock cleanly to the bottom taking their measured height, while `Memory Hex Editor` docks to the top and dynamically expands to occupy 100% of all available vertical space above them (35 rows in Full HD, 8 rows in 720p).
  - **Permanent Fixed Margins Across All Columns:** Enforced explicit symmetric panel frames (`SidePanel::left` 8px/4px, `CentralPanel` 4px/4px, `SidePanel::right` 4px/8px), creating uniform 8px spacing across all dividers and window edges; eliminated dead space in Disassembly so stream pulls flush against the vertical splitter; eliminated 18px CollapsingHeader indent in bottom tools so search and breakpoint cards align flush with Memory Hex Editor.
  - **Memory Hex Selection & Row-Wrapping Keyboard Navigation:** Single-click selects cell without opening input field; double-click or `Enter` enters inline editing; arrow keys navigate across cells with row wrapping (`ArrowLeft` col 0 to col 15 of previous row, `ArrowRight` col 15 to col 0 of next row) and auto-scrolling `base_addr` on boundary overflow; `Escape` deselects cleanly.
  - **Panel Margins & Font Robustness:** Eliminated nested scroll area conflicts and border clipping on right dock; vector-painted resolution-independent circular breakpoint indicators (`circle_filled` / `circle_stroke`) replacing font missing-glyph boxes; styled text buttons (`Save`, `Cancel`) for in-place instruction editing.
  - Guarded by 45 automated headless integration and unit tests across `crates/gui/tests/`.
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

### 3.5 Autonomous Bootstrapping, Documentation Ingestion & Knowledge Pipeline Refinement (Active)

To guarantee that autonomous AI agents can reconstruct and verify emulator subsystems cleanly, the repository bootstrapping and knowledge ingestion pipeline must undergo end-to-end verification and calibration:

- **Verification of Document Bootstrapping (`tools/bootstrap.ps1 -Doc` / Qdrant RAG):**
  - Verify end-to-end documentation ingestion against the local Qdrant vector database (`http://localhost:6333`).
  - Validate SHA-256 incremental hashing cache, chunking fidelity, multi-threaded fast embeddings, and offline sidecar vision descriptions (`<image>.txt`).
  - Ensure zero regressions or stalls during fresh database initialization and incremental reindexing.

- **Iterative Design Documentation Calibration Loop (`Obsidian/Amiga/Design/`):**
  - Establish a closed-loop calibration process for design specifications:
    1. Bootstrap design specifications into the RAG vector collection (`amiga`).
    2. Test semantic retrieval queries against core architectural concepts (e.g. CCK phases, bus contention, wait states, delayed register mutations).
    3. Inspect retrieved chunks, identifying gaps, ambiguity, or missing circuit invariants.
    4. Refine and calibrate the design Markdown files per [`.agents/rules/vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md) (inverted pyramid structure, dual-layer linking).
    5. Re-bootstrap into RAG and verify improved agent comprehension and code generation accuracy.

- **Staged Clean-Room Reconstruction & Autonomous Documentation-to-Code Regeneration Testing:**
  - Test and validate the end-to-end capability of an autonomous AI agent to reconstruct emulator subsystems directly from design documentation and rules.
  - Execute this methodology in incremental stages:
    1. **Targeted Subsystem Source Deletion:** Delete the source code of an isolated subsystem or module (e.g. an auxiliary crate, a peripheral device, or a coprocessor block).
    2. **Autonomous Re-generation:** Instruct the agent to re-synthesize the deleted implementation solely from curated design specifications (`Obsidian/Amiga/Design/`), architectural invariants, and test vectors.
    3. **Parity & Diff Evaluation:** Compare the re-generated source code against the original implementation and verify behavior against test suites.
    4. **Prompt & Documentation Refinement:** If the generated code diverges, misses hardware nuances, or fails tests, enhance the prompts, skill instructions, or design documentation with explicit invariants and edge-case guidance.
  - *Exploratory Methodology:* The exact evaluation framework, metrics, and tooling for this experiment represent uncharted territory (*terra incognita*) and will be formulated and refined iteratively as pilot experiments progress.

- **End-to-End Hardening of `tools/bootstrap.ps1`:**
  - Exhaustively test the complete PowerShell bootstrapper across all flag configurations (`-Test`, `-Graph`, `-Doc`, `-All`).
  - Verify clean-room resilience on fresh environments: archive decompression (`.gz`/`.zip`), directory creation, missing dependency warnings, and non-zero exit code reporting.

---

## 4. Post-Phase 1 Extensions: Reverse Engineering & Extraction

### 4.1 Resource Extractor & Reverse Engineering Assistant
- Extract graphics (bitplanes, sprites, palettes) and audio samples directly from memory buffers.
- Annotate assets, memory addresses, and game phases using LLM assistance.
- Map active assets to the visual 24-bit memory map.
