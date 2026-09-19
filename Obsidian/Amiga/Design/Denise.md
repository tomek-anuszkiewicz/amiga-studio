---
title: "Denise (MOS 8362 / 8373) Architecture & Hardware Specification"
aliases: ["Denise", "MOS 8362", "MOS 8373", "Video", "Bitplanes"]
tags: ["amiga", "design", "denise", "video", "display"]
category: "Design"
subsystem: "denise"
status: "active"
created: 2026-09-06
updated: 2026-09-19
related: ["[Sprites.md](Sprites.md)", "[Frame Buffer.md](Frame%20Buffer.md)", "[Agnus.md](Agnus.md)", "[MemoryBus.md](MemoryBus.md)", "[Main loop A500.md](Main%20loop%20A500.md)", "[Joystick.md](Joystick.md)", "[Mouse.md](Mouse.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/denise"
last_synced_commit: "0fcd519"
last_synced_date: "2026-09-19"
---
# Denise (MOS 8362 / 8373) Architecture & Hardware Specification

> [!NOTE]
> System execution constraints, memory bus arbitration, and Color Clock timing are defined in [AGENTS.md](../../../AGENTS.md), [MemoryBus.md](MemoryBus.md), and [Main loop A500.md](Main%20loop%20A500.md).
> Detailed inter-chip signal rules are codified in [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md).
> Detailed game port pinouts and host input bindings are documented in [Joystick.md](Joystick.md) and [Mouse.md](Mouse.md). Raster beam tracking is driven by [Agnus.md](Agnus.md), and display output feeds the frontend in [GUI.md](GUI.md) and [GUI Specification.md](GUI%20Specification.md).
> Hardware sprite multiplexing is specified in [Sprites.md](Sprites.md), and frame buffer generation in [Frame Buffer.md](Frame%20Buffer.md).

---

## 1. Scope & Chip Revisions

Denise is the video display processor and game port interface for the Amiga:

| Chip Model | Architecture | Pixel Clock | Key Capabilities |
| :--- | :---: | :---: | :--- |
| **MOS 8362** | OCS | **7 MHz / 14 MHz** | Up to 6 bitplanes, 32 palette colors (RGB444), HAM6, EHB, 8 sprites |
| **MOS 8373** | ECS | **7 MHz / 14 MHz / 28 MHz** | Programmable border blanking, Super-Hires ($1280$ pixels), ECS sprite resolution |

---

## 2. Module Decomposition & Workspace Architecture

Denise is partitioned into focused, single-responsibility workspace crates under `crates/`:

```
crates/
├── sprites/           // Denise 8 hardware sprites, position comparators, attached pairs
├── frame_builder/     // Denise raster scanline pixel compositor and 32-bit ARGB frame buffer
└── denise/            // Denise coordinator, video controls (BPLCON0..3), palette, collisions
```

### 2.1 Logical Subsystem Containment & Re-Exports
In accordance with the 3-tier re-export hierarchy, `crates/denise` encapsulates and re-exports its companion crates:
```rust
pub use frame_builder;
pub use sprites;

pub struct Denise {
    pub model: DeniseModel,
    pub sprites: sprites::Sprites,
    pub frame_builder: frame_builder::FrameBuilder,
    pub bplcon0: u16,
    pub color: [u16; COLOR_PALETTE_SIZE],
    // ... collision registers, window coordinates, and in-flight mutation pipeline
}
```

### 2.2 Passive Bus Latching & Zero Direct Memory Reads Invariant
Per [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md) and [General Architecture.md](General%20Architecture.md):
- **Zero DMA Address Generators:** Denise contains **no DMA pointer registers and no address generation circuitry**. All bitplane pointers (`BPLxPT`) and sprite pointers (`SPRxPT`) physically reside inside Agnus.
- **Zero Direct Memory Reads:** Denise **never holds a reference to `PhysicalMemory` and never executes `memory.read()`**.
- **Passive Data Latching:** During bitplane and sprite DMA time slots, Agnus drives the Chip RAM address and asserts `BPLxDAT` (`$110`..`$11A`) or `SPRxDAT`/`POS`/`CTL` (`$140`..`$17E`) on the internal `RGA` bus. Denise passively latches the 16-bit word off the shared data bus into its holding registers (`write_bpldat`, `sprites.write_reg`).
- **Prohibition of Direct Inter-Chip Smuggling:** Denise never directly calls methods on Agnus, Paula, or CPU. All synchronization occurs through beam coordinates and bus strobes routed by the top-level machine coordinator.

---

## 3. Register Memory Map

All Denise registers are mapped in the Custom Chip register space (`$DFF000`–`$DFF1BE`):

| Address | R/W | Symbol | Description |
| :--- | :---: | :--- | :--- |
| **`$DFF00A`** | R | **`JOY0DAT`** | Game Port 1 Joystick / Mouse quadrature counters |
| **`$DFF00C`** | R | **`JOY1DAT`** | Game Port 2 Joystick / Mouse quadrature counters |
| **`$DFF00E`** | R | **`CLXDAT`** | Collision Data Register (cleared upon read) |
| **`$DFF012`** | R | **`POT0DAT`** | Port 1 Proportional (potentiometer / button 2/3) counter |
| **`$DFF014`** | R | **`POT1DAT`** | Port 2 Proportional (potentiometer / button 2) counter |
| **`$DFF034`** | W | **`POTGO`** | Proportional pin drive and start timer |
| **`$DFF08E`** | W | **`DIWSTRT`** | Display Window Start (upper-left corner: VSTART, HSTART) |
| **`$DFF090`** | W | **`DIWSTOP`** | Display Window Stop (lower-right corner: VSTOP, HSTOP) |
| **`$DFF092`** | W | **`DDFSTRT`** | Display Data Fetch Start (bitplane DMA start CCK) |
| **`$DFF094`** | W | **`DDFSTOP`** | Display Data Fetch Stop (bitplane DMA stop CCK) |
| **`$DFF098`** | W | **`CLXCON`** | Collision Control (match masks and enable bits) |
| **`$DFF100`** | W | **`BPLCON0`** | Bitplane Control 0 (plane count BPU, HIRES, HAM, DBLPF, COLOR) |
| **`$DFF102`** | W | **`BPLCON1`** | Bitplane Control 1 (horizontal scroll offsets for PF1 and PF2) |
| **`$DFF104`** | W | **`BPLCON2`** | Bitplane Control 2 (playfield and sprite priority arbitration) |
| **`$DFF106`** | W | **`BPLCON3`** | Bitplane Control 3 (ECS border color, enhanced sprite control) |
| **`$DFF108`** | W | **`BPL1MOD`** | Bitplane Modulo (odd bitplanes 1, 3, 5) |
| **`$DFF10A`** | W | **`BPL2MOD`** | Bitplane Modulo (even bitplanes 2, 4, 6) |
| **`$DFF110`–`$DFF11A`** | W | **`BPL1DAT`–`BPL6DAT`** | Bitplane 1–6 Data Holding Latches |
| **`$DFF140`–`$DFF178`** | W | **`SPR0POS`–`SPR7POS`** | Sprite 0–7 Horizontal and Vertical Start Position |
| **`$DFF142`–`$DFF17A`** | W | **`SPR0CTL`–`SPR7CTL`** | Sprite 0–7 Stop Position and Attach Control |
| **`$DFF144`–`$DFF17E`** | W | **`SPR0DATA/B`–`SPR7DATA/B`** | Sprite 0–7 Image Data Registers |
| **`$DFF180`–`$DFF1BE`** | W | **`COLOR00`–`COLOR31`** | 32 Palette Color Registers (12-bit RGB444: 4 bits R, 4 bits G, 4 bits B) |

### 3.1 Register Access Semantics & Propagation Latency Pipeline
- **Clear-on-Read Mechanics:** `CLXDAT` (`$DFF00E`) latches sprite and playfield collision flags. Reading `CLXDAT` returns the active collision state and immediately clears all collision latches (`0x0000`). Debugger inspections via `peek_register(0x00E)` read non-destructively without clearing.
- **Write Staging Buffer:** Denise embeds an inline fixed-capacity mutation array `[Option<DelayedMutation>; 64]` sizing to its addressable write register set (`COLOR00..31`, `BPLCON0..3`, `SPR0..7`).
- **Propagation Timing:**
  - `BPLCON0` (`$DFF100`): Propagates with 1 CCK delay (`MutationMode::OverwritePending`). Denise decodes bitplane count and display mode within 1 CCK of the bus write.
  - Palette Registers (`COLOR00`–`COLOR31`): Propagate with 1 CCK delay (`MutationMode::Pipeline`) ensuring raster colors change deterministically on the subsequent Color Clock.
  - Display Window Registers (`DIWSTRT`, `DIWSTOP`, `DDFSTRT`, `DDFSTOP`): Propagate with 1 CCK delay (`MutationMode::OverwritePending`).
- **Defensive Overflow Protection:** If debugger injections saturate the 64-slot buffer, writes commit immediately with a defensive error log, preserving zero-panic invariants.

---

## 4. Pixel Pipeline & Display Modes

Denise receives bitplane word data fetched by Agnus DMA into latches `BPL1DAT`–`BPL6DAT`, serializes the bits into pixel streams, and outputs 12-bit RGB color signals:

```mermaid
flowchart TD
    DMA["Agnus DMA Fetches\n(BPL1DAT..BPL6DAT)"] --> SHIFT["Bitplane Serializers\n(1..6 Parallel Shift Registers)"]
    SHIFT --> SCROLL["Horizontal Delay Subsystem\n(BPLCON1: 0..15 Pixels Fine Scroll)"]
    SCROLL --> MODE_DECODE{"Display Mode Decoder\n(BPLCON0)"}
    
    MODE_DECODE -->|Standard| LUT["Palette Lookup (COLOR00..COLOR31)"]
    MODE_DECODE -->|Dual Playfield| DPF["Split PF1 (1,3,5) & PF2 (2,4,6)\nPriority via BPLCON2"]
    MODE_DECODE -->|EHB| EHB["Bit 6 = 1 -> Halve RGB Luminance"]
    MODE_DECODE -->|HAM6| HAM["Hold & Modify Engine\n(4096 Simultaneous Colors)"]
    
    DPF --> PRIORITY["Sprite / Playfield Priority Mixer"]
    LUT --> PRIORITY
    EHB --> PRIORITY
    HAM --> PRIORITY
    
    SPRITES["8 Hardware Sprites (SPR0..SPR7)\n(crates/sprites)"] --> PRIORITY
    PRIORITY --> RGB_OUT["12-Bit RGB444 Video Output"]
    RGB_OUT --> FB["Raster Frame Builder\n(crates/frame_builder)"]
```

### 4.1 Standard Bitplane Modes
- **Low-Resolution (320 Pixels):** Clocked at $7.09\ \text{MHz}$ ($140\ \text{ns}$ per pixel, $1$ pixel per CCK). Supports $1$ to $6$ bitplanes ($2$ to $32$ palette colors).
- **High-Resolution (640 Pixels):** Clocked at $14.19\ \text{MHz}$ ($70\ \text{ns}$ per pixel, $2$ pixels per CCK). Supports $1$ to $4$ bitplanes ($2$ to $16$ palette colors).

### 4.2 Dual Playfield Mode (`DBLPF` in `BPLCON0`)
- Splits the active bitplanes into two independent, overlapping playfield surfaces:
  - **Playfield 1:** Formed by odd bitplanes ($1, 3, 5$). Selects colors from `COLOR00`–`COLOR07`. Color $0$ is transparent.
  - **Playfield 2:** Formed by even bitplanes ($2, 4, 6$). Selects colors from `COLOR08`–`COLOR15`. Color $0$ is transparent.
- **Priority (`BPLCON2` bit 6 `PF2PRI`):** Determines whether Playfield 1 or Playfield 2 is in front.
- **Independent Scrolling:** Each playfield scrolls independently using the 4-bit nibbles in `BPLCON1` ($0$ to $15$ pixel delays).

### 4.3 Extra Half-Brite Mode (EHB)
- Activated when $6$ bitplanes are enabled in Low-Resolution without HAM or Dual Playfield.
- If Bitplane 6 is `0`: Color index ($0..31$) is read directly from `COLOR00`–`COLOR31`.
- If Bitplane 6 is `1`: The color index ($0..31$) is retrieved from `COLOR00`–`COLOR31`, but all RGB components are shifted right by 1 ($R/2, G/2, B/2$), creating 32 half-intensity shadow colors for a total of 64 simultaneous colors.

### 4.4 Hold-And-Modify Mode (HAM6)
- Activated via bit 11 (`HOMOD`) in `BPLCON0` with 6 bitplanes active.
- Provides **4,096 simultaneous colors** by modifying one RGB color component per pixel while holding the other two from the previous pixel:

| Control Bits (Planes 6 & 5) | Operation | Resulting Output Pixel Color |
| :---: | :--- | :--- |
| **`00`** | **Palette Lookup** | Output color is looked up in `COLOR00`–`COLOR15` using Planes 1–4. |
| **`01`** | **Modify Blue** | Hold Red and Green from preceding pixel; replace Blue with 4-bit data ($B = \text{Planes 1–4}$). |
| **`10`** | **Modify Red** | Hold Green and Blue from preceding pixel; replace Red with 4-bit data ($R = \text{Planes 1–4}$). |
| **`11`** | **Modify Green** | Hold Red and Blue from preceding pixel; replace Green with 4-bit data ($G = \text{Planes 1–4}$). |

---

## 5. Hardware Collision Detection (`CLXDAT` & `CLXCON`)

Denise tracks physical pixel collisions in hardware during rendering:
- **`CLXDAT` (`$DFF00E`):** 15-bit read-only latch recording:
  - Sprite-to-Sprite collisions (any combination of sprite pairs overlapping non-transparent pixels).
  - Sprite-to-Playfield collisions (sprites overlapping active pixels of Playfield 1 or Playfield 2).
- **Clear on Read:** Reading `CLXDAT` automatically clears all latch bits to zero.
- **`CLXCON` (`$DFF098`):** Collision control mask defining which bitplanes participate in collision checks and their required match values.

---

## 6. Subordinate Engines & Peripherals

Denise encapsulates two specialized display engines:

### 6.1 Hardware Sprites Engine
- **8 DMA Sprites:** 16-pixel wide hardware sprites with arbitrary vertical height.
- **Attached Mode:** Pairs (0+1, 2+3, 4+5, 6+7) can combine into 15-color sprites from `COLOR16`–`COLOR31`.
- **Multiplexing:** Scanline-by-scanline reuse down the screen.
- *Authoritative Specification:* See [Sprites.md](Sprites.md).

### 6.2 Frame Builder & Video Signal Generation
- **Raster Compositor:** Assembles pixels, performs Display Window clipping (`DIWSTRT`, `DIWSTOP`), and generates a 32-bit ARGB frame buffer.
- **Resistor DAC Physics:** Models discrete R-2R ladder (linear voltage 0.0V to 0.7V) and absence of broadcast gamma pre-correction.
- **Studio Quantization:** $n \times 16$ scaling matching broadcast studio captures.
- *Authoritative Specification:* See [Frame Buffer.md](Frame%20Buffer.md).

### 6.3 Game Port Inputs
Denise houses the directional and quadrature counters for Game Port 1 and Game Port 2 (`JOY0DAT`, `JOY1DAT`, `POT0DAT`, `POT1DAT`, `POTGO`).
- *Detailed Specifications:* See [Mouse.md](Mouse.md) and [Joystick.md](Joystick.md).

---

## 7. Reset Defaults

- **`BPLCON0` (`$DFF100`):** Reset to **`$0000`** (bitplanes disabled, video generation off).
- **`COLOR00`–`COLOR31`:** Default to **`$0000`** (black).
- **`CLXDAT` (`$DFF00E`):** Reset to **`$0000`**.
- **Sprites:** Halted; position and image data registers reset to `$0000`.

---

## 8. Reference Documentation & Upstream Ground Truth

- [Sprites Architecture Specification](Sprites.md): 8 hardware sprite engines, attached pairs, and position comparators.
- [Frame Buffer Specification](Frame%20Buffer.md): 32-bit ARGB raster compositor and resistor DAC circuit physics.
- [Amiga Hardware Reference Manual: Chapter 3 (Playfield Hardware)](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md): Authoritative guide for dual-playfield scrolling, bitplane priority, HAM6, and EHB video modes.
- [Amiga Hardware Reference Manual: Chapter 4 (Sprite Hardware)](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md): Hardware specification for 8 DMA sprite channels and collision detection (`CLXDAT`).
- [vAmiga Denise Component Implementation](../../../ref_src/vAmiga-4.5/Core/Components/Denise/Denise.cpp): Reference C++ pixel pipeline, bitplane serializer, and palette DAC conversion.
