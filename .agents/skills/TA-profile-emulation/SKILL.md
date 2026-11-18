---
name: TA-profile-emulation
description: Investigate selected emulation costs and frequent work, returning concrete optimization candidates without production edits.
---

# Profile the selected execution boundary

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `profile`.
Inputs: concrete performance question, exact candidate/build, selected workloads,
measurement boundary and output scope. A plan-only request supplies no measurement
authorization and returns that missing prerequisite.

1. Read relevant performance/domain requirements, the
   [measurement contract](references/optimization-evidence.md),
   [tool guidance](references/measurement-tools.md) and relevant
   [project examples](references/project-examples.md).
2. Establish an optimized-build baseline with repeatable workload/warmup and
   preserved output activity. Profile the selected steady-state thread/interval
   separately from unprofiled timing. Record build/symbol/input identities.
3. Inspect frequent paths and small callees hidden by inlining/sampling. Identify
   repeated decoding, scans, copies, conversions and duplicate work. Use bounded
   diagnostics or generated-code inspection only for concrete unresolved cost.
4. Write `micro-optimizations.md` in the task evidence directory with stable
   candidate IDs, concrete mechanisms, cost/frequency evidence, risks and bounded
   deciding measurements. Include the call tree when function profiling ran.
5. Return candidates, artifacts, actual measurements and limits in text/JSON.
   No surviving candidate is a valid outcome; do not invent savings.

Stop after investigation. Do not edit production code, start an optimization,
expand to an unrelated workload campaign or choose a next skill.

