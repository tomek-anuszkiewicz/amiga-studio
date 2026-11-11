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

Every `MemoryBus` byte or word access masks the address to 24 bits and selects its upper eight bits (`addr >> 16`). Three banks receive live device routing: `$BF` for CIAs, `$DC` for RTC, and `$DF` for custom registers. All other banks delegate to `PhysicalMemory`, including RAM, ROM, and unmapped space. The router does not consult `PhysicalMemory::bank_map` for those three live device cases; their entries in that table serve classification and direct physical-memory access. Within `PhysicalMemory`, the 256-entry table chooses a handler for each 64 KB bank.

| Address selected by the implementation | CPU access destination and behavior |
| --- | --- |
| `$000000-$07FFFF` | Kickstart ROM while the low-memory overlay is active; otherwise 512 KB Chip RAM. The physical-memory bank handlers make this selection. |
| `$200000-$5FFFFF` | Optional 4 MB Fast RAM in the expanded preset. Other addresses without configured storage use open bus. |
| `$BFD000-$BFDF00`, even bytes | CIA-B registers; address bits A7–A1 mirror each register. |
| `$BFE001-$BFEF01`, odd bytes | CIA-A registers; address bits A7–A1 mirror each register. Other bytes in bank `$BF` read as `$FF` and ignore writes. |
| `$C00000-$C7FFFF` | Optional 512 KB Slow RAM in standard and expanded presets. |
| `$DC0000-$DC003F` | RTC byte interface; its device implementation accepts odd addresses when a clock is installed. Other addresses in bank `$DC` return the fixed `$FF` placeholder on reads and ignore writes. |
| `$DF0000-$DFFFFF` | Custom-register routing, using the low register-offset bits. These addresses alias the same 512-byte register window in the current router. |
| `$F80000-$FFFFFF` | Kickstart ROM. A 256 KB image repeats across this 512 KB range; a 512 KB image fills it. CPU ROM writes are ignored. |

The preset table and installed storage are built in [`presets.rs`](../../../crates/physical_memory/src/presets.rs) and `PhysicalMemory::from_config()`. Unmapped physical-memory reads return the fixed `OPEN_BUS_BYTE` placeholder (`$FF`); word reads return `OPEN_BUS_WORD` (`$FFFF`). CIA inactive lanes and absent RTC responses use the same placeholder. See the [open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access). The map above describes implemented dispatch rather than every address reserved by Amiga hardware.

### CIA and boot overlay

Within bank `$BF`, the router uses the named CIA-A and CIA-B register addresses in [`cia_reg`](../../../crates/config/src/registers.rs) as canonical addresses. Address bits A7–A1 do not select a register, so addresses with different values in those bits mirror the same register. Address bits A11–A8 choose the CIA register. Each live-read, debug-read and write operation directly matches the masked address once and calls its named register method on the selected instance; CIA receives no motherboard address. Live accesses first synchronize through the selected CIA. An unaccepted retry returns `WaitState` without reading, acknowledging or queuing a write. Selector `$B` is unconnected. A word access splits into two ordered byte accesses, then combines or separates them in big-endian order; only the connected byte lane reaches a CIA. Other addresses read `$FF` and discard writes.

