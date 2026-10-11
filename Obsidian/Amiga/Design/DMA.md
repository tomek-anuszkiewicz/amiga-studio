---
title: "Agnus DMA Architecture, Slot Scheduling & Bus Arbitration Specification"
aliases: ["DMA", "DMA Scheduler", "DMA Arbitration", "DMACON", "DMACONR"]
tags: ["amiga", "design", "dma", "arbitration", "agnus", "scheduling"]
category: "Design"
subsystem: "dma"
status: "draft"
created: 2026-09-19
updated: 2026-10-11
related: ["[Agnus.md](Agnus.md)", "[Copper.md](Copper.md)", "[Blitter.md](Blitter.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/dma"
---
# Agnus DMA Architecture, Slot Scheduling & Bus Arbitration Specification

Agnus schedules Chip RAM access in Color Clock (CCK) memory slots. A slot defines
an opportunity for a channel to transfer a word and determines whether a competing
CPU access must wait. This document owns the DMA schedule and arbitration model.
[Machine Loop.md](Machine%20Loop.md) owns routing between connected components;
the [cross-chip catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
owns signal directions and latching connections.

The hardware basis is [HRM Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md),
"Blitter Operations and System DMA", including Figures 6-9 through 6-12, and the
[Fat Agnus technical specification](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md).
The existing Rust scheduler is an implementation to evaluate against this evidence.
It does not establish hardware correctness. This specification remains a draft
because the refresh boundary and the mapping from chart coordinates to the modeled
beam phase still require verification.

## 1. Ownership and Transfer Mechanism

[Agnus](Agnus.md) owns DMA addresses, pointer updates, and Chip RAM arbitration.
It selects the memory address and register address (RGA). For memory-to-chip
transfers, [Denise](Denise.md) or [Paula](Paula.md) latches the shared data bus on
the matching register strobe. Neither chip independently reads Chip RAM.

Disk transfers work in both directions: Paula receives a word read from RAM or
supplies a word to be written to RAM, while Agnus generates the address. Audio
requests originate in Paula; the Fat Agnus specification describes requests over
`DMAL`. Request, enable, address, transfer, and completion are distinct events.
A channel enable alone does not mean that every assigned slot transfers data.
A zero pointer is a valid Chip RAM address and is not a DMA disable condition.

Refresh occupies memory cycles without transferring a data word. The Fat Agnus
specification describes row addresses with both RAS signals active and CAS signals
inactive; RGA carries synchronization codes during these cycles. Refresh must not
be modeled as an ordinary word fetch into a peripheral data register.

## 2. Coordinates, Line Boundaries, and Fixed Slots

One HRM memory slot is one CCK, approximately 280 ns. A minimum 68000 bus cycle
takes four CPU clocks, or two CCKs. Converting CPU bus-cycle numbers to CCKs does
not reconstruct a DMA schedule.

HRM describes a horizontal line as approximately 227.5 CCKs, with 226 cycles
available for allocation. This is not an integer loop bound for all chip models.
Line timing belongs to [Agnus.md](Agnus.md); DMA must use the model's actual
counter range and wrap phase.

### 2.1 Reading Figure 6-9

![HRM Figure 6-9: DMA Time Slot Allocation](../Reference/Hardware%20Reference%20Manual/assets/chapter_06_figure_6-9_dma_time_slot_allocation.svg)

The figure uses hexadecimal horizontal coordinates. Its footnote warns that
chart cycle zero appears to omit a refresh cycle: the chart was adjusted to match
data-fetch and display-start requirements. The first refresh block must not
simply be assigned to internal `hpos == 0`. Nor does the figure alone establish
that the boundary refresh occurs at both 226 and 227.

The positions below use the chart's coordinate convention. Disk, audio, and
sprite positions provide a reference schedule; their correspondence to the
implementation's address and data phases must be checked separately.

| Channel | Allocation per horizontal line | Chart positions and boundary qualification |
| :--- | :--- | :--- |
| DRAM refresh | 4 memory cycles | Three early positions `1, 3, 5` (`$01, $03, $05`) and one refresh at the line boundary. Its exact internal `hpos` and phase remain unresolved. |
| Disk | Up to 3 word transfers | `7, 9, 11` (`$07, $09, $0B`). |
| Audio 0-3 | Up to 1 word per channel | `13, 15, 17, 19` (`$0D, $0F, $11, $13`), respectively. |
| Sprites 0-7 | Up to 2 words per channel | Odd positions `21..51` (`$15..$33`); sprite `n` has positions `21 + 4n` and `23 + 4n`. |
| Bitplanes | Depends on mode and fetch window | Display DMA timing, resolution, and plane count determine the pattern; extended fetching may overlap sprite opportunities. |

These counts are transfer opportunities. Unrequested slots are available to
other eligible users. Sprite transfers also depend on control/data state and
vertical timing, described in [Sprites.md](Sprites.md) and HRM Chapter 4.
The fixed table does not define that state machine.

### 2.2 Existing Counter Convention and Boundary Finding

Inspection and focused execution of production
[`Agnus::step_cck_ram()`](../../../crates/agnus/src/agnus.rs) on 2026-10-11 establish
these implementation facts:

- Reset starts at `hpos = 0`. Each step increments the counter, wraps it if needed,
  and then passes `beam().hpos` to the scheduler. The first step after reset
  arbitrates position 1; position 0 is arbitrated on wrap.
- PAL lines contain 227 steps and use `0..=226`. NTSC alternates 227 and 228 steps,
  using `0..=226` and `0..=227`, respectively.
- CPU-visible `VHPOSR` reads use a separate modeled pipeline offset. This readout
  is not the scheduler's input and has not been validated here against hardware.
- With disk, audio, and sprite DMA enabled and requesting service, and bitplane,
  Copper, and Blitter DMA disabled, the executed disk, audio, and sprite positions
  match the reference rows above.

The same execution with custom DMA disabled and enabled observes:

| Executed line | Refresh positions selected by the current scheduler | Count |
| :--- | :--- | :---: |
| PAL, 227 CCKs | `1, 3, 5, 226` | 4 |
| NTSC short, 227 CCKs | `1, 3, 5, 226` | 4 |
| NTSC long, 228 CCKs | `1, 3, 5, 226, 227` | 5 |

[`dma.rs`](../../../crates/dma/src/dma.rs) declares four refresh positions in
`HPOS_REFRESH_SLOTS`, but its fixed-slot matcher additionally accepts 227. The
fifth selection on a long NTSC line is an observed discrepancy with the HRM's
four-cycle description, not an approved fifth hardware slot. The correct boundary
rule requires model-specific timing evidence before a production repair. Choosing
a terminal position from line length alone would still assume the missing phase
mapping.

## 3. Request Gating and Arbitration

HRM describes priority groups with assigned opportunities, rather than eight
channels competing in a global order at every horizontal position:

1. Refresh reserves its cycles. Disk, audio, display, and sprite DMA have priority
   in their allocated slots when they need a transfer. Display DMA can displace
   sprite DMA when fetching overlaps sprite slots.
2. The [Copper](Copper.md) has priority over the Blitter and CPU, subject to its
   own fetch eligibility and execution state.
3. The [Blitter](Blitter.md) normally has priority over the CPU on remaining
   eligible cycles. Its requests depend on the operation's transfer sequence.
4. The CPU can use available memory cycles when its bus request is eligible.
   An unclaimed slot does not establish immediate CPU transaction completion.

Enabled custom DMA channels require both `DMAEN` and the channel's `DMACON` bit.
Request conditions determine whether an opportunity is used. Refresh is independent
of these enables. An idle or disabled channel releases its opportunities, subject
to the other users' eligibility.

With `BLTPRI = 0`, the DMA manager monitors unsatisfied CPU memory requests. After
three consecutive unsatisfied memory cycles, the Blitter releases the bus for one
cycle. This does not override refresh or other higher-priority DMA. With
`BLTPRI = 1`, this yield is disabled: the Blitter may claim every otherwise
available cycle that its operation needs. Busy status alone does not establish a
word transfer on every CCK or prevent all CPU progress.

## 4. Display DMA and CPU Bandwidth

Bitplane fetching depends on `DDFSTRT`, `DDFSTOP`, the vertical display interval,
resolution, and the number of planes in `BPLCON0`. Standard low-resolution fetch
settings are `$38..$D0`. The final fetch group must be accounted for; the stop
register is not a simple inclusive word-transfer boundary. Detailed display-window
behavior belongs to [Agnus.md](Agnus.md) and [Denise.md](Denise.md).

HRM Figure 6-11 gives this eight-position low-resolution sequence:

| Position within the group | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Plane number, 1-based | - | 4 | 6 | 2 | - | 3 | 5 | 1 |

With four or fewer planes, alternate cycles remain available. Plane 5 uses one
additional opportunity and plane 6 uses another, consuming 25% and 50% of those
otherwise available cycles, respectively. Plane order is not `1, 2, 3, 4`
on successive even phases.

HRM Figure 6-12 gives the high-resolution sequence `4, 2, 3, 1, 4, 2, 3, 1`.
Four high-resolution planes occupy every memory opportunity during active display
fetching; blanking still provides other access opportunities. These are hardware
plane numbers; a zero-based software index must subtract one explicitly.

HRM assigns even-numbered memory cycles to the 68000 in the normal sharing pattern.
This does not guarantee that every even internal `hpos` is free: display fetching
and other arbitration can consume these opportunities, and the chart-to-counter
phase must first be established. The previous inverted parity description is
superseded by the sequences above.

## 5. Bus Contention and Observable Effects

For a granted DMA transfer, distinguish address selection, data transfer or latch
strobe, pointer advancement, and completion notification. DMA ownership and CPU
blocking must describe the same modeled bus phase. Arbitration alone does not
prove that the intended word reached its destination.

[MemoryBus.md](MemoryBus.md) defines CPU address regions sharing this bus and how
a blocked access returns `BusResult::WaitState`. The CPU retains its active
micro-step while waiting. Independent Fast RAM does not compete for Chip RAM
slots; other CPU bus-cycle timing requirements still apply.

The machine loop routes requests and resulting bus signals. It does not implement
a second DMA timetable or bypass Agnus to generate DMA addresses.

## 6. Control Registers

| Address | Access | Register | DMA responsibility |
| :--- | :---: | :--- | :--- |
| `$DFF002` | Read | `DMACONR` | DMA control state and live Blitter busy/zero flags. |
| `$DFF092` | Write | `DDFSTRT` | Horizontal bitplane data-fetch start. |
| `$DFF094` | Write | `DDFSTOP` | Horizontal bitplane data-fetch stop. |
| `$DFF096` | Write | `DMACON` | Set/clear DMA enables and Blitter priority. |

`DMACON` bit 15 selects set versus clear. Bit 10 is `BLTPRI`, bit 9 is `DMAEN`,
and bits 8 through 4 enable bitplanes, Copper, Blitter, sprites, and disk.
Bits 3 through 0 enable audio channels 3 through 0. `DMACONR` bit 14 is `BBUSY`
and bit 13 is `BZERO`; these status bits are not writable channel enables.
Register propagation and ownership are specified in [Agnus.md](Agnus.md) and the
[register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md).

## 7. Remaining Verification Before Implementation Closure

The fixed-channel observation used a real Agnus, its embedded scheduler, and a
512 KiB Chip RAM buffer. It covered two complete lines per model and enable state,
after discarding the partial reset line, and recorded owner positions and their
agreement with the blocking flag. Disk requests were forced; the current Agnus
path supplies audio requests. No real Paula request sequence was exercised.
This is a production scheduling observation, not whole-machine transfer proof.

Before claiming hardware-correct DMA behavior:

- Establish the chart origin and address/data phases against the internal beam,
  CPU-visible readout, and model-specific line wrap.
- Resolve the boundary refresh with precise PAL and short/long NTSC timing
  evidence. Verify refresh counts and CPU blocking across each wrap, including
  with custom DMA disabled.
- Verify actual requests, released slots, transfer direction, destination latches,
  and pointer updates through the production machine/bus path.
- Exercise bitplane sequences, sprite overlap, Copper eligibility, and normal/nasty
  Blitter competition with an actual pending CPU request.

Existing isolated scheduler tests cover selected positions and simplified
arbitration; they do not establish the full beam-phase mapping. Ordinary CPU wait tests use an isolated bus and do not execute Agnus's scanline scheduler. Production arbitration needs separate integration tests.

## 8. Reference Documentation & Upstream Ground Truth

- [HRM Chapter 6: Blitter Hardware](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Allocation counts, Figure 6-9 and its adjusted-zero footnote, plane order in Figures 6-11/6-12, and Blitter/CPU priority.
- [A500/A2000 Technical Reference Manual: Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Address/RGA generation, Paula requests, bidirectional disk transfers, and refresh signals.
- [HRM Chapter 4: Sprite Hardware](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md): Sprite control and image-word fetching conditions.
- [Agnus.md](Agnus.md): Beam counters, line timing, and register ownership.
- [MemoryBus.md](MemoryBus.md): CPU address decoding and wait-state contract.
- [Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md): Request, data-bus, and register-strobe connections.
- [`crates/dma/src/dma.rs`](../../../crates/dma/src/dma.rs) and [`crates/agnus/src/agnus.rs`](../../../crates/agnus/src/agnus.rs): Existing scheduling and beam implementation, subject to hardware verification above.
