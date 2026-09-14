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

### Phase 2: Enhanced Chipset (ECS) & Later Models
- **A500 Rev 6A (1 MB Chip):** Fat Agnus 8372A with 1 MB Chip RAM jumper configuration.
- **A500 Plus:** Full ECS chipset (Agnus 8372A 1MB, Denise 8373 with Productivity modes), Kickstart 2.04 (512 KB), onboard battery-backed RTC.
- **A1200 (AGA):** Motorola 68EC020 (32-bit), 2 MB Chip RAM, Alice, Lisa, 24-bit color palette.

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

### Step 2: Custom Chipsets & Machine Integration (Agnus, Denise, Paula, CIAs — Active Focus)
- **Step 2.1: Minimal Machine Main Loop (A500::step_cck) & Subsystem Orchestration (Completed Baseline Scaffold):**
  - Scaffolded all 17 specialized hardware crates across the Cargo workspace in a flat, decoupled structure: `copper`, `blitter`, `dma`, `agnus`, `sprites`, `frame_builder`, `mouse`, `joystick`, `denise`, `audio`, `floppy`, `serial_port`, `paula`, `keyboard`, `parallel_port`, `cia`, and `machine_loop`.
  - Created top-level machine struct (`A500Machine`) owning all primary chips, coprocessors, and peripheral devices as flat fields with monotonic `u64` Color Clock (`cck`) counter and zero circular references.
  - Sibling crates maintain zero dependencies on each other; cycle execution uses parameter-based passing for clean borrow splitting.
  - Implemented multi-level stepping interfaces: `step_cck(cck: u64)`, `step_instruction()`, `step_cycles(n)`, `step_frame()`.
  - Central interrupt priority arbitration pipeline: samples Paula (Levels 1, 3, 4, 5), CIA-A (Level 2), and CIA-B (Level 6), calculates highest unmasked level, and drives `cpu.state.ipl`.
  - 100% verified: clean `cargo check --workspace`, 24 unit tests across new crates, architecture test validation, and `test_dma_cartesian` pass.
- **Step 2.2: Custom Chip Hardware Registers, Propagation Latency & Clock Domains [Completed: 2026-09-13]:**
  - **Hardware Register Mapping & Access Semantics:** Implemented custom chip hardware registers across Agnus ($DFF000–$DFF07E), Denise ($DFF080–$DFF0DE), Paula ($DFF0A0–$DFF0FE), and CIAs ($BFE001 / $BFD000) with strict hardware access permissions: write-only registers (`DMACON`, `INTENA`, `BPLCON0`, open-bus floating return `$FFFF`), read-only status registers (`DMACONR`, `INTENAR`, `VPOSR`), strobes (`COPJMP1/2`, `BLTSIZE`), and clear-on-read registers (`CLXDAT`, `ICR` with non-destructive `peek_register()`).
  - **Electronic Propagation Latency ($K$ CCK Phase Delay Pipeline):** Modeled physical circuit propagation delays where staged writes enter an inline delay pipeline and commit after calibrated Color Clock phases / CCK cycles (Denise colors/BPLCON0: 1 CCK; Paula interrupts: 1 CCK; Paula ADKCON: 2 CCK; Agnus DMACON: 2 CCK; Agnus BPLCON0 mirror: 4 CCK; CIA E-Clock: 5 CCK).
  - **Propagation Modes & Sizing:** Supported `MutationMode::Pipeline` for streaming data (colors, audio samples) and `MutationMode::OverwritePending` for control/strobe registers. Embedded inline fixed-capacity mutation buffers matching write register counts (Agnus: 64, Denise: 64, Paula: 32, CIA: 16) with zero runtime heap allocations and defensive overflow fallback with error logging.
  - **Cross-Chip Chain Reactions & Pin Cascades:** Changes in one register trigger domino effects across companion chips (`BPLCON0` simultaneous broadcast with dual latencies; `DMACON` broadcast from Agnus to Paula audio/disk DMA enables; `CIA-A _OVL` pin transition directly controlling Gary boot overlay on `MemoryBus`; Paula interrupt request evaluation asserting CPU IPL lines 1..6).
  - **CIA E-Clock Frequency Domain Decoupling:** Modeled the distinct, significantly slower clock domain of the dual MOS 8520 CIAs (5 CCK per E-Clock tick), 24-bit TOD atomic read-freeze on `TODHI` and unfreeze on `TODLO`, and `_LED` pin transition.
  - **Machine Loop Weaving & Zero-Cost Router:** Refactored `crates/memory_bus` into pure physical storage (`PhysicalMemory`), eliminating duplicate register buffers and the ~160M copies/sec synchronization loop in favor of a zero-cost stack-allocated `MemoryBus<'a>` router in `crates/machine_loop` decoding Bank `$DF` (Custom Chips), `$BF` (CIAs), and `$DC` (RTC) directly.
  - **Dedicated Unit & Integration Tests:** 100% verified across 7 test suites (`test_mutation`, `test_agnus_registers`, `test_denise_registers`, `test_paula_registers`, `test_cia_registers`, `test_register_propagation`, `test_rtc`).
