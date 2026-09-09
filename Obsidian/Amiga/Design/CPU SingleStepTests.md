
# M68000 SingleStepTests Suite Specification

> [!NOTE]
> Operational test running, command shortcuts, and debugging checklists are documented in the [m68k-singlestep-test](../../../.agents/skills/m68k-singlestep-test/SKILL.md) skill.
> Step-by-step instruction implementation is guided by [add-m68k-instruction](../../../.agents/skills/add-m68k-instruction/SKILL.md).

This document defines the complete specification and Rust data structures for running the **SingleStepTests** test suites against the M68000 CPU emulator. It specifies validation against both:
1. The **MAME SingleStepTests suite** in [`ref_src/SingleStepTests-m68000/v1/`](../../../ref_src/SingleStepTests-m68000/v1) (127 JSON files).
2. The **Tom Harte SingleStepTests-680x0 suite** in [`ref_src/SingleStepTests-680x0/68000/v1/`](../../../ref_src/SingleStepTests-680x0/68000/v1) (124 `.json` files, ~1,000,000 tests).

---

## 1. Dual Test Suite Overview

To ensure robust, ground-truth verification and eliminate single-source simulation artifacts, the M68000 CPU emulator is validated against **two independent, complementary single-step test suites**:

| Feature | Suite 1: MAME SingleStepTests | Suite 2: Tom Harte SingleStepTests |
| :--- | :--- | :--- |
| **Path** | [`ref_src/SingleStepTests-m68000/v1/`](../../../ref_src/SingleStepTests-m68000/v1) | [`ref_src/SingleStepTests-680x0/68000/v1/`](../../../ref_src/SingleStepTests-680x0/68000/v1) |
| **File Format** | Plain `.json` (and `.json.bin`) | Plain `.json` |
| **Suite Count** | **127 test files** | **124 test files** |
| **Test Scale** | ~1,000–5,000 tests per file | ~8,000+ tests per file (~1,000,000 total) |
| **Origin** | MAME cycle-exact microcoded core | Tom Harte's CLK processor test generator |
| **Unique Strengths** | Fast uncompressed loading; includes `ILLEGAL_LINEA`, `ILLEGAL_LINEF`, `STOP` | Massive randomized test coverage; independently verified |
| **`TAS` Indivisible RMW** | ⚠️ Caveat: does not model 5-cycle RMW timing | ✅ Fully modeled via explicit `"t"` bus transactions |
| **`TRAPV` $S$-bit** | ⚠️ Caveat: Known quirk with S-bit handling | ✅ Standard behavior |
| **Mode Coverage** | Supervisor & User mode | 99% Supervisor mode, 1% User mode |

### 1.1 Why Dual-Suite Testing?
1. **Triangulation of Simulator Quirks:** When an instruction test fails, comparing against both MAME and Tom Harte tests immediately clarifies whether the issue is a genuine core bug or a quirk of MAME's microcode generator (such as `TAS` and `TRAPV`).
2. **Exhaustive Address & Mode Variation:** Tom Harte's suite generates over 1 million test vectors, checking edge cases in word-aligned vs unaligned memory pointers and register boundary combinations.
3. **Comprehensive Coverage:** MAME provides specialized exception vector suites (`ILLEGAL_LINEA`, `ILLEGAL_LINEF`, `STOP`) that are not present in Tom Harte's 125-opcode collection.


### 1.2 Opcode Packaging Quirk: CMP and CMPM
In both the MAME and Tom Harte suites, there are **no separate `CMPM.b.json`, `CMPM.w.json`, or `CMPM.l.json` files**. Instead, all `CMPM (Ay)+, (Ax)+` test cases are bundled together inside `CMP.b.json`, `CMP.w.json`, and `CMP.l.json` alongside standard `CMP <ea>, Dn`.

- The test runner provides [`is_cmpm_postinc_opcode`](crates/test_runner/src/runner.rs) (`(op & 0xF138) == 0xB108`) to isolate `CMPM` cases when specific post-increment testing is required.
- **Batch 1.3 Complete:** Standard `CMP`, `CMPA`, and `CMPI` are fully implemented and verified. Both isolated `CMPM` tests and exhaustive full-suite `CMP.<size>.json` integration tests run in [`crates/test_runner/tests/test_singlestep.rs`](crates/test_runner/tests/test_singlestep.rs) and [`crates/test_runner/tests/test_dma_cartesian.rs`](crates/test_runner/tests/test_dma_cartesian.rs).

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
| `transactions` | `Array` | Log of cycle bus transactions (see Section 2.3) |
| `length` | `u32` | Total number of clock cycles taken (assuming immediate DTACK) |

