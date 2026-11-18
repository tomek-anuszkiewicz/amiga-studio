# Measurement Tools and Evidence

## Profiler Selection

[Clippy](https://doc.rust-lang.org/clippy/) performs static linting, including
performance suggestions. It does not measure where execution time is spent.
[samply](https://github.com/mstange/samply) is a sampling profiler. Check the
installed profiler's help and platform requirements before choosing a command;
do not assume a tool on another checkout or in an old temporary directory is
available or configured here.

The November 13 native investigations captured warmed machine execution with
Very Sleepy and used samply symbolication with matching PDBs to recover inline
frames. If retained, `.agent/tmp/profiling-kickstart/` contains `profile_machine.py`,
`symbolicate_inline.py` and analysis helpers. Later experiments under
`.agent/tmp/core-performance-stage1/`, `.agent/tmp/core-performance-stage2/` and
`.agent/tmp/sprite-line-buffer/` contain frozen builds and measurement drivers.
These are disposable historical artifacts, not supported tooling entry points.
Inspect their paths, workload assumptions, timing boundaries, symbol matching and
source manifests before reuse. Do not promote or repair that infrastructure
unless the current experiment needs it.

Use an optimized build with usable symbols. Preserve its executable and PDB (or
target-equivalent symbols) together before another build overwrites them. The
workspace's `verification` profile retains debug assertions and overflow checks;
do not treat its timings as release performance. Record any compiler/environment
overrides and use equivalent settings for baseline and candidate.

Report the selected thread/stack root, capture interval, sample weight semantics
and unresolved symbols. Inspect the exact binary's generated code when inline
attribution is ambiguous. Address ranges or symbol corrections from another
binary are not transferable evidence. Profile separately from throughput timing.

## CPU Measurements and Correctness

Use the maintained [benchmark catalog](../../../../benchmarks/m68k/README.md) and
[verification command reference](../../../../tools/verification/README.md#cpu-benchmark-entry-point).
From the repository root, a focused example is:

```powershell
python tools/verification/verify.py runner bench mnemonic ABCD
python tools/verification/verify.py test runner singlestep case test_abcd sample 50
```

Select the actual affected instructions and appropriate correctness coverage.
The commands above demonstrate distinct measurement and reference checks;
50 vectors are not a universal acceptance threshold. Use `--plan` to inspect
selection without running a measurement or test.

The CPU benchmark measures whole guest programs, including helpers and stepping.
It does not measure isolated arithmetic latency, chipset contention or complete
Amiga throughput. Preserve the runner's measurement policy. Compare compatible
workloads explicitly when interleaving filtered runs: automatic previous-run
selection expects a completed full run. Report all relevant series and noise;
the analyzer's display thresholds are not a minimum useful optimization gain.

Filtered runs keep reports in their run directories. A successful full run with
default settings also replaces tracked current snapshots and reports. Inspect
existing changes before selecting that operation. The approved baseline changes
only through the existing reference-approval procedure.

## Whole-Machine and Presentation Measurements

For combined CPU/chipset activity, use the maintained
[mixed game scene](../../../../crates/machine_loop/tests/programs/display/mixed_scene/README.md).
Its guest binary also loads in the GUI. Its native example separates an activity
audit from repeatable timed series, including synthetic input and audio draining.
Use the owning README for commands, counting boundaries and coverage limits.

Inspect the current public machine stepping API and existing harness before
building a diagnostic. Historical `machine_bench.rs` experiments called the
normal `step_frame` path, retaining pixel generation while excluding GUI and
pacing. They are not a maintained whole-machine benchmark command. If no suitable
harness remains, create only the minimal disposable diagnostic needed for the
requested measurement; do not revive retired verification workloads.

The earlier A-B-B-A comparisons used 300 warmup frames and several 500-frame
series per run. Treat these as starting examples, not universal constants.
Choose sufficient steady-state work for the observed noise and question. Record
affinity, power settings and competing load consistently; include process CPU
time when wall-time variation needs explanation. Never combine results from
different baseline snapshots into one before/after pair.

Use active scenes that exercise the proposed saving and its maintenance cost:
DMA and output enabled, collisions, register changes, or active inputs as
applicable. Confirm that the intended activity occurs. Separate ROM, synthetic
workload and real guest evidence. For GUI or browser conclusions, measure that
execution path as well; a native headless result or a Wasm compile is insufficient.

## Designing a Mixed Guest Workload

When a combined scene is requested, use a deterministic guest program that runs
the CPU, accesses Chip RAM, writes custom registers and schedules concurrent DMA.
Useful work includes moving and overlapping sprites, sprite/playfield collisions,
scrolling bitplanes, Copper raster changes, repeated Blitter operations and audio
DMA with nonzero samples. Fixed data or a fixed random seed makes comparisons
repeatable; visual polish is unnecessary.

The existing [live_stripes guest fixture](../../../../crates/machine_loop/tests/programs/display/live_stripes/program.as)
demonstrates guest-owned setup. Component integration fixtures under
[machine tests](../../../../crates/machine_loop/tests) cover Copper, Blitter,
audio and sprite behavior. They are reusable starting points, not an existing
combined benchmark. The historical sprite performance scenes stopped the CPU
and therefore do not cover sustained CPU work under competing DMA.

Let the host load the program and data, establish initial CPU state and execute
normal machine steps. Keep ongoing scene updates on guest CPU/Copper paths so
the workload exercises decoding, propagation, arbitration and waits. Use a
repeatable schedule with both stable output and changes that invalidate caches.
Keep completed guest work comparable: a scene that misses updates or completes
fewer blits must not appear faster merely because less work occurred.

Before timing, verify guest progress, completed blits, active sprite output and
collisions, display changes and nonzero audio production. Account for audio
buffer consumption so a full buffer does not silently change the measured work.
Keep detailed counters and output/state checks outside the timed build or region.
Report fixed-work throughput and variability separately from behavioral checks;
do not introduce a machine-dependent timing pass/fail gate. This scene complements
focused benchmarks and eventual real-game runs; it cannot establish performance
or compatibility for every game.
