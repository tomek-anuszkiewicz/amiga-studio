---
name: TA-research-hardware-measurements
description: Inspect original measured hardware evidence for one selected question and distinguish observations from reconstruction.
---

# Research the selected physical measurement

Apply [the invocation contract](../../references/workflow-contract.md) and
[hardware provenance](../TA-research-hardware-manuals/references/provenance.md).
Role: `TA-hardware-researcher`; result kind: `hardware_research`.
Inputs: concrete physical signal/timing question, supplied observations or source
locations and bounded research scope.

1. Inspect original measurement evidence. For a needed web source, use focused
   search and open the original supporting article, not a search snippet alone.
   When relevant, begin with `site:techtravels.org <specific signal/question>`.
2. For Paula/DMAL, the existing
   [logic-analyzer account](https://www.techtravels.org/2026/08/from-flux-to-filenames-335ms-inside-paula/)
   is a starting source to inspect, not automatic proof of every internal detail.
   Separate measured traces/conditions from the author's reconstructed circuitry.
3. Return supported observations, source URL/path and passage, acquisition limits,
   inference boundaries, disagreements and unavailable evidence.

Stop after measurement research. If access is absent, report it; do not switch to
emulator-model research, infer an unmeasured clock phase or edit project files.

