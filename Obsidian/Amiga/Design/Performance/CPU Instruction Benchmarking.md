---
title: "M68000 Instruction Benchmarking"
aliases: ["CPU Instruction Benchmarking", "Instruction Benchmarking Architecture", "CPU Benchmark Analysis Guide"]
tags: ["amiga", "design", "m68000", "benchmark", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-10-26
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](../Verification/CPU%20SingleStepTests.md)", "[Performance Profiling and Optimization Strategy.md](Performance%20Profiling%20and%20Optimization%20Strategy.md)"]
tracked_paths:
  - "crates/test_runner/Cargo.toml"
  - "crates/test_runner/benches"
  - "benchmarks/m68k"
  - "tools/verification/cpu_performance_tests.py"
---

# M68000 Instruction Benchmarking

The [CPU performance test catalog](../../../../benchmarks/m68k/README.md) is the
active CPU instruction measurement workload. It measures host time for complete
programs through the cycle-stepped CPU and mapped memory. Results include guest
helpers and the host stepping loop; nanoseconds per target is a whole-workload
cost, not isolated opcode latency. The separate [SingleStep verification](../Verification/CPU%20SingleStepTests.md)
checks CPU state, clocks and timed bus callbacks against reference vectors.
Neither benchmark nor SingleStep measures complete Amiga throughput.

## Workload and measurement

Each case has `program.s` and `case.json`; shared settings are in `config.json`.
The [runner](../../../../benchmarks/m68k/runner.rs) checks the pinned assembler,
substitutes the configured target count, assembles the complete image, initializes
CPU and memory, and executes to the declared end marker. It guards execution
bounds and instruction count. The first timed program establishes observed total
instructions and CPU clocks; later programs in the same attempt must match.
CPU and memory are reset outside the timer for each complete program.

The [command layer](../../../../tools/verification/cpu_performance_tests.py)
selects all cases by default or all matches for one exact case ID, mnemonic, or
declared four-digit primary opcode. The opcode is a catalog identifier, not a
claim about every word in the assembled image. The default is three series per
case, each accumulating at least 400 ms of timed whole-program execution. The
minimum nanoseconds per target across series is selected, while all series are
retained. Positive duration and series overrides support focused development.
No pilot, NOP control, warm-up, or separate guest inspection pass precedes timing.
A failed timed execution is a failed case.

The host monitor samples the runner's pinned CPU core from outside the benchmark
process during measured series (all logical cores when affinity is unavailable).
It subtracts the benchmark process's CPU time from busy time. More than 20%
average external busy time in any series, or unavailable
observation, marks an attempt disturbed. A disturbed case is rerun once with the
same settings. Both attempts remain in the report; persistent load remains
visible on the selected timing. The monitor does not wait for an idle threshold.

## Reports and comparison

Each invocation creates a portable ID under `benchmarks/results/`. Its manifest
records effective settings, selected cases, attempts, load samples, failures and
completion status. Assembled source, binary and symbols remain disposable under
`.agent/tmp/benchmarks/cpu_performance_tests/`. Completed runs automatically produce
aggregate JSON/CSV, anomaly findings, previous-run comparison and a readable report. Explicit
analysis regenerates those reports from the stored measurements without timing.
An incomplete run is ineligible for analysis and comparison.

The anomaly report records every series spread without a cutoff, plus persistent
host load and failed executions. These findings are diagnostic, not correctness gates. A
filtered run compares only its measured intersection. Its reference is the
latest earlier locally available, completed full run with matching host and settings;
a case additionally needs matching source, configuration, image, target count
and observed CPU clocks. Missing or incompatible rows are explicit. Raw timing
changes are reported without NOP adjustment. Thresholds for future anomaly
rules remain a separate decision.

The approved [JSON](../../../../benchmarks/m68k_benchmark_baseline.json) and
[CSV](../../../../benchmarks/m68k_benchmark_baseline.csv) baseline record the
first complete default-parameter CPU performance test run: 2,257 cases, three series
of at least 400 ms, and the minimum whole-program nanoseconds per target. Its
source run and manifest are retained only on the measuring host under the
ignored `benchmarks/results/`. The former decoder-derived baseline is retained
in Git history; its cases and NOP control
policy are incompatible with this catalog. Later comparisons use an earlier
locally available compatible full run, with the baseline files serving as the approved
reference snapshot. Future baseline corrections follow the
[reference-artifact approval procedure](../../../../.agents/rules/spec-compliance.md).
The [decoder inventory](../../../../crates/test_runner/benches/bench_instructions/inventory.rs)
remains available to the separate coverage audit; it is not a measurement path.

## References

- [Assembly case catalog](../../../../benchmarks/m68k/README.md): case format and whole-program execution.
- [Verification commands](../../../../tools/verification/README.md): measurement, analysis and profiling entry points.
- [Profiling strategy](Performance%20Profiling%20and%20Optimization%20Strategy.md): interpretation of host measurements.
- [Motorola 68000 timing tables](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): hardware timing context, separate from host wall time.
