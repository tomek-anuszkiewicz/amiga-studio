---
title: "M68000 Instruction Benchmarking"
aliases: ["CPU Instruction Benchmarking", "Instruction Benchmarking Architecture", "CPU Benchmark Analysis Guide"]
tags: ["amiga", "design", "m68000", "benchmark", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-10-25
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](../Verification/CPU%20SingleStepTests.md)", "[Performance Profiling and Optimization Strategy.md](Performance%20Profiling%20and%20Optimization%20Strategy.md)"]
tracked_paths:
  - "crates/test_runner/Cargo.toml"
  - "crates/test_runner/benches"
  - "benchmarks/m68k"
  - "tools/verification/benchmark_candidates.py"
  - "tools/verification/analyze_candidate_benchmarks.py"
---

# M68000 Instruction Benchmarking

Two benchmark workflows measure host cost for M68000 instruction workloads through the cycle-stepped CPU and mapped `PhysicalMemory`. They are performance experiments, not proofs that the CPU matches silicon. The tracked [JSON](../../../../baselines/benchmarks/m68k_benchmark_baseline.json) and [CSV](../../../../baselines/benchmarks/m68k_benchmark_baseline.csv) baselines contain the validated 4,571-case decoder-derived run. The former 108-case catalog and baseline are retained only in Git history; they use incompatible workloads.

The independent [SingleStep verification](../Verification/CPU%20SingleStepTests.md) supplies that correctness evidence: it compares CPU state, instruction duration, and timed bus callbacks with software-generated reference vectors on an isolated test bus. The [CPU model](../CPU%20Motorola%20M68000.md) and [micro-step specification](../CPU%20Micro-Step%20State%20Machine.md) define intended behavior; this benchmark reports how much host time the implementation spends executing it. Neither test runs the complete Amiga machine or establishes whole-system throughput.

## Decoder-derived benchmark

The [inventory](../../../../crates/test_runner/benches/bench_instructions/inventory.rs) enumerates the decoder's legal primary opcodes, excluding decode-exception entries, and groups entries by static micro-step sequence. The [candidate generator](../../../../tools/verification/benchmark_candidates.py) splits condition codes and MOVEM size/direction, selects representative register and operand encodings, and adds materially different branch, exception, division, shift, mask, A7-bank, and privilege paths. Each case has a stable ID. The generated `opcode_coverage.csv` maps every legal primary opcode to its group and distinguishes an executed representative from an alias that was not executed. The current decoder yields 45,815 legal entries, 19,721 decode exceptions, and 4,556 candidate workloads using 2,065 selected primary encodings. These numbers describe this source snapshot, not an architectural constant.

The [fixture validator](../../../../crates/test_runner/benches/bench_instructions/candidate_probe.rs) builds bounded programs, normally with 700 target instructions; the two halting `STOP` paths use one target per block. It verifies target opcode and PC at each step, expected control path, target count, permitted writes, and observed CPU clocks. Most blocks end at an exit address, `STOP` blocks end in the stopped state, and `JMP` blocks end after exactly 700 counted jumps without guest helper instructions. It repeats each program and compares the complete final CPU and memory state, recording hashes of the prepared image/input and final state. This confirms that the benchmark executes its intended program consistently in this emulator. SingleStep vectors remain the independent source for CPU state, duration, and bus-callback comparison with the published model.

The [composition exporter](../../../../crates/test_runner/benches/bench_instructions/composition.rs) replays each validated fixture and writes `instruction_composition.json` with the target instruction, target count, total executed guest instructions, and counts of additional instructions. Candidate, probe, and opcode-coverage hashes identify its inputs. These counts expose helper work in a measured block; they do not subtract helper or host-loop costs to claim an isolated opcode time.

The [measurement runner](../../../../crates/test_runner/benches/bench_instructions/candidate_measure.rs) reconstructs and checks the validated fixture before timing. CPU and memory reset occur outside each timer. Timed execution includes the host `step_instruction` loop and any guest helper instructions needed to make the block terminate. A short count-calibration pilot selects a fixed block count for three series near 400 ms, without a separate warm-up; the runner retains each calibration sample. A dispersion above 5% triggers one additional three-series attempt, with all samples retained. Each full or retry session takes one NOP control at the start, with three series targeting about 1.2 s each; an unstable control stops the session before candidate timing. The runner does not sample host CPU load or wait for an idle threshold. It records normal process priority, verified affinity where available, and the active power plan; it does not change the system power plan. One-target `STOP` blocks reach the five-million-block cap and are explicitly marked as shorter series.

