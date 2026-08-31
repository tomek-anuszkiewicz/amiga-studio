# CPU SingleStepTests & Verification

This document defines the complete specification for running the **SingleStepTests** test suite against the M68000 CPU emulator in Rust. It assumes all 127 test files in `ref_src/SingleStepTests-m68000/v1/` are available in JSON format (e.g. `ADD.b.json`, `MOVE.w.json`, `ILLEGAL_LINEA.json`).

Based on this specification, the entire test runner and all per-instruction test modules can be directly implemented.

---

## 1. Test Suite Overview

- **Location:** `ref_src/SingleStepTests-m68000/v1/*.json` (127 JSON files).
- **Origin:** Generated from MAME's cycle-exact, microcoded Motorola 68000 core.
- **Scope:** Exhaustive unit tests covering:
  - All standard instructions, sizes (`.b`, `.w`, `.l`), and addressing modes.
  - Condition code register (`CCR` / `SR`) calculations.
  - Internal instruction prefetch queue stages (`pf0`, `pf1`).
  - Exceptions & interrupts: Line-A (`ILLEGAL_LINEA`), Line-F (`ILLEGAL_LINEF`), `TRAP`, `TRAPV`, `CHK`, `RESET`, `STOP`.
  - Address Errors (unaligned word/long read/write operations triggering Vector 3 exceptions).
  - Bus cycle activity and cycle counts.

---

## 2. JSON Test Schema

Each `.json` file contains a JSON array of individual test cases: `[ { ... }, { ... } ]`.

### 2.1 Example JSON Test Object
```json
{
  "name": "ADD.B D1, D0 (0000)",
  "initial": {
    "d0": 0,
    "d1": 42,
    "d2": 0,
    "d3": 0,
    "d4": 0,
    "d5": 0,
    "d6": 0,
    "d7": 0,
    "a0": 0,
    "a1": 0,
    "a2": 0,
    "a3": 0,
    "a4": 0,
    "a5": 0,
    "a6": 0,
    "usp": 0,
    "ssp": 1048576,
    "sr": 9984,
    "pc": 1000,
    "prefetch": [ 53264, 0 ],
    "ram": [
      [ 1000, 208 ],
      [ 1001, 8 ],
      [ 1002, 0 ],
      [ 1003, 0 ]
    ]
  },
  "final": {
    "d0": 42,
    "d1": 42,
    "d2": 0,
    "d3": 0,
    "d4": 0,
    "d5": 0,
    "d6": 0,
    "d7": 0,
    "a0": 0,
    "a1": 0,
    "a2": 0,
    "a3": 0,
    "a4": 0,
    "a5": 0,
    "a6": 0,
    "usp": 0,
    "ssp": 1048576,
    "sr": 9984,
    "pc": 1004,
    "prefetch": [ 0, 0 ],
    "ram": [
      [ 1000, 208 ],
      [ 1001, 8 ]
    ]
  },
  "transactions": [
    [ "r", 0, 2, 1000, ".w", 53264, 1, 1 ],
    [ "r", 4, 2, 1002, ".w", 0, 1, 1 ],
    [ "n", 8 ]
  ],
  "length": 8
}
```

### 2.2 Field Descriptions
| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | `String` | Human-readable test description |
| `initial` | `Object` | CPU state and memory before execution |
| `final` | `Object` | Expected CPU state and memory after execution |
| `transactions` | `Array` | Log of cycle transactions (`r` = read, `w` = write, `n` = idle, `t` = TAS, `re` = read address error, `we` = write address error) |
| `length` | `u32` | Total number of clock cycles taken |

#### State Object Fields (`initial` and `final`):
- `d0`–`d7`: Data registers (`u32`)
- `a0`–`a6`: Address registers (`u32`)
- `usp`: User Stack Pointer (`u32`)
- `ssp`: Supervisor Stack Pointer (`u32`)
- `sr`: Status Register (`u16`) containing condition codes and system flags
- `pc`: Program Counter (`u32`, points to next prefetch address)
- `prefetch`: Two 16-bit words (`[u32; 2]`) in the CPU prefetch queue (`pf0`, `pf1`)
- `ram`: Array of `[address, byte_value]` pairs (`Vec<[u32; 2]>`)

### 2.3 Strict Schema Validation Rules
- **Optional Fields:** **None**. Every single field in the test schema is **mandatory**. If any expected field is missing from a test object or state object, deserialization must immediately fail with a descriptive error.
- **Unknown Fields:** **Forbidden**. Any unrecognized or unexpected keys present in the JSON must cause deserialization to fail.

---

## 3. Rust Deserialization Data Structures

