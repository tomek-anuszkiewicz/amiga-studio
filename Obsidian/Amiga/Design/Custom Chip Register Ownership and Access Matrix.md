---
title: "Custom Chip Register Ownership and Access Matrix"
aliases: ["Custom Chip Register Matrix", "Custom Register Map", "Hardware Register Catalog", "Chipset Register Reference"]
tags: ["amiga", "design", "registers", "agnus", "denise", "paula", "memory-map"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-09-14
updated: 2026-11-12
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Custom Chip Register Ownership and Access Matrix

This is the book-derived address map and current owner-delivery reference for the original custom-chip register block. The 68000 address is `$DFF000` plus the three-digit offset below. [Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md) supplies names, directions, chip listings, and short functions; [Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md) supplies fields, access cautions and exceptions. Hardware meaning, modeled delivery and missing functional effects are separate claims below.

## Reading the matrix

- `RO` and `WO` reproduce the manual's documented read or write direction. `—` means no **documented safe counterpart** at that offset. It does not mean that an opposite-direction bus cycle has no effect. Appendix A says a read of a write-only register can disturb it and a write to a read-only register can have unexpected results.
- `ER` is an early DMA read initiated by Agnus timing, not an ordinary CPU register read. `COR` marks `CLXDAT`, whose documented read also clears collision state. `STROBE` marks an address-triggered action rather than stored register bits. Both `COPJMP1` and `COPJMP2` can trigger a Copper jump on a CPU read or write. The four sync strobes (`STREQU`, `STRVBL`, `STRHOR`, `STRLONG`) are documented as write-address strobes used by Agnus during refresh; their CPU reads have no documented safe strobe semantics. The read/write distinction does not make an opposite-direction access harmless.
- A name suffix `&` means DMA-only use; `%` means usually DMA, sometimes processor; `+` marks a chip-memory pointer pair. A prefix `*` on the Appendix B address prohibits Copper writes. A prefix `-` permits Copper writes only with `COPCON.CDANG` set. These markers appear in the notes rather than the offset so the offset stays numeric.
- `Scope` records only what the book supports: `Multiple chips listed` where Appendix B names more than one chip. A single listed chip does not establish that every downstream effect stays inside that chip, so other rows say `Not established`. The appendices do not provide a general per-register CCK propagation delay, so `Delay` is `Unknown`; it must not be interpreted as zero.
- Chip listings reproduce the manual; they are not a register-write dispatch table. Use an explicit design-model note in a row to decide whether a listed chip needs a modeled write action.
- `Reserved` reproduces an explicit Appendix B label. `Unlisted` applies only to `$1C0–$1DA`, which the address-order table skips without labeling. Neither classification establishes open-bus values or other silicon behavior. Appendix A advises against accessing unused registers.

## Modeled interfaces and timing

The map applies to the supported PAL 8371 and NTSC 8370 Agnus with OCS 8362
Denise and 8364 Paula. It does not expose ECS/AGA registers. The current
[MemoryBus map](MemoryBus.md#a-cpu-access-through-the-router) selects bank `$DF`
and mirrors its low nine address bits throughout that bank. A0 selects the
read byte lane; it does not select a different register. A word uses one
aligned offset; a byte write on either lane duplicates its byte into both
halves. Reserved/unlisted writes acknowledge without enqueuing anything.
Dummy early reads, reads without a modeled response and `NO-OP(NULL)` return the fixed `$FFFF`
placeholder. These are emulator limits, not safe-access guarantees. Expanded
[MemoryBus](MemoryBus.md#cia-and-boot-overlay) defines Gary bank selection,
CIA aliases, dual selection and lane behavior.

The final column records the production write interface in
[`custom_registers.rs`](../../../crates/memory_bus/src/custom_registers.rs).
`A`, `D`, `P` name Agnus, Denise and Paula. A number followed by `P` is the delay in subsequent owner CCK steps and `Pipeline` mode, preserving each accepted data operation. Concrete reference timings recorded in the timing research are retained; unresolved custom data writes use a one-CCK fallback. Address strobes act immediately at accepted selection and allocate no queue entry. Shared data writes require every recipient to accept atomically. This is the user-approved model policy of 2026-11-11; the hardware `Delay` column remains unknown. A committed write can still wait for a later engine consumer. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) owns acceptance, retry and issue boundaries.

The named setters live in [Agnus registers](../../../crates/agnus/src/registers.rs),
[Blitter register staging](../../../crates/agnus/src/blitter_registers.rs),
[Copper register staging](../../../crates/agnus/src/copper_registers.rs),
[Denise registers](../../../crates/denise/src/registers.rs) and
[Paula registers](../../../crates/paula/src/registers.rs). Indexed methods use
zero-based channel indices; bitplane names 1–6 therefore use indices 0–5.
`set_color` covers indices 0–31 and sprite methods indices 0–7. Register-half
methods preserve separate high/low event identities. Address strobes ignore
bus data and act immediately. Agnus owns the data-write queue for its Copper
and Blitter children and handles their address strobes through named interfaces.

CPU writes and legal Copper MOVE destinations use the same setters. The
[Copper](Copper.md) applies the manual's address restrictions before emitting
its retained request: offsets below `$040` are prohibited, `$040–$07E` require
`COPCON.CDANG`, and `$080` upward are available. Direct calls to the router do
not enforce a Copper policy on CPU writes. A rejected request is retried;
Reset cancels unaccepted Copper output and queued events; DMA enable changes
gate the engine separately from accepted register delivery. Live CPU reads occur in CCK1 and writes in CCK2;
owner steps precede CPU issue. Copper issue also follows all custom-chip
steps. Thus the same accepted offset can mature at different recipients on
different later steps without implying a second motherboard delay.

### State owners and hardware meaning

Each family below expands to the individual addresses in the map. The linked
owning specifications and Appendix A sections define the fields and intended
hardware effects; the delivery column describes the current access mechanism.

| Family | State owner and fields / intended effect | Source sections |
| --- | --- | --- |
| `BLTDDAT`, `BLTCON0/1`, `BLTxPT`, `BLTxMOD`, `BLTxDAT`, `BLTAFWM/BLTALWM`, `BLTSIZE` | [Blitter](Blitter.md), inside Agnus: channel enables, shifts, Boolean minterm and line/fill/direction controls; A/B/C source and D destination pointer halves; signed per-channel modulo; A first/last masks; A/B/C data. `BLTSIZE` supplies width/height and starts an operation. `BLTDDAT` is the destination early-read dummy address. | [`bltcon0 / bltcon1`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bltcon0-bltcon1), [`bltsize`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bltsize), [`bltxpth / bltxptl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bltxpth-bltxptl) |
| `DMACON/DMACONR` | [DMA](DMA.md) enables and Blitter status in Agnus; Paula captures the shared write for audio/disk enables. Bit 15 selects set/clear; `DMACONR` reports enable/busy/zero state through Agnus alone. Denise has no modeled write action despite the Appendix B chip listing. | [`dmacon / dmaconr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dmacon-dmaconr) |
| `VPOSR/VPOSW`, `VHPOSR/VHPOSW` | [Agnus](Agnus.md) beam/frame state: high vertical bit and frame flag, low vertical byte and horizontal byte; VPOSR also reports revision/model bits. Read/write identities have different offsets. | [`vposr / vposw`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#vposr-vposw), [`vhposr / vhposw`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#vhposr-vhposw) |
| `REFPTR` | Agnus refresh address generator, writable for test only; Appendix A says the microprocessor should never write it. | [`refptr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#refptr) |
| `COP1LC/COP2LC`, `COPCON`, `COPJMP1/2`, `COPINS` | [Copper](Copper.md), inside Agnus: two list pointer pairs, dangerous-address permission, and address-triggered jumps. COPINS identifies an internal instruction fetch; it is a dummy bus address, not a general instruction-injection promise. | [`cop1lch / cop1lcl / cop2lch / cop2lcl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#cop1lch-cop1lcl-cop2lch-cop2lcl), [`copcon`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#copcon), [`copjmp1 / copjmp2`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#copjmp1-copjmp2), [`copins`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#copins) |
| `DIWSTRT/DIWSTOP`, `DDFSTRT/DDFSTOP` | Agnus owns vertical DMA gating and horizontal fetch boundaries; [Denise](Denise.md) also owns horizontal display clipping from DIW writes. Vertical/horizontal fields are distinct. The two DIW recipients have independent delays. | [`diwstop / diwstrt`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#diwstop-diwstrt), [`ddfstop / ddfstrt`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#ddfstop-ddfstrt) |
| `BPL1PT` through `BPL6PT`, `BPL1MOD/BPL2MOD` | Agnus owns six DMA pointer pairs and odd/even plane signed modulos. OCS pointers retain the high three and low fifteen word-address bits; [Agnus](Agnus.md) explains the even byte-address representation. | [`bplxpth / bplxptl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bplxpth-bplxptl), [`bpl1mod / bpl2mod`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bpl1mod-bpl2mod) |
| `BPLCON0/1/2`, `BPL1DAT` through `BPL6DAT` | Denise owns resolution/plane count, HAM/dual-playfield/interlace controls, two playfield scroll fields and priority controls; Agnus also captures BPLCON0 for fetch configuration. Six data latches supply sixteen pixels per word; BPL1DAT initiates the parallel transfer to serializers. | [`bplcon0 / bplcon1 / bplcon2`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bplcon0-bplcon1-bplcon2), [`bplxdat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bplxdat) |
| `SPR0PT` through `SPR7PT`, `SPR0POS/CTL` through `SPR7POS/CTL` | Agnus owns eight DMA pointer pairs and vertical start/stop/control state; Denise's [Sprites](Sprites.md) children own horizontal position, attachment and matching display latches. Both chips capture every POS and CTL write. | [`sprxpth / sprxptl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#sprxpth-sprxptl), [`sprxctl / sprxpos`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#sprxctl-sprxpos) |
| `SPR0DATA/DATB` through `SPR7DATA/DATB` | Denise's sprite children own the two image words per sprite and display arming behavior; no Agnus data recipient. | [`sprxdata / sprxdatb`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#sprxdata-sprxdatb) |
| `COLOR00` through `COLOR31` | Denise owns 32 palette words, with four red, green and blue bits each. Unused high bits are not additional color components. | [`colorxx`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#colorxx) |
| `CLXCON/CLXDAT` | Denise collision selection and accumulated pair results. CLXCON selects plane values/enables and sprite grouping; a live CLXDAT read returns then clears accumulated collisions. Debug reads retain them. | [`clxcon`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#clxcon), [`clxdat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#clxdat) |
| `JOY0DAT/JOY1DAT`, `JOYTEST` | Denise game-port counter read interface; each read contains an X and Y byte. JOYTEST should write bits 7–2 of both bytes of both ports, preserving the low two bits. | [`joy0dat / joy1dat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#joy0dat-joy1dat), [`joytest`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#joytest) |
| `STREQU/STRVBL/STRHOR/STRLONG` | Address strobes for equalization/vertical blank, vertical blank, horizontal sync and long-line identification. HRM lists Denise for all four and Paula for STRHOR. The approved [Paula model](Paula.md#rga-synchronization-and-vertb) receives all four, with STRLONG leaving its state unchanged. They carry actions, not stored bits. | [`strequ / strhor / strlong / strvbl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl) |
| `DSKPT`, `DSKLEN`, `DSKDAT/DSKDATR`, `DSKSYNC/DSKBYTR` | Agnus owns the disk pointer pair. Paula owns length/direction/arming, disk data, sync comparison, byte availability and DMA/write/sync status; live DSKBYTR acknowledges byte availability. DSKDATR is an early-read dummy. [Floppy](Floppy.md) owns the drive-side stream; media transfers are outside the current closure. | [`dskpth / dskptl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dskpth-dskptl), [`dsklen`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dsklen), [`dskdat / dskdatr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dskdat-dskdatr), [`dsksync`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dsksync), [`dskbytr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#dskbytr) |
| `AUD0LC` through `AUD3LC`, `AUD0LEN/PER/VOL/DAT` through `AUD3LEN/PER/VOL/DAT` | Agnus owns four location pairs. Paula's [Audio](Audio.md) children own length reload, period, six-bit volume plus full-volume selection and two signed sample bytes per data word. Transport does not certify playback or the relation between programmed reload and active counters. | [`audxlch / audxlcl`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#audxlch-audxlcl), [`audxlen`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#audxlen), [`audxper`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#audxper), [`audxvol`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#audxvol), [`audxdat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#audxdat) |
| `ADKCON/ADKCONR` | Paula owns audio modulation and disk/UART control; bit 15 selects set/clear on writes. The separate readable address returns active controls. | [`adkcon / adkconr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#adkcon-adkconr) |
| `INTENA/INTENAR`, `INTREQ/INTREQR` | Paula's [Interrupts](Interrupts.md) child owns source enables, master enable and pending requests. Bit 15 selects set/clear on writes. Read addresses expose active enable/request bits; they do not acknowledge by reading. Delivered INT2/INT3/INT6 levels are sampled after due register writes without a queue entry. | [`intena / intenar`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#intena-intenar), [`intreq / intreqr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#intreq-intreqr) |
| `SERDAT/SERDATR`, `SERPER` | Paula serial child owns transmit data/stop bits, receive data and status, period and long-word selection. | [`serdat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#serdat), [`serdatr`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#serdatr), [`serper`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#serper) |
| `POT0DAT/POT1DAT`, `POTGO/POTGOR` | Paula owns proportional counters and port control/sampled inputs. POTGO supplies start, output enables and output levels; POTGOR reads pins, and POTDAT pairs read counter bytes. | [`pot0dat / pot1dat`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#pot0dat-pot1dat), [`potgo / potgor`](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#potgo-potgor) |
### Read interfaces and current limits

`read_custom_word` calls the lowercase register-named reader on the chip named
in the read column, with two exceptions: `DSKBYTR` uses `Paula::read_dskbytr`,
and `COPJMP1/2` use `Agnus::strobe_copjmp1/2`. `DMACONR` reads Agnus, rather
than combining two software readers. `BLTDDAT/DSKDATR` return the placeholder.
Debug dispatch uses the corresponding `_debug` reader, except
`DSKBYTR` uses `peek_dskbytr`. Debug COPJMP reads return `$FFFF` without a
strobe. Live CLXDAT and DSKBYTR acknowledge in their readers; live COPJMP
reads act immediately rather than entering the write queue. Other addresses
use the fallback; opposite-direction silicon disturbances are not modeled.

The owning chip specifications describe reconstructed register effects.
The [guest verification work](../../../ROADMAP.md#cpu-memory-gary-verification)
retains the final register-wide access and observable-effect audit.

## Address map

| Offset | Read Name (Chip) | Write Name (Chip) | Access Mode | Scope | Delay | Function & Book Notes | Modeled write interface / delivery |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `$000` | `BLTDDAT` & (Agnus) | — | ER | Not established | Unknown | Blitter destination early read (dummy address). Copper write prohibited. | Ignored write; no queue |
| `$002` | `DMACONR` (Agnus, Paula) | — | RO | Multiple chips listed | Unknown | DMA control (and blitter status) read. Copper write prohibited. | Ignored write; no queue |
| `$004` | `VPOSR` (Agnus) | — | RO | Not established | Unknown | Read vertical most significant bit (and frame flop). Copper write prohibited. | Ignored write; no queue |
| `$006` | `VHPOSR` (Agnus) | — | RO | Not established | Unknown | Read vertical and horizontal position of beam. Copper write prohibited. | Ignored write; no queue |
| `$008` | `DSKDATR` & (Paula) | — | ER | Not established | Unknown | Disk data early read (dummy address). Copper write prohibited. | Ignored write; no queue |
| `$00A` | `JOY0DAT` (Denise) | — | RO | Not established | Unknown | Joystick-mouse 0 data (vertical, horizontal). Copper write prohibited. | Ignored write; no queue |
| `$00C` | `JOY1DAT` (Denise) | — | RO | Not established | Unknown | Joystick-mouse 1 data (vertical, horizontal). Copper write prohibited. | Ignored write; no queue |
| `$00E` | `CLXDAT` (Denise) | — | COR | Not established | Unknown | Collision data register (read and clear). Copper write prohibited. | Ignored write; no queue |
| `$010` | `ADKCONR` (Paula) | — | RO | Not established | Unknown | Audio, disk control register read. Copper write prohibited. | Ignored write; no queue |
| `$012` | `POT0DAT` (Paula) | — | RO | Not established | Unknown | Pot counter pair 0 data (vertical, horizontal). Copper write prohibited. | Ignored write; no queue |
| `$014` | `POT1DAT` (Paula) | — | RO | Not established | Unknown | Pot counter pair 1 data (vertical, horizontal). Copper write prohibited. | Ignored write; no queue |
| `$016` | `POTGOR` (Paula) | — | RO | Not established | Unknown | Pot port data read (formerly `POTINP`). Copper write prohibited. | Ignored write; no queue |
| `$018` | `SERDATR` (Paula) | — | RO | Not established | Unknown | Serial port data and status read. Copper write prohibited. | Ignored write; no queue |
| `$01A` | `DSKBYTR` (Paula) | — | RO | Not established | Unknown | Disk data byte and status read. Copper write prohibited. | Ignored write; no queue |
| `$01C` | `INTENAR` (Paula) | — | RO | Not established | Unknown | Interrupt enable bits read. Copper write prohibited. | Ignored write; no queue |
| `$01E` | `INTREQR` (Paula) | — | RO | Not established | Unknown | Interrupt request bits read. Copper write prohibited. | Ignored write; no queue |
| `$020` | — | `DSKPTH` + (Agnus) | WO | Not established | Unknown | Disk pointer (high 3 bits). Copper write prohibited. | `A.set_dskpth(value)` 1 P |
| `$022` | — | `DSKPTL` + (Agnus) | WO | Not established | Unknown | Disk pointer (low 15 bits). Copper write prohibited. | `A.set_dskptl(value)` 1 P |
| `$024` | — | `DSKLEN` (Paula) | WO | Not established | Unknown | Disk length. Copper write prohibited. | `P.set_dsklen(value)` 1 P |
| `$026` | — | `DSKDAT` & (Paula) | WO | Not established | Unknown | Disk DMA data write. Copper write prohibited. | `P.set_dskdat(value)` 1 P |
| `$028` | — | `REFPTR` & (Agnus) | WO | Not established | Unknown | Refresh pointer. Copper write prohibited. | `A.set_refptr(value)` 1 P; pending effect |
| `$02A` | — | `VPOSW` (Agnus) | WO | Not established | Unknown | Write vertical most significant bit (and frame flop). Copper write prohibited. | `A.set_vposw(value)` 1 P; pending effect |
| `$02C` | — | `VHPOSW` (Agnus) | WO | Not established | Unknown | Write vertical and horizontal position of beam. Copper write prohibited. | `A.set_vhposw(value)` 1 P; pending effect |
| `$02E` | — | `COPCON` (Agnus) | WO | Not established | Unknown | Coprocessor control register (CDANG). Copper write prohibited. | `A.set_copcon(value)` 1 P |
| `$030` | — | `SERDAT` (Paula) | WO | Not established | Unknown | Serial port data and stop bits write. Copper write prohibited. | `P.set_serdat(value)` 1 P |
| `$032` | — | `SERPER` (Paula) | WO | Not established | Unknown | Serial port period and control. Copper write prohibited. | `P.set_serper(value)` 1 P |
| `$034` | — | `POTGO` (Paula) | WO | Not established | Unknown | Pot port data write and start. Copper write prohibited. | `P.set_potgo(value)` 1 P |
| `$036` | — | `JOYTEST` (Denise) | WO | Not established | Unknown | Write to all 4 joystick-mouse counters at once. Copper write prohibited. | `D.set_joytest(value)` 1 P; pending effect |
| `$038` | — | `STREQU` & (Denise) | STROBE | Not established | Unknown | Write-address strobe for horizontal sync with vertical blank and equalization; Agnus emits it during refresh. Copper write prohibited. | `D.strobe_strequ()` immediate; `P.strobe_sync($038)` immediate: line phase, DMAL capture and VERTB on equalization entry; model timing |
| `$03A` | — | `STRVBL` & (Denise) | STROBE | Not established | Unknown | Write-address strobe for horizontal sync with vertical blank; Agnus emits it during refresh. Copper write prohibited. | `D.strobe_strvbl()` immediate; `P.strobe_sync($03A)` immediate: line phase, DMAL capture and equalization clear; model timing |
| `$03C` | — | `STRHOR` & (Denise, Paula) | STROBE | Multiple chips listed | Unknown | Write-address strobe for horizontal sync; Agnus emits it during refresh. Copper write prohibited. | `D.strobe_strhor()` immediate; `P.strobe_sync($03C)` immediate: line phase, DMAL capture and equalization clear; model timing |
| `$03E` | — | `STRLONG` & (Denise) | STROBE | Not established | Unknown | Write-address strobe identifying a long horizontal line; Agnus emits it during refresh. Copper write prohibited. | `D.strobe_strlong()` immediate; `P.strobe_sync($03E)` receives the address without changing local phase or equalization; model timing |
| `$040` | — | `BLTCON0` (Agnus) | WO | Not established | Unknown | Blitter control register 0. Copper write requires `COPCON.CDANG`. | `A.set_bltcon0(value)` 1 P |
| `$042` | — | `BLTCON1` (Agnus) | WO | Not established | Unknown | Blitter control register 1. Copper write requires `COPCON.CDANG`. | `A.set_bltcon1(value)` 1 P |
| `$044` | — | `BLTAFWM` (Agnus) | WO | Not established | Unknown | Blitter first word mask for source A. Copper write requires `COPCON.CDANG`. | `A.set_bltafwm(value)` 1 P |
| `$046` | — | `BLTALWM` (Agnus) | WO | Not established | Unknown | Blitter last word mask for source A. Copper write requires `COPCON.CDANG`. | `A.set_bltalwm(value)` 1 P |
| `$048` | — | `BLTCPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source C (high 3 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltcpth(value)` 1 P |
| `$04A` | — | `BLTCPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source C (low 15 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltcptl(value)` 1 P |
| `$04C` | — | `BLTBPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source B (high 3 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltbpth(value)` 1 P |
| `$04E` | — | `BLTBPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source B (low 15 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltbptl(value)` 1 P |
| `$050` | — | `BLTAPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source A (high 3 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltapth(value)` 1 P |
| `$052` | — | `BLTAPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to source A (low 15 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltaptl(value)` 1 P |
| `$054` | — | `BLTDPTH` + (Agnus) | WO | Not established | Unknown | Blitter pointer to destination D (high 3 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltdpth(value)` 1 P |
| `$056` | — | `BLTDPTL` + (Agnus) | WO | Not established | Unknown | Blitter pointer to destination D (low 15 bits). Copper write requires `COPCON.CDANG`. | `A.set_bltdptl(value)` 1 P |
| `$058` | — | `BLTSIZE` (Agnus) | WO | Not established | Unknown | Blitter start and size (window width, height). Copper write requires `COPCON.CDANG`. | `A.set_bltsize(value)` 1 P |
| `$05A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$05C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$05E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$060` | — | `BLTCMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source C. Copper write requires `COPCON.CDANG`. | `A.set_bltcmod(value)` 1 P |
| `$062` | — | `BLTBMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source B. Copper write requires `COPCON.CDANG`. | `A.set_bltbmod(value)` 1 P |
| `$064` | — | `BLTAMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for source A. Copper write requires `COPCON.CDANG`. | `A.set_bltamod(value)` 1 P |
| `$066` | — | `BLTDMOD` (Agnus) | WO | Not established | Unknown | Blitter modulo for destination D. Copper write requires `COPCON.CDANG`. | `A.set_bltdmod(value)` 1 P |
| `$068` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$06A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$06C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$06E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$070` | — | `BLTCDAT` % (Agnus) | WO | Not established | Unknown | Blitter source C data register. Copper write requires `COPCON.CDANG`. | `A.set_bltcdat(value)` 1 P |
| `$072` | — | `BLTBDAT` % (Agnus) | WO | Not established | Unknown | Blitter source B data register. Copper write requires `COPCON.CDANG`. | `A.set_bltbdat(value)` 1 P |
| `$074` | — | `BLTADAT` % (Agnus) | WO | Not established | Unknown | Blitter source A data register. Copper write requires `COPCON.CDANG`. | `A.set_bltadat(value)` 1 P |
| `$076` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$078` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$07A` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$07C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$07E` | — | `DSKSYNC` (Paula) | WO | Not established | Unknown | Disk sync pattern register for disk read. Copper write requires `COPCON.CDANG`. | `P.set_dsksync(value)` 1 P |
| `$080` | — | `COP1LCH` + (Agnus) | WO | Not established | Unknown | Coprocessor first location register (high 3 bits). | `A.set_cop1lch(value)` 1 P |
| `$082` | — | `COP1LCL` + (Agnus) | WO | Not established | Unknown | Coprocessor first location register (low 15 bits). | `A.set_cop1lcl(value)` 1 P |
| `$084` | — | `COP2LCH` + (Agnus) | WO | Not established | Unknown | Coprocessor second location register (high 3 bits). | `A.set_cop2lch(value)` 1 P |
| `$086` | — | `COP2LCL` + (Agnus) | WO | Not established | Unknown | Coprocessor second location register (low 15 bits). | `A.set_cop2lcl(value)` 1 P |
| `$088` | `COPJMP1` (Agnus) | `COPJMP1` (Agnus) | STROBE | Not established | Unknown | CPU read or write restarts the Copper at the first location; the manual demonstrates a read. Avoid read-modify-write. | `A.strobe_copjmp1()` immediate |
| `$08A` | `COPJMP2` (Agnus) | `COPJMP2` (Agnus) | STROBE | Not established | Unknown | CPU read or write restarts the Copper at the second location; Appendix A explicitly identifies both triggers. Avoid read-modify-write. | `A.strobe_copjmp2()` immediate |
| `$08C` | — | `COPINS` (Agnus) | WO | Not established | Unknown | Coprocessor instruction fetch identify. | `A.set_copins(value)` 1 P; pending effect |
| `$08E` | — | `DIWSTRT` (Agnus) | WO | Not established | Unknown | Display window start (upper left vertical-horizontal position). Design model: Agnus uses `VSTART` to begin bitplane DMA on whole lines; Denise uses `HSTART` to begin displaying pixels within a line, at low-resolution pixel granularity. The [Denise IC specification](https://www.amigawiki.org/lib/exe/fetch.php?media=de%3Aparts%3Adenise_specs.pdf) assigns the horizontal bits to Denise although Appendix B lists only Agnus. | `A.set_diwstrt(value)` 4 P; `D.set_diwstrt(value)` 1 P |
| `$090` | — | `DIWSTOP` (Agnus) | WO | Not established | Unknown | Display window stop (lower right vertical and horizontal position). Design model: Agnus uses `VSTOP` to end bitplane DMA on whole lines; Denise uses `HSTOP` to end visible pixels within a line, at low-resolution pixel granularity. | `A.set_diwstop(value)` 4 P; `D.set_diwstop(value)` 1 P |
| `$092` | — | `DDFSTRT` (Agnus) | WO | Not established | Unknown | Display bitplane data fetch start (horizontal position). Agnus starts fetching 16-bit bitplane words here; bitplane pointers select their Chip RAM addresses. Fetching begins before the pixels can appear at the `DIWSTRT` edge. | `A.set_ddfstrt(value)` 4 P |
| `$094` | — | `DDFSTOP` (Agnus) | WO | Not established | Unknown | Display bitplane data fetch stop (horizontal position). Agnus marks the final bitplane fetch block of each line here; transfers within that block can continue beyond this position. Together with `DDFSTRT`, this interval must supply the data needed for the visible playfield and any scrolling margin; its boundaries need not match the pixel-level display window. | `A.set_ddfstop(value)` 4 P |
| `$096` | — | `DMACON` (Agnus, Denise, Paula) | WO | Multiple chips listed | Unknown | DMA control write (clear or set). Agnus latches the DMA enables; Paula reads the write value from the shared register bus. Denise takes no action. See [DMA](DMA.md). | `A.set_dmacon(value)` 2 P; `P.set_dmacon(value)` 2 P |
| `$098` | — | `CLXCON` (Denise) | WO | Not established | Unknown | Collision control. | `D.set_clxcon(value)` 1 P |
| `$09A` | — | `INTENA` (Paula) | WO | Not established | Unknown | Interrupt enable bits (clear or set bits). | `P.set_intena(value)` 1 P |
| `$09C` | — | `INTREQ` (Paula) | WO | Not established | Unknown | Interrupt request bits (clear or set bits). | `P.set_intreq(value)` 1 P |
| `$09E` | — | `ADKCON` (Paula) | WO | Not established | Unknown | Audio, disk, UART control. | `P.set_adkcon(value)` 1 P |
| `$0A0` | — | `AUD0LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 0 location (high 3 bits). | `A.set_audlch(0, value)` 1 P |
| `$0A2` | — | `AUD0LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 0 location (low 15 bits). | `A.set_audlcl(0, value)` 1 P |
| `$0A4` | — | `AUD0LEN` (Paula) | WO | Not established | Unknown | Audio channel 0 length. | `P.set_audlen(0, value)` 1 P |
| `$0A6` | — | `AUD0PER` (Paula) | WO | Not established | Unknown | Audio channel 0 period. | `P.set_audper(0, value)` 1 P |
| `$0A8` | — | `AUD0VOL` (Paula) | WO | Not established | Unknown | Audio channel 0 volume. | `P.set_audvol(0, value)` 1 P |
| `$0AA` | — | `AUD0DAT` & (Paula) | WO | Not established | Unknown | Audio channel 0 data. | `P.set_auddat(0, value)` 1 P |
| `$0AC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0AE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0B0` | — | `AUD1LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 1 location (high 3 bits). | `A.set_audlch(1, value)` 1 P |
| `$0B2` | — | `AUD1LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 1 location (low 15 bits). | `A.set_audlcl(1, value)` 1 P |
| `$0B4` | — | `AUD1LEN` (Paula) | WO | Not established | Unknown | Audio channel 1 length. | `P.set_audlen(1, value)` 1 P |
| `$0B6` | — | `AUD1PER` (Paula) | WO | Not established | Unknown | Audio channel 1 period. | `P.set_audper(1, value)` 1 P |
| `$0B8` | — | `AUD1VOL` (Paula) | WO | Not established | Unknown | Audio channel 1 volume. | `P.set_audvol(1, value)` 1 P |
| `$0BA` | — | `AUD1DAT` & (Paula) | WO | Not established | Unknown | Audio channel 1 data. | `P.set_auddat(1, value)` 1 P |
| `$0BC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0BE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0C0` | — | `AUD2LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 2 location (high 3 bits). | `A.set_audlch(2, value)` 1 P |
| `$0C2` | — | `AUD2LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 2 location (low 15 bits). | `A.set_audlcl(2, value)` 1 P |
| `$0C4` | — | `AUD2LEN` (Paula) | WO | Not established | Unknown | Audio channel 2 length. | `P.set_audlen(2, value)` 1 P |
| `$0C6` | — | `AUD2PER` (Paula) | WO | Not established | Unknown | Audio channel 2 period. | `P.set_audper(2, value)` 1 P |
| `$0C8` | — | `AUD2VOL` (Paula) | WO | Not established | Unknown | Audio channel 2 volume. | `P.set_audvol(2, value)` 1 P |
| `$0CA` | — | `AUD2DAT` & (Paula) | WO | Not established | Unknown | Audio channel 2 data. | `P.set_auddat(2, value)` 1 P |
| `$0CC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0CE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0D0` | — | `AUD3LCH` + (Agnus) | WO | Not established | Unknown | Audio channel 3 location (high 3 bits). | `A.set_audlch(3, value)` 1 P |
| `$0D2` | — | `AUD3LCL` + (Agnus) | WO | Not established | Unknown | Audio channel 3 location (low 15 bits). | `A.set_audlcl(3, value)` 1 P |
| `$0D4` | — | `AUD3LEN` (Paula) | WO | Not established | Unknown | Audio channel 3 length. | `P.set_audlen(3, value)` 1 P |
| `$0D6` | — | `AUD3PER` (Paula) | WO | Not established | Unknown | Audio channel 3 period. | `P.set_audper(3, value)` 1 P |
| `$0D8` | — | `AUD3VOL` (Paula) | WO | Not established | Unknown | Audio channel 3 volume. | `P.set_audvol(3, value)` 1 P |
| `$0DA` | — | `AUD3DAT` & (Paula) | WO | Not established | Unknown | Audio channel 3 data. | `P.set_auddat(3, value)` 1 P |
| `$0DC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0DE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0E0` | — | `BPL1PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 1 pointer (high 3 bits). | `A.set_bplpth(0, value)` 1 P |
| `$0E2` | — | `BPL1PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 1 pointer (low 15 bits). | `A.set_bplptl(0, value)` 1 P |
| `$0E4` | — | `BPL2PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 2 pointer (high 3 bits). | `A.set_bplpth(1, value)` 1 P |
| `$0E6` | — | `BPL2PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 2 pointer (low 15 bits). | `A.set_bplptl(1, value)` 1 P |
| `$0E8` | — | `BPL3PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 3 pointer (high 3 bits). | `A.set_bplpth(2, value)` 1 P |
| `$0EA` | — | `BPL3PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 3 pointer (low 15 bits). | `A.set_bplptl(2, value)` 1 P |
| `$0EC` | — | `BPL4PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 4 pointer (high 3 bits). | `A.set_bplpth(3, value)` 1 P |
| `$0EE` | — | `BPL4PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 4 pointer (low 15 bits). | `A.set_bplptl(3, value)` 1 P |
| `$0F0` | — | `BPL5PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 5 pointer (high 3 bits). | `A.set_bplpth(4, value)` 1 P |
| `$0F2` | — | `BPL5PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 5 pointer (low 15 bits). | `A.set_bplptl(4, value)` 1 P |
| `$0F4` | — | `BPL6PTH` + (Agnus) | WO | Not established | Unknown | Bitplane 6 pointer (high 3 bits). | `A.set_bplpth(5, value)` 1 P |
| `$0F6` | — | `BPL6PTL` + (Agnus) | WO | Not established | Unknown | Bitplane 6 pointer (low 15 bits). | `A.set_bplptl(5, value)` 1 P |
| `$0F8` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0FA` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0FC` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$0FE` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$100` | — | `BPLCON0` (Agnus, Denise) | WO | Multiple chips listed | Unknown | Bitplane control register (misc. control bits). | `A.set_bplcon0(value)` 4 P; `D.set_bplcon0(value)` 1 P |
| `$102` | — | `BPLCON1` (Denise) | WO | Not established | Unknown | Bitplane control reg. (scroll value PF1, PF2). | `D.set_bplcon1(value)` 1 P |
| `$104` | — | `BPLCON2` (Denise) | WO | Not established | Unknown | Bitplane control reg. (priority control). | `D.set_bplcon2(value)` 1 P |
| `$106` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$108` | — | `BPL1MOD` (Agnus) | WO | Not established | Unknown | Bitplane modulo (odd planes). | `A.set_bpl1mod(value)` 1 P |
| `$10A` | — | `BPL2MOD` (Agnus) | WO | Not established | Unknown | Bitplane modulo (even planes). | `A.set_bpl2mod(value)` 1 P |
| `$10C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$10E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$110` | — | `BPL1DAT` & (Denise) | WO | Not established | Unknown | Bitplane 1 data (parallel-to-serial convert). | `D.set_bpldat(0, value)` 1 P |
| `$112` | — | `BPL2DAT` & (Denise) | WO | Not established | Unknown | Bitplane 2 data (parallel-to-serial convert). | `D.set_bpldat(1, value)` 1 P |
| `$114` | — | `BPL3DAT` & (Denise) | WO | Not established | Unknown | Bitplane 3 data (parallel-to-serial convert). | `D.set_bpldat(2, value)` 1 P |
| `$116` | — | `BPL4DAT` & (Denise) | WO | Not established | Unknown | Bitplane 4 data (parallel-to-serial convert). | `D.set_bpldat(3, value)` 1 P |
| `$118` | — | `BPL5DAT` & (Denise) | WO | Not established | Unknown | Bitplane 5 data (parallel-to-serial convert). | `D.set_bpldat(4, value)` 1 P |
| `$11A` | — | `BPL6DAT` & (Denise) | WO | Not established | Unknown | Bitplane 6 data (parallel-to-serial convert). | `D.set_bpldat(5, value)` 1 P |
| `$11C` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$11E` | — | — | Reserved | Not specified | Unknown | Appendix B labels this offset reserved. | Ignored write; no queue |
| `$120` | — | `SPR0PTH` + (Agnus) | WO | Not established | Unknown | Sprite 0 pointer (high 3 bits). | `A.set_sprpth(0, value)` 1 P |
| `$122` | — | `SPR0PTL` + (Agnus) | WO | Not established | Unknown | Sprite 0 pointer (low 15 bits). | `A.set_sprptl(0, value)` 1 P |
| `$124` | — | `SPR1PTH` + (Agnus) | WO | Not established | Unknown | Sprite 1 pointer (high 3 bits). | `A.set_sprpth(1, value)` 1 P |
| `$126` | — | `SPR1PTL` + (Agnus) | WO | Not established | Unknown | Sprite 1 pointer (low 15 bits). | `A.set_sprptl(1, value)` 1 P |
| `$128` | — | `SPR2PTH` + (Agnus) | WO | Not established | Unknown | Sprite 2 pointer (high 3 bits). | `A.set_sprpth(2, value)` 1 P |
| `$12A` | — | `SPR2PTL` + (Agnus) | WO | Not established | Unknown | Sprite 2 pointer (low 15 bits). | `A.set_sprptl(2, value)` 1 P |
| `$12C` | — | `SPR3PTH` + (Agnus) | WO | Not established | Unknown | Sprite 3 pointer (high 3 bits). | `A.set_sprpth(3, value)` 1 P |
| `$12E` | — | `SPR3PTL` + (Agnus) | WO | Not established | Unknown | Sprite 3 pointer (low 15 bits). | `A.set_sprptl(3, value)` 1 P |
| `$130` | — | `SPR4PTH` + (Agnus) | WO | Not established | Unknown | Sprite 4 pointer (high 3 bits). | `A.set_sprpth(4, value)` 1 P |
| `$132` | — | `SPR4PTL` + (Agnus) | WO | Not established | Unknown | Sprite 4 pointer (low 15 bits). | `A.set_sprptl(4, value)` 1 P |
| `$134` | — | `SPR5PTH` + (Agnus) | WO | Not established | Unknown | Sprite 5 pointer (high 3 bits). | `A.set_sprpth(5, value)` 1 P |
| `$136` | — | `SPR5PTL` + (Agnus) | WO | Not established | Unknown | Sprite 5 pointer (low 15 bits). | `A.set_sprptl(5, value)` 1 P |
| `$138` | — | `SPR6PTH` + (Agnus) | WO | Not established | Unknown | Sprite 6 pointer (high 3 bits). | `A.set_sprpth(6, value)` 1 P |
| `$13A` | — | `SPR6PTL` + (Agnus) | WO | Not established | Unknown | Sprite 6 pointer (low 15 bits). | `A.set_sprptl(6, value)` 1 P |
| `$13C` | — | `SPR7PTH` + (Agnus) | WO | Not established | Unknown | Sprite 7 pointer (high 3 bits). | `A.set_sprpth(7, value)` 1 P |
| `$13E` | — | `SPR7PTL` + (Agnus) | WO | Not established | Unknown | Sprite 7 pointer (low 15 bits). | `A.set_sprptl(7, value)` 1 P |
| `$140` | — | `SPR0POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 0 vertical-horizontal start position data. | `A.set_sprpos(0, value)` 1 P; `D.set_sprpos(0, value)` 1 P |
| `$142` | — | `SPR0CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 0 vertical stop position and control data. | `A.set_sprctl(0, value)` 1 P; `D.set_sprctl(0, value)` 1 P |
| `$144` | — | `SPR0DATA` % (Denise) | WO | Not established | Unknown | Sprite 0 image data register A. | `D.set_sprdata(0, value)` 1 P |
| `$146` | — | `SPR0DATB` % (Denise) | WO | Not established | Unknown | Sprite 0 image data register B. | `D.set_sprdatb(0, value)` 1 P |
| `$148` | — | `SPR1POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 1 vertical-horizontal start position data. | `A.set_sprpos(1, value)` 1 P; `D.set_sprpos(1, value)` 1 P |
| `$14A` | — | `SPR1CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 1 vertical stop position and control data. | `A.set_sprctl(1, value)` 1 P; `D.set_sprctl(1, value)` 1 P |
| `$14C` | — | `SPR1DATA` % (Denise) | WO | Not established | Unknown | Sprite 1 image data register A. | `D.set_sprdata(1, value)` 1 P |
| `$14E` | — | `SPR1DATB` % (Denise) | WO | Not established | Unknown | Sprite 1 image data register B. | `D.set_sprdatb(1, value)` 1 P |
| `$150` | — | `SPR2POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 2 vertical-horizontal start position data. | `A.set_sprpos(2, value)` 1 P; `D.set_sprpos(2, value)` 1 P |
| `$152` | — | `SPR2CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 2 vertical stop position and control data. | `A.set_sprctl(2, value)` 1 P; `D.set_sprctl(2, value)` 1 P |
| `$154` | — | `SPR2DATA` % (Denise) | WO | Not established | Unknown | Sprite 2 image data register A. | `D.set_sprdata(2, value)` 1 P |
| `$156` | — | `SPR2DATB` % (Denise) | WO | Not established | Unknown | Sprite 2 image data register B. | `D.set_sprdatb(2, value)` 1 P |
| `$158` | — | `SPR3POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 3 vertical-horizontal start position data. | `A.set_sprpos(3, value)` 1 P; `D.set_sprpos(3, value)` 1 P |
| `$15A` | — | `SPR3CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 3 vertical stop position and control data. | `A.set_sprctl(3, value)` 1 P; `D.set_sprctl(3, value)` 1 P |
| `$15C` | — | `SPR3DATA` % (Denise) | WO | Not established | Unknown | Sprite 3 image data register A. | `D.set_sprdata(3, value)` 1 P |
| `$15E` | — | `SPR3DATB` % (Denise) | WO | Not established | Unknown | Sprite 3 image data register B. | `D.set_sprdatb(3, value)` 1 P |
| `$160` | — | `SPR4POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 4 vertical-horizontal start position data. | `A.set_sprpos(4, value)` 1 P; `D.set_sprpos(4, value)` 1 P |
| `$162` | — | `SPR4CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 4 vertical stop position and control data. | `A.set_sprctl(4, value)` 1 P; `D.set_sprctl(4, value)` 1 P |
| `$164` | — | `SPR4DATA` % (Denise) | WO | Not established | Unknown | Sprite 4 image data register A. | `D.set_sprdata(4, value)` 1 P |
| `$166` | — | `SPR4DATB` % (Denise) | WO | Not established | Unknown | Sprite 4 image data register B. | `D.set_sprdatb(4, value)` 1 P |
| `$168` | — | `SPR5POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 5 vertical-horizontal start position data. | `A.set_sprpos(5, value)` 1 P; `D.set_sprpos(5, value)` 1 P |
| `$16A` | — | `SPR5CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 5 vertical stop position and control data. | `A.set_sprctl(5, value)` 1 P; `D.set_sprctl(5, value)` 1 P |
| `$16C` | — | `SPR5DATA` % (Denise) | WO | Not established | Unknown | Sprite 5 image data register A. | `D.set_sprdata(5, value)` 1 P |
| `$16E` | — | `SPR5DATB` % (Denise) | WO | Not established | Unknown | Sprite 5 image data register B. | `D.set_sprdatb(5, value)` 1 P |
| `$170` | — | `SPR6POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 6 vertical-horizontal start position data. | `A.set_sprpos(6, value)` 1 P; `D.set_sprpos(6, value)` 1 P |
| `$172` | — | `SPR6CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 6 vertical stop position and control data. | `A.set_sprctl(6, value)` 1 P; `D.set_sprctl(6, value)` 1 P |
| `$174` | — | `SPR6DATA` % (Denise) | WO | Not established | Unknown | Sprite 6 image data register A. | `D.set_sprdata(6, value)` 1 P |
| `$176` | — | `SPR6DATB` % (Denise) | WO | Not established | Unknown | Sprite 6 image data register B. | `D.set_sprdatb(6, value)` 1 P |
| `$178` | — | `SPR7POS` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 7 vertical-horizontal start position data. | `A.set_sprpos(7, value)` 1 P; `D.set_sprpos(7, value)` 1 P |
| `$17A` | — | `SPR7CTL` % (Agnus, Denise) | WO | Multiple chips listed | Unknown | Sprite 7 vertical stop position and control data. | `A.set_sprctl(7, value)` 1 P; `D.set_sprctl(7, value)` 1 P |
| `$17C` | — | `SPR7DATA` % (Denise) | WO | Not established | Unknown | Sprite 7 image data register A. | `D.set_sprdata(7, value)` 1 P |
| `$17E` | — | `SPR7DATB` % (Denise) | WO | Not established | Unknown | Sprite 7 image data register B. | `D.set_sprdatb(7, value)` 1 P |
| `$180` | — | `COLOR00` (Denise) | WO | Not established | Unknown | Color table 00. | `D.set_color(0, value)` 1 P |
| `$182` | — | `COLOR01` (Denise) | WO | Not established | Unknown | Color table 01. | `D.set_color(1, value)` 1 P |
| `$184` | — | `COLOR02` (Denise) | WO | Not established | Unknown | Color table 02. | `D.set_color(2, value)` 1 P |
| `$186` | — | `COLOR03` (Denise) | WO | Not established | Unknown | Color table 03. | `D.set_color(3, value)` 1 P |
| `$188` | — | `COLOR04` (Denise) | WO | Not established | Unknown | Color table 04. | `D.set_color(4, value)` 1 P |
| `$18A` | — | `COLOR05` (Denise) | WO | Not established | Unknown | Color table 05. | `D.set_color(5, value)` 1 P |
| `$18C` | — | `COLOR06` (Denise) | WO | Not established | Unknown | Color table 06. | `D.set_color(6, value)` 1 P |
| `$18E` | — | `COLOR07` (Denise) | WO | Not established | Unknown | Color table 07. | `D.set_color(7, value)` 1 P |
| `$190` | — | `COLOR08` (Denise) | WO | Not established | Unknown | Color table 08. | `D.set_color(8, value)` 1 P |
| `$192` | — | `COLOR09` (Denise) | WO | Not established | Unknown | Color table 09. | `D.set_color(9, value)` 1 P |
| `$194` | — | `COLOR10` (Denise) | WO | Not established | Unknown | Color table 10. | `D.set_color(10, value)` 1 P |
| `$196` | — | `COLOR11` (Denise) | WO | Not established | Unknown | Color table 11. | `D.set_color(11, value)` 1 P |
| `$198` | — | `COLOR12` (Denise) | WO | Not established | Unknown | Color table 12. | `D.set_color(12, value)` 1 P |
| `$19A` | — | `COLOR13` (Denise) | WO | Not established | Unknown | Color table 13. | `D.set_color(13, value)` 1 P |
| `$19C` | — | `COLOR14` (Denise) | WO | Not established | Unknown | Color table 14. | `D.set_color(14, value)` 1 P |
| `$19E` | — | `COLOR15` (Denise) | WO | Not established | Unknown | Color table 15. | `D.set_color(15, value)` 1 P |
| `$1A0` | — | `COLOR16` (Denise) | WO | Not established | Unknown | Color table 16. | `D.set_color(16, value)` 1 P |
| `$1A2` | — | `COLOR17` (Denise) | WO | Not established | Unknown | Color table 17. | `D.set_color(17, value)` 1 P |
| `$1A4` | — | `COLOR18` (Denise) | WO | Not established | Unknown | Color table 18. | `D.set_color(18, value)` 1 P |
| `$1A6` | — | `COLOR19` (Denise) | WO | Not established | Unknown | Color table 19. | `D.set_color(19, value)` 1 P |
| `$1A8` | — | `COLOR20` (Denise) | WO | Not established | Unknown | Color table 20. | `D.set_color(20, value)` 1 P |
| `$1AA` | — | `COLOR21` (Denise) | WO | Not established | Unknown | Color table 21. | `D.set_color(21, value)` 1 P |
| `$1AC` | — | `COLOR22` (Denise) | WO | Not established | Unknown | Color table 22. | `D.set_color(22, value)` 1 P |
| `$1AE` | — | `COLOR23` (Denise) | WO | Not established | Unknown | Color table 23. | `D.set_color(23, value)` 1 P |
| `$1B0` | — | `COLOR24` (Denise) | WO | Not established | Unknown | Color table 24. | `D.set_color(24, value)` 1 P |
| `$1B2` | — | `COLOR25` (Denise) | WO | Not established | Unknown | Color table 25. | `D.set_color(25, value)` 1 P |
| `$1B4` | — | `COLOR26` (Denise) | WO | Not established | Unknown | Color table 26. | `D.set_color(26, value)` 1 P |
| `$1B6` | — | `COLOR27` (Denise) | WO | Not established | Unknown | Color table 27. | `D.set_color(27, value)` 1 P |
| `$1B8` | — | `COLOR28` (Denise) | WO | Not established | Unknown | Color table 28. | `D.set_color(28, value)` 1 P |
| `$1BA` | — | `COLOR29` (Denise) | WO | Not established | Unknown | Color table 29. | `D.set_color(29, value)` 1 P |
| `$1BC` | — | `COLOR30` (Denise) | WO | Not established | Unknown | Color table 30. | `D.set_color(30, value)` 1 P |
| `$1BE` | — | `COLOR31` (Denise) | WO | Not established | Unknown | Color table 31. | `D.set_color(31, value)` 1 P |
| `$1C0` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1C2` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1C4` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1C6` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1C8` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1CA` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1CC` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1CE` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1D0` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1D2` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1D4` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1D6` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1D8` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1DA` | — | — | Unlisted | Not specified | Unknown | No entry in Appendix B; its status is not stated there. | Ignored write; no queue |
| `$1DC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1DE` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1E0` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1E2` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1E4` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1E6` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1E8` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1EA` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1EC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1EE` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1F0` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1F2` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1F4` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1F6` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1F8` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1FA` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1FC` | — | — | Reserved | Not specified | Unknown | Appendix B labels the enclosing address range reserved. | Ignored write; no queue |
| `$1FE` | — | — | Unspecified | Not specified | Unknown | `NO-OP(NULL)`: No-operation; Appendix B gives no access direction or chip. | Ignored write; no queue |

## Owner-delivery evidence

The register-delivery boundary is exercised through production interfaces:

| Evidence | Established boundary and limits |
| --- | --- |
| [`test_register_wiring.rs`](../../../crates/memory_bus/tests/test_register_wiring.rs), `every_ocs_write_reaches_only_its_owners_and_is_delivered_once_when_due` | All 256 aligned slots, including 174 queued data-write offsets and six immediate address strobes; word and both byte writes on PAL and NTSC (1,536 transfers). Checks every recipient's offset, payload, mode and remaining delay, no early due event, exactly one due event and no later duplicate. Uses real router and owner steps, without claiming later engine behavior. |
| Same suite, `custom_live_and_debug_reads_select_the_owner_or_explicit_fallback` | All 256 slots use the intended reader or fallback; distinct readable data detects wrong selection. Debug traversal preserves complete chip/queue state. Existing dedicated tests establish live collision/disk acknowledgment and Copper read strobes. This is reader selection, not validation of all computed hardware values. |
| [`test_router.rs`](../../../crates/memory_bus/tests/test_router.rs) | All thirty canonical CIA addresses, selected instance, selector and payload, synchronization before acceptance and falling-edge capture. Five initial E phases, inactive lanes, low-bit mirrors, byte/word lanes and OVL mapping have existing regression protection. Gary alias/dual-selection tests cover all banks and memory presets on PAL/NTSC; `test_gary_decoding.rs` exercises CPU transfers and committed OVL through the machine. |
| [`test_custom_dispatch.rs`](../../../crates/machine_loop/tests/test_custom_dispatch.rs) | Production-machine CPU/Copper issue boundaries, shared rejection/retry, queue order and complete replay, strobe delivery, interrupt-input versus CPU ordering, DMA competition and pending-state serialization. |
| [`test_cia_propagation.rs`](../../../crates/machine_loop/tests/test_cia_propagation.rs) and [`test_register_propagation.rs`](../../../crates/machine_loop/tests/test_register_propagation.rs) | CPU CIA retries/read acknowledgment, pending capture/output restore, reset cancellation, and committed CIA-A PRA/DDRA polarity and overlay mapping using synthetic ROM bytes. No firmware boot is required. |

Task `REGISTER-ACCESS-CLOSE-20261111` records the documentation-and-delivery
closure in [DIARY.md](../../../DIARY.md). Numerical delays remain configured
model values; owner delivery does not certify engine or silicon behavior.

## Reference Documentation & Upstream Ground Truth

- [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Address order, names, access symbols, chip listings, and concise functions used for every row.
- [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Opposite-direction access caution, collision read-and-clear behavior, Copper jump strobes, and Agnus refresh strobes.
- [MemoryBus](MemoryBus.md): Shared selection, acknowledgment, queue ordering, lane and issue-boundary contract.
- [Agnus register application](../../../crates/agnus/src/register_apply.rs), [Denise access](../../../crates/denise/src/registers.rs) and [Paula access](../../../crates/paula/src/registers.rs): Living private application and read paths behind the recorded interfaces.
- [Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md): Demonstrates a CPU read of `COPJMP1` that reloads the Copper program counter.
