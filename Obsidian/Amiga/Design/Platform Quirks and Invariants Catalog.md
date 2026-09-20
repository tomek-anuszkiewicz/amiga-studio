---
title: "Platform Quirks & Invariants Catalog"
aliases: ["Platform Quirks Catalog", "Hardware Quirks Index", "Silicon Traps Catalog"]
tags: ["amiga", "design", "quirks", "invariants", "m68000", "chipset"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-12
updated: 2026-09-20
related: ["[General Architecture.md](General%20Architecture.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Floppy.md](Floppy.md)", "[Keyboard.md](Keyboard.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)"]
---

# Platform Quirks & Invariants Catalog

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Subsystem Specifications:** [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) | [MemoryBus.md](MemoryBus.md) | [Agnus.md](Agnus.md) | [Denise.md](Denise.md) | [Paula.md](Paula.md) | [CIA.md](CIA.md) | [Floppy.md](Floppy.md) | [Keyboard.md](Keyboard.md)
- **Cross-Chip Catalogs:** [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md) | [Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)
- **Execution & Invariant Rules:** [`spec-compliance.md`](../../../.agents/rules/spec-compliance.md) | [`performance-and-readability.md`](../../../.agents/rules/performance-and-readability.md) | [`repro-first.md`](../../../.agents/rules/repro-first.md)

> [!IMPORTANT]
> **Anti-Tamper & Silicon Traps Defense:**
> Modern AI agents and software engineers frequently attempt to "correct" counter-intuitive hardware behaviors that appear anomalous in generic training data (e.g. prefetching opcodes before memory writeback, preserving zero-flags across multi-precision limbs, or unaligned stack pointer decrements).
> This catalog serves as the centralized index of **intentional hardware idiosyncrasies, silicon anomalies, and physical circuit constraints**. Deviating from or smoothing out these quirks violates cycle-exact compatibility.

---

## 1. Scope Boundary: Architectural Principles vs Silicon Quirks

To maintain strict conceptual hygiene across the repository, this catalog enforces a clean boundary:

> [!NOTE]
> **Core Subsystem Principles are NOT Quirks:**
> Foundational Amiga hardware operating principles—such as **Agnus synchronous DMA cycle scheduling** ([Agnus.md](Agnus.md)), **Blitter Nasty mode (`BLTPRI`)** ([Agnus.md](Agnus.md)), **Gary CIA partial address decoding & memory mirroring** ([MemoryBus.md](MemoryBus.md)), and **boot ROM overlay routing** ([MemoryBus.md](MemoryBus.md))—are **standard architectural operating modes**, not quirks or errata.
> They reside exclusively in their authoritative subsystem design documents and are **strictly excluded** from this quirks catalog.

This catalog is exclusively dedicated to **True Silicon Quirks, Hardware Errata, and Micro-Architectural Traps**:
1. **CPU Silicon Quirks:** Anomalous micro-step ordering, condition code side-effects, and ALU alignment quirks in the physical Motorola 68000 silicon.
2. **Motherboard & Chipset Errata:** Physical circuit traps, dropped bus phases, and peripheral flip-flop behaviors where real hardware diverges from idealized algorithms.

---

## 2. Motorola 68000 Silicon Quirks & CPU Pipeline Traps

Items in this section belong to one of three provenance categories:
- **Category A (Motorola PRM/UM):** Formally documented in Motorola manuals, but violates orthogonal ISA assumptions and constitutes a trap for generic CPU models or LLMs.
- **Category B (Micro-Bus Specs):** Documented in hardware bus cycle timing sheets and application notes, but omitted or glossed over in high-level programming references (PRM).
- **Category C (Silicon Reality):** Undocumented silicon micro-architectural anomalies or AGU latching behaviors uncovered via hardware reverse-engineering, bus analyzer traces, and single-step silicon test vectors.

| Silicon Quirk | Provenance & Source | Physical Hardware Reality | Generic Model Trap | Implementation Invariant | Authoritative Spec |
| :--- | :---: | :--- | :--- | :--- | :--- |
| **Class 0 RMW Prefetch Order** | **Category B** *(Micro-Bus Specs)* | On Read-Modify-Write instructions (`ASL <ea>`, `CLR <ea>`, `NOT <ea>`), the CPU executes the *next opcode prefetch* from `PC` into the pipeline prefetch register **before** writing the modified operand back to memory. | Reordering the sequence to write back operand before fetching next instruction (breaks self-modifying code where instruction modifies the next word). | Micro-step state machine executes `BusPrefetchToScratch` during step 3/4, then executes `BusWrite` on step 5. | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) |
| **A7 Stack Pointer Byte Alignment** | **Category A** *(Motorola PRM/UM)* | Byte operations referencing the stack pointer `A7` (e.g., `MOVE.B D0, -(A7)`) modify the pointer by **2 bytes (word alignment)** instead of 1 byte to keep the supervisor/user stack aligned. | Decrementing or incrementing A7 by 1 on byte operations. | Effective address engine explicitly adjusts decrement step to 2 when `reg == 7` and size is byte. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Multi-Precision $Z$-Flag Retention (`ADDX`/`SUBX`/`NEGX`)** | **Category A** *(Motorola PRM/UM)* | The Zero condition code flag ($Z$) is cleared if the arithmetic result is non-zero, but **remains completely unmodified** if the result is zero. This enables multi-precision chaining across limbs. | Setting $Z = 1$ when the current limb produces zero, obliterating previous non-zero limb results. | Flag calculation evaluates `if result != 0 { Z = 0 } else { /* retain existing Z */ }`. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Address Register Direct CCR Immunity & Sign Extension** | **Category A** *(Motorola PRM/UM)* | All operations modifying address registers (`MOVEA`, `ADDA`, `SUBA`, `ADDQ/SUBQ to An`) **never modify any CCR flags** ($X, N, Z, V, C$), and `.W` variants sign-extend the 16-bit operand to 32 bits before ALU evaluation. (`CMPA` is comparison-only and updates CCR). | Updating $N$ or $Z$ flags based on the address register result, or failing to sign-extend 16-bit word sources. | Handlers explicitly bypass CCR calculation and sign-extend word operands to full 32-bit width (`val as i16 as i32 as u32`). | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **`ASL` Sticky Overflow ($V$)** | **Category A** *(Motorola PRM/UM)* | During arithmetic left shifts, the overflow flag $V$ is set if the most significant bit changes at **any point** during the multi-bit shift sequence. Once set, $V$ remains sticky-set until the end of the operation. | Setting $V$ based solely on the final bit shifted or final sign comparison against initial operand. | Shift ALU loop accumulates $V$ via `v_flag |= (orig_msb ^ new_msb)` across each intermediate shift step. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Dual-Memory Address Error Deferral** | **Category C** *(Silicon Reality)* | In dual-memory operations (`CMPM`, `ADDX -(Ay), -(Ax)`), address error checks must evaluate source first. Unaligned word/long source reads trigger Vector 3 before destination address calculation starts. | Calculating both source and destination pointers before performing the read, erroneously incrementing or decrementing destination registers. | Uses dual staging registers (`addr1`, `addr2`). Destination address calculation is deferred to CCK2 of the source read. | [`performance-and-readability.md`](../../../.agents/rules/performance-and-readability.md) |
| **Multi-Cycle Division Overflow CCR Quirk** | **Category B/C** *(Bus Timing & Silicon Trace)* | When `DIVU` or `DIVS` overflows (quotient > 16 bits), the $V$ flag is set, $C$ is cleared, $N$ and $Z$ are undefined/preserved, and the destination register contents are left **completely unmodified**. | Writing partial quotient or zeroing destination register on overflow. | Division micro-steps abort register writeback upon quotient overflow detection, asserting $V=1$ and keeping $D_n$ intact. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Reset Vector Fetch & Double Bus Fault** | **Category A** *(Motorola UM)* | On hardware reset, the M68000 unconditionally reads Vector 0 (`SSP`) from `$000000` and Vector 1 (`PC`) from `$000004` without sanitization or Amiga-specific defaults. If initial `PC` is unaligned (bit 0 != 0, e.g. open bus `$FFFFFFFF`), instruction prefetch triggers an Address Error during reset exception processing, immediately escalating to a Double Bus Fault and permanently asserting physical `_HALT`. | Sanitizing odd PC addresses to `$000000` or open-bus reads to `$080000` within the CPU core. | `Cpu::reset()` unconditionally latches vectors from `bus`. If `(pc & 1) != 0`, sets `halted = true` and aborts prefetch. Synthetic boot vectors for ROM-less testing reside exclusively in `PhysicalMemory`. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Two-Word Prefetch Ahead-Offset ($PC = \text{Opcode} + 4$)** | **Category B** *(Micro-Bus Specs)* | The physical hardware PC register always leads execution by 2 words (4 bytes) because the two-word FIFO (`IR` + `IRC`) must be primed before instruction execution begins. Right at reset, before executing the opcode at Vector 1 ($A$), the bus has already fetched words at $A$ and $A+2$, placing the hardware PC at $A+4$. | Assuming $PC$ points to the executing opcode or next instruction, or attempting to eliminate `pc.wrapping_sub(4)` as an "off-by-four bug". | `state.pc` tracks physical bus prefetch ($A+4$); `state.instruction_pc = state.pc.wrapping_sub(4)` provides the true opcode address for debuggers/disassemblers. Subroutines (`JSR`/`BSR`) compute return addresses relative to the advanced hardware PC. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **Post-Increment `(An)+` Address Error AGU Register Commitment** | **Category C** *(Silicon Reality)* | On a **READ** from `(An)+` with an odd address, the Address Generation Unit (AGU) commits $A_n \leftarrow A_n + \text{increment}$ as the read bus cycle begins; if unaligned, $A_n$ remains incremented in the register file upon entering Vector 3. On a **WRITE** to `(An)+`, the processor checks alignment before post-incrementing; if unaligned, $A_n$ is **never** incremented. | Assuming symmetric AGU register rollback on both read and write faults, or failing to increment $A_n$ on faulting reads. | All linear EA resolvers (`ea.rs`) and specialized instructions (`cmpm.rs`) commit $A_n$ before read bus transaction alignment faults. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **`ASR` Count $\ge$ Width Silicon Exhaustion ($C=0, X=0$)** | **Category C** *(Silicon Reality)* | When the shift count exceeds the operand width ($count \ge 8$ for Byte, $\ge 16$ for Word, $\ge 32$ for Long), the physical shift register exhausts its internal latch pipeline after 32 shifts, forcing both **$C = 0$ and $X = 0$**, even when shifting negative numbers filled with replicated sign bits (`1`). | Assuming mathematical sign extension where shifting negative numbers infinitely preserves the replicated sign bit in $C$ and $X$ ($C=1, X=1$) per high-level PRM text. | `asr.rs` explicitly forces `C = 0` and `X = 0` when `count >= width_bits`. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |
| **`MOVE` to Predecrement `-(An)` Prefetch Inversion & Bus Ordering** | **Category B/C** *(Bus Timing & Silicon Trace)* | For Byte and Word `MOVE ..., -(An)`, the CPU prefetches the next instruction word **before** initiating the destination write bus cycle. If the write faults with an Address Error, the $IR$ pushed into the 7-word stack frame is the **prefetched instruction word**, not the `MOVE` opcode, and $A_n$ remains decremented by 2. For Long `MOVE.L ..., -(An)`, the write cycle occurs before prefetch; the 32-bit transfer writes low word first to $A_n - 2$ (faulting immediately at $A_n - 2$ if odd), pushing the `MOVE` opcode as faulting $IR$. | Pushing the executing `MOVE` opcode for all sizes, or decrementing $A_n$ by 4 when long writes fault. | Micro-step sequences execute prefetch before write for byte/word predecrement stores, and decrement low word first for long stores. | [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) |

