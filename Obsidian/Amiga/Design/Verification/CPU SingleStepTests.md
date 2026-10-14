---
title: "M68000 SingleStep Verification"
aliases: ["SingleStepTests", "Tom Harte Tests"]
tags: ["amiga", "design", "m68000", "singlestep", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-08-31
updated: 2026-10-14
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU Instruction Benchmarking.md](../Performance/CPU%20Instruction%20Benchmarking.md)"]
tracked_paths:
  - "crates/test_runner/tests"
  - "crates/test_runner/Cargo.toml"
  - "tools/verification/run_singlestep.py"
---

# M68000 SingleStep Verification

SingleStep tests execute one M68000 instruction from a supplied initial state and compare the result with a record captured from a physical 68000. They check the processor state, total instruction clocks, and the order and timing of memory reads and writes. The reference vectors live in [Tom Harte's 68000 dataset](../../../../ref_src/SingleStepTests-680x0/68000/v1); the repository has 124 instruction JSON files containing more than a million cases. This is CPU verification through an isolated test bus, not a run of the complete Amiga machine.

## 1. What a SingleStep case contains

One step starts with the instruction address, registers, two prefetched words, and selected RAM bytes. The CPU executes that instruction, including its bus activity and any exception it raises. The expected final state and bus timeline come from the same reference case. A file groups many cases by instruction or instruction family; selecting a sample changes the number of cases executed, not the comparisons made for each case.

### 1.1 JSON records

Each file is an array of records with five top-level fields:

| Field | Meaning |
| --- | --- |
| `name` | Human-readable case identifier; it has no execution semantics. |
| `initial` | Starting data and address registers, user and supervisor stack pointers, status register, formal instruction PC, two prefetch words, and `[address, byte]` RAM entries. |
| `final` | Expected registers, status, formal next PC, and prefetch words; the source also includes RAM entries. |
| `length` | Expected instruction duration in CPU clocks, assuming immediate DTACK. |
| `transactions` | Ordered bus and internal intervals that place memory operations on the instruction timeline. |

A bus interval has the form `["r", 4, 6, 3076, ".w", 1657]`: operation, duration in CPU clocks, function code, address, byte or word size, and transferred data. `"w"` denotes a write. `["n", duration]` advances time without an external transfer. A `"t"` interval describes the TAS read-modify-write operation and supplies the written byte. The complete sequence of interval durations must equal `length`. The [upstream format description](../../../../ref_src/SingleStepTests-680x0/68000/v1/README.md) defines the record fields and their hardware meaning.

### 1.2 Input Scope and Complete Verification

The test primes the CPU from `initial`, including both prefetch words, and initializes an isolated memory fixture from `initial.ram`. It then advances the CPU in Color Clock steps until the instruction finishes. Every executed case compares the data and address registers, both stack pointers, status register, formal final PC, and both final prefetch words. It also compares `length` and the timed memory callbacks. There is no state-only mode in the current SingleStep runner.

The transaction comparison checks each successful read or write for direction, address, size, data, order, and callback time. Internal `"n"` intervals remain in the expected timeline, so a correct total clock count cannot hide a read or write at the wrong point. TAS is expanded into its expected read and write callbacks. Successful writes are recorded after the test bus commits them; `final.ram` is retained from the reference format but is not a separate CPU assertion, because comparing it with the same fixture would not independently verify the CPU transfer.

Reviewed corrections for exact reference records are listed in the [SingleStep override catalog](../../../../crates/test_runner/tests/overrides/singlestep/README.md). The upstream JSON files remain intact, and reports identify executed overrides separately.

## 2. Timing and observation boundary

The [CPU micro-step model](../CPU%20Micro-Step%20State%20Machine.md) advances by Color Clock (CCK): one CCK is two CPU clocks. The test records when the CPU calls the bus, with read callbacks in CCK1 and write callbacks in CCK2. It checks those observed moments against the reference timeline at two-CPU-clock resolution, while comparing the total instruction duration in CPU clocks. Thus it can catch a memory operation in the wrong CCK even when the instruction ends on time.

These callbacks describe the modeled CPU-to-bus interface. They do not measure electrical pin edges, independently observe function-code or byte-strobe pins, or prove uninterrupted bus ownership during TAS. Function codes are parsed from JSON but are not compared with an observed signal. SingleStep also does not exercise production MemoryBus arbitration or independently establish that production memory commits writes.

## 3. Where and how we run it

The [`test_runner` integration target](../../../../crates/test_runner/tests/test_singlestep.rs) loads the JSON vectors and runs them against the CPU and its isolated test bus. The aggregate test groups instruction families and finalizes results after the selected groups finish. The [verification frontend](../../../../tools/verification/README.md#saved-singlestep-reports) owns command options, execution profiles, parallelism, and result-directory isolation.

```powershell
# Focused diagnostic sample for one instruction group
python tools/verification/verify.py test runner singlestep case test_add_b sample 50

# All vectors in the selected SingleStep suites
python tools/verification/verify.py test runner singlestep full
```

The full SingleStep run belongs to milestone verification and is available on explicit request or for diagnosis. Ordinary commit verification runs the standard test targets but excludes the full silicon dataset. Both sampled and full SingleStep runs use the same state, clock, and bus comparisons.

## 4. Checks around the comparator

The ordinary [JSON mutation tests](../../../../crates/test_runner/tests/test_singlestep_mutations.rs) alter selected silicon cases and send them through the normal loader and CPU comparator. They check that changed state, transfer, or timing expectations produce the relevant failure, and that malformed records are rejected. Control changes such as a renamed case or changed `final.ram` establish which fields are intentionally outside the CPU comparison. This is a bounded check of the test machinery, not a claim that every possible JSON mutation or instruction path has been exercised.

Ordinary [CPU wait-state tests](../../../../crates/cpu/tests/test_cck_bus.rs) complement the silicon vectors. They use a PhysicalMemory-backed fixture to refuse selected transfers, then check that the CPU retries without prematurely advancing its active step. Reads wait in CCK1 and writes in CCK2; each refusal adds two CPU clocks. These tests cover selected instructions and transfer patterns, while separate MemoryBus integration tests exercise the production memory path.

## 5. Failure diagnostics and saved reports

A failing case identifies its JSON file and vector, then reports expected and actual CPU state, clock count, and bus-transaction differences. Status flags and register differences help locate the mismatch. The runner writes results under `.agent/tmp/singlestep/`:

| Artifact | Use |
| --- | --- |
| `latest/<suite>.json`, `previous/<suite>.json` | Current saved suite outcome and its prior snapshot. |
| `summary.json`, `summary.md` | Saved suite totals, failures, and coverage overview. |
| `diff.md` | Changes in case status against a comparable previous selection. |

Filtered runs can leave older suites in the saved summary. Read the current-run metadata and selection before treating aggregate totals as fresh results. An interrupted run cannot finalize its reports. The [verification reference](../../../../tools/verification/README.md#saved-singlestep-reports) documents report interpretation and diagnostic commands.
Saved reports without required fields such as `selection` fail to load; a parsed report with an explicit empty selection is not a comparable baseline.

## 6. Performance measurements

The [CPU instruction benchmark](../Performance/CPU%20Instruction%20Benchmarking.md) is a separate target in `crates/test_runner/benches/bench_instructions/`. It executes generated instruction workloads to measure host execution time; it does not turn SingleStep JSON timings into a host-speed benchmark. The SingleStep result answers whether the emulated instruction matches the hardware reference. The benchmark answers how much host time the emulator spends on its instruction workloads.

The decoder-derived inventory, validation, measurement, and report commands are in the [benchmark specification](../Performance/CPU%20Instruction%20Benchmarking.md#decoder-derived-benchmark). `verify.py runner bench quick` still runs the historical 108-case profile and writes `m68k_benchmark.csv` and `analysis_report.md` under `.agent/tmp/benchmarks/quick/`; those results are not directly comparable with the new workload measurements.

## 7. What the layers establish

SingleStep provides broad, hardware-derived CPU cases with exact final-state and timed bus-callback comparisons. Mutation tests check selected failure-detection paths of that comparison. Ordinary CPU tests probe waiting and memory interactions with controlled fixtures, and MemoryBus integration tests cover the production memory path. These layers support different claims; passing one layer does not imply that the others ran.

## 8. Ordinary CPU Wait-State Tests

CPU wait tests belong to the ordinary crate verification path. They author expected transfer sequences and scripted refusals, then assert retry timing, unchanged state during a refusal, and the eventual committed transfer. They do not replay the million silicon vectors or enumerate every memory placement and DMA schedule. Production bus arbitration requires its own integration evidence.

```powershell
python tools/verification/verify.py test rust crate cpu
```

## Reference Documentation and Test Entrypoints

- [Tom Harte 68000 vectors](../../../../ref_src/SingleStepTests-680x0/68000/v1): Physical-processor cases and the source JSON format.
- [CPU design](../CPU%20Motorola%20M68000.md): Register, exception, and instruction semantics compared by the tests.
- [CPU micro-step model](../CPU%20Micro-Step%20State%20Machine.md): CCK phases and bus timing represented by the callbacks.
- [SingleStep integration target](../../../../crates/test_runner/tests/test_singlestep.rs): Living test entry point and instruction-group selection.
- [SingleStep override catalog](../../../../crates/test_runner/tests/overrides/singlestep/README.md): Reviewed record corrections and their evidence.
- [Verification frontend](../../../../tools/verification/README.md): Commands, profiles, reports, and completion policy.
- [CPU instruction benchmarking](../Performance/CPU%20Instruction%20Benchmarking.md): Separate host-performance measurement model.
