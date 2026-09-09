# Motorola 68000 CPU Design Specification

- **Module Location:** `m68000/`
- **Execution Model:** Cycle-exact micro-operations mapped to Color Clock phases (**CCK1** and **CCK2**).
- **Bus Interface:** Interacts with memory strictly via [MemoryBus.md](MemoryBus.md), handling `BusResult::WaitState` and executing direct 2-phase Color Clock read/write transactions (`step_bus_read_word`, `step_bus_write_word`, etc.).
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md) (wrapping arithmetic, Big-Endian decoding, zero panics).
- **Test Validation:** Verified via [CPU SingleStepTests.md](CPU%20SingleStepTests.md) and skill `m68k-singlestep-test`.

---

## 1. CPU State & Register Architecture

The CPU exposes a fully queryable, read-only state snapshot (`CpuState`) for inspection, debugging, and save states. The complete implementation resides in [`crates/m68000/src/state.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/state.rs).

### 1.1 Complete M68000 Programmer's Model & State Fields

| Register / Field | Width | Description | Hardware Behavior / Access Rules |
| :--- | :---: | :--- | :--- |
| **`d[0..=7]`** ($D_0-D_7$) | 32-bit | Data Registers | General data registers. Supports Byte, Word, and Long transfers. Low-size writes preserve unaffected high bits. Encapsulated via `d_byte`, `set_d_byte`, `d_word`, `set_d_word`, `d_long`, `set_d_long`. |
| **`a[0..=7]`** ($A_0-A_7$) | 32-bit | Address Registers | Base, pointer, and software stack registers. Byte accesses are invalid. Word writes are sign-extended to 32 bits (`set_a_word`). $A_7$ holds the currently active stack pointer ($USP$ or $SSP$). |
| **`usp`** ($USP$) | 32-bit | User Stack Pointer | Banked $A_7$ when running in User Mode ($SR.S = 0$). |
| **`ssp`** ($SSP$) | 32-bit | Supervisor Stack Pointer | Banked $A_7$ when running in Supervisor Mode ($SR.S = 1$). |
| **`pc`** ($PC$) | 32-bit | Program Counter | Points to instruction memory. 24-bit physical address space on MC68000; internally 32-bit wide. |
| **`sr`** ($SR$) | 16-bit | Status Register | High byte: System Byte (Trace, Supervisor, Interrupt Mask). Low byte: Condition Code Register (CCR). |
| **`prefetch[0..=1]`** | $2 \times 16$-bit | Instruction Prefetch Queue | Models hardware `IRC` (Capture) and `IRD` (Decode) holding staged instruction words. |
| **`ir`** ($IR$) | 16-bit | Instruction Register | Holds the opcode currently being executed. Immutable during micro-steps. |
| **`step`** | 16-bit | Sub-Cycle Phase / Step Index | Index within current micro-step sequence across Color Clock phases (CCK1/CCK2). |
| **`ipl`** ($IPL$) | 8-bit | Interrupt Priority Level | Sampled interrupt priority lines (0..7) driven by Paula/arbitration. |
| **`instruction_pc`** | 32-bit | Instruction Base PC | $PC + 2$ at instruction start; recorded for exception stack frames. |
| **`stopped`** | bool | STOP Instruction Latch | Processor halted awaiting interrupt higher than current interrupt mask. |
| **`halted`** | bool | Double Bus Fault Latch | Processor halted due to catastrophic hardware failure / reset. |
| **`micro`** | `CpuMicroState` | Execution Micro-State | Tracks Color Clock phase, latched bus words, Data Output Buffer, and wait cycles. |

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

#### Mechanical Sympathy: Branchless Condition Code Setters
To avoid host branch mispredictions in hot execution paths, the core employs direct branchless bitwise CCR updates (`set_ccr_xnzvc`, `set_ccr_nzvc`, `set_ccr_nz_clear_vc`, `set_ccr_nzc_clear_v`, `set_ccr_z_only`, `set_ccr_raw`). All CCR setter methods are marked `#[inline(always)]` in [`crates/m68000/src/state.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/state.rs).

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
   - Every CCK cycle where `MemoryBus` returns `MemoryBusResult::Blocked` adds exactly **1 CCK (2 CPU clocks)** to the instruction's total duration.

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
- **65,536-Entry Direct Descriptor Table (`OPCODE_DESCRIPTOR_TABLE: [OpcodeDescriptor; 65536]`):** Every 16-bit opcode indexes directly into a precalculated array of static opcode descriptors (`crates/m68000/src/micro/dispatch_table.rs`). Each entry contains a reference to an immutable slice of specialized atomic `MicroStep`s, along with pre-decoded register indices (`reg_src`, `reg_dst`).
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
> For emulator codebase function naming conventions, consult the operational rule in [opcode-naming.md](../../.agents/rules/opcode-naming.md).

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
    - **Action:** CPU stalls at CCK1 (accumulating wait state, `is_wait_state() == true`). Does NOT advance micro-step. Repeats CCK1 on next clock.
  - If `BusResult::Ready(data)`:
    - Data is stored directly into the target register (`source`, `destination`, `prefetch[0]`, or `irc`); CPU advances `phase` to `CckPhase::Cck2`.
- **CCK2 (S4–S7):**
  - Physical bus is already idle/released for custom chip DMA. The transaction is recorded directly from the target register, completing the bus cycle and advancing to the next step (`phase = CckPhase::Cck1`).

### 3.2 Write Transaction Contention
- **CCK1 (S0–S3):** The CPU outputs the address internally and asserts `_AS`.
  - CPU proceeds to CCK2 (`phase = CckPhase::Cck2`).
- **CCK2 (S4–S7):** The memory bus attempts committing the write via `bus.write_word(addr, val)` or `bus.write_byte(addr, val)` reading directly from `state.micro.destination`:
  - If `BusResult::WaitState` (Gary withholds `_DTACK` due to Chip RAM DMA contention):
    - **Action:** CPU stalls at CCK2 (`is_wait_state() == true`), holding write pins asserted until Agnus frees the bus.
  - If `BusResult::Ready(())`:
    - Byte/word commits to memory, transaction is recorded, and `phase` resets to `CckPhase::Cck1`.

### 3.3 Micro-Step State Machine & Instruction Lifecycle

> [!NOTE]
> For the complete, dedicated microcode architectural blueprint, specialized atomic bus primitives, strobe semantics, and cycle traces across all 10 instruction classes, see [[CPU Micro-Step State Machine.md]].

Instruction execution is driven via a cycle-exact micro-step state machine clocked at Color Clock (CCK) granularity (2 CPU clocks per CCK, 4 clocks per bus cycle). The execution engine and micro-state tracking are implemented in [`crates/m68000/src/micro/engine.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/engine.rs) and [`crates/m68000/src/core.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/core.rs).

- **Execution Micro-State (`CpuMicroState`):**
  - `phase`: Color Clock sub-phase (`CckPhase::Cck1` or `CckPhase::Cck2`).
  - `source`: Explicit 32-bit storage for ALU source operand (incoming bus data is stored directly here on CCK1).
  - `destination`: Explicit 32-bit storage for ALU destination operand and write-back data (bus write cycles read directly from here).
  - `irc`: Instruction Register Capture — physical 68000 prefetch latch holding prefetched opcodes before retirement into IR.
  - `ea_addr`: Resolved effective memory address for operands or branch/jump targets.
  - `ea_high`: High word of 32-bit absolute addresses (`(xxx).L`) or high address for split accesses.
  - `movem_mask`: 16-bit register transfer mask for `MOVEM`.
  - `movem_state`: Internal packing state (bit index and sub-word tracker) for `MOVEM`.
  - `internal_clocks`: Remaining internal CPU clocks for multi-cycle arithmetic/shift operations.
  - `micro_step`: Index of the currently executing micro-operation within the active opcode sequence.
  - `current_cycle_wait_cycles`: Wait cycles accumulated while stalled by Agnus DMA contention.
  - `target_refill`: Indicates whether instruction retirement must perform a branch/jump target refill.
  - `prefetch_retired`: Indicates whether prefetch pipeline has already retired into IR during microcode execution.

- **Unified Control Model (Zero `StepResult` Overhead):**
  - **`StepFn = fn(&mut Cpu, &mut MemoryBus) -> BusResult<()>`**: Micro-step handlers only report bus readiness (`BusResult::Ready(())` or `BusResult::WaitState`).
  - **`step_cck(&mut self, bus: &mut MemoryBus) -> bool`**: Executes a single CCK color clock cycle (~280 ns) and returns `true` when the instruction completes/retires, `false` otherwise.
  - **`step_instruction(&mut self, bus: &mut MemoryBus) -> u32`**: Steps through an entire instruction to retirement, returning the exact CPU clock cycles consumed.
  - **`is_wait_state(&self) -> bool`**: Returns whether the CPU is currently stalled by DMA contention.
  - **State flags**: Halted and Stopped states are queried directly on `cpu.state.halted` and `cpu.state.stopped`.

#### Specialized Direct Micro-Step Execution Handlers (`StepFn`)
To eliminate nested dynamic runtime size checks and dynamic branching (`match`) in the hot execution loop, micro-step operations are specialized directly into atomic function pointers (`StepFn = fn(&mut Cpu, &mut MemoryBus) -> BusResult<()>`):
- **Operand Reads:** `Cpu::step_bus_read_byte`, `Cpu::step_bus_read_word`, `Cpu::step_bus_read_long_high`, `Cpu::step_bus_read_long_low`.
- **Operand Writes:** `Cpu::step_bus_write_byte` (preserves unaddressed byte in 16-bit cell), `Cpu::step_bus_write_word`, `Cpu::step_bus_write_long_high`, `Cpu::step_bus_write_long_low`.
- **Stack & Control Flow:** `Cpu::step_bus_pop_stack`, `Cpu::step_bus_push_stack_high`, `Cpu::step_bus_push_stack_low`, `Cpu::step_bus_read_target_opcode`, `Cpu::step_prefetch_target_and_retire`.
- **Multi-Register Block Transfers:** `crate::instructions::movem::execute_movem_transfer`.

#### Pipeline & Dispatch Table Invariants
1. **Immutable `ir` During Micro-Steps:** The 65,536-entry static descriptor table (`OPCODE_DESCRIPTOR_TABLE`) is indexed upon instruction prefetch to cache `current_steps: &'static [MicroStep]`. Intermediate multi-step operations (e.g. `JSR` or taken `Bcc`) must never overwrite `cpu.state.ir` before final retirement. Target opcodes are staged in `scratch_prefetch` or `TargetRefill { target, new_ir }`, and committed to `cpu.state.ir` only upon retirement.
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

---

## 6. Reset Procedure (Cold and Warm)

On both **Cold** and **Warm** reset, the CPU execution flow begins at vector `$000000`:

1. **Overlay Active:** Low-memory overlay (`_OVL`) routes `$000000-$07FFFF` to Kickstart ROM.
2. **Status Register:** Set to `$2700` ($S=1, T=0, I=7$).
3. **Fetch Initial SSP:** Read 32-bit value from `$000000` into `ssp` (active `A7`).
4. **Fetch Initial PC:** Read 32-bit value from `$000004` into `pc`.
5. **Fill Prefetch:** Read word at `pc` into `ir`, increment `pc += 2`; read word at `pc` into `irc`, increment `pc += 2`.
6. **Execution:** Begin execution at `pc` in Kickstart ROM.
   - Kickstart inspects RAM contents for magic resident checksums to determine whether to perform a warm reboot or cold boot.

---

## 7. M68000 Instruction & Hardware Silicon Quirks

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
  - **Prefetch Bus Cycle First:** On real silicon (verified by Tom Harte test vectors), the CPU initiates the next instruction opcode prefetch during micro-step 0 (`cpu.initiate_prefetch()`, 4 clocks / 2 CCKs).
  - **Internal Idle Clocks:** The execution unit schedules internal processing clocks based on operand size and shift count:
    - **Byte / Word:** $idle\_clocks = 2 + 2 \times count$ (Total execution time: $6 + 2n$ clocks).
    - **Long (32-bit):** $idle\_clocks = 4 + 2 \times count$ (Total execution time: $8 + 2n$ clocks).
  - **Retirement:** Micro-step 2 marks standard sequential prefetch retirement (`mark_standard_prefetch_retire()`), advancing the program counter.

- **Memory Shift Micro-Step Pipeline (Class 0 Read-Modify-Write):**
  - Memory shifts are strictly **Word size** and shift by **1 bit** only (`count = 1`).
  - Follows the standard 3-step RMW sub-cycle sequence:
    - **Step 1 (Read):** Read 16-bit word from effective address memory via linear EA resolution.
    - **Step 2 (Prefetch & Compute):** Perform 1-bit shift, evaluate condition codes, initiate next opcode prefetch, and latch modified word in internal scratch.
    - **Step 3 (Writeback):** Write modified 16-bit word back to the target memory address via bus write cycle.
  - Concludes with pipeline refill from scratch prefetch latch. Total duration: 12–16 clocks depending on addressing mode (e.g. `(An)` is 12 clocks, `-(An)` is 14 clocks).

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

### 7.6 Address Error (Vector 3) Program Space Selection & Silicon Divergences

- **Function Code Selection on Address Error:**
  - If the unaligned word/long access was triggered by a **PC-relative addressing mode** (`(d16, PC)` or `(d8, PC, Xn)`), the CPU asserts **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode).
  - For standard data memory operands, the CPU asserts **Data Space** ($FC = 1$ in User mode, $FC = 5$ in Supervisor mode).
