---
title: "M68000 Instruction Benchmark Catalog"
aliases: ["Instruction Benchmark Catalog", "Benchmark Catalog"]
tags: ["amiga", "design", "m68000", "benchmark", "opcodes"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-09-12
related: ["[CPU Instruction Benchmarking.md](CPU%20Instruction%20Benchmarking.md)", "[CPU Instruction Benchmark Strategies.md](CPU%20Instruction%20Benchmark%20Strategies.md)", "[CPU Benchmark Analysis Guide.md](CPU%20Benchmark%20Analysis%20Guide.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)"]
---

# M68000 Instruction Benchmark Catalog

- **Parent Specification:** [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)
- **Execution Architecture:** [CPU Instruction Benchmarking.md](CPU%20Instruction%20Benchmarking.md)
- **Companion Specifications:** [CPU Instruction Benchmark Strategies.md](CPU%20Instruction%20Benchmark%20Strategies.md) | [CPU Benchmark Analysis Guide.md](CPU%20Benchmark%20Analysis%20Guide.md)
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../AGENTS.md).

> [!NOTE]
> This document defines the exhaustive catalog of representative instruction variants, sizes, and addressing modes evaluated by the M68000 benchmarking subsystem.
> It serves both as architectural documentation and as the formal data specification for the programmatic `BenchmarkProgramBuilder`.
> For execution architecture and anomaly detection formulas, see [CPU Instruction Benchmarking](CPU%20Instruction%20Benchmarking.md).
> For per-family test harness setups and cascading stacks, see [CPU Instruction Benchmark Strategies](CPU%20Instruction%20Benchmark%20Strategies.md).

---

## 1. Representative Register & Addressing Mode Policy

The Motorola 68000 instruction set supports 8 Data registers ($D_0 \dots D_7$) and 8 Address registers ($A_0 \dots A_7$). Testing every permutation of register indices ($8 \times 8 = 64$ variants per instruction) would cause combinatorial explosion without providing additional micro-architectural insight, because the internal ALU and micro-step sequencer operate identically across all data and address register indices.

### 1.1 Canonical Representative Registers
To achieve 100% coverage of all operational decoding paths, micro-step states, and addressing modes with maximum execution efficiency:
- **Canonical Destination Data Register:** $D_0$
- **Canonical Source Data Register:** $D_1$ (with rotation through $D_0 \dots D_3$ in unrolled loops where non-saturation is required)
- **Canonical Source Address Pointer:** $A_0$
- **Canonical Destination Address Pointer:** $A_1$
- **Canonical Frame Pointer:** $A_6$
- **Canonical Stack Pointer:** $A_7$ ($SP$)
- **Canonical Index Register:** $D_2\text{.W}$

### 1.2 The 12 Supported Addressing Modes
Every instruction family exercises all addressing modes permitted by the M68000 architecture:

| ID | Mode Name | Syntax | EA Mode (`mmm`) | EA Reg (`rrr`) | Extension Words |
| :--- | :--- | :--- | :---: | :---: | :---: |
| **0** | Data Register Direct | `Dn` | `000` | Reg (0..7) | 0 |
| **1** | Address Register Direct | `An` | `001` | Reg (0..7) | 0 |
| **2** | Address Register Indirect | `(An)` | `010` | Reg (0..7) | 0 |
| **3** | Address Indirect with Post-increment | `(An)+` | `011` | Reg (0..7) | 0 |
| **4** | Address Indirect with Pre-decrement | `-(An)` | `100` | Reg (0..7) | 0 |
| **5** | Address Indirect with Displacement | `d16(An)` | `101` | Reg (0..7) | 1 (`$0010`) |
| **6** | Address Indirect with Index | `d8(An, Xn)` | `110` | Reg (0..7) | 1 (`$2008`) |
| **7** | Absolute Short | `(xxx).W` | `111` | `000` | 1 (`$2000`) |
| **8** | Absolute Long | `(xxx).L` | `111` | `001` | 2 (`$0000`, `$2000`) |
| **9** | Program Counter with Displacement | `d16(PC)` | `111` | `010` | 1 (`$0008`) |
| **10** | Program Counter with Index | `d8(PC, Xn)` | `111` | `011` | 1 (`$2004`) |
| **11** | Immediate | `#<data>` | `111` | `100` | 1 or 2 |

