---
title: "Motorola 68000 CPU Design Specification"
aliases: ["M68000", "68000 CPU", "MC68000", "CPU Specification"]
tags: ["amiga", "design", "m68000", "cpu", "registers"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-09-20
related: ["[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Main loop A500.md](Main%20loop%20A500.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/cpu"
last_synced_commit: "9558483973639a25211924cab2ddea208895723e"
last_synced_date: "2026-09-20"
---
# Motorola 68000 CPU Design Specification

- **Module Location:** `crates/cpu/`
- **Execution Model:** Cycle-exact micro-operations mapped to Color Clock phases (**CCK1** and **CCK2**).
- **Bus Interface:** Interacts with memory strictly via [MemoryBus.md](MemoryBus.md), handling `BusResult::WaitState` and executing direct 2-phase Color Clock read/write transactions (`step_bus_read_word`, `step_bus_write_word`, etc.).
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md) (wrapping arithmetic, Big-Endian decoding, zero panics).
- **Test Validation:** Verified via [CPU SingleStepTests.md](CPU%20SingleStepTests.md) and skill `m68k-singlestep-test`.

---

## 1. CPU State & Register Architecture

The CPU exposes a fully queryable, read-only state snapshot (`CpuState`) for inspection, debugging, and save states. The complete implementation resides in [`crates/cpu/src/state.rs`](../../../crates/cpu/src/state.rs).

### 1.1 Complete M68000 Programmer's Model & State Fields

| Register / Field | Width | Description | Hardware Behavior / Access Rules |
| :--- | :---: | :--- | :--- |
| **`d[0..=7]`** ($D_0-D_7$) | 32-bit | Data Registers | General data registers. Supports Byte, Word, and Long transfers. Low-size writes preserve unaffected high bits. Encapsulated via `d_byte`, `set_d_byte`, `d_word`, `set_d_word`, `d_long`, `set_d_long`. |
| **`a[0..=7]`** ($A_0-A_7$) | 32-bit | Address Registers | Base, pointer, and software stack registers. Byte accesses are invalid. Word writes are sign-extended to 32 bits before committing via `set_a_long`. Encapsulated via `a_word`, `a_long`, `set_a_long`. $A_7$ holds the currently active stack pointer ($USP$ or $SSP$). |
| **`usp`** ($USP$) | 32-bit | User Stack Pointer | Banked $A_7$ when running in User Mode ($SR.S = 0$). |
| **`ssp`** ($SSP$) | 32-bit | Supervisor Stack Pointer | Banked $A_7$ when running in Supervisor Mode ($SR.S = 1$). |
| **`pc`** ($PC$) | 32-bit | Program Counter | Points to instruction memory. 24-bit physical address space on MC68000; internally 32-bit wide. |
| **`sr`** ($SR$) | 16-bit | Status Register | High byte: System Byte (Trace, Supervisor, Interrupt Mask). Low byte: Condition Code Register (CCR). |
| **`prefetch`** | 16-bit | Lookahead Prefetch Register | Models hardware `IR` holding the next staged instruction word (extension word or lookahead opcode) behind active `ir` (`IRD`). |
| **`ir`** ($IR$) | 16-bit | Instruction Register | Holds the opcode currently being executed. Immutable during micro-steps. |
| **`ipl`** ($IPL$) | 8-bit | Interrupt Priority Level | Sampled interrupt priority lines (0..7) driven by Paula/arbitration. |
| **`instruction_pc`** | 32-bit | Architectural Opcode PC | $PC_{\text{hardware}} - 4$ at instruction retirement; represents the base memory address of the executing opcode word (used by GUI disassembler, debugger, breakpoints, and exception vectors). |
| **`stopped`** | bool | STOP Instruction Latch | Processor halted awaiting interrupt higher than current interrupt mask. |
| **`halted`** | bool | Double Bus Fault Latch | Processor halted due to catastrophic hardware failure / reset. |
| **`reset_line_asserted`** | bool | External _RESET Pin Latch | Asserted by privileged `RESET` instruction to signal external chip/CIA reset via machine coordinator without resetting CPU state or RAM. |
| **`cycle_counter`** | 64-bit | Master CPU Cycle Counter | Monotonic accumulator of total elapsed CPU clock cycles since reset; queried via `cycle_counter()`, advanced by `advance_clocks(clocks)`, reset by `reset_cycle_counter()`. |
| **`micro`** | `CpuMicroState` | Execution Micro-State | Tracks Color Clock phase, `micro_step` index, latched bus words, dual staging registers (`addr1`, `addr2`), Data Output Buffer, and wait cycles. |

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

#### Host Hardware Efficiency: Branchless Condition Code Setters
To avoid host branch mispredictions in hot execution paths, the core employs direct branchless bitwise CCR updates (`set_ccr`, `set_ccr_xnzvc`, `set_ccr_nzvc`, `set_ccr_nz_clear_vc`, `set_ccr_z_only`, `set_ccr_v_clear_c`). All CCR setter methods are marked `#[inline(always)]` in [`crates/cpu/src/state.rs`](../../../crates/cpu/src/state.rs).

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
  - For `(d8, An, Xn)`: $EA = (A_n + X_n + \text{sign\_extend}(d_8)) \ \& \ \text{\$00FFFFFF}$.
  - For `(d8, PC, Xn)`: $EA = (PC_{extension} + X_n + \text{sign\_extend}(d_8)) \ \& \ \text{\$00FFFFFF}$.

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

The Motorola 68000 utilizes an overlapped, pipelined instruction prefetch mechanism. Execution of an instruction does not wait for opcode fetching; instead, memory bus fetching and ALU execution overlap concurrently across Color Clock phases.

#### A. The Two-Word Prefetch Queue (`ir` + `prefetch`)
Before **any** instruction can begin execution, the 68000 prefetch FIFO must be completely full:
- **`ir` (Instruction Register, 16-bit):** Holds the opcode currently being decoded and executed (`IRD`).
- **`prefetch` (Lookahead Prefetch Register / `IR`, 16-bit):** Holds the next lookahead word read from memory.
- **`micro.irc` (Instruction Register Capture / `IRC`, 16-bit):** Captures incoming bus data during micro-step execution.

During reset or after any pipeline flush (e.g. taken branch/jump), the processor primes the pipeline via two consecutive bus reads:
```text
Memory Stream:
  $001000:  4E71  (NOP           - 1st instruction)
  $001002:  3200  (MOVE.W D0, D1 - 2nd instruction)
  $001004:  4240  (CLR.W  D0     - 3rd instruction)

Reset / Pipeline Priming Sequence:
1. Bus reads $001000 -> latched into ir.          Hardware PC advances to $001002.
2. Bus reads $001002 -> latched into prefetch.    Hardware PC advances to $001004.
```

When execution of `NOP` begins at `$001000`:
- The active opcode `$4E71` is in `ir`.
- The next opcode `$3200` (`$001002`) is **already read into the CPU** and resides in `prefetch`.
- The physical hardware Program Counter register (`state.pc`) is **already pointing to `$001004`**.

#### B. Architectural Program Counter vs Hardware Bus PC
Because the physical hardware `PC` continuously runs 2 words (4 bytes) ahead on the address bus:
$$\mathbf{PC_{\text{hardware}} = \text{Opcode Address} + 4}$$

To present an intuitive, correct view to programmers, disassemblers, debuggers, and test suites, the state maintains a dedicated architectural field:
$$\mathbf{\text{instruction\_pc} = PC_{\text{hardware}} - 4}$$

At the end of every instruction, `retire_current_instruction()` updates:
```rust
self.state.instruction_pc = self.state.pc.wrapping_sub(4);
```
This guarantees that `instruction_pc` always points to the first byte of the active instruction being displayed or executed, while `state.pc` accurately reflects the silicon memory bus address pins.

#### C. Subroutine Return Addresses (`JSR` / `BSR`) and Stack Pushes
A common misconception is that the advanced hardware `PC` causes subroutine calls or exceptions to push distant return addresses onto the stack. In reality, the microcode offsets the hardware `PC` to calculate the exact return address:
1. **1-Word Calls (`JSR (An)`, `BSR.S`):**
   - Hardware `PC` is at $\text{Opcode} + 4$.
   - The ALU computes $\text{Return Address} = PC_{\text{hardware}} - 2 = \text{Opcode} + 2$.
   - Pushes the exact address of the following instruction onto the stack.
2. **Multi-Word Calls (`JSR $2000.W`, `BSR.W`):**
   - The CPU consumes the extension word from `prefetch`, advancing hardware `PC` to $\text{Opcode} + 4$.
   - Pushes $PC_{\text{hardware}}$ directly, which points exactly past the 4-byte instruction.
3. **PC-Relative Addressing (`d16, PC`):**
   - Per Motorola PRM, the base address for `(d16, PC)` is the instruction address plus two ($PC_{\text{hardware}} - 2$), corresponding to the address where the displacement word was fetched.
4. **Bus Error & Address Error Stack Frames:**
   - As documented in Section 5.3 of the *Motorola 68000 User's Manual*, the PC saved on the stack for Vector 2 and 3 can be **2 to 10 bytes beyond** the address of the instruction that caused the fault, reflecting the advanced state of the prefetch queue when the bus fault occurred.

---

## 2. Dynamic Opcode Cycle Calculation

The emulator does **NOT** rely on a static cycle lookup table. Cycles are calculated dynamically as an emergent property of the micro-operation state machine:

1. **Effective Addressing (EA) Timing:**
   - Register Direct: 0 additional cycles.
   - Address Register Indirect `(An)`: 4 clocks (2 CCK).
   - Post-increment / Pre-decrement `(An)+` / `-(An)`: 4 clocks (2 CCK).
   - Displacement `(d16, An)`: 8 clocks (4 CCK) — 1 prefetch word + address calc.
   - Indexed `(d8, An, Xn)`: 10 clocks (5 CCK) — 1 prefetch word + index addition + internal idle clock.
2. **Data-Dependent Operand Timing:**
   - **`DIVU` (Unsigned Division):** Dynamically takes between 38 and 140 CPU clocks depending on quotient bit loop cancellations and zero divisor detection.
   - **`DIVS` (Signed Division):** Dynamically takes between 122 and 158 CPU clocks depending on sign resolution and division iterations.
   - **`MULU` / `MULS`:** Takes 38 to 70 CPU clocks (cycles scale dynamically with the number of 1-bits in the multiplier).
   - **Shifts & Rotates (`ASL`, `LSR`, etc.):** 6 or 8 base clocks + 2 clocks per bit shifted.
3. **Branch Conditions:**
   - `Bcc`: 10 clocks if branch taken, 8 clocks if not taken.
4. **Bus Wait State Accumulation:**
   - Every CCK cycle where `MemoryBus` returns `BusResult::WaitState` adds exactly **1 CCK (2 CPU clocks)** to the instruction's total duration.

### 2.1 Modern Host CPU Pipelining & Direct Code Flow Architecture

Modern superscalar host CPUs (x86_64, aarch64, Apple Silicon) rely on deep 14–20+ stage execution pipelines, aggressive branch prediction, and speculative execution.

#### The Branch Misprediction Bottleneck in Emulation
In naive instruction dispatch, cascaded runtime condition checks:
```text
Opcode Fetch -> match (opcode >> 12) -> match opmode -> match ea_mode -> if size == Byte ...
```
force the host CPU through 8–15 conditional branches per emulated instruction. Because consecutive M68000 instructions change constantly in real guest code, host branch predictors suffer high misprediction rates. Each pipeline flush costs **15 to 20 wasted CPU cycles**.

#### Direct-Threaded / Table-Driven Opcode Dispatch
To maximize host throughput, verified reference emulators (Musashi via `m68kmake`, WinUAE via `gencpu`, and Moira via C++ template specialization) structure the CPU core as **direct, flattened code flows**:
- **65,536-Entry Direct Descriptor Table (`OPCODE_DESCRIPTOR_TABLE: [OpcodeDescriptor; 65536]`):** Every 16-bit opcode indexes directly into a precalculated array of static opcode descriptors (`crates/cpu/src/micro/dispatch_table.rs`). Each entry contains a reference to an immutable slice of specialized atomic `MicroStep`s, along with pre-decoded register indices (`reg_src`, `reg_dst`).
- **Cached Slice Pointer Dispatch (`current_steps`):**
  - Upon opcode prefetch and retirement, `state.micro.current_steps` caches the slice pointer directly from `OPCODE_DESCRIPTOR_TABLE[ir]`. All subsequent CCK ticks during the instruction index `current_steps[micro_step]` directly, completely eliminating 65,536-entry table lookups in the hot execution loop.
  - Micro-steps cleanly decouple atomic bus cycles (`BusReadWord`, `BusWriteByte`, `BusWriteWord`, `BusWriteLongHigh`, etc.) from parametric, bus-free ALU operations (`AluFn`).
  - Pre-decoded register operands (`reg_src`, `reg_dst`) collapse repetitive opcode implementations across all 8 data/address registers into a single shared ALU routine per operation, with zero dynamic branching or macro boilerplate.
  - Host CPU Branch Target Buffers (BTBs) predict indirect dispatches with high efficiency, maximizing instruction cache locality and superscalar throughput.

### 2.2 Endianness Bypass & Fast-Path Optimization Opportunities

The Motorola 68000 is strictly Big-Endian, while modern host architectures (x86_64, aarch64) are Little-Endian. While arithmetic operations (`ADD`, `SUB`, `CMP`, multiplies, divides) require strict Big-Endian value decoding due to directional carry propagation, several classes of instructions can **completely bypass endian byte swapping**:

#### 1. Bitwise Logical Operations (`AND`, `OR`, `EOR`, `NOT`)
Bitwise logical operations are strictly pointwise and commute with byte reversal:
$$\text{bswap}(A \ \& \ B) = \text{bswap}(A) \ \& \ \text{bswap}(B)$$
- When applying bitwise logic between raw memory buffers or host registers:
  - Performing a native 16-bit or 32-bit `AND`, `OR`, `XOR`, or `NOT` directly on raw guest bytes produces the **exact same memory result** without swapping before and after the operation.
- **Condition Codes (CCR):**
  - Zero flag ($Z$): Zero is zero regardless of byte ordering (`val == 0`).
  - Negative flag ($N$): In memory, the sign bit is simply bit 7 of the first byte (`addr[0] & 0x80 != 0`).
  - Overflow ($V$) and Carry ($C$): Always cleared to 0.
  - Extend ($X$): Unaffected.

#### 2. Direct Memory-to-Memory Transfers & Block Moves (`MOVE`, `MOVEM`, DMA)
- When moving memory words or longs between memory locations, or transferring blocks via DMA (Blitter, Copper, Floppy):
  - A sequence of bytes is identical regardless of endian interpretation.
  - Data can be transferred via raw native memory copies (`memcpy` or direct word/dword moves) without any `bswap` or `to_be_bytes`/`from_be_bytes` overhead.

#### 3. Clear & Zero-Check Operations (`CLR`, `TST`)
- **`CLR`:** Storing zeros (`0x00`, `0x0000`, `0x00000000`) is identical in all byte orders.
- **`TST`:** Testing for zero evaluates directly against 0; testing negative checks bit 7 of byte 0.

#### Summary Matrix: Endian Conversion Requirements

| Instruction Category | Examples | Endian Swapping Required? | Rationale |
| :--- | :--- | :---: | :--- |
| **Bitwise Logic** | `AND`, `OR`, `EOR`, `NOT` | ❌ **No (Bypassable)** | Pointwise operations commute with byte reversal; no carry propagation across bytes. |
| **Block Copies** | `MOVE (An), (Am)`, `MOVEM`, DMA | ❌ **No (Bypassable)** | Byte stream is identical in memory. |
| **Zeroing** | `CLR` | ❌ **No (Bypassable)** | All-zeros representation is identical in BE and LE. |
| **Arithmetic** | `ADD`, `SUB`, `CMP`, `NEG` | ✅ **Yes (Required)** | Carry propagates right-to-left across byte boundaries. |
| **Shifts & Rotates** | `LSL`, `LSR`, `ASL`, `ASR`, `ROL`, `ROR` | ✅ **Yes (Required)** | Bits cross byte boundaries. |
| **Address Calculation** | `(d16, An)`, `(d8, An, Xn)`, PC-relative | ✅ **Yes (Required)** | Mathematical pointer arithmetic. |

### 2.3 M68000 Opcode Bitfield Decoding & Instruction Topology

> [!NOTE]
> For emulator codebase function naming conventions, consult the operational rule in [opcode-naming.md](../../../.agents/rules/opcode-naming.md).

The 16-bit M68000 opcode word is partitioned into five canonical bitfields:

```text
 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
|  Major Group  |  Register |    Opmode     |   Mode    |  Register |
+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
 \_____________/ \_________/ \_____________/ \_______/   \_________/
    Bits 15-12    Bits 11-9     Bits 8-6      Bits 5-3    Bits 2-0
  Instruction    Destination   Direction /    EA Mode     EA Reg
     Family       / Condition  Size / Sub-op
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

#### 2. Bits 11–9: Register & Condition Field

- **Standard ALU (`ADD`, `SUB`, `AND`, `OR`, `CMP`):** Primary Data Register $D_n$ index (`0`–`7`).
- **`MOVE` instructions:** Destination Data/Address register index.
- **Branches (`Bcc`, `DBcc`, `Scc`):** 4-bit / 3-bit condition code test (`0000`–`1111`).
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

Memory access is mapped to Color Clock phases (**CCK1** and **CCK2**):
- $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks (S0-S7)} = 2\ \text{CCK cycles (CCK1 + CCK2)}$.

### 3.1 Read Transaction Contention
- **CCK1 (S0–S3):** The CPU asserts the target address and attempts reading via `bus.read_word(addr)` or `bus.read_byte(addr)`:
  - If `BusResult::WaitState` (Chip RAM access blocked by active Agnus DMA):
    - **Action:** CPU stalls at CCK1. Does NOT advance micro-step. Repeats CCK1 read step on next clock.
  - If `BusResult::Ready(data)`:
    - Data is stored directly into the target register (`source`, `destination`, `prefetch`, or `irc`); CPU advances to the CCK2 finish step.
- **CCK2 (S4–S7):**
  - Physical bus is already idle/released for custom chip DMA. The transaction is recorded directly from the target register, completing the bus cycle and advancing to the next micro-step.

### 3.2 Write Transaction Contention
- **CCK1 (S0–S3):** The CPU outputs the address internally and asserts `_AS` (`step_bus_write_idle`).
  - CPU proceeds to CCK2 (`step_bus_write_dst_*`).
- **CCK2 (S4–S7):** The memory bus attempts committing the write via `bus.write_word(addr, val)` or `bus.write_byte(addr, val)` reading directly from `state.micro.destination`:
  - If `BusResult::WaitState` (Gary withholds `_DTACK` due to Chip RAM DMA contention):
    - **Action:** CPU stalls at CCK2, holding write pins asserted until Agnus frees the bus without advancing `micro_step`.
  - If `BusResult::Ready(())`:
    - Byte/word commits to memory, transaction is recorded, completing the 4-clock bus cycle and advancing to the next micro-step.

### 3.3 Micro-Step State Machine & Instruction Lifecycle

> [!NOTE]
> For the complete, dedicated microcode architectural blueprint, specialized atomic bus primitives, strobe semantics, and cycle traces across all 10 instruction classes, see [[CPU Micro-Step State Machine.md]].

Instruction execution is driven via a cycle-exact micro-step state machine clocked at Color Clock (CCK) granularity (2 CPU clocks per CCK, 4 clocks per bus cycle). The execution engine and micro-state tracking are implemented in [`crates/cpu/src/micro/engine.rs`](../../../crates/cpu/src/micro/engine.rs) and [`crates/cpu/src/cpu.rs`](../../../crates/cpu/src/cpu.rs).

- **Execution Micro-State (`CpuMicroState`):**
  - `source`: Explicit 32-bit storage for ALU source operand (incoming bus data is stored directly here on CCK1).
  - `destination`: Explicit 32-bit storage for ALU destination operand and write-back data (bus write cycles read directly from here).
  - `irc`: Instruction Register Capture — physical 68000 prefetch latch holding prefetched opcodes before retirement into IR.
  - `ea_addr`: Resolved effective memory address for operands or branch/jump targets.
  - `addr1`: Dual Staging Register 1 ($X_1$) — pre-staged effective address for multi-phase and dual-memory transfers (e.g. source EA or high-word split EA in `CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`).
  - `addr2`: Dual Staging Register 2 ($X_2$) — pre-staged effective address for multi-phase transfers (e.g. destination EA or low-word split EA).
  - `ea_high`: High word of 32-bit absolute addresses (`(xxx).L`) or high address for split accesses.
  - `reg_src`, `reg_dst`: Pre-decoded source and destination register indices ($0..7$ for $D_n/A_n$).
  - `movem_mask`: 16-bit register transfer mask for `MOVEM`.
  - `movem_state`: Multi-cycle transfer state for `MOVEM` (bit 0 tracks CCK1 vs CCK2 sub-phase).
  - `clocks_remaining`: Remaining CPU clocks for the active micro-step countdown (0 when completed or between steps).
  - `micro_step`: Index of the currently executing micro-operation within the active opcode sequence.
  - `current_steps`: Cached slice pointer to the active opcode's compiled micro-step sequence (`&'static [MicroStep]`).
  - `target_refill`: Indicates whether instruction retirement must perform a branch/jump target refill.
  - `prefetch_retired`: Indicates whether prefetch pipeline has already retired into IR during microcode execution.
  - `fault_addr`: Latched memory address that triggered Group 0 Address Error / Bus Error exception.
  - `info_word`: 16-bit Internal Information Word ($R/\overline{W}$, $I/N$, Function Code bits $FC_0-FC_2$) for 7-word exception frame.
  - `ssp_base`: Base supervisor stack pointer snapshot at the start of exception frame stacking.

- **Unified Control Model (Zero `StepResult` Overhead):**
  - **`BusFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>`**: Micro-step handlers report bus readiness (`BusResult::Ready(())` or `BusResult::WaitState`) against any decoupled `AddressBus`.
  - **`step_cck(&mut self, bus: &mut dyn AddressBus) -> bool`**: Executes a single CCK color clock cycle (~280 ns) and returns `true` when the instruction completes/retires, `false` otherwise. Performs boundary validation and initiates uninitialized instructions.
  - **`step_cck_internal(&mut self, bus: &mut dyn AddressBus) -> bool`**: Internal hot-loop primitive executing micro-steps directly without redundant boundary checks.
  - **`step_instruction(&mut self, bus: &mut dyn AddressBus) -> u32`**: Steps through an entire instruction/opcode to retirement, returning the exact CPU clock cycles consumed.
  - **`restore_state(&mut self, state: CpuState)`**: Restores the CPU architectural and micro-state from a snapshot, automatically re-hydrating the cached `&'static [MicroStep]` execution slice pointer (`current_steps`) from `OPCODE_DESCRIPTOR_TABLE[ir]` without requiring external two-step hydration.
  - **`set_pc_and_prime_prefetch(&mut self, target_pc: u32, bus: &mut dyn AddressBus)`**: Synthetic pre-execution helper used by debugger and unit test suites; sets `instruction_pc` and `pc`, and primes the 2-word prefetch queue (`ir` and `prefetch`) directly without cold reset overhead.
  - **State flags**: Halted and Stopped states are queried directly on `cpu.state.halted` and `cpu.state.stopped`.

#### Specialized Direct Micro-Step Execution Handlers (`BusFn`)
To eliminate nested dynamic runtime size checks and dynamic branching (`match`) in the hot execution loop, micro-step operations are specialized directly into atomic function pointers (`BusFn = fn(cpu: &mut Cpu, bus: &mut dyn AddressBus) -> BusResult<()>`):
- **Operand Reads:** `Cpu::step_bus_read_src_byte`, `Cpu::step_bus_read_src_word`, `Cpu::step_bus_read_src_long_high`, `Cpu::step_bus_read_src_long_low`, `Cpu::step_bus_read_dst_byte`, `Cpu::step_bus_read_dst_word`, `Cpu::step_bus_read_dst_long_high`, `Cpu::step_bus_read_dst_long_low`.
- **Dual Staged Reads & Writes:** `Cpu::step_bus_read_addr1_*`, `Cpu::step_bus_read_addr2_*`, `Cpu::step_bus_write_addr2_*`.
- **Operand Writes:** `Cpu::step_bus_write_dst_byte`, `Cpu::step_bus_write_dst_word`, `Cpu::step_bus_write_dst_long_high`, `Cpu::step_bus_write_dst_long_low`.
- **Stack & Control Flow:** `Cpu::step_bus_pop_stack_*`, `Cpu::step_bus_push_stack_*`, `Cpu::step_bus_read_target_opcode_*`, `Cpu::step_prefetch_target_*`.
- **Multi-Register Block Transfers:** `crate::instructions::movem::execute_movem_transfer`.

#### Pipeline & Dispatch Table Invariants
1. **Immutable `ir` During Micro-Steps:** The 65,536-entry static descriptor table (`OPCODE_DESCRIPTOR_TABLE`) is indexed upon instruction prefetch to cache `current_steps: &'static [MicroStep]`. Intermediate multi-step operations (e.g. `JSR` or taken `Bcc`) must never overwrite `cpu.state.ir` before final retirement. Target opcodes are captured directly into `state.micro.irc` with `state.micro.target_refill = true`, and committed to `cpu.state.ir` only upon retirement in `retire_current_instruction()`.
2. **32-Bit Internal Program Counter:** The MC68000 Program Counter register is 32-bit wide internally. Across branch and jump target refills, `pc` is computed as `target.wrapping_add(4)` without 24-bit truncation mask `& 0x00FF_FFFF` (matching verified hardware tests in SingleStepTests).
3. **Control Addressing Modes Alignment:** Control addressing modes in `PEA` and `LEA` compute effective addresses without checking word alignment. Odd addresses can be pushed onto the stack by `PEA` without generating Vector 3 Address Error. Only an unaligned Stack Pointer ($SP$) during stack writeback triggers an Address Error.

---

## 4. Two-Word Instruction Prefetch Pipeline

The M68000 maintains a two-word prefetch queue:
- **`IR` (Instruction Register):** Holds the opcode currently being decoded and executed.
- **`IRC` (Instruction Register Capture):** Holds the next 16-bit word prefetched from memory.

### Prefetch Pipeline Rules
1. **Initial State (After Reset):**
   - `IR` contains the first opcode at `Initial_PC`.
   - `IRC` contains the prefetch word at `Initial_PC + 2`.
   - `PC` register holds `Initial_PC + 4`.
2. **Consuming Extension Words / Immediates:**
   - Any immediate data, 16-bit displacement, or extension word is **consumed directly from `IRC`**.
   - The CPU immediately initiates a prefetch read cycle at address `PC` to refill `IRC`.
   - `PC` is incremented by 2 (`pc = pc.wrapping_add(2)`).
3. **Instruction Retirement:**
   - During the final bus cycle of an instruction, `IRC` transfers to `IR` for the next opcode.
   - The CPU reads the next instruction stream word from `PC` into `IRC` and increments `PC` by 2.

---

## 5. Exception Processing & Vectors

- **Vector Table Range:** `$000000-$0003FF` (256 32-bit vector addresses).
- **Core Exceptions:**
  - `0`: Reset Initial SSP (High & Low word)
  - `1`: Reset Initial PC (High & Low word)
  - `2`: Bus Error (`_BERR` — omitted; stock A500 does not assert `_BERR`)
  - `3`: Address Error (unaligned word/long access aborts execution and pushes Address Error stack frame)
  - `4`: Illegal Instruction
  - `5`: Zero Divide
  - `6`: CHK Instruction
  - `7`: TRAPV Instruction
  - `8`: Privilege Violation
  - `9`: Trace
  - `10`: Line-A (`ILLEGAL_LINEA`)
  - `11`: Line-F (`ILLEGAL_LINEF`)
  - `25-31`: Autovector Interrupts Level 1–7

### 5.1 Address Error (Vector 3) 7-Word Stack Frame

When a word (`.w`) or long (`.l`) bus transfer targets an odd address (`addr & 1 != 0`), the MC68000 aborts instruction execution and pushes a **7-word Group 0/1 exception stack frame** onto the supervisor stack ($SSP$):

```text
SP + 00: [ R/W | I/N | Function Code (FC0-FC2) ]  (Internal Information Word)
SP + 02: [ High 16 bits of Access Address       ]
SP + 04: [ Low 16 bits of Access Address        ]
SP + 06: [ Instruction Register (IR)            ]
SP + 08: [ Status Register (SR)                 ]
SP + 10: [ High 16 bits of Program Counter (PC) ]
SP + 12: [ Low 16 bits of Program Counter (PC) ]
```
- **Internal Information Word (Word 0):**
  - Bit 4: $R/\overline{W}$ (`1` = Read fault, `0` = Write fault).
  - Bit 3: $I/N$ (`1` = Processor was executing instruction, `0` = Exception processing).
  - Bits 2–0: Function Code bits ($FC_2, FC_1, FC_0$).
- Vector address is loaded from `$00000C` (Vector 3), and execution resumes in supervisor mode ($S=1, T=0$).

### 5.2 Privilege Violation (Vector 8) Microcode Pipeline
When a privileged instruction (`RESET`, `STOP`, `RTE`, `MOVE to SR`, `ANDI/EORI/ORI to SR`, `MOVE USP`) is executed in User Mode ($SR.S = 0$), the processor traps to Vector 8 (`$000020`) consuming exactly 34 CPU clocks (17 CCKs) via `STEPS_PRIVILEGE_VIOLATION`:
1. **`ALU_PRIVILEGE_VIOLATION_INIT` (2 clocks):** Saves return PC (`instruction_pc`) in `source`, old SR in `destination`, switches to Supervisor mode ($S=1, T=0$), and loads Vector 8 address (`$000020`) into `ea_addr`.
2. **`ALU_IDLE` (2 clocks):** Internal exception setup latency.
3. **Stack Frame Pushes (12 clocks):** Pushes return PC low word to $SP-2$, old SR to $SP-6$, and return PC high word to $SP-4$, committing $SP \leftarrow SP - 6$.
4. **Vector Fetch (8 clocks):** Reads high and low words of handler address from `$000020` into `ea_addr`.
5. **Prefetch Target Refill (10 clocks):** Reads first target opcode from `ea_addr` and prefetches second word from `ea_addr + 2`, transferring control to the exception handler.

### 5.3 Divide-by-Zero (Vector 5) Microcode Pipeline
When `DIVU` or `DIVS` encounters a zero divisor (`source == 0`), the ALU halts division and triggers Vector 5 (`$000014`) consuming 38 CPU clocks via `STEPS_DIV_ZERO`:
1. **`ALU_IDLE_8CLK` (8 clocks):** Internal division zero-detection latency.
2. **Setup:** Updates SR flags ($N, Z, V, C$ cleared, $X$ preserved), switches to Supervisor mode ($S=1, T=0$), snapshots return PC and old SR.
3. **Stack Frame Pushes (12 clocks):** Pushes standard 3-word exception frame ($SP-2$ PC low, $SP-6$ SR, $SP-4$ PC high).
4. **Vector Fetch (8 clocks):** Reads 32-bit vector from `$000014`.
5. **Prefetch Target Refill (10 clocks):** Refills 2-word pipeline from target address.

### 5.4 Double Bus Fault on Odd Reset Vector 1
During cold or warm reset exception processing, if the initial Program Counter read from Vector 1 (`$000004`) has bit 0 set (`pc & 1 != 0`), instruction prefetch cannot proceed across the 16-bit data bus. Because an Address Error occurs during reset exception processing itself, the MC68000 silicon triggers an immediate **Double Bus Fault**, permanently halting the CPU (`state.halted = true`) until external hardware reset.

---

## 6. Reset Procedure & Hardware Pin Signaling

The Motorola 68000 features a dedicated bidirectional `_RESET` pin that operates in two distinct electronic modes:
1. **Input Mode (Cold & Warm System Reset):** An external active-low signal drives `_RESET` (along with `_HALT`), forcing processor initialization.
2. **Output Mode (Instruction Reset):** Executing the privileged `RESET` opcode drives `_RESET` low as an output, resetting peripheral devices without resetting the CPU core.

### 6.1 Cold and Warm CPU Reset Sequences

On both **Cold** and **Warm** reset, the CPU execution flow begins at vector `$000000`:

1. **Overlay Active:** Low-memory overlay (`_OVL`) routes `$000000-$07FFFF` to Kickstart ROM.
2. **Status Register:** Set to `$2700` ($S=1, T=0, I=7$).
3. **Fetch Initial SSP:** Read 32-bit value from `$000000` into `ssp` (active `A7`).
4. **Fetch Initial PC:** Read 32-bit value from `$000004` into `pc` and `instruction_pc`. If `pc & 1 != 0` (odd address), the processor cannot prefetch across the 16-bit bus, immediately halting the CPU (`halted = true`) via Double Bus Fault.
5. **Fill Prefetch:** Read word at `pc` into `ir`, increment `pc += 2`; read word at `pc` into `prefetch` (lookahead prefetch register), increment `pc += 2`.
6. **Execution:** Begin execution at `pc` in Kickstart ROM.
   - Kickstart inspects RAM contents for magic resident checksums to determine whether to perform a warm reboot or cold boot.

### 6.2 The `RESET` Instruction & External Pin Signaling (`reset_line_asserted`)

The `RESET` instruction is a privileged M68000 instruction that asserts the external `_RESET` line for 124 clock cycles (total instruction duration: 132 CPU clocks / 66 CCKs):
- **Privilege Gate:** If executed in User Mode ($SR.S = 0$), the processor immediately aborts execution and triggers a **Privilege Violation exception (Vector 8)** without asserting the reset pin.
- **CPU State Invariance:** When executed in Supervisor Mode ($SR.S = 1$), CPU data/address registers ($D_0-D_7, A_0-A_7$), stack pointers ($USP, SSP$), Program Counter ($PC$), and Status Register ($SR$) are **completely unaffected**. The processor simply executes internal idle clocks and sequential opcode prefetch.
- **Decoupled Pin Latching (`reset_line_asserted`):**
  - In `crates/cpu/src/instructions/reset.rs`, `alu_reset()` latches `state.reset_line_asserted = true`.
  - The top-level machine loop coordinator (`crates/machine_loop/src/machine_loop.rs`) samples this line during its Color Clock progression.
  - When asserted, `machine_loop` invokes `reset_external_devices()`, resetting Agnus, Denise, Paula, CIAs, and re-engaging the Gary boot overlay (`_OVL`) without touching RAM or CPU registers, cleanly decoupling microcode execution from machine-level coordination.

### 6.3 System Reset Comparison Matrix

| Feature / State | Cold Reset (`reset()`) | Warm Reset (`reset_warm()`) | Instruction Reset (`RESET` Opcode) |
| :--- | :--- | :--- | :--- |
| **Trigger Origin** | Power-on / Host UI cold start | Keyboard combo (`Ctrl+Amiga+Amiga`) / host warm reboot | Guest program executing privileged `RESET` opcode |
| **Data Registers ($D_0-D_7$)** | Cleared to `$00000000` | **Preserved intact** | **Preserved intact** |
| **Address Registers ($A_0-A_6, USP$)**| Cleared to `$00000000` | **Preserved intact** | **Preserved intact** |
| **Supervisor SP ($SSP / A_7$)** | Reloaded from Vector 0 (`$000000`) | Reloaded from Vector 0 (`$000000`) | **Preserved intact** |
| **Program Counter ($PC$)** | Reloaded from Vector 1 (`$000004`) | Reloaded from Vector 1 (`$000004`) | Advances sequentially to next instruction |
| **Status Register ($SR$)** | Set to `$2700` | Set to `$2700` | **Preserved intact** |
| **Physical RAM** | Initialized / cleared | **Preserved intact** | **Preserved intact** |
| **Custom Chips & CIAs** | Reset to defaults | Reset to defaults | Reset to defaults (`reset_external_devices()`) |
| **Gary Boot Overlay (`_OVL`)**| Asserted (Kickstart overlay active) | Asserted (Kickstart overlay active) | Asserted (re-engages overlay) |

---

## 7. M68000 Instruction & Hardware Silicon Quirks

> [!NOTE] Centralized Silicon Quirks Catalog
> Comprehensive physical silicon idiosyncrasies, micro-architectural traps (such as Post-Increment `(An)+` AGU commitment asymmetry, `ASR` count $\ge$ width register exhaustion, and `MOVE to -(An)` prefetch inversion), and motherboard circuit errata are centralized in [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md#2-motorola-68000-silicon-quirks--cpu-pipeline-traps).

### 7.1 TAS (Test And Set) Read-Modify-Write Hardware Bug

The `TAS` instruction tests a byte operand, updates the condition codes, and sets high bit 7 to 1:

- **Register Operand (`TAS Dn`):**
  - Operates purely internally in the CPU ALU.
  - Tests low byte `Dn[7:0]`.
  - Sets $N$ if bit 7 was set; sets $Z$ if byte was zero; clears $V$ and $C$ ($X$ unaffected).
  - Sets bit 7 of `Dn` to 1. Always succeeds.
- **Memory Operand (`TAS <ea>`):**
  - Initiates an unbroken Read-Modify-Write (RMW) cycle on the bus:
    1. **Read Phase:** Reads byte from memory address `<ea>`, evaluates value, and updates CCR ($N, Z$ updated, $V, C$ cleared, $X$ untouched).
    2. **Write Phase:** Attempts to write the value back with bit 7 set to 1.
  - **Amiga 500 Hardware Quirk:**
    - **Chip RAM (`$000000-$07FFFF`) & Slow RAM (`$C00000-$C7FFFF`):** Gary / Agnus fails to latch the write phase of an unbroken RMW bus cycle and **discards the write**. CCR flags are updated, but memory content is unmodified (bit 7 is NOT set).
    - **Fast RAM (`$200000-$5FFFFF`):** Full Read-Modify-Write cycle succeeds (CCR flags updated, bit 7 is set to 1 in memory).

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

### 7.5 Shifts & Rotates Micro-Step Pipeline & Silicon Quirks (`ASd`, `LSd`, `ROXd`, `ROd`)

Motorola 68000 Group 0xE encompasses four operation types across register and memory forms: Arithmetic Shift (`ASL`/`ASR`), Logical Shift (`LSL`/`LSR`), Rotate with Extend (`ROXL`/`ROXR`), and Rotate without Extend (`ROL`/`ROR`).

- **Register Shift Micro-Step Pipeline & Clocks ($6/8 + 2n$):**
  - **Prefetch Bus Cycle First:** On real silicon (verified by Tom Harte test vectors), the CPU initiates the next instruction opcode prefetch during micro-step 0 (`common::PREFETCH_NEXT_READ`, 4 clocks / 2 CCKs).
  - **Internal Idle Clocks:** The execution unit schedules internal processing clocks based on operand size and shift count:
    - **Byte / Word:** $idle\_clocks = 2 + 2 \times count$ (Total execution time: $6 + 2n$ clocks).
    - **Long (32-bit):** $idle\_clocks = 4 + 2 \times count$ (Total execution time: $8 + 2n$ clocks).
  - **Retirement:** Micro-step 2 marks standard sequential prefetch retirement (`common::BUS_READ_IDLE`), completing the prefetch into `state.micro.irc` and calling `retire_current_instruction()` to advance the program counter.

- **Memory Shift Micro-Step Pipeline (Class 0 Read-Modify-Write):**
  - Memory shifts are strictly **Word size** and shift by **1 bit** only (`count = 1`).
  - Follows the standard 3-step RMW sub-cycle sequence:
    - **Step 1 (Read):** Read 16-bit word from effective address memory via linear EA resolution.
    - **Step 2 (Prefetch & Compute):** Perform 1-bit shift, evaluate condition codes, initiate next opcode prefetch, and latch modified word into `state.micro.destination`.
    - **Step 3 (Writeback):** Write modified 16-bit word back to the target memory address via bus write cycle.
  - Concludes with pipeline retirement from `state.micro.irc`. Total duration: 12–16 clocks depending on addressing mode (e.g. `(An)` is 12 clocks, `-(An)` is 14 clocks).

- **Shift Count Modulo:**
  - When the shift count is held in a data register, the CPU evaluates only the lower 6 bits (`count % 64` / `count & 63`).
- **Zero Shift Count (`count == 0`):**
  - If the shift count is zero:
    - $C$ (Carry) is cleared to `0` for `ASd`, `LSd`, `ROd`. For `ROXd`, $C = X$.
    - $V$ (Overflow) is cleared to `0`.
    - **$X$ (Extend) is completely untouched** (retains prior value).
    - $N$ and $Z$ flags reflect the value of the unshifted operand.
- **Rotate Variations:**
  - **`ROd` (Rotate):** Circular rotation without extend bit. The last bit rotated out is copied to $C$. $X$ is completely unaffected.
  - **`ROXd` (Rotate with Extend):** 9-bit, 17-bit, or 33-bit rotation including the $X$ flag. The last bit shifted out is copied to both $C$ and $X$.
- **`ASL` Sticky Overflow ($V$) Quirk:**
  - In Arithmetic Shift Left, the $V$ flag indicates whether the sign bit changed.
  - In multi-bit shifts, if the sign bit (MSB) changes at **any intermediate bit shift step**, $V$ is set to `1` and **latches high (sticky)**, remaining `1` even if subsequent shift steps restore the sign bit.
- **`ASR` Count $\ge$ Width Silicon Exhaustion:**
  - When shift count exceeds operand width, physical shifter pipeline exhaustion forces $C=0, X=0$ per [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md#2-motorola-68000-silicon-quirks--cpu-pipeline-traps).

### 7.6 Address Error (Vector 3) Program Space Selection (FC 2 / 6)

- **Function Code Selection on Address Error:**
  - If the unaligned word/long access was triggered by a **PC-relative addressing mode** (`(d16, PC)` or `(d8, PC, Xn)`), the CPU asserts **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode).
  - For standard data memory operands, the CPU asserts **Data Space** ($FC = 1$ in User mode, $FC = 5$ in Supervisor mode).
- **AGU Register Commitment & Predecrement Bus Inversion:**
  - Detailed physical silicon rules governing `(An)+` read/write commitment asymmetry and `MOVE ..., -(An)` prefetch-before-write sequencing reside in [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md#2-motorola-68000-silicon-quirks--cpu-pipeline-traps).

### 7.7 MOVE.l 32-Bit Memory-to-Memory CCR Evaluation

- In 32-bit `MOVE.l <ea>, (An)` transfers:
  - **Real MC68000 Silicon:** Condition codes reflect the full 32-bit transfer ($N = \text{bit } 31$, $Z = \text{value } == 0$).
  - *Emulator Resolution:* The core strictly implements the full 32-bit condition code evaluation matching real silicon.

### 7.8 Branch & Control Flow Odd Target Address Error (FC 2 / 6)

- When `BRA`, `Bcc`, `JMP`, or `JSR` evaluates a target address with an odd destination (`target & 1 != 0`):
  - The MC68000 halts instruction execution and immediately triggers an **Address Error exception (Vector 3)**.
  - Because the instruction fetch pipeline caused the fault, the CPU asserts **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode) in the exception status word.
  - The pushed program counter in the stack frame points to the instruction boundary or target address.

### 7.9 Class 0 Read-Modify-Write (RMW) & Bit Manipulation Silicon Timings

- **Class 0 RMW Memory Writeback Sequence (`AND`, `OR`, `EOR`, `NOT` to `<ea>`):**
  - Memory-destination logical operations follow the Class 0 Read-Modify-Write sub-cycle pipeline in `and.rs`, `or.rs`, `eor.rs`, and `not.rs`:
    - **Step 1 (Read):** Read operand from effective address memory.
    - **Step 2 (Prefetch & Compute):** Perform bitwise operation, compute condition codes ($N, Z, V=0, C=0$), initiate prefetch of next opcode, and store result into `state.micro.destination`.
    - **Step 3+ (Writeback):** Commit modified value to target memory address (for 32-bit `Long` size: write low word to $addr + 2$, followed by high word write to $addr$).
    - Pipeline retires with next instruction opcode already latched into $IR$.
- **Bit Manipulation Cycle Timings (Tom Harte Silicon Verified):**
  - Implemented in dedicated per-mnemonic modules (`btst.rs`, `bchg.rs`, `bclr.rs`, `bset.rs`):
  - **Dynamic Bit Ops ($D_n$, `<ea>`):**
    - `BTST Dn, Dm`: 6 clocks (4 prefetch + 2 internal idle).
    - `BCHG Dn, Dm` / `BSET Dn, Dm`: 8 clocks (4 prefetch + 4 internal idle).
    - `BCLR Dn, Dm`: 10 clocks (4 prefetch + 6 internal idle).
    - Memory targets: Bit modulo 8; `BTST` is read-only; `BCHG`, `BCLR`, and `BSET` execute Class 0 RMW single-byte writeback.
  - **Static Bit Ops (`#<data>`, `<ea>`):**
    - `BTST #imm, Dm`: 10 clocks (4 extension read + 4 prefetch + 2 internal idle).
    - `BCHG #imm, Dm` / `BSET #imm, Dm`: 12 clocks (4 extension read + 4 prefetch + 4 internal idle).
    - `BCLR #imm, Dm`: 14 clocks (4 extension read + 4 prefetch + 6 internal idle).
    - Memory targets: Extension word fetch, effective address read, prefetch, and byte writeback.

### 7.10 Modular Per-Mnemonic Instruction Architecture & Single-Mnemonic Dispatch

The entire M68000 instruction set is organized into dedicated, single-responsibility files directly under [`crates/cpu/src/instructions/`](../../../crates/cpu/src/instructions/) following strict architectural rules:

- **1:1 Mnemonic-to-File Hierarchy & Zero Subdirectories Mandate:**
  - Every distinct M68000 instruction mnemonic has its own dedicated flat `.rs` file (e.g. `add.rs`, `sub.rs`, `mulu.rs`, `muls.rs`, `divu.rs`, `divs.rs`, `link.rs`, `unlk.rs`, `abcd.rs`, `sbcd.rs`, `nbcd.rs`, `trapv.rs`, `rtr.rs`, `rte.rs`, `stop.rs`, `reset.rs`, `move_usp.rs`, `bra.rs`, `bsr.rs`, `bcc.rs`, `asl.rs`, `asr.rs`, etc.).
  - **Zero Subdirectories:** Creating subdirectories or multi-file submodules under `crates/cpu/src/instructions/` is strictly forbidden. The hierarchy must remain 100% flat; any nested subdirectory at any depth causes `cargo test -p test_runner --test test_architecture_rules` to fail immediately.
  - **Elimination of Legacy Umbrella Files:** Disparate instructions are never grouped into composite umbrella modules. All legacy multi-instruction files have been decomposed:
    - `mul.rs` $\rightarrow$ [`mulu.rs`](../../../crates/cpu/src/instructions/mulu.rs), [`muls.rs`](../../../crates/cpu/src/instructions/muls.rs)
    - `div.rs` $\rightarrow$ [`divu.rs`](../../../crates/cpu/src/instructions/divu.rs), [`divs.rs`](../../../crates/cpu/src/instructions/divs.rs) (shared zero-divide exception micro-step sequences centralized in [`micro/common.rs`](../../../crates/cpu/src/micro/common.rs))
    - `link_unlk.rs` $\rightarrow$ [`link.rs`](../../../crates/cpu/src/instructions/link.rs), [`unlk.rs`](../../../crates/cpu/src/instructions/unlk.rs)
    - `bcd.rs` $\rightarrow$ [`abcd.rs`](../../../crates/cpu/src/instructions/abcd.rs), [`sbcd.rs`](../../../crates/cpu/src/instructions/sbcd.rs), [`nbcd.rs`](../../../crates/cpu/src/instructions/nbcd.rs)
    - `privileged.rs` $\rightarrow$ [`trapv.rs`](../../../crates/cpu/src/instructions/trapv.rs), [`rtr.rs`](../../../crates/cpu/src/instructions/rtr.rs), [`rte.rs`](../../../crates/cpu/src/instructions/rte.rs), [`stop.rs`](../../../crates/cpu/src/instructions/stop.rs), [`reset.rs`](../../../crates/cpu/src/instructions/reset.rs), [`move_usp.rs`](../../../crates/cpu/src/instructions/move_usp.rs)
  - **Justified Exceptions:**
    - `move_sr_ccr.rs`: Tightly coupled status register transfers sharing underlying privilege check and CCR/SR state latching (`MOVE to CCR`, `MOVE from SR`, `MOVE to SR`).
    - `logic_sr_ccr.rs`: Immediate status operations with identical privilege validation (`ANDI/EORI/ORI to CCR/SR`).
    - Size-based decompositions (`move_b.rs`, `move_w.rs`, `move_l.rs`) where high-cardinality addressing mode matrices make a single file unmanageable.

- **Zero Cascaded Runtime Branching in Dispatch Table (Rule 2.6):**
  - All 65,536 entries in `dispatch_table.rs` map directly to specialized static opcode handlers.
  - Methods that previously used dynamic opcode inspection inside the handler are separated into distinct single-mnemonic handlers:
    - `op_move_to_reg` vs `op_movea`: `move.rs` handles `Dn` destinations; `movea.rs` handles `An` destinations.
    - `op_bra`, `op_bsr`, and `op_bcc`: separated into `bra.rs`, `bsr.rs`, and `bcc.rs`.
    - Register and memory shifts/rotates: separated into 8 distinct pairs of handlers across `asl.rs`, `asr.rs`, `lsl.rs`, `lsr.rs`, `roxl.rs`, `roxr.rs`, `rol.rs`, and `ror.rs`.
    - System control immediate operations: `op_andi_to_ccr`, `op_andi_to_sr`, `op_ori_to_ccr`, `op_ori_to_sr`, `op_eori_to_ccr`, `op_eori_to_sr` mapped directly in `andi.rs`, `ori.rs`, and `eori.rs`.
- **Branchless 2-Phase Decomposition (`op_move_to_reg` vs `op_move_to_mem` in `move.rs` & `movea.rs`):**
  - **Phase 1: Source Effective Address Resolution & Read:**
    - Resolves source operand using compile-time constants `SRC_M` (mode) and `src_reg`.
    - Handles address error detection, extension word fetches, and pipeline sequencing.
    - Operands are staged in `state.micro.source`.
  - **Phase 2: Destination Effective Address Resolution & Write:**
    - **Register Destination (`Dn`):** Directly commits staged operand into target register. Evaluates condition codes for `MOVE` ($N = \text{MSB}$, $Z = \text{val} == 0$, $V = 0$, $C = 0$, $X$ preserved).
    - **Address Register Destination (`An` via `movea.rs`):** Sign-extends Word size and leaves CCR untouched.
    - **Memory Destination (`(An)`, `(An)+`, `-(An)`, `(d16,An)`, `(d8,An,Xn)`, `(xxx).w`, `(xxx).l`):** Resolves destination memory address, checks 16/32-bit word alignment, triggers Address Error (Vector 3) if unaligned, initiates write bus cycles, and latches prefetch according to hardware ordering (e.g. `-(An)` prefetch-before-write sequence).

### 7.11 Extended Arithmetic, Shifts, and Control Flow

- **Extended Arithmetic (`addx.rs`, `subx.rs`):**
  - **Z-Flag Retention Quirk:** The $Z$ condition code flag is cleared if the arithmetic result is non-zero, but **preserved intact** if the result is zero, enabling seamless chaining across multi-precision additions/subtractions.
  - **Register-to-Register Forms:**
    - Byte and Word: 4 clocks (2 CCKs, standard prefetch retire).
    - Long: 8 clocks (4 internal ALU clocks in Step 0 + 4 prefetch bus clocks in Step 1).
  - **Memory Predecrement `-(Ay), -(Ax)` Multi-Precision Ordering:**
    - In accordance with MC68000 hardware execution, multi-precision 32-bit arithmetic transfers operate low word first, then high word.
    - Byte and Word: 18 clocks (2 internal + 4 read Ay + 4 read Ax + 4 prefetch + 4 write Ax).
    - Long: 30 clocks (2 internal + 4 read Ay.low + 4 read Ay.high + 4 read Ax.low + 4 read Ax.high + 4 write Ax.low + 4 prefetch + 4 write Ax.high).
    - **Long Predecrement Address Error AGU Register Commitment:** For 32-bit transfers, the Address Generation Unit decrements $A_y$ by 2 for the initial low word access. If the address is unaligned, the processor triggers an Address Error immediately; $A_y$ remains decremented by 2 (never 4) in the final register state, and the pushed stack frame access address records `initial_Ay - 2`. Destination $A_x$ mirrors this behavior.
- **Control Flow Instructions (`bra.rs`, `bsr.rs`, `bcc.rs`, `jmp.rs`, `jsr.rs`, `rts.rs`, `trap.rs`, `nop.rs`):**
  - **NOP:** 4 CPU clocks (2 CCKs), standard prefetch retire.
  - **BRA & Bcc (16 conditions):**
    - Short displacement ($d_8 \neq 0$): 10 clocks if taken (2 internal + 4 target prefetch + 4 target+2 prefetch), 8 clocks if untaken (4 internal + 4 next prefetch).
    - Word displacement ($d_8 = 0$): 10 clocks if taken (2 internal + 4 target prefetch + 4 target+2 prefetch), 12 clocks if untaken (4 extension read + 4 internal + 4 next prefetch).
    - **BSR (Branch to Subroutine):** 18 clocks for both short and word forms. Pushes 32-bit return PC (2 write cycles to $A_7$) before refilling prefetch from the branch target.
  - **JMP (Jump):** Supports all 7 control addressing modes (`(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`). Cycle times: 8 clocks for `(An)`, 10 clocks for 1-extension modes, 12 clocks for `(xxx).l`, 14 clocks for indexed modes.
  - **JSR (Jump to Subroutine):** Supports all 7 control addressing modes. Pushes return PC to stack (high word to $A_7-4$, low word to $A_7-2$) and refills prefetch pipeline (16–22 CPU clocks).
  - **RTS (Return from Subroutine):** 16 clocks (8 CCKs). Reads return PC from stack ($A_7$, $A_7+2$), advances $A_7 \leftarrow A_7 + 4$, checks alignment, and refills pipeline from target address.
  - **TRAP (Trap Exception Processing):** 34 clocks (17 CCKs). Pushes return PC and SR to supervisor stack ($SSP$), switches to supervisor mode ($S=1, T=0$), fetches exception vector from `$000080 + \text{vec} \times 4$, and initiates double prefetch refill.
  - **Address Error (Vector 3) & 32-bit Target Fidelity:** Target addresses and stack values retain full 32-bit register width without artificial 24-bit truncation (`& 0x00FF_FFFF`), ensuring cycle-exact diagnostic and stack frame fidelity matching Tom Harte silicon test vectors.

### 7.12 Autovector Interrupt Processing & STOP Instruction Awakening

- **Interrupt Priority Levels (IPL 1–7):**
  - M68000 samples the 3-bit interrupt priority lines $\overline{\text{IPL0}}-\overline{\text{IPL2}}$ (driven in the emulator by `state.ipl` via `resolve_ipl()`).
  - **Mask Evaluation:** An interrupt is recognized if `ipl > interrupt_mask` (bits 8–10 of $SR$) and `ipl > 0`, or unconditionally if `ipl == 7` (Level 7 Non-Maskable Interrupt / NMI).
  - Interrupts are sampled at instruction boundaries (during `retire_current_instruction()`) and while the CPU is suspended by the `STOP` instruction (`state.stopped = true`).
- **Autovector Exception Micro-Step Pipeline (44 CPU Clocks / 22 CCKs):**
  - The Amiga 500 hardware asserts the $\overline{\text{VPA}}$ (Valid Peripheral Address) pin during interrupt acknowledge cycles ($A_{19}-A_{16} = 1111_2$), forcing the 68000 to generate an autovector exception according to the interrupt level ($24 + \text{level}$, Vectors 25 through 31 at physical addresses $\$000064$ through $\$00007C$).
  - Modeled by the cycle-exact sequence `STEPS_INTERRUPT` in [`crates/cpu/src/micro/common.rs`](../../../crates/cpu/src/micro/common.rs):
    1. **`ALU_INTERRUPT_INIT` (2 clocks):** Samples `ipl`, determines return PC (`state.pc` if waking from STOP, otherwise `state.instruction_pc`), saves old $SR$, switches to Supervisor mode ($S=1, T=0$), raises interrupt mask to `level`, calculates vector address, and clears `state.stopped`.
    2. **`ALU_IDLE_8CLK` (8 clocks):** Internal priority arbitration and exception setup latency.
    3. **`BUS_WRITE_IDLE` + `BUS_READ_IDLE` (4 clocks):** IACK CPU space cycle simulation (Address $A_1-A_3 = \text{level}, \overline{\text{VPA}}$ asserted).
    4. **Stack Pushes (12 clocks):** Pushes return PC low word to $SSP-2$, old $SR$ to $SSP-6$, and return PC high word to $SSP-4$, committing $SSP \leftarrow SSP - 6$.
    5. **Autovector Fetch (8 clocks):** Reads high word and low word of vector address from physical memory into `ea_addr`.
    6. **Prefetch Target Refill (10 clocks):** Reads first target opcode from `ea_addr` and prefetches second word from `ea_addr + 2`, completing target refill and transferring control to the Interrupt Service Routine (ISR).
- **Awakening the `STOP` Instruction:**
  - When the CPU executes `STOP #<data>`, it writes the immediate word to $SR$ (requiring Supervisor mode) and halts instruction stepping (`state.stopped = true`).
  - When a qualifying interrupt level (`ipl > mask || ipl == 7`) is asserted, `check_and_trigger_interrupt()` immediately initiates `STEPS_INTERRUPT`, clearing `state.stopped` and jumping directly to the registered autovector handler.

---

## 8. Reference Documentation & Upstream Ground Truth

- [68000 User's Manual: Section 2 (Introduction & Programmer's Model)](../Reference/68000%20User's%20Manual/02%20-%20Section%202%20-%20Introduction%20%26%20Programmer's%20Model.md): Data/address register architecture, status register bitfields, and supervisor vs. user stack pointers.
- [68000 User's Manual: Section 6 (Exception Processing & Stack Frames)](../Reference/68000%20User's%20Manual/06%20-%20Section%206%20-%20Exception%20Processing,%20Stack%20Frames%20%26%20Reset.md): Exception vectors (0–255), 7-word Address Error / Bus Error stack frame layouts, and interrupt processing.
- [68000 User's Manual: Section 8 (16-Bit Instruction Timing Tables)](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): Standard clock cycle tables, effective address calculation times, and bus read/write operation counts.
- [Instruction Prefetch on the Motorola 68000 Processor](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): Hardware prefetch queue behavior (`IRC`/`IRD`), extension word capture timing, and branch target refills.
- [Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis](../Reference/Motorola%2068000%20DIVU%20%26%20DIVS%20Cycle-Accurate%20Timing%20Analysis.md): Microcode division loop mechanics, quotient bit evaluations, and hardware execution cycle formulas.
- [M68000 Crate Source Implementation](../../../crates/cpu/src/cpu.rs): Living Rust implementation of the cycle-exact CPU core, micro-step dispatch, and instruction handlers.