- **Postincrement `(An)+` AGU Read vs. Write Silicon Nuance:**
  - **READ from `(An)+`:** The Address Generation Unit (AGU) increments $A_n$ as the read bus cycle begins; if the read address is unaligned, $A_n$ has already been incremented on real silicon (Tom Harte test vectors), whereas MAME's microcode interpreter aborts without updating $A_n$.
  - **WRITE to `(An)+`:** The processor checks alignment before postincrementing; if unaligned, $A_n$ is **never** incremented (both MAME and Tom Harte expect $A_n$ unincremented).

### 7.7 ASR (Arithmetic Shift Right) Count > Width Silicon Exhaustion

- When the shift count exceeds the operand width ($count \ge 8$ for Byte, $\ge 16$ for Word, $\ge 32$ for Long):
  - **Real MC68000 Silicon (verified by Tom Harte test suite):** The shift register exhausts its internal latch pipeline, forcing both **$C = 0$** and **$X = 0$**, even when shifting negative numbers filled with replicated sign bits (`1`).
  - **MAME Simulator Divergence:** MAME's C++ microcode simulator continues to shift replicated sign bits into $X$ and $C$, incorrectly setting $X = 1$ and $C = 1$ on negative operands when $count > width$.
  - *Emulator Resolution:* The core implements real silicon behavior ($C=0, X=0$), and the test harness accommodates MAME's divergence.

