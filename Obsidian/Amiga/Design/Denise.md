---
title: "Denise"
aliases: ["MOS 8362", "MOS 8373", "Video", "Bitplanes"]
tags: ["amiga", "design", "denise", "video", "display"]
category: "Design"
subsystem: "denise"
status: "active"
created: 2026-09-06
updated: 2026-11-23
related: ["[Sprites.md](Sprites.md)", "[Frame Buffer.md](Frame%20Buffer.md)", "[Agnus.md](Agnus.md)", "[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/denise"
---
# Denise

Denise is the Amiga display chip. [Agnus](Agnus.md) delivers bitplane and
sprite data over the custom-chip bus. Denise turns that data into pixels,
combines the layers using the display controls and color palette, and records
collisions. Denise also exposes controller input state. The input connections
are described in [Game Ports](Game%20Ports.md).

## From Chip RAM to a pixel

A playfield is stored as separate bitplanes in Chip RAM. Each plane contains
one bit for each pixel position. [Agnus](Agnus.md) fetches words from the
planes during the line's data-fetch interval and supplies them to Denise.
For each pixel position, Denise shifts one bit from each plane. Those bits
together form an index into the color palette. Each word supplies the bits
for sixteen successive pixel positions in one plane. The plane pointers advance with each
fetch and skip the unused portion of a larger image before the next line.
This lets memory hold an image wider than the part shown on screen.

The data-fetch interval determines which memory words become available. Agnus
uses the vertical fields of `DIWSTRT` and `DIWSTOP` to decide which lines can
use bitplane DMA. Denise uses the horizontal fields to clip playfield pixels
within each line. Fetching more data can support scrolling or a wider image.
The horizontal window still determines what is visible. Denise combines the
playfield with [hardware sprites](Sprites.md) according to display priority
and records collisions.

## Amiga 500 display modes

In an ordinary single playfield, up to five planes combine into a palette
index, allowing up to 32 simultaneous selected colors. Low and high horizontal resolution determine how quickly Denise emits pixels.
Interlace changes the vertical pattern of the fields. High resolution has less time to fetch each line's
data and supports fewer planes.

Dual-playfield mode groups odd and even planes into two independently
positioned pictures. Each has transparent pixels, and display priority decides
which picture is visible where they overlap. In hold-and-modify (HAM) mode,
some pixel codes choose a palette color while others change one red, green,
or blue component of the preceding pixel's color. The
[Hardware Reference Manual](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md#hold-and-modify-mode)
describes these modes and specifies five or six bitplanes for HAM.

## Reconstructed pixel processing

The BPLxDAT entry in HRM Appendix A defines BPL1DAT as the trigger for
converting parallel data to a serial stream of bits. When a BPL1DAT write
commits, Denise loads all six data latches into the shifters. A new write to that register is required to load another word. The unchanged
latches do not reload on their own. High resolution shifts four bits per CCK. Low resolution shifts two
and uses two samples per pixel on the shared high-resolution framebuffer
grid. BPLCON1 delays the odd and even plane streams independently by 0-15
pixel periods and keeps their history across adjacent words. Mid-line register
writes still take effect at their commit phase.

In the emulator, the horizontal counter wrap clears the flag that enables
bitplane output for the next line. Denise continues shifting pending bitplane
data at the start of the new count. While vertical blanking is inactive, the
pending pixels finish the previous visible row. If vertical blanking starts
at the field boundary, the shifters still advance and Denise skips the
remaining pixel writes. The frame builder clears the unwritten row tail.
[Frame Buffer](Frame%20Buffer.md#field-assembly-and-snapshots)

A `BPL1DAT` write that commits between the wrap and horizontal blanking can
enable output for the new line. At the start of horizontal blanking, the
emulator initializes the held HAM color from `COLOR00` as part of its display
timing model. The hardware manual specifies that no data is displayed during
vertical blanking.
[Hardware Reference Manual, Chapter 3](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md#maximum-display-window-size)

Denise decodes the current pixel using the committed mode, whether palette,
dual-playfield, HAM6 or Extra Half-Brite. HAM modifies one component of the
preceding color. EHB halves each four-bit component separately. Sprites
advance once per low-resolution pixel regardless of bitplane resolution. The
horizontal window counter phase, blanking region and two-high-resolution-pixel
output delay remain properties of the model, with no claim of calibrated
silicon edge offsets.

The horizontal DIW comparisons preserve all eight programmed bits, including
H0. OCS supplies H8=0 for start and H8=1 for stop (HRM Appendix A). If the
programmed start is even, forcing an odd start discards one low-resolution
period, or two hires samples. The window clips serialized pixels at the
delayed output phase, and the shifter continues to advance. The receiver's
two-period DIW counter origin and the output pair's one-low-resolution-period
delay are separate model phases. Denise compares the restart value before
advancing to delayed output, so a programmed start of 2 still opens the
window. A comparison at the current composition phase without the output
delay would clip the first two hires samples of the ordinary Workbench
window. The counter origin agrees with the [ARP Denise reconstruction](../../../ref_src/amiga_replacement_project/denise/src/denise.v).
Its exact relationship to this emulator's CCK delivery remains a model choice.

In the emulator, HAM and Extra Half-Brite each require six low-resolution
planes. Five-plane HAM remains outside the model. BPU=7 remains a distinct
decoding case. Pending BPLCON0 writes must not affect the active mode.

The renderer advances that two-sample output pipeline as a pair. Low resolution
composes one color for both new samples; high resolution composes each sample
separately. The previous pair still leaves the pipeline at the same phase,
including across mode changes and snapshot restoration.
During blanking, Denise skips framebuffer writes and lets the pipeline
continue with its composed colors. The first visible pair therefore retains
the preceding border color.

HRM 7 Tables 7-1/7-2 show how the two playfields rank in priority among sprite
pairs. A normal playfield uses PF2P. In dual mode, each nontransparent
playfield supplies its own priority condition and PF2PRI chooses between the
playfield colors. Before selecting which color to display, collision logic
uses all participating sprite pairs and the serialized plane bits with
inactive planes masked by BPU. CLXCON controls which planes take part and
which bit values match. It treats the odd and even groups separately,
regardless of the display mode. A group always matches if every plane in it
is excluded. CLXDAT accumulates all documented pair and plane collisions. A
live read clears it. Debug reads have no effect.

The [sprite line buffer](Sprites.md#derived-state-and-pixel-evaluation) supplies
potential collision bits along with its palette index. When a CLXCON write
takes effect, Denise precomputes the allowed playfield collision bits for
all 64 plane codes. If the effective value is identical, Denise keeps the
existing table. The write still follows the normal register-commit phase
and preserves accumulated collisions. For each visible pixel, Denise combines
the table entry with the sprites currently present. Precomputing the table
does not add to CLXDAT or select a future palette color.

## Register propagation

The queued setters in [registers.rs](../../../crates/denise/src/registers.rs)
deliver controls, palette, bitplane data and sprite latches with
**1 CCK Pipeline** timing. Accepted address strobes act immediately, with no
queue entry. CPU and DMA BPLxDAT writes use the same private operation to
apply a latch value. Committing BPL1DAT transfers the holding latches and
arms serialization. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips)
defines the common issue boundary. The fixed 64-entry queue holds more than
the CPU/Copper writes and one bitplane DMA word per CCK that can arrive
during its one-CCK delay. Denise applies each due event once. Shifter
movement and port input sampling remain separate operations.

JOYTEST updates both counter pairs as described under [Digital controller counters](#digital-controller-counters).
Live `CLXDAT` reads clear collision state; `clxdat_debug` inspects it without
acknowledgment or queue advancement.

## Delivered timing and controller inputs

The host queries `sync.field_started()` to see whether receiver initialization
has reached the point where field assembly begins. The query uses
synchronization, field lock and the existing horizontal blanking boundary.
It adds no chip state or timing. The frontend uses it to align the first input
delivery grid.

The serializable `DenisePins` state, exposed as `denise.pins`, keeps delivered
RGA strobes and game-port levels separate from registers, sprite state and
the pixel pipeline. Its `incoming` values retain the RGA event until the next
input sampling phase. Denise samples independent game-port levels directly
before engine work in the current CCK, with no additional pending copy.
See [pins.rs](../../../crates/denise/src/pins.rs).

`pins.rga_strobe` holds a synchronization event until Denise consumes it.
The [Machine Loop](Machine%20Loop.md#rga-synchronization-transport) delivers
the captured RGA address before receiver work and passes no Agnus counter
snapshot. `DeniseSync` stores the local horizontal and line position,
blanking and equalization state, the long-line marker and whether
synchronization has begun. CPU synchronization writes use the same local
operations as soon as the selection is accepted, with no queue entry.

Entering equalization begins a field, and successive line strobes advance
the local vertical position. Denise skips publication of the first partial field. Later complete fields
alternate between interlaced buffer rows.
The configured video standard selects the fixed presentation rectangle
described in [Frame Buffer](Frame%20Buffer.md). STRLONG marks the current
line as long without adding a line. Repeated identical strobes count as
distinct events. Restoring a snapshot preserves the receiver position,
unconsumed input, pixel pipeline and partially drawn buffer. The receiver
counters and retained edge offsets belong to the functional model. They
neither measure silicon phase nor copy Agnus's absolute beam position.

The separate [frame cursor](../../../crates/denise/src/frame_cursor.rs) maps
those receiver phases to final visible coordinates in the [frame buffer](Frame%20Buffer.md).
Its position and pending line survive snapshots with the pixel pipeline.

`pins.game_port_inputs` holds the raw direction and quadrature levels
delivered from both [game ports](Game%20Ports.md#connector-delivery). The
motherboard carries the levels. Denise decodes and counts them instead of
receiving JOY register values copied from device counters. Each sprite DMA
register write pairs a register address on RGA with its data word. The address
selects the same named queued setter used by a bus write, and the data word
supplies its value. The receiver delay starts after the common issue phase. If
the queue is full, the owner stops before admitting the word. Delivery is neither
deferred nor retried after the machine fails. Distinct words at the endpoint
confirm latch delivery. The old sprite scheduler and pixel pipeline fall outside
the scope of this observation.

## Digital controller counters

[`game_ports.rs`](../../../crates/denise/src/game_ports.rs) advances both JOY
registers from the current connector levels. In each axis, the low two bits
show the binary phase derived from the pins. Forward and backward quadrature
transitions advance the eight-bit count in the corresponding direction,
wrapping at the limits. If both phases change at once and the direction is
ambiguous, Denise updates the visible pin bits without counting a direction.
JOYTEST writes `$FCFC` on both registers and preserves the four low bits from
the inputs, following HRM Appendix A. Denise never copies a device-side
position into a JOY register.

The low bits also retain the last phase Denise sampled. JOYTEST and snapshot
restoration must preserve how those bits relate to subsequent connector
samples.

## References

- [General Architecture](General%20Architecture.md): Places Denise and its crates in the machine architecture.
- [Amiga Hardware Reference Manual, Chapter 3](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md): Explains bitplanes, fetch and display windows, resolution, dual playfields, and HAM.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Describes video priority and collision detection.
- [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#diwstop-diwstrt): Defines all eight programmable horizontal DIW bits and the OCS H8 extensions.
- [ARP Denise reconstruction](../../../ref_src/amiga_replacement_project/denise/src/denise.v): Reference-model counter restart and window comparison, separate from the emulator delivery phase.
- [`crates/denise/src/denise.rs`](../../../crates/denise/src/denise.rs): Living implementation of the Denise coordinator.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Register meanings, named accessors, recipient sets and missing effects.
- [`denise/pins.rs`](../../../crates/denise/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
