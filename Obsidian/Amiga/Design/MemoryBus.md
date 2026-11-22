---
title: "MemoryBus"
aliases: ["Amiga 500 MemoryBus", "Motherboard Address Router"]
tags: ["amiga", "design", "memory_bus", "physical_memory"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-08-31
updated: 2026-11-16
related: ["[General Architecture.md](General%20Architecture.md)", "[Physical Memory.md](Physical%20Memory.md)", "[Machine Loop.md](Machine%20Loop.md)", "[DMA.md](DMA.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[RTC.md](RTC.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/memory_bus"
---
# MemoryBus

The M68000 requests a read or write through the address-bus interface.
The motherboard router decodes the address and sends the access to
physical memory, a custom-chip register, a CIA, or the RTC.
[`MemoryBus<'a>`](../../../crates/memory_bus/src/memory_bus.rs),
[`AddressBus`](../../../crates/physical_memory/src/address_bus.rs), and
[`PhysicalMemory`](../../../crates/physical_memory/src/physical_memory.rs)

The machine owns RAM, the chips, and the machine clock. When it needs the
router, it creates it from temporary mutable references to the components.
[`A500Machine`](../../../crates/machine_loop/src/machine_loop.rs)

Through `AddressBus`, a CPU access returns `BusResult::Ready(value)` or
`BusResult::WaitState`. `PhysicalMemory` implements the same interface for
direct or synthetic use. During production stepping,
`A500Machine::step_cck()` supplies `MemoryBus`. The CPU handles instruction
sequencing and address errors. The router only dispatches accesses. The
machine loop advances time.

## A CPU access through the router

`A500Machine::step_cck()` increments its CCK counter and advances the other
subsystems before the CPU. During that work, Agnus advances against Chip
RAM and the machine copies `agnus.chip_ram_blocked` into
`physical_memory.chip_ram_blocked`. The machine then creates `MemoryBus`
from mutable references to physical memory, Agnus, Denise, Paula,
CIA-A, CIA-B, and RTC, and passes it to `cpu.step_cck()`.

A CPU access to Chip or Slow RAM therefore sees the contention state for
that step. A `WaitState` keeps the CPU bus micro-step pending while the
rest of the machine continues to advance on subsequent CCKs. The machine
loop specifies the complete step order. The DMA specification defines the
Agnus slot schedule. [Machine Loop](Machine%20Loop.md) and [DMA](DMA.md)

When the production CPU path refuses a Chip or Slow RAM access or a
custom-register access, it records a shared-bus wait in Agnus. The first
refusal starts the denial count. Agnus counts subsequent CCKs while the
request is pending. A successful access releases it. Internal CPU work
and private RAM or ROM accesses create no such request and impose no
Blitter yield.

Before a CPU access to a custom register has a live-read effect or
inserts a register entry, it checks bus availability. The common Copper
and DMA issue methods use their separate path for already-granted access.
The CPU keeps the refused operation in its existing micro-step. The
chipset holds no second copy of its address or write data. These rules
define functional request and retry behavior. Sub-CCK calibration of
Gary and CPU edges remains outside reconstruction acceptance. [DMA](DMA.md)

On an A500 motherboard, Gary decodes the CPU address to select RAM, ROM,
or a device. It also controls the connection between the CPU data bus
and the shared Chip RAM data bus. Agnus schedules access to the shared
bus, including which DMA channel runs, and generates the DRAM addresses
and timing. When Agnus requests the bus, Gary delays the CPU transfer
acknowledgment until the bus is available. The pending CPU access is what
the Chip or Slow RAM wait state models. [Agnus](Agnus.md)

A bidirectional, tri-state data latch connects the CPU and shared data
buses. While enabled it passes the input word. When closed it retains the
last sampled word. Its outputs can also disconnect electrically. Gary
sets the direction for a granted CPU transfer and isolates the CPU side
from the shared bus during an Agnus DMA transfer.

The motherboard buffer holds data in transit and isolates the buses. It
provides neither additional RAM storage nor a cache. Denise and Paula
capture display or audio DMA data in their own holding registers when
Agnus selects the corresponding register. The emulator reproduces the
resulting address routing and CPU wait state. It has no separate Gary
component or motherboard latch state.

For every byte or word access, `MemoryBus` masks the address to 24 bits
and uses the upper eight bits (`addr >> 16`) to select the bank. It routes
banks `$A0-$BF` to the CIA decoder, `$DC` to RTC, and `$DE-$DF` to the
shared custom-register decoder. It sends other banks to `PhysicalMemory`.
Shared bank bounds keep production routing and physical classification
consistent. The physical bank table uses device classes for placeholder
responses. Live chip access goes through the router.
[`address_map.rs`](../../../crates/config/src/address_map.rs)

| Address selected by the implementation | CPU access destination and behavior |
| --- | --- |
| `$000000-$07FFFF` | Kickstart ROM while the low-memory overlay is active; otherwise 512 KB Chip RAM. The physical-memory bank handlers make this selection. |
| `$200000-$5FFFFF` | Optional 4 MB Fast RAM in the expanded preset. Other addresses without configured storage use open bus. |
| `$A00000-$BFFFFF` | CIA selection by active-low A13/A12, register by A11..A8; CIA-B reads on the even/high lane and CIA-A on the odd/low lane. See the selection contract below. |
| `$C00000-$C7FFFF` | Optional 512 KB Slow RAM in standard and expanded presets. |
| `$DC0000-$DC003F` | RTC byte interface; its device implementation accepts odd addresses when a clock is installed. Other addresses in bank `$DC` return the fixed `$FF` placeholder on reads and ignore writes. |
| `$DE0000-$DFFFFF` | Custom-register routing, using the low register-offset bits. These addresses alias the same 512-byte register window in the current router. |
| `$F80000-$FFFFFF` | Kickstart ROM. The fixed 256 KiB image repeats across this 512 KiB range. CPU ROM writes are ignored. |

The map lists the emulator's implemented dispatch. Hardware address
reservations have a separate scope. The preset table and installed storage
come from the preset definitions and `PhysicalMemory::from_config()`.
[`presets.rs`](../../../crates/physical_memory/src/presets.rs)

Unmapped byte reads in physical memory return the fixed `OPEN_BUS_BYTE`
placeholder (`$FF`). Word reads return `OPEN_BUS_WORD` (`$FFFF`).
Inactive CIA lanes and absent RTC responses use the same placeholder.
[Open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access)

### CIA and boot overlay

CIA-A's canonical addresses are `$BFE001-$BFEF01`. CIA-B uses
`$BFD000-$BFDF00`. Within banks `$A0-$BF`, A12 low selects CIA-A and A13
low selects CIA-B.

| A13/A12 | Receivers | Word read lanes |
| --- | --- | --- |
| `00` | Both | CIA-B high, CIA-A low |
| `01` | CIA-B | CIA-B high, `$FF` low |
| `10` | CIA-A | `$FF` high, CIA-A low |
| `11` | Neither | `$FFFF` |

A11..A8 select the register. A20..A14 and A7..A1 are mirrors. For each
selected chip, the decoder finds the full named canonical address and
calls that instance's concrete register method. It passes no motherboard
address to the CIA. [`cia_reg`](../../../crates/config/src/registers.rs)

Selector B reads `$FF`, ignores writes, and does not synchronize. Byte
reads reach only the selected chip on the addressed lane. Inactive lanes
return fixed `$FF` without read effects. Byte writes reach every selected
chip regardless of parity. Word writes pass the high byte to CIA-B and
the low byte to CIA-A.

The first access attempt selects the receivers and starts each selected
CIA's synchronization countdown. A word read or a write can affect a
register only after all receiving CIAs have synchronized and the transfer
has been accepted. An accepted write commits on falling E one CCK later,
before the next synchronized transfer. This spacing keeps legal CPU traffic
below the capacity of the sixteen-slot CIA buffers.

Before a CIA write uses the synchronization state or stages an entry, it
checks all selected queues. An attempt to insert beyond capacity is
fatal and offers no WaitState retry. Direct setter injection must drain
before CPU traffic resumes. [CIA](CIA.md#runtime-register-capture-and-bus-synchronization)

Only an accepted transfer can consume synchronization, acknowledge a
read, or insert a write in a receiver. Acceptance consumes synchronization
for all ready receivers together, with one event per selected chip. A word
transfer has a single acceptance across its byte lanes. Debug reads use
the same selection and canonical register identities without clocking,
synchronization, or read effects.

The A500 Rev. 6A/7 CIA sheet shows U37 combining `VMA*` with A12/A13
for the two independent active-low selects. It also shows A11..A8 feeding
RS3..RS0 and CIA-A/B wired to D7..D0/D15..D8.
[CIA sheet](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=7)

The processor sheet shows Gary's VPA and the latches between the CPU and
shared buses. [Processor sheet](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=3)

The address-decode paragraph in HRM Appendix F prescribes canonical
addresses for software. Its binary and parity examples are inconsistent
and leave the full set of electrical aliases unspecified. The approved
model decodes `$A0-$BF` by qualifying the upper three bits as `101`, in
line with the Minimig CIA equations.
[Minimig CIA equations](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v)

vAmiga's memory decoder corroborates CIA selection, write fan-out, and
`$DE/$DF` custom routing. vAmiga sometimes uses the CPU instruction register
for inactive read lanes. This emulator uses fixed `$FF`.
[vAmiga's memory decoder](../../../ref_src/vAmiga/Core/Components/Memory/Memory.cpp)

The replacement Gary at revision `143ad5f1b5acae093ea66186e832a8ab4fb1bc0f`
qualifies CIA with A23..A20=`1011`. It also decodes RTC, ROM, and custom
mirrors outside this task. Those differences remain outside the implemented
map. The models describe their own decoding and leave measured silicon
timing and other board revisions unverified.
[Replacement Gary](../../../ref_src/amiga_replacement_project/gary/Gary.v)

The contract covers the supported OCS 8371 PAL and 8370 NTSC presets.
Installed RAM and RTC use their specified ranges. Additional
`$C0-$DD` custom mirrors remain disabled.

CIA-A pin PA0 drives OVL. When a queued `PRA` or `DDRA` write commits
on falling E, CIA-A reports a generic port change. In the same modeled
phase, the machine routes the change through `MemoryBus::update_ciaa_overlay()`
and derives the pin level from `(PRA | !DDRA) & $01`. An input pin is
pulled high. An output pin drives the stored `PRA` bit. When OVL is high
the router calls `map_kickstart_to_low_memory()`. When OVL is low it calls
`map_chip_ram_to_low_memory()`. The previous mapping stays in effect
while a write is pending. CIA-A alone drives this helper. The committed
pin level takes effect without an additional CCK propagation stage. The
CIA specification defines the separate synchronization, acceptance, and
capture boundaries. [CIA](CIA.md#runtime-register-capture-and-bus-synchronization)

The 8520 port specification defines direction control and pull-ups.
[8520 port specification](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#io-ports-pra-prb-ddra-ddrb)

vAmiga changes memory selection when PA0 changes. The Minimig and Amiga
Replacement Project Gary decoders use OVL directly in their address
equations. These models support omitting an additional clocked stage.
Physical propagation time remains unmeasured by this evidence.
[vAmiga](../../../ref_src/vAmiga/Core/Components/CIA/CIA.cpp),
[Minimig Gary decoder](../../../ref_src/minimig/software%20and%20firmware/old/source/fpga%20core/Gary.v),
and [Amiga Replacement Project Gary decoder](../../../ref_src/amiga_replacement_project/gary/Gary.v)

`PhysicalMemory` switches banks `$00-$07` between ROM and Chip RAM
handlers. At construction and reset, it engages the ROM overlay so the
CPU can read reset vectors from low memory. At both points, CIA-A pin PA0
is an input pulled high, which matches the overlay setting. Cold reset
clears RAM. Warm reset preserves it. Physical memory and the machine own
storage mapping and resets. The temporary `MemoryBus` carries no persistent
state for either.

### Custom registers and specialized chips

In banks `$DE` and `$DF`, `addr & $01FE` selects an even register offset.
A word operation reaches a single custom register. A byte read takes
the high or low byte according to address bit 0. A byte write duplicates
its value onto both halves of a 16-bit word, then calls
`write_custom_word()`.

For a modeled data write, one offset match calls a concrete queued
setter on each receiving chip. While a value is pending, reads, child
components, restart actions, and outputs still see the previous state.
Address strobes act as soon as they are accepted and use no queue entry.
The router ignores unmatched write offsets.

`read_custom_word()` routes readable registers to Agnus, Denise, or Paula.
Examples include Agnus beam and DMA status, Denise joystick and collision
status, and Paula interrupt, audio-control, and disk status. A live
`CLXDAT` or `DSKBYTR` read may have a side effect defined by its chip.
Reading `COPJMP1` or `COPJMP2` strobes Agnus's Copper and returns `$FFFF`.
Unhandled reads, including most reads of write-only registers, also return
`$FFFF`. `read_custom_word_debug()` inspects the chips without mutating
state or issuing Copper strobes.
[Open-bus quirk](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access)

`write_custom_word()` routes `DMACON` to Agnus and Paula. It routes
`BPLCON0`, `DIWSTRT`/`DIWSTOP`, and sprite position and control registers
to Denise and Agnus. Each chip independently latches the DIW fields
it uses, for vertical bitplane DMA in Agnus and horizontal pixel clipping
in Denise. The router sends the other listed display, sprite data, color,
and input registers to Denise. It sends listed interrupt, disk, serial,
potentiometer, and audio registers to Paula and the explicitly listed
Agnus registers to Agnus. Unmatched offsets stage no chip mutation.
This dispatch is the implemented CPU and Copper write path. The ownership
matrix provides the broader register context.
[Register ownership matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)

The custom chips hold accepted writes in ordered queues with fixed
capacity. Each setter reports whether it accepted the write. For a
shared register, the router checks every required receiver's capacity
before inserting any entry. If a queue would overflow, the model stops
before that write changes any recipient. Ordinary bus contention instead
returns `WaitState` for retry. After a successful insertion the CPU receives
`Ready`, and the write commits later.

All custom data writes currently use `Pipeline` to preserve every accepted
operation. The model uses the concrete reference timings recorded in the
timing research and falls back to **1 CCK** where timing is unresolved.
Neither the reference timings nor the fallback establish universal
silicon timing. A read observes current state at accepted selection
without flushing pending writes. CIA capture and RTC access follow their
own contracts.

DMACON retains **2 CCK** for Agnus and Paula. BPLCON0 and DIWSTRT/STOP
retain **4 CCK** for Agnus and **1 CCK** for Denise. Agnus retains
**4 CCK** for DDFSTRT/STOP. INTENA/INTREQ retain **1 CCK**. Other custom
data writes use the **1 CCK** fallback, including pointers, palette,
audio data and controls, and Blitter/Copper data registers.

Address strobes COPJMP1/2 and STREQU/STRVBL/STRHOR/STRLONG act immediately
without a queue capacity check. Data-bearing triggers such as BLTSIZE
and BPL1DAT take effect when their queued data commits. The engine
schedules subsequent work separately. Paula samples electrical
INT2/INT3/INT6 levels after applying due writes and uses no register
queue slot for them. [Paula](Paula.md#register-propagation)

These `delay_cck` values and per-register `MutationMode` choices
describe the current implementation. The one-CCK fallback and the
concrete reference-model delays are both provisional.
[Register mutation parameter policy](../../../.agents/references/register-mutation-modeling.md#provisional-register-mutation-parameters)

A write issued after a chip step starts counting its delay on the chip's
next step. After all three custom chips step, the machine issues Copper
register writes and then bitplane DMA data. The CPU issues its accesses
afterward, in the CPU phase. Recipients therefore count from the same
boundary regardless of host method ordering. This provisional order
defines deterministic model behavior. Priority during simultaneous
physical accesses remains unverified by it.

CPU CCK1/CCK2 acceptance follows the CPU's existing bus protocol. A
transfer inserts an entry only if accepted. Live COPJMP reads keep their
immediate read-side strobe and fixed open-bus result. Autonomous DMA
pointer increments, counter changes, pin sampling, and interrupt
generation are separate operations from register-bus writes.

Agnus owns DMA addressing and arbitration. The machine gives Agnus
access to Chip RAM during its step, polls for a bitplane word to deliver
to Denise, and handles the currently modeled floppy DMA transfer through
physical memory. Denise and Paula hold no `PhysicalMemory` reference.
[Agnus](Agnus.md)

The machine passes a pending Copper write through
`self.memory_bus().write_custom_word(reg, val, producer)`. Copper and
CPU writes therefore use the same register dispatch. For other cross-chip
signals and DMA data movement, the machine coordinates the transfer
without going through general CPU address reads in `MemoryBus`.

#### Owner delivery and functional reconstruction

The register matrix identifies the documented hardware registers and the
corresponding modeled dispatch. The live router supplies the named
interfaces under the timing and acceptance rules above.
[Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md#modeled-interfaces-and-timing)
and [router](../../../crates/memory_bus/src/custom_registers.rs)

Agnus owns Copper and Blitter register delivery. A due `(offset, value)`
returned by the owner shows that it consumed its event. Checking functional
correctness also requires observing the engine.

Guest programs exercise selected reconstructed register and receiver
effects. Other register behavior falls outside what these cases establish.
[Native disk verification](../../../ROADMAP.md#native-disk-verification)

Serializable chip state retains every pending custom event and any Copper
or bitplane output still waiting to enter a destination queue. Reset cancels
pending events. Loading a snapshot restores the remaining delays without
reissuing a write. If a CPU transfer receives `WaitState`, the CPU keeps
the same active micro-step and payload for the next attempt. An entry can
become due only after its transfer is accepted.

For a shared write, the preflight check stops the failed owner on overflow
before any destination accepts the write. Paula has 32 register-write
slots. Agnus and Denise each have 64. Interrupt input levels use no
register slot.

Each custom chip step returns every matured entry in retained order as a
borrowed slice. The shared queue owns occupied storage, the lifetime of
due results, and serialization. Acceptance, commitment, and pin sampling
keep their existing phases if the queue representation changes.
[Shared queue](Configuration.md#shared-propagation-state)

### RTC and exceptional access paths

The router forwards addresses `$DC0000-$DC003F` to the RTC. Word
operations split into two byte lanes in big-endian order. On even bytes
or when its model is `None`, the RTC returns `$FF` and ignores writes.
For an odd byte with a clock installed, it selects one of 16 four-bit
registers using `(addr >> 2) & $0F`.
[`RtcMsm6242b`](../../../crates/rtc/src/rtc.rs)

The RTC calendar uses the latest supplied host UTC sample and its
independent battery offset. The router handles mapped register accesses
without advancing the clock or reading host time. [RTC calendar](RTC.md)

`write_tas_byte()` checks the bank classification for the write phase
of a TAS read-modify-write operation. For Chip or Slow RAM, it waits if
Chip RAM is blocked. Once granted, the access completes without storing
the byte. Other targets use the ordinary byte write, and ordinary writes
to these RAM banks still use their memory handlers.

Inspection paths bypass CPU timing. `read_word_debug()` uses
side-effect-free custom and CIA reads or physical-memory debug reads
without CPU wait states. The RTC debug word path currently calls
the ordinary RTC read method, which is read-only. `write_bytes_debug()`
goes directly to physical memory for debugger or test injection.

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
