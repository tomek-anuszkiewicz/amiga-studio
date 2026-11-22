# Hardware research provenance

Identify whether each claim comes from a manual, a physical measurement or a
reference emulator or FPGA model. For each material claim, preserve the source
revision, path or URL, and supporting passage. Explain uncertainty and
disagreement. A claim about physical hardware needs evidence beyond model
behavior.

## Select the evidence source

Use existing evidence when it resolves the hardware question. Otherwise, start
with [manual research](../SKILL.md#manuals) unless the assignment selects another
type of source. Retrieve a focused passage before searching large manuals. Use the
Reference corpus under `Obsidian/Amiga/Reference/` for manual evidence. Verify the
relevant section or diagram and report gaps. When retrieval is unavailable or
insufficient, inspect local sources of the selected type and state in your
report that you inspected them directly.

Use measured hardware or a reference emulator/FPGA model when selected by the
assignment; a combined investigation may cover several named source families.
Missing evidence does not authorize expanding to another family. Draw claims
about physical behavior from manuals or measurements
and identify supporting passages or observed signals. Attribute modeled behavior
to the source revision. Project code, tests, Design and DevNotes can inform
implementation or engineering decisions. Repository instructions govern conduct
during research.

Use the [developer retrieval reference](../../../references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
for the CLI, optional MCP adapter, source scopes and status checks. Use the
[selected graph query](../../TA-Graphify-query/SKILL.md) for source relationships.
Bind the project or independent reference graph explicitly, verify definitions
and use `rg` or direct inspection when graph data is missing or stale. Select
indexing or graph refresh independently.

When permitted, write only research results. If the runtime prevents those
writes, return the data or text so the receiving session can save it. Report the
filesystem permissions the session actually inherited. A role label by itself
does not establish or enforce those permissions.
Do not edit source/documentation, publish indexes or refresh graphs.
