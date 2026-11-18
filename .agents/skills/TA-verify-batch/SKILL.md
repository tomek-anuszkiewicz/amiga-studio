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
integration also supplies its immutable base and applicable original source identity.

1. Confirm the full selected candidate is stable. The verifier executes the worktree,
   not merely the index; unrelated active writers must not contaminate it.
2. Run `python tools/verification/verify.py commit`. For the supplied integration
   scope use `commit --integration-base <B>` and the already required
   `--integration-source <S>` for an explicitly authorized rebase.
3. Inspect the final canonical verdict and retained artifacts. Preserve matching
   successful groups under the existing protocol; failed, missing, interrupted or
   invalidated groups must execute. Same command/results location supports a caller-
   selected continuation without inventing a cross-HEAD cache.
4. Return actual executed/retained groups, evidence identities, failures and limits.
   Reference metadata failures remain reported with paths and proposed corrections
   for their existing approval boundary; the verifier and this skill do not edit them.

Stop after verification. Do not fix findings, dispatch reviewers, stage, commit,
publish or start milestone verification. Only the verifier supplies its final verdict.
