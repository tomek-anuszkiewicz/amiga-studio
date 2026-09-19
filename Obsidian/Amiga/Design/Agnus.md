---
title: "Agnus (MOS 8370 / 8371 / 8372A) Architecture & Hardware Specification"
aliases: ["Agnus", "MOS 8370", "MOS 8371", "MOS 8372A"]
tags: ["amiga", "design", "agnus", "dma", "beam"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-06
updated: 2026-09-19
related: ["[Copper.md](Copper.md)", "[Blitter.md](Blitter.md)", "[DMA.md](DMA.md)", "[MemoryBus.md](MemoryBus.md)", "[Main loop A500.md](Main%20loop%20A500.md)", "[SaveState.md](SaveState.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/agnus"
last_synced_commit: "0fcd519"
last_synced_date: "2026-09-19"
---
# Agnus (MOS 8370 / 8371 / 8372A) Architecture & Hardware Specification

> [!NOTE]
> System execution constraints, memory bus arbitration, and Color Clock timing are defined in [AGENTS.md](../../../AGENTS.md), [MemoryBus.md](MemoryBus.md), and [Main loop A500.md](Main%20loop%20A500.md).
> Detailed inter-chip signal rules are codified in [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md).
> Save state structures for Agnus are specified in [SaveState.md](SaveState.md). Machine stepping and interrupt delivery are governed by [Main loop A500.md](Main%20loop%20A500.md). Video synchronization is coordinated with [Denise.md](Denise.md) and DMA audio/disk cycles with [Paula.md](Paula.md).
> Subordinate coprocessors and scheduling are specified in dedicated companion documents: [Copper.md](Copper.md), [Blitter.md](Blitter.md), and [DMA.md](DMA.md).

---

## 1. Scope & Chip Revisions

Agnus is the master bus controller, DMA arbiter, and primary coprocessor engine of the Amiga 500:

| Chip Model | Target Standard | Chip RAM Limit | Key Features |
| :--- | :---: | :---: | :--- |
| **MOS 8370** | NTSC OCS | **512 KB** (`$000000-$07FFFF`) | 262/263-line NTSC display timing, standard Blitter/Copper |
| **MOS 8371** | PAL OCS | **512 KB** (`$000000-$07FFFF`) | 312/313-line PAL display timing, standard Blitter/Copper |
| **MOS 8372A** | PAL / NTSC ECS | **1 MB** (`$000000-$0FFFFF`) | "Fat Agnus", pin-selectable PAL/NTSC, 1MB Chip RAM addressing |

---

## 2. Module Decomposition & Workspace Architecture

To prevent monolithic structures while maintaining strict flat Cargo workspace conventions, Agnus is partitioned into focused, single-responsibility crates under `crates/`:

```
crates/
├── copper/            // Agnus Copper coprocessor state machine (MOVE, WAIT, SKIP, CDANG)
├── blitter/           // 4-channel DMA Blitter, 256 minterms ALU, Bresenham line drawer
├── dma/               // Agnus scanline DMA slot scheduler (227.5 CCK horizontal schedule)
└── agnus/             // Agnus coordinator, beam counters (VHPOSR, VPOSR), register dispatch
```

### 2.1 Logical Subsystem Containment & Re-Exports
Although each subsystem lives in its own crate under `crates/` for fast, decoupled builds, `Agnus` logically encapsulates and re-exports them via the 3-tier re-export hierarchy:
```rust
pub use blitter;
pub use copper;
pub use dma;

pub struct Agnus {
    pub model: AgnusModel,
    pub copper: copper::Copper,
    pub blitter: blitter::Blitter,
    pub dma: dma::DmaScheduler,
    // ... beam counters, registers, and in-flight mutation pipeline
}
```

---

## 3. Register Memory Map

All Agnus registers are mapped within the Custom Chip register space (`$DFF000`–`$DFF07E`, `$DFF080`–`$DFF096`):

| Address | R/W | Symbol | Description |
| :--- | :---: | :--- | :--- |
| **`$DFF002`** | R | **`DMACONR`** | DMA Control register read (channel active flags & Blitter Nasty status) |
| **`$DFF004`** | R | **`VHPOSR`** | Vertical & Horizontal beam position read |
| **`$DFF006`** | R | **`VPOSR`** | Vertical beam position high bit, chip ID (PAL/NTSC), and `LOF` flag |
| **`$DFF02E`** | W | **`COPCON`** | Copper Control register (bit 1: `CDANG` Copper Danger mode) |
| **`$DFF040`** | W | **`BLTCON0`** | Blitter Control 0 (channel enables A–D, minterms LF0–LF7, shift A) |
| **`$DFF042`** | W | **`BLTCON1`** | Blitter Control 1 (shift B, descending flag, line mode, fill mode) |
| **`$DFF044`** | W | **`BLTAFWM`** | Blitter First Word Mask for Channel A |
| **`$DFF046`** | W | **`BLTALWM`** | Blitter Last Word Mask for Channel A |
| **`$DFF048`** | W | **`BLTCPTH`** | Blitter Channel C Pointer (High 5 bits) |
| **`$DFF04A`** | W | **`BLTCPTL`** | Blitter Channel C Pointer (Low 16 bits) |
| **`$DFF04C`** | W | **`BLTBPTH`** | Blitter Channel B Pointer (High 5 bits) |
| **`$DFF04E`** | W | **`BLTBPTL`** | Blitter Channel B Pointer (Low 16 bits) |
| **`$DFF050`** | W | **`BLTAPTH`** | Blitter Channel A Pointer (High 5 bits) |
| **`$DFF052`** | W | **`BLTAPTL`** | Blitter Channel A Pointer (Low 16 bits) |
| **`$DFF054`** | W | **`BLTDPTH`** | Blitter Channel D Pointer (High 5 bits) |
| **`$DFF056`** | W | **`BLTDPTL`** | Blitter Channel D Pointer (Low 16 bits) |
| **`$DFF058`** | W | **`BLTSIZE`** | Blitter start trigger: Height (rows, bits 6–15) and Width (words, bits 0–5) |
| **`$DFF060`** | W | **`BLTCMOD`** | Blitter Channel C Modulo (signed 16-bit) |
| **`$DFF062`** | W | **`BLTBMOD`** | Blitter Channel B Modulo (signed 16-bit) |
| **`$DFF064`** | W | **`BLTAMOD`** | Blitter Channel A Modulo (signed 16-bit) |
| **`$DFF066`** | W | **`BLTDMOD`** | Blitter Channel D Modulo (signed 16-bit) |
| **`$DFF070`** | W | **`BLTCDAT`** | Blitter Channel C Data latch |
| **`$DFF072`** | W | **`BLTBDAT`** | Blitter Channel B Data latch |
| **`$DFF074`** | W | **`BLTADAT`** | Blitter Channel A Data latch |
| **`$DFF080`** | W | **`COP1LCH`** | Copper First Location Pointer (High 5 bits) |
| **`$DFF082`** | W | **`COP1LCL`** | Copper First Location Pointer (Low 16 bits) |
| **`$DFF084`** | W | **`COP2LCH`** | Copper Second Location Pointer (High 5 bits) |
| **`$DFF086`** | W | **`COP2LCL`** | Copper Second Location Pointer (Low 16 bits) |
| **`$DFF088`** | W/R | **`COPJMP1`** | Copper Restart at `COP1LC` (strobe on write or read; read returns open-bus `$FFFF`) |
| **`$DFF08A`** | W/R | **`COPJMP2`** | Copper Restart at `COP2LC` (strobe on write or read; read returns open-bus `$FFFF`) |
| **`$DFF08C`** | W | **`COPINS`**  | Copper Instruction register latch |
| **`$DFF096`** | W | **`DMACON`**  | DMA Control write (bit 15: SET/CLR, bits 0–14: channel enables) |

### 3.1 Register Access Semantics & Propagation Latency Pipeline
- **Immediate Latch Reads:** `DMACONR` (`$DFF002`), `VHPOSR` (`$DFF004`), and `VPOSR` (`$DFF006`) return the active hardware register state immediately on the bus read phase with zero delay.
- **Write Staging Buffer:** Agnus embeds an inline fixed-capacity mutation array `[Option<DelayedMutation>; 64]` sizing to its addressable write register set.
- **Propagation Timing:**
  - `DMACON` (`$DFF096`): Propagates with 2 CCK delay (`MutationMode::OverwritePending`). Bit 15 determines SET/CLR behavior. Propagates simultaneously across the bus to synchronize Paula DMA channel enables (`AUD0..3`, `DSK`).
  - `BPLCON0` (`$DFF100` mirror): Propagates with 4 CCK delay (`MutationMode::OverwritePending`) to synchronize Agnus bitplane DMA slot schedule.
  - Strobes (`COPJMP1`, `COPJMP2`, `BLTSIZE`): Propagate with 2 CCK delay (`MutationMode::OverwritePending`).
- **Defensive Overflow Protection:** If debugger injections saturate the 64-slot buffer, writes commit immediately with a defensive error log, preserving zero-panic invariants.

---

## 4. Master Beam Counters (`VHPOSR` & `VPOSR`)

Agnus generates display timing and raster position counters synchronized to the Color Clock:

```
VHPOSR ($DFF004):
Bits 15-8: V7-V0  (Vertical scanline low 8 bits)
Bits  7-0: H8-H1  (Horizontal Color Clock position bits 8 to 1; H0 is internal sub-CCK)

VPOSR ($DFF006):
Bit    15: LOF    (Long Frame bit: toggles every frame in interlace mode)
Bits 14-8: Chip ID (0 = OCS PAL 8371 / NTSC 8370; 1 = ECS Fat Agnus 8372A)
Bit     0: V8     (Vertical scanline bit 8)
```

- **PAL Scanning:** $312$ lines ($0$ to $311$). $V_8$ is set for scanlines $\ge 256$. Total CCKs per frame: $70,937$ ($\approx 50.00\ \text{Hz}$).
- **NTSC Scanning:** $262$ lines ($0$ to $261$). Total CCKs per frame: $59,605$ ($\approx 60.05\ \text{Hz}$).
- **Horizontal Range:** Counts $0$ to $227$ CCKs per line (alternating 227/228 on PAL).

### 4.1 Beam Counter Implementation (`chips/agnus/beam.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoStandard {
    Pal,
    Ntsc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeamCounter {
    standard: VideoStandard,
    hpos: u16,
    vpos: u16,
    lof: bool,
}

impl BeamCounter {
    pub fn new(standard: VideoStandard) -> Self {
        Self {
            standard,
            hpos: 0,
            vpos: 0,
            lof: false,
        }
    }

    /// Advance raster beam by 1 CCK tick
    #[inline(always)]
    pub fn step_cck(&mut self) {
        let max_hpos = self.line_cck_count(self.vpos);
        self.hpos += 1;
        if self.hpos >= max_hpos {
            self.hpos = 0;
            self.vpos += 1;
            let max_vpos = match self.standard {
                VideoStandard::Pal => 312,
                VideoStandard::Ntsc => 262,
            };
            if self.vpos >= max_vpos {
                self.vpos = 0;
                self.lof = !self.lof; // Toggle interlace field
            }
        }
    }

    /// Returns the number of CCKs for the current scanline (alternating 227 and 228 on PAL)
    #[inline(always)]
    pub fn line_cck_count(&self, line: u16) -> u16 {
        match self.standard {
            VideoStandard::Pal => if (line & 1) == 0 { 228 } else { 227 },
            VideoStandard::Ntsc => 227,
        }
    }

    #[inline(always)]
    pub fn hpos(&self) -> u16 { self.hpos }

    #[inline(always)]
    pub fn vpos(&self) -> u16 { self.vpos }

    #[inline(always)]
    pub fn lof(&self) -> bool { self.lof }

    /// Format value for VHPOSR register ($DFF004)
    #[inline(always)]
    pub fn read_vhposr(&self) -> u16 {
        ((self.vpos & 0xFF) << 8) | ((self.hpos >> 1) & 0xFF)
    }

    /// Format value for VPOSR register ($DFF006)
    #[inline(always)]
    pub fn read_vposr(&self, is_ecs: bool) -> u16 {
        let lof_bit = if self.lof { 0x8000 } else { 0x0000 };
        let chip_id = if is_ecs { 0x2000 } else { 0x0000 };
        let v8_bit = if self.vpos >= 256 { 0x0001 } else { 0x0000 };
        lof_bit | chip_id | v8_bit
    }
}
```

---

## 5. Subordinate Coprocessors & Subsystems

Agnus encapsulates three specialized companion engines, fully specified in their respective architecture documents:

### 5.1 DMA Arbitration & Slot Scheduling
- **Master DMA Address Generator:** Agnus drives Chip RAM memory addresses and RGA bus register strobes across all channels.
- **Scanline Partitioning:** 227.5 CCK horizontal schedule allocating fixed slots for Refresh, Floppy, Audio, and Sprites, dynamic slots for Bitplanes, and residual cycles for Copper, Blitter, and CPU.
- **Contention Management:** Monitors Chip RAM blocking and arbitrates Blitter Nasty vs Normal mode with 3-cycle CPU starvation yield.
- *Authoritative Specification:* See [DMA.md](DMA.md).

### 5.2 Copper Display Coprocessor
- **Synchronized Coprocessor:** Executes 32-bit instructions (`MOVE`, `WAIT`, `SKIP`) in lockstep with the raster beam.
- **Danger Mode Protection:** Manages `CDANG` register write permissions below `$DFF080` via `COPCON`.
- **List Pointers & Strobes:** Manages `COP1LC`, `COP2LC`, `COPJMP1`, `COPJMP2`, and VBlank reset.
- *Authoritative Specification:* See [Copper.md](Copper.md).

### 5.3 4-Channel Blitter
- **Bit-Block Transferrer:** 4 DMA channels (A, B, C, D) supporting arbitrary rectangle copies, 256 boolean minterms, and sub-word barrel shifting.
- **Vector Line Drawing:** Hardware Bresenham line drawer with slope accumulators and octant direction control.
- **Area Fill & Zero Detect:** Inclusive/exclusive area filling and Zero flag evaluation.
- *Authoritative Specification:* See [Blitter.md](Blitter.md).

---

## 6. Reset Defaults & Coordination

- **`DMACON` (`$DFF096`):** Reset to **`$0000`** (all DMA channels disabled). Agnus immediately releases the Chip RAM bus, ensuring the CPU has unblocked access.
- **`COPCON` (`$DFF02E`):** Reset to **`$0000`** (`CDANG = 0`, Copper danger registers protected).
- **Copper State:** Halted until re-enabled by CPU via `DMACON` and triggered via `COPJMP1`.
- **Blitter State:** Idle (`BLTDONE` asserted, busy flag cleared).

---

## 7. Reference Documentation & Upstream Ground Truth

- [Copper Architecture Specification](Copper.md): Comprehensive 3-instruction coprocessor state machine and timing.
- [Blitter Architecture Specification](Blitter.md): 4-channel DMA block transferrer, minterms, and Bresenham line mode.
- [DMA Architecture & Scheduling](DMA.md): 227.5 CCK horizontal slot scheduling and 8-tier priority hierarchy.
- [Amiga Hardware Reference Manual: Chapter 2 (Coprocessor Hardware)](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md): Authoritative specification for Copper instruction formats (`MOVE`, `WAIT`, `SKIP`).
- [Amiga Hardware Reference Manual: Chapter 6 (Blitter Hardware)](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Circuit principles for 4-channel DMA Blitter, minterm generator (`BLTCON0`), shifters, and line drawing.
- [Amiga Hardware Reference Manual: Appendix B (Register Summary)](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Complete memory-mapped address order table and bitfield masks for all Agnus registers (`$DFF000`–`$DFF07E`).
- [vAmiga Agnus Component Implementation](../../../ref_src/vAmiga-4.5/Core/Components/Agnus/Agnus.cpp): Reference coordinator implementation for beam counters and custom register access.
