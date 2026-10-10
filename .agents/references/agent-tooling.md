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
diff, run the per-commit gates on the intended snapshot, and append observed results.
Stage only the agreed paths when a commit is authorized:

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

For a full English vocabulary scan, use `python tools/verification/verify.py check english`. Both commit
and milestone profiles include this scan. Install its pinned dictionary dependency with `python -m pip install -r tools/verification/checks/requirements.txt`. Scope exclusions and text formats live
in `tools/verification/checks/check_english.json`; maintain them according to
[language-policy.md](../rules/language-policy.md). Explicit file arguments
scan those entire files using the same English dictionary and project vocabulary. Reviewed terms live in [english_words/](../../tools/verification/checks/english_words/); deliberate literal examples use the scope catalog's exact-file `file_words` map. Invoke validation explicitly; use no lifecycle or Git hooks.


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
