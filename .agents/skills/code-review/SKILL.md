---
name: code-review
description: Arrange and perform independent review in fresh agent context before every authorized commit, at milestones, or on request; scale depth to the change and verify the exact snapshot.
---

# Independent Code Review

Use this skill before every authorized commit under [commit policy](../../rules/git-commits.md), for milestone or substantial feature review, and for an explicit review request. Ordinary edits alone do not trigger it. Review authorizes inspection and a report, not file changes, staging, or a commit.

The coordinating agent arranges the review and resolves findings. An assigned reviewer performs the review itself and returns its report; it does not delegate another copy of this procedure.

## 1. Delegate with Fresh Context

Start a separate `code_reviewer` without inherited conversation history. With Codex collaboration tools, use `spawn_agent` with `agent_type: "code_reviewer"` and `fork_turns: "none"`; the role's configuration selects its model. A separate process, host or checkout is not required. A role name or instruction to ignore history does not isolate an already inherited conversation.

Supply only the information needed to identify the assignment:

- Repository location, review mode and delivery stage, exact baseline and target snapshot, selected paths/hunks, and explicitly included new files.
- Authoritative requirement locations and verbatim approved requirement changes when they are not yet represented there. Exclude the author's reasoning, implementation narrative, suspected defects and prior review conclusions from the initial assignment.
- Locations of raw verification reports and reproduction artifacts, including the source identity they cover. The reviewer first inspects requirements and behavior, then uses this evidence to test its assessment.

Do not fork the implementation conversation or send an author-written recap as a substitute for the patch. If fresh-context delegation is unavailable, report the missing independent review; the coordinating agent's own assessment cannot supply its approval.

## 2. Establish Scope and Depth

Use the exact intended snapshot procedure in [verify-spec-compliance](../verify-spec-compliance/SKILL.md#1-establish-the-intended-snapshot) without triggering that entire skill merely to select files. Inspect explicitly selected untracked files, both endpoints of renames and deletions, and the actual proposed content for partial or mixed staged/unstaged changes. Never substitute all dirty files for the assigned patch.

Record the baseline revision and target identity. A commit SHA identifies a committed target; for an uncommitted target, pair its baseline with a content digest or saved patch that includes selected new files and identifies the exact proposed versions. HEAD alone cannot identify a dirty snapshot. Read only that version when the worktree differs. Report scope ambiguity or a target changing during review instead of approving it.

Choose depth by the actual change, not just its line count:

- **Focused:** Default for ordinary commits. Read the complete intended diff and necessary surrounding code, trace affected callers and the main failure paths, and examine relevant tests and evidence. For documentation or rules, trace changed obligations through their owners, procedures and incoming references. Keep the report short; do not audit unrelated subsystems.
- **Extended:** Required for milestone closure, substantial features, or changes with significant correctness risks, such as CPU/bus timing, shared state transitions, snapshot compatibility or verification/reference handling. Include affected cross-subsystem interactions, representative edge and failure paths, and whether tests exercise production integration. At a milestone, cover the completed stage relative to its start, including earlier commits; reviewing only its last patch is insufficient. Coordinate the required [semantic audit](../audit-semantic-parity/SKILL.md) rather than assuming code review replaces it.

The reviewer may deepen a focused review when a concrete risk appears; state the reason and additional scope. A focused review reduces breadth, not the evidence required to accept an identified risk.

## 3. Assess Behavior Independently

Read [AGENTS.md](../../../AGENTS.md) and its current rules for the affected domains. Requirements remain owned there; do not invent a second set of quality rules. Derive expected behavior from approved requirements and inspect the patch and necessary surrounding implementation before consulting the author's verification results.

For changed behavior, follow input through decisions, state changes and observable output. Inspect callers and consumers beyond the changed lines where contracts cross a boundary. Look for a reachable failure: incorrect ordering or timing, incomplete transitions, wrong ownership, lost data, broken error handling, or a replaced mechanism still used by an active caller. Apply only risks relevant to this patch.

For a defect repair, check that the evidence demonstrates the original failure and that the repair changes its cause. For refactoring, check that affected callers moved to the replacement and that required behavior remains intact. For instruction changes, walk a realistic invocation through selection, execution, evidence, failure handling and completion; check for conflicting triggers, circular delegation and unreachable required steps.

Compare relevant tests with these paths. Determine whether their fixture, assertions and integration boundary could detect the failure, rather than treating test presence or a passing aggregate as proof. Identify the missing observable protection when it matters; follow [testing policy](../../rules/unit-testing-policy.md) rather than requiring tests per file.

Keep hypotheses distinct from findings. Support a defect with a concrete trigger or reachable path, the resulting behavior, and its consequence. A focused read-only diagnostic may resolve uncertainty; do not change files, references or snapshots to demonstrate a problem. Report pre-existing issues separately from introduced regressions and explain any effect on required gates.

For review outside an authorized commit, assess known specification requirements and encountered conflicts; do not trigger the whole pre-commit compliance workflow unless requested. At commit or milestone review, inspect its scoped compliance/audit evidence and investigate concrete gaps; do not repeat an already current audit by default.

## 4. Check Evidence and Snapshot Freshness

Inspect raw results after forming the behavioral assessment. Check what actually ran, its outcome, source identity and coverage limits. Distinguish a synthetic fixture from production integration and static inspection from behavioral verification. Mark required obligations **unverified** when evidence is absent or stale.

For an authorized commit, require a passing `python tools/verification/verify.py commit` result for the final intended source; milestone completion also needs its milestone evidence and other required procedures. The owning workflow selects and runs these gates. Do not repeat them solely because this skill was invoked. A task-only review needs the checks applicable to that task, not an unrequested commit profile. Invoke [audit-code-quality](../audit-code-quality/SKILL.md) only under its own trigger.

Review cannot make a failed mandatory gate acceptable, change a protected reference, waive a requirement or approve an unauthorized contract change. Handle conflicts through [specification compliance](../../rules/spec-compliance.md). Future staging is not missing evidence when the reviewed target is intentionally unstaged; the coordinator must later compare the index with the approved content.

## 5. Report and Resolve Findings

Lead with actionable findings ordered by impact. For each, provide severity, affected path and location, the trigger or violated requirement, consequence, evidence, and the concrete correction or verification needed. Separate blocking defects and unmet obligations from optional suggestions; taste, speculative risks and unrelated cleanup do not become blockers.

Then report the baseline and target identity, mode and actual coverage, observed verification and its freshness, and material limits. State **APPROVED** only when there are no unresolved blocking findings or required evidence gaps within that scope; otherwise state **CHANGES REQUESTED** with next actions. If there are no findings, say so without claiming that unexamined behavior was proved. A short focused review does not need a table for every rule group.

The coordinator fixes supported findings within the authorized scope or returns concrete counter-evidence. An author disagreement alone does not close a finding. The reviewer checks the correction or evidence and explicitly closes or retains each blocker. Escalate unresolved requirement or scope decisions to the user; do not weaken a rule or keep retrying without new evidence.

For a recheck, the original reviewer may retain its own review context and receive the updated patch and raw evidence, without the implementation conversation. Examine changed paths and affected interactions, including regressions introduced by corrections. If the target or required evidence changed, issue an updated verdict tied to the final snapshot; earlier approval does not transfer automatically. An extended review may satisfy the focused commit review only when it covers that commit's final scope and current evidence. The coordinator verifies that the staged content matches the approved target before committing.
