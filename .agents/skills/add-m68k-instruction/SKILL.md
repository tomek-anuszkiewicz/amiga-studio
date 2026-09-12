---
name: add-m68k-instruction
description: >-
  Use this skill when implementing a new Motorola 68000 CPU instruction or addressing mode in the emulator. Provides a step-by-step recipe covering flat linear opcode execution, specialized compile-time addressing modes, inlined CCR calculations, CCK cycle phases, prefetch management, and SingleStepTest validation.
---

# Recipe: Implementing an M68000 CPU Instruction (Microcode Archetype Pattern)

Follow this specification to implement new instructions in the cycle-exact M68000 emulator core.
Every instruction must adhere to the 2-clock Color Clock micro-step state machine, flat branchless dispatch, and dual-tier verification.

---

## 1. Core Architectural Philosophy: Micro-Step State Machine

Modern host CPUs (x86_64, aarch64) feature deeply pipelined execution (14–20+ stages). Cascaded dynamic branches (`match opcode`, `match ea_mode`, `match size`) in the hot instruction dispatch loop flush the pipeline and waste 15–20 host cycles per misprediction.

The M68000 core models execution via the **Cycle-Exact Micro-Step State Machine**:
1. **The 2-Clock Color Clock Slice**: $1\ \text{MicroStep} = 1\ \text{Color Clock / CCK} = 2\ \text{CPU clocks}$.
   - $1\ \text{standard M68000 bus cycle} = 4\ \text{CPU clocks} = 2\ \text{CCK slices}\ (\text{CCK1} + \text{CCK2})$.
   - CCK1 initiates bus transfers, sets addresses, and computes effective addresses.
   - CCK2 completes bus latches, writes data, or advances prefetch queues.
2. **Stateless `MicroStep` Descriptor**:
   ```rust
   pub struct MicroStep {
       pub bus_fn: Option<BusFn>,   // Bus transfer / CCK action (None if idle or pure ALU)
       pub alu_fn: Option<AluFn>,     // Pure internal ALU operation (None if pure bus cycle)
       pub base_clocks: u8,           // Base clocks (typically 2 for CCK, 4 for internal 4-clock delays)
   }
   ```
3. **Parametric, Bus-Free ALU Functions (`AluFn`)**:
   ALU calculations do **not** take `MemoryBus`. By the time the ALU callback runs, all operands have already arrived in `CpuState`:
   - Memory read operand: `state.micro.source` (or `state.micro.destination` for RMW destination).
   - Immediate operand: `state.prefetch[0]` (low byte/word) or `state.micro.source` (long).
   - Quick immediate: passed directly as `reg_src: u8`.
   - Data / Address registers: read via accessors `state.d_*()` or `state.read_a()`.
   `AluFn` signature: `fn(&mut CpuState, reg_src: u8, reg_dst: u8)`.
4. **65,536 Static Descriptor Table (`OPCODE_DESCRIPTOR_TABLE`)**:
   Every 16-bit opcode maps to an `OpcodeDescriptor { steps: &'static [MicroStep], reg_src: u8, reg_dst: u8 }`.
   At instruction retirement, `state.micro.current_steps` caches the slice pointer, eliminating runtime table lookups during stepping ticks.
5. **Strict Prohibitions**:
   - Zero custom macros (`macro_rules!` is strictly forbidden).
   - Zero const-generic matrices (`fn op<const S: usize>` is strictly forbidden).
   - **Strict Flat Instruction Hierarchy**: Every instruction must reside in a single flat file `crates/m68000/src/instructions/<mnemonic>.rs`. Subdirectories under `instructions/` are strictly forbidden. If an instruction exceeds 800 lines due to exhaustive addressing modes (e.g. `add.rs`, `sub.rs`, `and.rs`), it must remain a single flat file and be registered under `LINE_COUNT_EXCEPTIONS` in `test_architecture_rules.rs`. Other non-instruction source files $\le 800$ lines.

---

## 2. Naming Standards

Name ALU handlers and micro-step sequences consistently:
- Leaf ALU function: `<mnemonic>_<size>` (e.g. `sub_b`, `sub_w`, `sub_l`, `subx_w`)
- ALU callback: `alu_<mnemonic>_<size>_<source>_<destination>` (e.g. `alu_sub_w_mem_dn`, `alu_subq_b_imm_mem`)
- Step slice: `STEPS_<MNEMONIC>_<SIZE>_<SOURCE>_<DESTINATION>` (e.g. `STEPS_SUB_W_AI_DN`, `STEPS_SUB_B_DN_PI`)
- Decoder function: `pub const fn decode_<mnemonic>_steps(...) -> Option<&'static [MicroStep]>`

