---
title: "M68000 Instruction Benchmarking"
aliases: ["CPU Instruction Benchmarking", "Instruction Benchmarking Architecture", "CPU Benchmark Analysis Guide", "Instruction Benchmark Catalog", "Benchmark Strategies"]
tags: ["amiga", "design", "m68000", "benchmark", "verification"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-10-14
related: ["[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)", "[CPU SingleStepTests.md](../Verification/CPU%20SingleStepTests.md)", "[Performance Profiling and Optimization Strategy.md](Performance%20Profiling%20and%20Optimization%20Strategy.md)"]
tracked_paths:
  - "crates/test_runner/Cargo.toml"
  - "crates/test_runner/benches"
  - "tools/verification/analyze_benchmarks.py"
  - "tools/verification/benchmark_candidates.py"
  - "tools/verification/analyze_candidate_benchmarks.py"
---

# M68000 Instruction Benchmarking

The decoder-derived benchmark measures host cost for representative M68000 instruction forms through the cycle-stepped CPU and mapped `PhysicalMemory`. It is a performance experiment, not a proof that the CPU matches silicon. The retained 108-case catalog runner and baseline remain available as historical evidence; their timings do not share the new workload fingerprints.

The independent [SingleStep verification](../Verification/CPU%20SingleStepTests.md) supplies that correctness evidence: it compares CPU state, instruction duration, and timed bus callbacks with silicon-derived vectors on an isolated test bus. The [CPU model](../CPU%20Motorola%20M68000.md) and [micro-step specification](../CPU%20Micro-Step%20State%20Machine.md) define intended behavior; this benchmark reports how much host time the implementation spends executing it. Neither test runs the complete Amiga machine or establishes whole-system throughput.

## Decoder-derived benchmark

The [inventory](../../../../crates/test_runner/benches/bench_instructions/inventory.rs) enumerates the decoder's legal primary opcodes, excluding decode-exception entries, and groups entries by static micro-step sequence. The [candidate generator](../../../../tools/verification/benchmark_candidates.py) splits condition codes and MOVEM size/direction, selects representative register and operand encodings, and adds materially different branch, exception, division, shift, mask, A7-bank, and privilege paths. Each case has a stable ID. The generated `opcode_coverage.csv` maps every legal primary opcode to its group and distinguishes an executed representative from an alias that was not executed. The current decoder yields 45,821 legal entries, 19,715 decode exceptions, and 4,571 candidate workloads using 2,071 selected primary encodings. These numbers describe this source snapshot, not an architectural constant.

The [fixture validator](../../../../crates/test_runner/benches/bench_instructions/candidate_probe.rs) builds bounded programs, normally with 700 target instructions; the two halting `STOP` paths use one target per block. It verifies target opcode and PC at each step, expected control path, target count, final exit or stopped state, permitted writes, and observed CPU clocks. It repeats each program and compares the complete final CPU and memory state, recording hashes of the prepared image/input and final state. This confirms that the benchmark executes its intended program consistently in this emulator. SingleStep vectors remain the independent source for CPU state, duration, and bus-callback correctness against silicon.

The [measurement runner](../../../../crates/test_runner/benches/bench_instructions/candidate_measure.rs) reconstructs and checks the validated fixture before timing. CPU and memory reset occur outside each timer. Timed execution includes the host `step_instruction` loop and any guest helper instructions needed to make the block terminate. A short count-calibration pilot selects a fixed block count for three series near 400 ms, without a separate warm-up; the runner retains each calibration sample. A dispersion above 5% triggers one additional three-series attempt, with all samples retained. A same-session NOP control runs every 100 cases. The runner records normal process priority, verified affinity where available, host idle activity, and the active power plan; it does not change the system power plan. One-target `STOP` blocks reach the five-million-block cap and are explicitly marked as shorter series.

The primary result is **host nanoseconds per validated target in the whole workload**. It includes executed guest helpers, so it is not isolated opcode latency. Observed `step_instruction` counts are **CPU clocks**, not CCK; the old catalog's estimated CCK annotations are not substituted for measured clocks. The [analysis tool](../../../../tools/verification/analyze_candidate_benchmarks.py) checks the raw JSONL against the fixture manifest, writes `cases.csv`, `controls.csv`, comparison tables, `summary.json`, and a readable report under `.agent/tmp/benchmarks/inventory/full_<run-id>/`. A complete validated analysis updates `latest_candidate_run.json` in the inventory directory. It accepts a targeted retry session only when candidate and validation hashes match. Different sessions retain their own NOP controls; addressing-mode and size deltas use only same-session cases. Protected baselines remain unchanged, and the 108-case baseline lacks the fixture and source provenance needed for direct comparison.

The measured scope is deliberately bounded. Decode exceptions are excluded; legal register/operand aliases are represented rather than individually executed. Odd-address faults, global trace behavior, asynchronous interrupt acceptance, and every possible register alias are not claimed as benchmarked paths. The generated `path_scope.json` records these dispositions. Those behaviors require dedicated correctness or machine-level evidence before being added as performance cases.

```powershell
cargo bench -p test_runner --bench bench_instructions -- --inventory
python tools/verification/benchmark_candidates.py
cargo bench -p test_runner --bench bench_instructions -- --probe-candidates
cargo bench -p test_runner --bench bench_instructions -- --measure-candidates
cargo bench -p test_runner --bench bench_instructions -- --retry-candidates
python tools/verification/analyze_candidate_benchmarks.py .agent/tmp/benchmarks/inventory/full_<run-id> --retry-dir .agent/tmp/benchmarks/inventory/retry_<run-id>
```

## Historical 108-case catalog runner

The following sections describe the retained catalog and its old timing/reporting path. They remain useful for interpreting historical data but do not establish coverage or timing validity for the decoder-derived benchmark.

### 1. What we measure and why

| Evidence | Workload or comparison | Question it answers | Limit |
| --- | --- | --- | --- |
| Instruction benchmarks | Repeated guest instruction sequences from the catalog | Which paths cost more host time, and did that cost change? | Generated setup, stepping, bus work, and control flow contribute to wall time. |
| Within-run comparisons | Register versus memory modes, byte/word/long sizes, related ALU operations, and distinct branch/division paths | Does a relative cost suggest an unusual dispatch, effective-address, bus, or exception path? | A difference is a diagnostic signal; it does not identify a root cause by itself. |
| Historical baseline comparison | Current result versus a retained JSON report for a matching variant | Is a change large enough to investigate? | Host hardware, build, temperature, and scheduling can alter timing. |
| SingleStep tests | One instruction against reference state, clocks, and bus callbacks | Does the CPU execute that case correctly? | Uses a synthetic bus rather than production arbitration. |
| Profiler | Sampling a concrete benchmark executable | Where does host time go within a slow workload? | Samples explain a measured cost; they do not establish guest correctness. |

The catalog selects representative registers such as `D0`, `D1`, `A0`, `A1`, `A6`, and `SP` to keep the suite practical. Its 108 entries are **selected variants**, not every register permutation, legal encoding, operand value, or instruction. Twelve effective-address forms are described below, but each instruction uses only forms legal and included for that instruction. Catalog CCK values are comparison metadata, including estimates for variable-duration operations; a row does not independently validate its own cycle count.

### 2. How a workload runs

The [catalog](../../../../crates/test_runner/benches/bench_instructions/catalog.rs) supplies an ID, syntax, opcode words, expected CCK count, category, and [construction strategy](../../../../crates/test_runner/benches/bench_instructions/builder.rs). The builder creates a program image in memory. The runner injects that image and initial registers into `Cpu` and `PhysicalMemory`, executes until the exit PC or another stopping condition, and repeats the program for timed passes. This is a synthetic CPU workload. It does not route accesses through the complete machine's DMA arbitration or peripherals.

The default unroll factor is $K=700$. A normal workload emits the target opcode 700 times, then jumps to the exit sentinel at `$004FFE`. The program starts at `$001000`; operand buffers begin at `$006000` and `$007000`; return chains use `$008000`; stack data starts near `$07F000`. Some workloads use a branch, a companion instruction, a return stack, or an exception handler, so **700 is a nominal target-operation count, not a guarantee that every block consists of 700 identical instructions**. The builder records `total_ops_per_pass = K` and `total_cck_per_pass = K × catalog CCK` for every strategy. These are declared counts, not an observed instruction or cycle trace.

Each timed iteration calls `BenchmarkProgram::inject_into` again, resetting CPU state and selected Chip RAM bytes. The timer in the [runner](../../../../crates/test_runner/benches/bench_instructions/runner.rs) surrounds these injections and guest execution. Consequently `host_ns_per_instruction` divides **whole-iteration time** by nominal target operations; it includes setup and any companion/handler instructions. The unrolled guest block reduces branch overhead within the block, but the result is not the isolated latency of one host ALU operation. `std::hint::black_box` keeps the CPU state observable to the compiler after iterations; it does not verify the final state.

#### 2.1 Operand and state strategies

The [deterministic PRNG](../../../../crates/test_runner/benches/bench_instructions/prng.rs) seeds initial data registers and fills operand buffers. The builder overrides those seeds where a path needs a particular condition. Re-injection gives each iteration the same initial state. Immediate operands come from catalog opcode words; they are not a freshly generated random stream for every emitted instruction.

| Workload family | Constructed condition | Why it matters and what remains in the timing |
| --- | --- | --- |
| Register moves and ALU | Repeated operations on representative registers; arithmetic wraps at the operand width. | Exposes dispatch and register paths, though repeated destructive operations can reach a stable value. |
| Memory modes and `CMPM` | Aligned pointers traverse PRNG-filled Chip RAM buffers. Post-increment and pre-decrement start at positions that leave room for the block. | Exercises effective-address and memory paths. Pointers are reset by the next injection, not by a guest loop footer. |
| `LINK`/`UNLK`, paired shifts, and terminal `DBcc` | The builder emits 350 pairs rather than 700 identical target operations. | Preserves a usable stack or operand/loop state; reported cost includes both members of the pair. |
| Logic, bit operations, and rotations | Catalog opcode words are repeated over seeded registers or memory. Selected left/right shifts use the paired construction; rotations and `SWAP` use the ordinary repeated block. | Exercises the selected decode and data paths, but the builder does not cycle bit indices or alternate fresh random masks within a block. Repeated destructive operations can change the operand distribution. |
| Comparisons and tests | Register `CMP`/`TST` cases repeat without changing operands; `CMPM` advances two seeded memory pointers. | Keeps simple comparisons stable while exercising a separate memory path for `CMPM`. It does not validate the resulting flags. |
| Multiply and divide | Valid unsigned division starts with `D1=$00A5`, `D0=$00050000`; signed division uses `D1=1`, `D0=$1234`. Overflow uses a high dividend word and small divisor. | Separates normal and early-overflow paths. Operand evolution and actual clocks need independent inspection when interpreting a result. |
| Divide by zero | Vector 5 points to a handler that decrements `D7` and uses `RTE` to repeat the fault until the count is exhausted. | Times exception entry **plus** handler, stack, and return activity. The generated entry uses `DIVU.W D1,D0`; catalog labels for both divide-by-zero variants are metadata, not proof of two distinct emitted faulting opcodes. |
| BCD and bounds check | BCD buffer bytes are generated as valid packed digits; `CHK` starts in bounds. | Measures selected valid-data paths rather than every carry or exception case. |
| Branches and calls | Taken branches skip inserted `NOP`s; untaken branches fall through. `BSR` uses an inline call/return sequence; `DBcc` has taken and terminal constructions. | Control-flow timing includes the selected layout and, for calls, return instructions. |
| `RTS`, `RTR`, `RTE` | Prepared stacks point each return at the next opcode and end at the exit PC. `RTR`/`RTE` use six-byte frames. | Exercises return or exception-return paths with no guest setup between returns; stack construction occurs during timed injection. |
| `TRAP #0` and privileged operations | A vector 32 handler contains `RTE`; system-register variants start in supervisor mode where required. | Measures the selected trap/privileged path and handler overhead; other exception classes described by the 68000 are not automatically benchmarked. |

For return chains, the stack contains target addresses for the next instruction and finally `$004FFE`. `RTS` pops four-byte PCs. `RTR` pops a CCR word and PC; `RTE` pops an SR word and PC. For divide-by-zero, the vector at `$000014` points to a handler at `$000500`; its `SUBQ`/`BEQ`/`RTE` loop and final stack adjustment make repeated faults terminate. These layouts are necessary to execute control-flow instructions repeatedly, and their overhead must be considered when comparing them with straight-line instructions.

The catalog's declared opcode words and the emitted program are different layers. The builder substitutes sequences for several strategies. For example, `FLOW-04` declares `6104` in [catalog data](../../../../crates/test_runner/benches/bench_instructions/catalog_data_b.rs), while the `SubroutineCall` builder emits `6102` inside its call/return block. The catalog table below retains the declared syntax and opcode metadata; use the builder or inspect an injected image to determine the actual timed instruction stream. Do not treat this mismatch as a validated reference correction.

#### 2.2 Sampling and termination

| Profile | Measured passes | Iterations per pass | Nominal target operations, if all passes finish |
| --- | ---: | ---: | ---: |
| `--quick` | 3 | 15 | 31,500 |
| `--standard` | 7 | 1,428 | 6,997,200 |
| `--thorough` | 15 | 14,285 | 149,992,500 |

For each specification the runner performs one unmeasured warm-up pass, then times the profile's passes. It computes the median and a 10% trimmed mean after applying Tukey fences when there are at least four samples. The coefficient of variation (CV) is calculated from the retained samples. If CV exceeds 3%, the runner waits 500 ms and retries up to twice, replacing the previous sample set rather than extending it. A final CV above 3% marks the environment noisy; above 5% also triggers a jitter anomaly. `quick` has only three samples, so its outlier filter cannot operate. A nominal operation count uses the raw number of **final** samples, not the filtered count.

The runner attempts to pin its thread to a selected core where supported and runs specifications sequentially. This is a host benchmark policy in [platform.rs](../../../../crates/test_runner/benches/bench_instructions/platform.rs), outside the portable emulation core. The pinning call's result is ignored, so the report's `p_core_pinning_active` field describes the requested/supported path, not confirmed OS affinity. Host load and thermal behavior still affect measurements. The runner stops a pass at an exit PC, a CPU stop/halt, zero clocks returned by a step, or a cycle cap based on ten times the nominal CCK budget (at least 1,000 CCK before multiplication). A cap hit produces a timeout anomaly; the mere absence of that flag does not prove that exactly $K$ target instructions executed.

### 3. Results, comparisons, and interpretation

The [persistence module](../../../../crates/test_runner/benches/bench_instructions/persistence.rs) writes JSON and CSV under `.agent/tmp/benchmarks/<profile>/`. It preserves the preceding canonical files as `_previous`, writes `m68k_benchmark.json` and `.csv`, and copies them to `_latest`. Generated files are local artifacts. Reviewed reference files live in `baselines/benchmarks/` and are protected by the [reference-artifact rule](../../../../.agents/rules/spec-compliance.md); a timing difference alone does not authorize editing them.

CSV has 14 columns: `timestamp`; seven declared or nominal fields (`mnemonic`, `variant`, `addressing_mode`, `category`, `opcode_hex`, `amiga_cck_cycles`, `total_guest_instructions`); and six host/result fields (`host_duration_median_ms`, `host_ns_per_instruction`, `host_ns_per_guest_cck`, `host_mips`, `host_jitter_pct`, `anomaly_flag`). The JSON report also contains environment metadata, nominal total CCK, min/max durations, a NOP differential, and anomaly notes. The CSV values are formatted to limited decimal precision, so use JSON or raw output for closer investigation. The `timestamp` identifies the run; it is not a deterministic invariant.

The key calculations are:

$$T_{\text{ns/op}}=\frac{\text{median pass nanoseconds}}{K\times\text{iterations}},\qquad R_{\text{norm}}=\frac{T_{\text{ns/op}}}{\text{catalog CCK/op}},\qquad \text{MIPS}=\frac{1000}{T_{\text{ns/op}}}.$$

`R_norm` expresses **host nanoseconds per declared guest CCK**, not measured Amiga clock accuracy. The NOP differential is `max(T_op − T_NOP, 0)` and is useful as a rough within-run comparison. The NOP and target workloads can have different memory, control-flow, or setup costs, so subtraction does not isolate an ALU or bus primitive.

The anomaly classifier uses the NOP `ns/CCK` value as its supplied family baseline and flags a result above 2.5 times that value, jitter above 5%, or a cycle-cap timeout. Its memory-mode comparison currently receives the NOP `ns/op` value as a register baseline but compares it with the target's `ns/CCK`: those units differ, so that Type B verdict is not a reliable measure of memory overhead. These flags identify candidates for investigation, not demonstrated cache, branch predictor, or inlining defects. The runner also loads a historical JSON baseline when available. Matching variants are compared by raw `ns/op`; when architecture/OS differs or NOP time differs by more than 15%, it instead compares the target-to-NOP ratios. Changes above 3% receive a warning, and above 7% receive a severe label and report anomaly. Ratio normalization reduces some clock-rate effects but cannot make different machines equivalent or make the benchmark fail the process solely for a performance regression.

#### 3.1 Questions to ask of a report

| Comparison | Calculation or observation | Interpretation check |
| --- | --- | --- |
| Functional symmetry | Compare `ADD.W`, `SUB.W`, `AND.W`, `OR.W`, `EOR.W`, and `CMP.W` register cases on the same run. | A large spread suggests a candidate path; confirm comparable instruction streams and noise before tracing code. |
| Addressing cost | $\Delta T_{\text{mode}}=T(\text{MOVE.W, mode})-T(\text{MOVE.W, Dn})$. | This includes guest cycle count, effective-address work, memory work, and setup; it is not a pure address-calculation cost. |
| Operand size | Compare `.B`, `.W`, and `.L` within the same instruction family and mode. | Different declared CCK and guest bus transfers can explain a size change. |
| Complex instructions | Compare raw `ns/op` and `R_norm` for `MOVEM`, multiply/divide, returns, and traps. | Handler, companion, and stack setup costs can dominate a nominal operation. |
| Cycle correlation | Fit $T_{\text{ns/op}}=\alpha\,C_{\text{catalog}}+\beta$ and inspect $R^2$. | Correlation describes this dataset; $\beta$ is not independently measured dispatch cost and a high $R^2$ does not validate cycle counts. |

The [analysis script](../../../../tools/verification/analyze_benchmarks.py) produces these tables and a complete result listing. Its thresholds for `R_norm` are presentation labels, not correctness gates. Some dimensions require a full catalog: a filtered or partial CSV may lack comparison members. Historical example numbers in `baselines/benchmarks/m68k_benchmark_baseline.csv` are a retained measurement from one environment, not a universal target or a current measurement. Investigate a change by repeating both revisions under comparable conditions, checking jitter and the actual guest sequence, and profiling the slow case.

For example, the retained CSV reports `MOVE.W D1,D0` at 16.20 ns/op (4 declared CCK), `MOVE.W (A0),D0` at 25.08 ns/op (8 CCK), and `MOVE.W 16(A0),D0` at 32.92 ns/op (12 CCK). The differences of 8.88 ns and 16.72 ns relative to register direct suggest increasing host cost with the selected effective-address paths. They do not isolate address calculation from guest clock stepping or timed injection. The same file reports `ADD.W D1,D0` at 15.96 ns/op and `SUB.W D1,D0` at 15.86 ns/op; that 0.10 ns difference is small compared with a cross-run performance claim. This example explains how to read a recorded dataset; its source run is not repeated or validated by this document.

### 4. How we verify the benchmark and CPU

1. **Check the workload definition.** Compare an ID, opcode words, addressing mode, category, strategy, and CCK metadata with the [living catalog parts](../../../../crates/test_runner/benches/bench_instructions/catalog_data_a.rs) and [part B](../../../../crates/test_runner/benches/bench_instructions/catalog_data_b.rs). The catalog contains 108 IDs. Inspect the builder for strategies that replace or supplement declared opcodes. This is a static consistency review, not behavioral proof.
2. **Check guest correctness separately.** Run focused SingleStep vectors for changed CPU behavior and the authoritative full suite when required by the project verification profile. The vectors compare state, clocks, and timed bus callbacks; the benchmark cannot replace them. For a memory or DMA change, use the relevant production-bus and machine tests as well, because this benchmark uses `PhysicalMemory` directly.
3. **Measure the affected workload.** Select one of the three supported profiles. A completed command means the harness produced files; inspect timeout and anomaly notes, row count, and whether each expected ID/variant is present. A cycle cap, `STOP`, or zero-clock exit needs investigation before accepting timing.
4. **Check report structure.** Compare CSV headers and rows with JSON results. Recompute nominal counts from $K ×$ iterations $×$ raw measured passes and derive `ns/op`, `ns/CCK`, and MIPS from the median. Compare declared fields with catalog data. Host durations, jitter, and anomaly verdicts can legitimately differ between runs. Whole-file checksums are unsuitable for generated reports.
5. **Interpret the performance evidence.** Review CV, machine/build context, NOP time, family and addressing comparisons, and a repeated baseline diff. Use a profiler for a suspected hot path. Do not rewrite a reference CSV or cycle expectation to turn a failed check into a pass; follow the [specification conflict procedure](../../../../.agents/rules/spec-compliance.md).

The benchmark does **not** currently assert the final register or memory state for each catalog row, compare an observed guest instruction count with the nominal count, or compare observed cycles with `amiga_cck_cycles`. Its metadata, timeout logic, and generated CSV therefore cannot independently establish execution correctness. A changed builder strategy deserves particular scrutiny because it can alter the timed sequence without changing the catalog row.

#### 4.1 Commands and artifacts

```powershell
# No profile prints help; a profile is required to measure.
cargo bench -p test_runner --bench bench_instructions -- --quick
cargo bench -p test_runner --bench bench_instructions -- --standard
cargo bench -p test_runner --bench bench_instructions -- --thorough

# The verification frontend measures and then analyzes a successful run.
python tools/verification/verify.py runner bench quick

# Analyze an existing CSV without remeasuring.
python tools/verification/verify.py runner analyze .agent/tmp/benchmarks/thorough/m68k_benchmark.csv report .agent/tmp/benchmarks/thorough/analysis_report.md
```

Direct `cargo bench` runs measurement and persistence. `verify.py runner bench` invokes that same Cargo target and, only after success, runs the Python analysis into `.agent/tmp/benchmarks/<profile>/analysis_report.md`. The [verification README](../../../../tools/verification/README.md) owns further command options and SingleStep procedures. The CLI accepts one of `--quick`, `--standard`, `--thorough`, or help; internal `BenchmarkConfig.filter` supports constructed subsets but is not a Cargo CLI option. A profile run is not part of ordinary task completion unless the changed behavior or user request calls for it.

### 5. Catalog: selected variants and addressing modes

The catalog table records **declared benchmark metadata**. It retains the 108 case IDs so a reader can map a report row to its strategy and the source definition. `~` marks a historical approximate CCK annotation; the executable catalog stores an integer. The code and its [Motorola timing reference](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md) must be reviewed before treating a value as an exact cycle requirement.

Data and address registers use canonical representatives: destination `D0`, source `D1`, source pointer `A0`, destination pointer `A1`, frame pointer `A6`, stack pointer `A7`, and index `D2.W`. This bounds the performance suite; it does not imply complete register-index decoding coverage.

| Mode | Example | Encoding `mmm/rrr` | Extension words |
| --- | --- | --- | --- |
| Data register direct | `Dn` | `000/register` | 0 |
| Address register direct | `An` | `001/register` | 0 |
| Address indirect | `(An)` | `010/register` | 0 |
| Post-increment | `(An)+` | `011/register` | 0 |
| Pre-decrement | `-(An)` | `100/register` | 0 |
| Displacement | `d16(An)` | `101/register` | 1 |
| Index | `d8(An,Xn)` | `110/register` | 1 |
| Absolute short | `(xxx).W` | `111/000` | 1 |
| Absolute long | `(xxx).L` | `111/001` | 2 |
| PC displacement | `d16(PC)` | `111/010` | 1 |
| PC index | `d8(PC,Xn)` | `111/011` | 1 |
| Immediate | `#data` | `111/100` | 1 or 2 |

#### Category 0: Calibration Baseline (`NOP`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **BASE-00** | `NOP` | — | Implied | `NOP` | `4E71` | 4 | `baseline` |

> [!TIP]
> `BASE-00` supplies the within-run NOP comparison. The differential includes workload setup and cannot isolate a pure instruction cost.

#### Category 1: Data Movement (`MOVE`, `MOVEM`, `EXG`, `LEA`, `PEA`, `LINK`, `UNLK`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **MOV-01** | `MOVE` | `.B` | Data Reg Direct | `MOVE.B D1, D0` | `1001` | 4 | `move_reg` |
| **MOV-02** | `MOVE` | `.W` | Data Reg Direct | `MOVE.W D1, D0` | `3001` | 4 | `move_reg` |
| **MOV-03** | `MOVE` | `.L` | Data Reg Direct | `MOVE.L D1, D0` | `2001` | 4 | `move_reg` |
| **MOV-04** | `MOVE` | `.W` | Address Indirect Read | `MOVE.W (A0), D0` | `3010` | 8 | `move_mem` |
| **MOV-05** | `MOVE` | `.W` | Post-increment Read | `MOVE.W (A0)+, D0` | `3018` | 8 | `move_mem` |
| **MOV-06** | `MOVE` | `.W` | Pre-decrement Read | `MOVE.W -(A0), D0` | `3020` | 10 | `move_mem` |
| **MOV-07** | `MOVE` | `.W` | Displacement Read | `MOVE.W 16(A0), D0` | `3028 0010` | 12 | `move_mem` |
| **MOV-08** | `MOVE` | `.W` | Index Read | `MOVE.W 8(A0, D2.W), D0` | `3030 2008` | 14 | `move_mem` |
| **MOV-09** | `MOVE` | `.W` | Absolute Short Read | `MOVE.W ($2000).W, D0` | `3038 2000` | 12 | `move_mem` |
| **MOV-10** | `MOVE` | `.W` | Absolute Long Read | `MOVE.W ($00002000).L, D0` | `3039 0000 2000` | 16 | `move_mem` |
| **MOV-11** | `MOVE` | `.W` | PC Displacement Read | `MOVE.W 8(PC), D0` | `303A 0008` | 12 | `move_mem` |
| **MOV-12** | `MOVE` | `.W` | Immediate Read | `MOVE.W #$1234, D0` | `303C 1234` | 8 | `move_imm` |
| **MOV-13** | `MOVE` | `.W` | Memory-to-Memory | `MOVE.W (A0)+, (A1)+` | `32D8` | 12 | `move_mem2mem` |
| **MOV-14** | `MOVEA` | `.W` | Data Reg to Addr Reg | `MOVEA.W D0, A0` | `3040` | 4 | `move_addr` |
| **MOV-15** | `MOVEA` | `.L` | Data Reg to Addr Reg | `MOVEA.L D0, A0` | `2040` | 4 | `move_addr` |
| **MOV-16** | `MOVEM` | `.W` | Reg to Mem (4 regs) | `MOVEM.W D0-D3, -(SP)` | `48A7 000F` | 24 | `move_multi` |
| **MOV-17** | `MOVEM` | `.W` | Mem to Reg (4 regs) | `MOVEM.W (SP)+, D0-D3` | `4CDF 000F` | 28 | `move_multi` |
| **MOV-18** | `EXG` | `.L` | Data Regs | `EXG D0, D1` | `C141` | 6 | `move_reg` |
| **MOV-19** | `EXG` | `.L` | Addr Regs | `EXG A0, A1` | `C149` | 6 | `move_reg` |
| **MOV-20** | `LEA` | `.L` | Displacement to Addr Reg | `LEA 16(A0), A1` | `43E8 0010` | 8 | `move_addr` |
| **MOV-21** | `PEA` | `.L` | Push Effective Address | `PEA 16(A0)` | `4868 0010` | 16 | `move_stack` |
| **MOV-22** | `LINK` | `.W` | Link Stack Frame | `LINK A6, #-16` | `4E56 FFF0` | 16 | `move_stack` |
| **MOV-23** | `UNLK` | `.L` | Unlink Stack Frame | `UNLK A6` | `4E5E` | 12 | `move_stack` |

---

#### Category 2: Integer Arithmetic (`ADD`, `SUB`, `MUL`, `DIV`, `NEG`, `CLR`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **ARITH-01** | `ADD` | `.B` | Data Reg Direct | `ADD.B D1, D0` | `D001` | 4 | `arith_reg` |
| **ARITH-02** | `ADD` | `.W` | Data Reg Direct | `ADD.W D1, D0` | `D041` | 4 | `arith_reg` |
| **ARITH-03** | `ADD` | `.L` | Data Reg Direct | `ADD.L D1, D0` | `D081` | 8 | `arith_reg` |
| **ARITH-04** | `ADD` | `.W` | Indirect Memory to Reg | `ADD.W (A0)+, D0` | `D058` | 8 | `arith_mem` |
| **ARITH-05** | `ADD` | `.W` | Reg to Indirect Memory | `ADD.W D0, (A0)` | `D150` | 12 | `arith_mem` |
| **ARITH-06** | `ADDA` | `.W` | Word to Addr Reg | `ADDA.W D0, A0` | `D0C0` | 8 | `arith_addr` |
| **ARITH-07** | `ADDA` | `.L` | Long to Addr Reg | `ADDA.L D0, A0` | `D0E0` | 8 | `arith_addr` |
| **ARITH-08** | `ADDQ` | `.W` | Quick Immediate to Reg | `ADDQ.W #4, D0` | `5840` | 4 | `arith_imm` |
| **ARITH-09** | `ADDX` | `.W` | Extended Add Regs | `ADDX.W D1, D0` | `D141` | 4 | `arith_reg` |
| **ARITH-10** | `ADDX` | `.W` | Extended Add Mem | `ADDX.W -(A1), -(A0)` | `D149` | 18 | `arith_mem` |
| **ARITH-11** | `SUB` | `.W` | Data Reg Direct | `SUB.W D1, D0` | `9041` | 4 | `arith_reg` |
| **ARITH-12** | `SUB` | `.L` | Data Reg Direct | `SUB.L D1, D0` | `9081` | 8 | `arith_reg` |
| **ARITH-13** | `SUBA` | `.W` | Word to Addr Reg | `SUBA.W D0, A0` | `90C0` | 8 | `arith_addr` |
| **ARITH-14** | `SUBQ` | `.W` | Quick Immediate to Reg | `SUBQ.W #4, D0` | `5940` | 4 | `arith_imm` |
| **ARITH-15** | `SUBX` | `.W` | Extended Sub Regs | `SUBX.W D1, D0` | `9141` | 4 | `arith_reg` |
| **ARITH-16** | `MULU` | `.W` | Unsigned Multiply Reg | `MULU D1, D0` | `C0C1` | ~54 | `arith_mul` |
| **ARITH-17** | `MULS` | `.W` | Signed Multiply Reg | `MULS D1, D0` | `C1C1` | ~54 | `arith_mul` |
| **ARITH-18** | `DIVU` | `.W` | Unsigned Divide Reg | `DIVU D1, D0` | `80C1` | ~140 | `arith_div` |
| **ARITH-18b**| `DIVU` | `.W` | Quotient Overflow Path | `DIVU D1, D0` (Overflow) | `80C1` | ~10 | `arith_div` |
| **ARITH-18c**| `DIVU` | `.W` | Divide-by-Zero Exception| `DIVU #0, D0` (Vector 5)| `80FC 0000` | ~42 | `arith_div` |
| **ARITH-19** | `DIVS` | `.W` | Signed Divide Reg | `DIVS D1, D0` | `81C1` | ~158 | `arith_div` |
| **ARITH-19b**| `DIVS` | `.W` | Quotient Overflow Path | `DIVS D1, D0` (Overflow) | `81C1` | ~10 | `arith_div` |
| **ARITH-19c**| `DIVS` | `.W` | Divide-by-Zero Exception| `DIVS #0, D0` (Vector 5)| `81FC 0000` | ~42 | `arith_div` |
| **ARITH-20** | `NEG` | `.W` | Negate Register | `NEG.W D0` | `4440` | 4 | `arith_reg` |
| **ARITH-21** | `NEGX` | `.W` | Negate with Extend Reg | `NEGX.W D0` | `4040` | 4 | `arith_reg` |
| **ARITH-22** | `CLR` | `.W` | Clear Register | `CLR.W D0` | `4240` | 4 | `arith_reg` |
| **ARITH-23** | `CLR` | `.W` | Clear Memory | `CLR.W (A0)+` | `4258` | 8 | `arith_mem` |
| **ARITH-24** | `EXT` | `.W` | Sign Extend Byte to Word | `EXT.W D0` | `4880` | 4 | `arith_reg` |
| **ARITH-25** | `EXT` | `.L` | Sign Extend Word to Long | `EXT.L D0` | `48C0` | 4 | `arith_reg` |

---

#### Category 3: Logic & Bit Operations (`AND`, `OR`, `EOR`, `NOT`, `BTST`, `BSET`, `BCLR`, `BCHG`, `TAS`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **LOGIC-01** | `AND` | `.W` | Data Reg Direct | `AND.W D1, D0` | `C041` | 4 | `logic_reg` |
| **LOGIC-02** | `AND` | `.L` | Data Reg Direct | `AND.L D1, D0` | `C081` | 8 | `logic_reg` |
| **LOGIC-03** | `ANDI` | `.W` | Immediate to Reg | `ANDI.W #$AAAA, D0` | `0240 AAAA` | 8 | `logic_imm` |
| **LOGIC-04** | `OR` | `.W` | Data Reg Direct | `OR.W D1, D0` | `8041` | 4 | `logic_reg` |
| **LOGIC-05** | `ORI` | `.W` | Immediate to Reg | `ORI.W #$5555, D0` | `0040 5555` | 8 | `logic_imm` |
| **LOGIC-06** | `EOR` | `.W` | Data Reg Direct | `EOR.W D1, D0` | `B140` | 4 | `logic_reg` |
| **LOGIC-07** | `EORI` | `.W` | Immediate to Reg | `EORI.W #$00FF, D0` | `0A40 00FF` | 8 | `logic_imm` |
| **LOGIC-08** | `NOT` | `.W` | Invert Register Bits | `NOT.W D0` | `4640` | 4 | `logic_reg` |
| **LOGIC-09** | `BTST` | `.L` | Static Bit Test Reg | `BTST #5, D0` | `0800 0005` | 10 | `bit_test` |
| **LOGIC-10** | `BTST` | `.L` | Dynamic Bit Test Reg | `BTST D1, D0` | `0100` | 6 | `bit_test` |
| **LOGIC-11** | `BSET` | `.L` | Dynamic Bit Set Reg | `BSET D1, D0` | `01C0` | 8 | `bit_mod` |
| **LOGIC-12** | `BCLR` | `.L` | Dynamic Bit Clear Reg | `BCLR D1, D0` | `0180` | 10 | `bit_mod` |
| **LOGIC-13** | `BCHG` | `.L` | Dynamic Bit Change Reg | `BCHG D1, D0` | `0140` | 8 | `bit_mod` |
| **LOGIC-14** | `TAS` | `.B` | Test & Set Memory (RMW)| `TAS (A0)+` | `4AD8` | 14 | `bit_rmw` |

---

#### Category 4: Shift & Rotate (`ASL`, `ASR`, `LSL`, `LSR`, `ROL`, `ROR`, `SWAP`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **SHIFT-01** | `LSL` | `.W` | Static Immediate Count | `LSL.W #3, D0` | `E748` | 12 | `shift_static` |
| **SHIFT-02** | `LSR` | `.W` | Static Immediate Count | `LSR.W #3, D0` | `E648` | 12 | `shift_static` |
| **SHIFT-03** | `LSL` | `.W` | Dynamic Register Count | `LSL.W D1, D0` | `E368` | ~12 | `shift_dynamic` |
| **SHIFT-04** | `ASL` | `.W` | Static Immediate Count | `ASL.W #2, D0` | `E540` | 10 | `shift_static` |
| **SHIFT-05** | `ASR` | `.W` | Static Immediate Count | `ASR.W #2, D0` | `E440` | 10 | `shift_static` |
| **SHIFT-06** | `ROL` | `.W` | Static Immediate Count | `ROL.W #4, D0` | `E958` | 14 | `shift_rotate` |
| **SHIFT-07** | `ROR` | `.W` | Static Immediate Count | `ROR.W #4, D0` | `E858` | 14 | `shift_rotate` |
| **SHIFT-08** | `SWAP` | `.W` | Swap 16-bit Halves | `SWAP D0` | `4840` | 4 | `shift_rotate` |

---

#### Category 5: Comparisons & Tests (`CMP`, `CMPA`, `CMPM`, `CMPI`, `TST`, `CHK`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **CMP-01** | `CMP` | `.B` | Data Reg Direct | `CMP.B D1, D0` | `B001` | 4 | `cmp_reg` |
| **CMP-02** | `CMP` | `.W` | Data Reg Direct | `CMP.W D1, D0` | `B041` | 4 | `cmp_reg` |
| **CMP-03** | `CMP` | `.L` | Data Reg Direct | `CMP.L D1, D0` | `B081` | 6 | `cmp_reg` |
| **CMP-04** | `CMPA` | `.W` | Word to Addr Reg | `CMPA.W D0, A0` | `B0C0` | 6 | `cmp_addr` |
| **CMP-05** | `CMPA` | `.L` | Long to Addr Reg | `CMPA.L D0, A0` | `B0E0` | 6 | `cmp_addr` |
| **CMP-06** | `CMPI` | `.W` | Immediate to Reg | `CMPI.W #$1234, D0` | `0C40 1234` | 8 | `cmp_imm` |
| **CMP-07** | `CMPM` | `.W` | Post-inc Memory Compare| `CMPM.W (A0)+, (A1)+`| `B348` | 12 | `cmp_mem` |
| **CMP-08** | `TST` | `.W` | Test Register | `TST.W D0` | `4A40` | 4 | `cmp_tst` |
| **CMP-09** | `TST` | `.W` | Test Memory | `TST.W (A0)+` | `4A58` | 8 | `cmp_tst` |
| **CMP-10** | `CHK` | `.W` | Check Bounds (In-bound)| `CHK.W D1, D0` | `4181` | 10 | `cmp_chk` |

---

#### Category 6: Decimal / BCD Arithmetic (`ABCD`, `SBCD`, `NBCD`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **BCD-01** | `ABCD` | `.B` | Data Regs | `ABCD D1, D0` | `C101` | 6 | `bcd_reg` |
| **BCD-02** | `ABCD` | `.B` | Pre-dec Memory | `ABCD -(A1), -(A0)` | `C109` | 18 | `bcd_mem` |
| **BCD-03** | `SBCD` | `.B` | Data Regs | `SBCD D1, D0` | `8101` | 6 | `bcd_reg` |
| **BCD-04** | `SBCD` | `.B` | Pre-dec Memory | `SBCD -(A1), -(A0)` | `8109` | 18 | `bcd_mem` |
| **BCD-05** | `NBCD` | `.B` | Data Reg | `NBCD D0` | `4800` | 6 | `bcd_reg` |

---

#### Category 7: Control Flow & Subroutines (`BRA`, `Bcc`, `BSR`, `RTS`, `RTR`, `DBcc`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **FLOW-01** | `BRA` | `.S` | Short Relative Branch | `BRA.S +2` | `6002` | 10 | `flow_branch` |
| **FLOW-02** | `Bcc` | `.S` | Branch Taken (BEQ) | `BEQ.S +2` | `6702` | 10 | `flow_branch` |
| **FLOW-03** | `Bcc` | `.S` | Branch Not Taken (BEQ)| `BEQ.S +2` | `6702` | 8 | `flow_branch` |
| **FLOW-04** | `BSR` | `.S` | Subroutine Branch | `BSR.S sub` | `6104` | 18 | `flow_call` |
| **FLOW-05** | `RTS` | — | Cascading Stack Return | `RTS` | `4E75` | 16 | `flow_return` |
| **FLOW-06** | `RTR` | — | Cascading Return with CCR| `RTR` | `4E77` | 20 | `flow_return` |
| **FLOW-07** | `DBcc` | `.W` | Decrement & Loop Taken| `DBF D7, target` | `51CF 0002` | 10 | `flow_loop` |
| **FLOW-08** | `DBcc` | `.W` | Loop Terminal Fallthrough| `DBF D7, exit` | `51CF 0002` | 14 | `flow_loop` |

---

#### Category 8: System & Privileged (`MOVE to SR/CCR`, `RTE`, `TRAP`, `TRAPV`)

| ID | Mnemonic | Size | Addressing Mode | Representative Syntax | Opcode Hex | Amiga CCK | Category Tag |
| :---: | :--- | :---: | :--- | :--- | :--- | :---: | :--- |
| **SYS-01** | `MOVE` | `.W` | Move to CCR | `MOVE.W D0, CCR` | `44C0` | 12 | `sys_ccr` |
| **SYS-02** | `MOVE` | `.W` | Move to SR (Supervisor) | `MOVE.W D0, SR` | `46C0` | 12 | `sys_sr` |
| **SYS-03** | `MOVE` | `.W` | Move from SR | `MOVE.W SR, D0` | `40C0` | 6 | `sys_sr` |
| **SYS-04** | `ANDI` | `.W` | AND Immediate to CCR | `ANDI.W #$001F, CCR` | `023C 001F` | 20 | `sys_ccr` |
| **SYS-05** | `ORI` | `.W` | OR Immediate to CCR | `ORI.W #$0000, CCR` | `003C 0000` | 20 | `sys_ccr` |
| **SYS-06** | `MOVE` | `.L` | Move USP to Addr Reg | `MOVE USP, A0` | `4E68` | 4 | `sys_usp` |
| **SYS-07** | `MOVE` | `.L` | Move Addr Reg to USP | `MOVE A0, USP` | `4E60` | 4 | `sys_usp` |
| **SYS-08** | `RTE` | — | Cascading Exception Return| `RTE` | `4E73` | 20 | `sys_rte` |
| **SYS-09** | `TRAP` | — | Software Trap Handler | `TRAP #0` | `4E40` | 34 | `sys_trap` |
| **SYS-10** | `TRAPV` | — | Trap on Overflow (Untaken)| `TRAPV` | `4E76` | 4 | `sys_trap` |

---

## Reference documentation and living sources

- [Motorola 68000 instruction timing tables](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): hardware cycle and bus-operation context for catalog values.
- [Motorola 68000 exception processing](../../Reference/68000%20User%27s%20Manual/06%20-%20Section%206%20-%20Exception%20Processing%2C%20Stack%20Frames%20%26%20Reset.md): vector, stack-frame, and return behavior used by exception workloads.
- [CPU architecture](../CPU%20Motorola%20M68000.md) and [micro-step state machine](../CPU%20Micro-Step%20State%20Machine.md): intended CPU behavior and CCK execution model.
- [SingleStep verification](../Verification/CPU%20SingleStepTests.md): independent reference-vector checks and their observation boundary.
- [Benchmark harness](../../../../crates/test_runner/benches/bench_instructions) and [analysis tool](../../../../tools/verification/analyze_benchmarks.py): living workload construction, timing, persistence, and report generation.
- [Retained benchmark CSV](../../../../baselines/benchmarks/m68k_benchmark_baseline.csv): historical host measurements used for the worked example and optional comparisons.
- [Performance profiling strategy](Performance%20Profiling%20and%20Optimization%20Strategy.md): broader host profiling and optimization workflow.
