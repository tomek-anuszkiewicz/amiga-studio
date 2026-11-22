---
title: "Custom Chip Register Ownership and Access Matrix"
aliases: ["Custom Chip Register Matrix", "Custom Register Map", "Hardware Register Catalog", "Chipset Register Reference"]
tags: ["amiga", "design", "registers", "agnus", "denise", "paula", "memory-map"]
category: "Design"
subsystem: "memory_bus"
status: "active"
created: 2026-09-14
updated: 2026-11-16
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
---

# Custom Chip Register Ownership and Access Matrix

To find a 68000 address in the original custom-chip register block, add the
three-digit offset in the address map to `$DFF000`. The map shows the register
names, access directions, chips and brief functions documented in the manual.
[Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md)

The manual also defines the register fields and describes access cautions and
exceptions. [Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md)

The accompanying text explains how the current emulator delivers each access to
its owner and identifies functional effects outside the model.

## Reading the matrix

- `RO` and `WO` give the manual's documented read or write direction. `—`
  means the manual gives no **documented safe counterpart** at that offset.
  Appendix A warns that reading a write-only register can disturb it and
  writing to a read-only register can have unexpected results.
- `ER` marks an early DMA read initiated by Agnus timing and distinguishes it
  from an ordinary CPU register read. `COR` marks `CLXDAT`, whose documented
  read also clears collision state. `STROBE` marks an address-triggered action.
  The strobe itself stores no register bits. A CPU read or write of `COPJMP1`
  or `COPJMP2` can trigger a Copper jump. The manual describes `STREQU`,
  `STRVBL`, `STRHOR` and `STRLONG` as write-address strobes that Agnus uses
  during refresh. It gives no safe strobe semantics for CPU reads of those
  addresses. A documented access direction gives no guarantee about the effects
  of using the other direction.
- The suffix `&` marks DMA-only use. The suffix `%` marks a register
  usually used by DMA and sometimes by the processor. The suffix `+` marks a
  chip-memory pointer pair. When an Appendix B address starts with `*`, the
  Copper cannot write to it. When it starts with `-`, the Copper can write to
  it only with `COPCON.CDANG` set. The table puts these markers in the notes
  to keep the offset numeric.
- The `Scope` column says `Multiple chips listed` when Appendix B names more
  than one chip. When it names one chip, the scope is `Not established`
  because effects may extend beyond that chip. The `Delay` column says
  `Unknown` because the appendices do not generally specify how many CCKs
  each register takes to propagate. Do not treat that label as a delay of zero.
- The chip listings reproduce the manual. Use an explicit design note in the
  row to decide whether a listed chip needs to act on a write in the emulator.
- `Reserved` reproduces an explicit Appendix B label. `Unlisted` applies
  only to `$1C0–$1DA`, which the address-order table skips without labeling.
  Both classifications leave open-bus values and other silicon behavior
  unspecified. Appendix A advises against accessing unused registers.

## Modeled interfaces and timing

