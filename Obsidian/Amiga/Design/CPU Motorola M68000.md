---
title: "Motorola 68000 CPU Design Specification"
aliases: ["M68000", "68000 CPU", "MC68000", "CPU Specification"]
tags: ["amiga", "design", "m68000", "cpu", "registers"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-10-11
related: ["[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/cpu"
---
# Motorola 68000 CPU Design Specification

The core implements the MC68000 programmer's model and executes observable bus
operations in Color Clock steps. One CCK advances two CPU clocks; an ordinary
byte or word bus cycle occupies two CCKs. Instruction state and transaction
ordering are constrained by silicon vectors; the internal descriptor table and
temporary fields are emulator mechanisms, not a reconstruction of silicon gates.

This document owns registers, addressing, instruction semantics, prefetch/PC,
exceptions and hardware requirements. The [micro-step specification](CPU%20Micro-Step%20State%20Machine.md)
owns dispatch, phase handlers, waits and execution classes. [MemoryBus](MemoryBus.md)
owns motherboard routing and [Machine Loop](Machine%20Loop.md) owns coordination.
The core takes an external AddressBus and performs no host filesystem, thread or
wall-clock operations. Native and WebAssembly execution share the same core.

## 1. CPU State & Register Architecture

### 1.1 Complete M68000 Programmer's Model & State Fields

The programmer-visible fields in [CpuState](../../../crates/cpu/src/state.rs)
follow Motorola User's Manual Section 2. Runtime latches are stored alongside
them so snapshots can describe stopped, halted and pending boundary processing.

| Field | Width | Contract |
| --- | --- | --- |
| D0-D7 | 32 | Byte/word writes preserve the other bits; long writes replace all bits. |
| A0-A6 | 32 | Address operations use word sign extension or long values; no byte operand form. |
| A7, USP, SSP | 32 | A7 is the active stack. Changing SR.S saves the old bank and selects the other bank. |
| pc | 32 | Emulator instruction-stream cursor; bus addresses are masked to 24 bits. |
| instruction_pc | 32 | Formal opcode address at execution entry; updated at retirement or STOP completion. |
| ir, prefetch | 16 each | Executing opcode and staged next word; extensions can occupy the staged word. |
| sr | 16 | T, S, I2-I0 and X/N/Z/V/C; reserved bits are masked by SR_MASK. |
| ipl | 8 | Encoded A500 input level 0-6, independent of the accepted interrupt level. External level 7 is ignored. |
| trace_pending | bool | T sampled at instruction entry; ordinary status writes cannot cancel that trace. |
| stopped, halted | bool | STOP wait and fatal double-fault state; neither performs instruction bus activity while idle. |
| reset_line_asserted | bool | CPU-owned RESET output, observed by the machine coordinator. |
| cycle_counter | 64 | Wrapping count of CPU clocks consumed by active CCKs, including waits. |
| micro | CpuMicroState | Execution temporaries; described by the owning micro-step document. |

#### Status Register (SR) Bit Allocation

```text
Bit:   15   14   13   12   11   10    9    8    7    6    5    4    3    2    1    0
Field:  T    0    S    0    0   I2   I1   I0    0    0    0    X    N    Z    V    C
       └──────── System Byte (Privileged) ────────┘   └── Unused ──┘   └───── CCR (User Byte) ────┘
```

- **System Byte (Bits 8–15, Privileged):**
  - **Bit 15 (`T`)**: Trace Mode enable.
  - **Bit 13 (`S`)**: Supervisor state (`1` = Supervisor, `0` = User). Swaps active $A_7$ between $SSP$ and $USP$.
  - **Bits 10–8 (`I2-I0`)**: Interrupt Priority Mask (levels 0 through 7).
- **User Byte / Condition Code Register (CCR, Bits 0–4):**
  - **Bit 4 (`X`)**: Extend flag (used for multi-precision arithmetic).
  - **Bit 3 (`N`)**: Negative flag (set if result MSB is 1).
  - **Bit 2 (`Z`)**: Zero flag (set if result is zero).
  - **Bit 1 (`V`)**: Overflow flag (set on 2's complement arithmetic overflow).
  - **Bit 0 (`C`)**: Carry flag (set on borrow or carry out).

### 1.2 Complete M68000 Addressing Modes Specification

The M68000 supports 12 fundamental addressing modes encoded via the effective address fields `mode` (bits 5–3) and `reg` (bits 2–0):

| Mode Name | Syntax | `mode` | `reg` | Extension Words | Clocks (Read / Write) |
| :--- | :--- | :---: | :---: | :---: | :---: |
| **Data Register Direct** | `Dn` | `000` | `000-111` | 0 | 0 / 0 |
| **Address Register Direct** | `An` | `001` | `000-111` | 0 | 0 / 0 |
| **Address Register Indirect** | `(An)` | `010` | `000-111` | 0 | 4 / 4 |
| **Address Reg Indirect with Postincrement** | `(An)+` | `011` | `000-111` | 0 | 4 / 4 |
| **Address Reg Indirect with Predecrement** | `-(An)` | `100` | `000-111` | 0 | 6 / 4 |
| **Address Reg Indirect with Displacement** | `(d16, An)` | `101` | `000-111` | 1 (`d16`) | 8 / 8 |
| **Address Reg Indirect with Index** | `(d8, An, Xn)` | `110` | `000-111` | 1 (Brief) | 10 / 10 |
| **Absolute Short** | `(xxx).W` | `111` | `000` | 1 | 8 / 8 |
| **Absolute Long** | `(xxx).L` | `111` | `001` | 2 | 12 / 12 |
| **Program Counter with Displacement** | `(d16, PC)` | `111` | `010` | 1 (`d16`) | 8 / N/A |
| **Program Counter with Index** | `(d8, PC, Xn)` | `111` | `011` | 1 (Brief) | 10 / N/A |
| **Immediate Data** | `#<data>` | `111` | `100` | 1 or 2 | 4 / N/A |

#### Brief Extension Word Format (`(d8, An, Xn)` and `(d8, PC, Xn)`)
The MC68000 exclusively supports the 16-bit **Brief Extension Word**:
```text
15   14        12 11   10         8 7                               0
+---+------------+----+------------+--------------------------------+
|D/A|  Register  |W/L | Scale (000)|   Signed 8-bit Displacement    |
+---+------------+----+------------+--------------------------------+
```
- **Bit 15 (D/A):** Index register type (`0` = Data Register $D_n$, `1` = Address Register $A_n$).
- **Bits 14–12 (Register):** Index register index (0–7).
- **Bit 11 (W/L):** Index size (`0` = sign-extended 16-bit word, `1` = 32-bit long).
- **Bits 10–8 (Scale):** Scale factor. On the MC68000, scale is always $1\times$ (`000`).
- **Bits 7–0 (Displacement):** Signed 8-bit integer ($d_8$), sign-extended to 32 bits.
- **Effective Address Formula:**
  - For `(d8, An, Xn)`: $EA = (A_n + X_n + \text{sign\_extend}(d_8)) \bmod 2^{32}$.
  - For `(d8, PC, Xn)`: $EA = (PC_{extension} + X_n + \text{sign\_extend}(d_8)) \bmod 2^{32}$.
  - The [effective-address helpers](../../../crates/cpu/src/micro/ea.rs) retain this 32-bit result, including when `LEA` writes it to an address register. A memory transfer separately uses $bus\_address = EA \ \& \ \text{\$00FFFFFF}$; the 24-bit bus mask does not truncate the internal EA.

#### Hardware Quirks & Rules
1. **Stack Pointer `A7` Byte Alignment Quirk:**
   - On byte-sized operations (`.b`), postincrement `(An)+` and predecrement `-(An)` normally adjust the address register by 1.
   - **Exception:** If the register is `A7` (either User Stack Pointer `USP` or Supervisor Stack Pointer `SSP`), the address is adjusted by **2** to keep the stack word-aligned.
2. **Sign Extension Rules:**
   - 16-bit displacement `d16` and 8-bit displacement `d8` must be sign-extended to 32 bits before addition.
   - Absolute short addresses `(xxx).W` are sign-extended from 16 to 32 bits (addressing `$00000000-$00007FFF` and `$FFFF8000-$FFFFFFFF`).
3. **24-bit Physical Address Truncation:**
   - Internal address registers and arithmetic are full 32-bit. However, the physical MC68000 address bus only routes 24 bits ($A_1-A_{23}$ plus $\overline{UDS}/\overline{LDS}$).
   - All physical bus transactions must mask addresses to 24 bits (`addr & 0x00FF_FFFF`).

### 1.3 Two-Word Prefetch Pipeline Architecture & Program Counter Dynamics

The emulator uses three word fields: `ir` for the executing opcode, `prefetch`
for the following staged word, and `micro.irc` for an incoming refill. These are
implementation roles; their names do not prove corresponding hidden silicon
registers. The prefetch observations are supported by the SingleStep initial and
final queues and the transaction records.

At a primed instruction boundary at address A, `instruction_pc = A`, `ir`
contains the word at A, `prefetch` contains A+2, and `pc = A+4`. For example,
NOP at $1000 starts with cursor $1004, reads $1004, then retires with opcode
address $1002 and cursor $1006. Prefetch preserves already fetched instruction
words when guest code changes memory; no host decoded-code cache is involved.

The four-byte relation is a boundary contract, not an invariant throughout an
instruction. Extension completion advances the cursor by two while leaving
`instruction_pc` at the executing opcode. A wait does not advance that cursor.
RMW or predecrement MOVE can advance or retire a refill before the final write;
the [execution classes](CPU%20Micro-Step%20State%20Machine.md#7-concrete-micro-step-traces-by-architectural-class)
specify those distinctions. Target refill commits cursor `target + 4` without
truncating its high byte; only external bus addresses are masked.

| Phase/path | Cursor and saved-PC rule |
| --- | --- |
| Initial priming | Formal opcode A, cursor A+4. Debugger and SingleStep adapters establish this explicitly. |
| Consumed extension | Successful finish advances cursor by two; a refused read preserves PC and prefetch. |
| Ordinary retirement | Advance an pending sequential refill, then set instruction_pc to cursor minus four. |
| Completed target refill | Set cursor to target+4 and instruction_pc to target. |
| STOP | No refill; following opcode address equals the current cursor after the immediate. |
| Illegal/Line A/Line F/privilege | Save the rejected instruction_pc. |
| TRAP | Save the following opcode address, cursor minus two for its one-word encoding. |
| CHK | Save the following opcode address after its operand extensions. |
| Divide-by-zero | Save the instruction_pc selected by the supplied fault vectors. |
| Trace/interrupt | Save the next not yet executed opcode address, including the STOP continuation. |
| Operand Address Error | Capture cursor minus four at the failed access phase; extension progress can make it differ from the original opcode. |
| Odd control target | Capture target minus four; this is an observed stack value, not proof of a physical subtraction circuit. |
| Handler/reset failure | Follow the separate fault/halting rules; there is no universal saved-PC subtraction. |

For `(d16,PC)` and `(d8,PC,Xn)`, the base is the displacement extension address,
normally opcode+2. JSR/BSR save the first opcode after their complete encoding;
the shared stack implementation must not substitute the prefetched cursor.
ADD.w vector 22 saves $0C02 for opcode $0C00; JMP vector 1 saves $CCE90755 for
odd target $CCE90759. Raw stack-write callbacks, rather than masked final RAM,
establish these two counterexamples to a universal opcode-based frame PC.

## 2. Dynamic Opcode Cycle Calculation

Motorola Section 8 specifies aggregate clocks and read/write counts. The
instruction implementation and silicon transactions determine the placement of
each internal interval, operand transfer and prefetch. A correct total does not
compensate for reordered callbacks. Multiply, divide and register shifts have
data-dependent internal counts; MOVEM count depends on its register mask.
The [class specification](CPU%20Micro-Step%20State%20Machine.md#7-concrete-micro-step-traces-by-architectural-class)
owns these sequence and timing parameters.

### 2.1 Opcode Dispatch

The opcode indexes a static descriptor table; the active slice is cached during
CCK stepping. This avoids repeated decoding but includes indirect calls and
dynamic condition/fault decisions. Host throughput or branch-prediction claims
require measurements from [CPU Instruction Benchmarking](Performance/CPU%20Instruction%20Benchmarking.md).

### 2.2 Endianness and Raw Data

Guest memory is big-endian. AddressBus returns decoded byte/word values and the
CPU assembles longs explicitly from high and low words. Bitwise operations and
raw copying commute with byte reversal, and zero is invariant under reversal.
That algebra alone does not authorize bypassing bus phases, faults, CCR.N sign
selection or memory routing. Arithmetic, shifting and address calculation use
decoded values; no host pointer cast or transmute is permitted.

### 2.3 M68000 Opcode Bitfield Decoding & Instruction Topology

> [!NOTE]
> For emulator codebase function naming conventions, consult the operational rule in [opcode-naming.md](../../../.agents/rules/opcode-naming.md).

Many M68000 opcode families use the following register, operation and EA fields.
Field boundaries depend on the family: `Bcc`, `DBcc` and `Scc` use bits 11–8
for their condition code, overlapping the register and operation fields below.

```text
 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
|  Major Group  |  Register |    Opmode     |   Mode    |  Register |
+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
 \_____________/ \_________/ \_____________/ \_______/   \_________/
    Bits 15-12    Bits 11-9     Bits 8-6      Bits 5-3    Bits 2-0
   Instruction    Destination   Direction /    EA Mode     EA Reg
      Family        Register    Size / Sub-op
```

#### 1. Bits 15–12: Major Opcode Class ($0..F$)

Every instruction's primary category is indexed by its highest nibble:

| Nibble (Bits 15–12) | Major Instruction Family | Key Instructions |
| :---: | :--- | :--- |
| **`$0`** (`0000`) | Bit Manipulation / Immediate / `MOVEP` | `BTST`, `BCHG`, `BCLR`, `BSET`, `MOVEP`, `ORI`, `ANDI`, `SUBI`, `ADDI`, `EORI`, `CMPI` |
| **`$1`** (`0001`) | Move Byte | `MOVE.B` |
| **`$2`** (`0010`) | Move Long | `MOVE.L`, `MOVEA.L` |
| **`$3`** (`0011`) | Move Word | `MOVE.W`, `MOVEA.W` |
| **`$4`** (`0100`) | Miscellaneous / System / Control | `LEA`, `PEA`, `CLR`, `NEG`, `NOT`, `TST`, `EXT`, `SWAP`, `TRAP`, `LINK`, `UNLK`, `JMP`, `JSR`, `RTS`, `RTE`, `MOVEM`, `CHK` |
| **`$5`** (`0101`) | Quick Math & Conditional Tests | `ADDQ`, `SUBQ`, `Scc`, `DBcc` |
| **`$6`** (`0110`) | PC-Relative Branches | `BRA`, `BSR`, `Bcc` (16 conditions: `BEQ`, `BNE`, `BGT`, etc.) |
| **`$7`** (`0111`) | Move Quick | `MOVEQ` (8-bit immediate sign-extended into $D_n$) |
| **`$8`** (`1000`) | Logical OR & Division | `OR`, `DIVU`, `DIVS`, `SBCD` |
| **`$9`** (`1001`) | Subtraction | `SUB`, `SUBA`, `SUBX` |
| **`$A`** (`1010`) | Line-A Emulator Trap | Reserved for host OS emulation traps (Vector 10) |
| **`$B`** (`1011`) | Compare & Exclusive OR | `CMP`, `CMPA`, `EOR`, `CMPM` |
| **`$C`** (`1100`) | Logical AND & Multiplication | `AND`, `MULU`, `MULS`, `ABCD`, `EXG` |
| **`$D`** (`1101`) | Addition | `ADD`, `ADDA`, `ADDX` |
| **`$E`** (`1110`) | Bit Shifts & Rotates | `ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR` |
| **`$F`** (`1111`) | Line-F Coprocessor Trap | Reserved for FPU/coprocessor traps (Vector 11) |

#### 2. Bits 11–9: Register or Quick Data; Bits 11–8: Condition

- **Standard ALU (`ADD`, `SUB`, `AND`, `OR`, `CMP`):** Primary Data Register $D_n$ index (`0`–`7`).
- **`MOVE` instructions:** Destination Data/Address register index.
- **Branches (`Bcc`, `DBcc`, `Scc`):** Bits 11–8 hold the 4-bit condition code test (`0000`–`1111`).
- **`ADDQ` / `SUBQ`:** 3-bit quick immediate data: values `1`–`7`; `000` encodes value `8`.
- **`MOVEQ`:** Destination Data Register $D_n$ index (bits 7–0 hold the signed 8-bit literal).

#### 3. Bits 8–6: Direction & Operand Size Field

- **ALU Instructions (`ADD`, `SUB`, `AND`, `OR`):**
  - **Bit 8 (Direction):**
    - `0`: $\langle\text{ea}\rangle + D_n \rightarrow D_n$ (Source is $\langle\text{ea}\rangle$, Destination is $D_n$).
    - `1`: $D_n + \langle\text{ea}\rangle \rightarrow \langle\text{ea}\rangle$ (Source is $D_n$, Destination is $\langle\text{ea}\rangle$).
  - **Bits 7–6 (Size):**
    - `00`: Byte (`.B`, 8-bit)
    - `01`: Word (`.W`, 16-bit)
    - `10`: Long (`.L`, 32-bit)
  - **Special Address Register Mode (`ADDA`, `SUBA`, `CMPA`):**
    - `011`: Word (`.W`, 16-bit sign-extended to 32-bit)
    - `111`: Long (`.L`, 32-bit)
- **`MOVE` Instructions (Bits 13–12 & 8–6):**
  - **Bits 13–12:** Size (`01` = Byte, `11` = Word, `10` = Long).
  - **Bits 8–6:** Destination Effective Address Mode (`000` = $D_n$, `001` = $A_n$, `010` = $(A_n)$, etc.).
- **Shifts & Rotates (`ASL`, `LSR`, etc.):**
  - **Bit 8 (Direction):** `0` = Right shift, `1` = Left shift.
  - **Bits 7–6 (Size):** `00` = Byte, `01` = Word, `10` = Long.

#### 4. Bits 5–0: Effective Address (<ea>) Field

- **Bits 5–3 (`mode`):** Addressing mode selector (`000` through `111`).
- **Bits 2–0 (`reg`):** Register number (`0` through `7`).
- **Mode 7 (`111`) Special Extensions:**
  - `000`: Absolute Short (`(xxx).W`)
  - `001`: Absolute Long (`(xxx).L`)
  - `010`: Program Counter with 16-bit Displacement (`(d16, PC)`)
  - `011`: Program Counter with 8-bit Index (`(d8, PC, Xn)`)
  - `100`: Immediate Data (`#<data>`)

---

## 3. Bus Stalling & CCK Phase Model

The CPU observes ready/refused transfers through AddressBus. Reads are modeled
in CCK1 and writes in CCK2; a refusal consumes two clocks while retaining the
active phase. Finish/setup phases do not perform an extra memory access.
The [micro-step bus contract](CPU%20Micro-Step%20State%20Machine.md#4-memory-bus-operations--strobe-semantics)
defines when state is latched and which side effects wait for success.

These callbacks are not electrical S0-S7 pin edges. Motorola Section 5 defines
physical read sampling and strobes; the emulator uses a CCK abstraction tested
against silicon interval records. An idle CPU callback does not alone prove that
Agnus has granted the physical bus to another master. Production arbitration
belongs to MemoryBus/Agnus integration tests.

## 4. Two-Word Instruction Prefetch Pipeline

The phase-dependent PC and queue contract is defined once in
[Section 1.3](#13-two-word-prefetch-pipeline-architecture--program-counter-dynamics).
Sequential, early RMW and control-target retirement are distinct mechanisms in
the micro-step specification. The active descriptor remains stable when IR is
advanced early; the next descriptor is selected only at a completion boundary.

## 5. Exception Processing & Vectors

Motorola Section 6 defines vector selection, supervisor transition and frame
contents. The vector table occupies $000000-$0003FF: each entry is a big-endian
32-bit address at four times the vector number. MC68000 has no VBR, format word,
later-CPU restart frame or coprocessor execution.

| Vector | Cause | Saved PC / supported evidence |
| --- | --- | --- |
| 0/1 | External reset SSP/PC | No frame; host setup API reads vectors without timing. |
| 2 | Bus Error | AddressBus has no BERR result; not implemented or claimed verified. |
| 3 | Address Error | Seven-word diagnostic frame; supplied operand/target vectors. |
| 4 | Illegal encoding | Rejected opcode PC; synthetic decode/frame controls. |
| 5/6/7 | Divide-by-zero, CHK, TRAPV | Supplied instruction-generated exception vectors. |
| 8 | Privilege violation | Rejected opcode PC before prohibited operand access; synthetic user-mode controls. |
| 9 | Trace | Next opcode PC; synthetic boundary, STOP and priority controls. |
| 10/11 | Line A / Line F | Rejected opcode PC; no later-CPU instruction execution. |
| 25-30 | Autovector levels 1-6 | Next opcode PC; fixed four-clock IACK abstraction. |
| 31 | Level 7 autovector | Defined by the MC68000; level 7 acceptance is outside the implemented Amiga 500 scope. |
| 32-47 | TRAP #0-15 | Following instruction PC; supplied vectors. |

A short frame occupies six bytes: saved SR at final SSP, PC high at SSP+2 and
PC low at SSP+4. The observed stack-write order is PC low, SR, PC high; final
SSP is committed after the last successful write. Vector and handler reads
follow. Uncontended Illegal/privilege/trace/TRAP entry takes 34 clocks; interrupt
entry takes 44 with the fixed IACK interval. Prefix work and waits are separate.

### 5.1 Address Error (Vector 3) 7-Word Stack Frame

| Offset from final SSP | Word |
| --- | --- |
| +0 | Information word: upper IR bits, R/W bit 4, I/N bit 3, FC bits 2-0 |
| +2, +4 | Fault address high, low |
| +6 | IR retained at fault |
| +8 | SR captured for entry |
| +10, +12 | Phase-selected PC high, low |

From the original SSP B, writes occur at B-2 (PC low), B-6 (SR), B-4 (PC
high), B-8 (IR), B-10 (fault low), B-14 (information), B-12 (fault high).
The final successful write commits SSP=B-14. The common entry contributes
50 clocks plus preceding completed instruction work and waits. No odd word
callback is issued for the rejected access.

Data-space FC is 1/5 for user/supervisor; program-space FC is 2/6. A PC-relative
arithmetic operand remains a data transfer. The current information-word
construction retains IR bits above bit 4, sets read/write and FC, and adds bit 3
on demonstrated odd control-target paths. It does not expose independent FC or
instruction-not pins. Existing comparator masks remain documented in
[CPU SingleStepTests](Verification/CPU%20SingleStepTests.md#12-input-scope-and-complete-verification).

### 5.2 Privilege Violation and Decode Exceptions

RESET, STOP, RTE, MOVE to SR, SR immediate logic and MOVE USP reject user mode
before prohibited operand access. MC68000 MOVE from SR and CCR operations are
unprivileged. Unassigned opcodes enter Illegal; high nibbles A/F select their
emulator vectors. Trace sampled for a rejected instruction is canceled.

### 5.3 Instruction Traps and Trace Priority

TRAP, taken TRAPV, CHK and zero divide finish their own exception processing
before a pending trace. Trace completes before pending interrupt entry. Each
entry saves its own SR and PC and clears live T, so handler execution can return
through the nested frames. Zero divide preserves X, clears N/Z/V/C and uses a
38-clock common path plus operand acquisition.

Trace samples T at instruction entry. Clearing T while an instruction executes
does not cancel that trace; setting T takes effect at the following instruction.
Trace saves the current status at entry, not a forced reconstruction of the
earlier T value. Reset, Address Error, Illegal and privilege rejection cancel it.
These rules follow [the original Motorola PDF, Section 6.3.8](https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf#page=100).
The local converted Section 6.3.8 has contrary wording about Illegal/privilege
and saved T; that conversion is not authoritative for these claims.

### 5.4 Double Fault and Unsupported Error Inputs

Odd SSP while constructing Address Error, or an odd vector-3 handler, halts
the CPU. Subsequent stepping produces no bus activity until external reset.
An odd handler for an ordinary short-frame exception starts Address Error;
vector-3 failure then halts. Vector table addresses themselves are even, so
address-alignment failure on a vector read is unreachable with internal vector
selection. External BERR at stack, vector, operand or handler accesses remains
unavailable through AddressBus and is not simulated by an odd-address fixture.

## 6. Reset Procedure & Hardware Pin Signaling

### 6.1 Cold and Warm CPU Reset Sequences

`reset()` and `reset_warm()` are host setup APIs using debug reads; they do not
execute the timed hardware reset sequence from Motorola Section 8.12. Both set
SR=$2700, clear pending boundary processing and stop/halt/output state, load SSP
and PC from vectors 0/1 and prime two words. They do not sanitize vector values.
An odd initial PC halts immediately; an odd SSP is retained until a later stack
access requires alignment. Cold reset clears D0-D7/A0-A6/USP as a project
initialization policy; warm reset preserves them. CPU reset itself does not
clear RAM or reset peer chips; the machine owns those operations.

### 6.2 The RESET Instruction & External Pin Signaling

Privileged RESET preserves CPU registers and SR, takes 132 clocks and asserts
`reset_line_asserted` for 124. Four released internal clocks precede the pulse;
four-clock sequential prefetch follows. Prefetch waits cannot extend the pulse.
The machine observes the assertion edge and resets external devices/overlay
without consuming the CPU output or clearing RAM. This partition is an emulator
mechanism; supplied RESET JSON constrains the aggregate idle interval and fetch.

### 6.3 System Reset Comparison Matrix

| State | Cold host reset | Warm host reset | Guest RESET |
| --- | --- | --- | --- |
| D0-D7/A0-A6/USP | Cleared | Preserved | Preserved |
| SSP/PC | Reload vectors | Reload vectors | Preserved / sequential PC |
| SR | $2700 | $2700 | Preserved |
| Pending trace/micro-state | Reset | Reset | Ordinary instruction completion |
| RAM and peer chips | Controlled by machine | Controlled by machine | External devices reset at output edge; RAM preserved |
| Timed bus sequence | Not implemented by host API | Not implemented by host API | 132 clocks plus prefetch waits |

## 7. M68000 Instruction & Hardware Silicon Quirks

The [platform catalog](Platform%20Quirks%20and%20Invariants%20Catalog.md) indexes
CPU and motherboard quirks. The rules below state semantics; the class document
owns execution ordering and timing parameters.

### 7.1 TAS (Test And Set) Read-Modify-Write Hardware Bug

TAS tests the original byte, sets N/Z, clears V/C, preserves X, then sets bit 7.
Register TAS updates Dn internally. Memory TAS uses the special read/modify/write
interval described by the micro-step class. On the A500, Chip/Slow RAM discard
its write and Fast RAM permits it. The CPU identifies its write through AddressBus.write_tas_byte. The production
MemoryBus uses the existing bank classification to suppress Chip/Slow writes
while preserving their wait behavior; other regions use ordinary byte routing.
The generic backplane default writes normally, preserving silicon-fixture
behavior. Production machine regressions cover both byte lanes, flags and
neighbors in Chip/Slow/Fast RAM; this is not an electrical bus-lock capture.

### 7.2 ADDX, SUBX & NEGX Multi-Precision Zero Flag ($Z$) Quirk

In standard arithmetic instructions (`ADD`, `SUB`, `NEG`), the Zero flag is set if the result equals zero and cleared otherwise ($Z = \text{result} == 0$).

- **Chained Multi-Precision Behavior (`ADDX`, `SUBX`, `NEGX`):**
  - The Zero flag ($Z$) is **only cleared if the result is non-zero** (`if res != 0 { Z = false; }`).
  - If the operation result is zero, **$Z$ retains its prior state without change**.
  - **Rationale:** This allows multi-precision arithmetic chains (e.g. 64-bit addition: `ADD.L D0, D1` followed by `ADDX.L D2, D3`) where the final $Z$ flag indicates whether the *entire* multi-word number is zero. If any intermediate word was non-zero, $Z$ remains `0` to the end.

### 7.3 Address Register Direct Operations Quirks (`MOVEA`, `ADDA`, `SUBA`, `ADDQ`, `SUBQ`)

The M68000 treats Address Registers ($A_0-A_7$) as dedicated pointer resources, applying unique rules compared to Data Registers:

- **`MOVEA` (Move Address):**
  - Word operations (`MOVEA.W`) **sign-extend the 16-bit word to full 32-bit** before writing into $A_n$.
  - **Condition Codes:** `MOVEA` **never alters any condition codes** ($X, N, Z, V, C$ are untouched), unlike standard `MOVE` which updates $N, Z$ and clears $V, C$.
- **`ADDA` and `SUBA` (Add/Subtract Address):**
  - Word operations sign-extend the 16-bit source to 32 bits.
  - Calculations operate across the entire 32-bit address register.
  - **Condition Codes:** **All condition codes ($X, N, Z, V, C$) remain completely unchanged**.
- **`ADDQ` and `SUBQ` (Add/Subtract Quick):**
  - Immediate value is $1..8$ (the 3-bit opcode field `000` encodes `8`).
  - **Destination $A_n$:**
    - Byte size (`.B`) is invalid/unsupported.
    - Word operations perform full 32-bit addition/subtraction without truncation.
    - **Condition Codes:** **No condition codes are modified** when destination is an address register.
  - **Destination $D_n$ or Memory:** Standard size masking and condition codes ($X, N, Z, V, C$) update normally.

### 7.4 Bit Manipulation Instructions Quirks (`BTST`, `BSET`, `BCLR`, `BCHG`)

Bit manipulation instructions evaluate individual bit positions:

- **Bit Number Modulo Addressing:**
  - **Data Register Destination ($D_n$):** Bit index is evaluated modulo 32 (`bit_num % 32`).
  - **Memory Destination (`<ea>`):** Bit index is evaluated modulo 8 (`bit_num % 8`).
- **Condition Codes:**
  - Only the **Zero flag ($Z$)** is updated: $Z = 1$ if the tested bit was zero; $Z = 0$ if the tested bit was one.
  - Flags **$X, N, V, C$ are completely unaffected** (preserving existing carry/extend states across bit tests).

`BTST` leaves the operand unchanged; `BCHG`, `BCLR` and `BSET` toggle, clear
or set the tested bit. Execution phases are described in the
[micro-step classes](CPU%20Micro-Step%20State%20Machine.md#7-concrete-micro-step-traces-by-architectural-class).

### 7.5 Shifts & Rotates

Register counts use the low six bits; immediate count zero encodes eight.
Byte/word timing is 6+2n, long timing 8+2n, with prefetch before the internal
tail. Memory variants operate on a word with count one and use ordinary RMW.
For zero register count, N/Z reflect the unchanged value, V clears, X stays;
C clears for ordinary shifts/rotates and equals X for rotate-through-extend.
ROL/ROR preserve X; ROXL/ROXR rotate through X and update C/X together.
ASL accumulates V if any intermediate sign transition occurs. ASR saturation
and the two reviewed ASL.b reference corrections remain scoped observations in
the [override catalog](../../../crates/test_runner/tests/overrides/singlestep/README.md).

### 7.6 Address Error (Vector 3) Operand and Instruction Space

- **Function Code Selection on Address Error:**
  - Operand reads use **Data Space** ($FC = 1$ in User mode, $FC = 5$ in Supervisor mode), including operands whose effective address is calculated relative to PC. PC-relative address calculation does not turn an arithmetic operand into an instruction fetch.
  - Instruction-stream fetches use **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode). Operand and instruction-fetch faults remain distinct in the common exception entry API.
- **AGU Register Commitment & Predecrement Bus Inversion:**
  - Detailed physical silicon rules governing `(An)+` read/write commitment asymmetry and `MOVE ..., -(An)` prefetch-before-write sequencing reside in [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md#2-motorola-68000-silicon-quirks--cpu-pipeline-traps).

### 7.7 MOVE and MOVEP Data Transfers

- In 32-bit `MOVE.l <ea>, (An)` transfers:
  - **Real MC68000 Silicon:** Condition codes reflect the full 32-bit transfer ($N = \text{bit } 31$, $Z = \text{value } == 0$).
  - *Emulator Resolution:* The core strictly implements the full 32-bit condition code evaluation matching real silicon.

[`movep.rs`](../../../crates/cpu/src/instructions/movep.rs) transfers a word or
long between Dn and alternate bytes beginning at `An + sign_extend(d16)`.
Bytes move from most to least significant at offsets 0, 2, 4 and 6 as needed;
An and all condition codes remain unchanged. Word loads preserve the upper
half of Dn. Each memory access is a byte transfer, so an odd effective address
is valid. The [micro-step specification](CPU%20Micro-Step%20State%20Machine.md#spaced-byte-transfer-variant-movep)
defines extension, transfer and prefetch ordering.

### 7.8 Branch & Control Flow Odd Target Address Error (FC 2 / 6)

- When `BRA`, taken `Bcc`/`DBcc`, `BSR`, `JMP`, `JSR`, `RTS`, `RTR`, or `RTE` evaluates a target address with an odd destination (`target & 1 != 0`):
  - The MC68000 halts instruction execution and immediately triggers an **Address Error exception (Vector 3)**.
  - Because the instruction fetch pipeline caused the fault, the CPU asserts **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode) in the exception status word.
  - The frame saves `target - 4` as PC and sets the instruction-not bit (bit 3). `BSR` writes its return address before the target check; `JSR` checks the target first. See the [shared refill model](CPU%20Micro-Step%20State%20Machine.md#51-the-two-word-pipeline-refill).

### 7.9 Read-Modify-Write and Bit Timing

Ordinary RMW reads the operand, computes flags/results with prefetch, then
writes. Long arithmetic/logic/unary RMW writes low word before high word.
Bit-register internal delays vary with operation and tested bit position;
immediate forms add extension acquisition. The
[class tables](CPU%20Micro-Step%20State%20Machine.md#class-5-memory-read-modify-write-operand-read-prefetch-writeback)
preserve the full ordering and addressing parameters.

### 7.10 Instruction Module Ownership

Mnemonic modules and documented family exceptions are flat under
[`instructions/`](../../../crates/cpu/src/instructions). The compile-time decoder
selects size/direction/addressing slices; runtime values remain parametric.
Operational naming, inlining and file-cohesion rules live in
[opcode-naming](../../../.agents/rules/opcode-naming.md) and the instruction skill,
rather than duplicated implementation history here.

### 7.11 Extended Arithmetic, Decimal and Control Semantics

ADD/SUB/NEG update X/N/Z/V/C with explicit wrapping at the operand width;
CMP/CMPI/CMPA update N/Z/V/C without writing the operand or X. Logic and MOVE
set N/Z, clear V/C and preserve X; MOVEA instead preserves CCR and sign-extends
a word. CLR still reads a memory destination before storing zero.

ABCD/SBCD/NBCD operate on packed decimal bytes with X input, decimal carry or
borrow copied to C/X, and cumulative Z. Motorola leaves some decimal N/V
results undefined; the implementation follows the supplied observable inputs
without promising a rule for every undocumented decimal pattern.

MULU/MULS multiply 16-bit operands to a 32-bit result, set N/Z, clear V/C and
preserve X. DIVU/DIVS divide a 32-bit dividend by a word, packing remainder in
the high word and quotient in the low word. Signed division truncates toward
zero and the remainder follows the dividend sign. Overflow retains Dn, sets V
and clears C; N/Z behavior remains constrained by the supplied cases. Zero
divisor selects the instruction-generated exception path. Data-dependent clocks
and prefetch placement belong to the variable-cycle class.

Branches calculate signed displacement from opcode+2; an untaken branch does
not validate an unused target. DBcc tests the condition before decrementing the
low counter word; expiry is $FFFF. Supplied DBcc vectors do not cover expiry.
Scc writes $FF or zero and preserves CCR. LINK saves the frame register and
applies a signed stack displacement; UNLK restores that register and stack.
JSR/BSR stack the following opcode PC. RTR restores CCR only; RTE restores the
complete masked SR and stack bank. Their bus ordering is owned by the control
classes, not duplicated here.

#### Status Transfers and Bounds Exceptions

MC68000 `MOVE SR,<ea>` is unprivileged; `MOVE <ea>,SR` and both USP transfer
directions require initial supervisor state. `MOVE <ea>,CCR` and immediate
CCR logic preserve the system byte. SR writes mask reserved bits and select
USP/SSP through banked A7 accessors. Operand faults precede status replacement;
rejected privilege checks leave operand registers and USP unchanged.

CHK compares signed words, preserves X, clears V/C, and selects Vector 6 on
an out-of-range value. Its supplied normal inputs retain N and clear Z. Prefetch
precedes condition evaluation and stacking; above-bound timing is tested before
the negative-only path. Untaken TRAPV preserves flags and completes normal
prefetch; taken TRAPV also performs that read before Vector 7 entry. TRAP selects
Vector `32 + n`. TRAP/TRAPV and CHK save the following opcode PC. The supplied
[CHK SingleStep vectors](../../../ref_src/SingleStepTests-680x0/68000/v1/CHK.json)
start register CHK at $0C00 and write $0C02 into the saved PC; RTE therefore
continues after CHK without retrying it. Divide-by-zero retains its separate
opcode-PC return rule. Each entry saves the selected status.
Their prefixes and shared frame order belong to the
[three-word exception class](CPU%20Micro-Step%20State%20Machine.md#class-13-three-word-software-exception-entry).

### 7.12 Autovector Interrupt Processing & STOP Instruction Awakening

The supported A500 interrupt inputs are levels 1-6, qualifying when greater
than SR.I. The accepted level is captured separately from the live input, so
a subsequent pin change cannot redirect the queued exception. The electrical
qualification and propagation delay of IPL pins are not modeled at sub-CCK
resolution.

The physical MC68000 supports non-maskable level 7 through its three encoded
IPL inputs; it has no separate NMI pin. The standard A500 motherboard does not
generate level 7. [HRM Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md)
describes its use by external debugging hardware through the expansion connector.
External NMI is outside this emulator's A500 scope and is not required of its
CPU implementation. IPL=7 is ignored: there is no edge history or pending NMI
latch. Supporting such expansion hardware would require extending that scope.
SR.I remains three bits wide; mask 7 blocks all supported interrupt levels.

Interrupt entry saves the next opcode and old SR, selects SSP, clears T, raises
SR.I to the accepted level and uses vector 24+level. The current four-clock
acknowledge interval is internal: AddressBus exposes no device vector, VPA,
spurious-IACK or CPU-space callback. 44 clocks is the uncontended autovector
contract, not a captured acknowledge-pin trace.

STOP loads its immediate SR and completes in four clocks with no bus transfer.
Its continuation address is retained without shifting an unfilled queue.
Pending trace wakes it before interrupt arbitration; an unmasked IRQ wakes it
and refills the handler. Idle stepping while stopped consumes no CPU clocks;
the machine's master CCK still progresses. RTE reads PC high, SR, PC low,
restores stack banking, then refills the return target. Trace enabled by RTE
applies to the following instruction, not RTE itself.

## 8. Reference Documentation & Upstream Ground Truth

- [Motorola User's Manual Section 2](../Reference/68000%20User's%20Manual/02%20-%20Section%202%20-%20Introduction%20%26%20Programmer's%20Model.md): register widths, SR and addressing.
- [Motorola User's Manual Section 5](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md): physical bus cycles and strobes, distinct from CCK callbacks.
- [Original Motorola User's Manual](https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf): Section 6.3.8 resolves the local trace-conversion error; Section 8 supplies aggregate timings.
- [Converted Section 6](../Reference/68000%20User's%20Manual/06%20-%20Section%206%20-%20Exception%20Processing,%20Stack%20Frames%20%26%20Reset.md): vector/frame navigation; verify disputed wording against the PDF.
- [Jorge Cwik prefetch analysis](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): version 1.3, supplementary instruction-order analysis, not a hardware circuit design.
- [Division timing analysis](../Reference/Motorola%2068000%20DIVU%20%26%20DIVS%20Cycle-Accurate%20Timing%20Analysis.md): separates original timing routines from project reconstruction.
- [Silicon verification contract](Verification/CPU%20SingleStepTests.md): input scope, approved overrides, state masks and unobserved signals.
- [Amiga HRM Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): standard motherboard interrupt levels 1-6 and external level-seven debugging inputs.
- [CPU implementation](../../../crates/cpu/src/cpu.rs), [state](../../../crates/cpu/src/state.rs) and [boundary tests](../../../crates/cpu/tests/test_exception_boundaries.rs): project mechanisms and synthetic coverage.
- [Moira exception implementation](../../../ref_src/vAmiga-4.5/Core/Components/CPU/Moira/MoiraExceptions_cpp.h): supporting MC68000 comparison, not an authority over unchanged silicon inputs.
