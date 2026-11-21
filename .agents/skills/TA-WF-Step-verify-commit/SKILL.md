---
name: TA-WF-Step-verify-commit
description: Run the selected commit verification profile and report actual checks without repairs, staging or commits.
---

# Run the selected commit profile

Executor: main session or `TA-test-runner` for an independent execution assignment.

Apply the [result contract](../../references/workflow-contract.md) and
[command reference](../../../tools/verification/README.md#completion-verification).
Inputs: selected checkout, requested verification scope and optional output directory.

1. Confirm required local tools and fixtures and avoid concurrent writers to shared
   build/coverage outputs.
2. Run `python tools/verification/verify.py commit`, adding `--results-dir <path>`
   when requested. The profile selects its existing checks, Rust tests, coverage
   and tool tests; every invocation executes its selected commands.
3. Report actual commands, outcomes, output paths, failures and unrun work. A
   profile result does not authorize a commit, review, merge or cleanup.

Stop after verification. Do not repair findings, stage, commit or dispatch reviewers.
