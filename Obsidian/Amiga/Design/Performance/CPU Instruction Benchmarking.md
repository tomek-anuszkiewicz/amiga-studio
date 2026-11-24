---
title: "M68000 Instruction Benchmarking"
aliases: ["CPU Instruction Benchmarking", "Instruction Benchmarking Architecture", "CPU Benchmark Analysis Guide"]
tags: ["amiga", "design", "m68000", "benchmark", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-10-26
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](../Verification/CPU%20SingleStepTests.md)"]
tracked_paths:
  - "crates/test_runner/Cargo.toml"
  - "crates/test_runner/benches"
  - "benchmarks/m68k"
  - "tools/verification/cpu_performance_tests.py"
---

# M68000 Instruction Benchmarking

The CPU performance tests time complete programs on the host. The runner
executes each program with the cycle-stepped CPU and mapped memory. The
reported nanoseconds per target give the cost of the whole workload, including
guest helper instructions and the host stepping loop. Isolated opcode latency
falls outside the scope of this measurement.
[CPU performance test catalog](../../../../benchmarks/m68k/README.md)

SingleStep verification checks CPU state, clocks, and timed bus callbacks
against reference vectors. Both activities stop at the CPU workload boundary.
Complete Amiga throughput is outside their scope.
[SingleStep verification](../Verification/CPU%20SingleStepTests.md)

## Workload and measurement

Each case has `program.s` and `case.json`. All cases use the shared settings
in `config.json`. The runner checks the pinned assembler, substitutes the
configured target count, and assembles the complete image. It initializes the
CPU and memory and runs the program to the declared end marker, checking
execution bounds and instruction count. The first timed execution establishes
the observed total instructions and CPU clocks. Every later execution in the
same attempt must match both totals. The runner resets CPU and memory outside
the timed interval for each complete program.
[Runner](../../../../crates/test_runner/benches/bench_instructions/runner.rs)

The command selects all cases by default. A filter selects every match for
one exact case ID, mnemonic, or declared four-digit primary opcode. The filter
uses the opcode solely as a catalog identifier. The contents of the assembled
image are outside the scope of that identifier. The image contains the whole
program, including its helper instructions. The default is three
series per case, each accumulating at least 400 ms of timed whole-program
execution. The report retains every series and uses the minimum nanoseconds
per target as the result. For focused development, the caller can supply
positive duration and series overrides. Timing begins with the first
whole-program execution and includes no pilot, NOP control, warm-up, or
separate guest inspection pass. A failed timed execution fails the case.
[Command layer](../../../../tools/verification/cpu_performance_tests.py)

While each series runs, the host monitor measures busy time from outside the
benchmark process. It samples the CPU core to which the runner is pinned or
all logical cores if affinity is unavailable. It subtracts CPU time used by
the benchmark process to estimate external load. If any series averages more
than 20% external busy time or the monitor cannot obtain an observation, the
attempt is marked as disturbed. The benchmark reruns that case once with the
same settings. The report retains both attempts and flags persistent load on
the selected timing. The monitor samples during execution without waiting for
an idle threshold.

## Reports and comparison

Each invocation creates a portable run ID and keeps its results under
`benchmarks/results/`. The run manifest records the effective settings,
selected cases, attempts, load samples, failures, and completion status.
Assembled source, binary, and symbols remain disposable under
`.agent/tmp/benchmarks/cpu_performance_tests/`. When a run completes, it
automatically produces aggregate JSON/CSV, anomaly findings, comparisons
against the previous run and approved baseline, comparisons between related
cases, and a readable report. Explicit analysis uses the stored measurements
to regenerate those reports without timing the programs again. Analysis and
comparison require a complete run.

The machine-readable anomaly data records every series spread, regardless of
size, as well as persistent host load and failed executions. A filtered run
compares only cases measured in both inputs. For a previous-run comparison,
the reference is the latest earlier completed full run available locally on
the same host with the same settings. Comparable cases must also match in
source, configuration, image, target count, and observed CPU clocks. The
approved-baseline comparison reads the tracked JSON/CSV pair and reports
missing, new, disturbed, and incompatible cases separately from timing changes.

Related-case analysis compares matching pairs of arithmetic, logical, and unary
instruction cases. The pairs must also have equal target counts, CPU clocks,
and program shape.
It groups canonical addressing forms by operand size and source or destination
direction. The median instruction gap across matched forms helps identify
unusual gaps for particular modes. The analysis compares register and
special-path variants directly. A separate A7 analysis compares each
stack-pointer workload with matching A0-A6 workloads of the same instruction
and program shape.

The Markdown report lists only findings. These include baseline regressions
and incompatible cases, instruction and mode outliers, A7 outliers, and
unstable or failed measurements. The JSON files retain all comparisons and
skipped cases. The 5% timing and 10% series-spread limits flag results for
review. Correctness gates are independent of these thresholds. Whole-program
timing includes helpers, and the reported raw changes carry no NOP adjustment.

The approved baseline records the latest snapshot the user approved. It
covers 2,257 cases, three series of at least 400 ms, and the minimum
whole-program nanoseconds per target. The JSON identifies the base full
run and the source run of each refreshed case. Manifests remain only on the
measuring host under the ignored `benchmarks/results/`. Earlier baselines
remain in Git history. The former decoder-derived baseline used an
incompatible catalog and NOP control policy.
[JSON](../../../../benchmarks/m68k_benchmark_baseline.json) and
[CSV](../../../../benchmarks/m68k_benchmark_baseline.csv)

A successful complete run with default settings writes trackable
`m68k_benchmark_current.json` and `.csv`, four machine-readable anomaly
reports, and `m68k_benchmark_anomaly_report.md` beside the approved baseline.
Filtered and shortened runs cannot automatically replace these root artifacts.
A completed, undisturbed filtered run with the same host and default settings
may explicitly refresh matching cases in the current snapshot. The current
snapshot JSON records the source run of each refreshed case and identifies
the base full run separately. The refresh regenerates the CSV hash and anomaly
reports from the combined snapshot.

The runner treats the approved files as read-only. After reviewing the current
snapshot and its reports, the user may approve the complete JSON/CSV pair as
the next baseline, including documented filtered-run refreshes. Follow the
[reference-artifact approval procedure](../../../../.agents/references/spec-compliance.md)
to promote the baseline and retain the provenance of each case.

The separate coverage audit can use the decoder inventory. Timing runs use
the assembly case catalog.
[Decoder inventory](../../../../crates/test_runner/benches/bench_instructions/inventory.rs)

## References

- Case format and whole-program execution. [Assembly case catalog](../../../../benchmarks/m68k/README.md)
- Measurement and analysis entry points. [Verification commands](../../../../tools/verification/README.md)
- Hardware timing context, separate from host wall time. [Motorola 68000 timing tables](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md)
