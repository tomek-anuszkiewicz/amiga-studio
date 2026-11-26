# Independent Graphify datasets

A single installation of Graphify 0.9.74 handles both independent datasets.

| Selection | Scan root | Output | Corpus |
| --- | --- | --- | --- |
| `project` (default) | Checkout root | `graphify-out/` | Code and allowed project documents; Git exclusions apply |
| `reference` | `ref_src/` | `ref_src/graphify-out/` | Supported code only; Git exclusions are disabled |

```powershell
python tools/graphify/refresh.py --graph project
python tools/graphify/refresh.py --graph reference
./tools/bootstrap/bootstrap.ps1 -Graphify
```

These commands create missing graphs and refresh existing ones for their
selections. Bootstrap runs both selections and fails if either fails. With
`-All`, it provisions the sources before refreshing the graphs.

During task closure, the
[TA-WF-Main-close-task](../../.agents/skills/TA-WF-Main-close-task/SKILL.md) skill
invokes [TA-Graphify-refresh](../../.agents/skills/TA-Graphify-refresh/SKILL.md).
That skill runs `python tools/graphify/refresh.py --graph project` once in the
primary checkout on local `master`, after the final master tests and review
succeed and any corrective commits are in place. Closure records the final
SHA, including any plan-removal commit.

Worktree creation copies both existing graph directories from the primary
checkout. If a copy is missing or stale, use `rg` and inspect the source
definitions. Refreshing the reference graph is a separate operation that requires
explicit selection. If reference sources are missing, provision them with
bootstrap `-Sources`. The reference graph excludes `vAmigaTS/` because its
assembly formats are unsupported. It also excludes
JSON vectors, generated directories and binary formats. Other supported code
throughout `ref_src/` is eligible, including C/C++ and Verilog. Even with a
supported extension, a parser error can leave incomplete symbol information.
Extension support alone cannot establish complete coverage.

`--root <checkout>` selects the target checkout. Use it when targeting an older
checkout with the script from the invoking checkout. Without this option, the
script selects the checkout it belongs to, regardless of the current
directory. Project paths are relative to the checkout, and reference paths are
relative to `ref_src/`. The command fixes
`GRAPHIFY_OUT` to `graphify-out` before importing Graphify, binds saved Git-ignore
settings explicitly, and anchors copied root sidecars locally. All cache,
manifest, label, report, HTML, lock and extraction state belongs to that output.
Never share mutable graph state across worktrees. The nested reference output is
copied with `ref_src/`, without overwriting tracked `ref_src/.graphifyignore`.

The command keeps Graphify's existing extraction, reconciliation, clustering
and reports. It remembers resolved source paths within a single rebuild.
It uses an existing Graphify interpreter or finds the uv tool environment.
It performs no installation or upgrade and leaves the installed package as is.
It reports an explicit failure on unsupported versions. When it relaunches,
it preserves the arguments and status, sets `PYTHONHASHSEED=0` only when absent,
and uses UTF-8 output. `GRAPHIFY_MAX_WORKERS` keeps its usual worker-limit behavior.

A structural refresh updates the graph's structure, removes excluded or deleted
sources and retains valid project semantics. Updating document meaning requires
semantic extraction. For initial or requested extraction, the host agent uses
the [semantic extraction skill](../../.agents/skills/TA-Graphify-extract-semantics/SKILL.md).
The procedure uses `semantic.py prepare --graph project` and `apply --graph project --run`
to stage batches for the host agent and check whether they still match the source
and prompt. It requires no new API service, API key or background semantic
process. Report the freshness of document semantics separately. Reference
extraction never processes documents.

For native traversal without query-stamp, query-log or bytecode writes, use
`python tools/graphify/read.py query "<question>" --graph graphify-out/graph.json`.
The pinned raw CLI writes a query stamp. The adapter suppresses that write
only in its own process and leaves the installed package unchanged.
The query helper accepts an explicit `--graph` and an optional directory
`--scope`. For example, run `python tools/graphify/query.py "Paula" --graph reference --scope minimig`.
The helper filters nodes and edges before traversal using a disposable view under
`.agent/tmp/`. See the [query procedure](../../.agents/skills/TA-Graphify-query/references/query.md)
for provenance and write restrictions.
Do not install the upstream skill over the maintained repository procedure.

Focused checks:

```powershell
python tools/verification/verify.py test python directory tools/graphify/tests
python tools/verification/verify.py test python
```

Earlier work on refresh performance added memoization. This tool now keeps
the reference graph separate. Checking in advance whether the project has
changed remains future work. Establish any performance improvement with
comparable measurements.