---

## 2. Exhaustive Instruction Benchmark Matrix

### 2.0 Category 0: Calibration Baseline (`NOP`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **BASE-00** | `NOP` | — | Implied | `NOP` | `4E71` | 4 | `baseline` |

> [!TIP]
> `BASE-00` represents the reference baseline. All other benchmarks can report both raw latency and differential latency ($\Delta T = T_{\text{op}} - T_{\text{NOP}}$) to isolate pure execution cost.

### 2.1 Category 1: Data Movement (`MOVE`, `MOVEM`, `EXG`, `LEA`, `PEA`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **MOV-01** | `MOVE` | `.B` | Data Reg Direct | `MOVE.B D1, D0` | `1001` | 4 | `move_reg` |
| **MOV-02** | `MOVE` | `.W` | Data Reg Direct | `MOVE.W D1, D0` | `3001` | 4 | `move_reg` |
| **MOV-03** | `MOVE` | `.L` | Data Reg Direct | `MOVE.L D1, D0` | `2001` | 4 | `move_reg` |
| **MOV-04** | `MOVE` | `.W` | Address Indirect Read | `MOVE.W (A0), D0` | `3010` | 8 | `move_mem` |
| **MOV-05** | `MOVE` | `.W` | Post-increment Read | `MOVE.W (A0)+, D0` | `3018` | 8 | `move_mem` |
| **MOV-06** | `MOVE` | `.W` | Pre-decrement Read | `MOVE.W -(A0), D0` | `3020` | 10 | `move_mem` |
| **MOV-07** | `MOVE` | `.W` | Displacement Read | `MOVE.W 16(A0), D0` | `3028 0010` | 12 | `move_mem` |
| **MOV-08** | `MOVE` | `.W` | Index Read | `MOVE.W 8(A0, D2.W), D0` | `3030 2008` | 14 | `move_mem` |
| **MOV-09** | `MOVE` | `.W` | Absolute Short Read | `MOVE.W ($2000).W, D0` | `3038 2000` | 12 | `move_mem` |
| **MOV-10** | `MOVE` | `.W` | Absolute Long Read | `MOVE.W ($00002000).L, D0` | `3039 0000 2000` | 16 | `move_mem` |
| **MOV-11** | `MOVE` | `.W` | PC Displacement Read | `MOVE.W 8(PC), D0` | `303A 0008` | 12 | `move_mem` |
| **MOV-12** | `MOVE` | `.W` | Immediate Read | `MOVE.W #$1234, D0` | `303C 1234` | 8 | `move_imm` |
| **MOV-13** | `MOVE` | `.W` | Memory-to-Memory | `MOVE.W (A0)+, (A1)+` | `32D8` | 12 | `move_mem2mem` |
| **MOV-14** | `MOVEA` | `.W` | Data Reg to Addr Reg | `MOVEA.W D0, A0` | `3040` | 4 | `move_addr` |
| **MOV-15** | `MOVEA` | `.L` | Data Reg to Addr Reg | `MOVEA.L D0, A0` | `2040` | 4 | `move_addr` |
| **MOV-16** | `MOVEM` | `.W` | Reg to Mem (4 regs) | `MOVEM.W D0-D3, -(SP)` | `48A7 000F` | 24 | `move_multi` |
| **MOV-17** | `MOVEM` | `.W` | Mem to Reg (4 regs) | `MOVEM.W (SP)+, D0-D3` | `4CDF 000F` | 28 | `move_multi` |
| **MOV-18** | `EXG` | `.L` | Data Regs | `EXG D0, D1` | `C141` | 6 | `move_reg` |
| **MOV-19** | `EXG` | `.L` | Addr Regs | `EXG A0, A1` | `C149` | 6 | `move_reg` |
| **MOV-20** | `LEA` | `.L` | Displacement to Addr Reg | `LEA 16(A0), A1` | `43E8 0010` | 8 | `move_addr` |
| **MOV-21** | `PEA` | `.L` | Push Effective Address | `PEA 16(A0)` | `4868 0010` | 16 | `move_stack` |
| **MOV-22** | `LINK` | `.W` | Link Stack Frame | `LINK A6, #-16` | `4E56 FFF0` | 16 | `move_stack` |
| **MOV-23** | `UNLK` | `.L` | Unlink Stack Frame | `UNLK A6` | `4E5E` | 12 | `move_stack` |

