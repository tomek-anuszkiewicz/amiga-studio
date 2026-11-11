# Amiga 500 Emulator: Development Roadmap

## Step 2: HRM-Aligned Custom Chipset Reconstruction

Current execution scope through 2.5 and the final propagation audit excludes
vAmigaTS execution (including adapted payloads), Kickstart/AmigaOS integration,
and floppy-media reads/writes. Use directly loaded authored assembly programs
and deterministic pin/latch inputs. ROM overlay checks use synthetic ROM bytes
without executing firmware. Later firmware, media, and external-suite work is
unscheduled and must not start automatically after the current plans.

- **2.2: Model Old Gary Address Decoding on the A500 (`MEM-GARY-01`)**
  - In every supported A500 preset, route banks `$A0-$BF` to the CIA decoder. A13/A12 are active-low selects: `00` selects both CIAs, `01` CIA-B, `10` CIA-A, and `11` neither. A11..A8 select the register; A7..A1 mirror it. `$BFC000` is one address in the `$BFC000-$BFCFFF` selected block, and the same selection pattern repeats in other mapped banks. CIA register selector `B` remains unconnected.
  - With both CIAs selected, an even byte read uses CIA-B and an odd byte read CIA-A; a word read puts CIA-B in the high byte and CIA-A in the low byte. A byte write sends the same value to every selected CIA regardless of parity; a word write sends the high byte to selected CIA-B and the low byte to selected CIA-A. Pass full register identities to the existing CIA recipients and preserve CIA-A Port A overlay effects on every alias. Retain the documented fixed placeholder for an inactive read lane rather than assuming a physical data-bus latch.
  - Route `$DE` and `$DF` through the same custom-register decoder, including its low-offset mirror, byte/word behavior, delayed mutations, side effects, and side-effect-free debug reads. Preserve installed Slow RAM and RTC precedence; evaluate other model-dependent mirrors separately. Update [MemoryBus](Obsidian/Amiga/Design/MemoryBus.md), [Physical Memory](Obsidian/Amiga/Design/Physical%20Memory.md), [CIA](Obsidian/Amiga/Design/CIA.md), and the [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) together with the code. Recheck the HRM wording; WinUAE `cia.cpp`/`memory.cpp` and `ref_src/vAmiga/Core/Components/Memory/Memory.cpp` corroborate the decode but do not prove physical A500 wiring without schematic or measured evidence.
  - Verify acceptance and retry behavior separately from address selection and delayed effects. Record common and model-specific coverage for both supported PAL/NTSC OCS models and applicable memory presets.
  - **Gate:** First reproduce failures against the original router and bank table. Focused `memory_bus` and `physical_memory` tests cover dual selection, byte/word lanes and writes, nonzero register selectors, low-bit and `$A0-$BE` aliases, inactive selection, overlay effects, `$DE` reads/writes/debug reads, and Fast/Slow RAM and RTC boundaries. Then pass adjacent bus and machine integration checks.

