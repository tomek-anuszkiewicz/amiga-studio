---
name: optimize-emulation
description: Profile Amiga emulation, report concrete micro-optimizations, and implement measured improvements through simpler calculations and less repeated work within the requested scope. Use for performance investigations and optimization passes across core execution and host presentation.
---

# Optimize Emulation

Reduce host work while preserving guest behavior and a clear hardware model.
Pursue useful savings throughout the requested scope, including small operations,
without making execution logic harder to understand. Combine sampled execution
cost with a logical audit of work frequency: even one repeated calculation can
matter millions of times per second. A profile ranks visible costs; it does not
exhaust the useful optimization candidates. Deliver the concrete
[micro-optimization report](#report) without waiting for a separate request.

## Simplicity Is an Acceptance Condition

An optimization must explain both why less work is needed and why the remaining
work is sufficient. A measured speedup does not justify obscure control flow,
hidden side effects, special-case hacks or extra coupling between components.
Prefer a local simplification over a new mechanism for coordinating execution.

Keep registers, signals, phases and units meaningful. Do not pass private cache
indices, packed execution hints, sentinel numbers or other invented control
values between chips to bypass their normal work. Follow the existing
[hardware interfaces and ownership](../../rules/hardware-bus-topology.md).
Documented hardware encodings and clearly named local table indices are valid;
repurposing a signal or register as an optimization backchannel is not.

Local derived state is useful when its dependencies and update points are simple
and explicit. It must not become another independently writable model of the
device. Reject a candidate that requires scattered invalidation, hidden ordering
assumptions or a parallel execution path merely to retain a speedup.

## Scope and Starting Evidence

- Follow the user's requested mode. Investigation or a report leaves production
  code unchanged; a plan-only request does not start measurements. An instruction
  to optimize authorizes implementation and measurement within that scope without
  another routine confirmation. Report broader proposals separately.
- Use [candidate discovery](#find-candidates-by-cost-and-frequency) for an
  investigation and the [optimization loop](#iterative-optimization) for
  implementation. The loop can start from a current report or a named candidate;
  it does not require a complete audit before each change.
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
2. Profile the relevant steady-state thread and interval. Present function costs
   and their calling contexts using the [call tree format](#call-tree-format).
   Resolve inline frames against the exact binary's symbols. Inclusive parent
   and child shares overlap; do not add them as independent costs. Sample weights
   are not exact call counts or hardware cycle measurements.
3. Always inspect frequent execution paths in scope, including small callees
   hidden by inlining or sampling resolution. Look for repeated decoding of
   unchanged registers, empty-capacity scans, temporary array initialization/copies,
   duplicate work between consumers, unnecessary conversions, and repeated
   dispatch or checks.
   Inspect generated optimized code when a suspected copy, call, bounds check or
   arithmetic cost is uncertain; source operation counts are not host timings.
   Record concrete findings in the micro-optimization report even when their
   whole-machine cost is below sampling resolution. A call tree or a ranking of
   large functions alone does not complete this step.
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
Look for these mechanisms, choosing only those that fit the inspected path:

- **Simplify calculations.** Use established ranges and invariants to remove
  redundant arithmetic, decoding and checks. Keep the formula and its domain
  clear; do not replace ordinary arithmetic with unexplained numeric tricks.
- **Do no work when nothing can change.** Visit occupied entries and active
  channels, return before an inactive helper, or skip reconstruction of an
  unchanged result. Derive guards from existing state where possible.
- **Compute less often.** Move stable decoding to effective input changes or the
  relevant sample, line or field boundary. Reuse a local result across consumers
  instead of resolving, copying or converting it again.
- **Compute on demand.** Derive inspection data when requested. Where elapsed
  emulated time fully determines a device's result, settle it at access or
  control changes instead of updating it every CCK. First establish that no
  intermediate output, interrupt or other consumer needs those skipped updates.
- **Precompute bounded mappings.** Use constants or small immutable tables for
  repeated decoding and arithmetic. Derive entries from named constants, a clear
  formula or an authoritative mapping. Compare table access and cache footprint
  with arithmetic rather than assuming a table wins.
- **Reduce data movement.** Write directly to final storage, reuse existing
  buffers and clear only invalid regions when their validity is known. Preserve
  required output, capacity and geometry.

The [project examples](references/project-examples.md) show these mechanisms in
queues, game-port counters, RTC access, CIA timers, audio, display and DMA. They
are prompts for source inspection, not permission to apply every technique.

For cached or skipped work, identify all dependencies and the moment they become
effective: applied register writes, DMA, reset, clone, restore, inspection and
mid-line changes. Prefer deriving an inactivity check from existing state when
that is sufficient. Include cache construction, invalidation and active-case
costs in the experiment. Avoid two independently writable sources of truth.

Preserve observable phases, write ordering, bus waits, read side effects,
interrupts, collisions and snapshot continuation. Precomputed output must not
apply future side effects early. Stable signal levels may avoid redundant
delivery; repeated strobes and writes can still be distinct events. Host input
collection and guest device clocks are separate concerns; an idle fast path does
not authorize less frequent guest sampling. Changes to guest timing are requirement decisions under
[specification compliance](../../rules/spec-compliance.md), not performance fixes.

## Iterative Optimization

Work through one shared shortlist from the micro-optimization report and call
tree. Do not force alternation between small calculations and expensive branches.
Reuse applicable evidence and existing measurement tools; investigate only the
gaps that prevent the next decision.

1. **Select one candidate.** Rank removable work by likely savings, frequency,
   evidence, simplicity and risk. In the call tree, inspect self cost and
   descendants to locate the actual work; a wrapper's large inclusive share is
   not itself a reason to rewrite it. Confirm the proposal is still outstanding
   in current source.
2. **Define the comparison.** Preserve the last accepted source/build as the
   iteration baseline and retain the original baseline for the entire pass.
   Record the expected saving mechanism, required behavior, focused checks and
   representative workloads with the candidate. These define what will decide
   whether to retain the change.
3. **Implement one coherent change.** Use the simplest mechanism that removes
   the identified work. Keep its diff separable from unrelated edits and earlier
   accepted changes. Do not stack another speculative optimization on top before
   deciding this one's outcome.
4. **Check correctness, then measure.** Run the relevant behavioral checks first,
   then follow [Compare and Verify](#compare-and-verify) for repeated unprofiled
   baseline/candidate measurements. Include affected active cases and maintenance
   costs. Use a focused benchmark when whole-machine noise hides a small saving;
   keep the scope of that evidence explicit.
5. **Keep, reject or investigate a bounded uncertainty.** Retain a change only
   when its benefit is supported and it meets the behavior, simplicity and
   workload acceptance conditions. Do not conceal a slower case inside an average
   gain or silently accept a new tradeoff. For an inconclusive result, choose a
   specific follow-up that can resolve it; do not repeat measurements indefinitely.
   If rejected or still inconclusive, remove only this experiment's edits, keep
   its evidence and reason, and continue from the accepted baseline.
6. **Record the decision and continue.** Update the same report with the result,
   actual checks, measured scope and remaining work. An accepted candidate becomes
   the next iteration baseline. Reassess affected candidates instead of proposing
   work already removed. Continue within the authorized scope without asking for
   routine approval at every iteration.

Refresh a profile when changed code or workloads make its ranking unreliable, or
when the next choice lacks adequate evidence. A tiny local edit does not require
a new full audit, reference-emulator survey or all-workload profiling campaign.
Compare vAmiga or WinUAE only for a concrete unresolved mechanism or an explicitly
requested reference investigation; do not make their broad review a prerequisite
for ordinary iterations. Old captures remain historical evidence, not current
cost measurements. Reuse checks only while their relevant inputs remain valid;
commit and milestone verification retain their existing triggers.

Finish when the selected scope is covered, a user-specified limit is reached, or
no further simple candidate in scope has a justified experiment. Report remaining
and inconclusive opportunities without claiming exhaustive discovery. For retained
changes, compare the final combination with the original baseline at the pass's
measurement boundary and confirm relevant behavior on the final source. Reuse
final-iteration evidence when it already covers that comparison; do not derive
the combined gain by adding individual percentages. Continuous optimization does
not authorize an unbounded run or automatic background work.

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

## Report

For each performed investigation or optimization pass, create or update
`micro-optimizations.md` in its task evidence directory under `.agent/tmp/`, unless
the user explicitly requests another destination or a conversation-only result.
Include a link in the final response. This is a required deliverable, not an
optional follow-up to a profile. A plan-only request specifies this future
deliverable without starting the audit or inventing findings.

Scale the report to the requested scope; do not impose a candidate count or expand
a focused task into a whole-workspace audit. Include:

- **Summary and inventory:** the most useful finding or measured result, stable
  candidate IDs, priorities and current statuses such as open, partial,
  implemented or rejected. Implementation, behavioral verification and measured
  speedup are separate claims.
- **Evidence and its limits:** source/build identity including local changes,
  workloads, measurement boundaries and links to available profiles, frequency
  observations, disassembly or diagnostics. Distinguish current observations,
  historical evidence and hypotheses; state missing measurements explicitly.
- **Concrete candidates:** for each ID, give the file/function, repeated work,
  relevant cost or frequency, the smallest clear change, why it is valid, and
  what remains after any existing optimization. Show the formula, skip condition
  or data flow rather than generic advice to optimize a function. Include
  dependencies, update or invalidation points, memory costs and behavior at risk
  where applicable.
  Describe focused verification and a bounded measurement that could decide the
  proposal; an algebraic identity alone proves neither a speedup nor full device
  correctness.
- **Unsupported or rejected ideas:** explain relevant cases where the compiler
  already removes the work, a guard or table costs more, the evidence is missing,
  or the change would complicate the model or violate hardware behavior.
- **Results and next steps:** checks actually run, before/after times and spread
  where measured, active/idle behavior, regressions, unresolved findings and the
  remaining implementation order. An unmeasured candidate carries no invented
  percentage gain. If none survives inspection, report that with its reasons.

Function-level profiling also follows the [call tree format](#call-tree-format);
link it from this report rather than using it as a substitute for the candidate
audit. After implementation, refresh affected statuses and residual proposals
against the final source. Retain earlier captures as historical evidence; do not
present their costs as current or recommend completed work again.

Use `.agent/tasks/` for active plans. Durable lessons belong in a few
evidence-backed project examples, not a copy of every profiling run. Staging and
commits follow the existing user scope.

### Call Tree Format

Make the call tree the primary function-level profiling result. Preserve calling
contexts; the same function may appear on several branches. Use expandable rows
in saved HTML reports. A flat function ranking may supplement the tree.

Show these three columns together, all relative to the entire captured thread:

| Column | Meaning |
| --- | --- |
| `Branch % capture` | Function plus all descendants as a percentage of the whole capture. |
| `Self % capture` | The function's own work, excluding descendants, as a percentage of the whole capture. |
| `Children % capture` | All descendants together as a percentage of the whole capture: branch minus self. |

Children includes every descendant level without counting overlapping branches
twice. Before rounding, `Branch = Self + Children`. Do not show `% of parent`.
Expanding, collapsing, filtering or changing the selected root must not change
the denominator or any retained node's percentages.

Above the tree, show the selected root and the whole selected tree's share of the
capture, plus the share outside that root. Do not normalize a selected subtree
to 100%. If milliseconds are included, label sampled time as an estimate for the
capture interval, not a measured duration per call or exact CPU/GPU time.
