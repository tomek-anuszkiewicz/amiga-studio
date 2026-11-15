# Amiga 500 Emulator: Development Roadmap

## Contents

- [Step 2: Native Disk Bring-Up and Verification](#step-2)
  - [2.5: Kickstart ROM & Floppy Subsystem Bring-Up for Native Program Execution](#task-2-5)
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
- [Step 8: Real-World Amiga Workloads & Pipeline Optimization (Post-Boot)](#step-8)
  - [8.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate](#task-8-1)
  - [8.2: End-to-End Bootable ADF Integration Testing](#task-8-2)
  - [8.3: CPU Core Memory Footprint Audit & Compaction Assessment](#task-8-3)
  - [8.4: Cross-Architecture & Mobile Performance Projections](#task-8-4)
  - [8.5: Fidelity & Regression Validation Gate](#task-8-5)
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

Complete the native Kickstart/floppy execution path, then run original vAmigaTS
ADF programs and compare their results with the applicable references. Reuse
existing component and machine tests; add focused regressions for concrete
bring-up failures. Light-pen/gun input (12.5), external genlock synchronization
(12.6), and expansion-connector INT2/INT6 sources (12.3) retain their deferred scope.

<a id="native-disk-bring-up"></a>

<a id="task-2-5"></a>

- **2.5: Kickstart ROM & Floppy Subsystem Bring-Up for Native Program Execution**
  - **Objective:** Complete the native disk/Kickstart/OS path and its remaining boot prerequisites so genuine Amiga programs execute from ADF disk images. Reuse the existing ROM loader, reset/overlay path, drive controls, MFM utilities and chipset DMA engines.
  - **Actionable Scope:**
    - **Missing media stream:** `FloppyController::step_cck()` currently applies drive-control edges but does not generate a runtime image stream. Image insertion and sector encoding helpers exist; the machine DMA test supplies raw pulses itself. Implement the missing production path rather than treating that fixture as a working drive.
    - **Drive and track execution:** Complete DF0: rotation, track/side selection, INDEX and timed MFM read-data generation from the selected ADF track through the existing connector pins. Preserve CIA-B motor/selection/STEP control and CIA-A sense routing. Keep media buffers and host loading outside core filesystem I/O; advance the drive from emulated CCKs.
    - **Complete disk DMA path:** Connect the autonomous drive stream to Paula's data separator, synchronization and FIFO, then through Paula DMAL requests and Agnus arbitration/address generation into Chip RAM. The guest interprets raw MFM words and sectors. Keep disk write-data/write-enable routing distinct and implement the media-write path when required by the selected native workloads.
    - **Transfer correctness:** Verify independently expected successive data, DSKSYNC/DSKBYTR behavior, DMA direction, pointer/count progression, block completion and interrupts. Exercise CPU/shared-bus contention, pending transfers and exactly-once effects. Reuse valid deterministic pin tests to isolate failures, while end-to-end runs obtain data from the inserted image. Include disk-operation reset and serialized continuation checks without extending this into a register-wide chipset campaign.
    - **Firmware and program startup:** Validate reset vectors and CIA-controlled ROM overlay through actual Kickstart execution, then boot a suitable ADF and verify the required system services, program launch and applicable return behavior. Resolve firmware, media and model failures through the production path; host-filled RAM or substituted OS initialization does not establish boot support.
  - **Verification Gate:**
    - Focused drive/MFM tests in `crates/floppy/tests/` and production DMA/boot integration tests in `crates/machine_loop/tests/` pass for the selected configurations.
    - An inserted ADF supplies expected data through autonomous drive execution, Paula and Agnus into Chip RAM, with correct completion and guest progress under contention.
    - Kickstart boots a selected ADF and its native program reaches an independently identified result within a bounded emulated-time budget. Record ROM, image, machine preset and actual result; the insert-disk screen alone does not establish media boot support.

<a id="native-disk-verification"></a>

<a id="task-2-6"></a>

- **2.6: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** After 2.5, execute original native vAmigaTS ADF programs through the verified disk/firmware/OS path, validating drive control, floppy DMA, CPU coordination and required operating-system interactions.
  - **Actionable Scope:**
    - Execute selected disk-based vAmigaTS cases, preserving their original startup and tested operation. Record source/revision, applicable OCS configuration, ROM/image, observation point and expected result; compare output with the applicable reference captures or data.
    - **Audio readiness:** Check independently expected digital sample sequences for the selected audio cases. Complete the basic host-playback portion of 6.1 before or during 2.6, before running those cases, so their sound can be heard as well as checked automatically. Keep native/WASM frontend audio boundaries; later presentation shaders do not gate these tests.
    - **Host input readiness:** Complete the basic keyboard, mouse and joystick/gamepad routing from 6.2 before or during 2.6, before executing tests that require those inputs. Identify each selected test's required controls and verify host key presses/releases, mouse motion/buttons and joystick directions/fire through the frontend boundary into the emulated devices and guest-visible behavior. Preserve native/WASM separation and core ownership. Advanced controller features and the deferred 12.2 peripherals are not prerequisites for ordinary input tests. See the [GUI specification](Obsidian/Amiga/Design/Application/GUI.md).
    - **Retire the old CPU examples:** Remove `examples/m68k/` together with the generator in `crates/debugger/tests/test_sample_binaries.rs` and the collection-specific consumers in `crates/debugger/tests/test_instruction_trace.rs`. Review distinct algorithm/debugger coverage and retain useful focused checks without recreating all five demos. Update README launch examples, testing documentation and path-specific verification configuration; verify that tests no longer recreate the directory. Use a verified runnable program for replacement instructions; a new standalone-program catalog is not a prerequisite.
    - Leverage unthrottled headless execution (running as fast as the host CPU permits without wall-clock rate limiting) for maximum throughput.
  - **Verification Gate:**
    - Automated reports identify the cases/configurations actually run, their expected outputs and comparison results, with zero unexpected halts. Unsupported cases and unrun coverage remain explicit.
    - Audio scenarios require functioning emulated sample generation, digital output checks and a verified host playback path.
    - Input-dependent scenarios require verified host-to-guest keyboard, mouse and joystick paths, including release/neutral transitions. Confirm the guest receives and reacts to the required inputs; UI event capture alone does not pass this gate.
    - Old-example removal leaves no active generator, consumer or runnable instruction pointing to `examples/m68k/`; affected debugger checks and the replacement launch example pass.

<a id="task-2-7"></a>

- **2.7: Lockstep Differential Tracer vs. vAmiga (Contingency)**
  - **Trigger:** Only if native disk verification (2.6), focused regressions and execution traces fail to isolate a divergence from the reference. This remains a contingency after native program execution, not a prerequisite for disk bring-up.
  - **Objective:** Build a cycle-exact co-simulation harness that runs the same ROM/ADF through both this emulator and vAmiga in lockstep, dumps a full machine state snapshot at every CCK boundary, and reports the first divergence point with a structured diff.
  - **Actionable Scope:**
    - Add a `StepTracer` trait (or feature-gated callback) to `MachineLoop`: at each `step_cck()`, serialize `CpuState` + all chip register banks into a compact binary or JSON snapshot. All state structs already implement `serde::Serialize` — snapshot is essentially `serde_json::to_string(&machine.snapshot())`.
    - Wire vAmiga's existing headless state-dump mode (`ref_src/vAmiga`) to emit equivalent snapshots at the same CCK boundaries.
    - Harness (`tools/verification/lockstep_diff.py`): load both snapshot streams, walk them in parallel, and stop at the first CCK where any field diverges — output: `CCK #N | field | expected (vAmiga) | actual (ours)`.
    - Keep tracer behind a feature flag (`--features tracer`) so zero overhead in production builds.
  - **Verification Gate:**
    - Tracer successfully identifies the CCK and field of a known injected regression (synthetic test).
    - `python tools/verification/verify.py test rust crate test_runner suite test_lockstep_tracer` (smoke test against a trivial ROM loop).

---

<a id="debugger-execution-control"></a>

<a id="step-3"></a>

## Step 3: Developer Studio GUI, Recording and Rewind (Ongoing Companion Track)

<a id="task-3-1"></a>

- **3.1: Continuous Testing & Refinement of Developer Studio (Debugger View)**
  - **Objective:** Actively test, harden, and refine the Developer Studio GUI (`crates/gui`) under live emulation across all display tiers and interactive workflows.
  - **Actionable Scope:**
    - Test interactive panels under live stepping: Disassembly infinite stream browsing, in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
    - **Power LED indicator:** Display the emulated power LED in the GUI using the machine's LED state. Keep the indicator current during running, stepping, reset and save-state restoration.
    - Verify existing file-based state operations and in-memory quick slots, including F6/F9, in both views. The persistent visual catalog is owned by 7.3.
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate gui suite test_interactions` (all headless egui interaction tests passing).

<a id="task-3-2"></a>

- **3.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability**
  - **Objective:** Complete audit, documentation, and user-facing surfacing of all implemented keyboard shortcuts across Developer Studio and Game View modes, ensuring zero hidden keys.
  - **Actionable Scope:**
    - Implement a dedicated in-app Keyboard Shortcuts Reference Modal (`?` / `F1` / Top Menu Help > Shortcuts).
    - Enrich button hover tooltips with contextual shortcut cues.
    - Surface all shortcuts: View Modes (`F12`/`F2`, `Escape`), Stepping Controls (`F5`/`Space`, `F10`, `Shift + F10`, `F11`), Tooling & Machine Controls (`F8`, `Alt + T`, `Ctrl + R`, `Ctrl + O`), Save States (`F6`, `F9`, `Ctrl + S`, `Ctrl + L`), Display Zoom (`Ctrl + +/=`, `Ctrl + -`, `Ctrl + 0`), and Hex/Disassembly editing navigation.
  - **Verification Gate:**
    - Headless integration test in `crates/gui/tests/` verifying modal dialog trigger and shortcut handling.

<a id="task-3-3"></a>

- **3.3: Watchpoint Execution Stops**
  - UI/backend watchpoint definitions and matching exist, but guest memory accesses do not invoke matching to pause execution. Route actual guest accesses to it while keeping debugger inspection neutral.
  - **Gate:** Verify matching access types/ranges and actual execution stops. See [Debugger](Obsidian/Amiga/Design/Application/Debugger.md).

<a id="task-3-4"></a>

- **3.4: Full-Machine Debugger Rewind and Differential State**
  - Current history restores CPU state only, not RAM or other chips. This is the detailed debugger history mode; the complementary checkpoint/replay mode for both views is in 3.6. Implement deterministic full-machine rewind: record base/target CCKs, changed subsystem values and reversible physical memory writes; apply/rewind all changed state only from a matching base/target CCK. The differential `A500DiffState` remains a design proposal until reviewed and implemented.
  - **Format proposal for review:** `DIFF` magic (4 B), base CCK (8 B), delta CCK (2 B), subsystem flags (1 B), variable subsystem payloads, memory count (2 B), then entries `[address: u24, new: u16, old: u16]` (7 B each). The illustrative 22 B/no-write and 29 B/one-write footprints depend on which fields change; this is not an implemented format or a saved-data migration.
  - **Gate:** Review the format before implementation and prove all-state restoration plus deterministic full-machine forward/backward continuation. See [Save states](Obsidian/Amiga/Design/Application/SaveState.md).

<a id="task-3-5"></a>

- **3.5: Deterministic Session Recording and Replay**
  - Record a playable session in Game and Debug views from an identified initial machine state, hardware configuration, ROM and disk contents. Capture keyboard press/release, mouse movement/buttons and joystick/gamepad input as delivered emulated-device events, together with disk insert/eject/swap actions, resets, state loads and other machine-affecting controls.
  - Timestamp events at their actual emulated CCK delivery boundaries and preserve ordering. Playback feeds the same production input/control paths at those boundaries. Define how external inputs such as host-supplied RTC updates are recorded or reproduced; the live RTC is currently excluded from snapshots. Keep event capture, asset loading and storage outside the core stepping loop.
  - Keep initial media contents and any subsequent guest writes reproducible, including the currently inserted disk. A machine snapshot alone does not currently serialize the host-owned ADF buffers. Use the same session/event model for standalone replay and checkpoint rewind rather than separate recording mechanisms.
  - **Gate:** On native and WASM, replay a recorded session from its declared starting point and compare machine progress, relevant state and outputs at selected checkpoints. Include held-key releases, mouse actions and disk swaps. Distinguish executable input replay from video capture in 7.7.

<a id="task-3-6"></a>

- **3.6: Checkpoint and Replay Rewind in Game and Debug Views**
  - Add a second history mode using periodic machine checkpoints, including differential checkpoints, and the event recording from 3.5. It is available in both views on native and WASM and supports returning to an earlier frame to retry gameplay.
  - Choose the checkpoint interval and retained history budget during implementation; one, two or five emulated seconds are examples, not fixed requirements. Keep reconstructible baseline/delta chains covering machine state, required media state and event positions.
  - To seek, restore the nearest checkpoint at or before the requested video frame/field, then execute recorded events through the production machine as fast as practical until the exact target boundary. Count actual PAL/NTSC video boundaries, including interlaced fields; a displayed screenshot alone is not restored execution. Resuming play starts from that restored state; decide whether to replace or retain the previous future history before implementation.
  - Keep frontend seeking responsive and allow normal continued play, quick save/load and switching views. Review interactions with the detailed debugger history in 3.4 and the persistent manual saves in 7.3; automatic rewind checkpoints and manual saves have separate lifetimes.
  - **Gate:** Seek to targets both on and between checkpoints and compare with uninterrupted execution. Verify continued play, disk changes and guest media writes, input releases, baseline/delta reconstruction and native/WASM behavior. Measure the memory/storage cost and seek delay before choosing defaults.

---

<a id="external-hardware-verification"></a>

<a id="step-4"></a>

## Step 4: vAmigaTS Silicon Verification Sub-Suites & Contention Track (After Chipset Bring-Up)

<a id="task-4-1"></a>

- **4.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)**
  - **Objective:** Enforce strict gating and categorization for tests requiring hardware beyond the baseline A500 OCS configuration.
  - **Actionable Scope:**
    - Maintain and audit formal gate categories: FPU Coprocessors (206 tests deferred to the scope decision in Step 10.2), ECS & AGA Silicon (112 tests deferred to Steps 10.1 and 10.2), 68010 CPU Architecture (91 tests deferred to Step 10.2), AmigaOS Floppy MFM Bootblock (7 tests deferred to Step 8.1), and Non-Visual Register Assertions (193 tests deferred to Step 4.2.6).
    - A future capture entry point must classify supported and deferred tests before executing baseline OCS suites; the previous runner has been removed.
  - **Verification Gate:**
    - Demonstrate supported/deferred selection through the future capture entry point before executing its baseline; no current Cargo suite provides this gate.

<a id="task-4-2"></a>

- **4.2: Targeted Subsystem Verification Sub-Suites Execution Track (1,468 Active Baseline Tests)**
  - **Objective:** Progressively execute and pass the 1,468 active baseline vAmigaTS tests in strict substrate-first causal order (Layer 0 $\to$ Layer 4). Re-establish the reporting format when this verification track resumes.
  - **Actionable Scope:**
    - **Sub-Suite 4.2.1: Agnus Master DMA Contention & Bus Arbitration (Layer 0 Substrate, 225 tests):** `Agnus/` (`DMACON/`, `BplDma/`, `DIW/`, `DDF/`, `bususage/`), verifying Color Clock phase allocation (odd/even), refresh cycles, CPU wait-state generation, and Blitter Nasty (`BLTPRI`).
    - **Sub-Suite 4.2.2: Agnus Blitter & Copper Coprocessor Engines (Layer 1 Coprocessors, 364 tests):** `Agnus/Blitter/` (250 tests: `line/`, `fill/`, `sblit/`, `timing/`, `bbusy/`, `bltint/`) and `Agnus/Copper/` (114 tests: `Wait/`, `Skip/`, `coptim/`, `coprace/`, `copvbl/`), verifying autonomous bus master operations, 256-minterm ALU, barrel shifts, modulos, and Copper 4-CCK timing.
    - **Sub-Suite 4.2.3: Denise Video, Bitplanes & Sprites (Layer 2 Video Serializer, 210 tests):** `Denise/` (`Registers/`, `Modes/`, `DIW/`, `Sprites/`), verifying pixel serialization, palette translation, display window clipping, and sprite multiplexing.
    - **Sub-Suite 4.2.4: Paula Audio & Floppy Subsystem (Layer 3 Peripherals, 107 tests):** `Paula/` (`Audio/`, `Interrupts/basicint/`), verifying PCM sample streaming, period clock division, and Level 1–4 interrupt requests.
    - **Sub-Suite 4.2.5: Complex CIA-A / CIA-B & Timers (Layer 3 Peripherals):** Timers A & B, TOD clock 50/60 Hz synchronization, serial shift register (SDR), and port handshake lines.
    - **Sub-Suite 4.2.6: Mainboard, Memory & Peripheral Assertions (Layer 4 System, 58 visual + 193 non-visual tests):** `Mainboard/`, `Memory/`, `Misc/`, verifying address decoding, port registers, and RAM expansion configurations.
    - **Sub-Suite 4.2.7: M68000 CPU Silicon Pipeline (Layer 4 System Integration, 503 tests):** `CPU/` (ALU, bitwise, shifts, exceptions, traps, IPL autovectors).
  - **Verification Gate:**
    - Restore a capture-runner entry point and verify each selected vAmigaTS category against its hardware reference captures.

<a id="task-4-3"></a>

- **4.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)**
  - **Objective:** Optimize the dynamic per-CCK DMA slot priority evaluation into a clean, pre-computed scanline slot lookup table (`[DmaSlot; 227]`).
  - **Actionable Scope:**
    - Localize optimization strictly within the Agnus DMA scheduler (`crates/agnus/src/dma.rs`).
    - Preserve code simplicity and readability, with zero complex asynchronous event wheels or timing skips.
    - Once a representative whole-machine workload is executable, compare fresh throughput before and after the optimization; CPU-only benchmarks do not measure Agnus scheduling.
  - **Verification Gate:**
    - Zero regression across all active baseline OCS vAmigaTS suites.
    - Run DMA, Agnus and production memory_bus regression tests, including contention and pending CPU-request arbitration; CPU-only waiting tests do not validate scheduler changes.

<a id="task-4-4"></a>

- **4.4: Display Modes, Color Fidelity and Dynamic Borders**
  - When separately scheduled, extend the display coverage from 2.6 and 4.2.3 with focused test programs and representative games once their native execution prerequisites are available.
  - **Resolution changes:** Test low/high resolution switches between frames, within a frame and within a scanline, including mixed-resolution images. Verify pixel placement, proportions and frontend scaling across transitions.
  - **Interlace:** Test interlaced output, alternating field alignment, field presentation and transitions between interlaced and non-interlaced modes.
  - **Color modes and accuracy:** Test supported OCS palette/bitplane configurations, dual playfield, Extra Half-Brite and HAM6, including palette and mode changes during rendering. Verify both generated color values and displayed colors against WinUAE and a physical Amiga, recording reference settings and hardware capture conditions.
  - **Borders and changing display area:** Test overscan and games that change the size or position of the visible area during a frame or between frames. Determine how to retain or crop the border and how cropping and scaling should react to those changes; check for lost visible content, unwanted image movement and stale pixels.
  - **Gate:** Record reproducible comparisons with WinUAE and a physical Amiga, covering static images and transition sequences. Agree on the border/cropping behavior from those cases, then verify it in the framebuffer and frontend; keep emulator agreement distinct from hardware evidence.

---

<a id="step-5"></a>

## Step 5: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)

<a id="task-5-1"></a>

- **5.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector**
  - **Objective:** Implement dedicated register inspection docks/tabs in Developer Studio (`crates/gui`) with live delta highlighting and mutation delay observability.
  - **Actionable Scope:**
    - Dedicated docks for Agnus, Denise, Paula, CIAs (A & B), and RTC.
    - Format values in hex/binary with electric cyan change/delta highlighting.
    - Bitfield breakdown widgets and interactive tooltips for control/status registers (`DMACON`, `INTENA`, `INTREQ`, `BPLCON0`-`BPLCON2`, `ADKCON`, `CRA`/`CRB`).
    - Visualize staged pipeline register latches taking effect across Color Clock phases ($CCK1 \to CCK2$).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying register dock rendering and delta tracking.

<a id="task-5-2"></a>

- **5.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer**
  - **Objective:** Build a horizontal scanline timeline visualizer displaying the 227 Color Clock slots per scanline (PAL) / 226 slots (NTSC).
  - **Actionable Scope:**
    - Fixed allocations: DRAM refresh (CCK 0..3), Disk DMA (CCK 4), Audio DMA 0–3 (CCK 5..8), Sprite pairs 0–7 (CCK 12..27).
    - Dynamic allocations: Bitplane DMA (BPL 1–6), Blitter DMA (channels A, B, C, D), and CPU bus slots.
    - Live beam position cursor tracking horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates.
    - Bus contention & wait-state indicator visually flagging cycles where CPU or Copper are stalled (BLTPRI / Blitter Nasty).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying timeline visualizer rendering.

<a id="task-5-3"></a>

- **5.3: Copper Coprocessor Inspector & Real-Time Execution Tracker**
  - **Objective:** Implement a dedicated Copper list disassembler panel decoding instruction streams directly from Chip RAM with real-time execution tracking.
  - **Actionable Scope:**
    - Decode instruction streams (`MOVE`, `WAIT`, `SKIP`) from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
    - Real-time execution pointer tracking: highlight executing Copper instruction, pending wait condition ($VPOS$/$HPOS$ comparison), and CDANG danger mode status.
    - Visual correlation with raster beam: highlight beam position where Copper interrupts or register modifications trigger palette swaps or Blitter dispatches.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying Copper disassembler panel.

<a id="task-5-4"></a>

- **5.4: Internal Chipset State Machines & Deep Diagnostics**
  - **Objective:** Surface live diagnostic panels for internal chipset state machines in `crates/gui`.
  - **Actionable Scope:**
    - *Blitter Engine Diagnostics:* Visual representation of active channels (A, B, C, D), 256-minterm truth table visualization ($LF$ code decomposition), shift/mask register stages, and Bresenham line-drawing state counters.
    - *Denise Video & Sprite Pipeline:* Live inspection of bitplane serializers, dual-playfield priority layers, 8 hardware sprite registers, and collision latches (`CLXDAT`/`CLXCON`).
    - *Paula Multi-Engine Status:* Audio channel frequency, length, volume, BLEP table synthesis status, floppy MFM bit-stream buffers/sync-word detector (`$4489`), and serial UART FIFO.
    - *CIA Timers & Port Observability:* Live countdown of Timers A & B, TOD clock sub-second counters, serial shift register (SDR) status, and I/O port pin states.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying state machine diagnostic views.

---

<a id="step-6"></a>

## Step 6: Host I/O Peripherals, Audio Playback & Controller Hub

<a id="task-6-1"></a>

- **6.1: Host Audio Playback & Image Presentation Shaders**
  - **Objective:** Operationalize host real-time audio output streaming, evaluate image enhancement filters, and support authentic CRT display presentation.
  - **Execution order:** Complete basic audio playback before or during 2.6, before its audio scenarios run. Keep later playback refinements and presentation shaders in this track; do not delay the basic sound path until after external-suite bring-up.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - Evaluate optional GPU shaders for pixel-art scaling and edge smoothing before selecting an algorithm. Compare image quality and rendering cost on native and WASM; preserve uniform image scaling and keep source framebuffer samples unchanged.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/paula/tests/`.
    - Headless shader initialization and rendering verification.
    - Visual comparison with unfiltered output, including text readability, mixed-resolution graphics, and motion, plus rendering cost measurements on native and WASM.

<a id="task-6-2"></a>

- **6.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/game_ports/src/mouse.rs`, `crates/game_ports/src/joystick.rs`, `crates/game_ports`, `crates/gui`).
  - **Execution order:** Complete basic keyboard, mouse and joystick/gamepad routing before or during 2.6, before its input-dependent scenarios run. Keep extended controller protocols, mapping options and input UI refinements in this track; do not defer the basic host-to-guest input path until after external-suite bring-up.
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Reusable Mapping Profiles:** Allow users to create, name, save and edit any number of host-to-Amiga keyboard mappings (initially configurable special keys), keyboard-as-joystick mappings, and gamepad-to-joystick mappings. Let users compose these mappings into named input profiles, select the active profile and apply it during emulation, including the selected game-port assignments. Keep input profiles distinct from the game/asset profiles in 7.5.
    - **Gamepad Button Assignments:** Let users choose which physical button produces joystick fire and assign other buttons to frontend actions such as pause/resume, quick save/load and opening the save/load catalog. Reuse the controls from 7.3/7.4 as they become available. Define how bindings consume or share an event between guest input and frontend actions; keep full gamepad-only interface navigation in 7.8.
    - **Keyboard Combination Fidelity (basic host adapter accepted; hardware fidelity remains open):** Define how host key combinations produce the same guest-visible effects as an A500 keyboard. Establish the original matrix/controller behavior from documentation and hardware measurements, including simultaneous-key limits, blocked keys and phantom key detection (ghosting), then compare it with host keyboard rollover limits and native/browser event delivery. Decide how to reproduce Amiga behavior, preserve held modifiers and press/release sequences, and handle conflicts with frontend or system shortcuts. Explicitly address combinations the host never reports: document the limits and decide on alternative mappings or other workarounds without assuming missing key events can be recovered. Record the agreed behavior in the keyboard and GUI specifications before implementation.
    - **Dual Game Port Hub:** Slot assignment for Port 1 & Port 2 (Mouse, Joystick, CD32 Pad, Keyboard-Joystick, Disconnected), simultaneous dual-mouse and dual-joystick modes, hardware button routing (CIA-A `PRA`, Paula `POTGOR`), and CD32 7-button shift register protocol.
    - **Physical Host Mouse Capture:** Viewport mouse grab/lock mode, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), and sensitivity tuning.
    - **Physical Gamepad & Joystick Ingestion:** Host gamepad enumeration via `gilrs` (native desktop) and HTML5 Gamepad API (WASM), analog deadzones, and button mapping.
    - **Developer Studio Input UI:** Interactive "Game Ports & Input" tab/dock displaying port status, device dropdowns, and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/` and `crates/game_ports/tests/`.
    - Headless UI input tests in `crates/gui/tests/`.
    - Create and persist several mappings of each supported type, compose and select input profiles, and verify actual guest key/direction/fire delivery and frontend actions on native and WASM. Switching profiles with controls held, focus loss and controller disconnect must release obsolete guest inputs without stuck keys or buttons; verify the agreed event-sharing behavior.
    - Verify representative multi-key combinations, modifiers, reset and keyboard-as-joystick controls against the agreed A500 behavior, including different press/release orders and focus loss. Combine deterministic event tests with physical host keyboard checks on native and WASM; compare guest-visible effects with Amiga reference evidence and record host limitations and agreed workarounds. Injected events alone do not verify physical keyboard combination support.

<a id="task-6-3"></a>

- **6.3: Audio Reconstruction and Filtering Experiments (Deferred Exploration)**
  - **Revisit when:** Games run and music can be loaded and heard through the host playback path from 6.1. This is an optional experiment, with no commitment to deliver an enhancement feature.
  - Compare faithful Paula output and Amiga analog filtering with per-channel reconstruction adapted to the current playback rate, such as band-limited interpolation. Use sample values and actual playback timing; the original recording sample rate is not required.
  - Distinguish aliasing introduced by host sample-rate conversion from spectral images present in the original DAC output. Explore cleaner reconstruction before channel mixing while retaining the Amiga analog filter model as a separate stage; distinguish sound enhancement from hardware fidelity.
  - **Evaluation:** Use listening comparisons, spectral measurements and processing-cost measurements on representative game sounds and music, including slow playback, period changes, loops and modulation. Record audible benefits and changes in character, then decide whether any approach merits an optional playback mode. A documented decision to retain the faithful path is a valid outcome.

---

<a id="step-7"></a>

## Step 7: Dedicated Player GUI & Frontend Experience

<a id="task-7-1"></a>

- **7.1: Shared Hardware Configuration and Recognized Kickstart ROM Selection**
  - Expose hardware configuration in both Game and Debug views on native and WASM: memory sizes, supported machine presets and other supported settings, plus Kickstart selection. Reuse the existing ROM file loader and machine configuration, including Basic A500 512 KB, Classic A500 1 MB (recommended) and Expanded A500 4 MB presets. Warn and obtain confirmation before applying a configuration or ROM change that requires resetting the current session, then use the required reset path.
  - Add a recognized ROM catalog with version/model labels and verified content hashes. Review established catalogs such as WinUAE's and record entry provenance and the digest algorithm before adopting recognition data. Recognition identifies a known image; it does not extend the supported ROM sizes or hardware models.
  - Scan a user-selected ROM directory on desktop and offer recognized images found there by version, including a default selection for a requested Kickstart such as 1.3. Define the default when several recognized variants match. Continue to allow direct selection of any structurally valid ROM supported by the emulator; an unknown hash alone must not reject it.
  - Define the browser equivalent using bundled assets or explicitly selected files/directories within browser capabilities. Associate the selected ROM and configuration with the game profile in 7.5.
  - **Gate:** Verify configuration access in both views, reset/application behavior, hash recognition independent of filenames, directory selection and version-default resolution, plus direct loading of an unrecognized supported ROM. Cover native and browser paths with their actual access constraints.

<a id="task-7-2"></a>

- **7.2: Multi-Drive Floppy Disk Manager (`DF0:` - `DF3:`)**
  - Provide disk insertion, ejection, write protection and live drive activity in both views, including DF0: and supported external drives. Keep external-drive enable/disable controls and drive-step sound in this track.
  - Present a game's ordered disk list from its profile, such as four associated ADF images, so a disk swap selects a known game disk without repeating file selection. Preserve the mounted disk/drive assignments when switching views; record machine-affecting changes for 3.5/3.6.
  - **Gate:** Verify multi-drive controls, profile disk selection and actual guest-visible media changes through the production floppy path; cover the native and browser asset sources.

<a id="task-7-3"></a>

- **7.3: Persistent Save State Catalog with Screenshots and Creation Times**
  - Provide a visual save/load catalog in Game and Debug views on both native and WASM. Reuse existing full-machine snapshots, PNG preview encoding, file import/export and quick slots. Snapshot cards show the saved image and creation date/time; saving must not require a user-entered name. Custom labels are optional.
  - Supply actual creation time at the host persistence boundary; capture currently writes a zero timestamp. Associate saves with the game/profile and its required assets. On desktop, keep saves with the portable game directory when that delivery mode is selected. For WASM, design persistent browser storage plus import/export, including storage-failure reporting, so user saves remain separate from the original bundle and survive reopening.
  - Make existing quick save/load readily accessible in both views and define their persistence alongside the catalog. Keep manual saves and quick slots distinct from automatic rewind history; filenames or internal identifiers need not become mandatory display names.
  - Restore saved hardware configuration, ROM, execution state and the required media coherently. Review the current host-owned disk-buffer gap before claiming a game is resumable. Preserve the active CPU micro-execution path during branches, exceptions and interrupted transfers; opcode-only reconstruction is insufficient.
  - **Gate:** Save multiple unnamed states, verify correct thumbnails and real creation times, reopen the application/page and restore the selected state. Check both views, quick save/load, required media and identical continued execution on native and WASM; verify that a new user save does not replace the bundled starting state.

<a id="task-7-4"></a>

- **7.4: Game View Controls and Layout**
  - Expand Game view with access to configuration/ROM selection (7.1), the game's disks and drives (7.2), save-state previews and quick save/load (7.3), playback/recording and rewind (3.5/3.6), and normal start/pause/reset controls. Keep the same machine and selections when switching to Debug view.
  - Explore moving Game view controls from the top toolbar into a side panel on wide displays, using the space beside the Amiga image. Compare this with a compact profile-driven interface where the game's assets are already associated. Decide the final arrangement from prototypes; retain usable controls in narrow browser windows.
  - Preserve uniform scaling and the framebuffer's width-to-height ratio. Extra side space does not establish a fixed 4:3 output requirement; retain the separate border/cropping decision in 4.4.
  - **Gate:** Review the selected layout visually on wide and narrow native/browser windows and verify the shared configuration, disk, state and execution actions in both views.

<a id="task-7-5"></a>

- **7.5: Game Profiles and WASM Bundles with Immediate State Resume**
  - Define a portable game profile associating hardware configuration, Kickstart, the game's ordered disks, optional artwork/preview and an optional starting save state. It is the shared description for launching a game and resolving its saves, not a requirement to embed every game in a separate executable.
  - Build a WASM delivery bundle containing the runtime and required game assets, including ROM/disks and the optional starting state/image. Once assets are available, launch directly into Game view. With a starting snapshot, present its game screen and resume that state rather than showing firmware boot and disk loading again; without one, use the normal native boot path.
  - Keep the packaged starting state separate from subsequent user saves. Resolve snapshot/profile configuration and media together; apply the persistent WASM save catalog from 7.3. Review browser startup interaction requirements for audio/input without making the guest repeat its boot sequence.
  - **Gate:** Open a generated bundle in a fresh browser session without manually selecting ROM/disks, verify the first game screen and continued execution from the supplied state, switch a bundled disk, then save, reopen and restore user progress. Also verify the profile's normal boot path when no starting snapshot is supplied.

<a id="task-7-6"></a>

- **7.6: Desktop Game Library and Portable Game Packages**
  - Provide a desktop list/gallery of game profiles with artwork where available and direct access to launch and saved progress. Reuse the profile and asset associations from 7.5; the library must not depend on games being distributed as WASM files.
  - Evaluate a portable game directory containing configuration, ROM, disk images, optional artwork/starting state and user saves alongside an executable with embedded game assets. Select the delivery form before implementation; the shared game profile remains usable with either choice.
  - **Gate:** Open multiple game profiles and resolve each game's assets and saves independently. Record and verify the chosen packaging approach; if directory packaging is selected, move the directory and launch/resume it using relative asset references. Other delivery variants remain proposals.

<a id="task-7-7"></a>

- **7.7: Gameplay Video Recording**
  - Plan video capture separately from deterministic session replay. Select the native/browser capture and export approach, including whether audio is included and its synchronization, before implementation. A recorded movie does not provide rewindable machine state or executable input history.
  - **Gate:** Verify the selected recording/export path produces playable output with correct frame timing and any selected audio, and assess its effect on gameplay responsiveness.

<a id="task-7-8"></a>

- **7.8: Full Frontend Control with a Gamepad (Deferred)**
  - After the input profiles in 6.2 and the player controls in this stage are available, provide an optional gamepad-only mode for the entire frontend, without requiring a host keyboard or mouse. Cover menu navigation, configuration, game/disk selection, save/load catalogs and session controls; plan any text-entry interaction needed by those flows.
  - Reuse configurable frontend button assignments from 6.2 and define transitions between guest control and interface navigation, including focus and return to play.
  - **Gate:** On native and WASM, complete the supported frontend flows and return to gameplay using only a physical gamepad, with no unintended guest input or stuck controls.

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
  - **Input diagnostic trigger:** As soon as Workbench boots and basic host input from 6.2 is available, run the interactive hardware-diagnostic checks below; do not wait for the rest of Step 8 or advanced input profiles.
  - **Actionable Scope:**
    - Load and execute `AmigaTestKit.adf`, `SysInfo.adf`, and Dhrystone on the authentic Kickstart / Amiga chipset stack.
    - Use suitable Amiga hardware-diagnostic programs, including the existing AmigaTestKit candidate, to exercise the complete physical host-to-guest input path: keyboard keys, modifiers and press/release combinations; mouse motion and buttons; joystick directions, diagonals and fire; and gamepad-to-joystick routing. Check selected mappings as they become available. Use the programs' actual supported tests and record gaps separately.
    - Leverage address guards, breakpoint traps, and instruction bounds for parameterized termination.
    - Assert functional correctness, numerical determinism, and measure baseline throughput (MIPS, host CPU cycle cost per emulated CCK).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate test_runner suite test_boot_adf` passing cleanly.
    - Record the diagnostic program/version, ROM/image, machine configuration, host platform, physical devices, selected mappings, exercised controls and guest-observed results. Exercise native and WASM delivery and report unrun cases explicitly; successful Workbench boot or synthetic event injection alone does not verify interactive input delivery.

<a id="task-8-3"></a>

- **8.3: CPU Core Memory Footprint Audit & Compaction Assessment**
  - **Objective:** Audit the exact memory footprint of the CPU core and assess compaction strategies.
  - **Actionable Scope:**
    - Audit host `.rodata` footprint (sizeof(OpcodeDescriptor) * 65,536, microcode arrays) and runtime state footprint (`Cpu`, `CpuState`, `CpuMicroState`).
    - Compare whether bit-packing or deduplication improves throughput on representative workloads, or whether the flat layout performs better.
    - Refactor slow handlers using host efficiency principles without custom macros or const-generics.
  - **Verification Gate:**
    - Memory footprint audit report and SingleStepTests passing with zero regressions.

<a id="task-8-4"></a>

- **8.4: Cross-Architecture & Mobile Performance Projections**
  - **Objective:** Extrapolate measured throughput to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile).
  - **Actionable Scope:**
    - Model CPU overhead margins to ensure sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation with full chipset contention and rendering.
  - **Verification Gate:**
    - Documented performance analysis projections in `Obsidian/Amiga/Design/`.

<a id="task-8-5"></a>

- **8.5: Fidelity & Regression Validation Gate**
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
  - In an isolated checkout, remove one selected subsystem and regenerate it from curated specs, rules, and test vectors. Compare behavior and source with the preserved original; use failures to refine documentation and instructions.
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
  - Extract bitplanes, sprites, palettes, and audio from guest memory; annotate assets, addresses, and game phases with optional LLM assistance and map them in the 24-bit address space.
  - **Gate:** Asset extraction unit tests in `crates/debugger/tests/` verify output and source-address provenance.

<a id="step-11"></a>

## Step 11: Four-Manual PDF-to-Book JSON Conversion

<a id="task-11-1"></a>

- **11.1: Convert Four Source PDFs to Reviewed `book.json` Recipes**
  - Prepare the A500/A2000 Technical Reference Manual, Amiga Hardware Reference Manual (second edition), M68000 Programmer's Reference Manual, and M68000 User's Manual (Revision 8). Keep a separate source identity and preparation profile for each PDF.
  - Import each prepared PDF into Book-to-Markdown, then adjust chapter organization, source selections, formatting, and graphics against the source. Accept the intended revision to produce each manual's `book.json`. Use the [book authoring workflow](tools/bootstrap/book-to-markdown/README.md#book-authoring-sequence).
  - **Gate:** Review and commit all four accepted `book.json` recipes with their matching full source hashes in `source.json`. A prepared PDF or unaccepted Working bundle is insufficient.

<a id="deferred-bus-and-peripherals"></a>

<a id="step-12"></a>

## Step 12: Deferred Bus Fidelity, Peripheral Extensions and CPU Interrupt Cases

<a id="task-12-1"></a>

- **12.1: Physical Open-Bus Values and Opposite-Direction Accesses**
  - The current model returns fixed `$FF/$FFFF`, with no retained motherboard data-bus latch or complete opposite-direction disturbances. Investigate whether an undriven read returns that fixed value, retains the previously driven value, or changes toward `$FF/$FFFF` over time. Treat decay as a hypothesis requiring address-, phase-, bus-lane- and board-specific evidence, not an assumed hardware rule.
  - Include dummy/unused/opposite-direction reads and writes and their possible disturbances. Obtain the evidence before proposing changes; preserve the current approximation until a behavior/reference change is approved.
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
  - **Expansion INT2/INT6:** When an expansion device is separately scheduled, route its active-low requests from the A500 side expansion connector (86 contacts, INT2 on contact 19 and INT6 on contact 22) to Paula alongside the CIA-A/CIA-B sources. Resolve shared levels so releasing one source cannot release another asserted source; retain released inputs with no expansion attached. Keep this connector support outside current chipset reconstruction.
  - **Level 7 NMI / autovector 31:** The current emulator ignores IPL=7. Standard Paula sources generate levels 1-6, but real A500 expansion hardware can drive the CPU interrupt lines to request level 7. Record this missing external-input support and consider implementing it only when such hardware or a concrete workload is in scope, including the CPU's special level-seven acceptance behavior.
  - **Uninitialized interrupt / vector 15:** The current bus interface has no device-supplied interrupt-vector response. Built-in A500 interrupts use autovectors, so there is no standard onboard source for this case. Consider support only if a selected external device or a broader CPU-interface requirement needs it; distinguish an uninitialized peripheral vector register from an absent handler in RAM.
  - **Spurious interrupt / vector 24:** The current interface cannot report BERR during interrupt acknowledgment. Standard A500 interrupts have no identified normal source for this case. Consider implementation only for a concrete external-device or bus-fault scenario, together with the deferred BERR interface in 12.1.
  - Keep these external-input cases as low-priority completeness gaps, outside the prerequisites for current chipset reconstruction and testing. Record the required hardware/input scope before implementation and preserve existing supported exceptions. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate if scheduled:** Reference-derived tests verify the selected input, interrupt acknowledgment or vector selection, exception frame, transaction/wait behavior and continuation. Do not treat recording these gaps as implemented support.

<a id="serial-parallel-device-support"></a>

<a id="task-12-4"></a>

- **12.4: Attached Serial and Parallel Devices and Communication Verification**
  - The current model assumes both connectors remain disconnected. There is no emulated attached serial or parallel device, connector transport or host adapter establishing a working connection. The reconstructed chip logic assumes disconnected inputs; its component tests do not prove that an attached device can communicate.
  - When separately scheduled, select a deterministic attached device or loopback model and implement the required connector routing. Verify serial transmit/receive and applicable control lines, plus parallel data, PC/strobe, ACK and status, including delayed/blocked responses, interrupts, reset and restored continuation. Physical host-port adapters require their own explicit scope.
  - **Gate if scheduled:** Guest-driven exchanges through the production machine path produce independently expected data and handshake/status transitions at the selected device. Keep connection and full-exchange support explicitly unverified until these tests pass. See [Paula](Obsidian/Amiga/Design/Paula.md) and [CIA](Obsidian/Amiga/Design/CIA.md).

<a id="task-12-5"></a>

- **12.5: Light Pen and Light Gun Input**
  - When separately scheduled, implement the selected game-port trigger and motherboard LPEN route to Agnus, beam-position latching and guest-visible readback. Keep this extension outside the current chipset reconstruction scope.
  - **Gate if scheduled:** Reference-derived integration tests verify trigger delivery and captured beam-position readback against the selected device's hardware specification.

<a id="task-12-6"></a>

- **12.6: Genlock and External Video Synchronization**
  - When separately scheduled, implement external HSYNC/VSYNC delivery from the 23-pin RGB video connector (contacts 11/12) to Agnus, the BPLCON0.ERSY input-direction control and the resulting beam synchronization. Establish the selected genlock device's external clock requirements from its hardware specification.
  - **Gate if scheduled:** Reference-derived tests verify external signal delivery, direction changes and beam synchronization, including reset and restored continuation. Ordinary internally generated video must remain supported without an attached genlock.