- **Step 2.3: Subsystem Action Dispatch & Multi-Chip Register Binding Pipeline:**
  - **Semantic Action Method Dispatch:** Bridge low-level register bits and latched state transitions into explicit, strongly typed action methods on subsystem structs (`FloppyDrive::set_motor`, `FloppyDrive::step_pulse`, `FloppyController::set_dsklen`, `Blitter::trigger_blit`, `Blitter::sync_pointers`, `Audio::set_dma_enables`, `Copper::strobe_jump1/2`, `Denise::set_bplcon0/1/2`, `Denise::set_color`, `Denise::set_diw`), replacing raw polling with event-driven hardware dispatch.
  - **Propagation-Aware Action Triggering:** When a delayed register mutation matures after its $K$ CCK phase delay (or immediately upon unqueued writes), the machine loop executes the targeted action method on the exact operational cycle.
  - **Multi-Chip Aggregate Device Control (Floppy Subsystem):** Full physical modeling across CIA-A Port A ($BFE001: `_CHNG`, `_WPROT`, `_TK0`, `_RDY`), CIA-B Port B ($BFD100: shared `_MTR` latching on `_SELx` falling edge, `_STEP` pulse, `_DIR`, `_SIDE`), and Paula (`DSKLEN` 2-write arming sequence, `DSKPTH/L`, `DSKSYNC`, `ADKCON`).
  - **Master Raster Beam Observation:** Decoupled `BeamPosition { hpos, vpos, lof }` progression in Agnus passed into `step_cck(beam)` on `Copper`, `Sprites`, and `FrameBuilder` without circular handles or runtime heap allocations.
  - **Dedicated Integration Tests:** 100% verified in `crates/floppy/tests/test_floppy.rs` and `crates/machine_loop/tests/test_action_dispatch.rs`.
- **Step 2.4: Machine-Wide Save State Serialization & Restoration (Machine Core & Developer Studio Integration) [Active Focus]:**
  - **Comprehensive State Schema (`A500State`):** Implement decoupled, serde-compatible state snapshot structs (`serde::Serialize`, `serde::Deserialize`) across all machine subsystems: CPU (`CpuState`, `CpuMicroState`), MemoryBus (physical RAM buffers, dynamic boot overlay state, bank descriptors), Agnus (`AgnusState`: beam counters, Copper program counter, Blitter registers/channels, DMA mask), Denise (`DeniseState`: bitplanes, sprites, color palette), Paula (`PaulaState`: 4 audio channels, periods, volumes, MFM floppy track stream), and dual CIAs (`CiaState`: timers A/B, TOD, ICR latches).
  - **Zero-Allocation Machine Snapshot API:** Expose public snapshot methods on `A500Machine`: `save_state() -> A500State` and `load_state(&state) -> Result<(), SaveStateError>` with zero dynamic allocations in the regular execution loop.
  - **Self-Contained & Referenced ROM Modes:** Support optional embedded Kickstart ROM byte slices or checksum guards (CRC32/SHA-256) with strict hardware profile compatibility verification (preventing crashes on mismatched RAM configurations).
  - **Deterministic Round-Trip CI Invariants (`test_save_state_roundtrip`):** Author comprehensive automated tests in `crates/test_runner` verifying that taking a state snapshot at cycle $T$, continuing execution, restoring at $T$, and re-stepping $N$ cycles yields bit-for-bit identical register states, memory buffers, and cycle counters.
  - **Developer Studio GUI Integration:** Wire snapshot triggers directly into the Developer Studio GUI (`crates/gui`):
    - Dedicated `State` menu bar items (`Save State to File...`, `Load State from File...`, quick slots 1–5).
    - Global keyboard shortcuts (`F6` quick-save, `F9` quick-load).
    - In-memory quick-slot ring and native OS file dialogs via `rfd`.
    - Instantaneous visual state restoration: live sync of register dock deltas, memory hex views, and CRT screen buffers upon loading state.