---

### 2.2 Category 2: Integer Arithmetic (`ADD`, `SUB`, `MUL`, `DIV`, `NEG`, `CLR`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **ARITH-01** | `ADD` | `.B` | Data Reg Direct | `ADD.B D1, D0` | `D001` | 4 | `arith_reg` |
| **ARITH-02** | `ADD` | `.W` | Data Reg Direct | `ADD.W D1, D0` | `D041` | 4 | `arith_reg` |
| **ARITH-03** | `ADD` | `.L` | Data Reg Direct | `ADD.L D1, D0` | `D081` | 8 | `arith_reg` |
| **ARITH-04** | `ADD` | `.W` | Indirect Memory to Reg | `ADD.W (A0)+, D0` | `D058` | 8 | `arith_mem` |
| **ARITH-05** | `ADD` | `.W` | Reg to Indirect Memory | `ADD.W D0, (A0)` | `D150` | 12 | `arith_mem` |
| **ARITH-06** | `ADDA` | `.W` | Word to Addr Reg | `ADDA.W D0, A0` | `D0C0` | 8 | `arith_addr` |
| **ARITH-07** | `ADDA` | `.L` | Long to Addr Reg | `ADDA.L D0, A0` | `D0E0` | 8 | `arith_addr` |
| **ARITH-08** | `ADDQ` | `.W` | Quick Immediate to Reg | `ADDQ.W #4, D0` | `5840` | 4 | `arith_imm` |
| **ARITH-09** | `ADDX` | `.W` | Extended Add Regs | `ADDX.W D1, D0` | `D141` | 4 | `arith_reg` |
| **ARITH-10** | `ADDX` | `.W` | Extended Add Mem | `ADDX.W -(A1), -(A0)` | `D149` | 18 | `arith_mem` |
| **ARITH-11** | `SUB` | `.W` | Data Reg Direct | `SUB.W D1, D0` | `9041` | 4 | `arith_reg` |
| **ARITH-12** | `SUB` | `.L` | Data Reg Direct | `SUB.L D1, D0` | `9081` | 8 | `arith_reg` |
| **ARITH-13** | `SUBA` | `.W` | Word to Addr Reg | `SUBA.W D0, A0` | `90C0` | 8 | `arith_addr` |
| **ARITH-14** | `SUBQ` | `.W` | Quick Immediate to Reg | `SUBQ.W #4, D0` | `5940` | 4 | `arith_imm` |
| **ARITH-15** | `SUBX` | `.W` | Extended Sub Regs | `SUBX.W D1, D0` | `9141` | 4 | `arith_reg` |
| **ARITH-16** | `MULU` | `.W` | Unsigned Multiply Reg | `MULU D1, D0` | `C0C1` | ~54 | `arith_mul` |
| **ARITH-17** | `MULS` | `.W` | Signed Multiply Reg | `MULS D1, D0` | `C1C1` | ~54 | `arith_mul` |
| **ARITH-18** | `DIVU` | `.W` | Unsigned Divide Reg | `DIVU D1, D0` | `80C1` | ~140 | `arith_div` |
| **ARITH-18b**| `DIVU` | `.W` | Quotient Overflow Path | `DIVU D1, D0` (Overflow) | `80C1` | ~10 | `arith_div` |
| **ARITH-18c**| `DIVU` | `.W` | Divide-by-Zero Exception| `DIVU #0, D0` (Vector 5)| `80FC 0000` | ~42 | `arith_div` |
| **ARITH-19** | `DIVS` | `.W` | Signed Divide Reg | `DIVS D1, D0` | `81C1` | ~158 | `arith_div` |
| **ARITH-19b**| `DIVS` | `.W` | Quotient Overflow Path | `DIVS D1, D0` (Overflow) | `81C1` | ~10 | `arith_div` |
| **ARITH-19c**| `DIVS` | `.W` | Divide-by-Zero Exception| `DIVS #0, D0` (Vector 5)| `81FC 0000` | ~42 | `arith_div` |
| **ARITH-20** | `NEG` | `.W` | Negate Register | `NEG.W D0` | `4440` | 4 | `arith_reg` |
| **ARITH-21** | `NEGX` | `.W` | Negate with Extend Reg | `NEGX.W D0` | `4040` | 4 | `arith_reg` |
| **ARITH-22** | `CLR` | `.W` | Clear Register | `CLR.W D0` | `4240` | 4 | `arith_reg` |
| **ARITH-23** | `CLR` | `.W` | Clear Memory | `CLR.W (A0)+` | `4258` | 8 | `arith_mem` |
| **ARITH-24** | `EXT` | `.W` | Sign Extend Byte to Word | `EXT.W D0` | `4880` | 4 | `arith_reg` |
| **ARITH-25** | `EXT` | `.L` | Sign Extend Word to Long | `EXT.L D0` | `48C0` | 4 | `arith_reg` |