### 2.1 Mandatory Idle Micro-Step Taxonomy & Prohibition of Anonymous Idle Structs

Every micro-step where the physical memory bus does not perform an active read/write transfer or assert address strobes must explicitly feature `IDLE` in its identifier:

| Idle Category | Clocks | Canonical Constant | Usage Description |
| :--- | :---: | :--- | :--- |
| **Bus Read Idle** | 2 | `common::BUS_READ_IDLE` | CCK2 idle completion of operand reads, opcode prefetch, target opcode reads, and SR/CCR prefetch queue refills. |
| **Bus Write Idle** | 2 | `common::BUS_WRITE_IDLE` | CCK1 write setup phase; address/data latched internally, external bus free for Agnus DMA. |
| **Internal ALU Idle** | 2 | `common::ALU_IDLE` | 2-clock internal execution delay with zero external bus activity (e.g. `CMPA.W`, `Bcc`, `BSET`, `CLR.L`). |
| **Internal Execution Idle (4-clock)** | 4 | `common::ALU_IDLE_4CLK` | 4-clock internal execution delay (e.g. 32-bit register arithmetic `ADD.L Dn, Dn`, `SUBA.L`). |
| **Internal Exception Idle (8-clock)** | 8 | `common::ALU_IDLE_8CLK` | 8-clock internal exception processing delay (`CHK` trap, `DIV` divide-by-zero). |
| **Reset Idle (128-clock)** | 128 | `common::ALU_IDLE_128CLK` | 128-clock external bus idle countdown for `RESET`. |
| **Stack Push Setup Idle** | 2 | `common::PUSH_STACK_HIGH_IDLE`<br>`common::EXCEPTION_PUSH_*_IDLE`<br>`common::AERR_PUSH_*_IDLE` | CCK1 SP decrement and alignment check; bus idle for DMA prior to CCK2 write. |

> [!IMPORTANT]
> **Strict Prohibition of Anonymous Idle Structs & Deprecated Aliases**:
> - Never author anonymous idle structs like `MicroStep { bus_fn: None, alu_fn: None, base_clocks: ... }`. Always reuse canonical constants from `crate::micro::common::*`.
> - Never use deprecated legacy aliases (`READ_WORD_FINISH`, `PREFETCH_NEXT_RETIRE`, `REFILL_FIRST_FINISH`, `REFILL_SECOND_FINISH`, etc.). Always use `common::BUS_READ_IDLE` or `common::ALU_IDLE*`.
> - Actively enforced by the automated test `test_idle_microstep_naming_and_prohibition_of_anonymous_idle_structs`.

---

## 3. Step-by-Step Implementation Pattern

### Step 3.1: Leaf ALU Arithmetic Functions
Annotated with `#[inline(always)]`, branchless, wrapping arithmetic, and explicit CCR flag generation:
```rust
#[inline(always)]
pub fn sub_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}
```

### Step 3.2: Parametric ALU Callbacks (`AluFn`)
Callbacks read operands from `state.micro.source`, `state.prefetch[0]`, or registers, invoke leaf functions, and write results back to registers or `state.micro.destination`:
```rust
/// SUB.W <ea>, Dn: source is in state.micro.source, destination in Dn
pub fn alu_sub_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

/// SUB.W Dn, <ea> (RMW Class 0): source in Dn, destination in state.micro.destination
pub fn alu_sub_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}
```

> [!NOTE]
> `AluFn` callbacks (`pub fn alu_...`), `StepFn` handlers, and decoders (`pub const fn decode_...`) are stored in tables or invoked via function pointers. **Do NOT annotate them with `#[inline]`** (indirect dispatch targets cannot be inlined at call sites).

### Step 3.2b: Cold Exception & Trap Triggers (`#[inline(never)]`)
When an instruction can fault, trap, or trigger an exception (e.g. `DIVU`/`DIVS` divide-by-zero, `CHK` boundary trap, `TRAPV`, Address Error, or Privilege Violation):
- All dedicated trap setup or trigger functions (e.g. `trigger_chk_trap`, `trigger_divide_by_zero`, `trigger_address_error`) **MUST be annotated with `#[inline(never)]`**.
- This guarantees that LLVM keeps cold exception stack frame generation out of the CPU instruction cache (L1i), preserving density in the hot execution path.
```rust
#[inline(never)]
pub fn trigger_chk_trap(state: &mut CpuState) {
    let old_sr = state.sr;
    state.set_supervisor(true);
    state.sr &= !0x8000;
    ...
}
```

