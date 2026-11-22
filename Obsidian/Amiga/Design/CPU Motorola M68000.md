---
title: "Motorola 68000 CPU Design Specification"
aliases: ["M68000", "68000 CPU", "MC68000", "CPU Specification"]
tags: ["amiga", "design", "m68000", "cpu", "registers"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-11-19
related: ["[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Physical Memory.md](Physical%20Memory.md)", "[DMA.md](DMA.md)", "[Interrupts.md](Interrupts.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/cpu"
---
# Motorola 68000 CPU Design Specification

The MC68000 is the Amiga 500's general-purpose processor. It fetches instructions,
reads and writes memory-mapped devices, and responds to exceptions and interrupt
levels. Its registers, bus cycles and control signals define how it works with
the rest of the machine. This note describes that interface and the corresponding
emulator behavior. Instruction semantics and the emulator's per-instruction
execution classes are outside its scope.

The [machine loop](Machine%20Loop.md) owns the CPU and advances it in Color Clock
(CCK) steps. The CPU requests each access through an external
[AddressBus](../../../crates/physical_memory/src/address_bus.rs). [MemoryBus](MemoryBus.md)
routes the requests to RAM, ROM and devices.
[Agnus](Agnus.md) arbitrates Chip RAM access for DMA, while
[Paula's interrupt controller](Interrupts.md) supplies the CPU interrupt level.
The internal scheduling of instruction phases is defined in the
[micro-step specification](CPU%20Micro-Step%20State%20Machine.md).

## Programmer-visible state

| Register | Width | Meaning |
| --- | --- | --- |
| D0-D7 | 32 bits each | Data registers. Byte and word writes change the low part and preserve the upper bits. |
| A0-A6 | 32 bits each | Address registers used for pointers and address arithmetic. |
| A7, USP, SSP | 32 bits each | A7 names the active stack pointer. Supervisor mode selects SSP; user mode selects USP. Changing modes switches the active bank without losing the other pointer. |
| PC | 32 bits | Address of the instruction stream. The external address bus uses only its low 24 bits. |
| SR | 16 bits | Trace, privilege, interrupt mask and condition codes. |

The emulator keeps a prefetched instruction word, stopped and halted state,
interrupt input and execution state in the running CPU state and records
them in snapshots. These internal fields are outside the programmer-visible
register set. [CPU state](../../../crates/cpu/src/state.rs)

MC68000 exception vectors start at physical address $000000. The MC68000 has no
vector base register.

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
condition code register (CCR). User code can access these condition codes while
the system fields stay privileged. Status writes mask reserved bits. Each
instruction determines which condition codes change.

## External bus and timing

The physical MC68000 presents a 24-bit address space and a 16-bit data bus.
Address lines A23-A1 select a word location. The upper and lower data strobes
(UDS/LDS) select its byte lanes. Address strobe (AS), read/write (R/W) and
acknowledge (DTACK) distinguish the access and its completion. Function-code
lines identify user or supervisor program or data space. Interrupt-priority
inputs (IPL) and reset and handshake lines provide separate control paths.
The AddressBus callbacks expose selected parts of that interface. Verifying
the full sequence of electrical pin transitions requires other evidence.

An ordinary byte or word bus cycle takes four CPU clocks, or two CCKs. In the
emulator, a read callback occurs in CCK1 and a write callback in CCK2. A read
selects an address and lane, then receives a byte or word from the selected
device. A write presents an address, lane and value for the selected device to
accept. A blocked access returns `WaitState`. The CPU then holds that bus phase
pending while the machine continues to advance. Physical S0-S7 timing and DTACK
pin edges are outside the callback model.

Memory is big-endian. The most significant byte of a word is at the lower
address. A 32-bit value takes two 16-bit transfers. Use the order specified by
the operation, since host byte order cannot determine the transfer order.
The CPU keeps 32-bit address arithmetic internally, while external accesses
wrap to 24 bits. Odd-address word or long accesses cause Address Error before
the rejected transfer. Unmapped reads use the [memory subsystem's](Physical%20Memory.md)
fixed open-bus placeholder, $FF for a byte and $FFFF for a word.

### Sharing Chip RAM with DMA

The CPU and DMA share Chip RAM. Agnus schedules DMA slots and generates DMA
addresses while the CPU executes instructions. On each CCK, the machine
advances Agnus, passes the resulting Chip RAM contention state to physical
memory, then steps the CPU through MemoryBus. If a CPU access to Chip or
installed Slow RAM hits a blocked slot, the CPU holds that phase and retries
the access later. The access can proceed when a slot is available. Slot
arbitration determines the order of CPU and DMA accesses, so they need not
alternate. Fast RAM and ROM are outside the scope of the Chip RAM contention
flag. See [DMA](DMA.md) for slot ownership and [MemoryBus](MemoryBus.md) for routing.

The physical 68000 also has bus-request and bus-grant signals for other bus
masters. In this emulator, bus results convey Agnus's Chip RAM contention.
The complete BR/BG/BGACK pin protocol is outside the model, and SingleStep
tests provide no evidence of expansion-bus arbitration.

## Instruction source layout

Keep `crates/cpu/src/instructions/` flat, with one mnemonic per file. The rule
prohibits subdirectories and umbrella files that group distinct mnemonics.
Exceptions for oversized instructions still use flat files. The exact grouping
exceptions are `move_sr_ccr.rs` and `logic_sr_ccr.rs`, the size-based files
`move_b.rs`, `move_w.rs` and `move_l.rs`, and the condition-family files
`bcc.rs`, `dbcc.rs` and `scc.rs`.

## Instruction stream and prefetch

Every instruction begins with a 16-bit opcode word. Its upper bits select an
instruction family. The family determines how to interpret other fields, which
may select registers, size, direction or an effective address. Extension words
can hold an immediate value, displacement, absolute address or register mask.
An encoding starts at two bytes and grows in two-byte words, which the CPU
fetches as needed. Register-direct, address-register indirect, indexed,
PC-relative, absolute and immediate are addressing forms. Choosing a form
does not introduce a separate bus protocol. Motorola's programmer's model
defines their complete encodings.

The CPU reads ahead into a two-word instruction queue. At an instruction
boundary where the queue is full and the next opcode is at A, the emulator
holds the opcode at A and the following word at A+2. The internal stream
cursor is then at A+4 and can point beyond the instruction currently executing.
Consuming extension words advances the stream. A branch or exception refills
the queue from a new target. After a guest write, the queue can still hold
bytes it read earlier from that address. This fetch order matters for
self-modifying code and exception PC values. The queue fields model observed
behavior. The silicon's hidden registers are outside this description.
See the [micro-step specification](CPU%20Micro-Step%20State%20Machine.md) for
the ordering of actions by phase.

## Exceptions, privilege and interrupts

An exception interrupts ordinary instruction flow. The CPU selects a vector
and enters supervisor mode. Using SSP, it saves status and a return PC on the
stack. It then fetches the handler address and refills its instruction queue.
A normal exception uses a six-byte SR/PC frame. Address Error uses a
seven-word diagnostic frame with the fault address, instruction word and
status of the failed access. The saved PC depends on whether the fault occurred
during an operand transfer, instruction fetch or control transfer. A universal
`opcode + 2` formula would miss those distinctions. See the
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

Missing external exception inputs are tracked in the [deferred bus and interrupt work](../../../ROADMAP.md#deferred-bus-and-peripherals).

The vector table starts at $000000. Each vector is a big-endian longword at
`4 * vector number`. The CPU's initial view of the table depends on the
[boot overlay](#startup-and-memory-overlay). Once the overlay is removed, RAM
at zero can hold a vector table prepared by firmware or software.

A short frame places the saved SR at the final SSP, followed by the high and
low words of the saved PC. The seven-word Address Error frame begins with an
information word, then holds the high and low words of the fault address, the
instruction word, the saved SR, and the high and low words of the saved PC,
in that order. The information word carries read/write status in bit 4, an
instruction/not marker in bit 3 and a function code in bits 2-0 alongside
retained instruction bits. The model derives these fields from the fault path.
Separate electrical FC or instruction/not pins are outside the AddressBus
interface.

RESET, STOP, RTE, writes to the full SR and USP transfers require supervisor
mode. A user-mode attempt raises Privilege Violation. On MC68000, user code
can read SR and perform CCR operations. The S bit switches the active stack.
Exception entry clears live trace and uses the supervisor stack. RTE restores
the saved SR and return PC, including stack-bank selection.

The CPU samples T at instruction entry. An instruction-generated exception
completes before a pending trace, and trace precedes a pending interrupt.
Reset, Address Error, Illegal and privilege rejection cancel the pending
trace. A T bit restored by RTE applies to the following instruction. The local
converted text has conflicting wording about Illegal, privilege rejection and
the saved T bit. Section 6.3.8 of the original
[Motorola manual](https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf)
governs these cases.

Paula collects enabled interrupt sources. The machine loop passes the resulting
IPL level to the CPU. At an instruction boundary, the CPU accepts a supported
level 1-6 above SR.I and records the accepted level, so later pin changes have
no effect on that decision. The CPU handles the interrupt through autovector
`24 + level` and raises SR.I to the accepted level on entry. Software normally
restores the previous status with RTE. The implemented acknowledge interval
runs inside the CPU. The bus API has no device vector, VPA, spurious-IACK or
CPU-space callback. The physical MC68000 can accept level 7. The standard
A500 does not generate it, and the emulator ignores IPL=7.

### Address Error and fatal halt

Function codes distinguish data-space operand accesses (user 1, supervisor 5)
from instruction-stream accesses (user 2, supervisor 6). PC-relative operands
use data space. The seven-word frame records whether a fault occurred during
an instruction fetch, as with an odd branch target, or during a data access,
as with an odd word or long operand address. A fault while constructing an
Address Error frame, such as an odd SSP or invalid vector-3 handler, halts the
CPU. An odd handler reached from an ordinary exception first attempts Address
Error. The halted CPU resumes instruction bus activity only after external
reset. External BERR is outside the current bus contract and is not simulated
by this alignment path.

STOP retains a continuation address and pauses instruction activity until a
pending trace or eligible interrupt wakes the CPU. The rest of the machine,
including DMA and its master CCK, continues to advance. Idle STOP stepping
consumes no CPU clocks.

## Startup and memory overlay

Startup begins with the motherboard's memory mapping. With Kickstart supplied,
the CPU reads its initial vectors through that mapping and begins executing
firmware at the entry point given by the PC vector.

1. At construction, cold reset or warm reset, physical memory maps Kickstart
   ROM over low addresses $000000-$07FFFF. Chip RAM remains a separate buffer
   behind that overlay. The CPU does not control this mapping by itself.
2. CPU reset enters supervisor mode with SR=$2700, clears stop/halt/trace state,
   reads a longword at $000000 into SSP and another at $000004 into PC, then
   primes two instruction words from that PC. The reset API uses debug reads
   without CPU timing. The physical reset-pin bus sequence is outside that API.
   An odd initial PC halts the CPU immediately.
3. Execution begins at the address supplied by the ROM's initial PC vector.
   The CPU does not search for a boot program or jump to a hardcoded ROM
   address. Kickstart code performs later machine initialization.
4. Firmware can write CIA-A Port A bit 0 to remove the overlay. The machine's
   memory mapping then exposes Chip RAM at zero without copying or erasing it.
   Firmware or later software can populate RAM vectors and configure Paula's
   interrupt enables and devices. Valid vectors must exist in the visible
   memory before software relies on exceptions there. None of those actions
   occurs automatically as part of CPU reset.

The early mapping of ROM at zero and the CIA-controlled OVL line are described
in the [Amiga hardware manual](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md).
The startup path after the initial fetch depends on the supplied ROM image.
If no ROM image is supplied, the emulator installs a synthetic vector image
for headless startup. It contains no Kickstart firmware.

### Reset paths

| Event | CPU effect | Machine and memory effect |
| --- | --- | --- |
| Cold machine reset | Clear D0-D7, A0-A6 and USP; set SR=$2700; reload SSP/PC and prefetch. | Reset chips and devices, clear installed RAM, restore ROM overlay and reset master CCK. |
| Warm machine reset, including keyboard reset | Preserve D0-D7, A0-A6 and USP; set SR=$2700; reload SSP/PC and prefetch. | Reset chips and devices, preserve RAM, restore ROM overlay and reset master CCK. |
| Guest `RESET` instruction | Privileged instruction; preserve CPU registers, SR and current execution path while asserting the CPU reset output. | The machine observes that output and resets external devices and overlay; RAM and CPU execution state remain. |

The host can request an external reset through the emulator's cold and warm
reset APIs. Timed electrical reset waveforms are outside their scope. When
the guest `RESET` instruction asserts the reset output, the CPU keeps its
SSP and PC. The model assigns the instruction a 132-clock duration and a
124-clock output pulse. The supplied vectors constrain aggregate timing and
leave electrical pin edges unverified. Re-engaging the overlay can change
what a later fetch at a low address sees. Words already fetched follow the
queue's rules. The model handles the fatal halted state and STOP separately,
with no independent input for the physical HALT pin.

## Amiga-specific TAS bus behavior

TAS performs a read-modify-write transfer and sets flags from the original
byte. The CPU issues a distinct TAS write through AddressBus. On this Amiga
bus, [MemoryBus](MemoryBus.md) discards the TAS write phase for Chip and Slow
RAM, permits it for Fast RAM and uses ordinary byte routing for other banks.
The effect is a CPU and motherboard interaction specific to TAS. Ordinary
memory writes and the details of other instructions are outside this rule. The
[platform catalog](Platform%20Quirks%20and%20Invariants%20Catalog.md) indexes
other ordering and fault cases that need special attention.

## Evidence and scope

[SingleStepTests](Verification/CPU%20SingleStepTests.md) compare completed CPU
state, instruction clocks and timed bus callbacks against generated reference
vectors. The tests constrain observable instruction behavior on a synthetic
test bus. The production MemoryBus, CIA overlay, Agnus DMA arbitration and
electrical pin levels are outside the test scope. Machine and bus integration
have their own tests and specifications. This note describes the architectural
boundaries and current model, including its lack of external BERR and
level-seven interrupt support. Multiple internal CPU implementations can
satisfy the instruction vectors.

## Reference Documentation & Upstream Ground Truth

- Register widths, status bits and the addressing model are defined in [Motorola User's Manual Section 2](../Reference/68000%20User's%20Manual/02%20-%20Section%202%20-%20Introduction%20%26%20Programmer's%20Model.md).
- Physical read and write bus cycles and strobes are described in [Motorola User's Manual Section 5](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md).
- The converted [Motorola User's Manual Section 6](../Reference/68000%20User's%20Manual/06%20-%20Section%206%20-%20Exception%20Processing,%20Stack%20Frames%20%26%20Reset.md) describes vectors and stack frames. Consult the original PDF for the disputed trace wording.
- The ROM overlay, early startup and A500 interrupt levels are described in [Amiga Hardware Reference Manual Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md).
- Reset, stepping, interrupt input and modeled registers are covered by the [CPU implementation](../../../crates/cpu/src/cpu.rs) and [state](../../../crates/cpu/src/state.rs).
- Reset routing, CCK coordination, contention and the ROM overlay are implemented in the [machine loop](../../../crates/machine_loop/src/machine_loop.rs) and [physical memory](../../../crates/physical_memory/src/physical_memory.rs).
- The exact vector comparison and its observation limits are specified in the [SingleStep verification contract](Verification/CPU%20SingleStepTests.md).
