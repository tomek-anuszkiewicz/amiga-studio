# Review decisions

Record reusable rules stating which reviewer claims to ignore and why they need
no repair. Give each entry a stable ID, a searchable subject and one short paragraph
with the ignored claim, any necessary condition and the reason. Link the owning
requirement where useful. Omit dates, conversation history and review verdicts.

Match the current claim and its conditions, not a reused finding number. General
workflow rules belong in their owning instructions; successfully repaired findings
belong in their reports. An entry does not override its owning requirement or
[protected verification](protected-verification.md#protected-verification).

## RD-001 Configured role defaults in environment inventories

Ignore findings demanding that environment inventories or telemetry also store
the agent role defaults (model and reasoning effort) or compare them per role.
This applies when the inventory records the correct `governing_commit`: the
configuration is already available in Git at that revision.
[Capture requirement](../skills/TA-WF-Step-capture-environment/SKILL.md).

## RD-002 Uncommitted changes during final verification

Ignore findings that final verification selects tests from uncommitted changes
instead of changes already recorded in commits. Pending files are a normal
intermediate state that the user resolves by discarding or committing changes.
New commits undergo verification; no additional mechanism is required to handle
this intermediate state.

## RD-003 Prepared and unchanged workflow environment

Normal workflow execution assumes an existing, available configuration for the
activity-recording services and unchanged Git ignore rules. Ignore demands for
additional handling of task startup without that configuration, record copying
while it is unavailable, or copying the same outputs again after Git ignore
rules change. These scenarios are outside the required operating conditions.

## RD-004 Effects of changing workflow tools

Ignore gaps in automatic test selection caused by changes to workflow programs,
their shared functions or the `.graphifyignore` filter. This includes omitted
tests of tools using the changed code and tests of later use of a generated
report. Also ignore obsolete columns, descriptions and links left in report
templates after changes to these programs. This exclusion includes effects
that appear during a later invocation.