---

### 2.3 Category 3: Logic & Bit Operations (`AND`, `OR`, `EOR`, `NOT`, `BTST`, `BSET`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **LOGIC-01** | `AND` | `.W` | Data Reg Direct | `AND.W D1, D0` | `C041` | 4 | `logic_reg` |
| **LOGIC-02** | `AND` | `.L` | Data Reg Direct | `AND.L D1, D0` | `C081` | 8 | `logic_reg` |
| **LOGIC-03** | `ANDI` | `.W` | Immediate to Reg | `ANDI.W #$AAAA, D0` | `0240 AAAA` | 8 | `logic_imm` |
| **LOGIC-04** | `OR` | `.W` | Data Reg Direct | `OR.W D1, D0` | `8041` | 4 | `logic_reg` |
| **LOGIC-05** | `ORI` | `.W` | Immediate to Reg | `ORI.W #$5555, D0` | `0040 5555` | 8 | `logic_imm` |
| **LOGIC-06** | `EOR` | `.W` | Data Reg Direct | `EOR.W D1, D0` | `B140` | 4 | `logic_reg` |
| **LOGIC-07** | `EORI` | `.W` | Immediate to Reg | `EORI.W #$00FF, D0` | `0A40 00FF` | 8 | `logic_imm` |
| **LOGIC-08** | `NOT` | `.W` | Invert Register Bits | `NOT.W D0` | `4640` | 4 | `logic_reg` |
| **LOGIC-09** | `BTST` | `.L` | Static Bit Test Reg | `BTST #5, D0` | `0800 0005` | 10 | `bit_test` |
| **LOGIC-10** | `BTST` | `.L` | Dynamic Bit Test Reg | `BTST D1, D0` | `0100` | 6 | `bit_test` |
| **LOGIC-11** | `BSET` | `.L` | Dynamic Bit Set Reg | `BSET D1, D0` | `01C0` | 8 | `bit_mod` |
| **LOGIC-12** | `BCLR` | `.L` | Dynamic Bit Clear Reg | `BCLR D1, D0` | `0180` | 10 | `bit_mod` |
| **LOGIC-13** | `BCHG` | `.L` | Dynamic Bit Change Reg | `BCHG D1, D0` | `0140` | 8 | `bit_mod` |
| **LOGIC-14** | `TAS` | `.B` | Test & Set Memory (RMW)| `TAS (A0)+` | `4AD8` | 14 | `bit_rmw` |

---

### 2.4 Category 4: Shift & Rotate (`ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `SWAP`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **SHIFT-01** | `LSL` | `.W` | Static Immediate Count | `LSL.W #3, D0` | `E748` | 12 | `shift_static` |
| **SHIFT-02** | `LSR` | `.W` | Static Immediate Count | `LSR.W #3, D0` | `E648` | 12 | `shift_static` |
| **SHIFT-03** | `LSL` | `.W` | Dynamic Register Count | `LSL.W D1, D0` | `E368` | ~12 | `shift_dynamic` |
| **SHIFT-04** | `ASL` | `.W` | Static Immediate Count | `ASL.W #2, D0` | `E540` | 10 | `shift_static` |
| **SHIFT-05** | `ASR` | `.W` | Static Immediate Count | `ASR.W #2, D0` | `E440` | 10 | `shift_static` |
| **SHIFT-06** | `ROL` | `.W` | Static Immediate Count | `ROL.W #4, D0` | `E958` | 14 | `shift_rotate` |
| **SHIFT-07** | `ROR` | `.W` | Static Immediate Count | `ROR.W #4, D0` | `E858` | 14 | `shift_rotate` |
| **SHIFT-08** | `SWAP` | `.W` | Swap 16-bit Halves | `SWAP D0` | `4840` | 4 | `shift_rotate` |

