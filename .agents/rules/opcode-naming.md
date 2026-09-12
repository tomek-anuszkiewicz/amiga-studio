---
trigger: model_decision
description: M68000 opcode naming conventions, canonical IDLE micro-step constants, and dual staging registers (addr1/addr2).
---

# M68000 Opcode Handler Naming & Bitfield Decoding Rules

This rule enforces total naming consistency and architectural alignment across the 65,536-entry static descriptor table (`crates/m68000/src/micro/dispatch_table.rs`) and all per-mnemonic instruction execution modules in `crates/m68000`.

---

## 1. Canonical Naming Formula

All opcode handler function identifiers follow Motorola's universal **`SOURCE, DESTINATION`** syntax order:

$$\mathbf{\text{op\_}\langle\text{mnemonic}\rangle\_\langle\text{size}\rangle\_\langle\text{source}\rangle\_\langle\text{destination}\rangle}$$

Where:
- **`op_`**: Standard mandatory prefix for all opcode execution handlers.
- **`<mnemonic>`**: Lowercase M68000 instruction mnemonic (`add`, `adda`, `addi`, `sub`, `move`, `cmp`, `ori`, `andi`, `eor`, `btst`, `asl`, etc.).
- **`<size>`**:
  - `b`: Byte (8-bit)
  - `w`: Word (16-bit)
  - `l`: Long (32-bit)
  - *(Omitted for instructions without size variants, e.g. `nop`, `rts`, `trap`, `jmp`, `jsr`)*.
- **`<source>` & `<destination>`**: Addressing mode or register tokens following Motorola convention (`SOURCE, DESTINATION`).
*(Hexadecimal opcode suffixes are omitted because mnemonic, size, source mode, and destination mode provide 100% unique identification across all 65,536 opcodes, eliminating opcode-number contradictions across variable register fields $D_0..D_7$ and $A_0..A_7$)*.

---

## 2. Standard Addressing Mode Vocabulary

To prevent naming divergence across thousands of generated handlers, only the following canonical tokens are permitted:

| Token | M68000 Addressing Mode | Syntax | Description |
| :--- | :--- | :---: | :--- |
| **`dn`** | Data Register Direct | `Dn` | Generic Data Register (`D0`–`D7`) |
| **`an`** | Address Register Direct | `An` | Generic Address Register (`A0`–`A7` / SP) |
| **`d0`..`d7`** | Fixed Data Register | `D0`..`D7` | Used when opcode specifies a fixed data register |
| **`a0`..`a7`** | Fixed Address Register | `A0`..`A7` | Used when opcode specifies a fixed address register |
| **`ai`** | Address Register Indirect | `(An)` | Indirect memory access |
| **`pi`** | Address Indirect Postincrement | `(An)+` | Memory read/write with postincrement |
| **`pd`** | Address Indirect Predecrement | `-(An)` | Memory read/write with predecrement |
| **`disp`** | Address Indirect with Displacement | `(d16, An)` | 16-bit signed displacement |
| **`idx`** | Address Indirect with Index | `(d8, An, Xn)` | Brief extension word with 8-bit displacement + index |
| **`absw`** | Absolute Short | `(xxx).W` | 16-bit sign-extended absolute address |
| **`absl`** | Absolute Long | `(xxx).L` | Full 32-bit (24-bit physical) absolute address |
| **`pcdisp`** | PC with Displacement | `(d16, PC)` | Program Counter relative with 16-bit displacement |
| **`pcidx`** | PC with Index | `(d8, PC, Xn)` | Program Counter relative with index |
| **`imm`** | Immediate Data | `#<data>` | Value loaded from immediate extension words |
| **`sr`** | Status Register | `SR` | 16-bit system status register |
| **`ccr`** | Condition Code Register | `CCR` | Low byte of Status Register |
| **`usp`** | User Stack Pointer | `USP` | Privileged User Stack Pointer |

---

## 3. Naming Patterns by Instruction Arity

1. **Dual-Operand Instructions (`ADD`, `SUB`, `MOVE`, `CMP`, `AND`, `OR`, `EOR`)**:
   - `op_<mnemonic>_<size>_<source>_<destination>`
   - `op_add_b_dn_dn` $\rightarrow$ `ADD.B Dm, Dn`
   - `op_add_w_ai_dn` $\rightarrow$ `ADD.W (An), Dn`
   - `op_add_l_dn_pd` $\rightarrow$ `ADD.L Dn, -(An)`
   - `op_adda_w_dn_an` $\rightarrow$ `ADDA.W Dn, An`
   - `op_move_w_ai_dn` $\rightarrow$ `MOVE.W (An), Dn`