#### State Object Fields (`initial` and `final`):
- `d0`–`d7`: Data registers (`u32`)
- `a0`–`a6`: Address registers (`u32`)
- `usp`: User Stack Pointer (`u32`)
- `ssp`: Supervisor Stack Pointer (`u32`)
- `sr`: Status Register (`u16`) containing condition codes and system flags
- `pc`: Program Counter (`u32`, points to next prefetch address)
- `prefetch`: Two 16-bit words (`[u32; 2]`) in the CPU prefetch queue (`pf0`, `pf1`)
- `ram`: Array of `[address, byte_value]` pairs (`Vec<[u32; 2]>`)

### 2.3 Transaction Formats (MAME vs Tom Harte)

While CPU state schemas are identical, the two suites format their bus transactions array slightly differently:

#### Tom Harte Transaction Format (`ref_src/SingleStepTests-680x0/`):
- **Bus Cycle:** `[type, duration, fc, addr, size, data]`
  - `type`: `"r"` (read), `"w"` (write), or `"t"` (**TAS indivisible read-modify-write cycle**).
  - `duration`: Transaction length in clock cycles (typically 4 cycles).
  - `fc`: Function code bits (`bit 0 = FC0, bit 1 = FC1, bit 2 = FC2`).
  - `addr`: 24-bit physical address.
  - `size`: `".b"` (byte) or `".w"` (word).
  - `data`: Value on the data bus (0–255 for byte, 0–65535 for word). For `"t"`, records final byte written.
- **Internal / Idle Cycle:** `["n", duration]`
  - CPU internal operations without external bus activity.

#### MAME Transaction Format (`ref_src/SingleStepTests-m68000/`):
- **Bus Cycle:** `[type, start_cycle, fc, addr, size, data, uds, lds]`
  - `type`: `"r"`, `"w"`, `"re"` (read address error), `"we"` (write address error).
  - `uds`, `lds`: Upper and lower data strobe line states (`1` or `0`).
- **Internal / Idle Cycle:** `["n", duration]`

### 2.4 Strict Schema Validation Rules
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
5. **Cycle-by-Cycle Bus Transaction Assertion:**
   - Iterate through `test.transactions` (`[type, duration, size, addr, str_size, data, uds, lds]`):
     - **CCK1 (S0–S3):** Assert the CPU asserts the exact address (`addr`), strobe signals (`uds`, `lds`), and transfer direction (`r` vs `w`).
     - **CCK2 (S4–S7):** Assert that on read, data captured in the latch matches `data`, and on write, data driven on the bus matches `data`.
     - Idle cycles (`n`) assert no active bus strobes for the given clock duration.
6. **Address Error & Exception Handling:**
   - If the test triggers an address error (unaligned word/long access), verify that the CPU:
     1. Pushes the Address Error stack frame (Function Code, Access Address, Instruction Register, Status Register, PC).
     2. Sets the Supervisor bit $S$ in $SR$.
     3. Fetches the exception handler address from Vector 3 (`$00000C`).

---

## 5. Rust Test Harness Implementation (`crates/test_runner`)

The test harness is implemented in the dedicated workspace crate [`crates/test_runner`](../../../crates/test_runner). It executes test vectors directly against `m68000::Cpu` and a dedicated test bus (`TestMemoryBus::new()`):

### 5.1 Module Structure
- **[`schema.rs`](../../../crates/test_runner/src/schema.rs):** Strict deserialization of `SingleStepTest` and `CpuTestState` using `#[serde(deny_unknown_fields)]`.
- **[`runner.rs`](../../../crates/test_runner/src/runner.rs):** Test execution loop, CPU prefetch priming, state comparison, cycle count validation, and RAM byte validation:
  - `run_single_test_detail(test, file_path, index, mode) -> Result<(), TestFailure>`: Executes a single test case with configurable `VerifyMode` (`StateOnly`, `StateAndCycles`, `Full`).
  - `run_test_file(path, limit) -> Result<(usize, usize), Box<dyn Error>>`: Reads plain `.json` test suites (MAME or Tom Harte), running up to `limit` test cases in `StateOnly` mode.
  - `run_test_file_with_mode(path, limit, mode)`: Configurable execution mode enabling cycle-exact and bus transaction verification.