- **2.3: Verify Inter-Chip Signal Routing and Propagation**
  - **Objective:** Verify and repair connections from chip outputs through motherboard routing to recipient inputs at the required CCK phase. Keep rules, skills, workflows, and design documentation consistent with the hardware-supported behavior. Use the register access and commit behavior established in Step 2.1 as the starting point.
  - **Mandatory order:** First review A500 wiring, hardware books, vAmiga, Minimig, and the Amiga replacement FPGA project, including keyboard, game ports, floppy, serial, parallel, and host/media boundaries. Then implement and verify motherboard signal routing and electrical propagation. Only after that start the book-driven chipset reconstruction in Step 2.4.
  - **Actionable Scope:**
    - Compare A500 wiring and the [cross-chip signals catalog](Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) with `memory_bus.rs` and `machine_loop.rs`. Trace register-bus strobes, DMA requests and grants, interrupts, sync, reset, and peripheral pins from source output through machine routing to recipient input at the required CCK phase; resolve missing, reversed, or direct peer connections against hardware evidence.
    - **Transferred from 2.1:** Implement and verify STREQU/STRVBL/STRHOR/STRLONG generation and delivery from Agnus refresh/RGA through Denise and, for STRHOR, Paula. Verify capture phase and recipient sampling; assign absent engine reactions explicitly to 2.4 with functional tests in 2.4/2.5. Verify CIA connector routes/protocols and mouse/joystick signal delivery using existing working interfaces or deterministic endpoints; 2.1 does not supply repaired pin/counter semantics. Review signal-route dependencies of the register effects reconstructed in 2.4. These are pending 2.3 requirements, not prerequisites for 2.1 closure.
    - **Propagation and ordering:** Verify output polling, recipient input updates, and downstream reactions, including interrupt priority, blitter busy, disk DMA, and Copper WAIT/SKIP. Distinguish the register-write staging and commit checked in Step 2.1 from the resulting signal's delivery to another chip; reuse that evidence instead of repeating the register inventory.
    - **Propagation substrate:** Implement scheduled output delivery and recipient sampling with explicit phase ordering, serializable pending signals, deterministic resume, and before/due/after boundary checks. Keep register commitment distinct from subsequent signal delivery and avoid double delay. Mark provisional delays separately from measured hardware values.
    - **Complete transfers and interaction:** Follow each in-scope DMA stream from request or scheduled opportunity through arbitration/grant, address/data transfer, receiving latch, and pointer/count update. Verify applicable bus retries, competing CPU/display/audio/Copper/Blitter activity, active configuration changes, and simultaneous interrupt assertion/acknowledgment. Record state ownership and PAL/NTSC-specific phase behavior. Inspect disk-media architecture as reference evidence only; restrict current disk checks to register/pin propagation with deterministic inputs, without drive/media transfers.
    - **Documentation:** Verify that `hardware-bus-topology.md`, relevant design specs, and `MachineLoop` documentation describe both register-write propagation (e.g. `COLOR00`, `BPLCON0`, `INTENA`) and inter-chip signal delivery with the established clock-phase ordering. Update affected governance guidance and missing sections. Resolve implementation/documentation conflicts against hardware evidence, and keep unverified timing explicit.
  - **Verification Gate:**
    - Record each signal's source, motherboard route, recipient, delivery phase, and observable effect. Focused machine integration tests cover distinct signal paths and ordering, including missing, reversed, or direct peer connections; reuse Step 2.1 evidence where coverage overlaps.
    - Explicitly cover all four transferred sync strobes and both STRHOR recipients, CIA/peripheral direction and return paths, and game-port signal delivery. Queue insertion alone does not pass this gate; any missing engine reaction must have a concrete 2.4 handoff rather than being discarded.
    - Establish transport and phase correctness using working paths or deterministic test endpoints. Record absent in-scope engine/device behavior and hand its full-exchange tests to Step 2.4; keep disk-media and firmware execution deferred; endpoint simulation does not prove that behavior.
    - Run the scoped specification review and [vault link/frontmatter audit](.agents/skills/obsidian-vault-linking/SKILL.md); the milestone command does not execute documentation maintenance. At point closure, run commit and milestone profiles at their existing triggers.
    - The intended commit changes were reviewed against documents selected by `tracked_paths`; real corrections are included, and accurate documents remain untouched.

- **2.4: Reconstruct Custom Chipset Behavior from Book Specifications**
  - **Objective:** Replace existing behavioral logic in `crates/machine_loop` and the custom chip subsystems (`agnus`, `denise`, `paula`, `cia`, `copper`, `blitter`, `floppy` and their affected children) with behavior derived from the hardware books. Preserve the established CPU/storage boundaries, address decoding, direct register interfaces, clocking, propagation, serialization, and supported hardware invariants.
  - **Prerequisite:** Complete Step 2.3's three-reference review and implementation/verification of signal propagation before replacing chipset behavioral engines. Reuse that substrate and its phase contracts; complete the device/engine behavior and full-exchange tests handed over by the review.
  - **Actionable Scope:**
    - Establish book-derived behavior contracts with exact source sections, supported models, state transitions, reset behavior, outputs, and explicit timing unknowns. Replace obsolete mechanisms and affected callers coherently; retain useful regression protection.
    - Identify one authoritative owner for each pointer, counter, latch, and pending event, separating programmed reload values from active engine state. Audit apparent duplicate DMA address state, including audio fields, against Agnus ownership and actual consumers.
    - **Transferred register behavior:** Reconstruct and verify register-induced actions, side effects and computed read values beyond 2.1's owner-delivery boundary. Explicitly repair CIA PRA/PRB latch versus sampled-pin semantics, JOYTEST counter ownership/effects and persistence through polling, VPOSW/VHPOSW beam changes/readback, and required REFPTR/COPINS internal effects. Complete sync/peripheral engine reactions handed over by 2.3. Preserve documented access/queue contracts and already working effects such as OVL. Acknowledged writes or consumed placeholders do not prove functional behavior; add focused tests here and complete guest write/read/output scenarios in 2.5.
    - Reconstruct complete in-scope DMA transfers and relevant active-state transitions in dependency order: bus/clocks and arbitration, autonomous DMA/Copper/Blitter, display/sprites, then peripherals/audio/CIA. Verify enable/disable, restart, parameter-update, interrupt acknowledgment, and reset boundaries according to each mechanism's contract.
    - Resolve conflicts among books, approved specifications, measurements, and existing behavior under [specification compliance](.agents/rules/spec-compliance.md). Preserve documented quirks and invariants unless an explicit change is approved; do not leave required functionality inert or call provisional delays measured hardware timing.
  - **Verification Gate:**
    - Book-derived behavioral and production-machine integration tests demonstrate actual data, output, state transitions, and reset behavior for reconstructed subsystems, including applicable PAL/NTSC differences. Register assignments and a successful build alone do not establish completion.
    - The named CIA port, JOYTEST, beam-write, REFPTR/COPINS and 2.3 functional handoffs have reference-supported contracts and executed tests of required behavior, resulting read values and side effects. Any reference-supported non-action must be explained explicitly rather than inferred from the current empty handler.
    - `python tools/verification/verify.py check build`
    - `python tools/verification/verify.py check architecture`
    - Run relevant native/WASM and subsystem checks, plus the existing commit and milestone closure procedures at their required triggers.