2. **Immediate Instructions (`ADDI`, `SUBI`, `ORI`, `ANDI`, `EORI`, `CMPI`)**:
   - `op_<mnemonic>_<size>_imm_<destination>`
   - `op_ori_b_imm_dn` $\rightarrow$ `ORI.B #imm, Dn`
   - `op_ori_w_imm_sr` $\rightarrow$ `ORI #imm, SR`
   - `op_ori_b_imm_ccr` $\rightarrow$ `ORI #imm, CCR`
   - `op_andi_w_imm_sr` $\rightarrow$ `ANDI #imm, SR`

3. **Single-Operand Instructions (`CLR`, `NEG`, `NOT`, `TST`, `SWAP`)**:
   - `op_<mnemonic>_<size>_<destination>`
   - `op_clr_b_dn` $\rightarrow$ `CLR.B Dn`
   - `op_clr_w_ai` $\rightarrow$ `CLR.W (An)`
   - `op_tst_l_dn` $\rightarrow$ `TST.L Dn`

4. **Zero-Operand & Control Instructions (`NOP`, `RTS`, `RTE`, `TRAP`, `JMP`, `JSR`)**:
   - `op_<mnemonic>` or `op_<mnemonic>_<target>`
   - `op_nop` $\rightarrow$ `NOP`
   - `op_rts` $\rightarrow$ `RTS`
   - `op_rte` $\rightarrow$ `RTE`
   - `op_trap` $\rightarrow$ `TRAP #<vector>`
   - `op_jsr_ai` $\rightarrow$ `JSR (An)`
   - `op_jmp_ai` $\rightarrow$ `JMP (An)`

---

## 4. M68000 Opcode Bitfield Decoding Reference

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

### A. Major Nibble Table (Bits 15–12)
- **`$0`**: Bit Manipulation (`BTST`, `BCHG`, `BCLR`, `BSET`), `MOVEP`, Immediate (`ORI`, `ANDI`, `SUBI`, `ADDI`, `EORI`, `CMPI`)
- **`$1`**: `MOVE.B`
- **`$2`**: `MOVE.L` / `MOVEA.L`
- **`$3`**: `MOVE.W` / `MOVEA.W`
- **`$4`**: Miscellaneous / System (`LEA`, `PEA`, `CLR`, `NEG`, `NOT`, `TST`, `EXT`, `SWAP`, `TRAP`, `LINK`, `UNLK`, `JMP`, `JSR`, `RTS`, `RTE`, `MOVEM`, `CHK`)
- **`$5`**: Quick Arithmetic & Tests (`ADDQ`, `SUBQ`, `Scc`, `DBcc`)
- **`$6`**: PC-Relative Branches (`BRA`, `BSR`, `Bcc` with 16 conditions)
- **`$7`**: Move Quick (`MOVEQ`)
- **`$8`**: Logical OR & Division (`OR`, `DIVU`, `DIVS`, `SBCD`)
- **`$9`**: Subtraction (`SUB`, `SUBA`, `SUBX`)
- **`$A`**: Line-A Trap (Vector 10 emulator)
- **`$B`**: Compare & Exclusive OR (`CMP`, `CMPA`, `EOR`, `CMPM`)
- **`$C`**: Logical AND & Multiplication (`AND`, `MULU`, `MULS`, `ABCD`, `EXG`)
- **`$D`**: Addition (`ADD`, `ADDA`, `ADDX`)
- **`$E`**: Shifts & Rotates (`ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `ROXL`, `ROXR`)
- **`$F`**: Line-F Coprocessor Trap (Vector 11 emulator)

### B. Direction & Size Rules
- **ALU Direction (Bit 8)**: `0` = $\langle\text{ea}\rangle \rightarrow D_n$; `1` = $D_n \rightarrow \langle\text{ea}\rangle$.
- **ALU Size (Bits 7–6)**: `00` = Byte (`.B`); `01` = Word (`.W`); `10` = Long (`.L`).
- **Address Register Destination (`ADDA`, `SUBA`, `CMPA`)**: Bits 8–6 = `011` (Word sign-extended); `111` (Long).
- **MOVE Size (Bits 13–12)**: `01` = Byte; `11` = Word; `10` = Long.

---

## 5. Execution Skill: `add-m68k-instruction`

When implementing a new opcode, addressing mode, or instruction family, follow the step-by-step recipe in [`add-m68k-instruction`](../skills/add-m68k-instruction/SKILL.md) for flat linear opcode dispatch, inlined CCR calculations, CCK cycle phase modeling, prefetch advancement, and SingleStepTest silicon verification.