- **[`transactions.rs`](../../../crates/test_runner/src/transactions.rs):** Deserializes and parses transaction logs across Tom Harte and MAME formats, matching recorded bus transactions (read/write/TAS direction, 24-bit address, size, bus value, FC lines, strobe signals) against silicon logs.
- **[`dma_harness.rs`](../../../crates/test_runner/src/dma_harness.rs):** Synthetic Agnus DMA bus contention runner sweeping single-cycle (`run_dma_contention_sweep`) and multi-cycle burst (`run_dma_burst_contention`) stalls across instruction execution phases, validating State Invariance and Cycle Invariance ($C = C_0 + 2 \times \text{wait\_cycles}$).
- **[`diagnostic.rs`](../../../crates/test_runner/src/diagnostic.rs):** Human-readable failure reporting, full CCR flag decomposition ($T, S, I, X, N, Z, V, C$), clock/CCK cycle metrics, and transaction diff formatting.
- **[`reporter.rs`](../../../crates/test_runner/src/reporter.rs):** Persistent results recording in `.test_results/`, differential regression detection, and global summary generation.

### 5.2 CPU State Setup & Execution Flow
```rust
let mut bus = TestMemoryBus::new();
bus.load_test_ram(&test.initial.ram);

let mut cpu = Cpu::new();
cpu.state.d = [test.initial.d0, test.initial.d1, /* ... */];
cpu.state.a = [test.initial.a0, test.initial.a1, /* ... */];
cpu.state.usp = test.initial.usp;
cpu.state.ssp = test.initial.ssp;
cpu.state.sr = test.initial.sr;

// Tom Harte suite initializes PC at start of instruction + 4 due to prefetch queue model
if is_harte {
    cpu.state.pc = test.initial.pc.wrapping_add(4);
} else {
    cpu.state.pc = test.initial.pc;
}
cpu.state.ir = (test.initial.prefetch[0] & 0xFFFF) as u16;
cpu.state.prefetch[0] = (test.initial.prefetch[1] & 0xFFFF) as u16;

// Execute instruction
let _ = cpu.step_instruction(&mut bus);
```

---

## 6. Integration Test Suite Structure (`tests/test_singlestep.rs`)

Integration tests reside in [`crates/test_runner/tests/test_singlestep.rs`](../../../crates/test_runner/tests/test_singlestep.rs). Rather than scattering tests across dozens of individual files, tests use a unified dual-suite runner function `run_dual_test`:

```rust
/// Helper function to execute a test against both MAME and Real 68k (Tom Harte) suites
fn run_dual_test(name: &str, limit: usize) {
    let mame_path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", name);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", name);

    // 1. Validate against MAME suite
    let (mame_passed, mame_failed) = run_test_file(&mame_path, Some(limit))
        .unwrap_or_else(|err| panic!("Failed MAME test '{}': {}", mame_path, err));
    assert_eq!(mame_failed, 0, "MAME tests failed for {}: {}/{} failed", name, mame_failed, mame_passed + mame_failed);

    // 2. Validate against Tom Harte (Real 68k) suite
    let (harte_passed, harte_failed) = run_test_file(&harte_path, Some(limit))
        .unwrap_or_else(|err| panic!("Failed Real 68k test '{}': {}", harte_path, err));
    assert_eq!(harte_failed, 0, "Real 68k tests failed for {}: {}/{} failed", name, harte_failed, harte_passed + harte_failed);
}
```

Tests are grouped into cohesive categories within `test_singlestep.rs`:
- **System & Control Flow:** `test_nop`, `test_rts`, `test_trap`, `test_bcc`, `test_jmp`, `test_jsr`
- **Data Movement:** `test_move_b/w/l`, `test_movea_w/l`
- **Integer Arithmetic:** `test_add_b/w/l`, `test_adda_w/l`, `test_sub_b/w/l`, `test_suba_w/l`
- **Logic & Bit Manipulation:** `test_and_b/w/l`, `test_or_b/w/l`, `test_btst`, `test_bset`, `test_bclr`, `test_bchg`
- **Shifts & Rotates:** `test_asl_b/w/l`, `test_asr_b/w/l`, `test_lsl_b/w/l`, `test_lsr_b/w/l`

---

## 7. Running Tests & CLI Commands

Tests are executed via standard Cargo commands or through the dedicated `test_runner` CLI:

```powershell
# Run all single-step integration tests
cargo test -p test_runner

# Run all tests for a specific opcode across both suites
cargo test -p test_runner -- test_add_b
cargo test -p test_runner -- test_move_w
cargo test -p test_runner -- test_nop

# Check for regressions or improvements against previous test run
cargo run -p test_runner -- --diff

# Display global pass/fail matrix and coverage summary
cargo run -p test_runner -- --summary

# Run a specific opcode suite directly with live diagnostic failure logs
cargo run -p test_runner -- --suite ADD.b
```

---

## 8. In-Code Test Mutation Strategy (Chip/Fast RAM & DMA Contention)

