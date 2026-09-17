---
title: "Custom Chip Register Ownership and Access Matrix"
aliases: ["Custom Chip Register Matrix", "Custom Register Map", "Hardware Register Catalog", "Chipset Register Reference"]
tags: ["amiga", "design", "registers", "agnus", "denise", "paula", "memory-map"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-09-14
updated: 2026-09-14
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Custom Chip Register Ownership and Access Matrix

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Memory Subsystem:** [MemoryBus.md](MemoryBus.md)
- **Companion Signal Catalog:** [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
- **Hardware Quirks Index:** [Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)

---

## 1. Architectural Scope & The Split Identity Trap

The Amiga custom chipset registers occupy the 512-byte address space `$DFF000..$DFF1FE`, physically mirrored throughout `$DFF000..$DFFFFFF`.

> [!WARNING]
> **The Split Read/Write Identity Trap:**
> A single memory address offset often maps to **two completely distinct registers** depending on whether the processor performs a READ or a WRITE.
> Furthermore, the reading circuit and writing circuit often reside on **physically different silicon chips**!
>
> *Example 1:* Reading `$DFF002` reads `DMACONR` from Agnus (and Blitter live status), while writing `$DFF002` writes `DMACON` with SET/CLR bit-15 control logic.
> *Example 2:* Reading `$DFF006` reads `VHPOSR` from Agnus, while writing `$DFF006` writes `VHPOSW` (test register).
> *Example 3:* Reading `$DFF010` reads `ADKCONR` from Paula, while writing `$DFF010` is reserved/ignored (the write counterpart `ADKCON` is located at `$DFF09E`).

### Access Mode Classifications
- **`RO` (Read-Only):** Guest writes are ignored or non-functional.
- **`WO` (Write-Only):** Guest reads **never drive the data bus**. The floating open bus returns `$FFFF` (or lingering prefetch noise). Emulators must never synthesize return values for write-only registers.
- **`RW` (Read/Write):** Symmetrical read and write to the same register latch (rare in OCS/ECS custom chips).
- **`COR` (Clear-on-Read):** Reading the register returns data and atomically resets internal status bits or interrupt flip-flops (e.g. `CLXDAT`, `DSKBYTR`).
- **`STROBE`:** Accessing the address (read or write) triggers a circuit action (e.g. `COPJMP1`, `BLTSIZE`) without storing data.

---

## 2. Chip Ownership & Scope Rules

Each register belongs strictly to one physical silicon chip:
- **`A` = Agnus (MOS 8370 / 8371 / 8372):** Beam counters, Chip RAM address generation, DMA arbitration, Copper, Blitter.
- **`D` = Denise (MOS 8362 / 8373):** Video bitplane serialization, palette DACs, hardware sprites, collision detection.
- **`P` = Paula (MOS 8364):** Interrupt controller, audio DACs, floppy MFM controller, serial UART, analog pot counters.

**Scope:**
- **`Internal`:** Register updates and latches remain 100% encapsulated inside the owning chip (e.g. color registers, audio volume/period, beam counters).
- **`Cross-Chip`:** Register updates trigger actions that propagate across chip boundaries (see [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)).

---

## 3. Exhaustive Custom Chip Register Matrix ($000 to $1FE)

### System Control, Beam Counters & Status ($000..$03E)

| Offset | Read Name (Chip) | Write Name (Chip) | Access Mode | Scope | Delay | Function & Silicon Quirks |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$000** | `BLTDDAT` (A) | `BLTDDAT` (A) | `RO` / `WO` | Internal | 1 CCK | Blitter destination early data latch (internal test). |
| **$002** | `DMACONR` (A) | `DMACON` (A) | `RO` / `WO` | Cross-Chip | 2 CCKs | **Read:** DMA control status + Blitter `BBUSY` (bit 15) & `BZERO` (bit 14).<br/>**Write:** SET/CLR (bit 15) DMA enables across all channels. |
| **$004** | `VPOSR` (A) | `VPOSW` (A) | `RO` / `WO` | Internal | 2 CCKs | **Read:** Vertical beam position high bit, chip ID, and long frame (`LOF`). |
| **$006** | `VHPOSR` (A) | `VHPOSW` (A) | `RO` / `WO` | Internal | 2 CCKs | **Read:** Vertical ($V_7..V_0$) and Horizontal ($H_8..H_1$) beam position. |
| **$008** | *(Open bus $FFFF)* | `DSKDATR` (P) | `WO` | Internal | 1 CCK | Early disk data latch (test). |
| **$00A** | `JOY0DAT` (D) | *(Reserved)* | `RO` | Internal | Live | Port 1 (Mouse/Joystick) counter (X/Y mouse quadrature counts). |
| **$00C** | `JOY1DAT` (D) | *(Reserved)* | `RO` | Internal | Live | Port 2 (Joystick) counter. |
| **$00E** | `CLXDAT` (D) | *(Reserved)* | `COR` | Internal | Live | Sprite/Bitplane collision detect data. **Atomically cleared on read.** |
| **$010** | `ADKCONR` (P) | *(Reserved)* | `RO` | Cross-Chip | Live | Audio, disk, and UART control readback. (Write is at `$09E`). |
| **$012** | `POT0DAT` (P) | *(Reserved)* | `RO` | Internal | Live | Pot counter 0 (Port 1 analog pin 5/9 voltage charge timer). |
| **$014** | `POT1DAT` (P) | *(Reserved)* | `RO` | Internal | Live | Pot counter 1 (Port 2 analog pin 5/9 voltage charge timer). |
| **$016** | `POTGOR` (P) | *(Reserved)* | `RO` | Internal | Live | Pot port pin state readback and middle/right mouse button levels. |
| **$018** | `SERDATR` (P) | *(Reserved)* | `RO` | Internal | Live | Serial UART receive data, `TBE` (bit 13), `TSRE` (bit 12), and errors. |
| **$01A** | `DSKBYTR` (P) | *(Reserved)* | `COR` | Internal | Live | Disk byte data and sync status. Bit 15 (`DSKBYT`) cleared on read. |
| **$01C** | `INTENAR` (P) | *(Reserved)* | `RO` | Internal | Live | Interrupt enable register readback. (Write is at `$09A`). |
| **$01E** | `INTREQR` (P) | *(Reserved)* | `RO` | Internal | Live | Interrupt request register readback. (Write is at `$09C`). |
| **$020** | *(Open bus $FFFF)* | `DSKPTH` (A) | `WO` | Cross-Chip | 2 CCKs | Disk DMA pointer (high 3 bits, Chip RAM). |
| **$022** | *(Open bus $FFFF)* | `DSKPTL` (A) | `WO` | Cross-Chip | 2 CCKs | Disk DMA pointer (low 15 bits, Chip RAM). |
| **$024** | *(Open bus $FFFF)* | `DSKLEN` (P) | `WO` | Cross-Chip | 2 CCKs | Disk length in words. **Requires 2 consecutive writes with bit 15 to arm.** |
| **$026** | *(Open bus $FFFF)* | `DSKDAT` (P) | `WO` | Internal | 1 CCK | Floppy MFM DMA write data buffer. |
| **$028** | *(Open bus $FFFF)* | `REFPTR` (A) | `WO` | Internal | 2 CCKs | DRAM refresh pointer (Agnus internal test). |
| **$02A** | *(Open bus $FFFF)* | `VPTR` (A) | `WO` | Internal | 2 CCKs | Vertical beam position write (Agnus internal test). |
| **$02C** | *(Open bus $FFFF)* | `COPCON` (A) | `WO` | Cross-Chip | 2 CCKs | Copper control: bit 1 (`CDANG`) enables Copper writes to `$000..$07E`. |
| **$030** | *(Open bus $FFFF)* | `SERDAT` (P) | `WO` | Internal | 1 CCK | Serial UART transmit data buffer (pipelined). |
| **$032** | *(Open bus $FFFF)* | `SERPER` (P) | `WO` | Internal | 2 CCKs | Serial UART baud rate period clock divisor. |
| **$034** | *(Open bus $FFFF)* | `POTGO` (P) | `WO` | Internal | 2 CCKs | Potentiometer charge gate start and pin direction control. |
| **$036** | `JOYTEST` (D) | `JOYTEST` (D) | `WO` | Internal | Live | Write-only mouse counter quadrature injection register for testing. |

---

### Blitter Engine Registers ($040..$074)

All Blitter registers are physically owned by **Agnus**. Reads from these offsets return floating open bus (`$FFFF`).

| Offset | Register Name | Silicon Owner | Access Mode | Scope | Delay | Function & Silicon Quirks |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$040** | `BLTCON0` | Agnus | `WO` | Internal | 2 CCKs | Channel enables (A,B,C,D), minterm logic function, Channel A shift. |
| **$042** | `BLTCON1` | Agnus | `WO` | Internal | 2 CCKs | Channel B shift, line draw mode, descending flag, fill mode. |
| **$044** | `BLTAFWM` | Agnus | `WO` | Internal | 2 CCKs | Channel A First Word Mask. |
| **$046** | `BLTALWM` | Agnus | `WO` | Internal | 2 CCKs | Channel A Last Word Mask. |
| **$048** | `BLTCPTH` | Agnus | `WO` | Internal | 2 CCKs | Channel C Chip RAM source pointer (high 3 bits). |
| **$04A** | `BLTCPTL` | Agnus | `WO` | Internal | 2 CCKs | Channel C Chip RAM source pointer (low 15 bits). |
| **$04C** | `BLTBPTH` | Agnus | `WO` | Internal | 2 CCKs | Channel B Chip RAM source pointer (high 3 bits). |
| **$04E** | `BLTBPTL` | Agnus | `WO` | Internal | 2 CCKs | Channel B Chip RAM source pointer (low 15 bits). |
| **$050** | `BLTAPTH` | Agnus | `WO` | Internal | 2 CCKs | Channel A Chip RAM source pointer (high 3 bits). |
| **$052** | `BLTAPTL` | Agnus | `WO` | Internal | 2 CCKs | Channel A Chip RAM source pointer (low 15 bits). |
| **$054** | `BLTDPTH` | Agnus | `WO` | Internal | 2 CCKs | Channel D Chip RAM destination pointer (high 3 bits). |
| **$056** | `BLTDPTL` | Agnus | `WO` | Internal | 2 CCKs | Channel D Chip RAM destination pointer (low 15 bits). |
| **$058** | `BLTSIZE` | Agnus | `STROBE` | Cross-Chip | 1 CCK | **Blitter Start Strobe:** Latches height/width and initiates blit. |
| **$060** | `BLTCMOD` | Agnus | `WO` | Internal | 2 CCKs | Channel C signed 16-bit modulo. |
| **$062** | `BLTBMOD` | Agnus | `WO` | Internal | 2 CCKs | Channel B signed 16-bit modulo. |
| **$064** | `BLTAMOD` | Agnus | `WO` | Internal | 2 CCKs | Channel A signed 16-bit modulo. |
| **$066** | `BLTDMOD` | Agnus | `WO` | Internal | 2 CCKs | Channel D signed 16-bit modulo. |
| **$070** | `BLTCDAT` | Agnus | `WO` | Internal | 1 CCK | Channel C data holding latch (pipelined). |
| **$072** | `BLTBDAT` | Agnus | `WO` | Internal | 1 CCK | Channel B data holding latch (pipelined). |
| **$074** | `BLTADAT` | Agnus | `WO` | Internal | 1 CCK | Channel A data holding latch (pipelined). |
| **$07E** | `DSKSYNC` | Paula | `WO` | Cross-Chip | 2 CCKs | Disk MFM 16-bit sync match word (default `$4489`). |

---

### Copper, Framing & Broadcast Control ($080..$09E)

| Offset | Register Name | Silicon Owner | Access Mode | Scope | Delay | Function & Silicon Quirks |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$080** | `COP1LCH` | Agnus | `WO` | Cross-Chip | 2 CCKs | Copper List 1 Chip RAM pointer (high 3 bits). |
| **$082** | `COP1LCL` | Agnus | `WO` | Cross-Chip | 2 CCKs | Copper List 1 Chip RAM pointer (low 15 bits). |
| **$084** | `COP2LCH` | Agnus | `WO` | Cross-Chip | 2 CCKs | Copper List 2 Chip RAM pointer (high 3 bits). |
| **$086** | `COP2LCL` | Agnus | `WO` | Cross-Chip | 2 CCKs | Copper List 2 Chip RAM pointer (low 15 bits). |
| **$088** | `COPJMP1` | Agnus | `STROBE` | Cross-Chip | 1 CCK | **Copper Restart 1:** Reloads Copper PC from `COP1LC`. |
| **$08A** | `COPJMP2` | Agnus | `STROBE` | Cross-Chip | 1 CCK | **Copper Restart 2:** Reloads Copper PC from `COP2LC`. |
| **$08E** | `DIWSTRT` | Agnus / Denise | `WO` | Cross-Chip | 4 CCKs | Display window upper-left start coordinates ($V_{start}, H_{start}$). |
| **$090** | `DIWSTOP` | Agnus / Denise | `WO` | Cross-Chip | 4 CCKs | Display window lower-right stop coordinates ($V_{stop}, H_{stop}$). |
| **$092** | `DDFSTRT` | Agnus | `WO` | Internal | 4 CCKs | Bitplane DMA fetch horizontal start Color Clock. |
| **$094** | `DDFSTOP` | Agnus | `WO` | Internal | 4 CCKs | Bitplane DMA fetch horizontal stop Color Clock. |
| **$096** | `DMACON` | Agnus / Paula | `WO` | Cross-Chip | 2 CCKs | **Broadcast:** SET/CLR (bit 15) master DMA control. |
| **$098** | `CLXCON` | Denise | `WO` | Internal | 2 CCKs | Collision detection control and bitplane enable masks. |
| **$09A** | `INTENA` | Paula | `WO` | Internal | 1 CCK | SET/CLR (bit 15) interrupt enable mask. Drives CPU IPL. |
| **$09C** | `INTREQ` | Paula | `WO` | Internal | 1 CCK | SET/CLR (bit 15) interrupt request mask. Software can trigger IRQs. |
| **$09E** | `ADKCON` | Paula | `WO` | Internal | 2 CCKs | SET/CLR (bit 15) Audio, Disk, and UART modulation control. |

---

### Audio Subsystem ($0A0..$0DE) — Split Silicon Ownership

Notice the split ownership between Agnus and Paula across each audio channel:

| Offset | Channel | Register Name | Silicon Owner | Access Mode | Scope | Delay | Function |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$0A0** | Ch 0 | `AUD0LCH` | **Agnus** | `WO` | Internal (Agnus) | 2 CCKs | Starting Chip RAM address (high 3 bits). |
| **$0A2** | Ch 0 | `AUD0LCL` | **Agnus** | `WO` | Internal (Agnus) | 2 CCKs | Starting Chip RAM address (low 15 bits). |
| **$0A4** | Ch 0 | `AUD0LEN` | **Paula** | `WO` | Internal (Paula) | 2 CCKs | Length in 16-bit words. |
| **$0A6** | Ch 0 | `AUD0PER` | **Paula** | `WO` | Internal (Paula) | 2 CCKs | Color Clock period divider (playback rate). |
| **$0A8** | Ch 0 | `AUD0VOL` | **Paula** | `WO` | Internal (Paula) | 2 CCKs | 6-bit linear volume ($0..64$). |
| **$0AA** | Ch 0 | `AUD0DAT` | **Paula** | `WO` | Internal (Paula) | 1 CCK | 16-bit signed PCM sample holding buffer (pipelined). |
| **$0B0..$0BA** | Ch 1 | `AUD1...` | **A / P** | `WO` | Internal | 1..2 CCKs | Channel 1 registers (same layout as Ch 0). |
| **$0C0..$0CA** | Ch 2 | `AUD2...` | **A / P** | `WO` | Internal | 1..2 CCKs | Channel 2 registers (same layout as Ch 0). |
| **$0D0..$0DA** | Ch 3 | `AUD3...` | **A / P** | `WO` | Internal | 1..2 CCKs | Channel 3 registers (same layout as Ch 0). |

---

### Video Bitplanes & Display ($0E0..$11E)

| Offset | Register Name | Silicon Owner | Access Mode | Scope | Delay | Function |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$0E0..$0F6**| `BPL1PTH..6L`| Agnus | `WO` | Internal | 2 CCKs | Bitplane 1..6 Chip RAM pointers (high and low words). |
| **$100** | `BPLCON0` | Denise / Agnus | `WO` | Cross-Chip | 1 CCK (D) / 4 CCK (A) | Planecount ($0..6$), High-Res, Interlace, Dual Playfield. |
| **$102** | `BPLCON1` | Denise | `WO` | Internal | 1 CCK | Horizontal scroll delays for Playfield 1 and 2. |
| **$104** | `BPLCON2` | Denise | `WO` | Internal | 1 CCK | Video priority between playfields and sprites. |
| **$108** | `BPL1MOD` | Agnus | `WO` | Internal | 2 CCKs | Odd bitplanes (1, 3, 5) modulo. |
| **$10A** | `BPL2MOD` | Agnus | `WO` | Internal | 2 CCKs | Even bitplanes (2, 4, 6) modulo. |
| **$110..$11A**| `BPL1DAT..6DAT`| Denise | `WO` | Internal | 1 CCK | Bitplane serialization data buffers (pipelined). |

---

### Hardware Sprites ($120..$17E) & Color Palette ($180..$1BE)

| Offset Range | Register Name | Silicon Owner | Access Mode | Scope | Delay | Function |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **$120..$13E** | `SPR0PTH..7L` | Agnus | `WO` | Internal | 2 CCKs | Sprite 0..7 Chip RAM pointers. |
| **$140..$17E** | `SPR0POS..7CTL`| Denise | `WO` | Internal | 1 CCK | Sprite vertical/horizontal positions, arming, and image data. |
| **$180..$1BE** | `COLOR00..31` | Denise | `WO` | Internal | 1 CCK | 32-entry RGB444 color palette registers (pipelined). |

---

## 4. Summary Rules for Implementors

1. **Reads from `WO` offsets return `$FFFF`:** Never synthesize synthetic status or zero returns on write-only custom registers.
2. **Registers with split identities have separate paths:** `read_custom_word` and `write_custom_word` must treat `$002`, `$004`, `$006`, `$010`, etc. as completely independent hardware entities.
3. **Internal registers stay encapsulated:** Only the registers labeled `Cross-Chip` ever require notifications outside their parent chip.

---

## 5. Reference Documentation & Upstream Ground Truth

- **Commodore Amiga Hardware Reference Manual**:
  - [Hardware Reference Manual](../Reference/Hardware%20Reference%20Manual): Exhaustive register address offsets, bit definitions, strobe behaviors, and read/write characteristics across Agnus, Denise, and Paula.
  - [Appendix A - Register Summary (Alphabetical)](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Canonical register names, address offsets, and read/write characteristics.
  - [Appendix B - Register Summary (Address Order)](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Numerical memory map order from `$000` to `$1FE`.
- **Undocumented Chipset Features**:
  - [Undocumented features of OCS, ECS and AGA chipsets.md](../Reference/Undocumented%20features%20of%20OCS,%20ECS%20and%20AGA%20chipsets.md): Definitive compendium of undocumented silicon behavior, floating open bus returns ($FFFF), and split read/write identities.
- **Silicon Reference Emulators**:
  - [`vAmiga Reference Source`](../../../ref_src/vAmiga): Clean-room implementation of custom chip register address mapping and strobe dispatch.
  - [`vAmigaTS Test Suite`](../../../ref_src/vAmigaTS): Verification test harness for custom register write side-effects and open-bus floating behavior.
- **Living Crate Source Files**:
  - [`crates/memory_bus/src/memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Motherboard address router dispatching custom register reads and writes live to subsystem handles.
  - [`crates/agnus/src/agnus.rs`](../../../crates/agnus/src/agnus.rs): Agnus beam counters, DMA control, and Copper/Blitter register handling.
  - [`crates/denise/src/denise.rs`](../../../crates/denise/src/denise.rs): Denise bitplane control, palette registers, and collision latches.
  - [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Paula interrupt enable/request masks, audio channel registers, and floppy/serial interfaces.

