# Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine

- **Parent Specification:** [[CPU Motorola M68000.md]]
- **Module Location:** `crates/m68000/`
- **Execution Model:** Cycle-exact micro-operations mapped to Color Clock phases (**CCK1** and **CCK2**).
- **Bus Interface:** Interacts with memory strictly via [[MemoryBus.md]], handling `BusResult::WaitState` and executing direct 2-phase CCK read/write transactions (`step_read_word_at`, `step_write_word_at`).
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md) (zero custom macros, zero const-generic handlers, wrapping arithmetic, Big-Endian decoding, zero panics).
- **Test Validation:** Verified via [[CPU SingleStepTests.md]] and skill `m68k-singlestep-test`.

---

## 1. Executive Summary & Design Philosophy

This document defines the architectural blueprint for the cycle-exact Motorola 68000 CPU emulator core based on the **Micro-Step State Machine Engine**.

The architecture mirrors the physical two-level microcode design of the Motorola 68000 silicon (the Stritter & Tredennick microcode patent):
1. **Instruction Sequencer (Macro-Level / Microrom)**: The 16-bit opcode word (`IR`) selects a static, compile-time blueprint of atomic micro-steps.
2. **Execution Unit & Bus Controller (Micro-Level / Nanorom)**: Drives the physical bus lines (`_AS`, `_UDS`, `_LDS`), Color Clock phases (**CCK1** and **CCK2**), dynamic ALU calculation delays, memory write decomposition, two-word pipeline refills, and Agnus DMA wait-state contention.

### Core Architectural Axioms
1. **The 65,536 Static Universe**:
   Because immediate values, displacements, and 32-bit addresses are fetched dynamically from memory during execution, **the micro-step sequence is a pure, immutable function of the 16-bit opcode word (`IR`)**. Exactly 65,536 static entries cover 100% of the instruction set.
2. **Zero Runtime Heap Allocation**:
   The entire micro-step table is static `const` data embedded in the host binary (`.rodata`). It requires **0 bytes of dynamic heap allocation** (`Vec`, `Box`, `malloc`) during runtime.
3. **Specialized Atomic Micro-Step Handlers (Zero-Branch Direct Dispatch)**:
   Transfer size (Byte vs. Word vs. 32-bit Long decomposition) is **specialized directly into atomic `StepFn` function pointers** (`Cpu::step_bus_read_byte`, `Cpu::step_bus_read_word`, `Cpu::step_bus_write_byte`, `Cpu::step_bus_write_word`, `Cpu::step_bus_write_long_high`, `Cpu::step_bus_write_long_low`). This eliminates nested `match size` branches and dynamic `match step.action` switches in the hot CCK execution loop, honoring host CPU mechanical sympathy.
4. **Pre-Allocated CPU-Level Prefetch Array**:
   `prefetch: [u16; 2]` and `ir: u16` are fixed fields inside `CpuState` (modeling hardware registers `IRC`, `IR`, and `IRD`). Zero dynamic queues.
5. **Parametric, Bus-Free ALU Function Pointers (`AluFn`)**:
   ALU steps do **not** take `MemoryBus`. By the time the ALU executes, all operands have already arrived in `CpuState` (`prefetch[0]`, `last_read`, or `d[]/a[]`). ALU functions take `(&mut CpuState, reg_src: u8, reg_dst: u8)`. This parameterization collapses 8–64 repetitive opcode functions into **one shared, elegant function per operation**.
6. **Zero-Cycle ALU Micro-Ops (Instantaneous Internal Ops)**:
   ALU calculations, flag updates, and Effective Address arithmetic consume **0 CCK cycles**. They execute instantly and immediately fall through to the subsequent bus step within the same host tick. If the subsequent bus step stalls due to Agnus DMA, **the ALU step is never repeated**, eliminating re-entrancy bugs and branch checks.
7. **Data Output Buffer (`write_buffer: u32`) & Exact Write Strobe Preservation**:
   Results destined for memory are held in `state.micro.write_buffer: u32` (hardware `DOB`). Memory writes strictly respect 68000 bus widths:
   - **`BusWriteByte`**: Drives $\overline{\text{UDS}}$ (even address, high byte) or $\overline{\text{LDS}}$ (odd address, low byte). The unaddressed byte in the 16-bit memory cell is **strictly preserved**.
   - **`BusWriteWord`**: Drives both $\overline{\text{UDS}}$ and $\overline{\text{LDS}}$ simultaneously on even address boundaries.
   - **`BusWriteLongHigh` & `BusWriteLongLow`**: Decomposed into two sequential 16-bit Word bus write cycles (High Word followed by Low Word).
8. **Two-Word Pipeline Refill for Control Flow (`JMP`, `JSR`, `RTS`, `Bcc`)**:
   When program execution flow changes, the prefetch stream is flushed and refilled from the target address via two sequential program-space read bus cycles (`Cpu::step_bus_read_target_opcode` and `Cpu::step_prefetch_target_and_retire`).
9. **Decoupled Internal Delay Countdown**:
   Variable-cycle operations (`DIVU`, `DIVS`, `MULU`, `MULS`, multi-bit shifts) calculate their math instantly, compute the hardware cycle penalty, and set `cpu.state.micro.clocks_remaining`. The host ticks down this counter by 2 clocks per CCK, leaving the external bus completely idle for Amiga custom chips (Copper, Blitter, Denise).
10. **Automatic Self-Modifying Code (SMC) Immunity**:
    Because opcodes directly index the static 65,536 table upon prefetch, and operands are read live from memory, **the engine requires zero bus snooping, zero page dirty tracking, and zero cache invalidation**.
11. **Cached Slice Pointer Dispatch (`current_steps: &'static [MicroStep]`)**:
    Upon opcode prefetch and retirement, `state.micro.current_steps` caches the slice pointer directly from `OPCODE_DESCRIPTOR_TABLE[ir]`. All subsequent CCK ticks during the instruction index `current_steps[micro_step]` directly, completely eliminating 65,536-entry table lookups in the hot execution loop.
12. **Dynamic Transfer Loop for Block Operations (`MOVEM`)**:
    `MOVEM` register count (0 to 16) is dynamically driven by the 16-bit extension mask in `scratch[0]` via `crate::instructions::movem::execute_movem_transfer`. Each set bit performs an atomic bus cycle and advances the mask, looping with 0 heap allocation and exact cycle timing.
13. **Cycle-Exact Hardware Exception Stacking**:
    Group 0 (Address/Bus Error) and Group 1/2 (Interrupts/Traps) exception stacking are modeled as dedicated micro-sequences (`EXCEPTION_GROUP0_STEPS`, `EXCEPTION_GROUP1_STEPS`). Stack writes and vector reads interact with `MemoryBus` and experience Chip RAM DMA wait states identical to real hardware.

### 1.1 The Microcode Archetype Baseline Architecture

To accelerate core refactoring, DMA contention modeling, and machine loop integration, the M68000 microcode engine is structured around **Microcode Archetypes**.

Instead of maintaining dozens of isomorphic instruction duplicates (such as `SUB` which mirrors `ADD`, `OR`/`AND`/`EOR` which share identical addressing and bus structures, or bit manipulation/shift variants), the core maintains 16 primary archetype modules that provide **100% structural representation** of all M68000 hardware bus cycles, addressing modes, and internal execution flows:

| Archetype Domain | Active Representative Modules | Hardware Mechanics Covered |
| :--- | :--- | :--- |
| **ALU & Arithmetic** | `add.rs`, `adda.rs`, `addi.rs`, `addq.rs`, `addx.rs`, `sub.rs`, `suba.rs`, `subi.rs`, `subq.rs`, `subx.rs`, `not.rs` | 2-phase operand read, RMW memory write-back, address register sign-extension, immediate latching, multi-precision extend (`X`) flag propagation, subtractive borrow/overflow evaluation, and bitwise complement. |
| **Bit & Shifts** | `bset.rs`, `asl.rs` | Read-Modify-Write bit cycles (static immediate & dynamic register-addressed), memory word shifts, register count-dependent loop delays, and condition evaluation. |
| **Data Movement** | `move_w.rs`, `movea.rs`, `moveq.rs`, `movem.rs` | Standard word transfers, address register loading, quick sign-extended immediates, and dynamic multi-register block transfers across arbitrary register lists. |
| **Memory Comparison** | `cmpm.rs` | Dual postincrement memory operand sequencing without destination write-back. |
| **Branch & Stack** | `bra.rs`, `bsr.rs`, `bcc.rs`, `pea.rs`, `jmp.rs`, `jsr.rs`, `rts.rs` | Short/long branches, subroutine return address stacking, stack popping, and 2-word pipeline refill across non-sequential Program Space addresses. |
| **Exceptions & System** | `nop.rs`, `trap.rs`, `system.rs` | Pipeline idling, software vector traps, supervisor stack frame generation, and status register manipulation. |

Derived and isomorphic instructions (`SUB*` [completed in Batch 1.1], `AND*`, `OR*`, `EOR*`, `CMP*`, `ASR`/`LS*`/`RO*`, `BTST`/`BCLR`/`BCHG`, `MOVE.B`/`MOVE.L`) inherit their micro-step sequences directly from these blueprints and are re-introduced in distinct, test-validated batches (see `ROADMAP.md` Step 1).

---
## 2. Data Structures & Type Definitions

The microcode data structures and static lookup tables are implemented in [`crates/m68000/src/micro/`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/).

### 2.1 The `AluFn` Function Pointer (Pure Internal CPU Operation)

Because all memory operands are already latched into `CpuState` (`prefetch[0]`, `last_read`, or `d[]/a[]`) before the ALU step runs, `AluFn` does **not** take `MemoryBus`. ALU handlers execute purely internally, operating directly on `CpuState` with pre-decoded register indices:
- Signature: `fn(state: &mut CpuState, reg_src: u8, reg_dst: u8)`
- Implementation: [`crates/m68000/src/micro/engine.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/engine.rs) and [`crates/m68000/src/micro/alu.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/alu.rs).

### 2.2 The `MicroStep` Descriptor (Stateless & Cache-Dense)

