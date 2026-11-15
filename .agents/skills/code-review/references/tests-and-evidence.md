# Tests and Verification Evidence Reviewer

Own test protection and the evidence supporting the exact candidate. Use the
common contract in [code-review](../SKILL.md#assigned-reviewer-common-contract).

- Match changed behavior and affected interactions to independently expected
  assertions. Executing a line, a changed test file or an aggregate percentage
  alone is not meaningful protection. Distinguish fixtures from real integration.
- Inspect current changed-code coverage when required for production emulator
  changes. Check uncovered spans, measured suites, conditional targets and stale
  object mappings before calling a path covered or missing. Resolve in-scope gaps.
- Verify the actual commands, outcomes, input identities, retained artifacts and
  evidence freshness. Matching successful groups may be reused; failed, missing
  or invalidated groups cannot count as passed.
- Distinguish renewed reviews from carried scoped approvals. Verify the comparison
  supporting unchanged content, requirements and evidence; preserve each carried
  verdict's original identity. Descriptive documentation edits alone do not require
  re-invoking unchanged runtime or architecture reviewers. Canonical test-evidence
  validity remains governed separately by the verifier.
- Check explicit approval for architectural tests/checkers/policy data, weakened
  assertions or gates, and protected golden/reference changes. A specific approved
  contract change can authorize its corresponding ordinary test update.
- At batch preparation require passing canonical commit evidence for the combined
  candidate; at actual milestone closure also require its selected milestone gates.
  Do not turn ordinary implementation review into unrequested batch preparation.
- Inspect partition coherence and claims about intermediate commits and unrun
  hardware, platform or performance checks. Report limitations accurately.

Read [testing policy](../../../rules/unit-testing-policy.md),
[commit policy](../../../rules/git-commits.md) and the relevant
[verification contract](../../../../tools/verification/README.md).