---

## 3. Motherboard & Chipset Silicon Errata & Circuit Traps

The following items represent physical silicon anomalies, hardware errata, and circuit wiring traps in the Amiga 500 chipset and motherboard:

| Hardware Quirk / Erratum | Physical Hardware Reality | Generic Model Trap | Implementation Invariant | Authoritative Spec |
| :--- | :--- | :--- | :--- | :--- |
| **TAS Read-Modify-Write Silicon Erratum** | The Motorola 68000 asserts `_AS` throughout an unbroken Read-Modify-Write cycle without a bus release. The Amiga 500 Agnus/Gary memory controller cannot handle this unbroken strobe on Chip RAM and Slow RAM, dropping the write phase. | Executing a standard atomic read-and-set write on Chip/Slow RAM. | MemoryBus drops the write phase of `TAS` on Chip RAM (`$000000-$07FFFF`) and Slow RAM (`$C00000-$C7FFFF`), updating condition codes but leaving memory bit 7 unmodified. | [MemoryBus.md](MemoryBus.md) |
| **Floppy Shared Motor Line Wiring** | CIA-B bit `DSKMTR` controls the spindle motor, but the line is physically shared across all four drives (`DF0..DF3`). Drive motor status changes only latch into individual drives that are actively selected (`DSKSELx = 0`). | Providing separate, independent motor control registers for each drive. | Motor latch evaluates `DSKMTR` combined with active individual drive select lines. | [Floppy.md](Floppy.md#L85) |
| **Floppy Disk Change Flip-Flop (`_CHNG`)** | The disk change signal (`_CHNG`) is latched by a hardware flip-flop. Inserting a new disk does not automatically clear `_CHNG`; software must pulse a step command to the drive head to reset the latch. | Instantly clearing disk change status upon disk file insertion. | `step()` pulse checks disk presence; `_CHNG` remains asserted until a physical step command is issued. | [Floppy.md](Floppy.md#L100) |
| **Keyboard Caps Lock Latch State Machine** | Unlike standard keys that send key-down and key-up pairs, Caps Lock has an internal latch. It sends a key-down on press and release only on subsequent toggle, simulating an alternate latch. | Generating standard key-up/key-down events for Caps Lock. | Keyboard controller models the dual-state toggle latch for keycode `$62`. | [Keyboard.md](Keyboard.md#L106) |
| **Keyboard Serial Handshake Delay** | CIA-A SP pin receives keyboard scancodes serially. The Amiga must acknowledge by pulling the clock line low for at least $85\ \mu\text{s}$ before the keyboard will transmit the next code. | Reading keyboard data immediately without waiting for software CIA-A handshake pulse. | Keyboard shift register waits for CIA-A handshake acknowledgment before deserializing subsequent packets. | [Keyboard.md](Keyboard.md) |
| **Linear Resistor DAC & Absence of Gamma Pre-Correction** | Denise outputs to an uncorrected R-2R ladder ($270/560\ \Omega$), producing strictly linear voltages ($0.0–0.7\text{V}$). Unlike broadcast video with gamma pre-compression ($\sim 0.45$), dark tones occupy equal voltage bandwidth, relying on CRT natural $\gamma \approx 2.8$ expansion. | Naively applying 8-bit bit replication `(n << 4) \| n` ($15 \times 17 = 255$) or sRGB curves. | Quantizes directly as $n \times 16 \to 0, 16, ..., 240$ with $\pm 1$ channel tolerance to account for studio range and YUV subcarrier rounding. | [Denise.md](Denise.md#L191) |

---

## 4. Summary Checklist for Subsystem Modification

Whenever refactoring or optimizing any subsystem:
- [ ] Has this catalog been checked for true silicon quirks affecting the target instruction or register?
- [ ] Are core architectural principles (Agnus DMA scheduling, Blitter Nasty, CIA partial decoding) checked against their dedicated subsystem specs rather than treated as quirks?
- [ ] Are condition code flags ($X, N, Z, V, C$) strictly matching silicon behavior rather than textbook math?
- [ ] If an edge-case or divergence is being resolved, was a failing reproduction test created in `tests/` per [`repro-first.md`](../../../.agents/rules/repro-first.md)?

---

## 5. Reference Documentation & Upstream Ground Truth

- **Commodore Amiga Hardware Reference Manual**:
  - [Hardware Reference Manual](../Reference/Hardware%20Reference%20Manual): Primary reference for custom chip register addresses, bit allocations, and DMA channel priorities.
- **Undocumented Chipset Features**:
  - [Undocumented features of OCS, ECS and AGA chipsets.md](../Reference/Undocumented%20features%20of%20OCS,%20ECS%20and%20AGA%20chipsets.md): Definitive compendium of undocumented silicon behavior across Agnus, Denise, and Paula.
- **Motorola 68000 Architecture & Prefetch Specs**:
  - [Instruction Prefetch on the Motorola 68000 Processor.md](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): Micro-step prefetch queue mechanics, pipeline fill timings, and bus cycle overlaps.
  - [68000 Programmer's Reference Manual](../Reference/68000%20Programmer's%20Reference%20Manual): Official instruction-level arithmetic, CCR flag truth tables, and exception vectors.
- **Silicon Test Suites & Reference Emulators**:
  - [`SingleStepTests-680x0`](../../../ref_src/SingleStepTests-680x0/68000/v1): Physical silicon ground truth vectors for all 68000 instructions.
  - [`vAmiga Reference Source`](../../../ref_src/vAmiga-4.5): Clean-room reference implementation for custom chip and bus timings.
