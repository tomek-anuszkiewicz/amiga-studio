# How the Emulator Models an Amiga 500

This overview explains the execution model and the responsibilities of the main
components. It is a starting point for understanding why timing and shared memory
matter in this emulator. Detailed register behavior and subsystem contracts live
in the [design specifications](../Obsidian/Amiga/Design/General%20Architecture.md).

## A Shared Clock Connects the Components

The project models execution in Color Clocks (CCKs), approximately 3.54 MHz for
PAL and 3.58 MHz for NTSC. The 68000 CPU runs at twice that frequency. A normal
CPU bus cycle takes four CPU clocks, spanning two Color Clocks, conventionally
called CCK1 and CCK2.

This lets the emulator represent when a memory access starts, when data is
transferred, and how other hardware activity affects it. Timing is part of the
behavior being modeled: a correct final register value can still be wrong if a
read or write happens at the wrong time.

The [main-loop specification](../Obsidian/Amiga/Design/Main%20loop%20A500.md)
describes how these phases are scheduled.

## The CPU and Custom Chips Share Memory

Chip RAM is used by both the CPU and the custom chipset. Agnus schedules DMA
activity for display data, sprites, audio, disk transfers, Copper and Blitter
operations. CPU access therefore depends on which bus slots are available.

When a Chip RAM access is blocked, the bus returns a wait state and the CPU keeps
its current instruction micro-step until it can continue. The delay changes
elapsed time without prematurely committing the access. Memory outside the
contended path has different access rules; the
[memory-bus specification](../Obsidian/Amiga/Design/MemoryBus.md)
provides the exact mapping and arbitration behavior.

This coordination is why bus timing matters to visible and audible results. A
CPU write can affect a display register, while DMA may delay the CPU before the
write occurs. Testing either component alone cannot establish the entire interaction.

## Components Have Distinct Responsibilities

The top-level machine owns the CPU, memory bus, Agnus, Denise, Paula and the two
CIAs. The machine loop and bus coordinate their interactions; peer chips do not
hold direct references to one another.

| Component | Main responsibility |
| --- | --- |
| 68000 CPU | Instruction execution, bus requests and exception handling. |
| Memory bus | Address decoding and routing accesses to memory or registers. |
| Agnus | DMA addresses and scheduling, including Copper and Blitter activity. |
| Denise | Display interpretation and video output from latched bus data. |
| Paula | Audio, disk and interrupt-related behavior. |
| CIAs | Timers, ports and peripheral control. |

Agnus supplies Chip RAM DMA addresses; Denise and Paula latch transferred data.
They do not independently read host memory. Register changes and cross-component
signals follow modeled phases and propagation delays, so a write need not affect
another component immediately.

## Separate the Guest Machine from Its Host

The core consumes external buffers and keeps host filesystem, thread and
wall-clock operations outside emulation. This supports both the native desktop
and WebAssembly frontends. Guest memory is big-endian, and invalid guest accesses
must produce the modeled machine behavior rather than panic the host process.

Serializable machine state is separate from runtime handles. The debugger uses
state and execution history to help inspect behavior; the
[debugger guide](debugger.md) explains the available views and controls.

For a source map, continue with the [developer orientation](developers.md).
For the evidence used to verify these relationships, read
[How the Emulator Is Verified](testing.md).
