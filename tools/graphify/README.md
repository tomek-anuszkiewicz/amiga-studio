# Unified graph refresh

Run from the repository root:

```powershell
python tools/graphify/refresh.py
```

The command uses Graphify 0.9.74's full AST rebuild and its existing corpus,
locking, reconciliation, clustering, report and HTML behavior. It memoizes source
path calculations on each resolver instance. The cache ends with that rebuild;
it cannot carry path identities between checkouts or refreshes. The installed
Graphify package is unchanged. Other versions fail explicitly because the command
uses Graphify's internal path resolver.

Use a Python interpreter with `graphifyy` installed. When the current interpreter
does not contain Graphify, the command locates the existing uv tool environment
and relaunches with its interpreter. It never installs or upgrades a tool.
`GRAPHIFY_OUT` and `GRAPHIFY_MAX_WORKERS` retain their Graphify meanings.
Like the original CLI, the command starts with `PYTHONHASHSEED=0` when the caller
has not selected a seed, waits for a relaunched interpreter and propagates its
status. Output streams use UTF-8. Explicit caller seeds remain unchanged.

This is the first change in `GRAPHIFY-REFRESH-PERFORMANCE-20261116`. The frozen
reference layer and unchanged-project preflight remain pending; refresh still
processes the selected reference sources. The command performs structural
extraction, without LLM calls; semantic document extraction remains a separate
operation described in the [Graphify skill](../../.agents/skills/graphify/SKILL.md).

Focused tests:

```powershell
python tools/verification/verify.py test python directory tools/graphify/tests
```

For graph equivalence and performance experiments, use independent copies of the
same graph/cache seed and the same source corpus for the original `graphify
update .` command and this command. Keep the repository root as their anchor and
compare the entire persisted graph, including node and edge attributes. Keep
clustering and all outputs enabled. Distinguish unchanged inputs from controlled
source changes; isolated timing observations are not typical performance claims.
