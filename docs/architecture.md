# How the Emulator Models an Amiga 500

The emulator models how the Amiga 500 components share memory and coordinate
their work over time. The design specifications define register behavior and the
requirements for each subsystem.
[Design specifications](../Obsidian/Amiga/Design/General%20Architecture.md)

## A Shared Clock Connects the Components

The project models execution in Color Clocks (CCKs), approximately 3.54 MHz for
PAL and 3.58 MHz for NTSC. The 68000 CPU runs at twice that frequency. A normal
CPU bus cycle takes four CPU clocks, spanning two Color Clocks, conventionally
called CCK1 and CCK2.

These phases let the emulator model when an access starts, when it transfers
data and how other hardware affects it. A register can end up with the expected
value even when the read or write occurred in the wrong phase. The main-loop
specification describes how the emulator schedules each phase.
[Main-loop specification](../Obsidian/Amiga/Design/Machine%20Loop.md)

## The CPU and Custom Chips Share Memory

The CPU and the custom chipset share Chip RAM. Agnus schedules DMA activity for
display data, sprites, audio, disk transfers, Copper and Blitter operations. The
CPU can use Chip RAM when a bus slot is available to it.

When the CPU requests a Chip RAM access and the slot is unavailable, the bus
returns a wait state. The CPU holds its current instruction micro-step and the
access remains pending while emulated time advances. The memory-bus
specification gives the exact address mapping and rules for granting access,
including the different rules for memory outside the contended path.
[Memory-bus specification](../Obsidian/Amiga/Design/MemoryBus.md)

Bus timing affects what appears on the display and what the audio plays. For
example, DMA can delay a CPU write to a display register. Testing either
component alone leaves the CPU and DMA interaction unverified.

## Components Have Distinct Responsibilities

The top-level machine owns the CPU, memory bus, Agnus, Denise, Paula and the two
CIAs. The machine loop and bus coordinate their interactions. Each chip operates
without a direct reference to its peers.

| Component | Main responsibility |
| --- | --- |
| 68000 CPU | Instruction execution, bus requests and exception handling. |
| Memory bus | Address decoding and routing accesses to memory or registers. |
| Agnus | DMA addresses and scheduling, including Copper and Blitter activity. |
| Denise | Display interpretation and video output from latched bus data. |
| Paula | Audio, disk and interrupt-related behavior. |
| CIAs | Timers, ports and peripheral control. |

Agnus supplies the Chip RAM DMA addresses, and Denise and Paula latch the data
transferred on the bus. Neither Denise nor Paula reads host memory independently.
Register changes and signals between components follow modeled clock phases and
propagation delays. A write may therefore take effect in another component later.

## Separate the Guest Machine from Its Host

The emulation core receives buffers from its caller. Keeping filesystem,
thread and wall-clock operations outside emulation lets the native desktop and
WebAssembly frontends use the same core. Guest memory is big-endian. If the
guest makes an invalid access, the emulator must reproduce the modeled machine's
response and keep the host process running.

The core can serialize machine state independently of its runtime handles. The
debugger uses machine state and execution history to let you inspect behavior.
[Debugger views and controls](debugger.md)

For a source map, continue with the [developer orientation](developers.md).
For the evidence used to verify these relationships, read
[How the Emulator Is Verified](testing.md).
