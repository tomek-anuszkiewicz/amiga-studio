# Amiga 500 Emulator: Development Roadmap

## Step 2: HRM-Aligned Custom Chipset Reconstruction

Current execution scope through 2.5, including the propagation audit, excludes
vAmigaTS execution (including adapted payloads), Kickstart/AmigaOS integration,
and floppy-media reads/writes. Use directly loaded authored assembly programs
and deterministic pin/latch inputs. ROM overlay checks use synthetic ROM bytes
without executing firmware. Later firmware, media, and external-suite work is
unscheduled and must not start automatically after the current plans.

<a id="gary-address-decoding"></a>

- **2.2: Model Old Gary Address Decoding on the A500 (`MEM-GARY-01`)**
  - Expand the current router map to cover the missing Old Gary bank/dual-CIA selection and aliases. In every supported A500 preset, route banks `$A0-$BF` to the CIA decoder. A13/A12 are active-low selects: `00` selects both CIAs, `01` CIA-B, `10` CIA-A, and `11` neither. A11..A8 select the register; A7..A1 mirror it. `$BFC000` is one address in the `$BFC000-$BFCFFF` selected block, and the same selection pattern repeats in other mapped banks. CIA register selector `B` remains unconnected.
  - With both CIAs selected, an even byte read uses CIA-B and an odd byte read CIA-A; a word read puts CIA-B in the high byte and CIA-A in the low byte. A byte write sends the same value to every selected CIA regardless of parity; a word write sends the high byte to selected CIA-B and the low byte to selected CIA-A. Pass full register identities to the existing CIA recipients and preserve CIA-A Port A overlay effects on every alias. Retain the documented fixed placeholder for an inactive read lane rather than assuming a physical data-bus latch.
  - Route `$DE` and `$DF` through the same custom-register decoder, including its low-offset mirror, byte/word behavior, delayed mutations, side effects, and side-effect-free debug reads. Preserve installed Slow RAM and RTC precedence; evaluate other model-dependent mirrors separately. Update [MemoryBus](Obsidian/Amiga/Design/MemoryBus.md), [Physical Memory](Obsidian/Amiga/Design/Physical%20Memory.md), [CIA](Obsidian/Amiga/Design/CIA.md), and the [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) together with the code. Recheck the HRM wording; WinUAE `cia.cpp`/`memory.cpp` and `ref_src/vAmiga/Core/Components/Memory/Memory.cpp` corroborate the decode but do not prove physical A500 wiring without schematic or measured evidence.
  - Verify acceptance and retry behavior separately from address selection and delayed effects. Record common and model-specific coverage for both supported PAL/NTSC OCS models and applicable memory presets.
  - **Gate:** First reproduce failures against the original router and bank table. Focused `memory_bus` and `physical_memory` tests cover dual selection, byte/word lanes and writes, nonzero register selectors, low-bit and `$A0-$BE` aliases, inactive selection, overlay effects, `$DE` reads/writes/debug reads, and Fast/Slow RAM and RTC boundaries. Then pass adjacent bus and machine integration checks.

<a id="inter-chip-signal-propagation"></a>

