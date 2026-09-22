---
title: "Denise Hardware Sprites Architecture & Specification"
aliases: ["Sprites", "Hardware Sprites", "SPRxPOS", "SPRxCTL", "Sprite Engine"]
tags: ["amiga", "design", "sprites", "denise", "video"]
category: "Design"
subsystem: "sprites"
status: "active"
created: 2026-09-19
updated: 2026-09-19
related: ["[Denise.md](Denise.md)", "[Frame Buffer.md](Frame%20Buffer.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Main loop A500.md](Main%20loop%20A500.md)"]
tracked_paths:
  - "crates/sprites"
last_synced_commit: "0fcd519"
last_synced_date: "2026-09-19"
---
# Denise Hardware Sprites Architecture & Specification

> [!NOTE]
> System execution constraints, memory bus arbitration, and Color Clock timing are defined in [AGENTS.md](../../../AGENTS.md), [MemoryBus.md](MemoryBus.md), and [Main loop A500.md](Main%20loop%20A500.md).
> Detailed inter-chip signal rules are codified in [`hardware-bus-topology.md`](../../../.agents/rules/hardware-bus-topology.md).
> Video frame compositing and pixel DAC output are documented in [Frame Buffer.md](Frame%20Buffer.md). Denise display modes and collision detection are governed by [Denise.md](Denise.md), and DMA slot allocation by [DMA.md](DMA.md).

---

## 1. Core Decisions & Architectural Principles

Denise contains 8 independent hardware sprite engines (`SPR0` through `SPR7`). Each sprite is 16 pixels wide and can span an arbitrary vertical height anywhere on the screen.

```mermaid
flowchart TD
    DMA["Agnus Sprite DMA (Slots 21..51 Odd)\nLatches into SPRxDATA & SPRxDATB"] --> ARMED{"Armed & Line Active?\n(vpos in [VSTART..VSTOP))"}
    ARMED -->|hpos_pixel == HSTART| LOAD["Load Parallel Words into\n16-Bit Shift Registers (shift_a, shift_b)"]
    LOAD --> SHIFT["Shift 1 Pixel per CCK (2 Bits)\nBit 0 from A, Bit 1 from B"]

    SHIFT --> PAIR{"Attached Sprite?\n(Odd SPRxCTL bit 7)"}
    PAIR -->|Independent (3 Colors)| PAL3["4-Color Palette Group\n(COLOR16..19, 20..23, 24..27, 28..31)"]
    PAIR -->|Attached (15 Colors)| PAL15["16-Color Palette\n(COLOR16..COLOR31)"]

    PAL3 --> MIXER["Priority Mixer & Compositor\n(Arbitrated against Playfields via BPLCON2)"]
    PAL15 --> MIXER
```

### 1.1 Core Hardware Invariants
1. **Passive Data Latching:** Sprites contain **zero DMA address generation circuitry**. Agnus owns `SPRxPT` pointers and drives Chip RAM addresses during odd cycles 21 to 51. Denise passively latches the incoming 16-bit words into `SPRxPOS`, `SPRxCTL`, `SPRxDATA`, and `SPRxDATB`.
2. **Arbitrary Vertical Height:** Sprites have no fixed height; vertical windowing is determined by comparing the running raster scanline against `VSTART` and `VSTOP`.
3. **Attached Sprite Pairs:** Odd sprites (1, 3, 5, 7) can attach to their preceding even partners (0, 2, 4, 6) to create 4-bitplane, 15-color sprites (with color 0 transparent).
4. **Vertical Multiplexing:** Because position comparators are re-evaluated scanline-by-scanline, a single sprite DMA channel can be displayed multiple times vertically down the screen by updating `SPRxPOS` and `SPRxCTL` via the Copper.

---

## 2. Module Architecture & Crate Containment (`crates/sprites`)

The sprite engines are implemented in `crates/sprites`:

```
crates/sprites/
├── Cargo.toml
├── src/
│   └── sprites.rs         // 8 sprite channels, comparators, serializers, priority mixer
└── tests/
    └── test_sprites.rs    // Positioning, clipping, attached mode, multiplexing tests
```