### Step 3.3: Composing Micro-Step Slices using `common::*` and `ea::*`
Combine specialized bus primitives and ALU callbacks into immutable static arrays:

#### 1. Register-to-Register (e.g. `SUB.W Dy, Dx` — 4 clocks / 2 CCKs):
```rust
pub static STEPS_SUB_W_DN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
```

#### 2. Memory-to-Register (e.g. `SUB.W (An), Dn` — 8 clocks / 4 CCKs):
```rust
pub static STEPS_SUB_W_AI_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
```

#### 3. Register-to-Memory Store (e.g. `MOVE.W Dn, (An)` — 8 clocks / 4 CCKs):
Fused ALU callback pattern: source extraction, CCR evaluation, and destination EA are combined into `alu_move_w_dn_dst_ai` attached to the first CCK write setup step:
```rust
pub static STEPS_MOVE_W_DN_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_move_w_dn_dst_ai),
        base_clocks: 2,
    },
    common::WRITE_DST_WORD,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
```

#### 4. Register-to-Memory RMW Class 0 (e.g. `SUB.W Dn, (An)` — 12 clocks / 6 CCKs):
Notice the M68000 RMW pipeline: read operand $\to$ prefetch next opcode to `irc` while executing ALU $\to$ write result and retire:
```rust
pub static STEPS_SUB_W_DN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
```

#### 5. Dual Staging Registers ($X_1, X_2$) for Dual-Memory Operations (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`):
For dual-memory instructions operating on two memory addresses (source $Ay$ and destination $Ax$), always utilize the dedicated staging registers `state.micro.addr1` and `state.micro.addr2`:

##### A. Byte Dual-Memory Operations (`CMPM.b`, `ABCD`, `SBCD`, `ADDX.b`, `SUBX.b`):
- **Upfront Calculation Safe:** Byte memory accesses **never trigger Address Errors**.
- Calculate both `addr1` and `addr2` in a single upfront ALU step (e.g. `ea_calc_dual_pi_b` or `ea_calc_dual_pd_b`), eliminating all intermediate ALU steps:
```rust
pub static STEPS_ABCD_PD_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dual_pd_b),
        base_clocks: 2,
    },
    common::READ_ADDR1_BYTE,
    common::BUS_READ_IDLE,
    common::READ_ADDR2_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_abcd_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_BYTE,
];
```

##### B. Word Dual-Memory Operations (`CMPM.w`, `ADDX.w`, `SUBX.w`):
- **Address Error Invariance Mandate:** If $Ay$ is unaligned (odd), the source read immediately triggers an Address Error exception. On real 68000 silicon, $Ax$ (or SSP/USP if A7) was **never touched** and must remain unchanged in the saved exception state.
- **Rule:** Do NOT calculate $Ax$ in Step 0. Calculate $Ay$ (`addr1`) in Step 0, and fuse $Ax$ (`addr2`) calculation into the **CCK2 idle phase of the source read**:
```rust
pub static STEPS_ADDX_W_PD_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_ADDR1_WORD,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w), // Fused in CCK2 of src read: only runs if src was aligned!
        base_clocks: 2,
    },
    common::READ_ADDR2_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_addx_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_WORD,
];
```

##### C. Long Dual-Memory Operations (`CMPM.l`, `ADDX.l`, `SUBX.l`):
- **Predecrement Long Split:** 68000 decrements $An$ by 2 first for the low-word read (checking for Address Error), then by another 2 for the high-word read (`ea_calc_src_pd_l_split` / `latch_src_lo_and_read_src_hi`).
- **Direct Write-Back via `addr2`:** Stage the base destination address in `addr2`. Use direct staged writes `WRITE_ADDR2_PD_LONG_LOW` (writes `destination & 0xFFFF` to `addr2 + 2`) and `WRITE_ADDR2_PD_LONG_HIGH` (writes `(destination >> 16) & 0xFFFF` to `addr2`).
- **Strict Prohibition:** Never juggle temporary pointers in `scratch[0..2]`, never shift `destination >>= 16`, and never use helper functions that mutate write pointers (`set_write_hi`). Direct writes eliminate all scratch register overhead.
```rust
pub static STEPS_ADDX_L_PD_PD: [MicroStep; 15] = [
    MicroStep { bus_fn: None, alu_fn: Some(ea::ea_calc_src_pd_l_split), base_clocks: 2 },
    common::READ_SRC_WORD,
    MicroStep { bus_fn: None, alu_fn: Some(ea::latch_src_lo_and_read_src_hi), base_clocks: 2 },
    common::READ_SRC_SPLIT_HIGH,
    MicroStep { bus_fn: None, alu_fn: Some(ea::ea_calc_dst_pd_l_split), base_clocks: 2 },
    common::READ_DST_WORD,
    MicroStep { bus_fn: None, alu_fn: Some(ea::latch_dst_lo_and_read_dst_hi), base_clocks: 2 },
    common::READ_DST_SPLIT_HIGH,
    MicroStep { bus_fn: None, alu_fn: Some(alu_addx_l_mem), base_clocks: 2 },
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_PD_LONG_LOW,  // Writes low word to addr2 + 2
    common::PREFETCH_IRC_READ,
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_ADDR2_PD_LONG_HIGH, // Writes high word to addr2
];
```


