# M68000 CPU Performance Tests

The catalog measures the host time spent running selected M68000 instruction
forms, addressing modes, and execution paths. Each case contains `program.s`
and `case.json` and uses the shared counts and timing settings in
[`config.json`](config.json). The [runner](runner.rs) assembles each program,
initializes the CPU and memory, and runs the program to its end marker.

## Run

From the repository root, install the pinned assembler and the monitor dependency
once.

```powershell
.\tools\bootstrap\bootstrap.ps1 -Assembler
python -m pip install -r tools/verification/requirements.txt
```

Use the first command to measure the complete catalog or the second to measure
one case.

```powershell
python tools/verification/verify.py runner bench
python tools/verification/verify.py runner bench case add/add_w_d1_postincrement
```

For selectors, shortened runs, analysis, and snapshot refresh commands, see the
[verification reference](../../tools/verification/README.md#cpu-benchmark-entry-point).

## Interpret results

By default, the runner measures each case in three series with at least 400 ms
of timed execution per series. It resets the CPU and memory outside the timer.
All executed instructions, including helpers, contribute to the measured time.
Use nanoseconds per target to compare whole-program cost per target instruction.
Measuring an individual instruction's latency requires separate evidence.

The runner checks execution bounds and the consistency of instruction and
CPU-clock counts. Establishing CPU correctness also calls for checks of final
registers, flags, and memory, which the runner leaves unverified. The selected
CPU workloads run without the chipset and Chip RAM contention, so full Amiga
throughput requires separate measurements.

Reports go to `benchmarks/results/<run-id>/`. The runner puts disposable
assembled files under `.agent/tmp/benchmarks/cpu_performance_tests/`. A successful
complete run with default settings updates the current snapshot under
`benchmarks/`. After a filtered run, updating that snapshot requires an explicit
refresh. Changes to the approved baseline require user approval. The baseline
consists of [`m68k_benchmark_baseline.json`](../m68k_benchmark_baseline.json) and
[`.csv`](../m68k_benchmark_baseline.csv).

See the [benchmark specification](../../Obsidian/Amiga/Design/Performance/CPU%20Instruction%20Benchmarking.md)
for measurement policy, comparisons, and baseline approval. For case format and
execution details, consult the [runner](runner.rs) and existing cases.