### 2.1 Channel State (`SpriteChannel`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpriteChannel {
    pub pt: u32,
    pub pos: u16,
    pub ctl: u16,
    pub data_a: u16,
    pub data_b: u16,
    pub shift_a: u16,
    pub shift_b: u16,
    pub pixel_counter: u8,
    pub is_armed: bool,
    pub is_active_line: bool,
}
```

---

## 3. Coordinate Decoding & Position Comparators

Sprite coordinates use 9-bit vertical and horizontal precision:

### 3.1 Vertical Start (`VSTART`) & Stop (`VSTOP`)
- **`VSTART` (9 bits):** Bits 15–8 of `SPRxPOS` provide the low 8 bits ($V_7..V_0$); bit 2 of `SPRxCTL` provides the 9th bit ($V_8$).
- **`VSTOP` (9 bits):** Bits 15–8 of `SPRxCTL` provide the low 8 bits ($V_7..V_0$); bit 1 of `SPRxCTL` provides the 9th bit ($V_8$).
- **Active Line Evaluation:** At the start of each scanline:
  $$\text{is\_active\_line} = (vpos \ge V_{\text{start}}) \land (vpos < V_{\text{stop}})$$

### 3.2 Horizontal Start (`HSTART`)
- **`HSTART` (9 bits, low-resolution pixel coordinate):** Bits 7–0 of `SPRxPOS` provide the high 8 bits ($H_8..H_1$); bit 0 of `SPRxCTL` provides the lowest sub-pixel bit ($H_0$).
- **Comparator Trigger:** When the horizontal beam reaches `HSTART` on an active line, the 16-bit shift registers are loaded:
  $$\text{shift\_a} = \text{data\_a}, \quad \text{shift\_b} = \text{data\_b}, \quad \text{pixel\_counter} = 16$$

---

## 4. Color Palettes & Attached Sprites

Each sprite pixel is shifted out at 1 pixel per CCK in low resolution (or 2 pixels per CCK in high resolution):

### 4.1 Independent Sprites (2 Bitplanes, 3 Colors + Transparent)
Sprites are grouped into four 2-channel pairs sharing 4-color palettes:
- **Sprites 0 & 1:** Colors from `COLOR16` through `COLOR19` (00 = transparent, 01 = color 17, 10 = color 18, 11 = color 19).
- **Sprites 2 & 3:** Colors from `COLOR20` through `COLOR23`.
- **Sprites 4 & 5:** Colors from `COLOR24` through `COLOR27`.
- **Sprites 6 & 7:** Colors from `COLOR28` through `COLOR31`.

### 4.2 Attached Sprites (`ATTACH` Mode)
- Enabled by setting bit 7 of `SPRxCTL` in the **odd** sprite channel (`SPR1CTL`, `SPR3CTL`, `SPR5CTL`, `SPR7CTL`).
- Merges the two 2-bitplane shift registers into a single 4-bitplane pixel:
  $$\text{color\_index} = (\text{odd\_pixel} \ll 2) \lor \text{even\_pixel}$$
- Displays 15 colors plus transparency selected from `COLOR16` through `COLOR31`.

---

## 5. Register Memory Map

All sprite registers reside within the custom chip address space ($DFF140–$DFF17E):

| Address Range | R/W | Symbol | Description |
| :--- | :---: | :--- | :--- |
| **`$DFF140`..`$DFF178`** | W | **`SPRxPOS`** | Sprite 0–7 Position (VSTART in 15..8, HSTART in 7..0) |
| **`$DFF142`..`$DFF17A`** | W | **`SPRxCTL`** | Sprite 0–7 Control (VSTOP in 15..8, ATTACH in bit 7, V8/H0 in 2..0) |
| **`$DFF144`..`$DFF17C`** | W | **`SPRxDATA`** | Sprite 0–7 Low Image Data holding latch |
| **`$DFF146`..`$DFF17E`** | W | **`SPRxDATB`** | Sprite 0–7 High Image Data holding latch |

---

## 6. Playfield Priority Arbitration (`BPLCON2`)

Bits 5–0 of `BPLCON2` govern the relative depth priority between sprites and bitplane playfields:
- Sprites can appear entirely in front of all playfields, sandwiched between Playfield 1 and Playfield 2 (in Dual Playfield mode), or tucked entirely behind background graphics.
- Lower-numbered sprites (Sprite 0) always take priority over higher-numbered sprites (Sprite 7) when pixels overlap.

---

## 7. Reference Documentation & Upstream Ground Truth

- [Amiga Hardware Reference Manual: Chapter 4 (Sprite Hardware)](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md): Authoritative specification for 8 DMA sprite channels, attached pairing, and coordinate registers.
- [Denise Architecture Specification](Denise.md): Video display controller, bitplane serializer, and collision detection (`CLXDAT`).
- [Frame Buffer Specification](Frame%20Buffer.md): 32-bit ARGB raster pixel compositing and video DAC output.
- [DMA Architecture & Scheduling](DMA.md): Odd-cycle sprite DMA slot allocation (slots 21..51).
