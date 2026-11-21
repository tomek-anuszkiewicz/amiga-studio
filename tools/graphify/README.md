# Independent Graphify datasets

Graphify 0.9.74 is one installed package with two independent datasets:

| Selection | Scan root | Output | Corpus |
| --- | --- | --- | --- |
| `project` (default) | Checkout root | `graphify-out/` | Code and allowed project documents; Git exclusions apply |
| `reference` | `ref_src/` | `ref_src/graphify-out/` | Supported code only; Git exclusions are disabled |

```powershell
python tools/graphify/refresh.py --graph project
python tools/graphify/refresh.py --graph reference
.\tools\bootstrap\bootstrap.ps1 -Graphify
```

Each operation creates a missing graph or refreshes its existing graph. Bootstrap
runs both selections and fails if either fails; `-All` provisions sources first.
During task closure, [TA-WF-Main-close-task](../../.agents/skills/TA-WF-Main-close-task/SKILL.md)
invokes [TA-Graphify-refresh](../../.agents/skills/TA-Graphify-refresh/SKILL.md), whose
procedure runs `python tools/graphify/refresh.py --graph project`. This refresh
runs once in the primary checkout on local `master` after successful final master
tests/review and any corrective commits. Closure records the final SHA, including
any plan-removal commit.
Structural refresh does not update document semantics. Worktree creation copies
both existing graph directories from the primary checkout. If a copy is missing
or stale, use `rg` and inspect the source definitions. The reference graph uses
an explicit refresh. Provision missing reference sources with bootstrap `-Sources`.
`vAmigaTS/` is excluded; its assembly formats are unsupported. JSON vectors,
generated directories and binary formats remain excluded. Other supported code
throughout `ref_src/` is eligible, including C/C++ and Verilog. Parser errors may
leave partial symbols, and supported extensions do not imply complete coverage.

`--root <checkout>` selects a target checkout explicitly, including when an older
target uses the invoking checkout's script. Without it the script selects its own
checkout, independently of the current directory. Project paths are relative to
the checkout; reference paths are relative to `ref_src/`. The command fixes
`GRAPHIFY_OUT` to `graphify-out` before importing Graphify, binds saved Git-ignore
settings explicitly, and anchors copied root sidecars locally. All cache,
manifest, label, report, HTML, lock and extraction state belongs to that output.
Never share mutable graph state across worktrees. The nested reference output is
copied with `ref_src/`, without overwriting tracked `ref_src/.graphifyignore`.

The command retains upstream extraction, reconciliation, clustering and reports,
with source-path memoization limited to one rebuild. It uses an existing Graphify
interpreter or locates the uv tool environment; it never installs or upgrades.
Unsupported versions fail explicitly. Relaunches preserve arguments and status,
set `PYTHONHASHSEED=0` only when absent, and use UTF-8 output. Worker limits retain
`GRAPHIFY_MAX_WORKERS` semantics. The installed package is unchanged.

Structural refresh preserves valid project semantics and removes excluded/deleted
sources; it does not refresh document meaning. Initial and requested semantic
extraction uses the host agent through the [semantic extraction skill](../../.agents/skills/TA-Graphify-extract-semantics/SKILL.md).
The explicit `semantic.py prepare --graph project` / `apply --graph project --run`
commands stage host extraction chunks with source-and-prompt freshness checks;
the skill owns their execution procedure. No new API service, API key or background
semantic process is required. Report
semantic freshness separately. Reference extraction never processes documents.

For native traversal without query-stamp, query-log or bytecode writes, use
`python tools/graphify/read.py query "<question>" --graph graphify-out/graph.json`.
The pinned raw CLI writes a query stamp; the adapter suppresses that incidental
writer only in its process and leaves the installed package unchanged. For the explicit `--graph` / optional directory
`--scope` helper, run `python tools/graphify/query.py "Paula" --graph reference --scope minimig`.
It filters nodes and edges before traversal using a disposable view under
`.agent/tmp/`. See the [query procedure](../../.agents/skills/TA-Graphify-query/references/query.md)
for provenance and write restrictions.
Do not install the upstream skill over the maintained repository procedure.

Focused checks:

```powershell
python tools/verification/verify.py test python directory tools/graphify/tests
python tools/verification/verify.py test python
```

The earlier refresh-performance work supplied memoization. Reference isolation is
now implemented here; unchanged-project preflight remains separate future work.
No performance improvement is implied without comparable measurements.
