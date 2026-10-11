---
title: "Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine"
aliases: ["CPU Micro-Step State Machine", "Micro-Step State Machine", "68000 Microcode"]
tags: ["amiga", "design", "m68000", "microstep", "cycle-exact"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-08
updated: 2026-10-11
related: ["[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/cpu/src/micro"
---
# Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine

The emulator executes a static opcode descriptor as a sequence of phase actions.
One active `step_cck` call consumes two CPU clocks. An AddressBus refusal retains
the active transfer and retries it on a later CCK; it does not repeat its entry
ALU callback or commit a refused access. Dynamic branches and exceptions select
another static slice, while MOVEM maintains an explicit transfer-loop state.

This is the project's execution contract. It does not identify a hardware
microrom/nanorom circuit or expose all electrical pins. The
[CPU specification](CPU%20Motorola%20M68000.md) owns instruction semantics,
registers, prefetch/PC and exception requirements. The
[silicon verification specification](Verification/CPU%20SingleStepTests.md)
owns comparator strength and limits. [MemoryBus](MemoryBus.md) and
[Machine Loop](Machine%20Loop.md) own physical routing and coordination.

## 1. Core Architecture & Execution Model

The [dispatch table](../../../crates/cpu/src/micro/dispatch_table.rs) contains
65,536 descriptors with a static slice and source/destination register parameters.
Compile-time decoding specializes transfer sizes and addressing forms. Every
unassigned encoding receives the common decode-exception slice; high nibbles A/F
select emulator-line vectors. Table coverage is not proof of every undocumented
instruction/fault behavior.

`Cpu::initiate_current_instruction` caches the selected slice and samples T.
The [CCK engine](../../../crates/cpu/src/cpu.rs) indexes that slice until
retirement or a dynamic redirect. All runtime temporaries have fixed storage;
hot stepping does not allocate. Indirect calls, readiness checks and dynamic
conditions remain. Throughput claims require profiling, not descriptor shape.

The ALU callback runs once when its step starts. It can be fused into a timed
CCK, consume no clocks in a fall-through step, or establish a dynamic countdown.
Fusing internal work changes dispatch placement without asserting the same
arithmetic instant on silicon. Operand ordering and fault-visible side effects
remain constrained by the accepted observations.

## 2. Data Structures & Type Definitions

### 2.1 The AluFn Function Pointer

AluFn receives CpuState and decoded source/destination indices. It never reads
the bus: registers and staged memory/immediate operands supply its inputs.
It writes results, CCR, effective addresses or a selected execution path.
Zero-clock callbacks continue within the current CCK; timed callbacks run once
before the first transfer attempt and are not repeated after a wait.

### 2.2 The MicroStep Descriptor

[`MicroStep`](../../../crates/cpu/src/micro/types.rs) contains optional BusFn and
AluFn pointers plus base_clocks. A CCK action normally has two base clocks.
An internal idle can count several CCKs. A zero-clock ALU action has no bus
transfer; a dynamic transfer handler with zero base clocks explicitly manages
its own progress. The engine distinguishes these cases rather than treating
every descriptor as exactly one CCK.

### 2.3 Specialized 2-Clock Atomic Micro-Step Handlers (BusFn)

BusFn receives Cpu and AddressBus and returns Ready or WaitState. Size-specific
functions avoid decoding operand size at each transfer. Operand and prefetch
handlers live in [step_execution](../../../crates/cpu/src/micro/step_execution.rs);
stack/vector/target handlers live in [step_control](../../../crates/cpu/src/micro/step_control.rs).
The [shared constants](../../../crates/cpu/src/micro/common.rs) name their
phase roles and are reused by instruction-class sequences.

### 2.4 CPU Micro-State Storage (CpuMicroState)

| Fields | Role |
| --- | --- |
| source, destination | Decoded operands/results, reused for captured PC/SR during frame writes. |
| irc | Incoming prefetch/target opcode, separate from the active descriptor. |
| ea_addr, ea_high | Resolved operand/target address and high-word assembly. |
| addr1, addr2 | Separate addresses for paired operands and partial transfers. |
| reg_src, reg_dst | Decoded register indices; interrupt entry reuses reg_src for its accepted level. |
| current_steps, micro_step | Active static slice and position. |
| clocks_remaining | Remaining clocks in the active phase/countdown; wait does not decrement it. |
| target_refill, prefetch_retired | Mutually interpreted retirement state for target or already advanced sequential refill. |
| movem_mask, movem_state | Immutable register mask and cursor/phase/dummy-read progress. |
| fault_addr, info_word, ssp_base | Address Error capture and original supervisor-stack base. |

The [micro-state definition](../../../crates/cpu/src/micro/engine.rs) is serialized
except current_steps, which is a runtime static pointer. The present restore API
reconstructs the opcode slice from IR. It does not encode an arbitrary redirected
branch/exception slice; restoring a snapshot halfway through such a sequence is
a known capability gap, distinct from instruction-boundary state restoration.

### 2.5 The 65,536 Static Dispatch Universe

The table is static binary data; each instruction caches its slice. No host code
generation, guest page tracking or decoded-code invalidation is required.
Prefetch still retains guest words already read before a self-modifying store.
CCR and arithmetic accessors use explicit wrapping and decoded big-endian values.

### 2.6 Addressing Mode Naming Convention & Canonical Module Layout

#### Canonical Addressing Mode Identifiers
To maintain rigorous execution efficiency and consistency with the Motorola M68000 Programmer's Reference Manual, static micro-step array slices and effective address functions follow standardized addressing mode identifiers:

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

#### Standardized Common Micro-Step Constants (`crates/cpu/src/micro/common.rs`)
All instruction modules reuse shared atomic Color Clock micro-step primitives rather than duplicating local slice definitions:
- **Bus & Internal Idle Primitives:** `BUS_WRITE_IDLE` (CCK1 write setup / bus idle), `BUS_READ_IDLE` (CCK2 read completion / bus idle), `ALU_IDLE` (internal processing 2-clock delay / bus idle), `ALU_IDLE_4CLK` (4-clock internal execution delay / bus idle), `ALU_IDLE_8CLK` (8-clock internal exception delay / bus idle), `RESET_ASSERT_IDLE` (124-clock external `RESET` assertion in reset.rs).
- **Operand Writes:** `WRITE_DST_BYTE`, `WRITE_DST_WORD`, `WRITE_DST_LONG_HIGH`, `WRITE_DST_LONG_LOW`.
- **Operand Reads:** `READ_SRC_BYTE`, `READ_SRC_WORD`, `READ_SRC_LONG_HIGH`, `READ_SRC_LONG_LOW`, `READ_DST_BYTE`, `READ_DST_WORD`, `READ_DST_LONG_HIGH`, `READ_DST_LONG_LOW`, `BUS_READ_IDLE` (shared CCK2 idle completion).
- **Dual Staged Address Reads & Writes:** `READ_ADDR1_BYTE`, `READ_ADDR1_WORD`, `READ_ADDR1_LONG_HIGH`, `READ_ADDR1_LONG_LOW`, `READ_ADDR2_BYTE`, `READ_ADDR2_WORD`, `READ_ADDR2_LONG_HIGH`, `READ_ADDR2_LONG_LOW`, `WRITE_ADDR2_BYTE`, `WRITE_ADDR2_WORD`, `WRITE_ADDR2_PD_LONG_HIGH`, `WRITE_ADDR2_PD_LONG_LOW`.
- **Prefetch & Extension:** `FETCH_EXT_READ`, `FETCH_EXT_FINISH`, `PREFETCH_IRC_READ`, `PREFETCH_IRC_FINISH`, `PREFETCH_NEXT_READ`, `BUS_READ_IDLE` (prefetch next finish / retirement).
- **Control Flow Refills:** `READ_TARGET_OPCODE_READ`, `BUS_READ_IDLE` (target opcode read finish), `PREFETCH_TARGET_READ`, `PREFETCH_TARGET_FINISH`.
- **Stack Operations:** `PUSH_STACK_HIGH_IDLE` (CCK1 push setup / bus idle), `PUSH_STACK_HIGH_WRITE`, `PUSH_STACK_LOW_WRITE`, `POP_STACK_HIGH_READ`, `POP_STACK_HIGH_FINISH`, `POP_STACK_LOW_READ`, `POP_STACK_LOW_FINISH`.
- **Exception Push Setup:** `EXCEPTION_PUSH_PCLO_IDLE`, `EXCEPTION_PUSH_SR_IDLE`, `EXCEPTION_PUSH_PCHI_IDLE`, `AERR_PUSH_*_IDLE` (CCK1 exception push setup / bus idle).

#### Canonical Layout for Instruction Modules
To maintain structural consistency and clean codebase navigation across all instruction files, every module adheres to the standard sequence:
1. **Module Doc Comment:** High-level summary of mnemonic, addressing modes, and timing.
2. **Imports:** Imports from `crate::micro::common::*`, `crate::state::CpuState`, `crate::micro::ea`.
3. **Leaf ALU Functions:** Direct arithmetic/logic helpers (`#[inline(always)] fn add_w(...)`, `sub_b(...)`) called directly by ALU callbacks to execute branchless wrapping math and CCR flag updates.
4. **Micro-Step ALU Callbacks (`AluFn`):** Functor callbacks (`fn alu_...`) stored as function pointers in `MicroStep.alu_fn`, dispatched dynamically via the micro-step engine.
5. **Specialized Micro-Step Handlers (`BusFn`):** Instruction-specific bus/execution handlers (if any required).
6. **Static Micro-Step Arrays:** `pub static STEPS_...: [MicroStep; N] = [...]`, ordered strictly by canonical addressing mode progression (`DN`, `AN`, `AI`, `PI`, `PD`, `D16_AN`, `IDX_AN`, `ABSW`, `ABSL`, `D16_PC`, `IDX_PC`, `IMM`). ALU and effective-address callbacks use timed fusion or zero-clock fall-through according to their operand dependencies.
7. **Opcode Decoder:** Fast compile-time function `pub const fn decode_..._steps(...) -> Option<&'static [MicroStep]>`.

---

## 3. Parametric ALU Handlers

Decoded register indices allow one operation callback to serve multiple register
combinations. Register results use size-specific setters; memory results remain
in destination until a successful write. EA callbacks use the same entry
mechanism. This reduces duplication without a claimed fixed reduction ratio.
Both timed fused callbacks and zero-clock callbacks are supported by the engine;
instructions use the form matching operand arrival and execution structure.

## 4. Memory Bus Operations & Strobe Semantics

### 4.1 Direct 2-Phase Bus Architecture (Zero Dynamic Strobe Boilerplate)

| Phase | CPU callback | Success | Refusal |
| --- | --- | --- | --- |
| Read CCK1 | read_byte/read_word | Latch data, then advance to finish. | Preserve active phase and pending state. |
| Read CCK2 | Finish/idle | Apply defined deferred pointer/PC updates; no second read. | No memory callback in an ordinary finish. |
| Write CCK1 | Setup/alignment | Establish address/result and check odd word address; no write. | No memory callback in ordinary setup. |
| Write CCK2 | write_byte/write_word | Commit data and any success-dependent update. | Retain address, data, phase and countdown. |

Each refused active CCK adds two clocks. Entry ALU is not repeated because its
countdown remains initialized. A redirected fault starts its exception sequence
in the rejected CCK, with no extra clock increment or rejected memory callback.
The callback convention corresponds to the vector matcher at CCK resolution;
it is not electrical data sampling at S6/S7 or proof of pin-level DMA ownership.

### 4.2 Byte Strobe Activation & Byte Preservation

Even bytes select the high lane and odd bytes the low lane; the other byte is
preserved. Byte accesses do not raise alignment errors. Word transfers require
even addresses. A7 byte postincrement/predecrement uses two bytes to preserve
stack alignment. AddressBus expresses transfer width and address, not independent
UDS/LDS or FC pin transitions.

### 4.3 32-Bit Long Decomposition

Longs use two word callbacks with explicit assembly/decomposition. Ordinary
loads acquire high then low. Instruction classes select store order: ordinary
MOVE stores high then low; long arithmetic/logic/unary RMW and predecrement MOVE
write low before high. Partial faults retain completed transfers and applicable
register changes; a bulk host copy cannot replace these bus operations.

### 4.4 Operation Comparison Matrix

| Operation | Read phases | Write phases | Special boundary |
| --- | --- | --- | --- |
| Byte/word read | CCK1 callback, CCK2 finish | None | Odd word aborts before callback. |
| Byte/word store | None unless RMW | CCK1 setup, CCK2 callback | Deferred updates commit after success. |
| Long | Two word pairs | Two word pairs if storing | Partial-transfer state is visible. |
| TAS memory | Read pair, modification interval | Write pair | Ten-clock combined interval; continuous AS/locking is not exposed. |

### 4.5 Function Codes & Information Word

The [CPU frame specification](CPU%20Motorola%20M68000.md#51-address-error-vector-3-7-word-stack-frame)
owns R/W, I/N, FC and frame layout. Source/program flags choose the captured
information word; they do not alone identify the execution phase or saved PC.
Live electrical FC lines, BERR and device IACK are outside the AddressBus API.

## 5. Control Flow, Pipeline Refill & Complex Addressing

### 5.1 The Two-Word Pipeline Refill

Sequential prefetch uses `common::PREFETCH_NEXT_READ` and `common::BUS_READ_IDLE`.
Instruction retirement shifts `ir = prefetch`, `prefetch = irc`, and `pc += 2`.
The class sequence determines whether retirement follows prefetch immediately or
waits for remaining internal clocks or writeback.

When control flow changes (`JMP`, `JSR`, `RTS`, `BRA`, taken `Bcc`):
- The existing prefetch pipeline contents are **flushed**.
- The CPU must perform a **two-word pipeline refill** directly from the new target address in Program Space (`FC2` User / `FC6` Supervisor):
  1. **Refill Bus Cycle 1 (`READ_TARGET_OPCODE_READ` & `BUS_READ_IDLE`)**:
     - Bus read from `ea_addr` (4 CPU clocks / 2 CCKs).
     - Data is latched into `state.micro.irc` (the new opcode).
  2. **Refill Bus Cycle 2 (`PREFETCH_TARGET_READ` & `PREFETCH_TARGET_FINISH`)**:
     - Bus read from `ea_addr + 2` (4 CPU clocks / 2 CCKs).
     - Data is latched into `state.prefetch`.
     - `state.ir` is loaded with `irc`.
     - `state.pc` is initialized to `ea_addr + 4`.
     - Instruction retires with target refill (`target_refill = true`, `step_cck` returns `true`).

`BRA`, taken `Bcc`/`DBcc`, `BSR`, `JMP`, `JSR`, `RTS`, `RTR` and `RTE` use
`READ_CONTROL_TARGET_OPCODE_READ` for the first read. Its successful transfer
uses the same [shared read phases](#23-specialized-2-clock-atomic-micro-step-handlers-busfn).
An odd target begins Address Error without a target memory callback: the frame
captures `target - 4` as PC, the full target as the fault address, and sets bit 3
(instruction-not) alongside the program-space function code. `RTE` restores SR
before this check, so the saved status and function code follow the restored mode.
`BSR` completes both return-address writes before checking target alignment;
`JSR` checks the target before those writes. Untaken branches retain their
sequential stream without checking the unused branch destination.

### 5.2 Control Targets and Stack Operations

The [call](#class-6-subroutine-call--stack-push-jsr-ea),
[return](#class-7-subroutine-return--stack-pop-rts),
[jump](#class-8-unconditional-jump-jmp-ea) and
[branch](#class-9-conditional-branching-bcc-dynamic-path-selection) classes own
their ordering. JMP performs two target reads. JSR reads the target opcode
before its return-address writes, then fetches target+2. BSR writes its return
address before testing its target. RTS reads PC high then low; RTR/RTE read PC
high, status, PC low. RTE selects stack banking only after completing the old
supervisor-stack reads. Alignment checks occur at each actual transfer.

### 5.6 Effective Address Calculation with Extension Word Prefetch

Worked input: ADD.W D0,(d16,A0), opcode $D168, displacement already in prefetch.
Starting with cursor A+4, A0=$2000, displacement=$0010 and D0=$0001:

| CPU clocks | Phase | Observable action |
| --- | --- | --- |
| 0-4 | Extension pair | Calculate EA=$2010 from the staged extension; read A+4 at clock 2, advance cursor at finish. |
| 4-8 | Operand pair | Read word $2010 at clock 6 into destination. |
| 8-12 | Prefetch pair | Compute ADD and CCR; read A+6 at clock 10, then retire staged refill before writeback. |
| 12-16 | Store pair | Write the result to $2010 at clock 16; complete instruction. |

The ALU placement is an emulator decision. The four bus intervals and their
order follow the ADD.w supplied vectors and the living ADD sequence. Shared
read/write phase behavior is defined in Section 4.1; no extra ALU interval is
inserted between operand acquisition and prefetch.

### 5.7 Indexed Addressing with Internal Calculation Clocks

Worked input: ADD.W (8,A0,D1.W),D0, opcode $D070, A0=$2000, D1.W=$0010,
extension=$1008, cursor A+4. The EA is $2018. Its two-clock calculation prefix
precedes extension refill (callback clock 4), operand read (clock 8) and final
prefetch (clock 12), completing at clock 14. Signed word indexes extend to 32
bits; brief MC68000 extensions have no scale/full-format interpretation.
The prefix and callback timestamps match the indexed ADD.w sequence; do not
infer a hidden three-input silicon adder from the emulator formula.

### 5.8 `MOVEM` Dynamic Register Transfer Engine

In `MOVEM <ea>, reglist` and `MOVEM reglist, <ea>`, the opcode specifies the transfer direction, size (Word vs. Long), and effective addressing mode. However, the **number of registers transferred (0 to 16)** is dynamic, specified by the 16-bit **extension mask word**:

```
Bit:   15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
Reg:   A7  A6  A5  A4  A3  A2  A1  A0  D7  D6  D5  D4  D3  D2  D1  D0   (Standard / Postincrement)
Reg:   D0  D1  D2  D3  D4  D5  D6  D7  A0  A1  A2  A3  A4  A5  A6  A7   (Predecrement -(An) mode)
```

#### The `MovemTransfer` Loop Mechanics:
Because a static array cannot predict the number of registers at compile time, `MOVEM` is modeled via the atomic **`execute_movem_transfer`** sub-loop:
1. **Step 0 (`FetchExtension`)**: Reads the 16-bit register mask from `PC` into `state.micro.movem_mask`, advancing `PC += 2`.
2. **Step 1 (`Alu`)**: Resolves `ea_addr = An` (or calculates displacement/index).
3. **Step 2 (`MovemTransfer`)**:
   - Scans set mask bits from low to high using the cursor in `movem_state`; the mask remains unchanged. Predecrement uses the reversed register mapping above.
   - In CCK1/CCK2, performs the atomic bus transfer (Read or Write, Word or 2-cycle Long) between register $R_k$ and memory at `ea_addr`.
   - Adjusts `ea_addr`: increments `ea_addr += size` (or decrements `ea_addr -= size` for `-(An)`).
   - The final store CCK completes the loop and commits the predecrement address without another internal interval. An empty store mask starts prefetch directly in the first transfer CCK.
   - Loads perform a final dummy word read at the next address before prefetch. Postincrement commits that address after the dummy read; the address register in the mask cannot replace the final pointer.
   - **Loop Condition**: While selected registers or the final load dummy read remain, `micro_step` does not advance. A wait state preserves the active word and CCK phase.
4. **Step 3 (`PREFETCH_NEXT_READ`)**: Standard sequential opcode prefetch, completing retirement via `retire_current_instruction()`.

#### Exact Hardware Timing:
- **Base Overhead**: With the opcode already prefetched, indirect stores use 8 CPU clocks (mask fetch and final prefetch); loads use 12, including the final dummy operand read. Address extensions and index calculation add their prefix clocks.
- **Per-Register Transfer**:
  - **Word (`.W`)**: Exactly **4 CPU clocks (2 CCKs)** per transferred register.
  - **Long (`.L`)**: Exactly **8 CPU clocks (4 CCKs: 2 bus cycles)** per transferred register.
- **Example (`MOVEM.W (A0)+, D0-D2/A0-A1` — 5 registers)**:
  $12 + 5 \times 4 = \mathbf{32\ \text{CPU clocks}}\ (16\ \text{CCKs})$, for this uncontended execution structure.

---

### 5.9 Cycle-Exact Hardware Exception Processing & Stacking

Frame content and PC selection belong to the [CPU specification](CPU%20Motorola%20M68000.md#5-exception-processing--vectors).
The common three-word sequence captures PC/SR, writes PC low/SR/PC high, reads
vector high/low, reads target opcode and target+2, then retires a target refill.
Decode, privilege and trace reuse this structure with distinct setup callbacks.
Interrupt adds ten clocks before the shared frame, including an internal
four-clock IACK interval; the accepted level is held in reg_src.

Address Error uses a dedicated seven-write sequence and original ssp_base;
odd SSP or odd vector-3 handler halts. Ordinary short-frame handler faults can
redirect to Address Error. The API has no Bus Error termination signal, so a
BERR frame or BERR during stacking/vector/reset is outside supported execution.

### 5.10 Stack Pointer Predecrement Alignment Quirk

A7 byte operations move by two. Word/long alignment rejection is ordered with
the applicable address update, not an automatic rollback. Paired-memory word/
long instructions resolve/read the source before modifying the destination
address register. Shared code must preserve this fault-visible dependency.

## 6. The Execution Engine & Stepping Model (step_cck)

The engine increments the wrapping clock counter by two on each active CCK,
initializes a new step countdown, runs its entry ALU once, follows any slice
redirect in the same CCK, and executes one timed action. Ready decrements the
countdown; WaitState returns without decrement or phase advancement. A zero-
clock ALU continues within the same CCK. MOVEM handlers can loop within a slice
while controlling their own word/phase state.

Completed sequential retirement advances the queue unless an earlier step
already did so; target retirement commits IR/cursor from the target refill.
Then instruction_pc is set, temporaries reset, pending trace queued before an
interrupt, or the next descriptor initiated. STOP clears its slice without
shifting an unfilled queue; pending trace is queued at that completion boundary.
Stopped/halted idle calls perform no CPU bus activity or CPU clock increment.

`step_instruction` loops over active CCKs until one instruction or queued
exception completes; its returned clocks include waits. It does not promise
instruction plus every queued exception in one call. Debugger priming and host
reset use debug reads and do not establish timed hardware reset behavior.

## 7. Concrete Micro-Step Traces by Architectural Class

The classes below retain worked execution structures and their distinct
variants. They share Section 4 phase conventions. Totals describe aligned,
uncontended paths unless a fault is explicitly stated; silicon vectors constrain
callback order and results, not the instant of every internal ALU operation.

### Class 1: Register-to-Register Operations (Pure Internal ALU, Zero Memory Writes)

#### Archetype: Register ADD (`ADD.W D1,D0`, Opcode `$D041`)
- **Variants**: `ADD.b/w Dn,Dn`, `ADD.w An,Dn`, and `ADD.l Dn/An,Dn` in [add.rs](../../../crates/cpu/src/instructions/add.rs).
- **Total Duration**: Byte/word: 4 CPU clocks (2 CCKs). Long: 8 CPU clocks (4 CCKs).
- **Result Writing**: Register destination (`Dn`). Zero external write cycles.
- **Prefetch**: Single sequential prefetch before the long variant's four internal clocks.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `PREFETCH_NEXT_READ` + `alu_fn` | 2 (CCK1) | Reads from `PC` into `irc`. Fused ALU adds the source register to `Dn` and updates CCR. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Completes the prefetch bus cycle. Byte/word variants retire here. |
| **2 (long only)** | `ALU_IDLE_4CLK` | 4 (2 internal CCKs) | Bus remains idle; the long variant retires after this delay. |

Retirement advances `IR = prefetch`, `prefetch = irc`, and `PC += 2`. For long
ADD, the four-clock internal interval follows the prefetch read/idle pair, as
specified by the [ADD.l silicon vectors](../../../ref_src/SingleStepTests-680x0/68000/v1/ADD.l.json).
The fused ALU callback is an implementation choice; the vectors constrain bus
timing and final state rather than the internal arithmetic instant.

Register `SUB`, `ADDX` and `SUBX` use the same size-dependent layout.
Other register variants differ only in their internal tail after prefetch:

| Variant | Internal tail clocks | Total clocks |
| --- | ---: | ---: |
| `CLR/NEG/NEGX/NOT.b/w Dn` | 0 | 4 |
| `CLR/NEG/NEGX/NOT.l Dn` | 2 | 6 |
| `ABCD/SBCD Dy,Dx`; `NBCD Dn` | 2 | 6 |
| `ADDQ/SUBQ.l #q,Dn`; `ADDQ/SUBQ.w #q,An` | 4 | 8 |
| `ADDQ/SUBQ.l #q,An` | 2 | 6 |
| `ADDA/SUBA.w/l Dn/An,An` | 4 | 8 |

Register comparisons use this prefetch-first layout with a two-clock tail for
`CMP.l` and `CMPA.w/l`; byte/word `CMP` and all register `TST` sizes have no tail.
Comparison callbacks update CCR without writing the operand registers.

Register bit modifications complete prefetch before their internal delay;
selecting a bit in the upper 16-bit word adds one internal CCK. Capture the
original bit number before writing the destination so register aliases preserve
this timing. Exact instruction timings follow the
[SingleStep vectors](Verification/CPU%20SingleStepTests.md).

---

#### System register and suspended-execution variants

`MOVE An,USP` and `MOVE USP,An` check privilege before changing either register,
then use four-clock sequential prefetch. MC68000 `MOVE SR,Dn` is unprivileged
and has a two-clock internal tail after prefetch.

[`STOP`](../../../crates/cpu/src/instructions/stop.rs) has two internal CCKs
and no bus transfer. The first checks privilege and loads the staged immediate
SR; the second sets `stopped`, records the following instruction PC, and clears
the active sequence of micro-steps without shifting an unfilled prefetch queue. Rejected
privilege redirects before the SR load. Trace or interrupt entry refills the queue.

[`RESET`](../../../crates/cpu/src/instructions/reset.rs) checks privilege, spends
four internal clocks with the output released, asserts the CPU-owned output for
124 clocks, then releases it before four-clock sequential prefetch. Waiting
on that prefetch does not extend assertion. Total: 132 clocks plus bus waits.
The [machine coordinator](Machine%20Loop.md) observes the assertion edge and
resets external devices without clearing the CPU output.

### Class 2: Immediate to Register Operations (Extension Prefetch + ALU, Zero Memory Writes)

#### Archetype: `ORI.B #$42, D0` (Opcode `$0000`)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: Register destination (`D0`). Zero external write cycles.
- **Prefetch**: Consume the immediate word already in `prefetch` while fetching the following word; the second bus cycle completes the sequential prefetch.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FETCH_EXT_READ` + `alu_fn` | 2 (CCK1) | Fused ALU consumes the staged immediate, computes `D0 = D0 \| imm` and sets CCR; reads the following word from `PC`. |
| **1** | `FETCH_EXT_FINISH` | 2 (CCK2) | Latches the fetched word into `prefetch`, advances `PC += 2`. |
| **2** | `PREFETCH_NEXT_READ` | 2 (CCK1) | Reads the next instruction word from `PC` into `irc`. |
| **3** | `BUS_READ_IDLE` | 2 (CCK2) | Prefetch finishes. Retires: `IR = prefetch`, `prefetch = irc`, `PC += 2`. |

Long immediate arithmetic (`ADD/SUB/ADDI/SUBI.l #imm,Dn`, `ADDA/SUBA.l #imm,An`)
uses two extension pairs, then prefetch and a four-clock internal tail: 16 clocks.
`ADDA/SUBA.w #imm,An` uses one extension pair and the same tail: 12 clocks.

Long immediate comparisons use two extension pairs, then prefetch and a
two-clock tail. `CMP.l #imm,Dn` and `CMPI.l #imm,Dn` share this ordering;
the immediate is staged before the comparison callback runs with prefetch.

---

#### Status immediate variant

`ANDI/ORI/EORI` to CCR/SR consume one extension pair, apply status in an
eight-clock internal interval, then reread both queue words: `REFILL_FIRST_READ`
at `PC - 2`, followed by `REFILL_SECOND_READ` at `PC`. Total: 20 clocks. SR forms
check privilege before extension acquisition; CCR preserves the system byte.
The first reread deliberately repeats the address fetched during extension
consumption. Both pairs use shared read completion and sequential retirement.

### Class 3: Memory Source to Register Load (Memory Read, Zero Memory Writes)

#### Archetype: `ADD/SUB.b/w <ea>,Dn` (`ADD.W (A0),D0`, Opcode `$D050`)
- **Total Duration**: 8 CPU clocks for indirect/postincrement, 10 for predecrement, 12 for displacement, and 14 for indexed addressing.
- **Result Writing**: Register destination (`Dn`). Zero external write cycles.
- **Prefetch**: Address phase, operand read, then next opcode prefetch.

Addressing modes add the following prefix before the common read/prefetch pipeline.
Class 5 uses the same address-phase layout for memory destinations.

| Addressing mode | Prefix clocks | Address phase before the operand read |
| --- | ---: | --- |
| `(An)`, `(An)+` | 0 | Fuse `ea_calc` into the operand read CCK; postincrement also updates `An` once. |
| `-(An)` | 2 | Internal `ea_calc` commits the decrement once; the following word/long read checks alignment. |
| `(d16,An)` | 4 | `FETCH_EXT_READ` with `ea_calc` using the staged extension word, then `FETCH_EXT_FINISH`. |
| `(d8,An,Xn)` | 6 | Internal indexed `ea_calc` CCK, then `FETCH_EXT_READ` and `FETCH_EXT_FINISH`. |
| `(xxx).w` | 4 | Calculate the sign-extended absolute address from the staged word while fetching the following word; finish the extension read. |
| `(xxx).l` | 8 | Two extension read/finish pairs assemble the high and low address words. |
| `(d16,PC)` | 4 | Calculate the operand address from the displacement-word address and staged displacement; finish the extension read. |
| `(d8,PC,Xn)` | 6 | Internal indexed address calculation, then the extension read/finish pair. |

Byte A7 operands decrement by two; other byte address registers decrement by
one. Word operands decrement by two; single-operand long variants decrement by
four. Long operands use two four-clock reads, high word then low word, before
the instruction-specific ALU/prefetch tail.

The frame-load variant [`UNLK`](../../../crates/cpu/src/instructions/unlk.rs)
sets SP from the selected frame register when the address is aligned, reads the
saved register high word then low word with a complete read pair for each,
and prefetches: 12 clocks. At prefetch entry it advances SP by four, then writes
the saved value to An; for `UNLK A7`, that final write supplies the active SP.
CCR is unchanged.

| Pipeline step after the address phase | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `READ_SRC_BYTE/WORD` | 2 (CCK1) | Reads from `ea_addr` into `source`. Indirect/postincrement resolves the address here. A stalled read does not repeat address calculation. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Completes the operand read bus cycle. |
| **2** | `PREFETCH_NEXT_READ` + `alu_fn` | 2 (CCK1) | Reads the next instruction word into `irc`; fused ALU adds `source` to `Dn` and updates CCR. |
| **3** | `BUS_READ_IDLE` | 2 (CCK2) | Completes prefetch and retires: `IR = prefetch`, `prefetch = irc`, `PC += 2`. |

Arithmetic variants retain operand reads before prefetch and place their internal
tail after it. Costs below exclude the shared address prefix:

| Variant | Operand clocks | Internal tail clocks | Total with prefetch |
| --- | ---: | ---: | ---: |
| `ADD/SUB.l` | 8 | 2 | 14 |
| `ADDA/SUBA.w` | 4 | 4 | 12 |
| `ADDA/SUBA.l` | 8 | 2 | 14 |

Read-only comparisons and tests reuse this address/read/prefetch layout without
destination write-back. Long `CMP` and both `CMPA` sizes retire after a two-clock
tail; `TST` has no tail. Memory `CMPI` consumes its immediate extension words
before the address prefix and operand reads, then prefetches without a tail.

#### Status load and bounds-check variants

`MOVE <ea>,SR/CCR` acquires the word through this class's address/read prefix,
applies status in a four-clock internal interval, then rereads both queue words
using the Class 2 status refill pair. Register sources omit the operand read;
immediate sources consume an extension pair. SR checks privilege before operand
side effects. Operand Address Error precedes status replacement and queue refill.

[`CHK`](../../../crates/cpu/src/instructions/chk.rs) uses the same operand prefix,
then a complete sequential prefetch pair before checking the bound. A normal
result retires after six internal clocks. Above-bound and negative-only results
select the [three-word exception class](#class-13-three-word-software-exception-entry).
Indexed operands use one internal address CCK, matching the shared prefix.

#### Dual-memory read variant: `CMPM (Ay)+,(Ax)+`

[`cmpm.rs`](../../../crates/cpu/src/instructions/cmpm.rs) reads the source before
the destination, then prefetches and retires without writes or an internal tail.
Long operands read high word then low word for each operand. Byte addresses can
be staged together; word/long destination calculation waits until source reads
succeed, preserving its address register on a source Address Error. Each
postincrement is committed once, including same-register operands and byte A7
alignment. The compare updates CCR while preserving X.

---

### Class 4: Register to Memory Store (Word and 32-Bit Long Write)

#### Archetype 4A: `MOVE.W D0, (A0)` (Opcode `$3080` — 16-Bit Word Store)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: Single 16-bit Word write cycle (`WRITE_DST_WORD`) at `(A0)`.
- **Prefetch**: Opcode prefetch follows write.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `WRITE_DST_IDLE` + `alu_fn` | 2 (CCK1) | Resolves `ea_addr = A0`, latches `destination = D0 as u32`, sets CCR flags ($N, Z, V=0, C=0$), and checks word alignment. No memory callback. |
| **1** | `WRITE_DST_WORD` | 2 (CCK2) | Drives 16-bit word `(destination & 0xFFFF)` to `ea_addr`. Both $\overline{\text{UDS}}$ and $\overline{\text{LDS}}$ assert. Stalls on Chip RAM contention. |
| **2** | `PREFETCH_NEXT_READ` | 2 (CCK1) | Initiates prefetch from `PC` into `irc`. |
| **3** | `BUS_READ_IDLE` | 2 (CCK2) | Prefetch finishes. Retires: `IR = prefetch`, `prefetch = irc`, `PC += 2`. |

#### Archetype 4B: `MOVE.L D0, (A0)` (Opcode `$2080` — 32-Bit Long Store Decomposition)
- **Total Duration**: 12 CPU clocks (6 CCKs).
- **Result Writing**: Two sequential 16-bit Word write cycles (`WRITE_DST_LONG_HIGH` followed by `WRITE_DST_LONG_LOW`).
- **Prefetch**: Opcode prefetch follows the second write cycle.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `WRITE_DST_IDLE` + `alu_fn` | 2 (CCK1) | Resolves `ea_addr = A0`, latches `destination = D0` (32 bits), sets CCR flags, and checks word alignment. No memory callback. |
| **1** | `WRITE_DST_LONG_HIGH` | 2 (CCK2) | Drives High Word `((destination >> 16) & 0xFFFF)` to `ea_addr`. Stalls if Chip RAM blocked. |
| **2** | `BUS_WRITE_IDLE` | 2 (CCK1) | Address bus setup for low word write to `ea_addr + 2`. No memory callback. |
| **3** | `WRITE_DST_LONG_LOW` | 2 (CCK2) | Drives Low Word `(destination & 0xFFFF)` to `ea_addr + 2`. Stalls if Chip RAM blocked. |
| **4** | `PREFETCH_NEXT_READ` | 2 (CCK1) | Initiates prefetch from `PC` into `irc`. |
| **5** | `BUS_READ_IDLE` | 2 (CCK2) | Prefetch finishes. Retires: `IR = prefetch`, `prefetch = irc`, `PC += 2`. |

---

#### MOVE destination variants

Memory sources use the [Class 3 address/read prefix](#class-3-memory-source-to-register-load-memory-read-zero-memory-writes), then the destination address and store sequence above; MOVE/MOVEA register loads have no internal tail. Register and immediate sources stage their value without an operand read. Source addressing completes before destination addressing, preserving aliases and source-fault state.

Predecrement stores prefetch before writing. Word and long variants use `PREFETCH_IRC_ADVANCE_IDLE` to advance the completed fetch address while retaining the executing IR. The final store shifts the staged prefetch and marks it retired without another PC increment. Word stores decrement once by two; long stores decrement by two for the low word, then by two for the high word. A rejected write starts Address Error in `WRITE_DST_IDLE`; subsequent transfer phases do not execute. Byte stores retain their existing prefetch retirement before the write.

For memory sources with an absolute-long destination, the first destination extension pair leaves the low address word staged. The write setup assembles the address without another fetch. The store precedes the remaining extension and final prefetch pairs. Register/immediate sources consume both destination extension pairs before their store.

#### Spaced-byte transfer variant: MOVEP

[`movep.rs`](../../../crates/cpu/src/instructions/movep.rs) uses an extension pair, two or four byte bus cycles, then prefetch: 16 or 24 clocks. Bytes transfer from most to least significant at addresses separated by two, with [shared read/write phases](#23-specialized-2-clock-atomic-micro-step-handlers-busfn). Reads assemble `source`; writes consume staged `destination`. Address and data advance only after a successful callback. Byte accesses permit odd addresses. Register-width and CCR semantics belong to the [CPU specification](CPU%20Motorola%20M68000.md#77-move-and-movep-data-transfers).

#### Control-address variant: LEA and PEA

LEA calculates the effective address without an operand read, then prefetches. Indexed modes place one internal CCK before the extension pair and one after it. PEA uses the same address calculation and the [shared stack push phases](#23-specialized-2-clock-atomic-micro-step-handlers-busfn). Indirect, displacement and indexed forms prefetch before the push; absolute forms push both words before prefetch. Effective-address parity does not cause a fault; stack-write alignment does.

The frame-store variant [`LINK`](../../../crates/cpu/src/instructions/link.rs)
uses an extension pair, writes the saved An high word then low word at `SP - 4`,
then prefetches: 16 clocks. Its setup stages the signed displacement and saved
register; `LINK A7` saves `SP - 4`. At prefetch entry it sets An to the frame base
and SP to that base plus displacement, preserving CCR. It follows the shared
long-store phases rather than the call/refill structure below.

### Class 5: Memory Read-Modify-Write (Operand Read, Prefetch, Writeback)

#### Archetype 5A: `ADD.b/w Dn,<ea>` (`ADD.W D0,(d16,A0)`, Opcode `$D168`)
- **Total Duration**: 12 CPU clocks for indirect/postincrement, 14 for predecrement, 16 for displacement, and 18 for indexed addressing.
- **Address Phase**: Use the [Class 3 address-phase layout](#class-3-memory-source-to-register-load-memory-read-zero-memory-writes) with destination `ea_calc` callbacks.
- **Prefetch**: Read the memory operand, prefetch the next instruction word, then write the modified operand.
- **Result Writing**: Commits the modified byte or word to `ea_addr` in the final step.
- **Size Variants**: Memory shifts use word operands; long RMW variants read both words before prefetch.

| Pipeline step after the address phase | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `READ_DST_BYTE/WORD` | 2 (CCK1) | Reads from `ea_addr` into `destination`. Indirect/postincrement resolves the address here. A stalled read does not repeat address calculation. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Completes the operand read bus cycle. |
| **2** | `PREFETCH_NEXT_READ` + `alu_fn` | 2 (CCK1) | Reads the next instruction word into `irc` before writeback. Fused ALU adds `Dn` to `destination` and updates CCR. |
| **3** | `PREFETCH_IRC_FINISH` | 2 (CCK2) | Advances IR/prefetch/cursor and marks prefetch_retired; instruction completion still waits for writeback. |
| **4** | `BUS_WRITE_IDLE` | 2 (CCK1) | Sets up the operand write; no bus access. |
| **5** | `WRITE_DST_BYTE/WORD` | 2 (CCK2) | Writes the modified operand to `ea_addr` and completes the already advanced sequential refill. |

The byte/word layout also covers subtraction, logic and unary memory operations.
`CLR` still reads its destination before prefetch and writeback.
Memory `Scc` uses the same byte layout: it discards the destination read,
writes `$FF` or `$00` according to the condition, and preserves CCR. Its
predecrement address phase consumes two internal clocks before the read,
committing An once; A7 decrements by two for the byte destination.

#### Long Arithmetic and Unary Read-Modify-Write Variants

Long arithmetic, logic and unary RMW uses **read high -> read low -> prefetch -> write low
-> write high**, then retires. Each transfer uses the shared four-clock bus cycle:
20 clocks after the address/immediate prefix. The base effective address stays
fixed; low-word accesses use `ea_addr + 2`, high-word accesses use `ea_addr`.
Immediate forms acquire their extension words before these operand reads, as in
Archetype 5B below; long immediates require two extension pairs.

#### Paired Predecrement Arithmetic

ADDX/SUBX include X and retain cumulative Z. Byte/word paired predecrement
uses an internal CCK, source read, destination read, prefetch and destination
write: 18 clocks. Long paired transfers use an internal CCK, source low/high,
destination low/high, destination low write, prefetch and destination high
write: 30 clocks. Each word uses shared bus phases. The first low-word address
decrements its register by two; if odd, only that decrement is retained and
destination address calculation has not started. Completing the high-word
phase accounts for the second decrement. These are distinct from the ordinary
long RMW layout and use addr1/addr2 to retain their separate progress.

ABCD/SBCD register forms use prefetch then an internal CCK, totaling six clocks.
Paired byte memory forms use a source read, a destination read whose finish
calculates the destination address, prefetch, an internal CCK and byte write.
They have no upfront predecrement idle. NBCD memory uses the unary byte layout
with its distinct predecrement prefix. BCD flag semantics belong to the CPU
specification; undefined manual N/V behavior is constrained only for supplied
silicon inputs.

#### TAS memory variant

[`TAS`](../../../crates/cpu/src/instructions/tas.rs) uses the shared byte address
prefix, then reads the original byte, modifies it during two internal clocks,
writes the byte with bit 7 set, and finally prefetches. The operand interval
is ten clocks: read pair, internal CCK, write pair. Unlike ordinary arithmetic
RMW, prefetch follows the write. Total: 14 clocks plus the address prefix.
Register TAS uses Class 1 four-clock prefetch without a memory transfer.

The [SingleStep comparator](Verification/CPU%20SingleStepTests.md) checks both
callbacks within the combined reference interval. These callbacks do not expose
continuous address-strobe assertion or establish Amiga Chip/Slow RAM write
suppression. The write handler supplies TAS context to AddressBus; MemoryBus
implements suppression independently of generic fixture writes. Continuous
AS and DMA bus locking remain [platform bus limits](Platform%20Quirks%20and%20Invariants%20Catalog.md).

#### Archetype 5B: `ADDI.W #$1234, (d16, A0)` (Opcode `$0668` — Immediate + Displacement Memory RMW)
- **Total Duration**: 20 CPU clocks (10 CCKs).
- **Prefetch**: 2 extension words (immediate `#$1234` + displacement `d16`), then memory read, then opcode prefetch before write.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FETCH_EXT_READ` + `latch_imm_w` | 2 (CCK1) | Saves the already staged immediate in `source`; reads the displacement word at `PC` into `prefetch`. |
| **1** | `FETCH_EXT_FINISH` | 2 (CCK2) | Advances `PC += 2`; displacement remains staged. |
| **2** | `FETCH_EXT_READ` + `ea_calc` | 2 (CCK1) | Resolves `ea_addr = A0 + d16` before replacing the staged displacement with the following opcode read at `PC`. |
| **3** | `FETCH_EXT_FINISH` | 2 (CCK2) | Advances `PC += 2`; following opcode remains staged. |
| **4** | `READ_DST_WORD` | 2 (CCK1) | Reads memory operand from `ea_addr` into `destination`. |
| **5** | `BUS_READ_IDLE` | 2 (CCK2) | Memory read finishes into `destination`. |
| **6** | `PREFETCH_IRC_READ` + `alu_fn` | 2 (CCK1) | **RMW Silicon Quirk:** Prefetches next opcode into `irc` *before* write! Fused ALU computes `res = destination + source`, updates CCR, sets `destination = res as u32`. |
| **7** | `PREFETCH_IRC_FINISH` | 2 (CCK2) | Advances IR/prefetch/cursor and marks the refill retired; writeback remains pending. |
| **8** | `BUS_WRITE_IDLE` | 2 (CCK1) | Bus write setup for `ea_addr`, without a memory callback. |
| **9** | `WRITE_DST_WORD` | 2 (CCK2) | Writes `(destination & 0xFFFF)` to `ea_addr`; completes without shifting the already advanced queue again. |

---

### Class 6: Subroutine Call / Stack Push (`JSR <ea>`)

#### Archetype: `JSR (A0)` (Opcode `$4E90`)
- **Total Duration**: 16 CPU clocks (8 CCKs).
- **Result Writing**: Pushes 32-bit return address onto stack (two 16-bit word writes to `-(SP)`).
- **Prefetch**: Two-word pipeline refill directly from target address `(A0)`.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `READ_CONTROL_TARGET_OPCODE_READ` + `alu_jsr_ai` | 2 (CCK1) | Resolves target and return address; reads target opcode into `irc`. An odd target faults before SP changes. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Completes the target opcode read. |
| **2** | `PUSH_STACK_HIGH_IDLE` | 2 (CCK1) | Decrements SP by four and checks stack alignment. |
| **3** | `PUSH_STACK_HIGH_WRITE` | 2 (CCK2) | Writes return PC high word to SP. |
| **4** | `BUS_WRITE_IDLE` | 2 (CCK1) | Prepares the low word write. |
| **5** | `PUSH_STACK_LOW_WRITE` | 2 (CCK2) | Writes return PC low word to SP + 2. |
| **6** | `PREFETCH_TARGET_READ` | 2 (CCK1) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. |
| **7** | `PREFETCH_TARGET_FINISH` | 2 (CCK2) | Sets `target_refill = true`, loads `IR = irc`, sets `PC = ea_addr + 4`, and completes retirement! |

Displacement/absolute-short modes add two internal clocks before the first
target read; indexed modes add six. Absolute-long uses one extension pair.
All JSR forms retain target opcode read, stack writes, then target lookahead read.

#### Relative Call Variant (`BSR`)

[`bsr.rs`](../../../crates/cpu/src/instructions/bsr.rs) computes
`target = opcode_pc + 2 + sign_extend(displacement)` in two internal clocks.
Both displacement forms then push the return PC high/low words using the shared
stack phases, followed by the two target refill pairs: 18 clocks. Short calls
save `opcode_pc + 2`; word calls save `opcode_pc + 4`. An odd target faults
after both stack writes and captures the target through the
[control-flow refill model](#51-the-two-word-pipeline-refill).

---

### Class 7: Subroutine Return / Stack Pop (`RTS`)

#### Archetype: `RTS` (Opcode `$4E75`)
- **Total Duration**: 16 CPU clocks (8 CCKs).
- **Result Writing**: None (Memory reads from stack). Updates `SP = SP + 4`.
- **Prefetch**: Two-word pipeline refill directly from popped return address.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `POP_STACK_HIGH_READ` | 2 (CCK1) | Reads return PC high word from `(SP)`. |
| **1** | `POP_STACK_HIGH_FINISH` | 2 (CCK2) | Latches high word into `ea_high`, updates `SP = SP + 2`. |
| **2** | `POP_STACK_LOW_READ` | 2 (CCK1) | Reads return PC low word from `(SP)`. |
| **3** | `POP_STACK_LOW_FINISH` | 2 (CCK2) | Completes the assembled return PC and advances SP by two. |
| **4** | `READ_CONTROL_TARGET_OPCODE_READ` | 2 (CCK1) | Checks target alignment and reads the new opcode into `irc`; an odd target uses the fault capture in Section 5.1. |
| **5** | `BUS_READ_IDLE` | 2 (CCK2) | Target opcode read completes (no additional CPU memory callback). |
| **6** | `PREFETCH_TARGET_READ` | 2 (CCK1) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. |
| **7** | `PREFETCH_TARGET_FINISH` | 2 (CCK2) | Sets `target_refill = true`, loads `IR = irc`, sets `PC = ea_addr + 4`, and completes retirement! |

---

#### Status-return variant: RTR and RTE

[`RTR`](../../../crates/cpu/src/instructions/rtr.rs) and
[`RTE`](../../../crates/cpu/src/instructions/rte.rs) use three stack read pairs
followed by the two target refill pairs: 20 clocks. Read order is PC high at
`SP + 2`, status at `SP`, then PC low at `SP + 4`. The first read leaves SP
unchanged. Status completion advances SP by four; low-word completion advances
it by two. RTR applies only CCR at status completion. RTE stages SR until all
three words have been read from the supervisor stack, then restores it before
target access; returning to user mode therefore leaves SSP advanced by six and
selects USP. Refused transfers preserve staged words and do not advance SP.

RTE checks privilege before any stack access. User execution selects the shared
34-clock privilege exception sequence. Operand alignment faults and target
alignment faults occur at their respective read phases; status restoration
does not precede a failed stack read. Register semantics belong to the
[CPU specification](CPU%20Motorola%20M68000.md).

### Class 8: Unconditional Jump (`JMP <ea>`)

#### Archetype: `JMP (A0)` (Opcode `$4ED0`)
- **Total Duration**: 8 CPU clocks (4 CCKs).
- **Result Writing**: None.
- **Prefetch**: Flushes pipeline and performs two-word pipeline refill from `(A0)`.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `READ_CONTROL_TARGET_OPCODE_READ` + `ea_calc` | 2 (CCK1) | Resolves `ea_addr = A0`. An odd target starts Address Error with the target-PC capture in Section 5.1; an aligned target reads the opcode into `irc`. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Target opcode read completes (no additional CPU memory callback). |
| **2** | `PREFETCH_TARGET_READ` | 2 (CCK1) | **Refill 2:** Reads prefetch word at `ea_addr + 2`. |
| **3** | `PREFETCH_TARGET_FINISH` | 2 (CCK2) | Sets `target_refill = true`, loads `IR = irc`, sets `PC = ea_addr + 4`, and completes retirement! |

---

### Class 9: Conditional Branching (`Bcc`) (Dynamic Path Selection)

[`bcc.rs`](../../../crates/cpu/src/instructions/bcc.rs) selects the path in a
zero-clock ALU trampoline. A taken branch computes
`ea_addr = opcode_pc + 2 + sign_extend(displacement)`. The short displacement
comes from IR; the word displacement is already in the lookahead register.
`BRA` uses the same taken layout without condition selection.

| Selected path | Internal delay | Following bus pairs | Total clocks |
| :--- | :---: | :--- | :---: |
| Taken short or word | 2 | `READ_CONTROL_TARGET_OPCODE_READ` / `BUS_READ_IDLE`, then `PREFETCH_TARGET_READ` / `PREFETCH_TARGET_FINISH` | 10 |
| Untaken short | 4 | `PREFETCH_NEXT_READ` / `BUS_READ_IDLE` | 8 |
| Untaken word | 4 | `READ_TARGET_OPCODE_READ` / `BUS_READ_IDLE` at the sequential PC, then `PREFETCH_TARGET_READ` / `PREFETCH_TARGET_FINISH` | 12 |

Each pair follows the [shared read phases](#23-specialized-2-clock-atomic-micro-step-handlers-busfn).
Taken paths refill at the destination and destination + 2. Untaken short paths
retain the lookahead opcode and fetch its following word. Untaken word paths
replace the displacement with the sequential opcode at `opcode_pc + 4` and
fetch the word at `opcode_pc + 6`. Retirement commits the staged opcode and
lookahead; the executing IR remains unchanged during the instruction.
Odd taken targets use the [shared control-target frame capture](#51-the-two-word-pipeline-refill).
Untaken paths do not check the unused displacement target.

#### Decrement-and-Branch Variant (`DBcc`)

[`dbcc.rs`](../../../crates/cpu/src/instructions/dbcc.rs) tests the condition
before changing Dn. A true condition leaves Dn unchanged and uses the 12-clock
sequential word path. A false condition decrements only the low word, preserves
the upper word and CCR, and selects the 10-clock taken path when the new counter
is not `$FFFF`. The word displacement is relative to `opcode_pc + 2`; taken
refill and odd-target capture use the same primitives as Bcc.

---

### Class 10: Variable-Cycle Arithmetic (`DIVU.W <ea>, Dn`)

#### Archetype: `DIVU.W (A0), D0` (Opcode `$80D0`)
- **Total Duration**: Register division timing plus four clocks for the indirect operand read; overflow takes 14 clocks, divide-by-zero entry takes 42.
- **Result Writing**: Register destination (`D0`). Sets quotient and remainder. Zero memory writes.
- **Prefetch**: Reads divisor word, waits internal division delay (`clocks_remaining`), then completes sequential opcode prefetch.

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `READ_SRC_WORD` | 2 (CCK1) | Reads 16-bit divisor from memory at `(A0)` into `source`. |
| **1** | `BUS_READ_IDLE` | 2 (CCK2) | Memory read finishes into `source`. |
| **2** | `ALU_IDLE` + `alu_fn` | 2 (CCK1) | Calls `alu_divu_mem`. Updates the result and CCR, then schedules the shift/subtract timing from `calc_divu_internal_clocks`, excluding the final four-clock prefetch. Overflow schedules six internal clocks; a zero divisor selects exception entry. |
| **3..N** | `ALU_IDLE` (Countdown) | Variable | Sits in idle bus countdown for `clocks_remaining` (freeing Chip RAM for Blitter/Copper). |
| **N+1** | `PREFETCH_NEXT_READ` | 2 (CCK1) | Initiates sequential opcode prefetch from `PC` into `irc`. |
| **N+2** | `BUS_READ_IDLE` | 2 (CCK2) | Completes prefetch, shifts `IR = prefetch`, `prefetch = irc`, `PC += 2`, and retires! |

Multiplication uses **operand acquisition -> prefetch -> variable internal
interval -> retirement**. Its internal interval is `34 + 2n` clocks: `n` counts
source one-bits for MULU and adjacent bit transitions in the source with a low
zero bit for MULS. Division completes its internal interval before prefetch;
its iteration and sign timing is defined in the [division timing reference](../Reference/Motorola%2068000%20DIVU%20%26%20DIVS%20Cycle-Accurate%20Timing%20Analysis.md).

#### Register Shifts and Rotates

Register shifts and rotates use **prefetch -> variable internal ALU interval ->
retirement**, unlike division above. The internal interval is `2 + 2n` clocks for
byte/word and `4 + 2n` for long; totals are `6 + 2n` and `8 + 2n` respectively.
Count decoding and flag behavior are defined in the [CPU specification, section
7.5](CPU%20Motorola%20M68000.md). Memory shifts use the Class 5 word RMW layout.

---

### Class 11: Multi-Register Block Transfer (`MOVEM <ea>, reglist`)

#### Archetype: `MOVEM.W (A0)+, D0-D2/A0-A1` (Opcode `$4CD8 $0307` — 5 Registers)
- **Total Duration**: Exactly 32 CPU clocks (16 CCKs).
- **Result Writing**: Loads registers $D_0, D_1, D_2, A_0, A_1$ from memory. Postincrements $A_0$ by $5 \times 2 = 10$ bytes. Zero memory writes.
- **Prefetch**: Consume the staged mask while fetching the following word; transfer the selected registers, read the final dummy operand word, then prefetch. See the [MOVEM loop mechanics](#58-movem-dynamic-register-transfer-engine).

| Step | Action / Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `FETCH_MASK_READ` | 2 (CCK1) | Latches staged mask `$0307` and fetches the following instruction word. |
| **1** | `FETCH_MASK_FINISH_AI` | 2 (CCK2) | Advances `PC += 2` and resolves `ea_addr = A0`. |
| **2** | `MOVEM_TRANSFER` (five register reads) | 20 (10 CCKs) | Reads selected registers in mask order, sign-extends words and increments the transfer address. |
| **2** | `MOVEM_TRANSFER` (dummy read) | 4 (2 CCKs) | Reads at the final transfer address, then commits $A_0 = ea\_addr$. |
| **3** | `PREFETCH_NEXT_READ` | 2 (CCK1) | Reads the next instruction word from `PC` into `irc`. |
| **4** | `BUS_READ_IDLE` | 2 (CCK2) | Completes sequential prefetch and retires. |

---

### Class 12: Group 0 Diagnostic Hardware Exception (Address Error Vector 3)

#### Archetype: Misaligned Word Access (Total Duration: Exactly 50 CPU clocks / 25 CCKs)
- **Total Duration**: 50 CPU clocks (25 CCKs), plus completed address and extension phases.
- **Entry CCK**: Alignment failure starts `ALU_AERR_INIT` in the rejected-access CCK, with no operand transfer.
- **Return PC**: `trigger_address_error` latches `pc - 4` for data-access faults and `instruction_pc` for generic program-space faults; control-target handlers override it with target minus four.
- **Physical Silicon Stacking Order**: Interleaved hardware write sequence supported by the supplied stack-write transactions; the manual defines frame layout, not every write phase.
- **Double Bus Fault**: If $SSP$ is odd during exception setup or stack writes, or if the assembled vector-3 handler target is odd, the processor halts immediately (`self.state.halted = true`).
- **Prefetch**: Vector fetch + two-word pipeline refill from exception handler.

| Step | Handler | Clocks | Description / Bus Transaction |
| :---: | :--- | :---: | :--- |
| **0** | `ALU_AERR_INIT` | 2 (CCK1) | S=1, T=0; checks SSP, saves `ssp_base` and old SR, retains latched return PC in `source`, sets vector `$00000C`. |
| **1** | `ALU_IDLE` | 2 (CCK2) | Internal execution cycle completing initial 4-clock entry interval. |
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
| **17** | `BUS_READ_IDLE` | 2 (CCK2) | Vector high word read completion (no additional CPU memory callback). |
| **18** | `READ_VECTOR_LOW_READ` | 2 (CCK1) | Vector fetch low word read from `$00000E` into `source`. |
| **19** | `READ_VECTOR_LOW_FINISH` | 2 (CCK2) | Checks target alignment (halts if double fault), sets `ea_addr = handler_address`. |
| **20** | `READ_TARGET_OPCODE_READ` | 2 (CCK1) | **Refill 1:** Reads first instruction opcode of handler from `ea_addr`. |
| **21** | `BUS_READ_IDLE` | 2 (CCK2) | Target opcode read completion (no additional CPU memory callback). |
| **22** | `ALU_IDLE` | 2 (CCK1/2) | 2-clock internal refill interval. |
| **23** | `PREFETCH_TARGET_READ` | 2 (CCK1) | **Refill 2:** Reads second instruction word from `ea_addr + 2` into `prefetch`. |
| **24** | `PREFETCH_TARGET_FINISH` | 2 (CCK2) | Arms `target_refill = true`, latches `IR = irc`, sets `PC = ea_addr + 4`, retires exception! |


---

### Class 13: Three-Word Software Exception Entry

This differs from Class 12's seven-word diagnostic frame. The shared tail uses
the [canonical bus phases](#23-specialized-2-clock-atomic-micro-step-handlers-busfn):
write saved PC low at `SSP - 2`, SR at `SSP - 6`, PC high at `SSP - 4`; read both
vector words; read the handler opcode, spend one internal CCK, then read handler
lookahead and retire through target refill. The tail takes 30 clocks. Entry saves
old SR before selecting supervisor mode and clearing trace.

| Entry | Prefix before the shared tail | Total excluding operand acquisition | Saved PC |
| --- | --- | ---: | --- |
| `TRAP #n` | Four internal clocks | 34 | Following instruction |
| Illegal/Line A/Line F/privilege | Four internal clocks | 34 | Rejected opcode |
| Trace | Four internal clocks | 34 | Next opcode after completion |
| Interrupt | Fourteen clocks, including internal IACK | 44 | Next opcode after completion |
| Taken `TRAPV` | Four-clock sequential prefetch pair | 34 | Following instruction |
| `CHK`, above bound | Prefetch pair, four internal clocks | 38 | Captured instruction_pc |
| `CHK`, negative only | Prefetch pair, six internal clocks | 40 | Captured instruction_pc |

CHK tests the signed upper bound before the negative condition. A negative
value greater than a negative bound uses above-bound timing. Normal CHK uses
the Class 3 six-clock tail; untaken TRAPV uses Class 1 four-clock prefetch.
Exception prefixes do not retire the sequential queue before stacking.
Privilege entry uses four internal clocks, saves the rejected opcode address,
and performs no operand read. Semantics and vector selection belong to the
[CPU specification](CPU%20Motorola%20M68000.md).

---

## 8. Architectural Quality Attributes & Systems Compliance

| Property | Implemented contract and evidence limit |
| --- | --- |
| Static dispatch | One opcode lookup at initiation, then a cached slice; dynamic branches and indirect calls remain. |
| Fixed runtime storage | No heap allocation in CPU execution or transfers; host/test artifacts may allocate. |
| Wait stability | Refusals retain active address/data/phase; ordinary CPU fixtures test repeated waits and single successful commit. |
| Endian/width discipline | Decoded big-endian word values and explicit long decomposition; no host pointer reinterpretation. |
| Exceptions | Dedicated seven-word and shared three-word frame structures; no BERR or device-vector IACK input. |
| Prefetch | Live bus reads with retained staged words; no host code cache. |
| Signal fidelity | CCK callbacks and RESET output; no independent AS/FC/strobe waveform or TAS lock observation. |
| Snapshot boundary | Serializable fields; redirected-sequence restoration remains a known limitation. |
| Platform integration | CPU waits alone do not prove Agnus arbitration; production TAS Chip/Slow suppression is separately tested, without an electrical lock capture. |

## 9. Reference Documentation & Upstream Ground Truth

- [Motorola Section 8](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): aggregate clocks and transfer counts; not a complete ordering oracle.
- [Original Motorola manual](https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf): physical bus phases, frames and trace priority; resolves disputed converted wording.
- [CPU silicon verification](Verification/CPU%20SingleStepTests.md): timed callback comparison and retained allowances.
- [Micro-state](../../../crates/cpu/src/micro/engine.rs), [descriptors](../../../crates/cpu/src/micro/types.rs), [dispatch](../../../crates/cpu/src/micro/dispatch_table.rs) and [stepping](../../../crates/cpu/src/cpu.rs): living project execution contracts.
- [Ordinary phase tests](../../../crates/cpu/tests/test_cck_bus.rs), [class tests](../../../crates/cpu/tests/test_micro_archetypes.rs) and [boundary tests](../../../crates/cpu/tests/test_exception_boundaries.rs): synthetic wait/fault/exception evidence, separate from motherboard captures.
