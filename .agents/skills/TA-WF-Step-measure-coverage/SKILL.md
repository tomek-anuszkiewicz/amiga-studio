---
name: TA-WF-Step-measure-coverage
description: Collect one explicitly selected coverage measurement through the existing verifier and return its scope and limits.
---

# Measure the assigned coverage scope

Apply [the result contract](../../references/workflow-contract.md).
Give `TA-test-runner` the selected crate or workspace scope, the exact
candidate, the reason or trigger for measurement, the output location and the
execution conditions.

1. Confirm the selected scope and the availability of required tools and
   fixtures. Read the coverage command contract in the
   [verifier reference](../../../tools/verification/README.md).
   Ordinary tests exclude `test_runner`.
   Collect coverage only for the selected request or preparation scope. Wait
   until the implementation is stable unless the user expressly requests an
   earlier measurement or there is a diagnostic reason for one. Keep the
   candidate and execution conditions stable.
2. Execute the supplied selection using
   `python tools/verification/verify.py test coverage crate <name>` or the
   already selected `test coverage workspace` scope. Both choices use the same
   measurement operation. Audit and repair require separate selection.
3. Keep execution evidence, exported data and the summary together. Report the
   command, exit status, candidate and build identity, reports, measured suites,
   covered scope, exclusions and limits of the observations. Attribute retained
   results to the original run and explain why they still apply.

Stop after measurement. Do not add tests, interpret uncovered code as an automatic
repair assignment, broaden the scope or launch a test audit.
