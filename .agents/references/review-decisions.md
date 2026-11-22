# Review decisions

Use this register for reusable decisions about reviewer claims that need no
repair. Give each entry a stable ID and a searchable subject. In one short
paragraph, state the claim, the conditions under which it can be ignored and
the reason. Link the owning requirement where useful. Omit dates, conversation
history and review verdicts.

Match the current claim and the conditions under which the decision applies.
A reused finding number is insufficient. Keep general workflow rules in their
owning instructions and successfully repaired findings in their reports. An
entry does not override its owning requirement or
[protected verification](protected-verification.md#protected-verification).

## RD-001 Configured role defaults in environment inventories

Ignore findings demanding that environment inventories or telemetry also store
the agent role defaults (model and reasoning effort) or compare them per role.
This applies when the inventory records the correct `governing_commit`.
Git already provides the configuration at that revision.
[Capture requirement](../skills/TA-WF-Step-capture-environment/SKILL.md).

## RD-002 Uncommitted changes during final verification

Ignore findings that final verification selects tests from uncommitted changes
instead of changes already recorded in commits. Pending files are a normal
intermediate state that the user resolves by discarding or committing changes.
New commits undergo verification; no additional mechanism is required to handle
this intermediate state.

## RD-003 Prepared and unchanged workflow environment

The workflow assumes that the configuration for the activity-recording services
already exists and is available, Git ignore rules stay the same, and archive
destinations are ignored and untracked. Ignore findings that demand extra
handling for starting a task without that configuration, copying records while
it is unavailable, or changes to a file's ignored or tracked status during the
workflow. Also ignore scenarios in which an ignored archive directory contains
a tracked or otherwise nonignored destination. This includes reusing an
identical existing file without repeating its exact-path ignore check, and
copying the same outputs again after Git ignore rules change. These scenarios
fall outside the required operating conditions.

## RD-004 Effects of changing workflow tools

Ignore gaps in automatic test selection caused by changes to workflow programs,
their shared functions or the `.graphifyignore` filter. This includes omitted
tests of tools using the changed code and tests of later use of a generated
report. Also ignore obsolete columns, descriptions and links left in report
templates after changes to these programs. This exclusion includes effects
that appear during a later invocation.
