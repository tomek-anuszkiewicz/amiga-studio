# Amiga 500 Emulator: Development Roadmap

## Step 2: HRM-Aligned Custom Chipset Reconstruction

Current execution scope through 2.5 excludes full native vAmigaTS execution,
Kickstart/AmigaOS integration and floppy-media reads/writes. Use directly loaded
small assembly programs, including selected standalone vAmigaTS adaptations,
and deterministic pin/latch inputs. Replace required firmware/startup setup
with explicit guest initialization in those adapted programs. ROM overlay
checks use synthetic ROM bytes without executing firmware. Later firmware,
media, and full external-suite work is unscheduled and must not start
automatically after the current plans.
Light-pen/gun input (12.5), external genlock synchronization (12.6), and
expansion-connector INT2/INT6 sources (12.3) remain outside Step 2.5.
The functional baseline requires complete supported OCS functionality within
the explicit exclusions above and below. Copper and Blitter operate during
active scanlines; sprite, palette and display-mode changes also take effect
within a line under the modeled sampling/latch rules. Exact cycle offsets,
few-cycle pipeline transients and cycle-exact silicon calibration are outside
2.5 acceptance. Keep CCK stepping, documented ordering and existing
signal/register propagation; do not batch all changes at line/frame boundaries
or suppress writes. Within 2.5, the user authorizes necessary code and test
corrections, including weakening or removing faulty tests. Record the reasons
and coverage changes; reconcile affected documentation with the verified
implementation at the end, since existing documentation may also be wrong.

<a id="cpu-memory-gary-verification"></a>

