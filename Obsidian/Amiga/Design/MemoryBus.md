---
title: "MemoryBus"
aliases: ["Amiga 500 MemoryBus", "Motherboard Address Router"]
tags: ["amiga", "design", "memory_bus", "physical_memory"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-08-31
updated: 2026-11-11
related: ["[General Architecture.md](General%20Architecture.md)", "[Physical Memory.md](Physical%20Memory.md)", "[Machine Loop.md](Machine%20Loop.md)", "[DMA.md](DMA.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[RTC.md](RTC.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/memory_bus"
---
# MemoryBus

[`MemoryBus<'a>`](../../../crates/memory_bus/src/memory_bus.rs) is the motherboard address router presented to the M68000 as an [`AddressBus`](../../../crates/physical_memory/src/address_bus.rs). It connects a CPU access to [`PhysicalMemory`](../../../crates/physical_memory/src/physical_memory.rs), a custom-chip register, a CIA, or the RTC. It is a temporary collection of mutable references, not an owner of RAM, chips, or the machine clock. [`A500Machine`](../../../crates/machine_loop/src/machine_loop.rs) owns those components and creates the router when it needs one.

The implementation has three cooperating parts:

| Part | Responsibility |
| --- | --- |
| `A500Machine` | Owns CPU, storage, chips, peripherals, and the CCK counter; orders subsystem steps, DMA delivery, signals, resets, and CPU stepping. |
| `MemoryBus` | Implements CPU byte/word bus operations and routes them by address; connects custom-register reads and writes to the appropriate chip interfaces. |
| [`PhysicalMemory`](Physical%20Memory.md) | Owns Chip, Slow, and Fast RAM, Kickstart ROM, the bank dispatch table, overlay, and CPU memory contention state. |

The CPU knows the `AddressBus` interface, whose accesses return `BusResult::Ready(value)` or `BusResult::WaitState`. `PhysicalMemory` also implements that interface for direct or synthetic use; a production CPU step in `A500Machine::step_cck()` receives `MemoryBus`. The CPU handles instruction sequencing and address errors; the router does not decode instructions or advance time.

## A CPU access through the router

`A500Machine::step_cck()` increments its CCK counter and advances other subsystems before stepping the CPU. In that subsystem step, Agnus advances against Chip RAM, and the machine copies `agnus.chip_ram_blocked` into `physical_memory.chip_ram_blocked`. The machine then constructs `MemoryBus` from mutable references to physical memory, Agnus, Denise, Paula, CIA-A, CIA-B, and RTC and passes it to `cpu.step_cck()`. Thus an attempted CPU Chip/Slow RAM access sees the contention state for that step. A `WaitState` keeps the CPU bus micro-step pending; the rest of the machine still advances on subsequent CCKs. The [Machine Loop](Machine%20Loop.md) describes the complete step order, while [DMA](DMA.md) owns the Agnus slot schedule.

On an A500 motherboard, Gary decodes the CPU address to select RAM, ROM, or a device and controls the connection between the CPU data bus and the shared Chip RAM data bus. [Agnus](Agnus.md) schedules access to that shared bus and generates the DRAM addresses and timing. When Agnus needs the bus, it requests it from Gary. Gary then delays the CPU's transfer acknowledgment until the bus is available, leaving the CPU access pending. This is the hardware reason for the Chip/Slow RAM wait state described above; Gary does not choose which DMA channel runs.

A bidirectional, tri-state data latch sits between the CPU and shared data buses. While enabled, a latch passes the input word; when closed, it retains the last sampled word. Its outputs can also be disconnected electrically. Gary controls this path so a granted CPU transfer can move data in the required direction, while an Agnus DMA transfer leaves the CPU side isolated from the shared bus. "Buffer" here means a small, temporary holding and isolation stage, not a second RAM store or a cache. It is also distinct from the holding registers in Denise and Paula, which capture DMA data for display or audio after Agnus selects their register. The emulator models the resulting address routing and CPU wait state; it has no separate Gary chip or motherboard latch state.

Every `MemoryBus` byte or word access masks the address to 24 bits and selects
its upper eight bits (`addr >> 16`). Banks `$A0-$BF` route to the CIA decoder,
`$DC` to RTC, and `$DE-$DF` to the shared custom-register decoder. Other banks
delegate to `PhysicalMemory`. Shared bank bounds in
[`address_map.rs`](../../../crates/config/src/address_map.rs) keep production
routing and physical classification consistent. Device classification in the
physical bank table provides placeholders, not live chip access.

| Address selected by the implementation | CPU access destination and behavior |
| --- | --- |
| `$000000-$07FFFF` | Kickstart ROM while the low-memory overlay is active; otherwise 512 KB Chip RAM. The physical-memory bank handlers make this selection. |
| `$200000-$5FFFFF` | Optional 4 MB Fast RAM in the expanded preset. Other addresses without configured storage use open bus. |
| `$A00000-$BFFFFF` | CIA selection by active-low A13/A12, register by A11..A8; CIA-B reads on the even/high lane and CIA-A on the odd/low lane. See the selection contract below. |
| `$C00000-$C7FFFF` | Optional 512 KB Slow RAM in standard and expanded presets. |
| `$DC0000-$DC003F` | RTC byte interface; its device implementation accepts odd addresses when a clock is installed. Other addresses in bank `$DC` return the fixed `$FF` placeholder on reads and ignore writes. |
| `$DE0000-$DFFFFF` | Custom-register routing, using the low register-offset bits. These addresses alias the same 512-byte register window in the current router. |
| `$F80000-$FFFFFF` | Kickstart ROM. A 256 KB image repeats across this 512 KB range; a 512 KB image fills it. CPU ROM writes are ignored. |

The preset table and installed storage are built in [`presets.rs`](../../../crates/physical_memory/src/presets.rs) and `PhysicalMemory::from_config()`. Unmapped physical-memory reads return the fixed `OPEN_BUS_BYTE` placeholder (`$FF`); word reads return `OPEN_BUS_WORD` (`$FFFF`). CIA inactive lanes and absent RTC responses use the same placeholder. See the [open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access). The map above describes implemented dispatch rather than every address reserved by Amiga hardware.

### CIA and boot overlay

Canonical CIA-A addresses are `$BFE001-$BFEF01`; CIA-B uses
`$BFD000-$BFDF00`. Within banks `$A0-$BF`, A12 low selects CIA-A and A13 low
selects CIA-B:

| A13/A12 | Receivers | Word read lanes |
| --- | --- | --- |
| `00` | Both | CIA-B high, CIA-A low |
| `01` | CIA-B | CIA-B high, `$FF` low |
| `10` | CIA-A | `$FF` high, CIA-A low |
| `11` | Neither | `$FFFF` |

A11..A8 select the register. A20..A14 and A7..A1 are mirrors. The decoder
normalizes each selected receiver to its full named canonical address in
[`cia_reg`](../../../crates/config/src/registers.rs), then calls that instance's
concrete register method. CIA receives no motherboard address. Selector B
reads `$FF`, ignores writes and does not synchronize. Byte reads reach only
the selected chip on the addressed lane; inactive lanes use fixed `$FF` without
read effects. Byte writes reach every selected chip regardless of parity.
Word writes supply the high byte to CIA-B and the low byte to CIA-A.

Selection, synchronization, acceptance and falling-E capture are separate.
Before a word read or a write can produce any register effect, all receiving
CIAs must be synchronized. Each selected receiver starts its countdown on the
first attempt. The sixteen-slot CIA buffers exceed legal CPU traffic: an
accepted write commits one CCK later, before the next synchronized transfer.
CIA bus acceptance has no queue-capacity check; direct setter injection must
be drained before CPU bus traffic resumes, as specified by [CIA](CIA.md#runtime-register-capture-and-bus-synchronization).
A refused transfer neither consumes synchronization nor acknowledges a read
or inserts a write in any receiver. Acceptance consumes all ready receivers
together, with one event per selected chip; it never splits a word into two
independently accepted byte operations. Debug reads reuse the same selection
and canonical identities without clocking, synchronization or read effects.

The A500 Rev. 6A/7 [CIA sheet](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=7)
shows U37 combining `VMA*` with A12/A13 for the two independent active-low
selects, A11..A8 feeding RS3..RS0, and CIA-A/B wired to D7..D0/D15..D8.
The [processor sheet](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=3)
shows Gary's VPA and the CPU/shared-bus latches. HRM Appendix F's address-decode
paragraph prescribes canonical software addresses and includes inconsistent
binary/parity examples; these are not an exhaustive electrical alias table.
The approved model decodes `$A0-$BF`, consistent with its upper-three-bit `101`
qualification and the [Minimig CIA equations](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v).
[vAmiga's memory decoder](../../../ref_src/vAmiga/Core/Components/Memory/Memory.cpp)
corroborates selection, write fan-out and `$DE/$DF` custom routing. Its inactive
read lanes sometimes use the CPU instruction register; this emulator retains
fixed `$FF`. The [replacement Gary](../../../ref_src/amiga_replacement_project/gary/Gary.v)
at revision `143ad5f1b5acae093ea66186e832a8ab4fb1bc0f` instead qualifies CIA with
A23..A20=`1011` and also decodes RTC/ROM/custom mirrors outside this task.
Those differences remain outside the implemented map. These models do not establish measured
silicon timing or other board revisions. The contract applies to the supported
OCS 8371 PAL and 8370 NTSC presets, with installed RAM and RTC taking their
specified ranges; no additional `$C0-$DD` custom mirrors are enabled.

CIA-A pin PA0 drives OVL. When a queued `PRA` or `DDRA` write commits on falling E, CIA-A reports a generic port change. The machine routes it through `MemoryBus::update_ciaa_overlay()` in that same modeled phase, updating the low-memory mapping from `(PRA | !DDRA) & $01`: an input pin is pulled high, while an output pin drives the stored `PRA` bit. OVL high calls `map_kickstart_to_low_memory()`; OVL low calls `map_chip_ram_to_low_memory()`. The pending write leaves the previous mapping intact. CIA-B never drives this helper. This is the effect of the committed CIA pin level, with no additional CCK propagation stage. [CIA](CIA.md#runtime-register-capture-and-bus-synchronization) specifies the separate synchronization, acceptance and capture boundaries. The [8520 port specification](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#io-ports-pra-prb-ddra-ddrb) defines direction control and pull-ups. [vAmiga](../../../ref_src/vAmiga/Core/Components/CIA/CIA.cpp) updates memory selection when PA0 changes, while the [Minimig Gary decoder](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v) and [Amiga Replacement Project Gary decoder](../../../ref_src/amiga_replacement_project/gary/Gary.v) use OVL directly in their address equations. These models support omitting an additional clocked propagation stage; they do not establish zero physical propagation time.

`PhysicalMemory` changes banks `$00-$07` between ROM and Chip RAM handlers. At construction and reset it engages the ROM overlay, consistent with CIA-A PA0 being an input with a pull-up, so reset vectors can be read from low memory. Cold reset clears RAM; warm reset preserves it. Storage mapping and resets belong to physical memory and the machine, not to a persistent `MemoryBus` state.

### Custom registers and specialized chips

For banks `$DE` and `$DF`, `addr & $01FE` selects an even register offset. A word operation reaches a single custom register. A byte read takes the high or low byte of that register according to address bit 0. A byte write duplicates its value onto both halves of a 16-bit word before calling `write_custom_word()`. Every modeled data write calls a concrete queued setter on each receiving chip through one offset match. Address strobes act immediately at accepted selection and allocate no queue entry. A pending value cannot alter a read, child component, restart action or output before commitment. Unmatched offsets remain ignored.

`read_custom_word()` dispatches readable registers to Agnus, Denise, or Paula. Examples include Agnus beam and DMA status, Denise joystick and collision status, and Paula interrupt, audio-control, and disk status. A live `CLXDAT` or `DSKBYTR` read may have a chip-defined side effect. Reading `COPJMP1` or `COPJMP2` strobes Agnus's Copper and returns `$FFFF`. Unhandled reads, including most reads of write-only registers, return `$FFFF`; see the [open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access). `read_custom_word_debug()` uses non-mutating chip inspection methods and does not issue the Copper strobes.

`write_custom_word()` routes `DMACON` to both Agnus and Paula. It routes `BPLCON0`, `DIWSTRT`/`DIWSTOP`, and sprite position/control registers to both Denise and Agnus. The chips independently latch the DIW fields used for vertical bitplane DMA and horizontal pixel clipping. The router sends other listed display, sprite data, color, and input registers to Denise; listed interrupt, disk, serial, potentiometer, and audio registers to Paula; and explicitly listed Agnus registers to Agnus. Unmatched offsets are ignored without staging a chip mutation. The [register ownership matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) gives the broader register context, while this dispatch is the implemented CPU/Copper write path.

The custom chips use fixed-capacity ordered propagation queues. Each setter reports acceptance; a full queue rejects a transfer without applying it. Shared-register arms check every required receiver before inserting any event, so a retry cannot duplicate a partially accepted transfer. The CPU receives `Ready` only after insertion and otherwise receives `WaitState`; acknowledgment does not imply commitment. All custom data writes currently use `Pipeline`, preserving every accepted operation. The current model retains concrete reference timings recorded in the timing research and uses **1 CCK** where timing remains unresolved. Reference-model timing and the fallback are not universal silicon measurements. Reads observe current state at accepted selection without flushing pending writes. CIA capture and RTC accesses retain their separate contracts.

DMACON retains **2 CCK** for Agnus and Paula. BPLCON0 and DIWSTRT/STOP retain **4 CCK** for Agnus and **1 CCK** for Denise; DDFSTRT/STOP retain **4 CCK** for Agnus. INTENA/INTREQ retain **1 CCK**. Remaining custom data writes use the **1 CCK** fallback, including pointers, palette, audio data and controls, and Blitter/Copper data registers. Address strobes COPJMP1/2 and STREQU/STRVBL/STRHOR/STRLONG act immediately without queue capacity checks. Data-bearing triggers such as BLTSIZE and BPL1DAT act when their queued data commits; downstream engine scheduling remains separate. Electrical INT2/INT3/INT6 levels are sampled by [Paula](Paula.md#register-propagation) after due writes and consume no register queue slot.

These `delay_cck` values and per-register `MutationMode` choices describe the current implementation and remain provisional under the [register mutation parameter policy](../../../.agents/rules/spec-compliance.md#provisional-register-mutation-parameters). Both the one-CCK fallback and the concrete reference-model delays share this status.

Writes issued after a chip step start counting on its next step. The machine issues Copper register writes and bitplane DMA data after all three custom chips have stepped, before the CPU phase, in Copper-then-DMA order; the CPU issues afterward. This deterministic provisional order does not establish physical simultaneous-access priority. All recipients therefore count from the same boundary, independently of host method ordering. CPU CCK1/CCK2 acceptance still follows the CPU's existing bus protocol; an unaccepted transfer inserts nothing. Live COPJMP reads keep their immediate read-side strobe and fixed open-bus result. Autonomous DMA pointer increments, counter changes, pin sampling and interrupt generation remain separate from register-bus writes.

Neither Denise nor Paula owns a `PhysicalMemory` reference. [Agnus](Agnus.md) owns DMA addressing and arbitration. The machine supplies Agnus with Chip RAM during its step, delivers a polled bitplane word to Denise, and handles the currently modeled floppy DMA transfer through physical memory. A pending Copper write goes through `self.memory_bus().write_custom_word(reg, val)`, so Copper and CPU writes share register dispatch. Other cross-chip signals and DMA data movement are coordinated by the machine, not by general CPU address reads through `MemoryBus`.

#### Owner delivery and functional reconstruction

The [existing register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md#modeled-interfaces-and-timing)
records each offset's named write interface, every recipient, queue mode and
configured CCK delay. It also identifies the read/debug access classes and
logical child owners. This is the single address catalog; no separate router
catalog is required. Agnus's queue is the owning interface for Copper and
Blitter register requests. A returned due `(offset, value)` proves that the
owner consumed its event; functional correctness needs engine observations.

Some register and receiver effects remain incomplete; see the
[chipset reconstruction work](../../../ROADMAP.md#chipset-reconstruction).

All pending custom events live in serializable chip state, along with retained
Copper/bitplane outputs that have not yet entered a destination queue. Reset
cancels pending events. Restore resumes the remaining delays without issuing
another write. The CPU holds the same active micro-step and payload on
`WaitState`, so a rejected transfer cannot mature; only an accepted transfer
can later be returned due. Shared preflight rejects a full receiver before
any destination accepts the write. Paula has 32 register-write slots; Agnus
and Denise each have 64. Interrupt input levels use no register slot.
All matured entries are returned in retained order.

### RTC and exceptional access paths

The router forwards addresses `$DC0000-$DC003F` to [`RtcMsm6242b`](../../../crates/rtc/src/rtc.rs). The RTC itself returns `$FF` and ignores writes on even bytes or when its model is `None`; on odd bytes it selects one of 16 four-bit registers using `(addr >> 2) & $0F`. Word operations split into two byte lanes in big-endian order. The machine advances the RTC once per subsystem CCK; the router only handles its mapped accesses.

`write_tas_byte()` implements the modeled TAS read-modify-write exception. It checks the physical-memory bank classification: on Chip or Slow RAM it waits if Chip RAM is blocked and otherwise completes without storing the byte. Other targets use the ordinary byte write. This is specific to the TAS write phase; ordinary writes to these RAM banks use their memory handlers.

The bus also exposes inspection paths. `read_word_debug()` uses side-effect-free custom and CIA reads, or physical-memory debug reads, without CPU wait states. `write_bytes_debug()` delegates directly to physical memory for debugger or test injection. The RTC debug word path currently calls its ordinary read method, which is read-only. These APIs are not CPU timing paths.

## Reference Documentation & Upstream Ground Truth

- [8520 I/O ports](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#io-ports-pra-prb-ddra-ddrb), [vAmiga CIA-A](../../../ref_src/vAmiga/Core/Components/CIA/CIA.cpp), [Minimig Gary](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v), and [Amiga Replacement Project Gary](../../../ref_src/amiga_replacement_project/gary/Gary.v): Define port direction and pull-ups, and show the reference models' immediate OVL-based memory selection.
- [Commodore A500 Service Manual, system block diagram](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=6), [bus control](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=9), [arbitration](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=10), and [Gary](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=17): Show the CPU/shared-bus latch, address decoding, bus isolation, and delayed CPU acknowledgment.
- [`memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Defines the live address router, RTC access, debug memory methods, and TAS handling.
- [`custom_registers.rs`](../../../crates/memory_bus/src/custom_registers.rs): Routes live and debug custom-register reads and queued writes, including transfers shared by multiple chips.
- [`cia_registers.rs`](../../../crates/memory_bus/src/cia_registers.rs): Routes CIA register access and applies committed CIA-A overlay output.
- [`address_bus.rs`](../../../crates/physical_memory/src/address_bus.rs): Defines the access protocol and ready/wait result consumed by the CPU.
- [`physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs): Owns buffers, overlay, contention flag, reset behavior, and the bank-handler entry points.
- [Physical Memory.md](Physical%20Memory.md): Specifies the installed bank map, storage operations, boot overlay, and direct DMA access behind the router.
- [`map.rs`](../../../crates/physical_memory/src/map.rs) and [`presets.rs`](../../../crates/physical_memory/src/presets.rs): Define memory bank handlers and configured address ranges.
- [`machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs): Shows when the router is constructed and how DMA, contention, chip signals, and CPU steps meet.
- [`test_router.rs`](../../../crates/memory_bus/tests/test_router.rs) and [`test_register_wiring.rs`](../../../crates/memory_bus/tests/test_register_wiring.rs): Exercise production router wiring, overlay changes, lane decoding, and register behavior.
- [Machine Loop.md](Machine%20Loop.md) and [DMA.md](DMA.md): Specify the coordinator sequence and Agnus-controlled DMA scheduling beyond the router's scope.
