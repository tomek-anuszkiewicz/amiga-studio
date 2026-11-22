---
name: TA-profile-emulation
description: Investigate selected emulation costs and frequent work, returning concrete optimization candidates without production edits.
---

# Profile the selected execution boundary

Apply [the result contract](../../references/workflow-contract.md).
The main session performs the profiling. It needs a concrete performance
question, the exact candidate and build, the selected workloads, what the
measurement covers and the intended output scope. For a plan-only request,
the main session reports measurement authorization as a missing prerequisite.

1. Read relevant performance/domain requirements, the
   [measurement contract](references/optimization-evidence.md),
   [tool guidance](references/measurement-tools.md) and relevant
   [project examples](references/project-examples.md).
2. Use an optimized build for the baseline. Choose a repeatable workload and
   warmup that preserve output activity. Profile the chosen thread and interval
   once the workload reaches steady state. Keep profiling separate from
   unprofiled timing. Record the exact build, symbols and inputs.
3. Inspect frequently executed paths and small functions that inlining or sampling
   can hide. Look for repeated decoding, scans, copies, conversions and duplicate
   work. Include recurring costs below the profiler's display threshold when
   evidence supports them. If a specific cost remains unclear, use a bounded
   diagnostic or inspect the generated code.
4. Write `micro-optimizations.md` in the task evidence directory with stable
   candidate IDs, concrete mechanisms, cost/frequency evidence, risks and bounded
   deciding measurements. Include the call tree when function profiling ran.
5. Return candidates, artifacts, actual measurements and limits in text and
   JSON. If the investigation finds no viable candidate, report that outcome.
   Report savings supported by the evidence.

Stop after investigation. Do not edit production code, start an optimization,
expand to an unrelated workload campaign or choose a next skill.
