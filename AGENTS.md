# Amiga 500 Emulator: Agent Instructions

## Working Agreements

Read these shared rules before task work; linked files require explicit reads:
- [Scoped changes and root-cause resolution](.agents/rules/structural-root-cause.md): Stay within the requested task, reuse definitions, and fix mechanisms.
- [Specification compliance](.agents/rules/spec-compliance.md): Surface conflicts with design specifications and never silently change golden references.
- [Path privacy](.agents/rules/no-external-paths.md): Keep committed paths portable.
- [Atomic commits](.agents/rules/git-commits.md): Validate each task; commit on user request or completion of a minor roadmap point, with a corresponding diary entry.

Answer in the user's language; repository content and commits are English. Read [language-policy.md](.agents/rules/language-policy.md) for language checks, [audio-transcription.md](.agents/rules/audio-transcription.md) when audio is supplied, and [model-reasoning-advisory.md](.agents/rules/model-reasoning-advisory.md) when reasoning-effort advice is relevant. Do not guess unavailable model metadata.

Run validation commands explicitly. Do not install, configure, or use lifecycle, code, or Git hooks. Use `.agent/tasks/` for English execution plans and `.agent/tmp/` for disposable diagnostics and artifacts. Follow the [execution-plan lifecycle](.agents/rules/structural-root-cause.md#execution-plan-lifecycle) when closing tasks. Treat existing temporary artifacts as unverified until their assumptions are checked.

First identify the question and whether existing evidence resolves it. Use scripts for concrete measurement, verification, or data processing; do not repair a tool unless that repair is needed for the task.

## Task-Specific Rules and Navigation

Read applicable domain rules before editing that domain. Use [Graphify](.agents/skills/graphify/SKILL.md) for source navigation and [amiga-rag.md](.agents/rules/amiga-rag.md) for domain/reference retrieval.

| Task | Rules and procedures |
| --- | --- |
| Rust implementation | [Rust practices](.agents/rules/rust-best-practices.md), [performance](.agents/rules/performance-and-readability.md), [inlining](.agents/rules/method-inlining.md), [file cohesion](.agents/rules/file-size-and-cohesion.md), [workspace boundaries](.agents/rules/workspace-structure-and-reexports.md), [clean refactoring](.agents/rules/clean-break-refactoring.md) |
| CPU instructions | [Opcode conventions and required implementation procedure](.agents/rules/opcode-naming.md) |
| Chip, DMA, or bus behavior | [Hardware topology](.agents/rules/hardware-bus-topology.md), [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) |
| Tests or defect repair | [Testing policy](.agents/rules/unit-testing-policy.md), [repro-first](.agents/rules/repro-first.md) |
| GUI | [egui practices and visual verification](.agents/rules/egui-best-practices.md) |
| Documentation | [Documentation maintenance](.agents/rules/docs-maintenance.md), [vault links](.agents/rules/vault-linking-and-graph-integrity.md), [information hierarchy](.agents/rules/information-hierarchy.md), [writing style](.agents/rules/practitioner-voice-and-tone.md) |
| Repository skills | [Instruction hierarchy and skill ownership](.agents/rules/information-hierarchy.md) |
| Commit or milestone | [Diary](.agents/rules/diary-maintenance.md), [roadmap](.agents/rules/roadmap-maintenance.md), [design maintenance](.agents/rules/docs-maintenance.md) |
| Merge or worktree | [Merge policy and required procedures](.agents/rules/git-merge-commits.md) |
| Reference conversion bootstrap | [Conversion programs](.agents/references/developer-setup.md#processing-raw-documents-into-markdown), [developer-led workflow](tools/bootstrap/reference-conversion-contract.md#development-workflow), [converter testing scope](.agents/rules/unit-testing-policy.md#bootstrap-converter-scope) |
| Long-running checks | [Parallel execution](.agents/rules/parallel-execution.md) |

Skill names and descriptions in `.agents/skills/` guide selection; read the selected
`SKILL.md` before following its procedure. Keep skill-specific selection conditions
and steps in the skill, and project obligations in the applicable rules. Read the
rules routed above and execute the skills they require for the current task;
discovery alone does not execute a skill. Do not duplicate the skill catalog here.

Roles live in `.codex/agents/*.toml`; delegate only when requested by the user or
applicable instructions. MCP configuration is `.codex/config.toml`.

## Machine Invariants

- Native desktop and `wasm32-unknown-unknown` remain supported. Core emulation consumes external buffers; it does not perform host filesystem, thread, or wall-clock I/O.
- The synchronization unit is the Color Clock (CCK), approximately 3.54 MHz PAL / 3.58 MHz NTSC. A 68000 bus cycle takes four CPU clocks, or two CCK cycles (CCK1 and CCK2). A blocked Chip RAM access returns `BusResult::WaitState`; the CPU waits without advancing its active micro-step.
- The top-level machine owns CPU, memory bus, Agnus, Denise, Paula, and both CIAs, tracking master CCKs with `cck: u64`. Peer chips hold no direct references and do not call one another. The machine loop and memory bus route coordination and interrupt arbitration.
- Agnus exclusively generates Chip RAM DMA addresses and drives RGA. Denise and Paula latch bus data without direct memory reads. Signal and register effects follow modeled physical phases and propagation delays.
- State snapshots remain queryable and serializable with `serde::Serialize` and `serde::Deserialize`, separate from runtime handles.
- Follow physical dependencies when planning: bus arbitration and clocks, autonomous DMA, display, peripherals, then firmware integration.
- Guest memory is big-endian. Use explicit endian conversion, never host-endian pointer casts or `transmute`. Bitwise operations and raw transfers may bypass swaps only when their semantics commute with byte reversal.
- Guest execution must not panic the host. Unmapped reads return `$FF` / `$FFFF` (configurable); odd-address word/long accesses raise Address Error, Vector 3. Runtime emulation uses no `unwrap` or `expect`.
- ALU operations and cycle counters use explicit wrapping arithmetic. Execution, memory access, and interrupt hot paths allocate no heap memory. Project rules prohibit custom `macro_rules!` and const-generic instruction handlers.

## Verification and Completion

Validate every discrete task. Create Git commits only on explicit user request or when closing a completed minor roadmap point (for example, 1.1 or 1.2). Ordinary task completion does not authorize staging or committing. Commits remain atomic and in English. **Every commit includes a new DIARY.md Section 10 entry.** Completion maintenance follows the diary, roadmap and design-maintenance rules routed above.

Before every authorized commit, complete the scoped documentation review required by
[git-commits.md](.agents/rules/git-commits.md). Review alone does not require file
edits or authorize another commit.

Per-commit commands:
```powershell
python tests/verify.py --commit
```
In commit mode, the verifier always runs quality gates, architecture, all ordinary crate/standard harness tests, and Python tool contracts. Only full SingleStepTests and Cartesian DMA depend on their source changes in commit mode; `--verify-singlestep` and `--verify-dma` are independent test sets that can run alone or alongside a completion profile. Every commit includes the full-repository English language audit. Milestone is a separate set containing the same full audit; it does not include commit, DMA or SingleStep sets. All sets may run at any time on the current working tree, including a clean tree. Automatic CPU-suite selection uses staged, unstaged and nonignored new paths. Completion evidence must match the final committed source. Standalone DMA/SingleStep runs use the current working tree and do not replace completion verification. Use `--plan` to inspect commands. Targeted manual tests remain available during development; do not duplicate completion commands outside the verifier.

Before closing a minor roadmap point or major milestone, run the per-commit
command above once for the final snapshot, followed by
`python tests/verify.py --milestone`. Complete the maintenance and required skill
procedures from the routed roadmap, design-maintenance and diary rules before
claiming closure. The verifier does not invoke these skills, and passing its gates
alone does not establish stage completion. Revalidate affected checks if a procedure
changes their inputs. Do not claim unrun checks passed. **Reference Artifact Integrity:** Never silently change reference traces, expected cycle totals, or silicon vectors to make a failing test pass; follow the conflict and approval procedure in specification-compliance rules.

## Authoritative References

- [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) routes to subsystem specifications under `Obsidian/Amiga/Design/`.
- Hardware manuals and PRMs live under `Obsidian/Amiga/Reference/`; use RAG before scanning large manuals.
- `rag_qdrant` is the canonical CLI for the `projects_docs` collection, `amiga` source; the optional adapter is [tools/amiga-rag-mcp-server](tools/amiga-rag-mcp-server).
- Reference emulators and suites live under `ref_src/`; 68000 silicon vectors are under `ref_src/SingleStepTests-680x0/68000/v1`.