- **Step 2.5: Machine-Wide Reset Sequencing (reset_cold & reset_warm):**
  - Physical _RESET line propagation across all chips.
  - Boot overlay engagement (map_kickstart_to_low_memory in MemoryBus).
  - *Cold Reset:* Zero physical RAM buffers ($00), reset chip registers to power-on defaults (DMACON = $0000, INTENA/INTREQ = $0000, CIA latches cleared), initialize CPU SR = $2700, load initial SSP/PC from $000000/$000004 (Kickstart ROM), prime prefetch queue (IR, IRC).
  - *Warm Reset:* Preserve RAM contents intact (ensuring Kickstart memory checksum and resident module discovery pass), re-engage _OVL, assert chip reset lines, reload initial vectors.
  - Hardware keyboard reset line: wire Ctrl-Amiga-Amiga reset trigger line to main machine reset flow.
- **Step 2.6: Agnus DMA Bus Arbiter (Baseline Model & Contention Exposure):**
  - Implement the baseline Agnus horizontal scanline DMA slot schedule (CCK 0..3 DRAM refresh, CCK 4 disk, CCK 5..8 audio, CCK 12..27 sprites, bitplanes, and even/odd slots).
  - CPU and Blitter contention arbitration (BLTPRI Blitter Nasty mode).
  - Direct bus lock exposure: drive bus lock methods (lock_chip_ram / unlock_chip_ram) so the CPU and all custom chips observe bus contention and stall with wait states (BusResult::WaitState), establishing correct bus contention physics even before individual channel internal DSP/rendering logic is fully completed.
- **Step 2.7: Decomposed Subsystem Deep Implementations:**
  - *Agnus:* Copper coprocessor state machine (MOVE, WAIT, SKIP, CDANG danger mode), 4-channel DMA Blitter (256 minterms ALU, barrel shifters, Bresenham line drawer, ascending/descending modes).
  - *Paula Audio Engine with Native BLEP Synthesis:* Precomputed alias-free BLEP tables (blep_tables.rs) across Paula's 4 DMA audio channels (dynamic CIA-A LED filter switching), floppy MFM track controller, serial UART, interrupt multiplexer.
  - *Denise:* Video pixel serializer, bitplanes (1–6), 8 hardware sprites, 32-color palette (RGB444), dual playfield, collision detection registers (CLXDAT, CLXCON).
  - *CIAs (Dual MOS 8520):* Timers A & B, TOD clock, serial shift register (SDR), parallel/control ports, E-clock synchronization.
- **Step 2.8: Host Audio, CRT Shaders & Copper/DMA Logic Analyzer:**
  - Audio sink: Ring buffer decoupled from host audio playback (cpal / Web Audio).
  - GPU post-processing shaders for authentic CRT TV look and feel (scanlines, shadow mask, curvature, phosphor bloom).
  - Copper list visualizer with live beam position cursor and DMA slot logic analyzer timeline.

### Step 3: Dedicated Player GUI & Frontend Experience
- **Step 3.1: Hardware Configuration & Kickstart ROM Selector:**
  - Amiga hardware profile selector (Basic A500 512 KB, Classic A500 1 MB [Recommended], Expanded A500 4 MB).
  - Explicit notification and confirmation modal informing the user that changing hardware parameters requires a cold machine reset.
  - Kickstart ROM manager: file picker for Kickstart ROM images (1.2, 1.3, custom ROMs) with automatic checksum validation (CRC32/SHA-256).
- **Step 3.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`):**
  - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
  - Individual drive enable/active toggle checkboxes to mount or disconnect external floppy drives on the fly.
  - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
  - Visual drive activity indicators and floppy motor/stepping audio feedback.
- **Step 3.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels):**
  - Interactive save/load state overlay and slot manager.
  - Visual snapshot cards containing:
    - **Automatic Screen Capture:** Embedded thumbnail screenshot of the active Amiga display captured at the exact moment of saving.
    - **Timestamp:** Formatted creation date and time.
    - **Custom Label:** Optional user-defined state name / description for memorable checkpoints and game phases.
    - **Configuration Integrity Guard:** Verifies matching hardware profiles (RAM sizes, chipset mode) before restoring state to prevent emulator panics or guest crashes.

### Step 4: Real-World Amiga Workloads, Host Cache Profiling & Pipeline Optimization (Post-Boot)
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

### 3.2 Automated vAmigaTS Test Suite Extraction
- Unpack and catalog the test disks in ref_src/vAmigaTS/.
- Implement headless runners that load each test ADF, execute until test completion, and verify screen buffers against reference PNG frame renders.

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
- **Interactive Developer Ergonomics, Draggable Splitters & Continuous Memory Browsing (Completed):**
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