Each micro-step is an immutable, cache-dense `Copy` struct in `.rodata`:
- `step_fn`: Optional direct atomic execution function pointer (`Option<StepFn>`, where `StepFn = fn(&mut Cpu, &mut dyn AddressBus) -> BusResult<()>`). When `None`, the CPU loop directly executes `BusResult::Ready(())` with zero function pointer call overhead or BTB branch misprediction penalty.
- `alu_fn`: Optional function pointer to pure internal ALU logic (`Option<AluFn>`).
- `base_clocks`: Base CPU clocks consumed (2 for 1 CCK micro-steps, 0 for instantaneous ALU / branch evaluation).

Pre-decoded register indices (`reg_src`, `reg_dst`) are held once per opcode in `OpcodeDescriptor` and cached into `state.micro.reg_src` / `reg_dst`.

### 2.3 Specialized 2-Clock Atomic Micro-Step Handlers (`StepFn`)

By decomposing 4-clock bus cycles into native 2-clock slices ($1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$), the execution core eliminates runtime size checks (`match size`) and dynamic action matching:

| Category | Primitives (CCK1 / CCK2 Slices) | Hardware Operation & Bus Semantics |
| :--- | :--- | :--- |
| **Operand Reads (Data Space)** | `step_bus_read_src_word`, `step_bus_read_src_byte`, `step_bus_read_dst_word`, `step_bus_read_dst_byte`, `step_bus_read_src_long_high`, `step_bus_read_src_long_low`, `step_bus_read_dst_long_high`, `step_bus_read_dst_long_low` | **CCK1**: Reads from `ea_addr`. Stalls if Chip RAM blocked. Latches into `source`/`destination`.<br>**CCK2** (`BUS_READ_IDLE`): Physical bus free for Agnus DMA (`step_fn: None`). |
| **Operand Writes (Data Space)** | `BUS_WRITE_IDLE`, `step_bus_write_dst_word`, `step_bus_write_dst_byte`, `step_bus_write_dst_long_high`, `step_bus_write_dst_long_low` | **CCK1** (`BUS_WRITE_IDLE`): Internal setup; physical bus free for Agnus DMA (`step_fn: None`).<br>**CCK2**: Drives data from `destination` to memory. Stalls if wait states asserted. Retires if final step. |
| **Stack Operations (Data Space)** | `step_bus_push_stack_high_idle`, `step_bus_push_stack_high_write`, `step_bus_push_stack_low_write`, `step_bus_pop_stack_high_read`, `step_bus_pop_stack_high_finish`, `step_bus_pop_stack_low_read`, `step_bus_pop_stack_low_finish` | Stack reads and pushes over `SP` ($A_7$). Validates address alignment, adjusts SP, and transfers high/low words across CCK1/CCK2 phases. |
| **Prefetch & Refill (Program Space)** | `step_fetch_extension_read`, `step_fetch_extension_finish`, `step_prefetch_irc_read`, `step_prefetch_irc_finish`, `step_prefetch_next_read`, `BUS_READ_IDLE`, `step_bus_read_target_opcode_read`, `step_prefetch_target_read`, `step_prefetch_target_finish` | Reads from `pc` or branch target in Program Space ($FC_2$ / $FC_6$). Refills pipeline across 2-clock phases and manages standard or target retirement. |
| **Internal & Exceptions** | `step_write_word_at`, `step_write_byte_at`, `step_read_word_at`, `step_read_byte_at` | Instantaneous (0 CCK) internal operations, CCR updates, condition evaluation, and 2-phase CCK bus primitives for exception processing. Pure ALU steps utilize `step_fn: None`. |

Handlers are organized cleanly across [`crates/m68000/src/micro/step_execution.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/step_execution.rs) and [`crates/m68000/src/micro/step_control.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/step_control.rs).

### 2.4 CPU Micro-State Storage (`CpuMicroState`)

Embedded in `CpuState` to track sub-cycle progress across Color Clock phases with zero intermediate buffers:
- `source`: Explicit 32-bit storage for ALU source operand (incoming bus data is stored directly here on CCK1).
- `destination`: Explicit 32-bit storage for ALU destination operand and write-back data (bus write cycles read directly from here).
- `irc`: Instruction Register Capture — physical 68000 prefetch latch holding prefetched opcodes before retirement into IR.
- `ea_addr`: Resolved effective memory address for operands or branch/jump targets.
- `addr1`: Dual Staging Register 1 ($X_1$) — pre-staged address for multi-phase transfers (e.g. source EA or high-word split EA).
- `addr2`: Dual Staging Register 2 ($X_2$) — pre-staged address for multi-phase transfers (e.g. destination EA or low-word split EA).
- `ea_high`: High word of 32-bit absolute addresses (`(xxx).L`) or high address for split accesses.
- `movem_mask`: 16-bit register transfer mask for `MOVEM`.
- `movem_state`: Multi-cycle transfer progress state for `MOVEM` (bit 0 tracks CCK1 vs CCK2 sub-phase).
- `clocks_remaining`: Clocks remaining for the active micro-step countdown (0 when completed or between steps, decrements by 2 on each CCK).
- `current_steps`: Cached slice pointer to active opcode's `&'static [MicroStep]`.
- `micro_step`: Step index within the current instruction's micro-operation sequence.
- `reg_src`, `reg_dst`: Pre-decoded register indices ($0..7$ for $D_n / A_n$).
- `read_to_dest`: Indicates whether the active CCK1 read was targeted to `destination` (true) or `source` (false) for CCK2 logging.

### 2.5 The 65,536 Static Dispatch Universe (`OPCODE_DESCRIPTOR_TABLE`)

- Embedded in host `.rodata` via [`crates/m68000/src/micro/table.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/table.rs).
- Exactly 65,536 `OpcodeDescriptor` entries mapping every 16-bit opcode word directly to its pre-compiled `&'static [MicroStep]` sequence and pre-decoded register indices (`reg_src`, `reg_dst`).
- Requires **zero dynamic heap allocations** (`0` bytes allocated at runtime).

### 2.6 Addressing Mode Naming Convention & Canonical Module Layout

#### Canonical Addressing Mode Identifiers
To maintain rigorous mechanical sympathy and consistency with the Motorola M68000 Programmer's Reference Manual, static micro-step array slices and effective address functions follow standardized addressing mode identifiers:

| Suffix | M68000 Addressing Mode | Syntax | Hardware Semantics |
| :--- | :--- | :--- | :--- |
| `DN` | Data Register Direct | `Dn` | Operand resides directly in $D_0 \dots D_7$. |
| `AN` | Address Register Direct | `An` | Operand resides directly in $A_0 \dots A_7$. |
| `AI` | Address Register Indirect | `(An)` | Base pointer in $A_n$, zero displacement. |
| `PI` | Address Register Indirect with Postincrement | `(An)+` | Access memory at $A_n$, post-advance $A_n$ by operand size. |
| `PD` | Address Register Indirect with Predecrement | `-(An)` | Pre-decrement $A_n$ by operand size, access memory at updated $A_n$. |
| `D16_AN` | Address Register Indirect with Displacement | `(d16, An)` | 16-bit signed displacement fetched from PC stream added to $A_n$. |
| `IDX_AN` | Address Register Indirect with Index | `(d8, An, Xn)` | 8-bit signed displacement + index register ($D_n/A_n$) added to $A_n$. Consumes 2 clocks internal delay. |
| `ABSW` | Absolute Short | `(xxx).W` | 16-bit sign-extended absolute memory address. |
| `ABSL` | Absolute Long | `(xxx).L` | 32-bit absolute memory address fetched via 2 extension reads. |
| `D16_PC` | Program Counter Indirect with Displacement | `(d16, PC)` | 16-bit signed displacement added to base PC. Program Space read ($FC_2/FC_6$). |
| `IDX_PC` | Program Counter Indirect with Index | `(d8, PC, Xn)` | 8-bit signed displacement + index register added to base PC. Consumes 2 clocks internal delay. |
| `IMM` | Immediate Data | `#<data>` | Operand embedded in instruction stream at PC. |

Composite array names follow the strict convention `STEPS_<MNEMONIC>_<SIZE>_<SRC>_<DST>` (or `STEPS_<MNEMONIC>_<SRC>_<DST>` if size is inherent, and `STEPS_<MNEMONIC>_<MODE>` for single-operand instructions like `JMP` or `NOT`).

#### Standardized Common Micro-Step Constants (`crates/m68000/src/micro/common.rs`)
All instruction modules reuse shared atomic Color Clock micro-step primitives rather than duplicating local slice definitions:
- **Bus & Internal Idle Primitives:** `BUS_WRITE_IDLE` (CCK1 write setup / bus idle), `BUS_READ_IDLE` (CCK2 read completion / bus idle), `ALU_IDLE` (internal processing 2-clock delay / bus idle), `ALU_IDLE_4CLK` (4-clock internal execution delay / bus idle), `ALU_IDLE_8CLK` (8-clock internal exception delay / bus idle), `ALU_IDLE_128CLK` (128-clock `RESET` bus idle delay).
- **Operand Writes:** `WRITE_DST_BYTE`, `WRITE_DST_WORD`, `WRITE_DST_LONG_HIGH`, `WRITE_DST_LONG_LOW`.
- **Operand Reads:** `READ_SRC_BYTE`, `READ_SRC_WORD`, `READ_SRC_LONG_HIGH`, `READ_SRC_LONG_LOW`, `READ_DST_BYTE`, `READ_DST_WORD`, `READ_DST_LONG_HIGH`, `READ_DST_LONG_LOW`, `BUS_READ_IDLE` (shared CCK2 idle completion).
- **Dual Staged Address Reads:** `READ_ADDR1_BYTE`, `READ_ADDR1_WORD`, `READ_ADDR1_LONG_HIGH`, `READ_ADDR1_LONG_LOW`, `READ_ADDR2_BYTE`, `READ_ADDR2_WORD`, `READ_ADDR2_LONG_HIGH`, `READ_ADDR2_LONG_LOW`.
- **Prefetch & Extension:** `FETCH_EXT_READ`, `FETCH_EXT_FINISH`, `PREFETCH_IRC_READ`, `PREFETCH_IRC_FINISH`, `PREFETCH_NEXT_READ`, `BUS_READ_IDLE` (prefetch next finish / retirement).
- **Control Flow Refills:** `READ_TARGET_OPCODE_READ`, `BUS_READ_IDLE` (target opcode read finish), `PREFETCH_TARGET_READ`, `PREFETCH_TARGET_FINISH`.
- **Stack Operations:** `PUSH_STACK_HIGH_IDLE` (CCK1 push setup / bus idle), `PUSH_STACK_HIGH_WRITE`, `PUSH_STACK_LOW_WRITE`, `POP_STACK_HIGH_READ`, `POP_STACK_HIGH_FINISH`, `POP_STACK_LOW_READ`, `POP_STACK_LOW_FINISH`.
- **Exception Push Setup:** `EXCEPTION_PUSH_PCLO_IDLE`, `EXCEPTION_PUSH_SR_IDLE`, `EXCEPTION_PUSH_PCHI_IDLE`, `AERR_PUSH_*_IDLE` (CCK1 exception push setup / bus idle).

