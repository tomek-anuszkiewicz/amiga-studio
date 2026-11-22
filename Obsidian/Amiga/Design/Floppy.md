---
title: "Amiga 500 Floppy Subsystem"
aliases: ["Floppy", "Floppy Drive"]
tags: ["amiga", "design", "floppy", "disk"]
category: "Design"
subsystem: "floppy"
status: "active"
created: 2026-09-06
updated: 2026-11-17
related: ["[CIA.md](CIA.md)", "[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/floppy"
---

# Amiga 500 Floppy Subsystem

A floppy drive reads and writes removable magnetic disks. The emulator's
drive model works with the chips that own the disk registers and keeps no
copies of those registers. See the
[floppy crate](../../../crates/floppy/src/floppy.rs).

[CIA-B](CIA.md) controls the drive, [CIA-A](CIA.md) receives its status, and
[Paula](Paula.md) owns disk data and control registers. [Agnus](Agnus.md) owns
the disk DMA addresses and grants access to the bus. The
[memory bus](MemoryBus.md) and [machine loop](Machine%20Loop.md) route register
accesses and signals between components. The floppy component makes no direct
changes to other chips.

## From a disk track to Chip RAM

The drive supplies a serial stream of magnetic transitions from the selected
track. In MFM encoding, each data bit has an accompanying clock bit chosen
from the preceding data bit. That clock information makes the raw track
stream larger than the sector data it represents. Paula's disk circuitry
recovers the timed bitstream, groups it into words and can detect a selected
synchronization pattern. A normal disk read transfers the
**raw MFM-encoded words** into a Chip RAM buffer. The hardware leaves the
stream encoded. Once the transfer finishes, software on the CPU interprets
the encoding, locates the AmigaDOS sectors and checks their contents. It may
use the Blitter to help process the encoded data.

For each eligible disk DMA transfer, Paula supplies a ready word and
[Agnus](Agnus.md) grants a bus opportunity and generates the Chip RAM address.
After the write, Agnus advances the transfer position. The transfer ends
when it reaches the configured word count and can raise a completion
interrupt at that point.
A matching sync pattern can signal the CPU separately. If synchronization
is enabled, the match can also determine when read DMA starts. When writing
a track, Agnus fetches raw encoded words from Chip RAM and Paula sends them
to the drive as a serial stream. [DMA](DMA.md) describes the shared grant and
transfer process. [CIA](CIA.md) describes the drive-control and status pins.

## Drive control and deterministic connector boundary

The serializable `FloppyPins` state in [pins.rs](../../../crates/floppy/src/pins.rs)
is exposed as `floppy.pins`. It groups the raw data, write gate and INDEX
levels separately from drive mechanics and host-supplied image buffers.

CIA-B's committed PB7/SEL3-0/PB2/PB1/PB0 levels control the motor, drive
selection, side, direction and STEP. The motherboard captures the effective
pin levels, including timer PB6/PB7 overrides, in `pins.incoming`. Before
engine work, the controller samples them into `pins.control_input`. A bit
configured as input releases high. The controller keeps the previous pin
levels so it can distinguish a selection or step edge from a held level.
Drive ready, track-zero, write-protect and change sense return on CIA-A
PA5-PA2, separately from data bytes and DMA. On selection, the drive latches the MOTOR level already on the connector.
A falling STEP edge moves the selected drive's head in the delivered
direction. SIDE selects the surface. A low level from a
selected sense output takes precedence over a high level. The motherboard
captures control delivery and the resulting status return as separate hops.
HRM Chapter 8 Table 8-5 requires separate writes to establish MOTOR before
the selection edge that latches it. The transport regression tests preserve
that setup sequence.

`pins.index_high` carries the drive's active-low INDEX output to CIA-B FLAG.
The model keeps it separate from serial CD and does not generate it from a
fixed arbitrary scanline count. `pins.read_data_high` reaches Paula's raw
disk input. Gary and the buffers route Paula's write data and asserted write
enable to `pins.write_data_high` and active-low `pins.write_enable_high`.
The [machine phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
captures those levels and delivers them before the next receiver step.
S4/S7 in the [signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
establish the buffering and polarity. Electrical rise times remain unknown.

The controller serializes sampled connector levels and its own incoming inputs.
Electronic reset cancels pending edges, releases the write gate and stops motors.
It retains mounted media, write protection, disk-change state, head cylinder
and angular position. The host handles insertion and ejection separately. Cold, warm and
guest RESET use that same drive contract.

## Physical drive attachment and discovery

On the physical drive, `DSKMOTOR*` also controls the front activity light
([HRM Chapter 8, Table 8-5](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md#table-8-5-disk-subsystem)).
The [GUI activity indicator](Application/GUI.md) observes effective CIA-B control
events and consumed Paula transfers. Selection with a latched running motor
keeps it active; deselected rotation alone does not. This presentation metadata
belongs outside the drive mechanics and does not change the physical LED contract.

DF0 is the internal drive. DF1 through DF3 can be connected with or without
media. A disconnected drive supplies no data, INDEX or sense output. A
connected external DD drive identifies itself even when empty. To read its
ID, software first initializes it with motor-on/off, then performs 32
select/read/deselect cycles on READY, most significant bit first. The
inverted stream is `$FFFFFFFF` for a standard Amiga DD drive and `$00000000`
when no drive is connected. See the [Technical Reference Manual, external disk identification](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md#external-disk-connector-identification-mode).
All bits of the single supported external device ID are identical. When a
connected drive is selected with the motor off, it asserts READY whether or
not media is present, so the model needs no shifting state for that ID. With
the motor on, READY still reflects media and speed. The model leaves spindle
acceleration unmodeled. Attachments persist in snapshots and through reset.

## Autonomous ADF reads and continuation

Insertion accepts only a standard 901,120-byte DD ADF and prepares all 160 raw
tracks before execution. Invalid size leaves the current medium unchanged.
Each sector has four preamble bytes, two $4489 sync words, odd/even header and
label data, masked XOR checksums and odd/even payload. Clock bits carry the
preceding data bit across boundaries. Eleven 1,088-byte sectors and a 532-byte
gap form a circular 12,500-byte track. The read model assumes an ideal 300 RPM and 500,000 raw cells per second.
The HRM FAST setting specifies two microseconds per cell.

The [media engine](../../../crates/floppy/src/media.rs) advances every
running spindle, even if the drive is deselected. The engine uses a rational phase accumulator to convert the configured
PAL/NTSC CCK frequency into cell timing. It retains the fractional remainder
across cells, avoiding a fixed rounding to seven CCK per cell. Selection,
side changes and STEP let the current angular phase continue. At the connector, a low level takes
precedence for selected transitions and INDEX. Each pulse lasts one CCK and
marks a sampled event without representing a calibrated analog width. The
model leaves spindle acceleration, head settling and magnetic writing unmodeled.

A snapshot embeds usable raw tracks and saves spindle phase, mechanics,
controls and pending pins. It can resume without the host image file.
[Paula](Paula.md#reconstructed-disk-engine-contract) owns data separation, the
FIFO, synchronization and write serialization. The drive does not interpret
sectors during execution. Write-data and gate routing remain in place. Read
boot evidence leaves writes to media and persistence back to the host ADF
unverified.

The [autonomous DMA tests](../../../crates/machine_loop/tests/test_floppy_media_dma.rs)
check exact successive raw words, track wrap, CPU contention, reset and
continuation from serialized state mid-transfer. The
[native boot tests](../../../crates/machine_loop/tests/test_native_disk_boot.rs)
use the original cross1 bootblock and Kickstart startup. The
[verification reference](../../../tools/verification/README.md#native-disk-boot-checks)
describes how to invoke the tests and what their results establish.
Full image comparisons and broader guest coverage remain in
[native verification](../../../ROADMAP.md#native-disk-verification).

## References

- [Signal comparison](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#three-reference-comparison-and-selected-transport) and [Machine Loop transport](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words): Define captured connector ordering independently of drive mechanics.
- [Commodore disk format](https://amigadev.elowar.com/read/ADCD_2.1/Devices_Manual_guide/node015B.html): Original sector layout and odd/even organization.
- [vAmiga sector encoder](../../../ref_src/vAmiga/Core/RetroVault/Images/Encoders/AmigaEncoder.cpp): Independent format/checksum and boundary-clock comparison; its longer default track is not the selected 300 RPM model.

- [CIA.md](CIA.md): Drive control and status register ownership.
- [Paula.md](Paula.md): Disk data and control register ownership.
- [Agnus.md](Agnus.md): Disk DMA address generation and arbitration.
- [MemoryBus.md](MemoryBus.md) and [Machine Loop.md](Machine%20Loop.md): Register access and signal routing.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Describes drive controls, raw MFM, synchronization, bidirectional disk DMA, and completion.
- [Floppy implementation](../../../crates/floppy/src/floppy.rs): Drive state and integration behavior.
- [`floppy/pins.rs`](../../../crates/floppy/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
