---
name: profile-external
description: Profile an available emulator workload with samply and compare compatible handwritten CPU benchmark runs; whole-machine profiling requires an executable workload.
---

# External Sampling Profiler & Performance Runbook

Use the current CPU benchmark target for measurements and sampling profiles.
The old vAmigaTS runner, chipset workload and chipset baseline have been removed.
CPU measurements describe the instruction workload; they do not measure the
production chipset scheduler or whole-machine FPS. A requested chipset profile
requires a runnable representative workload before measurements can begin.

For measurement design and limitations, read
[Performance Profiling and Optimization Strategy](../../../Obsidian/Amiga/Design/Performance/Performance%20Profiling%20and%20Optimization%20Strategy.md).
For inlining decisions, read [inlining-examples](references/inlining-examples.md)
alongside measured profiles. Commit and milestone triggers follow
[AGENTS.md](../../../AGENTS.md); profiling is not an automatic completion gate.

## Select the Workload

Profile when investigating a demonstrated performance problem or evaluating a
requested optimization. Use the same workload and settings before and after the
change. Record the source snapshot, host, measurement profile and relevant
execution settings. Check the workload actually reaches the target function;
CPU-only benchmarks cannot establish a chipset optimization's value.

The active CPU workload is the handwritten catalog under `benchmarks/m68k/`.
Use `--plan` to inspect selected commands without starting measurements.

## Measure and Profile

Install `samply` when needed:

```powershell
cargo install samply
```

Run fresh optimized measurements separately from sampling:

```powershell
python tools/verification/verify.py runner bench case add/add_w_d1_d0
python tools/verification/verify.py runner profile bench case add/add_w_d1_d0
python tools/verification/verify.py runner analyze benchmarks/results/<run-id>
```

The measurement command creates a run directory under `benchmarks/results/` and
runs analysis automatically. The profile launcher builds the optimized benchmark
without execution, selects the executable reported by Cargo, and launches that
case under `samply`. Build failure or a missing artifact stops the launcher.
Its implementation is [profile_benchmark.py](../../../tools/verification/profile_benchmark.py).
Profiles are observations of host execution, not emulated clock assertions.

## Interpret the Profile

Use the flamegraph to locate expensive call paths and the inverted call tree to
inspect self-time. Filter by Rust crate or method names actually present in the
profile. A smaller percentage share alone does not prove faster execution:
compare elapsed cost or throughput from fresh measurements under comparable
conditions. Do not attribute absent chipset symbols to a measured speedup.

For a supported Gecko JSON or folded-stack file, the retained aggregator reports
samples matched to its configured emulator method names:

```powershell
python .agents/skills/profile-external/scripts/aggregate_profile.py .agent/tmp/profile.json --target cpu-bench
```

Its matcher covers CPU, Agnus, Copper, Blitter, DMA, Denise, FrameBuilder, sprites,
Paula, audio, CIA, floppy and RTC entry methods. Inspect matched and unmatched
samples before using these percentages as an explanation. The aggregator's
legacy baseline-writing option does not restore the removed chipset workload or
baseline; ordinary profile analysis does not update references.

## Validate an Optimization

Confirm that the target is represented in the selected workload. Compare fresh
measurements and profiles before and after the focused change, and run relevant
behavioral tests under [unit-testing-policy.md](../../rules/unit-testing-policy.md).
Report workload limits, measurement variation and checks not run. Preserve
production hot paths without host timers or profiling probes.

The approved CPU baseline in `benchmarks/` records the first full handwritten
measurement run. The decoder-derived reference remains in Git history.
Reference changes require evidence and explicit approval under
[spec-compliance.md](../../rules/spec-compliance.md). Include an approved update
only in its authorized atomic commit; analysis alone does not authorize baseline
replacement or a commit.