#### Canonical Layout for Instruction Modules
To guarantee zero cognitive friction and seamless codebase navigation across all instruction files, every module adheres to the standard 6-section sequence:
1. **Module Doc Comment:** High-level summary of mnemonic, addressing modes, and timing.
2. **Imports:** Imports from `crate::micro::common::*`, `crate::core::*`, `crate::micro::ea`.
3. **Leaf ALU Functions:** Direct arithmetic/logic helpers (`#[inline(always)] fn add_w(...)`, `sub_b(...)`) called directly by ALU callbacks to execute branchless wrapping math and CCR flag updates.
4. **Micro-Step ALU Callbacks (`AluFn`):** Functor callbacks (`fn alu_...`) stored as function pointers in `MicroStep.alu_fn`, dispatched dynamically via the micro-step engine.
5. **Specialized Micro-Step Handlers (`StepFn`):** Instruction-specific bus/execution handlers (if any required).
6. **Static Micro-Step Arrays:** `pub static STEPS_...: [MicroStep; N] = [...]`, ordered strictly by canonical addressing mode progression (`DN`, `AN`, `AI`, `PI`, `PD`, `D16_AN`, `IDX_AN`, `ABSW`, `ABSL`, `D16_PC`, `IDX_PC`, `IMM`). All instantaneous internal steps use `MicroStep::alu(...)`.
7. **Opcode Decoder:** Fast compile-time function `pub const fn decode_..._steps(...) -> Option<&'static [MicroStep]>`.

---

## 3. Parametric ALU Handlers: 8x Code Reduction

Passing `reg_src` and `reg_dst` into `AluFn` collapses code duplication across all register combinations without using forbidden macros or const-generics:
1. **Register Destinations:** The ALU reads `state.d_byte(reg_dst)` or `state.d_word(reg_dst)`, evaluates condition codes via branchless CCR setters, and writes directly back to the register.
2. **Memory Destinations:** When targeting memory (e.g. `ORI.B #$42, (A0)`), the ALU reads `state.micro.destination`, evaluates condition codes, and stores the result directly back into `state.micro.destination` for the subsequent write bus cycle.
3. **Effective Address Arithmetic:** Because `Alu` micro-steps consume 0 CCKs, effective address calculations (such as `(d16, An)` or `(d8, An, Xn)`) use the identical `AluFn` mechanism to compute and store addresses in `state.micro.ea_addr`.

All specialized ALU handlers reside in [`crates/m68000/src/micro/alu.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/alu.rs).

---

## 4. Memory Bus Operations & Strobe Semantics

### 4.1 Direct 2-Phase Bus Architecture (Zero Dynamic Strobe Boilerplate)

On the Amiga 500, the 4-clock M68000 bus cycle maps to **two Color Clock phases (CCK1 and CCK2)**. Amiga Chip DRAM accesses complete in 1 CCK (280 ns), interleaving bus access 50/50 between the CPU and Agnus DMA:

```
            ┌──────────────────────┬──────────────────────┐
            │     CCK1 (S0–S3)     │     CCK2 (S4–S7)     │
────────────┼──────────────────────┼──────────────────────┤
 READ Cycle │ Read to target reg   │ CPU idle / DMA slot  │
────────────┼──────────────────────┼──────────────────────┤
 WRITE Cycle│ CPU setup (idle bus) │ Write to DRAM / Wait │
────────────┴──────────────────────┴──────────────────────┘
```

- **READ Cycles (`BusReadByte`, `BusReadWord`, `BusReadLongHigh`, `BusReadLongLow`, `FetchExtension`, `PrefetchNextOpcodeAndRetire`):**
  - **CCK1 (S0–S3):** Bus read attempt via `bus.read_word(addr)` or `bus.read_byte(addr)`. If `BusResult::WaitState` $\to$ insert wait state (CPU stalls in CCK1). If `BusResult::Ready(data)` $\to$ stores data directly into `source`, `destination`, `prefetch[0]`, or `irc`, advancing `phase = CCK2`.
  - **CCK2 (S4–S7):** **Do nothing on the bus!** Physical Chip RAM is already released for custom chip DMA (Blitter, Copper). The CPU records the transaction directly from the target register, finishes the 4-clock cycle (or prefetch retirement), and advances to the next micro-step (`phase = CCK1`).
- **WRITE Cycles (`BusWriteByte`, `BusWriteWord`, `BusWriteLongHigh`, `BusWriteLongLow`, `BusPushStackHigh`, `BusPushStackLow`):**
  - **CCK1 (S0–S3):** **CPU does not touch the bus!** Internal address propagation only. Chip RAM remains completely free for Agnus DMA. Advances `phase = CCK2`.
  - **CCK2 (S4–S7):** Bus write attempt via `bus.write_word(addr, val)` or `bus.write_byte(addr, val)` reading directly from `state.micro.destination`. If `BusResult::WaitState` (Gary withholds $\overline{\text{DTACK}}$) $\to$ insert wait state (CPU stalls in CCK2). Once `BusResult::Ready(())` $\to$ write is committed to `bus`, recording the transaction and advancing to the next micro-step (`phase = CCK1`).

Byte strobes ($\overline{\text{UDS}}$ / $\overline{\text{LDS}}$) are derived natively by `bus.read_byte(addr)` / `bus.write_byte(addr, val)` from `addr & 1`. **The CPU core eliminates all manual strobe calculations, `BusCycle` allocations, and intermediate latch buffering.**

#### Execution Dispatch Implementation
The 2-phase Color Clock stepping logic is implemented via dedicated, single-purpose helper functions (`step_bus_read_byte`, `step_bus_read_word`, `step_bus_write_byte`, `step_bus_write_word`, etc.) in [`crates/m68000/src/micro/engine.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/engine.rs). Each helper executes the exact CCK1/CCK2 protocol with zero branch cascading and zero heap allocation.

### 4.2 Byte Strobe Activation & Byte Preservation

During an 8-bit Byte write operation (`BusWriteByte`):
1. **Even Byte Address (`ea_addr & 1 == 0`)**:
   - $\overline{\text{UDS}}$ asserts low ($0$); $\overline{\text{LDS}}$ remains negated ($1$).
   - The data byte is driven on $D_8 \dots D_{15}$.
   - The memory array writes **only into the high byte** of the addressed 16-bit word cell.
   - **The low byte in the 16-bit word cell is strictly PRESERVED and unaltered.**
2. **Odd Byte Address (`ea_addr & 1 == 1`)**:
   - $\overline{\text{LDS}}$ asserts low ($0$); $\overline{\text{UDS}}$ remains negated ($1$).
   - The data byte is driven on $D_0 \dots D_7$.
   - The memory array writes **only into the low byte** of the addressed 16-bit word cell.
   - **The high byte in the 16-bit word cell is strictly PRESERVED and unaltered.**
3. **Hardware Pin Duplication**:
   - In physical 68000 silicon, the CPU duplicates the byte on both bus halves ($D_{15..8} = \text{byte}$ and $D_{7..0} = \text{byte}$). The RAM chips use $\overline{\text{UDS}}/\overline{\text{LDS}}$ as write-enable qualifiers.
4. **Zero Address Errors on Byte Accesses**:
   - Unlike Word or Long accesses, Byte accesses **never generate an Address Error** on odd addresses. Odd byte reads and writes are 100% legal.

### 4.3 32-Bit Long Decomposition: Why `BusWriteLongHigh` Writes a 16-Bit Word

On the Motorola 68000, **a 32-bit transfer physically does not exist on the external bus**. The data bus has only 16 pins ($D_0 \dots D_{15}$).
Therefore, a 32-bit (Long) read or write is **strictly composed of two 16-bit Word bus cycles**:
- **`BusWriteLongHigh`**: Writes a **16-bit Word** containing the upper 16 bits of `write_buffer` to `ea_addr`.
- **`BusWriteLongLow`**: Writes a **16-bit Word** containing the lower 16 bits of `write_buffer` to `ea_addr + 2`.

Each cycle is an independent 4-clock / 2-CCK bus transaction that can stall on Agnus DMA contention in Chip RAM.

### 4.4 Operation Comparison Matrix

| Action | Physical Bus Size | Target Address | Data Source / Destination | $\overline{\text{UDS}}$ / $\overline{\text{LDS}}$ | Preservation Behavior |
| :--- | :---: | :--- | :--- | :---: | :--- |
| **`BusReadByte`** | 8-bit Byte | `ea_addr` | `last_read = byte` | Parity | Read-only |
| **`BusReadWord`** | 16-bit Word | `ea_addr` | `last_read = word` | Both active | Read-only (Address error if odd) |
| **`BusReadLongHigh`** | 16-bit Word | `ea_addr` | `scratch[0] = word << 16` | Both active | Read-only (Address error if odd) |
| **`BusReadLongLow`** | 16-bit Word | `ea_addr + 2` | `last_read = scratch[0] \| word` | Both active | Read-only (Address error if odd) |
| **`BusWriteByte`** | 8-bit Byte | `ea_addr` | `(write_buffer & 0xFF) as u8` | Parity | **Unaddressed byte preserved!** |
| **`BusWriteWord`** | 16-bit Word | `ea_addr` | `(write_buffer & 0xFFFF) as u16` | Both active | Entire 16-bit cell overwritten |
| **`BusWriteLongHigh`** | 16-bit Word | `ea_addr` | `(write_buffer >> 16) as u16` | Both active | Upper 16-bit cell overwritten |
| **`BusWriteLongLow`** | 16-bit Word | `ea_addr + 2` | `(write_buffer & 0xFFFF) as u16` | Both active | Lower 16-bit cell overwritten |

### 4.5 Function Codes (FC0–FC2) & Special Status Word (SSW) Encoding

On physical M68000 silicon, the CPU outputs 3 Function Code lines to qualify address space on every bus cycle:

