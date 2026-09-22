---
name: profile-external
description: Profile Amiga 500 emulator execution hot paths using external sampling profilers (samply) and verify throughput against Git-tracked baselines.
---

# External Sampling Profiler & Performance Runbook

This skill guides the collection, analysis, and verification of emulator execution performance using external statistical sampling profilers (`samply` / Firefox Profiler) and Git-tracked benchmark baselines.

---

## 1. Core Architecture: Two-Tier Performance Monitoring

The emulator employs a two-tier performance monitoring strategy:
1. **Tier 1 — Automated Regression Sentinel (`chipset_benchmark_baseline.json`):**
   - Fast, deterministic benchmark suite running uninstrumented release code.
   - Compares active execution throughput (FPS and CCK frequency) against Git-tracked golden baselines (`tests/benchmarks/chipset_benchmark_baseline.json`).
   - Flags regressions exceeding the $-5.0\%$ threshold.
2. **Tier 2 — Deep Root-Cause Profiler (`samply`):**
   - External sampling profiler using OS thread timers and hardware counters.
   - Generates interactive, web-based flamegraphs in the Firefox Profiler UI.
   - Enables method-level and instruction-level drill-down with zero code instrumentation.

---

## 2. When to Profile & Collect Data

Execute profiling sessions under three explicit operational conditions:
- **Milestone Baseline Gates:** Run a benchmark audit before declaring major roadmap milestones complete.
- **Pre-Optimization Investigation:** When planning an optimization (e.g. Agnus DMA slot arbitration), run a profiling session first to record the exact percentage share and hot instructions before modifying code.
- **Post-Optimization Anti-Regression:** Profile immediately after an optimization to verify that the targeted function shrunk in the call tree and overall throughput improved.
- **Interactive Sluggishness Investigation:** If emulation dips below real-time 50 FPS (PAL) / 60 FPS (NTSC), record a trace to isolate the bottleneck.

---

## 3. Tool Installation (`samply`)

`samply` is a modern sampling profiler for Rust on Windows, Linux, and macOS:

```powershell
# Install samply once via Cargo
cargo install samply
```

---

## 4. Profiling Execution Runbook

### Step 1: Build the Release Binary
Always profile the optimized release build. Debug builds introduce artificial function call and un-inlined method overhead:

```powershell
cargo build --release -p test_runner
```

### Step 2: Record a Profiling Session with Samply
Run `samply record` targeting a representative workload or vAmigaTS test case:

```powershell
# Profile a Copper test over 100 frames
samply record target/release/test_runner.exe vamiga --test coptim1 --frames 100

# Profile the standard chipset benchmark suite
samply record target/release/test_runner.exe benchmark-chipset --frames 100
```

`samply` will launch a local server and automatically open the interactive Firefox Profiler UI in the default browser.

---

## 5. Analyzing Results in Firefox Profiler UI

In the Firefox Profiler web interface:

### A. Subsystem & Crate Filtering
Because Rust symbols retain their fully qualified crate and module hierarchy, you can filter by subsystem name in the search/filter box:
- `agnus` or `copper` or `blitter` or `dma`: Isolates all cycles consumed by Agnus coprocessors and bus arbitration.
- `denise` or `frame_builder` or `sprites`: Isolates rasterization, bitplane serialization, and pixel compositing.
- `paula` or `audio` or `floppy`: Isolates audio synthesis and disk controller processing.
- `m68000` or `memory_bus`: Isolates CPU micro-step execution and bank-mapped memory routing.

### B. View Options
1. **Flamegraph View:**
   - Visualizes horizontal stack width proportional to CPU time.
   - Wide rectangular bars indicate functions consuming the most execution time.
2. **Inverted Call Tree View (Bottom-Up):**
   - Shows the "self-time" of leaf functions.
   - Ranks the most expensive individual functions regardless of where they were called from.
3. **Marker Chart:**
   - Visualizes frame boundaries and vertical blanking intervals over time.

---

## 6. Extracting Method-Level Profile Breakdowns (`.agents/skills/profile-external/scripts/aggregate_profile.py`)

To track the exact percentage of time spent across individual chips and modules without adding invasive runtime probes, use `.agents/skills/profile-external/scripts/aggregate_profile.py`.

The aggregator inspects external sampling profiler traces (Samply / Firefox Gecko JSON or folded stack traces) and matches call frames against canonical entry methods:
- **CPU:** `Cpu::step_cck` (`cpu`)
- **Agnus:** `Agnus::step_cck_ram` (`agnus`)
  - `Copper::step_cck` (`copper`)
  - `Blitter::step_cck_ram` (`blitter`)
  - `DmaScheduler::arbitrate` (`dma`)
- **Denise:** `Denise::step_cck` (`denise`)
  - `FrameBuilder::set_cck_pixels` (`frame_builder`)
  - `SpriteEngine::step_cck` (`sprites`)
- **Paula:** `Paula::step_cck` (`paula`)
  - `Audio::step_cck` (`audio`)
- **CIAs:** `Cia::step_cck` (`cia`)
- **Floppy:** `FloppyController::step_cck` (`floppy`)
- **RTC:** `Rtc::step_cck` (`rtc`)

### Aggregation Command:
```powershell
# Parse a profile and view method breakdown table in terminal
python .agents/skills/profile-external/scripts/aggregate_profile.py profile.json --target coptim1

# Ingest profile results directly into the golden benchmark baseline
python .agents/skills/profile-external/scripts/aggregate_profile.py profile.json --target coptim1 --update-baseline tests/benchmarks/chipset_benchmark_baseline.json
```

---

## 7. Tier 1 Regression Sentinel: Baseline Verification

Before modifying code, check active performance against the Git-tracked baseline:

```powershell
# Run the automated regression audit
cargo run --release -p test_runner -- benchmark-chipset --compare
```

The comparison audit verifies overall FPS throughput against golden limits and prints the recorded baseline method profile breakdown.

### Updating the Golden Baseline
When an approved hardware optimization is verified and pass rates are preserved, update the committed golden baseline:

```powershell
# Record new golden metrics to tests/benchmarks/chipset_benchmark_baseline.json
cargo run --release -p test_runner -- benchmark-chipset --record --frames 50

# Verify the updated baseline comparison passes
cargo run --release -p test_runner -- benchmark-chipset --compare
```

Stage and commit `tests/benchmarks/chipset_benchmark_baseline.json` as part of the atomic commit for the optimization task.
