---
name: TA-WF-Step-verify-commit
description: Run the selected commit verification profile and report actual checks without repairs, staging or commits.
---

# Run the selected commit profile

The main session runs the profile, or `TA-test-runner` runs it when assigned
independent execution.

Apply the [result contract](../../references/workflow-contract.md) and
[command reference](../../../tools/verification/README.md#completion-verification).
Use the selected checkout and requested verification scope, with an output
directory if requested.

1. Confirm that the required local tools and fixtures are available. Avoid
   concurrent writes to shared build and coverage outputs.
2. Run `python tools/verification/verify.py commit`, adding `--results-dir <path>`
   when requested. On each invocation, the profile runs the existing checks,
   Rust tests, coverage measurements and tool tests it selects.
3. Report actual commands, outcomes, output paths, failures and unrun work. A
   profile result does not authorize a commit, review, merge or cleanup.

Stop after verification. Do not repair findings, stage, commit or dispatch reviewers.