The map covers the supported PAL 8371 and NTSC 8370 Agnus, with OCS 8362 Denise
and 8364 Paula. ECS/AGA registers are outside its scope. The current MemoryBus
map selects bank `$DF` and uses the low nine address bits to mirror register
offsets across the bank. A0 selects the byte lane within the register for a
byte read. A word access uses one aligned offset. On a byte write to either
lane, the router copies the byte into both halves of the word.
[MemoryBus map](MemoryBus.md#a-cpu-access-through-the-router)

The router acknowledges writes to reserved or unlisted offsets without queuing
an event. Dummy early reads, reads without a modeled response and `NO-OP(NULL)`
return the fixed `$FFFF` placeholder. These emulator responses leave the safety
of the corresponding hardware access unverified. MemoryBus also describes Gary
bank selection, CIA aliases, dual selection and lane behavior.
[MemoryBus](MemoryBus.md#cia-and-boot-overlay)

The production router dispatches accesses through
[`custom_registers.rs`](../../../crates/memory_bus/src/custom_registers.rs).
The MemoryBus specification defines each recipient's delay, Pipeline mode,
how a write reaches multiple recipients as one operation, and when the machine
issues an access. These rules are the user-approved emulator model policy of
2026-11-11. Hardware propagation measurements fall outside the policy's scope.
An address strobe acts as soon as the router accepts its selection, without a
queue entry. An engine may consume a committed data write later. Agnus handles
register delivery for its Copper and Blitter engines.
[MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips)

The production router dispatches both CPU writes and legal Copper MOVE writes.
Before the Copper emits its retained request, it checks the destination under
the manual's rules. Offsets below `$040` are prohibited. Offsets `$040–$07E`
require `COPCON.CDANG`, and the Copper can use offsets from `$080` upward.
Direct CPU writes are exempt from these Copper restrictions.

Arbitration retains requests while they wait. If a receiver queue would
overflow, the owner stops before delivering any part of the write. Reset
cancels unaccepted Copper output and queued events. Changes to DMA enables
control whether the engine can run. Writes the router has already accepted
still reach their recipients. [Copper](Copper.md)

The CPU performs live reads in CCK1 and writes in CCK2. The chips that own the
registers take their steps before the CPU issues an access. The Copper issues
requests after all custom-chip steps. Consequently, recipients of the same
accepted access can apply it on different later steps. That difference alone
leaves the existence of a second motherboard delay unverified.

### State owners and hardware meaning

Each register family below corresponds to individual addresses in the map.
The owning specifications and Appendix A sections define the register fields
and intended hardware effects. Consult the linked source for implementation
details.

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

For registers with a reader, `read_custom_word` calls the lowercase
register-named reader on the chip in the read column. Two entries use
different calls. `DSKBYTR` uses `Paula::read_dskbytr` and `COPJMP1/2` use
`Agnus::strobe_copjmp1/2`. `DMACONR` reads Agnus alone.
`BLTDDAT/DSKDATR` return the placeholder.

Debug dispatch uses the corresponding `_debug` reader. For `DSKBYTR`, it
uses `peek_dskbytr`. A debug COPJMP read returns `$FFFF` and leaves the strobe
inactive. The live CLXDAT and DSKBYTR readers acknowledge the access themselves.
A live COPJMP read triggers the jump strobe immediately, without adding a
write queue entry. Other addresses use the fallback. Silicon disturbances
from opposite-direction accesses are outside the emulator model.

The owning chip specifications describe the reconstructed register effects.
The native disk verification checks the observable effects exercised by
selected programs. A full account of register accesses and observable effects
requires broader coverage. [Native disk verification](../../../ROADMAP.md#native-disk-verification)

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
| `$096` | — | `DMACON` (Agnus, Denise, Paula) | WO | Multiple chips listed | Unknown | DMA control write (clear or set). Agnus latches the DMA enables; Paula reads the write value from the shared register bus. Denise takes no action. See [DMA](DMA.md). |
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

## Owner-delivery evidence

The tests below use production interfaces to exercise register delivery.

| Evidence | Established boundary and limits |
| --- | --- |
| [`test_register_wiring.rs`](../../../crates/memory_bus/tests/test_register_wiring.rs), `every_ocs_write_reaches_only_its_owners_and_is_delivered_once_when_due` | All 256 aligned slots, including 174 queued data-write offsets and six immediate address strobes; word and both byte writes on PAL and NTSC (1,536 transfers). Checks every recipient's offset, payload, mode and remaining delay, no early due event, exactly one due event and no later duplicate. Uses real router and owner steps, without claiming later engine behavior. |
| Same suite, `custom_live_and_debug_reads_select_the_owner_or_explicit_fallback` | All 256 slots use the intended reader or fallback; distinct readable data detects wrong selection. Debug traversal preserves complete chip/queue state. Existing dedicated tests establish live collision/disk acknowledgment and Copper read strobes. This is reader selection, not validation of all computed hardware values. |
| [`test_router.rs`](../../../crates/memory_bus/tests/test_router.rs) | All thirty canonical CIA addresses, selected instance, selector and payload, synchronization before acceptance and falling-edge capture. Five initial E phases, inactive lanes, low-bit mirrors, byte/word lanes and OVL mapping have existing regression protection. Gary alias/dual-selection tests cover all banks and memory presets on PAL/NTSC; `test_gary_decoding.rs` exercises CPU transfers and committed OVL through the machine. |
| [`test_custom_dispatch.rs`](../../../crates/machine_loop/tests/test_custom_dispatch.rs) | Production-machine CPU/Copper issue boundaries, fatal shared overflow atomicity, queue order and snapshot continuation, strobe delivery, interrupt-input versus CPU ordering, DMA competition and pending-state serialization. |
| [`test_cia_propagation.rs`](../../../crates/machine_loop/tests/test_cia_propagation.rs) and [`test_register_propagation.rs`](../../../crates/machine_loop/tests/test_register_propagation.rs) | CPU CIA retries/read acknowledgment, pending capture/output restore, reset cancellation, and committed CIA-A PRA/DDRA polarity and overlay mapping using synthetic ROM bytes. No firmware boot is required. |

Task `REGISTER-ACCESS-CLOSE-20261111` records the completed documentation and
register-delivery work. The emulator uses the configured numerical delays.
The tests establish delivery to the owner. The effects in the engine and on
silicon require separate evidence. [DIARY.md](../../../DIARY.md)

## Reference Documentation & Upstream Ground Truth

- The address order, names, access symbols, chip listings and brief functions in every row come from [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md).
- The manual describes cautions for opposite-direction access, collision read-and-clear behavior, Copper jump strobes and Agnus refresh strobes. [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md)
- The shared contract covers selection, acknowledgment, queue order, byte lanes and when an access issues. [MemoryBus](MemoryBus.md)
- The current private implementations apply writes and handle reads behind the documented interfaces. [Agnus register application](../../../crates/agnus/src/register_apply.rs), [Denise access](../../../crates/denise/src/registers.rs), [Paula access](../../../crates/paula/src/registers.rs)
- The manual demonstrates how a CPU read of `COPJMP1` reloads the Copper program counter. [Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md)