| FC2 | FC1 | FC0 | Address Space Type | Usage in Emulator |
| :---: | :---: | :---: | :--- | :--- |
| 0 | 0 | 1 | **User Data** (`FC_UD`) | Data space read/write when `SR.S == 0` |
| 0 | 1 | 0 | **User Program** (`FC_UP`) | Opcode/extension fetch when `SR.S == 0` |
| 1 | 0 | 1 | **Supervisor Data** (`FC_SD`) | Data space read/write and stack pushes when `SR.S == 1` |
| 1 | 1 | 0 | **Supervisor Program** (`FC_SP`) | Opcode/extension fetch when `SR.S == 1` |
| 1 | 1 | 1 | **CPU Space / IACK** | Interrupt Acknowledge cycle |

#### Special Status Word (SSW) Format during Group 0 Exceptions:
When an Address Error or Bus Error occurs, word 0 of the 7-word stack frame (`SP - 2`) stores the diagnostic Special Status Word:
```
Bit:   15..5     4      3      2     1     0
Field: Unused   R/W    I/N    FC2   FC1   FC0
```
- **Bits 0–2 (FC0–FC2)**: The Function Code of the bus cycle that faulted.
- **Bit 3 (I/N)**: `0` if fault occurred during instruction processing; `1` if during instruction prefetch.
- **Bit 4 (R/W)**: `1` if read cycle; `0` if write cycle.
- **Bits 5–15**: Reserved / floating bus lines.

---

## 5. Control Flow, Pipeline Refill & Complex Addressing

### 5.1 The Two-Word Pipeline Refill

In normal sequential execution, instructions end with `PrefetchNextOpcodeAndRetire`, shifting `ir = prefetch[0]`, `prefetch[0] = last_read`, and `pc += 2`.

When control flow changes (`JMP`, `JSR`, `RTS`, `BRA`, taken `Bcc`):
- The existing prefetch pipeline contents are **flushed**.
- The CPU must perform a **two-word pipeline refill** directly from the new target address in Program Space (`FC2` User / `FC6` Supervisor):
  1. **Refill Bus Cycle 1 (`BusReadTargetOpcode`)**:
     - Bus read from `ea_addr` (4 CPU clocks / 2 CCKs).
     - Data is latched into `state.micro.scratch_prefetch` (the new opcode).
  2. **Refill Bus Cycle 2 (`PrefetchTargetAndRetire`)**:
     - Bus read from `ea_addr + 2` (4 CPU clocks / 2 CCKs).
     - Data is latched into `state.prefetch[0]`.
     - `state.ir` is loaded with `scratch_prefetch`.
     - `state.pc` is initialized to `ea_addr + 4`.
     - Instruction retires with target refill (`step_cck` returns `true`).

### 5.2 `JMP <ea>` (Jump to Effective Address)

- **Target Address Calculation**:
  - `(An)`: 0 internal clocks.
  - `(d16, An)`, `(xxx).w`, `(d16, PC)`: 2 internal clocks.
  - `(d8, An, Xn)`, `(d8, PC, Xn)`: 6 internal clocks.
  - `(xxx).l`: 1 extension read bus cycle (`FetchExtension`, 4 clocks).
- **Execution Flow**:
  1. Calculate target into `ea_addr`.
  2. Verify `(ea_addr & 1) == 0`. If odd, trigger **Address Error (Vector 3)** immediately!
  3. Execute `BusReadTargetOpcode` at `ea_addr`.
  4. Execute `PrefetchTargetAndRetire` at `ea_addr + 2`.
- **Total Time**:
  - `JMP (An)`: Exactly 8 CPU clocks (4 CCKs).
  - `JMP (xxx).L`: Exactly 12 CPU clocks (6 CCKs).

### 5.3 `JSR <ea>` (Jump to Subroutine)

- **Execution Flow**:
  1. Calculate target into `ea_addr`. Verify `(ea_addr & 1) == 0`.
  2. Determine return address `return_pc = base_pc + extension_words`.
  3. Decrement stack pointer: `SP = SP - 4`. Verify `(SP & 1) == 0`.
  4. Latch `write_buffer = return_pc`.
  5. Push Return Address:
     - `BusPushStackHigh`: Write `(return_pc >> 16) as u16` to `SP` (4 clocks).
     - `BusPushStackLow`: Write `(return_pc & 0xFFFF) as u16` to `SP + 2` (4 clocks).
  6. Execute Two-Word Refill:
     - `BusReadTargetOpcode` at `ea_addr` (4 clocks).
     - `PrefetchTargetAndRetire` at `ea_addr + 2` (4 clocks).
- **Total Time**:
  - `JSR (An)`: Exactly 16 CPU clocks (8 CCKs: 2 stack writes + 2 prefetch reads).
  - `JSR (xxx).L`: Exactly 20 CPU clocks (10 CCKs: 1 extension read + 2 stack writes + 2 prefetch reads).

### 5.4 `RTS` (Return from Subroutine)

- **Fixed Timing**: Exactly 16 CPU clocks (8 CCKs: 2 stack pop reads + 2 prefetch reads).
- **Execution Flow**:
  1. Pop High Word: `BusPopStack` reads word from `(SP)` into upper 16 bits of `ea_addr` (4 clocks).
  2. Pop Low Word: `BusPopStack` reads word from `(SP + 2)` into lower 16 bits of `ea_addr`, and updates `SP = SP + 4` (4 clocks).
  3. Verify `(ea_addr & 1) == 0`. If odd, trigger **Address Error (Vector 3)**!
  4. Execute Two-Word Refill:
     - `BusReadTargetOpcode` at `ea_addr` (4 clocks).
     - `PrefetchTargetAndRetire` at `ea_addr + 2` (4 clocks).

### 5.5 `Bcc` (Conditional Branch State Machine: Taken vs Not-Taken)

Conditional branches (`BEQ`, `BNE`, `BGT`, `BLT`, etc.) introduce dynamic runtime branching based on the Condition Code Register (CCR):

```mermaid
flowchart TD
    Eval["Step 0: alu_bcc_short / alu_bcc_word (0 CCKs)<br/>eval_condition(cond)"]
    
    Eval -- "Taken" --> TakenPath["current_steps = &STEPS_BRANCH_TAKEN<br/>micro_step = 0"]
    TakenPath --> Refill1["Step 0: step_alu (2 clocks)<br/>Step 1-2: READ_TARGET_OPCODE (4 clocks)"]
    Refill1 --> Refill2["Step 3-4: PREFETCH_TARGET_READ & FINISH (4 clocks)<br/>PC = target + 4<br/>Retire! (Total: 10 clocks)"]
    
    Eval -- "Not Taken (Short .B)" --> NotTakenB["current_steps = &STEPS_BRANCH_NOT_TAKEN_SHORT<br/>micro_step = 0"]
    NotTakenB --> SeqRetire["Step 0-1: step_alu idle (4 clocks)<br/>Step 2-3: PREFETCH_NEXT_READ & RETIRE (4 clocks)<br/>Retire! (Total: 8 clocks)"]
    
    Eval -- "Not Taken (Word .W)" --> NotTakenW["current_steps = &STEPS_BRANCH_NOT_TAKEN_WORD<br/>micro_step = 0"]
    NotTakenW --> SkipExt["Step 0-1: step_alu idle (4 clocks)<br/>Step 2-3: FETCH_EXT_READ & FINISH (4 clocks)<br/>Step 4-5: PREFETCH_NEXT_READ & RETIRE (4 clocks)<br/>Retire! (Total: 12 clocks)"]
```

---

### 5.6 Effective Address Calculation with Extension Word Prefetch (Memory RMW: `ADD.W Dn, (d16, An)`)

A common architectural question is: **How does the CPU handle operations where arguments are read from memory, the address requires displacement/extension words, and the result is written back to memory?**

Consider **`ADD.W D0, (d16, A0)`** (Add data register `D0` to memory operand at `(A0 + d16)`, storing result at `(A0 + d16)`).

```mermaid
flowchart TD
    subgraph S0["Step 0: Fetch Extension & Calculate EA (4 Clocks / 2 CCKs)"]
        EA["Address Unit (AU):<br/>ea_addr = A0 + (prefetch[0] as i16)"]
        FetchExt["Bus Read at PC:<br/>prefetch[0] = next word<br/>PC += 2"]
        EA --- FetchExt
    end

    subgraph S1["Step 1: Read Memory Argument (4 Clocks / 2 CCKs)"]
        ReadOp["BusReadWord at ea_addr:<br/>last_read = Memory[ea_addr]"]
    end

    subgraph S2["Step 2: Instantaneous ALU (0 CCKs)"]
        Exec["Alu:<br/>res = last_read + D0<br/>Set CCR (X,N,Z,V,C)<br/>write_buffer = res"]
    end

    subgraph S3["Step 3: RMW Opcode Prefetch (4 Clocks / 2 CCKs)"]
        PrefetchOp["Bus Read at PC:<br/>scratch_prefetch = next opcode<br/>(Class 0 RMW Quirk)"]
    end

    subgraph S4["Step 4: Write Result to Memory (4 Clocks / 2 CCKs)"]
        WriteRes["BusWriteWord at ea_addr:<br/>Memory[ea_addr] = write_buffer<br/>IR = scratch_prefetch<br/>Retire! (Total: 16 clocks)"]
    end

    S0 --> S1 --> S2 --> S3 --> S4
```

#### Detailed Clock-by-Clock Cycle Breakdown:
1. **The Prefetch Reality at Instruction Start**:
   - The opcode `$D168` is in `IR`.
   - Because the *previous* instruction concluded with an opcode prefetch, **the displacement word `d16` is ALREADY resident in `state.prefetch[0]` with 0 latency!**
2. **Step 0: `FetchExtension` with `ea_calc_d16_an` (4 clocks / 2 CCKs)**:
   - The AU calculates: `ea_addr = A0.wrapping_add((prefetch[0] as i16) as u32)`.
   - The bus controller fetches the next word from `PC` into `last_read` (advancing `PC += 2`).
   - At CCK2, `prefetch[0] = last_read`.
3. **Step 1: `BusReadWord` (4 clocks / 2 CCKs)**:
   - Reads the 16-bit word from memory at `ea_addr` into `last_read`.
4. **Step 2: `Alu` (`alu_add_rmw_w`) (0 CCKs Instantaneous)**:
   - Computes `res = last_read.wrapping_add(D0)`.
   - Evaluates and updates CCR condition code flags ($X, N, Z, V, C$).
   - Latches `state.micro.write_buffer = res as u32`.
