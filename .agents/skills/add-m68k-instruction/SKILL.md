---
name: add-m68k-instruction
description: >-
  Use this skill when implementing a new Motorola 68000 CPU instruction or addressing mode in the emulator. Provides a step-by-step recipe covering flat linear opcode execution, specialized compile-time addressing modes, inlined CCR calculations, CCK cycle phases, prefetch management, and SingleStepTest validation.
---

# Recipe: Implementing an M68000 CPU Instruction (Flat & Linear Execution)

Follow this specification to implement new instructions in the cycle-exact M68000 emulator core.

---

## 1. Core Architectural Philosophy: Flat, Linear Handlers

Modern host CPUs (x86_64, aarch64) feature deeply pipelined execution (14–20+ stages). Cascaded dynamic branches (`match opcode`, `match ea_mode`, `match size`) in the hot instruction dispatch loop flush the pipeline and waste 15–20 host cycles per misprediction.

Because each handler in `dispatch_table.rs` is dedicated to a specific opcode or addressing mode variant:
1. **Zero Runtime Addressing Mode Matching:** The addressing mode is known at compile time (e.g. `ai`, `pi`, `pd`, `dn`). Never invoke generic multi-mode dispatchers like `read_ea_operand` inside specialized handlers.
2. **Zero Runtime Size Matching:** The operand size (`.b`, `.w`, `.l`) is statically fixed for the handler. Write concrete `u8`, `u16`, or `u32` math directly.
3. **Inlined CCR Calculations:** Calculate Condition Code Register flags ($X, N, Z, V, C$) directly using branchless bitwise formulas for the specific operand size. Never call generic dynamic multi-size CCR functions.
4. **No Macros & No Const-Generics:** Custom macros (`macro_rules!`) and const-generic matrices (`fn op<const S: usize>`) are strictly forbidden. Handlers must be explicit, self-documenting Rust functions.
5. **Zero `#[inline]` on Opcode Handlers:** Because opcode handlers (`pub fn op_...`) are indirect function pointer targets in `DISPATCH_TABLE: [OpcodeHandler; 65536]`, indirect calls can never be inlined at the call site. Never annotate top-level opcode methods with `#[inline]` or `#[inline(always)]` (per Rule 2.8 in `AGENTS.md`). Keep inlining strictly *internal* to the method body (e.g. inlined CCR formulas and branchless arithmetic).

---

## 2. Naming Standard

Name every handler strictly according to [.agents/rules/opcode-naming.md](../../rules/opcode-naming.md):

$$\mathbf{\text{op\_}\langle\text{mnemonic}\rangle\_\langle\text{size}\rangle\_\langle\text{source}\rangle\_\langle\text{destination}\rangle}$$

- Dual-operand: `op_add_w_ai_dn` (`ADD.W (An), Dn`)
- Immediate: `op_ori_b_imm_dn` (`ORI.B #imm, Dn`), `op_ori_w_imm_sr` (`ORI #imm, SR`)
- Single-operand: `op_clr_b_dn` (`CLR.B Dn`), `op_tst_l_dn` (`TST.L Dn`)
- Control / Zero-operand: `op_nop` (`NOP`), `op_rts` (`RTS`), `op_trap` (`TRAP #<vec>`)

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

### ✅ Good (Flat, Linear, Compile-Time Specialized):
```rust
/// ADD.W (An), Dn (Opcode 0xD050 family: 1101 <Dn:3> 0 01 010 <An:3>)
pub fn op_add_w_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let an_reg = (ir & 7) as usize;
    let dn_reg = ((ir >> 9) & 7) as usize;

    match cpu.state.micro.micro_step {
        0 => {
            // Bus cycle: read 16-bit word from (An)
            let addr = cpu.state.read_a(an_reg);
            if (addr & 1) != 0 {
                return trigger_address_error(cpu, addr, true, false, bus);
            }
            cpu.initiate_bus_cycle(BusCycle::new_read(
                addr,
                BusAccessSize::Word,
                data_fc(cpu),
            ));
            StepResult::StepCompleted
        }
        1 => {
            let src = cpu.state.micro.last_read as u16;
            let dst = cpu.state.d[dn_reg] as u16;
            let res = dst.wrapping_add(src);

            // Inlined Word CCR Calculation (Zero host branches)
            let n = (res as i16) < 0;
            let z = res == 0;
            let v = ((src ^ res) & (dst ^ res) & 0x8000) != 0;
            let c = (res < dst) || (res < src);
            let x = c;

            // Commit result to Dn
            cpu.state.d[dn_reg] = (cpu.state.d[dn_reg] & 0xFFFF_0000) | (res as u32);
            cpu.state.set_flags(x, n, z, v, c);

            // Complete instruction and prefetch next opcode
            cpu.retire_instruction(bus);
            StepResult::InstructionCompleted
        }
        _ => unreachable!(),
    }
}
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

- When consuming 16/32-bit immediate or displacement extension words, consume from `cpu.consume_extension_word(bus)`.
- When execution finishes normally, call `cpu.retire_instruction(bus)` and return `StepResult::InstructionCompleted`.
- When branching or jumping, reload prefetch using `cpu.state.micro.mark_target_refill_retire(target, new_ir)` or `cpu.reload_pc_and_prefetch(target, bus)`.

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