### Step 3.4: Decoder Function
Implement a `const fn` decoder that maps addressing mode, size, and direction to the static slice:
```rust
pub const fn decode_sub_steps(
    dir: u8,
    size: u8,
    mode: u8,
    reg: u8,
) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        // <ea>, Dn
        match size {
            0 => match mode {
                0 => Some(&STEPS_SUB_B_DN_DN),
                2 => Some(&STEPS_SUB_B_AI_DN),
                // ... other modes
                _ => None,
            },
            // ... word, long
            _ => None,
        }
    } else {
        // Dn, <ea>
        // ...
    }
}
```

### Step 3.5: Registration in `micro/dispatch_table.rs`
Populate `OPCODE_DESCRIPTOR_TABLE` inside `build_opcode_descriptor_table()`:
```rust
let mut op = 0x9000usize;
while op <= 0x9FFF {
    let ir = op as u16;
    let reg_d = ((ir >> 9) & 7) as u8;
    let dir = ((ir >> 8) & 1) as u8;
    let size = ((ir >> 6) & 3) as u8;
    let mode = ((ir >> 3) & 7) as u8;
    let reg = (ir & 7) as u8;

    if size < 3 {
        let is_subx = dir == 1 && (mode == 0 || mode == 1);
        if is_subx {
            // SUBX Dy, Dx or -(Ay), -(Ax)
            if let Some(steps) = crate::instructions::subx::decode_subx_steps(mode == 1, size) {
                table[op] = OpcodeDescriptor { steps, reg_src: reg, reg_dst: reg_d };
            }
        } else if let Some(steps) = crate::instructions::sub::decode_sub_steps(dir, size, mode, reg) {
            let (reg_src, reg_dst) = if dir == 0 { (reg, reg_d) } else { (reg_d, reg) };
            table[op] = OpcodeDescriptor { steps, reg_src, reg_dst };
        }
    } else {
        // SUBA <ea>, An
        let is_long = dir != 0;
        if let Some(steps) = crate::instructions::suba::decode_suba_steps(is_long, mode, reg) {
            table[op] = OpcodeDescriptor { steps, reg_src: reg, reg_dst: reg_d };
        }
    }
    op += 1;
}
```

---

## 4. Inlined Condition Code (CCR) Formulas

Always use branchless helpers in `CpuState`. Any flag not updated must be strictly preserved.

### Addition (`ADD`, `ADDI`, `ADDQ`, `NEG`)
```rust
let (res, c) = d.overflowing_add(s);
let v = ((!(s ^ d) & (d ^ res)) & MSB) != 0;
let n = (res & MSB) != 0;
let z = res == 0;
state.set_ccr_xnzvc(c, n, z, v, c);
```

### Subtraction (`SUB`, `SUBI`, `SUBQ`)
```rust
let (res, c) = d.overflowing_sub(s);
let v = (((s ^ d) & (d ^ res)) & MSB) != 0;
let n = (res & MSB) != 0;
let z = res == 0;
state.set_ccr_xnzvc(c, n, z, v, c);
```

### Extended Arithmetic with Borrow (`SUBX`, `NEGX`)
In multi-precision subtraction, `Z` is cleared if result is non-zero, but preserved if result is zero:
```rust
let x = if state.get_x() { 1 } else { 0 };
let (res1, c1) = d.overflowing_sub(s);
let (res, c2) = res1.overflowing_sub(x);
let c = c1 || c2;
let v = (((s ^ d) & (d ^ res)) & MSB) != 0; // Uses original s
let n = (res & MSB) != 0;
let z = if res != 0 { false } else { state.get_z() };
state.set_ccr_xnzvc(c, n, z, v, c);
```

### Comparison (`CMP`, `CMPI`, `CMPA`, `CMPM`, `TST`, `CHK`)
Calculates subtraction flags, updates $N, Z, V, C$, but **strictly preserves $X$**:
```rust
let (res, c) = d.overflowing_sub(s);
let v = (((s ^ d) & (d ^ res)) & MSB) != 0;
let n = (res & MSB) != 0;
let z = res == 0;
state.set_ccr_nzvc(n, z, v, c); // Preserves X!
```

