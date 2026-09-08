---
name: add-m68k-instruction
description: >-
  Use this skill when implementing a new Motorola 68000 CPU instruction or addressing mode in the emulator. Provides a step-by-step recipe covering flat linear opcode execution, specialized compile-time addressing modes, inlined CCR calculations, CCK cycle phases, prefetch management, and SingleStepTest validation.
---

# Recipe: Implementing an M68000 CPU Instruction (Flat & Linear Execution)

Follow this specification to implement new instructions in the cycle-exact M68000 emulator core.

---

## 1. Core Architectural Philosophy: Micro-Step State Machine

Modern host CPUs (x86_64, aarch64) feature deeply pipelined execution (14–20+ stages). Cascaded dynamic branches (`match opcode`, `match ea_mode`, `match size`) in the hot instruction dispatch loop flush the pipeline and waste 15–20 host cycles per misprediction.

The M68000 core models execution via the **Cycle-Exact Micro-Step State Machine**:
1. **The 65,536 Static Descriptor Table (`OPCODE_DESCRIPTOR_TABLE`)**: Every 16-bit opcode maps to an `OpcodeDescriptor` referencing an immutable slice of atomic `MicroStep`s and pre-decoded register indices (`reg_src`, `reg_dst`).
2. **Cached Slice Pointer Dispatch (`current_steps`)**: Upon opcode prefetch/retirement, `state.micro.current_steps` caches the slice pointer, completely eliminating 65,536-entry table lookups during execution ticks.
3. **Parametric, Bus-Free ALU Functions (`AluFn`)**: ALU calculations do **not** take `MemoryBus`. By the time the ALU executes, all operands have arrived in `CpuState` (`prefetch[0]`, `last_read`, or registers). `AluFn` has signature `fn(&mut CpuState, reg_src: u8, reg_dst: u8)`.
4. **Specialized Atomic Micro-Actions**: Transfer size is specialized directly into `MicroAction` (`BusReadByte`, `BusReadWord`, `BusWriteByte`, `BusWriteWord`, `BusWriteLongHigh`, etc.), eliminating all runtime size branches in the hot CCK loop.
5. **No Macros & No Const-Generics**: Custom macros (`macro_rules!`) and const-generic matrices (`fn op<const S: usize>`) are strictly forbidden.

---

## 2. Naming Standard

Name ALU handlers and micro-step sequences consistently:
- ALU callback: `alu_<mnemonic>_<size>_<source>_<destination>` (e.g. `alu_add_w_mem_dn`, `alu_ori_b_imm_dn`)
- Step slice: `STEPS_<MNEMONIC>_<SIZE>_<SOURCE>_<DESTINATION>` (e.g. `STEPS_ADD_W_AI_DN`)

---

## 3. Reference Implementation Pattern

### ❌ Bad (Cascaded Dynamic Branches & Generic Helpers):
```rust
// ANTI-PATTERN: Evaluates multiple dynamic branches on size and addressing mode at runtime!
pub fn op_add_bad(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let size = match (cpu.state.ir >> 6) & 3 { ... };
    let ea_mode = decode_ea_index(...);
    let val = read_ea_operand_generic(cpu, bus, size, ea_mode)?; // dynamic match!
    let res = calculate_add_generic(val, size);                 // dynamic match!
    update_ccr_generic(&mut cpu.state, res, size);              // dynamic match!
    ...
}
```

### ✅ Good (Micro-Step State Machine Architecture):

#### 1. Parametric ALU Callback (`AluFn`)
```rust
/// ADD.W memory operand into Dn
pub fn alu_add_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.last_read;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d, true); // Inlined branchless CCR calculation
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}
```

#### 2. Immutable Micro-Step Sequence
```rust
/// ADD.W (An), Dn (Opcode family 0xD050):
/// Step 0: Read 16-bit word from (An) into last_read (4 clocks / 2 CCKs)
/// Step 1: Execute ALU add + prefetch next opcode and retire (4 clocks / 2 CCKs)
pub static STEPS_ADD_W_AI_DN: [MicroStep; 2] = [
    MicroStep {
        action: MicroAction::BusReadWord,
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 4,
        flags: flags::READ | flags::DATA_SPACE,
    },
    MicroStep {
        action: MicroAction::PrefetchNextOpcodeAndRetire,
        alu_fn: Some(alu_add_w_mem_dn),
        base_clocks: 4,
        flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE,
    },
];
```

#### 3. Opcode Descriptor Registration
```rust
table[opcode as usize] = OpcodeDescriptor::new(&STEPS_ADD_W_AI_DN, an_reg, dn_reg);
```

---

## 4. Inlined Condition Code (CCR) Formulas

Always use the branchless `set_ccr_*` helper methods in `CpuState`. Any flag not specified in the method name is strictly preserved.

### Addition (`ADD`, `ADDI`, `ADDQ`, `NEG`)
```rust
// Word (.W) Example:
let res = dst.wrapping_add(src);
let n = (res as i16) < 0;
let z = res == 0;
let v = ((src ^ res) & (dst ^ res) & 0x8000) != 0;
let c = (res < dst) || (res < src);
cpu.state.set_ccr_xnzvc(c, n, z, v, c); // Updates X, N, Z, V, C
```

