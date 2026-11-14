# Project Optimization Examples

These examples record mechanisms and failure modes from November 13-14, 2026.
Use them to generate hypotheses, not as a list of transformations to apply
everywhere. Git commits and [DIARY.md](../../../../DIARY.md) retain the historical
results. Remeasure on the current source and workload before claiming a gain.
The November 14 source examples also include pending implementations; examples
without timings do not establish a measured speedup or completed verification.

## Scan Occupied Entries and Reuse Results

Commit `eb4c408f5400515560e41b08b34578455855183a` changed
[register mutation queues](../../../../crates/config/src/mutation.rs) from
full-capacity scans and full result arrays to a packed occupied prefix and
reusable, length-delimited due storage. The warm ROM diagnostic had almost all
slots empty. Matched release measurements reduced core frame time from 23.890
to 10.017 ms with pixel generation enabled and GUI work excluded.

The reusable lesson is to inspect initialization, returned copies and consumer
scans together; a cheap queue operation can still have an expensive result path.
Capacity, issue order, the complete due set and receiver phases remained intact.
Low observed occupancy did not justify reducing capacity or altering overflow
behavior. Differential state comparison established model preservation, not
hardware timing acceptance.

## Move Stable Decoding Out of the Pixel Loop

Commit `bbcbcdf9ad87e4105ffee26b33e5cea824a6be9d` cached horizontal starts,
attachment and collision controls in the
[sprite engine](../../../../crates/denise/src/sprites.rs), skipped inactive pairs,
and used small tables for color and collision decoding. It retained completion
of disarmed serializers. Compared with the preceding idle-sprite guard, measured
throughput increased about 3.6-4.9% across the recorded ROM and active scenes.

The useful question is how often a value changes compared with how often it is
decoded. Update derived values at effective register application, including
mid-line changes, and reconstruct them after restore. An inactive flag alone is
insufficient when a serializer still has output to consume. Idle-only results
cannot establish the gain for overlapping or attached sprites.

## Share Precomputed Output, Keep Side Effects at Consumption

Commit `c49dc32604bb486bc4c5eab803a7e6dd5bd05c62` introduced a lazily built
2 KiB sprite line table shared by pixel selection and collision paths, plus
playfield match masks derived from CLXCON. Recorded active-scene throughput gains
were about 2.9-3.4%; the ROM scene gained about 6.5%. GUI work was excluded, and
the active synthetic scenes used a stopped CPU.

Applied writes invalidate future output. Potential collision bits may be cached,
but CLXDAT accumulates only when the current pixel is consumed. Current palette
and priority remain effective. Review found an H=510 word wrapping into early
cache entries; clipping spans to the current window fixed premature collisions
while preserving continuation. The final measurements include that correction.

The lesson is to price invalidation and boundary behavior, not only table reads.
The remaining shifts used for construction, inspection and save state were not
per-pixel costs. Determine execution frequency before treating a familiar source
expression as the next hot path.

## Separate Requested Diagnostics from Continuous Execution

Commit `c7bde75` retained prepared debugger inspectors during continuous Run while
preserving live breakpoint checks and refreshing on actual stop or paused edits.
The recorded 60-field live_stripes comparison reduced Developer execution from
36.772 to 25.042 ms, excluding pacing waits. This was one local scenario, not a
general display FPS guarantee.

Repeated work may serve a different mode or refresh boundary. Identify when a
consumer needs fresh data before introducing a cache or skipping computation;
preserve explicit stepping and live controls.

## Resolve Elapsed Time on Access

The November 14 [RTC implementation](../../../../crates/rtc/src/rtc.rs) replaces
per-CCK stepping with elapsed-time settlement on register access and time/control
changes. It retains the last machine CCK and fractional second, then derives the
current time when needed. Debug reads evaluate a copy; machine reset settles the
old interval before rebasing the CCK origin.

The lesson is to defer a result whose intermediate updates have no required
consumer. Preserve STOP/HOLD behavior, control-write ordering, fractional time,
counter wrap and snapshot continuation. This uses emulated time, not host wall
time; it does not authorize deferring devices that emit intermediate events.

## Update Derived Results Only When Their Inputs Change

