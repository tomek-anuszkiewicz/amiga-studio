---
title: "vAmigaTS Verification Scorecard"
aliases: ["vAmigaTS Scorecard", "Silicon Verification Scorecard", "Test Scorecard"]
tags: ["amiga", "design", "verification", "vamiga", "testing"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-16
updated: 2026-09-16
related: ["[Testing Strategy and Quality Assurance.md](Testing%20Strategy%20and%20Quality%20Assurance.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# vAmigaTS Verification Scorecard

This document maintains the persistent, ground-truth verification scorecard for the automated **vAmigaTS physical silicon test suite** within the Amiga 500 emulator project.

It provides a single authoritative status reference for hardware verification without cluttering [`ROADMAP.md`](../../../ROADMAP.md) or git commit logs with transient, intermediate run percentages.

---

## 1. Executive Verification Summary (Ground Truth)

| Metric | Count | Percentage of Phase 1 Baseline |
| :--- | :--- | :--- |
| **Total vAmigaTS Test Cases Discovered** | 2,077 | — |
| **Formally Deferred Tests (Gates 1–5)** | 609 | — |
| **Active Phase 1 Baseline OCS Suite** | **1,468** | 100.0% |
| **Verified Passing Tests (0 Mismatched Pixels)** | **77+** | Tracked across validated clusters |
| **Active In-Progress Cluster** | Blitter `bbusy` | Copper/DMA cycle arbitration unblocking |

All passing tests execute against physical silicon golden reference captures (`.raw` / `_ocs.raw`), rendering 24-bit RGB viewports ($716 \times 285$ pixels) with zero tolerance (0 diff pixels allowed for 100.0% PASS).

---

## 2. Test Suite Categorization & Deferred Gates Matrix

The 2,077 tests discovered in `ref_src/vAmigaTS/` are partitioned into **1,468 active Phase 1 Baseline tests** and **609 formally deferred tests** across 5 architectural gates:

| Gate | Scope / Reason | Deferred Tests | Destination Milestone |
| :--- | :--- | :--- | :--- |
| **Gate 1: FPU Math Coprocessor** | `ref_src/vAmigaTS/FPU/` requiring MC68881/MC68882 coprocessor instructions | 206 | Phase 3 (AGA & FPU) |
| **Gate 2: ECS & AGA Silicon** | Tests providing exclusively `_ecs.raw`, `_plus.raw`, or `_aga.raw` captures (SuperHires, BPLCON3, 24-bit DACs) | 112 | Phase 2 (ECS) & Phase 3 (AGA) |
| **Gate 3: Motorola 68010 CPU** | Tests requiring `_68010.raw` (`BKPT`, `MOVE from CCR`, `MOVES`, `VBR`, loop mode) | 91 | 68010 Extension Milestone |
| **Gate 4: AmigaOS Floppy MFM Boot** | Bootblock tests requiring genuine floppy track MFM streaming and `dos.library` | 7 | Step 6 (Floppy MFM Boot) |
| **Gate 5: Non-Visual Register Assertions** | Peripheral tests lacking 24-bit RGB `.raw` frames (providing CRT photographs `.JPG` only) | 193 | Step 2.6 (Peripheral Harness) |
| **Total Deferred Tests** | — | **609** | — |

---

## 3. Sub-Suite Verification Breakdown (Substrate-First Order)

In accordance with the **Substrate-First Invariant**, sub-suites are ordered strictly by physical electronic causality (Layer 0 Bus Arbitration $\to$ Layer 1 DMA Coprocessors $\to$ Layer 2 Video Serializer $\to$ Layer 3 Peripherals $\to$ Layer 4 System Integration):

### Layer 0: Bus Arbitration & Clock Synchronization
- **Sub-Suite 2.1: Agnus Master DMA Contention & Bus Arbitration**
  - **Scope:** 225 tests (`Agnus/DMACON/`, `Agnus/BplDma/`, `Agnus/DIW/`, `Agnus/DDF/`, `Agnus/bususage/`).
  - **Core Silicon Capabilities:** CCK phase timing (`CCK1`/`CCK2`), even/odd cycle slot allocation, DRAM refresh (slots 0..3), CPU wait-state generation, Blitter Nasty (`BLTPRI`).
  - **Status:** Active focus (correcting Copper odd-cycle reservation to unblock CPU polling).

### Layer 1: Autonomous Coprocessors & DMA Channels
- **Sub-Suite 2.2: Agnus Blitter & Copper Engines**
  - **Scope:** 364 tests (250 Blitter, 114 Copper).
  - **Verified 100% Passing Clusters (59 tests):**
    - `sblit` cluster (16/16 PASS, 100.0%): Unconnected latch dither pair (`0xAAAA`/`0x5555`).
    - `fill` cluster (8/8 PASS, 100.0%): 2-hires pixel pipeline vs instant backdrop DAC timing.
    - `line` cluster (29/29 PASS, 100.0%): RetroShell cutout checkerboard, line mode `chold` invariance, and Copper 4-CCK WAIT pipeline.
    - `copper` base (6/6 PASS, 100.0%): 4-CCK instruction cycle timing, skip, and wake-up latency.
  - **In Progress:** `bbusy` cluster (resolving beam position parity on line 31/32 exit).

### Layer 2: Video Serializer & Display Pipeline
- **Sub-Suite 2.3: Denise Video, Bitplanes & Sprites**
  - **Scope:** 210 tests (`Denise/Registers/`, `Denise/Modes/`, `Denise/DIW/`, `Denise/Sprites/`).
  - **Verified Clusters:** DIW window clipping (5 tests passing 100.0%).
  - **Pending:** Dual-playfield priority multiplexing, sprite collision latches (`CLXDAT`/`CLXCON`).

### Layer 3: Peripherals & Interrupt Controllers
- **Sub-Suite 2.4: Paula Audio & Floppy Subsystem**
  - **Scope:** 107 tests (`Paula/Audio/`, `Paula/Interrupts/basicint/`).
  - **Verified Clusters:** 4 tests passing 100.0% (audio period clock division and Level 1–4 interrupt escalation).
- **Sub-Suite 2.5: Complex CIA-A / CIA-B & Timers**
  - **Scope:** Timers A & B, TOD clock 50/60 Hz synchronization, serial shift register (SDR), and port handshake lines.

### Layer 4: System Architecture & Mainboard
- **Sub-Suite 2.6: Mainboard, Addressing & Peripheral Assertions**
  - **Scope:** 58 tests (`Mainboard/`, `Memory/`, `Misc/`) + 193 non-visual peripheral assertion tests.
- **Sub-Suite 2.7: M68000 CPU Silicon Pipeline**
  - **Scope:** 503 tests (ALU, bitwise, shifts, exceptions, traps, autovectors). 7+ baseline tests verified passing 100.0%.

---

## 4. Operational Refresh Triggers & Maintenance Rules

To prevent documentation decay and eliminate unnecessary git churn:
1. **Milestone Completion Trigger:** This scorecard must be updated immediately upon completing any of the sub-suites in Section 3.
2. **Global Timing Alteration Trigger:** When a shared timing fix (e.g. Copper bus arbitration or DMA wait states) causes significant pass-rate shifts across multiple suites.
3. **Explicit User Request:** When the user requests an authoritative audit of verification status.

---

## 5. Related Architecture & Design Links

- [Testing Strategy and Quality Assurance.md](Testing%20Strategy%20and%20Quality%20Assurance.md) — Comprehensive testing philosophy and verification taxonomy.
- [Agnus.md](Agnus.md) — Agnus master DMA arbiter and Coprocessor architectures.
- [Denise.md](Denise.md) — Denise pixel serializer and display windowing.
- [Paula.md](Paula.md) — Paula audio and interrupt escalation pipeline.
- [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md) — Silicon traps and non-intuitive hardware invariants.
