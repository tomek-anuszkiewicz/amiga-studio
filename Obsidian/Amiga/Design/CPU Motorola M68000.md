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

    /// Address Registers A0-A7 (32-bit each). A7 holds the currently active stack pointer.
    pub a: [u32; 8],

    /// User Stack Pointer (stored A7 when Supervisor bit S = 0)
    pub usp: u32,

    /// Supervisor Stack Pointer (stored A7 when Supervisor bit S = 1)
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

    /// Sub-cycle execution micro-state (Color Clock phase and in-flight bus cycle)
    pub micro: CpuMicroState,
}

impl CpuState {
    /// Read address register by index (0-7 returns A0-A7) branchlessly
    #[inline(always)]
    pub fn read_a(&self, idx: usize) -> u32 {
        self.a[idx]
    }

    /// Write address register by index (0-7 writes A0-A7) branchlessly
    #[inline(always)]
    pub fn write_a(&mut self, idx: usize, val: u32) {
        self.a[idx] = val;
    }

    /// Sets supervisor mode, swapping active A7 with stored USP/SSP if privilege changes
    #[inline]
    pub fn set_supervisor(&mut self, supervisor: bool) {
        let is_super = (self.sr & 0x2000) != 0;
        if is_super == supervisor { return; }
        if supervisor {
            self.sr |= 0x2000;
            self.usp = self.a[7];
            self.a[7] = self.ssp;
        } else {
            self.sr &= !0x2000;
            self.ssp = self.a[7];
            self.a[7] = self.usp;
        }
    }