### 7.8 MOVE to Predecrement `-(An)` Prefetch Inversion & Bus Ordering

- On real MC68000 hardware, destination write ordering and instruction prefetch exhibit distinct behaviors across operand sizes:
  - **Byte and Word (`MOVE.b`, `MOVE.w ..., -(An)`):**
    - The CPU prefetches the next instruction word **before** initiating the destination write bus cycle.
    - If the destination write triggers an Address Error:
      - The Instruction Register ($IR$) pushed into the 7-word exception stack frame is the **prefetched instruction word**, not the current `MOVE` opcode.
      - $A_n$ is decremented by 2 by the AGU and remains decremented in the final register state.
  - **Long (`MOVE.l ..., -(An)`):**
    - The write bus cycles occur before instruction prefetch completion; the opcode itself is pushed as the faulting $IR$.
    - **Bus Write Ordering:** The 32-bit transfer is executed decrementing low word first to $A_n - 2$, then high word to $A_n - 4$.
    - If $A_n$ is odd, the initial write cycle faults immediately at $A_n - 2$.
    - On real silicon (Tom Harte), $A_n$ remains decremented by 2 ($A_n - 2$). MAME aborts without committing the internal AGU bus latch to $A_n$.

### 7.9 MOVE.l 32-Bit Memory-to-Memory CCR Evaluation vs MAME Simulator

