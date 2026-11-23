---
title: "M68000 SingleStep Verification"
aliases: ["SingleStepTests", "Tom Harte Tests"]
tags: ["amiga", "design", "m68000", "singlestep", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-11-23
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU Instruction Benchmarking.md](../Performance/CPU%20Instruction%20Benchmarking.md)"]
tracked_paths:
  - "crates/test_runner/tests"
  - "crates/test_runner/Cargo.toml"
  - "tools/verification/run_singlestep.py"
---

# M68000 SingleStep Verification

Each SingleStep test starts the CPU in a supplied state and runs one M68000
instruction on an isolated CPU test bus. It compares the resulting processor
state, total instruction clocks, and the order and timing of memory reads and
writes with a generated reference record. The repository has 124 instruction
JSON files containing more than a million cases.
[Tom Harte's 68000 dataset](../../../../ref_src/SingleStepTests-680x0/68000/v1)

The upstream project says it generated the cases with a software
implementation and checked that implementation against documentation, other
test sets and an emulated machine. The author confirmed that the records come
from software emulation. The comparison uses those records on an isolated
test bus. It includes neither direct measurements from a physical 68000 nor
execution of a complete Amiga system.
[Upstream methodology](https://github.com/SingleStepTests/680x0#methodology),
[author's confirmation](https://github.com/SingleStepTests/ProcessorTests/issues/27)

## 1. What a SingleStep case contains

One step starts with the instruction address, registers, two prefetched words
and selected RAM bytes. The CPU executes the instruction, including its bus
activity and any exception it raises. The same reference case supplies the
expected final state and bus timeline. Each file groups cases by instruction
or instruction family. A sampled run executes fewer cases, using the same
comparisons for each case.

### 1.1 JSON records

Each file is an array of records with five top-level fields.

| Field | Meaning |
| --- | --- |
| `name` | Human-readable case identifier; it has no execution semantics. |
| `initial` | Starting data and address registers, user and supervisor stack pointers, status register, formal instruction PC, two prefetch words, and `[address, byte]` RAM entries. |
| `final` | Expected registers, status, formal next PC, and prefetch words; the source also includes RAM entries. |
| `length` | Expected instruction duration in CPU clocks, assuming immediate DTACK. |
| `transactions` | Ordered bus and internal intervals that place memory operations on the instruction timeline. |

A bus interval such as `["r", 4, 6, 3076, ".w", 1657]` gives the operation,
duration in CPU clocks, function code, address, byte or word size, and
transferred data in that order. `"w"` denotes a write. `["n", duration]`
advances time without an external transfer. A `"t"` interval describes a TAS
read-modify-write operation and supplies the written byte. The complete
sequence of interval durations must add up to `length`. The upstream format
defines the record fields and their hardware meaning.
[Upstream format description](../../../../ref_src/SingleStepTests-680x0/68000/v1/README.md)

### 1.2 Input Scope and Complete Verification

The test sets the CPU's starting state from `initial`, including both prefetch
words, and fills an isolated memory fixture from `initial.ram`. It advances the
CPU in Color Clock steps until the instruction finishes. Every executed case
compares the data and address registers, both stack pointers, status register,
formal final PC and both final prefetch words. Every case also compares
`length` and timed memory callbacks. The current runner always performs all of
these comparisons.

For each successful read or write, the transaction comparison checks direction,
address, size, data, order and callback time. The expected timeline includes
internal `"n"` intervals, so the comparator can detect a transfer at the wrong
point even when the total clock count matches. The comparator expands TAS into
the expected read and write callbacks. Successful writes are recorded after
the test bus commits them. The runner retains `final.ram` from the reference format.
Comparing it with the same fixture would supply no independent evidence about
the transfer, so it is outside the CPU assertions.

The override catalog lists reviewed corrections for specific reference
records. The upstream JSON files stay intact, and reports identify executed
overrides separately.
[SingleStep override catalog](../../../../crates/test_runner/tests/overrides/singlestep/README.md)

Random generation leaves some execution paths uncovered. The supplied [`DBcc.json`](../../../../ref_src/SingleStepTests-680x0/68000/v1/DBcc.json) includes 12-clock condition-true cases, but no case starts with a zero low counter word, so it cannot exercise the 14-clock counter-expired path. [Motorola User's Manual, Table 8-9](../../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md) specifies its duration and three reads; a [focused CPU test](../../../../crates/cpu/tests/test_dbcc.rs) checks the speculative branch-target read, fallthrough prefetch, and wait-state timing. The supplied `DIVU` and `DIVS` cases never produce a zero quotient; [ordinary CPU regressions](../../../../crates/cpu/tests/test_div_zero_quotient.rs) check that result and its timed prefetch separately.

## 2. Timing and observation boundary

The CPU advances by Color Clock (CCK). One CCK is two CPU clocks. The test
records when the CPU calls the bus, with read callbacks in CCK1 and write
callbacks in CCK2. It checks each callback against the reference timeline at
two-CPU-clock resolution and compares the total instruction duration in CPU
clocks. The comparison can detect a memory operation in the wrong CCK even
when the instruction ends on time.
[CPU micro-step model](../CPU%20Micro-Step%20State%20Machine.md)

The callbacks record activity at the modeled CPU-to-bus interface. The test
does not measure electrical pin edges or independently observe function-code
or byte-strobe pins. The callbacks also cannot establish uninterrupted bus
ownership during TAS. The runner parses function codes from JSON and has no
observed signal to compare them with. SingleStep does not exercise production
MemoryBus arbitration or independently establish that production memory
commits writes.

## 3. Where and how we run it

The [`test_runner` integration target](../../../../crates/test_runner/tests/test_singlestep.rs)
loads the JSON vectors and runs them against the CPU and its isolated bus.
The aggregate test groups instruction families and finalizes the results
once the selected groups finish. The verification frontend defines the command
options, execution profiles, parallelism and how to isolate result directories.
[Verification frontend](../../../../tools/verification/README.md#saved-singlestep-reports)

```powershell
# Focused diagnostic sample for one instruction group
python tools/verification/verify.py test runner singlestep case test_add_b sample 50

# All vectors in the selected SingleStep suites
python tools/verification/verify.py test runner singlestep full
```

Milestone verification runs the full SingleStep suite. Engineers can also run
it on explicit request or for diagnosis. Ordinary commit verification runs
standard test targets when a change affects `test_runner` or calls for full
Rust verification. The full reference dataset is outside that profile. Sampled
and full SingleStep runs use the same state, clock and bus comparisons.
[Testing strategy](Testing%20Strategy%20and%20Quality%20Assurance.md#completion-entry-point)

## 4. Checks around the comparator

The ordinary mutation tests alter selected JSON reference cases and pass them
through the normal loader and CPU comparator. They check that changing state,
transfer or timing expectations produces the corresponding failure and that
the loader rejects malformed records. Control changes, such as renaming a case
or editing `final.ram`, show which fields the comparison intentionally ignores.
The results cover only the selected JSON mutations and instruction paths.
[JSON mutation tests](../../../../crates/test_runner/tests/test_singlestep_mutations.rs)

Ordinary wait-state tests complement the reference vectors. A
PhysicalMemory-backed fixture refuses selected transfers so the test can
check that the CPU retries before advancing the active step. Reads wait in CCK1
and writes in CCK2. Each refusal adds two CPU clocks. These tests cover selected
instructions and transfer patterns. MemoryBus integration tests exercise the
production memory path separately.
[CPU wait-state tests](../../../../crates/cpu/tests/test_cck_bus.rs)

## 5. Failure diagnostics and saved reports

For a failure, the runner identifies the JSON file and vector and reports
expected and actual CPU state, clock count and bus-transaction differences.
Status flags and register differences help locate the mismatch. Standalone and
milestone runs default to `.agent/tmp/singlestep/` when `SINGLESTEP_RESULTS_DIR`
is unset. Pass `--results-dir PATH` to the verifier to save SingleStep reports
in `PATH/singlestep/`. The verification reference explains where to find archived
evidence.
[Verification reference](../../../../tools/verification/README.md#saved-singlestep-reports)

The selected directory contains these artifacts.

| Artifact | Use |
| --- | --- |
| `latest/<suite>.json`, `previous/<suite>.json` | Current saved suite outcome and its prior snapshot. |
| `summary.json`, `summary.md` | Saved suite totals, failures, and coverage overview. |
| `diff.md` | Changes in case status against a comparable previous selection. |

The saved summary can retain results for older suites after a filtered run.
Check the current run's metadata and selection before treating the aggregate
totals as fresh results. An interrupted run cannot finalize its reports. The
verification reference explains how to interpret the reports and lists
diagnostic commands.
[Verification reference](../../../../tools/verification/README.md#saved-singlestep-reports)

A saved report fails to load if it lacks a required field such as `selection`.
A report with an explicitly empty selection can parse, but is unusable as a
baseline for comparison.

## 6. Performance measurements

The CPU performance tests have their own target in
`crates/test_runner/benches/bench_instructions/`. They time complete assembly
programs on the host, including helpers and stepping overhead. SingleStep
uses JSON timings to check whether an emulated instruction matches its
generated reference. Host-speed measurements come from running the
performance workloads.
[CPU performance tests](../Performance/CPU%20Instruction%20Benchmarking.md)

The benchmark specification explains what the current benchmarks measure, how
they observe load, what the reports contain and how to compare compatible
runs. The decoder inventory remains available for coverage auditing without
measuring generated instruction workloads.
[Benchmark specification](../Performance/CPU%20Instruction%20Benchmarking.md)

## 7. What the layers establish

SingleStep covers a broad set of software-generated CPU cases with exact
final-state and timed bus-callback comparisons. Mutation tests check that the
comparator detects selected failures. Ordinary CPU tests use controlled
fixtures to check waiting and memory interactions. MemoryBus integration tests
exercise the production memory path. A passing result establishes only what
the tests in that run observed and asserted.

## 8. Ordinary CPU Wait-State Tests

The [wait-state checks described above](#4-checks-around-the-comparator) run
with ordinary CPU crate tests. They complement SingleStep without replaying its
vectors or enumerating production DMA schedules.

```powershell
python tools/verification/verify.py test rust crate cpu
```

## Reference Documentation and Test Entrypoints

- [Tom Harte 68000 vectors](../../../../ref_src/SingleStepTests-680x0/68000/v1): Software-generated cases and the source JSON format.
- [Upstream 680x0 methodology](https://github.com/SingleStepTests/680x0#methodology): Describes how the reference cases were generated and checked.
- [CPU design](../CPU%20Motorola%20M68000.md): Register, exception, and instruction semantics compared by the tests.
- [CPU micro-step model](../CPU%20Micro-Step%20State%20Machine.md): CCK phases and bus timing represented by the callbacks.
- [SingleStep integration target](../../../../crates/test_runner/tests/test_singlestep.rs): Living test entry point and instruction-group selection.
- [SingleStep override catalog](../../../../crates/test_runner/tests/overrides/singlestep/README.md): Reviewed record corrections and their evidence.
- [Verification frontend](../../../../tools/verification/README.md): Commands, profiles, reports, and completion policy.
- [CPU instruction benchmarking](../Performance/CPU%20Instruction%20Benchmarking.md): Separate host-performance measurement model.