- **2.5: Book-Derived Guest Programs and Behavioral Test Suites**
  - **Objective:** Derive small executable M68000 scenarios from hardware-book descriptions of supported chip behavior. Load the same program/data artifacts through the GUI and automated production-machine tests, with independent expected pixels, digital audio samples, memory, and register/state results.
  - **Actionable Scope:**
    - Reuse useful isolated tests and shared test-side helpers. Each authored scenario has its own directory containing `test.rs`, `program.as`, and the compiled `program.bin`, book provenance, documented initial conditions, and bounded direct-load execution; guest code performs the operation under test rather than host code pre-filling its result.
    - Check sprite/display pixels in the frame buffer, Blitter destination data and completion, digital audio sequences, and relevant registers/memory. Preserve write-only, read-clear, and strobe semantics. Connect the existing GUI viewport to real frame output before claiming manual visual validation.
    - **Register functional verification transferred from 2.1:** Use guest accesses to verify the reconstructed effects, readable counterparts and side effects, including CIA port direction/input reads, JOYTEST counter writes and later polling/movement, beam writes/readback and applicable Blitter/Copper/sync actions. Include REFPTR/COPINS where their reference-supported access permits such a scenario. Verify what happens inside and after the receiving engine; do not treat 2.1's queue-delivery evidence as proof of these results.
    - Include complete DMA paths using distinct successive data, applicable `WaitState` retries with exactly-once effects, shared-bus competition, active configuration changes, and simultaneous interrupt assertion/acknowledgment. Exercise the interactions being tested rather than restricting every scenario to an idle bus or hblank.
    - Record PAL/NTSC OCS and memory-preset applicability, run required common paths on both supported Agnus models, and add model-specific timing/geometry cases. Separate supported-model coverage from default PAL results.
    - Compare uninterrupted and restored execution for relevant pending/active operations. Keep provisional model timing distinct from unknown silicon calibration; full vAmigaTS capture and lockstep work remain separate.
  - **Verification Gate:**
    - Each agreed behavior has independently derived output/state assertions and reproducible GUI/automatic program execution. Nonempty buffers, register assignments, or a few introductory programs alone do not establish complete coverage.
    - `python tools/verification/verify.py commit`
    - `python tools/verification/verify.py check architecture`
    - Run affected program, chip, machine, debugger/GUI and native/WASM checks. Tests cover relevant boundaries, interactions, and failures under the testing policy; apply existing roadmap-point closure gates when the full scope is verified.

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

- **2.8: vAmigaTS Native Disk-Based Verification & Self-Testing**
  - **Objective:** When separately scheduled after 2.7, execute native vAmigaTS using the verified disk/firmware/OS path, validating drive control, floppy DMA, CPU coordination, and required operating-system interactions.
  - **Actionable Scope:**
    - Execute disk-based vAmigaTS tests verifying that the emulator can run native Amiga software to test itself.
    - Leverage unthrottled headless execution (running as fast as the host CPU permits without wall-clock rate limiting) for maximum throughput.
  - **Verification Gate:**
    - Automated test run reports for disk-based test suites passing with zero unexpected halts.

