---
title: "M68000 Instruction Benchmarking Architecture & Anomaly Detection"
aliases: ["CPU Instruction Benchmarking", "Instruction Benchmarking Architecture"]
tags: ["amiga", "design", "m68000", "benchmark", "telemetry"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-10
updated: 2026-10-10
related: ["[CPU Benchmark Analysis Guide.md](CPU%20Benchmark%20Analysis%20Guide.md)", "[CPU Instruction Benchmark Catalog.md](CPU%20Instruction%20Benchmark%20Catalog.md)", "[CPU Instruction Benchmark Strategies.md](CPU%20Instruction%20Benchmark%20Strategies.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)"]
tracked_paths:
  - "crates/test_runner/Cargo.toml"
  - "crates/test_runner/benches"
  - "tools/verification/analyze_benchmarks.py"
---

# M68000 Instruction Benchmarking Architecture & Anomaly Detection

- **Parent Specification:** [CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md) | [CPU Micro-Step State Machine.md](../CPU%20Micro-Step%20State%20Machine.md)
- **Module Location:** `crates/test_runner/benches/bench_instructions/`
- **Companion Specifications:** [CPU Instruction Benchmark Catalog](CPU%20Instruction%20Benchmark%20Catalog.md) | [CPU Instruction Benchmark Strategies](CPU%20Instruction%20Benchmark%20Strategies.md) | [CPU Benchmark Analysis Guide](CPU%20Benchmark%20Analysis%20Guide.md)
- **Engineering Guidelines:** Follow systems rules in [AGENTS.md](../../../../AGENTS.md).

> [!NOTE]
> This document defines the architectural specification and execution model for the programmatic M68000 instruction benchmarking subsystem.
> For per-instruction testing strategies (cascading stacks, PRNG data, exception loops), see [CPU Instruction Benchmark Strategies](CPU%20Instruction%20Benchmark%20Strategies.md).
> For the exhaustive list of testable opcodes and addressing modes, see [CPU Instruction Benchmark Catalog](CPU%20Instruction%20Benchmark%20Catalog.md).

---

## 1. Benchmarking Goals & Architecture Overview

The Amiga 500 emulator is a cycle-exact, micro-stepped simulation. While functional correctness is validated against over 1 million test vectors via [SingleStepTests](../Verification/CPU%20SingleStepTests.md) and [Cartesian DMA Contention](../CPU%20Motorola%20M68000.md#6-dma-contention--wait-states), raw correctness does not guarantee optimal host CPU performance or absence of micro-architectural stalls.

The **M68000 Instruction Benchmarking Subsystem** exists to:
1. **Detect Host Execution Anomalies:** Uncover unexpected execution latency spikes where instructions with identical or lower Amiga cycle counts consume disproportionately more host CPU time due to branch mispredictions, poor code alignment, cold path cache pollution, or missed inlining.
2. **Profile Host Execution Efficiency:** Measure how efficiently modern pipelined host processors (x86_64 Zen/Core, aarch64 Apple Silicon/Neoverse) execute the 65,536-entry static dispatch table and micro-step state machine.
3. **Differential Baseline Calibration:** Isolate pure instruction execution cost by subtracting a standardized 700-op `NOP` baseline ($T_{\text{baseline}}$), neutralizing loop control and instruction fetch overhead.
4. **Prevent Performance Regressions:** Maintain persistent, version-controlled benchmark records over time (JSON/CSV) to track emulator throughput across commits and refactorings with automated diff alerting.
5. **Flexible Execution Scaling:** Support both lightning-fast sanity checks (seconds) and thorough statistical baseline generation without modifying source code.

---

## 2. Execution Hierarchy: Dual-Loop Architecture

Accurately micro-benchmarking individual instructions on modern out-of-order, superscalar host CPUs requires isolating the target instruction from loop control branch overhead, host micro-architectural prediction biases, and compiler dead-code elimination.


```mermaid
flowchart TD
    subgraph Host Runner ["Host Test Runner (Outer Loop)"]
        A[Load Benchmark Spec] --> B[Initialize Machine & Memory]
        B --> C[Warm-up Pass]
        C --> D[Sample Run 1..N]
        D --> E[Statistical Aggregation: Min, Median, Max, StdDev]
        E --> F[Persist to JSON & CSV]
    end

    subgraph Guest Emulation ["Guest Amiga MemoryBus (Inner Loop)"]
        G[Preamble: Init Registers & Reset Pointers] --> H[Unrolled Instruction Block: K = 700 ops]
        H --> I["Loop Footer: DBF D7, Target / Reset"]
        I -->|D7 != 0| H
        I -->|D7 == 0| J[Benchmark Complete / Exit]
    end

    D <-->|Executes Guest Machine| Guest Emulation
```

### 2.1 The Inner Loop: Unrolled Block ($K = 700$ Instructions)

If an inner loop executed only 1 or 2 instructions before branching back (e.g. `ADD.W D1, D0` followed by `DBF D7, loop`), the loop control mechanics (`DBF` micro-steps, PC recalculation, host branch prediction evaluation) would consume up to 60% of the total measured time.

To achieve pure instruction measurement:
1. **Configurable Unroll Factor ($K$):** By default, the benchmark generator emits a contiguous block of **$K = 700$ unrolled instances** of the instruction under test.
2. **Amortized Branch Overhead:** With 700 instructions per block, the single terminating `DBF` or jump represents less than **0.15%** of the executed cycles, negligible enough to provide near-perfect measurement isolation.
3. **Defeating Host Loop Stream Detectors (LSD):**
   - Modern x86 processors (Intel Skylake through Raptor Lake, AMD Zen 3/4) contain dedicated Loop Stream Detectors / $\mu\text{op}$ caches that identify tight loops under 64 instructions and stream decoded micro-ops directly to execution ports, completely bypassing instruction fetch and decode.
   - An unrolled block of 700 instructions produces 1,400 to 4,200 bytes of guest machine code. This comfortably exceeds the host CPU's LSD threshold (preventing synthetic micro-op loop bypass) while remaining well within standard L1 instruction caches (32 KB–64 KB), ensuring realistic host instruction dispatch.
4. **Host Branch Predictor Decorrelation:**
   - Where applicable, unrolled sequences alternate registers (e.g. $D_0 \dots D_3$) or stride memory addresses so that the host CPU does not over-optimize repeated register rename aliases.

### 2.2 The Outer Loop: Multi-Pass Statistical Sampling & Noise Compensation

Modern host operating systems introduce non-deterministic timing noise via background thread preemption, memory paging, antivirus scans, DPC/ISR spikes, and dynamic CPU frequency scaling (turbo boost / thermal throttling).

To guarantee that measurements reflect true emulator performance rather than transient host OS interference, the outer loop employs a multi-tiered noise compensation and filtering pipeline:

```mermaid
flowchart TD
    Start["Run N Passes (e.g. 100 passes x 700 ops)"] --> Collect["Collect Raw Latency Samples"]
    Collect --> OutlierFilter["1. Outlier Rejection: IQR / Tukey's Fences<br/>(Discard background OS preemption spikes)"]
    OutlierFilter --> VarianceCheck{"2. Jitter Check:<br/>Coefficient of Variation (CV) < 3%?"}

    VarianceCheck -- Yes: Stable --> Aggregate["3. Compute Robust Metrics:<br/>Median, Trimmed Mean, Min, Max"]
    Aggregate --> Save["Persist Clean Benchmark Record"]

    VarianceCheck -- No: Storm Detected --> StormAction{"Retry Count < Max Retries?"}
    StormAction -- Yes --> Cooldown["Wait Out the Storm:<br/>500ms-1s Cooldown Sleep<br/>+ Dynamic Pass Extension (+20 passes)"]
    Cooldown --> Collect

    StormAction -- No --> FlagNoisy["Mark Run as 'environment_noisy: true'<br/>Preserve record but isolate from regression alerts"]
```

#### 1. Warm-up Phase
- One or two unmeasured passes execute first to prime host L1i/L1d caches, populate the translation lookaside buffer (TLB), and bring CPU frequency governors up to maximum stable clock speed.

#### 2. Outlier Rejection (Tukey's Fences & Trimmed Statistics)
- Raw timing samples collected across passes undergo automatic outlier rejection:
  - Calculate Interquartile Range ($\text{IQR} = Q_3 - Q_1$).
  - Filter out extreme latency spikes where $T > Q_3 + 1.5 \times \text{IQR}$ (typical of host OS context switches and background service interference).
  - Compute a **trimmed mean** (discarding top and bottom 10% of samples) alongside the **median**.

#### 3. Adaptive Environmental Noise Compensation ("Wait Out the Storm")
If the filtered samples exhibit high jitter ($\text{CV} = \sigma / \mu > 3.0\%$), it indicates the host CPU is experiencing external load or thermal throttling:
1. **Cooldown Pause ("Wait Out the Storm"):** Sleep for 500 ms to 1 second to allow transient host spikes (disk I/O bursts, background threads) to settle.
2. **Dynamic Pass Extension:** Automatically collect additional passes (e.g. extending from 100 to 120+ passes) until a statistically stable cluster of runs is achieved.
3. **Graceful Noise Flagging:** If the host remains unstable after 3 retry cooldown attempts, the benchmark does not fail; instead, it flags the record with `"environment_noisy": true` and `"confidence": "low"`, preventing external host interference from being falsely reported as an emulator code regression.

### 2.3 Compiler Optimization Defense (`black_box` & State Checksumming)

Modern optimizing compilers (LLVM / `rustc -O`) are extraordinarily capable of detecting dead loops, constant results, or redundant state transitions:
- If a benchmark loop modifies `CpuState` but never reads the result outside the loop, LLVM may optimize away parts of the dispatch loop or state updates.
- **Defense Mechanism:**
  1. At the conclusion of each outer pass, the test harness passes the mutable reference `&mut cpu` through `std::hint::black_box()` to prevent LLVM from eliminating intermediate memory or register writes.
  2. The harness computes a rolling 64-bit CRC / XOR checksum of all registers ($D_0 \dots D_7, A_0 \dots A_7, SR, PC$) and asserts it via `black_box`, forcing complete hardware simulation fidelity on every pass.

### 2.4 Differential Baseline Subtraction (`NOP` Reference Calibration)

Even with 700 unrolled instructions, a baseline fraction of runtime is consumed by:
- Instruction fetch and prefetch queue advance (`IR`/`IRC`).
- Cycle counter incrementing and bus clock tick bookkeeping.
- The single terminating `DBF` branch and loop control logic.

To isolate the **pure execution latency ($\Delta T$)** of complex ALU or memory operations, the subsystem defines two baseline calibration models:

#### 1. The Standard `NOP-700` Homogeneous Baseline
- For the ~95% of instructions that are **100% homogeneous** (e.g. 700 pure `ADD.W`, 700 pure `MOVE.W`, 700 pure cascading `RTS`), the reference baseline is **`BASE-00`**: an identical block of 700 unrolled `NOP` instructions ($4\text{ CCK}$ each, the lowest-latency valid guest opcode).
- The unit dispatch overhead of one guest instruction is:
  $$T_{\text{unit\_nop}} = \frac{T_{\text{NOP-700}}}{700}$$
- **Differential Delta:**
  $$\Delta T_{\text{net}} = T_{\text{instruction}} - T_{\text{NOP-700}}$$
  This cancels out the `DBF` loop control, prefetch priming, and general micro-step loop bookkeeping, leaving strictly the net host time spent on operand effective address calculations, bus transfers, and ALU computation.

#### 2. Composite & Paired Blocks (Template-Matched Baselines)
For instructions that require auxiliary or state-balancing companions inside the 700-instruction block:
- **Paired Instructions (e.g. `LINK` + `UNLK`, `LSL` + `LSR`):**
  - The block maintains the exact same total length ($K_{\text{total}} = 700$), consisting of 350 pairs:
    $$350 \times \text{LINK} + 350 \times \text{UNLK} = 700\text{ instructions}$$
  - The runner reports metrics normalized **per individual target instruction**:
    $$\text{Host ns/op} = \frac{T_{\text{block}}}{350} - T_{\text{unit\_aux}}$$
  - If `UNLK` has already been calibrated independently, its measured cost is subtracted to isolate `LINK`.
- **Exception Trampoline Loops (e.g. `DIVU #0` with handler):**
  - The baseline can run a **structural template replacement**, replacing the faulting `DIVU` with an un-trapped instruction in the same loop harness to isolate the trampoline overhead (`SUBQ` + `BEQ` + `RTE`) from the core Vector 5 exception dispatch.

### 2.5 Sentinel Termination & Infinite Loop Protection

To ensure synthesized machine code terminates cleanly and deterministically without unbounded execution or host process hangs:
1. **The Sentinel Exit Address (`BENCH_EXIT_PC = $004FFE`):**
   - Every synthesized linear block terminates with a jump footer (`JMP $004FFE`, opcode `4EF9 0000 4FFE`), or in cascading stack workflows (`RTS`, `RTR`, `RTE`), pops the final return address pointing to `$004FFE`.
   - At address `$004FFE`, the injection builder places a supervisor stop instruction: `STOP #$2700` (opcode `4E 72 27 00`).
2. **Termination Loop Invariant:**
   - The inner stepping loop evaluates termination after each instruction:
     ```rust
     while !cpu.state.halted && !cpu.state.stopped && cpu.state.instruction_pc != program.exit_pc {
         cpu.step_instruction(&mut bus);
     }
     ```
   - Reaching `$004FFE` immediately breaks the loop, transitioning the CPU into the clean stopped state.
3. **Hard Cycle Timeout Threshold ($10\times$ Expected CCK):**
   - If an unexpected hardware defect, unaligned vector jump, or instruction corruption causes an infinite loop, execution is guaranteed to abort:
     $$\text{max\_cycles} = \max(C_{\text{expected\_per\_pass}}, 1000) \times 10$$
   - When `elapsed_cycles >= max_cycles`, the runner halts execution, marks `timed_out = true`, and flags the result with `AnomalyType::ExecutionTimeoutOrInfiniteLoop`.


---

## 3. Programmatic Code Generation (`BenchmarkProgramBuilder`)

Rather than maintaining hundreds of static `.bin` files and requiring an external M68000 assembler toolchain, the benchmarking subsystem synthesizes test programs **100% programmatically in Rust memory**.

```mermaid
flowchart LR
    Spec[Benchmark Catalog Spec] --> Builder[BenchmarkProgramBuilder]
    Builder --> Preamble[Preamble Synthesis]
    Builder --> Unroll[Unrolled K-Block Generator]
    Builder --> Footer[Footer Synthesis]
    Builder --> Memory[PRNG Memory Buffer Init]
    Preamble & Unroll & Footer & Memory --> Machine[Chip RAM Injection: $001000]
```

### 3.1 Architecture Advantages
- **Zero External Toolchain Dependencies:** Compiles and runs out-of-the-box on native Linux, macOS, Windows, and WebAssembly without requiring `vasm`, `gas`, or pre-assembled binaries.
- **Dynamic Parameterization:** Unroll factor $K$, register sets, memory addresses, and data patterns can be altered at runtime via test configuration flags.
- **Microsecond Synthesis:** Constructing a 700-instruction test program directly in a `Vec<u16>` and writing it into Chip RAM takes under 15 microseconds.

### 3.2 Program Structure
Every generated benchmark program conforms to a 4-stage memory layout:

```
+-------------------------------------------------------------+
| Vector Table & Exception Handlers ($000000 - $0003FF)       |
+-------------------------------------------------------------+
| PRNG Memory Operand Buffer ($000400 - $000FFF)              |
+-------------------------------------------------------------+
| Benchmark Program Entry ($001000):                          |
|   1. Preamble: Set registers, SP, flags                     |
|   2. Inner Loop Body: K unrolled target instructions        |
|   3. Footer: JMP $004FFE / Cascading Stack Return           |
|   4. Exit Sentinel: STOP #$2700 at $004FFE                  |
+-------------------------------------------------------------+
| Memory Buffers ($006000 A0 source, $007000 A1 destination)  |
| Subroutine Return Base ($008000 rts_base)                   |
| Cascading Stack / Push Buffer ($070000 - $07FFFF)           |
+-------------------------------------------------------------+
```

---

## 4. Execution Scaling & Runtime Configuration

The benchmark runner selects one execution profile through the command line, allowing the same catalog to serve as a quick measurement or a longer baseline generation run.

### 4.1 Execution Profiles

| Profile | CLI Profile | Nominal Total Instructions per Specification | Approximate Runtime (Whole Suite) | Primary Use Case |
| :--- | :--- | :--- | :--- | :--- |
| **Sanity / Quick** | `--quick` | $31{,}500$ | ~5 seconds | Pre-commit check, CI pipeline validation. |
| **Standard** | `--standard` | $6{,}997{,}200$ | ~2 minutes | PR review, instruction optimization validation. |
| **Thorough** | `--thorough` | $149{,}992{,}500$ | ~20 minutes | Nightly performance regression check. |

### 4.2 Fixed Measurement Settings

The command-line benchmark uses 700 instructions per generated block, the full
catalog, profile-defined passes and iterations, and host core affinity when
supported. Reports are written under `.agent/tmp/benchmarks/<profile>/`.
Only the measurement profile is selectable from the command line. Internal test
harnesses configure `BenchmarkConfig` directly for their constructed fixtures.

### 4.3 Multi-Core Policy & Performance Core (P-Core) Auto-Detection

```mermaid
flowchart TD
    Start["Benchmark Startup"] --> Mode{"Execution Mode"}
    Mode -- "quick (Smoke Test)" --> ParallelOpt{"Parallel Flag Set?"}
    ParallelOpt -- Yes --> MultiWorker["Run workers across N cores (-j N)"]
    ParallelOpt -- No --> SingleSeq

    Mode -- "standard / thorough" --> SingleSeq["Sequential Precision Profiling"]
    SingleSeq --> Detect["Detect Host Topology & Core Types"]
    Detect --> FindPCore["Identify Performance Cores (P-Cores)<br/>Windows: EfficiencyClass > 0<br/>Linux: Max cpuinfo_max_freq<br/>macOS: hw.perflevel0"]
    FindPCore --> Pin["1. Pin Worker Thread to Primary P-Core<br/>2. Elevate to High Priority Class"]
    Pin --> RunBench["Execute Benchmark Passes in Clean Isolation"]
```

#### 1. Why Sequential Profiling on P-Cores is Mandatory
Running multiple benchmark threads concurrently across all CPU cores causes **severe measurement degradation**:
- **Shared Cache Contention:** Concurrent threads evict each other's lines in the shared L3 cache.
- **DRAM Bus Saturation:** Memory-referencing instructions compete for shared memory bus bandwidth.
- **Thermal Downclocking:** Modern multi-core processors reduce clock speeds under all-core load (e.g. all-core boost clock is often 500–1000 MHz lower than peak single-core boost clock).
- **The Golden Rule:** High-precision instruction profiling (`standard`, `thorough`) must execute **sequentially on a single dedicated Performance Core (P-core)** to maintain peak single-core turbo frequencies, clean L1i/L2 caches, and minimal jitter. Multi-threaded execution (`-j <N>`) is strictly reserved for developer smoke tests (`quick`).

#### 2. Automatic Detection of Performance Cores (P-Cores vs E-Cores)
On modern hybrid architectures (Intel 12th–14th Gen, AMD Zen 4/5c, Apple Silicon M-series), host processors mix high-performance P-cores with lower-clocked Efficiency (E-cores). To guarantee benchmarks always execute on maximum-performance hardware:

- **Windows (`GetLogicalProcessorInformationEx`):**
  - Queries `RelationProcessorCore` topology.
  - Inspects the `EfficiencyClass` byte in each `PROCESSOR_CORE_INFO`:
    - `EfficiencyClass == 0`: Efficiency Core (E-core).
    - `EfficiencyClass > 0`: Performance Core (P-core) with highest computational throughput.
  - The runner selects a core with the highest `EfficiencyClass`.
- **Linux (`/sys/devices/system/cpu/`):**
  - Scans `/sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq`.
  - Cores reporting the highest frequency ceiling (e.g. 5.8 GHz vs 4.0 GHz) are classified as P-cores.
- **macOS (`sysctl`):**
  - Queries `hw.perflevel0.logicalcpu` (P-cores) and `hw.perflevel1.logicalcpu` (E-cores).

#### 3. Affinity Pinning & Thread Priority Elevation
Once the optimal P-core is resolved:
1. **Thread Affinity:** The runner binds the benchmark thread to the selected core mask (`SetThreadAffinityMask` on Windows, `pthread_setaffinity_np` on Linux/macOS), preventing the OS scheduler from migrating the thread across cores mid-pass.
2. **Process Priority:** The runner temporarily raises its priority to `HIGH_PRIORITY_CLASS` (Windows) or `nice -20` (Unix) to minimize preemption from background host tasks.

### 4.4 Portability & WebAssembly (WASM) Decoupling Architecture

A fundamental architectural mandate of this emulator is:
> *"Compiles seamlessly for native desktop (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`). System-agnostic core: zero direct OS or platform dependencies."*

To satisfy this requirement without compromising native P-core pinning or portability across different machines:

```mermaid
flowchart TD
    Runner["Benchmark Runner Entry"] --> Abstraction["Platform Abstraction Layer: HostEnvironment"]

    Abstraction -->|cfg target_os = windows| Win["Windows Native: GetLogicalProcessorInformationEx + SetThreadAffinityMask"]
    Abstraction -->|cfg target_os = linux| Lin["Linux Native: /sys/devices/system/cpu + sched_setaffinity"]
    Abstraction -->|cfg target_os = macos| Mac["macOS Native: sysctl hw.perflevel0 + thread_policy_set"]
    Abstraction -->|cfg target_arch = wasm32| Wasm["WebAssembly (WASM): Graceful No-Op Fallback + performance.now()"]
    Abstraction -->|Homogeneous Native| Generic["Generic Fallback: Pin to Core 0 / All Cores Equal"]
```

#### 1. Decoupled Platform Abstraction (`HostEnvironment`)
The benchmark harness defines a lightweight platform abstraction with zero runtime overhead in [`crates/test_runner/benches/bench_instructions/platform.rs`](../../../../crates/test_runner/benches/bench_instructions/platform.rs):
- **Struct [`HostEnvironment`](../../../../crates/test_runner/benches/bench_instructions/platform.rs):** Encapsulates `platform_name`, `p_core_id`, and `affinity_supported`.
- **`HostEnvironment::detect()`:** Discovers host topology dynamically on startup without hardcoding.
- **`HostEnvironment::pin_to_p_core()`:** Pins the worker thread to the resolved P-core with high priority class; compiles to a clean no-op on WebAssembly.

#### 2. Dynamic Hardware Discovery (Zero Hardcoded Paths or IDs)
- If the user moves the project from an Intel hybrid laptop (P+E cores) to an AMD desktop (16 identical Zen cores) or an Apple Silicon Mac (M-series):
  - **No hardcoded core numbers exist in the codebase.**
  - At startup, the OS query resolves the current machine's physical layout in microseconds.
  - On homogeneous processors (where all cores have identical performance), the detector recognizes that all cores are equal and pins to core 0 (or core 2 to stay clear of OS timer IRQs).

#### 3. WebAssembly (`wasm32-unknown-unknown`) Compatibility
In a WebAssembly environment (browsers, Node.js, Web Workers):
- **Browser Sandboxing:** Web browsers intentionally restrict access to physical CPU core topology and thread affinity to prevent side-channel timing attacks (Spectre) and device fingerprinting.
- **Graceful Compilation Fallback:** Under `#[cfg(target_arch = "wasm32")]`:
  - `pin_to_p_core()` compiles into a **zero-overhead, successful no-op** (`Ok(())`).
  - Core detection returns `affinity_supported: false`.
  - Timers automatically bind to high-resolution web standards (`performance.now()`) instead of host-native APIs.
- **Result:** The exact same benchmark code compiles cleanly for `wasm32-unknown-unknown`, allowing instruction benchmarking directly inside web browser test suites without compile errors, platform panics, or conditional code branching in the hot loop.



---

## 5. Memory Environment & PRNG Seeding

Memory-referencing instructions (`(An)`, `(An)+`, `-(An)`, `d(An)`, `d(An,Xi)`, `$xxxx.W`, `$xxxxxxxx.L`) must not operate on uninitialized, zeroed, or static memory:
- Operating entirely on zeros risks unrealistic branch and arithmetic shortcuts in the host ALU (e.g. zero division, no-op carries).
- Inverting bits or repeating single values creates synthetic CPU cache behaviors.

### 5.1 Deterministic Pseudo-Random Generator (PRNG)
To eliminate repetitive data patterns and prevent host processor branch/ALU shortcut optimizations, a deterministic PRNG is used for:
1. **Memory Operand Buffers:** Pre-filling Chip RAM operand buffers with varied, non-zero bit patterns.
2. **Initial Register Preamble:** Initializing Data and Address registers ($D_0 \dots D_7, A_0 \dots A_6$) with pseudo-random values.
3. **Immediate Operand Stream:** Emitting varying pseudo-random constants for immediate instruction streams (e.g. 700 unrolled `MOVE.L #<random>, D0` or `ADDI.W #<random>, D0`) rather than repeating a single static constant.

The generator does not require cryptographic security; a fast, lightweight 64-bit algorithm (such as `XorShift64`) is used:
```rust
// Deterministic seed ensures identical data across runs and host architectures
const BENCH_PRNG_SEED: u64 = 0x8547_5938_4721_9381;
```
- **Determinism:** Data is 100% bit-for-bit identical across runs, native architectures (x86_64 vs aarch64), and releases.
- **Valid Data Range Clamping:** For sensitive instructions (e.g. non-zero `DIVU`/`DIVS` divisors, valid decimal digits for `ABCD`/`SBCD`, word-aligned addresses for address registers), the PRNG output is clamped or masked to ensure architectural validity without triggering accidental hardware traps.


### 5.2 Cyclic Memory Buffers
For post-increment `(An)+` and pre-decrement `-(An)` modes:
- The buffer size is sized to match the unroll block (e.g. $700 \times 4\text{ bytes} = 2{,}800\text{ bytes}$ for `.L` operations).
- In the loop footer, $An$ is reset back to the buffer head, allowing endless cyclic traversals without memory overflow or fault generation.

---

## 6. Results Persistence & Output Schema

Upon completion of a benchmark run, results are serialized to disk into profile-isolated directories under `.agent/tmp/benchmarks/<profile>/`. To maintain a clean, deterministic repository without accumulating unbounded timestamped files, the harness implements **two-tier rotation**: before writing the new benchmark results, existing reports are automatically rotated to `_previous`:

1. `.agent/tmp/benchmarks/<profile>/m68k_benchmark.json` (Active structured dataset with environment metadata).
2. `.agent/tmp/benchmarks/<profile>/m68k_benchmark.csv` (Active flat tabular format for spreadsheet analysis and automated regression testing).
3. `.agent/tmp/benchmarks/<profile>/m68k_benchmark_latest.json` / `.csv` (Active run copy for automated regression diffing).
4. `.agent/tmp/benchmarks/<profile>/m68k_benchmark_previous.json` / `.csv` (Immediately preceding run preserved before rotation).

### 6.1 JSON Data Schema
```json
{
  "version": 1,
  "timestamp": "2026-09-10T12:00:00Z",
  "environment": {
    "host_os": "windows",
    "host_arch": "x86_64",
    "rustc_version": "rustc 1.82.0",
    "profile": "release",
    "unroll_factor": 700,
    "outer_passes": 7
  },
  "results": [
    {
      "mnemonic": "ADD.W",
      "variant": "ADD.W D1, D0",
      "addressing_mode": "DataRegDirect",
      "category": "Arithmetic",
      "opcode_hex": "D041",
      "amiga_cck_cycles": 4,
      "total_guest_instructions": 7000000,
      "total_guest_cck": 28000000,
      "host_duration_median_ms": 19.82,
      "host_duration_min_ms": 19.45,
      "host_duration_max_ms": 20.31,
      "host_jitter_pct": 1.25,
      "host_ns_per_instruction": 2.83,
      "host_ns_per_guest_cck": 0.708,
      "host_mips": 353.18,
      "anomaly_flag": false
    }
  ]
}
```

### 6.2 CSV Data Columns & Schema Reference

The `.csv` table provides identical telemetry and metadata to the JSON report in flat tabular form (`m68k_benchmark.csv`, `latest.csv`). All column headers are **100% unified with JSON property names**:

```csv
timestamp,mnemonic,variant,addressing_mode,category,opcode_hex,amiga_cck_cycles,total_guest_instructions,host_duration_median_ms,host_ns_per_instruction,host_ns_per_guest_cck,host_mips,host_jitter_pct,anomaly_flag
epoch_00020706_140107,ADD,ADD.W D1 D0,DataRegDirect,Arithmetic,D041,4,6997200,19.82,2.83,0.708,353.18,1.25,false
```

#### Complete 14-Column Specification Table

| Col # | Header Name (Unified with JSON) | Type | Example | Classification | Architectural Definition & Mathematical Formula |
| :---: | :--- | :---: | :--- | :---: | :--- |
| **0** | **`timestamp`** | `String` | `epoch_00020706_140107` | Run Metadata | Monotonically formatted execution timestamp or ISO-8601 string identifying the specific benchmark run. |
| **1** | **`mnemonic`** | `String` | `ADD` | **Deterministic Invariant** | Motorola M68000 base assembly mnemonic (e.g. `ADD`, `MOVE`, `SUB`, `JSR`). |
| **2** | **`variant`** | `String` | `ADD.W D1 D0` | **Deterministic Invariant** | Representative instruction syntax including operand size (`.B`, `.W`, `.L`) and operand types. Commas are replaced with spaces to avoid CSV quoting. |
| **3** | **`addressing_mode`** | `String` | `DataRegDirect` | **Deterministic Invariant** | Effective Addressing Mode name (e.g. `DataRegDirect`, `AddrIndirect`, `PreDecrement`, `Displacement`, `Immediate`, `PcDisplacement`). |
| **4** | **`category`** | `String` | `Arithmetic` | **Deterministic Invariant** | Functional instruction category (`Baseline`, `DataMovement`, `Arithmetic`, `Comparison`, `Logic`, `Shift`, `ControlFlow`, `Bcd`, `System`). |
| **5** | **`opcode_hex`** | `String` | `D041` | **Deterministic Invariant** | Hexadecimal machine code representation of the instruction (16-bit, 32-bit, or 48-bit words separated by space). |
| **6** | **`amiga_cck_cycles`** | `u64` | `4` | **Deterministic Invariant** | Cycle-exact Amiga Color Clock (**CCK**) cost on an Amiga 500 (~282 ns per CCK; $1\ \text{CCK} = 2\ \text{CPU clocks}$). |
| **7** | **`total_guest_instructions`** | `u64` | `6997200` | **Deterministic Invariant** | Strictly nominal total operations executed across all passes ($K \times \text{iterations} \times \text{passes}$). Exactly invariant across all 108 specs for a profile (`Quick`: 31,500; `Standard`: 6,997,200; `Thorough`: 149,992,500). |
| **8** | **`host_duration_median_ms`** | `f64` | `19.82` | **Stochastic Telemetry** | Median wall-clock duration per pass in milliseconds across all surviving passes after Tukey's fences outlier rejection. |
| **9** | **`host_ns_per_instruction`** | `f64` | `2.83` | **Stochastic Telemetry** | Host execution latency per instruction in nanoseconds: $$T_{\text{host\_ns}} = \frac{T_{\text{median\_ms}} \times 10^6}{K \times \text{iterations}}$$ |
| **10** | **`host_ns_per_guest_cck`** | `f64` | `0.708` | **Stochastic Telemetry** | **Normalized Emulation Tax ($R_{\text{norm}}$):** Host nanoseconds required to simulate one Amiga CCK phase. $$R_{\text{norm}} = \frac{\text{host\_ns\_per\_instruction}}{\text{amiga\_cck\_cycles}}$$ Target bound is $\le 3.50\text{ ns/CCK}$ (~80–110× real-time). |
| **11** | **`host_mips`** | `f64` | `353.18` | **Stochastic Telemetry** | Host execution throughput in Millions of Instructions Per Second: $$\text{MIPS} = \frac{1000}{\text{host\_ns\_per\_instruction}}$$ |
| **12** | **`host_jitter_pct`** | `f64` | `1.25` | **Stochastic Telemetry** | Coefficient of Variation ($\text{CV} = \sigma / \mu \times 100\%$) across passes. Values $< 3\%$ indicate high stability; $> 5\%$ flags excessive host jitter. |
| **13** | **`anomaly_flag`** | `bool` | `false` | **Stochastic Telemetry** | Boolean anomaly verdict (`true` / `false`). Triggered if Type A (intra-family spike $> 2.5\times$), Type B (memory overhead $> 3.0\times$), or Type C (jitter $> 5\%$) rules fail. |

---

## 7. Anomaly Detection: The Normalized Cycle Metric

The primary objective of the benchmarking suite is not merely measuring raw speed, but **identifying implementation anomalies**.

### 7.1 The Metric: Host Nanoseconds per Amiga Color Clock ($R_{\text{norm}}$)
$$R_{\text{norm}} = \frac{T_{\text{host}}\ (\text{ns})}{C_{\text{guest}}\ (\text{Amiga CCK})}$$

In a micro-stepped, cycle-exact emulator, each Amiga Color Clock represents a single step of the bus or internal state machine. Consequently, **the host time required per guest clock cycle should remain relatively uniform across all instructions of the same execution tier**.

### 7.2 Anomaly Classification Rules

```mermaid
flowchart TD
    Calc["Calculate R_norm = Host_ns / Guest_CCK"] --> Check1{"R_norm > 2.5x Family Baseline?"}
    Check1 -- Yes --> Anomaly1["⚠️ Type A Anomaly: Hot Path Stalling / Cold Dispatch"]
    Check1 -- No --> Check2{"R_norm (Memory) > 3.0x Register Baseline?"}
    Check2 -- Yes --> Anomaly2["⚠️ Type B Anomaly: Bus Contention / Memory Inlining Defect"]
    Check2 -- No --> Check3{"Jitter > 5.0% Across Passes?"}
    Check3 -- Yes --> Anomaly3["⚠️ Type C Anomaly: Host Branch Prediction Thrashing"]
    Check3 -- No --> Pass["✅ Normal Execution Profile"]
```

1. **Type A: Intra-Family Execution Spike ($R_{\text{norm}} > 2.5\times$ baseline)**
   - *Symptom:* `SUB.W D1, D0` takes $0.71\text{ ns/CCK}$, but `AND.W D1, D0` (both 4-cycle register operations) takes $2.45\text{ ns/CCK}$.
   - *Root Cause:* Dynamic match expressions evaluated at runtime instead of direct table dispatch, or missing compiler optimizations.
2. **Type B: Addressing Mode Inefficiency ($R_{\text{norm}} > 3.0\times$ register baseline)**
   - *Symptom:* Memory read `MOVE.W (A0), D0` exhibits an exorbitant cost per cycle compared to register `MOVE.W D1, D0`.
   - *Root Cause:* Memory access helpers (`read_word`, `bank_handler`) failing cross-crate inlining (`#[inline]`), creating indirect function call overhead on every bus cycle.
3. **Type C: Excessive Jitter ($\sigma / \text{median} > 5\%$)**
   - *Symptom:* Significant variance between consecutive outer loop passes on an idle system.
   - *Root Cause:* Host CPU Branch Target Buffer (BTB) capacity exceeded, or instruction stream crossing cache line boundaries unevenly.

### 7.3 Automated Historical Regression Diffing (`bench_diff`)

To make benchmark data immediately actionable across development cycles:
- **Baseline Comparison:** The test runner automatically looks for `baselines/benchmarks/m68k_benchmark_baseline.json`. If present, it computes the delta percentage for each instruction variant:
  $$\Delta \% = \frac{T_{\text{new}} - T_{\text{baseline}}}{T_{\text{baseline}}} \times 100\%$$
- **Regression Alert Thresholds:**
  - **$\Delta \le +3.0\%$:** Normal variance / noise tolerance.
  - **$+3.0\% < \Delta \le +7.0\%$:** Warning flag (`⚠️ Minor Regression: Investigate inlining`).
  - **$\Delta > +7.0\%$:** Failure alert (`🚨 Severe Regression: Blocking merge`).
- **Environment Metadata Verification:** The diff engine verifies that `host_arch`, `host_cpu`, and `profile` match before issuing regression alerts, ensuring measurements taken on different hardware are never compared directly.

For the comprehensive evaluation framework and analysis techniques (Functional Symmetry, Addressing Mode Latency Ladders, Size Scaling, and Normalized Emulation Tax), consult the dedicated [CPU Benchmark Analysis Guide](CPU%20Benchmark%20Analysis%20Guide.md).

---

## 8. CLI Execution Guide: Quick, Standard, and Thorough Runs & Automated Baseline Diffing

The benchmarking harness supports multiple operational profiles, targeted category filtering, and automatic differential regression comparison against a locked golden baseline.

### 8.1 Execution Profiles: Quick, Standard, and Thorough

Select an execution profile with the words `quick`, `standard`, or `thorough`:

| Profile Selector | Outer Passes | Iterations / Pass | Total Ops / Spec | Typical Full Suite Duration | Recommended Usage |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **`quick`** | 3 | 15 | ~10,500 | **~2–5 seconds** | Rapid developer sanity checks, pre-commit validation, CI pipelines. |
| **`standard`** *(default)* | 7 | 1,428 | ~1,000,000 | **~1–2 minutes** | Standard performance profiling, PR validation, Tukey outlier rejection. |
| **`thorough`** | 15 | 14,285 | ~10,000,000 | **~10–15 minutes** | High-precision baseline generation, jitter elimination ($\text{CV} < 1\%$). |

### 8.2 Results Output Files & Directory Structure

Tracked references and generated artifacts use separate directories:

```text
baselines/benchmarks/                         # Git-tracked references
    m68k_benchmark_baseline.json
    m68k_benchmark_baseline.csv
.agent/tmp/benchmarks/                    # Ignored CLI output
    quick/                               # Current/latest/previous JSON and CSV
    standard/
    thorough/
```

Benchmark runs write generated artifacts into ignored directories. Tracked baseline reports remain unchanged unless explicitly updated after review.

### 8.3 Automated Baseline Diffing & Cross-Hardware Normalization

The test runner features **built-in zero-configuration differential regression detection**:
1. **Automatic Detection:** On every benchmark run, the harness automatically checks whether `baselines/benchmarks/m68k_benchmark_baseline.json` exists.
2. **Cross-Hardware Normalization:** When running on a different CPU architecture (e.g. AMD Ryzen, Apple Silicon, CI) or when baseline NOP duration diverges by $> 15\%$ from the host's measured NOP latency, the runner automatically normalizes execution durations relative to `BASE-00` (`NOP`):
   $$R_{\text{nop}} = \frac{T_{\text{host}}(\text{Op})}{T_{\text{host}}(\text{NOP})}$$
   $$\Delta\% = \frac{R_{\text{nop, current}} - R_{\text{nop, baseline}}}{R_{\text{nop, baseline}}} \times 100\%$$
   Because host CPU clock speed variations scale both the target instruction and NOP equally, this ratio is hardware-invariant and eliminates false regression alarms across different developer machines.
3. **Immediate Diff Output:** The runner matches each opcode specification, calculates $\Delta\%$ change, and displays live terminal alerts:
   - **$\Delta \le +3.0\%$:** `OK (baseline)` — within normal noise and thermal jitter tolerance.
   - **$+3.0\% < \Delta \le +7.0\%$:** `⚠️ [WARN: Minor Regression]` — potential cache line misalignment or missed cross-crate inlining.
   - **$\Delta > +7.0\%$:** `🚨 [FAIL: Severe Regression]` — unacceptable performance drop; review git diff immediately.
4. **Promoting / Updating the Baseline:**
   To establish or update the golden baseline after intentional, verified optimizations:
   ```powershell
   Copy-Item .agent/tmp/benchmarks/thorough/m68k_benchmark_latest.json baselines/benchmarks/m68k_benchmark_baseline.json
   Copy-Item .agent/tmp/benchmarks/thorough/m68k_benchmark_latest.csv baselines/benchmarks/m68k_benchmark_baseline.csv
   ```

### 8.4 Command Cheat-Sheet & Comprehensive CLI Reference

CPU measurements have one executable entry point: the `bench_instructions`
Cargo benchmark target in [`Cargo.toml`](../../../../crates/test_runner/Cargo.toml).
The `test_runner` crate has no application binary. SingleStep reports are generated automatically when suite results are saved.

#### 8.4.1 Benchmark Invocation

```powershell
cargo bench -p test_runner --bench bench_instructions -- [OPTIONS]
```

Cargo builds this target with the optimized benchmark profile. The target uses
`harness = false`: its own entry point invokes the existing measurement engine,
which retains program synthesis, timing, statistics and report persistence.
Running without options prints instructions and exits without measuring. Select
one measurement profile with `--quick`, `--standard` or `--thorough`.
The [benchmark parser](../../../../crates/test_runner/benches/bench_instructions/cli.rs)
rejects unknown, repeated, missing or invalid options before execution with exit
code 2. Only a measurement profile or help is accepted. Cargo's appended
`--bench` marker is handled without changing the measurement configuration.

`verify.py runner bench` translates sentence selectors into the same Cargo
invocation. `verify.py runner profile bench` builds the target without execution,
reads the benchmark executable from Cargo artifact messages and runs that exact
artifact under `samply`. Build failure or a missing benchmark artifact stops
profiling before an older executable can run. The
[profiling launcher](../../../../tools/verification/profile_benchmark.py) owns
artifact selection and external profiler invocation.

#### 8.4.2 Benchmark Options

| Option | Argument | Default | Description |
| :--- | :---: | :---: | :--- |
| **`--quick`** | None | None | 3 passes with 15 iterations per specification. |
| **`--standard`** | None | None | 7 passes with 1,428 iterations per specification. |
| **`--thorough`** | None | None | 15 passes with 14,285 iterations per specification. |
| **`--help`, `-h`** | None | Help | Prints benchmark usage without measuring. |

#### 8.4.3 Practical CLI Execution Examples
```powershell
# 1. Quick sanity check across all 108 benchmark specifications (~2–5 seconds):
python tools/verification/verify.py runner bench quick

# 2. Standard benchmark run (default profile, ~1–2 minutes, full suite):
python tools/verification/verify.py runner bench
# Or explicitly:
python tools/verification/verify.py runner bench standard

# 3. Sampled SingleStep tests for one opcode (reports are saved automatically):
python tools/verification/verify.py test runner singlestep case test_add_b sample 50

# 4. Full markdown comparative analysis report generation from latest thorough run:
python tools/verification/verify.py runner analyze .agent/tmp/benchmarks/thorough/m68k_benchmark_latest.csv report .agent/tmp/benchmarks/thorough/analysis_report.md
```

---

## 9. Living Rust Source Code & Module Map

All components specified in this document are implemented without macros or const-generic dispatch, strictly conforming to the 800-line modularity threshold:

| Subsystem Component | Living Rust Source File | Responsibilities |
| :--- | :--- | :--- |
| **Benchmark Runner** | [`crates/test_runner/benches/bench_instructions/runner.rs`](../../../../crates/test_runner/benches/bench_instructions/runner.rs) | Dual-loop runner, baseline calibration, black_box compiler defense, and hard cycle cap timeout. |
| **Catalog Specifications** | [`crates/test_runner/benches/bench_instructions/catalog.rs`](../../../../crates/test_runner/benches/bench_instructions/catalog.rs)<br/>[`catalog_data_a.rs`](../../../../crates/test_runner/benches/bench_instructions/catalog_data_a.rs)<br/>[`catalog_data_b.rs`](../../../../crates/test_runner/benches/bench_instructions/catalog_data_b.rs) | Exhaustive static catalog of ~80 benchmark specifications covering Categories 0–8. |
| **Programmatic Builder** | [`crates/test_runner/benches/bench_instructions/builder.rs`](../../../../crates/test_runner/benches/bench_instructions/builder.rs) | Synthesizes unrolled instruction blocks ($K = 700$), cascading return stacks, and Chip RAM buffers. |
| **Platform Abstraction** | [`crates/test_runner/benches/bench_instructions/platform.rs`](../../../../crates/test_runner/benches/bench_instructions/platform.rs) | Windows P-core topology discovery, affinity pinning (`SetThreadAffinityMask`), priority elevation, WASM fallback. |
| **Deterministic PRNG** | [`crates/test_runner/benches/bench_instructions/prng.rs`](../../../../crates/test_runner/benches/bench_instructions/prng.rs) | `XorShift64` PRNG seeded with `BENCH_PRNG_SEED`, valid BCD generator. |
| **Statistical Analysis** | [`crates/test_runner/benches/bench_instructions/stats.rs`](../../../../crates/test_runner/benches/bench_instructions/stats.rs) | Multi-pass statistical sampling, Tukey's fences outlier rejection, jitter analysis ($\text{CV} < 3\%$), MIPS calculation. |
| **Anomaly Classifier** | [`crates/test_runner/benches/bench_instructions/anomaly.rs`](../../../../crates/test_runner/benches/bench_instructions/anomaly.rs) | Evaluates Type A, Type B, Type C, and `ExecutionTimeoutOrInfiniteLoop` anomalies; diffs against baseline. |
| **Persistence Engine** | [`crates/test_runner/benches/bench_instructions/persistence.rs`](../../../../crates/test_runner/benches/bench_instructions/persistence.rs) | Formats and saves ignored JSON and CSV measurement reports. |
| **Benchmark Entry** | [`crates/test_runner/benches/bench_instructions/cli.rs`](../../../../crates/test_runner/benches/bench_instructions/cli.rs) | Sole executable entry point and argument validation for `cargo bench`. |
| **Analysis & Reporting Tool** | [`tools/verification/analyze_benchmarks.py`](../../../../tools/verification/analyze_benchmarks.py) | Comparative analysis tool evaluating functional symmetry, addressing mode scaling, and emulation tax. |

---

## 10. Benchmark Report Fields and Nominal Operation Counts

A fundamental engineering challenge in micro-benchmarking is verifying that generated output files (`.agent/tmp/benchmarks/**/*.csv` and `.json`) are **structurally and functionally correct**, even though physical timing metrics (`host_ns_op`, `host_mips`, `jitter_pct`) naturally vary across host CPUs, operating systems, and ambient workloads.

### 10.1 Separating Stochastic Telemetry from Emulation Invariants

Every benchmark row in the CSV table contains 14 columns. The verification architecture strictly bifurcates these into two categories:

```mermaid
flowchart TD
    Row["Benchmark CSV Row (14 Columns)"] --> Invariants["Deterministic Invariants (Columns 1..=7)<br/>Must be 100% bit-for-bit identical across all hosts and runs"]
    Row --> Telemetry["Stochastic Host Telemetry (Columns 8..=13)<br/>Varies naturally with host CPU, clock speed, and OS preemption"]

    Invariants --> H1["1. mnemonic (e.g. ADD, MOVE)"]
    Invariants --> H2["2. variant (e.g. ADD.W D1 D0)"]
    Invariants --> H3["3. addressing_mode (e.g. DataRegDirect)"]
    Invariants --> H4["4. category (e.g. Arithmetic)"]
    Invariants --> H5["5. opcode_hex (e.g. D041)"]
    Invariants --> H6["6. amiga_cck_cycles (e.g. 4, 8, 34)"]
    Invariants --> H7["7. total_guest_instructions (Nominal count)"]

    Telemetry --> T1["8. host_duration_median_ms"]
    Telemetry --> T2["9. host_ns_per_instruction"]
    Telemetry --> T3["10. host_ns_per_guest_cck (R_norm)"]
    Telemetry --> T4["11. host_mips"]
    Telemetry --> T5["12. host_jitter_pct (CV)"]
    Telemetry --> T6["13. anomaly_flag (true/false)"]
```

- **Stochastic Telemetry (Columns 8–13):** Host nanoseconds, MIPS, and variance cannot be verified with static checksums. They are evaluated via statistical confidence bounds ($\text{CV} < 3\%$) and anomaly rules ($R_{\text{norm}}$ ratios) as detailed in [CPU Benchmark Analysis Guide](CPU%20Benchmark%20Analysis%20Guide.md).
- **Deterministic Invariants (Columns 1–7):** The opcode encodings, addressing mode names, cycle-exact Amiga timings, and total nominal operation counts are **strictly deterministic**. If an agent refactoring code accidentally changes an opcode encoding, modifies an addressing mode, alters an Amiga CCK cycle count, or skews test iteration counts, the invariant columns will diverge.

### 10.2 Strictly Nominal `total_ops` Invariance

In earlier iterations, `total_ops` was calculated from the number of passes surviving Tukey's fences outlier rejection (`instructions_per_pass * filtered_count`). Consequently, on noisy host runs where 1–2 outlier passes were dropped, `total_ops` fluctuated (e.g. 119M vs 149M), making nominal operation counts unstable.

To ensure absolute repeatability, `total_ops` is computed using **nominal pass counts** (`raw_count` in `stats.rs`):

$$\text{total\_ops} = K \times \text{iterations} \times \text{passes}$$

| Profile | Unroll Factor ($K$) | Inner Iterations | Outer Passes | Nominal `total_ops` (Invariant) |
| :--- | :---: | :---: | :---: | :---: |
| **`quick`** | 700 | 3 | 15 | **31,500** |
| **`standard`** | 700 | 1,428 | 7 | **6,997,200** |
| **`thorough`** | 700 | 14,285 | 15 | **149,992,500** |

All 108 benchmark specifications within a given profile have identical nominal `total_ops`.

---

## 11. Reference Documentation & Upstream Ground Truth

- [68000 User's Manual: Section 8 (16-Bit Instruction Execution Timing & Bus Tables)](../../Reference/68000%20User%27s%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): Baseline instruction execution times and memory bus cycle references.
- [CPU Instruction Benchmark Catalog](CPU%20Instruction%20Benchmark%20Catalog.md): Comprehensive catalog of 108 benchmarked instruction variants.
- [CPU Instruction Benchmark Strategies](CPU%20Instruction%20Benchmark%20Strategies.md): Strategy matrix for PRNG, cascading stacks, and pointer management.
- [CPU Benchmark Analysis Guide](CPU%20Benchmark%20Analysis%20Guide.md): Operational guide for interpreting host performance metrics and anomalies.
- [CPU Motorola M68000 Architecture](../CPU%20Motorola%20M68000.md): Register architecture, condition codes, and processor status.
- [CPU Micro-Step State Machine Specification](../CPU%20Micro-Step%20State%20Machine.md): Color Clock cycle decomposition and microcode execution.
- [Benchmark Harness Implementation](../../../../crates/test_runner/benches/bench_instructions): Living Rust benchmark harness, runners, and profiler models.




