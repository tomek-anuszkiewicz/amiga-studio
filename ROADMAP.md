# Amiga 500 Emulator: Development Roadmap

## Contents

- [Step 2: Native Disk Bring-Up and Verification](#step-2)
  - [2.6: vAmigaTS Native Disk-Based Verification & Self-Testing](#task-2-6)
  - [2.7: Lockstep Differential Tracer vs. vAmiga (Contingency)](#task-2-7)
- [Step 3: Developer Studio GUI, Recording and Rewind (Ongoing Companion Track)](#step-3)
  - [3.1: Continuous Testing & Refinement of Developer Studio (Debugger View)](#task-3-1)
  - [3.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability](#task-3-2)
  - [3.3: Watchpoint Execution Stops](#task-3-3)
  - [3.4: Full-Machine Debugger Rewind and Differential State](#task-3-4)
  - [3.5: Deterministic Session Recording and Replay](#task-3-5)
  - [3.6: Checkpoint and Replay Rewind in Game and Debug Views](#task-3-6)
- [Step 4: vAmigaTS Silicon Verification Sub-Suites & Contention Track (After Chipset Bring-Up)](#step-4)
  - [4.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)](#task-4-1)
  - [4.2: Targeted Subsystem Verification Sub-Suites Execution Track (1,468 Active Baseline Tests)](#task-4-2)
  - [4.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)](#task-4-3)
  - [4.4: Display Modes, Color Fidelity and Dynamic Borders](#task-4-4)
- [Step 5: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)](#step-5)
  - [5.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector](#task-5-1)
  - [5.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer](#task-5-2)
  - [5.3: Copper Coprocessor Inspector & Real-Time Execution Tracker](#task-5-3)
  - [5.4: Internal Chipset State Machines & Deep Diagnostics](#task-5-4)
  - [5.5: Debugger Register and Chip State Views: Screen Layout](#task-5-5)
- [Step 6: Host I/O Peripherals, Audio Playback & Controller Hub](#step-6)
  - [6.1: Host Audio Playback & Image Presentation Shaders](#task-6-1)
  - [6.2: Host Input Subsystem, Game Controller Mapping & Port Hub](#task-6-2)
  - [6.3: Audio Reconstruction and Filtering Experiments (Deferred Exploration)](#task-6-3)
- [Step 7: Dedicated Player GUI & Frontend Experience](#step-7)
  - [7.1: Shared Hardware Configuration and Recognized Kickstart ROM Selection](#task-7-1)
  - [7.2: Multi-Drive Floppy Disk Manager (`DF0:` - `DF3:`)](#task-7-2)
  - [7.3: Persistent Save State Catalog with Screenshots and Creation Times](#task-7-3)
  - [7.4: Game View Controls and Layout](#task-7-4)
  - [7.5: Game Profiles and WASM Bundles with Immediate State Resume](#task-7-5)
  - [7.6: Desktop Game Library and Portable Game Packages](#task-7-6)
  - [7.7: Gameplay Video Recording](#task-7-7)
  - [7.8: Full Frontend Control with a Gamepad (Deferred)](#task-7-8)
  - [7.9: Turbo Floppy Mode](#task-7-9)
  - [7.10: Automatic Disk Swapping (Deferred Exploration)](#task-7-10)
  - [7.11: Backward Compatibility for Save States](#task-7-11)
  - [7.12: Linux and macOS Compatibility Check](#task-7-12)
- [Step 8: Real-World Amiga Workloads & Pipeline Optimization (Post-Boot)](#step-8)
  - [8.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate](#task-8-1)
  - [8.2: End-to-End Bootable ADF Integration Testing](#task-8-2)
  - [8.3: CPU Core Memory Footprint Audit & Compaction Assessment](#task-8-3)
  - [8.4: Cross-Architecture & Mobile Performance Projections](#task-8-4)
  - [8.5: Profiler-Guided CPU Usage Reduction (Deferred)](#task-8-5)
  - [8.6: Fidelity & Regression Validation Gate](#task-8-6)
- [Step 9: Documentation-to-Code Regeneration Experiment](#step-9)
  - [9.1: Isolated Documentation-to-Code Regeneration Trial](#task-9-1)
- [Step 10: Later Hardware and Diagnostic Extensions](#step-10)
  - [10.1: ECS and Later A500 Models](#task-10-1)
  - [10.2: AGA, Later CPUs, and Optional Coprocessors](#task-10-2)
  - [10.3: Resource Extractor and Reverse Engineering Assistant](#task-10-3)
- [Step 11: Four-Manual PDF-to-Book JSON Conversion](#step-11)
  - [11.1: Convert Four Source PDFs to Reviewed `book.json` Recipes](#task-11-1)
- [Step 12: Deferred Bus Fidelity, Peripheral Extensions and CPU Interrupt Cases](#step-12)
  - [12.1: Physical Open-Bus Values and Opposite-Direction Accesses](#task-12-1)
  - [12.2: Four-Player Joystick Adapter and Analog Proportional Inputs](#task-12-2)
  - [12.3: Optional Expansion Interrupt Sources and CPU Interrupt Cases](#task-12-3)
  - [12.4: Attached Serial and Parallel Devices and Communication Verification](#task-12-4)
  - [12.5: Light Pen and Light Gun Input](#task-12-5)
  - [12.6: Genlock and External Video Synchronization](#task-12-6)

---

<a id="step-2"></a>

## Step 2: Native Disk Bring-Up and Verification

Complete the native execution path through Kickstart and the floppy drive. Then
run the original vAmigaTS ADF programs and compare their results with the
applicable references. Reuse existing component and machine tests. Add focused
regression tests for specific failures found while bringing up that path.
Light-pen/gun input (12.5), external genlock synchronization (12.6), and
expansion-connector INT2/INT6 sources (12.3) retain their deferred scope.

<a id="native-disk-verification"></a>

<a id="task-2-6"></a>

- **2.6: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** Run the original native vAmigaTS ADF programs through the verified path from disk to firmware and operating system. Use those runs to validate drive control, floppy DMA, CPU coordination, and the operating system interactions the programs require.
  - **Actionable Scope:**
    - Check screen and window state and save a readable frame to confirm that Workbench 1.3 reached the desktop. Use both the state and the saved image for visual acceptance. Investigate display corruption even if DOS or Workbench tasks have started.
    - Start selected disk-based vAmigaTS cases as originally intended and let them perform the original operation under test. Record the source and revision, applicable OCS configuration, ROM and image, observation point, and expected result. Compare what the emulator produces with the applicable reference captures or data.
    - **Audio readiness:** Compare the digital sample sequences for the selected audio cases with independent expectations. Complete basic host playback from 6.1 before or during 2.6, in time to hear the audio when those cases run as well as check it automatically. Preserve the native/WASM frontend audio boundaries. The tests can run before the later presentation shaders are ready.
    - **Host input readiness:** Complete the basic keyboard, mouse, and joystick/gamepad routing from 6.2 before or during 2.6, in time for the tests that require those inputs. Identify the controls each selected test requires. Verify that host key presses and releases, mouse motion and buttons, and joystick directions and fire reach the emulated devices through the frontend and produce guest-visible effects. Preserve native/WASM separation and core ownership. Ordinary input tests can run before the advanced controller features and the deferred peripherals in 12.2. See the [GUI specification](Obsidian/Amiga/Design/Application/GUI.md).
    - **Retire the old CPU examples:** Remove `examples/m68k/` together with the generator in `crates/debugger/tests/test_sample_binaries.rs` and the collection-specific consumers in `crates/debugger/tests/test_instruction_trace.rs`. Review distinct algorithm/debugger coverage and retain useful focused checks without recreating all five demos. Update README launch examples, testing documentation and path-specific verification configuration; verify that tests no longer recreate the directory. Use a verified runnable program for replacement instructions; a new standalone-program catalog is not a prerequisite.
    - For maximum throughput, run headless and let the emulator advance as fast as the host CPU permits, without wall-clock rate limiting.
  - **Verification Gate:**
    - Automated reports identify the cases and configurations actually run, their expected outputs, and the comparison results. The runs must have zero unexpected halts. Identify unsupported cases and unrun coverage explicitly.
    - Audio scenarios require functioning emulated sample generation, digital output checks and a verified host playback path.
    - For scenarios that depend on input, verify the host-to-guest keyboard, mouse, and joystick paths, including releases and returns to neutral. Follow the required inputs beyond UI event capture and confirm that the guest receives and reacts to them.
    - Old-example removal leaves no active generator, consumer or runnable instruction pointing to `examples/m68k/`; affected debugger checks and the replacement launch example pass.

<a id="task-2-7"></a>

- **2.7: Lockstep Differential Tracer vs. vAmiga (Contingency)**
  - **Trigger:** Consider this contingency after native program execution, only if native disk verification (2.6), focused regressions, and execution traces fail to isolate a divergence from the reference. Disk bring-up can proceed without it.
  - **Objective:** Build a cycle-exact harness that runs this emulator and vAmiga in lockstep with the same ROM/ADF. Capture the full machine state of each emulator at every CCK boundary. At the first divergence, report where it occurred and show the differences in a structured format.
  - **Actionable Scope:**
    - Add a `StepTracer` trait (or feature-gated callback) to `MachineLoop`. At each `step_cck()`, serialize `CpuState` and all chip register banks into a compact binary or JSON snapshot. All state structs already implement `serde::Serialize`, so the JSON snapshot is essentially `serde_json::to_string(&machine.snapshot())`.
    - Use vAmiga's existing headless state-dump mode (`ref_src/vAmiga`) to produce equivalent snapshots at the same CCK boundaries.
    - Have the harness (`tools/verification/lockstep_diff.py`) load both snapshot streams and compare them one CCK at a time. At the first CCK with a differing field, stop and report `CCK #N | field | expected (vAmiga) | actual (ours)`.
    - Enable the tracer only with the feature flag (`--features tracer`) so it adds zero overhead in production builds.
  - **Verification Gate:**
    - Tracer successfully identifies the CCK and field of a known injected regression (synthetic test).
    - `python tools/verification/verify.py test rust crate test_runner suite test_lockstep_tracer` (smoke test against a trivial ROM loop).

---

<a id="debugger-execution-control"></a>

<a id="step-3"></a>

## Step 3: Developer Studio GUI, Recording and Rewind (Ongoing Companion Track)

<a id="task-3-1"></a>

- **3.1: Continuous Testing & Refinement of Developer Studio (Debugger View)**
  - **Objective:** Test and refine the Developer Studio GUI (`crates/gui`) with the emulator running. Cover all display tiers and interactive workflows, and address the problems those checks expose.
  - **Actionable Scope:**
    - Step the machine live while testing the interactive panels. Browse the infinite Disassembly stream and patch an instruction in place. In Memory Hex, test selection and keyboard navigation that wraps between rows. Edit registers and CCR, follow steps in the Microcode Inspector, and exercise breakpoints and watchpoints.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
    - **Power LED indicator:** Display the emulated power LED in the GUI using the machine's LED state. Keep the indicator current during running, stepping, reset and save-state restoration.
    - Verify existing file-based state operations and in-memory quick slots, including F6/F9, in both views. The persistent visual catalog is owned by 7.3.
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate gui suite test_interactions` (all headless egui interaction tests passing).

<a id="task-3-2"></a>

- **3.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability**
  - **Objective:** Audit and document all implemented keyboard shortcuts in Developer Studio and Game View. Make every shortcut discoverable in the application.
  - **Actionable Scope:**
    - Implement a dedicated in-app Keyboard Shortcuts Reference Modal (`?` / `F1` / Top Menu Help > Shortcuts).
    - Show the corresponding shortcut in a button's hover tooltip where it applies.
    - Surface all shortcuts: View Modes (`F12`/`F2`, `Escape`), Stepping Controls (`F5`/`Space`, `F10`, `Shift + F10`, `F11`), Tooling & Machine Controls (`F8`, `Alt + T`, `Ctrl + R`, `Ctrl + O`), Save States (`F6`, `F9`, `Ctrl + S`, `Ctrl + L`), Display Zoom (`Ctrl + +/=`, `Ctrl + -`, `Ctrl + 0`), and Hex/Disassembly editing navigation.
  - **Verification Gate:**
    - Headless integration test in `crates/gui/tests/` verifying modal dialog trigger and shortcut handling.

<a id="task-3-3"></a>

- **3.3: Watchpoint Execution Stops**
  - The UI and backend already support defining watchpoints and checking whether an access matches. Actual guest memory accesses bypass that check, so they currently run past a matching watchpoint. Route guest accesses through matching so they can pause execution. Keep memory inspection by the debugger neutral.
  - **Gate:** Verify that watchpoints match the intended access types and address ranges and that matching guest accesses stop execution. See [Debugger](Obsidian/Amiga/Design/Application/Debugger.md).

<a id="task-3-4"></a>

- **3.4: Full-Machine Debugger Rewind and Differential State**
  - When the user rewinds through today's detailed debugger history, it restores CPU state and leaves RAM and the other chips in their later state. Extend this history to rewind the full machine deterministically. Record the base and target CCKs, changed subsystem values, and reversible physical memory writes. Apply all changes only at the matching base CCK, and reverse all changes only at the matching target CCK. The differential `A500DiffState` remains a design proposal until it is reviewed and implemented. The complementary checkpoint and replay mode for both views is in 3.6.
  - **Format proposal for review:** `DIFF` magic (4 B), base CCK (8 B), delta CCK (2 B), subsystem flags (1 B), variable subsystem payloads, memory count (2 B), then entries `[address: u24, new: u16, old: u16]` (7 B each). The illustrative 22 B/no-write and 29 B/one-write footprints depend on which fields change. The format is still a proposal and has no saved-data migration.
  - **Gate:** Review the format before implementation and prove all-state restoration plus deterministic full-machine forward/backward continuation. See [Save states](Obsidian/Amiga/Design/Application/SaveState.md).

<a id="task-3-5"></a>

- **3.5: Deterministic Session Recording and Replay**
  - Record a playable session in Game and Debug views from an identified initial machine state, hardware configuration, ROM and disk contents. Capture keyboard press/release, mouse movement/buttons and joystick/gamepad input as delivered emulated-device events, together with disk insert/eject/swap actions, resets, state loads and other machine-affecting controls.
  - In the recording, keep events in order and mark the emulated CCK boundary at which each one reaches the machine. In replay, deliver them at those boundaries through the same production input and control paths. Define how to record or reproduce external inputs such as host-supplied RTC updates. Current snapshots omit the live RTC. Keep event capture, asset loading, and storage outside the core stepping loop.
  - Keep initial media contents, subsequent swaps, and any future guest writes reproducible. A snapshot embeds the raw tracks currently inserted in the machine. Retain subsequent media events and their contents in the recording so replay can reproduce them. Use one session and event model for standalone replay and checkpoint rewind.
  - **Gate:** On native and WASM, replay a recorded session from its declared starting point and compare machine progress, relevant state and outputs at selected checkpoints. Include held-key releases, mouse actions and disk swaps. Distinguish executable input replay from video capture in 7.7.

<a id="task-3-6"></a>

- **3.6: Checkpoint and Replay Rewind in Game and Debug Views**
  - Add a second history mode using periodic machine checkpoints, including differential checkpoints, and the event recording from 3.5. It is available in both views on native and WASM and supports returning to an earlier frame to retry gameplay.
  - Choose the checkpoint interval and retained history budget during implementation. One, two, or five emulated seconds illustrate possible intervals. Keep enough baseline and differential checkpoints to reconstruct machine state, required media state, and event positions.
  - When the user seeks to a video frame or field, restore the nearest checkpoint at or before it. Replay the recorded events through the production machine as fast as practical until it reaches the exact target boundary. Count actual PAL/NTSC video boundaries, including interlaced fields. A screenshot can show the target image. Resuming play requires the restored machine state. Before implementation, decide whether resuming from an earlier point replaces or retains the history after that point.
  - Keep seeking responsive in the frontend and allow continued play, quick save/load, and view switching as usual. Review how this mode interacts with the detailed debugger history in 3.4 and persistent manual saves in 7.3. Automatic rewind checkpoints and manual saves have separate lifetimes.
  - **Gate:** Seek to targets both on and between checkpoints and compare with uninterrupted execution. Verify continued play, disk changes and guest media writes, input releases, baseline/delta reconstruction and native/WASM behavior. Measure the memory/storage cost and seek delay before choosing defaults.

---

<a id="external-hardware-verification"></a>

<a id="step-4"></a>

## Step 4: vAmigaTS Silicon Verification Sub-Suites & Contention Track (After Chipset Bring-Up)

<a id="task-4-1"></a>

- **4.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)**
  - **Objective:** Categorize tests that require hardware beyond the baseline A500 OCS configuration and strictly enforce those categories when selecting tests to run.
  - **Actionable Scope:**
    - Maintain and audit formal gate categories: FPU Coprocessors (206 tests deferred to the scope decision in Step 10.2), ECS & AGA Silicon (112 tests deferred to Steps 10.1 and 10.2), 68010 CPU Architecture (91 tests deferred to Step 10.2), AmigaOS Floppy MFM Bootblock (7 tests deferred to Step 8.1), and Non-Visual Register Assertions (193 tests deferred to Step 4.2.6).
    - The previous runner was removed. A future entry point for capture-based tests must classify tests as supported or deferred before it runs the baseline OCS suites.
  - **Verification Gate:**
    - Use the future capture entry point to demonstrate supported and deferred selection before running its baseline. The current Cargo suites do not provide this gate.

<a id="task-4-2"></a>

- **4.2: Targeted Subsystem Verification Sub-Suites Execution Track (1,468 Active Baseline Tests)**
  - **Objective:** Run and pass the 1,468 active baseline vAmigaTS tests in strict order, starting with the shared hardware behavior that the later tests depend on (Layer 0 $\to$ Layer 4). Re-establish the reporting format when this verification track resumes.
  - **Actionable Scope:**
    - **Sub-Suite 4.2.1: Agnus Master DMA Contention & Bus Arbitration (Layer 0 Substrate, 225 tests):** `Agnus/` (`DMACON/`, `BplDma/`, `DIW/`, `DDF/`, `bususage/`), verifying Color Clock phase allocation (odd/even), refresh cycles, CPU wait-state generation, and Blitter Nasty (`BLTPRI`).
    - **Sub-Suite 4.2.2: Agnus Blitter & Copper Coprocessor Engines (Layer 1 Coprocessors, 364 tests):** `Agnus/Blitter/` (250 tests: `line/`, `fill/`, `sblit/`, `timing/`, `bbusy/`, `bltint/`) and `Agnus/Copper/` (114 tests: `Wait/`, `Skip/`, `coptim/`, `coprace/`, `copvbl/`), verifying autonomous bus master operations, 256-minterm ALU, barrel shifts, modulos, and Copper 4-CCK timing.
    - **Sub-Suite 4.2.3: Denise Video, Bitplanes & Sprites (Layer 2 Video Serializer, 210 tests):** `Denise/` (`Registers/`, `Modes/`, `DIW/`, `Sprites/`), verifying pixel serialization, palette translation, display window clipping, and sprite multiplexing.
    - **Sub-Suite 4.2.4: Paula Audio & Floppy Subsystem (Layer 3 Peripherals, 107 tests):** `Paula/` (`Audio/`, `Interrupts/basicint/`), verifying PCM sample streaming, period clock division, and Level 1–4 interrupt requests.
    - **Sub-Suite 4.2.5: Complex CIA-A / CIA-B & Timers (Layer 3 Peripherals):** Timers A & B, TOD clock 50/60 Hz synchronization, serial shift register (SDR), and port handshake lines.
    - **Sub-Suite 4.2.6: Mainboard, Memory & Peripheral Assertions (Layer 4 System, 58 visual + 193 non-visual tests):** `Mainboard/`, `Memory/`, `Misc/`, verifying address decoding, port registers, and RAM expansion configurations.
    - **Sub-Suite 4.2.7: M68000 CPU Silicon Pipeline (Layer 4 System Integration, 503 tests):** `CPU/` (ALU, bitwise, shifts, exceptions, traps, IPL autovectors).
  - **Verification Gate:**
    - Restore the entry point for running capture-based tests. Compare the results for each selected vAmigaTS category with its hardware reference captures.

<a id="task-4-3"></a>

- **4.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)**
  - **Objective:** Replace repeated DMA slot priority evaluation at each CCK with a straightforward lookup table of precomputed scanline slots (`[DmaSlot; 227]`).
  - **Actionable Scope:**
    - Keep the optimization entirely within the Agnus DMA scheduler (`crates/agnus/src/dma.rs`).
    - Keep the code simple and readable. Preserve all timing steps and avoid complex asynchronous event wheels.
    - Once a representative whole-machine workload can run, measure its throughput before and after the optimization. Use fresh measurements from both versions. A CPU-only benchmark leaves Agnus scheduling unmeasured.
  - **Verification Gate:**
    - Zero regression across all active baseline OCS vAmigaTS suites.
    - Run DMA, Agnus, and production memory_bus regression tests, including contention and arbitration of pending CPU requests. CPU-only waiting tests cannot validate scheduler changes on their own.

<a id="task-4-4"></a>

- **4.4: Display Modes, Color Fidelity and Dynamic Borders**
  - When separately scheduled, extend the display coverage from 2.6 and 4.2.3 with focused test programs and representative games once their native execution prerequisites are available.
  - **Resolution changes:** Test low/high resolution switches between frames, within a frame and within a scanline, including mixed-resolution images. Verify pixel placement, proportions and frontend scaling across transitions.
  - **Interlace:** Test interlaced output, alternating field alignment, field presentation and transitions between interlaced and non-interlaced modes.
  - **Color modes and accuracy:** Test supported OCS palette/bitplane configurations, dual playfield, Extra Half-Brite and HAM6, including palette and mode changes during rendering. Verify both generated color values and displayed colors against WinUAE and a physical Amiga, recording reference settings and hardware capture conditions.
  - **Borders and changing display area:** Test overscan and games that change the size or position of the visible area during a frame or between frames. Determine how to retain or crop the border and how cropping and scaling should react to those changes. Check for lost visible content, unwanted image movement and stale pixels.
  - **Gate:** Record reproducible comparisons of static images and transition sequences with both WinUAE and a physical Amiga. Identify which results show agreement between emulators and which come from hardware. Use those cases to agree on how to handle borders and cropping, then verify that behavior in the framebuffer and frontend.

---

<a id="step-5"></a>

## Step 5: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)

<a id="task-5-1"></a>

- **5.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector**
  - **Objective:** Give Developer Studio (`crates/gui`) dedicated docks or tabs for inspecting registers. Highlight changes as they happen and show when a delayed register write takes effect.
  - **Actionable Scope:**
    - Dedicated docks for Agnus, Denise, Paula, CIAs (A & B), and RTC.
    - Format values in hex/binary with electric cyan change/delta highlighting.
    - Bitfield breakdown widgets and interactive tooltips for control/status registers (`DMACON`, `INTENA`, `INTREQ`, `BPLCON0`-`BPLCON2`, `ADKCON`, `CRA`/`CRB`).
    - Show when each staged register latch takes effect across the Color Clock phases ($CCK1 \to CCK2$).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying register dock rendering and delta tracking.

<a id="task-5-2"></a>

- **5.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer**
  - **Objective:** Show a horizontal timeline of bus allocation across a scanline in PAL and NTSC. Use the line lengths and beam progression defined by [Agnus](Obsidian/Amiga/Design/Agnus.md#motherboard-transport-boundary).
  - **Actionable Scope:**
    - Fixed allocations: DRAM refresh, Disk DMA, Audio DMA 0–3 and Sprite pairs 0–7, using the slot coordinates, eligibility and timing conventions in the [DMA specification](Obsidian/Amiga/Design/DMA.md#arbitration-and-complete-exchange-requirements).
    - Dynamic allocations: Bitplane DMA (BPL 1–6), Blitter DMA (channels A, B, C, D), and CPU bus slots.
    - Use a live cursor to track the horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates of the beam.
    - Mark bus contention and wait states by highlighting cycles where the CPU or Copper stalls (BLTPRI / Blitter Nasty).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying timeline visualizer rendering.

<a id="task-5-3"></a>

- **5.3: Copper Coprocessor Inspector & Real-Time Execution Tracker**
  - **Objective:** Add a dedicated panel that reads and disassembles Copper instruction lists directly from Chip RAM. Show where the Copper is in the list as it executes.
  - **Actionable Scope:**
    - Decode instruction streams (`MOVE`, `WAIT`, `SKIP`) from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
    - Track the execution pointer live. Highlight the current Copper instruction, any pending wait condition ($VPOS$/$HPOS$ comparison), and CDANG danger mode status.
    - Show the beam position where Copper interrupts or register changes trigger palette swaps or start Blitter operations.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying Copper disassembler panel.

<a id="task-5-4"></a>

- **5.4: Internal Chipset State Machines & Deep Diagnostics**
  - **Objective:** Add panels in `crates/gui` that show the chips' internal state machines as they run.
  - **Actionable Scope:**
    - *Blitter Engine Diagnostics:* Visual representation of active channels (A, B, C, D), 256-minterm truth table visualization ($LF$ code decomposition), shift/mask register stages, and Bresenham line-drawing state counters.
    - *Denise Video & Sprite Pipeline:* Live inspection of bitplane serializers, dual-playfield priority layers, 8 hardware sprite registers, and collision latches (`CLXDAT`/`CLXCON`).
    - *Paula Multi-Engine Status:* Audio channel frequency, length, volume, BLEP table synthesis status, floppy MFM bit-stream buffers/sync-word detector (`$4489`), and serial UART FIFO.
    - *CIA Timers & Port Observability:* Live countdown of Timers A & B, TOD clock sub-second counters, serial shift register (SDR) status, and I/O port pin states.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying state machine diagnostic views.

<a id="task-5-5"></a>

- **5.5: Debugger Register and Chip State Views: Screen Layout**
  - **Planned scope:** Bring the register and internal-state inspectors from 5.1–5.4 into the debugger GUI so their values and activity help explain how the emulated hardware operates. Include custom-chip and CIA-A/CIA-B registers; consider other register groups when planning the views.
  - Include views for the internal state of the Blitter, Agnus and its DMA scheduling, and Denise rendering and sprites. During layout design, decide which other chip states would be useful to show.
  - Work out how these views fit alongside the existing debugger tools. Consider tabs or grouped panels to keep the information readable within the available space. Decide the final arrangement during design.
  - **Gate when implemented:** Visually review register and chip-state views at representative window sizes, including switching between groups and interpreting hardware activity. This item records future work only.

---

<a id="step-6"></a>

## Step 6: Host I/O Peripherals, Audio Playback & Controller Hub

<a id="task-6-1"></a>

- **6.1: Host Audio Playback & Image Presentation Shaders**
  - **Objective:** Make real-time audio playback work on the host, evaluate filters that enhance the displayed image, and support presentation that resembles a CRT display.
  - **Execution order:** Complete basic audio playback before or during 2.6, in time for its audio scenarios. Basic sound is part of bringing up the external suite. Later playback refinements and presentation shaders stay in this track.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - Evaluate optional GPU shaders for pixel-art scaling and edge smoothing before selecting an algorithm. Compare image quality and rendering cost on native and WASM. Scale the image uniformly and preserve the source framebuffer samples.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/paula/tests/`.
    - Headless shader initialization and rendering verification.
    - Visual comparison with unfiltered output, including text readability, mixed-resolution graphics, and motion, plus rendering cost measurements on native and WASM.

<a id="task-6-2"></a>

- **6.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Translate host keyboard input comprehensively, capture the physical mouse, receive gamepad input, and let users switch the devices assigned to the two Amiga game ports (`crates/keyboard`, `crates/game_ports/src/mouse.rs`, `crates/game_ports/src/joystick.rs`, `crates/game_ports`, `crates/gui`).
  - **Execution order:** Complete basic keyboard, mouse, and joystick/gamepad routing before or during 2.6, in time for the scenarios that need input. The basic host-to-guest path is part of bringing up the external suite. Extended controller protocols, mapping options, and input UI refinements stay in this track.
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Reusable Mappings and Named Configurations:** Allow users to create, name, save and edit any number of host-to-Amiga keyboard mappings (initially configurable special keys), keyboard-as-joystick mappings, and gamepad-to-joystick mappings. Compose these into the shared named emulator configurations in 7.1, together with hardware and game-port assignments. Each game selects one configuration; its disks and saves remain separate game assets in 7.5. Include configurable frontend keyboard shortcuts and define their priority against guest keys and keyboard-as-joystick input.
    - **Gamepad Button Assignments:** Let users choose which physical button produces joystick fire and assign other buttons to frontend actions such as pause/resume, disk selection/swapping, quick save/load and opening the save/load catalog. Reuse the controls from 7.2/7.3/7.4 as they become available. Define how bindings consume or share an event between guest input and frontend actions; keep full gamepad-only interface navigation in 7.8.
    - **Keyboard Combination Fidelity (basic host adapter accepted; hardware fidelity remains open):** Define how host key combinations produce the same guest-visible effects as an A500 keyboard. Establish the original matrix/controller behavior from documentation and hardware measurements, including simultaneous-key limits, blocked keys and phantom key detection (ghosting), then compare it with host keyboard rollover limits and native/browser event delivery. Decide how to reproduce Amiga behavior, preserve held modifiers and press/release sequences, and handle conflicts with frontend or system shortcuts. Explicitly address combinations the host never reports: document the limits and decide on alternative mappings or other workarounds without assuming missing key events can be recovered. Record the agreed behavior in the keyboard and GUI specifications before implementation.
    - **Dual Game Port Hub:** For each Amiga port, first choose the emulated device (Mouse, Joystick, or Disconnected). Then choose its host source. A mouse uses the system mouse or touchpad. A joystick uses a keyboard mapping or a selected gamepad. To support two independent joysticks, route events to the assigned port. A lookup that stops at the first matching device cannot distinguish the two joysticks. Keep hardware button routing through CIA-A `PRA` and Paula `POTGOR`. Separate physical mice, independent dual-mouse sources, and the CD32 7-button protocol remain later extensions.
    - **Physical Host Mouse Capture:** Initially use one system mouse/touchpad source. Implement viewport mouse grab/lock, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), sensitivity tuning and reliable capture release for frontend controls.
    - **Physical Gamepad & Joystick Ingestion:** List gamepads through `gilrs` on native desktop and the HTML5 Gamepad API on WASM. Let users select a device for each port. Provide analog deadzones and button mapping. Handle missing or ambiguous devices and reconnection without silently assigning a device to the wrong player.
    - **Configuration and Input UI:** Edit mappings and port assignments in the shared configuration screen, outside the Game view control panel. Retain the planned Developer Studio "Game Ports & Input" inspection tab/dock with port status and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/` and `crates/game_ports/tests/`.
    - Headless UI input tests in `crates/gui/tests/`.
    - Create and persist several mappings of each supported type, compose and select named configurations, and verify actual guest key/direction/fire delivery and frontend actions on native and WASM. Exercise mouse plus keyboard joystick, mouse plus selected gamepad, and independent dual-joystick sources. Switching configurations with controls held, focus loss and controller disconnect must release obsolete guest inputs without stuck keys or buttons; verify the agreed event-sharing behavior.
    - Verify representative multi-key combinations, modifiers, reset and keyboard-as-joystick controls against the agreed A500 behavior, including different press/release orders and focus loss. Combine deterministic event tests with physical host keyboard checks on native and WASM; compare guest-visible effects with Amiga reference evidence and record host limitations and agreed workarounds. Injected events alone do not verify physical keyboard combination support.

<a id="task-6-3"></a>

- **6.3: Audio Reconstruction and Filtering Experiments (Deferred Exploration)**
  - **Revisit when:** Games can run and users can load music and hear it through the host playback path from 6.1. This optional experiment can end with a decision about whether to pursue an enhancement.
  - Compare faithful Paula output and Amiga analog filtering with per-channel reconstruction adapted to the current playback rate, such as band-limited interpolation. Use sample values and actual playback timing; the original recording sample rate is not required.
  - Distinguish aliasing introduced by host sample-rate conversion from spectral images present in the original DAC output. Explore cleaner reconstruction before channel mixing while retaining the Amiga analog filter model as a separate stage; distinguish sound enhancement from hardware fidelity.
  - **Evaluation:** Compare how the approaches sound, measure their spectra, and measure their processing cost. Use representative game sounds and music, including slow playback, period changes, loops, and modulation. Record audible benefits and changes in sound character, then decide whether an approach merits an optional playback mode. A documented decision to keep the faithful path is a valid outcome.

---

<a id="step-7"></a>

## Step 7: Dedicated Player GUI & Frontend Experience

<a id="task-7-1"></a>

- **7.1: Shared Hardware Configuration and Recognized Kickstart ROM Selection**
  - Provide a global collection of named emulator configurations. Each includes a supported hardware preset and video standard, Kickstart selection, connected drives, devices for both Amiga ports, and the host mappings from 6.2. A game refers to one shared configuration directly, without requiring a per-game copy or an override hierarchy. Save stable references so games can still find a configuration after it is renamed.
  - Edit configurations in a separate application screen reachable from the desktop library/manual setup, with access from the debugger; keep configuration editing out of the Game view panel. Reuse the existing three hardware presets and ROM file loader rather than introducing arbitrary memory combinations. Warn and obtain confirmation before applying hardware or ROM changes that reset/replace the current session.
  - Add a catalog of known ROM images with version and model labels and verified content hashes. Review established catalogs such as WinUAE's. Record the source of entries and the digest algorithm before adopting their recognition data. The catalog identifies known images within the emulator's existing ROM size and hardware model support.
  - Scan a user-selected ROM directory on desktop and offer recognized images found there by version, including a default selection for a requested Kickstart such as 1.3. Define the default when several recognized variants match. Continue to allow direct selection of any structurally valid ROM supported by the emulator; an unknown hash alone must not reject it.
  - **Current WASM entry:** With no bundle supplied, open manual setup so the user can select a configuration and files for Kickstart and DF0: through DF3:, then choose Start. The browser game library is outside this iteration. Save configurations in browser storage. If storage is lost, the user configures the emulator again. If ROM or disk contents are unavailable, require the user to select them again explicitly. Keep manual setup independent of the bundle format and loader. Bundle support belongs to future work in 7.5.
  - **Gate:** Create, edit, persist and reuse several named configurations across games; verify reset/application behavior and missing configuration/device/asset handling. Verify manual WASM startup without a bundle and configuration persistence/recreation. Cover hash recognition independent of filenames, directory selection and version-default resolution, plus direct loading of an unrecognized supported ROM when the recognition catalog is implemented.

<a id="task-7-2"></a>

- **7.2: Multi-Drive Floppy Disk Manager (`DF0:` - `DF3:`)**
  - Give Game view four drive rows for inserting and ejecting disks and showing live activity. Show distinct states for a disconnected external drive and an empty connected drive. Set whether a drive is connected in 7.1. Keep this control panel exclusive to Game view. Write-protection controls and drive-step sound stay in this track. Magnetic writes and ADF write-back require evidence beyond working disk reads.
  - Each drive initially uses a dropdown containing the game's ordered disk list, with labels/numbers and an empty/eject choice. Manual setup supplies the session's list when there is no game profile. Numbered disk-button grids remain a possible later alternative. Preserve actual mounted assignments and their displayed identities across Game/Debug switching and save/load; record machine-affecting changes when 3.5/3.6 are implemented.
  - **Gate:** Verify independent DF0: through DF3: selection, external-drive connection state, disk swaps and actual guest-visible media changes through the production floppy path. Cover native library assets and manually selected browser files. A DF0-only boot or four rendered controls falls short of this gate.

<a id="task-7-3"></a>

- **7.3: Persistent Save State Catalog with Screenshots and Creation Times**
  - Provide a visual save/load catalog in Game and Debug views on both native and WASM. Reuse existing full-machine snapshots, PNG preview encoding, file import/export and quick slots. Snapshot cards show the saved image and creation date/time; saving must not require a user-entered name. Custom labels are optional.
  - Persist the actual host creation time with each player save. Associate saves with the game/profile and the identities of the disks mounted at the time. On desktop, store saves in the game's directory and make them accessible from the library and Game panel. Keep the optional initial state separate from all user saves and quick slots.
  - **WASM sequencing:** Retain explicit save-file import/export and session quick slots for current manual setup. Implement a persistent browser save catalog with export/import alongside future bundle support, including storage-failure reporting and separation from the supplied initial state. Browser configuration persistence in 7.1 does not depend on this later catalog.
  - Make existing quick save/load readily accessible in both views and define their persistence alongside the catalog. Keep manual saves and quick slots distinct from automatic rewind history; filenames or internal identifiers need not become mandatory display names.
  - Restore the saved hardware and execution state with the verified source ROM and ADFs. Apply saved track replacements and preserve the frontend's game and mounted-disk identities. Preserve the active CPU micro-execution path during branches, exceptions and interrupted transfers; opcode-only reconstruction is insufficient. Saved machine state takes precedence over later edits to a named configuration; resolve incompatible host bindings without silently replacing saved hardware.
  - Current pre-release saves use the current schema. Compatibility across released versions is tracked in [7.11](#task-7-11).
  - **Gate:** Save multiple unnamed states, verify correct thumbnails and real creation times, reopen the application/page and restore the selected state. Check both views, quick save/load, required media and identical continued execution on native and WASM; verify that a new user save does not replace the bundled starting state.

<a id="task-7-4"></a>

- **7.4: Game View Controls and Layout**
  - Put the Game controls in a right-hand side panel on wide displays. Show four drive dropdowns (7.2), followed by pause/resume, reset, save/load, and quick access to save previews (7.3). Give users access to Library/Setup and Debug. Keep configuration and ROM editing in the separate setup screen. Switching to Debug preserves the same machine and selections. The side panel is exclusive to Game view.
  - Make the effect of each action clear. Start can use an optional initial state, Continue restores selected user progress, and hardware Reset resets the current session. Give users a reliable way to release captured input and use the side panel. Prototype a compact or collapsible arrangement for narrow native and browser windows.
  - Retain later playback/recording and rewind integration under 3.5/3.6; those features do not block the initial player controls.
  - Preserve uniform scaling and the framebuffer's width-to-height ratio. Extra side space does not establish a fixed 4:3 output requirement; retain the separate border/cropping decision in 4.4.
  - **Gate:** Review the layout visually on wide and narrow native/browser windows; verify disk/state/execution actions, capture and focus transitions, and Game/Debug switching without losing session state. Verify configuration changes through the separate setup flow.

<a id="task-7-5"></a>

- **7.5: Game Profiles and WASM Bundles with Immediate State Resume**
  - Define a game description with a stable identity/title, reference to one named configuration from 7.1, ordered disks and initial drive assignments, optional artwork, optional description and optional starting save state. Use relative game-asset paths; host configuration and device bindings remain separately managed. Manual authoring is sufficient initially; artwork or descriptions may be prepared externally with AI, without an integrated generation feature.
  - **Immediate launch:** When the user selects Start, restore the supplied initial state if one exists. Otherwise, boot normally from the selected configuration and media. When the user selects a save, restore that progress directly. Validate declared states before replacing the current state. If a state is invalid or incompatible, report the problem and require the user to choose normal boot explicitly.
  - **Future WASM bundles:** Later select a bundle format and build tooling to supply the runtime, Kickstart, disks and optional initial state/image for a demonstration. Launch into Game view with those assets; resume the supplied state without repeating guest boot. Current manual WASM setup in 7.1 must work before this feature and does not require its format to be designed.
  - Keep the packaged starting state separate from subsequent user saves. Resolve snapshot/profile configuration and media together; apply the persistent WASM save catalog from 7.3. Review browser startup interaction requirements for audio/input without making the guest repeat its boot sequence.
  - **Initial gate:** Launch a desktop game through both normal boot and its supplied starting state; continue a selected user save, preserve disk identities, and keep the initial state unchanged after saving. **Later bundle gate:** Open a generated bundle in a fresh browser session without manually selecting ROM/disks, verify the first game screen and continued execution, switch a bundled disk, then save, reopen and restore user progress. Also verify normal boot without a starting snapshot.

<a id="task-7-6"></a>

- **7.6: Desktop Game Library and Portable Game Packages**
  - Make the library the normal entry on desktop. The user selects a library root containing one manually prepared directory per game. Show games in tiles or a list, with optional artwork and description, a named configuration selector, Start, and previews of saved progress. Reuse the profiles in 7.5. Provide a route to manual setup for standalone Workbench or diagnostic media.
  - Use directory packages initially: game description/configuration reference, ordered ADF files, optional artwork/description/starting state and user saves. Support assigning a configuration in the application after manually preparing the directory. Keep assets relative to that directory; a missing global configuration or ROM on another installation requires explicit reassignment. No embedded executable package or automatic metadata generation is required.
  - **Gate:** Prepare at least two game directories manually, assign configurations, launch normally/from initial state, save, reopen the application and continue selected progress. Verify independent assets/saves, optional missing artwork, and relocation with relative paths under the same configuration catalog. Exercise actual mouse, keyboard-as-joystick and selected gamepad input, using the early diagnostic checks in 8.2 as soon as a suitable guest runs.

<a id="task-7-7"></a>

- **7.7: Gameplay Video Recording**
  - Plan video capture separately from deterministic session replay. Before implementation, choose how to capture and export video on native and browser frontends, whether to include audio, and how to synchronize included audio. The movie records the presentation. Rewinding the machine or replaying input requires the history covered by the separate replay work.
  - **Gate:** Verify the selected recording/export path produces playable output with correct frame timing and any selected audio, and assess its effect on gameplay responsiveness.

<a id="task-7-8"></a>

- **7.8: Full Frontend Control with a Gamepad (Deferred)**
  - After the named configurations and input mappings in 6.2/7.1 and the player controls in this stage are available, provide an optional gamepad-only mode for the entire frontend, without requiring a host keyboard or mouse. Cover menu navigation, configuration, game/disk selection, save/load catalogs and session controls; plan any text-entry interaction needed by those flows.
  - Reuse configurable frontend button assignments from 6.2 and define transitions between guest control and interface navigation, including focus and return to play.
  - **Gate:** On native and WASM, complete the supported frontend flows and return to gameplay using only a physical gamepad, with no unintended guest input or stuck controls.

<a id="task-7-9"></a>

- **7.9: Turbo Floppy Mode**
  - After verifying the normal floppy path, provide an optional mode that shortens disk loading. Before implementation, choose how to accelerate loading and assess whether the mechanism is compatible with guest timing. Keep normal drive timing available for accurate emulation and verification.
  - Expose turbo selection through the shared configuration/player controls. Distinguish faster disk loading from general emulation speed controls.
  - **Gate:** On native and WASM, compare loading time and successful boot/game continuation with normal mode on representative disks; verify mode switching and document compatibility limits.

<a id="task-7-10"></a>

- **7.10: Automatic Disk Swapping (Deferred Exploration)**
  - Explore an optional system that recognizes a game's request for another disk and selects it from the ordered media list in 7.2/7.5. Keep this distant work separate from manual disk controls.
  - Evaluate screen OCR for disk-request text and numbers (for example, disks 1-4), and consider game-specific memory signatures. The choice remains open and both approaches need evidence of reliability. Recognizing a number on its own is insufficient to identify a disk request. Determine how to match a request to the correct image and drive, and how to handle uncertain or repeated requests while keeping manual control available.
  - **Exploration gate:** Demonstrate recognition and correct swaps on selected multi-disk games, including unrelated screen numbers, ambiguous prompts and repeated requests; record supported cases, false detections and limitations before deciding whether to implement broader support.

<a id="task-7-11"></a>

- **7.11: Backward Compatibility for Save States**
  - Every new emulator version must load valid save states created by any earlier version and resume the saved machine state, even when the file format or internal state representation changes. Users can open their original save files directly in the current version.
  - **Gate:** Keep representative saves from every historical file layout, including distinct layouts that used the same version number. Verify that the current emulator loads each file and correctly resumes execution.

<a id="task-7-12"></a>

- **7.12: Linux and macOS Compatibility Check**
  - Check whether the native emulator builds, launches, and runs a representative Amiga session on Linux and macOS. Record the tested configurations and any platform-specific issues.

---

<a id="step-8"></a>

## Step 8: Real-World Amiga Workloads & Pipeline Optimization (Post-Boot)

<a id="task-8-1"></a>

- **8.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate**
  - **Objective:** Verify physical floppy track MFM streaming and full Kickstart 1.3 bootstrap test suites.
  - **Actionable Scope:**
    - Execute the 9 non-standard test ADFs using physical floppy MFM track streaming via `crates/floppy`.
    - Unfilter and pass all vAmigaTS tests requiring full Kickstart 1.3 bootstrap (`dos.library`, `intuition.library`, and filesystem calls).
  - **Verification Gate:**
    - vAmigaTS test runner passing all MFM and OS-level tests.

<a id="task-8-2"></a>

- **8.2: End-to-End Bootable ADF Integration Testing**
  - **Objective:** Boot and execute genuine Amiga benchmarks and diagnostic suites directly from floppy disk images (`python tools/verification/verify.py test rust crate test_runner suite test_boot_adf`).
  - **Input diagnostic trigger:** Run the interactive hardware-diagnostic checks below as soon as Workbench boots and basic host input from 6.2 is available. These checks can run before the rest of Step 8 and before advanced input mappings are ready.
  - **Actionable Scope:**
    - Load and execute `AmigaTestKit.adf`, `SysInfo.adf`, and Dhrystone on the authentic Kickstart / Amiga chipset stack.
    - Use suitable Amiga hardware-diagnostic programs, including the existing AmigaTestKit candidate, to exercise the whole path from physical host input to the guest. Cover keyboard keys, modifiers, and press/release combinations. Exercise mouse motion and buttons, joystick directions, diagonals, and fire, and routing from gamepad to joystick. Check selected mappings as they become available. Use the diagnostic tests each program supports for these input checks and record any gaps separately.
    - Use address guards, breakpoint traps, and instruction bounds as parameters for when execution stops.
    - Assert functional correctness and deterministic numerical results. Measure baseline throughput (MIPS, host CPU cycle cost per emulated CCK).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate test_runner suite test_boot_adf` passing cleanly.
    - Record the diagnostic program and version, ROM and image, machine configuration, host platform, physical devices, selected mappings, exercised controls, and results observed by the guest. Exercise delivery on native and WASM and report unrun cases explicitly. Use physical controls to verify interactive delivery from the host to the guest. A successful Workbench boot shows that Workbench starts, and synthetic events exercise only the part of the input path after injection.

<a id="task-8-3"></a>

- **8.3: CPU Core Memory Footprint Audit & Compaction Assessment**
  - **Objective:** Audit the exact memory footprint of the CPU core and assess compaction strategies.
  - **Actionable Scope:**
    - Audit host `.rodata` footprint (sizeof(OpcodeDescriptor) * 65,536, microcode arrays) and runtime state footprint (`Cpu`, `CpuState`, `CpuMicroState`).
    - Compare throughput for bit-packing, deduplication, and the flat layout on representative workloads.
    - Refactor slow handlers using host efficiency principles without custom macros or const-generics.
  - **Verification Gate:**
    - Memory footprint audit report and SingleStepTests passing with zero regressions.

<a id="task-8-4"></a>

- **8.4: Cross-Architecture & Mobile Performance Projections**
  - **Objective:** Use measured throughput to project performance on mobile and other constrained targets (e.g. mobile WebAssembly, ARM mobile).
  - **Actionable Scope:**
    - Model the available CPU headroom for sustaining cycle-exact emulation at 50 Hz (PAL) / 60 Hz (NTSC), including full chipset contention and rendering.
  - **Verification Gate:**
    - Documented performance analysis projections in `Obsidian/Amiga/Design/`.

<a id="task-8-5"></a>

- **8.5: Profiler-Guided CPU Usage Reduction (Deferred)**
  - **Objective:** Lower host CPU usage by reducing unnecessary work in measured emulator, host runtime and frontend hot paths, while preserving machine behavior and CCK timing.
  - Use representative whole-machine profiles to find critical code and inspect it manually, or use an agent and profiler interactively. For the interactive approach, profile, select a hotspot, simplify or optimize it, run the relevant correctness tests, measure again, and repeat.
  - **Verification Gate:** Compare the changed version with the original baseline under the same workloads and configurations. Compare CPU time at the same emulated progress and playback speed, and compare uncapped throughput. Cover native desktop and WASM. Retain measured improvements that preserve correctness and timing.

<a id="task-8-6"></a>

- **8.6: Fidelity & Regression Validation Gate**
  - **Objective:** Final regression certification across all optimization and refactoring phases.
  - **Actionable Scope:**
    - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity.
    - Pass the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") and full workspace unit/integration test suites.
  - **Verification Gate:**
    - `python tools/verification/verify.py test runner singlestep full`
    - `cargo test --workspace`

---

<a id="step-9"></a>

## Step 9: Documentation-to-Code Regeneration Experiment

<a id="task-9-1"></a>

- **9.1: Isolated Documentation-to-Code Regeneration Trial**
  - In an isolated checkout, remove one selected subsystem and regenerate it from curated specifications, rules, and test vectors. Compare the regenerated behavior and source with the preserved original. Use any failures to refine the documentation and instructions.
  - **Gate:** The regenerated subsystem passes its unit and integration suites without regressions.

<a id="step-10"></a>

## Step 10: Later Hardware and Diagnostic Extensions

<a id="task-10-1"></a>

- **10.1: ECS and Later A500 Models**
  - Add A500 Rev 6A 1 MB Chip RAM with Agnus 8372A, then A500 Plus with ECS Agnus/Denise, Productivity modes, Kickstart 2.04, and battery-backed RTC.
  - **Gate:** Enable and verify the ECS suites deferred by Step 4.1, including `_ecs.raw`, `_plus.raw`, SuperHires, and expanded Agnus registers.

<a id="task-10-2"></a>

- **10.2: AGA, Later CPUs, and Optional Coprocessors**
  - Add A1200 support with 68EC020, 2 MB Chip RAM, Alice/Lisa, eight bitplanes, and a 24-bit palette. Treat 68010 and FPU support as separate scope decisions tied to their deferred suites.
  - **Gate:** Enable and verify the relevant later-CPU, AGA, and selected coprocessor suites from Step 4.1 when their hardware prerequisites exist.

<a id="task-10-3"></a>

- **10.3: Resource Extractor and Reverse Engineering Assistant**
  - Extract bitplanes, sprites, palettes, and audio from guest memory. Annotate assets, addresses, and game phases, with optional LLM assistance, and map them in the 24-bit address space.
  - **Gate:** Asset extraction unit tests in `crates/debugger/tests/` verify the output and confirm which source addresses it came from.

<a id="step-11"></a>

## Step 11: Four-Manual PDF-to-Book JSON Conversion

<a id="task-11-1"></a>

- **11.1: Convert Four Source PDFs to Reviewed `book.json` Recipes**
  - Prepare the A500/A2000 Technical Reference Manual, Amiga Hardware Reference Manual (second edition), M68000 Programmer's Reference Manual, and M68000 User's Manual (Revision 8). Keep a separate source identity and preparation profile for each PDF.
  - Import each prepared PDF into Book-to-Markdown. Consult the source when adjusting chapter organization, source selections, formatting, and graphics. Accept the intended revision to produce each manual's `book.json`. Use the [book authoring workflow](tools/bootstrap/book-to-markdown/README.md#book-authoring-sequence).
  - **Gate:** Review and commit all four accepted `book.json` recipes with their matching full source hashes in `source.json`. Reaching the prepared PDF or an unaccepted Working bundle leaves this gate pending.

<a id="deferred-bus-and-peripherals"></a>

<a id="step-12"></a>

## Step 12: Deferred Bus Fidelity, Peripheral Extensions and CPU Interrupt Cases

<a id="task-12-1"></a>

- **12.1: Physical Open-Bus Values and Opposite-Direction Accesses**
  - The current model returns the fixed value `$FF/$FFFF`. It has no retained motherboard data-bus latch or complete model of disturbances from opposite-direction accesses. Investigate whether an undriven read returns the fixed value, retains the previously driven value, or changes toward `$FF/$FFFF` over time. Decay remains a hypothesis that needs evidence for the specific address, phase, bus lane, and board.
  - Investigate dummy, unused, and opposite-direction reads and writes, including their possible disturbances. Gather evidence before proposing a change and keep the current approximation until a change to behavior or the reference is approved.
  - **Missing external Bus Error (Vector 2):** `AddressBus` currently exposes only `Ready` and `WaitState`, so it cannot deliver BERR. Treat this as a low-priority completeness gap, probably unnecessary for the current A500 scope, rather than a prerequisite for chipset reconstruction or testing. Revisit only if a concrete workload or supported hardware requirement needs it. The implemented Address Error on odd word/long addresses (Vector 3) is separate and remains supported. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate:** Record hardware-supported values, timing and access effects, then follow specification approval before implementing and testing a replacement. See the [platform invariants](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access).

<a id="task-12-2"></a>

- **12.2: Four-Player Joystick Adapter and Analog Proportional Inputs**
  - Implement the optional four-player adapter only when separately scheduled, after the higher-priority emulator and debugger work. Keep ordinary two-port mouse/joystick support in its existing stages.
  - Implement analog proportional inputs with potentiometer sampling through POT0DAT/POT1DAT in the same deferred peripheral track.
  - **Gate:** Configuration, device behavior and integration tests verify the adapter and analog inputs against their hardware specifications. See [Configuration](Obsidian/Amiga/Design/Configuration.md).

<a id="optional-cpu-interrupts"></a>

<a id="task-12-3"></a>

- **12.3: Optional Expansion Interrupt Sources and CPU Interrupt Cases**
  - **Expansion INT2/INT6:** When an expansion device is separately scheduled, route its active-low requests from the A500 side expansion connector (86 contacts, INT2 on contact 19 and INT6 on contact 22) to Paula alongside the CIA-A/CIA-B sources. Resolve each shared level so that it stays asserted while any source asserts it. With no expansion attached, keep the expansion inputs released. This connector work is outside current chipset reconstruction.
  - **Level 7 NMI / autovector 31:** The current emulator ignores IPL=7. Standard Paula sources generate levels 1-6, but real A500 expansion hardware can drive the CPU interrupt lines to request level 7. Record this missing external-input support and consider implementing it only when such hardware or a concrete workload is in scope, including the CPU's special level-seven acceptance behavior.
  - **Uninitialized interrupt / vector 15:** The current bus interface has no device-supplied interrupt-vector response. Built-in A500 interrupts use autovectors, so there is no standard onboard source for this case. Consider support only if a selected external device or a broader CPU-interface requirement needs it; distinguish an uninitialized peripheral vector register from an absent handler in RAM.
  - **Spurious interrupt / vector 24:** The current interface cannot report BERR during interrupt acknowledgment. Standard A500 interrupts have no identified normal source for this case. Consider implementation only for a concrete external-device or bus-fault scenario, together with the deferred BERR interface in 12.1.
  - Keep these external-input cases as low-priority completeness gaps, outside the prerequisites for current chipset reconstruction and testing. Record the required hardware/input scope before implementation and preserve existing supported exceptions. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate if scheduled:** Reference-derived tests verify the selected input, interrupt acknowledgment or vector selection, exception frame, transaction/wait behavior and continuation. Do not treat recording these gaps as implemented support.

<a id="serial-parallel-device-support"></a>

<a id="task-12-4"></a>

- **12.4: Attached Serial and Parallel Devices and Communication Verification**
  - The current model assumes the serial and parallel connectors remain disconnected. It has no emulated attached device, connector transport, or host adapter that establishes a working connection. The reconstructed chip logic assumes disconnected inputs. Its component test results leave communication with an attached device unverified.
  - When separately scheduled, choose a deterministic attached device or loopback model and implement the required connector routing. Verify serial transmission, reception, and applicable control lines, as well as parallel data, PC/strobe, ACK, and status. Cover delayed or blocked responses, interrupts, reset, and continued operation after restoring state. Physical host-port adapters require their own explicit scope.
  - **Gate if scheduled:** Guest-driven exchanges through the production machine path produce independently expected data and handshake/status transitions at the selected device. Keep connection and full-exchange support explicitly unverified until these tests pass. See [Paula](Obsidian/Amiga/Design/Paula.md) and [CIA](Obsidian/Amiga/Design/CIA.md).

<a id="task-12-5"></a>

- **12.5: Light Pen and Light Gun Input**
  - When separately scheduled, implement the selected game-port trigger and motherboard LPEN route to Agnus, beam-position latching and guest-visible readback. Keep this extension outside the current chipset reconstruction scope.
  - **Gate if scheduled:** Reference-derived integration tests verify trigger delivery and captured beam-position readback against the selected device's hardware specification.

<a id="task-12-6"></a>

- **12.6: Genlock and External Video Synchronization**
  - When separately scheduled, implement external HSYNC/VSYNC delivery from the 23-pin RGB video connector (contacts 11/12) to Agnus, the BPLCON0.ERSY input-direction control and the resulting beam synchronization. Establish the selected genlock device's external clock requirements from its hardware specification.
  - **Gate if scheduled:** Reference-derived tests verify external signal delivery, direction changes and beam synchronization, including reset and restored continuation. Ordinary internally generated video must remain supported without an attached genlock.
