---
name: TA-research-reference-models
description: Inspect selected local emulator or FPGA sources and report modeled behavior with revision and limitations.
---

# Research the selected reference model

Apply [the invocation contract](../../references/workflow-contract.md) and
[hardware provenance](../TA-research-hardware-manuals/references/provenance.md).
Role: `TA-hardware-researcher`; result kind: `hardware_research`.
Inputs: concrete modeling question, selected sources/revisions and relevant prior
manual result when available.

1. Inspect the selected `ref_src/` implementation. For pinned `minimig/` and
   `amiga_replacement_project/`, read `_bootstrap_source.json` for source revision.
   Missing source is a reported gap, not permission to provision it.
2. For cross-file navigation, inspect the selected existing reference graph
   read-only, preserving source-qualified identities and `ref_src/` provenance.
   Read definitions directly; use targeted `rg` when graph data is absent/stale.
   Never refresh, install, extract or write graph/query state during research.
3. Return the model's behavior, concrete paths/locations, revisions, agreements or
   differences with supplied manual evidence, and parser/source limitations.

Stop after the selected model investigation. C/C++/Verilog graph support may omit
relationships; vAmigaTS assembly and JSON vectors are excluded. Do not substitute
model behavior for hardware proof or switch to manuals/web measurements.
