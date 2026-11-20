---
name: TA-verify-batch
description: Execute the canonical commit profile for an identified batch and report evidence without repairs or lifecycle escalation.
---

# Verify the selected batch

Apply [the invocation contract](../../references/workflow-contract.md),
[batch evidence policy](../../rules/git-commits.md) and the
[completion command contract](../../../tools/verification/README.md#completion-verification).
Executor: main session; result kind: `verification`; profile: `commit`.
Inputs: exact candidate, actual batch trigger, baseline and results directory;
planned work supplies the selected execution registration, work item and round.
Integration also supplies its immutable base and applicable original source identity.

1. Confirm the full selected candidate is stable. The verifier executes the worktree,
   not merely the index; unrelated active writers must not contaminate it.
2. In the selected task checkout, run the commit profile through the
   [selected-command launcher](../../../tools/workflow/README.md#run-a-command-for-a-selected-execution).
   It requires the authoritative registration and refuses missing or conflicting
   binding before launching. Use standalone invocation only for ad hoc work
   outside the selected plan.

   ```powershell
   python tools/workflow/run_selected.py --registration <receipt> --work-item <item> --round <round> -- python tools/verification/verify.py commit
   ```

   For integration, append `--integration-base <B>` to the verifier command.
   For selected Combined Sources, use the assembly checkout and registration,
   verify against its recorded master base and retain the
   exact selected sources for review and freeze.
3. Inspect the final canonical verdict and retained artifacts. Preserve matching
   successful groups under the existing protocol; failed, missing, interrupted or
   invalidated groups must execute. Same command/results location supports a caller-
   selected continuation. HEAD changes start a new ordinary batch. Only the
   frozen mechanical commit procedure bridges accepted integration commits.
   Treat coverage execution, export and summary as one evidence bundle.
4. Return actual executed/retained groups, evidence identities, failures and limits.
   Reference metadata failures remain reported with paths and proposed corrections
   for their existing approval boundary; the verifier and this skill do not edit them.

Stop after verification. Do not fix findings, dispatch reviewers, stage, commit,
publish or start milestone verification. Only the verifier supplies its final verdict.