5. **Step 3: `BusPrefetchToScratch` (4 clocks / 2 CCKs)**:
   - **Hardware Silicon Quirk (Class 0 RMW)**: The MC68000 executes the next instruction opcode prefetch *before* writing the modified operand back to memory!
   - Fetches the next opcode from `PC` into `state.micro.scratch_prefetch`, advancing `PC += 2`.
6. **Step 4: `BusWriteWordAndRetire` (4 clocks / 2 CCKs)**:
   - Drives 16-bit word `(write_buffer & 0xFFFF)` to `ea_addr`.
   - Overwrites the destination cell in memory.
   - At CCK2, shifts `IR = scratch_prefetch`, and completes instruction retirement!

**Total Execution Time**: Exactly **16 CPU clocks (8 CCKs)** (matching Motorola 68000 hardware: 12 clocks base + 4 clocks for `d16` addressing mode).

---

### 5.7 Indexed Addressing with Internal Calculation Clocks (`ADD.W (8, A0, D1.W), D0`)

When an instruction uses **Address Register Indirect with Index and 8-Bit Displacement** (`(d8, An, Xn)`):
1. The extension word specifies the 8-bit signed displacement and index register (`D1.W`).
2. The address calculation requires a 3-input addition: `ea_addr = An + Xn + disp8`.
3. In physical MC68000 silicon, this 3-input addition consumes **2 internal CPU clock cycles** ($N_{internal} = 2$ clocks / 1 CCK) with zero external bus activity.

#### Example Execution Trace: `ADD.W (8, A0, D1.W), D0` (Opcode `$D070`)
- **Total Duration**: Exactly **14 CPU clocks (7 CCKs)** (8 clocks base + 6 clocks for index mode: 4 bus clocks + 2 internal clocks).

```mermaid
flowchart TD
    subgraph P["Instruction Start (Opcode in IR)"]
        IR["IR = $D070 (Opcode)<br/>prefetch[0] = Extension Word (8-bit disp + D1.W)"]
    end

    subgraph S0["Step 0: AU 3-Input Add Calculation (2 Clocks / 1 CCK)"]
        EA["step_alu with ea_calc_src_idx_an:<br/>ea_addr = A0 + D1.W + 8<br/>Bus is completely idle!"]
    end

    subgraph S1["Step 1-2: Fetch Extension Word (4 Clocks / 2 CCKs)"]
        FetchExt["FETCH_EXT_READ & FINISH:<br/>prefetch[0] = next word (PC+4)<br/>PC += 2"]
    end

    subgraph S2["Step 3-4: Read Memory Operand (4 Clocks / 2 CCKs)"]
        ReadOp["READ_SRC_WORD & FINISH:<br/>source = Memory[ea_addr]"]
    end

    subgraph S3["Step 5-6: Opcode Prefetch, ALU & Retirement (4 Clocks / 2 CCKs)"]
        Prefetch["PREFETCH_NEXT_READ (alu_add_w) & RETIRE:<br/>D0 = D0 + source, set CCR<br/>Refill IR and prefetch[0]<br/>Retire! (Total: 14 clocks / 7 CCKs)"]
    end

    P --> S0 --> S1 --> S2 --> S3
```

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `step_alu`<br/>*(with `ea_calc_src_idx_an`)* | 2 (1 CCK) | AU decodes `prefetch[0]`: extracts `disp8 = +8`, `Xn = D1.W`, and sets `ea_addr = A0 + D1.W + 8`. Consumes 2 clocks (1 CCK) idle AU addition. |
| **1-2** | `FETCH_EXT_READ`<br/>`FETCH_EXT_FINISH` | 4 (2 CCKs) | Reads extension word from `PC` into `prefetch[0]`, `PC += 2`. |
| **3-4** | `READ_SRC_WORD`<br/>`BUS_READ_IDLE` | 4 (2 CCKs) | Reads 16-bit operand from `ea_addr` into `source`. |
| **5-6** | `PREFETCH_NEXT_READ`<br/>`BUS_READ_IDLE` | 4 (2 CCKs) | Computes `D0 = D0 + source`, sets CCR flags, refills `IR` and `prefetch[0]`, and retires! |

**Total Duration**: $4\ (\text{FetchExtension}) + 2\ (\text{Internal Delay}) + 4\ (\text{BusReadWord}) + 0\ (\text{ALU}) + 4\ (\text{Prefetch}) = \mathbf{14\ \text{CPU clocks}}\ (7\ \text{CCKs})$!

---

### 5.8 `MOVEM` Dynamic Register Transfer Engine

In `MOVEM <ea>, reglist` and `MOVEM reglist, <ea>`, the opcode specifies the transfer direction, size (Word vs. Long), and effective addressing mode. However, the **number of registers transferred (0 to 16)** is dynamic, specified by the 16-bit **extension mask word**:

```
Bit:   15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
Reg:   A7  A6  A5  A4  A3  A2  A1  A0  D7  D6  D5  D4  D3  D2  D1  D0   (Standard / Postincrement)
Reg:   D0  D1  D2  D3  D4  D5  D6  D7  A0  A1  A2  A3  A4  A5  A6  A7   (Predecrement -(An) mode)
```

#### The `MovemTransfer` Loop Mechanics:
Because a static array cannot predict the number of registers at compile time, `MOVEM` is modeled via the atomic **`execute_movem_transfer`** sub-loop:
1. **Step 0 (`FetchExtension`)**: Reads the 16-bit register mask from `PC` into `state.micro.scratch[0]`, advancing `PC += 2`.
2. **Step 1 (`Alu`)**: Resolves `ea_addr = An` (or calculates displacement/index).
3. **Step 2 (`MovemTransfer`)**:
   - Inspects `state.micro.scratch[0]`. If `scratch[0] == 0`, immediately falls through and advances `micro_step += 1`.
   - Finds the next active register bit $k$:
     - **For Memory-to-Register / Standard**: Finds lowest set bit ($k = \text{trailing\_zeros}(scratch[0])$).
     - **For Predecrement `-(An)`**: Finds highest set bit ($k = 15 - \text{leading\_zeros}(scratch[0])$).
   - In CCK1/CCK2, performs the atomic bus transfer (Read or Write, Word or 2-cycle Long) between register $R_k$ and memory at `ea_addr`.
   - Adjusts `ea_addr`: increments `ea_addr += size` (or decrements `ea_addr -= size` for `-(An)`).
   - Clears bit $k$ in `state.micro.scratch[0]`.
   - **Loop Condition**: If `state.micro.scratch[0] != 0`, `micro_step` **does NOT advance**, repeating `MovemTransfer` on the subsequent bus cycle!
4. **Step 3 (`PrefetchNextOpcodeAndRetire`)**: Standard sequential opcode prefetch, completing retirement.

#### Exact Hardware Timing:
- **Base Overhead**: 12 CPU clocks (6 CCKs: 1 opcode fetch + 1 mask fetch + 1 opcode prefetch) for `(An)`.
- **Per-Register Transfer**:
  - **Word (`.W`)**: Exactly **4 CPU clocks (2 CCKs)** per transferred register.
  - **Long (`.L`)**: Exactly **8 CPU clocks (4 CCKs: 2 bus cycles)** per transferred register.
- **Example (`MOVEM.W (A0)+, D0-D2/A0-A1` — 5 registers)**:
  $12 + 5 \times 4 = \mathbf{32\ \text{CPU clocks}}\ (16\ \text{CCKs})$, matching Motorola 68000 silicon to the exact clock!

---

### 5.9 Cycle-Exact Hardware Exception Processing & Stacking

When a hardware or software exception occurs (Address Error, Bus Error, Interrupt, Illegal Instruction, `TRAP`, Zero Divide), instruction execution halts, and the CPU enters **Exception Processing**.

On the Amiga 500, exception stack pushes and vector table reads occur over the external bus and **experience Agnus Chip RAM DMA wait states identical to standard instruction cycles**.

#### Group 0 Exceptions: Address Error (Vector 3) & Bus Error (Vector 2)
Group 0 exceptions push a **7-word (14-byte) diagnostic stack frame** to allow the OS (or Guru Meditation handler) to diagnose the bus fault:

```
SP - 2  -->  [ Special Status Word (SSW): R/W bit, I/N bit, Function Code FC0..FC2 ]
SP - 4  -->  [ Access Address High 16-bits ]
SP - 6  -->  [ Access Address Low 16-bits ]
SP - 8  -->  [ Instruction Register (IR) at time of fault ]
SP - 10 -->  [ Status Register (SR) prior to exception ]
SP - 12 -->  [ Program Counter (PC) High 16-bits ]
SP - 14 -->  [ Program Counter (PC) Low 16-bits ]
```

##### Group 0 Execution Flow:
1. Transition CPU to Supervisor Mode: `SR.S = 1`, `SR.T = 0`.
2. Push 7 Words onto Supervisor Stack (`SSP` / `A7'`):
   - 7 sequential word write bus cycles (`BusPushStackHigh` / `BusPushStackLow`).
3. Fetch Exception Vector:
   - 2 sequential word read bus cycles at address `$000000 + VectorNum * 4` (e.g. `$00000C` for Address Error).
4. Pipeline Refill:
   - Two-word pipeline refill (`BusReadTargetOpcode` + `PrefetchTargetAndRetire`) from the handler address loaded from the vector.
5. **Total Execution Duration**: Exactly **50 CPU clocks (25 CCKs)** base time + wait states.

#### Group 1 & Group 2 Exceptions: Interrupts (IPL 1–7), TRAPs, Illegal Instructions
Push a standard **3-word (6-byte) stack frame**:
```
Final SSP + 0  (old SSP - 6)  -->  [ Status Register (SR) prior to exception ]
Final SSP + 2  (old SSP - 4)  -->  [ Program Counter (PC) High 16-bits ]
Final SSP + 4  (old SSP - 2)  -->  [ Program Counter (PC) Low 16-bits ]
```

##### Physical Bus Write Sequence for 3-Word Frame (68000 Silicon Order):
On physical M68000 hardware, the internal ALU/bus microcode writes the 3-word frame in the following cycle order:
1. Write PC low word to `SSP - 2` (Supervisor Data, FC 5).
2. Write old SR to `SSP - 6` (Supervisor Data, FC 5).
3. Write PC high word to `SSP - 4` (Supervisor Data, FC 5) and commit updated `SSP = SSP - 6`.