- **2.9: Principled, Rule-Compliant Custom Chip & Register Verification**
  - **Objective:** Advance custom chips and registers systematically from clean baseline specifications to fully validated cycle-exact implementations.
  - **Actionable Scope:**
    - Implement and calibrate custom chip features following the substrate-first causality chain (Layer 0 $\to$ Layer 4).
    - Strictly adhere to defect resolution (`.agents/rules/structural-root-cause.md`) and 4-tier integration archetypes (`.agents/rules/unit-testing-policy.md`).
  - **Verification Gate:**
    - `python tools/verification/verify.py test rust crate machine_loop`
    - `python tools/verification/verify.py test rust integration`

---

## Step 3: Developer Studio GUI & Diagnostic Tooling Hardening (Ongoing Companion Track)

- **3.1: Continuous Testing & Refinement of Developer Studio (Debugger View)**
  - **Objective:** Actively test, harden, and refine the Developer Studio GUI (`crates/gui`) under live emulation across all display tiers and interactive workflows.
  - **Actionable Scope:**
    - Test interactive panels under live stepping: Disassembly infinite stream browsing, in-place instruction patching, Memory Hex selection and row-wrapping keyboard navigation, register/CCR editing, Microcode Inspector step progression, and breakpoint/watchpoint triggers.
    - Verify layout stability, margin geometry, scrollbar ergonomics, and responsive display tiers (`LayoutTier::FullHdWide`, `LayoutTier::StandardDesktop`, `LayoutTier::Compact`).
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

---

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
    - Localize optimization strictly within the Agnus DMA scheduler (`crates/dma/` / `crates/agnus/src/dma/`).
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
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/audio/tests/`.
    - Headless shader initialization and rendering verification.

- **6.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/mouse`, `crates/joystick`, `crates/game_ports`, `crates/gui`).
  - **Actionable Scope:**
    - **Host Keyboard Mapping:** Translation table from host keyboard events (`winit::keyboard::KeyCode` / `egui::Key`) to raw Amiga scancodes, qualifiers (Amiga keys, Alt, Ctrl, CapsLock, Help), `Ctrl + Left Amiga + Right Amiga` reset combo, and Keyboard-as-Joystick mapping.
    - **Dual Game Port Hub:** Slot assignment for Port 1 & Port 2 (Mouse, Joystick, CD32 Pad, Keyboard-Joystick, Disconnected), simultaneous dual-mouse and dual-joystick modes, hardware button routing (CIA-A `PRA`, Paula `POTGOR`), and CD32 7-button shift register protocol.
    - **Physical Host Mouse Capture:** Viewport mouse grab/lock mode, relative motion delta scaling into Amiga 8-bit wrap-around quadrature counters ($0..255$), and sensitivity tuning.
    - **Physical Gamepad & Joystick Ingestion:** Host gamepad enumeration via `gilrs` (native desktop) and HTML5 Gamepad API (WASM), analog deadzones, and button mapping.
    - **Developer Studio Input UI:** Interactive "Game Ports & Input" tab/dock displaying port status, device dropdowns, and live switch/button indicators.
  - **Verification Gate:**
    - Unit tests in `crates/keyboard/tests/`, `crates/mouse/tests/`, `crates/joystick/tests/`, and `crates/game_ports/tests/`.
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

- **10.3: Peripheral Extensions**
  - Add a parallel-port four-player joystick adapter, analog potentiometer sampling through POT0DAT/POT1DAT, and light-pen/gun beam-position latching.
  - **Gate:** Device behavior and integration tests verify the selected extension against its hardware specification.

- **10.4: Resource Extractor and Reverse Engineering Assistant**
  - Extract bitplanes, sprites, palettes, and audio from guest memory; annotate assets, addresses, and game phases with optional LLM assistance and map them in the 24-bit address space.
  - **Gate:** Asset extraction unit tests in `crates/debugger/tests/` verify output and source-address provenance.

## Step 11: Four-Manual PDF-to-Book JSON Conversion

- **11.1: Convert Four Source PDFs to Reviewed `book.json` Recipes**
  - Prepare the A500/A2000 Technical Reference Manual, Amiga Hardware Reference Manual (second edition), M68000 Programmer's Reference Manual, and M68000 User's Manual (Revision 8). Keep a separate source identity and preparation profile for each PDF.
  - Import each prepared PDF into Book-to-Markdown, then adjust chapter organization, source selections, formatting, and graphics against the source. Accept the intended revision to produce each manual's `book.json`. Use the [book authoring workflow](tools/bootstrap/book-to-markdown/README.md#book-authoring-sequence).
  - **Gate:** Review and commit all four accepted `book.json` recipes with their matching full source hashes in `source.json`. A prepared PDF or unaccepted Working bundle is insufficient.