    /// Flushes the active A7 into ssp (if supervisor) or usp (if user)
    #[inline]
    pub fn sync_stack_pointers(&mut self) {
        if (self.sr & 0x2000) != 0 {
            self.ssp = self.a[7];
        } else {
            self.usp = self.a[7];
        }
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

Instruction execution is driven via a micro-step state machine clocked at Color Clock (CCK) granularity (2 CPU clocks per CCK, 4 clocks per bus cycle):

```rust
use memory_bus::{BusCycle, CckPhase};
use serde::{Deserialize, Serialize};

/// Instruction retirement and pipeline refill mode upon concluding in-flight micro-operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MicroRetireMode {
    /// Instruction is actively progressing through micro-steps
    None,
    /// Standard sequential prefetch: ir = prefetch[0], prefetch[0] = last_read, pc += 2
    StandardPrefetch,
    /// RMW / Stack push: ir = prefetch[0], prefetch[0] = scratch_prefetch, pc += 2
    ScratchPrefetch,
    /// Taken branch / jump target refill: ir = new_ir, prefetch[0] = last_read, pc = target + 4
    TargetRefill { target: u32, new_ir: u16 },
}

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Current Color Clock phase (CCK1 or CCK2)
    pub phase: CckPhase,
    /// In-flight structured bus cycle (if awaiting memory response or holding wait states)
    pub active_bus_cycle: Option<BusCycle>,
    /// Last 16-bit word received from completed bus read cycle
    pub last_read: u16,
    /// Intermediate latched prefetch word (e.g. for Class 0 RMW where prefetch precedes write)
    pub scratch_prefetch: u16,
    /// Internal execution CPU clocks remaining (non-bus micro-operations)
    pub internal_clocks: u16,
    /// Step index within the current instruction's micro-operation sequence
    pub micro_step: u16,
    /// Intermediate temporary registers for multi-step micro-operations
    pub scratch: [u32; 2],
    /// Pipeline retirement mode upon concluding the current in-flight cycle
    pub retire_mode: MicroRetireMode,
    /// Optional transaction log for cycle-exact verification (disabled by default)
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Wait cycles accumulated during the currently active bus cycle
    pub current_cycle_wait_cycles: u32,
}

/// A recorded bus or internal transaction captured for cycle-exact verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedTransaction {
    Bus {
        is_read: bool,
        is_tas: bool,
        duration: u32,
        fc: u8,
        addr: u32,
        size: BusAccessSize,
        data: u16,
        uds: bool,
        lds: bool,
    },
    Internal {
        duration: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Micro-step completed within the current instruction
    StepCompleted,
    /// Instruction has retired (completed writeback and prefetched next opcode into IR/prefetch[0])
    InstructionCompleted,
    /// CPU stalled due to bus wait-state (MemoryBusResult::Blocked)
    WaitState,
    /// CPU entered or is in stopped state (STOP instruction)
    Stopped,
    /// CPU entered halted state (double bus fault / fatal reset)
    Halted,
}
```

#### The 6 Representative Micro-Step Archetypes (Verified Reference Model)

| Archetype | Sample Instruction | Total CPU Clocks | Color Clocks (CCKs) | Sub-Cycle Micro-Operation Sequence |
| :--- | :--- | :---: | :---: | :--- |
| **1. Internal Register ALU** | `NOP`, `MOVE.w Dx, Dy` | **4** | **2** | **Step 0:** ALU execution / register copy + initiate opcode prefetch at `PC`. Retires via `StandardPrefetch`. |
| **2. Memory Read** | `MOVE.w (Ax), Dy` | **8** | **4** | **Step 0:** Read data word from `(Ax)` (CCK1/CCK2).<br>**Step 1:** Latch `last_read` into `Dy`, update CCR, initiate opcode prefetch at `PC` (CCK1/CCK2). Retires via `StandardPrefetch`. |
| **3. Memory Write (Class 1)** | `MOVE.w Dx, (Ay)` | **8** | **4** | **Step 0:** Update CCR, write `Dx` to `(Ay)` (CCK1/CCK2).<br>**Step 1:** Initiate opcode prefetch at `PC` (CCK1/CCK2). Retires via `StandardPrefetch`. |
| **4. Read-Modify-Write (Class 0)** | `ADD.w Dx, (Ay)` | **12** | **6** | **Step 0:** Read destination word from `(Ay)` (CCK1/CCK2).<br>**Step 1:** Compute ALU sum, update CCR, initiate opcode prefetch at `PC` (CCK1/CCK2).<br>**Step 2:** Store prefetched opcode in `scratch_prefetch`, initiate write of ALU result to `(Ay)` (CCK1/CCK2). Retires via `ScratchPrefetch`. |
| **5. Conditional Branching** | `Bcc.s` / `BRA.s` | **8** (untaken)<br>**10** (taken) | **4** (untaken)<br>**5** (taken) | **Untaken:** 4 internal clocks + initiate prefetch at `PC` (4 clocks). Retires via `StandardPrefetch`.<br>**Taken:** 2 internal clocks + target opcode prefetch at `target` (4 clocks) + next word prefetch at `target + 2` (4 clocks). Retires via `TargetRefill`. |
| **6. Stack Push & Subroutine Call** | `PEA (An)`<br>`JSR (An)` | **12** (`PEA`)<br>**16** (`JSR`) | **6** (`PEA`)<br>**8** (`JSR`) | **`PEA (An)`:** Prefetch next word into `scratch_prefetch` (4 clocks) $\to$ Write address high word to `-(SP)` (4 clocks) $\to$ Write address low word to `SP + 2` (4 clocks). Retires via `ScratchPrefetch`.<br>**`JSR (An)`:** Prefetch target opcode into `scratch_prefetch` (4 clocks) $\to$ Write return PC high word to `-(SP)` (4 clocks) $\to$ Write return PC low word to `SP + 2` (4 clocks) $\to$ Prefetch target+2 word (4 clocks). Retires via `TargetRefill`. |

#### Pipeline & Dispatch Table Invariants
1. **Immutable `ir` During Micro-Steps:** The 65,536-entry static dispatch table (`DISPATCH_TABLE`) is indexed directly by `cpu.state.ir`. Intermediate multi-step operations (e.g. `JSR` or taken `Bcc`) must never overwrite `cpu.state.ir` before final retirement. Target opcodes must be staged in `scratch_prefetch` or `TargetRefill { target, new_ir }`, and committed to `cpu.state.ir` only upon retirement.
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