##### Modular 17-Step Microcode Pipeline for `TRAP #<vector>` (34 Clocks / 17 CCKs):
1. `Step 1 (ALU_TRAP_INIT)`: CCK1 of initial 4-clock processing. Switches to supervisor ($S=1, T=0$), computes vector address `$000080 + \text{vec} \times 4$, latches return PC into `source`, old SR into `destination`, and records 4-clock internal transaction.
2. `Step 2 (ALU_IDLE)`: CCK2 of initial internal clocks (bus idle for Agnus DMA).
3. `Step 3 (EXCEPTION_PUSH_PCLO_IDLE)`: CCK1 of writing return PC low word to `SSP - 2` (bus idle, alignment validation).
4. `Step 4 (EXCEPTION_PUSH_PCLO_WRITE)`: CCK2 of writing return PC low word to `SSP - 2`.
5. `Step 5 (EXCEPTION_PUSH_SR_IDLE)`: CCK1 of writing old SR to `SSP - 6` (bus idle, alignment validation).
6. `Step 6 (EXCEPTION_PUSH_SR_WRITE)`: CCK2 of writing old SR to `SSP - 6`.
7. `Step 7 (EXCEPTION_PUSH_PCHI_IDLE)`: CCK1 of writing return PC high word to `SSP - 4` (bus idle, alignment validation).
8. `Step 8 (EXCEPTION_PUSH_PCHI_WRITE)`: CCK2 of writing return PC high word to `SSP - 4` and committing `SSP = SSP - 6`.
9. `Step 9 (READ_VECTOR_HIGH_READ)`: CCK1 of reading vector high word from `ea_addr` into `ea_high`.
10. `Step 10 (BUS_READ_IDLE)`: CCK2 of reading vector high word (bus free for Agnus DMA).
11. `Step 11 (READ_VECTOR_LOW_READ)`: CCK1 of reading vector low word from `ea_addr + 2` into `source`.
12. `Step 12 (READ_VECTOR_LOW_FINISH)`: CCK2 of reading vector low word (verifies even target address alignment).
13. `Step 13 (READ_TARGET_OPCODE_READ)`: CCK1 of reading target opcode into `irc`.
14. `Step 14 (BUS_READ_IDLE)`: CCK2 of reading target opcode (bus free for Agnus DMA).
15. `Step 15 (ALU_IDLE)`: 2 internal clocks before prefetch (internal ALU delay).
16. `Step 16 (PREFETCH_TARGET_READ)`: CCK1 of reading target + 2 into `prefetch[0]`.
17. `Step 17 (PREFETCH_TARGET_FINISH)`: CCK2 of reading target + 2 (sets `target_refill = true`, triggers clean instruction retirement).

- **Total Duration**:
  - `TRAP #n`: Exactly **34 CPU clocks (17 CCKs)** (3 stack writes + 2 vector reads + 2 refill reads + 6 internal clocks).
  - Interrupt (Autovector): Exactly **44 CPU clocks (22 CCKs)** (includes 4-clock Interrupt Acknowledge `IACK` bus cycle with `FC = %111`).


---

### 5.10 Stack Pointer Predecrement Alignment Quirk (`-(SP)`)

In Motorola 68000 silicon, the Stack Pointer (`A7`, both `USP` and `SSP`) must **always remain on an even 16-bit word boundary**.
- When addressing mode `-(An)` is evaluated with `An = A7` for an **8-bit Byte** operation (e.g. `MOVE.B D0, -(SP)`):
  - In `ea_calc_predec_an`: the decrement amount is forced to **2 bytes** instead of 1 (advancing $A_7 \leftarrow A_7 - 2$).
  - The byte is written to the high byte (even address) with $\overline{\text{UDS}}$ asserted.
  - This ensures that stack frames and popped values never become misaligned.

---

## 6. The Execution Engine & Stepping Model (`step_cck`)

Execution is driven by the Amiga Color Clock (CCK, ~3.54 MHz). $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks} = 2\ \text{CCK phases}\ (\text{CCK1} + \text{CCK2})$.

```mermaid
flowchart TD
    Start(["step_cck()"]) --> Adv["advance_clocks(2)"]
    Adv --> CheckSeq["Execute Micro-Step (execute_micro_step)"]
    
    CheckSeq --> InitCheck{"clocks_remaining == 0?"}
    InitCheck -- Yes --> Init["clocks_remaining = base_clocks<br/>Execute alu_fn (if any)"] --> InstCheck{"clocks_remaining == 0?"}
    InstCheck -- Yes (Instant ALU) --> InstExec["micro_step += 1<br/>continue to next step"] --> CheckSeq
    InstCheck -- No --> ExecStep["Execute step_fn(self, bus)"]
    InitCheck -- No --> ExecStep
    
    ExecStep --> BusRes{"BusResult?"}
    BusRes -- WaitState --> Stall["CPU stalls (phase held)<br/>return false"]
    BusRes -- Ready --> Dec["clocks_remaining -= 2"]
    
    Dec --> DoneCheck{"clocks_remaining == 0?"}
    DoneCheck -- No --> Hold["return false (bus idle)"]
    DoneCheck -- Yes --> Next["micro_step += 1"]
    
    Next --> RetCheck{"micro_step >= steps.len()?"}
    RetCheck -- Yes --> Retire["retire_current_instruction()<br/>return true"]
    RetCheck -- No --> RetFalse["return false"]
```

### 6.1 Concrete Implementation: The High-Throughput `step_cck()` Dispatcher