To verify the CPU's bus arbitration and wait-state handling under real Amiga hardware conditions, we do **not** generate duplicate JSON files on disk. Instead, the test runner applies **programmatic test mutations in memory**:

### 8.1 Memory Classification & Gary Bus Equivalence
The test harness classifies accessed memory addresses using `MemoryType`:
- **`MemoryType::ChipRam`**: Contended by Agnus DMA ($000000-$07FFFF); memory bus accesses stall with `BusResult::WaitState` when `chip_ram_blocked` is asserted.
- **`MemoryType::FastRam`**: Uncontended memory; completely immune to Agnus DMA contention, executing with **zero wait states** even during 100% DMA bus stalls.

### 8.2 Full Cartesian Permutation Engine (`run_dma_full_cartesian_permutation`)
Rather than testing a small subset of arbitrary schedules, the test harness evaluates the full combinatorial Cartesian product across both dimensions:
1. **Address Permutation Space ($2^k$):** Memory cells accessed by the instruction are dynamically discovered during an uncontended pre-flight pass (`run_preflight`) and clustered into up to $k \le 4$ functional roles (`cluster_contacts`). Every combination of `ChipRam` vs `FastRam` is swept.
2. **DMA Schedule Permutation Space ($2^M$):** Each CCK cycle $t \in [0, M)$ of the instruction execution window is evaluated across all $2^M$ bit patterns (every possible combination of stalled vs free CCK slots).
3. **Hardware Invariants Verified per Run:**
   - **State Invariance:** CPU registers ($D_0-D_7$, $A_0-A_6$, $USP$, $SSP$, $SR$, $PC$, prefetch) and RAM contents match the golden uncontended run 100% bit-identically across all permutations.
   - **Cycle Invariance:** Contended execution total CPU clocks satisfy $C = C_0 + 2 \times \text{wait\_cycles}$, where wait cycles only accumulate when contested `ChipRam` cells are accessed during an active DMA stall.
   - **Fast RAM Immunity:** When all accessed contacts are assigned `FastRam`, wait states are strictly 0 ($C = C_0$) regardless of active DMA stalls.

### 8.3 Memory Inversion Guard (`invert_chip_ram`)
During stalled CCK cycles, the test bus temporarily inverts contested Chip RAM cells (`bus.invert_chip_ram()`) before stepping the CPU, and re-inverts them afterwards. Any unauthorized read during a wait state captures inverted/corrupted data, and any unauthorized write modifies inverted storage which becomes permanently corrupted upon un-inversion, immediately failing state assertions.

---

## 9. Dual-Suite Cross-Validation & Discrepancy Resolution

Having access to both the MAME and Tom Harte test suites provides an invaluable verification tool for cycle-exact M68000 emulation:

### 9.1 Comparative Matrix
| Operation / Quirk | MAME SingleStepTests (`ref_src/SingleStepTests-m68000/v1/`) | Tom Harte SingleStepTests (`ref_src/SingleStepTests-680x0/68000/v1/`) | Recommended Ground Truth |
| :--- | :--- | :--- | :--- |
| **Standard Instructions** (ADD, MOVE, etc.) | Cycle-exact; verified across 125 operations | Cycle-exact; verified across 125 operations (~8,000 cases each) | **Both must pass** |
| **`TAS` Instruction** | ⚠️ Flawed: Does not simulate 5-cycle indivisible read-modify-write timing | ✅ Accurate: Models indivisible RMW via `"t"` transaction | **Tom Harte Suite** |
| **`TRAPV` Exception** | ⚠️ Flawed: S-bit state quirk in MAME test generator | ✅ Standard 68000 behavior | **Tom Harte Suite** |
| **`ILLEGAL_LINEA` / `LINEF`** | ✅ Included (Vector 10 & Vector 11 tests) | Not included in basic opcode list | **MAME Suite** |
| **`STOP` Instruction** | ✅ Included (Supervisor privileged stop) | Not included in basic opcode list | **MAME Suite** |
| **User vs Supervisor Stack** | Both modes tested | 99% Supervisor mode, 1% User mode | **Both** |
| **`(An)+` Address Error AGU** | ⚠️ Aborts without advancing $A_n$ on address error | ✅ Real Silicon: $A_n$ advances in AGU prior to bus error trap | **Tom Harte Suite** |
| **Address Error Stack Frame PC** | Pushes $PC$ based on internal simulator microcode stage | Pushes target - 4 on jumps / hardware prefetch PC on faults | **Both (accommodated in runner)** |
| **PC-Relative Function Codes** | Uses Program Space (FC 2 / 6) on PC-relative operand faults | Uses Data Space (FC 1 / 5) on certain operand evaluations | **Handled in runner status word check** |

