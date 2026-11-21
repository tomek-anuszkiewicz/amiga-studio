---
name: TA-Hardware-research
description: Research a hardware question using selected manuals, original physical measurements or emulator/FPGA models, preserving each source's provenance and evidence limits.
---

# Research the selected hardware question

Apply the [result contract](../../references/workflow-contract.md) and
[hardware provenance](references/provenance.md).
Role: `TA-hardware-researcher`.
Inputs: concrete question, selected source families and locations/revisions,
existing evidence, output scope and any supplied limit.

Use existing evidence when it resolves the question. Start with manuals when the
assignment does not select another source family. A combined assignment can cover
several families; keep their claims and gaps distinct and do not expand beyond
the selected investigation because one source is insufficient.

## Manuals

Retrieve focused passages through `rag_search_reference`, not the mixed general
`rag_search` corpus as hardware-manual evidence. Verify that returned paths remain
under `Obsidian/Amiga/Reference/` and inspect supporting sections or diagrams.
If retrieval is unavailable or insufficient, search the same local Reference
corpus and report that route. Preserve exact book paths, sections and conflicts.

## Original physical measurements

Inspect original traces, acquisition conditions and supporting passages. When a
web source is needed, open the original article rather than relying on its search
snippet. For relevant questions, `site:techtravels.org <signal/question>` can help;
the [Paula/DMAL logic-analyzer account](https://www.techtravels.org/2026/08/from-flux-to-filenames-335ms-inside-paula/)
is a source to inspect, not proof of every internal detail. Distinguish measured
signals from reconstructed circuitry and unavailable clock-phase observations.

## Emulator and FPGA models

Inspect the selected `ref_src/` sources and record revisions. For pinned
`minimig/` and `amiga_replacement_project/`, read `_bootstrap_source.json`.
Missing sources are input gaps, not permission to provision them. Use the
selected existing reference graph read-only for navigation, then verify source
definitions; use `rg` when graph data is missing or stale. C/C++/Verilog parsing
can omit relationships; vAmigaTS assembly and JSON vectors are excluded.
Attribute modeled behavior to its implementation and revision.

Return supported claims, source paths/URLs and passages, model revisions,
agreements, disagreements and unresolved questions. Do not substitute model
behavior for physical hardware evidence or infer an unmeasured signal edge.
Stop after the selected investigation. Do not edit sources/documents, install
tools, index RAG, refresh graphs, launch another source-family investigation
outside the assignment or delegate nested research.