---

### 2.5 Category 5: Comparisons & Tests (`CMP`, `CMPA`, `CMPM`, `CMPI`, `TST`, `CHK`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **CMP-01** | `CMP` | `.B` | Data Reg Direct | `CMP.B D1, D0` | `B001` | 4 | `cmp_reg` |
| **CMP-02** | `CMP` | `.W` | Data Reg Direct | `CMP.W D1, D0` | `B041` | 4 | `cmp_reg` |
| **CMP-03** | `CMP` | `.L` | Data Reg Direct | `CMP.L D1, D0` | `B081` | 6 | `cmp_reg` |
| **CMP-04** | `CMPA` | `.W` | Word to Addr Reg | `CMPA.W D0, A0` | `B0C0` | 6 | `cmp_addr` |
| **CMP-05** | `CMPA` | `.L` | Long to Addr Reg | `CMPA.L D0, A0` | `B0E0` | 6 | `cmp_addr` |
| **CMP-06** | `CMPI` | `.W` | Immediate to Reg | `CMPI.W #$1234, D0` | `0C40 1234` | 8 | `cmp_imm` |
| **CMP-07** | `CMPM` | `.W` | Post-inc Memory Compare| `CMPM.W (A0)+, (A1)+`| `B348` | 12 | `cmp_mem` |
| **CMP-08** | `TST` | `.W` | Test Register | `TST.W D0` | `4A40` | 4 | `cmp_tst` |
| **CMP-09** | `TST` | `.W` | Test Memory | `TST.W (A0)+` | `4A58` | 8 | `cmp_tst` |
| **CMP-10** | `CHK` | `.W` | Check Bounds (In-bound)| `CHK.W D1, D0` | `4181` | 10 | `cmp_chk` |

---

### 2.6 Category 6: Decimal / BCD Arithmetic (`ABCD`, `SBCD`, `NBCD`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **BCD-01** | `ABCD` | `.B` | Data Regs | `ABCD D1, D0` | `C101` | 6 | `bcd_reg` |
| **BCD-02** | `ABCD` | `.B` | Pre-dec Memory | `ABCD -(A1), -(A0)` | `C109` | 18 | `bcd_mem` |
| **BCD-03** | `SBCD` | `.B` | Data Regs | `SBCD D1, D0` | `8101` | 6 | `bcd_reg` |
| **BCD-04** | `SBCD` | `.B` | Pre-dec Memory | `SBCD -(A1), -(A0)` | `8109` | 18 | `bcd_mem` |
| **BCD-05** | `NBCD` | `.B` | Data Reg | `NBCD D0` | `4800` | 6 | `bcd_reg` |

---

### 2.7 Category 7: Control Flow & Subroutines (`BRA`, `Bcc`, `BSR`, `JSR`, `RTS`, `RTR`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **FLOW-01** | `BRA` | `.S` | Short Relative Branch | `BRA.S +2` | `6002` | 10 | `flow_branch` |
| **FLOW-02** | `Bcc` | `.S` | Branch Taken (BEQ) | `BEQ.S +2` | `6702` | 10 | `flow_branch` |
| **FLOW-03** | `Bcc` | `.S` | Branch Not Taken (BEQ)| `BEQ.S +2` | `6702` | 8 | `flow_branch` |
| **FLOW-04** | `BSR` | `.S` | Subroutine Branch | `BSR.S sub` | `6102` | 18 | `flow_call` |
| **FLOW-05** | `RTS` | — | Cascading Stack Return | `RTS` | `4E75` | 16 | `flow_return` |
| **FLOW-06** | `RTR` | — | Cascading Return with CCR| `RTR` | `4E77` | 20 | `flow_return` |
| **FLOW-07** | `DBcc` | `.W` | Decrement & Loop Taken| `DBF D7, target` | `51CF 0002` | 10 | `flow_loop` |
| **FLOW-08** | `DBcc` | `.W` | Loop Terminal Fallthrough| `DBF D7, exit` | `51CF 0002` | 14 | `flow_loop` |

---

