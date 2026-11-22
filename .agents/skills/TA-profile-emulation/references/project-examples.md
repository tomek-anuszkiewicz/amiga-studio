# Project Optimization Examples

These examples describe changes investigated from November 13-14, 2026,
and what could go wrong with them. Use them to form hypotheses about the current
workload. Measure the current source and workload before claiming a gain.
The November 14 examples drawn from source include pending implementations.
Where an example gives no timings, the speedup and completion of verification
remain unconfirmed. Git commits and [DIARY.md](../../../../DIARY.md) record
the historical results.

## Scan Occupied Entries and Reuse Results

Commit `eb4c408f5400515560e41b08b34578455855183a` packed occupied entries at
the start of each queue so scans could stop at the last occupied entry. It
also reused storage for entries due for delivery and tracked the length of
the result, replacing full result arrays. Almost all slots were empty in the
warm ROM diagnostic. In matched release measurements, core frame time fell
from 23.890 to 10.017 ms with pixel generation enabled and GUI work
excluded. [Register mutation queues](../../../../crates/config/src/mutation.rs)

Inspect how the queue initializes storage, copies its results and passes them
to consumers that scan them. Processing the results can cost more than the queue
operation itself. The change preserved capacity, issue order, all entries due
for delivery and the phases in which receivers process them. Low observed
occupancy was insufficient evidence for reducing capacity or changing overflow
behavior. Comparing the baseline and candidate model states showed that the
change preserved the model. Hardware timing acceptance requires separate evidence.

## Move Stable Decoding Out of the Pixel Loop

Commit `bbcbcdf9ad87e4105ffee26b33e5cea824a6be9d` cached horizontal starts,
attachment and collision controls, skipped inactive pairs and used small
tables to decode colors and collisions. Disarmed serializers could still
finish emitting their output. Compared with the preceding idle-sprite guard,
measured throughput increased about 3.6-4.9% across the recorded ROM and
active scenes. [Sprite engine](../../../../crates/denise/src/sprites.rs)

Compare how often a value changes with how often the code decodes it. Update
derived values when register writes take effect, including mid-line changes,
and reconstruct them after restore. Check for pending serializer output even
when the inactive flag is set. To establish the gain for overlapping or
attached sprites, measure scenes in which those sprites are active.

## Share Precomputed Output, Keep Side Effects at Consumption

Commit `c49dc32604bb486bc4c5eab803a7e6dd5bd05c62` built a 2 KiB sprite line
table on demand and shared it between pixel selection and collision detection.
It also derived playfield match masks from CLXCON. Recorded throughput rose
about 2.9-3.4% in active scenes and about 6.5% in the ROM scene. The measurements
excluded GUI work, and the CPU was stopped in the active synthetic scenes.

When writes take effect, they invalidate cached output for future pixels. The
cache can hold potential collision bits. CLXDAT accumulates those bits only
when the current pixel is consumed, and the palette and priority in effect
at that point still apply. Review found that a word at H=510 wrapped into
early cache entries. Clipping spans to the current window prevented premature
collisions and preserved continuation. The final measurements used the
corrected code.

Account for the cost of table reads, invalidation and boundary handling. Table
construction, inspection and save state work use the remaining shifts outside
the per-pixel path. Determine how often an expression runs before treating it
as a frequent cost.

## Separate Requested Diagnostics from Continuous Execution

Commit `c7bde75` kept the prepared debugger inspectors during continuous Run.
Breakpoint checks still ran live. The inspectors refreshed when execution
stopped or the user made edits while paused. In the recorded 60-field
live_stripes comparison, Developer execution time fell from 36.772 to 25.042 ms
with pacing waits excluded. The result applies to that local scenario, so a
broader claim about display FPS needs more evidence.

Work repeated during continuous execution may only need to run in a different
mode or when the user next needs fresh data. Determine when each consumer
needs an update before adding a cache or skipping computation. Preserve
explicit stepping and live controls.

## Resolve Elapsed Time on Access

An earlier November 14 RTC experiment kept the last machine CCK and the
fractional second. It applied elapsed time when software accessed a register
or changed the time or control settings, replacing per-CCK stepping. The
experiment was rolled back. Under the explicitly approved functional contract,
the current clock uses host UTC plus an independent battery offset. Machine
snapshots exclude the clock. [Current RTC](../../../../Obsidian/Amiga/Design/RTC.md)

Defer a result only if every consumer can wait for the update. Preserve the
device's specified clock, the order of control writes and the observable
state. Using host time in place of emulated time changes the requirement and
needs a separate decision.

## Update Derived Results Only When Their Inputs Change

