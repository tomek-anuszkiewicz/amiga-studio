# Hardware research provenance

Manual requirements, measured physical signals and reference-emulator/FPGA models
are distinct evidence classes. Preserve revision, source path/URL and supporting
passage for each material claim. Explain uncertainty and disagreement; never
infer hardware truth solely from a model.

## Select the evidence source

Use existing evidence when it resolves the hardware question. Otherwise, select
[manual research](../SKILL.md) first and retrieve a focused passage before
searching large manuals. Use the Reference corpus under `Obsidian/Amiga/Reference/`
for manual evidence. Verify the relevant section or diagram and report gaps.
When retrieval is unavailable or insufficient, inspect local sources from the
selected family and identify that route.

Select measured hardware or a reference emulator/FPGA model separately when
the question calls for it. One skill reports missing evidence and returns to
the caller. Draw claims about physical behavior from manuals or measurements
and identify supporting passages or observed signals. Attribute modeled behavior
to the source revision. Project code, tests, Design and DevNotes can inform
implementation or engineering decisions. Repository instructions govern conduct
during research.

Use the [developer retrieval reference](../../../references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
for the CLI, optional MCP adapter, source scopes and status checks. Use the
[selected graph query](../../TA-query-graph/SKILL.md) for source relationships.
Bind the project or independent reference graph explicitly, verify definitions
and use `rg` or direct inspection when graph data is missing or stale. Select
indexing or graph refresh independently.

Research writes only result artifacts when permitted. If the runtime prevents
those writes, return the payload/text for the receiving session to persist.
Report actual inherited permissions; a role label is not a hard filesystem guard.
Do not edit source/documentation, publish indexes or refresh graphs.