### 9.2 Triangulation Protocol
When diagnosing a test mismatch:
1. **Fails in MAME, Passes in Tom Harte:**
   - Check if the instruction is `TAS`, `TRAPV`, or touches a known MAME microcode generator quirk. If Tom Harte passes and verified against [Moira 3.0](../../../ref_src/Moira-3.0), the core is behaving accurately.
2. **Fails in Tom Harte, Passes in MAME:**
   - Tom Harte generates vastly more random address combinations (~8,000 per opcode). A failure here typically reveals an unaligned address boundary edge case or unhandled condition code combination that MAME's smaller sample missed.
3. **Fails in Both:**
   - Definite implementation bug in decoding, effective address calculation, CCK phase alignment, or CCR flag updates.

### 9.3 Automated Dual-Suite Integration Test Matrix
The integration test suite in [`crates/test_runner/tests/test_singlestep.rs`](../../../crates/test_runner/tests/test_singlestep.rs) executes dual-suite cross-validation on every `cargo test` run. Each opcode test invokes `run_dual_test("<OPCODE>", limit)`, simultaneously validating vectors against:
- **MAME suite** (`ref_src/SingleStepTests-m68000/v1/<OPCODE>.json`)
- **Real 68k / Tom Harte suite** (`ref_src/SingleStepTests-680x0/68000/v1/<OPCODE>.json`)

Currently active dual-suite tests cover all implemented instructions:
- **System & Control**: `NOP`, `RTS`, `TRAP`, `Bcc` (`BRA`), `JMP`, `JSR`
- **Data Movement**: `MOVE.b`, `MOVE.w`, `MOVE.l`, `MOVEA.w`, `MOVEA.l`
- **Arithmetic**: `ADD.b`, `ADD.w`, `ADD.l`, `ADDA.w`, `ADDA.l`, `SUB.b`, `SUB.w`, `SUB.l`, `SUBA.w`, `SUBA.l`
- **Logic**: `AND.b`, `AND.w`, `AND.l`, `OR.b`, `OR.w`, `OR.l`
- **Bit Manipulation**: `BTST`, `BSET`, `BCLR`, `BCHG`
- **Shifts & Rotates**: `ASL.b`, `ASL.w`, `ASL.l`, `ASR.b`, `ASR.w`, `ASR.l`, `LSL.b`, `LSL.w`, `LSL.l`, `LSR.b`, `LSR.w`, `LSR.l`

---

## 10. Diagnostic Logging & Regression Tracking Architecture

To support autonomous developer and agent workflows, the test runner implements cycle-level diagnostic failure logging and persistent differential regression tracking.

### 10.1 Diagnostic Failure Reports
Whenever a test case fails, the runner produces a formatted diagnostic report containing:
- **Test Identification:** Opcode name, source JSON/GZ file, test vector index.
- **Cycle & Timing:** Exact test length in CPU clock cycles and Amiga CCK cycles ($1\ \text{CCK} = 2\ \text{CPU clocks}$).
- **Decomposed CCR Analysis:** Status Register decoded into human-readable flags ($T, S, I, X, N, Z, V, C$) indicating specifically which flags are unexpectedly SET or CLEARED.
- **Register & Memory Diffs:** Detailed expected vs actual values with decimal offsets for registers ($D_0-D_7, A_0-A_7, PC$) and RAM byte addresses.

### 10.2 Differential Regression Tracker (`.test_results/`)
Test outcomes are persisted across runs in the `.test_results/` workspace directory:
- `.test_results/latest/<suite>.json`: Full test results and failing test names from the most recent run.
- `.test_results/previous/<suite>.json`: Prior run snapshot rotated upon execution.
- `.test_results/summary.json`: Aggregated repository coverage matrix with pass rates and active failure cases.

#### Regression & Improvement Detection:
- 🔴 **Regressions:** Tests that previously passed in `.test_results/previous/` but failed in the current run are immediately flagged with `⚠️ [REGRESSION DETECTED]` in the terminal.
- 🟢 **Improvements:** Tests that previously failed but now pass are flagged with `🎉 [PROGRESS / FIX]`.

### 10.3 CLI Inspection Tool
Developers and AI agents can query test status and regressions directly:
```powershell
# Show regressions and fixed tests vs previous run
cargo run -p test_runner -- --diff

# Display global pass/fail matrix across all suites
cargo run -p test_runner -- --summary

# Run a specific opcode suite directly
cargo run -p test_runner -- --suite ADD.b
```