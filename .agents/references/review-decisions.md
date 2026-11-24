# Review decisions

Use this register for reusable decisions about reviewer claims that need no
repair. Give each entry a stable ID and a searchable subject. In one short
paragraph, state the claim, the conditions under which it can be ignored and
the reason. Link the owning requirement where useful. Omit dates, conversation
history and review verdicts.

Use the shared [criteria for reporting findings](review-common.md#which-findings-to-report)
for general questions about future changes, external file edits and test gaps.
Keep repaired findings in their review reports. Match each project decision
below by its subject and conditions. Its owning requirement and
[protected verification](protected-verification.md#protected-verification)
take precedence.

## RD-001 Configured role defaults in environment inventories

Ignore findings demanding that environment inventories or telemetry also store
the agent role defaults (model and reasoning effort) or compare them per role.
This applies when the inventory records the correct `governing_commit`.
Git already provides the configuration at that revision.
[Capture requirement](../skills/TA-WF-Step-capture-environment/SKILL.md).

## RD-002 Uncommitted changes during final verification

Ignore findings that final verification selects tests from uncommitted changes
instead of changes already recorded in commits. Pending edits are normal input
and new commits undergo verification. In a full English scan, also ignore
findings that demand an incomplete result solely because an ordinary directory
occupies the working-tree path of a regular file still in the Git index. Scan
the eligible files in the replacement directory and validate vocabulary removals
against the current working tree. Restoring the file changes the scan inputs.

## RD-003 Prepared and unchanged workflow environment

Task capture assumes that its service configuration exists and is available.
Archive operations assume that Git ignore rules and tracking status stay
unchanged and that each destination in the archive is ignored and untracked,
including an existing file reused for an identical copy. Assess claims about
capture and copying under those conditions. For ordinary
[file preservation](../skills/TA-WF-Step-collect-task-records/SKILL.md)
the service configuration is optional.

## RD-004 Effects of changing workflow tools

Ignore gaps in automatic test selection caused by changes to workflow programs,
their shared functions or the `.graphifyignore` filter. This includes omitted
tests of tools using the changed code and tests of later use of a generated
report. Also ignore obsolete columns, descriptions and links left in report
templates after changes to these programs. This exclusion includes effects
that appear during a later invocation.

## RD-005 Filesystem links and path redirection

Review ordinary file paths across operating systems. Ignore findings that depend
on symbolic links, hard links, junctions or equivalent filesystem aliases or
redirection. Apply any explicit link-handling requirement in the assigned scope.
