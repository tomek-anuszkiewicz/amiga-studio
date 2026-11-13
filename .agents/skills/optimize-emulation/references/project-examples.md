# Project Optimization Examples

These examples record mechanisms and failure modes from November 13, 2026.
Use them to generate hypotheses, not as a list of transformations to apply
everywhere. Git commits and [DIARY.md](../../../../DIARY.md) retain the historical
results. Remeasure on the current source and workload before claiming a gain.

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