The primary result is **host nanoseconds per validated target in the whole workload**. It includes executed guest helpers, so it is not isolated opcode latency. Observed `step_instruction` counts are **CPU clocks**, not CCK; the old catalog's estimated CCK annotations are not substituted for measured clocks. The [analysis tool](../../../../tools/verification/analyze_candidate_benchmarks.py) checks the raw JSONL against the fixture manifest and selects invalid cases plus matching-fixture raw-time differences above 3% in either direction for retry. The retry keeps each measurement attempt, accepts agreement within 3% between two measurements, and stops after at most three new attempts. An unresolved case remains invalid. The final analysis writes `cases.csv`, `controls.csv`, comparison tables, `summary.json`, and a readable report under `.agent/tmp/benchmarks/inventory/full_<run-id>/`. Only a complete validated analysis with no pending retry updates `latest_candidate_run.json`. It accepts a targeted retry session only when candidate and validation hashes match. Different sessions retain their own NOP controls; addressing-mode and size deltas use only same-session cases. The tracked baseline contains the merged results from `full_1791958071` and `retry_1791964545`, with source and raw-series hashes in its version-2 JSON. The analyzer compares each current case with the version-2 baseline when the executed fixture, target count, CPU clocks, host OS, architecture, power plan, priority, affinity, and NOP control policy match. It writes `baseline_comparison.csv`, `anomalies.csv`, and `historical_comparison.json`. It reports both raw host-time changes and target-to-NOP-ratio changes. A ratio increase above 3% is a warning and above 7% is a regression signal, not a correctness failure. The retained 4,571-case baseline uses the earlier control-every-100 policy, so a fresh once-at-start run uses its raw times only to select provisional retries; it cannot receive comparable NOP-adjusted regression labels until a new baseline is explicitly approved. The former 108-case baseline lacks the fixture and source provenance needed for this comparison.

The recorded baseline predates the counted-JMP fixture change. Its JMP fingerprints differ from the current workloads, so those rows cannot produce direct timing comparisons. Reanalyzing the old raw run against the current probe manifest also fails fixture validation; the retained hashes identify the historical source and derived results instead.

The measured scope is deliberately bounded. Decode exceptions are excluded; legal register/operand aliases are represented rather than individually executed. Odd-address faults, global trace behavior, asynchronous interrupt acceptance, and every possible register alias are not claimed as benchmarked paths. The generated `path_scope.json` records these dispositions. Those behaviors require dedicated correctness or machine-level evidence before being added as performance cases.

## Hand-written assembly cases

The [assembly case catalog](../../../../benchmarks/m68k/README.md) selects instruction forms, addressing modes, and execution paths for direct workload measurements. Each case contains `program.s` and `case.json`; shared settings live in `config.json`. The [shared runner](../../../../benchmarks/m68k/runner.rs) substitutes the target count, assembles the program with the pinned external assembler, loads the entire image into Fast RAM, and restores configured CPU and memory state between complete runs. The [assembler manifest](../../../../tools/bootstrap/assembler-manifest.json) pins release and executable hashes, which the runner checks before assembly.

The runner takes the target count from each case's declared loop structure, observes the total guest instruction count and CPU clocks before timing, and derives the helper count by subtraction. Timed runs must retain the observed totals. The measured duration includes the complete program and its helpers. Nanoseconds per target therefore describes host cost for that whole workload. These case reports do not update or compare against the tracked decoder-derived baseline and do not assert final CPU correctness against silicon.

```powershell
.\tools\bootstrap\bootstrap.ps1 -Assembler
cargo bench -p test_runner --bench bench_instructions -- --measure-asm add/add_w_d1_postincrement
```

```powershell
cargo bench -p test_runner --bench bench_instructions -- --inventory
python tools/verification/benchmark_candidates.py
cargo bench -p test_runner --bench bench_instructions -- --probe-candidates
cargo bench -p test_runner --bench bench_instructions -- --export-composition
cargo bench -p test_runner --bench bench_instructions -- --measure-candidates
python tools/verification/analyze_candidate_benchmarks.py --prepare-retry-latest
cargo bench -p test_runner --bench bench_instructions -- --retry-candidates
python tools/verification/analyze_candidate_benchmarks.py .agent/tmp/benchmarks/inventory/full_<run-id> --retry-dir .agent/tmp/benchmarks/inventory/retry_<run-id>
```

## Interpreting a comparison

The baseline is a preserved measurement of one source snapshot, not a timing
requirement for every host. Compare only rows with matching executed fixture
fingerprints and observed CPU clocks. A changed fingerprint requires a workload
review before timing conclusions. NOP adjustment reduces some shared host-speed
variation but cannot remove all scheduling, thermal, or host processor effects.
One initial NOP cannot reveal later frequency or thermal drift during a full run.
Investigate a flagged case by checking its selected series, session NOP control,
source changes, and a repeat measurement under comparable conditions. The
addressing-mode and size tables compare cases from the same measurement session;
retried cases remain separate from those cross-case deltas.

## References

- [Benchmark harness](../../../../crates/test_runner/benches/bench_instructions): decoder inventory, fixture validation, and host timing.
- [Assembly case catalog](../../../../benchmarks/m68k/README.md): case format, assembler provisioning, and whole-program measurement.
- [Candidate generator](../../../../tools/verification/benchmark_candidates.py) and [analysis tool](../../../../tools/verification/analyze_candidate_benchmarks.py): coverage selection, report validation, and baseline comparison.
- [Benchmark verification commands](../../../../tools/verification/README.md): current invocation and output workflow.
- [CPU SingleStep verification](../Verification/CPU%20SingleStepTests.md): independent state, cycle, and bus-callback correctness evidence.
- [Motorola 68000 instruction timing tables](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): hardware timing context, separate from host wall time.