The full CCK stepping engine is implemented in [`crates/m68000/src/micro/engine.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/micro/engine.rs) and driven via [`crates/m68000/src/core.rs`](file:///d:/Programowanie/Amiga/crates/m68000/src/core.rs):

1. **Uniform Micro-Step Timing via `clocks_remaining: u16`:**
   - On each CCK tick, `step_cck` advances global clocks by 2 (`self.advance_clocks(2)`).
   - If entering a step (`clocks_remaining == 0`), initializes `clocks_remaining = step.base_clocks` and fires `alu_fn`. Dynamic shift/ALU operations directly set `clocks_remaining = duration`.
   - On completion of a ready CCK phase, decrements `clocks_remaining = clocks_remaining.saturating_sub(2)`. When it reaches 0, concludes the micro-step and advances to the next (`micro_step += 1`).
2. **Instantaneous Fall-Through Micro-Steps:**
   - A fast internal loop executes zero-cycle micro-operations (`alu_fn`) when entering a step or when `base_clocks == 0` within the same host tick until encountering a bus cycle.
   - If a subsequent bus cycle stalls due to Chip RAM contention, the ALU calculation is **never repeated**.
3. **Two-Phase External Bus Execution via Atomic Micro-Steps:**
   - **CCK1 (Phase 1):** Read steps (`step_bus_read_*`) issue `bus.read_word(addr)` / `bus.read_byte(addr)`. If `BusResult::WaitState`, the CPU stalls without advancing `micro_step` (returns `false`). If `BusResult::Ready(data)`, samples memory data into `source` / `destination` / `prefetch` / `irc` and advances to CCK2 (`micro_step += 1`). For write actions, `step_bus_write_idle` does not touch the bus (bus idle for DMA) and simply advances to CCK2 (`micro_step += 1`).
   - **CCK2 (Phase 2):** Write steps (`step_bus_write_dst_*`) issue `bus.write_word(addr, val)` / `bus.write_byte(addr, val)`. If `BusResult::WaitState`, stalls without committing (returns `false`). If `BusResult::Ready(())`, commits data, records transaction, and advances `micro_step += 1`, completing the 4-clock bus cycle. Read finish steps (`step_bus_read_*_finish`) log the transaction, release the bus for Agnus DMA, and advance `micro_step += 1`.
4. **Pipeline Advance & Instruction Retirement:**
   - On retirement, shifts `ir = prefetch[0]`, `prefetch[0] = scratch_prefetch` (or target prefetch), advances `pc += 2`, updates `current_steps`, and returns `true`. Multi-cycle instruction steps call `step_instruction(&mut self, bus) -> u32` to step to retirement and return total CPU clocks consumed.

---

## 7. Concrete Micro-Step Traces by Architectural Class

The table below catalogs representative micro-step sequences for each fundamental instruction class, highlighting how **prefetch** and **result writing** are coordinated.

---

### Class 1: Register-to-Register Operations (Pure Internal ALU, Zero Memory Writes)

#### Archetype: `ADD.W D1, D0` (Opcode `$D041`)
- **Total Duration**: 4 CPU clocks (2 CCKs).
- **Result Writing**: Register destination (`D0`). Zero external write cycles.
- **Prefetch**: Single sequential prefetch of the next instruction opcode.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `Alu` | **0 (Instant)** | Calls `alu_add_w(&mut state, 1, 0)`. Computes `D0 = D0 + D1`, sets CCR flags ($X, N, Z, V, C$). |
| **1** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Schedules bus read at `PC`. Updates `prefetch[0] = last_read`. Latching new `IR`, `PC += 2`, retires! |

---

### Class 2: Immediate to Register Operations (Extension Prefetch + ALU, Zero Memory Writes)

#### Archetype: `ORI.B #$42, D0` (Opcode `$0000`)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: Register destination (`D0`). Zero external write cycles.
- **Prefetch**: First bus cycle fetches immediate extension word; second bus cycle prefetches next instruction opcode.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FetchExtension` | 4 (2 CCKs) | Bus read at `PC`, advances `PC += 2`. Latching immediate word `#$0042` into `last_read`. |
| **1** | `Alu` | **0 (Instant)** | Calls `alu_ori_b(&mut state, 0, 0)`. Computes `D0 = D0 \| imm`, updates CCR flags. |
| **2** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Bus read at `PC`. Latching next opcode into `IR`, `prefetch[0] = last_read`, `PC += 2`, retires! |

---

### Class 3: Memory Source to Register Load (Memory Read, Zero Memory Writes)

#### Archetype: `ADD.W (A0), D0` (Opcode `$D050`)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: Register destination (`D0`). Zero external write cycles.
- **Prefetch**: Read operand from memory, then prefetch next opcode.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `BusReadWord` | 4 (2 CCKs) | Bus read from `ea_addr = A0` into `last_read`. Stalls if Chip RAM blocked. |
| **1** | `Alu` | **0 (Instant)** | Calls `alu_add_w_mem(&mut state, 0, 0)`. Computes `D0 = D0 + last_read`, sets CCR flags. |
| **2** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Bus read at `PC`. Refills pipeline with next instruction opcode and retires! |

---

### Class 4: Register to Memory Store (Word and 32-Bit Long Write)

#### Archetype 4A: `MOVE.W D0, (A0)` (Opcode `$3080` — 16-Bit Word Store)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: Single 16-bit Word write cycle (`BusWriteWord`) at `(A0)`.
- **Prefetch**: Opcode prefetch follows write.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `Alu` | **0 (Instant)** | Latches `write_buffer = D0 as u32`, `ea_addr = A0`. Sets CCR flags ($N, Z, V=0, C=0$). |
| **1** | `BusWriteWord` | 4 (2 CCKs) | Drives 16-bit word `(write_buffer & 0xFFFF)` to `ea_addr`. Both $\overline{\text{UDS}}$ and $\overline{\text{LDS}}$ assert. |
| **2** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Bus read at `PC`. Prefetches next opcode and retires! |

#### Archetype 4B: `MOVE.L D0, (A0)` (Opcode `$2080` — 32-Bit Long Store Decomposition)
- **Total Duration**: 12 CPU clocks (6 CCKs).
- **Result Writing**: Two sequential 16-bit Word write cycles (`BusWriteLongHigh` followed by `BusWriteLongLow`).
- **Prefetch**: Opcode prefetch follows the second write cycle.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `Alu` | **0 (Instant)** | Latches `write_buffer = D0` (32 bits), `ea_addr = A0`. Sets CCR flags. |
| **1** | `BusWriteLongHigh` | 4 (2 CCKs) | Drives High Word `((write_buffer >> 16) & 0xFFFF)` to `ea_addr`. Stalls if Chip RAM blocked. |
| **2** | `BusWriteLongLow` | 4 (2 CCKs) | Drives Low Word `(write_buffer & 0xFFFF)` to `ea_addr + 2`. Stalls if Chip RAM blocked. |
| **3** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Bus read at `PC`. Prefetches next opcode and retires! |

---

### Class 5: Memory Read-Modify-Write with Extension Prefetch (`ADD.W Dn, (d16, An)`)

#### Archetype 5A: `ADD.W D0, (d16, A0)` (Opcode `$D168` — Memory RMW with Displacement)
- **Total Duration**: 16 CPU clocks (8 CCKs).
- **Prefetch**: Displacement `d16` in `prefetch[0]`; fetches next extension, then reads memory, then opcode prefetch before write!
- **Result Writing**: Commits modified word to `ea_addr` in the final step.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FetchExtension` | 4 (2 CCKs) | AU computes `ea_addr = A0 + disp16`. Bus reads next word at `PC`, advancing `PC += 2`. |
| **1** | `BusReadWord` | 4 (2 CCKs) | Reads memory operand from `ea_addr` into `last_read`. |
| **2** | `Alu` | **0 (Instant)** | Calls `alu_add_rmw_w`. Computes `res = last_read + D0`, updates CCR, sets `write_buffer = res as u32`. |
| **3** | `BusPrefetchToScratch` | 4 (2 CCKs) | **RMW Quirk:** Prefetches next opcode from `PC` into `scratch_prefetch`. |
| **4** | `BusWriteWordAndRetire` | 4 (2 CCKs) | Writes `write_buffer` to `ea_addr`. Shifts `scratch_prefetch` into `IR` and completes retirement! |

#### Archetype 5B: `ADDI.W #$1234, (d16, A0)` (Opcode `$0668` — Immediate + Displacement Memory RMW)
- **Total Duration**: 20 CPU clocks (10 CCKs).
- **Prefetch**: 2 extension words (immediate `#$1234` + displacement `d16`), then memory read, then opcode prefetch before write.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FetchExtension` | 4 (2 CCKs) | Latches immediate: `scratch[0] = prefetch[0]`. Reads `d16` from `PC` into `prefetch[0]`, `PC += 2`. |
| **1** | `FetchExtension` | 4 (2 CCKs) | AU computes `ea_addr = A0 + d16`. Reads next word from `PC` into `prefetch[0]`, `PC += 2`. |
| **2** | `BusReadWord` | 4 (2 CCKs) | Reads memory operand from `ea_addr` into `last_read`. |
| **3** | `Alu` | **0 (Instant)** | Computes `res = last_read + (scratch[0] as u16)`, updates CCR, sets `write_buffer = res as u32`. |
| **4** | `BusPrefetchToScratch` | 4 (2 CCKs) | **RMW Quirk:** Prefetches next opcode from `PC` into `scratch_prefetch`. |
| **5** | `BusWriteWordAndRetire` | 4 (2 CCKs) | Writes `write_buffer` to `ea_addr`. Shifts `scratch_prefetch` into `IR` and retires! |

---

### Class 6: Subroutine Call / Stack Push (`JSR <ea>`)

#### Archetype: `JSR (A0)` (Opcode `$4E90`)
- **Total Duration**: 16 CPU clocks (8 CCKs).
- **Result Writing**: Pushes 32-bit return address onto stack (two 16-bit word writes to `-(SP)`).
- **Prefetch**: Two-word pipeline refill directly from target address `(A0)`.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `Alu` | **0 (Instant)** | Resolves `ea_addr = A0`. Decrements `SP = SP - 4`. Latches `write_buffer = return_pc`. |
| **1** | `BusPushStackHigh` | 4 (2 CCKs) | Writes MSW `(write_buffer >> 16)` to `SP` (Data Space). |
| **2** | `BusPushStackLow` | 4 (2 CCKs) | Writes LSW `(write_buffer & 0xFFFF)` to `SP + 2` (Data Space). |
| **3** | `BusReadTargetOpcode` | 4 (2 CCKs) | **Refill 1:** Reads new opcode from Program Space at `ea_addr` into `scratch_prefetch`. |
| **4** | `PrefetchTargetAndRetire` | 4 (2 CCKs) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. Loads `IR = scratch_prefetch`, sets `PC = ea_addr + 4`, retires! |

---

### Class 7: Subroutine Return / Stack Pop (`RTS`)

#### Archetype: `RTS` (Opcode `$4E75`)
- **Total Duration**: 16 CPU clocks (8 CCKs).
- **Result Writing**: None (Memory reads from stack). Updates `SP = SP + 4`.
- **Prefetch**: Two-word pipeline refill directly from popped return address.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `BusPopStack` | 4 (2 CCKs) | Reads MSW from `(SP)` into upper 16 bits of `ea_addr`. |
| **1** | `BusPopStack` | 4 (2 CCKs) | Reads LSW from `(SP + 2)` into lower 16 bits of `ea_addr`, sets `SP = SP + 4`. Checks alignment. |
| **2** | `BusReadTargetOpcode` | 4 (2 CCKs) | **Refill 1:** Reads new opcode from Program Space at `ea_addr` into `scratch_prefetch`. |
| **3** | `PrefetchTargetAndRetire` | 4 (2 CCKs) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. Loads `IR = scratch_prefetch`, sets `PC = ea_addr + 4`, retires! |

---

### Class 8: Unconditional Jump (`JMP <ea>`)

#### Archetype: `JMP (A0)` (Opcode `$4ED0`)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: None.
- **Prefetch**: Flushes pipeline and performs two-word pipeline refill from `(A0)`.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `Alu` | **0 (Instant)** | Resolves `ea_addr = A0`. Checks `(ea_addr & 1) == 0`. (If odd, triggers Address Error). |
| **1** | `BusReadTargetOpcode` | 4 (2 CCKs) | **Refill 1:** Reads new opcode from Program Space at `ea_addr` into `scratch_prefetch`. |
| **2** | `PrefetchTargetAndRetire` | 4 (2 CCKs) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. Loads `IR = scratch_prefetch`, sets `PC = ea_addr + 4`, retires! |

---

### Class 9: Conditional Branching (`Bcc`) (Dynamic Path Selection)

#### Archetype 9A: `BNE.B <disp8>` (Opcode `$66xx` — Short Branch)
- **Total Duration**: 10 CPU clocks (5 CCKs) if Taken; 8 CPU clocks (4 CCKs) if Not Taken.
- **Result Writing**: None.
- **Prefetch**: Two-word refill if taken; sequential prefetch if not taken.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `BranchEval` | **0 (Instant)** | Evaluates condition code.<br/>- **Taken**: sets `current_steps = &STEPS_BRANCH_TAKEN`, `micro_step = 0`. *(Total: 10 clocks)*<br/>- **Not Taken**: sets `current_steps = &STEPS_BRANCH_NOT_TAKEN_SHORT`, `micro_step = 0`. *(Total: 8 clocks)* |
| **1 (Taken)** | `BusReadTargetOpcode` | 4 (2 CCKs) | **Refill 1:** Reads target opcode from `ea_addr` into `scratch_prefetch`. |
| **2 (Taken)** | `PrefetchTargetAndRetire` | 4 (2 CCKs) | **Refill 2:** Reads `ea_addr + 2`. Sets `IR = scratch_prefetch`, `prefetch[0] = last_read`, `PC = ea_addr + 4`, and retires! *(Total: 10 clocks)* |
| **3 (Not Taken)** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Performs standard sequential prefetch from `PC`, advancing to next instruction. Retires! *(Total: 8 clocks)* |

#### Archetype 9B: `BEQ.W <disp16>` (Opcode `$6700` — Word Branch)
- **Total Duration**: 10 CPU clocks (5 CCKs) if Taken; 12 CPU clocks (6 CCKs) if Not Taken.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `BranchEval` | **0 (Instant)** | Evaluates condition code.<br/>- **Taken**: sets `current_steps = &STEPS_BRANCH_TAKEN`, `micro_step = 0`. *(Total: 10 clocks)*<br/>- **Not Taken**: sets `current_steps = &STEPS_BRANCH_NOT_TAKEN_WORD`, `micro_step = 0`. *(Total: 12 clocks)* |
| **1 (Taken)** | `BusReadTargetOpcode` | 4 (2 CCKs) | **Refill 1:** Reads target opcode from `ea_addr`. |
| **2 (Taken)** | `PrefetchTargetAndRetire` | 4 (2 CCKs) | **Refill 2:** Reads `ea_addr + 2`. Loads new `IR`, sets `PC = ea_addr + 4`, retires! *(Total: 10 clocks)* |
| **4 (Not Taken)** | `FetchExtension` | 4 (2 CCKs) | Bus read to skip over displacement extension word in memory stream. |
| **5 (Not Taken)** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Performs standard sequential prefetch from `PC`. Retires! *(Total: 12 clocks)* |

---

### Class 10: Variable-Cycle Arithmetic (`DIVU.W <ea>, Dn`)

#### Archetype: `DIVU.W (A0), D0` (Opcode `$80D0`)
- **Total Duration**: Variable: 108 to 140 CPU clocks.
- **Result Writing**: Register destination (`D0`). Sets quotient and remainder. Zero memory writes.
- **Prefetch**: Reads divisor word, waits internal division delay (`clocks_remaining`), then completes sequential opcode prefetch.

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `BusReadWord` | 4 (2 CCKs) | Reads 16-bit divisor from memory at `(A0)`. Latches into `last_read`. |
| **1** | `Alu` | **0 (Instant)** | Calls `alu_divu_w(&mut state, 0, 0)`. Divides `D0` by `last_read`. Sets CCR flags. Sets quotient & remainder in `D0`. Sets `state.micro.clocks_remaining = 76 + quot_ones * 2`. Advances to Step 2. |
| **2** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Sits in idle bus countdown for `clocks_remaining` (freeing Chip RAM for Blitter/Copper). Once `clocks_remaining == 0`, completes opcode prefetch from `PC` and retires! |

---

### Class 11: Multi-Register Block Transfer (`MOVEM <ea>, reglist`)

#### Archetype: `MOVEM.W (A0)+, D0-D2/A0-A1` (Opcode `$4CD8 $0307` — 5 Registers)
- **Total Duration**: Exactly 32 CPU clocks (16 CCKs).
- **Result Writing**: Loads registers $D_0, D_1, D_2, A_0, A_1$ from memory. Postincrements $A_0$ by $5 \times 2 = 10$ bytes. Zero memory writes.
- **Prefetch**: Extension word provides 16-bit register mask `$0307` (`%0000_0011_0000_0111`).

| Step | Action | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FetchExtension` | 4 (2 CCKs) | Reads 16-bit mask `$0307` from `PC` into `state.micro.scratch[0]`, advancing `PC += 2`. |
| **1** | `Alu` | **0 (Instant)** | Resolves `ea_addr = A0`. Latches starting address. |
| **2** | `MovemTransfer`<br/>*(x5 iterations)* | **20 (10 CCKs)** | Loops across all set mask bits ($D_0, D_1, D_2, A_0, A_1$). Each register read takes 4 clocks (2 CCKs) from `ea_addr`, increments `ea_addr += 2`, and clears the bit. Stalls on Chip RAM DMA. |
| **3** | `PrefetchNextOpcodeAndRetire` | 4 (2 CCKs) | Commits final address write-back $A_0 = ea\_addr$. Reads next opcode from `PC`, refills pipeline, and retires! |

---

### Class 12: Group 0 Diagnostic Hardware Exception (Address Error Vector 3)

#### Archetype: Misaligned Word Access (Total Duration: Exactly 50 CPU clocks / 25 CCKs)
- **Total Duration**: Exactly 50 CPU clocks (25 CCKs).
- **Physical Silicon Stacking Order**: Interleaved hardware write sequence matching Motorola M68000 PRM Figure B-9 and MAME `state_address_error_df`.
- **Double Bus Fault**: If $SSP$ is odd during exception setup or stack writes, or if the low vector address is odd, the processor halts immediately (`self.state.halted = true`).
- **Prefetch**: Vector fetch + two-word pipeline refill from exception handler.

| Step | Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `ALU_AERR_INIT` | 2 (CCK1) | S=1, T=0, double-bus fault check on SSP, snapshots `ssp_base`, return PC, old SR, vector `$00000C`. Internal duration 4 logged. |
| **1** | `ALU_IDLE` | 2 (CCK2) | Internal execution cycle completing initial 4-clock hardware delay. |
| **2** | `AERR_PUSH_PCLO_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for return PC low word write to `SSP - 2`. |
| **3** | `AERR_PUSH_PCLO_WRITE` | 2 (CCK2) | Writes return PC low word (`source & 0xFFFF`) to `SSP - 2`. Stalls on Chip RAM DMA contention. |
| **4** | `AERR_PUSH_SR_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for Status Register write to `SSP - 6`. |
| **5** | `AERR_PUSH_SR_WRITE` | 2 (CCK2) | Writes pre-exception `SR` (`destination & 0xFFFF`) to `SSP - 6`. Stalls on Chip RAM DMA contention. |
| **6** | `AERR_PUSH_PCHI_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for return PC high word write to `SSP - 4`. |
| **7** | `AERR_PUSH_PCHI_WRITE` | 2 (CCK2) | Writes return PC high word (`(source >> 16) & 0xFFFF`) to `SSP - 4`. Stalls on Chip RAM DMA contention. |
| **8** | `AERR_PUSH_IR_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for Instruction Register write to `SSP - 8`. |
| **9** | `AERR_PUSH_IR_WRITE` | 2 (CCK2) | Writes opcode `state.ir` to `SSP - 8`. Stalls on Chip RAM DMA contention. |
| **10** | `AERR_PUSH_ADDR_LO_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for Access Address low word write to `SSP - 10`. |
| **11** | `AERR_PUSH_ADDR_LO_WRITE` | 2 (CCK2) | Writes `fault_addr & 0xFFFF` to `SSP - 10`. Stalls on Chip RAM DMA contention. |
| **12** | `AERR_PUSH_INFO_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for Internal Information Word write to `SSP - 14`. |
| **13** | `AERR_PUSH_INFO_WRITE` | 2 (CCK2) | Writes Special Status Word (`(IR & 0xFFE0) \| R/W \| FC`) to `SSP - 14`. Stalls on Chip RAM DMA contention. |
| **14** | `AERR_PUSH_ADDR_HI_IDLE` | 2 (CCK1) | Validates SSP alignment and idles bus for Access Address high word write to `SSP - 12`. |
| **15** | `AERR_PUSH_ADDR_HI_WRITE` | 2 (CCK2) | Writes `(fault_addr >> 16) & 0xFFFF` to `SSP - 12` and commits `SSP = ssp_base - 14`. |
| **16** | `READ_VECTOR_HIGH_READ` | 2 (CCK1) | Vector fetch high word read from `$00000C` into `ea_high`. |
| **17** | `BUS_READ_IDLE` | 2 (CCK2) | Vector high word read completion (bus free for Agnus DMA). |
| **18** | `READ_VECTOR_LOW_READ` | 2 (CCK1) | Vector fetch low word read from `$00000E` into `source`. |
| **19** | `READ_VECTOR_LOW_FINISH` | 2 (CCK2) | Checks target alignment (halts if double fault), sets `ea_addr = handler_address`. |
| **20** | `READ_TARGET_OPCODE_READ` | 2 (CCK1) | **Refill 1:** Reads first instruction opcode of handler from `ea_addr`. |
| **21** | `BUS_READ_IDLE` | 2 (CCK2) | Target opcode read completion (bus free for Agnus DMA). |
| **22** | `ALU_IDLE` | 2 (CCK1/2) | 2-clock internal hardware pipeline alignment delay. |
| **23** | `PREFETCH_TARGET_READ` | 2 (CCK1) | **Refill 2:** Reads second instruction word from `ea_addr + 2` into `prefetch[0]`. |
| **24** | `PREFETCH_TARGET_FINISH` | 2 (CCK2) | Arms `target_refill = true`, latches `IR = irc`, sets `PC = ea_addr + 4`, retires exception! |


---

## 8. Architectural Quality Attributes & Systems Compliance

| Requirement | Specification Implementation |
| :--- | :--- |
| **Host Mechanical Sympathy** | Static dispatch array; flat contiguous micro-steps; zero dynamic branching in inner loops; host L1d cache residency. |
| **Cached Slice Pointer Dispatch** | Active step slice pointer `state.micro.current_steps` cached upon opcode prefetch; eliminates 64K table lookups during instruction execution. |
| **Zero Runtime Allocations** | Complete microcode ROM is `const` in `.rodata`; zero heap allocation during emulation loops. |
| **Zero Cascaded Size Branches** | Transfer size specialized directly into `StepFn` handlers (`step_bus_read_byte`, `step_bus_read_word`, `step_bus_write_byte`, `step_bus_write_word`, `step_bus_write_long_high`, `step_bus_write_long_low`). |
| **Dynamic Multi-Register Transfer** | `MOVEM` dynamic register counts (0 to 16) executed cleanly via `execute_movem_transfer` sub-loop with zero heap allocation. |
| **Byte Write Strobe Fidelity** | Strict $\overline{\text{UDS}}$ / $\overline{\text{LDS}}$ parity driving: even writes preserve low byte, odd writes preserve high byte. Zero Address Error on odd byte. |
| **Control Flow & Refill Fidelity** | Exact two-word Program Space pipeline refill (`step_bus_read_target_opcode` + `step_prefetch_target_and_retire`) on JMP, JSR, RTS, and taken Bcc. |
| **Cycle-Exact Exception Stacking** | Group 0 (7-word diagnostic frame) and Group 1/2 (3-word frame) stacking modeled as cycle-exact bus writes stalling on Chip RAM DMA. |
| **Dynamic Branch Sequencing** | `step_branch_eval` handler dynamically selects between 10-clock taken and 8/12-clock not-taken micro-paths with zero heap allocation. |
| **32-Bit Write Decomposition** | Long writes cleanly decomposed into two atomic 16-bit bus write cycles (`BusWriteLongHigh` and `BusWriteLongLow`). |
| **Cycle-Exact Wait States** | Stalling pauses directly on the active bus step; wait states accumulated in increments of 1 CCK (2 clocks). ALU step never re-run. |
| **Fast-Path Non-Contended Access** | Single-comparison bypass (`addr < 0x200000`) for Fast RAM, Kickstart ROM, and Slow RAM, maximizing branch predictor efficiency. |
| **Exact-Moment Data Sampling** | Read data sampled at S6 (CCK2); writes committed at S6 (CCK2), accurately modeling Gary/Agnus bus handshakes. |
| **Self-Modifying Code (SMC)** | Zero invalidation overhead; live fetches from prefetch queue dynamically index the static table. |
| **Code Architecture & Standards** | Zero user-defined macros (`macro_rules!` forbidden); zero const-generic functions; wrapping arithmetic; strictly $\le 800$ lines per Rust file. |
