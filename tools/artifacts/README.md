# Local task artifacts

These tools report local storage usage, preserve task outputs and append
engineering diary entries.

## Append a diary entry

When the user explicitly requests an entry, the caller supplies its content to
`diary.py`. The script formats the entry and appends it to the final Rolling
Changelog in the checkout's `DIARY.md`. An ordinary commit can proceed without
a diary entry. At milestone closure, diary compaction retains settled knowledge
in the chapters that cover it.
[Diary compaction](../../.agents/skills/TA-WF-Step-compact-diary/SKILL.md)

Write the entry using the skill's history criteria. Use the selected committed
changes as evidence for what was completed, why and with what results. Keep
pending edits and commit status in the task plan. The appender formats your text.
The evidence for its claims comes from the selected work.
[History criteria](../../.agents/skills/TA-WF-Step-compact-diary/SKILL.md#history-criteria)

Preview an entry before writing it.

```text
python tools/artifacts/diary.py --title "Describe the change" --subsystems "tools" --changes "Describe the result" --rationale "Explain the decision" --results "Record actual checks" --dry-run
```

Omit `--dry-run` to append the entry. `--date` accepts an explicit timestamp.
The script selects `DIARY.md` relative to its own checkout, independently of the
current directory. It does not inspect changes or run verification.

## Assess local storage

When the user requests a storage assessment, run this command from the
repository root.

```text
python tools/artifacts/report.py
```

The report measures `target` and `.agent/tmp` and lists their largest directories.
For the files still present, it groups logical file sizes by creation date,
modification date and the later of those two dates. Report when you made the
observation and list any missing timestamps, scan errors or excluded links.
Exact disk allocation, historical growth, work sessions and last access are
outside the report's scope. Copies and extracted files may retain old timestamps.

Compare the older date groups against recent activity, the purpose of each
directory and the status of the task. Old build dependencies may still be needed,
and a recent build can be large during normal use. Check the context before
treating data as unused. If you recommend cleanup, cite observed sizes and
explain the context. Report uncertain ownership or task status as uncertainty,
without inventing a cleanup candidate or promising lasting space savings.

Cleanup requires a separate user decision and stopped affected builds, tests and
programs. The next build takes longer because it recreates needed artifacts.
These commands show the available cleanup options.

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
Keep the assessment in the conversation. It needs no report files, scheduled
jobs or hooks. If a scan fails, report the resulting limit on storage evidence.
Verification of the code change still stands if it was independently established.

## Preserve task outputs

Run collection after tests and any other processes writing outputs have
finished. Choose the task's ignored archive in the primary checkout. For a
dedicated task worktree, run this command from a checkout that will remain.

```text
python tools/artifacts/preserve.py --worktree <task-checkout> --destination <primary-checkout>/.agent/task-archives/<selected-task-archive>/outputs
```

The tool copies ignored files from `.agent/tmp/verification`, `.agent/tmp/verify`,
`.agent/tmp/singlestep`, `.agent/tmp/coverage` and `benchmarks/results` in that
worktree. Add `--include <relative-path>` for custom outputs such as
`.agent/tmp/<task>` and any task reports stored in a local archive directory.

For work performed on the shared primary checkout, pass that checkout as
`--worktree` and use repeated `--include` options for the paths the task's
commands produced. Select the run's subdirectory when a directory holds outputs
from several runs.

```text
python tools/artifacts/preserve.py --worktree <primary-checkout> --destination <primary-checkout>/.agent/task-archives/<selected-task-archive>/outputs --include .agent/tmp/verification/<run> --include benchmarks/results/<run>
```

Reports written directly to an archive in the primary checkout stay at their
existing paths. Include an individual archived report to record its path in the
result. The tool skips archive directories in the primary checkout during
discovery.
It keeps the source files, checks copied content and refuses linked paths. When
a destination exists, it can reuse an identical file or keep both versions at
separate paths. Inspect the reported destinations and include any missing custom
outputs. If a copy fails, keep the source and earlier copies, and resolve the
failure before cleanup. Copying works independently of telemetry and time
windows.

[TA-WF-Main-close-task](../../.agents/skills/TA-WF-Main-close-task/SKILL.md) runs preservation
before the existing `tools/worktree.ps1` removal command.
