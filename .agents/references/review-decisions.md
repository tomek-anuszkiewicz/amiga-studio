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
