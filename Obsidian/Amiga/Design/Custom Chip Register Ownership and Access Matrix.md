---
title: "Custom Chip Register Ownership and Access Matrix"
aliases: ["Custom Chip Register Matrix", "Custom Register Map", "Hardware Register Catalog", "Chipset Register Reference"]
tags: ["amiga", "design", "registers", "agnus", "denise", "paula", "memory-map"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-09-14
updated: 2026-10-13
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Custom Chip Register Ownership and Access Matrix

This is a book-derived address map of the original custom-chip register block. The 68000 address is `$DFF000` plus the three-digit offset below. [Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md) supplies names, directions, chip listings, and short functions; [Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md) supplies the access cautions and the two explicit exceptions noted below. Explicit design-model notes guide future implementation; the matrix makes no claim about the emulator's current implementation.

## Reading the matrix

- `RO` and `WO` reproduce the manual's documented read or write direction. `—` means no **documented safe counterpart** at that offset. It does not mean that an opposite-direction bus cycle has no effect. Appendix A says a read of a write-only register can disturb it and a write to a read-only register can have unexpected results.
- `ER` is an early DMA read initiated by Agnus timing, not an ordinary CPU register read. `COR` marks `CLXDAT`, whose documented read also clears collision state. `STROBE` marks an address-triggered action rather than stored register bits. Both `COPJMP1` and `COPJMP2` can trigger a Copper jump on a CPU read or write. The four sync strobes (`STREQU`, `STRVBL`, `STRHOR`, `STRLONG`) are documented as write-address strobes used by Agnus during refresh; their CPU reads have no documented safe strobe semantics. The read/write distinction does not make an opposite-direction access harmless.
- A name suffix `&` means DMA-only use; `%` means usually DMA, sometimes processor; `+` marks a chip-memory pointer pair. A prefix `*` on the Appendix B address prohibits Copper writes. A prefix `-` permits Copper writes only with `COPCON.CDANG` set. These markers appear in the notes rather than the offset so the offset stays numeric.
- `Scope` records only what the book supports: `Multiple chips listed` where Appendix B names more than one chip. A single listed chip does not establish that every downstream effect stays inside that chip, so other rows say `Not established`. The appendices do not provide a general per-register CCK propagation delay, so `Delay` is `Unknown`; it must not be interpreted as zero.
- Chip listings reproduce the manual; they are not a register-write dispatch table. Use an explicit design-model note in a row to decide whether a listed chip needs a modeled write action.
- `Reserved` reproduces an explicit Appendix B label. `Unlisted` applies only to `$1C0–$1DA`, which the address-order table skips without labeling. Neither classification establishes open-bus values or other silicon behavior. Appendix A advises against accessing unused registers.

## Address map

| Offset | Read Name (Chip) | Write Name (Chip) | Access Mode | Scope | Delay | Function & Book Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `$000` | `BLTDDAT` & (Agnus) | — | ER | Not established | Unknown | Blitter destination early read (dummy address). Copper write prohibited. |
| `$002` | `DMACONR` (Agnus, Paula) | — | RO | Multiple chips listed | Unknown | DMA control (and blitter status) read. Copper write prohibited. |
| `$004` | `VPOSR` (Agnus) | — | RO | Not established | Unknown | Read vertical most significant bit (and frame flop). Copper write prohibited. |
| `$006` | `VHPOSR` (Agnus) | — | RO | Not established | Unknown | Read vertical and horizontal position of beam. Copper write prohibited. |
| `$008` | `DSKDATR` & (Paula) | — | ER | Not established | Unknown | Disk data early read (dummy address). Copper write prohibited. |
| `$00A` | `JOY0DAT` (Denise) | — | RO | Not established | Unknown | Joystick-mouse 0 data (vertical, horizontal). Copper write prohibited. |
| `$00C` | `JOY1DAT` (Denise) | — | RO | Not established | Unknown | Joystick-mouse 1 data (vertical, horizontal). Copper write prohibited. |
| `$00E` | `CLXDAT` (Denise) | — | COR | Not established | Unknown | Collision data register (read and clear). Copper write prohibited. |
| `$010` | `ADKCONR` (Paula) | — | RO | Not established | Unknown | Audio, disk control register read. Copper write prohibited. |
| `$012` | `POT0DAT` (Paula) | — | RO | Not established | Unknown | Pot counter pair 0 data (vertical, horizontal). Copper write prohibited. |
| `$014` | `POT1DAT` (Paula) | — | RO | Not established | Unknown | Pot counter pair 1 data (vertical, horizontal). Copper write prohibited. |
| `$016` | `POTGOR` (Paula) | — | RO | Not established | Unknown | Pot port data read (formerly `POTINP`). Copper write prohibited. |
| `$018` | `SERDATR` (Paula) | — | RO | Not established | Unknown | Serial port data and status read. Copper write prohibited. |
| `$01A` | `DSKBYTR` (Paula) | — | RO | Not established | Unknown | Disk data byte and status read. Copper write prohibited. |
| `$01C` | `INTENAR` (Paula) | — | RO | Not established | Unknown | Interrupt enable bits read. Copper write prohibited. |
| `$01E` | `INTREQR` (Paula) | — | RO | Not established | Unknown | Interrupt request bits read. Copper write prohibited. |
| `$020` | — | `DSKPTH` + (Agnus) | WO | Not established | Unknown | Disk pointer (high 3 bits). Copper write prohibited. |
| `$022` | — | `DSKPTL` + (Agnus) | WO | Not established | Unknown | Disk pointer (low 15 bits). Copper write prohibited. |
| `$024` | — | `DSKLEN` (Paula) | WO | Not established | Unknown | Disk length. Copper write prohibited. |
| `$026` | — | `DSKDAT` & (Paula) | WO | Not established | Unknown | Disk DMA data write. Copper write prohibited. |
| `$028` | — | `REFPTR` & (Agnus) | WO | Not established | Unknown | Refresh pointer. Copper write prohibited. |
| `$02A` | — | `VPOSW` (Agnus) | WO | Not established | Unknown | Write vertical most significant bit (and frame flop). Copper write prohibited. |
| `$02C` | — | `VHPOSW` (Agnus) | WO | Not established | Unknown | Write vertical and horizontal position of beam. Copper write prohibited. |
| `$02E` | — | `COPCON` (Agnus) | WO | Not established | Unknown | Coprocessor control register (CDANG). Copper write prohibited. |
| `$030` | — | `SERDAT` (Paula) | WO | Not established | Unknown | Serial port data and stop bits write. Copper write prohibited. |
| `$032` | — | `SERPER` (Paula) | WO | Not established | Unknown | Serial port period and control. Copper write prohibited. |
| `$034` | — | `POTGO` (Paula) | WO | Not established | Unknown | Pot port data write and start. Copper write prohibited. |
| `$036` | — | `JOYTEST` (Denise) | WO | Not established | Unknown | Write to all 4 joystick-mouse counters at once. Copper write prohibited. |
| `$038` | — | `STREQU` & (Denise) | STROBE | Not established | Unknown | Write-address strobe for horizontal sync with vertical blank and equalization; Agnus emits it during refresh. Copper write prohibited. |
| `$03A` | — | `STRVBL` & (Denise) | STROBE | Not established | Unknown | Write-address strobe for horizontal sync with vertical blank; Agnus emits it during refresh. Copper write prohibited. |
| `$03C` | — | `STRHOR` & (Denise, Paula) | STROBE | Multiple chips listed | Unknown | Write-address strobe for horizontal sync; Agnus emits it during refresh. Copper write prohibited. |
| `$03E` | — | `STRLONG` & (Denise) | STROBE | Not established | Unknown | Write-address strobe identifying a long horizontal line; Agnus emits it during refresh. Copper write prohibited. |
| `$040` | — | `BLTCON0` (Agnus) | WO | Not established | Unknown | Blitter control register 0. Copper write requires `COPCON.CDANG`. |
| `$042` | — | `BLTCON1` (Agnus) | WO | Not established | Unknown | Blitter control register 1. Copper write requires `COPCON.CDANG`. |
| `$044` | — | `BLTAFWM` (Agnus) | WO | Not established | Unknown | Blitter first word mask for source A. Copper write requires `COPCON.CDANG`. |
| `$046` | — | `BLTALWM` (Agnus) | WO | Not established | Unknown | Blitter last word mask for source A. Copper write requires `COPCON.CDANG`. |
| `$048` | — | `BLTCPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source C (high 3 bits). Copper write requires `COPCON.CDANG`. |
| `$04A` | — | `BLTCPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source C (low 15 bits). Copper write requires `COPCON.CDANG`. |
| `$04C` | — | `BLTBPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source B (high 3 bits). Copper write requires `COPCON.CDANG`. |
| `$04E` | — | `BLTBPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source B (low 15 bits). Copper write requires `COPCON.CDANG`. |
| `$050` | — | `BLTAPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source A (high 3 bits). Copper write requires `COPCON.CDANG`. |
| `$052` | — | `BLTAPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source A (low 15 bits). Copper write requires `COPCON.CDANG`. |
| `$054` | — | `BLTDPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to destination D (high 3 bits). Copper write requires `COPCON.CDANG`. |
| `$056` | — | `BLTDPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to destination D (low 15 bits). Copper write requires `COPCON.CDANG`. |
| `$058` | — | `BLTSIZE` (Agnus) | WO | Not established | Unknown | Blitter start and size (window width, height). Copper write requires `COPCON.CDANG`. |
| `$05A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$05C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$05E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$060` | — | `BLTCMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source C. Copper write requires `COPCON.CDANG`. |
| `$062` | — | `BLTBMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source B. Copper write requires `COPCON.CDANG`. |
| `$064` | — | `BLTAMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source A. Copper write requires `COPCON.CDANG`. |
| `$066` | — | `BLTDMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for destination D. Copper write requires `COPCON.CDANG`. |
| `$068` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$06A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$06C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$06E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$070` | — | `BLTCDAT` % (Agnus) | WO | Not established | Unknown | Blitter source C data register. Copper write requires `COPCON.CDANG`. |
| `$072` | — | `BLTBDAT` % (Agnus) | WO | Not established | Unknown | Blitter source B data register. Copper write requires `COPCON.CDANG`. |
| `$074` | — | `BLTADAT` % (Agnus) | WO | Not established | Unknown | Blitter source A data register. Copper write requires `COPCON.CDANG`. |
| `$076` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$078` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$07A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$07C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$07E` | — | `DSKSYNC` (Paula) | WO | Not established | Unknown | Disk sync pattern register for disk read. Copper write requires `COPCON.CDANG`. |
| `$080` | — | `COP1LCH` + (Agnus) | WO | Not established | Unknown | Coprocessor first location register (high 3 bits). |
| `$082` | — | `COP1LCL` + (Agnus) | WO | Not established | Unknown | Coprocessor first location register (low 15 bits). |
| `$084` | — | `COP2LCH` + (Agnus) | WO | Not established | Unknown | Coprocessor second location register (high 3 bits). |
| `$086` | — | `COP2LCL` + (Agnus) | WO | Not established | Unknown | Coprocessor second location register (low 15 bits). |
| `$088` | `COPJMP1` (Agnus) | `COPJMP1` (Agnus) | STROBE | Not established | Unknown | CPU read or write restarts the Copper at the first location; the manual demonstrates a read. Avoid read-modify-write. |
| `$08A` | `COPJMP2` (Agnus) | `COPJMP2` (Agnus) | STROBE | Not established | Unknown | CPU read or write restarts the Copper at the second location; Appendix A explicitly identifies both triggers. Avoid read-modify-write. |
| `$08C` | — | `COPINS` (Agnus) | WO | Not established | Unknown | Coprocessor instruction fetch identify. |
| `$08E` | — | `DIWSTRT` (Agnus) | WO | Not established | Unknown | Display window start (upper left vertical-horizontal position). Design model: Agnus uses `VSTART` to begin bitplane DMA on whole lines; Denise uses `HSTART` to begin displaying pixels within a line, at low-resolution pixel granularity. The [Denise IC specification](https://www.amigawiki.org/lib/exe/fetch.php?media=de%3Aparts%3Adenise_specs.pdf) assigns the horizontal bits to Denise although Appendix B lists only Agnus. |
| `$090` | — | `DIWSTOP` (Agnus) | WO | Not established | Unknown | Display window stop (lower right vertical and horizontal position). Design model: Agnus uses `VSTOP` to end bitplane DMA on whole lines; Denise uses `HSTOP` to end visible pixels within a line, at low-resolution pixel granularity. |
| `$092` | — | `DDFSTRT` (Agnus) | WO | Not established | Unknown | Display bitplane data fetch start (horizontal position). Agnus starts fetching 16-bit bitplane words here; bitplane pointers select their Chip RAM addresses. Fetching begins before the pixels can appear at the `DIWSTRT` edge. |
| `$094` | — | `DDFSTOP` (Agnus) | WO | Not established | Unknown | Display bitplane data fetch stop (horizontal position). Agnus marks the final bitplane fetch block of each line here; transfers within that block can continue beyond this position. Together with `DDFSTRT`, this interval must supply the data needed for the visible playfield and any scrolling margin; its boundaries need not match the pixel-level display window. |
| `$096` | — | `DMACON` (Agnus, Denise, Paula) | WO | Multiple chips listed | Unknown | DMA control write (clear or set). Design model: Agnus latches the DMA enables; Denise and Paula have no `DMACON` write action or handler. Do not dispatch this write to either chip solely because the manual lists them; see [DMA](DMA.md). |
| `$098` | — | `CLXCON` (Denise) | WO | Not established | Unknown | Collision control. |
| `$09A` | — | `INTENA` (Paula) | WO | Not established | Unknown | Interrupt enable bits (clear or set bits). |
| `$09C` | — | `INTREQ` (Paula) | WO | Not established | Unknown | Interrupt request bits (clear or set bits). |
| `$09E` | — | `ADKCON` (Paula) | WO | Not established | Unknown | Audio, disk, UART control. |
| `$0A0` | — | `AUD0LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 0 location (high 3 bits). |
| `$0A2` | — | `AUD0LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 0 location (low 15 bits). |
| `$0A4` | — | `AUD0LEN` (Paula) | WO | Not established | Unknown | Audio channel 0 length. |
| `$0A6` | — | `AUD0PER` (Paula) | WO | Not established | Unknown | Audio channel 0 period. |
| `$0A8` | — | `AUD0VOL` (Paula) | WO | Not established | Unknown | Audio channel 0 volume. |
| `$0AA` | — | `AUD0DAT` & (Paula) | WO | Not established | Unknown | Audio channel 0 data. |
| `$0AC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0AE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0B0` | — | `AUD1LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 1 location (high 3 bits). |
| `$0B2` | — | `AUD1LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 1 location (low 15 bits). |
| `$0B4` | — | `AUD1LEN` (Paula) | WO | Not established | Unknown | Audio channel 1 length. |
| `$0B6` | — | `AUD1PER` (Paula) | WO | Not established | Unknown | Audio channel 1 period. |
| `$0B8` | — | `AUD1VOL` (Paula) | WO | Not established | Unknown | Audio channel 1 volume. |
| `$0BA` | — | `AUD1DAT` & (Paula) | WO | Not established | Unknown | Audio channel 1 data. |
| `$0BC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0BE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0C0` | — | `AUD2LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 2 location (high 3 bits). |
| `$0C2` | — | `AUD2LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 2 location (low 15 bits). |
| `$0C4` | — | `AUD2LEN` (Paula) | WO | Not established | Unknown | Audio channel 2 length. |
| `$0C6` | — | `AUD2PER` (Paula) | WO | Not established | Unknown | Audio channel 2 period. |
| `$0C8` | — | `AUD2VOL` (Paula) | WO | Not established | Unknown | Audio channel 2 volume. |
| `$0CA` | — | `AUD2DAT` & (Paula) | WO | Not established | Unknown | Audio channel 2 data. |
| `$0CC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0CE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0D0` | — | `AUD3LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 3 location (high 3 bits). |
| `$0D2` | — | `AUD3LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 3 location (low 15 bits). |
| `$0D4` | — | `AUD3LEN` (Paula) | WO | Not established | Unknown | Audio channel 3 length. |
| `$0D6` | — | `AUD3PER` (Paula) | WO | Not established | Unknown | Audio channel 3 period. |
| `$0D8` | — | `AUD3VOL` (Paula) | WO | Not established | Unknown | Audio channel 3 volume. |
| `$0DA` | — | `AUD3DAT` & (Paula) | WO | Not established | Unknown | Audio channel 3 data. |
| `$0DC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0DE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0E0` | — | `BPL1PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 1 pointer (high 3 bits). |
| `$0E2` | — | `BPL1PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 1 pointer (low 15 bits). |
| `$0E4` | — | `BPL2PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 2 pointer (high 3 bits). |
| `$0E6` | — | `BPL2PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 2 pointer (low 15 bits). |
| `$0E8` | — | `BPL3PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 3 pointer (high 3 bits). |
| `$0EA` | — | `BPL3PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 3 pointer (low 15 bits). |
| `$0EC` | — | `BPL4PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 4 pointer (high 3 bits). |
| `$0EE` | — | `BPL4PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 4 pointer (low 15 bits). |
| `$0F0` | — | `BPL5PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 5 pointer (high 3 bits). |
| `$0F2` | — | `BPL5PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 5 pointer (low 15 bits). |
| `$0F4` | — | `BPL6PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 6 pointer (high 3 bits). |
| `$0F6` | — | `BPL6PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 6 pointer (low 15 bits). |
| `$0F8` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0FA` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0FC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$0FE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$100` | — | `BPLCON0` (Agnus, Denise) | WO | Multiple chips listed | Unknown | Bitplane control register (misc. control bits). |
| `$102` | — | `BPLCON1` (Denise) | WO | Not established | Unknown | Bitplane control reg. (scroll value PF1, PF2). |
| `$104` | — | `BPLCON2` (Denise) | WO | Not established | Unknown | Bitplane control reg. (priority control). |
| `$106` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$108` | — | `BPL1MOD` (Agnus) | WO | Not established | Unknown | Bitplane modulo (odd planes). |
| `$10A` | — | `BPL2MOD` (Agnus) | WO | Not established | Unknown | Bitplane modulo (even planes). |
| `$10C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$10E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$110` | — | `BPL1DAT` & (Denise) | WO | Not established | Unknown | Bitplane 1 data (parallel-to-serial convert). |
| `$112` | — | `BPL2DAT` & (Denise) | WO | Not established | Unknown | Bitplane 2 data (parallel-to-serial convert). |
| `$114` | — | `BPL3DAT` & (Denise) | WO | Not established | Unknown | Bitplane 3 data (parallel-to-serial convert). |
| `$116` | — | `BPL4DAT` & (Denise) | WO | Not established | Unknown | Bitplane 4 data (parallel-to-serial convert). |
| `$118` | — | `BPL5DAT` & (Denise) | WO | Not established | Unknown | Bitplane 5 data (parallel-to-serial convert). |
| `$11A` | — | `BPL6DAT` & (Denise) | WO | Not established | Unknown | Bitplane 6 data (parallel-to-serial convert). |
| `$11C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$11E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. |
| `$120` | — | `SPR0PTH` + (Agnus) | WO | Not established | Unknown | Sprite 0 pointer (high 3 bits). |
| `$122` | — | `SPR0PTL` + (Agnus) | WO | Not established | Unknown | Sprite 0 pointer (low 15 bits). |
| `$124` | — | `SPR1PTH` + (Agnus) | WO | Not established | Unknown | Sprite 1 pointer (high 3 bits). |
| `$126` | — | `SPR1PTL` + (Agnus) | WO | Not established | Unknown | Sprite 1 pointer (low 15 bits). |
| `$128` | — | `SPR2PTH` + (Agnus) | WO | Not established | Unknown | Sprite 2 pointer (high 3 bits). |
| `$12A` | — | `SPR2PTL` + (Agnus) | WO | Not established | Unknown | Sprite 2 pointer (low 15 bits). |
| `$12C` | — | `SPR3PTH` + (Agnus) | WO | Not established | Unknown | Sprite 3 pointer (high 3 bits). |
| `$12E` | — | `SPR3PTL` + (Agnus) | WO | Not established | Unknown | Sprite 3 pointer (low 15 bits). |
| `$130` | — | `SPR4PTH` + (Agnus) | WO | Not established | Unknown | Sprite 4 pointer (high 3 bits). |
| `$132` | — | `SPR4PTL` + (Agnus) | WO | Not established | Unknown | Sprite 4 pointer (low 15 bits). |
| `$134` | — | `SPR5PTH` + (Agnus) | WO | Not established | Unknown | Sprite 5 pointer (high 3 bits). |
| `$136` | — | `SPR5PTL` + (Agnus) | WO | Not established | Unknown | Sprite 5 pointer (low 15 bits). |
| `$138` | — | `SPR6PTH` + (Agnus) | WO | Not established | Unknown | Sprite 6 pointer (high 3 bits). |
| `$13A` | — | `SPR6PTL` + (Agnus) | WO | Not established | Unknown | Sprite 6 pointer (low 15 bits). |
| `$13C` | — | `SPR7PTH` + (Agnus) | WO | Not established | Unknown | Sprite 7 pointer (high 3 bits). |
| `$13E` | — | `SPR7PTL` + (Agnus) | WO | Not established | Unknown | Sprite 7 pointer (low 15 bits). |
| `$140` | — | `SPR0POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 0 vertical-horizontal start position data. |
| `$142` | — | `SPR0CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 0 vertical stop position and control data. |
| `$144` | — | `SPR0DATA` % (Denise) | WO | Not established | Unknown | Sprite 0 image data register A. |
| `$146` | — | `SPR0DATB` % (Denise) | WO | Not established | Unknown | Sprite 0 image data register B. |
| `$148` | — | `SPR1POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 1 vertical-horizontal start position data. |
| `$14A` | — | `SPR1CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 1 vertical stop position and control data. |
| `$14C` | — | `SPR1DATA` % (Denise) | WO | Not established | Unknown | Sprite 1 image data register A. |
| `$14E` | — | `SPR1DATB` % (Denise) | WO | Not established | Unknown | Sprite 1 image data register B. |
| `$150` | — | `SPR2POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 2 vertical-horizontal start position data. |
| `$152` | — | `SPR2CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 2 vertical stop position and control data. |
| `$154` | — | `SPR2DATA` % (Denise) | WO | Not established | Unknown | Sprite 2 image data register A. |
| `$156` | — | `SPR2DATB` % (Denise) | WO | Not established | Unknown | Sprite 2 image data register B. |
| `$158` | — | `SPR3POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 3 vertical-horizontal start position data. |
| `$15A` | — | `SPR3CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 3 vertical stop position and control data. |
| `$15C` | — | `SPR3DATA` % (Denise) | WO | Not established | Unknown | Sprite 3 image data register A. |
| `$15E` | — | `SPR3DATB` % (Denise) | WO | Not established | Unknown | Sprite 3 image data register B. |
| `$160` | — | `SPR4POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 4 vertical-horizontal start position data. |
| `$162` | — | `SPR4CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 4 vertical stop position and control data. |
| `$164` | — | `SPR4DATA` % (Denise) | WO | Not established | Unknown | Sprite 4 image data register A. |
| `$166` | — | `SPR4DATB` % (Denise) | WO | Not established | Unknown | Sprite 4 image data register B. |
| `$168` | — | `SPR5POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 5 vertical-horizontal start position data. |
| `$16A` | — | `SPR5CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 5 vertical stop position and control data. |
| `$16C` | — | `SPR5DATA` % (Denise) | WO | Not established | Unknown | Sprite 5 image data register A. |
| `$16E` | — | `SPR5DATB` % (Denise) | WO | Not established | Unknown | Sprite 5 image data register B. |
| `$170` | — | `SPR6POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 6 vertical-horizontal start position data. |
| `$172` | — | `SPR6CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 6 vertical stop position and control data. |
| `$174` | — | `SPR6DATA` % (Denise) | WO | Not established | Unknown | Sprite 6 image data register A. |
| `$176` | — | `SPR6DATB` % (Denise) | WO | Not established | Unknown | Sprite 6 image data register B. |
| `$178` | — | `SPR7POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 7 vertical-horizontal start position data. |
| `$17A` | — | `SPR7CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 7 vertical stop position and control data. |
| `$17C` | — | `SPR7DATA` % (Denise) | WO | Not established | Unknown | Sprite 7 image data register A. |
| `$17E` | — | `SPR7DATB` % (Denise) | WO | Not established | Unknown | Sprite 7 image data register B. |
| `$180` | — | `COLOR00` (Denise) | WO | Not established | Unknown | Color table 00. |
| `$182` | — | `COLOR01` (Denise) | WO | Not established | Unknown | Color table 01. |
| `$184` | — | `COLOR02` (Denise) | WO | Not established | Unknown | Color table 02. |
| `$186` | — | `COLOR03` (Denise) | WO | Not established | Unknown | Color table 03. |
| `$188` | — | `COLOR04` (Denise) | WO | Not established | Unknown | Color table 04. |
| `$18A` | — | `COLOR05` (Denise) | WO | Not established | Unknown | Color table 05. |
| `$18C` | — | `COLOR06` (Denise) | WO | Not established | Unknown | Color table 06. |
| `$18E` | — | `COLOR07` (Denise) | WO | Not established | Unknown | Color table 07. |
| `$190` | — | `COLOR08` (Denise) | WO | Not established | Unknown | Color table 08. |
| `$192` | — | `COLOR09` (Denise) | WO | Not established | Unknown | Color table 09. |
| `$194` | — | `COLOR10` (Denise) | WO | Not established | Unknown | Color table 10. |
| `$196` | — | `COLOR11` (Denise) | WO | Not established | Unknown | Color table 11. |
| `$198` | — | `COLOR12` (Denise) | WO | Not established | Unknown | Color table 12. |
| `$19A` | — | `COLOR13` (Denise) | WO | Not established | Unknown | Color table 13. |
| `$19C` | — | `COLOR14` (Denise) | WO | Not established | Unknown | Color table 14. |
| `$19E` | — | `COLOR15` (Denise) | WO | Not established | Unknown | Color table 15. |
| `$1A0` | — | `COLOR16` (Denise) | WO | Not established | Unknown | Color table 16. |
| `$1A2` | — | `COLOR17` (Denise) | WO | Not established | Unknown | Color table 17. |
| `$1A4` | — | `COLOR18` (Denise) | WO | Not established | Unknown | Color table 18. |
| `$1A6` | — | `COLOR19` (Denise) | WO | Not established | Unknown | Color table 19. |
| `$1A8` | — | `COLOR20` (Denise) | WO | Not established | Unknown | Color table 20. |
| `$1AA` | — | `COLOR21` (Denise) | WO | Not established | Unknown | Color table 21. |
| `$1AC` | — | `COLOR22` (Denise) | WO | Not established | Unknown | Color table 22. |
| `$1AE` | — | `COLOR23` (Denise) | WO | Not established | Unknown | Color table 23. |
| `$1B0` | — | `COLOR24` (Denise) | WO | Not established | Unknown | Color table 24. |
| `$1B2` | — | `COLOR25` (Denise) | WO | Not established | Unknown | Color table 25. |
| `$1B4` | — | `COLOR26` (Denise) | WO | Not established | Unknown | Color table 26. |
| `$1B6` | — | `COLOR27` (Denise) | WO | Not established | Unknown | Color table 27. |
| `$1B8` | — | `COLOR28` (Denise) | WO | Not established | Unknown | Color table 28. |
| `$1BA` | — | `COLOR29` (Denise) | WO | Not established | Unknown | Color table 29. |
| `$1BC` | — | `COLOR30` (Denise) | WO | Not established | Unknown | Color table 30. |
| `$1BE` | — | `COLOR31` (Denise) | WO | Not established | Unknown | Color table 31. |
| `$1C0` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1C2` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1C4` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1C6` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1C8` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1CA` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1CC` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1CE` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1D0` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1D2` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1D4` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1D6` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1D8` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1DA` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. |
| `$1DC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1DE` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1E0` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1E2` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1E4` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1E6` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1E8` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1EA` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1EC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1EE` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1F0` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1F2` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1F4` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1F6` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1F8` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1FA` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1FC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. |
| `$1FE` | — | — | Unspecified | Not specified | Unknown | `NO-OP(NULL)`: No-operation; Appendix B gives no access direction or chip. |

## Reference Documentation & Upstream Ground Truth

- [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Address order, names, access symbols, chip listings, and concise functions used for every row.
- [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Opposite-direction access caution, collision read-and-clear behavior, Copper jump strobes, and Agnus refresh strobes.
- [Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md): Demonstrates a CPU read of `COPJMP1` that reloads the Copper program counter.