### Address Operations (`ADDA`, `SUBA`, `MOVEA`, `LEA`, `ADDQ/SUBQ to An`)
Condition codes are **completely unaffected**.

### Bitwise Logic (`AND`, `OR`, `EOR`, `NOT`, `ORI`, `ANDI`, `EORI`) & `MOVE`
Forces $V=0, C=0$, updates $N, Z$, preserves $X$:
```rust
let n = (res & MSB) != 0;
let z = res == 0;
state.set_ccr_nz_clear_vc(n, z);
```

### Single Bit Operations (`BTST`, `BSET`, `BCLR`, `BCHG`)
```rust
state.set_ccr_z_only(bit == 0); // Preserves X, N, V, C
```

---

## 5. Register Access & Stack Pointer Banking

Registers in `CpuState` are strictly private fields. Always use size-specific accessor methods annotated with `#[inline(always)]`:
- **Data Registers ($D_0$–$D_7$)**:
  - Byte: `state.d_byte(i) -> u8` / `state.set_d_byte(i, val)` (preserves upper bits 8..31).
  - Word: `state.d_word(i) -> u16` / `state.set_d_word(i, val)` (preserves upper bits 16..31).
  - Long: `state.d_long(i) -> u32` / `state.set_d_long(i, val)`.
- **Address Registers ($A_0$–$A_7$)**:
  - Word: `state.a_word(i) -> u16` / `state.set_a_word(i, val)` (automatically sign-extends 16-bit to 32-bit).
  - Long: `state.read_a(i) -> u32` / `state.write_a(i, val)` (automatically synchronizes $A_7$ USP/SSP).
  - Byte operations on $A_n$ do not exist in the M68000 ISA.

---

## 6. Module Organization & 800-Line Limit Compliance

Per `AGENTS.md` Rule 7, no Rust source file under `crates/*/src/` may exceed 800 lines:
- For medium instructions ($\le 800$ lines, e.g. `suba.rs`, `subi.rs`, `subq.rs`, `subx.rs`): keep as a single cohesive `.rs` file.
- For large instructions with bidirectional addressing modes (e.g. `sub.rs` which would be ~1,200 lines):
  Split into a clean submodule directory:
  - `instructions/sub/mod.rs` (ALU functions, callbacks, decoder, re-exports)
  - `instructions/sub/ea_dn.rs` (`<ea>, Dn` slices across sizes)
  - `instructions/sub/dn_ea.rs` (`Dn, <ea>` alterable memory slices across sizes)
  Each submodule is ~300–550 lines, well within the sweet spot.

---

## 7. Dual-Tier Verification Gate

Every implemented or reintroduced instruction must pass the mandatory dual-tier gate:

### Tier 1 — SingleStepTest Vector Verification
Add dual-suite tests (MAME + Tom Harte) in `crates/test_runner/tests/test_singlestep.rs`:
```rust
#[test]
fn test_sub_w() {
    run_dual_test("SUB.w", DEFAULT_SAMPLE_LIMIT);
}
```
Run targeted tests:
```powershell
cargo test -p test_runner --test test_singlestep -- test_sub
```

### Tier 2 — Cartesian DMA Contention Verification
Integrate the instruction into `crates/test_runner/tests/test_dma_cartesian.rs`:
```rust
#[test]
fn test_dma_cartesian_sub() {
    run_cartesian_for_opcodes(&["SUB.b", "SUB.w", "SUB.l", "SUBA.w", "SUBA.l"]);
}
```
Run Cartesian stress tests (validating cycle invariance $C = C_0 + 2 \times \text{wait\_states}$, Fast RAM immunity, and state invariance across the entire $2^k \times 2^M$ space):
```powershell
cargo test -p test_runner --test test_dma_cartesian -- test_dma_cartesian_sub
```

### Architecture, Inlining & Formatting Gate
Always verify before declaring complete:
```powershell
cargo fmt --all -- --check
cargo test -p test_runner --test test_architecture_rules
```
> [!IMPORTANT]
> The automated test `test_inlining_guidelines_compliance` automatically validates that all leaf ALU functions have `#[inline(always)]`, all cold exception/trap triggers have `#[inline(never)]`, and CCR mutators have `#[inline(always)]`. If an inlining attribute is missing or misconfigured, `test_architecture_rules` will fail immediately.

When completing a milestone or batch, run the full, exhaustive SingleStepTests without limits:
```powershell
$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
```
