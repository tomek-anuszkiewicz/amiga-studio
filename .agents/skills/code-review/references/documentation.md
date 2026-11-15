# Specifications, Documentation and Governance Reviewer

Own requirements and the agreement of documentation, instructions and procedures.
Use the common contract in [code-review](../SKILL.md#assigned-reviewer-common-contract).

- Trace changed requirements to affected code and tests, including unchanged
  implementation files. Approved specifications govern behavior; omissions allow
  compatible detail, not an undocumented requirement change.
- Review applicable scoped compliance evidence. At actual milestones, check the
  broader semantic-review obligations; do not repeat a current audit by default.
- Classify documentation corrections as descriptive or requirement-changing.
  For unchanged reviewed behavior, recheck the documentation without reopening
  other lanes. Identify additional lanes only when their requirements, criteria
  or evidence are affected; a changed overall fingerprint alone is insufficient.
- For instruction changes, walk a realistic invocation through ownership, scope,
  approvals, checks, failure handling and completion. Find contradictory routing,
  unreachable steps and active examples for removed mechanisms.
- When AGENTS, rules, skills, role instructions or verification requirements change,
  identify affected review criteria and check this lane's checklist and role routing.
  Ask the coordinator to route other affected checklist checks to their owning
  reviewers and consolidate the results. Missing synchronization is a finding;
  unaffected checklists need no cosmetic edit.
- Check repository language, portable paths, relevant links, and any triggered
  diary/roadmap closure duties. Preserve unrelated and historical material.
- Leave runtime correctness, numeric-definition inspection and detailed test/gate
  accounting to their owning lanes; report any cross-lane conflict you discover.

Read [instruction ownership](../../../rules/information-hierarchy.md),
[specification compliance](../../../rules/spec-compliance.md) and
[documentation maintenance](../../../rules/docs-maintenance.md). Reviewer
checklists are maintained operational criteria; upstream requirements remain authoritative.
