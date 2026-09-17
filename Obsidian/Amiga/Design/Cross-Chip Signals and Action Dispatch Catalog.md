---
title: "Cross-Chip Signals and Action Dispatch Catalog"
aliases: ["Cross-Chip Signals Catalog", "Action Dispatch Catalog", "Inter-Chip Signal Matrix", "Propagation Delay Catalog"]
tags: ["amiga", "design", "chips", "signals", "propagation", "action-dispatch"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-14
updated: 2026-09-14
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Floppy.md](Floppy.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Cross-Chip Signals and Action Dispatch Catalog

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Subsystem Specifications:** [MemoryBus.md](MemoryBus.md) | [Agnus.md](Agnus.md) | [Denise.md](Denise.md) | [Paula.md](Paula.md) | [CIA.md](CIA.md) | [Floppy.md](Floppy.md)
- **Companion Register Matrix:** [Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)
- **Hardware Quirks Index:** [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)

---

## 1. Architectural Scope: The Closed Silicon Signal Space

In the Amiga 500 hardware architecture, cross-chip communication does not use a dynamic software event-bus. The system consists of fixed silicon chips connected by discrete copper traces on the motherboard PCB.

> [!IMPORTANT]
> **The Closed-Set Invariant:**
> Over 90% of custom register writes stay entirely within the silicon boundaries of the chip that owns them (e.g. palette colors stay in Denise, audio volume stays in Paula).
> Only a strictly finite set of ~8 to 10 hardware signals ever cross physical chip boundaries to influence another subsystem.

This catalog establishes the definitive inventory of cross-chip boundary signals, their calibrated propagation latencies, conflict resolution behavior during in-flight writes, and their target action methods.

---

## 2. Electronic Propagation Latency & In-Flight Mutation Semantics

In physical NMOS silicon, signals do not propagate instantaneously across bus buffers and internal logic gates. When software writes to a register, the value enters an internal gate propagation pipeline and takes effect after a calibrated number of Color Clock phases ($K$ CCKs).

### The Two Mutation Propagation Modes

Our emulator models this physical behavior via `MutationMode` in [`crates/config/src/mutation.rs`](../../../crates/config/src/mutation.rs):

```mermaid
flowchart LR
    subgraph OVERWRITE ["MutationMode::OverwritePending (Control & Strobes)"]
        W1["Write 1 ($096 = $8200) at T=0\nDelay: 2 CCKs"] -->|T=1: In Flight (rem=1)| OVR["Write 2 ($096 = $0200) at T=1\nOVERWRITES Write 1!"]
        OVR -->|Restart countdown| W2["Delay: 2 CCKs (rem=2)\nCommits at T=3"]
    end
    
    subgraph PIPELINE ["MutationMode::Pipeline (Streaming Data)"]
        P1["Write 1 ($180 = Red) at T=0\nDelay: 1 CCK"] --> FIFO["Both Coexist in FIFO"]
        P2["Write 2 ($180 = Blue) at T=0.5\nDelay: 1 CCK"] --> FIFO
        FIFO --> C1["Red commits at T=1"]
        FIFO --> C2["Blue commits at T=1.5"]
    end
```

#### 1. `MutationMode::OverwritePending` (Control, Strobe & Configuration Registers)
- **Applicable Registers:** `DMACON`, `BPLCON0`, `ADKCON`, `INTENA`, `INTREQ`, `DSKLEN`, `DSKSYNC`, `COPJMP1/2`, `BLTSIZE`, `SERPER`, `POTGO`.
- **Physical Reality:** In control logic, register bits are held by gate capacitances and latches. If the CPU rapidly writes a new value while a previous voltage transition is still propagating through input buffers, the input electrical level adopts the newest value, resetting the latch stabilization time.
- **Emulator Behavior:** If an in-flight mutation for the exact same register already exists in the buffer:
  - The staged value is **replaced** with the new incoming value.
  - The remaining countdown is **reset** back to the full calibrated delay (`remaining_cck = delay_cck`).
  - The earlier write is discarded without committing.

#### 2. `MutationMode::Pipeline` (Streaming Data & Waveforms)
- **Applicable Registers:** `COLOR00..31`, `AUD0DAT..AUD3DAT`, `BLTADAT..BLTCDAT`, `SERDAT`.
- **Physical Reality:** Video rasters and audio DACs process high-speed streams where each written word represents a discrete temporal sample moving down an internal shift register or FIFO.
- **Emulator Behavior:** Subsequent writes to the same register **coexist** in the buffer. Each write preserves its independent timer and commits in strict temporal FIFO order.

#### 3. Defensive Overflow Fallback
Every chip embeds an inline fixed-capacity mutation array (Agnus: 64, Denise: 64, Paula: 32, CIA: 16) with zero runtime heap allocations. If extreme pathological guest code exhausts the buffer capacity, the emulator falls back to an **immediate commit**, preventing data loss, pipeline desynchronization, or host panics.

---

## 3. The Authoritative Cross-Chip Signal Matrix

The following table documents every cross-chip signal in the Amiga 500:

| Source Subsystem | Trigger Register / Event | Physical Wire / Bus Signal | Calibrated Delay | Mutation Mode | Target Subsystem | Architectural Action Method | Hardware Consequence |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Agnus** | `DMACON` ($096) write | Internal DMA Control Bus | 2 CCKs | `OverwritePending` | **Agnus** (Copper, Blitter) & **Paula** (Audio, Disk) | `dma.write_dmacon(val)`<br/>`audio.set_dma_enables(...)`<br/>`floppy.set_dma_enabled(...)` | Masters DMA gating across Copper, Blitter, Sprites, Bitplanes, Disk, and Audio. |
| **Agnus** | `COPJMP1` ($088) strobe | Copper Restart Strobe 1 | 1 CCK | `OverwritePending` | **Copper** | `copper.strobe_jump1(cop1lc)` | Forces Copper program counter to reload from `COP1LC` on the next CCK phase. |
| **Agnus** | `COPJMP2` ($08A) strobe | Copper Restart Strobe 2 | 1 CCK | `OverwritePending` | **Copper** | `copper.strobe_jump2(cop2lc)` | Forces Copper program counter to reload from `COP2LC` on the next CCK phase. |
| **Agnus** | `BLTSIZE` ($058) write | Blitter Start Trigger | 1 CCK | `OverwritePending` | **Blitter** | `blitter.sync_pointers(...)`<br/>`blitter.trigger_blit(val)` | Latches dimensions, copies staging pointers from Agnus, and sets `is_busy = true`. |
| **Agnus** | `DIWSTRT` ($08E) / `DIWSTOP` ($090) | Display Window Clipping Bus | 4 CCKs | `OverwritePending` | **Denise** | `denise.set_diw(strt, stop)` | Configures raster beam horizontal and vertical screen blanking boundaries. |
| **Agnus** | Blitter operation completes | `_BLITINT` (Interrupt line) | Live | Event Strobe | **Paula** | `paula.set_interrupt_request(0x0040)` | Asserts Level 3 interrupt request (bit 6 of `INTREQ`). |
| **Denise** | `BPLCON0` ($100) write | Bitplane Mode Bus | 4 CCKs (Agnus) / 1 CCK (Denise) | `OverwritePending` | **Agnus** | `agnus.set_bplcon0(val)` | Agnus allocates 0 to 6 DMA time slots per scanline based on BPL planecount. |
| **Paula** | `DSKLEN` ($024) write 2 | Disk DMA Arm / Start | 2 CCKs | `OverwritePending` | **FloppyController** | `floppy.set_dsklen(val)` | 2nd consecutive write with bit 15 set activates MFM DMA streaming. |
| **Paula** | `DSKSYNC` ($07E) write | MFM Word Sync Match | 2 CCKs | `OverwritePending` | **FloppyController** | `floppy.set_dsksyn(val)` | Updates the 16-bit bitstream match pattern (default `$4489`). |
| **Paula** | Audio channel sample buffer finishes | `AUDxDSR` (DMA Strobe/Restart) | Live | Event Strobe | **Agnus** | `agnus.reload_audio_ptr(channel)` | Agnus reloads active pointer `audpt[ch] = audlc[ch]` for looping. |
| **Paula** | Audio buffer finishes or floppy sync matches | `_INT4` / `_INT5` / `_INT1` | Live | Latch | **CPU (68000)** | `machine.resolve_ipl()` -> `cpu.state.ipl` | Updates CPU interrupt priority level (IPL 1..6) based on unmasked `INTREQ`. |
| **CIA-A** | Port A ($BFE001) bit 0 write | `_OVL` (Low-Memory Overlay) | 5 CCKs (1 E-Clock) | `OverwritePending` | **PhysicalMemory** | `mem.map_chip_ram_to_low_memory()` / `mem.map_kickstart_to_low_memory()` | Unmaps Kickstart ROM from `$000000` after reset vector boot execution. |
| **CIA-B** | Port B ($BFD100) write | Ribbon Cable Control Pins | 5 CCKs (1 E-Clock) | `OverwritePending` | **FloppyDrive (DF0..DF3)** | `floppy.handle_ciab_port_b_write(val)` | Falling edge of `_SELx` latches `_MTR`; falling edge of `_STEP` moves head. |
| **FloppyDrive** | Head position & mechanical sensors | Ribbon Cable Sense Pins | Live | Pin Sampling | **CIA-A** | `cia_a.set_input_pins_a(pins, 0x3C)` | Samples `_RDY`, `_TK0`, `_WPROT`, and `_CHNG` into Port A bits 2..5. |

---

## 4. Scanline Fixed DMA Slot Allocation Schedule

Outside of CPU-driven register writes, Agnus drives autonomous hardware DMA transfers during dedicated horizontal Color Clock slots on every scanline:

```
CCK Slot:   0   1   2   3   4   5   6   7   8   ...  12  ...  27  ... 227.5
Channel:  [ DRAM Refresh ] [DSK] [AUD0] [AUD1] [AUD2] [AUD3] ... [ Sprites 0..7 ] ... [ Bitplanes / Free ]
Target:    Physical DRAM    Floppy     Paula Audio Engine           Denise Video            Denise / Blitter
```

1. **Slots 0..3 (DRAM Refresh):** Agnus refreshes dynamic RAM banks.
2. **Slot 4 (Floppy Disk DMA):** If `DSKEN` and `DMAEN` are active in `DMACON`, Agnus fetches/stores 1 word from `dskpt` and advances pointer.
3. **Slots 5..8 (Audio Channels 0..3):** If `AUDxEN` is active and Paula has requested data:
   - Agnus reads Chip RAM at `audpt[ch]` and increments `audpt[ch] += 2`.
   - Paula latches the word into `auddat[ch]`, decrements `audlen[ch]`.
   - If length counter finishes ($1$), Paula asserts `AUDxDSR` to reload Agnus `audpt[ch] = audlc[ch]` and sets `INTREQ` audio interrupt.
4. **Slots 12..27 (Hardware Sprites 0..7):** Agnus reads sprite position and image data words directly into Denise sprite line buffers.

---

## 5. Reference Documentation & Upstream Ground Truth

- **Commodore Amiga Hardware Reference Manual**:
  - [Hardware Reference Manual](../Reference/Hardware%20Reference%20Manual): Primary reference for custom chip register addresses, bit allocations, and DMA channel priorities.
  - [Chapter 1 - Introduction to the Amiga Architecture](../Reference/Hardware%20Reference%20Manual/01%20-%20Chapter%201%20-%20Introduction.md): Custom chip physical bus layout, shared Chip RAM arbitration, and system bus timing.
- **Undocumented Chipset Features**:
  - [Undocumented features of OCS, ECS and AGA chipsets.md](../Reference/Undocumented%20features%20of%20OCS,%20ECS%20and%20AGA%20chipsets.md): Definitive compendium of undocumented silicon behavior across Agnus, Denise, and Paula.
- **Silicon Reference Emulators**:
  - [`vAmiga Reference Source`](../../../ref_src/vAmiga): Clean-room reference modeling for Agnus DMA slot allocations and cross-chip bus latencies.
  - [`vAmigaTS Test Suite`](../../../ref_src/vAmigaTS): Comprehensive hardware verification suite covering DMA cycle contention and bus interactions.
- **Living Crate Source Files**:
  - [`crates/config/src/mutation.rs`](../../../crates/config/src/mutation.rs): Definition of `MutationMode`, `DelayedMutation`, and electronic propagation countdown pipelines.
  - [`crates/memory_bus/src/memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Motherboard address router (`MemoryBus`) routing custom register accesses and CIA port selects.
  - [`crates/machine_loop/src/machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs): Machine loop coordinating CCK progression, action dispatch, and interrupt priority resolution.