Commit `2451f6c` uses a 16-entry pin-to-phase table in
[Denise game ports](../../../../crates/denise/src/game_ports.rs). The counter's
existing low bits retain the consumed phase, so unchanged inputs need no counter
arithmetic or separate dirty flag. Input sampling still occurs every CCK.

The November 14 [audio mixer](../../../../crates/paula/src/audio.rs) similarly
rebuilds stereo levels only when a contributing channel changes or derived mixer
state becomes invalid. Channel stepping, PCM scheduling and existing BLEP tails
continue. Data/control changes and restore retain explicit update boundaries.

Reuse existing state before adding a cache. If a validity flag is needed, keep
mutation paths controlled so callers cannot silently leave derived output stale.
Unchanged output does not mean the device clock or pending output can stop.

## Guard the Inactive Helper, Preserve the Device Clock

The November 14 [CIA timers](../../../../crates/cia/src/timers.rs) return early
when there is no E/CNT edge and neither timer has a live pulse. Port B resolution
is skipped when neither timer drives those pins, while hidden waveforms still
evolve. The [Paula serial port](../../../../crates/paula/src/serial.rs) has a local
guard for stable idle state, including empty transmit storage and current pin
and status levels.

Put the guard where no work is provably due. Skipping an entire CIA because its
timers are idle would also skip input, delayed-write or handshake work. A pulse
that must expire is active work even between clock edges. Measure the guard's
active path as well as the common idle case.

## Replace Repeated Decoding with Explicit Immutable Tables

The November 14 [dual-playfield decoder](../../../../crates/denise/src/decode.rs)
uses 64 input-code pairs and two 64-entry priority mappings, totaling 256 bytes.
The [DMA scheduler](../../../../crates/agnus/src/dma.rs) maps fixed horizontal
slots through a compile-time table while retaining live request/enable checks
and the line-length-dependent final refresh. Commit `a1d9b55` also introduced
the [RGB444 conversion table](../../../../crates/frame_builder/src/frame_builder.rs).

These tables replace bounded, deterministic calculations without mutable cache
invalidation. Their indices retain clear meanings: plane bits, horizontal CCK or
RGB444 color. Keep dynamic palette values and arbitration decisions live; a
table of opportunities is not a precomputed bus grant. Derive entries from the
existing mapping, then measure lookup cost and memory footprint.

## Reuse a Local Result and Expose Constant Arithmetic

November 14 [signal routing](../../../../crates/machine_loop/src/signals.rs)
consumes the Port B value returned by CIA-B polling instead of resolving the
same pins separately and discarding the poll result. Inside the
[Blitter](../../../../crates/agnus/src/blitter.rs), execution reuses its selected
phase for area work instead of asking for that phase again.

The [disk engine](../../../../crates/paula/src/disk.rs) selects between remainder
expressions with named constant periods, allowing the compiler to simplify each.
It retains remainder semantics when FAST changes mid-cell; compare-and-reset
would need a different proof. These are local data-flow or arithmetic changes,
not reasons to export private execution hints to another chip. Inspect optimized
instructions before attributing a gain to fewer source expressions.

## Write to Final Storage and Clear Only Invalid Regions

Commit `a1d9b55` gave the
[frame buffer](../../../../crates/frame_builder/src/frame_builder.rs) a fixed
row stride and paired pixel writes, avoiding row relocation as visible width
changes. Row completion clears invalid tails instead of clearing pixels that
will be overwritten; reset and layout changes retain their required clearing.

The lesson is to remove data movement by making storage straightforward. Keep
visible geometry, field pairing, pipeline order and consumer stride handling
explicit. A shorter frame or missing pixels are changed output, not a speedup.

## A Shorter Formula Is Still a Hypothesis

The November 12 BCD investigation considered alternative ABCD/SBCD/NBCD
arithmetic. An exhaustive arithmetic identity alone did not establish full CCR
equivalence or speed, and that investigation ended with a plan, not a measured
optimization. Current implementations are under
[CPU instructions](../../../../crates/cpu/src/instructions).

Include input X, cumulative Z, the other flags and invalid packed digits when
checking equivalence. Include operand preparation and flag construction when
measuring an alternative. Generated instructions and comparable execution time
decide performance; fewer Rust expressions do not. Respect a plan-only request.
