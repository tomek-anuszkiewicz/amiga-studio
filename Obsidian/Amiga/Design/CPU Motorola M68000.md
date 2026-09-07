# Motorola 68000 CPU Design Specification

- **Module Location:** `m68000/`
- **Execution Model:** Cycle-exact micro-operations mapped to Color Clock phases (**CCK1** and **CCK2**).
- **Bus Interface:** Interacts with memory strictly via [MemoryBus.md](MemoryBus.md), respecting `MemoryBusResult::Ready` vs `MemoryBusResult::Blocked`.
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md) (wrapping arithmetic, Big-Endian decoding, zero panics).
- **Test Validation:** Verified via [CPU SingleStepTests.md](CPU%20SingleStepTests.md) and skill `m68k-singlestep-test`.

---

## 1. CPU State & Register Architecture

The CPU exposes a fully queryable, read-only state snapshot for inspection, debugging, and save states:

```rust
use serde::{Deserialize, Serialize};

/// Complete register set and state for the Motorola 68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuState {
    /// Data Registers D0-D7 (32-bit each)
    pub d: [u32; 8],

    /// Address Registers A0-A6 (32-bit each)
    pub a: [u32; 7],

    /// User Stack Pointer (active A7 when Supervisor bit S = 0)
    pub usp: u32,

    /// Supervisor Stack Pointer (active A7 when Supervisor bit S = 1)
    pub ssp: u32,

    /// Program Counter (24-bit addressing on MC68000)
    pub pc: u32,

    /// Status Register (16-bit)
    /// - System Byte (Bits 8-15): Trace (T, bit 15), Supervisor (S, bit 13), Interrupt Mask (I2-I0, bits 10-8)
    /// - User Byte / CCR (Bits 0-7): Extend (X, bit 4), Negative (N, bit 3), Zero (Z, bit 2), Overflow (V, bit 1), Carry (C, bit 0)
    pub sr: u16,

    /// Internal Prefetch Queue [IRC (Capture), IRD (Decode)]
    pub prefetch: [u16; 2],

    /// Current Instruction Register (holds opcode being decoded/executed)
    pub ir: u16,

    /// Sub-cycle execution phase / step index within current instruction (CCK1/CCK2)
    pub step: u16,

    /// Sampled Interrupt Priority Level (0..7) driven from outside
    pub ipl: u8,

    /// Program Counter at the start of current instruction + 2 (used for exception stack frames)
    pub instruction_pc: u32,

    /// Execution control flags
    pub stopped: bool,
    pub halted: bool,
}

impl CpuState {
    /// Returns the currently active stack pointer (A7) based on the Supervisor flag
    #[inline]
    pub fn a7(&self) -> u32 {
        if (self.sr & 0x2000) != 0 { self.ssp } else { self.usp }
    }

    /// Sets the currently active stack pointer (A7) based on the Supervisor flag
    #[inline]
    pub fn set_a7(&mut self, val: u32) {
        if (self.sr & 0x2000) != 0 { self.ssp = val; } else { self.usp = val; }
    }
}
```

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
- **65,536-Entry Direct Dispatch Table (`[fn; 65536]`):** Every 16-bit opcode indexes directly into a precalculated array of specialized handlers.
- **Statically Inlined Parameters:** Within each specialized opcode handler, operand size (`.b`, `.w`, `.l`), addressing mode, and register indices are compile-time constants:
  - Eliminates runtime `match mode` and `if size == ...` checks.
  - The compiler generates straight-line host assembly instructions for arithmetic and CCR flag updates.
  - Modern CPU Branch Target Buffers (BTBs) predict indirect table dispatches with high efficiency, maximizing instruction cache locality and superscalar throughput.

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

---

## 3. Bus Stalling & CCK Phase Model

Memory access is mapped to Color Clock phases (**CCK1** and **CCK2**):
- $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks (S0-S7)} = 2\ \text{CCK cycles (CCK1 + CCK2)}$.

### 3.1 Read Transaction Contention
- **CCK1 (S0–S3):** The CPU asserts the target address and `_AS`.
  - If target is Chip RAM and Agnus DMA is active (`chip_ram_blocked == true`):
    - `MemoryBus` returns `MemoryBusResult::Blocked`.
    - **Action:** CPU stalls at the current micro-step. Does NOT advance `step`. Repeats CCK1 on next clock.
  - If unblocked:
    - `MemoryBus` loads data into `read_latch` and returns `MemoryBusResult::Phase1Ready`.
- **CCK2 (S4–S7):**
  - The CPU reads directly from `self.read_latch`, isolated from the external memory bus.
  - `MemoryBus` returns `MemoryBusResult::Ready(latch)`.
  - The CPU micro-step completes and advances to the next step.

### 3.2 Write Transaction Contention (Unbuffered)
- **CCK1 (S0–S3):** The CPU outputs the address and write data onto its external pins.
  - `MemoryBus` stores incoming data in temporary register (`pending_write_data`).
  - Returns `MemoryBusResult::Phase1Ready`. The CPU always proceeds to CCK2.
- **CCK2 (S4–S7):** The memory bus attempts to commit the write to RAM.
  - If target is Chip RAM and `chip_ram_blocked == true`:
    - Gary withholds `_DTACK`.
    - `MemoryBus` returns `MemoryBusResult::Blocked`.
    - **Action:** CPU stalls at CCK2, holding write pins asserted until Agnus frees the bus.
  - If unblocked:
    - Byte/word commits to memory. Returns `MemoryBusResult::Ready(0)`.

### 3.3 Micro-Step State Machine & Instruction Lifecycle

Instruction execution is driven via a micro-step state machine clocked at CCK granularity:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Micro-step completed within the current instruction
    StepCompleted,
    /// Instruction has retired (completed writeback and prefetched next opcode into IR/IRC)
    InstructionCompleted,
    /// CPU stalled due to bus wait-state (MemoryBusResult::Blocked)
    WaitState,
    /// CPU entered or is in stopped state (STOP instruction)
    Stopped,
    /// CPU entered halted state (double bus fault / fatal reset)
    Halted,
}
```

#### Execution Lifecycle Stages
1. **Decode & EA Calculation (CCK cycles):** Read effective address parameters and extension words from `IRC` if required, initiating refill prefetch reads at `PC`.
2. **Operand Fetch:** Read source operand from register or memory via 2-phase bus transactions (`read_phase1`/`read_phase2`).
3. **ALU Operation:** Compute operation using wrapping arithmetic and evaluate condition code flags (X, N, Z, V, C).
4. **Writeback:** Write destination operand (if memory destination, via `write_phase1`/`write_phase2`).
5. **Instruction Retirement:** Transfer `IRC -> IR`, fetch next stream word into `IRC`, increment `PC += 2`, and return `StepResult::InstructionCompleted`.

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

## 7. Instruction Quirks: TAS (Test And Set)

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