---
title: "Amiga 500 Floppy Subsystem"
aliases: ["Floppy", "Floppy Drive"]
tags: ["amiga", "design", "floppy", "disk"]
category: "Design"
subsystem: "floppy"
status: "active"
created: 2026-09-06
updated: 2026-11-15
related: ["[CIA.md](CIA.md)", "[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/floppy"
---

# Amiga 500 Floppy Subsystem

A floppy drive reads and writes removable magnetic disks. In the emulator, the [floppy crate](../../../crates/floppy/src/floppy.rs) represents the drives and integrates their behavior with disk-related registers owned by several chips. It does not duplicate those registers.

[CIA-B](CIA.md) provides drive control, [CIA-A](CIA.md) receives drive status, and [Paula](Paula.md) handles disk data and control registers. [Agnus](Agnus.md) owns disk DMA addresses and bus arbitration. Register accesses and signals that cross component boundaries are routed through the [memory bus](MemoryBus.md) and [machine loop](Machine%20Loop.md); the floppy component does not directly mutate peer chips.

## From a disk track to Chip RAM

The drive supplies a serial stream of magnetic transitions from the selected
track. In MFM encoding, each data bit is accompanied by a clock bit chosen
from the preceding data bit, so the raw track stream is larger than the sector
data it represents. The disk circuitry in Paula recovers the timed bitstream,
groups it into words, and can recognize a chosen synchronization pattern. In the usual
read path, the hardware transfers the **raw MFM-encoded words** into a Chip
RAM buffer. It does not turn the stream into AmigaDOS sectors. Software on
the CPU interprets the buffered encoding, locates sectors, and checks their
contents after the transfer; software may use the Blitter to help process
the encoded data.

For each eligible disk DMA transfer, Paula supplies a ready word and
[Agnus](Agnus.md) grants a bus opportunity, generates the Chip RAM address,
and advances the transfer position after the word is written. The configured
word count ends the block and can raise a completion interrupt. A matching
sync pattern can separately signal the CPU and, when synchronization is
enabled, determine when read DMA begins. For writing a track, the direction
reverses: Agnus fetches raw encoded words from Chip RAM and Paula sends them
to the drive as a serial stream. [DMA](DMA.md) describes the common grant and
transfer path; [CIA](CIA.md) describes the drive-control and status pins.

## Drive control and deterministic connector boundary

The serializable `FloppyPins` state in [pins.rs](../../../crates/floppy/src/pins.rs)
is exposed as `floppy.pins`. It groups the raw data, write gate and INDEX
levels separately from drive mechanics and host-supplied image buffers.

CIA-B committed PB7/SEL3-0/PB2/PB1/PB0 drive motor, selection, side, direction
and STEP. The motherboard captures effective pins, including timer PB6/PB7 overrides, and
stores them in `pins.incoming`. The controller samples that state into
`pins.control_input` before engine work, so
an input-configured bit releases high. The controller's retained previous pin
levels distinguish a selection/step edge from a held level. Drive ready,
track-zero, write-protect and change sense return on CIA-A PA5-PA2, separately
from data bytes and DMA. MOTOR latches the established level on selection,
STEP falling edges move a selected head in the delivered direction, and SIDE
selects its surface. Selected sense outputs resolve low dominant. Control
delivery and the resulting status return are separate captured hops. HRM Chapter 8 Table 8-5 requires MOTOR to be
established before the selection edge that latches it, using separate writes;
the transport regressions preserve that setup sequence.

`pins.index_high` is the drive's active-low INDEX output and reaches CIA-B FLAG;
it is not serial CD and is not synthesized by a fixed arbitrary scanline count.
`pins.read_data_high` reaches Paula's raw disk input. Paula's exposed write data and
asserted write-enable reach `pins.write_data_high` and active-low
`pins.write_enable_high` after Gary/buffer routing. The [machine phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
captures those levels and delivers them before the next receiver step. S4/S7
in the [signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
establish the buffering/polarity; electrical rise times remain unknown.

The controller serializes sampled connector levels and its own incoming inputs.
Electronic reset cancels pending edges, releases the write gate and stops motors.
It retains mounted media, write protection, disk-change state, head cylinder and
angular position; insertion/ejection are separate host operations. Cold, warm and
guest RESET use that same drive contract.

## Autonomous ADF reads and continuation

Insertion accepts only a standard 901,120-byte DD ADF and prepares all 160 raw
tracks before execution. Invalid size leaves the current medium unchanged.
Each sector has four preamble bytes, two $4489 sync words, odd/even header and
label data, masked XOR checksums and odd/even payload. Clock bits carry the
preceding data bit across boundaries. Eleven 1,088-byte sectors and a 532-byte
gap form a circular 12,500-byte track. This is an ideal 300 RPM, 500,000 raw-cell
per second read model; the HRM FAST setting specifies two microseconds per cell.

The [media engine](../../../crates/floppy/src/media.rs) advances every running
spindle, even when deselected. Its rational phase accumulator uses the configured
PAL/NTSC CCK frequency, retaining the fraction rather than rounding every cell to
seven CCK. Selection, side changes and STEP do not restart angular phase.
Selected transitions and INDEX resolve low dominant at the connector. Pulses
are one-CCK sampled event markers, not calibrated analog widths. Spindle
acceleration, head settling and magnetic write behavior are not modeled.

Snapshots embed usable raw tracks and retain spindle phase, mechanics, controls
and pending pins. They require no host image file to resume. [Paula](Paula.md#reconstructed-disk-engine-contract)
owns separation, FIFO, synchronization and write serialization; the drive never
interprets sectors during execution. Write-data/gate routing remains intact,
but neither media writes nor host ADF persistence are established by read boot.

The [autonomous DMA tests](../../../crates/machine_loop/tests/test_floppy_media_dma.rs)
check exact successive raw words, track wrap, CPU contention, reset and serialized
mid-transfer continuation. [Native boot tests](../../../crates/machine_loop/tests/test_native_disk_boot.rs)
use the original cross1 bootblock and Kickstart startup. Invocation and evidence
limits are in the [machine test guide](../../../crates/machine_loop/tests/README.md).
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
