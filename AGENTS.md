# Amiga 500 Emulator: Agent Instructions

## Working Agreements

Read these shared rules before task work; linked files require explicit reads:
- [Scoped changes and root-cause resolution](.agents/rules/structural-root-cause.md): Stay within the requested task, reuse definitions, and fix mechanisms.
- [Specification compliance](.agents/rules/spec-compliance.md): Surface conflicts with design specifications and never silently change golden references.
- [Path privacy](.agents/rules/no-external-paths.md): Keep committed paths portable.
- [Commit policy](.agents/rules/git-commits.md): Required commit points, user-controlled timing, validation, and diary entries.
- [Language policy](.agents/rules/language-policy.md): Keep repository artifacts in English and answer in the user's language.

Implement requested behavior and the technical changes necessary to make it work. Propose optional additions before implementing them; usefulness alone does not authorize a new feature. Leave product and editorial decisions to the user unless delegated explicitly. Before completion, review the diff against this boundary using the [scope rules](.agents/rules/structural-root-cause.md).

Run validation commands explicitly. Do not install, configure, or use lifecycle, code, or Git hooks. Use `.agent/tasks/` for English execution plans and `.agent/tmp/` for disposable diagnostics and artifacts. Treat existing temporary artifacts as unverified until their assumptions are checked.

First identify the question and whether existing evidence resolves it. Use scripts for concrete measurement, verification, or data processing; do not repair a tool unless that repair is needed for the task.

During ordinary code work, use relevant specifications as implementation requirements, but defer code-to-design and design-to-code synchronization reviews to the authorized pre-commit workflow. Handle a concrete specification conflict when encountered; direct documentation tasks and explicitly requested reviews remain in scope.

## Task-Specific Rules and Navigation

Read applicable domain rules before editing that domain. Use [Graphify](.agents/skills/graphify/SKILL.md) for source navigation and [amiga-rag.md](.agents/rules/amiga-rag.md) for domain/reference retrieval.

| Task | Rules and procedures |
| --- | --- |
| Rust workspace code | [Rust practices](.agents/rules/rust-best-practices.md), [performance](.agents/rules/performance-and-readability.md) |
| Production Rust source under `crates/*/src/` | [File cohesion](.agents/rules/file-size-and-cohesion.md) |
| Crate boundaries or public re-exports | [Workspace boundaries](.agents/rules/workspace-structure-and-reexports.md) |
| Internal API refactoring | [Clean refactoring](.agents/rules/clean-break-refactoring.md) |
| CPU instructions | [Opcode conventions and required implementation procedure](.agents/rules/opcode-naming.md) |
| Chip, DMA, or bus behavior | [Hardware topology](.agents/rules/hardware-bus-topology.md), [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) |
| Tests or suspected defect investigation/repair | [Testing policy](.agents/rules/unit-testing-policy.md), [defect resolution](.agents/rules/structural-root-cause.md#defect-resolution) |
| GUI | [egui practices and visual verification](.agents/rules/egui-best-practices.md) |
| Documentation | [Documentation maintenance and vault links](.agents/rules/docs-maintenance.md), [information hierarchy](.agents/rules/information-hierarchy.md), [writing style](.agents/rules/practitioner-voice-and-tone.md) |
| Repository skills | [Instruction hierarchy and skill ownership](.agents/rules/information-hierarchy.md) |
| PowerShell tooling | Use [write-powershell](.agents/skills/write-powershell/SKILL.md) for invocation behavior, parameter descriptions, and validation. |
| Commit or milestone | [Diary](.agents/rules/diary-maintenance.md), [roadmap](.agents/rules/roadmap-maintenance.md), [design maintenance](.agents/rules/docs-maintenance.md) |
| Merge or worktree | [Merge policy and required procedures](.agents/rules/git-merge-commits.md) |
| Reference conversion bootstrap | Read [bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable [PDF](tools/bootstrap/pdf-to-markdown/README.md) or [HTML](tools/bootstrap/html-to-markdown/README.md) program README and [converter testing scope](.agents/rules/unit-testing-policy.md#bootstrap-converter-scope) |

Skill names and descriptions in `.agents/skills/` guide selection. Read the selected
`SKILL.md` and execute required procedures; discovery alone does not execute a skill.

Roles live in `.codex/agents/*.toml`; delegate only when requested by the user or
applicable instructions. MCP configuration is `.codex/config.toml`.

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

Commits are mandatory when closing a minor roadmap point or major milestone. Otherwise, commit when the user explicitly requests it. Commits are atomic and written in English. **Every commit includes a new DIARY.md Section 10 entry.** At minor roadmap points or milestones, compact settled entries and synchronize affected design specifications and the remaining roadmap.

Per-commit verification:
```powershell
python tools/verification/verify.py commit
```
The canonical commit profile selects standard suites and architecture checks. Run relevant focused tests during development; full SingleStepTests and additional milestone checks are selected by the verification workflow at their required trigger. Do not claim unrun checks passed.

Milestone completion requires `python tools/verification/verify.py milestone`, [semantic parity review](.agents/skills/audit-semantic-parity/SKILL.md) of modified subsystems, design synchronization and checkpoint updates, diary compaction, and pruning completed roadmap steps. **Prohibition of Blind Golden Hash Modifications:** Never silently change golden hashes or cycle references; follow the conflict and approval procedure in specification-compliance rules.

## Authoritative References

- [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) routes to subsystem specifications under `Obsidian/Amiga/Design/`.
- Hardware manuals and PRMs live under `Obsidian/Amiga/Reference/`.
- Reference emulators and suites live under `ref_src/`; 68000 silicon vectors are under `ref_src/SingleStepTests-680x0/68000/v1`.
