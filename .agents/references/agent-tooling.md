# Maintaining Agent Tooling

This reference is for contributors changing the repository's agent rules, skills,
checks or configuration. For everyday collaboration, start with
[Working with an AI Agent](../../docs/ai_agents.md).

## Agent Tooling Maintenance


- Rules own obligations, scope, and exceptions. Skills own steps, commands,
  diagnostics, and templates; conditional examples live in the owning skill's
  `references/` directory. Mechanical requirements belong in scripts and tests.
- Keep verification orchestration and coverage result processing in `tools/verification/`,
  worktree management in `tools/worktree.ps1`, and concrete audits and checks in `tools/verification/checks/`.
  The shared diary appender lives in `tools/log_diary.py`.
  A specialized script used by at most one skill belongs under that skill's
  `scripts/`; a shared script uses the directory matching its role. Supporting
  references count as part of their owning skill, not as additional consumers.
- Adding, removing, or renaming a rule requires updating active links and
  affected procedure references. No manual audit registry is required.
  Historical diary references remain historical. Reach design specifications
  through domain rules or the design index; do not duplicate every document link.
- Passive rules do not need duplicate skills. Active procedures must remain
  reachable through their owning skills. Report concrete executed checks and
  their limits; static checks do not establish complete behavioral enforcement.
## Shared Verification Routing

Skills select checks and diagnostics through `python tools/verification/verify.py`.
Use its `test`, `check` and `runner` subcommands; completion uses `commit` and
`milestone`. The command reference is [tools/verification/README.md](../../tools/verification/README.md).
Skill-owned scripts keep their own entry points. Workflow operations such as
bootstrap, conversion, indexing, worktree management, diary appending, tool
installation and recording approved reference baselines remain separate.

## Recording and Validating Changes

Commit policy is in [git-commits.md](../rules/git-commits.md); diary policy
is in [diary-maintenance.md](../rules/diary-maintenance.md). Inspect the
diff, append the diary entry with verification already observed, and run the
batch gates on the complete final candidate. Put the profile outcome
in commit messages and the final report; continue the same batch after corrections
using validated evidence. Prepare all entries and commit messages before freeze.
Obtain the independent [code review](../skills/code-review/SKILL.md) required by
commit policy on that same snapshot. Stage only the agreed paths when preparation or a commit
is authorized, preserving unrelated index entries:

```powershell
python tools/log_diary.py `
  --title "<Commit-sized change>" `
  --subsystems "<relative components>" `
  --changes "<change 1>; <change 2>" `
  --rationale "<reason or trade-off>" `
  --results "<observed verification>"
```

Entries use `### [YYYY-MM-DD HH:MM CET/CEST] — <Title>`; use `--date` for an
explicit local timestamp. Record failures and unrun checks honestly.

Use an English Conventional Commit message, for example:

```text
docs(rules): separate policy from execution procedures

Explain the concrete change and rationale.
Verification: list checks actually run and their observed results.
```

Usual types are `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, and `chore`;
scope names identify the affected subsystem or tooling. Implementation, tests,
design updates, and diary for the same concern belong together.

For an explicit scoped language task, use
`python tools/verification/verify.py check english files <paths>`; for a full requested
audit, use `python tools/verification/verify.py check english`. Ordinary turns do
not trigger either scan. During preparation, use the canonical preliminary English
gate and resume after corrections; do not duplicate it as a standalone pass. Both commit
and milestone profiles include this scan. Install its pinned dictionary dependency with `python -m pip install -r tools/verification/checks/requirements.txt`. Scope exclusions and text formats live
in `tools/verification/checks/check_english.json`; maintain them according to
[language-policy.md](../rules/language-policy.md). Explicit file arguments
scan those entire files using the same English dictionary and project vocabulary, except for `always_excluded_directories`. Task plans under `.agent/tasks/` are always excluded and need no vocabulary or file-specific exceptions. Reviewed terms live in [english_words/](../../tools/verification/checks/english_words/); deliberate literal examples use the scope catalog's exact-file `file_words` map. Invoke validation explicitly; use no lifecycle or Git hooks.


## Local Artifact Storage

During preparation, after builds and diagnostics and before batch freeze, run the
shared read-only report once from the repository root. Report its observation time
and limitations in the completion message; do not scan again after each commit:

```powershell
python tools/artifacts/report.py
```

The report measures `target` and `.agent/tmp`, lists their largest directories,
and groups current file sizes by creation date, modification date, and the later
of those dates. Each observed date has its own row, so continuous work does not
collapse into one large period. These groups describe surviving files, not
measured historical growth, actual work sessions, or last access. A file created long ago but rewritten recently
belongs to a recent later-date group. Copies and extracted files can preserve
both timestamps, so inspect directory context before interpreting very old dates.
Missing creation times, scan errors, and excluded links are reported explicitly.
The totals are logical file sizes, not exact physical disk allocation.

Compare earlier groups with the latest activity and identify substantial
accumulation across multiple work dates or periods. Base a cleanup recommendation
on the amount and share of earlier data, its directory, and task status; quote
those observations. Do not use a fixed number of calendar days or a size threshold
alone. Old build dependencies may still be used, and a large fresh build may be
normal. Never label timestamp groups as proven unused or promise that their size
is the lasting space saving after rebuilding. If the data is mostly recent or
task status is unclear, report that without recommending a broad purge.

Include these optional commands, explaining that the next build recreates its
needed artifacts and takes longer. Run cleanup only on a separate user decision
and while affected builds, tests, and programs are stopped:

```powershell
cargo clean --target-dir target --dry-run
cargo clean --target-dir target
cargo clean --target-dir target/llvm-cov-target
```

The last command removes only the separate coverage build cache. At roadmap
closure, identify `.agent/tmp` directories belonging to the completed work,
confirm necessary conclusions are preserved in the committed record, and provide
`Remove-Item -LiteralPath '<verified repository-relative directory>' -Recurse -Force`
commands with actual reviewed paths. Resolve each proposed path inside
`.agent/tmp`; do not select an entire directory if it contains active work,
unresolved evidence, or files with uncertain ownership. Date alone is insufficient
to select these paths. If no directory qualifies, state that instead of inventing
a cleanup candidate. The commit and its roadmap closure share one report.

Keep the report in the conversation; do not create report files, scheduled jobs,
or hooks. A failed scan affects the report's completeness, not the validity of a
otherwise verified candidate. Test changes to this tool with
`python tools/verification/verify.py test python directory tools/artifacts/tests`.

## Configuration and Reference Provisioning

Project MCP registration is in [`.codex/config.toml`](../../.codex/config.toml).
Agent roles, model selections and permissions are defined in
[`.codex/agents/`](../../.codex/agents); review those files when changing a role.
Check effective client configuration before claiming that a changed setting is
active in a running session.

The [developer guide](../../docs/developers.md) owns setup for RAG, Graphify and external
sources. Accepted project documentation is indexed through
[index-amiga-rag](../skills/index-amiga-rag/SKILL.md). Converted references
are produced by the standalone [PDF](../../tools/bootstrap/pdf-to-markdown/README.md)
and [HTML](../../tools/bootstrap/html-to-markdown/README.md) programs during initial
provisioning. Routine agent work reads the resulting Markdown and assets.
