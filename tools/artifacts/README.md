# Local task artifacts

These tools report local storage usage, preserve task outputs and append
engineering diary entries.

## Append a diary entry

The caller supplies the content; `diary.py` formats it and appends it to the final
Rolling Changelog in the checkout's `DIARY.md` when an entry is explicitly
requested. Ordinary commits do not require an entry. Milestone closure uses
[diary compaction](../../.agents/skills/TA-WF-Step-compact-diary/SKILL.md) to retain
settled knowledge in its owning chapters.

Prepare the supplied text under the skill's
[history criteria](../../.agents/skills/TA-WF-Step-compact-diary/SKILL.md#history-criteria).
Describe completed work, its reason and actual results; use the selected committed
changes as evidence. Keep pending edits and commit status in the task plan.
The appender formats that text and does not establish that its claims are true.

Preview an entry before writing it:

```text
python tools/artifacts/diary.py --title "Describe the change" --subsystems "tools" --changes "Describe the result" --rationale "Explain the decision" --results "Record actual checks" --dry-run
```

Omit `--dry-run` to append the entry. `--date` accepts an explicit timestamp.
The script selects `DIARY.md` relative to its own checkout, independently of the
current directory. It does not inspect changes or run verification.

## Assess local storage

Run from the repository root when storage assessment is requested:

```text
python tools/artifacts/report.py
```

The report measures `target` and `.agent/tmp`, lists their largest directories,
and groups surviving file sizes by creation date, modification date and the later
of those dates. Report the observation time and any missing timestamps, scan errors
or excluded links. Sizes are logical file sizes, not exact physical disk allocation.
The groups do not measure historical growth, work sessions or last access; copies
and extracted files may preserve old timestamps.

Compare earlier groups with recent activity, directory purpose and task status.
Date or size alone does not establish that data is unused: old build dependencies
may still be needed, and a large fresh build may be normal. Quote the observed
amounts and context when recommending cleanup; report uncertain ownership or task
status without inventing a cleanup candidate or promising lasting space savings.

Cleanup requires a separate user decision and stopped affected builds, tests and
programs. The next build recreates needed artifacts and takes longer. Available
commands include:

```text
cargo clean --target-dir target --dry-run
cargo clean --target-dir target
cargo clean --target-dir target/llvm-cov-target
```

The last command removes only the separate coverage build cache. For selected
`.agent/tmp` cleanup, preserve necessary conclusions and evidence first. Resolve
each exact candidate inside `.agent/tmp`; exclude active work, unresolved evidence
and uncertain ownership. Provide commands only for reviewed paths, such as
`Remove-Item -LiteralPath '<verified repository-relative directory>' -Recurse -Force`.
Do not run them merely because a storage report or task closure was requested.
Keep the assessment in the conversation; no report files, scheduled jobs or hooks
are needed. A scan failure limits storage evidence rather than invalidating an
otherwise verified code change.

## Preserve outputs before removing a worktree

Run from a surviving checkout after tests and other producers have finished:

```text
python tools/artifacts/preserve.py --worktree <task-checkout> --destination <primary-checkout>/.agent/task-archives/<archive-name>/outputs
```

The destination must be ignored, inside the primary checkout and outside the
task checkout. The tool copies ignored files under `.agent/tmp/verification`,
`.agent/tmp/verify`, `.agent/tmp/singlestep`, `.agent/tmp/coverage`,
`.agent/task-archives` and `benchmarks/results`. It leaves source files in place,
checks copied content and refuses linked paths. Existing different files are
preserved under separate names; identical destination files can be reused.

Native test reports normally already live in the primary checkout's
`.agent/task-archives/` and survive task removal there. Local timing reports,
coverage JSON/gap summaries, SingleStep outputs, command logs and benchmark
measurements may still live in the task checkout and are included above.

Use repeated `--include <relative-path>` for known custom results, screenshots or
other requested ignored files. The tool does not copy all ignored assets, ROMs,
reference books, `.env`, build targets or arbitrary temporary directories.
Inspect its reported scope; zero copied files does not prove every custom output
was selected. A failed or incomplete copy stops cleanup. The tool never removes
a worktree or performs a commit, merge, registration or telemetry operation.

[TA-WF-Main-close-task](../../.agents/skills/TA-WF-Main-close-task/SKILL.md) runs preservation
before the existing `tools/worktree.ps1` removal command.