### Subtraction (`SUB`, `SUBI`, `SUBQ`)
```rust
// Word (.W) Example: res = dst - src
let res = dst.wrapping_sub(src);
let n = (res as i16) < 0;
let z = res == 0;
let v = ((src ^ dst) & (res ^ dst) & 0x8000) != 0;
let c = dst < src;
cpu.state.set_ccr_xnzvc(c, n, z, v, c); // Updates X, N, Z, V, C
```

### Comparison (`CMP`, `CMPI`, `CMPA`, `TST`, `CHK`)
```rust
// Word (.W) Example: res = dst - src
let res = dst.wrapping_sub(src);
let n = (res as i16) < 0;
let z = res == 0;
let v = ((src ^ dst) & (res ^ dst) & 0x8000) != 0;
let c = dst < src;
cpu.state.set_ccr_nzvc(n, z, v, c); // Updates N, Z, V, C; leaves X untouched!
```

### Bitwise Logic (`AND`, `OR`, `EOR`, `NOT`, `ORI`, `ANDI`, `EORI`) & `MOVE`
```rust
// Word (.W) Example:
let res = dst & src; // or dst | src, !src, etc.
let n = (res as i16) < 0;
let z = res == 0;
cpu.state.set_ccr_nz_clear_vc(n, z); // Sets N and Z, forces V=0 and C=0; leaves X untouched!
```

### Single Bit Operations (`BTST`, `BSET`, `BCLR`, `BCHG`)
```rust
cpu.state.set_ccr_z_only(bit == 0); // Updates Z only; leaves X, N, V, C untouched!
```

---

## 5. Register Access & Stack Pointer Banking

Data and address registers (`d` and `a`) in `CpuState` are **strictly private** fields to prevent accidental direct mutation and guarantee architectural invariants.

### Data Registers ($D_0$–$D_7$)
Always use size-specific accessor methods annotated with `#[inline(always)]`:
- **Byte (.B)**: `cpu.state.d_byte(idx) -> u8` and `cpu.state.set_d_byte(idx, val: u8)` (preserves upper bits 8–31).
- **Word (.W)**: `cpu.state.d_word(idx) -> u16` and `cpu.state.set_d_word(idx, val: u16)` (preserves upper bits 16–31).
- **Long (.L)**: `cpu.state.d_long(idx) -> u32` and `cpu.state.set_d_long(idx, val: u32)`.

### Address Registers ($A_0$–$A_7$)
- **Word (.W)**: `cpu.state.a_word(idx) -> u16` and `cpu.state.set_a_word(idx, val: u16)` (automatically sign-extends 16-bit word to 32 bits into $A_n$).
- **Long (.L)**: `cpu.state.a_long(idx) -> u32` and `cpu.state.set_a_long(idx, val: u32)` (automatically handles $A_7$ stack pointer bank synchronization if $idx == 7$).
- **Byte (.B) Access Forbidden**: In the Motorola 68000 ISA, byte operations on address registers do not exist. There are no byte accessors on $A_n$.
- Helper shortcuts `read_a(idx)` and `write_a(idx, val)` are equivalent to `a_long(idx)` and `set_a_long(idx, val)`.
- When privilege changes, call `cpu.state.set_supervisor(bool)` or `cpu.state.set_sr(new_sr)`.

---

## 6. Prefetch & Instruction Retirement

- **Extension Fetching**: Extension words (immediates, displacements, brief index words) are fetched via `MicroAction::FetchExtension`, optionally invoking an address calculation callback (`ea_calc_*`).
- **Standard Retirement**: Sequential instructions conclude with `MicroAction::PrefetchNextOpcodeAndRetire`, which reads the next opcode into `last_read`, updates `ir = prefetch[0]`, `prefetch[0] = last_read`, advances `pc += 2`, and caches the next opcode's `current_steps`.
- **Class 0 RMW Retirement**: Memory read-modify-write instructions prefetch the next opcode into `scratch_prefetch` via `MicroAction::BusPrefetchToScratch`, then commit the write cycle and retire via `MicroAction::BusWriteWordAndRetire` (or `BusWriteByteAndRetire` / `BusWriteLongLowAndRetire`).
- **Pipeline Refills (JMP, JSR, RTS, Taken Bcc)**: Flushes the prefetch queue and performs a two-word refill directly from the target address via `MicroAction::BusReadTargetOpcode` followed by `MicroAction::PrefetchTargetAndRetire`.

---

## 7. SingleStepTest Verification

Validate the instruction against the official test suite:
1. Locate vector in `ref_src/SingleStepTests-m68000/v1/<MNEMONIC>.<size>.json` or Tom Harte suite.
2. Run single-step verification:
   ```powershell
   cargo test -p test_runner --test test_singlestep
   ```
3. Run architecture rules verification:
   ```powershell
   cargo test -p test_runner --test test_architecture_rules
   ```
4. If failures occur, activate `m68k-singlestep-test` skill.
