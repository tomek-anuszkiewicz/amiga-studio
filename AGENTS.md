# Amiga 500 Emulator: Agent Instructions

## Working Agreements

Read these shared rules before task work; linked files require explicit reads:
- [Scoped changes and root-cause resolution](.agents/rules/structural-root-cause.md): Stay within the requested task, reuse definitions, and fix mechanisms.
- [Specification compliance](.agents/rules/spec-compliance.md): Surface conflicts with design specifications and never silently change golden references.
- [Path privacy](.agents/rules/no-external-paths.md): Keep committed paths portable.
- [Commit policy](.agents/rules/git-commits.md): Required commit points, user-controlled timing, validation, and diary entries.
- [Language policy](.agents/rules/language-policy.md): Keep repository artifacts in English and answer in the user's language.

Implement requested behavior and the technical changes necessary to make it work. Propose optional additions before implementing them; usefulness alone does not authorize a new feature. Leave product and editorial decisions to the user unless delegated explicitly. Before completion, review the diff against this boundary using the [scope rules](.agents/rules/structural-root-cause.md).

Run validation commands explicitly. Do not install, configure, or use lifecycle, code, or Git hooks. Use `.agent/tasks/` for execution plans and `.agent/tmp/` for disposable diagnostics and artifacts. Treat existing temporary artifacts as unverified until their assumptions are checked.

First identify the question and whether existing evidence resolves it. Use scripts for concrete measurement, verification, or data processing; do not repair a tool unless that repair is needed for the task.

For routine workflow steps, prefer existing tools and direct commands. Create helper scripts or extra evidence files only for a concrete task requirement those tools cannot meet.

During ordinary code work, read and follow relevant specifications as implementation requirements. Handle concrete conflicts immediately under [specification compliance](.agents/rules/spec-compliance.md). The authorized pre-commit workflow owns scoped compliance review and compatible documentation enrichment; milestone closure owns broader semantic review. Direct documentation tasks and explicitly requested reviews remain in scope.

## Development Defaults

This is a private, pre-release project without external users. Prefer the simplest correct implementation. Minimize code, abstractions, dependencies, and defensive behavior for hypothetical requirements; do not build speculative features or extensibility.

Use the nearest relevant implementation and tests as examples for conventions, helpers, structure, and verification commands. Reuse established patterns when they fit the task. Existing code and tests do not override approved specifications or hardware evidence; investigate inconsistencies rather than copying them.