- In 32-bit `MOVE.l <ea>, (An)` transfers:
  - **Real MC68000 Silicon:** Condition codes reflect the full 32-bit transfer ($N = \text{bit } 31$, $Z = \text{value } == 0$).
  - **MAME Microcode Interpreter Quirk:** MAME's microcode simulator (`mmrl1`) evaluates the lower 16-bit word (`m_dbin`) and calls `sr_nzvc()` during the *first* write bus cycle, and only updates $N$ and $Z$ with the upper 16-bit word during the *second* write bus cycle. If the first write bus cycle faults with an Address Error, MAME leaves CCR reflecting the lower 16-bit word rather than the full 32-bit value.
  - *Emulator Resolution:* The core strictly implements the full 32-bit condition code evaluation matching real silicon.

### 7.10 Branch & Control Flow Odd Target Address Error (FC 2 / 6)

- When `BRA`, `Bcc`, `JMP`, or `JSR` evaluates a target address with an odd destination (`target & 1 != 0`):
  - The MC68000 halts instruction execution and immediately triggers an **Address Error exception (Vector 3)**.
  - Because the instruction fetch pipeline caused the fault, the CPU asserts **Program Space** ($FC = 2$ in User mode, $FC = 6$ in Supervisor mode) in the exception status word.
  - The pushed program counter in the stack frame points to the instruction boundary or target address.

