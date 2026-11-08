# M68000 CPU Performance Tests

The catalog measures host execution time for selected M68000 instruction forms,
addressing modes, and execution paths. Each case contains `program.s` and
`case.json`; shared counts and timing settings are in [`config.json`](config.json).
The [runner](runner.rs) assembles each program, initializes CPU and memory,
and executes it to its end marker.

## Run

Run from the repository root. Install the pinned assembler and monitor dependency
once:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Assembler
python -m pip install -r tools/verification/requirements.txt
```

Measure the complete catalog or one case:

```powershell
python tools/verification/verify.py runner bench
python tools/verification/verify.py runner bench case add/add_w_d1_postincrement
```

For selectors, shortened runs, analysis, and snapshot refresh commands, see the
[verification reference](../../tools/verification/README.md#cpu-benchmark-entry-point).

## Interpret results

The defaults are three series of at least 400 ms of timed execution per case.
CPU and memory reset outside the timer; all executed instructions, including
helpers, contribute to the measured time. Nanoseconds per target therefore
expresses whole-program cost, not isolated instruction latency.

The runner checks execution bounds and consistent instruction and CPU-clock
counts. It does not verify final registers, flags, or memory. These benchmarks
do not establish CPU correctness or complete Amiga throughput; the chipset and
Chip RAM contention are not simulated.

Reports are written to `benchmarks/results/<run-id>/`. Assembled files are
disposable under `.agent/tmp/benchmarks/cpu_performance_tests/`. A successful
complete run with default settings updates the current snapshot under
`benchmarks/`; filtered runs require an explicit refresh. The approved baseline
is [`m68k_benchmark_baseline.json`](../m68k_benchmark_baseline.json) and
[`.csv`](../m68k_benchmark_baseline.csv), and changes only with user approval.

See the [benchmark specification](../../Obsidian/Amiga/Design/Performance/CPU%20Instruction%20Benchmarking.md)
for measurement policy, comparisons, and baseline approval. For case format and
execution details, consult the [runner](runner.rs) and existing cases.
