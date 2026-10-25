---
title: "Motorola 68000 CPU Design Specification"
aliases: ["M68000", "68000 CPU", "MC68000", "CPU Specification"]
tags: ["amiga", "design", "m68000", "cpu", "registers"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-10-25
related: ["[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Physical Memory.md](Physical%20Memory.md)", "[DMA.md](DMA.md)", "[Interrupts.md](Interrupts.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/cpu"
---
# Motorola 68000 CPU Design Specification

The MC68000 is the Amiga 500's general-purpose processor. It fetches instructions,
reads and writes memory-mapped devices, and responds to exceptions and interrupt
levels. Its registers, bus cycles and control signals define its relationship
with the rest of the machine. This note describes that external contract and
the corresponding emulator behavior. It does not catalog instruction semantics
or the emulator's per-instruction execution classes.

The [machine loop](Machine%20Loop.md) owns the CPU and advances it in Color Clock
(CCK) steps. The CPU sees an external [AddressBus](../../../crates/physical_memory/src/address_bus.rs);
[MemoryBus](MemoryBus.md) routes its accesses to RAM, ROM and devices.
[Agnus](Agnus.md) arbitrates Chip RAM access for DMA, while
[Paula's interrupt controller](Interrupts.md) supplies the CPU interrupt level.
The [micro-step specification](CPU%20Micro-Step%20State%20Machine.md) owns the
internal scheduling of instruction phases.

## Programmer-visible state

| Register | Width | Meaning |
| --- | --- | --- |
| D0-D7 | 32 bits each | Data registers. Byte and word writes change the low part and preserve the upper bits. |
| A0-A6 | 32 bits each | Address registers used for pointers and address arithmetic. |
| A7, USP, SSP | 32 bits each | A7 names the active stack pointer. Supervisor mode selects SSP; user mode selects USP. Changing modes switches the active bank without losing the other pointer. |
| PC | 32 bits | Address of the instruction stream. The external address bus uses only its low 24 bits. |
| SR | 16 bits | Trace, privilege, interrupt mask and condition codes. |

The [CPU state](../../../crates/cpu/src/state.rs) also stores a prefetched
instruction word, stopped and halted state, interrupt input and execution
state needed for snapshots. Those fields are emulator state, not additional
programmer-visible registers. MC68000 has no vector base register: exception
vectors begin at physical address $000000.

### Status register

| Bits | Field | Effect |
| --- | --- | --- |
| 15 | T | Trace enable. A completed instruction can request a trace exception. |
| 13 | S | Supervisor mode. Selects SSP rather than USP and permits privileged operations. |
| 10-8 | I2-I0 | Interrupt priority mask, levels 0-7. A supported incoming level must exceed it. |
| 4 | X | Extend for arithmetic across multiple words. |
| 3 | N | Negative condition. |
| 2 | Z | Zero condition. |
| 1 | V | Signed overflow condition. |
| 0 | C | Carry or borrow condition. |

Bits 14, 12, 11 and 7-5 are unused in this model. The low five bits form the
condition code register (CCR); user code can access these condition codes
without gaining access to privileged system fields. Status writes mask
reserved bits. Each instruction determines which condition codes change.

## External bus and timing

The physical MC68000 presents a 24-bit address space and a 16-bit data bus.
Address lines A23-A1 select a word location; upper and lower data strobes
(UDS/LDS) select its byte lanes. Address strobe (AS), read/write (R/W) and
acknowledge (DTACK) distinguish an access and its completion. Function-code
lines identify user/supervisor program or data space. Interrupt-priority
inputs (IPL) and reset/handshake lines provide separate control paths. These
are physical interface concepts; the emulator's AddressBus callbacks do not
expose or verify every electrical pin transition.

An ordinary byte or word bus cycle takes four CPU clocks, or two CCKs. In the
emulator, a read callback occurs in CCK1 and a write callback in CCK2. A read
selects an address and lane, then receives a byte or word from the selected
device. A write presents an address, lane and value, then the selected device
accepts it. A blocked access returns `WaitState`: the CPU keeps that bus phase
pending while the machine continues to advance. Physical S0-S7 timing and
DTACK pin edges are not represented by these callbacks.

Memory is big-endian: the most significant byte of a word is at the lower
address. A 32-bit value occupies two 16-bit transfers; their ordering can
depend on the operation, so it must not be inferred from host byte order.
The CPU keeps 32-bit address arithmetic internally, while external accesses
wrap to 24 bits. Odd-address word or long accesses cause Address Error before
the rejected transfer. Unmapped reads use the [memory subsystem's](Physical%20Memory.md)
fixed open-bus placeholder, $FF for a byte and $FFFF for a word.

### Sharing Chip RAM with DMA

CPU and DMA are separate users of Chip RAM. Agnus schedules DMA slots
and generates DMA addresses; it is not executing CPU instructions. On each CCK,
the machine advances Agnus, passes the resulting Chip RAM contention state to
physical memory, then steps the CPU through MemoryBus. If the CPU attempts a
Chip or installed Slow RAM access in a blocked slot, it waits and retries the
same phase later. It can continue when that access becomes available. This is
slot arbitration, not a fixed alternation of one CPU transfer and one DMA
transfer. Fast RAM and ROM do not use the Chip RAM contention flag. See
[DMA](DMA.md) for slot ownership and [MemoryBus](MemoryBus.md) for routing.

The physical 68000 also has bus-request and bus-grant signals for other bus
masters. This emulator models Agnus's Chip RAM contention through bus results;
it does not expose a complete BR/BG/BGACK pin protocol or prove expansion-bus
arbitration through SingleStep tests.

## Instruction stream and prefetch

Every instruction begins with a 16-bit opcode word. Its upper bits select an
instruction family; other fields may select registers, size, direction or an
effective address. The bit layout depends on the family. An instruction may
continue with extension words containing an immediate value, displacement,
absolute address or register mask. Thus an encoding is at least two bytes and
grows in two-byte words; the CPU fetches them as the instruction needs them.
Register-direct,
address-register indirect, indexed, PC-relative, absolute and immediate forms
are addressing forms, not separate bus protocols. Motorola's programmer's
model is the source for their complete encodings.

The CPU reads ahead into a two-word instruction queue. At a primed boundary
with the next opcode at A, the emulator holds the opcode at A, the following
word at A+2, and an internal stream cursor at A+4. That cursor is not always
the address of the instruction currently executing. Consuming extension words
advances the stream; a branch or exception refills it from a new target.
Prefetched bytes can therefore outlive a guest write to the same address.
Observable fetch order matters for self-modifying code and exception PC
values. The queue fields are an emulator model of observed behavior, not a
claim about hidden silicon registers. Phase-specific ordering belongs to the
[micro-step specification](CPU%20Micro-Step%20State%20Machine.md).

## Exceptions, privilege and interrupts

An exception interrupts ordinary instruction flow. The CPU selects a vector,
enters supervisor mode, uses SSP, saves status and a path-dependent return PC
on the stack, then fetches the handler address and refills its instruction
queue. A normal exception uses a six-byte SR/PC frame. Address Error uses a
seven-word diagnostic frame with the fault address, instruction word and
status of the failed access. The exact saved PC depends on whether the fault
occurred during an operand transfer, instruction fetch or control transfer;
there is no universal `opcode + 2` rule. See the
[pipeline and exception phases](CPU%20Micro-Step%20State%20Machine.md) for
transaction ordering.

| Vector | Cause and boundary |
| --- | --- |
| 0-1 | Reset supplies initial SSP and PC; it does not stack an exception frame. |
| 2 | External Bus Error is not available through the current AddressBus interface. |
| 3 | Address Error on an odd word/long transfer or instruction target. |
| 4, 10, 11 | Illegal opcode and Line A/F decode exceptions. |
| 5-7 | Instruction-generated divide-by-zero, CHK and TRAPV exceptions. |
| 8 | Privilege Violation before a prohibited operation proceeds. |
| 9 | Trace after an eligible instruction. |
| 15, 24, 31 | Uninitialized interrupt, spurious interrupt and level-seven autovector are defined by MC68000; the current A500 interrupt input does not generate these cases. |
| 25-30 | Implemented Amiga autovectors for interrupt levels 1-6. |
| 32-47 | Software TRAP vectors. |

The vector table starts at $000000; each vector is a big-endian longword at
`4 * vector number`. The initial table seen by the CPU depends on the
[boot overlay](#startup-and-memory-overlay). Once the overlay is removed, RAM
at zero can hold a vector table prepared by firmware or software.

A short frame places the saved SR at the final SSP, followed by the saved PC
high and low words. The seven-word Address Error frame instead contains, in
order, an information word, the fault address high and low words, the
instruction word, saved SR, and saved PC high and low words. Its information
word carries read/write status in bit 4, an instruction/not marker in bit 3
and a function code in bits 2-0 alongside retained instruction bits.
The model derives these fields
from the fault path; AddressBus does not expose separate electrical FC or
instruction/not pins.

RESET, STOP, RTE, writes to the full SR and USP transfers require supervisor
mode. A user-mode attempt raises Privilege Violation; reading SR on MC68000 and
CCR operations do not require supervisor mode. The S bit switches the active
stack. Exception entry clears live trace and uses the supervisor stack; RTE
restores the saved SR and return PC, including stack-bank selection.

Trace samples T at instruction entry. An instruction-generated exception
completes before a pending trace; trace precedes a pending interrupt.
Reset, Address Error, Illegal and privilege rejection cancel that pending
trace. A T bit restored by RTE applies to the following instruction. The
original [Motorola manual](https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf)
Section 6.3.8 governs these cases; the local converted text has conflicting
wording for Illegal/privilege and the saved T bit.

Paula collects enabled interrupt sources and presents an IPL level to the CPU
through the machine loop. At an instruction boundary, a supported level 1-6
above SR.I is accepted, saved independently of later pin changes, and handled
through autovector `24 + level`. Entry raises SR.I to the accepted level;
software normally restores the previous status with RTE. The implemented
acknowledge interval is internal: the bus API has no device vector, VPA,
spurious-IACK or CPU-space callback. The physical MC68000 can accept level 7,
but the standard A500 does not generate it and this emulator ignores IPL=7.

### Address Error and fatal halt

Function codes distinguish data-space operand accesses (user 1, supervisor 5)
from instruction-stream accesses (user 2, supervisor 6). A PC-relative operand
is still a data access. An odd branch target faults as an instruction fetch;
an odd operand word/long faults as a data access. The seven-word frame records
that distinction. A fault while constructing an Address Error frame, such as
an odd SSP or invalid vector-3 handler, halts the CPU. An odd handler reached
from an ordinary exception first attempts Address Error. While halted, the
CPU performs no instruction bus activity until external reset. External BERR
is outside the current bus contract and is not simulated by this alignment
path.

STOP is different from a fatal halt. It retains a continuation address and
ceases instruction activity until a pending trace or eligible interrupt wakes
it. The rest of the machine, including DMA and its master CCK, continues to
advance. Idle STOP stepping does not consume CPU clocks.

## Startup and memory overlay

The sequence involves the CPU, motherboard mapping and firmware:

1. At construction, cold reset or warm reset, physical memory maps Kickstart
   ROM over low addresses $000000-$07FFFF. Chip RAM remains a separate buffer
   behind that overlay. The CPU does not control this mapping by itself.
2. CPU reset enters supervisor mode with SR=$2700, clears stop/halt/trace state,
   reads a longword at $000000 into SSP and another at $000004 into PC, then
   primes two instruction words from that PC. The reset API uses debug reads
   without CPU timing; it does not reproduce the physical reset-pin bus sequence.
   An odd initial PC halts immediately.
3. Execution begins at the address supplied by the ROM's initial PC vector.
   The CPU does not search for a boot program or jump to a hardcoded ROM
   address. Kickstart code performs later machine initialization.
4. Firmware can write CIA-A Port A bit 0 to remove the overlay. The machine's
   memory mapping then exposes Chip RAM at zero without copying or erasing it.
   Firmware or later software can populate RAM vectors and configure Paula's
   interrupt enables and devices. Valid vectors must exist in the visible
   memory before software relies on exceptions there. None of those actions
   occurs automatically as part of CPU reset.

The [Amiga hardware manual](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md)
describes ROM at zero during early startup and the CIA-controlled OVL line.
The actual startup path after the initial fetch depends on the supplied ROM
image. Without one, the emulator installs a synthetic vector image for
headless startup; that image is not Kickstart firmware.

### Reset paths

| Event | CPU effect | Machine and memory effect |
| --- | --- | --- |
| Cold machine reset | Clear D0-D7, A0-A6 and USP; set SR=$2700; reload SSP/PC and prefetch. | Reset chips and devices, clear installed RAM, restore ROM overlay and reset master CCK. |
| Warm machine reset, including keyboard reset | Preserve D0-D7, A0-A6 and USP; set SR=$2700; reload SSP/PC and prefetch. | Reset chips and devices, preserve RAM, restore ROM overlay and reset master CCK. |
| Guest `RESET` instruction | Privileged instruction; preserve CPU registers, SR and current execution path while asserting the CPU reset output. | The machine observes that output and resets external devices and overlay; RAM and CPU execution state remain. |

Cold and warm reset are emulator host APIs for external reset events, not
timed electrical reset waveforms. The CPU's guest `RESET` output is distinct
from an incoming reset: it does not reload the CPU's SSP or PC. The guest
instruction has a modeled 132-clock duration and a 124-clock output pulse,
but the supplied vectors constrain aggregate timing rather than electrical
pin edges. Re-engaging the overlay can change what a later fetch at a low
address sees; already prefetched words are governed by the queue.
The physical HALT input is not an independent input in this emulator;
its fatal halted state and the STOP instruction are modeled separately.

## Amiga-specific TAS bus behavior

TAS performs a read-modify-write transfer and sets flags from the original
byte. On this Amiga bus, Chip and Slow RAM discard the TAS write phase while
Fast RAM permits it; other banks use ordinary byte routing. The CPU issues a
distinct TAS write through AddressBus;
[MemoryBus](MemoryBus.md) applies the bank-specific effect. This is a visible
CPU/motherboard interaction, not a general rule for ordinary memory writes
or a reason to document every instruction here. The
[platform catalog](Platform%20Quirks%20and%20Invariants%20Catalog.md) indexes
other ordering and fault cases that need special attention.

## Evidence and scope

[SingleStepTests](Verification/CPU%20SingleStepTests.md) compare completed CPU
state, instruction clocks and timed bus callbacks against generated reference vectors.
They constrain observable instruction behavior but run with a synthetic test
bus; they do not exercise the production MemoryBus, CIA overlay, Agnus DMA
arbitration or electrical pin levels. Machine and bus integration have their
own tests and specifications. This note states architectural boundaries and
the currently modeled behavior, including unsupported external BERR and
level-seven interrupt handling; the instruction vectors do not define a
unique internal CPU implementation.

## Reference Documentation & Upstream Ground Truth

- [Motorola User's Manual Section 2](../Reference/68000%20User's%20Manual/02%20-%20Section%202%20-%20Introduction%20%26%20Programmer's%20Model.md): Register widths, status bits and addressing model.
- [Motorola User's Manual Section 5](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md): Physical read/write bus cycles and strobes.
- [Motorola User's Manual Section 6](../Reference/68000%20User's%20Manual/06%20-%20Section%206%20-%20Exception%20Processing,%20Stack%20Frames%20%26%20Reset.md): Vector and stack-frame reference; disputed trace wording requires the original PDF.
- [Amiga Hardware Reference Manual Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): ROM overlay, early startup and A500 interrupt levels.
- [CPU implementation](../../../crates/cpu/src/cpu.rs) and [state](../../../crates/cpu/src/state.rs): Reset, stepping, interrupt input and modeled registers.
- [Machine loop](../../../crates/machine_loop/src/machine_loop.rs) and [physical memory](../../../crates/physical_memory/src/physical_memory.rs): Reset routing, CCK coordination, contention and ROM overlay.
- [SingleStep verification contract](Verification/CPU%20SingleStepTests.md): Exact vector comparison and its observation limits.