Add tests for meaningful behavior or identified risks, not merely because code changed. Prefer a few high-value tests over redundant edge cases or elaborate infrastructure. Simplify or remove tests whose original purpose disappears, subject to [protected verification](.agents/rules/unit-testing-policy.md#protected-verification); architectural test changes and weakening other tests or required gates need explicit user approval.

### Compatibility and Internal Refactoring

Backward compatibility is not the default for internal APIs, schemas, configuration, or generated formats. Internal APIs are maintained together in this workspace; refactor them completely rather than preserving obsolete interfaces for hypothetical consumers. Do not infer a compatibility requirement from existing code, tests, documentation, or possible consumers. When both approaches are correct, choose simplicity over extensibility.

- Update affected callers, imports, tests, and fixtures within the requested change. Review code-driven documentation changes before an authorized commit, unless documentation work was explicitly requested earlier.
- Migrate affected internal callers and delete replaced implementations, compatibility aliases, forwarding wrappers, obsolete re-exports, duplicate namespaces, and legacy fallback paths in the same change. Do not add migrations, legacy parsers, deprecated interfaces, adapters, or parallel implementations without a concrete requirement. Preserve compatibility only when the user explicitly requests it or a documented external contract requires it.
- Until the first release, snapshot backward compatibility is not required.
- Preserve external specifications. For external contracts or other user data, explain the impact and obtain direction before removing old-format support. Ask before deleting user data that cannot be recreated.
- Before completion, search active code and tests for replaced symbols and mechanisms, review each remaining reference, and run the required checks and relevant tests. Review documentation and instruction references before an authorized commit.
- Before an authorized commit, retire active command examples, skill procedures and ownership mappings for a removed mechanism. Retained historical descriptions must state that the capability is unavailable; future plans must identify its prerequisites rather than present retired commands as executable. An explicit user request to preserve documentation takes precedence; report the resulting debt.

## Task-Specific Rules and Navigation

Read applicable domain rules before editing that domain. Choose direct source inspection, `rg`, or [Graphify](.agents/skills/graphify/SKILL.md) to suit the navigation question. Follow [amiga-rag.md](.agents/rules/amiga-rag.md) for domain/reference retrieval.

| Task | Rules and procedures |
| --- | --- |
| Rust workspace code | [Rust practices](.agents/rules/rust-best-practices.md), [performance](.agents/rules/performance-and-readability.md) |
| Production Rust source under `crates/*/src/` | [File cohesion](.agents/rules/file-size-and-cohesion.md) |
| Crate boundaries or public re-exports | [Workspace boundaries](.agents/rules/workspace-structure-and-reexports.md) |
| M68000 CPU performance tests | [Benchmark catalog and runner](benchmarks/m68k/README.md) |
| Chip, DMA, or bus behavior | [Hardware topology](.agents/rules/hardware-bus-topology.md), [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) |
| Tests or suspected defect investigation/repair | [Testing policy](.agents/rules/unit-testing-policy.md), [defect resolution](.agents/rules/structural-root-cause.md#defect-resolution) |
| GUI | [egui practices and visual verification](.agents/rules/egui-best-practices.md) |
| Documentation | [Documentation maintenance and vault links](.agents/rules/docs-maintenance.md), [document and instruction ownership](.agents/rules/information-hierarchy.md), [writing style](.agents/rules/practitioner-voice-and-tone.md) |
| Repository skills | [Document and instruction ownership](.agents/rules/information-hierarchy.md) |
| PowerShell tooling | Use [write-powershell](.agents/skills/write-powershell/SKILL.md) for invocation behavior, parameter descriptions, and validation. |
| Commit or milestone | [Diary](.agents/rules/diary-maintenance.md), [roadmap](.agents/rules/roadmap-maintenance.md), [design maintenance](.agents/rules/docs-maintenance.md) |
| Merge or worktree | [Merge policy and required procedures](.agents/rules/git-merge-commits.md) |
| Reference conversion bootstrap | Read [bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable [PDF](tools/bootstrap/pdf-to-markdown/README.md) or [HTML](tools/bootstrap/html-to-markdown/README.md) program README and [converter testing scope](.agents/rules/unit-testing-policy.md#bootstrap-converter-scope) |

Skill names and descriptions in `.agents/skills/` guide selection. Read the selected
`SKILL.md` and execute required procedures; discovery alone does not execute a skill.

Roles live in `.codex/agents/*.toml`; delegate only when requested by the user or
applicable instructions. Every authorized commit requires a separate `code_reviewer`
with fresh context through [code-review](.agents/skills/code-review/SKILL.md), as
required by [commit policy](.agents/rules/git-commits.md). MCP configuration is
`.codex/config.toml`.

## Machine Invariants

- Native desktop and `wasm32-unknown-unknown` remain supported. Core emulation consumes external buffers; it does not perform host filesystem, thread, or wall-clock I/O.
- The synchronization unit is the Color Clock (CCK), approximately 3.54 MHz PAL / 3.58 MHz NTSC. A 68000 bus cycle takes four CPU clocks, or two CCK cycles (CCK1 and CCK2). A blocked Chip RAM access returns `BusResult::WaitState`; the CPU waits without advancing its active micro-step.
- The top-level machine owns CPU, memory bus, Agnus, Denise, Paula, and both CIAs, tracking master CCKs with `cck: u64`.
- State snapshots remain queryable and serializable with `serde::Serialize` and `serde::Deserialize`, separate from runtime handles.
- Follow physical dependencies when planning: bus arbitration and clocks, autonomous DMA, display, peripherals, then firmware integration.
- Guest memory is big-endian. Use explicit endian conversion, never host-endian pointer casts or `transmute`. Bitwise operations and raw transfers may bypass swaps only when their semantics commute with byte reversal.
- Guest execution must not panic the host. Unmapped reads return `$FF` / `$FFFF` (configurable); odd-address word/long accesses raise Address Error, Vector 3. Runtime emulation uses no `unwrap` or `expect`.
- ALU operations and cycle counters use explicit wrapping arithmetic. Execution, memory access, and interrupt hot paths allocate no heap memory. Project rules prohibit custom `macro_rules!` and const-generic instruction handlers.

## Verification and Completion

Never stage files or otherwise change the Git index unless the user explicitly requests staging or a commit. Editing a file that is already staged does not authorize staging its new version; leave the edit unstaged and preserve the existing index entry. If a required roadmap commit would need staging, ask the user first.

Commits are mandatory when closing a minor roadmap point or major milestone. Otherwise, commit when the user explicitly requests it. Commits are atomic and written in English. **Every commit includes a new entry in DIARY.md's final, unnumbered Rolling Changelog.** At minor roadmap points or milestones, compact settled entries, verify specification compliance, enrich compatible documentation details, and update the remaining roadmap.

Per-commit verification:
```powershell
python tools/verification/verify.py commit
```
The commit profile checks frontmatter in every local Markdown file under
`Obsidian/Amiga/`, including Git-ignored reference books. If it reports missing
metadata, show the user the affected paths and the proposed frontmatter for
each file, then ask for approval before editing those files. After approval,
repair the files and rerun verification. The verifier never edits them.
Reference book metadata follows [Reference Book Metadata](.agents/rules/docs-maintenance.md#reference-book-metadata).
The canonical commit profile selects standard suites and architecture checks. Run relevant focused tests during development; full SingleStepTests and additional milestone checks are selected by the verification workflow at their required trigger. Do not claim unrun checks passed.
Do not commit if the commit profile fails for any reason. Resolve the failure and rerun the full profile on the final intended source; milestone completion also requires a passing milestone profile.

Milestone completion requires `python tools/verification/verify.py milestone`, [semantic parity review](.agents/skills/audit-semantic-parity/SKILL.md) of modified subsystems, specification compliance and compatible documentation enrichment, checkpoint updates, diary compaction, and pruning completed roadmap steps. The broader audit may satisfy the scoped pre-commit review when its coverage and snapshot match. **Prohibition of Blind Golden Hash Modifications:** Never silently change golden hashes or cycle references; follow the conflict and approval procedure in specification-compliance rules.

## Authoritative References

- [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) routes to subsystem specifications under `Obsidian/Amiga/Design/`.
- Hardware manuals and PRMs live under `Obsidian/Amiga/Reference/`.
- Reference emulators and suites live under `ref_src/`; software-generated 68000 SingleStep vectors are under `ref_src/SingleStepTests-680x0/68000/v1`.
