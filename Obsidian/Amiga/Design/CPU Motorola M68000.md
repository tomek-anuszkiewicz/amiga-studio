
- Keep code in directory `M68000`
- Code should be cycle-exact
- CPU state should be read-only (queryable)
- Implement all registers, including internal ones (prefetch queue, instruction stage)
- Provide methods to get and set whole CPU state at once for save/load state
- Build instruction execution state machine (e.g. big switch / table for instructions)
- Memory bus operations and instruction execution stages are modeled using Color Clock phases: **CCK1** and **CCK2** (replacing traditional S-states, where 1 bus access = 4 CPU clocks = 2 CCK cycles: CCK1 and CCK2):
	- When reading/writing during a CCK phase, check memory bus availability via `MemoryBusResult`:
		- If bus is ready (`MemoryBusResult::Ready`), progress to the next state/phase.
		- If bus is blocked (`MemoryBusResult::Blocked` / `Wait`), remain at the same state/phase (insert wait state).
- Execute prefetch in defined CCK cycles as per the MC68000 manual
- For each instruction, calculate exact cycle timings based on CCK phases
- Only progress when CPU cycle requirements are satisfied and the bus is unblocked
	- Note that because CPU may be blocked by Chip RAM / DMA, it can wait additional CCKs
- Use `MemoryBus` to access memory
- **CPU State & Register Architecture**:
  - Implement full register state in a dedicated struct (`CpuState` / `Cpu`):
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
- **Exception Vector Table & Exception Processing**:
  - Implement full MC68000 Exception Vector Table (`$000000-$0003FF`, 256 32-bit vector addresses in memory)
  - Vectors include Reset SP/PC (0-1), Address Error (3), Illegal Instruction (4), Zero Divide (5), Privilege Violation (8), Trace (9), TRAP #0-#15 (32-47), and Autovector Interrupts Level 1-7 (Vectors 25-31 at `$000064-$00007C`)
- **Interrupt Handling**:
  - CPU exposes an interface to sample external interrupt lines (e.g. `set_ipl(level: u8)`)
  - When the sampled interrupt level is greater than the current CPU interrupt mask in the Status Register (`SR` bits 8-10, `I0-I2`) — or on level 7 (NMI) — CPU processes an autovector interrupt exception at instruction boundary
- **Error Handling**:
  - **Address Error (Vector 3, `$00000C`)**: **Must be handled**. Triggered internally by the CPU whenever a 16-bit word or 32-bit long-word access is attempted on an odd address (bit 0 set). Pushes the 68000 Address Error stack frame.
  - **Bus Error (`_BERR`, Vector 2, `$000008`)**: **Omitted** in current hardware emulation. Standard Amiga 500 hardware does not assert `_BERR` (unmapped addresses simply float and return `$FF` / `$FFFF`).
- **Instruction Quirks & Hardware Behavior**:
  - **TAS (Test And Set)**:
    - Register operand (`TAS Dn`): Always operates normally (tests value, sets $N/Z$, clears $V/C$, sets bit 7).
    - Memory operand (`TAS <ea>`): Performs an atomic Read-Modify-Write cycle. On the Amiga:
      - **Chip RAM & Slow RAM**: Write phase is ignored by hardware (CCR updated, memory unmodified).
      - **Fast RAM**: Full Read-Modify-Write succeeds (CCR updated, bit 7 set in memory).
- Method to progress one cycle / CCK forward
- **Verification**:
  - Validated against [`CPU SingleStepTests.md`](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/CPU%20SingleStepTests.md) using test vectors from `ref_src/SingleStepTests-m68000/v1/`
- Initial milestone: Implement and verify `NOP` instruction
### Step-by-Step Reset Procedure

When triggering a reset on the Amiga 500, execute the following actions in order:

#### 1. Configure the Memory Bus (Kickstart Low-Memory Mapping)

- Set the low-memory overlay active so that accesses to the vector address range (`$000000–$000007`) are routed directly to the Kickstart ROM rather than physical Chip RAM.
    
- Clear any DMA wait state or bus-lock flags.
    

#### 2. Initialize Internal Status & Control Flags

- **Status Register (SR):** Set to `$2700`.
    
    - Sets the **Supervisor bit ($S = 1$)**, granting full supervisor privileges.
        
    - Clears the **Trace mode bit ($T = 0$)**, disabling single-step tracing.
        
    - Sets the **Interrupt Priority Mask to Level 7 ($I2..I0 = 111$)**, masking all standard maskable interrupts (only Level 7 / NMI can interrupt, if present).
        
    - Clears all condition code register (CCR) flags ($X, N, Z, V, C = 0$).
        
- **Stop / Halt State:** Ensure the CPU is taken out of any `STOP` or halted condition.
    

#### 3. Fetch the Initial Supervisor Stack Pointer (SSP / A7)

- Read the 16-bit word at address `$000000` (High Word).
    
- Read the 16-bit word at address `$000002` (Low Word).
    
- Combine them into a 32-bit value and write it directly to the active Supervisor Stack Pointer (`SSP` / `A7`).
    

#### 4. Fetch the Initial Program Counter (PC)

- Read the 16-bit word at address `$000004` (High Word).
    
- Read the 16-bit word at address `$000006` (Low Word).
    
- Combine them into a 32-bit value and set it as the starting `PC`.
    

#### 5. Prime the Instruction Prefetch Pipeline

Before instruction execution can begin, the two-stage prefetch queue must be filled from the initial target address:

- **Instruction Register (IR):** Read the 16-bit word from the current `PC` address. Store this word into `IR` (this represents the first opcode to be decoded).
    
- **Increment PC:** Add `2` to the `PC`.
    
- **Instruction Prefetch Buffer (IRC):** Read the next 16-bit word from the updated `PC` address. Store this word into `IRC` (ready as the next immediate word or subsequent opcode).
    
- **Increment PC:** Add `2` to the `PC` once more.
    

#### 6. Transition to Execution

- At this point, `IR` holds opcode 1, `IRC` holds word 2, and `PC` points to $Initial\_PC + 4$.
    
- Hand control over to the main instruction execution loop.