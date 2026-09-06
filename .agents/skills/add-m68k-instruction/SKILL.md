---
name: add-m68k-instruction
description: >-
  Use this skill when implementing a new Motorola 68000 CPU instruction or addressing mode in the emulator. Provides a step-by-step recipe covering opcode decoding, CCK cycle phases, effective address (EA) resolution, condition code (CCR) calculations, prefetch management, and SingleStepTest validation.
---

# Recipe: Implementing an M68000 CPU Instruction

Follow this checklist to implement a new instruction in the cycle-exact M68000 emulator core.

---

## 1. Consult Reference Specifications

Before writing code, verify exact micro-timing and flag behavior:
- **Documentation**: Query `amiga-rag` using `rag_search` for the instruction name and timing in the M68000 Programmer's Reference Manual.
- **Reference Implementations**: When sub-cycle timing or prefetch order is ambiguous, inspect:
  - `ref_src/Moira-3.0/` (clean C++ cycle-exact 68000 implementation)
  - `ref_src/mame-mame0289/src/devices/cpu/m68000/` (authoritative microcoded core)

---

## 2. Opcode Decoding & Dispatch

1. Identify the 16-bit opcode bit pattern, operation size (`.b` = 00/01, `.w` = 01/11, `.l` = 10/10), and operand fields (Data/Address registers, Effective Address mode/register).
2. Wire the opcode into the decoder dispatch table or match tree:
   - Ensure reserved or invalid bit combinations branch to an **Illegal Instruction** exception (Vector 4, `$000010`) or Line-A / Line-F exceptions.

---

## 3. Effective Address (EA) & Color Clock (CCK) Phases

1. Bus cycles are modeled in Color Clock phases (**CCK1** and **CCK2**):
   - $1\ \text{bus access} = 2\ \text{CCK cycles} = 4\ \text{CPU clocks}$.
2. Check memory bus availability on each access:
   - If `bus.read()` / `bus.write()` returns `MemoryBusResult::Blocked`, the CPU must hold its current phase and insert a wait state.
   - If `MemoryBusResult::Ready`, advance to the next execution step.
3. Check for unaligned access on word (`.w`) and long (`.l`) accesses:
   - If `address & 1 != 0`, abort normal execution immediately and initiate the **Address Error** sequence (Vector 3).

---

## 4. ALU Execution & Condition Codes (CCR)

1. Use **wrapping arithmetic** (`wrapping_add`, `wrapping_sub`, `overflowing_add`, etc.) to prevent debug build panics.
2. **Dynamic Cycle Accumulation:** Do not hardcode static cycle constants. Micro-operations naturally advance cycle counters based on operand addressing modes, bit loops (e.g. `DIVU`/`DIVS` or `MULS`/`MULU`), branch conditions, and bus wait states.
3. Update the Condition Code Register (lower byte of `SR`):
   - **X (Extend, bit 4)**: Set by arithmetic instructions; unchanged by logical operations, `MOVE`, or bit operations.
   - **N (Negative, bit 3)**: Set if MSB of result is 1; cleared otherwise.
   - **Z (Zero, bit 2)**: Set if result is 0; cleared otherwise.
   - **V (Overflow, bit 1)**: Set if signed two's complement overflow occurred; cleared on logical ops.
   - **C (Carry, bit 0)**: Set if unsigned carry/borrow occurred; cleared on logical ops.

---

## 5. Prefetch Pipeline Synchronization

1. Any immediate values or 16/32-bit extension words must be consumed from the internal prefetch register (`IRC`), advancing `PC` by 2.
2. Refill the prefetch queue from the new `PC` address.
3. Before finishing the instruction, ensure the next opcode is prefetched into `IRC` and transferred to `IR`, leaving `PC` pointing to $Next\_Opcode + 2$.

---

## 6. Verification with SingleStepTests

1. Locate the test file in `ref_src/SingleStepTests-m68000/v1/<INSTRUCTION>.<size>.json`.
2. Add a corresponding test in `tests/cpu/test_<instruction>.rs`:
   ```rust
   use crate::tests::cpu::common::run_test_file;

   #[test]
   fn test_my_instruction() {
       run_test_file("ref_src/SingleStepTests-m68000/v1/MY_INSTR.w.json");
   }
   ```
3. Run the test:
   ```powershell
   cargo test tests::cpu::test_my_instruction
   ```
4. If assertions fail, activate the `m68k-singlestep-test` skill to diagnose the mismatch.

---

## 7. Update Design Documentation (Definition of Done)

If implementing or debugging this instruction revealed, clarified, or modified any architectural assumption (e.g. prefetch timing, condition code quirks, or bus wait-state behavior), update [CPU Motorola M68000.md](../../../Obsidian/Amiga/Design/CPU%20Motorola%20M68000.md) or [MemoryBus.md](../../../Obsidian/Amiga/Design/MemoryBus.md).