### 2.8 Category 8: System & Privileged (`MOVE to SR/CCR`, `RTE`, `TRAP`, `TRAPV`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **SYS-01** | `MOVE` | `.W` | Move to CCR | `MOVE.W D0, CCR` | `44C0` | 12 | `sys_ccr` |
| **SYS-02** | `MOVE` | `.W` | Move to SR (Supervisor) | `MOVE.W D0, SR` | `46C0` | 12 | `sys_sr` |
| **SYS-03** | `MOVE` | `.W` | Move from SR | `MOVE.W SR, D0` | `40C0` | 6 | `sys_sr` |
| **SYS-04** | `ANDI` | `.W` | AND Immediate to CCR | `ANDI.W #$001F, CCR` | `023C 001F` | 20 | `sys_ccr` |
| **SYS-05** | `ORI` | `.W` | OR Immediate to CCR | `ORI.W #$0000, CCR` | `003C 0000` | 20 | `sys_ccr` |
| **SYS-06** | `MOVE` | `.L` | Move USP to Addr Reg | `MOVE USP, A0` | `4E68` | 4 | `sys_usp` |
| **SYS-07** | `MOVE` | `.L` | Move Addr Reg to USP | `MOVE A0, USP` | `4E60` | 4 | `sys_usp` |
| **SYS-08** | `RTE` | — | Cascading Exception Return| `RTE` | `4E73` | 20 | `sys_rte` |
| **SYS-09** | `TRAP` | — | Software Trap Handler | `TRAP #0` | `4E40` | 34 | `sys_trap` |
| **SYS-10** | `TRAPV` | — | Trap on Overflow (Untaken)| `TRAPV` | `4E76` | 4 | `sys_trap` |

---

## 3. Benchmark Catalog Execution Filtering & Trace Dumps

The programmatic runner exposes `--filter <TAG>` / `BENCH_FILTER=<TAG>` to isolate specific subsets during profiling, and `--dump-traces` to produce single-pass audit files:

```powershell
# Run only data register moves and memory moves:
cargo bench -p test_runner --bench bench_instructions -- --filter "move_"

# Run only heavy arithmetic (multiplications and divisions):
cargo bench -p test_runner --bench bench_instructions -- --filter "arith_mul|arith_div"

# Run only subroutine returns and branching:
cargo bench -p test_runner --bench bench_instructions -- --filter "flow_"

# Dump single-pass execution traces for LLM / human audit verification:
cargo run -p test_runner --release -- bench --dump-traces --filter "arith_|move_"
```

---

## 4. Reference Documentation & Upstream Ground Truth

- [68000 User's Manual: Section 8 (16-Bit Instruction Execution Timing & Bus Tables)](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): Standard instruction timings and bus operation counts.
- [CPU Instruction Benchmarking Architecture](CPU%20Instruction%20Benchmarking.md): Benchmark harness execution hierarchy and anomaly detection formulas.
- [CPU Instruction Benchmark Strategies](CPU%20Instruction%20Benchmark%20Strategies.md): Strategy matrix for cascading stacks and data generators.
- [CPU Benchmark Analysis Guide](CPU%20Benchmark%20Analysis%20Guide.md): Operational guide for interpreting host performance metrics and anomalies.
- [CPU Motorola M68000 Architecture](CPU%20Motorola%20M68000.md): Register architecture, condition codes, and processor status.
- [CPU Micro-Step State Machine Specification](CPU%20Micro-Step%20State%20Machine.md): Color Clock cycle decomposition and microcode execution.
- [Benchmark Catalog Core Implementation](../../../crates/test_runner/src/benchmark/catalog.rs): Living Rust catalog definitions and filter parsers.
- [Benchmark Catalog Part A](../../../crates/test_runner/src/benchmark/catalog_data_a.rs): Categories 0–3 specifications.
- [Benchmark Catalog Part B](../../../crates/test_runner/src/benchmark/catalog_data_b.rs): Categories 4–8 specifications.
- [Benchmark Program Builder](../../../crates/test_runner/src/benchmark/builder.rs): Programmatic instruction sequence synthesis.
- [Benchmark Single-Pass Tracer](../../../crates/test_runner/src/benchmark/tracer.rs): Trace execution and state-delta recording.
- [Benchmark State-Delta Verification Suite](../../../crates/test_runner/tests/test_benchmark_trace.rs): Regression tests for trace generation.