- **2.5: CPU/Gary Propagation and Guest-Program Integration**
  - **Objective:** Verify the production CPU-to-memory/device path by running small, simple integration programs and checking their results using the reconstructed chipset. Cover Gary decoding, CPU waiting, ROM overlay, observable motherboard data-latch behavior, CIA/custom-register propagation, and display, sprite, Blitter, digital-audio and interaction scenarios derived from books or suitable vAmigaTS cases. Reuse 2.2-2.4 evidence and fixtures; extend missing coverage without duplicating equivalent tests.
  - **Prerequisite:** Build on the reconstructed engines, book-derived component tests, production DMA scheduler and live GUI frames recorded in [the reconstruction chapter](DIARY.md#15-book-derived-custom-chipset-reconstruction), correcting code and tests where needed. This stage authors detailed whole-machine integration and guest/output tests (`BOOK-GUEST-PROGRAM-TESTS-20261111`).
  - **Program contract:** Keep guest sources, compiled binaries and run instructions in a separate shared catalog organized by subsystem and case; the proposed root is `test_programs/`. Rust integration tests remain in the owning crate's `tests/` tree. Include the existing `live_stripes` program and migrate its machine, GUI and save-state test consumers to the single catalog binary. Record source/reference provenance, initial conditions, independent expected results and a bounded CCK/frame budget; reproduce each binary from its source during validation. Every binary must load and run from the GUI with equivalent setup and the same result as automation, including the expected screen effect for visual cases. Guest instructions perform initialization and accesses under test; host code supplies the documented environment and observes results and CCK/phase boundaries.
  - **Selected vAmigaTS adaptations:** Inspect source, startup dependencies, reference screenshots/data and validation scripts. Port suitable small cases to standalone binaries without Kickstart, OS services or disk loading, retaining the tested operation and independently justified result. Record original case/revision, adaptations, machine applicability and GUI reproduction. Full native-suite execution remains in 2.8; an adapted case does not establish that its original boot path passes.
  - **Run control:** The same binary loops indefinitely in normal GUI execution. Automation runs a configured number of emulated video frames from the documented initial state, captures the result at the final boundary, validates it and ends the run. No guest completion marker or separate test mode is required. Record the counting unit/origin and PAL/NTSC applicability; use a CCK safety limit if video progress fails. For example, 450 non-interlaced PAL frames represent nominally nine seconds at 50 Hz.
  - **Old example collection:** Review retiring `examples/m68k/` after the shared catalog is usable. Include its binary-generating and binary-consuming debugger tests, old README instructions and stale configuration/documentation references. Retain only useful distinct coverage; do not migrate all five CPU demos automatically or leave a generator that recreates the removed directory.
  - **Actionable Scope:**
    - **CPU memory transactions:** Exercise byte, word and long accesses, big-endian byte lanes, read/write direction, installed Chip/Slow/Fast RAM and ROM, bank boundaries and applicable memory presets. Confirm ROM write protection, modeled unmapped accesses and that unrelated storage is unchanged.
    - **Gary selection and aliases:** Verify CPU-visible CIA selection, dual-CIA byte/word lanes, register selectors and aliases, custom `$DE/$DF` mirrors, and installed RAM/RTC precedence according to the completed 2.2 contract. Observe the selected devices' effects as well as returned data; a correct bank classification alone is insufficient.
    - **CPU waiting and DMA handover:** Generate actual shared-bus activity through the scheduler completed in 2.4. Verify denied CPU transfers stay pending without advancing their bus micro-step or changing data, that chips continue advancing, and that the transfer completes once after a grant. Cover CPU-to-DMA-to-CPU transitions, independent successive data and targets, required shared-bus priorities, and memory regions that must remain free of Chip RAM contention.
    - **ROM overlay:** Use distinct synthetic ROM and RAM contents at the same low addresses. Let the guest change CIA-A PRA/DDRA through relevant aliases; inspect before/at/after commitment, input/output direction, both overlay states, resets and restored pending changes. Verify CPU mapping versus direct Agnus Chip RAM access and preservation of underlying RAM. Keep test code outside the switched window when required; do not let debugger loading silently remove the overlay being tested. No Kickstart/OS execution is required.
    - **Motherboard latches and exceptional accesses:** Verify observable address/data stability, byte-lane preservation and bus-direction/isolation effects across CPU/DMA handovers, plus the documented TAS behavior on Chip/Slow RAM and other targets. Test the modeled external contract without requiring a separate Gary/latch struct. Physical open-bus retention/decay and additional opposite-direction effects remain in 12.1; keep provisional timing distinct from hardware measurements.
    - **Reconstructed register effects and continuation:** Build the complete coverage map from the register/engine inventory and tests established in 2.4. Complete the per-register recipient, read-clear/strobe, pending/commit, retry and exactly-once checks through the same production execution path. Verify debug inspection is neutral, and compare uninterrupted execution against serialized/restored pending transfers and overlay changes. Record PAL/NTSC and preset applicability. Deferred BERR and external interrupt cases remain in Step 12.
    - Check sprite/display pixels in the frame buffer, Blitter destination data and completion, digital audio sequences, and relevant registers/memory. Preserve write-only, read-clear, and strobe semantics. Use the live GUI frame output completed in 2.4 for manual visual validation.
    - **Agnus–Denise synchronization and display DMA:** Verify automatic RGA sync strobes, receiver reactions and bitplane/sprite transfers through the production scheduler and bus into Denise latches and independently expected pixels/lines. Include supported CPU-initiated strobes, line/frame boundaries and competition with audio, Copper, Blitter and CPU accesses; delivery of a strobe or data word alone does not prove correct display behavior.
    - **Agnus–Paula synchronization and audio DMA:** Verify STREQU/STRVBL/STRHOR synchronization during blanked and active lines, STRLONG phase preservation, VERTB once per field, Paula DMAL requests, Agnus request capture/arbitration, RAM-to-AUDxDAT transfers and independently expected digital samples. Check successive data, pointer/count/reload transitions and interrupts, including pending-request enable changes and shared-bus competition. Trace the complete production exchange; manually supplying requests or audio data does not establish scheduler integration.
    - **Disk DMA without media:** Verify guest-configured requests, arbitration, actual Paula/Chip RAM transfers, pointer/count updates and completion with deterministic disk data. Include CPU contention and independently expected data; no image/flux-source emulation or firmware is required.
    - **Remaining guest coverage:** Complete the guest/output scenarios for the reconstructed engines, using independently expected readable values, memory and outputs after the owner applies each request.
    - **Disconnected serial and parallel ports:** Verify the 2.4 chip logic through guest accesses, including UART transmit progression/status/interrupts, CIA direction/readback and PC strobes, inactive external inputs, reset and continuation. Reuse the deterministic component receive/FLAG tests from 2.4. These checks do not establish communication with an attached device; that remains [12.4](#serial-parallel-device-support).
    - Include complete DMA paths using distinct successive data, applicable `WaitState` retries with exactly-once effects, shared-bus competition and simultaneous interrupt assertion/acknowledgment. Verify the full 2.4 feature inventory through applicable guest/output scenarios, including interlaced fields, collisions, all Blitter/audio modulation modes and all CIA modes. Exercise Copper/Blitter during active rendering and sprite, palette and mode changes within a line under modeled sampling/latch rules; exact cycle offsets and few-cycle transients are not required.
    - Record PAL/NTSC OCS and memory-preset applicability, run required common paths on both supported Agnus models, and add model-specific timing/geometry cases. Separate supported-model coverage from default PAL results.
    - Compare uninterrupted and restored execution for relevant pending/active operations within the reconstructed saved-state format; old-format compatibility is not required. Keep provisional model timing distinct from unknown silicon calibration; full vAmigaTS capture and lockstep work remain separate.
  - **Verification Gate:** Retained and corrected component tests pass, and executed ASM/BIN integration cases prove expected memory/device results, pixels, Blitter output and digital sample sequences, including the relevant waiting, grant, latch and overlay boundaries. Independently derived outputs and reproducible GUI/automatic execution are required; a nonempty buffer, register assignment or four introductory programs do not prove complete coverage. Record which layer establishes each contract and explain changed or removed coverage. Run affected memory, bus, machine and native/WASM checks and the existing roadmap closure profiles; preserve the final register-wide coverage requirement. See [MemoryBus](Obsidian/Amiga/Design/MemoryBus.md), [Physical Memory](Obsidian/Amiga/Design/Physical%20Memory.md), [DMA](Obsidian/Amiga/Design/DMA.md) and the [register matrix](Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md).
    - Pure CPU benchmarks, direct host writes or eventual readback alone do not establish production bus timing.
    - `python tools/verification/verify.py commit`
    - `python tools/verification/verify.py check architecture`

- **2.6: Lockstep Differential Tracer vs. vAmiga (Contingency)**
  - **Trigger:** Only if HRM-aligned tests (2.5) fail to isolate a regression — i.e. tests pass but behavior diverges from reference in ways not yet covered by the test suite.
  - **Objective:** Build a cycle-exact co-simulation harness that runs the same ROM/ADF through both this emulator and vAmiga in lockstep, dumps a full machine state snapshot at every CCK boundary, and reports the first divergence point with a structured diff.
  - **Actionable Scope:**
    - Add a `StepTracer` trait (or feature-gated callback) to `MachineLoop`: at each `step_cck()`, serialize `CpuState` + all chip register banks into a compact binary or JSON snapshot. All state structs already implement `serde::Serialize` — snapshot is essentially `serde_json::to_string(&machine.snapshot())`.
    - Wire vAmiga's existing headless state-dump mode (`ref_src/vAmiga`) to emit equivalent snapshots at the same CCK boundaries.
    - Harness (`tools/verification/lockstep_diff.py`): load both snapshot streams, walk them in parallel, and stop at the first CCK where any field diverges — output: `CCK #N | field | expected (vAmiga) | actual (ours)`.
    - Keep tracer behind a feature flag (`--features tracer`) so zero overhead in production builds.
  - **Verification Gate:**
    - Tracer successfully identifies the CCK and field of a known injected regression (synthetic test).
    - `python tools/verification/verify.py test rust crate test_runner suite test_lockstep_tracer` (smoke test against a trivial ROM loop).

- **2.7: Kickstart ROM & Floppy Subsystem Bring-Up for Native Program Execution**

  - **Objective:** When separately scheduled, bring up the native disk/Kickstart/OS path and its boot prerequisites; this work is deferred from the current plans. Load and execute genuine Amiga programs from disk images (`.adf`).
  - **Actionable Scope:**
    - Implement `DF0:` drive mechanics, MFM track deserializer, and DMA track streaming.
    - Wire Kickstart ROM overlay boot sequence ($000000 / $FC0000).
    - Establish end-to-end capability to boot ADF disk images as the foundational prerequisite for native test harnesses.
    - Verify guest OS startup, required services, program launching, and return behavior. Address firmware/media/model failures without substituting host-simulated OS initialization.
  - **Verification Gate:**
    - Unit tests in `crates/floppy/tests/`
    - Machine loop boot integration tests in `crates/machine_loop/tests/`

<a id="native-disk-verification"></a>

- **2.8: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** When separately scheduled after 2.7, execute native vAmigaTS using the verified disk/firmware/OS path, validating drive control, floppy DMA, CPU coordination, and required operating-system interactions.
  - **Actionable Scope:**
    - Execute disk-based vAmigaTS tests verifying that the emulator can run native Amiga software to test itself.
    - **Audio readiness:** Complete Paula audio/DMA simulation and digital sample generation in 2.4, with independently expected sample sequences checked in 2.5. Complete the basic host-playback portion of 6.1 before or during 2.8, before running its audio scenarios, so their sound can be heard as well as checked automatically. Keep native/WASM frontend audio boundaries; later CRT shaders do not gate these tests.
    - **Host input readiness:** Complete the basic keyboard, mouse and joystick/gamepad routing from 6.2 before or during 2.8, before executing tests that require those inputs. Identify each selected test's required controls and verify host key presses/releases, mouse motion/buttons and joystick directions/fire through the frontend boundary into the emulated devices and guest-visible behavior. Preserve native/WASM separation and core ownership. Deterministic inputs remain sufficient for 2.5; advanced controller features and the deferred 12.2 peripherals are not prerequisites for ordinary input tests. See the [GUI specification](Obsidian/Amiga/Design/Application/GUI.md).
    - Leverage unthrottled headless execution (running as fast as the host CPU permits without wall-clock rate limiting) for maximum throughput.
  - **Verification Gate:**
    - Automated test run reports for disk-based test suites passing with zero unexpected halts.
    - Audio scenarios require functioning emulated sample generation, digital output checks, and a verified host playback path; do not postpone these prerequisites until after 2.8.
    - Input-dependent scenarios require verified host-to-guest keyboard, mouse and joystick paths, including release/neutral transitions. Confirm the guest receives and reacts to the required inputs; UI event capture alone does not pass this gate.

---

<a id="debugger-execution-control"></a>

## Step 3: Developer Studio GUI & Diagnostic Tooling Hardening (Ongoing Companion Track)

- **3.1: Continuous Testing & Refinement of Developer Studio (Debugger View)**
  - **Objective:** Actively test, harden, and refine the Developer Studio GUI (`crates/gui`) under live emulation across all display tiers and interactive workflows.
  - **Actionable Scope:**
    - Test interactive panels under live stepping: Disassembly infinite stream browsing, in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
    - **Power LED indicator:** Display the emulated power LED in the GUI using the machine's LED state. Keep the indicator current during running, stepping, reset and save-state restoration.
    - Test upcoming Save State management (`State` menu, in-memory quick slots 1–5, `F6`/`F9` shortcuts, and native file dialogs).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate gui suite test_interactions` (all headless egui interaction tests passing).

- **3.2: Developer Studio & Debugger Keyboard Shortcuts Documentation & Discoverability**
  - **Objective:** Complete audit, documentation, and user-facing surfacing of all implemented keyboard shortcuts across Developer Studio and Game View modes, ensuring zero hidden keys.
  - **Actionable Scope:**
    - Implement a dedicated in-app Keyboard Shortcuts Reference Modal (`?` / `F1` / Top Menu Help > Shortcuts).
    - Enrich button hover tooltips with contextual shortcut cues.
    - Surface all shortcuts: View Modes (`F12`/`F2`, `Escape`), Stepping Controls (`F5`/`Space`, `F10`, `Shift + F10`, `F11`), Tooling & Machine Controls (`F8`, `Alt + T`, `Ctrl + R`, `Ctrl + O`), Save States (`F6`, `F9`, `Ctrl + S`, `Ctrl + L`), Display Zoom (`Ctrl + +/=`, `Ctrl + -`, `Ctrl + 0`), and Hex/Disassembly editing navigation.
  - **Verification Gate:**
    - Headless integration test in `crates/gui/tests/` verifying modal dialog trigger and shortcut handling.

- **3.3: Watchpoint Execution Stops**
  - UI/backend watchpoint definitions and matching exist, but guest memory accesses do not invoke matching to pause execution. Route actual guest accesses to it while keeping debugger inspection neutral.
  - **Gate:** Verify matching access types/ranges and actual execution stops. See [Debugger](Obsidian/Amiga/Design/Application/Debugger.md).

- **3.4: Full-Machine Rewind and Differential State**
  - Current history restores CPU state only, not RAM or other chips. Implement deterministic full-machine rewind: record base/target CCKs, changed subsystem values and reversible physical memory writes; apply/rewind all changed state only from a matching base/target CCK. The differential `A500DiffState` remains a design proposal until reviewed and implemented.
  - **Format proposal for review:** `DIFF` magic (4 B), base CCK (8 B), delta CCK (2 B), subsystem flags (1 B), variable subsystem payloads, memory count (2 B), then entries `[address: u24, new: u16, old: u16]` (7 B each). The illustrative 22 B/no-write and 29 B/one-write footprints depend on which fields change; this is not an implemented format or a saved-data migration.
  - **Gate:** Review the format before implementation and prove all-state restoration plus deterministic full-machine forward/backward continuation. See [Save states](Obsidian/Amiga/Design/Application/SaveState.md).

---

<a id="external-hardware-verification"></a>

## Step 4: vAmigaTS Silicon Verification Sub-Suites & Contention Track (After Chipset Bring-Up)

- **4.1: Test Suite Categorization, Filtering & Deferred Execution Matrix (609 Deferred Tests)**
  - **Objective:** Enforce strict gating and categorization for tests requiring hardware beyond the baseline A500 OCS configuration.
  - **Actionable Scope:**
    - Maintain and audit formal gate categories: FPU Coprocessors (206 tests deferred to the scope decision in Step 10.2), ECS & AGA Silicon (112 tests deferred to Steps 10.1 and 10.2), 68010 CPU Architecture (91 tests deferred to Step 10.2), AmigaOS Floppy MFM Bootblock (7 tests deferred to Step 8.1), and Non-Visual Register Assertions (193 tests deferred to Step 4.2.6).
    - A future capture entry point must classify supported and deferred tests before executing baseline OCS suites; the previous runner has been removed.
  - **Verification Gate:**
    - Demonstrate supported/deferred selection through the future capture entry point before executing its baseline; no current Cargo suite provides this gate.

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

- **4.3: Dynamic Agnus DMA Slot Arbitration Optimization (Deferred Performance Track)**
  - **Objective:** Optimize the dynamic per-CCK DMA slot priority evaluation into a clean, pre-computed scanline slot lookup table (`[DmaSlot; 227]`).
  - **Actionable Scope:**
    - Localize optimization strictly within the Agnus DMA scheduler (`crates/agnus/src/dma.rs`).
    - Preserve code simplicity and readability, with zero complex asynchronous event wheels or timing skips.
    - Once a representative whole-machine workload is executable, compare fresh throughput before and after the optimization; CPU-only benchmarks do not measure Agnus scheduling.
  - **Verification Gate:**
    - Zero regression across all active baseline OCS vAmigaTS suites.
    - Run DMA, Agnus and production memory_bus regression tests, including contention and pending CPU-request arbitration; CPU-only waiting tests do not validate scheduler changes.

---

## Step 5: Custom Chipset Debugger & Deep Architectural Observability (Developer Studio Extension)

- **5.1: Custom Chipset Registers & Mutation Delay Pipeline Inspector**
  - **Objective:** Implement dedicated register inspection docks/tabs in Developer Studio (`crates/gui`) with live delta highlighting and mutation delay observability.
  - **Actionable Scope:**
    - Dedicated docks for Agnus, Denise, Paula, CIAs (A & B), and RTC.
    - Format values in hex/binary with electric cyan change/delta highlighting.
    - Bitfield breakdown widgets and interactive tooltips for control/status registers (`DMACON`, `INTENA`, `INTREQ`, `BPLCON0`-`BPLCON2`, `ADKCON`, `CRA`/`CRB`).
    - Visualize staged pipeline register latches taking effect across Color Clock phases ($CCK1 \to CCK2$).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying register dock rendering and delta tracking.

- **5.2: Agnus DMA Slot Scheduler & Real-Time Bus Allocation Visualizer**
  - **Objective:** Build a horizontal scanline timeline visualizer displaying the 227 Color Clock slots per scanline (PAL) / 226 slots (NTSC).
  - **Actionable Scope:**
    - Fixed allocations: DRAM refresh (CCK 0..3), Disk DMA (CCK 4), Audio DMA 0–3 (CCK 5..8), Sprite pairs 0–7 (CCK 12..27).
    - Dynamic allocations: Bitplane DMA (BPL 1–6), Blitter DMA (channels A, B, C, D), and CPU bus slots.
    - Live beam position cursor tracking horizontal ($HPOS$) and vertical ($VPOS$) raster coordinates.
    - Bus contention & wait-state indicator visually flagging cycles where CPU or Copper are stalled (BLTPRI / Blitter Nasty).
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying timeline visualizer rendering.

- **5.3: Copper Coprocessor Inspector & Real-Time Execution Tracker**
  - **Objective:** Implement a dedicated Copper list disassembler panel decoding instruction streams directly from Chip RAM with real-time execution tracking.
  - **Actionable Scope:**
    - Decode instruction streams (`MOVE`, `WAIT`, `SKIP`) from Chip RAM pointers (`COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`).
    - Real-time execution pointer tracking: highlight executing Copper instruction, pending wait condition ($VPOS$/$HPOS$ comparison), and CDANG danger mode status.
    - Visual correlation with raster beam: highlight beam position where Copper interrupts or register modifications trigger palette swaps or Blitter dispatches.
  - **Verification Gate:**
    - Headless integration tests in `crates/gui/tests/` verifying Copper disassembler panel.

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

## Step 6: Host I/O Peripherals, Audio Playback & Controller Hub

- **6.1: Host Audio Playback & CRT Presentation Shaders**
  - **Objective:** Operationalize host real-time audio output streaming and authentic CRT display presentation.
  - **Execution order:** Complete basic audio playback before or during 2.8, before its audio scenarios run. Keep later playback refinements and CRT shaders in this track; do not delay the basic sound path until after external-suite bring-up.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/paula/tests/`.
    - Headless shader initialization and rendering verification.

- **6.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/game_ports/src/mouse.rs`, `crates/game_ports/src/joystick.rs`, `crates/game_ports`, `crates/gui`).
  - **Execution order:** Complete basic keyboard, mouse and joystick/gamepad routing before or during 2.8, before its input-dependent scenarios run. Keep extended controller protocols, mapping options and input UI refinements in this track; do not defer the basic host-to-guest input path until after external-suite bring-up.
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Dual Game Port Hub:** Slot assignment for Port 1 & Port 2 (Mouse, Joystick, CD32 Pad, Keyboard-Joystick, Disconnected), simultaneous dual-mouse and dual-joystick modes, hardware button routing (CIA-A `PRA`, Paula `POTGOR`), and CD32 7-button shift register protocol.
    - **Physical Host Mouse Capture:** Viewport mouse grab/lock mode, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), and sensitivity tuning.
    - **Physical Gamepad & Joystick Ingestion:** Host gamepad enumeration via `gilrs` (native desktop) and HTML5 Gamepad API (WASM), analog deadzones, and button mapping.
    - **Developer Studio Input UI:** Interactive "Game Ports & Input" tab/dock displaying port status, device dropdowns, and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/` and `crates/game_ports/tests/`.
    - Headless UI input tests in `crates/gui/tests/`.

---

## Step 7: Dedicated Player GUI & Frontend Experience

- **7.1: Hardware Configuration & Kickstart ROM Selector**
  - **Objective:** Deliver user-friendly Amiga hardware profile selection and Kickstart ROM management.
  - **Actionable Scope:**
    - Profile selector: Basic A500 512 KB, Classic A500 1 MB (Recommended), Expanded A500 4 MB.
    - Reset warning confirmation modal informing user that changing hardware configuration requires a cold machine reset.
    - Kickstart ROM manager: file picker for ROM images (1.2, 1.3, custom) with automatic checksum validation (CRC32/SHA-256).
  - **Verification Gate:**
    - Headless UI tests in `crates/gui/tests/` for profile selection and ROM validation modals.

- **7.2: Multi-Drive Floppy Disk Manager (`DF0:` – `DF3:`)**
  - **Objective:** Visual floppy drive slot manager with live drive activity indicators and disk insertion controls.
  - **Actionable Scope:**
    - Drive slot manager displaying primary internal drive `DF0:` and optional external drives (`DF1:`–`DF3:`).
    - Per-drive enable/disable toggles to mount or disconnect external floppy drives on the fly.
    - ADF file picker per drive with quick insert, eject, and write-protect latch controls.
    - Visual drive activity indicators and drive stepping audio feedback.
  - **Verification Gate:**
    - Unit tests for multi-drive management and headless UI interaction tests.

- **7.3: Visual Save State Manager (Screenshots, Timestamps & Custom Labels)**
  - **Objective:** Provide an interactive visual save/load state overlay and slot manager with screenshot previews and configuration guards.
  - **Actionable Scope:**
    - Visual snapshot cards containing embedded thumbnail screenshots captured at the moment of saving.
    - Formatted creation date and time timestamps and user-defined state labels/descriptions.
    - Configuration integrity guard verifying matching hardware profiles before restoring state to prevent guest crashes.
    - Preserve the active CPU execution path when restoring a snapshot during a redirected branch or exception; opcode-only slice reconstruction does not establish this. Gate with interrupted execution and identical continuation after serialization.
  - **Verification Gate:**
    - Save state serialization unit tests and UI state manager tests in `crates/gui/tests/`.

---

## Step 8: Real-World Amiga Workloads & Pipeline Optimization (Post-Boot)

- **8.1: Deferred Full-OS & MFM Floppy vAmigaTS Test Suites Verification Gate**
  - **Objective:** Verify physical floppy track MFM streaming and full Kickstart 1.3 bootstrap test suites.
  - **Actionable Scope:**
    - Execute the 9 non-standard test ADFs using physical floppy MFM track streaming via `crates/floppy`.
    - Unfilter and pass all vAmigaTS tests requiring full Kickstart 1.3 bootstrap (`dos.library`, `intuition.library`, and filesystem calls).
  - **Verification Gate:**
    - vAmigaTS test runner passing all MFM and OS-level tests.

- **8.2: End-to-End Bootable ADF Integration Testing**
  - **Objective:** Boot and execute genuine Amiga benchmarks and diagnostic suites directly from floppy disk images (`python tools/verification/verify.py test rust crate test_runner suite test_boot_adf`).
  - **Actionable Scope:**
    - Load and execute `AmigaTestKit.adf`, `SysInfo.adf`, and Dhrystone on the authentic Kickstart / Amiga chipset stack.
    - Leverage address guards, breakpoint traps, and instruction bounds for parameterized termination.
    - Assert functional correctness, numerical determinism, and measure baseline throughput (MIPS, host CPU cycle cost per emulated CCK).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate test_runner suite test_boot_adf` passing cleanly.

- **8.3: CPU Core Memory Footprint Audit & Compaction Assessment**
  - **Objective:** Audit the exact memory footprint of the CPU core and assess compaction strategies.
  - **Actionable Scope:**
    - Audit host `.rodata` footprint (sizeof(OpcodeDescriptor) * 65,536, microcode arrays) and runtime state footprint (`Cpu`, `CpuState`, `CpuMicroState`).
    - Compare whether bit-packing or deduplication improves throughput on representative workloads, or whether the flat layout performs better.
    - Refactor slow handlers using host efficiency principles without custom macros or const-generics.
  - **Verification Gate:**
    - Memory footprint audit report and SingleStepTests passing with zero regressions.

- **8.4: Cross-Architecture & Mobile Performance Projections**
  - **Objective:** Extrapolate measured throughput to target mobile and constrained environments (e.g. mobile WebAssembly, ARM mobile).
  - **Actionable Scope:**
    - Model CPU overhead margins to ensure sustained 50 Hz (PAL) / 60 Hz (NTSC) cycle-exact emulation with full chipset contention and rendering.
  - **Verification Gate:**
    - Documented performance analysis projections in `Obsidian/Amiga/Design/`.

- **8.5: Fidelity & Regression Validation Gate**
  - **Objective:** Final regression certification across all optimization and refactoring phases.
  - **Actionable Scope:**
    - Ensure every optimized handler retains 100% cycle-exact Color Clock fidelity.
    - Pass the full exhaustive SingleStepTests suite ($env:SINGLESTEP_FULL = "1") and full workspace unit/integration test suites.
  - **Verification Gate:**
    - `python tools/verification/verify.py test runner singlestep full`
    - `cargo test --workspace`

---

## Step 9: Documentation-to-Code Regeneration Experiment

- **9.1: Isolated Documentation-to-Code Regeneration Trial**
  - In an isolated checkout, remove one selected subsystem and regenerate it from curated specs, rules, and test vectors. Compare behavior and source with the preserved original; use failures to refine documentation and instructions.
  - **Gate:** The regenerated subsystem passes its unit and integration suites without regressions.

## Step 10: Later Hardware and Diagnostic Extensions

- **10.1: ECS and Later A500 Models**
  - Add A500 Rev 6A 1 MB Chip RAM with Agnus 8372A, then A500 Plus with ECS Agnus/Denise, Productivity modes, Kickstart 2.04, and battery-backed RTC.
  - **Gate:** Enable and verify the ECS suites deferred by Step 4.1, including `_ecs.raw`, `_plus.raw`, SuperHires, and expanded Agnus registers.

- **10.2: AGA, Later CPUs, and Optional Coprocessors**
  - Add A1200 support with 68EC020, 2 MB Chip RAM, Alice/Lisa, eight bitplanes, and a 24-bit palette. Treat 68010 and FPU support as separate scope decisions tied to their deferred suites.
  - **Gate:** Enable and verify the relevant later-CPU, AGA, and selected coprocessor suites from Step 4.1 when their hardware prerequisites exist.

- **10.3: Resource Extractor and Reverse Engineering Assistant**
  - Extract bitplanes, sprites, palettes, and audio from guest memory; annotate assets, addresses, and game phases with optional LLM assistance and map them in the 24-bit address space.
  - **Gate:** Asset extraction unit tests in `crates/debugger/tests/` verify output and source-address provenance.

## Step 11: Four-Manual PDF-to-Book JSON Conversion

- **11.1: Convert Four Source PDFs to Reviewed `book.json` Recipes**
  - Prepare the A500/A2000 Technical Reference Manual, Amiga Hardware Reference Manual (second edition), M68000 Programmer's Reference Manual, and M68000 User's Manual (Revision 8). Keep a separate source identity and preparation profile for each PDF.
  - Import each prepared PDF into Book-to-Markdown, then adjust chapter organization, source selections, formatting, and graphics against the source. Accept the intended revision to produce each manual's `book.json`. Use the [book authoring workflow](tools/bootstrap/book-to-markdown/README.md#book-authoring-sequence).
  - **Gate:** Review and commit all four accepted `book.json` recipes with their matching full source hashes in `source.json`. A prepared PDF or unaccepted Working bundle is insufficient.

<a id="deferred-bus-and-peripherals"></a>

## Step 12: Deferred Bus Fidelity, Peripheral Extensions and CPU Interrupt Cases

- **12.1: Physical Open-Bus Values and Opposite-Direction Accesses**
  - The current model returns fixed `$FF/$FFFF`, with no retained motherboard data-bus latch or complete opposite-direction disturbances. Investigate whether an undriven read returns that fixed value, retains the previously driven value, or changes toward `$FF/$FFFF` over time. Treat decay as a hypothesis requiring address-, phase-, bus-lane- and board-specific evidence, not an assumed hardware rule.
  - Include dummy/unused/opposite-direction reads and writes and their possible disturbances. Obtain the evidence before proposing changes; preserve the current approximation until a behavior/reference change is approved.
  - **Missing external Bus Error (Vector 2):** `AddressBus` currently exposes only `Ready` and `WaitState`, so it cannot deliver BERR. Treat this as a low-priority completeness gap, probably unnecessary for the current A500 scope, rather than a prerequisite for chipset reconstruction or testing. Revisit only if a concrete workload or supported hardware requirement needs it. The implemented Address Error on odd word/long addresses (Vector 3) is separate and remains supported. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate:** Record hardware-supported values, timing and access effects, then follow specification approval before implementing and testing a replacement. See the [platform invariants](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access).

- **12.2: Four-Player Joystick Adapter and Analog Proportional Inputs**
  - Implement the optional four-player adapter only when separately scheduled, after the higher-priority emulator and debugger work. Keep ordinary two-port mouse/joystick support in its existing stages.
  - Implement analog proportional inputs with potentiometer sampling through POT0DAT/POT1DAT in the same deferred peripheral track.
  - **Gate:** Configuration, device behavior and integration tests verify the adapter and analog inputs against their hardware specifications. See [Configuration](Obsidian/Amiga/Design/Configuration.md).

<a id="optional-cpu-interrupts"></a>

- **12.3: Optional Expansion Interrupt Sources and CPU Interrupt Cases**
  - **Expansion INT2/INT6:** When an expansion device is separately scheduled, route its active-low requests from the A500 side expansion connector (86 contacts, INT2 on contact 19 and INT6 on contact 22) to Paula alongside the CIA-A/CIA-B sources. Resolve shared levels so releasing one source cannot release another asserted source; retain released inputs with no expansion attached. Keep this connector support outside current chipset reconstruction.
  - **Level 7 NMI / autovector 31:** The current emulator ignores IPL=7. Standard Paula sources generate levels 1-6, but real A500 expansion hardware can drive the CPU interrupt lines to request level 7. Record this missing external-input support and consider implementing it only when such hardware or a concrete workload is in scope, including the CPU's special level-seven acceptance behavior.
  - **Uninitialized interrupt / vector 15:** The current bus interface has no device-supplied interrupt-vector response. Built-in A500 interrupts use autovectors, so there is no standard onboard source for this case. Consider support only if a selected external device or a broader CPU-interface requirement needs it; distinguish an uninitialized peripheral vector register from an absent handler in RAM.
  - **Spurious interrupt / vector 24:** The current interface cannot report BERR during interrupt acknowledgment. Standard A500 interrupts have no identified normal source for this case. Consider implementation only for a concrete external-device or bus-fault scenario, together with the deferred BERR interface in 12.1.
  - Keep these external-input cases as low-priority completeness gaps, outside the prerequisites for current chipset reconstruction and testing. Record the required hardware/input scope before implementation and preserve existing supported exceptions. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate if scheduled:** Reference-derived tests verify the selected input, interrupt acknowledgment or vector selection, exception frame, transaction/wait behavior and continuation. Do not treat recording these gaps as implemented support.

<a id="serial-parallel-device-support"></a>

- **12.4: Attached Serial and Parallel Devices and Communication Verification**
  - The current model assumes both connectors remain disconnected. There is no emulated attached serial or parallel device, connector transport or host adapter establishing a working connection. The reconstructed chip logic assumes disconnected inputs; its component tests and Step 2.5 disconnected-port checks do not prove that an attached device can communicate.
  - When separately scheduled, select a deterministic attached device or loopback model and implement the required connector routing. Verify serial transmit/receive and applicable control lines, plus parallel data, PC/strobe, ACK and status, including delayed/blocked responses, interrupts, reset and restored continuation. Physical host-port adapters require their own explicit scope.
  - **Gate if scheduled:** Guest-driven exchanges through the production machine path produce independently expected data and handshake/status transitions at the selected device. Keep connection and full-exchange support explicitly unverified until these tests pass. See [Paula](Obsidian/Amiga/Design/Paula.md) and [CIA](Obsidian/Amiga/Design/CIA.md).

- **12.5: Light Pen and Light Gun Input**
  - When separately scheduled, implement the selected game-port trigger and motherboard LPEN route to Agnus, beam-position latching and guest-visible readback. Keep this extension outside the current chipset reconstruction scope.
  - **Gate if scheduled:** Reference-derived integration tests verify trigger delivery and captured beam-position readback against the selected device's hardware specification.

- **12.6: Genlock and External Video Synchronization**
  - When separately scheduled, implement external HSYNC/VSYNC delivery from the 23-pin RGB video connector (contacts 11/12) to Agnus, the BPLCON0.ERSY input-direction control and the resulting beam synchronization. Establish the selected genlock device's external clock requirements from its hardware specification.
  - **Gate if scheduled:** Reference-derived tests verify external signal delivery, direction changes and beam synchronization, including reset and restored continuation. Ordinary internally generated video must remain supported without an attached genlock.
