---
name: optimize-emulation
description: Profile Amiga emulation, identify redundant work and micro-optimizations, and report or implement measured improvements within the requested scope. Use for performance investigations and optimization passes across core execution and host presentation.
---

# Optimize Emulation

Reduce host work while preserving guest behavior. Combine sampled execution cost
with a logical audit of work frequency: even one repeated calculation can matter
millions of times per second. A profile ranks visible costs; it does not exhaust
the useful optimization candidates.

## Scope and Starting Evidence

- Follow the user's requested mode. Investigation or a report leaves production
  code unchanged; a plan-only request does not start measurements. An instruction
  to optimize authorizes implementation and measurement within that scope without
  another routine confirmation. Report broader proposals separately.
- Read [performance rules](../../rules/performance-and-readability.md), the
  applicable domain rules from [AGENTS.md](../../../AGENTS.md), and specifications
  for the affected mechanism before editing it. Existing behavior is a comparison
  baseline, not authority to contradict hardware requirements.
- Inspect current source, index and relevant prior evidence first. Keep unrelated
  work and staged entries intact. Bind measurements to the actual source content,
  build settings, executable and symbols; a commit ID alone misses local edits.
- Select the question: core throughput, instruction cost, GUI responsiveness,
  presentation, or browser execution. Separate emulation, buffer conversion,
  upload/drawing and pacing waits. A capped display FPS is not core throughput.
- For comparable prior mechanisms, read
  [project examples](references/project-examples.md). For profiler setup and
  existing measurement entry points, read
  [measurement tools](references/measurement-tools.md). Revalidate disposable
  diagnostics before reuse; their old results are not fresh measurements.

## Find Candidates by Cost and Frequency

1. Establish an optimized-build baseline with a repeatable workload and warmup.
   Preserve normal pixel/audio generation unless the experiment explicitly
   isolates it. Record machine configuration, PAL/NTSC, input/ROM identity, frame
   or instruction count, and whether GUI and pacing are included.
2. Profile the relevant steady-state thread and interval. Report functions with
   self and inclusive cost and their call context, not just totals per chip.
   Resolve inline frames against the exact binary's symbols. Inclusive parent
   and child shares overlap; do not add them as independent costs. Sample weights
   are not exact call counts or hardware cycle measurements.
3. Inspect frequent execution paths, including small callees hidden by inlining
   or sampling resolution. Look for repeated decoding of unchanged registers,
   empty-capacity scans, temporary array initialization/copies, duplicate work
   between consumers, unnecessary conversions, and repeated dispatch or checks.
   Inspect generated optimized code when a suspected copy, call, bounds check or
   arithmetic cost is uncertain; source operation counts are not host timings.
4. Where frequency or activity is unknown, use a bounded diagnostic with counters
   for calls, changed inputs, occupied entries, active channels and actual writes.
   Prefer a representative frame or interval over extensive per-CCK logging.
   Compare diagnostic output/state with the unchanged path; measure speed in an
   uninstrumented build. Idle observations do not establish maximum occupancy.
5. Rank candidates by total likely savings, evidence, complexity and correctness
   risk. Include small repeatable savings; do not impose a minimum percentage.
   Frequency times estimated cost saved is a prioritization estimate, not proof
   of saved host cycles. State what remains unknown and how to measure it.

For each candidate, explain the unnecessary work in plain language, identify the
file/function, show the observed cost or frequency, describe the proposed change,
and name the behavior that must remain intact. Give the user enough detail to
evaluate the idea independently. Label hypotheses and measured findings clearly.

## Reduce Repeated Work Without Changing Time

Prefer the smallest representation or computation change that removes the cost.
Useful questions include whether work can run only on an input change, inspect
only occupied entries, reuse a result, or use a small lookup table. Compare table
access and cache footprint with arithmetic rather than assuming a table wins.

For cached or skipped work, identify all dependencies and the moment they become
effective: applied register writes, DMA, reset, clone, restore, inspection and
mid-line changes. Prefer deriving an inactivity check from existing state when
that is sufficient. Include cache construction, invalidation and active-case
costs in the experiment. Avoid two independently writable sources of truth.

Preserve observable phases, write ordering, bus waits, read side effects,
interrupts, collisions and snapshot continuation. Precomputed output must not
apply future side effects early. Host input collection and guest device clocks
are separate concerns; an idle fast path does not authorize less frequent guest
sampling. Changes to guest timing are requirement decisions under
[specification compliance](../../rules/spec-compliance.md), not performance fixes.

## Compare and Verify

- Preserve baseline and candidate builds before timing; use separate output
  locations when rebuilding would replace the baseline or symbols. Never reset
  someone else's source/index to obtain a baseline.
- Keep compiler, optimization settings, features, workload, warmup and host
  conditions comparable. Run repeated unprofiled A/B measurements; interleave
  versions, for example A-B-B-A, to expose drift. Avoid concurrent builds, tests
  and profilers. Report sample count and observed spread; investigate disturbed
  runs without discarding inconvenient results.
- Measure the affected active cases as well as idle ones, including relevant
  transitions and frequent invalidation. For core changes, measure whole-machine
  execution when available. A tiny gain reproducible only in a focused benchmark
  can still be useful; label whole-machine impact as unresolved if below noise.
  Native speed does not establish Wasm speed; preserve both supported builds and
  measure in a browser when the requested conclusion concerns browser performance.
- Report absolute time and relative change with units and scope. Frame-time
  reduction and throughput increase use different denominators. A larger sample
  share after other work shrinks does not by itself demonstrate a regression.
- Use existing focused behavioral tests and add coverage for concrete risks.
  For timing/state refactors, compare deterministic outputs and state at relevant
  boundaries, including active transitions and restore where affected. Differential
  equality preserves the starting model; it does not independently prove silicon
  behavior. CPU benchmarks do not replace reference-vector checks.
- Follow [testing policy](../../rules/unit-testing-policy.md) for selection and
  protected checks. Keep reference timings and goldens intact; do not hide a
  difference with a mask or loosen acceptance criteria to retain an optimization.
  Commit/milestone profiles run only at their existing triggers.

Keep changes supported by the measurements and correctness evidence. If the
result is inconclusive, report that and choose a bounded follow-up appropriate to
the task; do not accumulate complexity on an assumed speedup. Remove only this
task's rejected experiments, preserving unrelated changes and useful evidence.
After a successful change, reprofile when needed to choose the next candidate.
Complete the requested pass and report remaining opportunities; continuous
optimization does not mean an unbounded run or automatic background work.

## Report

Lead with the measured result or the most useful finding. Include:

- Workload, source/build identity, measurement method and evidence location.
- Function-level costs and candidate explanations for an investigation.
- Before/after time, spread, active/idle results and measured scope for changes.
- Checks actually run, behavior preserved, regressions and unresolved uncertainty.
- Remaining proposals that need user judgment or another scoped pass.

Keep the report in the conversation unless a saved artifact is requested or needed
for reproducibility. Use `.agent/tmp/` for diagnostics and `.agent/tasks/` for
active plans. Durable lessons belong in a few evidence-backed examples, not a
copy of every profiling run. Staging and commits follow the existing user scope.