Commit `2451f6c` uses a 16-entry table to map input pins to phases. The
counter's existing low bits record the last phase the counter consumed, so
unchanged inputs require neither counter arithmetic nor a separate dirty
flag. The ports still sample inputs every CCK.
[Denise game ports](../../../../crates/denise/src/game_ports.rs)

An earlier November 14 audio-mixer experiment rebuilt stereo levels only when
a contributing channel changed or the stored levels became invalid. Those
conditions show what a candidate that reuses levels must check. The trial was
rejected and removed, and the current mixer runs unconditionally.
[Current mixer](../../../../crates/paula/src/audio.rs)

Reuse existing state before adding a cache. If you need a validity flag, keep
changes to inputs in code that also updates the flag so callers cannot leave
derived output stale. An unchanged result is insufficient reason to stop
advancing the device clock or processing pending output.

## Guard the Inactive Helper, Preserve the Device Clock

The November 14 CIA timers return early when there is no E/CNT edge and
neither timer has a live pulse. If neither timer drives the Port B pins, the
code skips resolving those pins while the hidden waveforms continue to evolve.
[CIA timers](../../../../crates/cia/src/timers.rs)

The Paula serial port checks for stable idle state locally. The check includes
empty transmit storage and the current pin and status levels.
[Paula serial port](../../../../crates/paula/src/serial.rs)

Put the guard at the point where you can establish that no work is due.
Skipping an entire CIA because its timers are idle would also skip input,
delayed-write or handshake work. A pulse that must expire still needs handling
between clock edges. Measure the path through the guard when work is due and
in the common idle case.

## Replace Repeated Decoding with Explicit Immutable Tables

The November 14 dual-playfield decoder uses 64 input-code pairs and two
64-entry priority mappings, totaling 256 bytes.
[Dual-playfield decoder](../../../../crates/denise/src/decode.rs)

The DMA scheduler looks up fixed horizontal slots in a compile-time table.
It still checks live requests and enables, and the final refresh depends on
line length. [DMA scheduler](../../../../crates/agnus/src/dma.rs)

Commit `a1d9b55` also introduced the RGB444 conversion table.
[RGB444 conversion table](../../../../crates/frame_builder/src/frame_builder.rs)

These tables replace bounded, deterministic calculations and need no mutable
cache invalidation. Each index retains its meaning as plane bits, a horizontal
CCK or an RGB444 color. The slot table identifies opportunities to use the bus.
The scheduler still decides who receives each grant using the live state, and
dynamic palette values still apply. Derive entries from the existing mapping,
then measure lookup cost and memory footprint.

## Reuse a Local Result and Expose Constant Arithmetic

November 14 signal routing uses the Port B value returned by CIA-B polling.
Previously, it discarded that value and resolved the same pins separately.
[Signal routing](../../../../crates/machine_loop/src/signals.rs)

The Blitter reuses the phase it already selected when executing area work.
[Blitter](../../../../crates/agnus/src/blitter.rs)

The disk engine picks a remainder expression whose period is a named constant,
so the compiler can simplify each expression. The code retains remainder
semantics when FAST changes mid-cell. Compare-and-reset would need a separate
proof. [Disk engine](../../../../crates/paula/src/disk.rs)

Keep these changes to data flow and arithmetic local to the component. Private
execution hints remain private. Inspect optimized instructions before
attributing a gain to fewer source expressions.

## Write to Final Storage and Clear Only Invalid Regions

Commit `a1d9b55` used a fixed row stride and wrote pixels in pairs so the
frame buffer could leave rows in place when visible width changed. At the end
of a row it clears the invalid tail, while incoming pixel writes replace the
previous visible data. Reset and layout changes still clear the required
regions. [Frame buffer](../../../../crates/frame_builder/src/frame_builder.rs)

Choose a storage layout that avoids moving data. Keep visible geometry, field
pairing, pipeline order and the stride consumers use explicit. Treat a shorter
frame or missing pixels as a change in output when assessing any timing gain.

## A Shorter Formula Is Still a Hypothesis

The November 12 BCD investigation considered alternative ABCD/SBCD/NBCD
arithmetic. The exhaustive arithmetic identity left full CCR equivalence
and speed unverified. The investigation ended with a plan for further work.
The current implementations are in the CPU instructions directory.
[CPU instructions](../../../../crates/cpu/src/instructions)

Include input X, cumulative Z, the other flags and invalid packed digits when
checking equivalence. When measuring an alternative, include operand
preparation and flag construction. Use the generated instructions and
comparable execution times to judge performance. A request for a plan ends
at the planning stage.