CIA-A pin PA0 drives OVL. When a queued `PRA` or `DDRA` write commits on falling E, CIA-A reports a generic port change. The machine routes it through `MemoryBus::update_ciaa_overlay()` in that same modeled phase, updating the low-memory mapping from `(PRA | !DDRA) & $01`: an input pin is pulled high, while an output pin drives the stored `PRA` bit. OVL high calls `map_kickstart_to_low_memory()`; OVL low calls `map_chip_ram_to_low_memory()`. The pending write leaves the previous mapping intact. CIA-B never drives this helper. This is the effect of the committed CIA pin level, with no additional CCK propagation stage. [CIA](CIA.md#runtime-register-capture-and-bus-synchronization) specifies the separate synchronization, acceptance and capture boundaries. The [8520 port specification](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#io-ports-pra-prb-ddra-ddrb) defines direction control and pull-ups. [vAmiga](../../../ref_src/vAmiga/Core/Components/CIA/CIA.cpp) updates memory selection when PA0 changes, while the [Minimig Gary decoder](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v) and [Amiga Replacement Project Gary decoder](../../../ref_src/amiga_replacement_project/gary/Gary.v) use OVL directly in their address equations. These models support omitting an additional clocked propagation stage; they do not establish zero physical propagation time.

`PhysicalMemory` changes banks `$00-$07` between ROM and Chip RAM handlers. At construction and reset it engages the ROM overlay, consistent with CIA-A PA0 being an input with a pull-up, so reset vectors can be read from low memory. Cold reset clears RAM; warm reset preserves it. Storage mapping and resets belong to physical memory and the machine, not to a persistent `MemoryBus` state.

### Custom registers and specialized chips

For bank `$DF`, `addr & $01FE` selects an even register offset. A word operation reaches a single custom register. A byte read takes the high or low byte of that register according to address bit 0. A byte write duplicates its value onto both halves of a 16-bit word before calling `write_custom_word()`. Register writes call chip interfaces; effects that a chip stages for later propagation are not made instantaneous by the router.

`read_custom_word()` dispatches readable registers to Agnus, Denise, or Paula. Examples include Agnus beam and DMA status, Denise joystick and collision status, and Paula interrupt, audio-control, and disk status. A live `CLXDAT` or `DSKBYTR` read may have a chip-defined side effect. Reading `COPJMP1` or `COPJMP2` strobes Agnus's Copper and returns `$FFFF`. Unhandled reads, including most reads of write-only registers, return `$FFFF`; see the [open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access). `read_custom_word_debug()` uses non-mutating chip inspection methods and does not issue the Copper strobes.

`write_custom_word()` routes `DMACON` to both Agnus and Paula. It routes `BPLCON0`, `DIWSTRT`/`DIWSTOP`, and sprite position/control registers to both Denise and Agnus. The chips independently latch the DIW fields used for vertical bitplane DMA and horizontal pixel clipping. The router sends other listed display, sprite data, color, and input registers to Denise; listed interrupt, disk, serial, potentiometer, and audio registers to Paula; and explicitly listed Agnus registers to Agnus. Unmatched offsets are ignored without staging a chip mutation. `REFPTR`, `VPOSW`, `VHPOSW`, and `COPINS` reach Agnus's delayed mutation queue, but currently have no implemented commit effect. The [register ownership matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) gives the broader register context, while this dispatch is the implemented CPU/Copper write path.

Neither Denise nor Paula owns a `PhysicalMemory` reference. [Agnus](Agnus.md) owns DMA addressing and arbitration. The machine supplies Agnus with Chip RAM during its step, delivers a polled bitplane word to Denise, and handles the currently modeled floppy DMA transfer through physical memory. A pending Copper write goes through `self.memory_bus().write_custom_word(reg, val)`, so Copper and CPU writes share register dispatch. Other cross-chip signals and DMA data movement are coordinated by the machine, not by general CPU address reads through `MemoryBus`.

### RTC and exceptional access paths

The router forwards addresses `$DC0000-$DC003F` to [`RtcMsm6242b`](../../../crates/rtc/src/rtc.rs). The RTC itself returns `$FF` and ignores writes on even bytes or when its model is `None`; on odd bytes it selects one of 16 four-bit registers using `(addr >> 2) & $0F`. Word operations split into two byte lanes in big-endian order. The machine advances the RTC once per subsystem CCK; the router only handles its mapped accesses.

`write_tas_byte()` implements the modeled TAS read-modify-write exception. It checks the physical-memory bank classification: on Chip or Slow RAM it waits if Chip RAM is blocked and otherwise completes without storing the byte. Other targets use the ordinary byte write. This is specific to the TAS write phase; ordinary writes to these RAM banks use their memory handlers.

The bus also exposes inspection paths. `read_word_debug()` uses side-effect-free custom and CIA reads, or physical-memory debug reads, without CPU wait states. `write_bytes_debug()` delegates directly to physical memory for debugger or test injection. The RTC debug word path currently calls its ordinary read method, which is read-only. These APIs are not CPU timing paths.

## Reference Documentation & Upstream Ground Truth

- [8520 I/O ports](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#io-ports-pra-prb-ddra-ddrb), [vAmiga CIA-A](../../../ref_src/vAmiga/Core/Components/CIA/CIA.cpp), [Minimig Gary](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v), and [Amiga Replacement Project Gary](../../../ref_src/amiga_replacement_project/gary/Gary.v): Define port direction and pull-ups, and show the reference models' immediate OVL-based memory selection.
- [Commodore A500 Service Manual, system block diagram](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=6), [bus control](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=9), [arbitration](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=10), and [Gary](https://www.manualslib.com/manual/4225732/Commodore-Computers-A500.html?page=17): Show the CPU/shared-bus latch, address decoding, bus isolation, and delayed CPU acknowledgment.
- [`memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Defines the live address router, register dispatch, CIA/RTC access, debug methods, and TAS handling.
- [`address_bus.rs`](../../../crates/physical_memory/src/address_bus.rs): Defines the access protocol and ready/wait result consumed by the CPU.
- [`physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs): Owns buffers, overlay, contention flag, reset behavior, and the bank-handler entry points.
- [Physical Memory.md](Physical%20Memory.md): Specifies the installed bank map, storage operations, boot overlay, and direct DMA access behind the router.
- [`map.rs`](../../../crates/physical_memory/src/map.rs) and [`presets.rs`](../../../crates/physical_memory/src/presets.rs): Define memory bank handlers and configured address ranges.
- [`machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs): Shows when the router is constructed and how DMA, contention, chip signals, and CPU steps meet.
- [`test_router.rs`](../../../crates/memory_bus/tests/test_router.rs) and [`test_register_wiring.rs`](../../../crates/memory_bus/tests/test_register_wiring.rs): Exercise production router wiring, overlay changes, lane decoding, and register behavior.
- [Machine Loop.md](Machine%20Loop.md) and [DMA.md](DMA.md): Specify the coordinator sequence and Agnus-controlled DMA scheduling beyond the router's scope.