### 7.11 Post-Increment `(An)+` Address Error AGU Register Commitment

- When resolving Post-Increment addressing modes (`(An)+`, including `CMPM (Ay)+, (Ax)+`):
  - **Silicon Reality (Tom Harte SingleStepTests):** The M68000 Address Generation Unit (AGU) computes the operand address from $A_n$ and simultaneously advances $A_n \leftarrow A_n + \text{increment}$ (2 for word/byte-A7, 4 for long).
  - If the computed base address is odd (`addr & 1 != 0`) for word or long accesses, the Address Error exception triggers on the bus read/write cycle.
  - Crucially, $A_n$ **remains updated with the incremented value** in the final register state.
  - *Emulator Resolution:* In all linear EA resolvers (`ea.rs`) and specialized instructions (`cmpm.rs`), the address register update occurs prior to checking unaligned address error traps.

### 7.12 Class 0 Read-Modify-Write (RMW) & Bit Manipulation Silicon Timings

- **Class 0 RMW Memory Writeback Sequence (`AND`, `OR`, `EOR`, `NOT` to `<ea>`):**
  - Memory-destination logical operations follow the Class 0 Read-Modify-Write sub-cycle pipeline in `and.rs`, `or.rs`, `eor.rs`, and `not.rs`:
    - **Step 1 (Read):** Read operand from effective address memory.
    - **Step 2 (Prefetch & Compute):** Perform bitwise operation, compute condition codes ($N, Z, V=0, C=0$), initiate prefetch of next opcode, and store result in internal scratch register.
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

### 7.13 Modular Per-Mnemonic Instruction Architecture & Single-Mnemonic Dispatch

The entire M68000 instruction set is organized into dedicated, single-responsibility files by instruction mnemonic (e.g. `add.rs`, `move.rs`, `movea.rs`, `bra.rs`, `bsr.rs`, `bcc.rs`, `asl.rs`, `asr.rs`, etc.):

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
    - Operands are staged in `cpu.scratch`.
  - **Phase 2: Destination Effective Address Resolution & Write:**
    - **Register Destination (`Dn`):** Directly commits staged operand into target register. Evaluates condition codes for `MOVE` ($N = \text{MSB}$, $Z = \text{val} == 0$, $V = 0$, $C = 0$, $X$ preserved).
    - **Address Register Destination (`An` via `movea.rs`):** Sign-extends Word size and leaves CCR untouched.
    - **Memory Destination (`(An)`, `(An)+`, `-(An)`, `(d16,An)`, `(d8,An,Xn)`, `(xxx).w`, `(xxx).l`):** Resolves destination memory address, checks 16/32-bit word alignment, triggers Address Error (Vector 3) if unaligned, initiates write bus cycles, and latches prefetch according to hardware ordering (e.g. `-(An)` prefetch-before-write sequence).

### 7.14 Extended Arithmetic, Shifts, and Control Flow

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