To enforce strict validation, all structs use `#[serde(deny_unknown_fields)]`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingleStepTest {
    pub name: String,
    pub initial: CpuTestState,
    #[serde(rename = "final")]
    pub final_state: CpuTestState,
    pub transactions: Vec<serde_json::Value>,
    pub length: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuTestState {
    pub d0: u32,
    pub d1: u32,
    pub d2: u32,
    pub d3: u32,
    pub d4: u32,
    pub d5: u32,
    pub d6: u32,
    pub d7: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub a4: u32,
    pub a5: u32,
    pub a6: u32,
    pub usp: u32,
    pub ssp: u32,
    pub sr: u16,
    pub pc: u32,
    pub prefetch: [u32; 2],
    pub ram: Vec<[u32; 2]>, // [address, byte]
}
```

---

## 4. Test Execution Loop

Each test case is executed independently in an isolated test harness:

```mermaid
flowchart TD
    A[Load JSON Test File] --> B[For Each SingleStepTest Case]
    B --> C[Create Test MemoryBus & Reset CPU]
    C --> D[Populate Initial RAM Bytes]
    D --> E[Load Registers D0-D7, A0-A6, USP, SSP, SR, PC]
    E --> F[Prime Prefetch Queue with pf0, pf1]
    F --> G[Step CPU by CCKs until instruction finishes]
    G --> H[Assert D0-D7 and A0-A6 match final_state]
    H --> I[Assert active Stack Pointer USP/SSP matches]
    I --> J[Assert SR Status Register / Condition Codes match]
    J --> K[Assert PC and Prefetch Queue match]
    K --> L[Assert final RAM bytes in test memory]
```

### 4.1 Step-by-Step Test Runner Logic

1. **Memory Initialization:**
   - Use a lightweight test memory model (e.g. flat byte array or sparse hash map `HashMap<u32, u8>`).
   - Populate test memory with all `[address, byte]` pairs from `initial.ram`.
2. **CPU State Setup:**
   - Load $D_0-D_7$ and $A_0-A_6$.
   - Load $USP$ and $SSP$. Set active $A_7$ based on the Supervisor bit ($S$) in $SR$.
   - Set $SR$ and $PC$.
   - Prime the internal prefetch buffer with `initial.prefetch[0]` and `initial.prefetch[1]`.
3. **Execute Instruction:**
   - Step the CPU cycle-by-cycle (or CCK-by-CCK) until the current instruction completes.
   - Guard against infinite loops with a timeout threshold (e.g., `length * 2` cycles or max 1,000 cycles).
4. **State Verification:**
   - **Data Registers:** Assert `cpu.d(i) == final_state.d[i]` for $i \in 0..7$.
   - **Address Registers:** Assert `cpu.a(i) == final_state.a[i]` for $i \in 0..6$.
   - **Stack Pointers:** Assert `cpu.usp == final_state.usp` and `cpu.ssp == final_state.ssp`.
   - **Status Register (SR / CCR):** Assert `cpu.sr == final_state.sr`.
   - **Program Counter:** Assert `cpu.pc == final_state.pc`.
   - **Prefetch Queue:** Assert `cpu.prefetch == final_state.prefetch`.
   - **RAM Verification:** Iterate through all `[address, expected_byte]` pairs in `final_state.ram` and assert that the test memory contains the exact byte.
5. **Address Error & Exception Handling:**
   - If the test triggers an address error (unaligned word/long access), verify that the CPU:
     1. Pushes the Address Error stack frame (Function Code, Access Address, Instruction Register, Status Register, PC).
     2. Sets the Supervisor bit $S$ in $SR$.
     3. Fetches the exception handler address from Vector 3 (`$00000C`).

---

## 5. Rust Test Harness Implementation (`common.rs`)

```rust
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use crate::m68000::cpu::Cpu;
use crate::m68000::bus::TestMemoryBus;

pub fn run_test_file<P: AsRef<Path>>(json_path: P) {
    let file = File::open(json_path.as_ref())
        .unwrap_or_else(|_| panic!("Failed to open test file: {:?}", json_path.as_ref()));
    let reader = BufReader::new(file);
    let tests: Vec<SingleStepTest> = serde_json::from_reader(reader)
        .unwrap_or_else(|e| panic!("Failed to parse JSON {:?}: {}", json_path.as_ref(), e));

    for test in tests {
        run_single_test(&test);
    }
}

pub fn run_single_test(test: &SingleStepTest) {
    let mut bus = TestMemoryBus::new();
    
    // 1. Populate RAM
    for ram_entry in &test.initial.ram {
        let addr = ram_entry[0];
        let val = ram_entry[1] as u8;
        bus.write_byte(addr, val);
    }

    // 2. Instantiate and configure CPU
    let mut cpu = Cpu::new();
    cpu.set_d0(test.initial.d0);
    cpu.set_d1(test.initial.d1);
    cpu.set_d2(test.initial.d2);
    cpu.set_d3(test.initial.d3);
    cpu.set_d4(test.initial.d4);
    cpu.set_d5(test.initial.d5);
    cpu.set_d6(test.initial.d6);
    cpu.set_d7(test.initial.d7);
    
    cpu.set_a0(test.initial.a0);
    cpu.set_a1(test.initial.a1);
    cpu.set_a2(test.initial.a2);
    cpu.set_a3(test.initial.a3);
    cpu.set_a4(test.initial.a4);
    cpu.set_a5(test.initial.a5);
    cpu.set_a6(test.initial.a6);
    
    cpu.set_usp(test.initial.usp);
    cpu.set_ssp(test.initial.ssp);
    cpu.set_sr(test.initial.sr);
    cpu.set_pc(test.initial.pc);
    cpu.set_prefetch(test.initial.prefetch[0] as u16, test.initial.prefetch[1] as u16);

    // 3. Step execution
    let mut cycles = 0;
    while !cpu.is_instruction_finished() && cycles < (test.length * 2 + 10) {
        cpu.step_cck(&mut bus);
        cycles += 1;
    }

    // 4. Assert Final Registers
    assert_eq!(cpu.d0(), test.final_state.d0, "D0 mismatch in test: {}", test.name);
    assert_eq!(cpu.d1(), test.final_state.d1, "D1 mismatch in test: {}", test.name);
    assert_eq!(cpu.d2(), test.final_state.d2, "D2 mismatch in test: {}", test.name);
    assert_eq!(cpu.d3(), test.final_state.d3, "D3 mismatch in test: {}", test.name);
    assert_eq!(cpu.d4(), test.final_state.d4, "D4 mismatch in test: {}", test.name);
    assert_eq!(cpu.d5(), test.final_state.d5, "D5 mismatch in test: {}", test.name);
    assert_eq!(cpu.d6(), test.final_state.d6, "D6 mismatch in test: {}", test.name);
    assert_eq!(cpu.d7(), test.final_state.d7, "D7 mismatch in test: {}", test.name);

    assert_eq!(cpu.a0(), test.final_state.a0, "A0 mismatch in test: {}", test.name);
    assert_eq!(cpu.a1(), test.final_state.a1, "A1 mismatch in test: {}", test.name);
    assert_eq!(cpu.a2(), test.final_state.a2, "A2 mismatch in test: {}", test.name);
    assert_eq!(cpu.a3(), test.final_state.a3, "A3 mismatch in test: {}", test.name);
    assert_eq!(cpu.a4(), test.final_state.a4, "A4 mismatch in test: {}", test.name);
    assert_eq!(cpu.a5(), test.final_state.a5, "A5 mismatch in test: {}", test.name);
    assert_eq!(cpu.a6(), test.final_state.a6, "A6 mismatch in test: {}", test.name);

    assert_eq!(cpu.usp(), test.final_state.usp, "USP mismatch in test: {}", test.name);
    assert_eq!(cpu.ssp(), test.final_state.ssp, "SSP mismatch in test: {}", test.name);
    assert_eq!(cpu.sr(), test.final_state.sr, "SR mismatch in test: {}", test.name);
    assert_eq!(cpu.pc(), test.final_state.pc, "PC mismatch in test: {}", test.name);
    assert_eq!(cpu.prefetch(), [test.final_state.prefetch[0] as u16, test.final_state.prefetch[1] as u16], "Prefetch mismatch in test: {}", test.name);

    // 5. Assert Final RAM
    for ram_entry in &test.final_state.ram {
        let addr = ram_entry[0];
        let expected = ram_entry[1] as u8;
        let actual = bus.read_byte(addr);
        assert_eq!(actual, expected, "RAM mismatch at 0x{:06X} in test: {}", addr, test.name);
    }
}
```

---

## 6. Rust Test Structure & Organization

Each JSON file in `ref_src/SingleStepTests-m68000/v1/` has a corresponding test function or test module.

### 6.1 Directory & Module Layout
```text
tests/
  cpu/
    mod.rs
    common.rs

    // Arithmetic & Logic
    test_add_b.rs          // ADD.b.json
    test_add_w.rs          // ADD.w.json
    test_add_l.rs          // ADD.l.json
    test_adda_w.rs         // ADDA.w.json
    test_adda_l.rs         // ADDA.l.json
    test_addx_b.rs         // ADDX.b.json
    test_addx_w.rs         // ADDX.w.json
    test_addx_l.rs         // ADDX.l.json
    test_sub_b.rs          // SUB.b.json
    test_sub_w.rs          // SUB.w.json
    test_sub_l.rs          // SUB.l.json
    test_mulu.rs           // MULU.json
    test_muls.rs           // MULS.json
    test_divu.rs           // DIVU.json
    test_divs.rs           // DIVS.json
    test_and_b.rs          // AND.b.json
    test_or_b.rs           // OR.b.json
    test_eor_b.rs          // EOR.b.json
    test_neg_b.rs          // NEG.b.json
    test_not_b.rs          // NOT.b.json
    test_clr_b.rs          // CLR.b.json
    test_tst_b.rs          // TST.b.json
    test_cmp_b.rs          // CMP.b.json

    // Shifts & Rotates
    test_asl_b.rs          // ASL.b.json
    test_asr_b.rs          // ASR.b.json
    test_lsl_b.rs          // LSL.b.json
    test_lsr_b.rs          // LSR.b.json
    test_rol_b.rs          // ROL.b.json
    test_ror_b.rs          // ROR.b.json
    test_roxl_b.rs         // ROXL.b.json
    test_roxr_b.rs         // ROXR.b.json

    // Data Movement
    test_move_b.rs         // MOVE.b.json
    test_move_w.rs         // MOVE.w.json
    test_move_l.rs         // MOVE.l.json
    test_moveq.rs          // MOVE.q.json
    test_movea_w.rs        // MOVEA.w.json
    test_movea_l.rs        // MOVEA.l.json
    test_movem_w.rs        // MOVEM.w.json
    test_movem_l.rs        // MOVEM.l.json
    test_movep_w.rs        // MOVEP.w.json
    test_movep_l.rs        // MOVEP.l.json
    test_lea.rs            // LEA.json
    test_pea.rs            // PEA.json
    test_exg.rs            // EXG.json
    test_swap.rs           // SWAP.json
    test_ext_w.rs          // EXT.w.json
    test_ext_l.rs          // EXT.l.json

    // BCD & Bit Operations
    test_abcd.rs           // ABCD.json
    test_sbcd.rs           // SBCD.json
    test_nbcd.rs           // NBCD.json
    test_btst.rs           // BTST.json
    test_bset.rs           // BSET.json
    test_bclr.rs           // BCLR.json
    test_bchg.rs           // BCHG.json

    // Branch & Control Flow
    test_bcc.rs            // Bcc.json
    test_dbcc.rs           // DBcc.json
    test_bsr.rs            // BSR.json
    test_jmp.rs            // JMP.json
    test_jsr.rs            // JSR.json
    test_rts.rs            // RTS.json
    test_rte.rs            // RTE.json
    test_rtr.rs            // RTR.json
    test_link.rs           // LINK.json
    test_unlink.rs         // UNLINK.json
    test_nop.rs            // NOP.json

    // Status Register & System Operations
    test_andi_to_ccr.rs    // ANDItoCCR.json
    test_andi_to_sr.rs     // ANDItoSR.json
    test_eori_to_ccr.rs    // EORItoCCR.json
    test_eori_to_sr.rs     // EORItoSR.json
    test_ori_to_ccr.rs     // ORItoCCR.json
    test_ori_to_sr.rs      // ORItoSR.json
    test_move_to_ccr.rs    // MOVEtoCCR.json
    test_move_to_sr.rs     // MOVEtoSR.json
    test_move_from_sr.rs   // MOVEfromSR.json
    test_move_to_usp.rs    // MOVEtoUSP.json
    test_move_from_usp.rs  // MOVEfromUSP.json

    // Exceptions, Interrupts & Error Cases
    test_illegal_linea.rs  // ILLEGAL_LINEA.json (Line 1010)
    test_illegal_linef.rs  // ILLEGAL_LINEF.json (Line 1111)
    test_chk.rs            // CHK.json
    test_trap.rs           // TRAP.json
    test_trapv.rs          // TRAPV.json
    test_reset.rs          // RESET.json
    test_stop.rs           // STOP.json
```

### 6.2 Example Individual Test File (`test_add_b.rs`)

```rust
use crate::tests::cpu::common::run_test_file;

#[test]
fn test_add_b() {
    run_test_file("ref_src/SingleStepTests-m68000/v1/ADD.b.json");
}
```

### 6.3 Example Exception Test File (`test_illegal_linea.rs`)

```rust
use crate::tests::cpu::common::run_test_file;

#[test]
fn test_illegal_linea() {
    run_test_file("ref_src/SingleStepTests-m68000/v1/ILLEGAL_LINEA.json");
}
```

---

## 7. Running Tests

Individual instruction suites or error cases can be executed selectively:
```powershell
# Run only NOP tests
cargo test tests::cpu::test_nop

# Run specific arithmetic tests
cargo test tests::cpu::test_add_b

# Run exception & trap tests
cargo test tests::cpu::test_illegal_linea
cargo test tests::cpu::test_trap

# Run all CPU tests
cargo test tests::cpu
```