- **2.3: Verify Inter-Chip Signal Routing and Propagation**
  - **Objective:** Verify and repair connections from chip outputs through motherboard routing to recipient inputs at the required CCK phase. Keep rules, skills, workflows, and design documentation consistent with the hardware-supported behavior. Use the register access and commit behavior established in Step 2.1 as the starting point.
  - **Mandatory order:** First review A500 wiring, hardware books, vAmiga, Minimig, and the Amiga replacement FPGA project, including keyboard, game ports, floppy, serial, parallel, and host/media boundaries. Then implement and verify motherboard signal routing and electrical propagation. Only after that start the book-driven chipset reconstruction in Step 2.4.
  - **Actionable Scope:**
    - Compare A500 wiring and the [cross-chip signals catalog](Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) with `memory_bus.rs` and `machine_loop.rs`. Trace register-bus strobes, DMA requests and grants, interrupts, sync, reset, and peripheral pins from source output through machine routing to recipient input at the required CCK phase; resolve missing, reversed, or direct peer connections against hardware evidence.
    - **Agnus-Paula DMA protocol documentation:** Establish and document the complete exchange used to arrange audio/disk DMA: line synchronization, Paula request generation, `DMAL` encoding and sampling, channel/request identification, Agnus scheduling and address ownership, `RGA`/shared-data-bus selection, transfer direction and receiving/supplying latch, and pointer/count/reload effects. State which chip produces and consumes each signal, how a requested transfer becomes an actual transfer, and the relevant CCK phases, enable conditions, pending-state retention and reset behavior. Separate measured facts from reference-model assumptions and unknown timing; do not invent a dedicated grant wire where the transfer is identified through the existing buses. Keep disk-media transfers deferred.
    - **Other DMA coordination paths:** Audit whether any additional cross-chip information affects transfer eligibility, timing or completion, especially between Agnus and Denise. Verify the documented distinction between Paula's request-driven `DMAL` path and Agnus-scheduled bitplane/sprite delivery to Denise. Record any actual return path or its evidence-supported absence, shared control-register effects, and synchronization dependencies. Also review Gary/CPU bus ownership and other relevant recipients; distinguish internal Agnus/Copper/Blitter state from motherboard signals. Update [DMA](Obsidian/Amiga/Design/DMA.md), the signal catalog and affected chip/coordinator specifications with the agreed contracts and named missing routes or engine handoffs to 2.4/2.5.
    - **Beam timing and sync representation:** Review the information carried by Agnus refresh strobes `$038/$03A/$03C/$03E` STREQU/STRVBL/STRHOR/STRLONG before choosing their software representation. Separate generation/routing of these automatic strobes may be unnecessary for Denise: the preferred direction is to retain the existing `BeamPosition` structure passed from Agnus through the machine coordinator to Denise, carrying decoded line, within-line timing and field/interlace information needed by the receiver. Establish which information and sampling phase are required and whether the structure covers them. Verify Paula's STRHOR synchronization separately; this Agnus-to-Denise representation does not settle Paula's input or extra CPU-initiated strobe effects. Investigate those effects without assuming harmless reads or stored bits. Missing receiver behavior belongs to [2.4](#chipset-reconstruction), followed by 2.5 guest/output verification. See the [sync evidence](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md#cpu-writes-to-refresh-sync-strobes).
    - **CIA/peripheral and game-port routes:** Verify CIA connector direction/return paths and keyboard, parallel, floppy-control and serial-handshake protocols, plus mouse/joystick signal delivery. Use working interfaces or deterministic endpoints while port/counter effects are reconstructed. Pass named missing engine/device reactions to 2.4 with their 2.5 full-exchange tests; keep drive/media transfers and firmware deferred.
    - **Hardware timing and revision evidence:** Custom 1/2/4 CCK delays, queue modes and issue order are provisional; CIA synchronization is reference-model evidence. Review exact sampling edges, Gary delays, analog thresholds and board-revision differences, recording unsupported details as unknown. Reconcile Rev. 6A/7 BLIT/BLISS with older DBR descriptions before equating signals; do not invent a dedicated Agnus-to-Paula VERTB wire from incomplete evidence. Measured calibration remains separately scheduled. See the [signal catalog](Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md).
    - **Propagation and ordering:** Verify output polling, recipient input updates, and downstream reactions, including interrupt priority, blitter busy, disk DMA, and Copper WAIT/SKIP. Distinguish the register-write staging and commit checked in Step 2.1 from the resulting signal's delivery to another chip; reuse that evidence instead of repeating the register inventory.
    - **Propagation substrate:** Implement scheduled output delivery and recipient sampling with explicit phase ordering, serializable pending signals, deterministic resume, and before/due/after boundary checks. Keep register commitment distinct from subsequent signal delivery and avoid double delay. Mark provisional delays separately from measured hardware values.
    - **Complete transfers and interaction:** Follow each in-scope DMA stream from request or scheduled opportunity through arbitration/grant, address/data transfer, receiving latch, and pointer/count update. Verify applicable bus retries, competing CPU/display/audio/Copper/Blitter activity, active configuration changes, and simultaneous interrupt assertion/acknowledgment. Record state ownership and PAL/NTSC-specific phase behavior. Inspect disk-media architecture as reference evidence only; restrict current disk checks to register/pin propagation with deterministic inputs, without drive/media transfers.
    - **Documentation:** Verify that `hardware-bus-topology.md`, relevant design specs, and `MachineLoop` documentation describe both register-write propagation (e.g. `COLOR00`, `BPLCON0`, `INTENA`) and inter-chip signal delivery with the established clock-phase ordering. Update affected governance guidance and missing sections. Resolve implementation/documentation conflicts against hardware evidence, and keep unverified timing explicit.
  - **Verification Gate:**
    - Record each signal's source, motherboard route, recipient, delivery phase, and observable effect. Focused machine integration tests cover distinct signal paths and ordering, including missing, reversed, or direct peer connections; reuse Step 2.1 evidence where coverage overlaps.
    - The Agnus-Paula DMA exchange has a source-referenced protocol description from synchronization/request through the transferred word and resulting state, including encoding, ownership and timing unknowns. The completeness review explicitly covers Agnus-Denise and other transfer-control dependencies, with supported routes, confirmed absences and unresolved questions distinguished. Verify the transport/phase contracts using working paths or deterministic endpoints; pass absent engine behavior to 2.4 and full guest scenarios to 2.5 without requiring disk-media execution here.
    - Record the selected beam/sync representation, its information content and phase contract, including whether separate automatic strobes are needed for Denise. Verify the required behavior for both Denise and Paula and complete the CIA/peripheral/game-port source/recipient and phase requirements above. Record revision-aware evidence and timing unknowns; retain explicit 2.4 ownership for absent engine reactions.
    - Establish transport and phase correctness using working paths or deterministic test endpoints. Record absent in-scope engine/device behavior and hand its full-exchange tests to Step 2.4; keep disk-media and firmware execution deferred; endpoint simulation does not prove that behavior.
    - Run the scoped specification review and [vault link/frontmatter audit](.agents/skills/obsidian-vault-linking/SKILL.md); the milestone command does not execute documentation maintenance. At point closure, run commit and milestone profiles at their existing triggers.
    - The intended commit changes were reviewed against documents selected by `tracked_paths`; real corrections are included, and accurate documents remain untouched.

<a id="chipset-reconstruction"></a>

- **2.4: Reconstruct Custom Chipset Behavior from Book Specifications**
  - **Objective:** Replace existing behavioral logic in `crates/machine_loop` and the custom chip subsystems (`agnus`, `denise`, `paula`, `cia`, `copper`, `blitter`, `floppy` and their affected children) with behavior derived from the hardware books. Preserve the established CPU/storage boundaries, address decoding, direct register interfaces, clocking, propagation, serialization, and supported hardware invariants.
  - **Prerequisite:** Complete Step 2.3's three-reference review and implementation/verification of signal propagation before replacing chipset behavioral engines. Reuse that substrate and its phase contracts; complete the device/engine behavior and full-exchange tests handed over by the review.
  - **Actionable Scope:**
    - Establish book-derived behavior contracts with exact source sections, supported models, state transitions, reset behavior, outputs, and explicit timing unknowns. Replace obsolete mechanisms and affected callers coherently; retain useful regression protection.
    - **Preserve `BeamPosition`:** Keep the existing shared beam-position structure and its Agnus-to-machine-coordinator-to-Denise path during reconstruction. The current fields are `hpos` (horizontal CCK position), `vpos` (line) and `lof` (long-field state). Retain this informational interface for beam timing and supply any additional required within-line or field/interlace information through it as established in 2.3; distinguish pixel coordinates from CCK position and interlace enable from long-field state. Do not introduce a parallel automatic sync-strobe mechanism for Denise if the structure already provides the required information and timing.
    - Identify one authoritative owner for each pointer, counter, latch, and pending event, separating programmed reload values from active engine state. Audit apparent duplicate DMA address state, including audio fields, against Agnus ownership and actual consumers.
    - **Required register and engine work:** Recheck the findings below, reconstruct the behavior and run focused tests in this stage, preserving established access, queues, ownership and working OVL behavior. Carry the stated guest/output scenarios into 2.5 and include the reconstructed effects in the final register audit.
      - **CIA port reads:** PRA/PRB readers return shared stored fields instead of separately resolved input pins and output latches; polling can replace PRA bits. CIA-B PRB with DDRB=`$00`, latch=`$00` and no external low driver reads `$00` instead of `$FF`. Separate latches, external inputs and direction-controlled pin resolution; preserve committed PRA/DDRA OVL polarity, mapping, reset and continuation. Run focused tests here and guest direction/input/readback scenarios in 2.5. See the [CIA specification](Obsidian/Amiga/Design/CIA.md).
      - **JOYTEST:** `$036` reaches an empty Denise application arm. Write bits 7 through 2 of both counter bytes for both ports, preserve the low two bits, and retain the change through subsequent GamePorts polling/movement. Repair authoritative counter ownership rather than a transient readable latch. Run production-machine tests here and guest write/read/movement scenarios in 2.5. See [Denise](Obsidian/Amiga/Design/Denise.md).
      - **Beam writes:** `$02A` VPOSW and `$02C` VHPOSW mature in Agnus without changing beam state. Establish supported OCS field behavior and the resulting VPOSR/VHPOSR values from references. Run focused readback tests here and guest scenarios in 2.5. See the [register matrix](Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md).
      - **Refresh/Copper internal addresses:** `$028` REFPTR and `$08C` COPINS mature without application. Resolve refresh/test behavior and Copper instruction-fetch identity against exact book sections; COPINS is a dummy fetch address, not a general instruction-injection interface. Explain any reference-supported non-action explicitly. Run reference-derived tests here, with 2.5 scenarios only where supported access permits them. See [Agnus](Obsidian/Amiga/Design/Agnus.md).
      - **Beam/sync receiver behavior:** `$038/$03A/$03C/$03E` STREQU/STRVBL/STRHOR/STRLONG currently have no receiver reaction. Reconstruct the required synchronization behavior using the representation established in [2.3](#inter-chip-signal-propagation), preserving `BeamPosition` for Agnus-to-Denise timing. Separate automatic strobe handlers for Denise are conditional on that review; verify the required behavior rather than requiring a duplicate transport mechanism. Preserve the separate investigation of Paula's STRHOR synchronization and supported CPU-initiated strobe effects. Together with JOYTEST, beam writes and REFPTR/COPINS, these nine custom offsets still require an access/effect decision for ten receiver paths because STRHOR is shared. Run receiver behavior tests here and guest/output verification in 2.5. See the [sync evidence](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md#cpu-writes-to-refresh-sync-strobes).
      - **CIA/peripheral and game-port reactions:** Complete named missing engine/device reactions handed over by 2.3 after connector direction, return paths and signal delivery are verified. Cover keyboard, parallel, floppy-control, serial-handshake and mouse/joystick behavior with focused and full-exchange tests, followed by 2.5 functional exchanges. Keep drive/media transfers and firmware deferred. See the [signal catalog](Obsidian/Amiga/Design/Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md).
      - **Other chipset and peripheral effects:** Complete reference-derived timer/serial/TOD/handshake, read-clear/strobe, counter/reload, active-state and DMA behavior, including named missing device/engine exchanges handed over by 2.3. Queue acceptance/due events do not establish internal effects or full exchanges. Preserve existing working behavior and useful regressions. Run focused tests here and guest outputs and interaction scenarios in 2.5; retain the no-firmware/no-media boundary.
      - **Paula UART:** Serial transmit/receive shift-clock progression is a scaffold in `crates/paula/src/serial.rs`. Implement book-derived bit timing, data/status transitions and interrupts; retain separate frontend serial I/O boundaries. Run focused engine tests here and deterministic serial exchange in 2.5. See [Paula](Obsidian/Amiga/Design/Paula.md).
    - Reconstruct complete in-scope DMA transfers and relevant active-state transitions in dependency order: bus/clocks and arbitration, autonomous DMA/Copper/Blitter, display/sprites, then peripherals/audio/CIA. Verify enable/disable, restart, parameter-update, interrupt acknowledgment, and reset boundaries according to each mechanism's contract.
    - **Basic DMA scheduling before the propagation audit:** Complete the existing Agnus-owned production scheduler in 2.4 so refresh, bitplane/sprite fetches, audio, Copper, Blitter and CPU Chip RAM accesses share eligible CCK slots according to book-derived enable and priority rules. Connect scheduled fetch opportunities or routed requests to actual grants/transfers, retain Agnus address ownership, and return CPU `WaitState` while Chip RAM is occupied. Complete focused production-machine regressions before 2.5; the combined 2.5 stage verifies propagation and broader program behavior. Keep disk checks within the register/pin boundary. Optimize the scheduler later in 4.3; distinguish basic functional scheduling from unverified silicon timing calibration. See [DMA](Obsidian/Amiga/Design/DMA.md).
    - **Frame generation and live GUI output before 2.5:** Complete the reconstructed display-to-frame-buffer path and connect actual machine frames to the existing GUI viewport, replacing its standby image. Verify that a directly loaded authored program produces the expected visible output and that stepping/running updates the screen. This is a required deliverable of 2.4 and a prerequisite for starting 2.5, not later frontend polish. Preserve native/WASM boundaries; CRT presentation effects remain later work. See the [GUI specification](Obsidian/Amiga/Design/Application/GUI.md).
    - Resolve conflicts among books, approved specifications, measurements, and existing behavior under [specification compliance](.agents/rules/spec-compliance.md). Preserve documented quirks and invariants unless an explicit change is approved; do not leave required functionality inert or call provisional delays measured hardware timing.
  - **Verification Gate:**
    - Book-derived behavioral and production-machine integration tests demonstrate actual data, output, state transitions, and reset behavior for reconstructed subsystems, including applicable PAL/NTSC differences. Register assignments and a successful build alone do not establish completion.
    - Before starting 2.5, demonstrate expected machine-generated frame contents and visible, updating output from the same authored program in the existing GUI viewport. A standby pattern or a nonempty buffer does not satisfy this prerequisite.
    - Every required register/engine item above, including the named 2.3 handoffs, has a source-supported contract and executed behavior/readback/side-effect tests. Explain any supported non-action explicitly; a consumed queue event is not functional proof. Remove each verified item and its obsolete source caveat only after its required acceptance checks pass; retain pending 2.5 scenarios until that stage verifies them.
    - `python tools/verification/verify.py check build`
    - `python tools/verification/verify.py check architecture`
    - Run relevant native/WASM and subsystem checks, plus the existing commit and milestone closure procedures at their required triggers.

<a id="cpu-memory-gary-verification"></a>

- **2.5: Unit Tests, CPU/Gary Propagation and Book-Derived Guest Programs**
  - **Objective:** Verify the production CPU-to-memory/device path immediately after 2.4, including Gary decoding, CPU waiting, ROM overlay, observable motherboard data-latch behavior, the CIA/custom-register propagation audit, and book-derived display, sprite, Blitter, digital-audio and interaction scenarios. Reuse 2.2-2.4 evidence and fixtures; extend missing coverage with guest programs rather than duplicating equivalent tests.
  - **Prerequisite:** Complete 2.4, including the production DMA scheduler, verified frame generation and live machine output in the GUI viewport.
  - **Program contract:** Each authored guest-program integration test has its own directory under `crates/machine_loop/tests/programs/<subsystem>/<case>/`, containing `test.rs`, `program.as` and the compiled `program.bin`. Record book provenance, initial conditions, independent expected results and a bounded CCK/frame budget; reproduce the binary from its source during validation. The automated test and GUI load the same binary. Guest instructions perform the accesses under test; host code supplies initial conditions and observes results and CCK/phase boundaries. Component unit tests follow their owning crate's existing layout and do not require ASM/BIN artifacts.
  - **Actionable Scope:**
    - **Focused unit tests:** Audit and fill meaningful gaps in each component's own behavior, such as register masks, local pending/commit transitions, read-clear/strobe effects and reset behavior. Supply only the inputs, clock steps or minimal fixtures needed by the component under test. Do not recreate the machine loop, motherboard routing, DMA orchestration or peer-chip state machines inside unit tests. Cross-chip propagation, arbitration, overlay routing and phase ordering use the real `A500Machine` integration path described below; reuse existing coverage instead of testing the same contract twice.
    - **CPU memory transactions:** Exercise byte, word and long accesses, big-endian byte lanes, read/write direction, installed Chip/Slow/Fast RAM and ROM, bank boundaries and applicable memory presets. Confirm ROM write protection, modeled unmapped accesses and that unrelated storage is unchanged.
    - **Gary selection and aliases:** Verify CPU-visible CIA selection, dual-CIA byte/word lanes, register selectors and aliases, custom `$DE/$DF` mirrors, and installed RAM/RTC precedence according to the completed 2.2 contract. Observe the selected devices' effects as well as returned data; a correct bank classification alone is insufficient.
    - **CPU waiting and DMA handover:** Generate actual shared-bus activity through the scheduler completed in 2.4. Verify denied CPU transfers stay pending without advancing their bus micro-step or changing data, that chips continue advancing, and that the transfer completes once after a grant. Cover CPU-to-DMA-to-CPU transitions, independent successive data and targets, required shared-bus priorities, and memory regions that must remain free of Chip RAM contention.
    - **ROM overlay:** Use distinct synthetic ROM and RAM contents at the same low addresses. Let the guest change CIA-A PRA/DDRA through relevant aliases; inspect before/at/after commitment, input/output direction, both overlay states, resets and restored pending changes. Verify CPU mapping versus direct Agnus Chip RAM access and preservation of underlying RAM. Keep test code outside the switched window when required; do not let debugger loading silently remove the overlay being tested. No Kickstart/OS execution is required.
    - **Motherboard latches and exceptional accesses:** Verify observable address/data stability, byte-lane preservation and bus-direction/isolation effects across CPU/DMA handovers, plus the documented TAS behavior on Chip/Slow RAM and other targets. Test the modeled external contract without requiring a separate Gary/latch struct. Physical open-bus retention/decay and additional opposite-direction effects remain in 12.1; keep provisional timing distinct from hardware measurements.
    - **Final register audit and continuation:** Complete the existing per-register recipient, read-clear/strobe, pending/commit, retry and exactly-once checks through the same production execution path. Verify debug inspection is neutral, and compare uninterrupted execution against serialized/restored pending transfers and overlay changes. Record PAL/NTSC and preset applicability. Deferred BERR and external interrupt cases remain in Step 12.
    - Check sprite/display pixels in the frame buffer, Blitter destination data and completion, digital audio sequences, and relevant registers/memory. Preserve write-only, read-clear, and strobe semantics. Use the live GUI frame output completed in 2.4 for manual visual validation.
    - **Existing implementation gaps:** Complete the guest/output scenarios handed over by [2.4](#chipset-reconstruction), using independently expected readable values, memory and outputs after the owner applies each request.
    - Include complete DMA paths using distinct successive data, applicable `WaitState` retries with exactly-once effects, shared-bus competition, active configuration changes, and simultaneous interrupt assertion/acknowledgment. Exercise the interactions being tested rather than restricting every scenario to an idle bus or hblank.
    - Record PAL/NTSC OCS and memory-preset applicability, run required common paths on both supported Agnus models, and add model-specific timing/geometry cases. Separate supported-model coverage from default PAL results.
    - Compare uninterrupted and restored execution for relevant pending/active operations. Keep provisional model timing distinct from unknown silicon calibration; full vAmigaTS capture and lockstep work remain separate.
  - **Verification Gate:** Focused unit tests verify component-local contracts, and executed ASM/BIN integration cases prove expected memory/device results, pixels, Blitter output and digital sample sequences, including the relevant waiting, grant, latch and overlay boundaries. Independently derived outputs and reproducible GUI/automatic execution are required; a nonempty buffer, register assignment or four introductory programs do not prove complete coverage. Record which layer establishes each contract. Pure CPU benchmarks, direct host writes or eventual readback alone do not establish production bus timing. Run affected memory, bus, machine and native/WASM checks and the existing roadmap closure profiles; preserve the final register-wide coverage requirement. See [MemoryBus](Obsidian/Amiga/Design/MemoryBus.md), [Physical Memory](Obsidian/Amiga/Design/Physical%20Memory.md), [DMA](Obsidian/Amiga/Design/DMA.md) and the [register matrix](Obsidian/Amiga/Design/Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md).
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
  - **Execution order:** Complete basic audio playback before or during 2.8, before its audio scenarios run. Keep later playback refinements and CRT shaders in this track; do not delay the basic sound path until after external-suite bring-up.
  - **Actionable Scope:**
    - Audio sink ring buffer decoupled from host audio playback (`cpal` / Web Audio) with dynamic resampling and ring buffer underflow/overflow protection.
    - GPU post-processing shaders for authentic CRT TV presentation (scanlines, shadow mask, curvature, phosphor bloom).
  - **Verification Gate:**
    - Audio ring buffer unit tests in `crates/audio/tests/`.
    - Headless shader initialization and rendering verification.

- **6.2: Host Input Subsystem, Game Controller Mapping & Port Hub**
  - **Objective:** Implement comprehensive host-to-Amiga keyboard translation, physical mouse capture, gamepad ingestion, and dual game port switching (`crates/keyboard`, `crates/mouse`, `crates/joystick`, `crates/game_ports`, `crates/gui`).
  - **Execution order:** Complete basic keyboard, mouse and joystick/gamepad routing before or during 2.8, before its input-dependent scenarios run. Keep extended controller protocols, mapping options and input UI refinements in this track; do not defer the basic host-to-guest input path until after external-suite bring-up.
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
  - Add light-pen/gun beam-position latching. The parallel-port four-player joystick adapter and analog proportional inputs are deferred to 12.2.
  - **Gate:** Device behavior and integration tests verify the selected extension against its hardware specification.

- **10.4: Resource Extractor and Reverse Engineering Assistant**
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

- **12.3: Optional External CPU Interrupt Cases**
  - **Level 7 NMI / autovector 31:** The current emulator ignores IPL=7. Standard Paula sources generate levels 1-6, but real A500 expansion hardware can drive the CPU interrupt lines to request level 7. Record this missing external-input support and consider implementing it only when such hardware or a concrete workload is in scope, including the CPU's special level-seven acceptance behavior.
  - **Uninitialized interrupt / vector 15:** The current bus interface has no device-supplied interrupt-vector response. Built-in A500 interrupts use autovectors, so there is no standard onboard source for this case. Consider support only if a selected external device or a broader CPU-interface requirement needs it; distinguish an uninitialized peripheral vector register from an absent handler in RAM.
  - **Spurious interrupt / vector 24:** The current interface cannot report BERR during interrupt acknowledgment. Standard A500 interrupts have no identified normal source for this case. Consider implementation only for a concrete external-device or bus-fault scenario, together with the deferred BERR interface in 12.1.
  - Keep all three as low-priority completeness gaps, outside the prerequisites for current chipset reconstruction and testing. Record the required hardware/input scope before implementation and preserve existing supported exceptions. See the [CPU specification](Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md).
  - **Gate if scheduled:** Reference-derived tests verify the selected input, interrupt acknowledgment or vector selection, exception frame, transaction/wait behavior and continuation. Do not treat recording these gaps as implemented support.
