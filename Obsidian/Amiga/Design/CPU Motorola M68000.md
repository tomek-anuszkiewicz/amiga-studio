# Motorola 68000 CPU Design Specification

- **Module Location:** `m68000/`
- **Execution Model:** Cycle-exact micro-operations mapped to Color Clock phases (**CCK1** and **CCK2**).
- **Bus Interface:** Interacts with memory strictly via [MemoryBus.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/MemoryBus.md), respecting `MemoryBusResult::Ready` vs `MemoryBusResult::Blocked`.
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](file:///d:/Programowanie/Amiga/AGENTS.md) (wrapping arithmetic, Big-Endian decoding, zero panics).
- **Test Validation:** Verified via [CPU SingleStepTests.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/CPU%20SingleStepTests.md) and skill `m68k-singlestep-test`.

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