---
name: TA-WF-Step-implement-change
description: Implement an approved feature, refactor, repair or selected optimization with its tests, necessary documentation and focused verification.
---

# Implement the approved scope

`TA-implementer` carries out the steps below and applies [the result contract](../../references/workflow-contract.md).
Start with the approved change and test plans or explicitly assigned equivalent
requirements, the candidate, the write scope, the relevant specifications and
the focused checks. A repair may also include selected finding IDs and evidence.
An optimization needs the selected candidate or a concrete account of how to
save work, the original baseline and the measurement scope.

Apply [vocabulary-only maintenance](../../references/workflow-contract.md#vocabulary-only-maintenance)
to justified word-list and exact-file literal additions. They do not trigger this
operation's checks or repairs; continue the current assignment. Changes to code,
authored text or other verification configuration keep their normal requirements.

1. Read the shared [Rust best practices](../../references/rust-best-practices.md)
   for Rust changes and [egui best practices](../../references/egui-best-practices.md)
   for GUI changes. Read affected specifications, nearby implementations and agreed test
   scenarios. Applicable review standards remain binding without loading all lanes.
   Apply [specification compliance](../../references/spec-compliance.md) to code and
   requirement changes. Record approved requirement changes before implementing
   them. For internal APIs, schemas or formats, apply
   [internal migrations](../../references/internal-migrations.md). Register mutation
   parameter changes use the [provisional modeling guidance](../../references/register-mutation-modeling.md).
   For test changes, apply the shared [testing criteria](../../references/testing-criteria.md).
   Converter changes follow their [technical/editorial testing scope](../../../tools/bootstrap/AGENTS.md#converter-testing).
   Check the linked design against the active plan, current code and existing
   tests. Reuse the design where it still applies. Distinguish the observations
   the plan calls for from results obtained by running the tests. Recheck any
   [protected-change approval](../../references/protected-verification.md#protected-verification)
   at the point of edit.
   For a defect, load [defect resolution](references/defect-resolution.md)
   and establish the original failure before changing behavior.
   For selected repairs or optimizations, apply the corresponding requirements
   below before mutation, including any required baseline preservation.
2. Implement the assigned behavior and agreed tests, migrating affected internal
   callers completely. Preserve unrelated source/index versions and protected
   references. Choose fixtures and assertions, and add meaningful cases for the
   approved behavior where needed. Selected review or coverage findings may
   justify filling gaps without approval for each test. Changes to behavior,
   acceptance criteria or write scope still need the applicable decision.
   Update documentation needed to keep the selected change accurate in the same
   assignment. Apply shared [document/instruction ownership](../../references/information-hierarchy.md)
   and [Documentation Value](../../references/documentation-criteria.md#documentation-value),
   with the shared [prose and narrative criteria](../../references/narrative-criteria.md)
   appropriate to each updated document's genre.
   A code edit alone does not require more prose. Preserve independent requirements
   and source-backed knowledge; update necessary links, examples and metadata.
   Honor an explicit documentation-preservation boundary and report required
   out-of-scope updates.
   When adding or extending a suite, check how its normal runner actually
   schedules individual cases. Use the existing runner that limits concurrent
   cases where supported. Mark cases that need to run in sequence and report
   unexpected sequential execution. Follow the shared criteria for test
   placement and isolation.
3. Check affected implementation and tests, including unchanged files when the
   selected behavior or requirements affect them. Editorial changes alone do not
   require code changes. Run the assigned focused checks and review the diff
   against authorized scope.
   Repeat checks only for changed inputs, failures or unresolved concerns. Use
   `verify.py test python` to check Python contracts for affected owners and
   shared inputs. Explicit directory selection serves diagnostics. For GUI,
   workspace-manifest or browser-pipeline changes, include
   `python tools/verification/verify.py check web`.
   Keep the candidate stable during checks and isolate concurrent writable targets.
4. Return changes, actual checks, findings and remaining work in text and JSON.
   Distinguish implementation from executed verification. No completed-source
   receipt or delivery declaration is required.

## Selected repairs

Read the selected finding's evidence and cited criterion. When there are no
review IDs, use the directly assigned defect. Confirm the original failure and
its mechanism before editing, and keep any causes the evidence cannot confirm
as hypotheses. Repair only the selected problem and the consumers and tests
that need to change. Use the [pruning procedure](references/pruning.md) for
confirmed unused-code findings. A scanner hit needs corroborating evidence
before you delete code or its tests.

Repeat the original reproduction and relevant focused regressions. Preserve supplied
finding IDs and report `repaired_pending_verification` until evidence verifies the
expected behavior. Keep unselected, deferred and unresolved findings visible.

## Selected optimizations

Read the [measurement contract](../TA-profile-emulation/references/optimization-evidence.md).
Check that the evidence still applies. Before editing, preserve the original
source, baseline build and matching symbols. If a required measurement input
is missing, return that gap. Explain which repeated work the change removes, what
that work depends on, when updates take effect and why the remaining work
preserves behavior. Reject hidden backchannels,
scattered invalidation and parallel sources of truth.

Run focused correctness checks before comparable repeated unprofiled A/B timing.
Include active, idle and transitional cases and maintenance costs. Keep a change
when evidence supports its benefit, behavior and simplicity; report regressions,
tradeoffs and inconclusive results. Remove only this experiment's rejected edits,
preserving unrelated and previously accepted work. Honor an explicit no-measurement
boundary and report the performance benefit as unverified.

Update the selected `micro-optimizations.md` report. Compare the final accepted
combination directly with the original baseline; never sum individual percentages.
Further investigation needs a concrete approach within the existing scope and
any user-supplied limit. Do not select another candidate or launch broad profiling outside
the assignment. Ordinary implementation and repairs do not start performance measurements.

Stop at the assigned result or contract boundary. Do not begin coverage measurement,
broad review, an unrelated documentation audit, staging, commits or another skill.
Required unresolved gaps remain blockers; report them for an explicit next step.
